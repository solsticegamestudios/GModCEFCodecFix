set GN_DEFINES=symbol_level=1 is_official_build=true use_thin_lto=false proprietary_codecs=true ffmpeg_branding=Chrome enable_widevine=true
set GN_ARGUMENTS=--ide=vs2022 --sln=cef --filters=//cef/*

set WINDOWSSDKDIR=D:/Windows Kits/10

REM
REM Don't forget to change between 32-bit and 64-bit stuff for Developer Console, VCVARS, and Build Type!
REM
set vs2022_install=D:/Program Files/Microsoft Visual Studio/2022/Community
set CEF_VCVARS=%vs2022_install%/VC/Auxiliary/Build/vcvars64.bat

REM --no-build --x64-build --no-debug-build --no-distrib --force-distrib --client-distrib --no-distrib-archive --chromium-checkout=refs/tags/84.0.4147.83
REM No --x64-build if you want x86
D:\Python311_64\python.exe D:\GModCEFCodecFixDev\Internal\automate\automate-git.py --download-dir=D:\GModCEFCodecFixDev\Internal\windows --force-build --x64-build --no-debug-build --force-distrib --client-distrib --no-distrib-archive --with-pgo-profiles --branch=6367 --chromium-checkout=refs/tags/124.0.6367.119
