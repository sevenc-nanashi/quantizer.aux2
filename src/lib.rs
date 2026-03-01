mod find;
mod grid;
mod gui;
mod marker;
use aviutl2::tracing;
use std::sync::atomic::{AtomicBool, Ordering};

#[aviutl2::plugin(GenericPlugin)]
struct QuantizerAux2 {
    gui: aviutl2_eframe::EframeWindow,
    marker: aviutl2::generic::SubPlugin<marker::IgnoreMarker>,
}

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();
pub static RESET_GAPS_ON_PROJECT_LOAD: AtomicBool = AtomicBool::new(false);

impl aviutl2::generic::GenericPlugin for QuantizerAux2 {
    fn new(info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();
        aviutl2::tracing::info!("Initializing Rusty Metronome Plugin...");
        Ok(Self {
            gui: aviutl2_eframe::EframeWindow::new("QuantizerAux2", gui::create_gui)?,
            marker: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "quantizer.aux2".to_string(),
            information: format!(
                "Quantize objects to BPM Grid / v{} / https://github.com/sevenc-nanashi/quantizer.aux2",
                env!("CARGO_PKG_VERSION")
            ),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        let _ = registry.register_window_client("quantizer.aux2", &self.gui);
        registry.register_filter_plugin(&self.marker);
        registry.register_menus::<Self>();

        EDIT_HANDLE.init(registry.create_edit_handle());
    }

    fn on_project_load(&mut self, _project: &mut aviutl2::generic::ProjectFile) {
        RESET_GAPS_ON_PROJECT_LOAD.store(true, Ordering::Relaxed);
    }
}

#[aviutl2::generic::menus]
impl QuantizerAux2 {
    #[object(name = "[quantizer.aux2] 対象外にする")]
    fn ignore_object(&mut self) -> aviutl2::AnyResult<()> {
        let objects = EDIT_HANDLE.call_edit_section(|edit| edit.get_selected_objects())??;
        crate::find::mark_ignored(&objects, &mut std::collections::HashMap::new())?;
        Ok(())
    }
}

aviutl2::register_generic_plugin!(QuantizerAux2);
