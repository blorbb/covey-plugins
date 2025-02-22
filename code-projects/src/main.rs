use std::{path::PathBuf, sync::LazyLock};

use covey_plugin::{
    Action, List, ListItem, Plugin, Result, anyhow::Context as _, clone_async, rank,
};
use serde::Deserialize;
use tokio::fs;

covey_plugin::include_manifest!();

static PROJECTS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let config_dir = dirs::config_dir().unwrap();
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
    type Config = ();

    async fn new(_: ()) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<List> {
        let path = fs::read(&*PROJECTS_PATH)
            .await
            .context("could not open project-manager projects data")?;
        let value: Vec<Data> =
            serde_json::from_slice(&path).context("failed to parse project-manager projects")?;

        let list = value
            .into_iter()
            .map(|value| {
                ListItem::new(value.name)
                    .with_description(value.root_path.clone())
                    .on_activate(clone_async!(path = value.root_path, || {
                        // https://github.com/brpaz/ulauncher-vscode-projects/blob/master/vscode_projects/listeners/item_enter.py
                        Ok([
                            Action::Close,
                            Action::RunCommand("code".to_string(), vec![path]),
                        ])
                    }))
            })
            .collect::<Vec<_>>();

        Ok(List::new(
            rank::rank(&query, &list, rank::Weights::with_history()).await,
        ))
    }
}

fn main() {
    covey_plugin::run_server::<CodeProjects>(env!("CARGO_PKG_NAME"));
}
