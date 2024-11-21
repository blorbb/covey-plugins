use std::collections::HashMap;

use qpmu_plugin::{rank, Action, ActivationContext, Input, ListItem, Plugin, Result};
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
    async fn new(config: String) -> Result<Self> {
        let urls: HashMap<String, OpenTarget> = toml::from_str(&config)?;
        Ok(Self {
            urls: urls.into_iter().collect(),
        })
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        if let Some((new_query, target)) = self.urls.iter().find_map(|(prefix, target)| {
            query
                .strip_prefix(prefix)
                .and_then(|s| s.strip_prefix(' ')) // require space after
                .map(|new_query| (new_query.trim(), target))
        }) {
            // query matches one of the prefixes in the list
            let search = ListItem::new(format!("Search {} for {}", target.name, new_query))
                .with_description(target.url.replace("%s", new_query));

            Ok(vec![search])
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

            let ranking = rank::rank(&query, &items, rank::Weights::with_history()).await;
            Ok(ranking)
        }
    }

    async fn activate(&self, cx: ActivationContext) -> Result<Vec<Action>> {
        if !cx.item.metadata.is_empty() {
            let input = self
                .complete(cx)
                .await?
                .expect("complete should return string as metadata is non empty");
            return Ok(vec![Action::SetInput(input)]);
        }

        // actually go to the url
        let url = cx.item.description;
        Ok(vec![
            Action::Close,
            Action::RunCommand("xdg-open".to_owned(), vec![url]),
        ])
    }

    async fn complete(
        &self,
        ActivationContext { item, .. }: ActivationContext,
    ) -> Result<Option<Input>> {
        // if it has metadata, still typing the prefix.
        // complete the prefix selected.
        if item.metadata.is_empty() {
            return Ok(None);
        };
        let index: usize = item.metadata.parse().expect("metadata should be an int");

        let prefix = &self.urls[index].0;
        Ok(Some(Input::new(format!("{prefix} "))))
    }
}

fn main() {
    qpmu_plugin::main::<Open>()
}
