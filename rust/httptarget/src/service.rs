use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{
    body::{Bytes, Incoming},
    Method, Request, Response, StatusCode,
};

pub(crate) async fn handle(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match req.uri().path() {
        "/" => match req.method() {
            &Method::GET => Ok(root()),
            _ => Ok(error(StatusCode::METHOD_NOT_ALLOWED)),
        },
        "/help" => match req.method() {
            &Method::GET => Ok(help()),
            _ => Ok(error(StatusCode::METHOD_NOT_ALLOWED)),
        },
        "/hello" => match req.method() {
            &Method::GET => Ok(hello()),
            _ => Ok(error(StatusCode::METHOD_NOT_ALLOWED)),
        },
        _ => Ok(error(StatusCode::NOT_FOUND)),
    }
}

fn root() -> Response<BoxBody<Bytes, hyper::Error>> {
    full("Use /help to see what's possible")
}

fn help() -> Response<BoxBody<Bytes, hyper::Error>> {
    full(
        "/hello: Return a short message
/help: Return this message",
    )
}

fn hello() -> Response<BoxBody<Bytes, hyper::Error>> {
    full("Hello, World!")
}

fn error(code: StatusCode) -> Response<BoxBody<Bytes, hyper::Error>> {
    let mut r = empty();
    *r.status_mut() = code;
    r
}

fn full<T: Into<Bytes>>(chunk: T) -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::new(
        Full::new(chunk.into())
            .map_err(|never| match never {})
            .boxed(),
    )
}

fn empty() -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::new(
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed(),
    )
}
