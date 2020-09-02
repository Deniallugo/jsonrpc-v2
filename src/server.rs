use crate::documentation::{DocNotification, DocRoute, SpecHandler};
use crate::error::Error;
use crate::handler::{Factory, Handler};
use crate::middleware::{Middleware, Next};
use crate::request::{BytesRequestObject, FromRequest, RequestKind, RequestObject};
use crate::response::{ManyResponseObjects, ResponseObject, ResponseObjects, SingleResponseObject};
use crate::router::{MapRouter, Route, Router};
use crate::{Id, OneOrManyRawValues};
use paperclip::v2::schema::Apiv2Schema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use futures::{
    future::{self, Future, FutureExt},
    stream::StreamExt,
};

#[cfg(not(feature = "bytes-v04"))]
use bytes::Bytes;

#[cfg(feature = "bytes-v04")]
use bytes_v04::Bytes;

/// Server/request handler
pub struct Server<M>
where
    M: Metadata,
{
    router: MapRouter<M>,
    #[allow(dead_code)]
    middlewares: Vec<Arc<dyn Middleware<M>>>,
}

/// Builder used to add methods to a server
///
/// Created with `Server::new` or `Server::with_state`
pub struct ServerBuilder<M>
where
    M: Metadata,
{
    router: MapRouter<M>,
    routes: Vec<DocRoute>,
    notifications: Vec<DocNotification>,
    middlewares: Vec<Arc<dyn Middleware<M>>>,
}

impl<M: Metadata> Server<M> {
    pub fn new(middlewares: Vec<Arc<dyn Middleware<M>>>) -> ServerBuilder<M> {
        Self::with_router(MapRouter::default(), middlewares)
    }
    pub fn with_router(
        router: MapRouter<M>,
        middlewares: Vec<Arc<dyn Middleware<M>>>,
    ) -> ServerBuilder<M> {
        ServerBuilder { router, routes: Vec::default(), notifications: Vec::default(), middlewares }
    }
}

impl<M: Metadata> ServerBuilder<M> {
    /// Add a method handler to the server
    ///
    /// The method is an async function that takes up to 5 [`FromRequest`](trait.FromRequest.html) items
    /// and returns a value that can be resolved to a `TryFuture`, where `TryFuture::Ok` is a serializable object, e.g.:
    ///

    pub fn with_method<'de, N, S, E, T, F>(self, name: N, handler: F) -> Self
    where
        N: Into<String> + Clone,
        F: Factory<S, E, T, M> + Send + Sync + 'static,
        S: Serialize + Deserialize<'de> + Send + Apiv2Schema + 'static,
        Error: From<E>,
        E: 'static,
        T: FromRequest + Deserialize<'de> + Send + Apiv2Schema + 'static,
    {
        self.with_method_middleware(name, handler, vec![])
    }

    pub fn with_method_middleware<'de, N, S, E, T, F>(
        mut self,
        name: N,
        handler: F,
        mut middlewares: Vec<Arc<dyn Middleware<M>>>,
    ) -> Self
    where
        N: Into<String> + Clone,
        F: Factory<S, E, T, M> + Send + Sync + 'static,
        S: Serialize + Deserialize<'de> + Send + Apiv2Schema + 'static,
        Error: From<E>,
        E: 'static,
        T: FromRequest + Deserialize<'de> + Send + Apiv2Schema + 'static,
    {
        self.routes.push(DocRoute {
            name: name.clone().into(),
            request: T::raw_schema(),
            response: S::raw_schema(),
        });
        self.middlewares.iter().for_each(|el| middlewares.push(el.clone()));
        let route = Route { handler: Handler::new(handler).into(), middlewares };
        self.router.insert(name.into(), route);
        self
    }

    /// Convert the server builder into the finished struct, wrapped in an `Arc`
    pub fn finish(self) -> Arc<Server<M>> {
        let builder = self.add_documentation_route();
        Arc::new(Server { router: builder.router, middlewares: builder.middlewares })
    }

    fn add_documentation_route(mut self) -> Self {
        let spec_handler =
            SpecHandler { routes: self.routes.clone(), notifications: self.notifications.clone() };
        let route = Route { handler: Handler::new(spec_handler).into(), middlewares: vec![] };
        self.router.insert("__docs__".into(), route);
        self
    }

    /// Convert the server builder into the finished struct
    pub fn finish_unwrapped(self) -> Server<M> {
        let ServerBuilder { router, routes: _, notifications: _, middlewares } = self;
        Server { router, middlewares }
    }

    pub fn with_notification<'de, N: Deserialize<'de> + Send + Apiv2Schema + 'static>(
        mut self,
        name: String,
    ) -> Self {
        self.notifications.push(DocNotification { notification: N::raw_schema(), name });
        self
    }
}

