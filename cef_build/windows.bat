set GN_DEFINES=symbol_level=1 is_official_build=true proprietary_codecs=true ffmpeg_branding=Chrome enable_widevine=true

set WINDOWSSDKDIR=D:/Windows Kits/10
set vs2022_install=D:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools

REM Don't forget to change between 32-bit and 64-bit stuff for Developer Console, VCVARS, and Build Type!
set CEF_VCVARS=%vs2022_install%/VC/Auxiliary/Build/vcvars64.bat
REM set CEF_VCVARS=%vs2022_install%/VC/Auxiliary/Build/vcvars32.bat

REM --force-clean --no-build --x64-build --no-debug-build --no-distrib --force-distrib --client-distrib --no-distrib-archive --chromium-checkout=refs/tags/84.0.4147.83
REM No --x64-build if you want x86
D:\Python312_64\python.exe D:\GModCEFCodecFixDev\Internal\automate\automate-git.py --download-dir=D:\GModCEFCodecFixDev\Internal\windows --force-build --x64-build --no-debug-build --force-distrib --client-distrib --no-distrib-archive --with-pgo-profiles --branch=7103
