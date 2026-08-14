use crate::{
    AppState,
    error::{HttpError, HttpResult},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{Json, extract::State, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, Utc};
use qsonaut_protocol::{BootstrapRequest, Credentials, CurrentUser, SetupStatus};
use rand::RngCore;
use sha2::{Digest, Sha256};
use time::Duration;

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
    if !valid {
        return Err(HttpError::unauthorized());
    }
    Ok((create_session(&state, jar, user.id).await?, Json(user)))
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
fn token_hash(token: &str) -> Vec<u8> {
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
