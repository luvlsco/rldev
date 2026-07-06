use std::path::{Path, PathBuf};
use std::process::Command;

pub fn fixture(base: &str, name: &str) -> PathBuf {
	PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests"))
		.join(base)
		.join(name)
}

pub fn bin() -> Command {
	Command::new(env!("CARGO_BIN_EXE_rltoml"))
}

pub fn run(input: &Path, args: &[&str]) -> (bool, String, String) {
	let out = bin().arg(input).args(args).output().unwrap();
	(
		out.status.success(),
		String::from_utf8_lossy(&out.stdout).into_owned(),
		String::from_utf8_lossy(&out.stderr).into_owned(),
	)
}
