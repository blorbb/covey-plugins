use qpmu_api::{
    export,
    host::{self, Capture},
    ListItem, Plugin, PluginAction,
};

struct Qalc;

impl Plugin for Qalc {
    fn input(query: String) -> Vec<ListItem> {
        let Ok(output) = host::spawn("qalc", [query], Capture::STDOUT) else {
            return vec![ListItem {
                title: "failed to open qalc".to_string(),
                description: String::new(),
                metadata: String::new(),
            }];
        };

        vec![ListItem {
            title: String::from_utf8(output.stdout).expect("TODO").trim().to_string(),
            description: String::new(),
            metadata: String::new(),
        }]
    }

    fn activate(item: ListItem) -> Vec<PluginAction> {
        vec![PluginAction::Close, PluginAction::Copy(item.title)]
    }
}

export!(Qalc with_types_in qpmu_api::bindings);
