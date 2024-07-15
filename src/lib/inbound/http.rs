use std::sync::Arc;

use anyhow::Context;
use axum::routing::{get, post};
use axum::Router;
use tokio::net;

use crate::domain::messages::ports::MessageService;
use crate::inbound::http::handlers::create_message::create_message;
use crate::inbound::http::handlers::get_message::get_message;
use crate::inbound::http::handlers::queue_list::queue_list;

mod errors;
mod handlers;
mod responses;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpServerConfig<'a> {
    pub port: &'a str,
}

#[derive(Debug, Clone)]
struct AppState<MS: MessageService> {
    message_service: Arc<MS>,
}

pub struct HttpServer {
    router: axum::Router,
    listener: net::TcpListener,
}

impl HttpServer {
    pub async fn new(
        service: impl MessageService,
        config: HttpServerConfig<'_>,
    ) -> anyhow::Result<Self> {
        let trace_layer = tower_http::trace::TraceLayer::new_for_http().make_span_with(
            |request: &axum::extract::Request<_>| {
                let url = request.uri().to_string();
                tracing::info_span!("http_request", method = ?request.method(), url)
            },
        );

        let state = AppState {
            message_service: Arc::new(service),
        };

        let router = axum::Router::new()
            .nest("/api", api_routes())
            .layer(trace_layer)
            .with_state(state);
        let listener = net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
            .await
            .with_context(|| format!("failed to listen on {}", config.port))?;

        Ok(Self { router, listener })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::debug!("listening on {}", self.listener.local_addr().unwrap());
        axum::serve(self.listener, self.router)
            .await
            .context("received error from running server")?;
        Ok(())
    }
}

fn api_routes<MS: MessageService>() -> Router<AppState<MS>> {
    Router::new()
        .route("/", get(queue_list::<MS>))
        .route("/:queue_name", post(create_message::<MS>))
        .route("/:queue_name", get(get_message::<MS>))
        .route("/:queue_name/:uid", get(get_message::<MS>))
}
