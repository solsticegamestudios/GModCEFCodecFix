const ABOUT: &str = "GModPatchTool

Formerly: GModCEFCodecFix

Copyright 2020-2025, Solstice Game Studios (www.solsticegamestudios.com)
LICENSE: GNU General Public License v3.0

Purpose: Patches Garry's Mod to resolve common launch/performance issues, Update Chromium Embedded Framework (CEF), and Enable proprietary codecs in CEF.

Guide: https://www.solsticegamestudios.com/fixmedia/
FAQ/Common Issues: https://www.solsticegamestudios.com/fixmedia/faq/
Discord: https://www.solsticegamestudios.com/discord/
Email: contact@solsticegamestudios.com\n";

// TODO: Change from master to files branch
const TXT_SERVER_ROOTS: [&str; 2] = [
	"https://raw.githubusercontent.com/solsticegamestudios/GModPatchTool/refs/heads/master/",
	"https://www.solsticegamestudios.com/gmodpatchtool/"
];

const PATCH_SERVER_ROOTS: [&str; 2] = [
	"https://media.githubusercontent.com/media/solsticegamestudios/GModPatchTool/refs/heads/files/",
	"https://www.solsticegamestudios.com/gmodpatchtool/"
];

const GMOD_STEAM_APPID: &str = "4000";
const BLANK_FILE_SHA256: &str = "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855";

mod gui;

use eframe::egui::TextBuffer;
use tokio;
use tracing::{debug, error, info, warn};
use tracing_subscriber::filter::EnvFilter;
use clap::Parser;
use std::io::IsTerminal;
use phf::phf_map;
use phf::Map;
use std::{thread, time};
use std::thread::JoinHandle;
use std::path::Path;
use std::path::PathBuf;
use std::fs::read_to_string;
use keyvalues_parser::Vdf;
use std::collections::HashMap;
use steamid::SteamId;
use sysinfo::System;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io;
use rayon::prelude::*;
use std::sync::mpsc;

#[derive(Parser)]
#[command(version)]
struct Args {
	/// Force a specific Garry's Mod launch entry
	#[arg(short, long, value_name = "LAUNCH_OPTION")]
	auto_mode: Option<u8>,

