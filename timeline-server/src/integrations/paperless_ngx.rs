use crate::integration::{IntegrationT, ItemT};

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::collections::HashMap;

use async_stream::try_stream;
use chrono::prelude::*;

use futures_core::stream::Stream;
pub fn new(host: String, token: String) -> PaperlessIntegration {
    PaperlessIntegration { host, token }
}

use sqlx::postgres::PgPool;

#[derive(Clone, Debug, Deserialize)]
pub struct PaperlessIntegration {
    host: String,
    token: String,
}

#[derive(Clone, Debug, Deserialize)]
struct APIDocResponse {
    count: i32,
    next: Option<String>,
    results: Vec<APIDoc>,
}
#[derive(Clone, Debug, Deserialize, sqlx::FromRow)]
pub struct APIDoc {
    pub id: i32,
    pub title: String,
    pub created: DateTime<Utc>,
}

impl ItemT for APIDoc {
    async fn insert(&self, pool: &PgPool) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
    INSERT INTO documents ( external_id, created, title )
    VALUES ( $1, $2, $3 )
    ON CONFLICT (external_id) DO UPDATE
        SET created = EXCLUDED.created,
            title = EXCLUDED.title
    RETURNING external_id
            "#,
            self.id,
            self.created,
            self.title
        )
        .fetch_one(pool)
        .await?;

        Ok(())
    }
}

impl std::fmt::Display for APIDoc {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{} {} {}", self.id, self.title, self.created)
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct APIError(#[from] reqwest::Error);

impl PaperlessIntegration {
    fn document_stream(&self) -> impl Stream<Item = anyhow::Result<impl ItemT>> {
        let host = self.host.clone();

        let token = self.token.clone();
        try_stream! {
            let mut maybe_next_url = Some(format!("{host}/api/documents/"));

            let client = reqwest::Client::new();
            while let Some(next_url) = maybe_next_url {
                tracing::debug!("Getting URL {} ", next_url);

                let response = request(&client, &next_url, &token).await?;
                tracing::debug!("HTTP Response {:?}", response);
                let response_body = response
                     .json::<APIDocResponse>()
                     .await?;

                tracing::debug!("Next URL {:?}", response_body.next);
                maybe_next_url = response_body.next;

                for doc in response_body.results {
                    yield doc
                }
            }
        }
    }
}

#[tracing::instrument(skip_all)]
async fn request(
    client: &reqwest::Client,
    url: &String,
    token: &String,
) -> anyhow::Result<reqwest::Response> {
    let tracing_headers = service_conventions::tracing_http::get_tracing_headers();
    Ok(client
        .get(url)
        .header(AUTHORIZATION, format!("Token {token}"))
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(tracing_headers)
        .send()
        .await?)
}
use futures_util::stream::StreamExt;
impl IntegrationT for PaperlessIntegration {
    fn name(&self) -> String {
        String::from("Paperless NGX")
    }

    fn go(&self, pool: &PgPool) -> impl Stream<Item = anyhow::Result<impl ItemT>> {
        let s = self.document_stream();
        use futures_util::pin_mut;
        try_stream! {
            pin_mut!(s);
            while let Some(maybe_ok_value) = s.next().await {
                match maybe_ok_value {
                    Ok(value) => yield value,
                    Err(e) => println!("Error: {:?}", e),
                }
            }
        }
    }
    //fn get(&self) -> impl Stream<Item = impl ItemT>{
    //    self.documents()
    //}
}
