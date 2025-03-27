pub mod service_chain;
pub mod service_trait;
pub mod tower_hyper;
pub mod tower_reqwest;

#[cfg(test)]
/// Helper for header echoing wiremock
async fn start_echo_server() -> wiremock::MockServer {
    use wiremock::{Respond, ResponseTemplate};

    struct EchoResponseHeader;
    impl Respond for EchoResponseHeader {
        fn respond(&self, request: &wiremock::Request) -> wiremock::ResponseTemplate {
            ResponseTemplate::new(200).append_headers(request.headers.iter())
        }
    }

    // Start a background HTTP server on a random local port
    let mock_server = wiremock::MockServer::start().await;

    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/headers"))
        .respond_with(EchoResponseHeader)
        .mount(&mock_server)
        .await;

    mock_server
}
