use crate::handler::BoxedHandler;
use crate::middleware::Middleware;
use crate::server::Metadata;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MapRouter<M: Metadata>(HashMap<String, Route<M>>);

pub struct Route<M: Metadata> {
    pub(crate) handler: BoxedHandler<M>,
    pub(crate) middlewares: Vec<Arc<dyn Middleware<M>>>,
}

impl<M: Metadata> Default for MapRouter<M> {
    fn default() -> Self {
        MapRouter(HashMap::default())
    }
}

pub trait Router<M: Metadata>: Default {
    fn get(&self, name: &str) -> Option<&Route<M>>;
    fn insert(&mut self, name: String, route: Route<M>) -> Option<Route<M>>;
}

impl<M: Metadata> Router<M> for MapRouter<M> {
    fn get(&self, name: &str) -> Option<&Route<M>> {
        self.0.get(name)
    }
    fn insert(&mut self, name: String, route: Route<M>) -> Option<Route<M>> {
        self.0.insert(name, route)
    }
}
