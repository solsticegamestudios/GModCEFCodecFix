use crate::*;
use clap::{CommandFactory, Parser};
use clap::error::ErrorKind;
use std::path::PathBuf;
use std::collections::HashMap;
use rayon::prelude::*;
use std::time::Instant;
use qbsdiff::Bsdiff;
use std::sync::Mutex;
use std::ops::Deref;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
	/// Path for Original files shipped with GMod
	original_path: PathBuf,

	/// Path for Fixed (already-patched) files
	fixed_path: PathBuf,

	/// Path for where to put the output Patch (bsdiff) files
	patch_path: PathBuf,

	/// Path for where to copy the compressed versions of the Original files
	original_compressed_path: PathBuf
}

fn get_files_recursive(source: &str, path_base: String, files: &mut HashMap<String, HashMap<String, PathBuf>>, dir_path: PathBuf) {
	for entry in std::fs::read_dir(dir_path).unwrap() {
		let entry = entry.unwrap();
		let entry_path = entry.path();
		let entry_filename = entry.file_name().into_string().unwrap();

		// TODO: Move gmod-update.txt and .sym files from these directories
		if entry_filename != "gmod-update.txt" && !entry_filename.contains(".sym") {
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

fn hash_diff_compress_file(patch_path: PathBuf, filename: &String, file_paths: &HashMap<String, PathBuf>, original_compressed_path: PathBuf) -> Result<(f64, HashMap<String, String>), (bool, String)> {
	let now = Instant::now();
	let mut hashes: HashMap<String, String> = HashMap::new();

	let original_path = file_paths.get("original");
	let fixed_path = file_paths.get("fixed");
	let original_hash = if original_path.is_some() { get_file_hash(original_path.unwrap()) } else { Ok("null".to_string()) };
	let fixed_hash = if fixed_path.is_some() { get_file_hash(fixed_path.unwrap()) } else { Ok("null".to_string()) };

	if original_hash.is_err() {
		return Err((true, original_hash.unwrap_err()));
	}
	if fixed_hash.is_err() {
		return Err((true, fixed_hash.unwrap_err()));
	}

	let original_hash = original_hash.unwrap();
	let fixed_hash = fixed_hash.unwrap();

	if original_hash == fixed_hash {
		return Err((false, "Skipped: Original hash matches Fixed hash".to_string()));
	}

	// Create patch file
	// Skip entirely if the "fixed" version is just deleting the file
	// If the original file doesn't exist, we "generate" the patch against an empty file
	if fixed_hash != "null" {
		let original = if original_path.is_some() { std::fs::read(original_path.unwrap()) } else { Ok(Vec::new()) };
		let fixed = if fixed_path.is_some() { std::fs::read(fixed_path.unwrap()) } else { Ok(Vec::new()) };

		if original.is_err() {
			return Err((true, original.unwrap_err().to_string()));
		}
		if fixed.is_err() {
			return Err((true, fixed.unwrap_err().to_string()));
		}

		let original = original.unwrap();
		let fixed = fixed.unwrap();

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

		if executable {
			hashes.insert("executable".to_string(), executable.to_string());
		}

		let mut patch = Vec::new();
		let diff_result = Bsdiff::new(&original, &fixed).compare(std::io::Cursor::new(&mut patch));

		if diff_result.is_err() {
			return Err((true, diff_result.unwrap_err().to_string()));
		}

		// Free original/fixed memory before writing the patch file (which might take a while)
		std::mem::drop(original);
		std::mem::drop(fixed);

		// Copy patch to patch file
		let filename = format!("{filename}.bsdiff");
		let file_parts: Vec<&str> = filename.split("/").collect();
		let patch_file_path = extend_pathbuf_and_return(patch_path, &file_parts[..]);
		let mut patch_file_path_dir = patch_file_path.clone();
		patch_file_path_dir.pop();

		let create_dir_result = std::fs::create_dir_all(patch_file_path_dir);
		if create_dir_result.is_err() {
			return Err((true, create_dir_result.unwrap_err().to_string()));
		}

		let patch_write_result = std::fs::write(patch_file_path.clone(), &patch);
		if patch_write_result.is_err() {
			return Err((true, patch_write_result.unwrap_err().to_string()));
		}

		// NOTE(winter): qbsdiff compresses the patch file already, so we don't need to do it ourselves
		// Compress patch file
		//let mut patch_compressed: Vec<u8> = Vec::new();
		//let compress_result = zstd::stream::copy_encode(&patch[..], &mut patch_compressed, 11);
		//if compress_result.is_err() {
		//	return Err((true, compress_result.unwrap_err().to_string()));
		//}

		//let compressed_write_result = std::fs::write(patch_file_path.clone(), &patch_compressed);
		//if compressed_write_result.is_err() {
		//	return Err((true, compressed_write_result.unwrap_err().to_string()));
		//}

		// Hash patch file (AFTER compression, since qbsdiff does it itself)
		let patch_hash = format!("{}", blake3::hash(&patch));
		hashes.insert("patch".to_string(), patch_hash);
	}

	// Create a compressed copy of the original file
	if original_hash != "null" {
		let original_path = original_path.unwrap();
		let filename = format!("{filename}.zst");
		let file_parts: Vec<&str> = filename.split("/").collect();
		let original_compressed_file_path = extend_pathbuf_and_return(original_compressed_path, &file_parts[..]);

		let original_file = std::fs::OpenOptions::new().read(true).open(original_path);
		if original_file.is_err() {
			return Err((true, original_file.unwrap_err().to_string()));
		}
		let original_file = original_file.unwrap();

		let mut original_compressed_file_path_dir = original_compressed_file_path.clone();
		original_compressed_file_path_dir.pop();

		let create_dir_result = std::fs::create_dir_all(original_compressed_file_path_dir);
		if create_dir_result.is_err() {
			return Err((true, create_dir_result.unwrap_err().to_string()));
		}

		let original_file_compressed = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(original_compressed_file_path);
		if original_file_compressed.is_err() {
			return Err((true, original_file_compressed.unwrap_err().to_string()));
		}
		let original_file_compressed = original_file_compressed.unwrap();

		let compress_result = zstd::stream::copy_encode(original_file, original_file_compressed, 0);
		if compress_result.is_err() {
			return Err((true, compress_result.unwrap_err().to_string()));
		}
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
	let original_compressed_path = pathbuf_to_canonical_pathbuf(args.original_compressed_path.clone(), false);

	let mut cmd = Args::command();
	if original_path.is_err() {
		cmd.error(
			ErrorKind::InvalidValue,
			format!("Original Path (Input): {}", original_path.unwrap_err().to_string()),
		)
		.exit();
	}

	let original_path = original_path.unwrap();
	let original_path_str = original_path.to_string_lossy();
	println!("Original Path (Input): {}\n", original_path_str);

	if fixed_path.is_err() {
		cmd.error(
			ErrorKind::InvalidValue,
			format!("Fixed Path (Input): {}", fixed_path.unwrap_err().to_string()),
		)
		.exit();
	}

	let fixed_path = fixed_path.unwrap();
	let fixed_path_str = fixed_path.to_string_lossy();
	println!("Fixed Path (Input): {}\n", fixed_path_str);

	if patch_path.is_err() {
		cmd.error(
			ErrorKind::InvalidValue,
			format!("Patch Path (Output): {}", patch_path.unwrap_err().to_string()),
		)
		.exit();
	}

	let patch_path = patch_path.unwrap();
	let patch_path_str = patch_path.to_string_lossy();
	println!("Patch Path (Output): {}\n", patch_path_str);

	if original_compressed_path.is_err() {
		cmd.error(
			ErrorKind::InvalidValue,
			format!("Original Compressed Path (Output): {}", original_compressed_path.unwrap_err().to_string()),
		)
		.exit();
	}

	let original_compressed_path = original_compressed_path.unwrap();
	let original_compressed_path_str = original_compressed_path.to_string_lossy();
	println!("Original Compressed Path (Output): {}\n", original_compressed_path_str);

	if original_path == fixed_path {
		cmd.error(
			ErrorKind::ValueValidation,
			"Original Path cannot match Fixed Path.",
		)
		.exit();
	}

	if original_compressed_path == fixed_path {
		cmd.error(
			ErrorKind::ValueValidation,
			"Original Compressed Path cannot match Fixed Path.",
		)
		.exit();
	}

	let mut manifest_file_path = patch_path.clone();
	manifest_file_path.pop();
	let manifest_file_path = extend_pathbuf_and_return(manifest_file_path, &["manifest.json"]);

	println!("Deleting Old Patches Dir, Compressed Original Dir, and Manifest...\n");

	std::fs::remove_dir_all(&patch_path);
	std::fs::create_dir(&patch_path);
	std::fs::remove_dir_all(&original_compressed_path);
	std::fs::create_dir(&original_compressed_path);
	std::fs::remove_file(&manifest_file_path);

	println!("*** GENERATING PATCH FILES ***\n");

	let mut files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
	get_files_recursive("original", "".to_string(), &mut files, original_path);
	get_files_recursive("fixed", "".to_string(), &mut files, fixed_path);

	let manifest: Mutex<HashMap<String, HashMap<String, HashMap<String, HashMap<String, String>>>>> = Mutex::new(HashMap::new());

	files.par_iter().for_each(|(filename, file_paths)| {
		let result = hash_diff_compress_file(patch_path.clone(), filename, file_paths, original_compressed_path.clone());
		if result.is_ok() {
			let (time, hashes) = result.unwrap();
			println!("\t{filename}\n\t\tTook {time} second(s)");

			let file_parts: Vec<&str> = filename.split("/").collect();
			let platform = file_parts[0].to_string();
			let gmod_branch = file_parts[1].to_string();
			let filename = file_parts[2..].join("/");

			let mut manifest_locked = manifest.lock().unwrap();

			let mut platform_branches = manifest_locked.get_mut(&platform);
			if platform_branches.is_none() {
				manifest_locked.insert(platform.clone(), HashMap::new());
				platform_branches = manifest_locked.get_mut(&platform);
			}
			let platform_branches = platform_branches.unwrap();

			let mut platform_branch_files = platform_branches.get_mut(&gmod_branch);
			if platform_branch_files.is_none() {
				platform_branches.insert(gmod_branch.clone(), HashMap::new());
				platform_branch_files = platform_branches.get_mut(&gmod_branch);
			}
			let platform_branch_files = platform_branch_files.unwrap();

			platform_branch_files.insert(filename, hashes);
		} else {
			let (fatal, error_string) = result.unwrap_err();
			println!("\t{filename}\n\t\t{error_string}");

			if fatal {
				println!("\t\tFATAL ERROR, EXITING...\n");
				std::process::exit(1);
			}
		}
	});

	// TODO: Sort file names alphabetically so Git doesn't think it changes every time we generate it
	let manifest_guard = manifest.lock().unwrap();
	let manifest = manifest_guard.deref();

	println!("\n*** GENERATING MANIFEST JSON ***\n");

	// HACK(winter): Replace the stupid double-space indentation with proper tabbed indentation
	// The correct way to do this is with serde::Serialize, but that requires importing the whole serde library, so I don't care
	// WARN(winter): This may break at some point!...but it'll probably still work, just with wonky formatting
	// Also add newline at the end to make Git happy
	let mut manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
	manifest_json = manifest_json.replace("  ", "	");
	manifest_json += "\n";

	let write_result = std::fs::write(manifest_file_path.clone(), &manifest_json);
	write_result.unwrap();

	let now = now.elapsed().as_secs_f64();
	println!("Patch generation complete! Took {now} second(s).");
}
