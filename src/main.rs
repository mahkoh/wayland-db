#![expect(clippy::needless_update, clippy::single_char_add_str)]

use {crate::generate::GeneratorError, error_reporter::Report};

mod ast;
mod collector;
mod generate;
mod id_source;
mod parser;

fn main() -> Result<(), Report<GeneratorError>> {
    generate::main().map_err(Report::new)
}
