use std::{
    error::Error,
    fmt,
    fs,
    io,
    path::{Path, PathBuf},
};

use rusqlite::Connection;

pub fn connect(site: &Path) -> Result<Connection, ConnectionError> {
    fs::create_dir_all(site)
        .map_err(|raw| ConnectionError::CreateSite { path: site.to_owned(), raw })?;
    let path = site.join("reverie.db");
    let db = Connection::open(&path).map_err(|raw| ConnectionError::Open { path, raw })?;
    db.pragma_update(None, "journal_mode", "WAL").map_err(ConnectionError::SetWal)?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS thoughts (
            slug TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            date TEXT NOT NULL,
            reverie TEXT NOT NULL,
            html TEXT
        )",
        (),
    )
    .map_err(ConnectionError::CreateThoughts)?;
    Ok(db)
}

#[derive(Debug)]
pub enum ConnectionError {
    CreateSite { path: PathBuf, raw: io::Error },
    Open { path: PathBuf, raw: rusqlite::Error },
    SetWal(rusqlite::Error),
    CreateThoughts(rusqlite::Error),
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSite { path, raw: _ } => {
                write!(
                    f,
                    "failed to create directory for reverie artifacts at `{}`",
                    path.display()
                )
            }
            Self::Open { path, raw: _ } => {
                write!(f, "failed to open database at `{}`", path.display())
            }
            Self::SetWal(_) => f.write_str("failed to enable write-ahead logging"),
            Self::CreateThoughts(_) => f.write_str("failed to create `thoughts` table"),
        }
    }
}

impl Error for ConnectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateSite { path: _, raw } => Some(raw),
            Self::Open { path: _, raw } | Self::SetWal(raw) | Self::CreateThoughts(raw) => {
                Some(raw)
            }
        }
    }
}
