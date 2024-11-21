use std::{path::PathBuf, sync::LazyLock};

use anyhow::{Context, Result};
use qpmu_plugin::*;
use serde::Deserialize;
use tokio::fs;

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
    async fn new(_: String) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        let path = fs::read(&*PROJECTS_PATH)
            .await
            .context("could not open project-manager projects data")?;
        let value: Vec<Data> =
            serde_json::from_slice(&path).context("failed to parse project-manager projects")?;

        let list = value
            .into_iter()
            .map(|value| ListItem::new(value.name).with_description(value.root_path))
            .collect::<Vec<_>>();

        Ok(rank::rank(&query, &list, rank::Weights::with_history()).await)
    }

    async fn activate(
        &self,
        ActivationContext { item, .. }: ActivationContext,
    ) -> Result<Vec<Action>> {
        // https://github.com/brpaz/ulauncher-vscode-projects/blob/master/vscode_projects/listeners/item_enter.py
        Ok(vec![
            Action::Close,
            Action::RunCommand("code".to_string(), vec![item.description]),
        ])
    }
}

fn main() {
    qpmu_plugin::main::<CodeProjects>();
}
