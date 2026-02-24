#[aviutl2::plugin(FilterPlugin)]
pub struct IgnoreMarker;

pub const IGNORE_MARKER_NAME: &str = "quantizer.aux2対象外";
impl aviutl2::filter::FilterPlugin for IgnoreMarker {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        aviutl2::filter::FilterPluginTable {
            name: IGNORE_MARKER_NAME.to_string(),
            label: Some("quantizer.aux2".to_string()),
            information: "quantizer.aux2 : Mark this object to be ignored by quantizer.aux2."
                .to_string(),
            flags: aviutl2::bitflag! {
                aviutl2::filter::FilterPluginFlags {
                    video: true,
                    audio: true,
                }
            },
            config_items: vec![],
        }
    }

    fn proc_video(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _video: &mut aviutl2::filter::FilterProcVideo,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }

    fn proc_audio(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _audio: &mut aviutl2::filter::FilterProcAudio,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}
