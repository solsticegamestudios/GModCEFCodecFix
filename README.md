# GModPatchTool <sub>_Formerly GModCEFCodecFix_</sub>

![GModPatchTool](GModPatchToolLogo.png)

***GModPatchTool** does what Facepunch [don't](https://github.com/Facepunch/gmod-html/pull/3)!*

**Created by Solstice Game Studios (www.solsticegamestudios.com)**

# 🛠️ Patches We Apply
### All Platforms
- Fixes various launch/missing main menu issues on macOS and Linux
- Improves the Legacy VGUI Theme with our custom SourceScheme.res
- Replaces Debug/Console fonts with [PT Mono](https://fonts.google.com/specimen/PT+Mono) to improve consistency/readability across platforms
  - This is particularly important for Proton, where text using those fonts are broken/tiny out of the box (no Lucida Console)

### In-Game Web Browser ([Chromium Embedded Framework, aka CEF](https://en.wikipedia.org/wiki/Chromium_Embedded_Framework))
- Updates CEF to 137.0.10 (Chromium 137.0.7151.69)
- Enables [Proprietary Video/Audio codec](https://www.chromium.org/audio-video), like H.264 (MP4) and AAC, support
- Enables [Widevine](https://www.widevine.com) support (but [no VMP](https://github.com/solsticegamestudios/GModPatchTool/issues/100), so Netflix et al. don't work currently...)
- Enables Software WebGL
- Enables partial GPU acceleration
- Improves performance for texture updates
- Disables Hardware Media Keys control of media
- Re-enables Site Isolation (security feature; some sites require it to function)

### Linux
- Can fix Steam Overlay/MangoHud/etc not working
  - Put `GMOD_ENABLE_LD_PRELOAD=1 %command%` in GMod's Launch Options to try it!
  - This is disabled by default because it could just crash GMod instead
- Sets `mesa_glthread=true` for more OpenGL performance with Mesa drivers
- Sets `DRI_PRIME=1` to automatically use the Dedicated GPU with Mesa drivers in Laptops
  - For Nvidia proprietary driver users: Please use the `prime-run` command, or look at `hl2.sh` after patches are applied
- Sets `ulimit -n $(ulimit -Hn)` to fix issues opening/mounting many files (many addons, Lua autorefresh, etc)

# ❓ Players: How to use
Download the **[Latest Release](https://github.com/solsticegamestudios/GModPatchTool/releases)** and run the application.

Need a more in-depth guide? Take a look at https://www.solsticegamestudios.com/fixmedia/

# 👩‍💻 Developers: How to use
Direct players to follow the Players' instructions above. This patch is CLIENTSIDE only!

**To Detect Patched CEF:** Check out our [Lua detection example](examples/detection_example.lua).

> [!WARNING]
> Our  CEF builds have Site Isolation enabled, which means **you must pay attention to where you're calling JavaScript-related DHTML functions!**
>
> If you call [DHTML.AddFunction](https://wiki.facepunch.com/gmod/DHTML:AddFunction), [DHTML.QueueJavascript](https://wiki.facepunch.com/gmod/DHTML:QueueJavascript), or [DHTML.RunJavascript](https://wiki.facepunch.com/gmod/Panel:RunJavascript) before the page begins loading, it WILL NOT WORK! Make sure you're calling them in [DHTML.OnBeginLoadingDocument](https://wiki.facepunch.com/gmod/Panel:OnBeginLoadingDocument) or later.
>
> Site Isolation destroys JavaScript state is on navigation like how real web browsers work.
>
> This tool includes a patch for mainmenu.lua that addresses GMod's own issues not using the correct approach, but **this is a breaking change** for any addon that doesn't handle HTML panel states properly for JS.

**If you want to go more in-depth:** Check out [our fork of gmod-html](https://github.com/solsticegamestudios/gmod-html) and [our CEF build scripts](cef_build).

# 📢 Need Help / Contact Us
* Read the FAQ: https://www.solsticegamestudios.com/fixmedia/faq/
* Discord: https://www.solsticegamestudios.com/discord/
* Email: contact@solsticegamestudios.com

# 💖 Help Support Us
This project is open source and provided free of charge for the Garry's Mod community.

**If you like what we're doing here, consider [throwing a few dollars our way](https://www.solsticegamestudios.com/donate/)!** Our work is 100% funded by users of the tool!
