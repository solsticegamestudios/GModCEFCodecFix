# GModCEFCodecFix
#
# Copyright 2019, Solstice Game Studios (www.solsticegamestudios.com)
# LICENSE: GNU General Public License v3.0
#
# Purpose: Automatically patches Garry's Mod's internal Chromium Embedded Framework to enable Proprietary Video/Audio codec support
#
# Contact:
#	Discord: https://www.solsticegamestudios.com/chat.html
#	Email: contact@solsticegamestudios.com

# Set up At-Exit handler so it doesn't just close immediately when it's done
import atexit
import os

@atexit.register
def exitHandler():
	os.system("pause")

# Set the title so it's not just some boring path
os.system("title Garry's Mod: CEF Codec Fix")

import sys
import http.client
import colorama
from termcolor import colored

colorama.init()

# Spit out the Software Info
print(colored("GMod CEF Codec Fix\nCreated by: Solstice Game Studios\nContact Us:\n\tDiscord: https://www.solsticegamestudios.com/chat.html\n\tEmail: contact@solsticegamestudios.com\n", "cyan"))

# Get CEFCodecFix's version and compare it with the version we have on the website
localVersion = 0
remoteVersion = 0

with open(getattr(sys, "frozen", False) and sys._MEIPASS + "\\version.txt" or "version.txt", "r") as versionFile:
	localVersion = int(versionFile.read())

#print("Local Version: " + str(localVersion))

versionCon = http.client.HTTPSConnection("raw.githubusercontent.com")
versionCon.request("GET", "/solsticegamestudios/GModCEFCodecFix/master/version.txt")
versionResp = versionCon.getresponse()

if versionResp.status == 200:
	remoteVersion = int(versionResp.read())
	versionCon.close()

	#print("Remote Version: " + str(remoteVersion) + "\n")

	if remoteVersion > localVersion:
		print(colored("WARNING: CEFCodecFix is out of date! Please get the latest version at https://github.com/solsticegamestudios/GModCEFCodecFix/releases\n", "red"))
	else:
		print(colored("You are running the latest version of CEFCodecFix!\n", "green"))
else:
	print(colored("WARNING: Could not get CEFCodecFix remote version.\n", "yellow"))

# Let's start the show
from time import perf_counter
import winreg
import vdf
from requests.structures import CaseInsensitiveDict
from steamfiles import appinfo
from steamid import SteamID
import json
from hashlib import sha256
from concurrent.futures import ThreadPoolExecutor
from urllib.parse import urlparse
from bsdiff4 import file_patch
from subprocess import Popen

timeStart = perf_counter()

contactInfo = "\n\nIf you need help, contact us:\n- Discord: https://www.solsticegamestudios.com/chat.html\n- Email: contact@solsticegamestudios.com"

# Find Steam
reg = winreg.ConnectRegistry(None, winreg.HKEY_CURRENT_USER)
steamKey = winreg.OpenKey(reg, "Software\\Valve\\Steam")
steamPathValue = winreg.QueryValueEx(steamKey, "SteamPath")
steamPath = steamPathValue[0].replace("/", "\\")

print("Steam Path:\n" + steamPath + "\n")

# Find Steam Config
if not os.path.isfile(steamPath + "\\Config\\config.vdf"):
	sys.exit(colored("Error: Steam Config File Not Found!" + contactInfo, "red"))

with open(steamPath + "\\Config\\config.vdf", "r", encoding="UTF-8") as steamConfigFile:
	steamConfig = vdf.load(steamConfigFile, mapper=CaseInsensitiveDict)
	steamConfig = steamConfig["InstallConfigStore"]["Software"]["Valve"]["Steam"]

# Get Steam Libraries
steamLibraries = []
if os.path.isdir(steamPath + "\\steamapps"):
	steamLibraries.append(steamPath + "\\steamapps")

for configKey in steamConfig:
	if "BaseInstallFolder" in configKey:
		steamLibraries.append(steamConfig[configKey])

if len(steamLibraries) == 0:
	sys.exit(colored("Error: No Steam Libraries Found!" + contactInfo, "red"))

print("Steam Libraries:")
print(steamLibraries)

