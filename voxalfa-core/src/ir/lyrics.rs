use crate::{
    ast::{
        lyrics::{LyricLine, LyricOperator, LyricOperatorKind, LyricSpecialChar},
        symbols::{LyricStringId, ScopeId},
    },
    ir::utils::{UnderlineMarker, UnderlineRange},
};

#[derive(Debug, Default)]
pub struct LyricLineIr {
    pub sid: ScopeId,
    pub columns: Vec<LyricColumnIR>,
    pub operators: Vec<LyricOperator>,
    pub anchor: bool,
}

impl LyricLineIr {
    pub fn new(line: &LyricLine) -> Self {
        Self {
            sid: line.sid,
            anchor: line.anchor.is_some(),
            ..Default::default()
        }
    }

    pub fn fit_underlines(&mut self, underlines: &[UnderlineRange]) {
        let partials = self
            .columns
            .iter_mut()
            .flat_map(|p| &mut p.chunks)
            .flat_map(|c| &mut c.primitives);

        for (partial_idx, partial) in partials.enumerate() {
            partial.underline.left = underlines.iter().any(|u| u.start == partial_idx);
            partial.underline.right = underlines.iter().any(|u| u.end - 1 == partial_idx);
        }
    }
}

#[derive(Debug, Default)]
pub struct LyricColumnIR {
    pub sid: ScopeId,
    pub chunks: Vec<LyricChunkIR>,
    pub operators: Vec<LyricOperatorKind>,
    pub placeholder: bool,
    pub span: usize,
}

impl LyricColumnIR {
    pub fn new(sid: ScopeId, span: usize) -> Self {
        Self {
            sid,
            span,
            ..Default::default()
        }
    }

    pub fn add_chunk(&mut self, strings: Vec<LyricStringIR>) {
        let primitives = strings
            .into_iter()
            .map(|s| LyricPrimitive {
                underline: UnderlineMarker::default(),
                string: s,
            })
            .collect();

        self.chunks.push(LyricChunkIR { primitives });
    }
}

#[derive(Debug)]
pub struct LyricChunkIR {
    pub primitives: Vec<LyricPrimitive>,
}

#[derive(Debug)]
pub struct LyricPrimitive {
    pub underline: UnderlineMarker,
    pub string: LyricStringIR,
}

#[derive(Debug)]
pub enum LyricStringIR {
    Reference(LyricStringId),
    Special(LyricSpecialChar),
}
