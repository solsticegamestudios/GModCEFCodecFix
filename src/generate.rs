// TODO: Multithread generate bsdiff patches (with qbsdiff crate)
// TODO: If fixed file is deleted, just write "null" hash to manifest, DON'T generate a patch to get from the original to blank
// TODO: Compress patch files with zstd (or whatever's best; maybe not in this tool?)
// TODO: Compress original files too (probably not in this tool)
// TODO: Generate manifest.json

use crate::*;
use clap::{CommandFactory, Parser};
use clap::error::ErrorKind;
use std::path::PathBuf;
use std::collections::HashMap;
use rayon::prelude::*;
use std::time::Instant;
use qbsdiff::Bsdiff;

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

fn get_files_recursive(source: &str, path_base: String, files: &mut HashMap<String, HashMap<String, PathBuf>>, dir_path: PathBuf) {
	for entry in std::fs::read_dir(dir_path).unwrap() {
		let entry = entry.unwrap();
		let entry_path = entry.path();
		let entry_filename = entry.file_name().into_string().unwrap();

		// TODO: Just remove gmod-update.txt from the directory
		// TODO: .sym files
		if entry_filename != "gmod-update.txt" {
			let entry_relative_path_str = if path_base.is_empty() { entry_filename } else { format!("{path_base}/{entry_filename}") };

			if entry_path.is_dir() {
				get_files_recursive(source, entry_relative_path_str, files, entry_path);
			} else if entry_path.is_file() {
				let file_hashmap = files.get_mut(&entry_relative_path_str);

				if file_hashmap.is_some() {
					let file_hashmap = file_hashmap.unwrap();
					file_hashmap.insert(source.to_string(), entry_path);
				} else {
					files.insert(entry_relative_path_str, HashMap::from([
						(source.to_string(), entry_path)
					]));
				}
			}
		}
	}
}

fn hash_and_diff_file(patch_path: PathBuf, filename: &String, file_paths: &HashMap<String, PathBuf>) -> Result<(f64, HashMap<String, String>), (f64, String)> {
	let now = Instant::now();
	let mut hashes: HashMap<String, String> = HashMap::new();

	let original_path = file_paths.get("original");
	let fixed_path = file_paths.get("fixed");

	let original_hash = match original_path.map(get_file_hash).unwrap_or_else(|| Ok("null".to_string())) {
		Ok(original_hash) => original_hash,
		Err(error) => return Err((now.elapsed().as_secs_f64(), error)),
	};
	let fixed_hash =  match fixed_path.map(get_file_hash).unwrap_or_else(|| Ok("null".to_string())) {
		Ok(fixed_hash) => fixed_hash,
		Err(error) => return Err((now.elapsed().as_secs_f64(), error)),
	};

	if original_hash == fixed_hash {
		return Err((now.elapsed().as_secs_f64(), "Skipped: Original hash matches Fixed hash".to_string()));
	}

	// Create patch file
	// Skip entirely if the "fixed" version is just deleting the file
	// If the original file doesn't exist, we "generate" the patch against an empty file
	if fixed_hash != "null" {
		let original = match original_path.map(std::fs::read).unwrap_or(Ok(Vec::new())) {
			Ok(original) => original,
			Err(error) => return Err((now.elapsed().as_secs_f64(), error.to_string()))
		};
		let fixed = match fixed_path.map(std::fs::read).unwrap_or(Ok(Vec::new())) {
			Ok(fixed) => fixed,
			Err(error) => return Err((now.elapsed().as_secs_f64(), error.to_string()))
		};

		let mut patch = io::Cursor::new(Vec::new());
		if let Err(error) = Bsdiff::new(&original, &fixed).compare(&mut patch) {
			return Err((now.elapsed().as_secs_f64(), error.to_string()));
		}
		let patch = patch.into_inner();

		// Copy patch to patch file
		let filename = format!("{filename}.bsdiff");
		let patch_file_path = patch_path.extend_and_return(filename.split("/"));
		if let Some(parent) = patch_file_path.parent() {
			if let Err(error) = std::fs::create_dir_all(parent) {
				return Err((now.elapsed().as_secs_f64(), error.to_string()));
			}
		}

		if let Err(error) = std::fs::write(&patch_file_path, &patch) {
			return Err((now.elapsed().as_secs_f64(), error.to_string()));
		}

		// Hash patch file (BEFORE compression)
		let patch_hash = format!("{}", blake3::hash(&patch));
		hashes.insert("patch".to_string(), patch_hash);

		// Figure out if the fixed file is an executable, and if so, mark it
		let mut executable = false;

		// MZ (Windows, technically DOS but not going to dig for PE header)
		// 0x4D 0x5A
		if fixed[0x00] == b'M' && fixed[0x01] == b'Z' {
			executable = true;
		}

		// ELF (Linux)
		// 0x7F 0x45 0x4C 0x46
		if fixed[0x00] == 0x7F && fixed[0x01] == b'E' && fixed[0x02] == b'L' && fixed[0x03] == b'F' {
			executable = true;
		}

		// Mach-O (macOS)
		// 0xCF 0xFA 0xED 0xFE
		if fixed[0x00] == 0xCF && fixed[0x01] == 0xFA && fixed[0x02] == 0xED && fixed[0x03] == 0xFE {
			executable = true;
		}

		hashes.insert("executable".to_string(), executable.to_string());
	}

	hashes.insert("original".to_string(), original_hash);
	hashes.insert("fixed".to_string(), fixed_hash);

	Ok((now.elapsed().as_secs_f64(), hashes))
}

