#!/usr/bin/env bash
export GN_DEFINES="symbol_level=1 is_official_build=true proprietary_codecs=true ffmpeg_branding=Chrome enable_widevine=true"
python3 ../automate/automate-git.py --download-dir=/Users/akiko/cefcodecfix/darwin --force-build --x64-build --no-debug-build --force-distrib --client-distrib --no-distrib-archive --with-pgo-profiles --branch=7151
