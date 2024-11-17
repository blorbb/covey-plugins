use std::time::Duration;

use qpmu_api::{anyhow::Result, tokio, Action, ListItem, Plugin};

struct Echo;

impl Plugin for Echo {
    async fn new(_: String) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(vec![ListItem::new(query)])
    }

    async fn activate(&self, _query: ListItem) -> Result<Vec<Action>> {
        Ok(vec![])
    }
}

fn main() {
    qpmu_api::main::<Echo>()
}
