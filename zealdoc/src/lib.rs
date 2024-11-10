use std::path::PathBuf;

use qpmu_api::{anyhow::Result, *};

struct Docset {
    lang: String,
    path: String,
}

struct Zealdoc {
    docsets: Vec<Docset>,
}

impl Plugin for Zealdoc {
    fn new(_: String) -> Result<Self> {
        let docsets_path = host::data_dir().join("Zeal/Zeal/docsets/");
        let docsets = host::read_dir(dbg!(docsets_path))?;

        let docsets = docsets
            .into_iter()
            .map(PathBuf::from)
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
            .collect();

        Ok(Self { docsets })
    }

    fn query(&mut self, query: String) -> Result<QueryResult> {
        let lang_query = self.docsets.iter().find_map(|docset| {
            Some((
                query.strip_prefix(&docset.lang)?.strip_prefix([' ', ':'])?,
                docset,
            ))
        });

        // search specific language with sql
        if let Some((stripped_query, docset)) = lang_query {
            return Ok(QueryResult::Defer(DeferredAction::Spawn((
                "sqlite3".to_string(),
                vec![docset.path.clone(), sql_query(stripped_query)],
            ))));
        }

        // just search the prefixes
        let list_items: Vec<ListItem> = self
            .docsets
            .iter()
            .map(|docset| ListItem::new(&docset.lang).with_icon(Some("zeal")))
            .collect();

        Ok(QueryResult::SetList(host::rank(
            &query,
            &list_items,
            Weights::default(),
        )))
    }

    fn handle_deferred(&mut self, query: String, result: DeferredResult) -> Result<QueryResult> {
        let target_docset = self
            .docsets
            .iter()
            .find(|docset| {
                query
                    .strip_prefix(&docset.lang)
                    .is_some_and(|rest| rest.starts_with([' ', ':']))
            })
            .expect("action only dispatched after matching docset");

        match result {
            DeferredResult::ProcessOutput(output) => {
                let items: Vec<_> = String::from_utf8(output?.stdout)?
                    .lines()
                    .map(|line| {
                        ListItem::new(line)
                            .with_metadata(&target_docset.lang)
                            .with_icon(Some("zeal"))
                    })
                    .collect();

                Ok(QueryResult::SetList(items))
            }
        }
    }

    fn activate(&mut self, selected: ListItem) -> Result<impl IntoIterator<Item = Action>> {
        if selected.metadata.is_empty() {
            return Ok(vec![Action::SetInputLine(
                self.complete(String::new(), selected)?
                    .expect("complete always completes"),
            )]);
        }

        let lang = selected.metadata;
        let query = selected.title;

        Ok(vec![
            Action::Close,
            Action::RunCommand(("zeal".to_string(), vec![format!("{lang}:{query}")])),
        ])
    }

    fn complete(&mut self, _: String, selected: ListItem) -> Result<Option<InputLine>> {
        if selected.metadata.is_empty() {
            // no language selected yet, autocomplete the language
            Ok(Some(InputLine::new(format!("{}:", selected.title))))
        } else {
            // language selected, complete the language and query
            Ok(Some(InputLine::new(format!(
                "{}:{}",
                selected.metadata, selected.title
            ))))
        }
    }
}

fn sql_query(query: &str) -> String {
    // search is just based on whether the string is contained
    let sanitized = query.replace(['\'', '"', ';'], "");
    format!("SELECT DISTINCT name FROM searchIndex WHERE name LIKE '%{sanitized}%' LIMIT 0,30;")
}

register!(Zealdoc);
