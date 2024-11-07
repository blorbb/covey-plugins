use qpmu_api::{anyhow::Result, register, ListItem, Plugin, PluginAction, QueryResult};

struct Echo;

impl Plugin for Echo {
    fn query(query: String) -> Result<QueryResult> {
        Ok(QueryResult::SetList(vec![ListItem::new(query)]))
    }

    fn activate(_: ListItem) -> Result<impl IntoIterator<Item = PluginAction>> {
        Ok([])
    }
}

register!(Echo);
