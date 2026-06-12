use clap::Command;

/// Prints a message followed by an empty line.
pub fn print_line(msg: impl std::fmt::Display) {
	println!("{msg}");
	if !msg.to_string().ends_with('\n') {
		println!();
	}
}

/// Prints an error message to stderr followed by an empty line.
pub fn eprint_line(msg: impl std::fmt::Display) {
	eprintln!("{msg}");
	if !msg.to_string().ends_with('\n') {
		eprintln!();
	}
}

/// Prints the help message for the calling binary, used when parsing empty args.
pub fn print_help(cmd: Command) {
	let bin_name = crate::common::filesystem::get_bin_name(&cmd);

	cmd.bin_name(bin_name)
		.print_help()
		.unwrap();

	println!();
}
