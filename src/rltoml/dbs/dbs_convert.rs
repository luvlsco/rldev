/*
 RlToml: TOML to DBS conversion utilities
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

use crate::error_formatter;
use encoding_rs::SHIFT_JIS;

#[derive(Debug, thiserror::Error)]
pub enum DbsWriteError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    TomlParse(#[from] toml_edit::TomlError),
    #[error("{0}")]
    InvalidStructure(String),
    #[error("{0}")]
    Encoding(String),
    #[error("{0}")]
    Archive(#[from] super::dbs_decompress::DbsError),
}

#[derive(Debug)]
struct TomlColumn {
    id: u32,
    type_code: u32,
}

#[derive(Debug)]
enum TomlCell {
    String(String),
    Integer(u32),
}

#[derive(Debug)]
struct TomlRow {
    id: u32,
    cells: Vec<TomlCell>,
}

/// Converts TOML directly into the obfuscated `.dbs` archive.
pub fn toml_to_dbs(input_path: &str, output_path: &str, verbose: bool) -> Result<(), DbsWriteError> {
    let content = std::fs::read_to_string(input_path)?;
    let doc = content.parse()?;
    let data = build_dbs_bin(&doc)?;
    super::dbs_decompress::write_bin_as_dbs(&data, output_path, verbose)?;
    Ok(())
}

/// Formats a TOML-to-DBS error in the same compact style as GAN conversion.
pub fn format_toml_to_dbs_error(err: &DbsWriteError, verbose: bool) -> String {
    error_formatter::format_write_error(&err.to_string(), verbose)
}

fn build_dbs_bin(doc: &toml_edit::Document<std::string::String>) -> Result<Vec<u8>, DbsWriteError> {
    let root = doc
        .as_table()
        .get("dbs")
        .ok_or_else(|| DbsWriteError::InvalidStructure("missing [dbs] section".into()))?
        .as_table()
        .ok_or_else(|| DbsWriteError::InvalidStructure("[dbs] must be a table".into()))?;

    let column_item = root
        .get("column")
        .ok_or_else(|| DbsWriteError::InvalidStructure("missing [[dbs.column]] entries".into()))?;
    let column_tables = column_item
        .as_array_of_tables()
        .ok_or_else(|| DbsWriteError::InvalidStructure("'column' must be an array of tables".into()))?;

    let mut columns = Vec::with_capacity(column_tables.len());
    for table in column_tables.iter() {
        let id = table_u32(table, "column_id")?;
        let type_name = table
            .get("type")
            .and_then(|item| item.as_str())
            .ok_or_else(|| DbsWriteError::InvalidStructure("column 'type' must be a string".into()))?;
        let type_code = match type_name {
            "string" => 0x53,
            "integer" => 0x56,
            "unknown" => table_u32(table, "type_code")?,
            _ => {
                return Err(DbsWriteError::InvalidStructure(format!("unknown column type '{type_name}'")));
            }
        };
        columns.push(TomlColumn { id, type_code });
    }

    let row_item = root
        .get("row")
        .ok_or_else(|| DbsWriteError::InvalidStructure("missing [[dbs.row]] entries".into()))?;
    let row_tables = row_item
        .as_array_of_tables()
        .ok_or_else(|| DbsWriteError::InvalidStructure("'row' must be an array of tables".into()))?;

    let mut rows = Vec::with_capacity(row_tables.len());
    for table in row_tables.iter() {
        let id = table_u32(table, "row_id")?;
        let mut cells = Vec::with_capacity(columns.len());
        for (index, column) in columns.iter().enumerate() {
            let key = format!("row_column_{index}");
            let item = table
                .get(&key)
                .ok_or_else(|| DbsWriteError::InvalidStructure(format!("row missing '{key}'")))?;
            cells.push(match column.type_code {
                0x53 => TomlCell::String(
                    item.as_str()
                        .ok_or_else(|| DbsWriteError::InvalidStructure(format!("'{key}' must be a string")))?
                        .to_string(),
                ),
                _ => TomlCell::Integer(item_u32(item, &key)?),
            });
        }
        rows.push(TomlRow { id, cells });
    }

    write_dbs_bin(&columns, &rows)
}

fn table_u32(table: &toml_edit::Table, key: &str) -> Result<u32, DbsWriteError> {
    let item = table
        .get(key)
        .ok_or_else(|| DbsWriteError::InvalidStructure(format!("missing '{key}'")))?;
    item_u32(item, key)
}

fn item_u32(item: &toml_edit::Item, key: &str) -> Result<u32, DbsWriteError> {
    let value = item
        .as_integer()
        .ok_or_else(|| DbsWriteError::InvalidStructure(format!("'{key}' must be an integer")))?;
    u32::try_from(value).map_err(|_| DbsWriteError::InvalidStructure(format!("'{key}' is outside the u32 range")))
}

fn write_dbs_bin(columns: &[TomlColumn], rows: &[TomlRow]) -> Result<Vec<u8>, DbsWriteError> {
    let row_count = u32::try_from(rows.len()).map_err(|_| DbsWriteError::InvalidStructure("too many rows".into()))?;
    let column_count = u32::try_from(columns.len()).map_err(|_| DbsWriteError::InvalidStructure("too many columns".into()))?;
    let row_id_offset = 0x1C_u32;
    let type_list_offset = checked_offset(row_id_offset, row_count.checked_mul(4))?;
    let value_list_offset = checked_offset(type_list_offset, column_count.checked_mul(8))?;
    let string_table_offset = checked_offset(
        value_list_offset,
        row_count.checked_mul(column_count).and_then(|n| n.checked_mul(4)),
    )?;

    let mut values = Vec::with_capacity(rows.len().saturating_mul(columns.len()));
    let mut string_table = Vec::new();
    for row in rows {
        if row.cells.len() != columns.len() {
            return Err(DbsWriteError::InvalidStructure(
                "row column count does not match schema".into(),
            ));
        }
        for cell in &row.cells {
            match cell {
                TomlCell::String(value) => {
                    let offset = u32::try_from(string_table.len())
                        .map_err(|_| DbsWriteError::InvalidStructure("string table is too large".into()))?;
                    let encoded = encode_shift_jis(value)?;
                    string_table.extend_from_slice(&encoded);
                    string_table.push(0);
                    values.push(offset);
                }
                TomlCell::Integer(value) => values.push(*value),
            }
        }
    }

    let string_end = checked_offset(string_table_offset, u32::try_from(string_table.len()).ok())?;
    let mut out = Vec::with_capacity(string_end as usize + 3);
    for value in [
        string_end,
        row_count,
        column_count,
        row_id_offset,
        type_list_offset,
        value_list_offset,
        string_table_offset,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for row in rows {
        out.extend_from_slice(&row.id.to_le_bytes());
    }
    for column in columns {
        out.extend_from_slice(&column.id.to_le_bytes());
        out.extend_from_slice(&column.type_code.to_le_bytes());
    }
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&string_table);
    while out.len() % 64 != 0 {
        out.push(0);
    }
    Ok(out)
}

fn checked_offset(base: u32, add: Option<u32>) -> Result<u32, DbsWriteError> {
    base.checked_add(add.ok_or_else(|| DbsWriteError::InvalidStructure("DBS binary is too large".into()))?)
        .ok_or_else(|| DbsWriteError::InvalidStructure("DBS binary is too large".into()))
}

fn encode_shift_jis(value: &str) -> Result<Vec<u8>, DbsWriteError> {
    let (encoded, _, had_errors) = SHIFT_JIS.encode(value);
    if had_errors {
        return Err(DbsWriteError::Encoding("cannot encode string as Shift_JIS".into()));
    }
    Ok(encoded.into_owned())
}
