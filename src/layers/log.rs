use std::{future::Future, pin::Pin};

use axum::http::Request;
use chrono::Utc;
use colored::Colorize;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct Log<T> {
    inner: T,
}

impl<T> Log<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T, ReqBody> Service<Request<ReqBody>> for Log<T>
where
    T: Service<Request<ReqBody>> + Send + Clone + 'static,
    <T as Service<Request<ReqBody>>>::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = T::Response;

    type Error = T::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let req_method = req.method().to_string();
        let req_uri = req.uri().to_string();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let res = inner.call(req).await;
            let status = if res.is_ok() {
                "SUCCESS".green()
            } else {
                "ERROR".red()
            };
            println!(
                "[{}] {} {} -> {}",
                Utc::now().to_rfc3339().underline(),
                status,
                req_method.yellow(),
                req_uri.bright_blue(),
            );
            res
        })
    }
}

#[derive(Clone)]
pub struct LogLayer;

impl<S> Layer<S> for LogLayer
where
    S: Send + 'static,
{
    type Service = Log<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Log::new(inner)
    }
}
