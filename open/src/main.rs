use std::collections::HashMap;

use qpmu_plugin::{clone_async, rank, Action, Input, List, ListItem, Plugin, Result};
use serde::Deserialize;

struct Open {
    urls: Vec<(String, OpenTarget)>,
    // list items to show when the prompt does not match any of the url prefixes
    prefix_prompt: Vec<ListItem>,
}

#[derive(Debug, Deserialize)]
struct OpenTarget {
    name: String,
    url: String,
}

impl Plugin for Open {
    async fn new(config: String) -> Result<Self> {
        let urls: HashMap<String, OpenTarget> = toml::from_str(&config)?;
        let prefix_prompt = urls
            .iter()
            .map(|(prefix, target)| {
                ListItem::new(format!("{prefix}: {}", target.name))
                    .with_description(&target.url)
                    // do the same thing as complete
                    .on_activate(clone_async!(
                        #[double]
                        prefix,
                        || Ok(vec![Action::SetInput(Input::new(format!("{prefix} ")))])
                    ))
                    .on_complete(clone_async!(
                        #[double]
                        prefix,
                        || Ok(Some(Input::new(format!("{prefix} "))))
                    ))
            })
            .collect::<Vec<_>>();

        Ok(Self {
            urls: urls.into_iter().collect(),
            prefix_prompt,
        })
    }

    async fn query(&self, query: String) -> Result<List> {
        let matching_target = self.urls.iter().find_map(|(prefix, target)| {
            query
                .strip_prefix(prefix)
                .and_then(|new_query| new_query.strip_prefix(' ')) // require space after
                .map(|new_query| (new_query.trim(), target))
        });

        if let Some((new_query, target)) = matching_target {
            // query matches one of the prefixes in the list
            // show a single item "search .. for .."
            let replaced_url = target.url.replace("%s", new_query);

            let search = ListItem::new(format!("Search {} for {}", target.name, new_query))
                .with_description(replaced_url.clone())
                .on_activate(clone_async!(replaced_url, || {
                    Ok(vec![
                        Action::Close,
                        Action::RunCommand("xdg-open".to_string(), vec![replaced_url]),
                    ])
                }));

            Ok(List::new(vec![search]))
        } else {
            // query doesn't match any prefix in the list: rank the prefixes
            let ranking =
                rank::rank(&query, &self.prefix_prompt, rank::Weights::with_history()).await;
            Ok(List::new(ranking))
        }
    }
}

fn main() {
    qpmu_plugin::main::<Open>()
}
