use covey_plugin::{Action, Input, List, ListItem, Plugin, Result, clone_async, rank};

// mapping from
// https://github.com/joom/latex-unicoder.vim/blob/master/autoload/unicoder.vim
// with ',' removed for simpler parsing.

covey_plugin::include_manifest!();

struct Latex {
    info: Vec<ListItem>,
}

impl Plugin for Latex {
    type Config = ();

    async fn new(_: ()) -> Result<Self> {
        let info: Vec<_> = include_str!("../mapping.csv")
            .lines()
            .map(|line| {
                line.split_once(',')
                    .unwrap_or_else(|| panic!("failed to split line {line}"))
            })
            .map(|(latex, unicode)| {
                ListItem::new(latex)
                    .with_icon_text(unicode)
                    .on_activate(clone_async!(unicode, || {
                        Ok([Action::close(), Action::copy(unicode)])
                    }))
                    .on_complete(clone_async!(|| Ok(Input::new(unicode))))
            })
            .collect();

        Ok(Latex { info })
    }

    async fn query(&self, query: String) -> Result<List> {
        let ranking: Vec<_> = rank::rank(&query, &self.info, rank::Weights::with_history())
            .await
            .into_iter()
            .take(100)
            .collect();

        Ok(List::new(ranking).as_grid())
    }
}

fn main() {
    covey_plugin::run_server::<Latex>(env!("CARGO_PKG_NAME"));
}
