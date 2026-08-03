/*
 RLdev: Common test utilities
 Copyright (C) 2026 luvlsco

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
