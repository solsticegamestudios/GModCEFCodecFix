// https://stackoverflow.com/a/65393488
use std::io;

fn main() -> io::Result<()> {
    #[cfg(target_os = "windows")]
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winresource::WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("GModPatchToolLogo.ico")
            .compile()?;
    }
    Ok(())
}
