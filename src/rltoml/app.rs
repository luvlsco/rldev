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

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
	name = "RlToml",
	version = env!("CARGO_PKG_VERSION"),
	about = concat!(
		"convertor between RealLive auxiliary data formats and TOML\n",
		"Use --info to show more information about this program"),
	help_template = "{name} {version}: {about}\n\n{usage-heading} {usage}\n\n{all-args}\n",
	disable_help_flag = true,
	disable_version_flag = true,
)]
pub struct Args {
    #[arg(
        long = "help",
        help = "display this usage information"
    )]
    pub help: bool,

    #[arg(
        long = "version",
        help = "display RlToml version information"
    )]
    pub version: bool,

	#[arg(
		long = "info",
		help = "display detailed information about RlToml and its usage"
	)]
	pub info: bool,

    #[arg(short, long)]
    pub verbose: bool,
    
	#[arg(value_name = "FILES")]
    pub files: Vec<String>,
}

pub fn parse_args() -> Args {
    Args::parse()
}