# Find most recent Steam User, which is probably the one they're using/want
if not os.path.isfile(steamPath + "\\Config\\loginusers.vdf"):
	sys.exit(colored("Error: Steam LoginUsers File Not Found!" + contactInfo, "red"))

steamUser = {"Timestamp": 0}
with open(steamPath + "\\Config\\loginusers.vdf", "r", encoding="UTF-8") as steamLoginUsersFile:
	steamLoginUsers = vdf.load(steamLoginUsersFile, mapper=CaseInsensitiveDict)
	steamLoginUsers = steamLoginUsers["users"]

	for userSteamID64 in steamLoginUsers:
		curSteamUser = steamLoginUsers[userSteamID64]

		if steamLoginUsers[userSteamID64]["mostrecent"] == "1":
			steamUser = {"steamID64": userSteamID64, "PersonaName": curSteamUser["PersonaName"], "Timestamp": int(curSteamUser["Timestamp"])}
			break
		elif steamLoginUsers[userSteamID64]["Timestamp"] > steamUser["Timestamp"]:
			steamUser = {"steamID64": userSteamID64, "PersonaName": curSteamUser["PersonaName"], "Timestamp": int(curSteamUser["Timestamp"])}

if steamUser["Timestamp"] > 0:
	steamUser["steamID3"] = SteamID(steamUser["steamID64"]).steam3()
	print("\nGot Most Recent Steam User: " + steamUser["PersonaName"] + " (" + steamUser["steamID64"] + " / " + steamUser["steamID3"] + ")")
else:
	sys.exit(colored("Error: Could not find Most Recent Steam User! Have you ever launched Steam?" + contactInfo, "red"))

# Find GMod
foundGMod = False
gmodPath = ""
possibleGModPaths = [
	"\\steamapps\\common\\GarrysMod",
	"\\common\\GarrysMod",
	"\\GarrysMod"
]
for path in steamLibraries:
	for curGModPath in possibleGModPaths:
		if os.path.isdir(path + curGModPath):
			foundGMod = True
			gmodPath = path + curGModPath
			break

if foundGMod:
	print("\nFound Garry's Mod:\n" + gmodPath + "\n")
else:
	sys.exit(colored("Error: Could Not Find Garry's Mod!" + contactInfo, "red"))

# Get GMod's Steam AppInfo
osTypeMap = {
	"win32": b"windows",
	"darwin": b"macos",
	"linux": b"linux"
}

print("Getting Steam AppInfo for GMod...")

if not os.path.isfile(steamPath + "\\appcache\\appinfo.vdf"):
	sys.exit(colored("Error: Steam AppInfo File Not Found!" + contactInfo, "red"))

# Get GMod Executable Paths
gmodEXELaunchOptions = []
with open(steamPath + "\\appcache\\appinfo.vdf", "rb") as steamAppInfoFile:
	steamAppInfo = appinfo.load(steamAppInfoFile)
	gmodLaunchConfig = steamAppInfo[4000]["sections"][b"appinfo"][b"config"][b"launch"]
	for option in gmodLaunchConfig:
		option = gmodLaunchConfig[option]

		if option[b"config"][b"oslist"] == osTypeMap[sys.platform] and os.path.isfile(gmodPath + "\\" + option[b"executable"].decode("UTF-8")):
			gmodEXELaunchOptions.append(option)

gmodEXELaunchOptionsLen = len(gmodEXELaunchOptions)
if gmodEXELaunchOptionsLen > 0:
	print("GMod EXE Launch Options Detected: " + str(gmodEXELaunchOptionsLen) + "\n")
else:
	sys.exit(colored("Error: Could not detect GMod EXE Launch Options!" + contactInfo, "red"))

# Get the User Launch Options for GMod
steamUserLocalConfigPath = steamPath + "\\userdata\\" + steamUser["steamID3"].split(":")[2][:-1] + "\\config\\localconfig.vdf"
if not os.path.isfile(steamUserLocalConfigPath):
	sys.exit(colored("Error: Steam User LocalConfig File Not Found!" + contactInfo, "red"))

