use std::process::Stdio;

use qpmu_plugin::{
    anyhow::Context, rank, Action, ActivationContext, Input, List, ListItem, Plugin, Result,
};
use tokio::{fs, process::Command};
use tokio_stream::{wrappers::ReadDirStream, StreamExt};

struct Docset {
    lang: String,
    path: String,
}

struct Zealdoc {
    docsets: Vec<Docset>,
}

impl Plugin for Zealdoc {
    async fn new(_: String) -> Result<Self> {
        let docsets_path = dirs::data_dir()
            .context("missing data directory")?
            .join("Zeal/Zeal/docsets/");
        let docsets = fs::read_dir(docsets_path).await?;

        let docsets = ReadDirStream::new(docsets)
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

        Ok(Self { docsets })
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
                        .with_metadata(&docset.lang)
                        .with_icon_name("zeal")
                })
                .collect();
            Ok(List::new(items))
        } else {
            // just search the prefixes
            let list_items: Vec<ListItem> = self
                .docsets
                .iter()
                .map(|docset| ListItem::new(&docset.lang).with_icon_name("zeal"))
                .collect();

            Ok(List::new(
                rank::rank(&query, &list_items, rank::Weights::with_history()).await,
            ))
        }
    }

    async fn activate(&self, cx: ActivationContext) -> Result<Vec<Action>> {
        if cx.item.metadata.is_empty() {
            return Ok(vec![Action::SetInput(
                self.complete(cx).await?.expect("complete always completes"),
            )]);
        }

        let item = cx.item;
        let lang = item.metadata;
        let query = item.title;

        Ok(vec![
            Action::Close,
            Action::RunCommand("zeal".to_string(), vec![format!("{lang}:{query}")]),
        ])
    }

    async fn complete(
        &self,
        ActivationContext { item, .. }: ActivationContext,
    ) -> Result<Option<Input>> {
        if item.metadata.is_empty() {
            // no language selected yet, autocomplete the language
            Ok(Some(Input::new(format!("{}:", item.title))))
        } else {
            // language selected, complete the language and query
            Ok(Some(Input::new(format!(
                "{}:{}",
                item.metadata, item.title
            ))))
        }
    }
}

fn sql_query(query: &str) -> String {
    // search is just based on whether the string is contained
    let sanitized = query.replace(['\'', '"', ';'], "");
    format!("SELECT DISTINCT name FROM searchIndex WHERE name LIKE '%{sanitized}%' LIMIT 0,30;")
}

fn main() {
    qpmu_plugin::main::<Zealdoc>()
}
