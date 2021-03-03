use crate::error::Error;
use crate::{BoxedSerialize, Id, V2};
use serde::{self, Serialize};

/// The individual response object
#[derive(Serialize)]
#[serde(untagged)]
pub enum ResponseObject {
    Result { jsonrpc: V2, result: BoxedSerialize, id: Id },
    Error { jsonrpc: V2, error: Error, id: Id },
}

impl ResponseObject {
    pub(crate) fn result(result: BoxedSerialize, id: Id) -> Self {
        ResponseObject::Result { jsonrpc: V2, result, id }
    }

    pub(crate) fn error(error: Error, id: Id) -> Self {
        ResponseObject::Error { jsonrpc: V2, error, id }
    }
}

/// Container for the response object(s) or `Empty` for notification request(s)
#[derive(Serialize)]
#[serde(untagged)]
pub enum ResponseObjects {
    One(ResponseObject),
    Many(Vec<ResponseObject>),
    Empty,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum ManyResponseObjects {
    Many(Vec<ResponseObject>),
    Empty,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum SingleResponseObject {
    One(ResponseObject),
    Empty,
}

impl From<ManyResponseObjects> for ResponseObjects {
    fn from(t: ManyResponseObjects) -> Self {
        match t {
            ManyResponseObjects::Many(many) => ResponseObjects::Many(many),
            ManyResponseObjects::Empty => ResponseObjects::Empty,
        }
    }
}

impl From<SingleResponseObject> for ResponseObjects {
    fn from(t: SingleResponseObject) -> Self {
        match t {
            SingleResponseObject::One(one) => ResponseObjects::One(one),
            SingleResponseObject::Empty => ResponseObjects::Empty,
        }
    }
}

impl SingleResponseObject {
    pub(crate) fn result(result: BoxedSerialize, opt_id: Id) -> Self {
        match &opt_id {
            Id::Null => SingleResponseObject::Empty,
            _ => SingleResponseObject::One(ResponseObject::result(result, opt_id)),
        }
    }

    pub(crate) fn error(error: Error, opt_id: Id) -> Self {
        match &opt_id {
            Id::Null => SingleResponseObject::Empty,
            _ => SingleResponseObject::One(ResponseObject::error(error, opt_id)),
        }
    }
}
