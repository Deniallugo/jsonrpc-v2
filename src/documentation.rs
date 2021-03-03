use serde::{Deserialize, Serialize};

use paperclip::v2::models::DefaultSchemaRaw;


use crate::error::Error;
use crate::handler::Factory;
use crate::request::Params;
use crate::server::Metadata;
use crate::DummyReq;

type DocType = Vec<DocRoute>;
type Notifications = Vec<DocNotification>;

#[derive(Clone)]
pub(crate) struct SpecHandler {
    pub(crate) routes: DocType,
    pub(crate) notifications: Notifications,
}

#[async_trait::async_trait]
impl<M> Factory<(DocType, Notifications), Error, Params<DummyReq>, M> for SpecHandler
where
    M: Metadata,
{
    async fn call(&self, _: Params<DummyReq>, _: M) -> Result<(DocType, Notifications), Error> {
        Ok((self.routes.clone(), self.notifications.clone()))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DocNotification {
    pub(crate) name: String,
    pub(crate) notification: DefaultSchemaRaw,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DocRoute {
    pub(crate) name: String,
    pub(crate) request: DefaultSchemaRaw,
    pub(crate) response: DefaultSchemaRaw,
}
