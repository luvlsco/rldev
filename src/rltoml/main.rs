/*
 RlToml: Converter between RealLive auxiliary data formats and TOML
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

mod app;
mod gan;
mod toml_formatter;
mod binary_reader;
mod error_formatter;

use rldev::common::cli::{get_file_name, print_line, eprint_line};

fn main() {
	let raw_args: Vec<_> = std::env::args_os().collect();
	let args = app::parse_args();

	// Arg: --help
	// Print help if "--help" is called explicitly
	// or if no other arguments or files are provided
	if args.help || raw_args.len() == 1 || args.files.is_empty() {
		let cmd = <app::Args as clap::CommandFactory>::command();
		rldev::common::cli::print_help(cmd);
		std::process::exit(0);
	}

	// Arg: --version
	// Print RlToml version
	if args.version {
		print_line(format!("RlToml {} - Converter between RealLive auxiliary data formats and TOML", env!("CARGO_PKG_VERSION")));
		std::process::exit(0);
	}

	// Arg: --info
	// Print detailed information about RlToml and its usage
	if args.info {
		print_line(format!("RlToml {}: Converter between RealLive auxiliary data formats and TOML", env!("CARGO_PKG_VERSION")));
		std::process::exit(0);
	}

	let file = &args.files[0];
	let path = std::path::Path::new(file);

	let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
	let is_gan_toml = file_name.ends_with(".gan.toml");

	let (out_path, conversion): (std::path::PathBuf, &str) =
		if is_gan_toml {
			let gan_path = path.with_extension("").with_extension("gan");
			(gan_path, "GAN binary")
		} else if file_name.ends_with(".gan") {
			let toml_path = path.with_extension("gan.toml");
			(toml_path, "TOML")
		} else {
			eprint_line(format!("Unknown file type: {}", file));
			std::process::exit(1);
		};

	match conversion {
		"TOML" => {
			let toml = gan::gan_to_toml(file).unwrap_or_else(|err| {
				eprint_line(format!("Failed to convert \"{}\" to TOML, {}", get_file_name(file), gan::format_gan_to_toml_error(&err, file, args.verbose, args.uppercase)));
				std::process::exit(1);
			});
			if let Err(err) = std::fs::write(&out_path, toml) {
				eprint_line(format!("Error writing {}: {}", out_path.display(), err));
				std::process::exit(1);
			}
			print_line(format!("success: {}", out_path.display()));
		}
		"GAN binary" => {
			gan::toml_to_gan(file, out_path.to_str().unwrap()).unwrap_or_else(|err| {
				eprint_line(format!("Failed to convert \"{}\" to GAN, {}", get_file_name(file), gan::format_toml_to_gan_error(&err, args.verbose)));
				std::process::exit(1);
			});
			print_line(format!("success: {}", out_path.display()));
		}
		_ => unreachable!(),
	}
}
