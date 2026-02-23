use aviutl2::log;
use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};

pub(crate) struct QuantizerGuiApp {
    handle: AviUtl2EframeHandle,
    show_info: bool,
    suppress_info_close_once: bool,
    header_collapsed: bool,
    version: String,
    frame_count: i32,
    target_start: bool,
    target_middle: bool,
    target_end: bool,
}

impl QuantizerGuiApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>, handle: AviUtl2EframeHandle) -> Self {
        let header_collapsed = cc
            .egui_ctx
            .data_mut(|data| data.get_persisted::<bool>(egui::Id::new("header_collapsed")))
            .unwrap_or(false);
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "M+ 1p".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "./fonts/mplus-1p-regular.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .expect("Failed to get Proportional font family")
            .insert(0, "M+ 1p".to_owned());

        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals = aviutl2_eframe::aviutl2_visuals();
        });
        cc.egui_ctx.set_fonts(fonts);

        Self {
            handle,
            show_info: false,
            suppress_info_close_once: false,
            header_collapsed,
            version: env!("CARGO_PKG_VERSION").to_string(),
            frame_count: 0,
            target_start: true,
            target_middle: true,
            target_end: true,
        }
    }

    fn render_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let clicked = ui.heading("quantizer.aux2").interact(egui::Sense::click());
                if clicked.secondary_clicked() {
                    let _ = self.handle.show_context_menu();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let info = ui
                        .add_sized(
                            egui::vec2(
                                ui.text_style_height(&egui::TextStyle::Heading),
                                ui.text_style_height(&egui::TextStyle::Heading),
                            ),
                            egui::Button::new("i"),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("プラグイン情報");
                    if info.clicked() {
                        self.show_info = true;
                        self.suppress_info_close_once = true;
                    }

                    let collapse = ui
                        .add_sized(
                            egui::vec2(
                                ui.text_style_height(&egui::TextStyle::Heading),
                                ui.text_style_height(&egui::TextStyle::Heading),
                            ),
                            egui::Button::new("^"),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("ヘッダーを折りたたむ");
                    if collapse.clicked() {
                        self.header_collapsed = true;
                    }
                });
            });
        });
    }

    fn render_collapsed_header(&mut self, ctx: &egui::Context) {
        let toolbar = egui::TopBottomPanel::top("header")
            .exact_height(8.0)
            .show(ctx, |_ui| {});
        let response = toolbar
            .response
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.hovered() {
            let hover_color = egui::Color32::from_white_alpha(32);
            response.ctx.layer_painter(response.layer_id).rect_filled(
                response.rect,
                0.0,
                hover_color,
            );
        }
        if response.interact(egui::Sense::click()).clicked() {
            self.header_collapsed = false;
        }
    }

    fn render_main_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let shift_pressed = ui.input(|i| i.modifiers.shift);
            let fix_label = if shift_pressed {
                "すべてのズレを直す"
            } else {
                "ズレを直す"
            };
            ui.horizontal(|ui| {
                if ui.button("次のズレ").clicked() {
                    log::info!("次のズレ button clicked");
                }
                if ui.button(fix_label).clicked() {
                    log::info!("{} button clicked", fix_label);
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("フレーム数:");
                ui.add_sized(
                    egui::vec2(80.0, ui.spacing().interact_size.y),
                    egui::DragValue::new(&mut self.frame_count).range(0..=i32::MAX),
                );
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("対象:");
                ui.checkbox(&mut self.target_start, "開始位置");
                ui.separator();
                ui.checkbox(&mut self.target_middle, "中継点");
                ui.separator();
                ui.checkbox(&mut self.target_end, "終了位置");
            });
        });
    }

    fn render_info_window(&mut self, ctx: &egui::Context) {
        if !self.show_info {
            return;
        }

        let screen_rect = ctx.content_rect();
        let dim_color = egui::Color32::from_black_alpha(128);
        let dim_response = egui::Area::new(egui::Id::new("info_window_dim_layer"))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                ui.set_min_size(screen_rect.size());
                let (rect, response) =
                    ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(rect, 0.0, dim_color);
                response
            })
            .inner;

        let mut open = true;
        let response = egui::Window::new("QuantizerAux2")
            .collapsible(false)
            .movable(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label(format!("バージョン: {}", self.version));
                ui.label("キーフレームのズレを移動・補正する補助プラグインです。");
                ui.add_space(8.0);
                ui.label("開発者");
                ui.hyperlink_to("Nanashi.", "https://sevenc7c.com");
                ui.add_space(4.0);
                ui.label("ソースコード:");
                ui.hyperlink_to(
                    "sevenc-nanashi/quantizer.aux2",
                    "https://github.com/sevenc-nanashi/quantizer.aux2",
                );
            });

        if self.suppress_info_close_once {
            self.suppress_info_close_once = false;
        } else if dim_response.clicked() {
            self.show_info = false;
        } else if let Some(response) = response
            && response.response.clicked_elsewhere()
        {
            self.show_info = false;
        }
        if !open {
            self.show_info = false;
        }
    }
}

impl eframe::App for QuantizerGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.header_collapsed {
            self.render_collapsed_header(ctx);
        } else {
            self.render_header(ctx);
        }
        self.render_main_panel(ctx);
        self.render_info_window(ctx);
        ctx.data_mut(|data| {
            data.insert_persisted(egui::Id::new("header_collapsed"), self.header_collapsed);
        });
    }
}

pub(crate) fn create_gui(
    cc: &eframe::CreationContext<'_>,
    handle: AviUtl2EframeHandle,
) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Box::new(QuantizerGuiApp::new(cc, handle)))
}
