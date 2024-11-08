use qpmu_api::{
    anyhow::Result, host, register, Capture, DeferredAction, DeferredResult, ListItem, Plugin,
    PluginAction, QueryResult,
};

struct Qalc;

impl Plugin for Qalc {
    fn query(query: String) -> Result<QueryResult> {
        Ok(QueryResult::Defer(DeferredAction::Spawn((
            "qalc".to_string(),
            vec![query],
        ))))
    }

    fn activate(item: ListItem) -> Result<impl IntoIterator<Item = PluginAction>> {
        let Ok(output) = host::spawn("qalc", ["-t", &item.metadata], Capture::STDOUT) else {
            return Ok(vec![]);
        };

        Ok(vec![
            PluginAction::Close,
            PluginAction::Copy(String::from_utf8(output.stdout)?.trim().to_string()),
        ])
    }

    fn handle_deferred(_query: String, result: DeferredResult) -> Result<QueryResult> {
        match result {
            DeferredResult::ProcessOutput(output) => Ok(QueryResult::SetList(vec![ListItem::new(
                String::from_utf8(output?.stdout)?.trim().to_string(),
            )
            .with_icon(Some("qalculate"))])),
        }
    }
}

register!(Qalc);
