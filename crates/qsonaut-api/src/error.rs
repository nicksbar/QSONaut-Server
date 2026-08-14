use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use qsonaut_protocol::ApiError;

pub(crate) struct HttpError {
    status: StatusCode,
    message: String,
}
impl HttpError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
    pub(crate) fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "authentication required".into(),
        }
    }
    pub(crate) fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: "administrator access required".into(),
        }
    }
    pub(crate) fn forbidden_with(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }
    pub(crate) fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }
    pub(crate) fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "internal server error".into(),
        }
    }
    pub(crate) fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: "record not found".into(),
        }
    }
}
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiError {
                error: self.message,
            }),
        )
            .into_response()
    }
}
pub(crate) type HttpResult<T> = Result<T, HttpError>;
impl From<sqlx::Error> for HttpError {
    fn from(error: sqlx::Error) -> Self {
        if matches!(error, sqlx::Error::RowNotFound) {
            return Self::not_found();
        }
        tracing::error!(%error, "database request failed");
        if matches!(error, sqlx::Error::Database(ref db) if db.is_unique_violation()) {
            return Self::conflict("record already exists");
        }
        Self::internal()
    }
}
