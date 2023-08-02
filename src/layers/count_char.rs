use std::{future::Future, pin::Pin};

use axum::{
    body::{BoxBody, Bytes, HttpBody},
    http::{Request, Response},
};
use hyper::Body;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct CountChar<T> {
    inner: T,
    c: char,
}

impl<T> CountChar<T> {
    fn new(inner: T, c: char) -> Self {
        Self { inner, c }
    }
}

impl<T, ReqBody> Service<Request<ReqBody>> for CountChar<T>
where
    T: Service<Request<BoxBody>, Response = Response<BoxBody>> + Send + Clone + 'static,
    <T as Service<Request<BoxBody>>>::Future: Send + 'static,
    <T as Service<Request<BoxBody>>>::Error: Send,
    ReqBody: HttpBody + Send + 'static,
    <ReqBody as HttpBody>::Data: Send,
    <ReqBody as HttpBody>::Error: std::fmt::Debug,
{
    type Response = T::Response;

    type Error = T::Error;

    type Future =
        Pin<Box<(dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static)>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();
        let c = Some(self.c);

        Box::pin(async move {
            let (headers, body) = req.into_parts();
            let bytes = hyper::body::to_bytes(body).await.unwrap();
            println!(
                "Request payload has {} {}",
                count_chars(&bytes, c),
                c.as_ref().unwrap()
            );
            let res = inner
                .call(Request::from_parts(
                    headers,
                    Body::from(bytes).map_err(axum::Error::new).boxed_unsync(),
                ))
                .await;
            match res {
                Ok(response) => {
                    let (headers, body) = response.into_parts();
                    let bytes = hyper::body::to_bytes(body).await.unwrap();
                    println!(
                        "Response payload has {} {}",
                        count_chars(&bytes, c),
                        c.as_ref().unwrap()
                    );
                    Ok(Response::from_parts(
                        headers,
                        Body::from(bytes).map_err(axum::Error::new).boxed_unsync(),
                    ))
                }
                Err(err) => Err(err),
            }
        })
    }
}

#[derive(Clone)]
pub struct CountCharLayer(char);

impl CountCharLayer {
    pub fn new(c: char) -> Self {
        Self(c)
    }
}

impl<S> Layer<S> for CountCharLayer
where
    S: Send + 'static,
{
    type Service = CountChar<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CountChar::new(inner, self.0)
    }
}

fn count_chars(bytes: &Bytes, c: Option<char>) -> usize {
    bytes
        .iter()
        .filter(|b| char::from_u32(**b as u32) == c)
        .count()
}
