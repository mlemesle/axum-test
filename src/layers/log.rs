use std::{future::Future, task::Poll};

use axum::{
    body::BoxBody,
    http::{Request, Response, StatusCode},
};

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

#[pin_project::pin_project]
pub struct LogFuture<F, E>
where
    F: Future<Output = Result<Response<BoxBody>, E>>,
{
    req_method: String,
    req_uri: String,
    #[pin]
    resp_fut: F,
}

impl<F, E> LogFuture<F, E>
where
    F: Future<Output = Result<Response<BoxBody>, E>>,
{
    fn new(req_method: String, req_uri: String, resp_fut: F) -> Self {
        Self {
            req_method,
            req_uri,
            resp_fut,
        }
    }
}

impl<F, E> Future for LogFuture<F, E>
where
    F: Future<Output = Result<Response<BoxBody>, E>>,
{
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let project = self.project();
        match project.resp_fut.poll(cx) {
            Poll::Ready(res) => {
                let (message, status) = res
                    .as_ref()
                    .map(|response| response.status())
                    .map(|status| {
                        (
                            if status.is_success() {
                                "SUCCESS".green()
                            } else if status.is_informational() {
                                "INFORMATION".blue()
                            } else if status.is_redirection() {
                                "REDIRECTION".bright_blue()
                            } else if status.is_client_error() {
                                "CLIENT ERROR".red()
                            } else if status.is_server_error() {
                                "SERVER ERROR".red()
                            } else {
                                "INTERNAL ERROR".red()
                            },
                            status,
                        )
                    })
                    .unwrap_or_else(|_| {
                        ("INTERNAL ERROR".red(), StatusCode::INTERNAL_SERVER_ERROR)
                    });
                println!(
                    "[{}] {} {} / {} -> {}",
                    Utc::now().to_rfc3339().underline(),
                    status,
                    message,
                    project.req_method.yellow(),
                    project.req_uri.bright_blue(),
                );
                Poll::Ready(res)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T, ReqBody, Fut> Service<Request<ReqBody>> for Log<T>
where
    T: Service<Request<ReqBody>, Response = Response<BoxBody>, Future = Fut>,
    Fut: Future<
        Output = Result<
            <T as Service<Request<ReqBody>>>::Response,
            <T as Service<Request<ReqBody>>>::Error,
        >,
    >,
{
    type Response = T::Response;

    type Error = T::Error;

    type Future = LogFuture<Fut, <T as Service<Request<ReqBody>>>::Error>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let req_method = req.method().to_string();
        let req_uri = req.uri().to_string();
        LogFuture::new(req_method, req_uri, self.inner.call(req))
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
