use crate::{
    AppState,
    error::{HttpError, HttpResult},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use qsonaut_protocol::{
    BootstrapRequest, Credentials, CurrentUser, DeviceAuthorizationApproval,
    DeviceAuthorizationDecisionInput, DeviceAuthorizationRequest, DeviceAuthorizationResponse,
    DeviceAuthorizationTokenResponse, DeviceCredentials, DeviceRegistration, DeviceToken,
    DeviceTokenRecord, PasswordResetInput, ProfileUpdateInput, SetupStatus, UserProfile,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use time::Duration;
use uuid::Uuid;
use qsonaut_store::DeviceAuthorizationGrantInput;

const COOKIE_NAME: &str = "qsonaut_session";
const DEVICE_AUTHORIZATION_TTL_MINUTES: i64 = 10;
const DEVICE_AUTHORIZATION_INTERVAL_SECONDS: u64 = 5;
const DEVICE_USER_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
pub(crate) const DEVICE_SCOPES: &[&str] = &[
    "events:read",
    "messages:read",
    "messages:write",
    "presence:write",
    "logs:write",
    "diagnostics:write",
];
#[utoipa::path(get, path = "/api/v1/auth/setup", tag = "authentication", responses((status = 200, body = SetupStatus)))]
pub(crate) async fn setup_status(State(state): State<AppState>) -> HttpResult<Json<SetupStatus>> {
    Ok(Json(SetupStatus {
        setup_required: state.store.setup_required().await?,
    }))
}
#[utoipa::path(post, path = "/api/v1/auth/setup", tag = "authentication", request_body = BootstrapRequest, responses((status = 200, body = CurrentUser), (status = 409, description = "Setup already completed")))]
pub(crate) async fn bootstrap(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<BootstrapRequest>,
) -> HttpResult<impl IntoResponse> {
    validate_identity(&input.callsign, &input.display_name, &input.password)?;
    if !state.store.setup_required().await? {
        return Err(HttpError::conflict("server setup is already complete"));
    }
    let hash = hash_password(input.password).await?;
    let user = state
        .store
        .create_administrator(
            &normalize_call(&input.callsign),
            input.display_name.trim(),
            &hash,
        )
        .await?
        .ok_or_else(|| HttpError::conflict("server setup is already complete"))?;
    Ok((create_session(&state, jar, user.id).await?, Json(user)))
}
#[utoipa::path(post, path = "/api/v1/auth/login", tag = "authentication", request_body = Credentials, responses((status = 200, body = CurrentUser), (status = 401, description = "Invalid credentials")))]
pub(crate) async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<Credentials>,
) -> HttpResult<impl IntoResponse> {
    let user = verify_credentials(&state, input).await?;
    Ok((create_session(&state, jar, user.id).await?, Json(user)))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/device/authorize",
    tag = "authentication",
    request_body = DeviceAuthorizationRequest,
    responses((status = 200, body = DeviceAuthorizationResponse), (status = 400, description = "Invalid device authorization request"))
)]
pub(crate) async fn device_authorize(
    State(state): State<AppState>,
    Json(input): Json<DeviceAuthorizationRequest>,
) -> HttpResult<Json<DeviceAuthorizationResponse>> {
    let client_id = validate_device_authorization_field(&input.client_id, 80, "client id")?;
    let device_name = validate_device_authorization_field(&input.device_name, 100, "device name")?;
    let client_version =
        validate_device_authorization_field(&input.client_version, 40, "client version")?;
    let mut device_bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut device_bytes);
    let device_code = URL_SAFE_NO_PAD.encode(device_bytes);
    let user_code = generate_user_code();
    let device_code_hash = token_hash(&device_code);
    let user_code_hash = token_hash(&user_code);
    let expires_at = Utc::now() + ChronoDuration::minutes(DEVICE_AUTHORIZATION_TTL_MINUTES);
    state
        .store
        .create_device_authorization_grant(&DeviceAuthorizationGrantInput {
            id: Uuid::new_v4(),
            client_id,
            device_name,
            client_version,
            device_code_hash: &device_code_hash,
            user_code_hash: &user_code_hash,
            expires_at,
            interval_seconds: i32::try_from(DEVICE_AUTHORIZATION_INTERVAL_SECONDS)
                .expect("constant fits i32"),
        })
        .await?;
    let verification_uri = format!(
        "{}/link",
        state.public_base_url.trim_end_matches('/')
    );
    let verification_uri_complete = Some(format!("{verification_uri}?user_code={user_code}"));
    Ok(Json(DeviceAuthorizationResponse {
        device_code,
        user_code,
        verification_uri,
        verification_uri_complete,
        expires_in: u64::try_from(DEVICE_AUTHORIZATION_TTL_MINUTES * 60)
            .expect("constant fits u64"),
        interval: DEVICE_AUTHORIZATION_INTERVAL_SECONDS,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct DeviceAuthorizationLookup {
    user_code: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/device/approval",
    tag = "authentication",
    params(("user_code" = String, Query)),
    responses((status = 200, body = DeviceAuthorizationApproval), (status = 401), (status = 404, description = "Authorization request not found or expired"))
)]
pub(crate) async fn device_approval(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<DeviceAuthorizationLookup>,
) -> HttpResult<Json<DeviceAuthorizationApproval>> {
    let _user = require_user(&state, &jar).await?;
    let user_code = normalize_user_code(&query.user_code);
    if user_code.is_empty() {
        return Err(HttpError::not_found());
    }
    let approval = state
        .store
        .device_authorization_for_user_code(&token_hash(&user_code))
        .await?
        .ok_or_else(HttpError::not_found)?;
    Ok(Json(DeviceAuthorizationApproval {
        device_name: approval.device_name,
        client_id: approval.client_id,
        client_version: approval.client_version,
        expires_at: approval.expires_at,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/device/approval",
    tag = "authentication",
    request_body = DeviceAuthorizationDecisionInput,
    responses((status = 204), (status = 401), (status = 404, description = "Authorization request not found or expired"))
)]
pub(crate) async fn decide_device_approval(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Json(input): Json<DeviceAuthorizationDecisionInput>,
) -> HttpResult<StatusCode> {
    let user = require_user(&state, &jar).await?;
    let user_code = normalize_user_code(&input.user_code);
    if user_code.is_empty()
        || !state
            .store
            .decide_device_authorization(user.id, &token_hash(&user_code), input.approved)
            .await?
    {
        return Err(HttpError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct DeviceTokenRequest {
    grant_type: String,
    device_code: String,
    client_id: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/device/token",
    tag = "authentication",
    request_body = DeviceTokenRequest,
    responses((status = 200, body = DeviceAuthorizationTokenResponse), (status = 400, description = "Device authorization is pending, denied, expired, or invalid"))
)]
pub(crate) async fn device_token(
    State(state): State<AppState>,
    Json(input): Json<DeviceTokenRequest>,
) -> HttpResult<Response> {
    if input.grant_type != "urn:ietf:params:oauth:grant-type:device_code" {
        return Ok(device_flow_error(StatusCode::BAD_REQUEST, "invalid_grant"));
    }
    let device_code = input.device_code.trim();
    let client_id = input.client_id.trim();
    if device_code.is_empty() || client_id.is_empty() {
        return Ok(device_flow_error(StatusCode::BAD_REQUEST, "invalid_grant"));
    }
    let Some((status, expires_at, _interval)) = state
        .store
        .device_authorization_status(&token_hash(device_code), client_id)
        .await?
    else {
        return Ok(device_flow_error(StatusCode::BAD_REQUEST, "expired_token"));
    };
    if expires_at <= Utc::now() || status == "consumed" {
        return Ok(device_flow_error(StatusCode::BAD_REQUEST, "expired_token"));
    }
    if status == "pending" {
        return Ok(device_flow_error(
            StatusCode::BAD_REQUEST,
            "authorization_pending",
        ));
    }
    if status == "denied" {
        return Ok(device_flow_error(StatusCode::BAD_REQUEST, "access_denied"));
    }
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let access_token = URL_SAFE_NO_PAD.encode(bytes);
    let expires_in = u64::try_from(ChronoDuration::days(90).num_seconds())
        .expect("constant fits u64");
    let scopes = serde_json::json!(DEVICE_SCOPES);
    let Some((user_id, _device_name)) = state
        .store
        .issue_device_token_from_authorization(
            &token_hash(device_code),
            client_id,
            Uuid::new_v4(),
            &token_hash(&access_token),
            Utc::now() + ChronoDuration::seconds(i64::try_from(expires_in).expect("fits i64")),
            &scopes,
        )
        .await?
    else {
        return Ok(device_flow_error(StatusCode::BAD_REQUEST, "expired_token"));
    };
    if state.store.user(user_id).await?.is_none() {
        return Ok(device_flow_error(StatusCode::BAD_REQUEST, "expired_token"));
    }
    Ok(Json(DeviceAuthorizationTokenResponse {
        access_token,
        token_type: "Bearer".to_owned(),
        expires_in,
    })
    .into_response())
}

fn device_flow_error(status: StatusCode, error: &'static str) -> Response {
    (status, Json(serde_json::json!({ "error": error }))).into_response()
}

fn validate_device_authorization_field<'a>(
    value: &'a str,
    max_length: usize,
    name: &str,
) -> HttpResult<&'a str> {
    let value = value.trim();
    if value.is_empty() || value.len() > max_length {
        return Err(HttpError::bad_request(format!(
            "{name} must contain 1 to {max_length} characters"
        )));
    }
    Ok(value)
}

fn generate_user_code() -> String {
    let mut bytes = [0_u8; 8];
    rand::rng().fill_bytes(&mut bytes);
    let mut code = String::with_capacity(9);
    for (index, byte) in bytes.into_iter().enumerate() {
        if index == 4 {
            code.push('-');
        }
        code.push(DEVICE_USER_CODE_ALPHABET[usize::from(byte) % DEVICE_USER_CODE_ALPHABET.len()] as char);
    }
    code
}

fn normalize_user_code(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

#[utoipa::path(post, path = "/api/v1/auth/device", tag = "authentication", request_body = DeviceCredentials, responses((status = 200, body = DeviceToken), (status = 401, description = "Invalid credentials")))]
pub(crate) async fn device_login(
    State(state): State<AppState>,
    Json(input): Json<DeviceCredentials>,
) -> HttpResult<Json<DeviceToken>> {
    let user = verify_credentials(
        &state,
        Credentials {
            callsign: input.callsign,
            password: input.password,
        },
    )
    .await?;
    issue_device_token(&state, user, &input.device_name).await
}

#[utoipa::path(post, path = "/api/v1/auth/device/session", tag = "authentication", request_body = DeviceRegistration, responses((status = 200, body = DeviceToken), (status = 401, description = "Not authenticated")))]
pub(crate) async fn register_session_device(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<DeviceRegistration>,
) -> HttpResult<Json<DeviceToken>> {
    let user = require_user(&state, &jar).await?;
    issue_device_token(&state, user, &input.device_name).await
}

#[utoipa::path(get, path = "/api/v1/auth/devices", tag = "authentication", responses((status = 200, body = [DeviceTokenRecord])))]
pub(crate) async fn session_devices(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<Vec<DeviceTokenRecord>>> {
    let user = require_user(&state, &jar).await?;
    Ok(Json(state.store.device_tokens(user.id).await?))
}

#[utoipa::path(delete, path = "/api/v1/auth/devices/{token_id}", tag = "authentication", params(("token_id" = Uuid, Path)), responses((status = 204), (status = 404)))]
pub(crate) async fn revoke_session_device(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(token_id): Path<Uuid>,
) -> HttpResult<StatusCode> {
    let user = require_user(&state, &jar).await?;
    if !state
        .store
        .delete_device_token_by_id(user.id, token_id)
        .await?
    {
        return Err(HttpError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/auth/devices/{token_id}/reissue", tag = "authentication", params(("token_id" = Uuid, Path)), responses((status = 200, body = DeviceToken), (status = 404)))]
pub(crate) async fn reissue_session_device(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(token_id): Path<Uuid>,
) -> HttpResult<Json<DeviceToken>> {
    let user = require_user(&state, &jar).await?;
    let name = state
        .store
        .device_token_name(user.id, token_id)
        .await?
        .ok_or_else(HttpError::not_found)?;
    let replacement = issue_device_token(&state, user.clone(), &name).await?;
    state
        .store
        .delete_device_token_by_id(user.id, token_id)
        .await?;
    Ok(replacement)
}

async fn issue_device_token(
    state: &AppState,
    user: CurrentUser,
    device_name: &str,
) -> HttpResult<Json<DeviceToken>> {
    let name = device_name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(HttpError::bad_request(
            "device name must contain 1 to 100 characters",
        ));
    }
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let expires_at = Utc::now() + ChronoDuration::days(90);
    let scopes = serde_json::json!(DEVICE_SCOPES);
    state
        .store
        .create_device_token(user.id, name, &token_hash(&token), expires_at, &scopes)
        .await?;
    Ok(Json(DeviceToken {
        token,
        user,
        expires_at,
        scopes: DEVICE_SCOPES
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect(),
    }))
}

#[utoipa::path(delete, path = "/api/v1/auth/device", tag = "authentication", responses((status = 204, description = "Device token revoked")))]
pub(crate) async fn revoke_device(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> HttpResult<StatusCode> {
    let token = bearer_token(&headers)?;
    require_device(&state, &headers).await?;
    state.store.delete_device_token(&token_hash(token)).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get, path = "/api/v1/auth/me", tag = "authentication", responses((status = 200, body = CurrentUser), (status = 401, description = "Not authenticated")))]
pub(crate) async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<CurrentUser>> {
    Ok(Json(require_user(&state, &jar).await?))
}

#[utoipa::path(patch, path = "/api/v1/auth/me", tag = "authentication", request_body = ProfileUpdateInput, responses((status = 200, body = CurrentUser), (status = 401)))]
pub(crate) async fn update_me(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ProfileUpdateInput>,
) -> HttpResult<Json<CurrentUser>> {
    let user = require_user(&state, &jar).await?;
    validate_display_name(&input.display_name)?;
    Ok(Json(
        state
            .store
            .update_user_display_name(user.id, input.display_name.trim())
            .await?
            .ok_or_else(HttpError::not_found)?,
    ))
}

#[utoipa::path(get, path = "/api/v1/auth/profile", tag = "authentication", responses((status = 200, body = UserProfile), (status = 401)))]
pub(crate) async fn profile(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<UserProfile>> {
    let user = require_user(&state, &jar).await?;
    Ok(Json(
        state
            .store
            .user_profile(user.id)
            .await?
            .ok_or_else(HttpError::not_found)?,
    ))
}

#[utoipa::path(patch, path = "/api/v1/auth/profile", tag = "authentication", request_body = ProfileUpdateInput, responses((status = 200, body = UserProfile), (status = 401)))]
pub(crate) async fn update_profile(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ProfileUpdateInput>,
) -> HttpResult<Json<UserProfile>> {
    let user = require_user(&state, &jar).await?;
    validate_profile(&input)?;
    Ok(Json(
        state
            .store
            .update_user_profile(user.id, &input)
            .await?
            .ok_or_else(HttpError::not_found)?,
    ))
}

#[derive(Debug, serde::Deserialize)]
struct HamDbResponse {
    hamdb: HamDbPayload,
}

#[derive(Debug, serde::Deserialize)]
struct HamDbPayload {
    callsign: HamDbCallsign,
}

#[derive(Debug, serde::Deserialize)]
struct HamDbCallsign {
    #[serde(default)]
    call: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    expires: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    grid: String,
    #[serde(default, alias = "lat")]
    latitude: String,
    #[serde(default, alias = "lon")]
    longitude: String,
    #[serde(default, alias = "fname")]
    first_name: String,
    #[serde(default, alias = "mi")]
    middle_name: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    suffix: String,
    #[serde(default, alias = "addr1")]
    address_line_1: String,
    #[serde(default, alias = "addr2")]
    address_line_2: String,
    #[serde(default)]
    state: String,
    #[serde(default, alias = "zip")]
    postal_code: String,
    #[serde(default)]
    country: String,
}

#[utoipa::path(post, path = "/api/v1/auth/profile/hamdb", tag = "authentication", responses((status = 200, body = UserProfile), (status = 502, description = "HamDB unavailable or returned invalid data")))]
pub(crate) async fn refresh_hamdb_profile(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<Json<UserProfile>> {
    let user = require_user(&state, &jar).await?;
    let url = format!("https://api.hamdb.org/{}/json/QSONaut", user.callsign);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("QSONaut-Server/0.1")
        .build()
        .map_err(|_| HttpError::bad_gateway("could not initialize HamDB client"))?;
    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "HamDB request failed");
            state
                .store
                .set_hamdb_error(user.id, "HamDB could not be reached")
                .await?;
            return Err(HttpError::bad_gateway("HamDB could not be reached"));
        }
    };
    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "HamDB returned an error");
            state
                .store
                .set_hamdb_error(user.id, "HamDB rejected the lookup")
                .await?;
            return Err(HttpError::bad_gateway("HamDB rejected the lookup"));
        }
    };
    let response = match response.json::<HamDbResponse>().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "HamDB response was invalid");
            state
                .store
                .set_hamdb_error(user.id, "HamDB returned invalid profile data")
                .await?;
            return Err(HttpError::bad_gateway(
                "HamDB returned invalid profile data",
            ));
        }
    };
    let ham = response.hamdb.callsign;
    if normalize_call(&ham.call) != user.callsign || ham.status.eq_ignore_ascii_case("invalid") {
        state
            .store
            .set_hamdb_error(user.id, "HamDB did not return the requested callsign")
            .await?;
        return Err(HttpError::bad_gateway(
            "HamDB did not return the requested callsign",
        ));
    }
    let profile = ProfileUpdateInput {
        display_name: user.display_name,
        grid: ham.grid,
        qth: String::new(),
        first_name: ham.first_name,
        middle_name: ham.middle_name,
        surname: ham.name,
        suffix: ham.suffix,
        license_class: ham.class,
        license_status: ham.status,
        license_expires_on: parse_hamdb_date(&ham.expires),
        address_line_1: ham.address_line_1,
        address_line_2: ham.address_line_2,
        state: ham.state,
        postal_code: ham.postal_code,
        country: ham.country,
        latitude: ham.latitude,
        longitude: ham.longitude,
    };
    validate_profile(&profile)?;
    Ok(Json(
        state
            .store
            .update_hamdb_profile(user.id, &profile)
            .await?
            .ok_or_else(HttpError::not_found)?,
    ))
}

#[utoipa::path(post, path = "/api/v1/auth/me/password", tag = "authentication", request_body = PasswordResetInput, responses((status = 204), (status = 401)))]
pub(crate) async fn update_my_password(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<PasswordResetInput>,
) -> HttpResult<StatusCode> {
    let user = require_user(&state, &jar).await?;
    validate_password(&input.password)?;
    let hash = hash_password(input.password).await?;
    if !state
        .store
        .update_member_password_hash(user.id, &hash)
        .await?
    {
        return Err(HttpError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "authentication", responses((status = 204, description = "Session revoked")))]
pub(crate) async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> HttpResult<impl IntoResponse> {
    if let Some(token) = jar.get(COOKIE_NAME).map(|c| c.value().to_owned()) {
        state.store.delete_session(&token_hash(&token)).await?;
    }
    Ok((
        jar.remove(
            Cookie::build(COOKIE_NAME)
                .path("/")
                .max_age(Duration::ZERO)
                .build(),
        ),
        axum::http::StatusCode::NO_CONTENT,
    ))
}
pub(crate) async fn require_user(state: &AppState, jar: &CookieJar) -> HttpResult<CurrentUser> {
    let token = jar
        .get(COOKIE_NAME)
        .ok_or_else(HttpError::unauthorized)?
        .value();
    state
        .store
        .session_user(&token_hash(token))
        .await?
        .ok_or_else(HttpError::unauthorized)
}
pub(crate) async fn require_admin(state: &AppState, jar: &CookieJar) -> HttpResult<CurrentUser> {
    let user = require_user(state, jar).await?;
    if user.global_role != "administrator" {
        return Err(HttpError::forbidden());
    }
    Ok(user)
}
pub(crate) async fn require_device(
    state: &AppState,
    headers: &HeaderMap,
) -> HttpResult<CurrentUser> {
    state
        .store
        .device_token_user(&token_hash(bearer_token(headers)?))
        .await?
        .ok_or_else(HttpError::unauthorized)
}

pub(crate) async fn require_device_with_scopes(
    state: &AppState,
    headers: &HeaderMap,
) -> HttpResult<(CurrentUser, Vec<String>)> {
    let token = bearer_token(headers)?;
    let hash = token_hash(token);
    let user = state
        .store
        .device_token_user(&hash)
        .await?
        .ok_or_else(HttpError::unauthorized)?;
    let scopes = state
        .store
        .device_token_scopes(&hash)
        .await?
        .ok_or_else(HttpError::unauthorized)?;
    Ok((user, scopes))
}

async fn verify_credentials(state: &AppState, input: Credentials) -> HttpResult<CurrentUser> {
    let Some((user, hash)) = state
        .store
        .user_credentials(&normalize_call(&input.callsign))
        .await?
    else {
        return Err(HttpError::unauthorized());
    };
    let password = input.password;
    let valid = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).is_ok_and(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
    })
    .await
    .map_err(|_| HttpError::internal())?;
    if valid {
        Ok(user)
    } else {
        Err(HttpError::unauthorized())
    }
}

