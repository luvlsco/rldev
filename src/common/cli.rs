use clap::Command;
use std::path::Path;

pub fn get_bin_name(cmd: &Command) -> String {
	std::env::args_os()
		.next()
		.and_then(|p| Path::new(&p).file_name().map(|n| n.to_string_lossy().into_owned()))
		.unwrap_or_else(|| cmd.get_name().to_string())
}

pub fn print_help(cmd: Command) {
	let bin_name = get_bin_name(&cmd);

	cmd.bin_name(bin_name)
		.print_help()
		.unwrap();

	println!();
}
