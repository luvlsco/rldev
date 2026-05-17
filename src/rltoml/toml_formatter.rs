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

#![allow(unused)]

use itertools::Itertools;

pub trait TomlFrameAttrs: Clone + Default {
	fn to_inline_table_fields(&self) -> Vec<(String, String)>;
	fn diff_from(&self, defaults: &Self) -> Self;
}

pub fn format_inline_table(fields: &[(String, String)]) -> String {
	let formatted = fields
		.iter()
		.map(|(key, value)| format!("{} = {}", key, value))
		.join(", ");
	format!("{{ {} }}", formatted)
}

pub fn write_section_header(section_name: &str) -> String {
	format!("[[{}]]", section_name)
}
