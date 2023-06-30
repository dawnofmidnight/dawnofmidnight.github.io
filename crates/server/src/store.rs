use std::{fs, io, path::PathBuf};

use crate::{
    db::{self, ConnectionError},
    thought::{self, SelectError, Thought},
};

pub fn store(config: &Args) -> Result<(), Error> {
    let Args { archive, site } = config;
    let db = db::connect(site).map_err(Error::DbConnect)?;
    let thoughts = thought::select_all(&db).map_err(Error::Select)?;
    let _ = fs::remove_dir_all(archive);
    fs::create_dir_all(archive)
        .map_err(|raw| Error::CreateArchive { path: archive.clone(), raw })?;
    for thought in thoughts {
        let Thought { slug, title, date, reverie, html: _ } = thought;
        let path = archive.join(&slug).with_extension("rev");
        let contents = format!(
            "slug: {slug}\ntitle: {title}\ndate: {}\n~~~\n\n{reverie}\n",
            date.format("%Y-%m-%d")
        );
        fs::write(&path, contents).map_err(|raw| Error::WriteThought { path, raw })?;
    }
    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// The path to the archive directory.
    #[clap(short, long)]
    archive: PathBuf,

    /// the site directory, which holds the database and the static assets
    #[clap(short, long)]
    site: PathBuf,
}

#[derive(Debug)]
pub enum Error {
    DbConnect(ConnectionError),
    Select(SelectError),
    CreateArchive { path: PathBuf, raw: io::Error },
    WriteThought { path: PathBuf, raw: io::Error },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DbConnect(_) => f.write_str("failed to connect to database"),
            Self::Select(_) => f.write_str("failed to select thoughts"),
            Self::CreateArchive { path, raw: _ } => {
                write!(f, "failed to create archive directory at `{}`", path.display())
            }
            Self::WriteThought { path, raw: _ } => {
                write!(f, "failed to write thought to `{}`", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DbConnect(raw) => Some(raw),
            Self::Select(raw) => Some(raw),
            Self::CreateArchive { path: _, raw } | Self::WriteThought { path: _, raw } => Some(raw),
        }
    }
}
