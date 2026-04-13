mod app;

use clap::CommandFactory;

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
}
