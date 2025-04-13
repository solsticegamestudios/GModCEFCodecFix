// TODO: Multithread generate bsdiff patches (with qbsdiff crate)
// TODO: If fixed file is deleted, just write "null" hash to manifest, DON'T generate a patch to get from the original to blank
// TODO: Compress patch files with zstd (or whatever's best; maybe not in this tool?)
// TODO: Compress original files too (probably not in this tool)
// TODO: Generate manifest.json

use crate as root;
use clap::{CommandFactory, Parser};
use clap::error::ErrorKind;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
	/// Path for Original files shipped with GMod
	original_path: PathBuf,

	/// Path for Fixed (already-patched) files
	fixed_path: PathBuf,

	/// Path for where to put the output Patch (bsdiff) files
	patch_path: PathBuf
}

pub fn main() {
	println!("{}", root::ABOUT);

	// Parse the args (will also exit if something's wrong with them)
	let args = Args::parse();

	let original_path = root::pathbuf_to_canonical_pathbuf(args.original_path.clone(), true);
	let fixed_path = root::pathbuf_to_canonical_pathbuf(args.fixed_path.clone(), true);
	let patch_path = root::pathbuf_to_canonical_pathbuf(args.patch_path.clone(), false);

	let mut cmd = Args::command();
	if original_path.is_err() {
		cmd.error(
			ErrorKind::InvalidValue,
			format!("Original Path: {}", original_path.unwrap_err().to_string()),
		)
		.exit();
	}

	let original_path = original_path.unwrap();
	let original_path_str = original_path.to_string_lossy();
	println!("Original Path: {}\n", original_path_str);

	if fixed_path.is_err() {
		cmd.error(
			ErrorKind::InvalidValue,
			format!("Fixed Path: {}", fixed_path.unwrap_err().to_string()),
		)
		.exit();
	}

	let fixed_path = fixed_path.unwrap();
	let fixed_path_str = fixed_path.to_string_lossy();
	println!("Fixed Path: {}\n", fixed_path_str);

	if patch_path.is_err() {
		cmd.error(
			ErrorKind::InvalidValue,
			format!("Patch Path: {}", patch_path.unwrap_err().to_string()),
		)
		.exit();
	}

	let patch_path = patch_path.unwrap();
	let patch_path_str = patch_path.to_string_lossy();
	println!("Patch Path: {}\n", patch_path_str);

	if original_path == fixed_path {
		cmd.error(
			ErrorKind::ValueValidation,
			"Original Path cannot match Fixed Path.",
		)
		.exit();
	}

	println!("*** GENERATING PATCH FILES ***\n");

	// TODO
}
