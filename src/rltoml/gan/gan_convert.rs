/*
 RlToml: TOML to GAN convertion utilities
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

use std::fs::File;
use std::io::{BufWriter, Write};
use byteorder::{LittleEndian, WriteBytesExt};

use super::gan_parser::GanParser_Frame as GanFrame;
use super::FrameAttrs;

/// A frame parsed from TOML, containing override values for set defaults.
#[derive(Default, Debug, Clone)]
struct TomlFrame {
	params: FrameAttrs,
}

/// A set of animation frames with shared default attribute values.
#[derive(Default, Debug, Clone)]
struct TomlSet {
	defaults: FrameAttrs,
	frames: Vec<TomlFrame>,
}

/// The complete TOML representation of a GAN file.
#[derive(Default, Debug, Clone)]
struct TomlGan {
	bitmap: String,
	sets: Vec<TomlSet>,
}

#[derive(Debug, thiserror::Error)]
pub enum GanWriteError {
	#[error("{0}")]
	Io(#[from] std::io::Error),
	#[error("{0}")]
	TomlParse(#[from] toml_edit::TomlError),
	#[error("{0}")]
	InvalidStructure(String),
}

/// Formats a TOML-to-GAN conversion error for display.
/// Non-verbose mode shows only the first line; verbose mode shows full context.
pub fn format_toml_to_gan_error(err: &GanWriteError, verbose: bool) -> String {
	let full_msg = err.to_string();

	if !verbose {
		return full_msg.lines().next().map(|l| format!("{}.", l.trim())).unwrap_or_else(|| full_msg.clone());
	}

	let mut result = String::new();
	let mut last_line = "";
	for line in full_msg.lines() {
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
		let first = last_line.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
		let rest = &last_line[last_line.char_indices().next().unwrap().0 + first.len()..];
		let fixed = if last_line.ends_with('.') {
			format!("{}{}", first, rest)
		} else {
			format!("{}{}.", first, rest)
		};
		result = result.trim_end_matches(last_line).to_string();
		if !result.is_empty() && !result.ends_with('\n') {
			result.push('\n');
		}
		result.push_str(&fixed);
	}

	result
}

/// Converts a TOML file to a GAN binary file.
pub fn toml_to_gan(toml_path: &str, gan_path: &str) -> Result<(), GanWriteError> {
	let content = std::fs::read_to_string(toml_path)?;
	let doc: toml_edit::Document<std::string::String> = content.parse()?;
	let gan = parse_toml_gan(&doc)?;
	let file = File::create(gan_path)?;
	let mut oc = BufWriter::new(file);
	write_gan(&mut oc, &gan)?;
	oc.flush()?;
	Ok(())
}

/// Parses a single frame from a TOML inline table.
fn parse_frame_from_inline_table(table: &toml_edit::InlineTable) -> Result<TomlFrame, GanWriteError> {
	let mut frame = TomlFrame::default();
	for (key, value) in table.iter() {
		let v = value
			.as_integer()
			.ok_or_else(|| GanWriteError::InvalidStructure(format!("frame field '{}' must be an integer", key)))?;
		match key {
			"pattern" => frame.params.pattern = Some(v as i32),
			"x" => frame.params.x = Some(v as i32),
			"y" => frame.params.y = Some(v as i32),
			"time" => frame.params.time = Some(v as i32),
			"alpha" => frame.params.alpha = Some(v as i32),
			"other" => frame.params.other = Some(v as i32),
			_ => return Err(GanWriteError::InvalidStructure(format!("unknown frame field '{}'", key))),
		}
	}
	Ok(frame)
}

/// Parses a set table from TOML, including default attributes and all frames.
fn parse_set_from_table(table: &toml_edit::Table) -> Result<TomlSet, GanWriteError> {
	let mut set = TomlSet::default();
	set.defaults.pattern = table.get("pattern").and_then(|i| i.as_integer()).map(|v| v as i32);
	set.defaults.x = table.get("x").and_then(|i| i.as_integer()).map(|v| v as i32);
	set.defaults.y = table.get("y").and_then(|i| i.as_integer()).map(|v| v as i32);
	set.defaults.time = table.get("time").and_then(|i| i.as_integer()).map(|v| v as i32);
	set.defaults.alpha = table.get("alpha").and_then(|i| i.as_integer()).map(|v| v as i32);
	set.defaults.other = table.get("other").and_then(|i| i.as_integer()).map(|v| v as i32);

	let frames_item = table
		.get("frames")
		.ok_or_else(|| GanWriteError::InvalidStructure("set missing 'frames' array".to_string()))?;

	let frames_array = frames_item
		.as_array()
		.ok_or_else(|| GanWriteError::InvalidStructure("'frames' must be an array".to_string()))?;

	for item in frames_array.iter() {
		let inline_table = item
			.as_inline_table()
			.ok_or_else(|| GanWriteError::InvalidStructure("frame must be an inline table".to_string()))?;
		set.frames.push(parse_frame_from_inline_table(inline_table)?);
	}

	Ok(set)
}

/// Parses a complete TOML document into a `TomlGan` structure.
fn parse_toml_gan(doc: &toml_edit::Document<std::string::String>) -> Result<TomlGan, GanWriteError> {
	let root = doc
		.as_table()
		.get("gan")
		.ok_or_else(|| GanWriteError::InvalidStructure("missing [gan] section".to_string()))?
		.as_table()
		.ok_or_else(|| GanWriteError::InvalidStructure("[gan] must be a table".to_string()))?;

	let bitmap = root
		.get("bitmap")
		.ok_or_else(|| GanWriteError::InvalidStructure("[gan] missing 'bitmap'".to_string()))?
		.as_str()
		.ok_or_else(|| GanWriteError::InvalidStructure("'bitmap' must be a string".to_string()))?
		.to_string();

	let set_item = root
		.get("set")
		.ok_or_else(|| GanWriteError::InvalidStructure("missing [[gan.set]] entries".to_string()))?;

	let set_array = set_item
		.as_array_of_tables()
		.ok_or_else(|| GanWriteError::InvalidStructure("'set' must be an array of tables".to_string()))?;

	let mut gan = TomlGan { bitmap, sets: Vec::new() };
	for set_table in set_array.iter() {
		gan.sets.push(parse_set_from_table(set_table)?);
	}

	Ok(gan)
}

/// Writes a single frame's tag-value pairs to the binary output.
/// Uses set defaults for any attributes not specified in the frame.
fn write_frame(oc: &mut BufWriter<File>, frame: &TomlFrame, defaults: &FrameAttrs) -> Result<(), GanWriteError> {
	for (tag, value) in [
		(i64::from(&GanFrame::Pattern) as i32, frame.params.pattern.or(defaults.pattern)),
		(i64::from(&GanFrame::X) as i32, frame.params.x.or(defaults.x)),
		(i64::from(&GanFrame::Y) as i32, frame.params.y.or(defaults.y)),
		(i64::from(&GanFrame::Time) as i32, frame.params.time.or(defaults.time)),
		(i64::from(&GanFrame::Alpha) as i32, frame.params.alpha.or(defaults.alpha)),
		(i64::from(&GanFrame::Other) as i32, frame.params.other.or(defaults.other)),
	] {
		if let Some(v) = value {
			oc.write_i32::<LittleEndian>(tag)?;
			oc.write_i32::<LittleEndian>(v)?;
		}
	}
	oc.write_i32::<LittleEndian>(i64::from(&GanFrame::FrameEnd) as i32)?;
	Ok(())
}

/// Writes a set marker, frame count, and all frames in the set.
fn write_set(oc: &mut BufWriter<File>, set: &TomlSet) -> Result<(), GanWriteError> {
	oc.write_i32::<LittleEndian>(30_000)?;
	oc.write_u32::<LittleEndian>(set.frames.len() as u32)?;
	for frame in &set.frames {
		write_frame(oc, frame, &set.defaults)?;
	}
	Ok(())
}

/// Writes the complete GAN file: header, bitmap name, data section with all sets.
fn write_gan(oc: &mut BufWriter<File>, gan: &TomlGan) -> Result<(), GanWriteError> {
	oc.write_i32::<LittleEndian>(10_000)?;
	oc.write_i32::<LittleEndian>(10_000)?;
	oc.write_i32::<LittleEndian>(10_100)?;
	let bitmap_bytes = gan.bitmap.as_bytes();
	oc.write_u32::<LittleEndian>(bitmap_bytes.len() as u32 + 1)?;
	oc.write_all(bitmap_bytes)?;
	oc.write_u8(0)?;
	oc.write_i32::<LittleEndian>(20_000)?;
	oc.write_u32::<LittleEndian>(gan.sets.len() as u32)?;
	for set in &gan.sets {
		write_set(oc, set)?;
	}
	Ok(())
}
