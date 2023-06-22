mod route;

use axum::{
    routing::{get, on, post, MethodFilter},
    Router,
};
use hyper::{client::HttpConnector, Body};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

pub type HttpClient = hyper::client::Client<HttpConnector, Body>;

pub fn create_router() -> Router {
    let client: HttpClient = hyper::Client::builder().build(HttpConnector::new());

    Router::new()
        .route("/file/bot:bot_id/*file_path", get(route::handle_serve_file))
        .route("/bot:bot_id/GetFile", post(route::handle_post_file))
        .route("/*proxy", on(MethodFilter::all(), route::handle_proxy))
        .with_state(client)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::default().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
}
