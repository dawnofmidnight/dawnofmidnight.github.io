#![deny(clippy::pedantic, rust_2018_idioms)]
#![feature(os_str_display)]

struct Args {
    build: std::path::PathBuf,
    static_: std::path::PathBuf,
}

fn main() -> Result<(), Error> {
    let mut parser = lexopt::Parser::from_env();
    let args = cli::parse(&mut parser).map_err(Error::Cli)?;
    build::prepare(&args).map_err(Error::PrepareBuild)?;
    static_::copy(&args).map_err(Error::Static)?;
    println!("build: {}, static: {}", args.build.display(), args.static_.display());
    Ok(())
}

enum Error {
    PrepareBuild(build::Error),
    Cli(cli::Error),
    Static(static_::Error),
}

// quirk of returning `Result` from `main`, the error uses `Debug`
impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PrepareBuild(e) => write!(f, "(prepare build) {e}"),
            Self::Cli(e) => write!(f, "(cli) {e}"),
            Self::Static(e) => write!(f, "(static) {e}"),
        }
    }
}

impl std::error::Error for Error {}

mod build {
    pub(crate) fn prepare(crate::Args { build, .. }: &crate::Args) -> Result<(), Error> {
        let _ = std::fs::remove_dir_all(build);
        std::fs::create_dir_all(build)
            .map_err(|source| Error::Create { path: build.clone(), source })
    }

    #[derive(Debug)]
    pub(crate) enum Error {
        Create { path: std::path::PathBuf, source: std::io::Error },
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Error::Create { path, source } => {
                    write!(f, "failed to create build directory `{}` ({source})", path.display())
                }
            }
        }
    }

    impl std::error::Error for Error {}
}

mod cli {
    pub(crate) fn parse(parser: &mut lexopt::Parser) -> Result<crate::Args, Error> {
        const OPTION_BUILD: OptionData =
            OptionData { name: "build directory", short: "b", long: "build" };
        const OPTION_STATIC: OptionData =
            OptionData { name: "static file directory", short: "s", long: "stattic" };

        let (mut build, mut static_) = (None, None);
        while let Some(arg) = parser.next().map_err(Error::Lexopt)? {
            match arg {
                lexopt::Arg::Short('b') | lexopt::Arg::Long("build") => {
                    if build.is_some() {
                        return Err(Error::Duplicate(OPTION_BUILD));
                    }
                    build = Some(parser.value().map_err(Error::Lexopt)?.into());
                }
                lexopt::Arg::Short('s') | lexopt::Arg::Long("static") => {
                    if static_.is_some() {
                        return Err(Error::Duplicate(OPTION_STATIC));
                    }
                    static_ = Some(parser.value().map_err(Error::Lexopt)?.into());
                }
                arg @ (lexopt::Arg::Short(_) | lexopt::Arg::Long(_) | lexopt::Arg::Value(_)) => {
                    return Err(Error::Lexopt(arg.unexpected()))
                }
            }
        }
        Ok(crate::Args {
            build: build.unwrap_or(std::path::PathBuf::from("build")),
            static_: static_.unwrap_or(std::path::PathBuf::from("static")),
        })
    }

    #[derive(Debug)]
    pub(crate) struct OptionData {
        name: &'static str,
        short: &'static str,
        long: &'static str,
    }

    #[derive(Debug)]
    pub(crate) enum Error {
        Lexopt(lexopt::Error),
        Duplicate(OptionData),
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Lexopt(e) => write!(f, "{e}"),
                Self::Duplicate(OptionData { name, short, long }) => {
                    write!(f, "duplicate {name}, specified '-{short}' or '--{long}' twice")
                }
            }
        }
    }

    impl std::error::Error for Error {}
}

mod static_ {
    pub(crate) fn copy(crate::Args { build, static_ }: &crate::Args) -> Result<(), Error> {
        let mut work = vec![static_.clone()];
        while let Some(dir) = work.pop() {
            for entry in
                dir.read_dir().map_err(|source| Error::ReadDir { path: dir.clone(), source })?
            {
                let entry =
                    entry.map_err(|source| Error::ReadDirEntry { path: dir.clone(), source })?;
                println!("{}", entry.path().display());
                let new_path = build.join(entry.path().strip_prefix(static_).unwrap());
                match entry
                    .file_type()
                    .map_err(|source| Error::GetType { path: dir.clone(), source })?
                {
                    typ if typ.is_file() => {
                        std::fs::copy(entry.path(), new_path.clone()).map_err(|source| {
                            Error::CopyFile { src: entry.path(), dest: new_path, source }
                        })?;
                    }
                    typ if typ.is_dir() => {
                        std::fs::create_dir_all(&new_path)
                            .map_err(|source| Error::CreateDir { path: new_path, source })?;
                        work.push(entry.path());
                    }
                    _ => return Err(Error::WrongType(entry.path())),
                }
            }
        }
        Ok(())
    }

    #[derive(Debug)]
    pub(crate) enum Error {
        ReadDir { path: std::path::PathBuf, source: std::io::Error },
        ReadDirEntry { path: std::path::PathBuf, source: std::io::Error },
        GetType { path: std::path::PathBuf, source: std::io::Error },
        WrongType(std::path::PathBuf),
        CopyFile { src: std::path::PathBuf, dest: std::path::PathBuf, source: std::io::Error },
        CreateDir { path: std::path::PathBuf, source: std::io::Error },
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::ReadDir { path, source } => {
                    write!(f, "failed to read dir `{}` ({source})", path.display())
                }
                Self::ReadDirEntry { path, source } => {
                    write!(f, "failed to read dir entry in `{}` ({source})", path.display())
                }
                Self::GetType { path, source } => {
                    write!(f, "failed to get file type of `{}` ({source})", path.display())
                }
                Self::WrongType(path) => {
                    write!(f, "`{}` must be a file or directory", path.display())
                }
                Self::CopyFile { src, dest, source } => {
                    write!(
                        f,
                        "failed to copy file from `{}` to `{}` ({source})",
                        src.display(),
                        dest.display()
                    )
                }
                Self::CreateDir { path, source } => {
                    write!(f, "failed to create directory at `{}` ({source})", path.display())
                }
            }
        }
    }

    impl std::error::Error for Error {}
}
