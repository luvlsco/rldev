use clap::Command;
use std::path::Path;

/// Returns the file name (without path) of the given path, or the path itself if it has no file name.
pub fn get_file_name(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.as_ref().display().to_string())
}

/// Returns the binary name to be used in help/version output, based on the first command-line argument.
pub fn get_bin_name(cmd: &Command) -> String {
    std::env::args_os()
        .next()
        .map(get_file_name)
        .unwrap_or_else(|| cmd.get_name().to_string())
}