pub fn main() {
	let now = Instant::now();

	println!("{}", ABOUT);

	// Parse the args (will also exit if something's wrong with them)
	let args = Args::parse();

	let mut cmd = Args::command();
	let original_path = match args.original_path.to_canonical_pathbuf(true) {
		Ok(original_path) => original_path,
		Err(error) => cmd.error(
			ErrorKind::InvalidValue,
			format!("Original Path: {error}"),
		)
		.exit(),
	};
	let fixed_path = match args.fixed_path.to_canonical_pathbuf(true) {
		Ok(fixed_path) => fixed_path,
		Err(error) => cmd.error(
			ErrorKind::InvalidValue,
			format!("Fixed Path: {error}"),
		)
		.exit(),
	};
	let patch_path = match args.patch_path.to_canonical_pathbuf(false) {
		Ok(patch_path) => patch_path,
		Err(error) => cmd.error(
			ErrorKind::InvalidValue,
			format!("Patch Path: {error}"),
		)
		.exit(),
	};

	println!("Original Path: {}\n", original_path.display());
	println!("Fixed Path: {}\n", fixed_path.display());
	println!("Patch Path: {}\n", patch_path.display());

	if original_path == fixed_path {
		cmd.error(
			ErrorKind::ValueValidation,
			"Original Path cannot match Fixed Path.",
		)
		.exit();
	}

	// TODO: Delete all files in patches path (+WARNING)

	println!("*** GENERATING PATCH FILES ***\n");

	let mut files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
	get_files_recursive("original", "".to_string(), &mut files, original_path);
	get_files_recursive("fixed", "".to_string(), &mut files, fixed_path);

	//println!("{:#?}", files);

	#[allow(clippy::type_complexity)]
	let _manifest: HashMap<String, HashMap<String, HashMap<String, HashMap<String, String>>>> = HashMap::new();

	// TODO: Early exit if any diffs/hashes fail
	files.par_iter().for_each(|(filename, file_paths)| {
		let result = hash_and_diff_file(patch_path.clone(), filename, file_paths);
		println!("{:#?}", result);
		// TODO
	});

	// TODO

	let _now = now.elapsed().as_secs_f64();
}
