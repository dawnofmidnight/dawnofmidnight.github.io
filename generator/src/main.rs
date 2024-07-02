#![deny(clippy::pedantic, rust_2018_idioms)]
#![feature(os_str_display)]

struct Args {
    build: std::path::PathBuf,
    static_: std::path::PathBuf,
    content: std::path::PathBuf,
}

fn main() -> Result<(), Error> {
    let start = std::time::Instant::now();
    let mut parser = lexopt::Parser::from_env();
    let args = cli::parse(&mut parser).map_err(Error::Cli)?;
    build_dir::prepare(&args).map_err(Error::BuildDir)?;
    static_::copy(&args).map_err(Error::Static)?;
    content::build(&args).map_err(Error::Content)?;
    println!("finished building to `{}` in {:?}", args.build.display(), start.elapsed());
    Ok(())
}

enum Error {
    BuildDir(build_dir::Error),
    Cli(cli::Error),
    Static(static_::Error),
    Content(content::Error),
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
            Self::BuildDir(e) => write!(f, "(prepare build) {e}"),
            Self::Cli(e) => write!(f, "(cli) {e}"),
            Self::Static(e) => write!(f, "(static) {e}"),
            Self::Content(e) => write!(f, "(content) {e}"),
        }
    }
}

impl std::error::Error for Error {}

mod cli {
    pub(crate) fn parse(parser: &mut lexopt::Parser) -> Result<crate::Args, Error> {
        const OPTION_BUILD: OptionData =
            OptionData { name: "build directory", short: "b", long: "build" };
        const OPTION_STATIC: OptionData =
            OptionData { name: "static file directory", short: "s", long: "static" };
        const OPTION_CONTENT: OptionData =
            OptionData { name: "content directory", short: "c", long: "content" };

        let (mut build, mut static_, mut content) = (None, None, None);
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
                lexopt::Arg::Short('c') | lexopt::Arg::Long("content") => {
                    if content.is_some() {
                        return Err(Error::Duplicate(OPTION_CONTENT));
                    }
                    content = Some(parser.value().map_err(Error::Lexopt)?.into());
                }
                arg @ (lexopt::Arg::Short(_) | lexopt::Arg::Long(_) | lexopt::Arg::Value(_)) => {
                    return Err(Error::Lexopt(arg.unexpected()))
                }
            }
        }
        Ok(crate::Args {
            build: build.unwrap_or(std::path::PathBuf::from("build")),
            static_: static_.unwrap_or(std::path::PathBuf::from("static")),
            content: content.unwrap_or(std::path::PathBuf::from("content")),
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

mod build_dir {
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
                Self::Create { path, source } => {
                    write!(f, "failed to create build directory `{}` ({source})", path.display())
                }
            }
        }
    }

    impl std::error::Error for Error {}
}

mod static_ {
    pub(crate) fn copy(crate::Args { build, static_, .. }: &crate::Args) -> Result<(), Error> {
        crate::dir::iter(static_, build, None, |src, dest| {
            std::fs::copy(&src, &dest).map_err(|source| CopyError { src, dest, source })?;
            Ok(())
        })
    }

    pub(crate) type Error = crate::dir::Error<CopyError>;

    #[derive(Debug)]
    pub(crate) struct CopyError {
        src: std::path::PathBuf,
        dest: std::path::PathBuf,
        source: std::io::Error,
    }

    impl core::fmt::Display for CopyError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let Self { src, dest, source } = self;
            write!(
                f,
                "failed to copy file from `{}` to `{}` ({source})",
                src.display(),
                dest.display()
            )
        }
    }

    impl std::error::Error for CopyError {}
}

mod content {
    use jotdown::{Container, Event};

    pub(crate) fn build(crate::Args { build, content, .. }: &crate::Args) -> Result<(), Error> {
        crate::dir::iter(content, build, Some("html"), |src, dest| {
            let source = std::fs::read_to_string(&src)
                .map_err(|source| IterError::Read { path: src, source })?;
            let output = postprocess(&source);
            std::fs::write(&dest, &output)
                .map_err(|source| IterError::Write { path: dest, source })?;
            Ok(())
        })
    }

