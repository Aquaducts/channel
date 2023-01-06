use actix_http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use serde_json::json;
use tracing::error;

macro_rules! default_http_status {
    ($name:ident, $status:expr) => {
        pub fn $name<T: Into<String>>(msg: T) -> Error {
            Error {
                code: $status,
                msg: msg.into(),
            }
        }
    };
}

#[derive(Debug)]
pub struct Error {
    pub code: StatusCode,
    pub msg: String,
}

impl Error {
    pub fn new(code: StatusCode, msg: String) -> Error {
        Self { code, msg }
    }

    pub fn failed_to_create(resource: String) -> Error {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            msg: format!(
                "There was an error while trying to create that {}. Please try again later.",
                resource
            ),
        }
    }

    pub fn mesage(&mut self, new_message: String) -> &Self {
        self.msg = new_message;
        self
    }

    default_http_status!(bad_request, StatusCode::BAD_REQUEST);
    default_http_status!(forbidden, StatusCode::FORBIDDEN);
    default_http_status!(internal_server_error, StatusCode::INTERNAL_SERVER_ERROR);
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status code {}: {}", self.code, self.msg)
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        error!("Route returned a {} error code: {}", self.code, self.msg);
        HttpResponseBuilder::new(self.code).json(json!({
            "msg": self.msg
        }))
    }
}