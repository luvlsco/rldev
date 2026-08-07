/*
 RlToml: Converter between RealLive auxiliary data formats and TOML
 Copyright (C) 2026 luvlsco

 Based on RlXml, originally developed in OCaml by:
  Copyright (C) 2006 Haeleth

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

mod app;
mod binary_reader;
mod dbs;
mod error_formatter;
mod gan;

use rldev::common::filesystem::get_file_name;
use rldev::common::options::OutputRequest;

fn get_file_type(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    // .gan file
    if file_name.ends_with(".gan") {
        Some(path.with_extension("").with_extension("gan.toml"))
    } else if file_name.ends_with(".gan.toml") {
        Some(path.with_extension("gan"))

    // .dbs file
    } else if file_name.ends_with(".dbs") {
        Some(path.with_extension("").with_extension("dbs.bin"))
    } else if file_name.ends_with(".dbs.bin") {
        Some(path.with_extension("bin.toml"))
    } else if file_name.ends_with(".dbs.bin.toml") {
        Some(path.with_extension("").with_extension(""))

    // Unknown file
    } else {
        None
    }
}

fn derive_output_path(input: &std::path::Path) -> std::path::PathBuf {
    get_file_type(input).unwrap_or_else(|| {
        eprintln!("Unknown file type: {}", input.display());
        rldev::common::cli::exit(1);
    })
}

/// Default output for a `.dbs` input in chain mode: the final `.dbs.bin.toml`.
fn chain_derive_output_path(input: &std::path::Path) -> std::path::PathBuf {
    let file_name = input.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if file_name.ends_with(".dbs") {
        input.with_extension("dbs.bin.toml")
    } else {
        derive_output_path(input)
    }
}

fn csv_derive_output_path(input: &std::path::Path) -> std::path::PathBuf {
    input.with_extension("csv")
}

/// CSV title row: the input's base name truncated at the first dot.
fn csv_title(file_name: &str) -> String {
    file_name.split('.').next().unwrap_or(file_name).to_string()
}

fn convert_single(file: &str, out_path: &std::path::Path, verbose: bool, uppercase: bool, to_csv: bool, to_bin: bool) {
    let path = std::path::Path::new(file);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // .gan
    if file_name.ends_with(".gan") {
        let toml = gan::gan_to_toml(file, verbose).unwrap_or_else(|err: error_formatter::ParseError| {
            eprintln!(
                "Failed to convert \"{}\" to \"{}\" (GAN -> TOML): {}",
                get_file_name(file),
                get_file_name(out_path),
                gan::format_gan_to_toml_error(&err, file, verbose, uppercase)
            );
            rldev::common::cli::exit(1);
        });
        if let Err(err) = std::fs::write(out_path, toml) {
            eprintln!(
                "Error writing {}: {}",
                get_file_name(out_path),
                error_formatter::format_io_error(&err)
            );
            rldev::common::cli::exit(1);
        }
        println!("Successfully converted: {}", get_file_name(out_path));

    // .gan.toml
    } else if file_name.ends_with(".gan.toml") {
        gan::toml_to_gan(file, &out_path.display().to_string(), verbose).unwrap_or_else(|err| {
            eprintln!(
                "Failed to convert \"{}\" to \"{}\" (TOML -> GAN): {}",
                get_file_name(file),
                get_file_name(out_path),
                gan::format_toml_to_gan_error(&err, verbose)
            );
            rldev::common::cli::exit(1);
        });
        println!("Successfully converted: {}", get_file_name(out_path));

    // .dbs.bin.toml
    } else if file_name.ends_with(".dbs.bin.toml") {
        dbs::toml_to_dbs(file, &out_path.display().to_string(), verbose).unwrap_or_else(|err| {
            eprintln!(
                "Failed to convert \"{}\" to \"{}\" (TOML -> DBS): {}",
                get_file_name(file),
                get_file_name(out_path),
                dbs::format_toml_to_dbs_error(&err, verbose)
            );
            rldev::common::cli::exit(1);
        });
        println!("Successfully converted: {}", get_file_name(out_path));

    // .dbs.bin
    } else if file_name.ends_with(".dbs.bin") {
        if to_csv {
            let csv = dbs::dbs_bin_to_csv(file, &csv_title(file_name), verbose).unwrap_or_else(|err| {
                eprintln!(
                    "Failed to convert \"{}\" to \"{}\" (BIN -> CSV): {}",
                    get_file_name(file),
                    get_file_name(out_path),
                    err
                );
                rldev::common::cli::exit(1);
            });
            if let Err(err) = std::fs::write(out_path, csv) {
                eprintln!(
                    "Error writing {}: {}",
                    get_file_name(out_path),
                    error_formatter::format_io_error(&err)
                );
                rldev::common::cli::exit(1);
            }
        } else {
            let toml = dbs::dbs_bin_to_toml(file, verbose).unwrap_or_else(|err: error_formatter::ParseError| {
                eprintln!(
                    "Failed to convert \"{}\" to \"{}\" (BIN -> TOML): {}",
                    get_file_name(file),
                    get_file_name(out_path),
                    error_formatter::format_parse_error(&err)
                );
                rldev::common::cli::exit(1);
            });
            if let Err(err) = std::fs::write(out_path, toml) {
                eprintln!(
                    "Error writing {}: {}",
                    get_file_name(out_path),
                    error_formatter::format_io_error(&err)
                );
                rldev::common::cli::exit(1);
            }
        }
        println!("Successfully converted: {}", get_file_name(out_path));

    // .dbs
    } else if file_name.ends_with(".dbs") {
        if to_csv {
            let csv = dbs::dbs_to_csv(file, &csv_title(file_name), verbose).unwrap_or_else(|err| {
                eprintln!(
                    "Failed to convert \"{}\" to \"{}\" (DBS -> CSV): {}",
                    get_file_name(file),
                    get_file_name(out_path),
                    err
                );
                rldev::common::cli::exit(1);
            });
            if let Err(err) = std::fs::write(out_path, csv) {
                eprintln!(
                    "Error writing {}: {}",
                    get_file_name(out_path),
                    error_formatter::format_io_error(&err)
                );
                rldev::common::cli::exit(1);
            }
            println!("Successfully converted: {}", get_file_name(out_path));
            return;
        }

        let bytes = dbs::dbs_to_bin_bytes(file, verbose).unwrap_or_else(|err| {
            let message = match &err {
                dbs::dbs_decompress::DbsError::Io(err) => error_formatter::format_io_error(err),
                _ => err.to_string(),
            };
            eprintln!(
                "Failed to convert \"{}\" to \"{}\" (DBS -> BIN): {}",
                get_file_name(file),
                get_file_name(out_path),
                error_formatter::format_write_error(&message, verbose)
            );
            rldev::common::cli::exit(1);
        });

        if to_bin {
            if verbose {
                eprintln!("Writing raw database binary");
            }
            if let Err(err) = std::fs::write(out_path, &bytes) {
                eprintln!(
                    "Error writing {}: {}",
                    get_file_name(out_path),
                    error_formatter::format_io_error(&err)
                );
                rldev::common::cli::exit(1);
            }
            println!("Successfully converted: {}", get_file_name(out_path));
            return;
        }

        let toml = dbs::dbs_bytes_to_toml(&bytes, verbose).unwrap_or_else(|err: error_formatter::ParseError| {
            eprintln!(
                "Failed to convert \"{}\" to \"{}\" (BIN -> TOML): {}",
                get_file_name(file),
                get_file_name(out_path),
                error_formatter::format_parse_error(&err)
            );
            rldev::common::cli::exit(1);
        });
        if let Err(err) = std::fs::write(out_path, toml) {
            eprintln!(
                "Error writing {}: {}",
                get_file_name(out_path),
                error_formatter::format_io_error(&err)
            );
            rldev::common::cli::exit(1);
        }
        println!("Successfully converted: {}", get_file_name(out_path));
    }
}

fn main() {
    let raw_args: Vec<_> = std::env::args_os().collect();
    let args = app::parse_args();

    if args.version {
        app::Args::print_version();
        rldev::common::cli::exit(0);
    }

    if args.info {
        app::Args::print_info();
        rldev::common::cli::exit(0);
    }

    // Arg: --help
    // Print help if "--help" is called explicitly
    // or if no other arguments or files are provided
    if args.help || raw_args.len() == 1 || args.files.is_empty() {
        let cmd = <app::Args as clap::CommandFactory>::command();
        rldev::common::cli::print_help(cmd);
        rldev::common::cli::exit(0);
    }

    let verbose = args.verbose;
    let uppercase = args.uppercase;
    let to_csv = args.to_csv;
    let to_bin = args.to_bin;

    let inputs: Vec<std::path::PathBuf> = args.files.iter().map(std::path::PathBuf::from).collect();

    // Validate file types before derive_output_path sees unknown extensions
    for path in &inputs {
        if get_file_type(path).is_none() {
            eprintln!("Unknown file type: {}", path.display());
            rldev::common::cli::exit(1);
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if to_csv && !name.ends_with(".dbs") && !name.ends_with(".dbs.bin") {
            eprintln!("--to-csv requires .dbs or .dbs.bin input files");
            rldev::common::cli::exit(1);
        }
        if to_bin && !name.ends_with(".dbs") {
            eprintln!("--to-bin requires a .dbs input file");
            rldev::common::cli::exit(1);
        }
    }

    // Append target extension to single-file conversion
    let output: Option<String> = if to_csv {
        if inputs.len() == 1 {
            args.output.map(|output_name| {
                if output_name.ends_with(".csv") {
                    output_name
                } else {
                    format!("{}.csv", output_name)
                }
            })
        } else {
            args.output
        }
    } else if to_bin {
        if inputs.len() == 1 {
            args.output.map(|output_name| {
                if output_name.ends_with(".dbs.bin") {
                    output_name
                } else {
                    format!("{}.dbs.bin", output_name)
                }
            })
        } else {
            args.output
        }
    } else if inputs.len() == 1 {
        args.output.map(|output_name| {
            let converted_output_suffix = inputs[0].file_name().and_then(|n| n.to_str()).and_then(|file_name| {
                if file_name.ends_with(".gan.toml") {
                    Some(".gan")
                } else if file_name.ends_with(".gan") {
                    Some(".gan.toml")
                } else if file_name.ends_with(".dbs.bin.toml") {
                    Some(".dbs")
                } else if file_name.ends_with(".dbs.bin") || file_name.ends_with(".dbs") {
                    Some(".dbs.bin.toml")
                } else {
                    None
                }
            });
            converted_output_suffix
                .filter(|suffix| !output_name.ends_with(suffix))
                .map_or(output_name.clone(), |suffix| format!("{}{}", output_name, suffix))
        })
    } else {
        args.output
    };

    let output = output.as_deref();
    let out_paths = rldev::common::options::resolve_output_path(OutputRequest {
        output,
        outdir: None,
        inputs: &inputs,
        derive: if to_csv {
            csv_derive_output_path
        } else if to_bin {
            derive_output_path
        } else {
            chain_derive_output_path
        },
    })
    .unwrap();

    for (file, out_path) in args.files.iter().zip(out_paths.iter()) {
        convert_single(file, out_path, verbose, uppercase, to_csv, to_bin);
    }
    rldev::common::cli::exit(0);
}
