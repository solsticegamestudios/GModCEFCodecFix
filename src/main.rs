pub const ABOUT: &str = "GModPatchTool

Formerly: GModCEFCodecFix

Copyright 2020-2025, Solstice Game Studios (www.solsticegamestudios.com)
LICENSE: GNU General Public License v3.0

Purpose: Patches Garry's Mod to resolve common launch/performance issues, Updates Chromium Embedded Framework (CEF), and Enables proprietary codecs in CEF.

Guide: https://www.solsticegamestudios.com/fixmedia/
FAQ/Common Issues: https://www.solsticegamestudios.com/fixmedia/faq/
Discord: https://www.solsticegamestudios.com/discord/
Email: contact@solsticegamestudios.com\n";

use std::path::{Path, PathBuf};
use blake3;

fn pathbuf_dir_not_empty(pathbuf: &PathBuf) -> bool {
	// If this is a valid file in the directory, the directory isn't empty
	if pathbuf.is_file() {
		return true;
	}

	let pathbuf_dir = pathbuf.read_dir();
	return if pathbuf_dir.is_ok() && pathbuf_dir.unwrap().next().is_some() { true } else { false };
}

pub fn pathbuf_to_canonical_pathbuf(pathbuf: PathBuf, checkdirempty: bool) -> Result<PathBuf, String> {
	let pathbuf_result = pathbuf.canonicalize();

	if pathbuf_result.is_ok() {
		let pathbuf = pathbuf_result.unwrap();

		if !checkdirempty || pathbuf_dir_not_empty(&pathbuf) {
			Ok(pathbuf)
		} else {
			Err("Directory is empty".to_string())
		}
	} else {
		Err(pathbuf_result.unwrap_err().to_string())
	}
}

pub fn string_to_canonical_pathbuf(path_str: String) -> Option<PathBuf> {
	let pathbuf_result = Path::new(&path_str).canonicalize();

	if pathbuf_result.is_ok() {
		let pathbuf = pathbuf_result.unwrap();

		if pathbuf_dir_not_empty(&pathbuf) {
			Some(pathbuf)
		} else {
			None
		}
	} else {
		None
	}
}

pub fn extend_pathbuf_and_return(mut pathbuf: PathBuf, segments: &[&str]) -> PathBuf {
	pathbuf.extend(segments);
	return pathbuf;
}

pub fn get_file_hash(file_path: &PathBuf) -> Result<String, String> {
	let mut hasher = blake3::Hasher::new();
	let hash_result = hasher.update_mmap_rayon(file_path);

	if hash_result.is_ok() {
		return Ok(format!("{}", hasher.finalize()));
	} else {
		return Err(hash_result.unwrap_err().to_string())
	}
}

#[cfg(feature = "generate")]
mod generate;

#[cfg(feature = "patch")]
mod patch;

fn main() {
	#[cfg(feature = "generate")]
	generate::main();

	#[cfg(feature = "patch")]
	patch::main();
}
