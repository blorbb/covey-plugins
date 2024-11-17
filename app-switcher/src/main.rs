use std::{path::PathBuf, process::Stdio, sync::LazyLock};

use anyhow::{bail, Result};
use freedesktop_entry_parser as desktop;
use qpmu_api::*;

struct AppSwitcher {
    entries: Vec<ListItem>,
}

async fn parse_file(path: PathBuf) -> Option<ListItem> {
    let entry =
        desktop::Entry::parse(tokio::fs::read(path.canonicalize().ok()?).await.ok()?).ok()?;
    let entry = entry.section("Desktop Entry");

    // filter out some desktop entries that are probably irrelevant
    // has NoDisplay, or no Icon attribute, or all it's categories are useless.
    if entry.attr("NoDisplay") == Some("true")
        || !entry.has_attr("Icon")
        || entry.attr("Categories").is_none_or(|cats| {
            cats.split_terminator(';')
                .filter(|cat| !USELESS_CATEGORIES.contains(cat))
                .count()
                == 0
        })
    {
        return None;
    }

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
        entry
            .attr("StartupWMClass")
            .unwrap_or(path.file_stem()?.to_str()?)
    );

    Some(
        ListItem::new(entry.attr("Name")?)
            .with_description(entry.attr("Comment").unwrap_or_default())
            .with_metadata(metadata)
            .with_icon(entry.attr("Icon")),
    )
}

impl Plugin for AppSwitcher {
    async fn new(_: String) -> Result<Self> {
        let mut entries = Vec::new();

        let mut read_dir = tokio::fs::read_dir("/usr/share/applications").await?;

        while let Some(file) = read_dir.next_entry().await? {
            entries.extend(parse_file(file.path()).await);
        }

        Ok(Self { entries })
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        Ok(rank::rank(&query, &self.entries, rank::Weights::default()))
    }

    async fn activate(&self, selected: ListItem) -> Result<Vec<Action>> {
        let (exec_cmd, class) = selected.metadata.split_once('\n').unwrap();

        if !class.is_empty() {
            // try and activate it with kdotool
            if activate_kdotool(class).await.is_ok() {
                return Ok(vec![Action::Close]);
            }
        }

        Ok(vec![Action::Close, Action::RunShell(exec_cmd.to_string())])
    }
}

/// Tries to open the window using kdotool, returning `Err` if it fails.
async fn activate_kdotool(class: &str) -> Result<()> {
    let output = tokio::process::Command::new(&*KDOTOOL_PATH)
        .arg("search")
        .arg("--limit")
        .arg("1")
        .arg("--class")
        .arg(class)
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    // prints an empty string if nothing matches
    if output.stdout.is_empty() {
        bail!("window not found")
    };

    let exit = tokio::process::Command::new(&*KDOTOOL_PATH)
        .arg("windowactivate")
        .arg(String::from_utf8(output.stdout)?.trim())
        .spawn()?
        .wait()
        .await?;

    if exit.success() {
        Ok(())
    } else {
        bail!("kdotool failed to activate: {:?}", exit)
    }
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

fn main() {
    qpmu_api::main::<AppSwitcher>()
}
