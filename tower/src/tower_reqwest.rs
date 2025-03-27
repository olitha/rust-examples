//! Shows how to use [tower] layers with
//! [reqwest]

#[tokio::test]
async fn adapt_to_reqwest() {
    use http::{HeaderName, HeaderValue};
    use tower::{Service, ServiceBuilder};
    use tower_reqwest::HttpClientLayer;
    let mock_server = crate::start_echo_server().await;

    let client = reqwest::Client::new();
    let mut client_service = ServiceBuilder::new()
        .layer(tower_http::set_header::SetRequestHeaderLayer::appending(
            HeaderName::from_static("x-test"),
            HeaderValue::from_static("true"),
        ))
        .layer(HttpClientLayer)
        .service(client.clone());

    let req = http::request::Builder::new()
        .method(http::Method::GET)
        .uri(format!("{}/headers", mock_server.uri()))
        .body(reqwest::Body::default())
        .unwrap();

    let response = client_service.call(req).await.unwrap();

    assert!(response
        .headers()
        .into_iter()
        .any(|(name, value)| name == "x-test" && value == "true"));

    println!("response code {}", response.status());
    println!("response headers {:?}", response.headers());
}
