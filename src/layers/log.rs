use std::{
    future::Future,
    pin::{pin, Pin},
};

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

struct DumbFut<F> {
    f: F,
}

impl<T, Fut, F> DumbFut<F>
where
    Fut: Future<Output = T>,
{
    fn new(f: F) -> Self {
        Self { f }
    }
}

impl<T, Fut> Future for DumbFut<Fut>
where
    Fut: Future<Output = T>,
{
    type Output = Fut::Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let f = pin!(self.f);
        f.poll(cx)
    }
}

fn toto<S, Fut>(service: &Log<S>, req: Request<BoxBody>) -> impl Future<Output = Fut::Output> + '_
where
    Fut: Future<
        Output = Result<
            <S as Service<Request<BoxBody>>>::Response,
            <S as Service<Request<BoxBody>>>::Error,
        >,
    >,
    S: Service<Request<BoxBody>, Response = Response<BoxBody>, Future = Fut>
        + Send
        + Clone
        + 'static,
{
    let mut inner = service.inner.clone();
    async move {
        let req_method = req.method().to_string();
        let req_uri = req.uri().to_string();
        let res = inner.call(req).await;
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
            .unwrap_or_else(|_| ("INTERNAL ERROR".red(), StatusCode::INTERNAL_SERVER_ERROR));
        println!(
            "[{}] {} {} / {} -> {}",
            Utc::now().to_rfc3339().underline(),
            status,
            message,
            req_method.yellow(),
            req_uri.bright_blue(),
        );
        res
    }
}

impl<T, ReqBody, Fut> Service<Request<ReqBody>> for Log<T>
where
    T: Service<Request<ReqBody>, Response = Response<BoxBody>, Future = Fut>
        + Send
        + Clone
        + 'static,
    <T as Service<Request<ReqBody>>>::Future: Send + 'static,
    ReqBody: Send + 'static,
    Fut: Future<
            Output = Result<
                <T as Service<Request<ReqBody>>>::Response,
                <T as Service<Request<ReqBody>>>::Error,
            >,
        > + Send,
{
    type Response = T::Response;

    type Error = T::Error;

    type Future = DumbFut<Fut>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();

        DumbFut::new(toto(self.inner, req))
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
