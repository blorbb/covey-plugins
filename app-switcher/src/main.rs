use std::{process::Stdio, sync::LazyLock};

use covey_plugin::{
    Icon, List, ListItem, Plugin, Result,
    anyhow::{Context, anyhow, bail},
    clone_async, rank, spawn,
};
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

    // entry.parse_exec() doesn't parse correctly (quoted args with spaces inside).
    let exec = parse_exec(&entry, locales)
        .context("failed to parse app Exec")
        .map_err(|e| format!("{e:#}"));
    let class = entry.startup_wm_class().unwrap_or(entry.id()).to_string();

    Some(
        ListItem::new(entry.name(locales)?)
            .with_description(entry.comment(locales).unwrap_or_default())
            .with_icon(entry.icon().map(|name| Icon::Name(name.to_string())))
            .on_activate(clone_async!(class, exec, |menu| {
                menu.close();
                if class.is_empty() || activate_kdotool(&class).await.is_err() {
                    let exec = exec.map_err(|s| anyhow!(s))?;
                    let (program, args) = exec.split_first().context("missing Exec command")?;
                    spawn::command(program, args)?;
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

/// Parses the Exec entry mostly according to
/// https://specifications.freedesktop.org/desktop-entry/latest/exec-variables.html
fn parse_exec(entry: &DesktopEntry, locales: &[impl AsRef<str>]) -> Result<Vec<String>> {
    let exec = shlex::split(entry.exec().context("missing Exec key")?)
        .context("failed to parse Exec key")?;

    let mut parsed_exec = Vec::new();

    for arg in exec {
        if !arg.contains('%') {
            parsed_exec.push(arg);
            continue;
        }

        let mut parsed_arg = String::new();
        let mut after_percent = false;
        for char in arg.chars() {
            if after_percent {
                match char {
                    // %% -> %
                    '%' => parsed_arg.push('%'),
                    // ignore extra files or uris
                    'f' | 'F' | 'u' | 'U' => {}
                    // ignore deprecated
                    'd' | 'D' | 'n' | 'N' | 'v' | 'm' => {}
                    // icon -- I don't understand what the spec means so ignoring
                    'i' => {}
                    // translated name of the application
                    'c' => parsed_arg.push_str(&entry.name(locales).context("missing Name key")?),
                    // location of the desktop file
                    'k' => parsed_arg.push_str(
                        entry
                            .path
                            .to_str()
                            .context("desktop file path is not UTF-8")?,
                    ),
                    _ => bail!("unknown field code %{char}"),
                }
            } else {
                match char {
                    '%' => after_percent = true,
                    _ => parsed_arg.push(char),
                }
            }
        }

        // for something like `firefox %f`, should become `firefox` not `firefox ''`.
        if !parsed_arg.is_empty() {
            parsed_exec.push(parsed_arg);
        }
    }

    Ok(parsed_exec)
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
