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
    KaitaiWithContext {
        err: kaitai::KError,
        context: Option<ReadContext>,
    },
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
        ParseError::Kaitai(kaitai::KError::IoError {
            msg: format_io_error(&e),
        })
    }
}

/// Formats a value as `<decimal> (0x<hex>, bytes: <hex bytes>)`.
pub fn format_value(value: i32, bytes: &[u8], uppercase: bool) -> String {
    format!(
        "{} ({}, bytes: {})",
        value,
        binary_reader::format_hex_u32(value as u32, uppercase),
        binary_reader::format_bytes_hex(bytes, uppercase)
    )
}

/// Formats a 16-byte hex dump at `offset` with a caret under `field_len` bytes.
pub fn format_dump(path: &str, offset: usize, field_len: usize, uppercase: bool) -> Option<String> {
    let dump_start = offset & !0xF;
    let dump_len = 16usize;
    let size = std::fs::metadata(path).ok()?.len();
    let header = format!(
        "Dump ({} of {} bytes shown, starting at offset {}, error at offset {}):",
        dump_len,
        size,
        binary_reader::format_hex_u32_padded(dump_start as u32, 8, uppercase),
        binary_reader::format_hex_u32_padded(offset as u32, 8, uppercase)
    );
    let (line, caret) = binary_reader::hex_dump_at(path, dump_start, dump_len, offset, field_len, uppercase).ok()?;
    Some(format!("{}\n{}\n{}", header, line, caret))
}

/// Formats a single-value magic-number validation error with hex dump.
pub fn format_magic(spec: MagicSpec, path: &str, verbose: bool, uppercase: bool) -> String {
    let expected_bytes = le_bytes(spec.expected);
    let (got, got_bytes) = match binary_reader::read_u4_le(path, spec.offset) {
        Ok((v, b)) => (v as i32, b),
        Err(_) => {
            return format!("invalid value at {}: (could not re-read file).", spec.label);
        }
    };

    if !verbose {
        return format!("invalid value at {}: found {} (expected {}).", spec.label, got, spec.expected);
    }

    let out = format!(
        indoc::indoc! {"\
			invalid value at {label}:
			 Expected: {expected} ({exp_hex}, bytes: {exp_hex_bytes})
			 Found: {got} ({got_hex}, bytes: {got_hex_bytes})
			 Kaitai Error: {kind:?} @ {src}
			 Hex offset: {} (decimal: {offset})
		"},
        binary_reader::format_hex_u32_padded(spec.offset as u32, 8, uppercase),
        label = spec.label,
        expected = spec.expected,
        got = got,
        kind = spec.kind,
        src = spec.src_path,
        offset = spec.offset,
        exp_hex = binary_reader::format_hex_u32(spec.expected as u32, uppercase),
        exp_hex_bytes = binary_reader::format_bytes_hex(&expected_bytes, uppercase),
        got_hex = binary_reader::format_hex_u32(got as u32, uppercase),
        got_hex_bytes = binary_reader::format_bytes_hex(&got_bytes, uppercase),
    );

    if let Some(dump) = format_dump(path, spec.offset, 4, uppercase) {
        let mut result = out;
        result.push('\n');
        result.push_str(&dump);
        result
    } else {
        out
    }
}

/// Formats an "any of" enum validation error with hex dump.
pub fn format_any_of(spec: AnyOfSpec, path: &str, verbose: bool, uppercase: bool) -> String {
    if !verbose {
        let any_list_short: Vec<String> = spec.any_of.iter().map(|v| v.to_string()).collect();
        let list_str = any_list_short.join(", ");
        return match spec.got {
            Some((g, _)) => format!(
                "invalid value at {}: found {} (expected any of: {}).",
                spec.label, g, list_str
            ),
            None => format!("invalid value at {} (expected any of: {}).", spec.label, list_str),
        };
    }

    let mut out = format!("invalid value at {}:\n", spec.label);
    out.push_str("Expected any of:\n");
    for v in spec.any_of {
        out.push_str(&format!(" - {}\n", format_value(*v, &le_bytes(*v), uppercase)));
    }

    if let Some((g, bytes)) = &spec.got {
        out.push('\n');
        out.push_str(&format!("Found: {}\n", format_value(*g, bytes, uppercase)));
    }

    out.push_str(&format!("Kaitai Error: {:?} @ {}\n", spec.kind, spec.src_path));

    if let Some(offset) = spec.offset {
        out.push_str(&format!(
            "Hex offset: {} (decimal: {})\n",
            binary_reader::format_hex_u32_padded(offset as u32, 8, uppercase),
            offset
        ));
        if let Some(dump) = format_dump(path, offset, 4, uppercase) {
            out.push('\n');
            out.push_str(&dump);
        }
    }

    out.trim_end().to_string()
}

/// Converts an `i32` to its little-endian byte representation.
fn le_bytes(value: i32) -> [u8; 4] {
    (value as u32).to_le_bytes()
}

/// Translates a raw Kaitai Error into a human-readable message.
fn format_kaitai_error(err: &kaitai::KError) -> String {
    use kaitai::KError;
    match err {
        KError::IoError { .. } => "cannot read file: Input/output error.".to_string(),
        KError::Eof { requested, available } => format!("file is truncated (expected {} bytes, found {}).", requested, available),
        KError::NoTerminatorFound => "expected string terminator not found.".to_string(),
        KError::EmptyIterator => "expected data but found none.".to_string(),
        KError::UnknownEncoding { name } => format!("unknown encoding: {}.", name),
        KError::ReadBitsTooLarge { requested } => format!("read bits too large ({}).", requested),
        KError::MissingRoot | KError::MissingParent => "internal error: missing reference.".to_string(),
        KError::BytesDecodingError { msg } => {
            format!("bytes decoding error: {}.", msg.trim_end_matches('.'))
        }
        KError::CastError => "internal cast error.".to_string(),
        KError::UndecidedEndianness { .. } => "internal error: undecided endianness.".to_string(),
        _ => "an unknown error occurred.".to_string(),
    }
}

/// Translates a parse error (Kaitai with or without read context) into a human-readable message.
pub fn format_parse_error(err: &ParseError) -> String {
    let kerr = match err {
        ParseError::Kaitai(k) => k,
        ParseError::KaitaiWithContext { err: k, .. } => k,
    };
    format_kaitai_error(kerr)
}

/// Formats a TOML-to-binary conversion error for display.
/// Non-verbose mode shows only the first line; verbose mode shows full context.
pub fn format_write_error(msg: &str, verbose: bool) -> String {
    if !verbose {
        return msg
            .lines()
            .next()
            .map(|l| format!("{}.", l.trim()))
            .unwrap_or_else(|| msg.to_string());
    }

    let mut result = String::new();
    let mut last_line = "";
    for line in msg.lines() {
        if !result.is_empty() {
            result.push('\n');
        }

        if line.trim().is_empty() {
            continue;
        }

        last_line = line;
        result.push_str(line);
    }

    if !result.contains('\n') {
        if !last_line.is_empty() {
            return format!("{}.", last_line);
        }

        return last_line.to_string();
    }

    if let Some(pos) = result.find('\n') {
        result.insert(pos, ':');
    }

    if !last_line.is_empty() {
        let fixed = rldev::common::cli::format_output(last_line);
        result = result.trim_end_matches(last_line).to_string();
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }

        result.push_str(&fixed);
    }

    result
}

