use crate::error::Error;
use crate::handler::BoxedHandler;
use crate::request::RequestObject;
use crate::server::Metadata;
use crate::BoxedSerialize;
use std::sync::Arc;

pub struct Next<'a, 'b, M: Metadata> {
    pub(crate) endpoint: &'b BoxedHandler<M>,
    pub(crate) next_middleware: &'a [Arc<dyn Middleware<M>>],
}

#[async_trait::async_trait]
pub trait Middleware<M: Metadata>: Send + Sync + 'static {
    async fn handle(
        &self,
        req: RequestObject,
        metadata: M,
        next: Next<'_, '_, M>,
    ) -> Result<BoxedSerialize, Error>;
}

impl<M: Metadata> Next<'_, '_, M> {
    pub(crate) async fn run(
        mut self,
        req: RequestObject,
        metadata: M,
    ) -> Result<BoxedSerialize, Error> {
        if let Some((current, next)) = self.next_middleware.split_first() {
            self.next_middleware = next;
            current.handle(req, metadata, self).await
        } else {
            (&self.endpoint.0)(req, metadata).await
        }
    }
}