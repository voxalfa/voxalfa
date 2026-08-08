pub mod ast;
pub mod event;
pub mod ir;

use tree_sitter::Tree;

use crate::{
    ast::parser::Parser,
    error::Error,
    ir::builder::IrBuilder,
    output::FinalOutput,
    ts_utils::context::TSContext,
    validation::{ast::AstValidator, ir::IrValidator},
};

pub struct Validator {
    ts_context: TSContext,
}

impl Validator {
    pub fn init() -> Result<Self, Error> {
        Ok(Self {
            ts_context: TSContext::new()?,
        })
    }

    pub fn analyze(&mut self, source: &str) -> FinalOutput {
        let tree = self.ts_context.parse(source.as_bytes());

        if let Some(tree) = tree {
            self.analyze_with_tree(source, &tree).with_tree(tree)
        } else {
            Default::default()
        }
    }

    pub fn analyze_with_tree(&mut self, source: &str, tree: &Tree) -> FinalOutput {
        let parser = Parser::new(source);
        let parser_out = parser.parse(tree, &mut self.ts_context);
        let ast_validator = AstValidator::new(&parser_out);
        let ast_validator_out = ast_validator.validate();
        let ir_builder = IrBuilder::new(&parser_out.symbols);
        let ir_builder_out = ir_builder.build(parser_out.body);
        let ir_validator = IrValidator::new(&parser_out.header, &parser_out.symbols);
        let ir_validator_out = ir_validator.validate(&ir_builder_out.body);

        let diagnostics = parser_out
            .reporter
            .merge(ast_validator_out.reporter)
            .merge(ir_builder_out.reporter)
            .merge(ir_validator_out.reporter)
            .into_diagnostics_vec();

        FinalOutput {
            timelines: ir_validator_out.timelines,
            diagnostics,
            symbols: parser_out.symbols,
            header: parser_out.header,
            body: ir_builder_out.body,
            tree: None,
        }
    }
}
