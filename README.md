# Actix Request Hook [![Continuous Integration](https://github.com/eisberg-labs/actix-request-hook/actions/workflows/ci.yml/badge.svg)](https://github.com/eisberg-labs/actix-request-hook/actions/workflows/ci.yml) [![cargo-badge][]][cargo] [![license-badge][]][license]

Actix web middleware hook for requests. Enables subscribing to request start and end, request id, elapsed time between requests and more.

# Example
Code:

```rust
struct RequestLogger;

impl Observer for RequestLogger {
    fn on_request_started(&self, data: RequestStartData) {
        println!("started {}", data.uri)
    }
    fn on_request_ended(&self, data: RequestEndData) {
        println!("ended {} after {}ms", data.uri, data.elapsed.as_millis())
    }
}

struct UselessObserver {
    started: RefCell<bool>,
    ended: RefCell<bool>,
}

impl Default for UselessObserver {
    fn default() -> Self {
        Self {
            started: RefCell::new(false),
            ended: RefCell::new(false),
        }
    }
}

impl Observer for UselessObserver {
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

async fn index() -> Result<String, Error> {
    Ok("Hi there!".to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        // You can register many different observers.
        // One could be for logging, other for notifying other parts of the system etc.
        let request_hook = RequestHook::new()
            .exclude("/bye") // bye route shouldn't be logged
            .exclude_regex("^/\\d$") // excludes any numbered route like "/123456"
            .register(Rc::new(RequestLogger {}))
            .register(Rc::new(UselessObserver::default()));
        
        App::new()
            .wrap(request_hook)
            .route("/bye", web::get().to(index))
            .route("/hey", web::get().to(index))
    }).bind("127.0.0.1:0").expect("Can not bind to 127.0.0.1:0")
        .run().await
}
```
## Possible Use Cases
- logging requests when started and ended
- notifying sentry with all request data 
...
## What can you use
In request start there are:
- `request_id` - unique id of a request, same for request start and end.
- `req` - borrowed `ServiceRequest`.
- `uri` - uri of request.
- `method` - body of request.
- `body` - body of request. Useful when debugging client requests e.g. maybe use it in Sentry.

In request end there are:
- `request_id` - unique id of a request, same for request start and end.
- `elapsed` - elapsed time between request start and end hook.
- `uri` - uri of request.
- `method` - body of request.
- `status` - response status.

## Caveats
Including `RequestHook` middleware might affect performance of your actix webapp. Observers are executed in a blocking manner, and
there's also request body repacking on each request. 

## Contributing

This project welcomes all kinds of contributions. No contribution is too small!

If you want to contribute to this project but don't know how to begin or if you need help with something related to this project, 
feel free to send me an email <https://www.eisberg-labs.com/> (contact form at the bottom).

Some pointers on contribution are in [Contributing.md](./CONTRIBUTING.md)

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).


# License

Distributed under the terms of [MIT license](./LICENSE-MIT) and [Apache license](./LICENSE-APACHE).

[cargo-badge]: https://img.shields.io/crates/v/actix-request-hook.svg?style=flat-square
[cargo]: https://crates.io/crates/actix-request-hook
[license-badge]: https://img.shields.io/badge/license-MIT/Apache--2.0-lightgray.svg?style=flat-square
[license]: #license
