use futures::{future, Future, Stream};

use hyper::{
    service::{service_fn, Service},
    Body, Method, Request, Response, StatusCode,
};

use tracing_subscriber::{filter::Filter, reload::Handle};

pub struct TracingEndpointService<S>
where
    S: tracing::Subscriber,
{
    handle: Handle<Filter, S>,
}

impl<S> From<Handle<Filter, S>> for TracingEndpointService<S>
where
    S: tracing::Subscriber,
{
    fn from(handle: Handle<Filter, S>) -> Self {
        TracingEndpointService { handle }
    }
}

impl<S> Clone for TracingEndpointService<S>
where
    S: tracing::Subscriber,
{
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl<S> TracingEndpointService<S>
where
    S: tracing::Subscriber,
{
    fn set_from(&self, chunk: hyper::Chunk) -> Result<(), String> {
        use std::str;
        let bytes = chunk.into_bytes();
        let filter = str::from_utf8(&bytes.as_ref()).map_err(|e| format!("{}", e))?;
        trace!(filter);
        let parsed_filter = filter.parse::<Filter>().map_err(|e| format!("{}", e))?;
        self.handle
            .reload(parsed_filter)
            .map_err(|e| format!("{}", e))
    }
}

impl<S> Service for TracingEndpointService<S>
where
    S: tracing::Subscriber,
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Response<Body>, Error = Self::Error> + Send + 'static>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        match (req.method(), req.uri().path()) {
            (&Method::PUT, "/filter") => {
                let handle = self.clone();
                trace!("reloading tracing filter");
                let f = req
                    .into_body()
                    .concat2()
                    .map(move |chunk| match handle.set_from(chunk) {
                        Err(e) => Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(format!("{}", e).into())
                            .expect("building response cannot fail"),
                        Ok(()) => Response::builder()
                            .status(StatusCode::OK)
                            .body(Body::empty())
                            .expect("building response cannot fail"),
                    });
                Box::new(f)
            }
            _ => Box::new(future::ok(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .expect("building response cannot fail"),
            )),
        }
    }
}

pub fn run<S>(handle: Handle<Filter, S>, el: &mut tokio_core::reactor::Core) -> Result<(), String>
where
    S: tracing::Subscriber,
{
    let tracing_addr = ([127, 0, 0, 1], 3000).into();
    let tracing_server = hyper::Server::bind(&tracing_addr)
        .serve(move || {
            let handle = handle.clone();
            service_fn(move |req| {
                TracingEndpointService::from(handle.clone()).call(req)
            })
        });

    el.run(tracing_server).map_err(|_| "some error".into())
}
