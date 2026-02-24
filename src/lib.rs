mod grid;
mod gui;
mod find;

use aviutl2::{
    anyhow::{self, Context},
    generic::{Effect, ObjectHandle},
};

#[aviutl2::plugin(GenericPlugin)]
struct QuantizerAux2 {
    gui: aviutl2_eframe::EframeWindow,
}

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

impl aviutl2::generic::GenericPlugin for QuantizerAux2 {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        aviutl2::logger::LogBuilder::new()
            .filter_level(aviutl2::logger::LevelFilter::Debug)
            .init();
        aviutl2::log::info!("Initializing Rusty Metronome Plugin...");
        Ok(Self {
            gui: aviutl2_eframe::EframeWindow::new("QuantizerAux2", gui::create_gui)?,
        })
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        let _ = registry.register_window_client("quantizer.aux2", &self.gui);

        EDIT_HANDLE.init(registry.create_edit_handle());
    }
}

aviutl2::register_generic_plugin!(QuantizerAux2);
