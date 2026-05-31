use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("{1}")]
    HttpStatus(u16, String),
    #[error("{message}")]
    HttpJson {
        status: u16,
        code: String,
        message: String,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::HttpStatus(status, code) => {
                let status =
                    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                (status, Json(json!({ "code": code, "message": code }))).into_response()
            }
            AppError::HttpJson {
                status,
                code,
                message,
            } => {
                let status =
                    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                (status, Json(json!({ "code": code, "message": message }))).into_response()
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": "internal_error", "message": "服务端错误" })),
            )
                .into_response(),
        }
    }
}
