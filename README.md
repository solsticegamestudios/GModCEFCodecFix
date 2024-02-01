# GModCEFCodecFix

![GModCEFCodecFix](GModCEFCodecFixIcon.png)

Automatically patches [Garry's Mod](https://gmod.facepunch.com/)'s internal [Chromium Embedded Framework](https://en.wikipedia.org/wiki/Chromium_Embedded_Framework) to:
- Bring CEF up to date
- Fix GMod missing menu/launch issues on macOS and Linux
- Enable [Proprietary Video/Audio codec](https://www.chromium.org/audio-video), like H.264 (MP4) and AAC, support
- Enable [Widevine](https://www.widevine.com) support (but [no VMP](https://github.com/chromiumembedded/cef/issues/3404), so Netflix et al. don't work currently...)
- Enable Software WebGL
- Enable partial GPU acceleration

**Created by Solstice Game Studios (www.solsticegamestudios.com)**

# ❓ Players: How to use
Download the **[Latest Release](https://github.com/solsticegamestudios/GModCEFCodecFix/releases)** and run the application.

Need a more in-depth guide? Take a look at https://www.solsticegamestudios.com/forums/threads/60/

# 👩‍💻 Developers: How to use
Direct players to follow the Players' instructions above. This patch is CLIENTSIDE only!

**To Detect CEFCodecFix:** Check out our [Lua detection example](detection_example.lua)

# 📢 Need Help / Contact Us
* Discord: https://www.solsticegamestudios.com/discord/
* Email: contact@solsticegamestudios.com

# 💖 Help Support Us
This project is open source and provided free of charge for the Garry's Mod community.

**If you like what we're doing here, consider [throwing a few dollars our way](https://www.solsticegamestudios.com/donate/)!** Our work is 100% funded by users of the tool!
