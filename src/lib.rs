//! Actix web middleware hook for request start and end. Subscribe to request start and end, request data, elapsed time, request id and response status.
//!
//! Setup:
//! ```
//! use std::rc::Rc;
//! use actix_web::{App, Error, HttpServer, web};
//! use actix_request_hook::observer::{Observer, RequestEndData, RequestStartData};
//! use actix_request_hook::RequestHook;
//! struct RequestLogger;
//!
//! impl Observer for RequestLogger {
//!     fn on_request_started(&self, data: RequestStartData) {
//!         println!("started {}", data.uri)
//!     }
//!
//!     fn on_request_ended(&self, data: RequestEndData) {
//!         println!("ended {} after {}ms", data.uri, data.elapsed.as_millis())
//!     }
//! }
//!
//! async fn index() -> Result<String, Error> {
//!     Ok("Hi there!".to_string())
//! }
//!
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!
//!     HttpServer::new(|| {
//!         // You can register many different observers.
//!         // One could be for logging, other for notifying other parts of the system etc.
//!         let request_hook = RequestHook::new()
//!            .exclude("/bye") // bye route shouldn't be logged
//!            .exclude_regex("^/\\d$") // excludes any numbered route like "/123456"
//!            .register(Rc::new(RequestLogger{}));
//!         App::new()
//!             .wrap(request_hook)
//!             .route("/bye", web::get().to(index))
//!             .route("/hey", web::get().to(index))
//!     }).bind("127.0.0.1:0").expect("Can not bind to 127.0.0.1:0")
//!       .run().await
//! }
//!
//! ```
//!
use std::cell::RefCell;
use std::collections::HashSet;
use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::rc::Rc;
use std::time::Instant;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::web::{Buf, BytesMut};
use actix_web::{Error, HttpMessage};
use futures_util::task::{Context, Poll};
use futures_util::StreamExt;
use regex::RegexSet;
use uuid::Uuid;

use crate::observer::{Observer, RequestEndData, RequestStartData};
use crate::util::get_payload;

pub mod observer;
mod tests;
mod util;

/// Middleware for subscribing to request start and end. Enables access to request data, id, status and request duration.
pub struct RequestHook(Rc<Inner>);

impl Default for RequestHook {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestHook {
    pub fn new() -> Self {
        Self(Rc::new(Inner {
            exclude: HashSet::new(),
            exclude_regex: RegexSet::empty(),
            observers: Vec::new(),
        }))
    }

    /// Ignore and do not log access info for specified path.
    pub fn exclude<T: Into<String>>(mut self, path: T) -> Self {
        Rc::get_mut(&mut self.0)
            .unwrap()
            .exclude
            .insert(path.into());
        self
    }

    /// Ignore and do not log access info for paths that match regex.
    pub fn exclude_regex<T: Into<String>>(mut self, path: T) -> Self {
        let inner = Rc::get_mut(&mut self.0).unwrap();
        let mut patterns = inner.exclude_regex.patterns().to_vec();
        patterns.push(path.into());
        let regex_set = RegexSet::new(patterns).unwrap();
        inner.exclude_regex = regex_set;
        self
    }

    /// Registers an [Observer].
    pub fn register<T: 'static + Observer>(mut self, observer: Rc<T>) -> Self {
        Rc::get_mut(&mut self.0).unwrap().observers.push(observer);
        self
    }
}

/// Contains configuration for [RequestHook]
///
/// # Properties
/// * `exclude` - excluded path is ignored.
/// * `exclude_regex` - same as `exclude`, just uses regex instead of exact match.
/// * `observers` - a list of observers for actix request.
#[derive(Clone)]
struct Inner {
    exclude: HashSet<String>,
    exclude_regex: RegexSet,
    observers: Vec<Rc<dyn Observer>>,
}

impl<S: 'static, B> Transform<S, ServiceRequest> for RequestHook
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Response = S::Response;
    type Error = Error;
    type Transform = RequestHookMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestHookMiddleware {
            service: Rc::new(RefCell::new(service)),
            inner: self.0.clone(),
        }))
    }
}

pub struct RequestHookMiddleware<S> {
    inner: Rc<Inner>,
    service: Rc<RefCell<S>>,
}

impl<S: 'static, B> Service<ServiceRequest> for RequestHookMiddleware<S>
where
    B: MessageBody,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        let excluded = self.inner.exclude.contains(req.path())
            || self.inner.exclude_regex.is_match(req.path());
        if excluded {
            return Box::pin(svc.call(req));
        }

        let observers = self.inner.observers.clone();

        let start = Instant::now();
        let request_id = Uuid::new_v4();
        let uri = req.uri().to_string();
        let method = req.method().to_string();

        let future_response = async move {
            let mut payload = req.take_payload();
            let mut body = BytesMut::new();
            while let Some(chunk) = payload.next().await {
                body.extend_from_slice(chunk.unwrap().chunk())
            }

            let handler_body = body.clone();
            let repacked_payload = get_payload(body.freeze());

            for observer in &observers {
                observer.on_request_started(RequestStartData {
                    req: &req,
                    request_id,
                    uri: uri.to_string(),
                    method: method.to_string(),
                    body: handler_body.clone(),
                })
            }

            req.set_payload(repacked_payload);
            let res: Result<ServiceResponse<B>, Error> = svc.call(req).await;

            let elapsed = start.elapsed();

            let (response, status) = match res {
                Err(err) => {
                    let status = err.error_response().status();
                    (Err(err), status)
                }
                Ok(service_response) => {
                    let status = service_response.status();

                    (Ok(service_response), status)
                }
            };
            for observer in &observers {
                observer.on_request_ended(RequestEndData {
                    request_id,
                    elapsed,
                    uri: uri.to_string(),
                    method: method.to_string(),
                    status,
                })
            }

            response
        };

        Box::pin(future_response)
    }
}
