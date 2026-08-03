/*
 RlToml: Shared TOML formatting utilities
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

use toml_edit::{InlineTable, Value};

/// Trait for frame attribute types that can be serialized to TOML inline tables.
pub trait TomlFrameAttrs: Clone + Default {
    fn to_inline_table_fields(&self) -> Vec<(String, String)>;
    fn diff_from(&self, defaults: &Self) -> Self;
    fn common_attrs(frames: &[Self]) -> Self;
    fn to_block_fields(&self) -> Vec<(String, String)> {
        self.to_inline_table_fields()
    }
}

/// Formats block fields as `"key = value"` lines for TOML output.
pub fn write_block_fields(attrs: &impl TomlFrameAttrs) -> Vec<String> {
    attrs
        .to_block_fields()
        .into_iter()
        .map(|(name, val)| format!("{} = {}", name, val))
        .collect()
}

/// Builds a TOML inline table from key-value pairs, parsing each value.
pub fn build_inline_table(fields: &[(String, String)]) -> InlineTable {
    let mut table = InlineTable::new();
    for (key, value) in fields {
        let parsed_value: Value = value.parse().unwrap_or_else(|_| {
            let toml_str = format!("x = \"{}\"", value.replace('"', "\\\""));
            toml_str
                .parse::<toml_edit::Item>()
                .ok()
                .and_then(|item| item.as_value().cloned())
                .unwrap_or_else(|| "\"\"".parse().unwrap())
        });
        table.insert(key, parsed_value);
    }
    table
}
