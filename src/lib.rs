/*!
A very small and very fast JSON-RPC 2.0 server-focused framework.

Provides integrations for both `hyper` and `actix-web` (both 1.x and 2.x).
Enable features `actix-web-v1-integration`, `actix-web-v2-integration`, or `hyper-integration` depending on need.

`actix-web-v2-integration` is enabled by default. Make sure to add `default-features = false` if using `hyper` or `actix-web` 1.x.

Also see the `easy-errors` feature flag (not enabled by default). Enabling this flag will implement [`ErrorLike`](https://docs.rs/jsonrpc-v2/&#42;/jsonrpc_v2/trait.ErrorLike.html)
for anything that implements `Display`, and the display value will be provided in the `message` field of the JSON-RPC 2.0 `Error` response.

Otherwise, custom errors should implement [`ErrorLike`](https://docs.rs/jsonrpc-v2/&#42;/jsonrpc_v2/trait.ErrorLike.html) to map errors to the JSON-RPC 2.0 `Error` response.

Individual method handlers are `async` functions that can take various kinds of args (things that can be extracted from the request, like
the `Params` or `Data`), and should return a `Result<Item, Error>` where the `Item` is serializable. See examples below.

# Usage

*/

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use serde_json::value::RawValue;

pub mod documentation;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod notification;
pub mod request;
pub mod response;
pub mod router;
pub mod server;

pub use error::{Error, ErrorLike};
pub use notification::NotificationBuilder;
pub use request::{DummyReq, Params};
use serde::export::Formatter;
pub use server::{Metadata, Server};


pub type BoxedSerialize = Box<dyn erased_serde::Serialize + Send>;

#[doc(hidden)]
#[derive(Default, Debug)]
pub struct V2;

impl Serialize for V2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        "2.0".serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for V2 {
    fn deserialize<D>(deserializer: D) -> Result<V2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        if s == "2.0" {
            Ok(V2)
        } else {
            Err(serde::de::Error::custom("Could not deserialize V2"))
        }
    }
}

/// Container for the request ID, which can be a string, number, or null.
/// Not typically used directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
    Num(i64),
    Str(Box<str>),
    Null,
}

impl Id {
    pub fn is_null(&self) -> bool {
        match self {
            Id::Null => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Null => write!(f, "null"),
            Id::Num(res) => write!(f, "{}", res),
            Id::Str(res) => write!(f, "{}", res),
        }
    }
}

impl From<i64> for Id {
    fn from(t: i64) -> Self {
        Id::Num(t)
    }
}

impl<'a> From<&'a str> for Id {
    fn from(t: &'a str) -> Self {
        Id::Str(t.into())
    }
}

impl From<String> for Id {
    fn from(t: String) -> Self {
        Id::Str(t.into())
    }
}

impl Default for Id {
    fn default() -> Self {
        Id::Null
    }
}

#[derive(Debug)]
enum OneOrManyRawValues<'a> {
    Many(Vec<&'a RawValue>),
    One(&'a RawValue),
}

impl<'a> OneOrManyRawValues<'a> {
    pub fn try_from_slice(slice: &'a [u8]) -> Result<Self, serde_json::Error> {
        if slice.first() == Some(&b'[') {
            Ok(OneOrManyRawValues::Many(serde_json::from_slice::<Vec<&RawValue>>(slice)?))
        } else {
            Ok(OneOrManyRawValues::One(serde_json::from_slice::<&RawValue>(slice)?))
        }
    }
}
