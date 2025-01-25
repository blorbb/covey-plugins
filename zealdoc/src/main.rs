use std::process::Stdio;

use covey_plugin::{
    anyhow::Context, clone_async, rank, Action, Input, List, ListItem, Plugin, Result,
};
use tokio::{fs, process::Command};
use tokio_stream::{wrappers::ReadDirStream, StreamExt};

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
                    .on_activate(clone_async!(
                        #[double]
                        lang = docset.lang,
                        || Ok(vec![Action::SetInput(Input::new(format!("{lang}:")))])
                    ))
                    .on_complete(clone_async!(
                        #[double]
                        lang = docset.lang,
                        || Ok(Some(Input::new(format!("{lang}:"))))
                    ))
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
                        .on_activate(clone_async!(
                            #[double]
                            lang = docset.lang,
                            #[double]
                            stripped_query,
                            || {
                                Ok(vec![
                                    Action::Close,
                                    Action::RunCommand(
                                        "zeal".to_string(),
                                        vec![format!("{lang}:{stripped_query}")],
                                    ),
                                ])
                            }
                        ))
                        .on_complete(clone_async!(
                            #[double]
                            lang = docset.lang,
                            #[double]
                            line,
                            || Ok(Some(Input::new(format!("{lang}:{line}"))))
                        ))
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
    covey_plugin::main::<Zealdoc>()
}
