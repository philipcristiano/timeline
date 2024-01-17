use crate::integration::{IntegrationT, ItemT};

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;

use async_stream::try_stream;
use chrono::prelude::*;

use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
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
    next: String,
    results: Vec<APIDoc>,
}
#[derive(Clone, Debug, Deserialize, sqlx::FromRow)]
struct APIDoc {
    id: i32,
    title: String,
    created: DateTime<Utc>,
}

impl ItemT for APIDoc {
    async fn insert(&self, pool: &PgPool) -> anyhow::Result<(), anyhow::Error> {
        let rec = sqlx::query!(
            r#"
    INSERT INTO documents ( external_id, created, title, notes )
    VALUES ( $1, $2, $3, $3 )
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
        println!("Inserting!");

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
    fn documents(&self) -> impl Stream<Item = Result<APIDoc, reqwest::Error>> {
        let host = self.host.clone();

        let token = self.token.clone();
        try_stream! {
            let mut maybe_next_url = Some(format!("{host}/api/documents/"));

            let client = reqwest::Client::new();
            while let Some(next_url) = maybe_next_url {
                println!("Getting URL {} ", next_url);

                let response = client
                     .get(next_url)
                     .header(AUTHORIZATION, format!("Token {token}"))
                     .header(CONTENT_TYPE, "application/json")
                     .header(ACCEPT, "application/json")
                     .send()
                     .await?;
                println!("HTTP Response {:?}", response);
                let response_body = response
                     .json::<APIDocResponse>()
                     .await?;

                println!("Next URL {}", response_body.next);
                maybe_next_url = Some(response_body.next);
                //maybe_next_url = Some(response_body.next);

                for doc in response_body.results {
                    yield doc

                }
            }
        }
    }
}

impl IntegrationT for PaperlessIntegration {
    fn name(&self) -> String {
        String::from("Paperless NGX")
    }

    fn get(&self) -> impl Stream<Item = Result<impl ItemT, anyhow::Error>> {
        try_stream! {
        let docs = self.documents();
        pin_mut!(docs);
        while let Some(doc) = docs.next().await {
            match doc {
                Err(e) => println!("Error {}", e),
                Ok(okdoc) => yield okdoc,
            };
        }}
    }
}
