pub trait IntegrationT {
    fn name(&self) -> String;

    async fn get(&self);
}
