#[cfg(test)]
mod tests {
    use crate::{Observer, RequestEndData, RequestStartData};
    use actix_http::HttpMessage;
    use actix_web::test;
    use actix_web::web::{Buf, BytesMut};
    use futures_util::StreamExt;
    use serde::Serialize;
    use std::cell::RefCell;
    use uuid::Uuid;

    #[actix_web::test]
    async fn test_receives_start_end_request() {
        struct MyObserver {
            sent_messages: RefCell<Vec<String>>,
        }
        #[derive(Serialize)]
        struct TestJson {
            size: usize,
        }

        impl Observer for MyObserver {
            fn on_request_started(&self, data: RequestStartData) {
                let body_value = String::from_utf8_lossy(&data.body);
                self.sent_messages.borrow_mut().push(format!(
                    "started {} - json `{}`",
                    data.request_id, body_value
                ))
            }

            fn on_request_ended(&self, data: RequestEndData) {
                self.sent_messages
                    .borrow_mut()
                    .push(format!("ended {}", data.request_id))
            }
        }

        let request_id = Uuid::new_v4();
        let my_observer = MyObserver {
            sent_messages: RefCell::new(vec![]),
        };
        let mut service_req = test::TestRequest::post()
            .set_json(TestJson { size: 1122 })
            .insert_header(("Content-type", "application/json"))
            .to_srv_request();

        // this is for test only, beware that once body is consumed it will be unavailable later. repack it.
        let mut payload = service_req.take_payload();
        let mut body = BytesMut::new();
        while let Some(chunk) = payload.next().await {
            body.extend_from_slice(chunk.unwrap().chunk())
        }

        my_observer.on_request_started(RequestStartData {
            req: &service_req,
            request_id,
            uri: "".to_string(),
            method: "".to_string(),
            body,
        });
        my_observer.on_request_ended(RequestEndData {
            request_id,
            elapsed: Default::default(),
            uri: "".to_string(),
            method: "".to_string(),
            status: Default::default(),
        });

        assert_eq!(
            my_observer.sent_messages.into_inner(),
            vec![
                format!("started {} - json `{}`", request_id, "{\"size\":1122}"),
                format!("ended {}", request_id)
            ]
        )
    }
}