	/// Force a specific Steam install path (NOT a Steam library path)
	#[arg(short, long)]
	steam_path: Option<String>,
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

fn pathbuf_dir_not_empty(pathbuf: &PathBuf) -> bool {
	// If this is a valid file in the directory, the directory isn't empty
	if pathbuf.is_file() {
		return true;
	}

	let pathbuf_dir = pathbuf.read_dir();
	return if pathbuf_dir.is_ok() && pathbuf_dir.unwrap().next().is_some() { true } else { false };
}

fn pathbuf_to_canonical_pathbuf(pathbuf: PathBuf) -> Option<PathBuf> {
	let pathbuf_result = pathbuf.canonicalize();

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

fn string_to_canonical_pathbuf(path_str: String) -> Option<PathBuf> {
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

fn extend_pathbuf_and_return(mut pathbuf: PathBuf, segments: &[&str]) -> PathBuf {
	pathbuf.extend(segments);
	return pathbuf;
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum IntegrityStatus {
	NeedOriginal = 0,
	NeedWipeFix = 1,
	NeedFix = 2,
	Fixed = 3
}

fn determine_file_integrity_status(gmod_path: PathBuf, filename: &String, hashes: &HashMap<String, String>) -> Result<IntegrityStatus, String> {
	let file_parts: Vec<&str> = filename.split("/").collect();
	let file_path = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(gmod_path, &file_parts[..]));
	let mut file_hash = BLANK_FILE_SHA256.to_string();

	if file_path.is_some() {
		let file = File::open(&file_path.unwrap());
		if file.is_ok() {
			let mut file = file.unwrap();
			let mut hasher = Sha256::new();
			let copy_result = io::copy(&mut file, &mut hasher);
			if copy_result.is_ok() {
				file_hash = format!("{:X}", hasher.finalize());
			}
		} else {
			return Err(file.unwrap_err().to_string());
		}
	}

	if file_hash == hashes["fixed"] {
		return Ok(IntegrityStatus::Fixed);
	} else {
		// File needs to be fixed...
		if file_hash == hashes["original"] {
			// The file is the original, so we just to apply the patch
			return Ok(IntegrityStatus::NeedFix);
		} else if hashes["original"] == BLANK_FILE_SHA256 {
			// The original file was empty, so we need to wipe the file, then patch it
			return Ok(IntegrityStatus::NeedWipeFix);
		} else {
			// We don't recognize the hash, so we need to first replace the file with the original (which we'll download), then apply the patch to that file
			return Ok(IntegrityStatus::NeedOriginal);
		}
	}
}

async fn main_script_internal<W>(writer: fn() -> W, writer_is_interactive: bool, args: Args) -> Result<(), AlmightyError>
where
	W: std::io::Write + 'static
{
	// Get local version
	let local_version: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();

	// Get remote version
	terminal_write(writer, "Getting remote version...", true, None);

	let mut txt_server_id: u8 = 0;
	let mut remote_version_response = None;
	while (txt_server_id as usize) < TXT_SERVER_ROOTS.len() {
		let url = TXT_SERVER_ROOTS[txt_server_id as usize].to_string() + "version.txt";
		let remote_version_response_result = reqwest::get(url.clone()).await;

		if remote_version_response_result.is_ok() {
			remote_version_response = remote_version_response_result.ok();

			let remote_version_status_code = remote_version_response.as_ref().unwrap().status().as_u16();
			if remote_version_status_code == 200 {
				break;
			} else {
				terminal_write(writer, format!("\n{url}\n\tBad HTTP Status Code: {remote_version_status_code}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
				remote_version_response = None;
				txt_server_id += 1;
			}
		} else {
			let error = remote_version_response_result.unwrap_err().without_url();
			terminal_write(writer, format!("\n{url}\n\tHTTP Error: {error}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
			remote_version_response = None;
			txt_server_id += 1;
		}
	}

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

	// TODO: Warn about running as root
	// TODO: Force confirmation? Tell them how to abort (CTRL+C)?
	let root = false;
	if root {
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
	}

	// Abort if GMod is currently running
	let sys = System::new_all();

	// Windows
	for _ in sys.processes_by_exact_name("gmod.exe".as_ref()) {
		return Err(AlmightyError::Generic("Garry's Mod is currently running. Please close it before running this tool.".to_string()));
	}

	// Linux / macOS
	for _ in sys.processes_by_exact_name("gmod".as_ref()) {
		return Err(AlmightyError::Generic("Garry's Mod is currently running. Please close it before running this tool.".to_string()));
	}

	// Find Steam
	let mut steam_path = None;
	if args.steam_path.is_some() {
		// Make sure the path the user is forcing actually exists
		let steam_path_arg = args.steam_path.unwrap();
		let steam_path_arg_pathbuf = string_to_canonical_pathbuf(steam_path_arg.clone());

		if steam_path_arg_pathbuf.is_some() {
			steam_path = steam_path_arg_pathbuf;
		} else {
			return Err(AlmightyError::Generic(format!("Please check the --steam_path argument is pointing to a valid path:\n\t{steam_path_arg}")));
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
			steam_path = pathbuf_to_canonical_pathbuf(steam_data_path);
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
				let pathbuf = pathbuf_to_canonical_pathbuf(pathbuf);
				if pathbuf.is_some() {
					valid_steam_paths.push(pathbuf.unwrap());
				}
			}

			// $XDG_DATA_HOME/Steam
			let steam_xdg_path = dirs::data_dir();
			if steam_xdg_path.is_some() {
				let steam_xdg_pathbuf = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(steam_xdg_path.unwrap(), &["Steam"]));
				if steam_xdg_pathbuf.is_some() {
					valid_steam_paths.push(steam_xdg_pathbuf.unwrap());
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
	let gmod_scheduledautoupdate = gmod_manifest.get("ScheduledAutoUpdate").unwrap()[0].clone().unwrap_str();
	if gmod_stateflags != "4" || gmod_scheduledautoupdate != "0" {
		return Err(AlmightyError::Generic("Garry's Mod isn't Ready. Check Steam > Downloads and make sure it is fully installed and up to date.".to_string()));
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
	let gmod_path = pathbuf_to_canonical_pathbuf(extend_pathbuf_and_return(steam_path.clone(), &["steamapps", "common", &gmod_path]));

	if gmod_path.is_none() {
		return Err(AlmightyError::Generic("Couldn't find Garry's Mod directory. Is Garry's Mod installed?".to_string()));
	}

	let gmod_path = gmod_path.unwrap();
	let gmod_path_str = gmod_path.to_string_lossy();

	terminal_write(writer, format!("GMod Path: {gmod_path_str}\n").as_str(), true, None);

	// Abort if they're running as root AND the GMod directory isn't owned by root
	// Will hopefully prevent broken installs/updating
	#[cfg(not(any(windows, target_os = "macos")))]
	if root {
		let gmod_dir_meta = std::fs::metadata(gmod_path);
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
		let steam_user_localconfig_gmod = steam_user_localconfig_gmod.clone().unwrap()[0].clone().unwrap_obj();
		let steam_user_localconfig_gmod_launchopts = steam_user_localconfig_gmod.get("LaunchOptions");

		if steam_user_localconfig_gmod_launchopts.is_some() {
			let steam_user_localconfig_gmod_launchopts = steam_user_localconfig_gmod_launchopts.clone().unwrap()[0].clone().unwrap_str();
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

	let mut txt_server_id: u8 = 0;
	let mut remote_manifest_response = None;
	while (txt_server_id as usize) < TXT_SERVER_ROOTS.len() {
		let url = TXT_SERVER_ROOTS[txt_server_id as usize].to_string() + "manifest.json";
		let remote_manifest_response_result = reqwest::get(url.clone()).await;

		if remote_manifest_response_result.is_ok() {
			remote_manifest_response = remote_manifest_response_result.ok();

			let remote_version_status_code = remote_manifest_response.as_ref().unwrap().status().as_u16();
			if remote_version_status_code == 200 {
				break;
			} else {
				terminal_write(writer, format!("{url}\n\tBad HTTP Status Code: {remote_version_status_code}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
				remote_manifest_response = None;
				txt_server_id += 1;
			}
		} else {
			let error = remote_manifest_response_result.unwrap_err().without_url();
			terminal_write(writer, format!("{url}\n\tHTTP Error: {error}").as_str(), true, if writer_is_interactive { Some("red") } else { None });
			remote_manifest_response = None;
			txt_server_id += 1;
		}
	}

	if remote_manifest_response.is_none() {
		terminal_write(writer, "", true, None); // Newline
		return Err(AlmightyError::Generic("Couldn't get remote manifest. Please check your internet connection!".to_string()));
	}

	let remote_manifest_response = remote_manifest_response.unwrap();
	let remote_manifest = remote_manifest_response.json::<HashMap<String, HashMap<String, HashMap<String, HashMap<String, String>>>>>()
	.await?;

	terminal_write(writer, "GModPatchTool Manifest Loaded!\n", true, None);

	// HACK: REMOVE ME!!
	let platform_masked = "win32";
	let test_remote_manifest = r#"
	{
		"windows": {
			"x86-64": {
				"bin/chrome_elf.dll": {
					"original": "31CB72D373FE4B6D4B06F75442B983223016D1FD1550C799B5C9583567CE1A8E",
					"patch": "685E1F915724159D3ADB8F9091629BF6CF4C63D541733015E7AB77E7BC9A6383",
					"fixed": "0DDE88487A4CAD9FC606CA895A38DF362F69828545BF407B2081928EA8962B2A"
				}
			}
		},
		"macos": {
			"x86-64": {
				"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework": {
					"original": "5AAA46AF0469FEE56C3D3CA9AA08CC0B3D8D54C8C1BEA267D4E3A3ADAD8DB71C",
					"patch": "6B290562C9403BF5D8DA7D334F900FBC5CA44A9F373701F372009F36AA715DD2",
					"fixed": "032AEFFA9B7562E8D59069017684F172578BF9EB55808463F7CABAD5F3C4CD5E"
				}
			}
		},
		"linux": {
			"x86-64": {
				"bin/linux32/chromium/locales/zh-TW.pak": {
					"original": "0347DE149FA81F961D1658CFA332E315A231158A56F81247AE5FE7B930D8D81F",
					"patch": "F1AE892AADD4F5D27951078E04DA22657CF46C767297F1A456A6FAB160CA8AF7",
					"fixed": "C07FF3E19D202E37BA8E80A6FA3C71F30E4F4624BFFB1D65A004CD3AB31B4AB7"
				}
			}
		}
	}
	"#;
	// ENDHACK

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
		(IntegrityStatus::NeedOriginal, "Needs Original + Fix"),
		(IntegrityStatus::NeedWipeFix, "Needs Wipe + Fix"),
		(IntegrityStatus::NeedFix, "Needs Fix"),
		(IntegrityStatus::Fixed, "Already Fixed")
	]);

	let (tx, rx) = mpsc::channel();
	let integrity_results: Vec<(&String, Result<IntegrityStatus, String>)> = platform_branch_files.par_iter()
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

		tx.send(integrity_result.is_err()).unwrap();

		return (filename, integrity_result);
	}).collect();

	let integrity_fatal_error = rx.recv().unwrap();
	if integrity_fatal_error {
		return Err(AlmightyError::Generic(format!("Couldn't check integrity of one or more files! Please try again.")));
	}

	for (filename, result) in integrity_results {
		//println!("{filename}: {:#?}", result);
	}

	// TODO: Patch files, etc
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
	println!("Press Enter to continue...");
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

fn main() {
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
