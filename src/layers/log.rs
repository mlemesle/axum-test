use axum::http::Request;
use chrono::Utc;
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
    T: Service<Request<ReqBody>>,
{
    type Response = T::Response;

    type Error = T::Error;

    type Future = T::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        println!(
            "[{}] {} -> {}",
            Utc::now().to_rfc3339(),
            req.method(),
            req.uri()
        );
        self.inner.call(req)
    }
}

#[derive(Clone)]
pub struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = Log<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Log::new(inner)
    }
}
