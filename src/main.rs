#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::single_match_else)]
#![allow(clippy::unused_async)]

mod config;
mod handlers;

use std::{env, net::TcpListener, process::exit};

use log::{debug, error, info, trace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::CONFIG;

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", CONFIG.log_level.as_str());
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    trace!("Config: {:?}", *CONFIG);

    debug!("Starting application");

    let listener = {
        let listen_address = (CONFIG.host.as_str(), CONFIG.port);

        TcpListener::bind(listen_address).unwrap_or_else(|e| {
            error!("Failed to bind to address {:?}: {}", &listen_address, e);
            exit(1);
        })
    };
    info!("Listening on http://{:?}", listener.local_addr().unwrap());

    let router = handlers::create_router();

    axum::Server::from_tcp(listener)
        .expect("failed to bind server to address")
        .http1_preserve_header_case(true)
        .http1_title_case_headers(true)
        .serve(router.into_make_service())
        .await
        .expect("server failed");
}
