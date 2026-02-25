use std::{
    cmp::Reverse,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, mpsc},
    time::Instant,
};

use covey_plugin::{
    Input, List, ListItem, PluginBlocking, Result, anyhow::Context, clone_async, spawn,
};
use ignore::WalkState;
use nucleo_matcher::{
    Matcher, Utf32String,
    pattern::{CaseMatching, Normalization, Pattern},
};
use rayon::prelude::*;

covey_plugin::include_manifest!();

struct DirContents {
    dir: PathBuf,
    // All sub-dirs as well if true, otherwise just the immediate children of `dir`.
    recursive: bool,
    // excludes the `dir` prefix.
    contents: Arc<[String]>,
}

struct Find {
    cache: RwLock<Option<DirContents>>,
    // Where all search queries start from
    root: PathBuf,
}

impl Find {
    fn query_dir_to_path(&self, search_dir: &str) -> PathBuf {
        self.root.join(search_dir)
    }

    /// Gets the contents of the given `search_dir`, using the cache value if it
    /// exists, otherwise recalculating and writing to the cache.
    fn get_dir_contents(&self, absolute_dir: PathBuf, recursive: bool) -> Result<Arc<[String]>> {
        if let Some(cache) = &*self.cache.read().unwrap()
            && cache.dir == absolute_dir
            && cache.recursive == recursive
        {
            eprintln!("retrieved from cache");
            Ok(Arc::clone(&cache.contents))
        } else if recursive {
            eprintln!("RECURSIVE recompute");
            let start = Instant::now();

            let to_search = std::thread::scope(|s| {
                let (tx, rx) = mpsc::channel();
                let absolute_dir = absolute_dir.clone();
                s.spawn(move || {
                    ignore::WalkBuilder::new(&absolute_dir)
                        .build_parallel()
                        .run(|| {
                            let tx = tx.clone();
                            let absolute_dir = absolute_dir.clone();
                            Box::new(move |dir| {
                                let Ok(dir) = dir else { return WalkState::Skip };
                                let Some(mut path) = dir
                                    .path()
                                    .strip_prefix(&absolute_dir)
                                    .unwrap()
                                    .to_str()
                                    .map(str::to_owned)
                                else {
                                    return WalkState::Skip;
                                };

                                if path.is_empty() {
                                    return WalkState::Continue;
                                };

                                if dir.file_type().is_some_and(|t| t.is_dir()) {
                                    path.push('/');
                                }

                                tx.send(path).unwrap();

                                WalkState::Continue
                            })
                        })
                });

                rx.iter().collect()
            });

            eprintln!("RECOMPUTE took {:?}", start.elapsed());

            *self.cache.write().unwrap() = Some(DirContents {
                dir: absolute_dir,
                recursive: true,
                contents: Arc::clone(&to_search),
            });

            Ok(to_search)
        } else {
            eprintln!("FLAT recompute");
            let to_search = std::fs::read_dir(&absolute_dir)?
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let mut file_name = entry.file_name().into_string().ok()?;
                    if entry.file_type().ok()?.is_dir() {
                        file_name.push('/');
                    }
                    Some(file_name)
                })
                .collect();

            *self.cache.write().unwrap() = Some(DirContents {
                dir: absolute_dir,
                recursive: false,
                contents: Arc::clone(&to_search),
            });

