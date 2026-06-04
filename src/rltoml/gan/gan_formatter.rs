/*
 RlToml: GAN to TOML formatting utilities
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

use crate::error_formatter::{self, AnyOfSpec, MagicSpec, ParseError, ParseResult};
use crate::toml_formatter::{self, TomlFrameAttrs};

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
	/// Iterates all frame attribute (name, value) pairs, including unset fields as `None`.
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

	/// Builds `FrameAttrs` by mapping each entry tag in a parsed animation frame to its corresponding field.
	fn from_frame(frame: &GanAnimFrame) -> Self {
		frame.entries().iter().fold(FrameAttrs::default(), |mut attrs, entry_rc| {
			let entry = entry_rc.get();
			attrs.set_attr(entry.tag().clone(), *entry.value());
			attrs
		})
	}

	/// Assigns a value to the matching field for the given frame tag, ignoring `FrameEnd` and `Unknown`.
	fn set_attr(&mut self, tag: GanFrame, value: i32) {
		match tag {
			GanFrame::Pattern => self.pattern = Some(value),
			GanFrame::X => self.x = Some(value),
			GanFrame::Y => self.y = Some(value),
			GanFrame::Time => self.time = Some(value),
			GanFrame::Alpha => self.alpha = Some(value),
			GanFrame::Other => self.other = Some(value),
			GanFrame::FrameEnd | GanFrame::Unknown(_) => (),
		}
	}
}

impl TomlFrameAttrs for FrameAttrs {
	/// Converts set attributes into key-value pairs for inline table output, skipping `None` fields.
	fn to_inline_table_fields(&self) -> Vec<(String, String)> {
		self.iter_fields()
			.filter_map(|(name, value)| {
				value.map(|v| (name.to_string(), v.to_string()))
			})
			.collect()
	}

	/// Returns a new `FrameAttrs` with only the fields that differ from `defaults`.
	fn diff_from(&self, defaults: &Self) -> FrameAttrs {
		FrameAttrs {
			pattern: diff_opt(self.pattern, defaults.pattern),
			x: diff_opt(self.x, defaults.x),
			y: diff_opt(self.y, defaults.y),
			time: diff_opt(self.time, defaults.time),
			alpha: diff_opt(self.alpha, defaults.alpha),
			other: diff_opt(self.other, defaults.other),
		}
	}

	/// Returns the set of attributes that are identical across all frames, discarding any that vary.
	fn common_attrs(frames: &[FrameAttrs]) -> FrameAttrs {
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
}

/// Parses a GAN file from disk using Kaitai Struct binary parser.
pub fn parse_gan(path: &str) -> ParseResult<OptRc<GanParser>> {
	let reader = crate::binary_reader::TrackingReader::open(path)?;
	GanParser::read_into::<_, GanParser>(&reader, None, None).map_err(|err| {
		let context = reader.read_context();
		ParseError::kaitai_with_context(err, context)
	})
}

/// Converts a GAN animation file to TOML format.
pub fn gan_to_toml(path: &str) -> ParseResult<String> {
	let gan = parse_gan(path)?;
	let header = gan.gan_header().get();
	let data_section = gan.gan_data_section().get();

	let mut lines = vec![
		"[gan]".to_string(),
		format!("bitmap = \"{}\"", header.bitmap_name()),
		String::new(),
	];

	for set_rc in data_section.sets().iter() {
		let set = &set_rc.get();
		let frames: Vec<FrameAttrs> = set.frames().iter().map(|rc| FrameAttrs::from_frame(&rc.get())).collect();
		let defaults = FrameAttrs::common_attrs(&frames);

		lines.push("[[gan.set]]".to_string());
		lines.extend(toml_formatter::write_block_fields(&defaults));
		lines.push("frames = [".to_string());

		for frame in frames {
			let diff = frame.diff_from(&defaults);
			let fields = diff.to_inline_table_fields();
			lines.push(format!("  {},", toml_formatter::build_inline_table(&fields)));
		}

		lines.push("]".to_string());
		lines.push(String::new());
	}

	Ok(lines.join("\n"))
}

/// Formats a Kaitai validation error with context, expected/found values, and hex dump.
pub fn format_gan_to_toml_error(err: &ParseError, path: &str, verbose: bool) -> String {
	let kerr = match err {
		ParseError::Kaitai(k) => k,
		ParseError::KaitaiWithContext { err: k, .. } => k,
	};
	let KError::ValidationFailed(validation) = kerr else {
		return format!("{:?}", err);
	};
	let src = validation.src_path.as_str();
	let kind = &validation.kind;
	let context = err.read_context();
	let (got, offset) = crate::binary_reader::context_to_got_offset(context);

	match src {
		"/types/gan_header/seq/0" => error_formatter::format_magic(
			MagicSpec { label: "first GAN header", expected: 10_000, offset: 0, kind, src_path: src },
			path,
			verbose,
		),
		"/types/gan_header/seq/1" => error_formatter::format_magic(
			MagicSpec { label: "second GAN header", expected: 10_000, offset: 4, kind, src_path: src },
			path,
			verbose,
		),
		"/types/gan_header/seq/2" => error_formatter::format_magic(
			MagicSpec { label: "third GAN header", expected: 10_100, offset: 8, kind, src_path: src },
			path,
			verbose,
		),
		"/types/gan_data_section/seq/0" => match compute_data_section_offset(path) {
			Some(off) => error_formatter::format_magic(
				MagicSpec { label: "data section start marker", expected: 20_000, offset: off, kind, src_path: src },
				path,
				verbose,
			),
			None => "invalid data section start marker (expected 20000)".to_string(),
		},
		"/types/gan_data_section/types/animation_set/seq/0" => match compute_set_marker_offset(path) {
			Some(off) => error_formatter::format_magic(
				MagicSpec { label: "animation set start marker", expected: 30_000, offset: off, kind, src_path: src },
				path,
				verbose,
			),
			None => "invalid animation set start marker (expected 30000)".to_string(),
		},
		"/types/gan_data_section/types/frame_entry/seq/0" => error_formatter::format_any_of(
			AnyOfSpec { label: "frame entry tag", any_of: &valid_frame_tags(), kind, src_path: src, got, offset },
			path,
			verbose,
		),
		_ => "parse failed at an unexpected location".to_string(),
	}
}

/// All valid frame entry tag values, including `FrameEnd`.
fn valid_frame_tags() -> [i32; 7] {
	[
		i64::from(&GanFrame::Pattern) as i32,
		i64::from(&GanFrame::X) as i32,
		i64::from(&GanFrame::Y) as i32,
		i64::from(&GanFrame::Time) as i32,
		i64::from(&GanFrame::Alpha) as i32,
		i64::from(&GanFrame::Other) as i32,
		i64::from(&GanFrame::FrameEnd) as i32,
	]
}

/// Computes the expected offset of the data section start marker (20000).
fn compute_data_section_offset(path: &str) -> Option<usize> {
	crate::binary_reader::read_u4_le_at(path, 12).ok().map(|n| 16 + n as usize).filter(|&o| o != 0)
}

/// Computes the expected offset of the animation set start marker (30000).
fn compute_set_marker_offset(path: &str) -> Option<usize> {
	crate::binary_reader::read_u4_le_at(path, 12)
		.ok()
		.map(|n| 16 + n as usize + 8)
		.filter(|&o| o != 0)
}

/// Returns `a` if it differs from `b`, otherwise `None`.
fn diff_opt(a: Option<i32>, b: Option<i32>) -> Option<i32> {
	a.filter(|v| Some(*v) != b)
}

/// Returns `a` if it equals `b`, otherwise `None`.
fn keep_if_eq(a: Option<i32>, b: Option<i32>) -> Option<i32> {
	a.filter(|v| Some(*v) == b)
}
