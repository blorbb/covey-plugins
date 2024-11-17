use qpmu_api::{anyhow::Result, Action, ListItem, Plugin};

struct Echo;

impl Plugin for Echo {
    async fn new(_: String) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        Ok(vec![ListItem::new(query)])
    }

    async fn activate(&self, query: ListItem) -> Result<Vec<Action>> {
        Ok(vec![])
    }
}

fn main() {
    qpmu_api::main::<Echo>()
}
