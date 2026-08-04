use tree_sitter::Tree;

use crate::{
    ast::parser::Parser,
    error::Error,
    ir::builder::IRBuilder,
    output::FinalOutput,
    ts_utils::context::TSContext,
    validation::validator::{Validator, ValidatorOutput},
};

pub mod ast;
pub mod data_types;
pub mod diagnostics;
pub mod error;
pub mod ir;
pub mod output;
pub mod ts_utils;
pub mod validation;

#[cfg(test)]
mod tests;

pub const LIB_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SUPPORTED_VERSION: &str = "=0.1.0-alpha";

pub struct MultiStepValidator {
    ts_context: TSContext,
}

impl MultiStepValidator {
    pub fn init() -> Result<Self, Error> {
        Ok(Self {
            ts_context: TSContext::new()?,
        })
    }

    pub fn analyze(&mut self, source: &str) -> FinalOutput {
        let tree = self.ts_context.parse(source.as_bytes());

        if let Some(tree) = tree {
            self.analyze_tree(source, &tree).with_tree(tree)
        } else {
            Default::default()
        }
    }

    // TODO: add filters to skip some steps
    pub fn analyze_tree(&mut self, source: &str, tree: &Tree) -> FinalOutput {
        let parser = Parser::new(source);
        let parser_out = parser.parse(tree, &mut self.ts_context);
        let ir_builder = IRBuilder::new(&parser_out.symbols);

        let mut validator = Validator::new(&parser_out.symbols, &parser_out.header);
        validator.validate_body(&parser_out.body);

        let builder_out = ir_builder.build(parser_out.body);
        validator.validate_body_ir(&builder_out.body);

        let ValidatorOutput {
            timelines,
            reporter,
        } = validator.finalize();

        let diagnostics = reporter
            .merge(builder_out.reporter)
            .merge(parser_out.reporter)
            .into_diagnostics_vec();

        FinalOutput {
            timelines,
            diagnostics,
            symbols: parser_out.symbols,
            header: parser_out.header,
            ir: builder_out.body,
            delimiters: parser_out.delimiters,
            tree: None,
        }
    }
}
