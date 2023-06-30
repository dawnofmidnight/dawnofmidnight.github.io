use core::ops::Index;
use std::ops::Range;

use ariadne::{Label, Report, ReportKind};

#[derive(Debug)]
pub enum Error {
    ExpectedFound { expected: u8, found: u8, span: Span },
    ExpectedArgs { found: u8, span: Span },
    IncorrectArgCount { expected: usize, found: usize, span: Span },
    ExpectedInlineArg { span: Span },
    ExpectedBlockArg { span: Span },
    ExpectedIdentArg { span: Span },
    ExpectedStringArg { span: Span },
}

impl Error {
    #[must_use]
    pub fn into_report(&self) -> Report<'_, (&'static str, Range<usize>)> {
        let (Self::ExpectedFound { span, .. }
        | Self::ExpectedArgs { span, .. }
        | Self::IncorrectArgCount { span, .. }
        | Self::ExpectedInlineArg { span }
        | Self::ExpectedBlockArg { span }
        | Self::ExpectedIdentArg { span }
        | Self::ExpectedStringArg { span }) = self;

        let message = match self {
            Self::ExpectedFound { expected, found, .. } => {
                format!("expected `{}`, found `{}`", *expected as char, *found as char)
            }
            Self::ExpectedArgs { found, .. } => {
                format!("expected (, [, or {{, found `{}`", *found as char)
            }
            Self::IncorrectArgCount { expected, found, .. } => {
                format!("expected `{expected}` arguments, found `{found}`")
            }
            Self::ExpectedInlineArg { .. } => "expected an inline argument".to_owned(),
            Self::ExpectedBlockArg { .. } => "expected a block argument".to_owned(),
            Self::ExpectedIdentArg { .. } => "expected an identifier argument".to_owned(),
            Self::ExpectedStringArg { .. } => "expected a string argument".to_owned(),
        };

        Report::build(ReportKind::Error, "rev", span.start as usize)
            .with_message(message)
            .with_label(Label::new(("rev", span.start as usize..span.end as usize)))
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

impl Index<Span> for [u8] {
    type Output = [u8];

    fn index(&self, span: Span) -> &[u8] {
        &self[span.start as usize..span.end as usize]
    }
}
