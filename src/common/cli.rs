use clap::Command;

/// Prints a trailing blank line then exits, keeping terminal output clean.
pub fn exit(code: i32) -> ! {
    eprintln!();
    std::process::exit(code);
}

/// Prints the help message for the calling binary, used when parsing empty args.
pub fn print_help(cmd: Command) {
    let bin_name = crate::common::filesystem::get_bin_name(&cmd);

    cmd.bin_name(bin_name).print_help().unwrap();
}

/// Capitalizes the first character, drops the last line of multi-line input,
/// and ensures the line ends with a period (skipped when multi-line or already ending with ':').
pub fn format_output(text: &str) -> String {
    let text = text.trim_end();
    let content = text
        .rsplit_once('\n')
        .map_or(text, |(before, _)| before.trim_end());

    if content.is_empty() {
        return String::new();
    }

    let mut chars = content.chars();
    let result = format!(
        "{}{}",
        chars.next().unwrap().to_uppercase(),
        chars.as_str()
    );

    if result.contains('\n') || result.ends_with(':') {
        result
    } else {
        format!("{result}.")
    }
}
