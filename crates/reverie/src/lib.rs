#![feature(lint_reasons)]

mod error;
mod generator;
mod parser;

pub use ariadne::Source;
pub use error::Error;
use generator::Generator;
use parser::Parser;

#[expect(clippy::missing_errors_doc)]
pub fn to_html(source: &str) -> Result<Vec<u8>, Error> {
    let bytes = source.as_bytes();
    let ast = Parser::new(bytes).parse()?;
    Generator::new(bytes, ast).generate()
}
