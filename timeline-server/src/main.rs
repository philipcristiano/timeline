use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use clap::Parser;
use maud::{html, DOCTYPE};
use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;

use sqlx::postgres::PgPool;
use sqlx::postgres::PgPoolOptions;

use once_cell::sync::OnceCell;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies, Key};
use rust_embed::RustEmbed;

const COOKIE_NAME: &str = "auth_flow";
static KEY: OnceCell<Key> = OnceCell::new();

#[derive(RustEmbed, Clone)]
#[folder = "static/"]
struct StaticAssets;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    bind_addr: String,
    #[arg(short, long, default_value = "timeline.toml")]
    config_file: String,
    #[arg(short, long, value_enum, default_value = "INFO")]
    log_level: tracing::Level,
    #[arg(long, action)]
    log_json: bool,
}

mod auth;
mod html;
mod integration;
mod integration_config;
mod integrations;

#[derive(Clone, Debug, Deserialize)]
struct AppConfig {
    //auth: auth::AuthConfig,
    integration: Vec<integration_config::IntegrationConfig>,
    database_url: String,
    auth: service_conventions::oidc::OIDCConfig,
}

#[derive(Clone)]
struct AppState {
    auth: service_conventions::oidc::AuthConfig,
    db: PgPool,
}

impl AppState {
    fn from_config(item: AppConfig, db: PgPool) -> Self {
        let auth_config = service_conventions::oidc::AuthConfig {
            oidc_config: item.auth,
            post_auth_path: "/".to_string(),
            scopes: vec!["profile".to_string(), "email".to_string()],
        };
        AppState {
            auth: auth_config,
            db,
        }
    }
}

use tower_http::trace::{self, TraceLayer};
use tracing::Level;

#[tokio::main]
async fn main() {
    // initialize tracing
    let my_key: &[u8] = &[0; 64]; // Your real key must be cryptographically random
    KEY.set(Key::from(my_key)).ok();

    let args = Args::parse();

    service_conventions::tracing::setup(args.log_level);

    let config_file_error_msg = format!("Could not read config file {}", args.config_file);
    let config_file_contents = fs::read_to_string(args.config_file).expect(&config_file_error_msg);

    let app_config: AppConfig =
        toml::from_str(&config_file_contents).expect("Problems parsing config file");
    tracing::debug!("Config {:?}", app_config);

    tracing::info!("connecting to {}", app_config.database_url);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&app_config.database_url)
        .await
        .expect("Cannot connect to DB");

    let mut integrations = Vec::new();

    for i in app_config.clone().integration {
        println!("Make integration => {:?}", i);
        integrations.push(i.into_integration())
    }

    let pool2 = pool.clone();
    tokio::spawn(async move {
        integration::sync_all(integrations.clone(), pool2).await;
    });

    let app_state = AppState::from_config(app_config, pool);

    let oidc_router = service_conventions::oidc::router(app_state.auth.clone());
    let serve_assets = axum_embed::ServeEmbed::<StaticAssets>::new();
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(http_get_docs))
        .with_state(app_state.clone())
        .nest("/oidc", oidc_router.with_state(app_state.auth.clone()))
        .nest_service("/static", serve_assets)
        .layer(CookieManagerLayer::new())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );

    let addr: SocketAddr = args.bind_addr.parse().expect("Expected bind addr");
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

use pretty_date::pretty_date_formatter::PrettyDateFormatter;
async fn http_get_docs(state: State<AppState>,
    user: Option<service_conventions::oidc::OIDCUser>) -> Response {

    if let None = user {
        return html::maud_page(html! {
            p { "Welcome! You need to login" }
            a href="/oidc/login" { "Login" }
        })
        .into_response()
    };
    let docs = sqlx::query_as!(
        integrations::paperless_ngx::APIDoc,
        "select external_id as id, created, title from documents order by created desc;"
    )
    .fetch_all(&state.db)
    .await
    .expect("DB call failed");

    let content = html! {
            p { "Paperless NGX Documents!"}
            @for doc in &docs {
                li { (doc.title) (" ") (doc.created.naive_utc().format_pretty())}
            }
    };
    let page = html::maud_page(content);

    page.into_response()
}
