#!/usr/bin/env python3

# GenCEFCodecFix: A manifest generation tool for GModCEFCodecFix
#
# NOTE: 64-bit required due to Memory requirements!
#
# Copyright 2020, Solstice Game Studios (www.solsticegamestudios.com)
# LICENSE: GNU General Public License v3.0

from time import process_time
import os
import bsdiff4
from hashlib import sha256
import json

timeStart = process_time()

originalPathRoot = r"E:\GModCEFCodecFix_Internal\release_original"
fixedPathRoot = r"E:\GModCEFCodecFix_Internal\release_fixed"
patchTargetPathRoot = r"E:\GModCEFCodecFix"
manifestDest = r"E:\GModCEFCodecFix\manifest.json"

httpServerPathRoot = r"https://raw.githubusercontent.com/solsticegamestudios/GModCEFCodecFix/master"

filesToDiff = {
	"win32": {
		"x86-64": [
			r"bin\chrome_elf.dll",
			r"bin\d3dcompiler_47.dll",
			r"bin\html_chromium.dll",
			r"bin\icudtl.dat",
			r"bin\libcef.dll",
			r"bin\libEGL.dll",
			r"bin\libGLESv2.dll",
			r"bin\snapshot_blob.bin",
			r"bin\v8_context_snapshot.bin",

			r"bin\chromium\cef.pak",
			r"bin\chromium\cef_100_percent.pak",
			r"bin\chromium\cef_200_percent.pak",
			r"bin\chromium\cef_extensions.pak",
			r"bin\chromium\devtools_resources.pak",

			r"bin\win64\chrome_elf.dll",
			r"bin\win64\d3dcompiler_47.dll",
			r"bin\win64\html_chromium.dll",
			r"bin\win64\icudtl.dat",
			r"bin\win64\libcef.dll",
			r"bin\win64\libEGL.dll",
			r"bin\win64\libGLESv2.dll",
			r"bin\win64\snapshot_blob.bin",
			r"bin\win64\v8_context_snapshot.bin"
		]
	},
	"darwin": {
		"x86-64": [
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework",

			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libEGL.dylib",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libGLESv2.dylib",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libswiftshader_libEGL.dylib",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libswiftshader_libGLESv2.dylib",

			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef.pak",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_100_percent.pak",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_200_percent.pak",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_extensions.pak",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/devtools_resources.pak",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/icudtl.dat",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/snapshot_blob.bin",
			r"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/v8_context_snapshot.bin"

			# TODO: GMod HTML DLL
		]
	},
	"linux": {
		"x86-64": [
			r"bin/linux32/chromium/cef.pak",
			r"bin/linux32/chromium/cef_100_percent.pak",
			r"bin/linux32/chromium/cef_200_percent.pak",
			r"bin/linux32/chromium/cef_extensions.pak",
			r"bin/linux32/chromium/devtools_resources.pak",

			#r"bin/linux64/chromium_process",
			#r"bin/linux64/html_chromium_client.so",
			r"bin/linux64/icudtl.dat",
			r"bin/linux64/libcef.so",
			r"bin/linux64/libEGL.so",
			r"bin/linux64/libGLESv2.so",
			r"bin/linux64/snapshot_blob.bin",
			r"bin/linux64/v8_context_snapshot.bin"

			# TODO: GMod HTML DLL
		]
	}
}

print("Generating BSDIFF patches...")

for platform in filesToDiff:
	for branch in filesToDiff[platform]:
		for file in filesToDiff[platform][branch]:
			print("\t" + file)
			fileTimeStart = process_time()

			os.makedirs(os.path.dirname(os.path.join(patchTargetPathRoot, platform, branch, file)), exist_ok=True)
			bsdiff4.file_diff(os.path.join(originalPathRoot , platform, file), os.path.join(fixedPathRoot, platform, file), os.path.join(patchTargetPathRoot, platform, branch, file + ".bsdiff"))

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
for platform in filesToDiff:
	manifest[platform] = platform in manifest and manifest[platform] or {}

	for branch in filesToDiff[platform]:
		manifest[platform][branch] = branch in manifest[platform] and manifest[platform][branch] or {}

		for file in filesToDiff[platform][branch]:
			originalHash = getFileSHA256(os.path.join(originalPathRoot, platform, file))
			fixedHash = getFileSHA256(os.path.join(fixedPathRoot, platform, file))

			if originalHash != fixedHash:
				manifest[platform][branch][file] = {
					"original": originalHash,
					"patch": getFileSHA256(os.path.join(patchTargetPathRoot, platform, branch, file + ".bsdiff")),
					"patch-url": httpServerPathRoot + "/" + platform + "/" + branch + "/" + file.replace("\\", "/").replace(" ", "%20") + ".bsdiff",
					"fixed": fixedHash
				}
			else:
				print("Warning: Original matches Fixed hash for " + file + ", removing...")
				os.remove(os.path.join(patchTargetPathRoot, platform, branch, file + ".bsdiff"))

with open(manifestDest, "w+") as manifestFile:
	json.dump(manifest, manifestFile, indent=4)

print("\tTook " + str(process_time() - manifestTimeStart) + " second(s)")

print("\nCEFCodecFix Generation Complete, took " + str(process_time() - timeStart) + " second(s). NOTE: Remember to update GitHub with all of this!")
