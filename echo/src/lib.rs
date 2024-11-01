use qpmu_api::{export, ListItem, Plugin, PluginAction};

struct Echo;

impl Plugin for Echo {
    fn input(query: String) -> Vec<ListItem> {
        vec![ListItem {
            title: query,
            description: String::new(),
            metadata: String::new(),
        }]
    }

    fn activate(_: ListItem) -> Vec<PluginAction> {
        vec![]
    }
}

export!(Echo with_types_in qpmu_api::bindings);
