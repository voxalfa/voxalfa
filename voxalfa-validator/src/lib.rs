use crate::{
    ast::parser::Parser,
    error::Error,
    ir::builder::IRBuilder,
    output::FinalOutput,
    ts_utils::context::TSContext,
    validator::{Validator, ValidatorOutput},
};

pub mod ast;
pub mod diagnostics;
pub mod error;
pub mod ir;
pub mod output;
pub mod render;
pub mod timeline;
pub mod ts_utils;
pub mod validator;

pub struct MultiStepValidator {
    ts_context: TSContext,
}

impl MultiStepValidator {
    pub fn init() -> Result<Self, Error> {
        Ok(Self {
            ts_context: TSContext::new()?,
        })
    }

    // TODO: add filters to skip some steps
    pub fn process(&mut self, source: &str) -> FinalOutput {
        let parser = Parser::new(source);
        let parser_out = parser.parse(&mut self.ts_context);
        let ir_builder = IRBuilder::new(&parser_out.tree);

        let mut validator = Validator::new(&parser_out.tree, &parser_out.header);
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
            tree: parser_out.tree,
            header: parser_out.header,
            ir: builder_out.body,
            timelines,
            diagnostics,
        }
    }
}
