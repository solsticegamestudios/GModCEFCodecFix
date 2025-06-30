#![cfg_attr(target_os = "windows", windows_subsystem = "console")]

fn main() {
	#[cfg(feature = "generate")]
	gmodpatchtool::generate::main();

	#[cfg(feature = "patch")]
	gmodpatchtool::patch::main();
}
