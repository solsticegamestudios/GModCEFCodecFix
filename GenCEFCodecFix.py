# Copyright 2019, Solstice Game Studios (www.solsticegamestudios.com)
# LICENSE: GNU General Public License v3.0

from time import process_time
import bsdiff4
from hashlib import sha256
import json

timeStart = process_time()

originalPathRoot = r"D:\SteamLibrary\steamapps\common\GarrysMod"
fixedPathRoot = r"D:\SteamLibrary\steamapps\common\GarrysMod\garrysmod\addons\gmcl_shared_content\ChromiumH264\GarrysMod"
patchTargetPathRoot = r"D:\SteamLibrary\steamapps\common\GarrysMod\garrysmod\addons\sgs_gmodcefcodecfix\win32\x86-64"
manifestDest = r"D:\SteamLibrary\steamapps\common\GarrysMod\garrysmod\addons\sgs_gmodcefcodecfix\manifest.json"

# TODO: Support more than just x86-64 and Windows
httpServerPathRoot = r"https://raw.githubusercontent.com/solsticegamestudios/GModCEFCodecFix/master/win32/x86-64"

filesToDiff = [
	r"bin\chrome_elf.dll",
	r"bin\d3dcompiler_47.dll",
	#"bin\icudtl.dat",
	r"bin\libcef.dll",
	r"bin\libEGL.dll",
	r"bin\libGLESv2.dll",
	#r"bin\natives_blob.bin",
	#r"bin\snapshot_blob.bin",
	#r"bin\v8_context_snapshot.bin",

	r"bin\chromium\cef.pak",
	r"bin\chromium\cef_100_percent.pak",
	r"bin\chromium\cef_200_percent.pak",
	r"bin\chromium\cef_extensions.pak",
	r"bin\chromium\devtools_resources.pak",

	r"bin\win64\chrome_elf.dll",
	r"bin\win64\d3dcompiler_47.dll",
	#r"bin\win64\icudtl.dat"
	r"bin\win64\libcef.dll",
	r"bin\win64\libEGL.dll",
	r"bin\win64\libGLESv2.dll",
	#r"bin\win64\natives_blob.bin",
	#r"bin\win64\snapshot_blob.bin",
	#r"bin\win64\v8_context_snapshot.bin"
]

print("Generating BSDIFF patches for " + str(len(filesToDiff)) + " files...")

for file in filesToDiff:
	print("\t" + file)
	fileTimeStart = process_time()
	bsdiff4.file_diff(originalPathRoot + "\\" + file, fixedPathRoot + "\\" + file, patchTargetPathRoot + "\\" + file + ".bsdiff")
	print("\t\tTook " + str(process_time() - fileTimeStart) + " second(s)")

print("\nGenerating manifest.json...")
manifestTimeStart = process_time()

def getFileSHA256(filePath):
	fileSHA256 = sha256()

	with open(filePath, "rb") as file:
		while True:
			fileData = file.read(10485760) # Read about 10MB at a time
			if not fileData:
				break
			fileSHA256.update(fileData)

	return fileSHA256.hexdigest().upper()

manifest = {}
manifest["win32"] = {}
manifest["win32"]["x86-64"] = {}

for file in filesToDiff:
	manifest["win32"]["x86-64"][file] = {
		"original": getFileSHA256(originalPathRoot + "\\" + file),
		"patch": getFileSHA256(patchTargetPathRoot + "\\" + file + ".bsdiff"),
		"patch-url": httpServerPathRoot + "/" + file.replace("\\", "/") + ".bsdiff",
		"fixed": getFileSHA256(fixedPathRoot + "\\" + file)
	}

with open(manifestDest, "w+") as manifestFile:
	json.dump(manifest, manifestFile, indent=4)

print("\tTook " + str(process_time() - manifestTimeStart) + " second(s)")

print("\nCEFCodecFix Generation Complete, took " + str(process_time() - timeStart) + " second(s). NOTE: Remember to update the web server with all of this!")