fn bearer_token(headers: &HeaderMap) -> HttpResult<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or_else(HttpError::unauthorized)
}
async fn create_session(
    state: &AppState,
    jar: CookieJar,
    user_id: uuid::Uuid,
) -> HttpResult<CookieJar> {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    state
        .store
        .create_session(
            user_id,
            &token_hash(&token),
            Utc::now() + ChronoDuration::days(7),
        )
        .await?;
    Ok(jar.add(
        Cookie::build((COOKIE_NAME, token))
            .http_only(true)
            .same_site(SameSite::Strict)
            .secure(state.secure_cookies)
            .path("/")
            .max_age(Duration::days(7))
            .build(),
    ))
}
pub(crate) fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
pub(crate) fn normalize_call(call: &str) -> String {
    call.trim().to_ascii_uppercase()
}
pub(crate) fn validate_identity(call: &str, name: &str, password: &str) -> HttpResult<()> {
    validate_callsign(call)?;
    validate_display_name(name)?;
    validate_password(password)
}

pub(crate) fn validate_callsign(call: &str) -> HttpResult<()> {
    let call = normalize_call(call);
    if call.len() < 3
        || call.len() > 16
        || !call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
    {
        return Err(HttpError::bad_request("enter a valid callsign"));
    }
    Ok(())
}

