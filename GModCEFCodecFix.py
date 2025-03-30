#!/usr/bin/env python3

# GModCEFCodecFix
#
# Copyright 2020-2024, Solstice Game Studios (www.solsticegamestudios.com)
# LICENSE: GNU General Public License v3.0
#
# Purpose: Automatically patches Garry's Mod's internal Chromium Embedded Framework to enable Proprietary Video/Audio codec support
#
# Contact:
#	Discord: https://www.solsticegamestudios.com/discord/
#	Email: contact@solsticegamestudios.com

# TODO: Check if GMod is currently running
# TODO: Enable HTTP/2 with httpx?
# TODO: Support patching in an updated version of BASS? Test results: https://discord.com/channels/104385214364536832/459880607120490496/1212241220844392519
# TODO: Make "READ THE FAQ FIRST" more obvious
# TODO: Switch to x86-64 beta automatically before patching

# NOTE: Update everytime we release!
VERSION = 20240926

import sys
import os
from subprocess import Popen

if sys.version_info.major != 3:
	sys.exit("ERROR: You're using a version of Python that's not supported. You must use Python 3.")

if sys.platform == "linux":
	import psutil
	import shutil

# Hold up, gotta check if it's running in a Terminal or not on Linux
# TODO: Auto Mode shouldn't require a TTY (still need to notify about errors!)
possibleTerminals = [
	"x-terminal-emulator",
	"gnome-terminal",
	"terminator",
	"xfce4-terminal",
	"konsole",
	"xterm",
	"urxvt",
	"rxvt",
	"termit",
	"Eterm",
	"aterm",
	"uxterm",
	"roxterm",
	"termite",
	"lxterminal",
	"mate-terminal",
	"terminology",
	"st",
	"qterminal",
	"lilyterm",
	"tilix",
	"terminix",
	"kitty",
	"guake",
	"tilda",
	"alacritty",
	"hyper",
	"foot",
	"kgx",
	"cosmic-term",
	"ptyxis"
]
termNotFoundError = "GModCEFCodecFix could not find a suitable Terminal Emulator!\n\tIf one is installed, Contact Us about this:\n- Discord: https://www.solsticegamestudios.com/chat.html\n- Email: contact@solsticegamestudios.com"

if sys.platform == "linux":
	if os.path.isfile("ERROR_TerminalNotFound.txt"):
		os.remove("ERROR_TerminalNotFound.txt")

	if not sys.__stdin__.isatty():
		print("ERROR: GModCEFCodecFix must run in a Terminal! Attempting to open it in one...")

		foundTerm = False
		for termEXE in possibleTerminals:
			if shutil.which(termEXE) != None:
				print("Found Terminal: " + termEXE + ", attempting to re-launch...")
				Popen([termEXE, "-e", *sys.argv], stdin=None, stdout=None, stderr=None, close_fds=True)
				foundTerm = True
				break

		if not foundTerm:
			with open("ERROR_TerminalNotFound.txt", "w") as termNotFoundFile:
				termNotFoundFile.write(termNotFoundError)

		sys.exit(not foundTerm and "Terminal not found! Writing error info to ERROR_TerminalNotFound.txt...")

# Set up At-Exit handler so it doesn't just close immediately when it's done
import atexit

launchSuccess = False
autoMode = False

@atexit.register
def exitHandler():
	if not launchSuccess or autoMode is False:
		input("Press Enter to continue...")

# Set the title so it's not just some boring path
if sys.platform == "win32":
	os.system("title GModCEFCodecFix")
else:
	print("\33]0;GModCEFCodecFix\a", end='', flush=True)

import urllib.request
import httpx
import colorama
from termcolor import colored
from time import sleep
from socket import gaierror

colorama.init()

# Spit out the Software Info
print(colored("GModCEFCodecFix\nCreated by: Solstice Game Studios\nHow To Guide/FAQ:\n\thttps://www.solsticegamestudios.com/fixmedia/\nContact Us:\n\tDiscord: https://www.solsticegamestudios.com/discord/\n\tEmail: contact@solsticegamestudios.com\n", "cyan"))

contactInfo = "\n\nIf you need help, look at the Guide/FAQ first:\n- https://www.solsticegamestudios.com/fixmedia/\n\nIf that doesn't work, contact us:\n- Discord: https://www.solsticegamestudios.com/discord/\n- Email: contact@solsticegamestudios.com\n"

# Get CEFCodecFix's version and compare it with the version we have on the website
remoteVersion = 0
systemProxies = urllib.request.getproxies()