impl<M> Server<M>
where
    M: Metadata,
{
    #[cfg(feature = "actix-web-v1-integration")]
    fn handle_bytes_compat(
        &self,
        bytes: Bytes,
        metadata: M,
    ) -> impl futures_v01::Future<Item = ResponseObjects, Error = ()> {
        self.handle_bytes(bytes, metadata).unit_error().boxed().compat()
    }

    /// Handle requests, and return appropriate responses
    pub fn handle<I: Into<RequestKind>>(
        &self,
        req: I,
        metadata: M,
    ) -> impl Future<Output = ResponseObjects> + '_ {
        match req.into() {
            RequestKind::Bytes(bytes) => future::Either::Left(self.handle_bytes(bytes, metadata)),
            RequestKind::RequestObject(req) => future::Either::Right(future::Either::Left(
                self.handle_request_object(req, metadata).map(From::from),
            )),
            RequestKind::ManyRequestObjects(reqs) => future::Either::Right(future::Either::Right(
                self.handle_many_request_objects(reqs, metadata).map(From::from),
            )),
        }
    }

    fn handle_request_object(
        &self,
        req: RequestObject,
        metadata: M,
    ) -> impl Future<Output = SingleResponseObject> + '_ {
        let opt_id = match req.id {
            Some(Some(ref id)) => Some(id.clone()),
            Some(None) => Some(Id::Null),
            None => None,
        };

        if let Some(route) = self.router.get(req.method.as_ref()) {
            let next = Next { endpoint: &route.handler, next_middleware: &route.middlewares };

            let out = next.run(req, metadata).then(|res| match res {
                Ok(val) => future::ready(SingleResponseObject::result(val, opt_id)),
                Err(e) => future::ready(SingleResponseObject::error(e, opt_id)),
            });
            future::Either::Left(out)
        } else {
            future::Either::Right(future::ready(SingleResponseObject::error(
                Error::METHOD_NOT_FOUND,
                opt_id,
            )))
        }
    }

    fn handle_many_request_objects<I: IntoIterator<Item = RequestObject>>(
        &self,
        reqs: I,
        metadata: M,
    ) -> impl Future<Output = ManyResponseObjects> + '_ {
        reqs.into_iter()
            .map(|r| self.handle_request_object(r, metadata.clone()))
            .collect::<futures::stream::FuturesUnordered<_>>()
            .filter_map(|res| async move {
                match res {
                    SingleResponseObject::One(r) => Some(r),
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .map(|vec| {
                if vec.is_empty() {
                    ManyResponseObjects::Empty
                } else {
                    ManyResponseObjects::Many(vec)
                }
            })
    }

    fn handle_bytes(
        &self,
        bytes: Bytes,
        metadata: M,
    ) -> impl Future<Output = ResponseObjects> + '_ {
        if let Ok(raw_values) = OneOrManyRawValues::try_from_slice(bytes.as_ref()) {
            match raw_values {
                OneOrManyRawValues::Many(raw_reqs) => {
                    if raw_reqs.is_empty() {
                        return future::Either::Left(future::ready(ResponseObjects::One(
                            ResponseObject::error(Error::INVALID_REQUEST, Id::Null),
                        )));
                    }

                    let (okays, errs) = raw_reqs
                        .into_iter()
                        .map(|x| {
                            serde_json::from_str::<BytesRequestObject>(x.get())
                                .map(RequestObject::from)
                        })
                        .partition::<Vec<_>, _>(|x| x.is_ok());

                    let errs = errs
                        .into_iter()
                        .map(|_| ResponseObject::error(Error::INVALID_REQUEST, Id::Null))
                        .collect::<Vec<_>>();

                    future::Either::Right(future::Either::Left(
                        self.handle_many_request_objects(okays.into_iter().flatten(), metadata)
                            .map(|res| match res {
                                ManyResponseObjects::Many(mut many) => {
                                    many.extend(errs);
                                    ResponseObjects::Many(many)
                                }
                                ManyResponseObjects::Empty => {
                                    if errs.is_empty() {
                                        ResponseObjects::Empty
                                    } else {
                                        ResponseObjects::Many(errs)
                                    }
                                }
                            }),
                    ))
                }
                OneOrManyRawValues::One(raw_req) => {
                    match serde_json::from_str::<BytesRequestObject>(raw_req.get())
                        .map(RequestObject::from)
                    {
                        Ok(rn) => future::Either::Right(future::Either::Right(
                            self.handle_request_object(rn, metadata).map(|res| match res {
                                SingleResponseObject::One(r) => ResponseObjects::One(r),
                                _ => ResponseObjects::Empty,
                            }),
                        )),
                        Err(_) => future::Either::Left(future::ready(ResponseObjects::One(
                            ResponseObject::error(Error::INVALID_REQUEST, Id::Null),
                        ))),
                    }
                }
            }
        } else {
            future::Either::Left(future::ready(ResponseObjects::One(ResponseObject::error(
                Error::PARSE_ERROR,
                Id::Null,
            ))))
        }
    }
}

pub trait Metadata: Clone + Send + 'static {}

impl Metadata for () {}