pub(crate) fn validate_display_name(name: &str) -> HttpResult<()> {
    if name.trim().is_empty() || name.trim().len() > 100 {
        return Err(HttpError::bad_request(
            "display name must contain 1 to 100 characters",
        ));
    }
    Ok(())
}

pub(crate) fn validate_password(password: &str) -> HttpResult<()> {
    if !(12..=256).contains(&password.len()) {
        return Err(HttpError::bad_request(
            "password must contain 12 to 256 characters",
        ));
    }
    Ok(())
}

fn validate_profile(input: &ProfileUpdateInput) -> HttpResult<()> {
    validate_display_name(&input.display_name)?;
    let fields = [
        ("grid", input.grid.as_str(), 16),
        ("QTH", input.qth.as_str(), 160),
        ("first name", input.first_name.as_str(), 80),
        ("middle name", input.middle_name.as_str(), 80),
        ("surname", input.surname.as_str(), 120),
        ("suffix", input.suffix.as_str(), 40),
        ("license class", input.license_class.as_str(), 40),
        ("license status", input.license_status.as_str(), 40),
        ("address", input.address_line_1.as_str(), 160),
        ("address", input.address_line_2.as_str(), 160),
        ("state", input.state.as_str(), 80),
        ("postal code", input.postal_code.as_str(), 32),
        ("country", input.country.as_str(), 80),
        ("latitude", input.latitude.as_str(), 32),
        ("longitude", input.longitude.as_str(), 32),
    ];
    if let Some((name, _, max)) = fields
        .iter()
        .find(|(_, value, max)| value.trim().len() > *max)
    {
        return Err(HttpError::bad_request(format!(
            "{name} is too long (maximum {max} characters)"
        )));
    }
    Ok(())
}

fn parse_hamdb_date(value: &str) -> Option<NaiveDate> {
    ["%Y-%m-%d", "%m/%d/%Y", "%Y/%m/%d"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(value.trim(), format).ok())
}
pub(crate) async fn hash_password(password: String) -> HttpResult<String> {
    tokio::task::spawn_blocking(move || {
        let mut bytes = [0_u8; 16];
        rand::rng().fill_bytes(&mut bytes);
        let salt = SaltString::encode_b64(&bytes).map_err(|_| HttpError::internal())?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| HttpError::internal())
    })
    .await
    .map_err(|_| HttpError::internal())?
}
