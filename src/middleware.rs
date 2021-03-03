use crate::error::Error;
use crate::handler::BoxedHandler;
use crate::request::RequestObject;
use crate::server::Metadata;
use crate::BoxedSerialize;

use futures::Future;

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
    pub async fn run(mut self, req: RequestObject, metadata: M) -> Result<BoxedSerialize, Error> {
        if let Some((current, next)) = self.next_middleware.split_first() {
            self.next_middleware = next;
            current.handle(req, metadata, self).await
        } else {
            (&self.endpoint.0)(req, metadata).await
        }
    }
}

#[async_trait::async_trait]
impl<M, FN, I> Middleware<M> for FN
where
    M: Metadata,
    I: Future<Output = Result<(RequestObject, M), Error>> + Send + 'static,
    FN: Fn(RequestObject, M) -> I + Send + Sync + 'static,
{
    async fn handle(
        &self,
        req: RequestObject,
        metadata: M,
        next: Next<'_, '_, M>,
    ) -> Result<BoxedSerialize, Error> {
        let (req, metadata) = (self)(req, metadata).await?;
        next.run(req, metadata).await
    }
}

pub struct LoggerMiddleware;

#[async_trait::async_trait]
impl<M> Middleware<M> for LoggerMiddleware
where
    M: Metadata,
{
    async fn handle(
        &self,
        req: RequestObject,
        metadata: M,
        next: Next<'_, '_, M>,
    ) -> Result<BoxedSerialize, Error> {
        // TODO move it to another thread
        let method_name = req.method.clone();
        let req_id = req.id.clone();
        log::info!("request \n {}", &req);
        let res = next.run(req, metadata).await;
        let resp =
            serde_json::to_string_pretty(&res).expect("Response should be json serializable");
        log::info!("response \n method {} \n params {} \n id {}", &method_name, &resp, &req_id,);
        res
    }
}
