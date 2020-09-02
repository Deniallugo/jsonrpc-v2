use crate::request::{InnerParams, RequestObject};
use crate::V2;
use serde_json::Value;

/// Builder struct for a notification request object
#[derive(Default)]
pub struct NotificationBuilder<M = ()> {
    params: Option<Value>,
    method: M,
}

impl<M> NotificationBuilder<M> {
    pub fn with_params<I: Into<Value>>(mut self, params: I) -> Self {
        self.params = Some(params.into());
        self
    }
}

impl NotificationBuilder<()> {
    pub fn with_method<I: Into<String>>(self, method: I) -> NotificationBuilder<String> {
        let NotificationBuilder { params, .. } = self;
        NotificationBuilder { params, method: method.into() }
    }
}

impl NotificationBuilder<String> {
    pub fn finish(self) -> RequestObject {
        let NotificationBuilder { method, params } = self;
        RequestObject {
            jsonrpc: V2,
            method: method.into_boxed_str(),
            params: params.map(InnerParams::Value),
            id: None,
        }
    }
}
