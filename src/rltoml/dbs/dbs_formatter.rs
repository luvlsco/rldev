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

use encoding_rs::SHIFT_JIS;
use kaitai::{BytesReader, KStruct, OptRc};

use super::dbs_parser::{DbsParser, DbsParser_ColumnType};
use crate::error_formatter::{self, ParseError, ParseResult};

/// Converts a decrypted DBS binary into the human-readable TOML schema.
pub fn dbs_bin_to_toml(path: &str, verbose: bool) -> ParseResult<String> {
    let data = std::fs::read(path).map_err(ParseError::from)?;
    dbs_bytes_to_toml(&data, verbose)
}

/// Converts decrypted DBS bytes into the human-readable TOML schema.
pub fn dbs_bytes_to_toml(data: &[u8], verbose: bool) -> ParseResult<String> {
    if verbose {
        println!("Reading DBS binary");
    }
    let dbs = parse_bytes(data)?;
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
                    toml_edit::Value::from(cell.str_value().map_err(ParseError::from)?.as_str()).to_string()
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

/// Converts a decrypted DBS binary into CSV (CSV2DBS layout: title row,
/// `#DATANO` / `#DATATYPE` rows, then one row per item). Shift_JIS (CP932),
/// CRLF => the encoding CSV2DBS reads natively.
pub fn dbs_bin_to_csv(path: &str, title: &str, verbose: bool) -> Result<Vec<u8>, String> {
    let data = std::fs::read(path).map_err(|e| error_formatter::format_io_error(&e))?;
    dbs_bytes_to_csv(&data, title, verbose)
}

/// Converts a wrapped `.dbs` archive into CSV in one step: decompress,
/// decrypt, then extract.
pub fn dbs_to_csv(path: &str, title: &str, verbose: bool) -> Result<Vec<u8>, String> {
    let data = super::dbs_decompress::dbs_to_bin_bytes(path, verbose).map_err(|e| e.to_string())?;
    dbs_bytes_to_csv(&data, title, verbose)
}

fn dbs_bytes_to_csv(data: &[u8], title: &str, verbose: bool) -> Result<Vec<u8>, String> {
    if verbose {
        println!("Reading DBS binary");
    }
    let dbs = parse_bytes(data).map_err(|e| error_formatter::format_parse_error(&e))?;
    let types = dbs.types().map_err(kerr_to_string)?;

    let mut datano = String::from("#DATANO");
    let mut datatype = String::from("#DATATYPE");
    for (column_index, column_rc) in types.iter().enumerate() {
        let column = column_rc.get();
        datano.push(',');
        datano.push_str(&(*column.column_id()).to_string());
        datatype.push(',');
        datatype.push_str(match &*column.data_type() {
            DbsParser_ColumnType::String => "S",
            DbsParser_ColumnType::Integer => "V",
            DbsParser_ColumnType::Unknown(code) => {
                return Err(format!("unknown column type 0x{:02X} in column {}.", code, column_index));
            }
        });
    }

    let row_ids = dbs.row_ids().map_err(kerr_to_string)?;
    let rows = dbs.rows().map_err(kerr_to_string)?;
    let mut lines = vec![
        format!("{}{}", title, ",".repeat(types.len())),
        String::new(),
        datano,
        datatype,
        String::new(),
    ];
    for (row_index, row_rc) in rows.iter().enumerate() {
        let row = row_rc.get();
        let cells = row.cells().map_err(kerr_to_string)?;
        let mut line = row_ids[row_index].to_string();
        for cell_rc in cells.iter() {
            let cell = cell_rc.get();
            line.push(',');
            line.push_str(&match &*cell.col_type().map_err(kerr_to_string)? {
                DbsParser_ColumnType::String => csv_quote(cell.str_value().map_err(kerr_to_string)?.as_str()),
                DbsParser_ColumnType::Integer | DbsParser_ColumnType::Unknown(_) => {
                    (*cell.raw_value().map_err(kerr_to_string)?).to_string()
                }
            });
        }
        lines.push(line);
    }

    if verbose {
        println!("Generating CSV");
    }

    let csv = lines.join("\r\n");
    let (encoded, _, had_errors) = SHIFT_JIS.encode(&csv);
    if had_errors {
        return Err("cannot encode CSV as Shift_JIS".to_string());
    }
    Ok(encoded.into_owned())
}

/// Parses a decrypted DBS binary from memory using the generated Kaitai parser.
fn parse_bytes(data: &[u8]) -> ParseResult<OptRc<DbsParser>> {
    let reader = BytesReader::from(data);
    DbsParser::read_into::<_, DbsParser>(&reader, None, None).map_err(ParseError::from)
}

/// Quotes a CSV string field (always quoted, so empty strings stay
/// distinguishable from missing fields); escapes embedded quotes.
fn csv_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn kerr_to_string(e: kaitai::KError) -> String {
    error_formatter::format_parse_error(&ParseError::from(e))
}
