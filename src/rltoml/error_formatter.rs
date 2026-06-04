/*
 RlToml: Shared error formatting utilities
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

use crate::binary_reader::{self, ReadContext};

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
	#[error("{0:?}")]
	Kaitai(kaitai::KError),

	#[error("{err:?}")]
	KaitaiWithContext { err: kaitai::KError, context: Option<ReadContext> },
}

/// Expected value spec for magic-number validation errors.
pub struct MagicSpec<'a> {
	pub label: &'a str,
	pub expected: i32,
	pub offset: usize,
	pub kind: &'a kaitai::ValidationKind,
	pub src_path: &'a str,
}

/// Expected-values spec for enum validation errors (e.g. frame entry tags).
pub struct AnyOfSpec<'a> {
	pub label: &'a str,
	pub any_of: &'a [i32],
	pub kind: &'a kaitai::ValidationKind,
	pub src_path: &'a str,
	pub got: Option<(i32, Vec<u8>)>,
	pub offset: Option<usize>,
}

impl ParseError {
	/// Wraps a Kaitai error with the read context captured before failure.
	pub fn kaitai_with_context(err: kaitai::KError, context: Option<ReadContext>) -> Self {
		ParseError::KaitaiWithContext { err, context }
	}

	/// Returns the read context from a `KaitaiWithContext` error, if present.
	pub fn read_context(&self) -> Option<&ReadContext> {
		match self {
			ParseError::KaitaiWithContext { context, .. } => context.as_ref(),
			_ => None,
		}
	}
}

impl From<kaitai::KError> for ParseError {
	/// Converts a Kaitai error into a `ParseError::Kaitai` without context.
	fn from(e: kaitai::KError) -> Self {
		ParseError::Kaitai(e)
	}
}

impl From<std::io::Error> for ParseError {
	/// Converts an I/O error into a `ParseError::Kaitai` with an I/O error message.
	fn from(e: std::io::Error) -> Self {
		ParseError::Kaitai(kaitai::KError::IoError { msg: e.to_string() })
	}
}

/// Formats a value as `<decimal> (0x<hex>, bytes: <hex bytes>)`.
pub fn format_value(value: i32, bytes: &[u8]) -> String {
	format!("{} (0x{:X}, bytes: {})", value, value, binary_reader::format_bytes_hex(bytes))
}

/// Formats a 16-byte hex dump at `offset` with a caret under `field_len` bytes.
pub fn format_dump(path: &str, offset: usize, field_len: usize) -> Option<String> {
	let dump_start = offset & !0xF;
	let dump_len = 16usize;
	let size = binary_reader::file_size(path).ok()?;
	let header = format!(
		"Dump ({} of {} bytes shown, starting at offset 0x{:08X}, error at offset 0x{:08X}):",
		dump_len, size, dump_start, offset
	);
	let (line, caret) = binary_reader::hex_dump_at(path, dump_start, dump_len, offset, field_len).ok()?;
	Some(format!("{}\n{}\n{}", header, line, caret))
}

/// Formats a single-value magic-number validation error with hex dump.
pub fn format_magic(spec: MagicSpec, path: &str, verbose: bool) -> String {
	let expected_bytes = le_bytes(spec.expected);
	let (got, got_bytes) = match binary_reader::read_u4_le_full(path, spec.offset) {
		Ok((v, b)) => (v as i32, b),
		Err(_) => {
			return format!("invalid value at {}: (could not re-read file).", spec.label);
		}
	};

	if !verbose {
		return format!("invalid value at {}: found {} (expected {}).", spec.label, got, spec.expected);
	}

	let mut out = format!(
		indoc::indoc! {"\
			invalid value at {label}:
			 Expected: {expected} (0x{expected:X}, bytes: {exp_hex})
			 Found: {got} (0x{got:X}, bytes: {got_hex})
			 Kaitai Error: {kind:?} @ {src}
			 Hex offset: 0x{offset:08X} (decimal: {offset})
		"},
		label = spec.label,
		expected = spec.expected,
		got = got,
		kind = spec.kind,
		src = spec.src_path,
		offset = spec.offset,
		exp_hex = binary_reader::format_bytes_hex(&expected_bytes),
		got_hex = binary_reader::format_bytes_hex(&got_bytes),
	);

	if let Some(dump) = format_dump(path, spec.offset, 4) {
		out.push('\n');
		out.push_str(&dump);
	}

	out
}

/// Formats an "any of" enum validation error with hex dump.
pub fn format_any_of(spec: AnyOfSpec, path: &str, verbose: bool) -> String {
	if !verbose {
		let any_list_short: Vec<String> = spec.any_of.iter().map(|v| v.to_string()).collect();
		let list_str = any_list_short.join(", ");
		return match spec.got {
			Some((g, _)) => format!("invalid value at {}: found {} (expected any of: {}).", spec.label, g, list_str),
			None => format!("invalid value at {} (expected any of: {}).", spec.label, list_str),
		};
	}

	let mut out = format!("invalid value at {}:\n", spec.label);
	out.push_str("Expected any of:\n");
	for v in spec.any_of {
		out.push_str(&format!(" - {}\n", format_value(*v, &le_bytes(*v))));
	}

	if let Some((g, bytes)) = &spec.got {
		out.push('\n');
		out.push_str(&format!("Found: {}\n", format_value(*g, bytes)));
	}

	out.push_str(&format!("Kaitai Error: {:?} @ {}\n", spec.kind, spec.src_path));

	if let Some(offset) = spec.offset {
		out.push_str(&format!("Hex offset: 0x{:08X} (decimal: {})\n", offset, offset));
		if let Some(dump) = format_dump(path, offset, 4) {
			out.push('\n');
			out.push_str(&dump);
		}
	}

	out
}

/// Converts an `i32` to its little-endian byte representation.
fn le_bytes(value: i32) -> [u8; 4] {
	(value as u32).to_le_bytes()
}
