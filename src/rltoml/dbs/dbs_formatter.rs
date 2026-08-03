/*
 RlToml: DBS to TOML & CSV conversion utilities
 Copyright (C) 2026 luvlsco

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use kaitai::{KStruct, OptRc};

use super::dbs_parser::{DbsParser, DbsParser_ColumnType};
use crate::error_formatter::{self, ParseError, ParseResult};

/// Parses a decrypted DBS binary using the generated Kaitai parser.
pub fn parse_dbs_bin(path: &str) -> ParseResult<OptRc<DbsParser>> {
    let reader = crate::binary_reader::TrackingReader::open(path)?;
    DbsParser::read_into::<_, DbsParser>(&reader, None, None).map_err(|err| {
        let context = reader.read_context();
        ParseError::kaitai_with_context(err, context)
    })
}

/// Converts a decrypted DBS binary into the human-readable TOML schema.
pub fn dbs_bin_to_toml(path: &str, verbose: bool) -> ParseResult<String> {
    if verbose {
        println!("Reading DBS binary");
    }

    let dbs = parse_dbs_bin(path)?;
    let types = dbs.types().map_err(ParseError::from)?;
    let mut lines = vec!["[dbs]".to_string(), String::new()];

    for column in types.iter() {
        let column = column.get();
        lines.push("[[dbs.column]]".to_string());
        lines.push(format!("column_id = {}", *column.column_id()));
        match &*column.data_type() {
            DbsParser_ColumnType::String => lines.push("type = \"string\"".to_string()),
            DbsParser_ColumnType::Integer => lines.push("type = \"integer\"".to_string()),
            DbsParser_ColumnType::Unknown(code) => {
                lines.push("type = \"unknown\"".to_string());
                lines.push(format!("type_code = {}", code));
            }
        }
        lines.push(String::new());
    }

    let row_ids = dbs.row_ids().map_err(ParseError::from)?;
    let rows = dbs.rows().map_err(ParseError::from)?;
    for (row_index, row_rc) in rows.iter().enumerate() {
        let row = row_rc.get();
        let cells = row.cells().map_err(ParseError::from)?;
        lines.push("[[dbs.row]]".to_string());
        lines.push(format!("row_id = {}", row_ids[row_index]));

        for (column_index, cell_rc) in cells.iter().enumerate() {
            let cell = cell_rc.get();
            let value = match &*cell.col_type().map_err(ParseError::from)? {
                DbsParser_ColumnType::String => {
                    toml_edit::Value::from(cell.str_value().map_err(ParseError::from)?.as_str())
                        .to_string()
                }
                DbsParser_ColumnType::Integer | DbsParser_ColumnType::Unknown(_) => {
                    (*cell.raw_value().map_err(ParseError::from)?).to_string()
                }
            };
            lines.push(format!("row_column_{} = {}", column_index, value));
        }
        lines.push(String::new());
    }

    if verbose {
        println!("Generating TOML");
    }
    Ok(lines.join("\n"))
}

/// Formats a DBS Kaitai error without inventing magic-number diagnostics.
pub fn format_dbs_bin_to_toml_error(
    err: &ParseError,
    _path: &str,
    _verbose: bool,
    _uppercase: bool,
) -> String {
    let kerr = match err {
        ParseError::Kaitai(kerr) => kerr,
        ParseError::KaitaiWithContext { err: kerr, .. } => kerr,
    };

    error_formatter::format_kaitai_error(kerr)
}
