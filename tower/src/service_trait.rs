//! Example for a simple leaf service in tower
use std::{future::Future, pin::Pin, task::Poll};
use tower::Service;

pub struct AddService;
impl Service<(i32, i32)> for AddService {
    type Response = i32;

    type Error = &'static str;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, (a, b): (i32, i32)) -> Self::Future {
        Box::pin(async move { Ok(a + b) })
    }
}

#[tokio::test]
async fn leaf_service() {
    let mut add_service = AddService;
    let result = add_service.call((2, 5)).await;
    assert_eq!(result, Ok(7));
}
