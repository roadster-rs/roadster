use crate::app_context::AppContext;
use crate::controller::middleware::Middleware;
use axum::Router;

#[derive(Default)]
pub struct BulkMiddleware {
    middleware: Vec<Box<dyn Middleware>>,
}

impl BulkMiddleware {
    #[allow(dead_code)]
    pub fn prepend_all(mut self, middleware: Vec<Box<dyn Middleware>>) -> Self {
        let mut tmp = self.middleware;
        self.middleware = middleware;
        self.middleware.append(&mut tmp);
        self
    }

    pub fn append_all(self, middleware: Vec<Box<dyn Middleware>>) -> Self {
        middleware
            .into_iter()
            .fold(self, |bulk, middleware| bulk.append(middleware))
    }

    #[allow(dead_code)]
    fn prepend(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middleware.insert(0, middleware);
        self
    }

    fn append(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middleware.push(middleware);
        self
    }
}

impl Middleware for BulkMiddleware {
    fn name(&self) -> String {
        "bulk".to_string()
    }

    fn install(&self, router: Router, context: &AppContext) -> Router {
        self.middleware
            .iter()
            .rev()
            .fold(router, |router, middleware| {
                middleware.install(router, context)
            })
    }
}
