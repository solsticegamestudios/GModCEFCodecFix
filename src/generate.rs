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
			let entry_relative_path_str = if path_base == "" { entry_filename } else { format!("{path_base}/{entry_filename}") };

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
	let original_hash = if original_path.is_some() { get_file_hash(&original_path.unwrap()) } else { Ok("null".to_string()) };
	let fixed_hash = if fixed_path.is_some() { get_file_hash(&fixed_path.unwrap()) } else { Ok("null".to_string()) };

	if original_hash.is_err() {
		return Err((now.elapsed().as_secs_f64(), original_hash.unwrap_err()));
	}
	if fixed_hash.is_err() {
		return Err((now.elapsed().as_secs_f64(), fixed_hash.unwrap_err()));
	}

	let original_hash = original_hash.unwrap();
	let fixed_hash = fixed_hash.unwrap();

	if original_hash == fixed_hash {
		return Err((now.elapsed().as_secs_f64(), "Skipped: Original hash matches Fixed hash".to_string()));
	}

	// Create patch file
	// Skip entirely if the "fixed" version is just deleting the file
	// If the original file doesn't exist, we "generate" the patch against an empty file
	if fixed_hash != "null" {
		let original = if original_path.is_some() { std::fs::read(original_path.unwrap()) } else { Ok(Vec::new()) };
		let fixed = if fixed_path.is_some() { std::fs::read(fixed_path.unwrap()) } else { Ok(Vec::new()) };

		if original.is_err() {
			return Err((now.elapsed().as_secs_f64(), original.unwrap_err().to_string()));
		}
		if fixed.is_err() {
			return Err((now.elapsed().as_secs_f64(), fixed.unwrap_err().to_string()));
		}

		let original = original.unwrap();
		let fixed = fixed.unwrap();

		let mut patch = Vec::new();
		let diff_result = Bsdiff::new(&original, &fixed).compare(std::io::Cursor::new(&mut patch));

		if diff_result.is_err() {
			return Err((now.elapsed().as_secs_f64(), diff_result.unwrap_err().to_string()));
		}

		// Copy patch to patch file
		let filename = format!("{filename}.bsdiff");
		let file_parts: Vec<&str> = filename.split("/").collect();
		let patch_file_path = extend_pathbuf_and_return(patch_path, &file_parts[..]);
		let mut patch_file_path_dir = patch_file_path.clone();
		patch_file_path_dir.pop();

		let create_dir_result = std::fs::create_dir_all(patch_file_path_dir);
		if create_dir_result.is_err() {
			return Err((now.elapsed().as_secs_f64(), create_dir_result.unwrap_err().to_string()));
		}

		let write_result = std::fs::write(patch_file_path.clone(), &patch);
		if write_result.is_err() {
			return Err((now.elapsed().as_secs_f64(), write_result.unwrap_err().to_string()));
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

	let original_path = pathbuf_to_canonical_pathbuf(args.original_path.clone(), true);
	let fixed_path = pathbuf_to_canonical_pathbuf(args.fixed_path.clone(), true);
	let patch_path = pathbuf_to_canonical_pathbuf(args.patch_path.clone(), false);

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

	// TODO: Delete all files in patches path (+WARNING)

	println!("*** GENERATING PATCH FILES ***\n");

	let mut files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
	get_files_recursive("original", "".to_string(), &mut files, original_path);
	get_files_recursive("fixed", "".to_string(), &mut files, fixed_path);

	//println!("{:#?}", files);

	let mut manifest: HashMap<String, HashMap<String, HashMap<String, HashMap<String, String>>>> = HashMap::new();

	// TODO: Early exit if any diffs/hashes fail
	files.par_iter().for_each(|(filename, file_paths)| {
		let result = hash_and_diff_file(patch_path.clone(), filename, file_paths);
		println!("{:#?}", result);
		// TODO
	});

	// TODO

	let now = now.elapsed().as_secs_f64();
}