    fn postprocess(source: &str) -> String {
        let mut events = jotdown::Parser::new(source).collect::<Vec<_>>();
        link_headings(&mut events);
        for event in events.clone() {
            println!("{event:?}");
        }
        let html = jotdown::html::render_to_string(events.into_iter());
        include_str!("template.html").replace("{insert main here}", &html)
    }

    fn link_headings(events: &mut Vec<Event<'_>>) {
        let mut position = 0;
        while let Some(event) = events.get(position) {
            match event {
                Event::Start(Container::Heading { id, .. }, _) => {
                    let link = Container::Link(
                        format!("#{id}").into(),
                        jotdown::LinkType::Span(jotdown::SpanLinkType::Inline),
                    );
                    events.insert(position, Event::Start(link, jotdown::Attributes::new()));
                    position += 1;
                }
                Event::End(Container::Heading { id, .. }) => {
                    let link = Container::Link(
                        format!("#{id}").into(),
                        jotdown::LinkType::Span(jotdown::SpanLinkType::Inline),
                    );
                    events.insert(position + 1, Event::End(link));
                    position += 1;
                }
                _ => {}
            }
            position += 1;
        }
    }

    pub(crate) type Error = crate::dir::Error<IterError>;

    #[derive(Debug)]
    pub(crate) enum IterError {
        Read { path: std::path::PathBuf, source: std::io::Error },
        Write { path: std::path::PathBuf, source: std::io::Error },
    }

    impl core::fmt::Display for IterError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Read { path, source } => {
                    write!(f, "failed to read ray file `{}` ({source})", path.display())
                }
                Self::Write { path, source } => {
                    write!(f, "failed to write html file `{}` ({source})", path.display())
                }
            }
        }
    }

    impl std::error::Error for IterError {}
}

mod dir {
    pub fn iter<Handler, HandlerError>(
        src: &std::path::Path,
        dest: &std::path::Path,
        new_ext: Option<&'static str>,
        mut handler: Handler,
    ) -> Result<(), Error<HandlerError>>
    where
        Handler: FnMut(std::path::PathBuf, std::path::PathBuf) -> Result<(), HandlerError>,
        HandlerError: std::error::Error,
    {
        let mut work = vec![src.to_owned()];
        while let Some(dir) = work.pop() {
            for entry in
                dir.read_dir().map_err(|source| Error::ReadDir { path: dir.clone(), source })?
            {
                let entry =
                    entry.map_err(|source| Error::ReadDirEntry { path: dir.clone(), source })?;
                let mut new_path = dest.join(entry.path().strip_prefix(src).unwrap());
                match entry
                    .file_type()
                    .map_err(|source| Error::GetType { path: dir.clone(), source })?
                {
                    typ if typ.is_file() => {
                        if let Some(ext) = new_ext {
                            new_path = new_path.with_extension(ext);
                        }
                        handler(entry.path(), new_path).map_err(Error::Handler)?;
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
    pub(crate) enum Error<Handler> {
        ReadDir { path: std::path::PathBuf, source: std::io::Error },
        ReadDirEntry { path: std::path::PathBuf, source: std::io::Error },
        GetType { path: std::path::PathBuf, source: std::io::Error },
        WrongType(std::path::PathBuf),
        CreateDir { path: std::path::PathBuf, source: std::io::Error },
        Handler(Handler),
    }

    impl<Handler: std::error::Error> core::fmt::Display for Error<Handler> {
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
                Self::CreateDir { path, source } => {
                    write!(f, "failed to create directory at `{}` ({source})", path.display())
                }
                Self::Handler(error) => core::fmt::Display::fmt(error, f),
            }
        }
    }

    impl<Handler: std::error::Error> std::error::Error for Error<Handler> {}
}
