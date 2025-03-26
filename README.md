# GModPatchTool / GModCEFCodecFix

![GModPatchTool](GModPatchToolLogo.png)

***GModPatchTool** does what Facepunch [don't](https://github.com/Facepunch/gmod-html/pull/3)!*

Automatically patches [Garry's Mod](https://gmod.facepunch.com/)'s internal [Chromium Embedded Framework](https://en.wikipedia.org/wiki/Chromium_Embedded_Framework) to:
- Bring CEF up to date
- Fix GMod missing menu/launch issues on macOS and Linux
- Enable [Proprietary Video/Audio codec](https://www.chromium.org/audio-video), like H.264 (MP4) and AAC, support
- Enable [Widevine](https://www.widevine.com) support (but [no VMP](https://github.com/solsticegamestudios/GModPatchTool/issues/100), so Netflix et al. don't work currently...)
- Enable Software WebGL
- Enable partial GPU acceleration

**Created by Solstice Game Studios (www.solsticegamestudios.com)**

# ❓ Players: How to use
Download the **[Latest Release](https://github.com/solsticegamestudios/GModPatchTool/releases)** and run the application.

Need a more in-depth guide? Take a look at https://www.solsticegamestudios.com/fixmedia/

# 👩‍💻 Developers: How to use
Direct players to follow the Players' instructions above. This patch is CLIENTSIDE only!

**To Detect Patched CEF:** Check out our [Lua detection example](detection_example.lua).

> [!WARNING]
> Our  CEF builds have Site Isolation enabled, which means **you must pay attention to where you're calling JavaScript-related DHTML functions!**
>
> If you call [DHTML.AddFunction](https://wiki.facepunch.com/gmod/DHTML:AddFunction), [DHTML.QueueJavascript](https://wiki.facepunch.com/gmod/DHTML:QueueJavascript), or [DHTML.RunJavascript](https://wiki.facepunch.com/gmod/Panel:RunJavascript) before the page begins loading, it WILL NOT WORK! Make sure you're calling them in [DHTML.OnBeginLoadingDocument](https://wiki.facepunch.com/gmod/Panel:OnBeginLoadingDocument) or later.
>
> Site Isolation destroys JavaScript state is on navigation like how real web browsers work.
>
> This tool includes a patch for mainmenu.lua that addresses GMod's own issues with the new approach, but **this is a breaking change** for any addon that doesn't handle HTML panel states properly for JS.

**If you want to go more in-depth:** Check out [our fork of gmod-html](https://github.com/solsticegamestudios/gmod-html) and [our CEF build scripts](cef_build).

# 📢 Need Help / Contact Us
* Read the FAQ: https://www.solsticegamestudios.com/fixmedia/faq/
* Discord: https://www.solsticegamestudios.com/discord/
* Email: contact@solsticegamestudios.com

# 💖 Help Support Us
This project is open source and provided free of charge for the Garry's Mod community.

**If you like what we're doing here, consider [throwing a few dollars our way](https://www.solsticegamestudios.com/donate/)!** Our work is 100% funded by users of the tool!