if systemProxies:
	print("System Proxies:\n" + str(systemProxies) + "\n")

try:
	print("Getting remote version...")
	versionRequest = httpx.get("https://raw.githubusercontent.com/solsticegamestudios/GModCEFCodecFix/master/version.txt", follow_redirects=True, timeout=60)

	if versionRequest.status_code == 200:
		remoteVersion = int(versionRequest.text)

		if remoteVersion > VERSION:
			print(colored("WARNING: CEFCodecFix is out of date! Please get the latest version at\nhttps://github.com/solsticegamestudios/GModCEFCodecFix/releases", "red"))

			secsToContinue = 5
			while secsToContinue:
				print(colored("\tContinuing in " + str(secsToContinue) + " seconds...", "yellow"), end="\r")
				sleep(1)
				secsToContinue -= 1

			sys.stdout.write("\033[K\n")
		else:
			print(colored("You are running the latest version of CEFCodecFix [Local: " + str(VERSION) + " / Remote: " + str(remoteVersion) + "]!\n", "green"))
	else:
		sys.exit(colored("Error: Could not get CEFCodecFix remote version!\n\tStatus Code: " + str(versionRequest.status_code) + contactInfo, "red"))
except gaierror as e:
	sys.exit(colored("Error: Could not get CEFCodecFix remote version!\n\tLooks like you're having DNS Problems [Errno " + str(e.errno) + "].\n\tSee the 1.1.1.1 Setup instructions at https://1.1.1.1/dns/\n\tThey'll change your DNS Settings to something that'll probably work." + contactInfo, "red"))
except httpx.TimeoutException as e:
	sys.exit(colored("Error: Could not get CEFCodecFix remote version!\n\tThe request timed out." + contactInfo, "red"))
except Exception as e:
	sys.exit(colored("Error: Could not get CEFCodecFix remote version!\n\tException: " + str(e) + contactInfo, "red"))

# Let's start the show
import argparse
from time import perf_counter
import vdf
from requests.structures import CaseInsensitiveDict # TODO: Replace this so we don't need to require requests anymore
from steam.utils.appcache import parse_appinfo
from steamid import SteamID
from hashlib import sha256
from concurrent.futures import ThreadPoolExecutor
from urllib.parse import urlparse
from bsdiff4 import file_patch
from pathlib import Path
from tempfile import gettempdir

# Specific platform imports
if sys.platform == "win32":
	import winreg
if sys.platform == "linux":
	from xdg import XDG_DATA_HOME
	from xdg import XDG_CACHE_HOME

# Optional command line arguments
parser = argparse.ArgumentParser(prog="GModCEFCodecFix")
parser.add_argument("-a", required=False, type=int, metavar="LAUNCH_OPTION", help="Force a specific GMod launch option (auto mode)")
parser.add_argument("-steam_path", required=False, help="Force a specific Steam install path (NOT a Steam library path)")
args = parser.parse_args()

if args.a:
	autoMode = int(args.a)
	print(colored("AUTO MODE: Enabled - Option " + str(autoMode) + "\n", "cyan"))

timeStart = perf_counter()

# Get Home Dir (used for finding Steam if necessary)
homeDir = str(Path.home())

# Find Steam
steamPath = args.steam_path
steamPathHints = {}

if steamPath:
	# Make sure the path they're forcing actually exists
	if not os.path.isdir(steamPath):
		sys.exit(colored("Error: Forced Steam Path Does Not Exist!\nPlease check the -steam_path argument is pointing to a valid path:\n\t" + steamPath + contactInfo, "red"))
