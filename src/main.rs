const ABOUT: &str = "GModPatchTool

Formerly: GModCEFCodecFix

Copyright 2020-2025, Solstice Game Studios (www.solsticegamestudios.com)
LICENSE: GNU General Public License v3.0

Purpose: Patches Garry's Mod to resolve common launch/performance issues, Update Chromium Embedded Framework (CEF), and Enable proprietary codecs in CEF.

Guide: https://www.solsticegamestudios.com/fixmedia/
FAQ/Common Issues: https://www.solsticegamestudios.com/fixmedia/faq/
Discord: https://www.solsticegamestudios.com/discord/
Email: contact@solsticegamestudios.com\n";

const GMOD_STEAM_APPID: &str = "4000";

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

fn path_exists_and_dir_not_empty(path: &Path) -> bool {
	let path_dir = path.read_dir().ok();
	return if path.exists() && path_dir.is_some() && path_dir.unwrap().next().is_some() { true } else { false }
}

fn extend_pathbuf_and_return(mut pathbuf: PathBuf, segments: &[&str]) -> PathBuf {
	pathbuf.extend(segments);
	return pathbuf;
}

async fn main_script_internal<W>(writer: fn() -> W, writer_is_interactive: bool, args: Args) -> Result<(), AlmightyError>
where
	W: std::io::Write + 'static
{
	// Get local version
	let local_version: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();

	// Get remote version
	terminal_write(writer, "Getting remote version...", true, None);

	let remote_version_response = reqwest::get("https://raw.githubusercontent.com/solsticegamestudios/GModPatchTool/refs/heads/master/version.txt")
	.await?;

	let remote_version_status_code = remote_version_response.status().as_u16();
	if remote_version_status_code != 200 {
		return Err(AlmightyError::Generic(format!("Bad HTTP Status Code: {remote_version_status_code}")));
	}

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
			terminal_write(writer, "\x1B[0K\n", false, if writer_is_interactive { Some("yellow") } else { None });
		}
	}

	// TODO: Warn about running as root
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
			terminal_write(writer, "\x1B[0K\n", false, if writer_is_interactive { Some("yellow") } else { None });
		}
	}

	// Abort if GMod is currently running
	let sys = System::new_all();

	// Windows
	for _ in sys.processes_by_exact_name("gmod.exe".as_ref()) {
		return Err(AlmightyError::Generic(format!("Garry's Mod is currently running. Please close it before running this tool.")));
	}

	// Linux / macOS
	for _ in sys.processes_by_exact_name("gmod".as_ref()) {
		return Err(AlmightyError::Generic(format!("Garry's Mod is currently running. Please close it before running this tool.")));
	}

	// Find Steam
	let mut steam_path = None;
	if args.steam_path.is_some() {
		// Make sure the path the user is forcing actually exists
		let steam_path_arg = args.steam_path.unwrap();
		let steam_path_arg_path = Path::new(&steam_path_arg);

		if path_exists_and_dir_not_empty(steam_path_arg_path) {
			steam_path = Some(steam_path_arg);
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
					steam_path = Some(steam_reg_path.unwrap());
				}
			}
		}
		// macOS
		#[cfg(target_os = "macos")]
		{
			// $HOME/Library/Application Support/Steam
			let mut steam_data_path = dirs::data_dir().unwrap();
			steam_data_path.push("Steam");

			steam_path = Some(steam_data_path.to_str().unwrap().to_string());
		}

		// Anything else (we assume *nix)
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
				extend_pathbuf_and_return(home_dir.clone(), &[".steam"]),
			];
			let mut valid_steam_paths = vec![];

			for pathbuf in possible_steam_paths {
				if path_exists_and_dir_not_empty(&*pathbuf) {
					valid_steam_paths.push((*pathbuf).to_str().unwrap().to_string());
				}
			}

			// $XDG_DATA_HOME/Steam
			let steam_xdg_path = dirs::data_dir();
			if steam_xdg_path.is_some() {
				let steam_xdg_path = extend_pathbuf_and_return(steam_xdg_path.unwrap(), &["Steam"]);

				if path_exists_and_dir_not_empty(&*steam_xdg_path) {
					valid_steam_paths.push((*steam_xdg_path).to_str().unwrap().to_string());
				}
			}

			// Set the Steam path if at least one is valid
			// Warn if there's more than one
			if valid_steam_paths.len() >= 1 {
				if valid_steam_paths.len() > 1 {
					let valid_steam_paths_str: String = "\n\t- ".to_owned() + &valid_steam_paths.join("\n\t- ");

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
						terminal_write(writer, "\x1B[0K\n", false, if writer_is_interactive { Some("yellow") } else { None });
					}
				}

				steam_path = Some(valid_steam_paths[0].clone());
			}
		}
	}

	if steam_path.is_none() {
		return Err(AlmightyError::Generic("Couldn't find Steam. If it's installed, try using the --steam_path argument to force a specific path.".to_string()));
	}

	let steam_path_str = &steam_path.clone().unwrap();
	let steam_path = Path::new(steam_path_str);

	terminal_write(writer, format!("Steam Path: {steam_path_str}\n").as_str(), true, None);

	// Get most recent Steam User, which is probably the one they're using/want
	let steam_loginusers_path = extend_pathbuf_and_return(steam_path.to_path_buf(), &["config", "loginusers.vdf"]);
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
	steam_user.insert("SteamID3", steam_id.steam3id());

	terminal_write(writer, format!("Steam User: {} ({} / {})\n", steam_user.get("PersonaName").unwrap(), steam_user.get("SteamID64").unwrap(), steam_user.get("SteamID3").unwrap()).as_str(), true, None);

	// Get Steam Libraries
	let steam_libraryfolders_path = extend_pathbuf_and_return(steam_path.to_path_buf(), &["steamapps", "libraryfolders.vdf"]);
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
			let steam_library_path = steam_library_path.unwrap_str();

			if Path::new(steam_library_path.as_str()).exists() {
				gmod_steam_library_path = Some(steam_library_path);
			}
		}
	}

	if gmod_steam_library_path.is_none() {
		return Err(AlmightyError::Generic("Couldn't find Garry's Mod app registration. Is Garry's Mod installed?".to_string()));
	}

	let gmod_steam_library_path_str = &gmod_steam_library_path.clone().unwrap();
	let gmod_steam_library_path = Path::new(gmod_steam_library_path_str.as_str());

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
		return Err(AlmightyError::Generic("Garry's Mod isn't Ready. Please make sure it is fully installed and up to date (check Steam > Downloads for pending updates).".to_string()));
	}

	terminal_write(writer, format!("GMod App State: {gmod_stateflags} / {gmod_scheduledautoupdate}\n").as_str(), true, None);

	// Get GMod branch
	// TODO: Change branch if not x86-64
	let gmod_userconfig = gmod_manifest.get("UserConfig").unwrap()[0].clone().unwrap_obj();
	let gmod_branch = gmod_userconfig.get("BetaKey");
	let gmod_branch = if gmod_branch.is_some() { gmod_branch.unwrap()[0].clone().unwrap_str().to_string() } else { "public".to_string() };

	terminal_write(writer, format!("GMod Beta Branch: {gmod_branch}\n").as_str(), true, None);

	// Determine target platform
	// Get GMod CompatTool config (Steam Linux Runtime, Proton, etc) on Linux
	// NOTE: platform_masked is specifically for Proton
	let platform = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "macos" } else { "linux" };
	let mut platform_masked = platform;

	#[cfg(not(any(windows, target_os = "macos")))]
	{
		// Get Steam config
		let steam_config_path = extend_pathbuf_and_return(steam_path.to_path_buf(), &["config", "config.vdf"]);
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
				let gmod_compattool = steam_config_compattoolmapping.get(GMOD_STEAM_APPID).unwrap()[0].clone().unwrap_obj().get("name").unwrap()[0].clone().unwrap_str().to_lowercase();

				if gmod_compattool.contains("proton") {
					platform_masked = "windows";
				}
			}
		}
	}

	// TODO: The rest of the owl

	Ok(())
}

pub fn main_script<W>(writer: fn() -> W, writer_is_interactive: bool) -> JoinHandle<()>
where
	W: std::io::Write + 'static
{
	// HACK: Default is typically 2 MiB, but Vdf parsing can sometimes overflow the stack on Windows?
	// TODO: Report config.vdf overflow (maybe related to Issue #54?): https://github.com/CosmicHorrorDev/vdf-rs/issues
	let builder = thread::Builder::new().stack_size(4194304); // 4 MiB

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
