use crate::{
    AppState,
    error::{HttpError, HttpResult},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, Utc};
use qsonaut_protocol::{
    BootstrapRequest, Credentials, CurrentUser, DeviceCredentials, DeviceRegistration, DeviceToken,
    DeviceTokenRecord, SetupStatus,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use time::Duration;
use uuid::Uuid;

const COOKIE_NAME: &str = "qsonaut_session";
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
    state
        .store
        .create_device_token(user.id, name, &token_hash(&token), expires_at)
        .await?;
    Ok(Json(DeviceToken {
        token,
        user,
        expires_at,
        scopes: vec![
            "events:read".to_owned(),
            "messages:read".to_owned(),
            "messages:write".to_owned(),
            "presence:write".to_owned(),
            "logs:write".to_owned(),
            "diagnostics:write".to_owned(),
        ],
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
        PasswordHash::new(&hash).ok().is_some_and(|parsed| {
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
