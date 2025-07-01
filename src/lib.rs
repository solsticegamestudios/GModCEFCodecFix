#[cfg(feature = "generate")]
pub mod generate;

#[cfg(feature = "patch")]
pub mod patch;

#[cfg(feature = "patch")]
mod gui;

const ABOUT: &str = "GModPatchTool

Formerly: GModCEFCodecFix

Copyright 2020-2025, Solstice Game Studios (www.solsticegamestudios.com)
LICENSE: GNU General Public License v3.0

Purpose: Patches Garry's Mod to resolve common launch/performance issues, Updates Chromium Embedded Framework (CEF), and Enables proprietary codecs in CEF.

Guide: https://www.solsticegamestudios.com/fixmedia/
FAQ/Common Issues: https://www.solsticegamestudios.com/fixmedia/faq/
Discord: https://www.solsticegamestudios.com/discord/
Email: contact@solsticegamestudios.com\n";

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use rayon::prelude::*;

type Manifest = HashMap<String, HashMap<String, HashMap<String, HashMap<String, String>>>>;

fn pathbuf_dir_not_empty(pathbuf: &Path) -> bool {
	// If this is a valid file in the directory, the directory isn't empty
	if pathbuf.is_file() {
		return true;
	}

	let pathbuf_dir = pathbuf.read_dir();

	pathbuf_dir.is_ok() && pathbuf_dir.unwrap().next().is_some()
}

fn pathbuf_to_canonical_pathbuf(pathbuf: PathBuf, checkdirempty: bool) -> Result<PathBuf, String> {
	#[cfg(windows)]
	use dunce::canonicalize;
	#[cfg(not(windows))]
	let canonicalize = Path::canonicalize;

	let pathbuf_result = canonicalize(pathbuf.as_path());

	match pathbuf_result {
		Ok(pathbuf) => {
			if !checkdirempty || pathbuf_dir_not_empty(&pathbuf) {
				Ok(pathbuf)
			} else {
				Err("Directory is empty".to_string())
			}
		},
		Err(error) => {
			Err(error.to_string())
		}
	}
}

fn string_to_canonical_pathbuf(path_str: String) -> Option<PathBuf> {
	#[cfg(windows)]
	use dunce::canonicalize;
	#[cfg(not(windows))]
	let canonicalize = Path::canonicalize;

	let pathbuf_result = canonicalize(Path::new(&path_str));

	if let Ok(pathbuf) = pathbuf_result {
		if pathbuf_dir_not_empty(&pathbuf) {
			return Some(pathbuf);
		}
	}

	None
}

fn extend_pathbuf_and_return(mut pathbuf: PathBuf, segments: &[&str]) -> PathBuf {
	pathbuf.extend(segments);

	pathbuf
}

fn get_file_hash(file_path: &PathBuf) -> Result<String, String> {
	let mut hasher = blake3::Hasher::new();
	let hash_result = hasher.update_mmap_rayon(file_path);

	match hash_result {
		Ok(_) => {
			Ok(format!("{}", hasher.finalize()))
		},
		Err(error) => {
			Err(error.to_string())
		}
	}
}
