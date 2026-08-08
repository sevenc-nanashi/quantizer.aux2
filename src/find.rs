use aviutl2::{
    anyhow::{self, Context},
    generic::ObjectHandle,
};

#[derive(Debug)]
pub struct FindTarget {
    pub start: bool,
    pub keyframe: bool,
    pub end: bool,
    pub project_end: bool,
}

pub fn max_frames_per_beat() -> anyhow::Result<f64> {
    crate::EDIT_HANDLE.call_read_section(|edit| {
        let info = crate::EDIT_HANDLE.get_edit_info();
        let bpm_list = edit.get_grid_bpm_list()?;
        crate::grid::max_frames_per_beat(&info, &bpm_list)
    })?
}

#[derive(Debug, Clone)]
pub struct OffbeatInfo {
    pub timing_type: TimingType,
    pub offset_frames: i64,
    pub object: ObjectHandle,
    pub layer_name: String,
    pub position: aviutl2::generic::ObjectLayerFrame,
    pub frame: usize,
}
#[derive(Debug, Clone)]
pub enum TimingType {
    Start {
        object_name: String,
    },
    Keyframe {
        object_name: String,
        keyframe_index: usize,
    },
    End {
        object_name: String,
    },
    EndThenStart {
        object_name_left: String,
        object_handle_left: ObjectHandle,
        object_name_right: String,
    },
}

pub fn find_offsync_objects(
    find_target: &FindTarget,
    distance: usize,
) -> anyhow::Result<Vec<OffbeatInfo>> {
    crate::EDIT_HANDLE.call_edit_section(|edit| {
        let bpm_list = edit.get_grid_bpm_list()?;
        let mut all_timings = Vec::new();
        for layer in edit.layers() {
            let layer_name = layer.get_name()?.unwrap_or_else(|| {
                format!(
                    "{}{}",
                    aviutl2::config::get_language_text("Name", "Layer"),
                    layer.index + 1
                )
            });
            for (position, object) in layer.objects() {
                let alias = edit.object(object).get_alias_parsed()?;

                let frames: Vec<usize> = alias
                    .get_table("Object")
                    .context("Object table not found")?
                    .parse_value("frame")
                    .context("frame column not found")??;

                let object_name = get_object_name(&alias)?;

                for (i, &frame) in frames.iter().enumerate() {
                    all_timings.push(OffbeatInfo {
                        frame,
                        timing_type: if i == 0 {
                            TimingType::Start {
                                object_name: object_name.clone(),
                            }
                        } else if i > 0 && i < frames.len() - 1 {
                            TimingType::Keyframe {
                                object_name: object_name.clone(),
                                keyframe_index: i - 1,
                            }
                        } else if i == frames.len() - 1 {
                            TimingType::End {
                                object_name: object_name.clone(),
                            }
                        } else {
                            unreachable!()
                        },
                        object,
                        layer_name: layer_name.clone(),
                        position,
                        offset_frames: 0,
                    });
                }
            }
        }

        let mut joined_timings = Vec::new();
        for (i, timing) in all_timings.iter().enumerate() {
            if i > 0
                && let TimingType::Start {
                    object_name: object_name_right,
                } = &timing.timing_type
            {
                let last_timing: &OffbeatInfo = joined_timings.last().unwrap();
                if timing.position.layer == last_timing.position.layer
                    && timing.frame == last_timing.frame + 1
                    && let TimingType::End {
                        object_name: object_name_left,
                    } = &last_timing.timing_type
                {
                    let object_handle_left = last_timing.object;
                    let object_name_left = object_name_left.clone();
                    joined_timings.pop();
                    joined_timings.push(OffbeatInfo {
                        timing_type: TimingType::EndThenStart {
                            object_name_left,
                            object_handle_left,
                            object_name_right: object_name_right.clone(),
                        },
                        ..timing.clone()
                    });
                    continue;
                }
            }

            joined_timings.push(timing.clone());
        }

        let mut result = Vec::new();
        for (i, timing) in joined_timings.iter().enumerate() {
            let is_target = match &timing.timing_type {
                TimingType::Start { .. } => find_target.start,
                TimingType::Keyframe { .. } => find_target.keyframe,
                TimingType::End { .. } => find_target.end,
                TimingType::EndThenStart { .. } => find_target.start || find_target.end,
            };
            if !is_target {
                continue;
            }

            if !find_target.project_end && timing.frame == edit.info.frame_max {
                continue;
            }

            // NOTE: 終端はBPMグリッドに右に触れる感じで合っていてほしいので、そう補正する
            let offset = if matches!(timing.timing_type, TimingType::End { .. }) {
                1
            } else {
                0
            };
            let adjusted_frame = timing.frame as i64 + offset;
            let nearest_beat_frame =
                crate::grid::nearest_grid_frame(&edit.info, &bpm_list, adjusted_frame as f64)?
                    as usize;
            let offset_frames = adjusted_frame - nearest_beat_frame as i64;
            let adjusted_nearest_beat_frame = nearest_beat_frame as i64 - offset;
            if offset_frames.unsigned_abs() as usize > distance || offset_frames == 0 {
                continue;
            }

            if edit.count_object_effect(timing.object, crate::marker::IGNORE_MARKER_NAME)? > 0 {
                continue;
            }

            if i > 0 {
                let prev_timing = &joined_timings[i - 1];
                if prev_timing.position.layer == timing.position.layer
                    && adjusted_nearest_beat_frame <= (prev_timing.frame as i64)
                {
                    continue;
                }
            }
            if i < joined_timings.len() - 1 {
                let next_timing = &joined_timings[i + 1];
                if next_timing.position.layer == timing.position.layer
                    && adjusted_nearest_beat_frame >= (next_timing.frame as i64)
                {
                    continue;
                }
            }

            result.push(OffbeatInfo {
                offset_frames,
                ..timing.clone()
            });
        }

        anyhow::Ok(result)
    })?
}

