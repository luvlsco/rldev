/*
 RlToml: DBS integration tests
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

use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("src")
		.join("rltoml")
		.join("dbs")
		.join(name)
}

fn run(input: &Path, args: &[&str]) -> (bool, String, String) {
	super::common::run(input, args)
}

#[test]
fn bin_to_toml_contains_parser_data() {
	let output = std::env::temp_dir().join("rltoml_dbs_formatter");
	let output_name = output.to_str().unwrap();
	let (ok, _, stderr) = run(&fixture("dangopedia.dbs.bin"), &["-o", output_name]);
	assert!(ok, "{stderr}");

	let output = output.with_extension("dbs.bin.toml");
	let toml = std::fs::read_to_string(output).unwrap();
	assert!(toml.contains("column_id = 0\ntype = \"string\""));
	assert!(toml.contains("row_id = 0\nrow_column_0 = \"Anpan\""));
	assert!(toml.contains("row_column_2 = 4502\nrow_column_3 = 87"));
}

#[test]
fn dbs_toml_round_trip() {
	let prefix = std::env::temp_dir().join("rltoml_dbs_round_trip");
	let prefix_name = prefix.to_str().unwrap();

	let (ok, _, stderr) = run(&fixture("dangopedia.dbs"), &["-o", prefix_name]);
	assert!(ok, "{stderr}");
	let bin = prefix.with_extension("dbs.bin");

	let toml_prefix = std::env::temp_dir().join("rltoml_dbs_round_trip_toml");
	let toml_prefix_name = toml_prefix.to_str().unwrap();
	let (ok, _, stderr) = run(&bin, &["-o", toml_prefix_name]);
	assert!(ok, "{stderr}");
	let toml = toml_prefix.with_extension("dbs.bin.toml");

	let dbs_prefix = std::env::temp_dir().join("rltoml_dbs_round_trip_rebuilt");
	let dbs_prefix_name = dbs_prefix.to_str().unwrap();
	let (ok, _, stderr) = run(&toml, &["-o", dbs_prefix_name]);
	assert!(ok, "{stderr}");
	let rebuilt_dbs = dbs_prefix.with_extension("dbs");

	let rebuilt_bin_prefix = std::env::temp_dir().join("rltoml_dbs_round_trip_rebuilt_bin");
	let rebuilt_bin_prefix_name = rebuilt_bin_prefix.to_str().unwrap();
	let (ok, _, stderr) = run(&rebuilt_dbs, &["-o", rebuilt_bin_prefix_name]);
	assert!(ok, "{stderr}");
	let rebuilt_bin = rebuilt_bin_prefix.with_extension("dbs.bin");
	assert_eq!(std::fs::read(&rebuilt_bin).unwrap().len() % 64, 0);

	let final_toml_prefix = std::env::temp_dir().join("rltoml_dbs_round_trip_final_toml");
	let final_toml_prefix_name = final_toml_prefix.to_str().unwrap();
	let (ok, _, stderr) = run(&rebuilt_bin, &["-o", final_toml_prefix_name]);
	assert!(ok, "{stderr}");
	let final_toml = final_toml_prefix.with_extension("dbs.bin.toml");

	assert_eq!(std::fs::read_to_string(toml).unwrap(), std::fs::read_to_string(final_toml).unwrap());
}
