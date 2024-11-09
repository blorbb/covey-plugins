use qpmu_api::{
    anyhow::Result, host, register, Action, Capture, DeferredAction, DeferredResult, InputLine,
    ListItem, Plugin, QueryResult,
};

struct Qalc;

impl Plugin for Qalc {
    fn query(query: String) -> Result<QueryResult> {
        Ok(QueryResult::Defer(DeferredAction::Spawn((
            "qalc".to_string(),
            vec![query],
        ))))
    }

    fn activate(item: ListItem) -> Result<impl IntoIterator<Item = Action>> {
        let Ok(output) = host::spawn("qalc", ["-t", &item.metadata], Capture::STDOUT) else {
            return Ok(vec![]);
        };

        Ok(vec![
            Action::Close,
            Action::Copy(String::from_utf8(output.stdout)?.trim().to_string()),
        ])
    }

    fn handle_deferred(query: String, result: DeferredResult) -> Result<QueryResult> {
        match result {
            DeferredResult::ProcessOutput(output) => Ok(QueryResult::SetList(vec![ListItem::new(
                String::from_utf8(output?.stdout)?.trim().to_string(),
            )
            .with_metadata(query)
            .with_icon(Some("qalculate"))])),
        }
    }

    fn complete(_query: String, selected: ListItem) -> Result<Option<InputLine>> {
        let output = host::spawn("qalc", ["-t", &selected.metadata], Capture::STDOUT);

        Ok(Some(InputLine::new(
            String::from_utf8(output?.stdout)?.trim().to_string(),
        )))
    }
}

register!(Qalc);
