/*
 RlToml: Shared binary reading utilities
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

use std::cell::{Ref, RefCell, RefMut};
use kaitai::{BytesReader, ReaderState, KStream, KResult};

/// Information about the last successful read, captured at runtime for
/// error reporting. Used by the kaitai wrapper to surface the value and
/// file offset that triggered a `ValidationFailed` error.
#[derive(Debug, Clone)]
pub struct ReadContext {
	pub offset: usize,
	pub value: Vec<u8>,
}

/// `KStream` wrapper that records the offset and bytes of the last
/// successful read. Used to surface the failing value's location when
/// kaitai reports a `ValidationFailed` error.
pub struct TrackingReader {
	inner: BytesReader,
	last_offset: RefCell<Option<usize>>,
	last_value: RefCell<Option<Vec<u8>>>,
}

impl TrackingReader {
	/// Opens a file and wraps it in a `TrackingReader`.
	pub fn open(path: &str) -> KResult<Self> {
		Ok(Self {
			inner: BytesReader::open(path)?,
			last_offset: RefCell::new(None),
			last_value: RefCell::new(None),
		})
	}

	/// Returns the file offset of the last successful read.
	pub fn last_read_offset(&self) -> Option<usize> {
		self.last_offset.borrow().clone()
	}

	/// Returns the bytes from the last successful read.
	pub fn last_read_value(&self) -> Option<Vec<u8>> {
		self.last_value.borrow().clone()
	}

	/// Builds a `ReadContext` from the last successful read.
	pub fn read_context(&self) -> Option<ReadContext> {
		let offset = self.last_read_offset()?;
		let value = self.last_read_value()?;
		Some(ReadContext { offset, value })
	}
}

impl KStream for TrackingReader {
	/// Clones the inner `BytesReader`.
	fn clone(&self) -> BytesReader {
		Clone::clone(&self.inner)
	}

	/// Returns the total size of the underlying reader.
	fn size(&self) -> usize {
		self.inner.size()
	}

	/// Delegates to the inner reader's state.
	fn get_state(&self) -> Ref<'_, ReaderState> {
		self.inner.get_state()
	}

	/// Delegates to the inner reader's mutable state.
	fn get_state_mut(&self) -> RefMut<'_, ReaderState> {
		self.inner.get_state_mut()
	}

	/// Reads bytes and records the offset and value for error tracking.
	fn read_bytes(&self, len: usize) -> KResult<Vec<u8>> {
		let offset = self.inner.pos();
		let result = KStream::read_bytes(&self.inner, len);
		if let Ok(ref bytes) = result {
			*self.last_offset.borrow_mut() = Some(offset);
			*self.last_value.borrow_mut() = Some(bytes.clone());
		}
		result
	}

	/// Reads all remaining bytes and records the offset and value for error tracking.
	fn read_bytes_full(&self) -> KResult<Vec<u8>> {
		let offset = self.inner.pos();
		let result = self.inner.read_bytes_full();
		if let Ok(ref bytes) = result {
			*self.last_offset.borrow_mut() = Some(offset);
			*self.last_value.borrow_mut() = Some(bytes.clone());
		}
		result
	}
}

/// Opens a `BytesReader` from a file path.
pub fn open(path: &str) -> KResult<BytesReader> {
	BytesReader::open(path)
}

/// Reads a little-endian u32 at the given offset.
pub fn read_u4_le_at(path: &str, offset: usize) -> KResult<i64> {
	let reader = open(path)?;
	reader.seek(offset)?;
	Ok(reader.read_u4le()? as i64)
}

/// Reads a little-endian u32 and its raw bytes at the given offset.
pub fn read_u4_le_full(path: &str, offset: usize) -> KResult<(i64, [u8; 4])> {
	let reader = open(path)?;
	reader.seek(offset)?;
	let mut bytes = [0u8; 4];
	bytes.copy_from_slice(&reader.read_bytes(4)?);
	let value = u32::from_le_bytes(bytes) as i64;
	Ok((value, bytes))
}

/// Formats bytes as space-separated hex values.
pub fn format_bytes_hex(bytes: &[u8], uppercase: bool) -> String {
	if !uppercase {
		bytes
			.iter()
			.map(|b| format!("{:02x}", b))
			.collect::<Vec<_>>()
			.join(" ")
	} else {
		bytes
			.iter()
			.map(|b| format!("{:02X}", b))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

/// Formats a u32 as hex with optional uppercase ("0x1a2b" / "0x1A2B").
pub fn format_hex_u32(value: u32, uppercase: bool) -> String {
	if !uppercase {
		format!("0x{:x}", value)
	} else {
		format!("0x{:X}", value)
	}
}

/// Formats a u32 as zero-padded hex ("0x0000abcd" / "0x0000ABCD").
pub fn format_hex_u32_padded(value: u32, width: usize, uppercase: bool) -> String {
	if !uppercase {
		format!("0x{:0width$x}", value, width = width)
	} else {
		format!("0x{:0width$X}", value, width = width)
	}
}

/// Formats a hex dump line with a caret under the field at `field_offset`.
pub fn hex_dump_at(path: &str, dump_offset: usize, dump_len: usize, field_offset: usize, field_len: usize, uppercase: bool) -> KResult<(String, String)> {
	let reader = open(path)?;
	reader.seek(dump_offset)?;
	let buf = reader.read_bytes(dump_len)?;
	let dump_line = format!("{} | {}", format_hex_u32_padded(dump_offset as u32, 8, uppercase), format_bytes_hex(&buf, uppercase));
	let caret_col = 11 + (field_offset - dump_offset) * 3;
	let caret_len = field_len * 3 - 1;
	let caret_line = format!("{:width$}{}", "", "^".repeat(caret_len), width = caret_col);
	Ok((dump_line, caret_line))
}

/// Returns the file size in bytes.
pub fn file_size(path: &str) -> std::io::Result<u64> {
	Ok(std::fs::metadata(path)?.len())
}

/// Converts the read context into the "got" value and offset for error reporting.
pub fn context_to_got_offset(context: Option<&ReadContext>) -> (Option<(i32, Vec<u8>)>, Option<usize>) {
	let Some(c) = context else {
		return (None, None);
	};
	let offset = Some(c.offset);
	let got = if c.value.len() == 4 {
		let mut arr = [0u8; 4];
		arr.copy_from_slice(&c.value);
		Some((u32::from_le_bytes(arr) as i32, c.value.clone()))
	} else {
		None
	};
	(got, offset)
}
