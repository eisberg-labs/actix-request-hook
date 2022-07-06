//! [`Observer`] trait and function implementations.
use std::time::Duration;

use actix_web::dev::ServiceRequest;
use actix_web::http::StatusCode;
use actix_web::web::BytesMut;
use uuid::Uuid;

/// Request start arguments container
///
/// # Properties
///
/// * `req` - borrowed ServiceRequest.
/// * `request_id` - unique identifier of a request, identifies connection between request start and end.
/// * `uri` - uri of request.
/// * `method` - http method of request.
pub struct RequestStartData<'l> {
    pub req: &'l ServiceRequest,
    pub request_id: Uuid,
    pub uri: String,
    pub method: String,
    pub body: BytesMut,
}

/// Request end arguments container
///
/// # Properties
///
/// * `request_id` - unique identifier of a request, identifies connection between request start and end.
/// * `elapsed` - elapsed time between request start and end hook.
/// * `uri` - uri of request.
/// * `method` - http method of request.
/// * `status` - http status code of response.
pub struct RequestEndData {
    pub request_id: Uuid,
    pub elapsed: Duration,
    pub uri: String,
    pub method: String,
    pub status: StatusCode,
}

/// An Observer is notified before a request is passed for processing, and after processing into a response.
/// Use case could be logging before and after request:
/// ```
/// use actix_request_hook::observer::{Observer, RequestEndData, RequestStartData};
/// struct RequestLogger;
///
/// impl Observer for RequestLogger {
///     fn on_request_started(&self, data: RequestStartData) {
///         println!("[start - {}] {} {}", data.request_id.to_string(), data.method, data.uri);
///     }
///
///     fn on_request_ended(&self, data: RequestEndData) {
///         let time_elapsed_millis = data.elapsed.as_millis();
///         println!("[end - {}] {} {} ended with {} after {}ms", data.request_id.to_string(), data.method, data.uri, data.status, time_elapsed_millis);
///     }
/// }
///
/// ```
pub trait Observer {
    /// Fired before handler call. See [RequestStartData] for available arguments.
    fn on_request_started(&self, data: RequestStartData);

    /// Fired after handler call. See [RequestEndData] for available arguments.
    fn on_request_ended(&self, data: RequestEndData);
}
