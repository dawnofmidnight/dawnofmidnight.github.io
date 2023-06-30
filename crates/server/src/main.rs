#![feature(lint_reasons, slice_group_by)]

mod build;
mod db;
mod load;
mod store;
mod thought;

use std::{fmt, fs, path::PathBuf, time::Instant};

use clap::{Parser, Subcommand};

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let start = Instant::now();
    match args.command {
        Command::Build(config) => build::build(&config).map_err(Error::Build)?,
        Command::Load(config) => load::load(&config).map_err(Error::Load)?,
        Command::Store(config) => store::store(&config).map_err(Error::Store)?,
        Command::Clean(config) => clean(&config),
    }
    println!("finished in {:?}", start.elapsed());
    Ok(())
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Build(build::Args),
    Load(load::Args),
    Store(store::Args),
    Clean(CleanArgs),
}

enum Error {
    Build(build::Error),
    Load(load::Error),
    Store(store::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(_) => f.write_str("failed to build site"),
            Self::Load(_) => f.write_str("failed to load archive"),
            Self::Store(_) => f.write_str("failed to store archive"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Build(error) => Some(error),
            Self::Load(error) => Some(error),
            Self::Store(error) => Some(error),
        }
    }
}

// for some odd reason, it's the Debug impl that is printed.
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::error::Error as _;
        write!(f, "{self}")?;
        if self.source().is_some() {
            f.write_str("\n\nContext:")?;
            write_source(f, self.source(), 0)?;
        }
        Ok(())
    }
}

fn write_source(
    f: &mut fmt::Formatter<'_>,
    error: Option<&(dyn std::error::Error + 'static)>,
    count: u8,
) -> fmt::Result {
    match error {
        Some(error) => {
            write!(f, "\n    {count}: {error}")?;
            write_source(f, error.source(), count + 1)
        }
        None => Ok(()),
    }
}

fn clean(config: &CleanArgs) {
    let CleanArgs { site } = config;
    let _ = fs::remove_dir_all(site);
}

#[derive(clap::Args)]
struct CleanArgs {
    /// the site directory, which holds the database and the static assets
    #[clap(short, long)]
    site: PathBuf,
}
