//! Exampples for a [tower] service chain.
//! Manually creating the chain and creating it with layers.

#[tokio::test]
async fn simple_service_chain() {
    use crate::service_trait::AddService;
    use std::time::Duration;
    use tower::timeout::Timeout;
    use tower::Service;

    let add_service = AddService;
    let mut timeout_service = Timeout::new(add_service, Duration::from_secs(1));

    let result = timeout_service.call((12, 4)).await.unwrap();
    assert_eq!(result, 16);
}

#[tokio::test]
async fn layer_service_chain() {
    use crate::service_trait::AddService;
    use std::time::Duration;
    use tower::{timeout::TimeoutLayer, Service, ServiceBuilder};

    let mut service = ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(1)))
        .service(AddService);

    let result = service.call((12, 4)).await.unwrap();
    assert_eq!(result, 16);
}
