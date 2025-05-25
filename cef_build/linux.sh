#!/usr/bin/env bash

# NOTE: If using VirtualBox Shared Folders, copy this folder + automate to the VM first. Symlinks don't work with it on NTFS
# NOTE: use_cups=false might be a little bit fucked. If CEF crashes after compiling with it, compile *without* it, then just overwrite libcef.so with the non-cups version

export GN_DEFINES="is_official_build=true use_sysroot=true symbol_level=0 is_cfi=false use_cups=false proprietary_codecs=true ffmpeg_branding=Chrome enable_widevine=true"
/usr/bin/python3 ../automate/automate-git.py --build-target=cefsimple --download-dir=/home/winter/cefcodecfix/linux --force-build --x64-build --no-debug-build --force-distrib --client-distrib --no-distrib-archive --with-pgo-profiles --branch=7103
