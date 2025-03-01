// https://stackoverflow.com/a/65393488
use {
	std::{
		env,
		io,
	},
	winresource::WindowsResource,
};

fn main() -> io::Result<()> {
	if env::var_os("CARGO_CFG_WINDOWS").is_some() {
		WindowsResource::new()
			// This path can be absolute, or relative to your crate root.
			.set_icon("GModPatchToolLogo.ico")
			.compile()?;
	}
	Ok(())
}
