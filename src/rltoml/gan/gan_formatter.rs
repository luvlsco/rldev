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
use toml_edit::{InlineTable, Value};

use super::FrameAttrs;
use super::gan_parser::GanParser;
use super::gan_parser::GanParser_Frame as GanFrame;
use super::gan_parser::GanParser_GanDataSection_AnimationFrame as GanAnimFrame;

use crate::error_formatter::{self, AnyOfSpec, MagicSpec, ParseError, ParseResult};

/// Kaitai validation site src_path strings, mirroring the `valid:` blocks
/// in gan_parser.ksy (as generated in gan_parser.rs).
const SRC_EMPTY_SET: &str = "/types/gan_data_section/types/animation_set/seq/1";
const SRC_FRAME_TAG: &str = "/types/gan_data_section/types/frame_entry/seq/0";

type MagicSite = (&'static str, &'static str, i32, fn(&str) -> Option<usize>);

/// Display metadata for each magic-number validation site in gan_parser.ksy.
/// (src_path, label, expected, offset)
const MAGIC_SITES: [MagicSite; 5] = [
    ("/types/gan_header/seq/0", "first GAN header", 10_000, |_| Some(0)),
    ("/types/gan_header/seq/1", "second GAN header", 10_000, |_| Some(4)),
    ("/types/gan_header/seq/2", "third GAN header", 10_100, |_| Some(8)),
    (
        "/types/gan_data_section/seq/0",
        "data section start marker",
        20_000,
        compute_data_section_offset,
    ),
    (
        "/types/gan_data_section/types/animation_set/seq/0",
        "animation set start marker",
        30_000,
        compute_set_marker_offset,
    ),
];

impl FrameAttrs {
    /// Iterates all frame attribute (name, value) pairs, including unset fields as `None`.
    fn iter_fields(&self) -> impl Iterator<Item = (&'static str, Option<i32>)> {
        [
            ("pattern", self.pattern),
            ("x", self.x),
            ("y", self.y),
            ("time", self.time),
            ("alpha", self.alpha),
            ("z", self.z),
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
            GanFrame::Z => self.z = Some(value),
            GanFrame::FrameEnd | GanFrame::Unknown(_) => (),
        }
    }

    /// Converts set attributes into key-value pairs for inline table output, skipping `None` fields.
    fn to_inline_table_fields(&self) -> Vec<(String, String)> {
        self.iter_fields()
            .filter_map(|(name, value)| value.map(|v| (name.to_string(), v.to_string())))
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
            z: diff_opt(self.z, defaults.z),
        }
    }

    /// Returns the set of attributes that are identical across all frames, discarding any that vary.
    fn common_attrs(frames: &[FrameAttrs]) -> FrameAttrs {
        frames
            .iter()
            .skip(1)
            .fold(frames.first().cloned().unwrap_or_default(), |common, frame| FrameAttrs {
                pattern: keep_if_eq(common.pattern, frame.pattern),
                x: keep_if_eq(common.x, frame.x),
                y: keep_if_eq(common.y, frame.y),
                time: keep_if_eq(common.time, frame.time),
                alpha: keep_if_eq(common.alpha, frame.alpha),
                z: keep_if_eq(common.z, frame.z),
            })
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
pub fn gan_to_toml(path: &str, verbose: bool) -> ParseResult<String> {
    if verbose {
        println!("Reading GAN header");
    }
    let gan = parse_gan(path)?;
    let header = gan.gan_header().get();
    let data_section = gan.gan_data_section().get();

    if verbose {
        println!("Reading GAN set data");
    }
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
        for (name, val) in defaults.to_inline_table_fields() {
            lines.push(format!("{} = {}", name, val));
        }

        lines.push("frames = [".to_string());

        for frame in frames {
            let diff = frame.diff_from(&defaults);
            let fields = diff.to_inline_table_fields();
            lines.push(format!("  {},", build_inline_table(&fields)));
        }

        lines.push("]".to_string());
        lines.push(String::new());
    }

    if verbose {
        println!("Generating TOML");
    }

    Ok(lines.join("\n"))
}

/// Formats a Kaitai validation error with context, expected/found values, and hex dump.
pub fn format_gan_to_toml_error(err: &ParseError, path: &str, verbose: bool, uppercase: bool) -> String {
    let kerr = match err {
        ParseError::Kaitai(k) => k,
        ParseError::KaitaiWithContext { err: k, .. } => k,
    };

    let KError::ValidationFailed(validation) = kerr else {
        return error_formatter::format_parse_error(err);
    };

    let src_path = validation.src_path.as_str();
    let context = err.read_context();
    let (got, offset) = crate::binary_reader::context_to_got_offset(context);

    if let Some((_, label, expected, compute_offset)) = MAGIC_SITES.iter().find(|(site, ..)| *site == src_path) {
        return match compute_offset(path) {
            Some(off) => error_formatter::format_magic(
                MagicSpec {
                    label,
                    expected: *expected,
                    offset: off,
                },
                path,
                verbose,
                uppercase,
            ),
            None => format!("invalid {} (expected {})", label, expected),
        };
    }

    if src_path == SRC_EMPTY_SET {
        return "animation set must contain at least one frame.".to_string();
    }

    if src_path == SRC_FRAME_TAG {
        return error_formatter::format_any_of(
            AnyOfSpec {
                label: "frame entry tag",
                any_of: &valid_frame_tags(),
                got,
                offset,
            },
            path,
            verbose,
            uppercase,
        );
    }

    "parse failed at an unexpected location".to_string()
}

/// All valid frame entry tag values, including `FrameEnd`.
fn valid_frame_tags() -> [i32; 7] {
    [
        i64::from(&GanFrame::Pattern) as i32,
        i64::from(&GanFrame::X) as i32,
        i64::from(&GanFrame::Y) as i32,
        i64::from(&GanFrame::Time) as i32,
        i64::from(&GanFrame::Alpha) as i32,
        i64::from(&GanFrame::Z) as i32,
        i64::from(&GanFrame::FrameEnd) as i32,
    ]
}

/// Computes the expected offset of the data section start marker (20000).
fn compute_data_section_offset(path: &str) -> Option<usize> {
    crate::binary_reader::read_u4_le(path, 12)
        .ok()
        .map(|(n, _)| 16 + n as usize)
        .filter(|&o| o != 0)
}

/// Computes the expected offset of the animation set start marker (30000).
fn compute_set_marker_offset(path: &str) -> Option<usize> {
    crate::binary_reader::read_u4_le(path, 12)
        .ok()
        .map(|(n, _)| 16 + n as usize + 8)
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

/// Builds a TOML inline table from key-value pairs, parsing each value.
fn build_inline_table(fields: &[(String, String)]) -> InlineTable {
    let mut table = InlineTable::new();
    for (key, value) in fields {
        let parsed_value: Value = value.parse().unwrap_or_else(|_| {
            let toml_str = format!("x = \"{}\"", value.replace('"', "\\\""));
            toml_str
                .parse::<toml_edit::Item>()
                .ok()
                .and_then(|item| item.as_value().cloned())
                .unwrap_or_else(|| "\"\"".parse().unwrap())
        });
        table.insert(key, parsed_value);
    }
    table
}
