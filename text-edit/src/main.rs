use std::borrow::Cow;

use base64::{Engine, prelude::BASE64_STANDARD};
use convert_case::{Boundary, Case, Casing};
use covey_plugin::{
    Input, List, ListItem, Plugin, Result, clone_async,
    rank::{self, Weights},
};

covey_plugin::include_manifest!();

struct TextEdit;

impl Plugin for TextEdit {
    type Config = ();

    async fn new(_config: Self::Config) -> Result<Self> {
        Ok(Self)
    }

    async fn query(&self, query: String) -> Result<List> {
        let output = match query.split_once(' ') {
            None => List::new(
                rank::rank(
                    &query,
                    vec![
                        &ListItem::new("case").on_complete(async |menu| {
                            menu.set_input(Input::new("case "));
                            Ok(())
                        }),
                        &ListItem::new("encode").on_complete(async |menu| {
                            menu.set_input(Input::new("encode "));
                            Ok(())
                        }),
                        &ListItem::new("decode").on_complete(async |menu| {
                            menu.set_input(Input::new("decode "));
                            Ok(())
                        }),
                    ],
                    Weights::with_history(),
                )
                .await,
            ),

            Some(("case", arg)) => {
                // boolean is whether it should be considered 'plain text'
                // i.e. only split by spaces.
                // false means it's a programming case, split by
                // the defaults.
                let cases = [
                    (true, Case::Sentence, "Sentence case"),
                    (true, Case::Title, "Title Case"),
                    (true, Case::Lower, "lowercase"),
                    (true, Case::Upper, "UPPERCASE"),
                    (false, Case::Camel, "camelCase"),
                    (false, Case::UpperCamel, "UpperCamelCase"),
                    (false, Case::Snake, "snake_case"),
                    (false, Case::UpperSnake, "UPPER_SNAKE_CASE"),
                    (false, Case::Kebab, "kebab-case"),
                    (false, Case::Flat, "oneword"),
                    (false, Case::UpperFlat, "UPPERONEWORD"),
                    (true, Case::Alternating, "sPoNgEbOb"),
                ];

                List::new(
                    cases
                        .into_iter()
                        .map(|(is_plain, case, name)| {
                            let cased = if is_plain {
                                arg.with_boundaries(&[Boundary::SPACE]).to_case(case)
                            } else {
                                arg.to_case(case)
                            };

                            ListItem::new(cased.clone())
                                .with_description(name)
                                .on_activate(clone_async!(cased, |menu| {
                                    menu.close();
                                    menu.copy(cased);
                                    Ok(())
                                }))
                                .on_complete(clone_async!(cased, |menu| {
                                    menu.set_input(Input::new(format!("case {cased}")));
                                    Ok(())
                                }))
                        })
                        .collect(),
                )
            }

            Some(("encode", arg)) => {
                let b64 = BASE64_STANDARD.encode(arg);
                let url = urlencoding::encode(arg).into_owned();
                let html = html_escape::encode_text(arg).into_owned();
                List::new(vec![
                    ListItem::new(b64.clone())
                        .with_description("base64")
                        .on_activate(clone_async!(b64, |menu| {
                            menu.close();
                            menu.copy(b64);
                            Ok(())
                        })),
                    ListItem::new(url.clone())
                        .with_description("url")
                        .on_activate(clone_async!(url, |menu| {
                            menu.close();
                            menu.copy(url);
                            Ok(())
                        })),
                    ListItem::new(html.clone())
                        .with_description("html")
                        .on_activate(clone_async!(html, |menu| {
                            menu.close();
                            menu.copy(html);
                            Ok(())
                        })),
                ])
            }
            Some(("decode", arg)) => {
                let b64 = BASE64_STANDARD
                    .decode(arg)
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .map_err(|e| e.to_string());
                let url = urlencoding::decode(arg)
                    .map(Cow::into_owned)
                    .map_err(|e| e.to_string());
                let html = Ok(html_escape::decode_html_entities(arg).into_owned());

                // reorder so that any errors will show up at the bottom

                let (oks, errs): (Vec<_>, Vec<_>) = [(b64, "base64"), (url, "url"), (html, "html")]
                    .map(|(result, kind)| ((result.clone().ok(), kind), (result.err(), kind)))
                    .into_iter()
                    .unzip();

                let items = oks
                    .into_iter()
                    .chain(errs)
                    .filter_map(|(title, format)| {
                        title.map(|a| ListItem::new(a).with_description(format))
                    })
                    .collect();
                List::new(items)
            }

            Some((other, _)) => List::new(vec![ListItem::new(format!(
                "Error: unknown subcommand {other}"
            ))]),
        };

        Ok(output)
    }
}

fn main() {
    covey_plugin::run_server::<TextEdit>(env!("CARGO_PKG_NAME"));
}
