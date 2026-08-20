pub mod dynamics;
pub mod evaluator;
pub mod event;
pub mod lyrics;
pub mod metrics;
pub mod tempo;
pub mod voice;

use tree_sitter::Tree;

use crate::{
    ast::{header::Header, symbols::SymbolTree},
    data_types::Voice,
    diagnostics::types::Diagnostic,
    ir::BodyIr,
    output::{
        event::TimelineMap,
        voice::{NoteContext, VoiceLine},
    },
};

pub const MIN_COLUMN_WIDTH: u8 = 4;

#[derive(Debug, Default)]
pub struct FinalOutput {
    pub tree: Option<Tree>,
    pub symbols: SymbolTree,
    pub header: Header,
    pub body: BodyIr,
    pub diagnostics: Vec<Diagnostic>,
    pub timelines: TimelineMap,
}

impl FinalOutput {
    pub fn with_tree(mut self, tree: Tree) -> Self {
        self.tree = Some(tree);
        self
    }

    pub fn has_error(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn has_syntax_error(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_syntactic())
    }

    pub fn resolve_maximum_factor(&self) -> u8 {
        self.body
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .flat_map(|s| &s.solfa)
            .flat_map(|s| s.pulses.iter().map(|p| p.factor).max())
            .max()
            .unwrap_or(1)
    }

    pub fn build_voice_line(&self, voice: Voice) -> VoiceLine<'_> {
        let mut notes = Vec::new();
        let mut timeline = Vec::new();

        for (section_id, section) in self.body.sections.iter().enumerate() {
            for (sub_section_id, sub_section) in section.items.iter().enumerate() {
                let Some(solfa) = sub_section.solfa.iter().find(|s| s.voice == voice) else {
                    continue;
                };

                if let Some(partial) = self.timelines.get(sub_section.sid) {
                    timeline.extend(partial);
                }

                let mut lyric_id = 0;
                let mut pulse_id = 0;

                for (id, pulse) in solfa.pulses.iter().enumerate() {
                    let view = &sub_section.views[id];

                    for note in &pulse.columns {
                        notes.push(NoteContext {
                            column: note,
                            factor: pulse.factor,
                            lyric_id,
                            pulse_id,
                            section_id,
                            sub_section_id,
                        });

                        if view.factor > 1 {
                            lyric_id += 1
                        }
                    }

                    if view.factor == 1 {
                        lyric_id += 1;
                    }

                    pulse_id += 1;
                }
            }
        }

        VoiceLine::new(voice, notes, timeline)
    }
}
