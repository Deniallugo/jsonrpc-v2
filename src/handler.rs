use crate::error::Error;
use crate::request::{FromRequest, RequestObject};
use crate::server::Metadata;
use crate::BoxedSerialize;
use futures::Future;
use serde::Serialize;
use std::marker::PhantomData;
use std::sync::Arc;

#[doc(hidden)]
#[async_trait::async_trait]
pub trait Factory<S, E, T, M> {
    async fn call(&self, param: T, metadata: M) -> Result<S, E>;
}

#[doc(hidden)]
pub(crate) struct Handler<F, S, E, T, M>
where
    F: Factory<S, E, T, M>,
{
    hnd: F,
    _t: PhantomData<fn() -> (S, E, T, M)>,
}

impl<F, S, E, T, M> Handler<F, S, E, T, M>
where
    F: Factory<S, E, T, M>,
{
    pub(crate) fn new(hnd: F) -> Self {
        Handler { hnd, _t: PhantomData }
    }
}

#[async_trait::async_trait]
impl<FN, I, S, E, T, M> Factory<S, E, T, M> for FN
where
    S: 'static,
    E: 'static,
    I: Future<Output = Result<S, E>> + Send + 'static,
    T: FromRequest + Send + 'static,
    FN: Fn(T, M) -> I + Sync,
    M: Metadata,
{
    async fn call(&self, param: T, meta: M) -> Result<S, E> {
        (self)(param, meta).await
    }
}

impl<F, S, E, T, M> From<Handler<F, S, E, T, M>> for BoxedHandler<M>
where
    F: Factory<S, E, T, M> + 'static + Send + Sync,
    S: Serialize + Send + 'static,
    Error: From<E>,
    E: 'static,
    M: Metadata,
    T: FromRequest + 'static + Send,
{
    fn from(t: Handler<F, S, E, T, M>) -> BoxedHandler<M> {
        let hnd = Arc::new(t.hnd);
        let inner = move |req: RequestObject, metadata: M| {
            let hnd = Arc::clone(&hnd);
            Box::pin(async move {
                let out = {
                    let param = T::from_request(&req).await?;

                    hnd.call(param, metadata).await?
                };
                Ok(Box::new(out) as BoxedSerialize)
            })
                as std::pin::Pin<Box<dyn Future<Output = Result<BoxedSerialize, Error>> + Send>>
        };

        BoxedHandler(Box::new(inner))
    }
}

type HandlerResult = std::pin::Pin<Box<dyn Future<Output = Result<BoxedSerialize, Error>> + Send>>;

pub struct BoxedHandler<M: Metadata>(
    pub(crate) Box<dyn Fn(RequestObject, M) -> HandlerResult + Send + Sync>,
);
