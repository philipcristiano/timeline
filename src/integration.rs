use futures_core::stream::Stream;
use futures_util::stream::StreamExt;

pub trait IntegrationT {
    fn name(&self) -> String;

    fn get(&self) -> impl Stream<Item = Result<impl ItemT, anyhow::Error>>;
}

use sqlx::postgres::PgPool;

pub trait ItemT {
    async fn insert(&self, pool: &PgPool) -> anyhow::Result<(), anyhow::Error>;
}
