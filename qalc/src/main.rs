use std::process::Stdio;

use covey_plugin::{Action, Input, List, ListItem, Plugin, Result, clone_async};
use tokio::process::Command;

covey_plugin::include_manifest!();

struct Qalc;

async fn get_qalc_output(query: &str, extra_args: &[&str]) -> Result<String> {
    let output = Command::new("qalc")
        .args(["-defaults", "-set", "upxrates 0"])
        .args(extra_args)
        .arg("--")
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
        // update exchange rates
        Command::new("qalc").args(["--exrates", "--", ""]).spawn()?;
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<List> {
        let equation = get_qalc_output(&query, &[]).await?;
        let terse = get_qalc_output(&query, &["-t"]).await?;

        let item = ListItem::new(equation.clone())
            .with_icon_name("qalculate")
            .on_activate(clone_async!(terse, || Ok([
                Action::close(),
                Action::copy(terse)
            ])))
            .on_alt_activate(clone_async!(equation, || {
                Ok([
                    Action::close(),
                    Action::copy(equation.lines().last().unwrap_or_default()),
                ])
            }))
            .on_complete(clone_async!(terse, || Ok(Input::new(terse))));
        Ok(List::new(vec![item]))
    }
}

fn main() {
    covey_plugin::run_server::<Qalc>(env!("CARGO_PKG_NAME"))
}
