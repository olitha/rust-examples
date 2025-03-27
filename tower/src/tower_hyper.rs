//! Shows how to use [hyper] with [tower] layers

use std::{
    error::Error,
    task::{Context, Poll},
};

use futures_util::future::BoxFuture;
use http::{HeaderName, HeaderValue, Uri};
use http_body_util::Empty;
use hyper::{
    body::{Body, Bytes},
    client::conn::{http1, http2},
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpStream;
use tower::{Service, ServiceBuilder};
use tower_http::set_header::SetRequestHeaderLayer;

use wiremock::MockServer;

/// Holds a [hyper::client::conn::http1::SendRequest] or [hyper::client::conn::http2::SendRequest]
#[derive(Clone)]
struct HyperService<SendRequestType> {
    send_request: SendRequestType,
}

impl<SendRequestType> HyperService<SendRequestType> {
    fn new(send_request: SendRequestType) -> Self {
        HyperService { send_request }
    }
}

/// Implementation of [tower::Service] for [hyper::client::conn::http1::SendRequest]
/// The returned future is boxed as the [hyper::client::conn::http2::SendRequest::send_request]
/// returns an opaque type
impl<B> Service<hyper::Request<B>> for HyperService<http1::SendRequest<B>>
where
    B: Body + Send + Unpin + 'static,
{
    type Response = http::Response<hyper::body::Incoming>;
    type Error = hyper::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: hyper::Request<B>) -> Self::Future {
        let future = self.send_request.send_request(req);
        Box::pin(future)
    }
}

/// Implementation of [tower::Service] for [hyper::client::conn::http2::SendRequest]
impl<B> Service<hyper::Request<B>> for HyperService<http2::SendRequest<B>>
where
    B: Body + Send + 'static,
{
    type Response = hyper::Response<hyper::body::Incoming>;
    type Error = hyper::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: hyper::Request<B>) -> Self::Future {
        let future = self.send_request.send_request(req);
        Box::pin(future)
    }
}

/// Example of usage with http1
#[tokio::test]
async fn tower_hyper_http1() -> Result<(), Box<dyn Error>> {
    let mock_server = crate::start_echo_server().await;

    let socket = TcpStream::connect(mock_server.address()).await?;
    let adapt_stream = TokioIo::new(socket);

    let (send_request, connection) = http1::handshake::<_, Empty<Bytes>>(adapt_stream).await?;

    tokio::spawn(async move {
        if let Err(err) = connection.await {
            eprintln!("Connection error: {:?}", err);
        }
    });

    let service = HyperService::new(send_request);
    tower_hyper(service, &mock_server).await?;

    Ok(())
}

/// Example of usage with http2
#[tokio::test]
async fn tower_hyper_http2() -> Result<(), Box<dyn Error>> {
    let mock_server = crate::start_echo_server().await;

    let socket = TcpStream::connect(mock_server.address()).await?;
    let adapt_stream = TokioIo::new(socket);

    let (send_request, connection) =
        http2::handshake::<_, _, Empty<Bytes>>(TokioExecutor::new(), adapt_stream).await?;

    tokio::spawn(async move {
        if let Err(err) = connection.await {
            eprintln!("Connection error: {:?}", err);
        }
    });

    let service = HyperService { send_request };
    tower_hyper(service, &mock_server).await?;

    Ok(())
}

async fn tower_hyper<S, RB>(service: S, mock_server: &MockServer) -> Result<(), Box<dyn Error>>
where
    S: Service<hyper::Request<Empty<Bytes>>, Response = hyper::Response<RB>, Error = hyper::Error>,
{
    let mut client_service = ServiceBuilder::new()
        .layer(SetRequestHeaderLayer::appending(
            HeaderName::from_static("x-test"),
            HeaderValue::from_static("true"),
        ))
        .service(service);

    let url = hyper::Uri::builder()
        .scheme("http")
        .authority(mock_server.address().to_string())
        .path_and_query("/headers")
        .build()?;

    let request = hyper::Request::builder()
        .uri(url)
        .header("test", HeaderValue::from_static("hello"))
        .body(Empty::<Bytes>::new())?;

    let response = client_service.call(request).await?;

    assert!(response
        .headers()
        .into_iter()
        .any(|(name, value)| name == "x-test" && value == "true"));

    println!("response code {}", response.status());
    println!("response headers {:?}", response.headers());

    Ok(())
}
