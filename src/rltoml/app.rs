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

use clap::Parser;

/// Command-line arguments/options for RlToml.
#[derive(Parser, Debug)]
#[command(
	name = "\x1b[30;107m[ RlToml",
	version = concat!(env!("CARGO_PKG_VERSION"), " ]\x1b[0m"),
	about = concat!(
		"Converter between RealLive auxiliary data formats and TOML\n",
		"Use \x1b[1m--info\x1b[0m to show more information about this program"),
	help_template = concat!(
		"{name} {version}: {about}\n\n",
		"\x1b[1;4mUsage:\x1b[0m {usage}\n\n",
		"{all-args}\n"),
	disable_help_flag = true,
	disable_version_flag = true,
)]
pub struct Args {
	// RlToml Information
	#[arg(
		long = "help",
		help = "display this usage information",
		help_heading = "Information",
	)]
	pub help: bool,

	#[arg(
		long = "version",
		help = "display RlToml version information",
		help_heading = "Information",
	)]
	pub version: bool,

	#[arg(
		long = "info",
		help = "display detailed information about RlToml and its usage",
		help_heading = "Information",
	)]
	pub info: bool,

	// RlToml Options
	#[arg(
		short = 'v',
		long = "verbose",
		help = "show detailed information about what RlToml is doing",
		help_heading = "Options",
	)]
	pub verbose: bool,

	#[arg(
		long = "uppercase",
		help = "use uppercase hex digits (A-F) in hex dumps",
		requires = "verbose",
		help_heading = "Options",
	)]
	pub uppercase: bool,

	#[arg(
		short = 'o',
		long = "output",
		value_name = "NAME",
		help = concat!(
			"if only one file is being converted, sets the output\n",
			"filename, otherwise names the directory to place\n",
			"outputs in"),
		help_heading = "Options",
	)]
	pub output: Option<String>,

	#[arg(
		value_name = "FILE/FILES",
		help = "input file(s) to convert",
	)]
	pub files: Vec<String>,
}

/// Parses command-line arguments using Clap.
pub fn parse_args() -> Args {
	Args::parse()
}
