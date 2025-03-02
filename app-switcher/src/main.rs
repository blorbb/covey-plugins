use std::{process::Stdio, sync::LazyLock};

use covey_plugin::{Icon, List, ListItem, Plugin, Result, anyhow::bail, clone_async, rank, spawn};
use freedesktop_desktop_entry::{self as desktop, DesktopEntry};

covey_plugin::include_manifest!();

struct AppSwitcher {
    entries: Vec<ListItem>,
}

fn process_entry(entry: DesktopEntry, locales: &[impl AsRef<str>]) -> Option<ListItem> {
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

    let class = entry.startup_wm_class().unwrap_or(entry.id()).to_string();

    Some(
        ListItem::new(entry.name(locales)?)
            .with_description(entry.comment(locales).unwrap_or_default())
            .with_icon(entry.icon().map(|name| Icon::Name(name.to_string())))
            .on_activate(clone_async!(class, exec, |menu| {
                menu.close();
                if class.is_empty() || activate_kdotool(&class).await.is_ok() {
                    spawn::script(exec)?;
                }

                Ok(())
            })),
    )
}

impl Plugin for AppSwitcher {
    type Config = ();

    async fn new(_: ()) -> Result<Self> {
        let locales = desktop::get_languages_from_env();
        let mut entries = Vec::new();
        for entry in desktop::Iter::new(desktop::default_paths()).entries(Some(&locales)) {
            entries.extend(process_entry(entry, &locales));
        }

        Ok(Self { entries })
    }

    async fn query(&self, query: String) -> Result<List> {
        Ok(List::new(
            rank::rank(&query, &self.entries, rank::Weights::with_history()).await,
        ))
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
    covey_plugin::run_server::<AppSwitcher>(env!("CARGO_PKG_NAME"))
}
