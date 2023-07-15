# CEF Build Scripts

These are the build scripts we use to build CEF. They are **not** grab and go! At the very least, you will have to edit the paths to make them work for your environment.

Check out these Wiki articles for help setting up the CEF build environment:
- https://bitbucket.org/chromiumembedded/cef/wiki/MasterBuildQuickStart
- https://bitbucket.org/chromiumembedded/cef/wiki/BranchesAndBuilding
- https://bitbucket.org/chromiumembedded/cef/wiki/AutomatedBuildSetup

To make your CEF build work with GMod, you'll need to compile gmod-html using it:
- https://github.com/solsticegamestudios/gmod-html

So, broad strokes steps are:
1. Set up the CEF build environment
2. Modify the CEF build scripts included here to meet your environment/needs
3. Build CEF
4. Build gmod-html using your CEF binary distrib
5. Overwrite the files in GMod with gmod-html's INSTALL target output
