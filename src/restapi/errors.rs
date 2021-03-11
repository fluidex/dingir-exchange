use serde::Serialize;
use std::fmt;
use thiserror::Error;

use actix_web::error::{QueryPayloadError, ResponseError};
use actix_web::{http::StatusCode, HttpResponse};

// It is better to use strong typed error for APIs.
// Use thiserror rather than anyhow if you are a library or service that wants to design your own dedicated error type(s)
// so that on failures the caller gets exactly the information that you choose.
// Use Anyhow if you don't care what error type your functions return, you just want it to be easy.
#[derive(Error, Debug, Clone, Serialize)]
pub enum ErrorType {
    #[error("Requested resource was not found")]
    NotFound,
    #[error("You are forbidden to access")]
    Forbidden,
    #[error("Invalid request")]
    BadRequest,
    #[error("Unknown Internal Error")]
    Unknown,
}

#[derive(Serialize, Clone, Debug)]
pub struct RpcError {
    pub error: ErrorType,
    pub message: String,
}

impl RpcError {
    pub fn new(error: ErrorType, message: String) -> RpcError {
        RpcError { error, message }
    }
    pub fn unknown(message: &str) -> RpcError {
        Self::new(ErrorType::Unknown, message.to_string())
    }
    pub fn bad_request(message: &str) -> RpcError {
        Self::new(ErrorType::BadRequest, message.to_string())
    }
}
impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.error, self.message)
    }
}

impl ResponseError for RpcError {
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }

    fn error_response(&self) -> HttpResponse {
        // all http response are 200. we handle the error inside json
        HttpResponse::build(StatusCode::OK).json(self)
    }
}

impl From<QueryPayloadError> for RpcError {
    fn from(original: QueryPayloadError) -> RpcError {
        RpcError::bad_request(&original.to_string())
    }
}

impl From<sqlx::Error> for RpcError {
    fn from(original: sqlx::Error) -> RpcError {
        match original {
            sqlx::error::Error::RowNotFound => RpcError::new(ErrorType::NotFound, "db query nothing".to_string()),
            sqlx::error::Error::Database(e) => RpcError::bad_request(&e.to_string()),
            _ => RpcError::unknown(&original.to_string()),
        }
    }
}
