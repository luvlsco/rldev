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

use kaitai::{BytesReader, KStruct, KResult, KError, OptRc};

use super::gan_parser::GanParser;
use super::gan_parser::GanParser_Frame as GanFrame;
use super::gan_parser::GanParser_GanDataSection_AnimationFrame as GanAnimFrame;

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
		frame.entries().iter().fold(FrameAttrs::default(), |mut attrs, entry_rc| {
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
pub fn parse_gan(path: &str) -> KResult<OptRc<GanParser>> {
	let reader = BytesReader::open(path)?;
	let gan = GanParser::read_into::<_, GanParser>(&reader, None, None)?;
	Ok(gan)
}

/// Converts a GAN animation file to TOML format.
pub fn gan_to_toml(path: &str) -> KResult<String> {
	let gan = parse_gan(path)?;
	let header = gan.gan_header().get();
	let data_section = gan.gan_data_section().get();

	let mut lines = Vec::new();
	lines.push("[gan]".to_string());
	lines.push(format!("bitmap = \"{}\"", header.bitmap_name()));
	lines.push(String::new());

	for set_rc in data_section.sets().iter() {
		let set = &set_rc.get();
		let frames: Vec<FrameAttrs> = set.frames().iter()
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

pub fn format_gan_error(err: &kaitai::KError, verbose: bool) -> String {
	match err {
		KError::ValidationFailed(e) => {
			let error_message = match e.src_path.as_str() {
				"/types/gan_header/seq/0" =>
				"invalid value at first GAN header (expected 10000) - maybe it's not a GAN file?",

				"/types/gan_header/seq/1" =>
				"invalid value at second GAN header (expected 10000) - maybe it's not a GAN file?",
				
				"/types/gan_header/seq/2" =>
				"invalid value at third GAN header (expected 10100) - maybe it's not a GAN file?",
				
				"/types/gan_data_section/seq/0" =>
				"invalid data section start marker (expected 20000)",

				"/types/gan_data_section/types/animation_set/seq/0" =>
				"invalid animation set start marker (expected 30000)",

				"/types/gan_data_section/types/frame_entry/seq/0" =>
				"unknown GAN frame entry tag (expected 30100, 30101, 30102, 30103, 30104, 30105, or 999999)",

				_ => "parse failed at an unexpected location, enable verbose mode for details",
			};

			let error_details = format!("KError::ValidationFailed: [{:?} @ {}]", e.kind, e.src_path);

			if verbose {
				format!("{}\n{}", error_message, error_details)
			} else {
				error_message.to_string()
			}
		}
		_ => format!("{:?}", err),
	}
}
