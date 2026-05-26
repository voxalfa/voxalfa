use std::{
    fs::OpenOptions,
    io::{self, Write},
};

const PARSER_FILE: &str = include_str!("../tree-sitter-voxalfa/src/parser.c");

fn main() -> Result<(), io::Error> {
    let mut output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("src/ts_utils/generated.rs")?;

    let mut buffer = String::from("pub mod node_types {\n");
    let mut process_flag = false;

    for line in PARSER_FILE.lines() {
        if line.contains("enum ts_symbol_identifiers {") {
            process_flag = true;
        }

        if !process_flag {
            continue;
        }

        if let Some(rest) = line.trim_start().strip_prefix("sym_") {
            let parts = rest.split_whitespace().collect::<Vec<_>>();
            let name = parts[0];

            if name.starts_with("_") {
                continue;
            }

            let lhs = name.to_uppercase();
            let rhs = parts[2].replace(",", ";");
            let line = format!("    pub const {lhs}: u16 = {rhs}\n");
            buffer.push_str(&line);
        }

        if line.contains("}") {
            buffer.push_str("}");
            break;
        }
    }

    output_file.write(buffer.as_bytes())?;

    Ok(())
}
