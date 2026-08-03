/*
 RlToml: Basic I/O & Args tests
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

use std::path::PathBuf;

mod common;

#[path = "rltoml/gan.rs"]
mod gan;

#[path = "rltoml/dbs.rs"]
mod dbs;

fn fixture(name: &str) -> PathBuf {
    common::fixture("rltoml/gan", name)
}

fn run(input: &std::path::Path, args: &[&str]) -> (bool, String, String) {
    common::run(input, args)
}

// --- CLI flags ---

#[test]
fn cli_help() {
    let (ok, stdout, _) = run(&fixture("success.gan"), &["--help"]);
    assert!(ok);
    assert!(stdout.contains("Usage"));
}

#[test]
fn cli_version() {
    let (ok, stdout, _) = run(&fixture("success.gan"), &["--version"]);
    assert!(ok);
    assert!(stdout.contains("RlToml"));
}

#[test]
fn cli_info() {
    let (ok, stdout, _) = run(&fixture("success.gan"), &["--info"]);
    assert!(ok);
    assert!(stdout.contains("Supported formats"));
}

#[test]
fn cli_output_multi() {
    let tmp = std::env::temp_dir().join("rltoml_multi");
    let _ = std::fs::create_dir_all(&tmp);
    let a = tmp.join("a.gan");
    let b = tmp.join("b.gan");
    std::fs::copy(fixture("success.gan"), &a).unwrap();
    std::fs::copy(fixture("success.gan"), &b).unwrap();
    let outdir = tmp.join("multi_out");

    let (ok, stdout, _) = run(&a, &["-o", outdir.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(ok);
    assert!(stdout.contains("Successfully converted"));

    assert!(outdir.join("a.gan").exists());
    assert!(outdir.join("b.gan").exists());
}

#[test]
fn cli_unknown_file_type() {
    let bad = std::env::temp_dir().join("rltoml_test.txt");
    std::fs::write(&bad, "test").unwrap();

    let (ok, _, stderr) = run(&bad, &[]);
    assert!(!ok);
    assert!(stderr.contains("Unknown file type"));
}

#[test]
fn verbose_output() {
    let input = fixture("success.gan");
    let output = std::env::temp_dir().join("rltoml_verbose.gan.toml");

    let (ok, stdout, _) = run(&input, &["-v", "-o", output.to_str().unwrap()]);
    assert!(ok);
    assert!(stdout.contains("Reading"));
}

#[test]
fn uppercase_requires_verbose() {
    let (ok, _, _) = run(&fixture("success.gan"), &["-u"]);
    assert!(!ok);
}
