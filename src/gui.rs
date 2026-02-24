use aviutl2::{anyhow, log};
use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use std::sync::atomic::Ordering;

pub(crate) struct QuantizerGuiApp {
    handle: AviUtl2EframeHandle,
    show_info: bool,
    suppress_info_close_once: bool,
    header_collapsed: bool,
    version: String,
    frame_count: usize,
    target_start: bool,
    target_middle: bool,
    target_end: bool,

    gaps: Option<Vec<crate::find::OffbeatInfo>>,
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
            frame_count: 1,
            target_start: true,
            target_middle: true,
            target_end: true,
            gaps: None,
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
        if self.gaps.is_some() {
            self.render_gaps_panel(ctx);
        } else {
            self.render_find_panel(ctx);
        }
    }

    fn render_find_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui
                .add_sized(
                    egui::vec2(ui.available_width(), 40.0),
                    egui::Button::new("ズレを検出"),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                let find_target = crate::find::FindTarget {
                    start: self.target_start,
                    keyframe: self.target_middle,
                    end: self.target_end,
                };
                match crate::find::find_offsync_objects(&find_target, self.frame_count) {
                    Ok(gaps) => {
                        log::info!("Found {} off-sync objects", gaps.len());
                        self.gaps = Some(gaps);
                    }
                    Err(e) => {
                        log::error!("Failed to find off-sync objects: {e}");
                        self.gaps = None;
                    }
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("フレーム数：");
                let max_frames = crate::find::max_frames_per_beat();
                ui.add_sized(
                    egui::vec2(80.0, ui.spacing().interact_size.y),
                    egui::DragValue::new(&mut self.frame_count)
                        .range(1..=((max_frames / 2.0).floor() as i32)),
                );
            });

            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label("対象：");
                ui.checkbox(&mut self.target_start, "開始位置");
                ui.checkbox(&mut self.target_middle, "中継点");
                ui.checkbox(&mut self.target_end, "終了位置");
            });
        });
    }

    fn render_gaps_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let return_response = ui
                .add_sized(
                    egui::vec2(ui.available_width(), 40.0),
                    egui::Button::new("検出に戻る"),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if return_response.clicked() {
                self.gaps = None;
                return;
            }
            ui.label(format!(
                "見つかったズレ: {} 件",
                self.gaps.as_ref().unwrap().len()
            ));

            if self.gaps.as_ref().unwrap().is_empty() {
                return;
            }
            ui.add_space(8.0);
            ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(ui.visuals().warn_fg_color);
                ui.label("手動でオブジェクトを修正した場合は「検出に戻る」を押してください。")
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut remove_indices = std::collections::HashSet::new();
                let mut remap = std::collections::HashMap::new();
                for (i, gap) in self.gaps.as_ref().unwrap().iter().enumerate() {
                    if self.draw_gap_card(ui, gap, &mut remap) {
                        remove_indices.insert(i);
                    }
                }
                for (i, gap) in self.gaps.as_mut().unwrap().iter_mut().enumerate() {
                    if let Some(new_handle) = remap.get(&gap.object) {
                        if let Some(new_handle) = new_handle {
                            gap.object = *new_handle;
                        } else {
                            remove_indices.insert(i);
                        }
                    }
                    if let crate::find::TimingType::EndThenStart {
                        object_handle_left, ..
                    } = &mut gap.timing_type
                        && let Some(new_handle) = remap.get(object_handle_left)
                    {
                        if let Some(new_handle) = new_handle {
                            *object_handle_left = *new_handle;
                        } else {
                            remove_indices.insert(i);
                        }
                    }
                }
                let mut remove_indices: Vec<usize> = remove_indices.into_iter().collect();
                remove_indices.sort_unstable();

                for i in remove_indices.into_iter().rev() {
                    self.gaps.as_mut().unwrap().remove(i);
                }
            });
        });
    }

    fn draw_gap_card(
        &self,
        ui: &mut egui::Ui,
        gap: &crate::find::OffbeatInfo,
        object_handle_map: &mut std::collections::HashMap<
            aviutl2::generic::ObjectHandle,
            Option<aviutl2::generic::ObjectHandle>,
        >,
    ) -> bool {
        let frame = egui::Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .inner_margin(egui::Margin::symmetric(8, 4));
        let available_width = ui.available_width();
        let mut remove = false;
        ui.allocate_ui_with_layout(
            egui::vec2(available_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        match &gap.timing_type {
                            crate::find::TimingType::Start { object_name } => {
                                ui.label("種別：開始位置");
                                ui.label(format!("オブジェクト：{}", object_name));
                            }
                            crate::find::TimingType::Keyframe {
                                object_name,
                                keyframe_index,
                            } => {
                                ui.label(format!("種別：中継点（{}）", keyframe_index + 1));
                                ui.label(format!("オブジェクト：{}", object_name));
                            }
                            crate::find::TimingType::End { object_name } => {
                                ui.label("種別：終了位置");
                                ui.label(format!("オブジェクト：{}", object_name));
                            }
                            crate::find::TimingType::EndThenStart {
                                object_name_left,
                                object_name_right,
                                ..
                            } => {
                                ui.label("種別：境界");
                                ui.label(format!(
                                    "オブジェクト：{} → {}",
                                    object_name_left, object_name_right
                                ));
                            }
                        }
                        ui.label(format!("レイヤー：{}", gap.layer_name));
                        ui.label(format!("フレーム：{}f", gap.frame));
                        ui.label(format!(
                            "ずれ：{}",
                            if gap.offset_frames > 0 {
                                format!("+{}f", gap.offset_frames)
                            } else {
                                format!("{}f", gap.offset_frames)
                            }
                        ));
                        ui.add_space(4.0);
                        if ui
                            .add_sized(
                                egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                                egui::Button::new("ジャンプ"),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            let res = self.jump_to_gap(gap);
                            if let Err(e) = res {
                                log::error!("Failed to jump to gap: {e}");
                            }
                        }
                        if ui
                            .add_sized(
                                egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                                egui::Button::new("除外"),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            let res = crate::find::mark_ignored(&[gap.object], object_handle_map);
                            match res {
                                Ok(_) => {
                                    log::info!("Gap ignored successfully");
                                    remove = true;
                                }
                                Err(e) => {
                                    log::error!("Failed to add marker: {e}");
                                }
                            }
                        }
                        if ui
                            .add_sized(
                                egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                                egui::Button::new("補正"),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            let res = crate::find::fix_offbeat(gap, object_handle_map);
                            match res {
                                Ok(_) => {
                                    log::info!("Gap fixed successfully");
                                    remove = true;
                                }
                                Err(e) => {
                                    log::error!("Failed to fix gap: {e}");
                                }
                            }
                        }
                    });
                });
            },
        );
        remove
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

    fn jump_to_gap(&self, gap: &crate::find::OffbeatInfo) -> aviutl2::AnyResult<()> {
        crate::EDIT_HANDLE.call_edit_section(|edit| {
            edit.set_cursor_layer_frame(gap.position.layer, gap.frame)?;
            edit.focus_object(&gap.object)?;

            anyhow::Ok(())
        })??;
        Ok(())
    }
}

impl eframe::App for QuantizerGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if crate::RESET_GAPS_ON_PROJECT_LOAD.swap(false, Ordering::Relaxed) {
            self.gaps = None;
        }
        if !crate::EDIT_HANDLE.is_ready() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading...");
                });
            });
            return;
        }
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