else:
	if sys.platform == "win32":
		# Windows
		try:
			reg = winreg.ConnectRegistry(None, winreg.HKEY_CURRENT_USER)
			steamKey = winreg.OpenKey(reg, "Software\\Valve\\Steam")
			steamPathValue = winreg.QueryValueEx(steamKey, "SteamPath")
			steamPath = steamPathValue[0].replace("/", "\\")
		except:
			# We wanna make sure it doesn't crash and burn while looking for the Registry Key, but we also wanna handle it below
			pass

		steamPathHints["win32"] = "Is it installed properly and been run at least once?"
	elif sys.platform == "darwin":
		# macOS
		if os.path.isdir(os.path.join(homeDir, "Library", "Application Support", "Steam")):
			steamPath = os.path.join(homeDir, "Library", "Application Support", "Steam")

		steamPathHints["darwin"] = "Is it installed somewhere other than " + os.path.join(homeDir, "Library", "Application Support", "Steam") + " ?"
	else:
		# Linux
		snapSteamPath = os.path.realpath(os.path.join(homeDir, "snap", "steam", "common", ".local", "share", "Steam"))
		flatpakSteamPath = os.path.realpath(os.path.join(homeDir, ".var", "app", "com.valvesoftware.Steam", ".local", "share", "Steam"))
		homeSteamPath = os.path.realpath(os.path.join(homeDir, ".steam", "steam"))
		xdgSteamPath = os.path.realpath(os.path.join(str(XDG_DATA_HOME), "Steam"))

		# Check for Snap/Flatpak early to prevent conflicts for users with SteamCMD installed
		linuxSteamPaths = []
		if os.path.isdir(snapSteamPath):
			linuxSteamPaths.append(snapSteamPath)

		if os.path.isdir(flatpakSteamPath) and flatpakSteamPath not in linuxSteamPaths:
			linuxSteamPaths.append(flatpakSteamPath)

		if os.path.isdir(homeSteamPath) and homeSteamPath not in linuxSteamPaths:
			linuxSteamPaths.append(homeSteamPath)

		if os.path.isdir(xdgSteamPath) and xdgSteamPath not in linuxSteamPaths:
			linuxSteamPaths.append(xdgSteamPath)

		linuxSteamPathsLen = len(linuxSteamPaths)
		if linuxSteamPathsLen > 1:
			listOfLinuxSteamPaths = ""
			for path in linuxSteamPaths:
				listOfLinuxSteamPaths += "\n\t- " + path

			print(colored("Warning: Multiple Steam Installations Detected! This may cause issues:" + listOfLinuxSteamPaths + "\n", "yellow"))

			secsToContinue = 5
			while secsToContinue:
				print(colored("\tContinuing in " + str(secsToContinue) + " seconds...", "yellow"), end="\r")
				sleep(1)
				secsToContinue -= 1
			sys.stdout.write("\033[K\n")

		steamPath = linuxSteamPaths[0] if linuxSteamPathsLen > 0 else None

		steamPathHints["linux"] = ("Is it installed somewhere other than the following paths?" +
			"\n\t- " + snapSteamPath +
			"\n\t- " + flatpakSteamPath +
			"\n\t- " + homeSteamPath +
			"\n\t- " + xdgSteamPath)

if steamPath:
	steamPath = os.path.normcase(os.path.realpath(steamPath))
	print("Steam Path:\n" + steamPath + "\n")
else:
	sys.exit(colored("Error: Steam Path Not Found!\n" + steamPathHints[sys.platform] + contactInfo, "red"))

# Find most recent Steam User, which is probably the one they're using/want
steamLoginUsersPath = os.path.join(steamPath, "config", "loginusers.vdf")
if not os.path.isfile(steamLoginUsersPath):
	sys.exit(colored("Error: Steam LoginUsers File Not Found! Is the Steam Path valid? Have you ever launched Steam?" + contactInfo, "red"))

steamUser = {"Timestamp": 0}
with open(steamLoginUsersPath, "r", encoding="UTF-8", errors="ignore") as steamLoginUsersFile:
	steamLoginUsers = vdf.load(steamLoginUsersFile, mapper=CaseInsensitiveDict)
	steamLoginUsers = steamLoginUsers["users"]

	for userSteamID64 in steamLoginUsers:
		curSteamUser = steamLoginUsers[userSteamID64]

		if str(steamLoginUsers[userSteamID64]["mostrecent"]) == "1":
			steamUser = {"steamID64": userSteamID64, "AccountName": curSteamUser["AccountName"], "PersonaName": curSteamUser["PersonaName"], "Timestamp": int(curSteamUser["Timestamp"])}
			break
		elif int(steamLoginUsers[userSteamID64]["Timestamp"]) > steamUser["Timestamp"]:
			steamUser = {"steamID64": userSteamID64, "PersonaName": curSteamUser["PersonaName"], "Timestamp": int(curSteamUser["Timestamp"])}

if steamUser["Timestamp"] > 0:
	steamUser["steamID3"] = SteamID(steamUser["steamID64"]).steam3()
	print("Got Most Recent Steam User: " + steamUser["PersonaName"] + " (" + steamUser["steamID64"] + " / " + steamUser["steamID3"] + ")" + "\n")
else:
	sys.exit(colored("Error: Could not find Most Recent Steam User! Have you ever launched Steam?" + contactInfo, "red"))

