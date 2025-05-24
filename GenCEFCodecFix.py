#!/usr/bin/env python3

# GenCEFCodecFix: A manifest generation tool for GModCEFCodecFix
#
# NOTE: 64-bit required due to Memory requirements!
#
# EXAMPLE: python GenCEFCodecFix.py "D:\GModCEFCodecFixDev\Internal" "D:\GModCEFCodecFixDev\External"
#
# Copyright 2020-2025, Solstice Game Studios (www.solsticegamestudios.com)
# LICENSE: GNU General Public License v3.0

from time import time
import sys
import os
import http.client
import bsdiff4
from hashlib import sha256
import json
from concurrent.futures import ThreadPoolExecutor

timeStart = time()

originalPathRoot = os.path.join(sys.argv[-2], "release_original")
fixedPathRoot = os.path.join(sys.argv[-2], "release_fixed")
patchTargetPathRoot = sys.argv[-1]
manifestDest = os.path.join(patchTargetPathRoot, "manifest.json")

httpServerPathRoot = "https://media.githubusercontent.com/media/solsticegamestudios/GModCEFCodecFix/master"

locales = ["af", "am", "ar", "bg", "bn", "ca", "cs", "da", "de", "el", "en-GB", "en_GB", "en-US", "en_US", "es", "es-419", "es_419", "et", "fa", "fi", "fil", "fr", "gu", "he", "hi", "hr", "hu", "id", "it", "ja", "kn", "ko", "lt", "lv", "ml", "mr", "ms", "nb", "nl", "pl", "pt-BR", "pt_BR", "pt-PT", "pt_PT", "ro", "ru", "sk", "sl", "sr", "sv", "sw", "ta", "te", "th", "tr", "uk", "ur", "vi", "zh-CN", "zh_CN", "zh-TW", "zh_TW"]

filesToDiff = {
	"win32": {
		#"chromium": [],
		"x86-64": [
			"bin/chrome_100_percent.pak",
			"bin/chrome_200_percent.pak",
			"bin/resources.pak",

			"bin/chrome_elf.dll",
			"bin/d3dcompiler_47.dll",
			"bin/gmod.exe",
			"bin/html_chromium.dll",
			"bin/icudtl.dat",
			"bin/libcef.dll",
			"bin/libEGL.dll",
			"bin/libGLESv2.dll",
			"bin/snapshot_blob.bin",
			"bin/v8_context_snapshot.bin",
			"bin/vk_swiftshader.dll",
			"bin/vk_swiftshader_icd.json",
			"bin/vulkan-1.dll",

			"bin/chromium/cef.pak",
			"bin/chromium/cef_100_percent.pak",
			"bin/chromium/cef_200_percent.pak",
			"bin/chromium/cef_extensions.pak",
			"bin/chromium/devtools_resources.pak",

			"bin/win64/chrome_100_percent.pak",
			"bin/win64/chrome_200_percent.pak",
			"bin/win64/resources.pak",

			"bin/win64/chrome_elf.dll",
			"bin/win64/d3dcompiler_47.dll",
			"bin/win64/dxcompiler.dll",
			"bin/win64/dxil.dll",
			"bin/win64/gmod.exe",
			"bin/win64/html_chromium.dll",
			"bin/win64/icudtl.dat",
			"bin/win64/libcef.dll",
			"bin/win64/libEGL.dll",
			"bin/win64/libGLESv2.dll",
			"bin/win64/snapshot_blob.bin",
			"bin/win64/v8_context_snapshot.bin",
			"bin/win64/vk_swiftshader.dll",
			"bin/win64/vk_swiftshader_icd.json",
			"bin/win64/vulkan-1.dll",

			"garrysmod/lua/menu/mainmenu.lua"
		]
	},
	"darwin": {
		"x86-64": [
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework",

			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libEGL.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libGLESv2.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libswiftshader_libEGL.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libswiftshader_libGLESv2.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/libvk_swiftshader.dylib",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Libraries/vk_swiftshader_icd.json",

			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_100_percent.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_200_percent.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/chrome_100_percent.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/chrome_200_percent.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/cef_extensions.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/devtools_resources.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/gpu_shader_cache.bin",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/icudtl.dat",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/Info.plist",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/resources.pak",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/snapshot_blob.bin",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/v8_context_snapshot.bin",
			"GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/v8_context_snapshot.x86_64.bin",

			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Alerts).app/Contents/_CodeSignature/CodeResources",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Alerts).app/Contents/Info.plist",
			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Alerts).app/Contents/PkgInfo",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Alerts).app/Contents/MacOS/gmod Helper (Alerts)",

			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (GPU).app/Contents/_CodeSignature/CodeResources",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (GPU).app/Contents/Info.plist",
			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (GPU).app/Contents/PkgInfo",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (GPU).app/Contents/MacOS/gmod Helper (GPU)",

			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Plugin).app/Contents/_CodeSignature/CodeResources",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Plugin).app/Contents/Info.plist",
			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Plugin).app/Contents/PkgInfo",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Plugin).app/Contents/MacOS/gmod Helper (Plugin)",

			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Renderer).app/Contents/_CodeSignature/CodeResources",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Renderer).app/Contents/Info.plist",
			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Renderer).app/Contents/PkgInfo",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper (Renderer).app/Contents/MacOS/gmod Helper (Renderer)",

			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper.app/Contents/_CodeSignature/CodeResources",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper.app/Contents/Info.plist",
			#"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper.app/Contents/PkgInfo",
			"GarrysMod_Signed.app/Contents/Frameworks/gmod Helper.app/Contents/MacOS/gmod Helper",

			#"GarrysMod_Signed.app/Contents/MacOS/gmod",
			"GarrysMod_Signed.app/Contents/MacOS/html_chromium.dylib",

			"garrysmod/html/js/menu/control.Menu.js",
			"garrysmod/lua/menu/mainmenu.lua"
		]
	},
	"linux": {
		"x86-64": [
			"bin/linux32/chromium/cef.pak",
			"bin/linux32/chromium/cef_100_percent.pak",
			"bin/linux32/chromium/cef_200_percent.pak",
			"bin/linux32/chromium/cef_extensions.pak",
			"bin/linux32/chromium/devtools_resources.pak",

			"bin/linux64/chrome_100_percent.pak",
			"bin/linux64/chrome_200_percent.pak",
			"bin/linux64/resources.pak",

			"bin/linux64/chrome-sandbox",
			"bin/linux64/chromium_process",
			"bin/linux64/gmod",
			"bin/linux64/html_chromium_client.so",
			"bin/linux64/icudtl.dat",
			"bin/linux64/libcef.so",
			"bin/linux64/libEGL.so",
			"bin/linux64/libGLESv2.so",
			"bin/linux64/libvk_swiftshader.so",
			"bin/linux64/libvulkan.so.1",
			"bin/linux64/snapshot_blob.bin",
			"bin/linux64/v8_context_snapshot.bin",
			"bin/linux64/vk_swiftshader_icd.json",

			"garrysmod/lua/menu/mainmenu.lua"
		]
	}
}

