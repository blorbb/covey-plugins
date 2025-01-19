# comette Open plugin

Opens URLs.

## Config format

```toml
[[plugins]]
name = "open"
prefix = "@"

[plugins.config.urls]
cr = { name = "crates docs", url = "https://docs.rs/%s" }
std = { name = "Rust stdlib", url = "https://doc.rust-lang.org/std/?search=%s" }
g = { name = "Google", url = "https://www.google.com/search?q=%s" }
```

Requires a table with the type `<key> = { name = ..., url = ... }`.

The left is the prefix used to search with this search engine, the `name` is a longer description of the engine, and the `url` is the URL to use. Replaces `%s` with your query.

Currently opens with `xdg-open`.

TODO: support windows/macos.
