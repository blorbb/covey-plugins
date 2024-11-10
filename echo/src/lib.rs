use qpmu_api::{anyhow::Result, register, Action, ListItem, Plugin, QueryResult};

struct Echo;

impl Plugin for Echo {
    fn new(_: String) -> Result<Self> {
        Ok(Self)
    }
    
    fn query(&mut self, query: String) -> Result<QueryResult> {
        Ok(QueryResult::SetList(vec![ListItem::new(query)]))
    }

    fn activate(&mut self, _: ListItem) -> Result<impl IntoIterator<Item = Action>> {
        Ok([])
    }
}

register!(Echo);
