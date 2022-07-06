use actix_http::Payload;
use actix_web::web::Bytes;

/// Converts bytes to payload stream
pub fn get_payload(bytes: Bytes) -> Payload {
    let mut repack_payload = actix_http::h1::Payload::create(true);
    repack_payload.1.unread_data(bytes);
    repack_payload.1.into()
}
