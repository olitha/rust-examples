use std::{error::Error, time::Duration};

use axum::{extract::Request, routing::post, Json, Router};

use opentelemetry::global;
use opentelemetry_http::HeaderExtractor;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    resource::TelemetryResourceDetector,
    trace::{BatchConfigBuilder, SdkTracerProvider},
    Resource,
};
use serde_json::Value;
use tower_http::trace::{OnRequest, TraceLayer};
use tracing::{info, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{
    fmt::{self},
    util::SubscriberInitExt as _,
    EnvFilter,
};

fn resource() -> Resource {
    Resource::builder_empty()
        .with_detector(Box::new(TelemetryResourceDetector))
        .with_service_name("API Gateway")
        .build()
}

fn batch_with_processor() -> SdkTracerProvider {
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Error spanexporter");
    let batch = opentelemetry_sdk::trace::BatchSpanProcessor::builder(span_exporter)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_queue_size(2048)
                .with_max_export_batch_size(512)
                .build(),
        )
        .build();

    SdkTracerProvider::builder()
        .with_resource(resource())
        .with_span_processor(batch)
        .build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + 'static>> {
    let tracer_provider = batch_with_processor();

    // tracer function comes from the impl of TracerProvider
    // from opentelemetry crate
    // to make this clear the use is put in a block
    let telemetry = {
        use opentelemetry::trace::TracerProvider as _;
        tracing_opentelemetry::layer().with_tracer(tracer_provider.tracer("otlp tracer"))
    };

    use tracing_subscriber::layer::SubscriberExt;
    tracing_subscriber::registry()
        // a filter layer
        .with(EnvFilter::new(
            "debug,tower_http=debug,axum::rejection=trace,hyper=INFO",
        ))
        .with(fmt::layer())
        .with(telemetry)
        .init();

    // we set a global propagator that makes it easier to use it later
    global::set_text_map_propagator(TraceContextPropagator::new());

    info!("Starting up");

    let app = Router::new()
        .route("/event", post(body))
        .layer(TraceLayer::new_for_http().on_request(OtelOnRequest));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[derive(Clone)]
struct OtelOnRequest;

impl<B> OnRequest<B> for OtelOnRequest {
    fn on_request(&mut self, request: &Request<B>, span: &Span) {
        let otel_context = global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderExtractor(request.headers()))
        });
        span.set_parent(otel_context);
    }
}

#[tracing::instrument]
async fn body(Json(event): Json<Value>) -> &'static str {
    info!("got request {event:?}");
    do_something().in_current_span().await
}

async fn do_something() -> &'static str {
    for i in 0..4 {
        info!(repetition = i, "processing");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    "processed"
}
