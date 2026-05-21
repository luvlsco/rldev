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

use kaitai::{BytesReader, KStruct, KResult, OptRc};

use super::gan_parser::GanParser as GP;
use super::gan_parser::GanParser_DataSection_AnimationFrame as GPAnimFrame;

use crate::toml_formatter::{self, TomlFrameAttrs};

#[repr(u32)]
enum Frame {
	Pattern = 30100,
	X = 30101,
	Y = 30102,
	Time = 30103,
	Alpha = 30104,
	Other = 30105,
	FrameEnd = 999999,
}

impl TryFrom<u32> for Frame {
	type Error = ();

	fn try_from(v: u32) -> Result<Self, ()> {
		match v {
			30100 => Ok(Frame::Pattern),
			30101 => Ok(Frame::X),
			30102 => Ok(Frame::Y),
			30103 => Ok(Frame::Time),
			30104 => Ok(Frame::Alpha),
			30105 => Ok(Frame::Other),
			999999 => Ok(Frame::FrameEnd),
			_ => Err(()),
		}
	}
}

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
	fn from_frame(frame: &GPAnimFrame) -> Self {
		frame.entries().iter().fold(FrameAttrs::default(), |mut attrs, entry_rc| {
			let entry = entry_rc.get();
			let tag = *entry.tag();
			if let Ok(t) = Frame::try_from(tag) {
				attrs.set_attr(t, *entry.value());
			}
			attrs
		})
	}

	/// Sets an attribute value based on its tag.
	fn set_attr(&mut self, tag: Frame, value: i32) {
		match tag {
			Frame::Pattern => self.pattern = Some(value),
			Frame::X => self.x = Some(value),
			Frame::Y => self.y = Some(value),
			Frame::Time => self.time = Some(value),
			Frame::Alpha => self.alpha = Some(value),
			Frame::Other => self.other = Some(value),
			Frame::FrameEnd => (),
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
pub fn parse_gan(path: &str) -> KResult<OptRc<GP>> {
	let reader = BytesReader::open(path)?;
	let gan = GP::read_into::<_, GP>(&reader, None, None)?;
	Ok(gan)
}

/// Converts a GAN animation file to TOML format.
pub fn gan_to_toml(path: &str) -> KResult<String> {
	let gan = parse_gan(path)?;
	let header = gan.header().get();
	let data_section = gan.data_section().get();

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
