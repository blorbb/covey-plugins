use std::{path::PathBuf, sync::LazyLock};

use freedesktop_entry_parser as desktop;
use qpmu_api::{
    anyhow::{bail, Result},
    host, register, Action, Capture, ListItem, Plugin, QueryResult, Weights,
};

struct AppSwitcher {
    entries: Vec<ListItem>,
}

impl Plugin for AppSwitcher {
    fn new(_: String) -> Result<Self> {
        let entries = host::read_dir("/usr/share/applications")?
            .into_iter()
            .map(PathBuf::from)
            .filter_map(|entry| {
                Some((
                    entry.file_stem()?.to_str()?.to_string(),
                    desktop::Entry::parse(host::read_file(entry).ok()?).ok()?,
                ))
            })
            .filter(|(_, entry)| {
                let entry = entry.section("Desktop Entry");

                // NoDisplay=true desktop entries aren't for user use.
                // if there are no icons then it's probably not a user app.
                entry.attr("NoDisplay") != Some("true")
                    && entry.has_attr("Icon")
                    && entry.attr("Categories").is_some_and(|cats| {
                        cats.split_terminator(';')
                            .filter(|cat| !USELESS_CATEGORIES.contains(cat))
                            .count()
                            > 0
                    })
            })
            .filter_map(|(file_stem, entry)| {
                let entry = entry.section("Desktop Entry");
                // https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html
                // lots of allocations but its a tiny string anyways, doesn't matter
                let exec = entry
                    .attr("Exec")?
                    .replace("%u", "")
                    .replace("%U", "")
                    .replace("%f", "")
                    .replace("%F", "")
                    .replace(
                        "%i",
                        &entry
                            .attr("Icon")
                            .map_or_else(String::new, |icon| format!("--icon {icon}")),
                    )
                    .replace("%c", entry.attr("Name").unwrap_or_default())
                    .replace("%k", "");

                let metadata = format!(
                    "{}\n{}",
                    exec,
                    entry.attr("StartupWMClass").unwrap_or(&file_stem)
                );

                Some(
                    ListItem::new(entry.attr("Name")?)
                        .with_description(entry.attr("Comment").unwrap_or_default())
                        .with_metadata(metadata)
                        .with_icon(entry.attr("Icon")),
                )
            })
            .collect();

        Ok(Self { entries })
    }

    fn query(&mut self, query: String) -> Result<QueryResult> {
        Ok(QueryResult::SetList(host::rank(
            &query,
            &self.entries,
            Weights::default(),
        )))
    }

    fn activate(&mut self, selected: ListItem) -> Result<impl IntoIterator<Item = Action>> {
        let (exec_cmd, class) = selected.metadata.split_once('\n').unwrap();

        if !class.is_empty() {
            // try and activate it with kdotool
            if activate_kdotool(class).is_ok() {
                return Ok(vec![Action::Close]);
            }
        }

        Ok(vec![Action::Close, Action::RunShell(exec_cmd.to_string())])
    }
}

register!(AppSwitcher);

/// Tries to open the window using kdotool, returning None if it fails.
fn activate_kdotool(class: &str) -> Result<()> {
    dbg!(class);
    let out = host::spawn(
        &KDOTOOL_PATH,
        &["search", "--limit", "1", "--class", class],
        Capture::STDOUT,
    )?;

    // prints an empty string if nothing matches
    if out.stdout.is_empty() {
        bail!("window not found")
    };

    host::spawn(
        &KDOTOOL_PATH,
        ["windowactivate", String::from_utf8(out.stdout)?.trim()],
        Capture::empty(),
    )?;

    Ok(())
}

const USELESS_CATEGORIES: [&str; 7] = [
    "System",
    "Development",
    "Qt",
    "KDE",
    "GNOME",
    "GTK",
    "Application",
];

static KDOTOOL_PATH: LazyLock<String> = LazyLock::new(|| {
    let mut s = std::env::var("HOME").expect("HOME variable must be set");
    s.push_str("/.cargo/bin/kdotool");
    s
});
