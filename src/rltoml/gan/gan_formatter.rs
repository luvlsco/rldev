/*
 RlToml: GAN to TOML format handling
 Copyright (C) 2026 luvlsco

 Based on RlXml, originally developed in OCaml by:
  Copyright (C) 2006 Haeleth

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

use kaitai::{KError, KStruct, OptRc};

use super::gan_parser::GanParser;
use super::gan_parser::GanParser_Frame as GanFrame;
use super::gan_parser::GanParser_GanDataSection_AnimationFrame as GanAnimFrame;

use crate::toml_formatter::{self, TomlFrameAttrs};
use crate::binary_reader::{self, ParseError, ParseResult};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct FrameAttrs {
	pattern: Option<i32>,
	x: Option<i32>,
	y: Option<i32>,
	time: Option<i32>,
	alpha: Option<i32>,
	other: Option<i32>,
}

impl FrameAttrs {
	/// Returns an iterator over all (field name, value) pairs in insertion order.
	fn iter_fields(&self) -> impl Iterator<Item = (&'static str, Option<i32>)> {
		[
			("pattern", self.pattern),
			("x", self.x),
			("y", self.y),
			("time", self.time),
			("alpha", self.alpha),
			("other", self.other),
		]
		.into_iter()
	}

	/// Constructs frame attributes from a Kaitai-parsed animation frame.
	fn from_frame(frame: &GanAnimFrame) -> Self {
		frame
			.entries()
			.iter()
			.fold(FrameAttrs::default(), |mut attrs, entry_rc| {
				let entry = entry_rc.get();
				let tag = entry.tag().clone();
				let value = *entry.value();
				attrs.set_attr(tag, value);
				attrs
			})
	}

	/// Sets an attribute value based on its tag.
	fn set_attr(&mut self, tag: GanFrame, value: i32) {
		match tag {
			GanFrame::Pattern => self.pattern = Some(value),
			GanFrame::X => self.x = Some(value),
			GanFrame::Y => self.y = Some(value),
			GanFrame::Time => self.time = Some(value),
			GanFrame::Alpha => self.alpha = Some(value),
			GanFrame::Other => self.other = Some(value),
			GanFrame::FrameEnd => (),
			GanFrame::Unknown(_) => (),
		}
	}
}

impl TomlFrameAttrs for FrameAttrs {
	/// Converts frame attributes to inline TOML table fields.
	fn to_inline_table_fields(&self) -> Vec<(String, String)> {
		self.iter_fields()
			.filter_map(|(name, value)| {
				value.map(|v| {
					let formatted = match name {
						"pattern" => format!("\"{}\"", v),
						_ => v.to_string(),
					};
					(name.to_string(), formatted)
				})
			})
			.collect()
	}

	/// Computes the difference from a default set of attributes.
	fn diff_from(&self, defaults: &FrameAttrs) -> FrameAttrs {
		FrameAttrs {
			pattern: diff_opt(self.pattern, defaults.pattern),
			x: diff_opt(self.x, defaults.x),
			y: diff_opt(self.y, defaults.y),
			time: diff_opt(self.time, defaults.time),
			alpha: diff_opt(self.alpha, defaults.alpha),
			other: diff_opt(self.other, defaults.other),
		}
	}
}

/// Parses a GAN file from disk using Kaitai Struct binary parser.
pub fn parse_gan(path: &str) -> ParseResult<OptRc<GanParser>> {
	let reader = binary_reader::TrackingReader::open(path)?;
	match GanParser::read_into::<_, GanParser>(&reader, None, None) {
		Ok(gan) => Ok(gan),
		Err(err) => {
			let ctx = reader
				.last_read_offset()
				.zip(reader.last_read_value())
				.map(|(offset, value)| binary_reader::ReadContext { offset, value });
			Err(ParseError::kaitai_with_context(err, ctx))
		}
	}
}

/// Converts a GAN animation file to TOML format.
pub fn gan_to_toml(path: &str) -> ParseResult<String> {
	let gan = parse_gan(path)?;
	let header = gan.gan_header().get();
	let data_section = gan.gan_data_section().get();

	let mut lines = Vec::new();
	lines.push("[gan]".to_string());
	lines.push(format!("bitmap = \"{}\"", header.bitmap_name()));
	lines.push(String::new());

	for set_rc in data_section.sets().iter() {
		let set = &set_rc.get();
		let frames: Vec<FrameAttrs> = set
			.frames()
			.iter()
			.map(|rc| FrameAttrs::from_frame(&rc.get()))
			.collect();
		let defaults = detect_common_attrs(&frames);

		lines.push("[[gan.set]]".to_string());
		lines.extend(toml_formatter::write_block_fields(&defaults));
		lines.push("frames = [".to_string());

		for frame in frames {
			let diff = frame.diff_from(&defaults);
			let fields = diff.to_inline_table_fields();
			let inline_table = toml_formatter::build_inline_table(&fields);
			lines.push(format!("  {},", inline_table));
		}

		lines.push("]".to_string());
		lines.push(String::new());
	}

	Ok(lines.join("\n"))
}

pub fn format_gan_error(err: &ParseError, path: &str, verbose: bool) -> String {
	let validation = match err {
		ParseError::Kaitai(KError::ValidationFailed(v)) => v,
		ParseError::KaitaiWithContext(KError::ValidationFailed(v), _) => v,
		_ => return format!("{:?}", err),
	};
	let src = validation.src_path.as_str();
	let kind = &validation.kind;
	let ctx = err.read_context();

	match src {
		"/types/gan_header/seq/0" => format_magic_error("first GAN header", 10000, path, 0, kind, src, verbose,),

		"/types/gan_header/seq/1" => format_magic_error("second GAN header", 10000, path, 4, kind, src, verbose,),

		"/types/gan_header/seq/2" => format_magic_error("third GAN header", 10100, path, 8, kind, src, verbose,),

		"/types/gan_data_section/seq/0" => {
			let offset = binary_reader::read_u4_le_at(path, 12)
				.map(|name_len| 16 + name_len as usize)
				.unwrap_or(0);
			if offset == 0 {
				"invalid data section start marker (expected 20000)".to_string()
			} else {
				format_magic_error("data section start marker", 20000, path, offset, kind, src, verbose)
			}
		}

		"/types/gan_data_section/types/animation_set/seq/0" => {
			let offset = binary_reader::read_u4_le_at(path, 12)
				.map(|name_len| 16 + name_len as usize + 8)
				.unwrap_or(0);
			if offset == 0 {
				"invalid animation set start marker (expected 30000)".to_string()
			} else {
				format_magic_error("animation set start marker", 30000, path, offset, kind, src, verbose)
			}
		}

		"/types/gan_data_section/types/frame_entry/seq/0" => {
			format_frame_entry_error(ctx, kind, src, path, verbose)
		}

		_ => "parse failed at an unexpected location".to_string(),
	}
}

fn format_magic_error(
	label: &'static str,
	expected: i64,
	path: &str,
	offset: usize,
	kind: &kaitai::ValidationKind,
	src_path: &str,
	verbose: bool,
) -> String {
	let (got, got_bytes) = match binary_reader::read_u4_le_full(path, offset) {
		Ok(v) => v,
		Err(_) => {
			return format!(
				"invalid value at {}: (could not re-read file)",
				label
			);
		}
	};
	let expected_bytes = (expected as u32).to_le_bytes();

	if !verbose {
		return format!(
			"invalid value at {}: found {} (expected {})",
			label, got, expected
		);
	}

	let dump_start = offset & !0xF;
	let dump_len = 16;

	let dump_header = match binary_reader::file_size(path) {
		Ok(size) => format!(
			"Dump ({} of {} bytes shown, starting at offset 0x{:08X}, error at offset 0x{:08X}):",
			dump_len, size, dump_start, offset
		),
		Err(_) => format!(
			"Dump (starting at offset 0x{:08X}, error at offset 0x{:08X}):",
			dump_start, offset
		),
	};

	let (dump, caret) = binary_reader::hex_dump_at(path, dump_start, dump_len, offset, 4)
		.unwrap_or_else(|_| (String::new(), String::new()));

	format!(
		"invalid value at {label}:\n\
		 Expected: {expected} (0x{expected:X}, bytes: {exp_hex})\n\
		 Found: {got} (0x{got:X}, bytes: {got_hex})\n\
		 Kaitai Error: {kind:?} @ {src_path}\n\
		 Error offset: 0x{offset:08X} (byte: {offset})\n\
		 \n\
		 {dump_header}\n\
		 {dump}\n\
		 {caret}",
		exp_hex = binary_reader::format_bytes_hex(&expected_bytes),
		got_hex = binary_reader::format_bytes_hex(&got_bytes),
	)
}

fn valid_frame_tags() -> [i64; 7] {
	[
		i64::from(&GanFrame::Pattern),
		i64::from(&GanFrame::X),
		i64::from(&GanFrame::Y),
		i64::from(&GanFrame::Time),
		i64::from(&GanFrame::Alpha),
		i64::from(&GanFrame::Other),
		i64::from(&GanFrame::FrameEnd),
	]
}

fn format_tag_entry(tag: i64) -> String {
	let bytes = (tag as u32).to_le_bytes();
	format!("{} (0x{:X}, bytes: {})", tag, tag, binary_reader::format_bytes_hex(&bytes))
}

fn format_frame_entry_error(
	ctx: Option<&binary_reader::ReadContext>,
	kind: &kaitai::ValidationKind,
	src_path: &str,
	path: &str,
	verbose: bool,
) -> String {
	let tags = valid_frame_tags();
	let any_list_short = tags
		.iter()
		.map(|t| t.to_string())
		.collect::<Vec<_>>()
		.join(", ");

	let (got, got_bytes): (Option<i64>, Option<Vec<u8>>) = match ctx {
		Some(c) if c.value.len() == 4 => {
			let mut arr = [0u8; 4];
			arr.copy_from_slice(&c.value);
			(Some(u32::from_le_bytes(arr) as i64), Some(c.value.clone()))
		}
		_ => (None, None),
	};

	if !verbose {
		return match got {
			Some(g) => format!(
				"invalid value at frame entry tag: found {} (expected any of: {})",
				g, any_list_short
			),
			None => format!(
				"invalid value at frame entry tag (expected any of: {})",
				any_list_short
			),
		};
	}

	let mut out = String::from("invalid value at frame entry tag:\n");
	out.push_str("Expected any of:\n");
	for t in tags.iter() {
		out.push_str(&format!(" - {}\n", format_tag_entry(*t)));
	}

	if let (Some(g), Some(bytes)) = (got, got_bytes) {
		out.push('\n');
		out.push_str(&format!(
			"Found: {} (0x{:X}, bytes: {})\n",
			g,
			g,
			binary_reader::format_bytes_hex(&bytes)
		));
	}

	out.push_str(&format!("Kaitai Error: {:?} @ {}\n", kind, src_path));

	if let (Some(_), Some(c)) = (got, ctx) {
		out.push_str(&format!(
			"Error offset: 0x{:08X} (byte: {})\n",
			c.offset, c.offset
		));
		out.push('\n');

		let dump_start = c.offset & !0xF;
		let dump_len = 16usize;
		let dump_header = match binary_reader::file_size(path) {
			Ok(size) => format!(
				"Dump ({} of {} bytes shown, starting at offset 0x{:08X}, error at offset 0x{:08X}):",
				dump_len, size, dump_start, c.offset
			),
			Err(_) => format!(
				"Dump (starting at offset 0x{:08X}, error at offset 0x{:08X}):",
				dump_start, c.offset
			),
		};
		let field_len = 4;
		let (dump, caret) = binary_reader::hex_dump_at(path, dump_start, dump_len, c.offset, field_len)
			.unwrap_or_else(|_| (String::new(), String::new()));
		out.push_str(&format!("{}\n{}\n{}\n", dump_header, dump, caret));
	}

	out
}

/// Returns `a` if it differs from `b`, otherwise `None`.
fn diff_opt(a: Option<i32>, b: Option<i32>) -> Option<i32> {
	a.filter(|v| Some(*v) != b)
}

/// Returns `a` if it equals `b`, otherwise `None`.
fn keep_if_eq(a: Option<i32>, b: Option<i32>) -> Option<i32> {
	a.filter(|v| Some(*v) == b)
}

/// Detects attributes that are common (identical) across all frames in a set.
fn detect_common_attrs(frames: &[FrameAttrs]) -> FrameAttrs {
	frames.iter().skip(1).fold(
		frames.first().cloned().unwrap_or_default(),
		|common, frame| FrameAttrs {
			pattern: keep_if_eq(common.pattern, frame.pattern),
			x: keep_if_eq(common.x, frame.x),
			y: keep_if_eq(common.y, frame.y),
			time: keep_if_eq(common.time, frame.time),
			alpha: keep_if_eq(common.alpha, frame.alpha),
			other: keep_if_eq(common.other, frame.other),
		},
	)
}