# Find Steam Library Folders Config
steamLibraryFoldersConfigPath = os.path.join(steamPath, "steamapps", "libraryfolders.vdf")
if not os.path.isfile(steamLibraryFoldersConfigPath):
	sys.exit(colored("Error: Steam Library Folders Config File Not Found!" + contactInfo, "red"))

with open(steamLibraryFoldersConfigPath, "r", encoding="UTF-8", errors="ignore") as steamLibraryFoldersConfigFile:
	steamLibraryFoldersConfig = vdf.load(steamLibraryFoldersConfigFile, mapper=CaseInsensitiveDict)
	steamLibraryFoldersConfig = steamLibraryFoldersConfig["LibraryFolders"]

# Get Steam Libraries
steamLibraries = []
steamLibraries.append(steamPath) # Default

for configKey in steamLibraryFoldersConfig:
	try:
		int(configKey) # Try to convert it to an int as a test
		configVal = steamLibraryFoldersConfig[configKey]

		# Figure out if this is a string path or assume it's an array
		# Also don't allow duplicates
		configPath = configVal if isinstance(configVal, str) else configVal["path"]
		configPath = os.path.normcase(os.path.realpath(configPath))

		if configPath not in steamLibraries:
			steamLibraries.append(configPath)
	except (FileNotFoundError, ValueError):
		continue

if len(steamLibraries) == 0:
	sys.exit(colored("Error: No Steam Libraries Found!" + contactInfo, "red"))

print("Steam Libraries:")
print(steamLibraries)
print("") # Newline

# Find GMod Manifest
foundGModManifest = False
gmodManifestPath = ""
gmodManifestStr = ""
gmodSteamLibraryPath = None
possibleGModManifestPaths = [
	["steamapps", "appmanifest_4000.acf"]
]
for path in steamLibraries:
	for curGModManifestPath in possibleGModManifestPaths:
		curGModManifestPath = os.path.join(path, *curGModManifestPath)
		if os.path.isfile(curGModManifestPath) and os.path.getsize(curGModManifestPath) > 0:
			curGModManifestStr = ""
			with open(curGModManifestPath, "r", encoding="UTF-8", errors="ignore") as gmodManifestFile:
				curGModManifestStr = gmodManifestFile.read().strip().replace("\x00", "")
			if curGModManifestStr:
				if foundGModManifest:
					# Assume the GMod paths are where they're supposed to be
					install1 = "\n\tGMod Install #1:\n\t\t" + gmodManifestPath
					install1GModPath = os.path.join(gmodSteamLibraryPath, "steamapps", "common", "GarrysMod")
					if os.path.isdir(install1GModPath):
						install1 += "\n\t\t" + install1GModPath

					install2 = "\n\tGMod Install #2:\n\t\t" + curGModManifestPath
					install2GModPath = os.path.join(path, "steamapps", "common", "GarrysMod")
					if os.path.isdir(install2GModPath):
						install2 += "\n\t\t" + install2GModPath

					sys.exit(colored("Error: Multiple Garry's Mod Installations Detected!\nPlease manually remove the unused version:", "red") + colored(install1 + "\n" + install2, "yellow") + colored(contactInfo, "red"))
				else:
					foundGModManifest = True
					gmodManifestPath = curGModManifestPath
					gmodManifestStr = curGModManifestStr
					gmodSteamLibraryPath = path

if foundGModManifest:
	print("Found Garry's Mod Manifest:\n" + gmodManifestPath + "\n")
else:
	sys.exit(colored("Error: Could Not Find Valid Garry's Mod Manifest! Is Garry's Mod Installed?" + contactInfo, "red"))

# Find GMod
# TODO: Do something if their steamapps folder has non-lowercase capitalization on a case-sensitive filesystem
foundGMod = False
gmodPath = ""
possibleGModPaths = [
	["steamapps", "common", "GarrysMod"],
	["steamapps", steamUser["AccountName"], "GarrysMod"]
]
for curGModPath in possibleGModPaths:
	curGModPath = os.path.join(gmodSteamLibraryPath, *curGModPath)
	if os.path.isdir(curGModPath):
		if foundGMod:
			sys.exit(colored("Error: Multiple Garry's Mod Installations Detected!\nPlease manually remove the unused version(s):\n\t" + gmodPath + "\n\t" + curGModPath + contactInfo, "red"))
		else:
			foundGMod = True
			gmodPath = curGModPath

if foundGMod:
	print("Found Garry's Mod:\n" + gmodPath + "\n")
