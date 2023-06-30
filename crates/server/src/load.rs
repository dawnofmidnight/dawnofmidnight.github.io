use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;

use crate::{
    db::{self, ConnectionError},
    thought::{self, InsertError, Thought},
};

pub fn load(config: &Args) -> Result<(), Error> {
    let Args { archive, site } = config;
    let mut thoughts = Vec::new();
    for entry in
        fs::read_dir(archive).map_err(|raw| Error::ReadArchive { path: archive.clone(), raw })?
    {
        let entry = entry.map_err(Error::FailedDirEntry)?;
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|raw| Error::GetFileType { path: path.clone(), raw })?
            .is_file()
        {
            return Err(Error::ExpectedFile(path));
        }
        let source =
            fs::read_to_string(&path).map_err(|raw| Error::ReadFile { path: path.clone(), raw })?;
        let thought =
            process_source(&path, &source).map_err(|raw| Error::ParseFile { path, raw })?;
        thoughts.push(thought);
    }
    let db = db::connect(site).map_err(Error::DbConnect)?;
    thought::insert_many(&db, &thoughts).map_err(Error::Insert)?;
    Ok(())
}

fn process_source(path: &Path, source: &str) -> Result<Thought, ParseError> {
    let Some((header, content)) = source.split_once("~~~") else {
        return Err(ParseError::MissingHeader { path: path.to_owned() });
    };
    let mut slug = None::<String>;
    let mut title = None;
    let mut date = None;
    for line in header.lines() {
        let Some((name, value)) = line.split_once(':') else {
            return Err(ParseError::MalformedHeader { path: path.to_owned(), content: line.trim().to_owned() })
        };
        let (name, value) = (name.trim(), value.trim());
        let var = match name {
            "slug" => &mut slug,
            "title" => &mut title,
            "date" => &mut date,
            _ => {
                return Err(ParseError::UnknownHeader {
                    path: path.to_owned(),
                    name: name.to_owned(),
                })
            }
        };
        match var {
            Some(value) => {
                return Err(ParseError::DuplicatedHeader {
                    path: path.to_owned(),
                    name: name.to_string(),
                    value: value.clone(),
                })
            }
            None => *var = Some(value.to_owned()),
        }
    }
    let Some(slug) = slug else { return Err(ParseError::MissingSlug { path: path.to_owned() }); };
    let Some(title) = title else { return Err(ParseError::MissingTitle { path: path.to_owned() }); };
    let Some(date) = date else { return Err(ParseError::MissingDate { path: path.to_owned() }); };
    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|raw| {
        ParseError::InvalidDate { path: path.to_owned(), value: date.clone(), raw }
    })?;
    let reverie = content.trim().to_owned();
    let html =
        Some(reverie::to_html(&reverie).map(|bytes| String::from_utf8(bytes).unwrap()).map_err(
            |raw| ParseError::InvalidReverie {
                path: path.to_owned(),
                source: reverie.clone(),
                raw,
            },
        )?);
    Ok(Thought { slug, title, date, reverie, html })
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
    ReadArchive { path: PathBuf, raw: io::Error },
    FailedDirEntry(io::Error),
    GetFileType { path: PathBuf, raw: io::Error },
    ExpectedFile(PathBuf),
    ReadFile { path: PathBuf, raw: io::Error },
    ParseFile { path: PathBuf, raw: ParseError },
    DbConnect(ConnectionError),
    Insert(InsertError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadArchive { path, .. } => {
                write!(f, "failed to read archive directory at `{}`", path.display())
            }
            Self::FailedDirEntry(_) => f.write_str("failed to get information for archive entry"),
            Self::GetFileType { path, raw: _ } => {
                write!(f, "failed to get file type of `{}`", path.display())
            }
            Self::ExpectedFile(path) => {
                write!(f, "expected `{}` to be a file, but it was not", path.display())
            }
            Self::ReadFile { path, raw: _ } => {
                write!(f, "failed to read file at `{}`", path.display())
            }
            Self::ParseFile { path, raw: _ } => {
                write!(f, "failed to parse file at `{}`", path.display())
            }
            Self::DbConnect(_) => f.write_str("failed to connect to database"),
            Self::Insert(_) => f.write_str("failed to insert thoughts"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadArchive { path: _, raw }
            | Self::FailedDirEntry(raw)
            | Self::GetFileType { path: _, raw }
            | Self::ReadFile { path: _, raw } => Some(raw),
            Self::ExpectedFile(_) => None,
            Self::ParseFile { path: _, raw } => Some(raw),
            Self::DbConnect(raw) => Some(raw),
            Self::Insert(raw) => Some(raw),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    MissingHeader { path: PathBuf },
    MalformedHeader { path: PathBuf, content: String },
    UnknownHeader { path: PathBuf, name: String },
    DuplicatedHeader { path: PathBuf, name: String, value: String },
    MissingSlug { path: PathBuf },
    MissingTitle { path: PathBuf },
    MissingDate { path: PathBuf },
    InvalidDate { path: PathBuf, value: String, raw: chrono::ParseError },
    InvalidReverie { path: PathBuf, source: String, raw: reverie::Error },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHeader { path } => {
                write!(f, "expected header followed by ~~~ in `{}`", path.display())
            }
            Self::MalformedHeader { path, content } => {
                write!(
                    f,
                    "malformed header in `{}`. expected `name: value`, found `{content}`",
                    path.display()
                )
            }
            Self::UnknownHeader { path, name } => {
                write!(f, "unknown header `{name}` in `{}`", path.display())
            }
            Self::DuplicatedHeader { path, name, value } => {
                write!(
                    f,
                    "duplicated `{name}` header in `{}`. previous value was `{value}`",
                    path.display()
                )
            }
            Self::MissingSlug { path } => {
                write!(f, "missing `slug:` header in `{}`", path.display())
            }
            Self::MissingTitle { path } => {
                write!(f, "missing `title:` header in `{}`", path.display())
            }
            Self::MissingDate { path } => {
                write!(f, "missing `date:` header in `{}`", path.display())
            }
            Self::InvalidDate { path, value, raw: _ } => {
                write!(f, "invalid date in `{}`. value: `{value}`", path.display())
            }
            Self::InvalidReverie { path, source, raw } => {
                let report = raw.into_report();
                let mut buffer = Vec::new();
                report.write(("rev", reverie::Source::from(source)), &mut buffer).unwrap();
                let s = std::str::from_utf8(&buffer).unwrap();
                writeln!(f, "invalid reverie in `{}`", path.display())?;
                f.write_str(s)
            }
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingHeader { path: _ }
            | Self::MalformedHeader { path: _, content: _ }
            | Self::UnknownHeader { path: _, name: _ }
            | Self::DuplicatedHeader { path: _, name: _, value: _ }
            | Self::MissingSlug { path: _ }
            | Self::MissingTitle { path: _ }
            | Self::MissingDate { path: _ }
            | Self::InvalidReverie { path: _, source: _, raw: _ } => None,
            Self::InvalidDate { path: _, value: _, raw } => Some(raw),
        }
    }
}
