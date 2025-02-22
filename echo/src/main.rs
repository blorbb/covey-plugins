use std::time::Duration;

use covey_plugin::{List, ListItem, Plugin, Result};

struct Echo;

impl Plugin for Echo {
    type Config = ();

    async fn new(_: ()) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<List> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(List::new(vec![ListItem::new(query)]))
    }
}

fn main() {
    covey_plugin::run_server::<Echo>(env!("CARGO_PKG_NAME"))
}
