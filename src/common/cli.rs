use clap::Command;

/// Prints a trailing blank line then exits, keeping terminal output clean.
pub fn quit(code: i32) -> ! {
    eprintln!();
    std::process::exit(code);
}

/// Prints the help message for the calling binary, used when parsing empty args.
pub fn print_help(cmd: Command) {
    let bin_name = crate::common::filesystem::get_bin_name(&cmd);

    cmd.bin_name(bin_name).print_help().unwrap();
}

/// Capitalizes the first character and ensures the line ends with a period.
/// Strips the last line if non-empty.
/// Skips the period when the result is multi-line or already ends with ':'.
pub fn format_output(text: &str) -> String {
    let text = text.trim_end();
    let content = if let Some((before, last)) = text.rsplit_once('\n') {
        if last.trim().is_empty() {
            text
        } else {
            before.trim_end()
        }
    } else {
        text
    };

    if content.is_empty() {
        return String::new();
    }

    let mut chars = content.chars();
    let first = chars.next().unwrap();
    let result = format!("{}{}", first.to_uppercase(), chars.collect::<String>());

    if result.contains('\n') || result.ends_with(':') {
        result
    } else {
        format!("{}.", result)
    }
}