else:
	sys.exit(colored("Error: Could Not Find Garry's Mod!" + contactInfo, "red"))

# Get GMod Branch
gmodManifest = vdf.loads(gmodManifestStr, mapper=CaseInsensitiveDict)
gmodBranch = "betakey" in gmodManifest["AppState"]["UserConfig"] and gmodManifest["AppState"]["UserConfig"]["betakey"] or "main"

print("Garry's Mod Branch:\n" + gmodBranch + "\n")

# Make sure GMod is in a good state (fully installed, not updating)
gmodState = gmodManifest["AppState"]["StateFlags"]
if gmodState != "4" or gmodManifest["AppState"]["ScheduledAutoUpdate"] != "0":
	sys.exit(colored("Error: Garry's Mod isn't Ready. Please make sure it's fully installed, up to date (check Steam > Downloads), and not corrupt (Steam > Garry's Mod > Properties > Installed Files > Verify Integrity)." + contactInfo, "red"))

print("Garry's Mod State:\n" + gmodState + "\n")

# Get Steam Config for Proton
# NOTE: We have a need to lie about about what OS is running from here on out. Reference both sys.platform and sysPlatformProtonMasked
sysPlatformProtonMasked = sys.platform
if sys.platform == "linux":
	print("Getting Steam Config...")

	steamConfigPath = os.path.join(steamPath, "config", "config.vdf")
	if not os.path.isfile(steamConfigPath):
		sys.exit(colored("Error: Steam Config File Not Found!" + contactInfo, "red"))

	with open(steamConfigPath, "r", encoding="UTF-8", errors="ignore") as steamConfigPath:
		steamConfig = vdf.load(steamConfigPath, mapper=CaseInsensitiveDict)
		steamConfig = steamConfig["InstallConfigStore"]["Software"]["Valve"]["Steam"]

		if "CompatToolMapping" in steamConfig:
			steamCompatToolMapping = steamConfig["CompatToolMapping"]

			if "4000" in steamCompatToolMapping and "proton" in steamCompatToolMapping["4000"]["name"].lower():
				sysPlatformProtonMasked = "win32"

				print(colored("Warning: Using Proton with Garry's Mod is not recommended.\n\t- Please consider going to Steam > Garry's Mod > Properties > Compatibility and turning off Compatibility Tools to use the Native Linux build.\n\t- If you MUST use Proton, use Proton 8.0-4 or newer for best compatibility.\n", "yellow"))

				secsToContinue = 5
				while secsToContinue:
					print(colored("\tContinuing in " + str(secsToContinue) + " seconds...", "yellow"), end="\r")
					sleep(1)
					secsToContinue -= 1
				sys.stdout.write("\033[K\n")

# Get GMod's Steam AppInfo
osTypeMap = {
	"win32": "windows",
	"darwin": "macos",
	"linux": "linux"
}

print("Getting Steam AppInfo for GMod...")

steamAppInfoPath = os.path.join(steamPath, "appcache", "appinfo.vdf")
if not os.path.isfile(steamAppInfoPath):
	sys.exit(colored("Error: Steam AppInfo File Not Found!" + contactInfo, "red"))

# Get GMod Executable Paths
gmodEXELaunchOptions = []
with open(steamAppInfoPath, "rb") as steamAppInfoFile:
	_, steamAppInfo = parse_appinfo(steamAppInfoFile, mapper=CaseInsensitiveDict)

	gmodLaunchConfig = None
	for app in steamAppInfo:
		if app["appid"] == 4000:
			gmodLaunchConfig = app["data"]["appinfo"]["config"]["launch"]
			break

	print("\tPlatform: " + sys.platform)

	if sys.platform == "linux":
		print("\tIs Using Proton: " + ("Yes" if sysPlatformProtonMasked != sys.platform else "No"))
		# TODO
		#print("\tIs Using Steam Runtime: " + ("Yes" if sysPlatformProtonMasked != sys.platform else "No"))

	for option in gmodLaunchConfig:
		option = gmodLaunchConfig[option]

		if option["config"]["oslist"] == osTypeMap[sysPlatformProtonMasked] and ("betakey" not in option["config"] or option["config"]["betakey"] == gmodBranch):
			pathParts = [os.sep]
			pathParts.extend(gmodPath.replace("\\", "/").split("/"))
			pathParts.extend(option["executable"].replace("\\", "/").split("/"))
			pathParts.insert(2, os.sep)

			print("\t" + os.path.join(*pathParts))

			# os.path.isfile failed sometimes
			try:
				with open(os.path.join(*pathParts), "rb"):
					print("\t\tEXE Found")
					gmodEXELaunchOptions.append(option)
			except OSError as e:
				print("\t\t[Errno " + str(e.errno) + "] " + e.strerror)
			except Exception as e:
				print("\t\t" + str(e))

