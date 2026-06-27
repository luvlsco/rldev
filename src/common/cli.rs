use clap::Command;

/// Prints the help message for the calling binary, used when parsing empty args.
pub fn print_help(cmd: Command) {
	let bin_name = crate::common::filesystem::get_bin_name(&cmd);

	cmd.bin_name(bin_name)
		.print_help()
		.unwrap();

	println!();
}
