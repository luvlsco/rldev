/* 
 RlToml: convert RealLive auxiliary file formats <> TOML
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

mod app;
mod gan;

use clap::CommandFactory;
use std::fs;
use std::path::Path;

fn main() {
	let raw_args: Vec<_> = std::env::args_os().collect();
	let args = app::parse_args();

	if raw_args.len() == 1 || args.help {
		let cmd = app::Args::command();
		rldev::common::cli::print_help(cmd);
		std::process::exit(0);
	}
	
	if args.version {
		println!("RlToml {} - convertor between RealLive auxiliary data formats and TOML", env!("CARGO_PKG_VERSION"));
		std::process::exit(0);
	}

	if args.info {
        println!("RlToml {}: convertor between RealLive auxiliary data formats and TOML", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
	}

	let file = &args.files[0];

	let toml = gan::gan_convert::gan_to_toml(file).unwrap_or_else(|err| {
		eprintln!("error converting {} to TOML: {:?}", file, err);
		std::process::exit(1);
	});

	let out_path = Path::new(file).with_extension("gan.toml");
	if let Err(err) = fs::write(&out_path, toml) {
		eprintln!("error writing {}: {}", out_path.display(), err);
		std::process::exit(1);
	}

	println!("sucess: {}", out_path.display());
}