            Ok(to_search)
        }
    }

    fn paths_to_list(
        &self,
        search_dir: &str,
        mut ranking: Vec<(&String, u32)>,
        pattern: &str,
    ) -> List {
        ranking.par_iter_mut().for_each(|(path, score)| {
            if path.ends_with('/') {
                *score += 1;
                *score = ((*score as f64) * 1.1) as u32;
            }
            let slashes = path.chars().filter(|c| *c == '/').count();
            *score += 5u32.saturating_sub(slashes as u32) * 10;
        });

        // By reverse score, then alphabetically
        ranking.sort_by_key(|(path, score)| (Reverse(*score), *path));

        let absolute_search_dir = self.query_dir_to_path(search_dir);

        let list = ranking
            .into_par_iter()
            .take(100)
            .map(|(path, score)| {
                ListItem::new(format!("{path}    ({score})"))
                    // navigates to the directory of the selected item
                    .on_complete(clone_async!(search_dir, path, |menu| {
                        let path_without_file = path.trim_end_matches(|c| c != '/');
                        let new_path = format!("/{search_dir}{path_without_file}");
                        menu.set_input(Input::new(new_path));
                        Ok(())
                    }))
                    .on_activate(clone_async!(absolute_search_dir, path, |menu| {
                        menu.close();
                        spawn::program("xdg-open", &[absolute_search_dir.join(path)])?;
                        Ok(())
                    }))
                    // TODO: need list-wide shortcuts. this doesn't work
                    // if there are no output list items!
                    .on_parent_dir(clone_async!(search_dir, pattern, |menu| {
                        let parent_dir = Path::new(&search_dir)
                            .parent()
                            .unwrap_or(Path::new(&search_dir))
                            .to_str()
                            .unwrap();
                        let with_suffix = if parent_dir.is_empty() || parent_dir == "/" {
                            format!("{parent_dir}")
                        } else {
                            format!("{parent_dir}/")
                        };
                        menu.set_input(Input::new(format!("/{with_suffix} {pattern}",)));
                        Ok(())
                    }))
            })
            .collect();

        List::new(list)
    }

    fn find_in_children(&self, search_dir: &str, pattern: &str, recursive: bool) -> Result<List> {
        dbg!(&search_dir, &pattern);
        let to_search = self.get_dir_contents(self.query_dir_to_path(search_dir), recursive)?;

        let nucleo_pattern = nucleo_matcher::pattern::Pattern::parse(
            pattern,
            CaseMatching::Smart,
            Normalization::Smart,
        );
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT.match_paths());

        let start = Instant::now();
        let ranked = parallel_match(nucleo_pattern, &mut matcher, &to_search);
        eprintln!("RANKING took {:?}", start.elapsed());

        let start = Instant::now();
        let list = self.paths_to_list(search_dir, ranked, pattern);
        eprintln!("RESORTING took {:?}", start.elapsed());

        Ok(list)
    }
}

fn parallel_match<'a>(
    pattern: Pattern,
    matcher: &mut Matcher,
    items: &'a [String],
) -> Vec<(&'a String, u32)> {
    if pattern.atoms.is_empty() {
        return items.into_iter().map(|item| (item, 0)).collect();
    }
    // TODO: reuse a buffer instead of using Utf32String.
    let items: Vec<_> = items
        .into_par_iter()
        .filter_map(|item| {
            pattern
                .score(
                    Utf32String::from(item.as_ref()).slice(..),
                    &mut matcher.clone(),
                )
                .map(|score| (item, score))
        })
        .collect();
    items
}

impl PluginBlocking for Find {
    type Config = Config;

    async fn new(_: Self::Config) -> Result<Self> {
        Ok(Find {
            cache: RwLock::new(None),
            root: dirs::home_dir().context("could not find home directory")?,
        })
    }

    fn query(&self, mut query: String) -> Result<List> {
        // Normalise the query to "some/thing/" for relative to home dir,
        // or "/some/thing/" for relative to root dir, or "" if empty.
        if query.starts_with('/') {
            query.remove(0);
        };

        if let Some((search_dir, pattern)) = query.split_once("/ ") {
            let search_dir = format!("{search_dir}/");
            Ok(self
                .find_in_children(&search_dir, pattern, true)
                .inspect_err(|e| eprintln!("{e:#}"))
                .unwrap_or(List::new(vec![])))
        } else if let Some(pattern) = query.strip_prefix(' ') {
            Ok(self
                .find_in_children("", pattern, true)
                .inspect_err(|e| eprintln!("{e:#}"))
                .unwrap_or(List::new(vec![])))
        } else {
            // x    -> ""    "x"   (want search dir to be "")
            // x/   -> "x"   ""    (want search dir to be "x/")
            // /x   -> ""    "x"   (want search dir to be "/")
            // x/a  -> "x"   "a"   (want search dir to be "x/")
            // /x/a -> "/x"  "a"   (want search dir to be "/x/")
            let (search_dir, pattern) = query.rsplit_once('/').unwrap_or(("", &query));

            let search_dir = if query.contains('/') {
                format!("{search_dir}/")
            } else {
                format!("{search_dir}")
            };
            // search dir should start with "/" IFF we query from the root.
            // otherwise, it should not start with "/" and we query from home dir.
            // it will also always end in /

            Ok(self
                .find_in_children(&search_dir, pattern, false)
                .inspect_err(|e| eprintln!("{e:#}"))
                .unwrap_or(List::new(vec![])))
        }
    }
}

fn main() {
    covey_plugin::run_server_blocking::<Find>(env!("CARGO_PKG_NAME"))
}
