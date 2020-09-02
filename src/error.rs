use serde::{Serialize};

use crate::BoxedSerialize;

/// Error object in a response
#[derive(Serialize)]
#[serde(untagged)]
pub enum Error {
    Full {
        code: i64,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<BoxedSerialize>,
    },
    Provided {
        code: i64,
        message: &'static str,
    },
}

impl Error {
    pub const INVALID_REQUEST: Self = Error::Provided { code: -32600, message: "Invalid Request" };
    pub const METHOD_NOT_FOUND: Self =
        Error::Provided { code: -32601, message: "Method not found" };
    pub const INVALID_PARAMS: Self = Error::Provided { code: -32602, message: "Invalid params" };
    pub const INTERNAL_ERROR: Self = Error::Provided { code: -32603, message: "Internal Error" };
    pub const PARSE_ERROR: Self = Error::Provided { code: -32700, message: "Parse error" };

    pub fn internal<D: std::fmt::Display + Send>(e: D) -> Self {
        Error::Full {
            code: -32603,
            message: "Internal Error".into(),
            data: Some(Box::new(e.to_string())),
        }
    }
}

/// Trait that can be used to map custom errors to the [`Error`](enum.Error.html) object.
pub trait ErrorLike: std::fmt::Display {
    /// Code to be used in JSON-RPC 2.0 Error object. Default is 0.
    fn code(&self) -> i64 {
        0
    }

    /// Message to be used in JSON-RPC 2.0 Error object. Default is the `Display` value of the item.
    fn message(&self) -> String {
        self.to_string()
    }

    /// Any additional data to be sent with the error. Default is `None`.
    fn data(&self) -> Option<BoxedSerialize> {
        None
    }
}

impl<T> From<T> for Error
where
    T: ErrorLike,
{
    fn from(t: T) -> Error {
        Error::Full { code: t.code(), message: t.message(), data: t.data() }
    }
}

#[cfg(feature = "easy-errors")]
impl<T> ErrorLike for T where T: std::fmt::Display {}