gmodUserLaunchOptions = ""
with open(steamUserLocalConfigPath, "r", encoding="UTF-8") as steamUserLocalConfigFile:
	steamUserLocalConfig = vdf.load(steamUserLocalConfigFile, mapper=CaseInsensitiveDict)
	steamUserLocalConfig = steamUserLocalConfig["UserLocalConfigStore"]["Software"]["Valve"]["Steam"]
	gmodLocalConfig = steamUserLocalConfig["Apps"]["4000"]
	if "LaunchOptions" in gmodLocalConfig:
		gmodUserLaunchOptions = " " + gmodLocalConfig["LaunchOptions"]

# Find GMod Manifest
foundGModManifest = False
gmodManifestPath = ""
possibleGModManifestPaths = [
	"\\steamapps\\appmanifest_4000.acf",
	"\\appmanifest_4000.acf"
]
for path in steamLibraries:
	for curGModManifestPath in possibleGModManifestPaths:
		if os.path.isfile(path + curGModManifestPath):
			foundGModManifest = True
			gmodManifestPath = path + curGModManifestPath
			break

if foundGModManifest:
	print("Found Garry's Mod Manifest:\n" + gmodManifestPath + "\n")
else:
	sys.exit(colored("Error: Could Not Find Garry's Mod Manifest!" + contactInfo, "red"))

# Get GMod Branch
with open(gmodManifestPath, "r", encoding="UTF-8") as gmodManifestFile:
	gmodManifest = vdf.load(gmodManifestFile, mapper=CaseInsensitiveDict)
	gmodBranch = "betakey" in gmodManifest["AppState"]["UserConfig"] and gmodManifest["AppState"]["UserConfig"]["betakey"] or "main"

print("Garry's Mod Branch:\n" + gmodBranch + "\n")

# Get CEFCodecFix Manifest
manifestCon = http.client.HTTPSConnection("raw.githubusercontent.com")
manifestCon.request("GET", "/solsticegamestudios/GModCEFCodecFix/master/manifest.json")
manifestResp = manifestCon.getresponse()

if manifestResp.status != 200:
	sys.exit(colored("Error: CEFCodecFix Manifest Failed to Load!" + contactInfo, "red"))

manifest = json.loads(manifestResp.read())
manifestCon.close()

if not sys.platform in manifest:
	sys.exit(colored("Error: This Operating System is not yet supported by CEFCodecFix!" + contactInfo, "red"))

if not gmodBranch in manifest[sys.platform]:
	sys.exit(colored("Error: This Branch of Garry's Mod is not supported! Please switch to the x86-64 branch and then try again." + contactInfo, "red"))

# Check File Status
manifest = manifest[sys.platform][gmodBranch]
print("CEFCodecFix Manifest Loaded!\nChecking Files to see what needs to be Fixed...")

def getFileSHA256(filePath):
	fileSHA256 = sha256()

	with open(filePath, "rb") as cefFile:
		while True:
			fileData = cefFile.read(10485760) # Read about 10MB at a time
			if not fileData:
				break
			fileSHA256.update(fileData)

	return fileSHA256.hexdigest().upper()

filesToUpdate = []
fileNoMatchOriginal = False
printLock = False
def determineFileIntegrityStatus(file):
	fileSHA256 = getFileSHA256(gmodPath + "\\" + file)

	if fileSHA256 != manifest[file]["fixed"]:
		# File needs to be fixed
		if fileSHA256 == manifest[file]["original"]:
			# And it matches the original
			filesToUpdate.append(file)
			return "\t" + file + ": Needs Fix"
		else:
			# And it doesn't match the original...
			fileNoMatchOriginal = True
			return "\t" + file + ": Does Not Match Original!"
	else:
		return "\t" + file + ": Already Fixed"

with ThreadPoolExecutor() as executor:
	for fileIntegrityResult in executor.map(determineFileIntegrityStatus, manifest):
		print(fileIntegrityResult)

# Something's wrong; bail before we break their installation or something
if fileNoMatchOriginal:
	sys.exit(colored("\nError: One or More Files Failed to Match the Original Checksum!\n\tPlease Validate Garry's Mod Integrity and Try Again!" + contactInfo, "red"))