gmodEXELaunchOptionsLen = len(gmodEXELaunchOptions)
if gmodEXELaunchOptionsLen > 0:
	print("GMod EXE Launch Options Detected: " + str(gmodEXELaunchOptionsLen) + "\n")
else:
	sys.exit(colored("Error: Could not detect GMod EXE Launch Options!" + contactInfo, "red"))

# Get the User Launch Options for GMod
steamUserLocalConfigPath = os.path.join(steamPath, "userdata", steamUser["steamID3"].split(":")[2][:-1], "config", "localconfig.vdf")
if not os.path.isfile(steamUserLocalConfigPath):
	sys.exit(colored("Error: Steam User LocalConfig File Not Found!" + contactInfo, "red"))

gmodUserLaunchOptions = ""
with open(steamUserLocalConfigPath, "r", encoding="UTF-8", errors="ignore") as steamUserLocalConfigFile:
	steamUserLocalConfig = vdf.load(steamUserLocalConfigFile, mapper=CaseInsensitiveDict)
	steamUserLocalConfig = steamUserLocalConfig["UserLocalConfigStore"]["Software"]["Valve"]["Steam"]
	gmodLocalConfig = steamUserLocalConfig["Apps"]["4000"]
	if "LaunchOptions" in gmodLocalConfig:
		gmodUserLaunchOptions = " " + gmodLocalConfig["LaunchOptions"]

# Some stupid guides include this
if "-nochromium" in gmodUserLaunchOptions:
	print(colored("WARNING: -nochromium is in GMod's Launch Options! CEF will not work with this.\n\tPlease go to Steam > Garry's Mod > Properties > General and remove it.\n\tAdditionally, if you have gmod-lua-menu installed, please uninstall it.", "red"))

	secsToContinue = 5
	while secsToContinue:
		print(colored("\tContinuing in " + str(secsToContinue) + " seconds...", "yellow"), end="\r")
		sleep(1)
		secsToContinue -= 1

	sys.stdout.write("\033[K\n")

# Get CEFCodecFix Manifest
try:
	manifestRequest = httpx.get("https://raw.githubusercontent.com/solsticegamestudios/GModCEFCodecFix/master/manifest.json", follow_redirects=True, timeout=60)

	if manifestRequest.status_code != 200:
		sys.exit(colored("Error: CEFCodecFix Manifest Failed to Load! Status Code: " + str(manifestRequest.status_code) + contactInfo, "red"))
except Exception as e:
	sys.exit(colored("Error: CEFCodecFix Manifest Failed to Load! Exception: " + str(e) + contactInfo, "red"))

manifest = manifestRequest.json()

if not sys.platform in manifest:
	sys.exit(colored("Error: This Operating System is not supported by CEFCodecFix!" + contactInfo, "red"))

if not gmodBranch in manifest[sysPlatformProtonMasked]:
	sys.exit(colored("Error: This Branch of Garry's Mod is not supported! Please go to Steam > Garry's Mod > Properties > Betas, select the x86-64 beta, then try again!" + contactInfo, "red"))

# Check File Status
manifest = manifest[sysPlatformProtonMasked][gmodBranch]
print("CEFCodecFix Manifest Loaded!\nChecking Files to see what needs to be Fixed...")

def getFileSHA256(filePath):
	fileSHA256 = sha256()

	try:
		with open(filePath, "rb") as cefFile:
			while True:
				fileData = cefFile.read(10485760) # Read about 10MB at a time
				if not fileData:
					break
				fileSHA256.update(fileData)
	except Exception as e:
		# Probably some read/write issue
		return False, str(e)

	return True, fileSHA256.hexdigest().upper()

