use crate::{
    emitter::SvgEmitter,
    error::Result,
    layout::{A4_HEIGHT_PX, A4_PADDING, A4_WIDTH_PX, LINE_GAP, PRINTABLE_WIDTH, SYSTEM_GAP},
    resolver::GridResolver,
    types::{CellContent, Element, ElementKind, RenderContext, TextElement, VirtualGrid},
};

use taffy::{
    AlignItems, Display, FlexDirection, JustifyContent, NodeId, Rect, Size, Style, TaffyTree,
    prelude::{fr, minmax, span, zero},
    style_helpers::{auto, length, percent},
};
use voxalfa_core::output::FinalOutput;

pub struct Renderer<'a> {
    resolver: GridResolver<'a>,
}

impl<'a> Renderer<'a> {
    pub fn new(data: FinalOutput) -> Result<Self> {
        let resolver = GridResolver::new(data)?;
        Ok(Self { resolver })
    }

    pub fn render_to_svg(self) -> Result<String> {
        let mut tree = TaffyTree::<()>::new();

        let elements = self.render(&mut tree)?;
        let emitter = SvgEmitter::new(tree);
        let svg = emitter.render_to_svg(&elements)?;

        Ok(svg)
    }

    fn render(&self, tree: &mut TaffyTree<()>) -> Result<Vec<Element>> {
        let mut context = RenderContext::new(tree);

        let header_node = self.render_header(&mut context)?;
        let body_node = self.render_body(&mut context)?;
        let elements = context.into_elements();

        let document_node = tree.new_with_children(
            Style {
                size: Size {
                    width: length(A4_WIDTH_PX),
                    height: length(A4_HEIGHT_PX),
                },
                padding: Rect::length(A4_PADDING),
                flex_direction: FlexDirection::Column,
                gap: length(25),
                ..Default::default()
            },
            &[header_node, body_node],
        )?;

        tree.compute_layout(
            document_node,
            Size {
                width: length(A4_WIDTH_PX),
                height: length(A4_HEIGHT_PX),
            },
        )?;

        Ok(elements)
    }

    fn render_header(&self, ctx: &mut RenderContext) -> Result<NodeId> {
        let name_container = ctx.tree.new_with_children(
            Style {
                size: Size {
                    width: length(PRINTABLE_WIDTH),
                    height: auto(),
                },
                justify_content: Some(JustifyContent::SPACE_BETWEEN),
                ..Default::default()
            },
            &[],
        )?;

        let params_container = ctx.tree.new_with_children(
            Style {
                size: Size {
                    width: length(PRINTABLE_WIDTH),
                    height: auto(),
                },
                ..Default::default()
            },
            &[],
        )?;

        let container_node = ctx.tree.new_with_children(
            Style {
                size: Size {
                    width: length(PRINTABLE_WIDTH),
                    height: auto(),
                },
                flex_direction: FlexDirection::Column,
                align_items: Some(AlignItems::CENTER),
                gap: length(10),
                ..Default::default()
            },
            &[],
        )?;

        self.render_title(ctx, container_node)?;
        self.render_names(ctx, name_container)?;
        self.render_params(ctx, params_container)?;

        ctx.tree.add_child(container_node, name_container)?;
        ctx.tree.add_child(container_node, params_container)?;

        Ok(container_node)
    }

    fn render_title(&self, ctx: &mut RenderContext, parent: NodeId) -> Result<()> {
        let Some(title) = self.resolver.data.header.get_metadata(|m| &m.title) else {
            return Ok(());
        };

        let node = self.create_text_node(ctx, title, "title", 400, 50)?;
        ctx.tree.add_child(parent, node)?;

        Ok(())
    }

    fn render_names(&self, ctx: &mut RenderContext, parent: NodeId) -> Result<()> {
        if let Some(author) = self.resolver.data.header.get_metadata(|m| &m.author) {
            let content = author
                .iter()
                .map(|n| n.value.clone())
                .collect::<Vec<_>>()
                .join(", ");
            let node = self.create_text_node(ctx, &content, "name", 200, 20)?;
            ctx.tree.add_child(parent, node)?;
        }

        if let Some(composer) = self.resolver.data.header.get_metadata(|m| &m.composer) {
            let content = composer
                .iter()
                .map(|n| n.value.clone())
                .collect::<Vec<_>>()
                .join(", ");
            let node = self.create_text_node(ctx, &content, "name", 200, 20)?;
            ctx.tree.add_child(parent, node)?;
        }

        Ok(())
    }

