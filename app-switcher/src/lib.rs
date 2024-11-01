use std::{fs, sync::LazyLock};

use freedesktop_entry_parser as desktop;
use qpmu_api::{export, ListItem, Plugin, PluginAction};

static ENTRIES: LazyLock<Vec<ListItem>> = LazyLock::new(|| {
    let Ok(entries) = fs::read_dir("/usr/share/applications") else {
        return vec![];
    };
    entries
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| desktop::parse_entry(entry.path()).ok())
        .filter_map(|entry| {
            Some(ListItem {
                title: entry.section("Desktop Entry").attr("Name")?.to_string(),
                description: entry
                    .section("Desktop Entry")
                    .attr("Comment")
                    .unwrap_or_default()
                    .to_string(),
                metadata: entry
                    .section("Desktop Entry")
                    .attr("Exec")?
                    .replace("%u", "")
                    .replace("%U", ""),
            })
        })
        .collect()
});

struct AppSwitcher;

impl Plugin for AppSwitcher {
    fn input(query: String) -> Vec<ListItem> {
        let mut entries = ENTRIES.clone();
        entries.sort_by_cached_key(|k| sublime_fuzzy::best_match(&query, &k.title));
        entries.reverse();
        entries
    }

    fn activate(selected: ListItem) -> Vec<PluginAction> {
        vec![PluginAction::Close, PluginAction::RunCommandString(selected.metadata)]
    }
}

export!(AppSwitcher with_types_in qpmu_api::bindings);
