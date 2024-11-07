use qpmu_api::{
    export,
    host::{self, Capture},
    ListItem, Plugin, PluginAction,
};

struct Qalc;

impl Plugin for Qalc {
    fn input(query: String) -> Vec<ListItem> {
        let Ok(output) = host::spawn("qalc", [&query], Capture::STDOUT) else {
            return vec![ListItem::new("failed to open qalc")];
        };

        vec![ListItem::new(String::from_utf8(output.stdout).unwrap().trim()).with_metadata(query)]
    }

    fn activate(item: ListItem) -> Vec<PluginAction> {
        let Ok(output) = host::spawn("qalc", ["-t", &item.metadata], Capture::STDOUT) else {
            return vec![];
        };
 
        vec![
            PluginAction::Close,
            PluginAction::Copy(String::from_utf8(output.stdout).unwrap().trim().to_string()),
        ]
    }
}

export!(Qalc with_types_in qpmu_api::bindings);
