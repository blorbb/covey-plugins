use anyhow::Result;
use qpmu_api::*;

// mapping from
// https://github.com/joom/latex-unicoder.vim/blob/master/autoload/unicoder.vim
// with ',' removed for simpler parsing.

struct Latex {
    info: Vec<ListItem>,
}

impl Plugin for Latex {
    fn new(_: String) -> Result<Self> {
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

    fn query(&mut self, query: String) -> Result<QueryResult> {
        let ranking = host::rank(
            &query,
            &self.info,
            Weights::default().title(0.0).description(1.0),
        )
        .into_iter()
        .take(100)
        .collect();

        Ok(QueryResult::SetList(ranking))
    }

    fn activate(&mut self, selected: ListItem) -> Result<impl IntoIterator<Item = Action>> {
        Ok([Action::Close, Action::Copy(selected.title)])
    }

    fn complete(&mut self, _: String, selected: ListItem) -> Result<Option<InputLine>> {
        Ok(Some(InputLine::new(selected.description)))
    }
}

register!(Latex);
