/*
 RlToml: GAN format tests
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

use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
	crate::common::fixture("rltoml/gan", name)
}

fn run(input: &std::path::Path, args: &[&str]) -> (bool, String, String) {
	crate::common::run(input, args)
}

// --- Success ---

#[test]
fn gan_to_toml() {
	let input = fixture("success.gan");
	let expected = std::fs::read(fixture("success.gan.toml")).unwrap();
	let output = std::env::temp_dir().join("rltoml_gan_to_toml.gan.toml");

	let (ok, stdout, stderr) = run(&input, &["-o", output.to_str().unwrap()]);
	assert!(ok, "stderr: {}", stderr);
	assert!(stdout.contains("Successfully converted"));

	let actual = std::fs::read(&output).unwrap();
	assert_eq!(actual, expected, "TOML output differs from fixture");
}

#[test]
fn toml_to_gan() {
	let input = fixture("success.gan.toml");
	let expected = std::fs::read(fixture("success.gan.bak")).unwrap();
	let output = std::env::temp_dir().join("rltoml_toml_to_gan.gan");

	let (ok, stdout, _) = run(&input, &["-o", output.to_str().unwrap()]);
	assert!(ok);
	assert!(stdout.contains("Successfully converted"));

	let actual = std::fs::read(&output).unwrap();
	assert_eq!(actual, expected, "GAN output differs from fixture");
}

// --- GAN error output ---

#[test]
fn gan_error_first_magic_verbose() {
	let (ok, _, stderr) = run(&fixture("error_first_magic.gan"), &["-v"]);
	assert!(!ok);
	assert!(stderr.contains("Failed to convert \"error_first_magic.gan\" to \"error_first_magic.gan.toml\" (TOML): invalid value at first GAN header:"));
	let expected_dump = "\
Dump (16 of 457 bytes shown, starting at offset 0x00000000, error at offset 0x00000000):
0x00000000 | 10 27 10 00 10 27 00 00 74 27 00 00 09 00 00 00
             ^^^^^^^^^^^";
	assert!(stderr.contains(expected_dump), "caret alignment broken:\n{}", stderr);
}

#[test]
fn gan_error_empty_set() {
	let (ok, _, _) = run(&fixture("error_empty_set.gan"), &[]);
	assert!(!ok);
}

// --- TOML errors ---

macro_rules! toml_error_test {
	($name:ident, $file:expr) => {
		#[test]
		fn $name() {
			let (ok, _, _) = run(&fixture($file), &[]);
			assert!(!ok);
		}
	};
}

toml_error_test!(toml_error_no_gan, "error_no_gan.gan.toml");
toml_error_test!(toml_error_gan_not_table, "error_gan_not_table.gan.toml");
toml_error_test!(toml_error_no_bitmap, "error_no_bitmap.gan.toml");
toml_error_test!(toml_error_bitmap_not_string, "error_bitmap_not_string.gan.toml");
toml_error_test!(toml_error_no_set, "error_no_set.gan.toml");
toml_error_test!(toml_error_set_not_array, "error_set_not_array.gan.toml");
toml_error_test!(toml_error_no_frames, "error_no_frames.gan.toml");
toml_error_test!(toml_error_frames_not_array, "error_frames_not_array.gan.toml");
toml_error_test!(toml_error_frame_not_table, "error_frame_not_table.gan.toml");
toml_error_test!(toml_error_field_not_int, "error_field_not_int.gan.toml");
toml_error_test!(toml_error_unknown_field, "error_unknown_field.gan.toml");
toml_error_test!(toml_error_bad_toml, "error_bad_toml.gan.toml");
toml_error_test!(toml_error_empty_set, "error_empty_set.gan.toml");

// --- TOML error verbose output ---

#[test]
fn toml_error_bad_toml_verbose() {
	let (ok, _, stderr) = run(&fixture("error_bad_toml.gan.toml"), &["-v"]);
	assert!(!ok);
	assert!(!stderr.is_empty());
}
