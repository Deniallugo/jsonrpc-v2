use crate::error::Error;
use crate::notification::NotificationBuilder;
use crate::{Id, V2};
use erased_serde::private::serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;
use serde_json::Value;
use std::sync::Arc;

#[cfg(not(feature = "bytes-v04"))]
use bytes::Bytes;

#[cfg(feature = "bytes-v04")]
use bytes_v04::Bytes;

/// Builder struct for a request object
#[derive(Default)]
pub struct RequestBuilder<M = ()> {
    id: Id,
    params: Option<Value>,
    method: M,
}

impl<M> RequestBuilder<M> {
    pub fn with_id<I: Into<Id>>(mut self, id: I) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_params<I: Into<Value>>(mut self, params: I) -> Self {
        self.params = Some(params.into());
        self
    }
}

impl RequestBuilder<()> {
    pub fn with_method<I: Into<String>>(self, method: I) -> RequestBuilder<String> {
        let RequestBuilder { id, params, .. } = self;
        RequestBuilder { id, params, method: method.into() }
    }
}

impl RequestBuilder<String> {
    pub fn finish(self) -> RequestObject {
        let RequestBuilder { id, params, method } = self;
        RequestObject {
            jsonrpc: V2,
            method: method.into_boxed_str(),
            params: params.map(InnerParams::Value),
            id: Some(Some(id)),
        }
    }
}

/// [`FromRequest`](trait.FromRequest.html) wrapper for request params
///
/// Use a tuple to deserialize by-position params
/// and a map or deserializable struct for by-name params: e.g.
///
/// ```
#[derive(paperclip::actix::Apiv2Schema, Deserialize)]
pub struct Params<T>(pub T);

/// A trait to extract data from the request
#[async_trait::async_trait]
pub trait FromRequest: Sized {
    async fn from_request(req: &RequestObject) -> Result<Self, Error>;
}

#[async_trait::async_trait]
impl<T: DeserializeOwned> FromRequest for Params<T> {
    async fn from_request(req: &RequestObject) -> Result<Self, Error> {
        let res = match req.params {
            Some(InnerParams::Raw(ref value)) => serde_json::from_str(value.get()),
            Some(InnerParams::Value(ref value)) => serde_json::from_value(value.clone()),
            None => serde_json::from_value(Value::Null),
        };

        Ok(res.map(Params).map_err(|_| Error::INVALID_PARAMS)?)
    }
}

/// Data/state storage container
pub struct Data<T>(pub Arc<T>);

impl<T> Data<T> {
    pub fn new(t: T) -> Self {
        Data(Arc::new(t))
    }
}

impl<T> std::ops::Deref for Data<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InnerParams {
    Value(Value),
    Raw(Box<RawValue>),
}

/// Request/Notification object
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct RequestObject {
    pub jsonrpc: V2,
    pub method: Box<str>,
    pub params: Option<InnerParams>,
    #[serde(deserialize_with = "RequestObject::deserialize_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Option<Id>>,
}

/// Request/Notification object
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct BytesRequestObject {
    pub(crate) jsonrpc: V2,
    pub(crate) method: Box<str>,
    pub(crate) params: Option<Box<RawValue>>,
    #[serde(deserialize_with = "RequestObject::deserialize_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<Option<Id>>,
}

impl From<BytesRequestObject> for RequestObject {
    fn from(t: BytesRequestObject) -> Self {
        let BytesRequestObject { jsonrpc, method, params, id } = t;
        RequestObject { jsonrpc, method, params: params.map(InnerParams::Raw), id }
    }
}

impl RequestObject {
    /// Build a new request object
    pub fn request() -> RequestBuilder {
        RequestBuilder::default()
    }

    /// Build a new notification request object
    pub fn notification() -> NotificationBuilder {
        NotificationBuilder::default()
    }

    fn deserialize_id<'de, D>(deserializer: D) -> Result<Option<Option<Id>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Some(Option::deserialize(deserializer)?))
    }
}

/// An enum to contain the different kinds of possible requests: using the provided
/// [`RequestObject`](struct.RequestObject.html), an array of `RequestObject`s, or raw bytes.
///
/// Typically not use directly, [`Server::handle`](struct.Server.html#method.handle) can take the individual variants
pub enum RequestKind {
    RequestObject(RequestObject),
    ManyRequestObjects(Vec<RequestObject>),
    Bytes(Bytes),
}

impl From<RequestObject> for RequestKind {
    fn from(t: RequestObject) -> Self {
        RequestKind::RequestObject(t)
    }
}

impl From<Vec<RequestObject>> for RequestKind {
    fn from(t: Vec<RequestObject>) -> Self {
        RequestKind::ManyRequestObjects(t)
    }
}

impl From<Bytes> for RequestKind {
    fn from(t: Bytes) -> Self {
        RequestKind::Bytes(t)
    }
}

impl<'a> From<&'a [u8]> for RequestKind {
    fn from(t: &'a [u8]) -> Self {
        Bytes::from(t.to_vec()).into()
    }
}

#[derive(paperclip::actix::Apiv2Schema, Serialize, Deserialize)]
pub struct DummyReq {}
