use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use opentelemetry::KeyValue;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_semantic_conventions::{
    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use rand::Rng;
use std::{convert::Infallible, net::SocketAddr};

use tracing_core::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Create a Resource that captures information about the entity for which telemetry is recorded.
fn resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "develop"),
        ],
        SCHEMA_URL,
    )
}

// Construct Tracer for OpenTelemetryLayer
fn init_tracer() -> anyhow::Result<opentelemetry_sdk::trace::Tracer> {
    let pipeline = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(resource()))
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(pipeline)
}

fn init_tracing_subscriber() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(init_tracer()?))
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing_subscriber()?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}

#[tracing::instrument]
async fn kek() {}

#[tracing::instrument]
async fn do_work(job: &str) {
    kek().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
}

#[tracing::instrument]
async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    do_work("uno").await;
    do_work("dos").await;

    let mut response = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/rolldice") => {
            let random_number = rand::thread_rng().gen_range(1..7);
            *response.body_mut() = Body::from(format!("{random_number}\n"));
            tracing::info!("rolled {random_number}");
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            tracing::error!(path = req.uri().path(), "unknown path")
        }
    };

    Ok(response)
}
