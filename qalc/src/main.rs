use std::process::Stdio;

use covey_plugin::{clone_async, Action, Input, List, ListItem, Plugin, Result};
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
    type Config = ();

    async fn new(_: ()) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<List> {
        let line = get_qalc_output(&query, &[]).await?;
        let terse = get_qalc_output(&query, &["-t"]).await?;

        let item = ListItem::new(line.clone())
            .with_icon_name("qalculate")
            .on_activate(clone_async!(
                #[double]
                terse,
                || Ok(vec![Action::Close, Action::Copy(terse)])
            ))
            .on_alt_activate(clone_async!(line, || {
                Ok(vec![Action::Close, Action::Copy(line)])
            }))
            .on_complete(clone_async!(terse, || Ok(Some(Input::new(terse)))));
        Ok(List::new(vec![item]))
    }
}

fn main() {
    covey_plugin::main::<Qalc>()
}
