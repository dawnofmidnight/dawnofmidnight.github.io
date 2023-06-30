use std::{fs, io, path::PathBuf};

use chrono::Datelike;
use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    db::{self, ConnectionError},
    thought::{self, SelectError, Thought},
};

pub fn build(config: &Args) -> Result<(), Error> {
    let Args { site, build } = config;
    let db = db::connect(site).map_err(Error::DbConnect)?;
    let mut thoughts = thought::select_all(&db).map_err(Error::Select)?;
    let _ = fs::remove_dir_all(build);
    fs::create_dir_all(build.join("thoughts"))
        .map_err(|raw| Error::CreateBuild { path: build.clone(), raw })?;
    let css_path = build.join("index.css");
    fs::write(&css_path, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/styles.css")))
        .map_err(|raw| Error::WriteCss { path: css_path, raw })?;
    for thought in &thoughts {
        let path = build.join("thoughts").join(&thought.slug).with_extension("html");
        let contents = thought_page(thought).into_string();
        fs::write(&path, contents).map_err(|raw| Error::WriteThought { path, raw })?;
    }

    let thoughts_path = build.join("thoughts").with_extension("html");
    let contents = thoughts_page(&mut thoughts).into_string();
    fs::write(&thoughts_path, contents)
        .map_err(|raw| Error::WriteThought { path: thoughts_path, raw })?;

    let index_path = build.join("index").with_extension("html");
    let contents = index_page().into_string();
    fs::write(&index_path, contents)
        .map_err(|raw| Error::WriteThought { path: index_path, raw })?;
    Ok(())
}

fn index_page() -> Markup {
    base_page(
        "dawnofmidnight",
        html! {
            aside { "The internet must have a " em { "lot" } " of corners.  I hope it's because it's a weird, magical shape and not because it's filled with tiny cubicles." }
            p { "Hi. I'm dawn, and this is my little corner of the internet." }
        },
    )
}

fn thoughts_page(thoughts: &mut [Thought]) -> Markup {
    thoughts.sort_unstable_by_key(|thought| std::cmp::Reverse(thought.date));
    let years = thoughts.group_by(|a, b| a.date.year() == b.date.year());
    base_page(
        "Thoughts",
        html! {
            div class="thoughts" {
                p { "An index of all the posts here, grouped by year." }
                @for thoughts in years {
                    h2 { (thoughts[0].date.year()) }
                    ul {
                        @for thought in thoughts {
                            li {
                                (thought.date.format("%Y-%m-%d"))
                                " — "
                                a href=(format!("/thoughts/{}.html", thought.slug)) { (thought.title) }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn thought_page(thought: &Thought) -> Markup {
    base_page(
        &thought.title,
        html! {
            p class="date" { (thought.date.format("%Y-%m-%d")) }
            (PreEscaped(thought.html.as_deref().expect("page was inserted into DB without compiled reverie")))
        },
    )
}

#[expect(clippy::needless_pass_by_value)]
fn base_page(title: &str, html: Markup) -> Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset = "utf-8";
            title { (title) }
            link rel="stylesheet" href="/index.css";
        }
        body {
            header {
                div .home { a href="/" { "dawn" } }
                nav {
                    a href="/thoughts.html" { "thoughts" }
                    a href="/rss.xml" { "rss" }
                }
            }
            main {
                h1 { (title) }
                (html)
            }
        }
    }
}

#[derive(clap::Args)]
pub struct Args {
    /// the site directory, which holds the database and the static assets
    #[clap(short, long)]
    site: PathBuf,

    /// The build directory, which holds the final output.
    #[clap(short, long)]
    build: PathBuf,
}

#[derive(Debug)]
pub enum Error {
    DbConnect(ConnectionError),
    Select(SelectError),
    CreateBuild { path: PathBuf, raw: io::Error },
    WriteCss { path: PathBuf, raw: io::Error },
    WriteThought { path: PathBuf, raw: io::Error },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DbConnect(_) => f.write_str("failed to connect to database"),
            Self::Select(_) => f.write_str("failed to select thoughts"),
            Self::CreateBuild { path, raw: _ } => {
                write!(f, "failed to create build directory at `{}`", path.display())
            }
            Self::WriteCss { path, raw: _ } => {
                write!(f, "failed to write css to `{}`", path.display())
            }
            Self::WriteThought { path, raw: _ } => {
                write!(f, "failed to write thouht to `{}`", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DbConnect(raw) => Some(raw),
            Self::Select(raw) => Some(raw),
            Self::CreateBuild { path: _, raw }
            | Self::WriteCss { path: _, raw }
            | Self::WriteThought { path: _, raw } => Some(raw),
        }
    }
}
