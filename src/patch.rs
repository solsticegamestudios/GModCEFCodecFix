const VERSION_SERVER_ROOTS: [&str; 2] = [
	"https://raw.githubusercontent.com/solsticegamestudios/GModPatchTool/refs/heads/master/",
	"https://www.solsticegamestudios.com/gmodpatchtool/"
];

const MANIFEST_SERVER_ROOTS: [&str; 2] = [
	"https://raw.githubusercontent.com/solsticegamestudios/GModPatchTool/refs/heads/files/",
	"https://www.solsticegamestudios.com/gmodpatchtool/"
];

// TODO: Compress patch/original files on these servers (GitHub does not serve them with Content-Encoding, + Bandwidth/Storage costs!!!)
const PATCH_SERVER_ROOTS: [&str; 2] = [
	//"https://media.githubusercontent.com/media/solsticegamestudios/GModPatchTool/refs/heads/files/",
	"https://media.githubusercontent.com/media/solsticegamestudios/GModCEFCodecFix/refs/heads/files/",
	"https://www.solsticegamestudios.com/gmodpatchtool/"
];

const GMOD_STEAM_APPID: &str = "4000";
const BLANK_FILE_HASH: &str = "null";

use crate::*;
mod gui;

use eframe::egui::TextBuffer;
use tracing::error;
use tracing_subscriber::filter::EnvFilter;
use clap::Parser;
use std::io::IsTerminal;
use phf::phf_map;
use phf::Map;
use std::{thread, time};
use std::thread::JoinHandle;
use std::path::PathBuf;
use std::fs::read_to_string;
use keyvalues_parser::Vdf;
use std::collections::HashMap;
use steamid::SteamId;
use sysinfo::System;
use std::fs::File;
use std::io;
use rayon::prelude::*;
use reqwest::Response;
use tokio::time::Instant;
use tokio::task::JoinSet;
use qbsdiff::Bspatch;

#[cfg(windows)]
use is_elevated::is_elevated;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Parser)]
#[command(version)]
struct Args {
	/// Force a specific Garry's Mod launch entry
	#[arg(short, long, value_name = "LAUNCH_OPTION")]
	auto_mode: Option<u8>,

	/// Force a specific Steam install path (NOT a Steam library path)
	#[arg(short, long)]
	steam_path: Option<PathBuf>,

	/// Allow running the tool as root/admin (NOT RECOMMENDED!!!)
	#[arg(long)]
	run_as_root_with_security_risk: bool,
}

const COLOR_LOOKUP: Map<&'static str, &'static str> =
phf_map! {
	"red" => "\x1B[1;31m",
	"green" => "\x1B[1;32m",
	"yellow" => "\x1B[1;33m",
	"cyan" => "\x1B[1;36m"
};

fn terminal_write<W>(writer: fn() -> W, output: &str, newline: bool, color: Option<&str>)
where
	W: std::io::Write + 'static
{
	if color.is_some() && COLOR_LOOKUP.contains_key(color.unwrap()) {
		write!(writer(), "{}", COLOR_LOOKUP[color.unwrap()]).unwrap();
	}

	if newline {
		writeln!(writer(), "{output}").unwrap();
	} else {
		write!(writer(), "{output}").unwrap();
	}

	if color.is_some() {
		write!(writer(), "\x1B[0m").unwrap();
	}
}

