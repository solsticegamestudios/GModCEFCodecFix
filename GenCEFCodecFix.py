#!/usr/bin/env python3

# GenCEFCodecFix: A manifest generation tool for GModCEFCodecFix
#
# NOTE: 64-bit required due to Memory requirements!
#
# EXAMPLE: python GenCEFCodecFix.py "D:\GModCEFCodecFixDev\Internal" "D:\GModCEFCodecFixDev\External"
#
# Copyright 2020, Solstice Game Studios (www.solsticegamestudios.com)
# LICENSE: GNU General Public License v3.0

from time import process_time
import sys
import os
import http.client
import bsdiff4
from hashlib import sha256
import json

timeStart = process_time()

originalPathRoot = os.path.join(sys.argv[-2], "release_original")
fixedPathRoot = os.path.join(sys.argv[-2], "release_fixed")
patchTargetPathRoot = sys.argv[-1]
manifestDest = os.path.join(patchTargetPathRoot, "manifest.json")

httpServerPathRoot = "https://raw.githubusercontent.com/solsticegamestudios/GModCEFCodecFix/master"

filesToDiff = {
	"win32": {
		"chromium": [
			"bin/chrome_elf.dll",
			"bin/d3dcompiler_47.dll",
			"bin/html_chromium.dll",
			"bin/icudtl.dat",
			"bin/libcef.dll",
			"bin/libEGL.dll",
			"bin/libGLESv2.dll",
			"bin/snapshot_blob.bin",
			"bin/v8_context_snapshot.bin",

			"bin/chromium/cef.pak",
			"bin/chromium/cef_100_percent.pak",
			"bin/chromium/cef_200_percent.pak",
			"bin/chromium/cef_extensions.pak",
			"bin/chromium/devtools_resources.pak",

			"bin/win64/chrome_elf.dll",
			"bin/win64/d3dcompiler_47.dll",
			"bin/win64/html_chromium.dll",
			"bin/win64/icudtl.dat",
			"bin/win64/libcef.dll",
			"bin/win64/libEGL.dll",
			"bin/win64/libGLESv2.dll",
			"bin/win64/snapshot_blob.bin",
			"bin/win64/v8_context_snapshot.bin"
		]
	},
	"darwin": {
		"x86-64": [
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework",

			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libEGL.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libGLESv2.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libswiftshader_libEGL.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libswiftshader_libGLESv2.dylib",

			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_100_percent.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_200_percent.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_extensions.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/devtools_resources.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/icudtl.dat",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/snapshot_blob.bin",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/v8_context_snapshot.bin"

			# TODO: GMod HTML DLL
		]
	},
	"linux": {
		"x86-64": [
			"bin/linux32/chromium/cef.pak",
			"bin/linux32/chromium/cef_100_percent.pak",
			"bin/linux32/chromium/cef_200_percent.pak",
			"bin/linux32/chromium/cef_extensions.pak",
			"bin/linux32/chromium/devtools_resources.pak",

			#"bin/linux64/chromium_process",
			#"bin/linux64/html_chromium_client.so",
			"bin/linux64/icudtl.dat",
			"bin/linux64/libcef.so",
			"bin/linux64/libEGL.so",
			"bin/linux64/libGLESv2.so",
			"bin/linux64/snapshot_blob.bin",
			"bin/linux64/v8_context_snapshot.bin"

			# TODO: GMod HTML DLL
		]
	}
}

print("\nArguments: " + str(sys.argv) + "\n")
print("Original Path: " + originalPathRoot)
print("Patch Path: " + patchTargetPathRoot + "\n")

# Get existing Manifest
try:
	manifestCon = http.client.HTTPSConnection("raw.githubusercontent.com")
	manifestCon.request("GET", "/solsticegamestudios/GModCEFCodecFix/master/manifest.json")
	manifestResp = manifestCon.getresponse()

	if manifestResp.status != 200:
		sys.exit("Error: Existing Manifest Failed to Load! Status Code: " + manifestResp.status)