cacheFileFailed = "\nError: Cannot Access One or More Files in CEFCodecFix cache.\nPlease verify that CEFCodecFix has read/write permissions to the CEFCodecFixFiles directory (try running as admin)" + contactInfo
gmodFileFailed = "\nError: Cannot Access One or More Files in Garry's Mod Installation.\nPlease verify that Garry's Mod is closed, Steam is not updating it, and that CEFCodecFix has read/write permissions to its directory (try running as admin)" + contactInfo
blankFileSHA256 = "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"
filesToWipe = []
filesToUpdate = []
fileNoMatchOriginal = False
def determineFileIntegrityStatus(file):
	global fileNoMatchOriginal
	originalFilePath = os.path.join(gmodPath, file)
	originalFilePath = originalFilePath if os.path.isfile(originalFilePath) else ("NUL" if sys.platform == "win32" else "/dev/null")
	success, fileSHA256OrException = getFileSHA256(originalFilePath)

	if success:
		if fileSHA256OrException != manifest[file]["fixed"]:
			# File needs to be fixed
			if fileSHA256OrException == manifest[file]["original"]:
				# And it matches the original
				filesToUpdate.append(file)
				return True, "\t" + file + ": Needs Fix"
			elif manifest[file]["original"] == blankFileSHA256:
				# And it was empty originally, so we're gonna wipe it first
				filesToWipe.append(file)
				filesToUpdate.append(file)
				return True, "\t" + file + ": Needs Wipe + Fix"
			else:
				# And it doesn't match the original...
				fileNoMatchOriginal = True
				return True, "\t" + file + ": Does Not Match Original!"
		else:
			return True, "\t" + file + ": Already Fixed"
	else:
		return False, "\t" + file + ": " + fileSHA256OrException

with ThreadPoolExecutor() as executor:
	for fileIntegrityResultList in executor.map(determineFileIntegrityStatus, manifest):
		success, fileIntegrityResult = fileIntegrityResultList

		if success:
			print(fileIntegrityResult)
		else:
			# Probably some read/write issue
			print(colored(fileIntegrityResult, "yellow"))
			sys.exit(colored(gmodFileFailed, "red"))

# TODO: Download the original file WE have and overwrite, then patch into the Fixed file (solves the 1-3h timelag for patch updates, since we can't ship Fixed files wholesale)
# Something's wrong; bail before we break their installation or something
if fileNoMatchOriginal:
	sys.exit(colored("\nError: One or More Files failed to match the Original Checksum!\n\tPlease go to Steam > Garry's Mod > Properties > Installed Files, Verify Integrity, then try again!" + contactInfo, "red"))

if len(filesToUpdate) > 0:
	print("\nFixing Files...")

	if sys.platform == "win32":
		# Windows
		cacheDir = os.path.join(homeDir, "AppData", "Local", "Temp")
	elif sys.platform == "darwin":
		# macOS
		cacheDir = os.path.join(homeDir, "Library", "Caches")
	else:
		# Linux
		cacheDir = XDG_CACHE_HOME

	if not os.path.isdir(cacheDir):
		# Cache root doesn't exist, let tempfile give us one instead
		cacheDir = gettempdir()

	cacheDir = os.path.join(cacheDir, "GModCEFCodecFix")
	cacheExists = os.path.isdir(cacheDir)
	
	if not cacheExists:
		os.mkdir(cacheDir)

	for file in filesToUpdate:
		# Download needed patch files to local cache
		# TODO: Multithreading?
		cachedFileValid = False
		patchFilePath = os.path.normcase(os.path.realpath(os.path.join(cacheDir, file + ".bsdiff")))

		if cacheExists and os.path.isfile(patchFilePath):
			# Use cached patch files if available, but check the checksums first
			# We don't care about handling an exception here; we'll just overwrite the file
			success, fileSHA256OrException = getFileSHA256(patchFilePath)
			if success and fileSHA256OrException == manifest[file]["patch"]:
				cachedFileValid = True

		if not cachedFileValid:
			patchURL = manifest[file]["patch-url"]
			print("\tDownloading: " + patchURL + "...")

			# TODO: Retry up to 3 times in case of shenanigans
			patchURLRequest = httpx.get(patchURL, follow_redirects=True, timeout=None)

			if patchURLRequest.status_code != 200:
				sys.exit(colored("Error: Failed to Download " + file + " | HTTP " + str(patchURLRequest.status_code) + contactInfo, "red"))
			else:
				# Create needed directories if they don't exist already
				os.makedirs(os.path.dirname(patchFilePath), exist_ok = True)
				with open(patchFilePath, "wb") as newCEFPatch:
					newCEFPatch.write(patchURLRequest.content)

	for file in filesToUpdate:
		print("\tPatching: " + file + "...")

		originalFilePath = os.path.join(gmodPath, file)
		patchFilePath = os.path.normcase(os.path.realpath(os.path.join(cacheDir, file + ".bsdiff")))
		fixedFilePath = originalFilePath # The original file path might be different from the fixed file path

		# Wipe any original files that need wiping
		if file in filesToWipe:
			try:
				os.remove(originalFilePath)
			except Exception as e:
				# Probably some read/write issue
				print(colored("\tException (Original Wipe): " + str(e), "yellow"))
				sys.exit(colored(gmodFileFailed, "red"))

		if not os.path.isfile(originalFilePath):
			print("\t\tOriginal doesn't exist, setting to NULL")
			originalFilePath = "NUL" if sys.platform == "win32" else "/dev/null"
 
		# Try and open target files, creating them if they don't exist
		try:
			os.makedirs(os.path.dirname(fixedFilePath), exist_ok = True)
			open(fixedFilePath, "a+b").close()
		except Exception as e:
			print(colored("\tException (Fixed): " + str(e), "yellow"))
			sys.exit(colored(gmodFileFailed, "red"))

		if os.access(patchFilePath, os.R_OK):
			if not os.access(fixedFilePath, os.W_OK):
				sys.exit(colored(gmodFileFailed, "red"))
		else:
			sys.exit(colored(cacheFileFailed, "red"))

		try:
			file_patch(originalFilePath, fixedFilePath, patchFilePath)
		except Exception as e:
			# Probably some read/write issue
			print(colored("\tException: " + str(e), "yellow"))
			sys.exit(colored(gmodFileFailed, "red"))
