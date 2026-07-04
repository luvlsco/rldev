use std::path::{Path, PathBuf};

/// Caller-supplied input for output-path resolution.
///
/// Each binary's `app.rs` owns its clap arg shape (which short letters
/// `-o` and `-d` map to, and whether they exist at all). The binary
/// translates its own flags into the normalized fields below before
/// calling [`resolve`].
pub struct OutputRequest<'a> {
	/// Value of `-o` / `--output` (if the binary defined one).
	pub output: Option<&'a str>,

	/// Value of `-d` / `--outdir` (if the binary defined one).
	pub outdir: Option<&'a str>,

	/// Resolved input paths.
	pub inputs: &'a [PathBuf],

	/// Caller-supplied default-output derivation (e.g. extension swap).
	pub derive: fn(&Path) -> PathBuf,
}

/// Resolves output paths from an [`OutputRequest`].
///
/// Rules (first match wins):
/// 1. `outdir` set -> ensure the directory exists; join each input's file name to it.
/// 2. `output` set with exactly one input -> use the value as a literal output path.
/// 3. `output` set with multiple inputs -> use the value as a directory name in
///    the current working directory; create it if missing; join each input's
///    file name to it.
/// 4. Neither flag set -> fall back to `derive(input)` for each input.
pub fn resolve_output_path(req: OutputRequest) -> std::io::Result<Vec<PathBuf>> {
	if let Some(dir_str) = req.outdir {
		let dir = PathBuf::from(dir_str);
		std::fs::create_dir_all(&dir)?;
		return Ok(req
			.inputs
			.iter()
			.map(|i| dir.join(i.file_name().unwrap_or_default()))
			.collect());
	}

	if let Some(name) = req.output {
		if req.inputs.len() == 1 {
			return Ok(vec![PathBuf::from(name)]);
		}

		let dir = PathBuf::from(name);
		std::fs::create_dir_all(&dir)?;
		return Ok(req
			.inputs
			.iter()
			.map(|i| dir.join(i.file_name().unwrap_or_default()))
			.collect());
	}

	Ok(req.inputs.iter().map(|i| (req.derive)(i)).collect())
}
