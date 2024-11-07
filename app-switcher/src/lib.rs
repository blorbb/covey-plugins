use std::{cmp, fs, sync::LazyLock};

use freedesktop_entry_parser as desktop;
use qpmu_api::{
    export,
    host::{self, Capture},
    ListItem, Plugin, PluginAction,
};

const USELESS_CATEGORIES: [&str; 7] = [
    "System",
    "Development",
    "Qt",
    "KDE",
    "GNOME",
    "GTK",
    "Application",
];

static ENTRIES: LazyLock<Vec<ListItem>> = LazyLock::new(|| {
    let Ok(entries) = fs::read_dir("/usr/share/applications") else {
        return vec![];
    };

    entries
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            Some((
                entry.path().file_stem()?.to_str()?.to_string(),
                desktop::parse_entry(entry.path()).ok()?,
            ))
        })
        .filter(|(_, entry)| {
            // NoDisplay=true desktop entries aren't for user use.
            entry.section("Desktop Entry").attr("NoDisplay") != Some("true")
                && entry
                    .section("Desktop Entry")
                    .attr("Categories")
                    .is_some_and(|cats| {
                        cats.split_terminator(';')
                            .filter(|cat| !USELESS_CATEGORIES.contains(cat))
                            .count()
                            > 0
                    })
        })
        .filter_map(|(file_stem, entry)| {
            let metadata = format!(
                "{}\n{}",
                entry
                    .section("Desktop Entry")
                    .attr("Exec")?
                    .replace("%u", "")
                    .replace("%U", ""),
                entry
                    .section("Desktop Entry")
                    .attr("StartupWMClass")
                    .unwrap_or(&file_stem)
            );

            Some(
                ListItem::new(entry.section("Desktop Entry").attr("Name")?)
                    .with_description(
                        entry
                            .section("Desktop Entry")
                            .attr("Comment")
                            .unwrap_or_default(),
                    )
                    .with_metadata(metadata),
            )
        })
        .collect()
});

static KDOTOOL_PATH: LazyLock<String> = LazyLock::new(|| {
    let mut s = std::env::var("HOME").expect("HOME variable must be set");
    s.push_str("/.cargo/bin/kdotool");
    s
});

struct AppSwitcher;

/// Tries to open the window using kdotool, returning None if it fails.
fn activate_kdotool(class: &str) -> Option<()> {
    dbg!(class);
    let out = host::spawn(
        &KDOTOOL_PATH,
        &["search", "--limit", "1", "--class", class],
        Capture::STDOUT,
    )
    .ok()?;

    // prints an empty string if nothing matches
    if out.stdout.is_empty() {
        return None;
    };

    host::spawn(
        &KDOTOOL_PATH,
        ["windowactivate", String::from_utf8(out.stdout).ok()?.trim()],
        Capture::empty(),
    )
    .ok()?;

    Some(())
}

impl Plugin for AppSwitcher {
    fn input(query: String) -> Vec<ListItem> {
        let entries = ENTRIES.clone();
        let mut entries: Vec<_> = entries
            .into_iter()
            // filter out anything that doesn't even closely match
            .filter_map(|li| Some((sublime_fuzzy::best_match(&query, &li.title)?.score(), li)))
            .collect();

        entries.sort_unstable_by_key(|(score, _)| cmp::Reverse(*score));

        entries.into_iter().map(|(_, li)| li).collect()
    }

    fn activate(selected: ListItem) -> Vec<PluginAction> {
        let (exec_cmd, class) = selected.metadata.split_once('\n').unwrap();
        if !class.is_empty() {
            // try and activate it with kdotool
            if activate_kdotool(class).is_some() {
                return vec![PluginAction::Close];
            }
        }
        vec![
            PluginAction::Close,
            PluginAction::RunCommandString(exec_cmd.to_string()),
        ]
    }
}

export!(AppSwitcher with_types_in qpmu_api::bindings);
