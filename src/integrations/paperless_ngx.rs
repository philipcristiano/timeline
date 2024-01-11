use crate::integration::IntegrationT;

use serde::{ Deserialize};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, ACCEPT};

use async_stream::{stream, try_stream};

use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
pub fn new(host: String, token: String) -> PaperlessIntegration {
    PaperlessIntegration{
        host,
        token
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PaperlessIntegration {
    host: String,
    token: String,
}

pub struct DocumentMeta {
    id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct APIDocResponse {
    count: u32,
    next: String,
    results: Vec<APIDoc>
}
#[derive(Clone, Debug, Deserialize)]
struct APIDoc {
    id: u32,
    title: String,
}

impl std::fmt::Display for APIDoc {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.title)
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

    async fn get(&self) {

        let docs = self.documents();
        pin_mut!(docs);
         while let Some(doc) = docs.next().await {
             match doc {
                 Err(e) => println!("Error {}", e),
                 Ok(okdoc) => println!("Doc {}", okdoc),
             };

        }


    }

}
