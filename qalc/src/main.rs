use std::{
    process::Stdio,
    sync::{Arc, RwLock},
};

use covey_plugin::{Input, List, ListItem, Plugin, Result, clone_async};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

covey_plugin::include_manifest!();

#[derive(Debug, Serialize, Deserialize)]
struct HistoryEntry {
    query: String,
    equation: String,
    result: String,
}

#[derive(Clone, Default)]
struct Qalc {
    history: Arc<RwLock<Vec<HistoryEntry>>>,
}

impl Plugin for Qalc {
    type Config = ();

    async fn new(_: ()) -> Result<Self> {
        // update exchange rates
        Command::new("qalc").args(["--exrates", "--", ""]).spawn()?;

        let history = try_read_history().await.unwrap_or_default();
        Ok(Self {
            history: Arc::new(RwLock::new(history)),
        })
    }

    async fn query(&self, query: String) -> Result<List> {
        let output = get_qalc_output(&query, &[]).await?;
        let equation = output.lines().last().unwrap_or_default().to_string();
        let terse = get_qalc_output(&query, &["-t"]).await?;

        let item = ListItem::new(output)
            .with_icon_name("qalculate")
            .on_copy(clone_async!(this = self, query, equation, terse, |menu| {
                this.add_to_history(&query, equation, &terse);
                menu.close();
                menu.copy(terse);
                menu.set_input(Input::new(query));
                Ok(())
            }))
            .on_copy_equation(clone_async!(this = self, query, equation, terse, |menu| {
                this.add_to_history(&query, &equation, terse);
                menu.close();
                menu.copy(equation);
                menu.set_input(Input::new(query));
                Ok(())
            }))
            .on_complete(clone_async!(this = self, query, equation, terse, |menu| {
                this.add_to_history(&query, equation, &terse);
                menu.set_input(Input::new(terse));
                Ok(())
            }));

        // add history items
        let history = self.history.read().unwrap();
        let history = history.iter().rev().map(
            |HistoryEntry {
                 query: history_query,
                 equation,
                 result,
             }| {
                ListItem::new(equation)
                    .on_append_history_result(clone_async!(query, result, |menu| {
                        menu.set_input(Input::new(format!("{query}{result}")));
                        Ok(())
                    }))
                    .on_insert_history_query(clone_async!(history_query, |menu| {
                        menu.set_input(Input::new(history_query));
                        Ok(())
                    }))
            },
        );

        Ok(List::new(std::iter::once(item).chain(history).collect()))
    }
}

impl Qalc {
    pub fn add_to_history(
        &self,
        query: &str,
        equation: impl Into<String>,
        result: impl Into<String>,
    ) {
        // only add to history if the query changed
        let mut history = self.history.write().unwrap();
        if history.last().is_none_or(|last| last.query != query) {
            let entry = HistoryEntry {
                query: query.to_string(),
                equation: equation.into(),
                result: result.into(),
            };
            history.push(entry);

            // only keep up to 500 items
            let len = history.len();
            history.drain(..len.saturating_sub(500));

            let json = serde_json::to_string(&*history).expect("serialization should not fail");
            tokio::spawn(async move {
                tokio::fs::write(history_file_path(), json)
                    .await
                    .expect("(TODO) oops failed to write to file");
            });
        }
    }
}

fn history_file_path() -> std::path::PathBuf {
    covey_plugin::plugin_data_dir().join("history.json")
}

async fn try_read_history() -> std::io::Result<Vec<HistoryEntry>> {
    let entries: Vec<HistoryEntry> =
        serde_json::from_slice(&tokio::fs::read(history_file_path()).await?)?;

    Ok(entries)
}

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

fn main() {
    covey_plugin::run_server::<Qalc>(env!("CARGO_PKG_NAME"))
}
