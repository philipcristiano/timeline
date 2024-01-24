use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;

pub trait IntegrationT {
    fn name(&self) -> String;

    //async fn go(&self, pool: &PgPool) -> ();
    //fn go(&self, pool: &PgPool) -> impl std::future::Future<Output = ()> + std::marker::Send;
    fn go(&self, pool: &PgPool) -> impl Stream<Item = anyhow::Result<impl ItemT + Send>> + Send;
    //fn get(&self) -> impl Stream<Item = impl ItemT>;
}

use sqlx::postgres::PgPool;

pub trait ItemT {
    fn insert(
        &self,
        pool: &PgPool,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + std::marker::Send;
}

use tokio::time::{sleep, Duration};
pub async fn sync_all(integrations: Vec<impl IntegrationT + Clone + Send + 'static>, pool: PgPool) {
    loop {
        for integration in integrations.clone() {
            let pool2 = pool.clone();

            tokio::spawn(async move {
                let i = integration.clone();
                let s = i.go(&pool2);
                pin_mut!(s);
                while let Some(Ok(value)) = s.next().await {
                    println!("insert ");
                    value.insert(&pool2).await;
                }
            });
        }
        sleep(Duration::from_millis(1000 * 60 * 30)).await;
    }
}
// async fn sync_all() {
//
// }
// async fn sync(integration: impl IntegrationT, pool: &PgPool) -> Result<(), anyhow::Error> {
//
// }