use thiserror::Error;
#[derive(Debug, Error)]
enum AlmightyError {
	#[error("HTTP Error: {0}")]
	Http(#[from] reqwest::Error),
	#[error("Remote Version parsing error: {0}")]
	Parse(#[from] std::num::ParseIntError),
	#[error("{0}")]
	Generic(String)
}

async fn get_http_response<W>(writer: fn() -> W, writer_is_interactive: bool, servers: &[&str], filename: &str) -> Option<Response>
where
	W: std::io::Write + 'static
{
	let mut server_id: u8 = 0;
	let mut try_count: u8 = 0;
	let mut response = None;
	while (server_id as usize) < servers.len() {
		let url = servers[server_id as usize].to_string() + filename;
		let response_result = reqwest::get(url.clone()).await; // TODO: Timeout check

		if response_result.is_ok() {
			response = response_result.ok();

			let response_status_code = response.as_ref().unwrap().status().as_u16();
			if response_status_code == 200 {
				break;
			} else {
				terminal_write(writer, format!("\n{url}\n\tBad HTTP Status Code: {response_status_code}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
				response = None;
				server_id += 1;
				try_count = 0;
			}
		} else {
			let error = response_result.unwrap_err().without_url();
			terminal_write(writer, format!("\n{url}\n\tHTTP Error: {error}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
			response = None;
			try_count += 1;

			// Try each server 3 times for full HTTP errors (Anti-DDoS, etc)
			if try_count >= 3 {
				server_id += 1;
				try_count = 0;
			}
		}
	}

	return response;
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum IntegrityStatus {
	NeedDelete = 0,
	NeedOriginal = 1,
	NeedWipeFix = 2,
	NeedFix = 3,
	Fixed = 4
}

fn determine_file_integrity_status(gmod_path: PathBuf, filename: &String, hashes: &HashMap<String, String>) -> Result<IntegrityStatus, String> {
	let file_parts: Vec<&str> = filename.split("/").collect();
	let file_path = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(gmod_path, &file_parts[..]), false);
	let mut file_hash = BLANK_FILE_HASH.to_string();

	if file_path.is_ok() {
		let file_hash_result = get_file_hash(&file_path.unwrap());
		if file_hash_result.is_ok() {
			file_hash = file_hash_result.unwrap();
		} else {
			return Err(file_hash_result.unwrap_err());
		}
	}

	if file_hash == hashes["fixed"] {
		return Ok(IntegrityStatus::Fixed);
	} else {
		// File needs to be fixed...
		if hashes["fixed"] == BLANK_FILE_HASH {
			// This is a file that doesn't exist anymore after patching
			return Ok(IntegrityStatus::NeedDelete);
		} else if hashes["original"] == BLANK_FILE_HASH {
			// The original file didn't exist, so we need to wipe/create the file, then patch it
			return Ok(IntegrityStatus::NeedWipeFix);
		} else if file_hash == hashes["original"] {
			// The file is the original, so we just to apply the patch
			return Ok(IntegrityStatus::NeedFix);
		} else {
			// We don't recognize the hash, so we need to first replace the file with the original (which we'll download), then apply the patch to that file
			return Ok(IntegrityStatus::NeedOriginal);
		}
	}
}

async fn download_file_to_cache<W>(writer: fn() -> W, writer_is_interactive: bool, cache_dir: PathBuf, filename: String, target_hash: String) -> Result<(), ()>
where
	W: std::io::Write + 'static
{
	let filename_no_zst = if filename.ends_with(".zst") {
		let len = filename.len() - 4;
		filename[..len].to_string()
	} else {
		filename.clone()
	};
	let file_parts: Vec<&str> = filename_no_zst.split("/").collect();
	let cache_file_path = extend_pathbuf_and_return(cache_dir, &file_parts[..]);
	let cache_file_path_result = pathbuf_to_canonical_pathbuf(cache_file_path.clone(), false);

	terminal_write(writer, format!("\tDownloading: {filename} ...").as_str(), true, None);

	// Look in the cache to see if the file already exists
	if cache_file_path_result.is_ok() {
		let file_hash_result = get_file_hash(&cache_file_path);
		if file_hash_result.is_ok() {
			let file_hash = file_hash_result.unwrap();

			if file_hash == target_hash {
				terminal_write(writer, format!("\tDownloaded (From Cache): {filename}").as_str(), true, None);
				return Ok(());
			}
		}
	}

	// If it's not in the cache, or there's a checksum mismatch with the version in the cache, (re-)download it
	let response = get_http_response(writer, writer_is_interactive, &PATCH_SERVER_ROOTS, filename.as_str()).await;
	if response.is_some() {
		let response = response.unwrap();
		let bytes_raw = response.bytes().await;

		if bytes_raw.is_ok() {
			let bytes_raw = bytes_raw.unwrap();

			// Create directories if needed
			let mut cache_file_path_dir = cache_file_path.clone();
			cache_file_path_dir.pop();
			let cache_file_path_dir_canonical = pathbuf_to_canonical_pathbuf(cache_file_path_dir.clone(), false);

			if cache_file_path_dir_canonical.is_err() {
				let create_dir_result = std::fs::create_dir_all(cache_file_path_dir);
				if create_dir_result.is_err() {
					let error_string = create_dir_result.unwrap_err().to_string();
					terminal_write(writer, format!("\tFailed to Download: {filename} | Step 2: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
					return Err(());
				}
			}

			// Decompress Zstandard files
			let mut bytes: Vec<u8> = if filename.ends_with(".zst") { Vec::new() } else { bytes_raw.to_vec() };
			if filename.ends_with(".zst") {
				terminal_write(writer, format!("\tDecompressing: {filename} ...").as_str(), true, None);

				let decompress_result = zstd::stream::copy_decode(&bytes_raw[..], &mut bytes);
				if decompress_result.is_ok() {
					terminal_write(writer, format!("\tDecompressed: {filename}").as_str(), true, None);
				} else {
					let error_string = decompress_result.unwrap_err().to_string();
					terminal_write(writer, format!("\tFailed to Decompress: {filename} | {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
					return Err(());
				}
			}

			let write_result = std::fs::write(cache_file_path.clone(), bytes);
			if write_result.is_ok() {
				let file_hash_result = get_file_hash(&cache_file_path);

				if file_hash_result.is_ok() {
					let file_hash = file_hash_result.unwrap();

					if file_hash == target_hash {
						terminal_write(writer, format!("\tDownloaded: {filename}").as_str(), true, None);
						return Ok(());
					} else {
						terminal_write(writer, format!("\tFailed to Download: {filename} | Step 4: Checksum mismatch").as_str(), true, if writer_is_interactive { Some("red") } else { None });
					}
				} else {
					let error_string = file_hash_result.unwrap_err().to_string();
					terminal_write(writer, format!("\tFailed to Download: {filename} | Step 3: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
				}
			} else {
				let error_string = write_result.unwrap_err().to_string();
				terminal_write(writer, format!("\tFailed to Download: {filename} | Step 2: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
			}
		} else {
			let error_string = bytes_raw.unwrap_err().to_string();
			terminal_write(writer, format!("\tFailed to Download: {filename} | Step 1: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
		}
	}

	return Err(());
}

#[cfg(unix)]
#[link(name = "c")]
unsafe extern "C" {
	safe fn geteuid() -> u32;
}

async fn main_script_internal<W>(writer: fn() -> W, writer_is_interactive: bool, args: Args) -> Result<(), AlmightyError>
where
	W: std::io::Write + 'static
{
	let now = Instant::now();

	// Get local version
	let local_version: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();

	// Get remote version
	terminal_write(writer, "Getting remote version...", true, None);

	let remote_version_response = get_http_response(writer, writer_is_interactive, &VERSION_SERVER_ROOTS, "version.txt").await;

	if remote_version_response.is_none() {
		return Err(AlmightyError::Generic(format!("Couldn't get remote version. Please check your internet connection!")));
	}

	let remote_version_response = remote_version_response.unwrap();
	let remote_version: u32 = remote_version_response.text()
	.await?
	.trim()
	.parse()?;

	if local_version >= remote_version {
		terminal_write(writer, format!("You are running the latest version of GModPatchTool [Local: {local_version} / Remote: {remote_version}]!\n").as_str(), true, if writer_is_interactive { Some("green") } else { None });
	} else {
		terminal_write(writer, "WARNING: GModPatchTool is out of date! Please get the latest version at\nhttps://github.com/solsticegamestudios/GModPatchTool/releases", true, if writer_is_interactive { Some("red") } else { None });

		let mut secs_to_continue: u8 = 5;
		while secs_to_continue > 0 {
			terminal_write(writer, format!("\tContinuing in {secs_to_continue} second(s)...\r").as_str(), false, if writer_is_interactive { Some("yellow") } else { None });
			writer().flush().unwrap();
			tokio::time::sleep(time::Duration::from_secs(1)).await;
			secs_to_continue -= 1;
		}

		// Clear continuing line
		if writer_is_interactive {
			terminal_write(writer, "\x1B[0K\n", false, None);
		}
	}

	// Warn/Exit if running as root/admin
	#[cfg(windows)]
	let root = is_elevated();

	#[cfg(unix)]
	let root = geteuid() == 0;

	if root {
		if args.run_as_root_with_security_risk {
			terminal_write(writer, "WARNING: You are running GModPatchTool as root/with admin privileges. This may cause issues and is not typically necessary.", true, if writer_is_interactive { Some("red") } else { None });

			let mut secs_to_continue: u8 = 10;
			while secs_to_continue > 0 {
				terminal_write(writer, format!("\tContinuing in {secs_to_continue} second(s)...\r").as_str(), false, if writer_is_interactive { Some("yellow") } else { None });
				writer().flush().unwrap();
				tokio::time::sleep(time::Duration::from_secs(1)).await;
				secs_to_continue -= 1;
			}

			// Clear continuing line
			if writer_is_interactive {
				terminal_write(writer, "\x1B[0K\n", false, None);
			}
		} else {
			return Err(AlmightyError::Generic("You are running GModPatchTool as root/with admin privileges. This may cause issues and is not typically necessary.\n\nIF YOU KNOW WHAT YOU'RE DOING, you can allow this using --run-as-root-with-security-risk. Aborting...".to_string()));
		}
	}

	// Abort if GMod is currently running
	let sys = System::new_all();
	if sys.processes_by_exact_name("gmod.exe".as_ref()).next().is_some() || sys.processes_by_exact_name("gmod".as_ref()).next().is_some() {
		return Err(AlmightyError::Generic("Garry's Mod is currently running. Please close it before running this tool.".to_string()));
	}

	// Find Steam
	let mut steam_path = None;
	if args.steam_path.is_some() {
		// Make sure the path the user is forcing actually exists
		let steam_path_arg = args.steam_path.unwrap();
		let steam_path_arg_pathbuf = pathbuf_to_canonical_pathbuf(steam_path_arg.clone(), true);

		if steam_path_arg_pathbuf.is_ok() {
			steam_path = Some(steam_path_arg_pathbuf.unwrap());
		} else {
			return Err(AlmightyError::Generic(format!("Please check the --steam_path argument is pointing to a valid path:\n\t{}", steam_path_arg_pathbuf.unwrap_err())));
		}
	} else {
		// Windows
		#[cfg(windows)]
		{
			let steam_reg_key = windows_registry::CURRENT_USER.open("Software\\Valve\\Steam");
			if steam_reg_key.is_ok() {
				let steam_reg_path = steam_reg_key.unwrap().get_string("SteamPath");

				if steam_reg_path.is_ok() {
					steam_path = string_to_canonical_pathbuf(steam_reg_path.unwrap());
				}
			}
		}

		// macOS
		#[cfg(target_os = "macos")]
		{
			// $HOME/Library/Application Support/Steam
			let mut steam_data_path = dirs::data_dir().unwrap();
			steam_data_path.push("Steam");
			steam_path = pathbuf_to_canonical_pathbuf(steam_data_path, true).ok();
		}

		// Anything else (we assume Linux)
		#[cfg(not(any(windows, target_os = "macos")))]
		{
			let home_dir = dirs::home_dir().unwrap();
			let possible_steam_paths = vec![
				// Snap
				extend_pathbuf_and_return(home_dir.clone(), &["snap", "steam", "common", ".local", "share", "Steam"]),
				extend_pathbuf_and_return(home_dir.clone(), &["snap", "steam", "common", ".steam", "steam"]),
				// Flatpak
				extend_pathbuf_and_return(home_dir.clone(), &[".var", "app", "com.valvesoftware.Steam", ".local", "share", "Steam"]),
				extend_pathbuf_and_return(home_dir.clone(), &[".var", "app", "com.valvesoftware.Steam", ".steam", "steam"]),
				// Home
				extend_pathbuf_and_return(home_dir.clone(), &[".steam", "steam"]),
				//extend_pathbuf_and_return(home_dir.clone(), &[".steam"]),
			];
			let mut valid_steam_paths = vec![];

			for pathbuf in possible_steam_paths {
				let pathbuf = pathbuf_to_canonical_pathbuf(pathbuf, true);
				if pathbuf.is_ok() {
					let pathbuf = pathbuf.unwrap();
					if !valid_steam_paths.contains(&pathbuf) {
						valid_steam_paths.push(pathbuf);
					}
				}
			}

			// $XDG_DATA_HOME/Steam
			let steam_xdg_path = dirs::data_dir();
			if steam_xdg_path.is_some() {
				let steam_xdg_pathbuf = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(steam_xdg_path.unwrap(), &["Steam"]), true);
				if steam_xdg_pathbuf.is_ok() {
					let steam_xdg_pathbuf = steam_xdg_pathbuf.unwrap();
					if !valid_steam_paths.contains(&steam_xdg_pathbuf) {
						valid_steam_paths.push(steam_xdg_pathbuf);
					}
				}
			}

			// Set the Steam path if at least one is valid
			// Warn if there's more than one
			if valid_steam_paths.len() >= 1 {
				if valid_steam_paths.len() > 1 {
					let mut valid_steam_paths_str: String = "".to_string();
					for pathbuf in &valid_steam_paths {
						valid_steam_paths_str += "\n\t- ";
						valid_steam_paths_str += &pathbuf.to_string_lossy().to_string();
					}

					terminal_write(writer, format!("Warning: Multiple Steam Installations Detected! This may cause issues:{valid_steam_paths_str}").as_str(), true, if writer_is_interactive { Some("yellow") } else { None });

					let mut secs_to_continue: u8 = 5;
					while secs_to_continue > 0 {
						terminal_write(writer, format!("\tContinuing in {secs_to_continue} second(s)...\r").as_str(), false, if writer_is_interactive { Some("yellow") } else { None });
						writer().flush().unwrap();
						tokio::time::sleep(time::Duration::from_secs(1)).await;
						secs_to_continue -= 1;
					}

					// Clear continuing line
					if writer_is_interactive {
						terminal_write(writer, "\x1B[0K\n", false, None);
					}
				}

				steam_path = Some(valid_steam_paths[0].clone());
			}
		}
	}

	if steam_path.is_none() {
		return Err(AlmightyError::Generic("Couldn't find Steam. If it's installed, try using the --steam_path argument to force a specific path.".to_string()));
	}

	let steam_path = steam_path.unwrap();
	let steam_path_str = steam_path.to_string_lossy();

	terminal_write(writer, format!("Steam Path: {steam_path_str}\n").as_str(), true, None);

	// Get most recent Steam User, which is probably the one they're using/want
	let steam_loginusers_path = extend_pathbuf_and_return(steam_path.clone(), &["config", "loginusers.vdf"]);
	let steam_loginusers_str = read_to_string(steam_loginusers_path);

	if steam_loginusers_str.is_err() {
		return Err(AlmightyError::Generic("Couldn't find Steam loginusers.vdf. Have you ever launched/signed in to Steam?".to_string()));
	}

	let steam_loginusers_str = steam_loginusers_str.unwrap();
	let steam_loginusers = Vdf::parse(steam_loginusers_str.as_str());

	if steam_loginusers.is_err() {
		return Err(AlmightyError::Generic("Couldn't parse Steam loginusers.vdf. Is the file corrupt?".to_string()));
	}

	let mut steam_user: HashMap<&str, String> = HashMap::new();
	let steam_loginusers = steam_loginusers.unwrap();
	for (other_steam_id_64, other_steam_user) in steam_loginusers.value.unwrap_obj().iter() {
		let other_steam_user = other_steam_user[0].clone().unwrap_obj().into_inner();

		let mostrecent = other_steam_user.get("MostRecent");
		let mostrecent = if mostrecent.is_some() { mostrecent.unwrap()[0].get_str().unwrap() } else { "0" };
		let timestamp = other_steam_user.get("Timestamp");
		let timestamp = if timestamp.is_some() { timestamp.unwrap()[0].get_str().unwrap().parse::<i32>().unwrap() } else { 0 };

		if steam_user.get("Timestamp").is_none() || (mostrecent == "1") || (timestamp > steam_user.get("Timestamp").unwrap().parse::<i32>().unwrap()) {
			steam_user.insert("SteamID64", other_steam_id_64.to_string());
			steam_user.insert("Timestamp", timestamp.to_string());

			let accountname = other_steam_user.get("AccountName");
			steam_user.insert("AccountName", accountname.unwrap()[0].get_str().unwrap().to_string());

			let personaname = other_steam_user.get("PersonaName");
			steam_user.insert("PersonaName", personaname.unwrap()[0].get_str().unwrap().to_string());
		}
	}

	if steam_user.get("Timestamp").is_none() {
		return Err(AlmightyError::Generic("Couldn't find Steam User. Have you ever launched/signed in to Steam?".to_string()));
	}

	let steam_id = SteamId::new(steam_user.get("SteamID64").unwrap().parse::<u64>().unwrap()).unwrap();

	terminal_write(writer, format!("Steam User: {} ({} / {})\n", steam_user.get("PersonaName").unwrap(), steam_user.get("SteamID64").unwrap(), steam_id.steam3id()).as_str(), true, None);

	// Get Steam Libraries
	let steam_libraryfolders_path = extend_pathbuf_and_return(steam_path.clone(), &["steamapps", "libraryfolders.vdf"]);
	let steam_libraryfolders_str = read_to_string(steam_libraryfolders_path);

	if steam_libraryfolders_str.is_err() {
		return Err(AlmightyError::Generic("Couldn't find Steam libraryfolders.vdf. Have you ever launched/signed in to Steam?".to_string()));
	}

	let steam_libraryfolders_str = steam_libraryfolders_str.unwrap();
	let steam_libraryfolders = Vdf::parse(steam_libraryfolders_str.as_str());

	if steam_libraryfolders.is_err() {
		return Err(AlmightyError::Generic("Couldn't parse Steam libraryfolders.vdf. Is the file corrupt?".to_string()));
	}

	// Get GMod Steam ibrary
	let mut gmod_steam_library_path = None;
	let steam_libraryfolders = steam_libraryfolders.unwrap();
	for (_, steam_library) in steam_libraryfolders.value.unwrap_obj().iter() {
		let steam_library = steam_library[0].clone().unwrap_obj().into_inner();
		let steam_library_apps = steam_library.get("apps").unwrap()[0].clone().unwrap_obj().into_inner();

		if steam_library_apps.get(GMOD_STEAM_APPID).is_some() {
			let steam_library_path = steam_library.get("path").unwrap()[0].clone();
			gmod_steam_library_path = string_to_canonical_pathbuf(steam_library_path.unwrap_str().to_string());
		}
	}

	if gmod_steam_library_path.is_none() {
		return Err(AlmightyError::Generic("Couldn't find Garry's Mod app registration. Is Garry's Mod installed?".to_string()));
	}

	let gmod_steam_library_path = gmod_steam_library_path.unwrap();
	let gmod_steam_library_path_str = gmod_steam_library_path.to_string_lossy();

	terminal_write(writer, format!("GMod Steam Library: {gmod_steam_library_path_str}\n").as_str(), true, None);

	// Get GMod manifest
	let gmod_manifest_path = extend_pathbuf_and_return(gmod_steam_library_path.to_path_buf(), &["steamapps", "appmanifest_4000.acf"]);
	let gmod_manifest_str = read_to_string(gmod_manifest_path);

	if gmod_manifest_str.is_err() {
		return Err(AlmightyError::Generic("Couldn't find GMod's appmanifest_4000.acf. Is Garry's Mod installed?".to_string()));
	}

	let gmod_manifest_str = gmod_manifest_str.unwrap();
	let gmod_manifest = Vdf::parse(gmod_manifest_str.as_str());

	if gmod_manifest.is_err() {
		return Err(AlmightyError::Generic("Couldn't parse GMod's appmanifest_4000.acf. Is the file corrupt?".to_string()));
	}

	let gmod_manifest = gmod_manifest.unwrap();
	let gmod_manifest = gmod_manifest.value.unwrap_obj();

	// Get GMod app state
	let gmod_stateflags = gmod_manifest.get("StateFlags").unwrap()[0].clone().unwrap_str();
	let gmod_downloadtype = gmod_manifest.get("DownloadType").unwrap()[0].clone().unwrap_str();
	let gmod_scheduledautoupdate = gmod_manifest.get("ScheduledAutoUpdate").unwrap()[0].clone().unwrap_str();
	// TODO(?): FullValidateBeforeNextUpdate / FullValidateAfterNextUpdate
	if gmod_stateflags != "4" || gmod_downloadtype != "2" || gmod_scheduledautoupdate != "0" {
		return Err(AlmightyError::Generic("Garry's Mod isn't Ready. Check Steam > Downloads and make sure it is fully installed and up to date. If that doesn't work, try launching the game, closing it, then running the tool again.".to_string()));
	}

	terminal_write(writer, format!("GMod App State: {gmod_stateflags} / {gmod_scheduledautoupdate}\n").as_str(), true, None);

	// Get GMod branch
	// TODO: Change branch if not x86-64
	let gmod_userconfig = gmod_manifest.get("UserConfig").unwrap()[0].clone().unwrap_obj();
	let gmod_branch = gmod_userconfig.get("BetaKey");
	let gmod_branch = if gmod_branch.is_some() { gmod_branch.unwrap()[0].clone().unwrap_str().to_string() } else { "public".to_string() };

	terminal_write(writer, format!("GMod Beta Branch: {gmod_branch}\n").as_str(), true, None);

	// Get GMod path
	// TODO: What about case-sensitive filesystems where it's named SteamApps or something
	// TODO: What about `steamapps/<username>/GarrysMod`? Is that still a thing, or did SteamPipe kill/migrate it completely?
	let gmod_path = gmod_manifest.get("installdir").unwrap()[0].clone().unwrap_str();
	let gmod_path = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(gmod_steam_library_path.clone(), &["steamapps", "common", &gmod_path]), true);

	if gmod_path.is_err() {
		return Err(AlmightyError::Generic("Couldn't find Garry's Mod directory. Is Garry's Mod installed?".to_string()));
	}

	let gmod_path = gmod_path.unwrap();
	let gmod_path_str = gmod_path.to_string_lossy();

	terminal_write(writer, format!("GMod Path: {gmod_path_str}\n").as_str(), true, None);

	// Abort if they're running as root AND the GMod directory isn't owned by root
	// Will hopefully prevent broken installs/updating
	#[cfg(unix)]
	if root {
		let gmod_dir_meta = std::fs::metadata(&gmod_path);
		if gmod_dir_meta.is_ok() {
			let gmod_dir_meta = gmod_dir_meta.unwrap();
			if gmod_dir_meta.uid() != 0 {
				return Err(AlmightyError::Generic("You are running GModPatchTool as root, but the Garry's Mod directory isn't owned by root. Either fix your permissions or don't run as root! Aborting...".to_string()));
			}
		}
	}

	// Determine target platform
	// Get GMod CompatTool config (Steam Linux Runtime, Proton, etc) on Linux
	// NOTE: platform_masked is specifically for Proton
	let platform = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "macos" } else { "linux" };
	let mut platform_masked = platform;
	let mut gmod_compattool = "none".to_string();

	#[cfg(not(any(windows, target_os = "macos")))]
	{
		// Get Steam config
		let steam_config_path = extend_pathbuf_and_return(steam_path.clone(), &["config", "config.vdf"]);
		let steam_config_str = read_to_string(steam_config_path);

		if steam_config_str.is_err() {
			return Err(AlmightyError::Generic("Couldn't find Steam config.vdf. Have you ever launched/signed in to Steam?".to_string()));
		}

		let steam_config_str = steam_config_str.unwrap();
		let steam_config = Vdf::parse(steam_config_str.as_str());

		if steam_config.is_err() {
			return Err(AlmightyError::Generic("Couldn't parse Steam config.vdf. Is the file corrupt?".to_string()));
		}

		let steam_config = steam_config.unwrap();
		let steam_config = steam_config.value.unwrap_obj();
		let steam_config = steam_config.get("Software").unwrap()[0].clone().unwrap_obj()
				.get("Valve").unwrap()[0].clone().unwrap_obj()
				.get("Steam").unwrap()[0].clone().unwrap_obj();
		let steam_config_compattoolmapping = steam_config.get("CompatToolMapping");

		if steam_config_compattoolmapping.is_some() {
			let steam_config_compattoolmapping = steam_config_compattoolmapping.clone().unwrap()[0].clone().unwrap_obj();
			if steam_config_compattoolmapping.contains_key(GMOD_STEAM_APPID) {
				let compattool = steam_config_compattoolmapping.get(GMOD_STEAM_APPID).unwrap()[0].clone().unwrap_obj().get("name").unwrap()[0].clone().unwrap_str().to_lowercase();

				if compattool.contains("proton") {
					platform_masked = "windows";
				}

				gmod_compattool = compattool;
			}
		}
	}

	terminal_write(writer, format!("Target Platform: {platform_masked} ({gmod_compattool})\n").as_str(), true, None);

	// Warn if -nochromium is in launch options
	// Some GMod "menu error fix" guides include it + gmod-lua-menu
	let steam_user_localconfig_path = extend_pathbuf_and_return(steam_path.clone(), &["userdata", steam_id.account_id().into_u32().to_string().as_str(), "config", "localconfig.vdf"]);
	let steam_user_localconfig_str = read_to_string(steam_user_localconfig_path);

	if steam_user_localconfig_str.is_err() {
		return Err(AlmightyError::Generic("Couldn't find Steam localconfig.vdf. Have you ever launched/signed in to Steam?".to_string()));
	}

	let steam_user_localconfig_str = steam_user_localconfig_str.unwrap();
	let steam_user_localconfig = Vdf::parse(steam_user_localconfig_str.as_str());

	if steam_user_localconfig.is_err() {
		return Err(AlmightyError::Generic("Couldn't parse Steam localconfig.vdf. Is the file corrupt?".to_string()));
	}

	let steam_user_localconfig = steam_user_localconfig.unwrap();
	let steam_user_localconfig = steam_user_localconfig.value.unwrap_obj();
	let steam_user_localconfig_apps = steam_user_localconfig.get("Software").unwrap()[0].clone().unwrap_obj()
		.get("Valve").unwrap()[0].clone().unwrap_obj()
		.get("Steam").unwrap()[0].clone().unwrap_obj()
		.get("apps").unwrap()[0].clone().unwrap_obj();
	let steam_user_localconfig_gmod = steam_user_localconfig_apps.get(GMOD_STEAM_APPID);

	if steam_user_localconfig_gmod.is_some() {
		let steam_user_localconfig_gmod = steam_user_localconfig_gmod.unwrap()[0].clone().unwrap_obj();
		let steam_user_localconfig_gmod_launchopts = steam_user_localconfig_gmod.get("LaunchOptions");

		if steam_user_localconfig_gmod_launchopts.is_some() {
			let steam_user_localconfig_gmod_launchopts = steam_user_localconfig_gmod_launchopts.unwrap()[0].clone().unwrap_str();
			if steam_user_localconfig_gmod_launchopts.contains("-nochromium") {
				terminal_write(writer, "WARNING: -nochromium is in GMod's Launch Options! CEF will not work with this.\n\tPlease go to Steam > Garry's Mod > Properties > General and remove it.\n\tAdditionally, if you have gmod-lua-menu installed, uninstall it.", true, if writer_is_interactive { Some("yellow") } else { None });

				let mut secs_to_continue: u8 = 5;
				while secs_to_continue > 0 {
					terminal_write(writer, format!("\tContinuing in {secs_to_continue} second(s)...\r").as_str(), false, if writer_is_interactive { Some("yellow") } else { None });
					writer().flush().unwrap();
					tokio::time::sleep(time::Duration::from_secs(1)).await;
					secs_to_continue -= 1;
				}

				// Clear continuing line
				if writer_is_interactive {
					terminal_write(writer, "\x1B[0K\n", false, None);
				}
			}
		}
	} else {
		return Err(AlmightyError::Generic("Couldn't find Garry's Mod in user localconfig.vdf. Is Garry's Mod installed?".to_string()));
	}

	// Get remote manifest
	terminal_write(writer, "Getting remote manifest...", true, None);

	let remote_manifest_response = get_http_response(writer, writer_is_interactive, &MANIFEST_SERVER_ROOTS, "manifest.json").await;

	if remote_manifest_response.is_none() {
		terminal_write(writer, "", true, None); // Newline
		return Err(AlmightyError::Generic("Couldn't get remote manifest. Please check your internet connection!".to_string()));
	}

	let remote_manifest_response = remote_manifest_response.unwrap();
	let remote_manifest = remote_manifest_response.json::<HashMap<String, HashMap<String, HashMap<String, HashMap<String, String>>>>>()
	.await?;

	terminal_write(writer, "GModPatchTool Manifest Loaded!\n", true, None);

	let platform_branches = remote_manifest.get(platform_masked);
	if platform_branches.is_none() {
		return Err(AlmightyError::Generic(format!("This operating system ({platform_masked}) is not supported!")));
	}

	let platform_branch_files = platform_branches.unwrap().get(&gmod_branch);
	if platform_branch_files.is_none() {
		return Err(AlmightyError::Generic(format!("This Beta Branch of Garry's Mod ({gmod_branch}) is not supported! Please go to Steam > Garry's Mod > Properties > Betas, select the x86-64 beta, then try again.")));
	}

	let platform_branch_files = platform_branch_files.unwrap();

	// Determine file integrity status
	terminal_write(writer, "Determining file integrity status...", true, None);

	// TODO: phf_map for these
	let integrity_status_strings = HashMap::from([
		(IntegrityStatus::NeedDelete, "Needs Delete"),
		(IntegrityStatus::NeedOriginal, "Needs Original + Fix"),
		(IntegrityStatus::NeedWipeFix, "Needs Wipe + Fix"),
		(IntegrityStatus::NeedFix, "Needs Fix"),
		(IntegrityStatus::Fixed, "Already Fixed")
	]);

	let integrity_results: Vec<(&String, Result<IntegrityStatus, String>, &HashMap<String, String>)> = platform_branch_files.par_iter()
	.map(|(filename, hashes)| {
		let integrity_result = determine_file_integrity_status(gmod_path.clone(), filename, hashes);
		let integrity_result_clone = integrity_result.clone();

		if integrity_result_clone.is_ok() {
			let integrity_status_string = integrity_status_strings[&integrity_result_clone.unwrap()];
			terminal_write(writer, format!("\t{filename}: {integrity_status_string}").as_str(), true, None);
		} else {
			let integrity_status_string = integrity_result_clone.unwrap_err();
			terminal_write(writer, format!("\t{filename}: {integrity_status_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
		}

		return (filename, integrity_result, hashes);
	}).collect();

	// Filter out fixed files, and if there were any i/o errors getting the hash, exit early
	let mut pending_files: Vec<(&String, IntegrityStatus, &HashMap<String, String>)> = vec![];
	for (filename, result, hashes) in integrity_results {
		if result.is_ok() {
			let result = result.unwrap();
			if result != IntegrityStatus::Fixed {
				pending_files.push((filename, result, hashes));
			}
		} else {
			return Err(AlmightyError::Generic("Failed to get integrity status of one or more files!".to_string()));
		}
	}

	let pending_files_len = pending_files.len();
	if pending_files_len > 0 {
		// Figure out where our cache should go based on OS
		let dirs_cache_dir = dirs::cache_dir();
		let os_cache_dir = if dirs_cache_dir.is_some() { dirs_cache_dir.unwrap() } else { std::env::temp_dir() };

		// Delete old GModCEFCodecFix cache directory
		#[cfg(windows)]
		let old_cache_dir = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(os_cache_dir.clone(), &["Temp", "GModCEFCodecFix"]), false);

		#[cfg(not(windows))]
		let old_cache_dir = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(os_cache_dir.clone(), &["GModCEFCodecFix"]), false);

		if old_cache_dir.is_ok() {
			let old_cache_dir_result = std::fs::remove_dir_all(old_cache_dir.unwrap());
			if old_cache_dir_result.is_ok() {
				terminal_write(writer,"Successfully removed old GModCEFCodecFix cache directory.", true, None);
			} else {
				let old_cache_dir_error_str = old_cache_dir_result.unwrap_err().to_string();
				terminal_write(writer, format!("Failed to remove old GModCEFCodecFix cache directory: {old_cache_dir_error_str}").as_str(), true, if writer_is_interactive { Some("yellow") } else { None });
			}
		}

		// Create new GModPatchTool cache directory if it doesn't exist
		let cache_path = extend_pathbuf_and_return(os_cache_dir, &["GModPatchTool"]);
		let mut cache_path_str = cache_path.to_string_lossy();
		let mut cache_dir = pathbuf_to_canonical_pathbuf(cache_path.clone(), false);
		if cache_dir.is_err() {
			let _ = std::fs::create_dir(cache_path.clone());
			cache_dir = pathbuf_to_canonical_pathbuf(cache_path.clone(), false);
		}

		// Can't access or create the cache directory!
		if cache_dir.is_err() {
			let cache_dir_err_str = cache_dir.unwrap_err().to_string();
			return Err(AlmightyError::Generic(format!("Failed to create cache directory ({cache_dir_err_str}):\n\t{cache_path_str}")));
		}

		let cache_dir = cache_dir.unwrap();
		cache_path_str = cache_dir.to_string_lossy();

		terminal_write(writer, format!("\nGModPatchTool Cache Directory: {cache_path_str}\n").as_str(), true, None);

		// Download what we need
		terminal_write(writer, "Downloading patch files...", true, None);

		let mut download_futures = JoinSet::new();
		for (filename, integrity_status, hashes) in &pending_files {
			// Need Original
			if *integrity_status == IntegrityStatus::NeedOriginal {
				download_futures.spawn(download_file_to_cache(writer, writer_is_interactive, cache_dir.clone(), format!("originals/{platform_masked}/{gmod_branch}/{filename}.zst"), hashes["original"].clone()));
			}

			// Need Fix (we filtered out IntegrityStatus::Fixed above, but we still need IntegrityStatus::NeedDelete for later)
			if *integrity_status != IntegrityStatus::NeedDelete {
				download_futures.spawn(download_file_to_cache(writer, writer_is_interactive, cache_dir.clone(), format!("patches/{platform_masked}/{gmod_branch}/{filename}.bsdiff"), hashes["patch"].clone()));
			}
		}

		while let Some(download_result) = download_futures.join_next().await {
			if download_result.is_err() {
				return Err(AlmightyError::Generic("Failed to download one or more patch files!".to_string()));
			}
		}

		// Patch the files
		terminal_write(writer, format!("\nPatching {pending_files_len} file(s)...").as_str(), true, None);

		// TODO: Early exit if any patches fail
		let patch_results: Vec<(&String, IntegrityStatus)> = pending_files.par_iter()
		.map(|(filename, integrity_status, hashes)| {
			terminal_write(writer, format!("\tPatching: {filename} ...").as_str(), true, None);

			let mut new_integrity_status: IntegrityStatus = integrity_status.clone();
			let integrity_status_string = integrity_status_strings[&new_integrity_status];
			let gmod_file_parts: Vec<&str> = filename.split("/").collect();
			let gmod_file_path = extend_pathbuf_and_return(gmod_path.clone(), &gmod_file_parts[..]);

			// Delete the file since it's not used anymore
			// If we can't delete it outright, try and truncate it
			// We could alternatively "patch" it into being empty...but that's a waste of CPU cycles, and if truncating doesn't work, that won't work either
			if new_integrity_status == IntegrityStatus::NeedDelete {
				let delete_result = std::fs::remove_file(&gmod_file_path);
				if delete_result.is_ok() {
					terminal_write(writer, format!("\tPatched: {filename}").as_str(), true, None);
					new_integrity_status = IntegrityStatus::Fixed;
				} else {
					let truncate_result = File::create(&gmod_file_path);
					if truncate_result.is_ok() {
						terminal_write(writer, format!("\tPatched: {filename}").as_str(), true, None);
						new_integrity_status = IntegrityStatus::Fixed;
					} else {
						let error_string = truncate_result.unwrap_err().to_string();
						terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string}: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
					}
				}
			}

			// Copy/overwrite the target gmod file with original copy we have
			if new_integrity_status == IntegrityStatus::NeedOriginal {
				let original_filename = format!("originals/{platform_masked}/{gmod_branch}/{filename}");
				let original_file_parts: Vec<&str> = original_filename.split("/").collect();
				let original_cache_file_path = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(cache_dir.clone(), &original_file_parts[..]), false);

				if original_cache_file_path.is_ok() {
					let original_cache_file_path = original_cache_file_path.unwrap();
					let copy_result = std::fs::copy(original_cache_file_path, &gmod_file_path);

					if copy_result.is_ok() {
						new_integrity_status = IntegrityStatus::NeedFix;
					} else {
						let error_string = copy_result.unwrap_err().to_string();
						terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string}: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
					}
				}
			}

			// Create/truncate original file (it doesn't exist without patches applied)
			if new_integrity_status == IntegrityStatus::NeedWipeFix {
				let gmod_file_path_dir = gmod_file_path.parent().unwrap().to_path_buf();
				let gmod_file_path_dir_path = pathbuf_to_canonical_pathbuf(gmod_file_path_dir.clone(), false);

				if gmod_file_path_dir_path.is_err() {
					let create_dir_result = std::fs::create_dir_all(gmod_file_path_dir);
					if create_dir_result.is_err() {
						let error_string = create_dir_result.unwrap_err().to_string();
						terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string}: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
					}
				}

				let create_result = File::create(&gmod_file_path);

				if create_result.is_ok() {
					new_integrity_status = IntegrityStatus::NeedFix;
				} else {
					let error_string = create_result.unwrap_err().to_string();
					terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string}: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
				}
			}

			// Patch the original file into the fixed one!
			if new_integrity_status == IntegrityStatus::NeedFix {
				let gmod_file_path = pathbuf_to_canonical_pathbuf(gmod_file_path, false);

				if gmod_file_path.is_ok() {
					let gmod_file_path = gmod_file_path.unwrap();
					let patch_filename = format!("patches/{platform_masked}/{gmod_branch}/{filename}.bsdiff");
					let patch_file_parts: Vec<&str> = patch_filename.split("/").collect();
					let patch_file_path = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(cache_dir.clone(), &patch_file_parts[..]), false);

					if patch_file_path.is_ok() {
						let patch_file_path = patch_file_path.unwrap();
						let gmod_file = std::fs::read(gmod_file_path.clone());

						if gmod_file.is_ok() {
							let gmod_file = gmod_file.unwrap();
							let patch_file = std::fs::read(patch_file_path);

							if patch_file.is_ok() {
								let patch_file = patch_file.unwrap();
								let patcher = Bspatch::new(&patch_file);

								if patcher.is_ok() {
									let patcher = patcher.unwrap();
									let mut new_gmod_file = Vec::with_capacity(patcher.hint_target_size() as usize);
									let patch_result = patcher.apply(&gmod_file, io::Cursor::new(&mut new_gmod_file));

									if patch_result.is_ok() {
										let write_result = std::fs::write(&gmod_file_path, &new_gmod_file);

										if write_result.is_ok() {
											// Sanity check the final checksum
											let file_hash_result = get_file_hash(&gmod_file_path);

											if file_hash_result.is_ok() {
												let file_hash = file_hash_result.unwrap();

												if file_hash == hashes["fixed"] {
													terminal_write(writer, format!("\tPatched: {filename}").as_str(), true, None);
													new_integrity_status = IntegrityStatus::Fixed;
												} else {
													terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 9: Checksum mismatch").as_str(), true, if writer_is_interactive { Some("red") } else { None });
												}
											} else {
												let error_string = file_hash_result.unwrap_err().to_string();
												terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 8: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
											}
										} else {
											let error_string = write_result.unwrap_err().to_string();
											terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 7: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
										}
									} else {
										let error_string = patch_result.unwrap_err().to_string();
										terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 6: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
									}
								} else if let Err(patcher) = patcher {
									let error_string = patcher.to_string();
									terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 5: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
								}
							} else {
								let error_string = patch_file.unwrap_err().to_string();
								terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 4: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
							}
						} else {
							let error_string = gmod_file.unwrap_err().to_string();
							terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 3: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
						}
					} else {
						let error_string = patch_file_path.unwrap_err().to_string();
						terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 2: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
					}
				} else {
					let error_string = gmod_file_path.unwrap_err().to_string();
					terminal_write(writer, format!("\tFailed to Patch: {filename} | {integrity_status_string} / Step 1: {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
				}
			}

			return (*filename, new_integrity_status)
		}).collect();

		for (_, integrity_status) in patch_results {
			if integrity_status != IntegrityStatus::Fixed {
				return Err(AlmightyError::Generic("Failed to patch one or more files!".to_string()));
			}
		}
	} else {
		terminal_write(writer, "No files need patching!", true, None);
	}

	// Make sure executables are executable on Linux and macOS
	// TODO: Windows support...but at the time of writing it's not well supported in Rust
	// This is done separately because we want it to apply to ALL files regardless of if they needed to be patched
	// https://github.com/solsticegamestudios/GModPatchTool/issues/161
	#[cfg(unix)]
	{
		terminal_write(writer, "\nApplying file permissions...", true, None);

		for (filename, fileinfo) in platform_branch_files {
			let executable = fileinfo.get("executable");

			if executable.is_some() {
				let executable = executable.unwrap();

				if executable == "true" {
					let gmod_file_parts: Vec<&str> = filename.split("/").collect();
					let gmod_file_path = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(gmod_path.clone(), &gmod_file_parts[..]), true);

					if gmod_file_path.is_ok() {
						let gmod_file_path = gmod_file_path.unwrap();
						let metadata = std::fs::metadata(&gmod_file_path);

						if metadata.is_ok() {
							// Ensure the executable bit is present and apply it to the file
							let mut perms = metadata.unwrap().permissions();
							perms.set_mode(perms.mode() | 0o111);
							let perms_result = std::fs::set_permissions(&gmod_file_path, perms);

							if perms_result.is_ok() {
								terminal_write(writer, format!("\t{filename}").as_str(), true, None);
							} else {
								let error_string = perms_result.unwrap_err().to_string();
								terminal_write(writer, format!("\tFailed to Apply Permissions: {filename} | {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
								// TODO: Fatal?
							}
						} else {
							let error_string = metadata.unwrap_err().to_string();
							terminal_write(writer, format!("\tFailed to Apply Permissions: {filename} | {error_string}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
							// TODO: Fatal?
						}
					}
				}
			}
		}
	}

	// TODO: Incorporate some of this: https://github.com/ret-0/gmod-linux-patcher/blob/master/gmod-linux-patcher.sh
	// TODO: Consider this: https://www.protondb.com/app/4000#vZBBKPhbFd
	// TODO: Bass updates?

	let now = now.elapsed().as_secs_f64();
	terminal_write(writer, format!("\nGModPatchTool applied successfully! Took {now} second(s).\nYou can now launch Garry's Mod in Steam.\n").as_str(), true, if writer_is_interactive { Some("green") } else { None });

	// TODO: AppInfo launch options for auto-starting GMod? What if we just relied on steam://rungameid/4000 instead?

	Ok(())
}

pub fn main_script<W>(writer: fn() -> W, writer_is_interactive: bool) -> JoinHandle<()>
where
	W: std::io::Write + 'static
{
	// HACK: Default is typically 2 MiB, but Vdf parsing can sometimes overflow the stack...
	// TODO: Report localconfig.vdf/config.vdf overflow (maybe related to Issue #54?): https://github.com/CosmicHorrorDev/vdf-rs/issues
	let builder = thread::Builder::new().stack_size(8388608); // 8 MiB

	// This is a separate thread because the GUI (if it exists) blocks the main thread
	builder.spawn(move || {
		terminal_write(writer, ABOUT, true, if writer_is_interactive { Some("cyan") } else { None });

		// Parse the args (will also exit if something's wrong with them)
		let args = Args::parse();

		if args.auto_mode.is_none() && !writer_is_interactive {
			error!("Interactive tty is required when not using --auto-mode");
			return
		}

		// Enable async on this thread
		tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(
			main_script_internal(writer, writer_is_interactive, args)
		).unwrap_or_else(|err| {
			error!("{}\n", err);
		});
	}).unwrap()
}

fn terminal_exit_handler() {
	println!("Press Enter to exit...");
	std::io::stdin().read_line(&mut String::new()).unwrap();
}

pub fn init_logger<W>(ansi: bool, writer: fn() -> W)
where
	W: std::io::Write + 'static
{
	tracing_subscriber::fmt()
	.with_env_filter(EnvFilter::from_default_env())
	.with_ansi(ansi)
	.without_time()
	.with_target(false)
	.with_writer(writer)
	.init();
}

pub fn main() {
	// Try to launch GUI -> Fallback to Terminal -> Fallback to File
	gui::main().unwrap_or_else(|error| {
		let stdout = std::io::stdout;
		let stdout_is_terminal = stdout().is_terminal();
		init_logger(stdout_is_terminal, stdout);

		error!("GUI | {}\n", error);

		if stdout_is_terminal {
			// Fallback to Terminal output if a Terminal is available

			// Set terminal title
			// NOTE: Doesn't work on legacy terminals like Windows <=10 Command Prompt
			print!("\x1B]0;GModPatchTool\x07");

			main_script(stdout, true).join().unwrap();

			terminal_exit_handler();
		} else {
			// Fallback to writing a file if there's no Terminal
			// TODO: Write to file
			main_script(stdout, false).join().unwrap();
		}
	});
}