for locale in locales:
	filesToDiff["win32"]["x86-64"].append("bin/chromium/locales/" + locale + ".pak")
	filesToDiff["win32"]["x86-64"].append("bin/locales/" + locale + ".pak")
	filesToDiff["win32"]["x86-64"].append("bin/win64/locales/" + locale + ".pak")
	filesToDiff["linux"]["x86-64"].append("bin/linux32/chromium/locales/" + locale + ".pak")
	filesToDiff["linux"]["x86-64"].append("bin/linux64/locales/" + locale + ".pak")
	filesToDiff["darwin"]["x86-64"].append("GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/" + locale.replace("-", "_") + ".lproj/locale.pak")

# macOS has gotta be special
filesToDiff["darwin"]["x86-64"].append("GarrysMod_Signed.app/Contents/Frameworks/Chromium Embedded Framework.framework/Resources/en.lproj/locale.pak")

print("\nArguments: " + str(sys.argv) + "\n")
print("Original Path: " + originalPathRoot)
print("Patch Path: " + patchTargetPathRoot + "\n")

def getFileSHA256(filePath):
	fileSHA256 = sha256()

	try:
		with open(filePath, "rb") as file:
			while True:
				fileData = file.read(10485760) # Read about 10MB at a time
				if not fileData:
					break
				fileSHA256.update(fileData)
	except FileNotFoundError:
		pass

	return fileSHA256.hexdigest().upper()

print("Generating BSDIFF patches...")

manifest = {}
fileHashes = {}
filesToSkip = {}
def hashAndDiffFile(file):
	output = "\t" + os.path.join(platform, branch, file)

	fileHashes[platform][branch][file] = {}

	originalFilePath = os.path.join(originalPathRoot, platform, branch, file)
	fixedFilePath = os.path.join(fixedPathRoot, platform, "x86-64" if branch == "x86-64-temp" else branch, file) # x86-64-temp is the same as x86-64, but with different original files
	patchFilePath = os.path.join(patchTargetPathRoot, platform, branch, file + ".bsdiff")

	originalHash = getFileSHA256(originalFilePath)
	fixedHash = getFileSHA256(fixedFilePath)

	fileHashes[platform][branch][file]["original"] = originalHash
	fileHashes[platform][branch][file]["fixed"] = fixedHash

	if originalHash != fixedHash:
		fileTimeStart = time()

		os.makedirs(os.path.dirname(patchFilePath), exist_ok=True)

		if not os.path.isfile(originalFilePath):
			output += "\n\t\tOriginal doesn't exist, setting to NULL"
			originalFilePath = "NUL" if sys.platform == "win32" else "/dev/null"
		elif not os.path.isfile(fixedFilePath):
			output += "\n\t\tFixed doesn't exist, setting to NULL"
			fixedFilePath = "NUL" if sys.platform == "win32" else "/dev/null"

		bsdiff4.file_diff(originalFilePath, fixedFilePath, patchFilePath)

		output += "\n\t\tTook " + str(time() - fileTimeStart) + " second(s)"
	else:
		output += "\n\t\tSkipped: Original matches Fixed hash" + (" (Files Not Found!)" if originalHash == "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855" else "")
		filesToSkip[platform][branch].append(file)

	return output

for platform in filesToDiff:
	manifest[platform] = platform in manifest and manifest[platform] or {}
	fileHashes[platform] = platform in fileHashes and fileHashes[platform] or {}
	filesToSkip[platform] = platform in filesToSkip and filesToSkip[platform] or {}

	for branch in filesToDiff[platform]:
		manifest[platform][branch] = branch in manifest[platform] and manifest[platform][branch] or {}
		fileHashes[platform][branch] = branch in fileHashes[platform] and fileHashes[platform][branch] or {}
		filesToSkip[platform][branch] = branch in filesToSkip[platform] and filesToSkip[platform][branch] or []

		# Legacy single-threaded approach
		#for file in filesToDiff[platform][branch]:
		#	print(hashAndDiffFile(file))

		# Multithreaded hashing/diffing
		with ThreadPoolExecutor() as executor:
			for output in executor.map(hashAndDiffFile, filesToDiff[platform][branch]):
				print(output)

print("\nGenerating New Manifest...")
manifestTimeStart = time()

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

print("\tTook " + str(time() - manifestTimeStart) + " second(s)")

print("\nCEFCodecFix Generation Complete, took " + str(time() - timeStart) + " second(s). NOTE: Remember to update GitHub with all of this!")
