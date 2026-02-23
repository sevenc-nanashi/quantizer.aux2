mod gui;

#[aviutl2::plugin(GenericPlugin)]
struct QuantizerAux2 {
    gui: aviutl2_eframe::EframeWindow,
}

impl aviutl2::generic::GenericPlugin for QuantizerAux2 {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self {
            gui: aviutl2_eframe::EframeWindow::new("QuantizerAux2", gui::create_gui)?,
        })
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        let _ = registry.register_window_client("quantizer.aux2", &self.gui);
    }
}

aviutl2::register_generic_plugin!(QuantizerAux2);
