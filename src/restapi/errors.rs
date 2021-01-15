use serde::Serialize;
use std::fmt;
use thiserror::Error;

use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};

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
    error: ErrorType,
    message: String,
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
