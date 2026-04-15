/* 
 RlToml: GAN format handling
 Copyright (C) 2026 Lucas Velasco

 Based on RlXml, originally developed in OCaml by:
  Copyright (C) 2006 Haeleth
  Revised 2009-2011 by Richard 23

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

use kaitai::BytesReader;
use kaitai::KStruct;
use kaitai::KResult;
use kaitai::OptRc;

use super::gan_parser::GanParser as GP;
use super::gan_parser::GanParser_DataSection_AnimationFrame as GPAnimFrame;
use super::gan_parser::GanParser_DataSection_AnimationSet as GPAnimSet;

use itertools::Itertools;

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
	fn set_attr(&mut self, tag: u32, value: i32) {
		match tag {
			30_100 => self.pattern = Some(value),
			30_101 => self.x = Some(value),
			30_102 => self.y = Some(value),
			30_103 => self.time = Some(value),
			30_104 => self.alpha = Some(value),
			30_105 => self.other = Some(value),
			_ => (),
		}
	}

	fn from_frame(frame: &GPAnimFrame) -> Self {
		frame.entries().iter().fold(FrameAttrs::default(), |mut attrs, entry_rc| {
			let entry = entry_rc.get();
			let tag = *entry.tag();
			if tag != 999_999 {
				attrs.set_attr(tag, *entry.value());
			}
			attrs
		})
	}

	fn diff_from(&self, defaults: &FrameAttrs) -> FrameAttrs {
		FrameAttrs {
			pattern: (self.pattern != defaults.pattern).then_some(self.pattern).flatten(),
			x: (self.x != defaults.x).then_some(self.x).flatten(),
			y: (self.y != defaults.y).then_some(self.y).flatten(),
			time: (self.time != defaults.time).then_some(self.time).flatten(),
			alpha: (self.alpha != defaults.alpha).then_some(self.alpha).flatten(),
			other: (self.other != defaults.other).then_some(self.other).flatten(),
		}
	}
}

fn common_attrs(frames: &[FrameAttrs]) -> FrameAttrs {
	frames.iter().skip(1).fold(frames.first().cloned().unwrap_or_default(),
		|mut acc, frame| {
			if acc.pattern != frame.pattern { acc.pattern = None; }
			if acc.x != frame.x { acc.x = None; }
			if acc.y != frame.y { acc.y = None; }
			if acc.time != frame.time { acc.time = None; }
			if acc.alpha != frame.alpha { acc.alpha = None; }
			if acc.other != frame.other { acc.other = None; }
			acc
		},
	)
}

fn format_inline_table(attrs: &FrameAttrs) -> String {
	let fields = [
		attrs.pattern.map(|p| format!("pattern = \"{}\"", p)),
		attrs.x.map(|v| format!("x = {}", v)),
		attrs.y.map(|v| format!("y = {}", v)),
		attrs.time.map(|v| format!("time = {}", v)),
		attrs.alpha.map(|v| format!("alpha = {}", v)),
		attrs.other.map(|v| format!("other = {}", v)),
	].into_iter().flatten().join(", ");
	format!("{{ {} }}", fields)
}

fn format_set(set: &GPAnimSet) -> String {
	let frames: Vec<FrameAttrs> = set.frames().iter()
		.map(|rc| FrameAttrs::from_frame(&rc.get()))
		.collect();
	let defaults = common_attrs(&frames);

	let mut lines = Vec::new();
	lines.push("[[gan.set]]".to_string());
	if let Some(p) = defaults.pattern { lines.push(format!("pattern = \"{}\"", p)); }
	if let Some(v) = defaults.x { lines.push(format!("x = {}", v)); }
	if let Some(v) = defaults.y { lines.push(format!("y = {}", v)); }
	if let Some(v) = defaults.time { lines.push(format!("time = {}", v)); }
	if let Some(v) = defaults.other { lines.push(format!("other = {}", v)); }
	lines.push("frames = [".to_string());

	lines.extend(frames.iter()
		.map(|frame| format!("  {},", format_inline_table(&frame.diff_from(&defaults))))
	);

	lines.push("]".to_string());
	lines.join("\n")
}

pub fn parse_gan(path: &str) -> KResult<OptRc<GP>> {
	let reader = BytesReader::open(path)?;
	let gan = GP::read_into::<_, GP>(&reader, None, None)?;
	Ok(gan)
}

pub fn gan_to_toml(path: &str) -> KResult<String> {
	let gan = parse_gan(path)?;
	let header = gan.header().get();
	let data_section = gan.data_section().get();

	let mut lines = Vec::new();
	lines.push("[gan]".to_string());
	lines.push(format!("bitmap = \"{}\"", header.bitmap_name()));
	lines.push(String::new());

	for set_rc in data_section.sets().iter() {
		lines.push(format_set(&set_rc.get()));
		lines.push(String::new());
	}

	Ok(lines.join("\n"))
}
