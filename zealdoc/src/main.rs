use std::process::Stdio;

use covey_plugin::{
    Input, List, ListItem, Plugin, Result, anyhow::Context, clone_async, rank, spawn,
};
use tokio::{fs, process::Command};
use tokio_stream::{StreamExt, wrappers::ReadDirStream};

covey_plugin::include_manifest!();

struct Docset {
    lang: String,
    path: String,
}

struct Zealdoc {
    docsets: Vec<Docset>,
    // what to show if the query doesn't match one of the languages yet
    prefix_prompt: Vec<ListItem>,
}

impl Plugin for Zealdoc {
    type Config = ();
    async fn new(_: ()) -> Result<Self> {
        let docsets_path = dirs::data_dir()
            .context("missing data directory")?
            .join("Zeal/Zeal/docsets/");
        let docsets = fs::read_dir(docsets_path).await?;

        let docsets: Vec<_> = ReadDirStream::new(docsets)
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "docset"))
            .filter_map(|path| {
                Some(Docset {
                    lang: path.file_stem()?.to_str()?.to_lowercase(),
                    path: path
                        .join("Contents/Resources/docSet.dsidx")
                        .to_str()?
                        .to_string(),
                })
            })
            .collect()
            .await;

        let prefix_prompt = docsets
            .iter()
            .map(|docset| {
                ListItem::new(&docset.lang)
                    .with_icon_name("zeal")
                    .on_complete(clone_async!(lang = docset.lang, |menu| {
                        menu.set_input(Input::new(format!("{lang}:")));
                        Ok(())
                    }))
            })
            .collect();

        Ok(Self {
            docsets,
            prefix_prompt,
        })
    }

    async fn query(&self, query: String) -> Result<List> {
        let lang_query = self.docsets.iter().find_map(|docset| {
            Some((
                query.strip_prefix(&docset.lang)?.strip_prefix([' ', ':'])?,
                docset,
            ))
        });

        if let Some((stripped_query, docset)) = lang_query {
            // search specific language with sql
            let output = Command::new("sqlite3")
                .arg(&docset.path)
                .arg(sql_query(stripped_query))
                .stdout(Stdio::piped())
                .spawn()?
                .wait_with_output()
                .await?;

            let items = String::from_utf8(output.stdout)?
                .lines()
                .map(|line| {
                    ListItem::new(line)
                        .with_icon_name("zeal")
                        .on_activate(clone_async!(lang = docset.lang, stripped_query, |menu| {
                            menu.close();
                            spawn::program("zeal", [format!("{lang}:{stripped_query}")])?;
                            Ok(())
                        }))
                        .on_complete(clone_async!(lang = docset.lang, line, |menu| {
                            menu.set_input(Input::new(format!("{lang}:{line}")));
                            Ok(())
                        }))
                })
                .collect();
            Ok(List::new(items))
        } else {
            // just search the prefixes
            Ok(List::new(
                rank::rank(&query, &self.prefix_prompt, rank::Weights::with_history()).await,
            ))
        }
    }
}

fn sql_query(query: &str) -> String {
    // search is just based on whether the string is contained
    let sanitized = query.replace(['\'', '"', ';'], "");
    format!("SELECT DISTINCT name FROM searchIndex WHERE name LIKE '%{sanitized}%' LIMIT 0,30;")
}

fn main() {
    covey_plugin::run_server::<Zealdoc>(env!("CARGO_PKG_NAME"))
}
