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

use clap::{CommandFactory, Parser};

/// Command-line arguments/options for RlToml.
#[derive(Parser, Debug)]
#[command(
	name = "\x1b[1m[ RlToml",
	version = concat!(env!("CARGO_PKG_VERSION"), " ]\x1b[0m"),
	about = concat!(
		"Converter between RealLive auxiliary data formats and TOML\n",
		"Use \x1b[1m--info\x1b[0m to show more information about this program"),
	help_template = concat!(
		"{name} {version}: {about}\n\n",
		"{usage-heading} {usage}\n\n",
		"{all-args}\n"),
	disable_help_flag = true,
	disable_version_flag = true,
)]

pub struct Args {
    // RlToml Information
    #[arg(
        long = "version",
        help = "display RlToml version information",
        help_heading = "Information"
    )]
    pub version: bool,

    #[arg(
        long = "info",
        help = "display detailed information about RlToml and its usage",
        help_heading = "Information"
    )]
    pub info: bool,

    #[arg(
        long = "help",
        help = "display this usage information",
        help_heading = "Information"
    )]
    pub help: bool,

    // RlToml Options
    #[arg(
        short = 'v',
        long = "verbose",
        help = "show detailed information about what RlToml is doing",
        help_heading = "Options"
    )]
    pub verbose: bool,

    #[arg(
        short = 'u',
        long = "uppercase",
        help = "use uppercase hex digits (A-F) in hex dumps",
        requires = "verbose",
        help_heading = "Options"
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

    #[arg(value_name = "FILE/FILES", help = "input file(s) to convert")]
    pub files: Vec<String>,
}

/// Parses command-line arguments using Clap.
/// Error messages are rendered monochrome to avoid clap's default ANSI coloring.
pub fn parse_args() -> Args {
    match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            let mut cmd = <Args as CommandFactory>::command().color(clap::ColorChoice::Never);
            let formatted = e.format(&mut cmd);
            eprintln!(
                "{}",
                rldev::common::cli::format_output(&formatted.to_string())
            );
            rldev::common::cli::exit(formatted.exit_code());
        }
    }
}

impl Args {
    /// Prints version number and changelog.
    pub fn print_version() {
        indoc::printdoc! {"
			\x1b[1m[ RlToml {rldev_version} ]\x1b[0m: Converter between RealLive auxiliary data formats and TOML
			Based on RlXml, originally developed in OCaml by Haeleth (2006).

			{changelog}",
            rldev_version = env!("CARGO_PKG_VERSION"),
            changelog = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/rltoml/CHANGELOG")),
        }
    }

    /// Prints program description and supported features.
    pub fn print_info() {
        indoc::printdoc! {"
			\x1b[1m[ RlToml {rldev_version} ]\x1b[0m: Converter between RealLive auxiliary data formats and TOML
			Based on RlXml, originally developed in OCaml by Haeleth (2006).

			\x1b[1;4mSupported formats:\x1b[0m
			      \x1b[1m.gan\x1b[0m: bidirectional conversion with \x1b[1m.gan.toml\x1b[0m
		", rldev_version = env!("CARGO_PKG_VERSION")}
    }
}
