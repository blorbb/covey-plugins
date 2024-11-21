use std::{process::Stdio, sync::LazyLock};

use anyhow::{bail, Result};
use freedesktop_desktop_entry::{self as desktop, DesktopEntry};
use qpmu_api::*;

struct AppSwitcher {
    entries: Vec<ListItem>,
}

fn process_entry(entry: DesktopEntry<'_>, locales: &[impl AsRef<str>]) -> Option<ListItem> {
    // filter out some desktop entries that are probably irrelevant
    // has NoDisplay, or no Icon attribute, or all it's categories are useless.
    if entry.no_display()
        || entry.icon().is_none()
        || entry.categories().is_none_or(|cats| {
            cats.into_iter()
                .filter(|cat| !USELESS_CATEGORIES.contains(cat))
                .filter(|cat| !cat.is_empty())
                .count()
                == 0
        })
    {
        return None;
    }

    // https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html
    // lots of allocations but its a tiny string anyways, doesn't matter
    let exec = entry.parse_exec().ok()?.join(" ");

    let metadata = format!(
        "{}\n{}",
        exec,
        entry.startup_wm_class().unwrap_or(entry.id())
    );

    Some(
        ListItem::new(entry.name(locales)?)
            .with_description(entry.comment(locales).unwrap_or_default())
            .with_metadata(metadata)
            .with_icon(entry.icon()),
    )
}

impl Plugin for AppSwitcher {
    async fn new(_: String) -> Result<Self> {
        let locales = desktop::get_languages_from_env();
        let mut entries = Vec::new();
        for entry in desktop::Iter::new(desktop::default_paths()).entries(Some(&locales)) {
            entries.extend(process_entry(entry, &locales));
        }

        Ok(Self { entries })
    }

    async fn query(&self, query: String) -> Result<Vec<ListItem>> {
        Ok(rank::rank(&query, &self.entries, rank::Weights::with_history()).await)
    }

    async fn activate(
        &self,
        ActivationContext { item, .. }: ActivationContext,
    ) -> Result<Vec<Action>> {
        let (exec_cmd, class) = item.metadata.split_once('\n').unwrap();

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
