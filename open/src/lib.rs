use std::collections::HashMap;

use qpmu_api::{anyhow::Result, host, Action, InputLine, ListItem, Plugin, QueryResult, Weights};
use serde::Deserialize;

struct Open {
    urls: Vec<(String, OpenTarget)>,
}

#[derive(Debug, Deserialize)]
struct OpenTarget {
    name: String,
    url: String,
}

impl Plugin for Open {
    fn new(config: String) -> Result<Self> {
        let urls: HashMap<String, OpenTarget> = toml::from_str(&config)?;
        Ok(Self {
            urls: urls.into_iter().collect(),
        })
    }

    fn query(&mut self, query: String) -> Result<QueryResult> {
        if let Some((new_query, target)) = self.urls.iter().find_map(|(prefix, target)| {
            query
                .strip_prefix(prefix)
                .and_then(|s| s.strip_prefix(' ')) // require space after
                .map(|new_query| (new_query.trim(), target))
        }) {
            // query matches one of the prefixes in the list
            let search = ListItem::new(format!("Search {} for {}", target.name, new_query))
                .with_description(target.url.replace("%s", new_query));

            Ok(QueryResult::SetList(vec![search]))
        } else {
            // query doesn't match any prefix in the list: rank them
            let items = self
                .urls
                .iter()
                .enumerate()
                .map(|(index, (prefix, target))| {
                    ListItem::new(format!("{prefix}: {}", target.name))
                        .with_description(&target.url)
                        .with_metadata(index.to_string())
                })
                .collect::<Vec<_>>();

            let ranking = host::rank(&query, &items, Weights::default());
            Ok(QueryResult::SetList(ranking))
        }
    }

    fn activate(&mut self, selected: ListItem) -> Result<impl IntoIterator<Item = Action>> {
        if !selected.metadata.is_empty() {
            let input = self
                .complete(String::new(), selected)?
                .expect("complete should return string as metadata is non empty");
            return Ok(vec![Action::SetInputLine(input)]);
        }

        // actually go to the url
        let url = selected.description;
        Ok(vec![
            Action::Close,
            Action::RunCommand(("xdg-open".to_owned(), vec![url])),
        ])
    }

    fn complete(&mut self, _: String, selected: ListItem) -> Result<Option<InputLine>> {
        // if it has metadata, still typing the prefix.
        // complete the prefix selected.
        if selected.metadata.is_empty() {
            return Ok(None);
        };
        let index: usize = selected
            .metadata
            .parse()
            .expect("metadata should be an int");

        let prefix = &self.urls[index].0;
        Ok(Some(InputLine::new(format!("{prefix} "))))
    }
}

qpmu_api::register!(Open);
