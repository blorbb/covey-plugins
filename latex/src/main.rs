use anyhow::Result;
use qpmu_api::*;

// mapping from
// https://github.com/joom/latex-unicoder.vim/blob/master/autoload/unicoder.vim
// with ',' removed for simpler parsing.

struct Latex {
    info: Vec<ListItem>,
}

impl Plugin for Latex {
    async fn new(_: String) -> Result<Self> {
        let info: Vec<_> = include_str!("../mapping.csv")
            .lines()
            .map(|line| {
                line.split_once(',')
                    .unwrap_or_else(|| panic!("failed to split line {line}"))
            })
            .map(|(latex, unicode)| ListItem::new(unicode).with_description(latex))
            .collect();

        Ok(Latex { info })
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        let ranking = rank::rank(
            &query,
            &self.info,
            rank::Weights::default().title(0.0).description(1.0),
        )
        .into_iter()
        .take(100)
        .collect();

        Ok(ranking)
    }

    async fn activate(&self, selected: ListItem) -> Result<Vec<Action>> {
        Ok(vec![Action::Close, Action::Copy(selected.title)])
    }

    async fn complete(&self, _: String, selected: ListItem) -> Result<Option<InputLine>> {
        Ok(Some(InputLine::new(selected.description)))
    }
}

fn main() {
    qpmu_api::main::<Latex>();
}