if len(filesToUpdate) > 0:
	print("\nFixing Files...")

	curDir = os.path.dirname(os.path.realpath(__file__))
	cacheDir = curDir + "\\GModCEFCodecFixFiles"
	cacheExists = os.path.isdir(cacheDir)

	if not cacheExists:
		os.mkdir(cacheDir)

	for file in filesToUpdate:
		cachedFileValid = False

		if cacheExists and os.path.isfile(cacheDir + "\\" + file + ".bsdiff"):
			# Use cached patch files if available, but check the checksums first
			fileSHA256 = getFileSHA256(cacheDir + "\\" + file + ".bsdiff")
			if fileSHA256 == manifest[file]["patch"]:
				cachedFileValid = True

		if not cachedFileValid:
			patchURL = manifest[file]["patch-url"]
			print("\tDownloading: " + patchURL + "...")
			patchURLParsed = urlparse(patchURL)
			if patchURLParsed.scheme == "https":
				cefPatchCon = http.client.HTTPSConnection(patchURLParsed.netloc)
			else:
				cefPatchCon = http.client.HTTPConnection(patchURLParsed.netloc)

			cefPatchCon.request("GET", patchURLParsed.path)
			cefPatchResp = cefPatchCon.getresponse()
			if cefPatchResp.status != 200:
				cefPatchCon.close()
				sys.exit(colored("Error: Failed to Download " + file + " | HTTP " + str(cefPatchResp.status) + " " + cefPatchResp.reason + contactInfo, "red"))
			else:
				# Create needed directories if they don't exist already
				os.makedirs(os.path.dirname(cacheDir + "\\" + file + ".bsdiff"), exist_ok = True)
				with open(cacheDir + "\\" + file + ".bsdiff", "wb") as newCEFPatch:
					newCEFPatch.write(cefPatchResp.read())
				cefPatchCon.close()

	readFailed = "\nError: Cannot Access One or More Files in CEFCodecFix cache.\nPlease verify that CEFCodecFix has read permissions to the CEFCodecFixFiles directory (try running as admin)" + contactInfo
	writeFailed = "\nError: Cannot Access One or More Files in Garry's Mod Installation.\nPlease verify that Garry's Mod is closed, Steam is not updating it, and that CEFCodecFix has write permissions to its directory (try running as admin)" + contactInfo
	for file in filesToUpdate:
		print("\tPatching: " + file + "...")

		if os.access(cacheDir + "\\" + file + ".bsdiff", os.R_OK):
			if not os.access(gmodPath + "\\" + file, os.W_OK):
				sys.exit(colored(writeFailed, "red"))
		else:
			sys.exit(colored(readFailed, "red"))

		try:
			file_patch(gmodPath + "\\" + file, gmodPath + "\\" + file, cacheDir + "\\" + file + ".bsdiff")
		except Exception as e:
			# Probably some read/write issue
			sys.exit(colored(writeFailed, "red"))

	# Mark steam.inf so Lua knows it's available
	with open(gmodPath + "\\garrysmod\\steam.inf", "a") as gmodSteamINFFile:
		print("\tWriting Marker: garrysmod\\steam.inf...")
		gmodSteamINFFile.write("CEFCodecFix=true\n")
else:
	print("\nNo Files Need Fixing!")

print(colored("\nCEFCodecFix applied successfully! Took " + str(round(perf_counter() - timeStart, 4)) + " second(s).", "green"))

if gmodEXELaunchOptionsLen == 1:
	gmodEXESelected = 0
else:
	validGModEXESelection = False
	while validGModEXESelection == False:
		print("\nPlease enter the option number you want to launch Garry's Mod with:")
		optionNum = 0
		for option in gmodEXELaunchOptions:
			print("\t" + str(optionNum) + " | " + option[b"description"].decode("UTF-8"))
			optionNum += 1

		gmodEXESelected = input(">>> ")
		try:
			gmodEXESelected = int(gmodEXESelected)
			if gmodEXESelected < gmodEXELaunchOptionsLen:
				validGModEXESelection = True
			else:
				print("That's not a valid option.")
		except ValueError:
			print("That's not a valid option.")

print(colored("\nLaunching Garry's Mod:", "green"))

gmodEXE = gmodPath + "\\" + gmodEXELaunchOptions[gmodEXESelected][b"executable"].decode("UTF-8") + " " + gmodEXELaunchOptions[gmodEXESelected][b"arguments"].decode("UTF-8")

print(gmodEXE + gmodUserLaunchOptions)

Popen(gmodEXE + gmodUserLaunchOptions, stdin=None, stdout=None, stderr=None, close_fds=True)
