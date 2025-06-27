//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // TODO: Hide console window on Windows in release

use tracing::{debug, error, info, warn};
use eframe;
use eframe::egui;

struct GUIApp {
	name: String,
	age: u32,
}

impl Default for GUIApp {
	fn default() -> Self {
		Self {
			name: "Arthur".to_owned(),
			age: 42,
		}
	}
}

impl eframe::App for GUIApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.horizontal(|ui| {
				let name_label = ui.label("Your name: ");
				ui
				.text_edit_singleline(&mut self.name)
				.labelled_by(name_label.id);
			});
			ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
			if ui.button("Increment").clicked() {
				self.age += 1;
			}
			ui.label(format!("Hello '{}', age {}", self.name, self.age));
		});
	}
}

pub fn main() -> eframe::Result {
	if true {
		return Err(eframe::Error::AppCreation(Box::from(
			"not implemented",
		)));
	}

	let viewport = egui::ViewportBuilder {
		inner_size: Some([640.0, 240.0].into()),
		resizable: Some(false),
		maximize_button: Some(false),
		icon: Some(eframe::icon_data::from_png_bytes(&include_bytes!("../../GModPatchToolLogo.png")[..]).unwrap().into()), // Needed for Taskbar/GUI App Icon
		..Default::default()
	};

	let options = eframe::NativeOptions {
		viewport,
		centered: true,
		..Default::default()
	};

	eframe::run_native(
		"GModPatchTool",
		options,
		Box::new(|cc| {
			let handler = Box::<GUIApp>::default();

			// TODO: Is this the right place for this? It should be AFTER any errors should happen during UI init. We can't init the logger more than once!
			//crate::init_logger(true, gui_terminal);
			//crate::main_script();

			Ok(handler)
		})
	)
}
