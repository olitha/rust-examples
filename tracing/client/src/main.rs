use opentelemetry::trace::TracerProvider as _;
use opentelemetry::global;
use opentelemetry_http::HeaderInjector;
use opentelemetry_sdk::resource::TelemetryResourceDetector;
use opentelemetry_sdk::trace::{self, BatchConfigBuilder, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::trace::{HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE};
use reqwest::header::HeaderMap;
use tracing::field::Empty;
use tracing::{info, info_span, span, Instrument, Span};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    // Create an OTEL tracerprovider
    // From the TracerProvider you get Tracer instances that are used in
    // your Code
    let tracer_provider = batch_with_processor();

    // Tracing Subscriber Layers can be configured with
    // filters like env_logger crate. This way every layer can have
    // a different filter
    let filter = tracing_subscriber::EnvFilter::new("DEBUG")
        .add_directive("hyper_util=info".parse().unwrap())
        .add_directive("opentelemetry=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=debug".parse().unwrap())
        .add_directive("reqwest=debug".parse().unwrap());

    // To bridge OTEL and tracing
    // a layer is created for the Tracing subscriber.
    // It uses the a tracer created from the OTEL tracerprovider
    let telemetry =
        tracing_opentelemetry::layer().with_tracer(tracer_provider.clone().tracer("app tracer"));

    // The registry is a Subscriber with support of layers
    // is is installed as the process global subscriber
    tracing_subscriber::registry()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    let root_span = span!(tracing::Level::DEBUG, "root span",).entered();
    global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new());
    info!("This event will be logged in the root span.");

    client_request().instrument(info_span!("client_request",
            { HTTP_REQUEST_METHOD } = Empty,
            { HTTP_RESPONSE_STATUS_CODE } = Empty
        )).await;
    root_span.exit();

    // make sure the Spanexporter has time to flush stored
    // spans
    tracer_provider.force_flush().unwrap();
    tracer_provider.shutdown().unwrap();
}

async fn client_request() {

        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        use tracing_opentelemetry::OpenTelemetrySpanExt;
        let ctx = Span::current().context();
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&ctx, &mut HeaderInjector(&mut headers));
        });

        let request = client
            .post("http://localhost:3000/event")
            .json(&serde_json::json!({ "counter": 1}))
            .headers(headers)
            .build()
            .unwrap();

        Span::current().record(HTTP_REQUEST_METHOD, "POST");
        let response = client
            .execute(request)
            .await
            .unwrap();
        Span::current().record(HTTP_RESPONSE_STATUS_CODE, response.status().as_u16());

}

fn batch_with_processor() -> SdkTracerProvider {
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Error spanexporter");
    let batch = opentelemetry_sdk::trace::BatchSpanProcessor::builder(span_exporter)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_queue_size(4096)
                .with_max_export_batch_size(512)
                .build(),
        )
        .build();

    trace::SdkTracerProvider::builder()
        .with_resource(resource())
        .with_span_processor(batch)
        .build()
}

fn resource() -> Resource {
    Resource::builder_empty()
        .with_detector(Box::new(TelemetryResourceDetector))
        .with_service_name("Small Client")
        .build()
}

