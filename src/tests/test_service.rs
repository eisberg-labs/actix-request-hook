#[cfg(test)]
mod tests {
    use crate::{Observer, RequestEndData, RequestHook, RequestStartData};
    use actix_web::dev::Service;
    use actix_web::dev::Transform;
    use actix_web::test;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct MyObserver1 {
        sent_messages: RefCell<Vec<String>>,
    }

    impl Default for MyObserver1 {
        fn default() -> Self {
            Self {
                sent_messages: RefCell::new(vec![]),
            }
        }
    }

    impl Observer for MyObserver1 {
        fn on_request_started(&self, data: RequestStartData) {
            self.sent_messages
                .borrow_mut()
                .push(format!("started {}", data.request_id));
        }

        fn on_request_ended(&self, data: RequestEndData) {
            self.sent_messages
                .borrow_mut()
                .push(format!("ended {}", data.request_id));
        }
    }

    struct MyObserver2 {
        started: RefCell<bool>,
        ended: RefCell<bool>,
    }

    impl Default for MyObserver2 {
        fn default() -> Self {
            Self {
                started: RefCell::new(false),
                ended: RefCell::new(false),
            }
        }
    }

    impl Observer for MyObserver2 {
        fn on_request_started(&self, _data: RequestStartData) {
            *self.started.borrow_mut() = true;
        }

        // testing if request ended receives mutated property
        fn on_request_ended(&self, _data: RequestEndData) {
            let is_started = self.started.borrow();
            if *is_started {
                *self.ended.borrow_mut() = true;
            }
        }
    }

    #[actix_web::test]
    async fn test_excluded() {
        let service_req1 = test::TestRequest::with_uri("/12").to_srv_request();
        let service_req2 = test::TestRequest::with_uri("/mypath").to_srv_request();
        let observer = MyObserver1::default();
        let rc = Rc::new(observer);
        let service = RequestHook::new().exclude("/mypath").register(rc.clone());

        let srv = service.new_transform(test::ok_service()).await.unwrap();

        let result1 = srv.call(service_req1).await;
        assert!(result1.is_ok());

        let result2 = srv.call(service_req2).await;
        assert!(result2.is_ok());

        let sent_messages = rc.sent_messages.borrow();
        assert_eq!((*sent_messages).len(), 2)
    }

    #[actix_web::test]
    async fn test_no_observers() {
        let service_req = test::TestRequest::with_uri("/").to_srv_request();

        let service = RequestHook::new();

        let srv = service.new_transform(test::ok_service()).await.unwrap();

        let result = srv.call(service_req).await;

        assert!(result.is_ok());
    }

    #[actix_web::test]
    async fn test_all_observers_are_called() {
        let service_req = test::TestRequest::with_uri("/12").to_srv_request();
        // also testing rust doesn't complain about different observer impls
        let observer1 = Rc::new(MyObserver1::default());
        let observer2 = Rc::new(MyObserver2::default());
        let service = RequestHook::new()
            .register(observer1.clone())
            .register(observer2.clone());

        let srv = service.new_transform(test::ok_service()).await.unwrap();

        let result = srv.call(service_req).await;

        assert!(result.is_ok());
        assert_eq!(observer1.sent_messages.borrow().len(), 2);
        assert!(*observer2.started.borrow());
        assert!(*observer2.ended.borrow());
    }
}
