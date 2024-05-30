use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use opentelemetry::{
    global::{self, ObjectSafeSpan},
    trace::{SpanKind, Status, Tracer},
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::TracerProvider;
use rand::Rng;
use std::{convert::Infallible, net::SocketAddr};

fn init_tracing() -> anyhow::Result<()> {
    let span_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317")
        .build_span_exporter()?;

    let provider = TracerProvider::builder()
        .with_config(opentelemetry_sdk::trace::config().with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue {
                key: opentelemetry_semantic_conventions::resource::SERVICE_NAME.into(),
                value: "demo_server".into(),
            },
        ])))
        .with_batch_exporter(span_exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    global::set_tracer_provider(provider);

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());
    let tracer = global::tracer("dice_server");

    let mut span = tracer
        .span_builder(format!("{} {}", req.method(), req.uri().path()))
        .with_kind(SpanKind::Server)
        .start(&tracer);

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/rolldice") => {
            let random_number = rand::thread_rng().gen_range(1..7);
            *response.body_mut() = Body::from(format!("{random_number}\n"));
            span.set_status(Status::Ok);
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            span.set_status(Status::error("Not Found"));
        }
    };

    Ok(response)
}
