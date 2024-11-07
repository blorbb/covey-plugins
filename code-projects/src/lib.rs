use std::{cmp::Reverse, path::PathBuf, sync::LazyLock};

use qpmu_api::{
    anyhow::{Context, Result},
    host, register, ListItem, Plugin, PluginAction, QueryResult,
};
use serde::Deserialize;

static PROJECTS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let config_dir = host::config_dir();
    config_dir.join("Code/User/globalStorage/alefragnani.project-manager/projects.json")
});

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    name: String,
    root_path: String,
}

struct CodeProjects;

impl Plugin for CodeProjects {
    fn query(query: String) -> Result<QueryResult> {
        let path = host::read_file(&*PROJECTS_PATH)
            .context("could not open project-manager projects data")?;
        let value: Vec<Data> = serde_json_wasm::from_slice(&path)
            .context("failed to parse project-manager projects")?;

        let mut list = value
            .into_iter()
            .map(|value| ListItem::new(value.name).with_description(value.root_path))
            .filter_map(|item| Some((sublime_fuzzy::best_match(&query, &item.title)?, item)))
            .collect::<Vec<_>>();

        list.sort_by_key(|(score, _)| Reverse(score.score()));

        Ok(QueryResult::SetList(
            list.into_iter().map(|(_, item)| item).collect(),
        ))
    }

    fn activate(selected: ListItem) -> Result<impl IntoIterator<Item = PluginAction>> {
        // https://github.com/brpaz/ulauncher-vscode-projects/blob/master/vscode_projects/listeners/item_enter.py
        Ok([
            PluginAction::Close,
            PluginAction::RunCommand(("code".to_string(), vec![selected.description])),
        ])
    }
}

register!(CodeProjects);
