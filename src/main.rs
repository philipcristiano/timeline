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

const COOKIE_NAME: &str = "auth_flow";
static KEY: OnceCell<Key> = OnceCell::new();

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
mod integration;
mod integration_config;
mod integrations;

#[derive(Clone, Debug, Deserialize)]
struct AppConfig {
    //auth: auth::AuthConfig,
    integration: Vec<integration_config::IntegrationConfig>,
    database_url: String,
}

#[derive(Clone)]
struct AppState {
    db: PgPool,
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

    for i in app_config.integration {
        println!("Make integration => {:?}", i);
        integrations.push(i.into_integration())
    }

    let pool2 = pool.clone();
    tokio::spawn(async move {
        integration::sync_all(integrations.clone(), pool2).await;
    });

    let app_state = AppState { db: pool.clone() };

    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(http_get_docs))
        // .route(
        //     "/login",
        //     get(oidc_login).with_state(app_config.auth.clone()),
        // )
        // .route("/login_auth", get(oidc_login_auth))
        // .with_state(app_config.auth.clone())
        .with_state(app_state)
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
async fn http_get_docs(state: State<AppState>) -> Response {
    let docs = sqlx::query_as!(
        integrations::paperless_ngx::APIDoc,
        "select external_id as id, created, title from documents order by created desc;"
    )
    .fetch_all(&state.db)
    .await
    .expect("DB call failed");

    html! {
       (DOCTYPE)
            p { "Welcome!"}
            @for doc in &docs {
            li { (doc.title) (doc.created.naive_utc().format_pretty())}
        }
    }
    .into_response()
}

#[tracing::instrument]
async fn oidc_login(State(config): State<auth::AuthConfig>, cookies: Cookies) -> impl IntoResponse {
    let auth_client = auth::construct_client(config.clone()).await.unwrap();
    let auth_content = auth::get_auth_url(auth_client).await;
    let key = KEY.get().unwrap();
    let private_cookies = cookies.private(key);
    let cookie_val = serde_json::to_string(&auth_content.verify).unwrap();
    private_cookies.add(Cookie::new(COOKIE_NAME, cookie_val));

    Redirect::temporary(&auth_content.redirect_url.to_string())
}

#[derive(Debug, Deserialize)]
struct OIDCAuthCode {
    code: String,
    state: String,
}

#[derive(Debug)]
struct AuthError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        tracing::info!("Auth error {:?}", self);
        let resp = html! {
        (DOCTYPE)
            p { "You are not authorized"}
            a href="/login" { "Restart" }
        };
        (StatusCode::UNAUTHORIZED, resp).into_response()
    }
}

// impl From<serde_json::Error> for AuthError {
//     fn from(_err: serde_json::Error) -> AuthError {
//         AuthError(anyhow::anyhow!("Json serialization error"))
//     }
// }
// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AuthError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[tracing::instrument]
async fn oidc_login_auth(
    State(config): State<auth::AuthConfig>,
    cookies: Cookies,
    Query(oidc_auth_code): Query<OIDCAuthCode>,
) -> Result<Response, AuthError> {
    let auth_client = auth::construct_client(config.clone()).await.unwrap();
    let key = KEY.get().unwrap();
    let private_cookies = cookies.private(key);
    let cookie = match private_cookies.get(COOKIE_NAME) {
        Some(c) => c,
        _ => return Ok(StatusCode::UNAUTHORIZED.into_response()),
    };

    let cookie_str = cookie.value();
    let auth_verify: auth::AuthVerify = serde_json::from_str(&cookie_str)?;

    if auth_verify.csrf_token.secret() != &oidc_auth_code.state {
        tracing::error!("CSRF State doesn't match");
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }

    let claims = auth::next(auth_client, auth_verify, oidc_auth_code.code).await?;

    let resp = html! {
        (DOCTYPE)
        p { "User " (claims.subject().as_str()) " has authenticated successfully"}
        p { "Email: " (
                        claims
                        .email()
                        .map(|email| email.as_str())
                        .unwrap_or("<not provided>")) }
        a href="/login" { "Restart" }
    };

    Ok(resp.into_response())
}
