use voxalfa_core::ir::solfa::NoteKind;

pub fn note_text(note: &NoteKind) -> (String, Option<i8>) {
    match note {
        NoteKind::Note(note) => (
            note.text(),
            if note.octave == 0 {
                None
            } else {
                Some(note.octave)
            },
        ),
        NoteKind::ProlongedNote => ("-".to_string(), None),
        NoteKind::EmptyNote => (String::new(), None),
    }
}

pub fn division_symbol(cursor: u32, total: u32) -> &'static str {
    if total == 0 {
        return "";
    }

    let double = cursor.saturating_mul(2);
    let quad = cursor.saturating_mul(4);

    if double == total {
        "."
    } else if quad == total {
        ","
    } else if quad == total.saturating_mul(3) {
        ".,"
    } else if total == 3 && cursor > 0 {
        "‘"
    } else if double.is_multiple_of(total) {
        "."
    } else if quad.is_multiple_of(total) {
        ","
    } else {
        ""
    }
}