else:
	print("\nNo Files Need Fixing!")

print(colored("\nCEFCodecFix applied successfully! Took " + str(round(perf_counter() - timeStart, 4)) + " second(s).", "green"))

if gmodEXELaunchOptionsLen == 1:
	gmodEXESelected = 0

	validShouldLaunch = False
	while validShouldLaunch == False:
		print("\nDo you want to Launch Garry's Mod now? (yes/no)")

		if autoMode is not False:
			print(">>> " + colored("AUTO MODE: yes", "cyan"))

		shouldLaunch = "yes" if autoMode is not False else input(">>> ")
		try:
			shouldLaunch = shouldLaunch.lower()
			if shouldLaunch == "yes" or shouldLaunch == "y":
				validShouldLaunch = True
				shouldLaunch = True
			elif shouldLaunch == "no" or shouldLaunch == "n":
				validShouldLaunch = True
				shouldLaunch = False
			else:
				print("That's not a valid option.")
				autoMode = False
		except ValueError:
			print("That's not a valid option.")
			autoMode = False

	if not shouldLaunch:
		sys.exit()

elif sys.platform == "win32":
	# TODO: Proper multi-EXE selection on Linux and macOS

	validGModEXESelection = False
	while validGModEXESelection == False:
		print("\nPlease enter the option number you want to launch Garry's Mod with (or CTRL+C to quit):")
		optionNum = 0
		for option in gmodEXELaunchOptions:
			print("\t" + str(optionNum) + " | " + option["description"])
			optionNum += 1

		if autoMode is not False:
			print(">>> " + colored("AUTO MODE: Selected Option " + str(autoMode), "cyan"))

		try:
			gmodEXESelected = autoMode if autoMode is not False else input(">>> ")
			try:
				gmodEXESelected = int(gmodEXESelected)
				if gmodEXESelected < gmodEXELaunchOptionsLen:
					validGModEXESelection = True
				else:
					print("That's not a valid option.")
					autoMode = False
			except ValueError:
				print("That's not a valid option.")
				autoMode = False
		except KeyboardInterrupt:
			sys.exit("CTRL+C\n")

print(colored("\nLaunching Garry's Mod:", "green"))

if sys.platform == "win32":
	gmodEXE = os.path.join(gmodPath, gmodEXELaunchOptions[gmodEXESelected]["executable"]) + " " + gmodEXELaunchOptions[gmodEXESelected]["arguments"]
	print(gmodEXE + gmodUserLaunchOptions + "\n")
	Popen(gmodEXE + gmodUserLaunchOptions, stdin=None, stdout=None, stderr=None, close_fds=True)
elif sys.platform == "darwin":
	print("open steam://rungameid/4000\n")
	Popen(["open", "steam://rungameid/4000"], stdin=None, stdout=None, stderr=None, close_fds=True)
else:
	linuxGModLaunchCommand = "xdg-open steam://rungameid/4000 >/dev/null 2>&1 &"
	print(linuxGModLaunchCommand + "\n")
	Popen(linuxGModLaunchCommand, shell=True, stdin=None, stdout=None, stderr=None, close_fds=True)

launchSuccess = True