except Exception as e:
	sys.exit("Error: Existing Manifest Failed to Load! Exception: " + e)

existingManifest = json.loads(manifestResp.read())
manifestCon.close()
print("Existing Manifest Loaded!\n")

def getFileSHA256(filePath):
	fileSHA256 = sha256()

	with open(filePath, "rb") as file:
		while True:
			fileData = file.read(10485760) # Read about 10MB at a time
			if not fileData:
				break
			fileSHA256.update(fileData)

	return fileSHA256.hexdigest().upper()

print("Generating BSDIFF patches...")

manifest = {}
fileHashes = {}
filesToSkip = {}

for platform in filesToDiff:
	manifest[platform] = platform in manifest and manifest[platform] or {}
	fileHashes[platform] = platform in fileHashes and fileHashes[platform] or {}
	filesToSkip[platform] = platform in filesToSkip and filesToSkip[platform] or {}

	for branch in filesToDiff[platform]:
		manifest[platform][branch] = branch in manifest[platform] and manifest[platform][branch] or {}
		fileHashes[platform][branch] = branch in fileHashes[platform] and fileHashes[platform][branch] or {}
		filesToSkip[platform][branch] = branch in filesToSkip[platform] and filesToSkip[platform][branch] or []

		for file in filesToDiff[platform][branch]:
			fileHashes[platform][branch][file] = {}

			originalHash = getFileSHA256(os.path.join(originalPathRoot, platform, branch, file))
			fixedHash = getFileSHA256(os.path.join(fixedPathRoot, platform, branch, file))

			fileHashes[platform][branch][file]["original"] = originalHash
			fileHashes[platform][branch][file]["fixed"] = fixedHash

			print("\t" + os.path.join(platform, branch, file))
			if originalHash != fixedHash:
				if file not in existingManifest[platform][branch] or originalHash != existingManifest[platform][branch][file]["original"] or fixedHash != existingManifest[platform][branch][file]["fixed"]:
					fileTimeStart = process_time()

					os.makedirs(os.path.dirname(os.path.join(patchTargetPathRoot, platform, branch, file)), exist_ok=True)
					bsdiff4.file_diff(os.path.join(originalPathRoot, platform, branch, file), os.path.join(fixedPathRoot, platform, branch, file), os.path.join(patchTargetPathRoot, platform, branch, file + ".bsdiff"))

					print("\t\tTook " + str(process_time() - fileTimeStart) + " second(s)")
				else:
					print("\t\tSkipped: Up to date")
					manifest[platform][branch][file] = existingManifest[platform][branch][file]
					filesToSkip[platform][branch].append(file)
			else:
				print("\t\tSkipped: Original matches Fixed hash")
				filesToSkip[platform][branch].append(file)

print("\nGenerating New Manifest...")
manifestTimeStart = process_time()

for platform in filesToDiff:
	for branch in filesToDiff[platform]:
		for file in [file for file in filesToDiff[platform][branch] if file not in filesToSkip[platform][branch]]:
			originalHash = fileHashes[platform][branch][file]["original"]
			fixedHash = fileHashes[platform][branch][file]["fixed"]

			manifest[platform][branch][file] = {
				"original": originalHash,
				"patch": getFileSHA256(os.path.join(patchTargetPathRoot, platform, branch, file + ".bsdiff")),
				"patch-url": httpServerPathRoot + "/" + platform + "/" + branch + "/" + file.replace("\\", "/").replace(" ", "%20") + ".bsdiff",
				"fixed": fixedHash
			}

with open(manifestDest, "w+") as manifestFile:
	json.dump(manifest, manifestFile, indent=4)

print("\tTook " + str(process_time() - manifestTimeStart) + " second(s)")

print("\nCEFCodecFix Generation Complete, took " + str(process_time() - timeStart) + " second(s). NOTE: Remember to update GitHub with all of this!")
