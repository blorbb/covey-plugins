use std::process::Stdio;

use qpmu_plugin::{Action, ActivationContext, Input, List, ListItem, Plugin, Result};
use tokio::process::Command;

struct Qalc;

async fn get_qalc_output(query: &str, extra_args: &[&str]) -> Result<String> {
    let output = Command::new("qalc")
        .args(extra_args)
        .arg(query)
        .stdin(Stdio::null())
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

    async fn query(&self, query: String) -> Result<List> {
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
            .with_icon_name("qalculate");
        Ok(List::new(vec![item]))
    }

    async fn activate(
        &self,
        ActivationContext { item, .. }: ActivationContext,
    ) -> Result<Vec<Action>> {
        Ok(vec![
            Action::Close,
            Action::Copy(get_qalc_output(&item.metadata, &["-t"]).await?),
        ])
    }

    async fn alt_activate(
        &self,
        ActivationContext { item, .. }: ActivationContext,
    ) -> Result<Vec<Action>> {
        // copy the entire output string, not just the final expression
        Ok(vec![
            Action::Close,
            Action::Copy(get_qalc_output(&item.metadata, &[]).await?),
        ])
    }

    async fn complete(
        &self,
        ActivationContext { item, .. }: ActivationContext,
    ) -> Result<Option<Input>> {
        Ok(Some(Input::new(
            get_qalc_output(&item.metadata, &["-t"]).await?,
        )))
    }
}

fn main() {
    qpmu_plugin::main::<Qalc>()
}
