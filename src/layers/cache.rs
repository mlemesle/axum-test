use std::{
    collections::HashMap, fmt::Display, future::Future, pin::Pin, sync::OnceLock, task::Poll,
};

use axum::{
    body::{BoxBody, Bytes, HttpBody},
    http::{Request, Response},
    response::IntoResponse,
};
use tower::{Layer, Service};

struct Cache {
    inner: HashMap<String, Bytes>,
}

static mut CACHE: OnceLock<Cache> = OnceLock::new();

fn get_cache() -> &'static Cache {
    unsafe { CACHE.get_or_init(Cache::new) }
}

fn get_cache_mut() -> &'static mut Cache {
    unsafe { CACHE.get_mut().unwrap() }
}

impl Cache {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    async fn get(&self, key: String) -> Option<Bytes> {
        self.inner.get(&key).cloned()
    }

    async fn set(&mut self, key: String, body: BoxBody) -> Bytes {
        let bytes = hyper::body::to_bytes(body).await.unwrap();
        self.inner.insert(key, bytes.clone());
        bytes
    }
}

type CacheGetFut = Pin<Box<(dyn Future<Output = Option<Bytes>> + Send + 'static)>>;
type CacheSetFut = Pin<Box<(dyn Future<Output = Bytes> + Send + 'static)>>;

enum State<InnerCallFut> {
    Init,
    CacheGet(CacheGetFut),
    CacheSet(CacheSetFut),
    InnerCall(Pin<Box<InnerCallFut>>),
}

impl<T> Display for State<T>
where
    T: Future,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Init => f.write_str("[State::Init]"),
            State::CacheGet(_) => f.write_str("[State::CacheGet]"),
            State::CacheSet(_) => f.write_str("[State::CacheSet]"),
            State::InnerCall(_) => f.write_str("[State::InnerCall]"),
        }
    }
}

#[pin_project::pin_project]
pub struct CacheFuture<S, ReqBody>
where
    S: Service<Request<ReqBody>, Response = Response<BoxBody>>,
    S::Future: Future<Output = Result<S::Response, S::Error>>,
{
    state: State<S::Future>,
    key: String,
    service: S,
    req: Option<Request<ReqBody>>,
}

impl<S, ReqBody> CacheFuture<S, ReqBody>
where
    S: Service<Request<ReqBody>, Response = Response<BoxBody>>,
{
    fn new(key: String, service: S, req: Request<ReqBody>) -> Self {
        Self {
            state: State::Init,
            key,
            service,
            req: Some(req),
        }
    }
}

impl<S, ReqBody> Future for CacheFuture<S, ReqBody>
where
    S: Service<Request<ReqBody>, Response = Response<BoxBody>> + Send,
    S::Future: Send + 'static,
    ReqBody: HttpBody + Send + 'static,
    S::Future: Future<Output = Result<S::Response, S::Error>>,
{
    type Output = Result<S::Response, S::Error>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let project = self.project();
        loop {
            print!("{} ", project.state);
            match project.state {
                State::Init => {
                    println!("Init step");
                    *project.state =
                        State::CacheGet(Box::pin(get_cache().get(project.key.clone())));
                }
                State::CacheGet(get_fut) => match get_fut.as_mut().poll(cx) {
                    Poll::Ready(Some(bytes)) => {
                        println!("HIT, yielding response");
                        return Poll::Ready(Ok(bytes.into_response()));
                    }
                    Poll::Ready(None) => {
                        println!("MISS, proceed to Inner");
                        let req = project.req.take().unwrap();
                        *project.state = State::InnerCall(Box::pin(project.service.call(req)));
                    }
                    Poll::Pending => {
                        println!("PENDING");
                        return Poll::Pending;
                    }
                },
                State::CacheSet(set_fut) => match set_fut.as_mut().poll(cx) {
                    Poll::Ready(bytes) => {
                        println!("DONE");
                        return Poll::Ready(Ok(bytes.into_response()));
                    }
                    Poll::Pending => {
                        println!("PENDING");
                        return Poll::Pending;
                    }
                },
                State::InnerCall(inner_fut) => match inner_fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(response)) => {
                        println!("OK, set in cache");
                        let body = response.into_body();
                        *project.state = State::CacheSet(Box::pin(
                            get_cache_mut().set(project.key.clone(), body),
                        ));
                    }
                    Poll::Ready(Err(err)) => {
                        println!("ERROR");
                        return Poll::Ready(Err(err));
                    }
                    Poll::Pending => {
                        println!("PENDING");
                        return Poll::Pending;
                    }
                },
            };
        }
    }
}

#[derive(Clone)]
pub struct CacheService<S> {
    inner: S,
}

impl<S> CacheService<S> {
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, ReqBody> Service<Request<ReqBody>> for CacheService<S>
where
    S: Service<Request<ReqBody>, Response = Response<BoxBody>> + Send + Clone,
    S::Future: Send + 'static,
    ReqBody: HttpBody + Send + 'static,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = CacheFuture<S, ReqBody>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        CacheFuture::new(req.uri().to_string(), self.inner.clone(), req)
    }
}

#[derive(Clone)]
pub struct CacheLayer;

impl<S> Layer<S> for CacheLayer
where
    S: Send + 'static,
{
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService::new(inner)
    }
}
