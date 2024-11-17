use std::process::Stdio;

use anyhow::Result;
use qpmu_api::*;
use tokio::process::Command;

struct Qalc;

async fn get_terse_qalc_output(query: &str) -> Result<String> {
    let output = Command::new("qalc")
        .arg("-t")
        .arg(query)
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

impl Plugin for Qalc {
    async fn new(_: String) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        let output = Command::new("qalc")
            .arg(&query)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        let line = String::from_utf8(output.stdout)?.trim().to_string();
        let item = ListItem::new(line)
            .with_metadata(query)
            .with_icon(Some("qalculate"));
        Ok(vec![item])
    }

    async fn activate(&self, item: ListItem) -> Result<Vec<Action>> {
        Ok(vec![
            Action::Close,
            Action::Copy(get_terse_qalc_output(&item.metadata).await?),
        ])
    }

    async fn complete(&self, _query: String, selected: ListItem) -> Result<Option<InputLine>> {
        Ok(Some(InputLine::new(
            get_terse_qalc_output(&selected.metadata).await?,
        )))
    }
}

fn main() {
    qpmu_api::main::<Qalc>()
}