    fn render_params(&self, ctx: &mut RenderContext, parent: NodeId) -> Result<()> {
        if let Some(key) = self.resolver.data.header.get_params(|m| &m.key) {
            let content = format!("Do dia {}", key);
            let node = self.create_text_node(ctx, &content, "key", 100, 20)?;
            ctx.tree.add_child(parent, node)?;
        }

        if let Some(time) = self.resolver.data.header.get_params(|m| &m.time) {
            let content = format!("{}/{}", time.top, time.bottom);
            let node = self.create_text_node(ctx, &content, "time", 50, 20)?;
            ctx.tree.add_child(parent, node)?;
        }

        Ok(())
    }

    fn create_text_node(
        &self,
        ctx: &mut RenderContext,
        content: &str,
        class: &'static str,
        width: u32,
        height: u32,
    ) -> Result<NodeId> {
        let node_id = ctx.tree.new_leaf(Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            ..Default::default()
        })?;

        ctx.add_element(
            node_id,
            ElementKind::Text(TextElement {
                content: content.to_string(),
                class,
            }),
        );

        Ok(node_id)
    }

    fn render_body(&self, ctx: &mut RenderContext) -> Result<NodeId> {
        let body_node = ctx.tree.new_with_children(
            Style {
                size: Size {
                    width: percent(100),
                    height: percent(100),
                },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &[],
        )?;

        for system in self.resolver.collect_systems() {
            let grid = self.resolver.build_system_grid(&system)?;
            let system_node = self.render_system(&grid, ctx)?;
            ctx.tree.add_child(body_node, system_node)?;
        }

        Ok(body_node)
    }

    fn render_system(&self, grid: &VirtualGrid, ctx: &mut RenderContext) -> Result<NodeId> {
        let system_node = ctx.tree.new_with_children(
            Style {
                size: Size {
                    width: percent(100),
                    height: auto(),
                },
                flex_direction: FlexDirection::Column,
                gap: length(SYSTEM_GAP),
                ..Default::default()
            },
            &[],
        )?;

        let lead_col_width = self.resolver.font.solfa.get_width(".,");
        let mut grid_columns = Vec::with_capacity(grid.column_count);

        for _ in 0..grid.column_count / 2 {
            grid_columns.push(length(lead_col_width));
            grid_columns.push(minmax(length(self.resolver.col_width), fr(1.0)));
        }

        let grid_style = Style {
            display: Display::Grid,
            grid_template_columns: grid_columns,
            size: Size {
                width: length(PRINTABLE_WIDTH),
                height: auto(),
            },
            gap: Size {
                width: zero(),
                height: length(LINE_GAP),
            },
            ..Default::default()
        };

        let grid_node = ctx.tree.new_with_children(grid_style, &[])?;

        for row in &grid.rows {
            for cell in row {
                let node_id = self.render_cell(cell, ctx)?;
                ctx.tree.add_child(grid_node, node_id)?;
            }
        }

        ctx.tree.add_child(system_node, grid_node)?;

        Ok(system_node)
    }

    fn render_cell(
        &self,
        cell: &crate::types::GridCell,
        ctx: &mut RenderContext,
    ) -> Result<NodeId> {
        let style = Style {
            grid_column: span(cell.col_span),
            grid_row: span(cell.row_span),
            ..Default::default()
        };

        match &cell.content {
            CellContent::Text { content, class } => {
                let node_id = ctx.tree.new_leaf(Style {
                    size: Size {
                        width: auto(),
                        height: length(self.resolver.font.solfa.ascent),
                    },
                    ..style
                })?;

                ctx.add_element(
                    node_id,
                    ElementKind::Text(TextElement {
                        content: content.clone(),
                        class,
                    }),
                );

                Ok(node_id)
            }
            CellContent::Barline => {
                let node_id = ctx.tree.new_leaf(Style {
                    size: Size {
                        width: auto(),
                        height: auto(),
                    },
                    ..style
                })?;

                ctx.add_element(node_id, ElementKind::Barline);

                Ok(node_id)
            }
            CellContent::Empty => {
                let node_id = ctx.tree.new_leaf(style)?;
                Ok(node_id)
            }
        }
    }
}