fn get_object_name(alias: &aviutl2::alias::Table) -> anyhow::Result<String> {
    let object_table = alias
        .get_table("Object")
        .context("Object table not found")?;
    if let Some(name) = object_table.get_value("name") {
        return Ok(name.to_string());
    }

    let object_0_table = object_table
        .get_table("0")
        .context("Object.0 table not found")?;
    let effect_name = object_0_table
        .get_value("effect.name")
        .context("effect.name not found")?;
    let effect_name = if effect_name == "フィルタオブジェクト" {
        let object_1_table = object_table
            .get_table("1")
            .context("Object.1 table not found")?;
        object_1_table
            .get_value("effect.name")
            .context("effect.name in Object.1 not found")?
    } else {
        effect_name
    };

    let section_translated_name = aviutl2::config::get_language_text(effect_name, effect_name);
    if &section_translated_name != effect_name {
        return Ok(section_translated_name);
    }

    let effect_translated_name = aviutl2::config::get_language_text("Effect", effect_name);
    Ok(effect_translated_name)
}

pub fn fix_offbeat(offbeat_info: &OffbeatInfo) -> anyhow::Result<()> {
    crate::EDIT_HANDLE.call_edit_section(|edit| {
        let object = edit.object(offbeat_info.object);
        match &offbeat_info.timing_type {
            TimingType::Start { .. } => {
                let position = object.get_layer_frame()?;
                edit.move_object_section(
                    *object,
                    0,
                    (position.start as i64 - offbeat_info.offset_frames)
                        .try_into()
                        .context("fixed frame out of range")?,
                )?;
            }
            TimingType::End { .. } => {
                let position = object.get_layer_frame()?;
                edit.move_object_section(
                    *object,
                    object.get_section_num()?,
                    (position.end as i64 - offbeat_info.offset_frames)
                        .try_into()
                        .context("fixed frame out of range")?,
                )?;
            }
            TimingType::Keyframe { keyframe_index, .. } => {
                let position = object.get_section_frame(*keyframe_index + 1)?;
                edit.move_object_section(
                    *object,
                    *keyframe_index + 1,
                    (position as i64 - offbeat_info.offset_frames)
                        .try_into()
                        .context("fixed frame out of range")?,
                )?;
            }
            TimingType::EndThenStart {
                object_handle_left, ..
            } => {
                if offbeat_info.offset_frames > 0 {
                    let position = object.get_layer_frame()?;
                    let left_position = edit.get_object_layer_frame(*object_handle_left)?;
                    edit.move_object_section(
                        *object_handle_left,
                        edit.get_object_section_num(*object_handle_left)?,
                        (left_position.end as i64 - offbeat_info.offset_frames)
                            .try_into()
                            .context("fixed frame out of range")?,
                    )?;
                    edit.move_object_section(
                        *object,
                        0,
                        (position.start as i64 - offbeat_info.offset_frames)
                            .try_into()
                            .context("fixed frame out of range")?,
                    )?;
                } else {
                    let position = object.get_layer_frame()?;
                    let left_position = edit.get_object_layer_frame(*object_handle_left)?;
                    edit.move_object_section(
                        *object,
                        0,
                        (position.start as i64 - offbeat_info.offset_frames)
                            .try_into()
                            .context("fixed frame out of range")?,
                    )?;
                    edit.move_object_section(
                        *object_handle_left,
                        edit.get_object_section_num(*object_handle_left)?,
                        (left_position.end as i64 - offbeat_info.offset_frames)
                            .try_into()
                            .context("fixed frame out of range")?,
                    )?;
                }
            }
        }

        anyhow::Ok(())
    })??;
    Ok(())
}

pub fn mark_ignored(objects: &[ObjectHandle]) -> anyhow::Result<()> {
    crate::EDIT_HANDLE.call_edit_section(|edit| {
        for object in objects {
            let object = edit.object(*object);
            if object.count_effect(crate::marker::IGNORE_MARKER_NAME)? > 0 {
                continue;
            }
            object.create_effect(crate::marker::IGNORE_MARKER_NAME)?;
        }
        anyhow::Ok(())
    })??;
    Ok(())
}