/// Formats an I/O error with a POSIX-like English description.
pub fn format_io_error(err: &std::io::Error) -> String {
    let message = match err.kind() {
        std::io::ErrorKind::NotFound => "No such file or directory",
        std::io::ErrorKind::PermissionDenied => "Permission denied",
        std::io::ErrorKind::AlreadyExists => "File exists",
        std::io::ErrorKind::BrokenPipe => "Broken pipe",
        std::io::ErrorKind::WouldBlock => "Resource temporarily unavailable",
        std::io::ErrorKind::NotADirectory => "Not a directory",
        std::io::ErrorKind::IsADirectory => "Is a directory",
        std::io::ErrorKind::DirectoryNotEmpty => "Directory not empty",
        std::io::ErrorKind::ReadOnlyFilesystem => "Read-only file system",
        std::io::ErrorKind::InvalidInput => "Invalid argument",
        std::io::ErrorKind::InvalidData => "Invalid data",
        std::io::ErrorKind::TimedOut => "Connection timed out",
        std::io::ErrorKind::WriteZero => "No space left on device",
        std::io::ErrorKind::StorageFull => "No space left on device",
        std::io::ErrorKind::NotSeekable => "Illegal seek",
        std::io::ErrorKind::QuotaExceeded => "Disk quota exceeded",
        std::io::ErrorKind::FileTooLarge => "File too large",
        std::io::ErrorKind::ResourceBusy | std::io::ErrorKind::ExecutableFileBusy => "Device or resource busy",
        std::io::ErrorKind::Deadlock => "Resource deadlock avoided",
        std::io::ErrorKind::CrossesDevices => "Invalid cross-device link",
        std::io::ErrorKind::TooManyLinks => "Too many links",
        std::io::ErrorKind::Interrupted => "Interrupted system call",
        std::io::ErrorKind::Unsupported => "Operation not supported",
        std::io::ErrorKind::UnexpectedEof => "Unexpected end of file",
        std::io::ErrorKind::OutOfMemory => "Cannot allocate memory",
        _ => "Input/output error",
    };

    match err.raw_os_error() {
        Some(code) => format!("{} (os error {})", message, code),
        None => message.to_string(),
    }
}
