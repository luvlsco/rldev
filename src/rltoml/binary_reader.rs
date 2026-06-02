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

use kaitai::{BytesReader, KStream, ReaderState};
use thiserror::Error;

/// Information about the last successful read, captured at runtime for
/// error reporting. Used by the kaitai wrapper to surface the value and
/// file offset that triggered a `ValidationFailed` error.
#[derive(Debug, Clone)]
pub struct ReadContext {
	pub offset: usize,
	pub value: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum ParseError {
	#[error("{0:?}")]
	Kaitai(kaitai::KError),

	#[error("{0:?}")]
	KaitaiWithContext(kaitai::KError, Option<ReadContext>),
}

impl ParseError {
	pub fn kaitai_with_context(err: kaitai::KError, ctx: Option<ReadContext>) -> Self {
		ParseError::KaitaiWithContext(err, ctx)
	}

	pub fn read_context(&self) -> Option<&ReadContext> {
		match self {
			ParseError::KaitaiWithContext(_, ctx) => ctx.as_ref(),
			_ => None,
		}
	}
}

impl From<kaitai::KError> for ParseError {
	fn from(e: kaitai::KError) -> Self {
		ParseError::Kaitai(e)
	}
}

impl From<std::io::Error> for ParseError {
	fn from(e: std::io::Error) -> Self {
		ParseError::Kaitai(kaitai::KError::IoError { msg: e.to_string() })
	}
}

pub type ParseResult<T> = Result<T, ParseError>;

pub fn open(path: &str) -> ParseResult<BytesReader> {
	Ok(BytesReader::open(path)?)
}

pub fn read_u4_le_at(path: &str, offset: usize) -> ParseResult<i64> {
	let reader = open(path)?;
	reader.seek(offset)?;
	Ok(reader.read_u4le()? as i64)
}

pub fn read_u4_le_full(path: &str, offset: usize) -> ParseResult<(i64, [u8; 4])> {
	let reader = open(path)?;
	reader.seek(offset)?;
	let mut bytes = [0u8; 4];
	bytes.copy_from_slice(&reader.read_bytes(4)?);
	let value = u32::from_le_bytes(bytes) as i64;
	Ok((value, bytes))
}

pub fn format_bytes_hex(bytes: &[u8]) -> String {
	bytes
		.iter()
		.map(|b| format!("{:02X}", b))
		.collect::<Vec<_>>()
		.join(" ")
}

pub fn hex_dump_at(
	path: &str,
	dump_offset: usize,
	dump_len: usize,
	field_offset: usize,
	field_len: usize,
) -> ParseResult<(String, String)> {
	let reader = open(path)?;
	reader.seek(dump_offset)?;
	let buf = reader.read_bytes(dump_len)?;
	let dump_line = format!("{:08X} | {}", dump_offset, format_bytes_hex(&buf));
	let caret_col = 11 + (field_offset - dump_offset) * 3;
	let caret_len = field_len * 3 - 1;
	let caret_line = format!(
		"{:width$}{}",
		"",
		"^".repeat(caret_len),
		width = caret_col
	);
	Ok((dump_line, caret_line))
}

pub fn file_size(path: &str) -> ParseResult<u64> {
	Ok(std::fs::metadata(path)?.len())
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
	pub fn open(path: &str) -> kaitai::KResult<Self> {
		Ok(Self {
			inner: BytesReader::open(path)?,
			last_offset: RefCell::new(None),
			last_value: RefCell::new(None),
		})
	}

	pub fn last_read_offset(&self) -> Option<usize> {
		self.last_offset.borrow().clone()
	}

	pub fn last_read_value(&self) -> Option<Vec<u8>> {
		self.last_value.borrow().clone()
	}
}

impl KStream for TrackingReader {
	fn clone(&self) -> BytesReader {
		Clone::clone(&self.inner)
	}

	fn size(&self) -> usize {
		self.inner.size()
	}

	fn get_state(&self) -> Ref<'_, ReaderState> {
		self.inner.get_state()
	}

	fn get_state_mut(&self) -> RefMut<'_, ReaderState> {
		self.inner.get_state_mut()
	}

	fn read_bytes(&self, len: usize) -> kaitai::KResult<Vec<u8>> {
		let offset = self.inner.pos();
		let result = KStream::read_bytes(&self.inner, len);
		if let Ok(ref bytes) = result {
			*self.last_offset.borrow_mut() = Some(offset);
			*self.last_value.borrow_mut() = Some(bytes.clone());
		}
		result
	}

	fn read_bytes_full(&self) -> kaitai::KResult<Vec<u8>> {
		let offset = self.inner.pos();
		let result = self.inner.read_bytes_full();
		if let Ok(ref bytes) = result {
			*self.last_offset.borrow_mut() = Some(offset);
			*self.last_value.borrow_mut() = Some(bytes.clone());
		}
		result
	}
}
