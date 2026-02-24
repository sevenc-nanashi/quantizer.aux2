use aviutl2::{
    anyhow::{self, Context},
    generic::ObjectHandle,
};

#[derive(Debug)]
pub struct FindTarget {
    pub start: bool,
    pub keyframe: bool,
    pub end: bool,
}

pub fn max_frames_per_beat() -> f64 {
    let info = crate::EDIT_HANDLE.get_edit_info();
    let fps = *info.fps.numer() as f64 / *info.fps.denom() as f64;
    let bpm = info.grid_bpm_tempo as f64;
    60.0 * fps / bpm
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
        let mut all_timings = Vec::new();
        for layer in edit.layers() {
            let layer_name = layer.get_name()?.unwrap_or_else(|| {
                format!(
                    "{}{}",
                    aviutl2::config::get_language_text("Name", "Layer").unwrap(),
                    layer.index + 1
                )
            });
            for (position, object) in layer.objects() {
                let alias = edit.object(&object).get_alias_parsed()?;

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

            // NOTE: 終端はBPMグリッドに右に触れる感じで合っていてほしいので、そう補正する
            let adjusted_frame = if matches!(timing.timing_type, TimingType::End { .. }) {
                timing.frame + 1
            } else {
                timing.frame
            };
            let current_beat = crate::grid::frame_to_beat(&edit.info, adjusted_frame as f64);
            let nearest_beat = current_beat.round();
            let nearest_beat_frame =
                crate::grid::beat_to_frame_int(&edit.info, nearest_beat) as usize;
            let offset_frames = adjusted_frame as i64 - nearest_beat_frame as i64;
            if offset_frames.unsigned_abs() as usize > distance || offset_frames == 0 {
                continue;
            }

            if i > 0 {
                let prev_timing = &joined_timings[i - 1];
                if prev_timing.position.layer == timing.position.layer
                    && nearest_beat_frame <= prev_timing.frame
                {
                    continue;
                }
            }
            if i < joined_timings.len() - 1 {
                let next_timing = &joined_timings[i + 1];
                if next_timing.position.layer == timing.position.layer
                    && nearest_beat_frame >= next_timing.frame
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

    Ok(effect_name.to_string())
}

pub fn fix_offbeat(
    offbeat_info: &OffbeatInfo,
    object_handle_map: &mut std::collections::HashMap<ObjectHandle, ObjectHandle>,
) -> anyhow::Result<()> {
    crate::EDIT_HANDLE.call_edit_section(|edit| {
        let object = edit.object(&offbeat_info.object);
        let alias = object.get_alias_parsed()?;
        match &offbeat_info.timing_type {
            TimingType::Start { .. } => {
                let new_alias = fix_starting_gap(&alias, offbeat_info.offset_frames)?;
                let position = object.get_layer_frame()?;
                object.delete_object()?;
                let new_object = edit.create_object_from_alias(
                    &new_alias.to_string(),
                    position.layer,
                    position.start - offbeat_info.offset_frames as usize,
                    0,
                )?;

                object_handle_map.insert(offbeat_info.object, new_object);
            }
            TimingType::End { .. } => {
                let new_alias = fix_ending_gap(&alias, offbeat_info.offset_frames)?;
                let position = object.get_layer_frame()?;
                object.delete_object()?;
                let new_object = edit.create_object_from_alias(
                    &new_alias.to_string(),
                    position.layer,
                    position.start,
                    0,
                )?;

                object_handle_map.insert(offbeat_info.object, new_object);
            }
            TimingType::Keyframe { keyframe_index, .. } => {
                let new_alias =
                    fix_keyframe_gap(&alias, *keyframe_index, offbeat_info.offset_frames)?;
                let position = object.get_layer_frame()?;
                object.delete_object()?;
                let new_object = edit.create_object_from_alias(
                    &new_alias.to_string(),
                    position.layer,
                    position.start,
                    0,
                )?;

                object_handle_map.insert(offbeat_info.object, new_object);
            }
            TimingType::EndThenStart {
                object_handle_left, ..
            } => {
                let left_object = edit.object(object_handle_left);
                let left_alias = left_object.get_alias_parsed()?;
                let new_left_alias = fix_ending_gap(&left_alias, offbeat_info.offset_frames)?;
                let position = left_object.get_layer_frame()?;
                left_object.delete_object()?;
                let new_left_object = edit.create_object_from_alias(
                    &new_left_alias.to_string(),
                    position.layer,
                    position.start,
                    0,
                )?;

                let right_alias = object.get_alias_parsed()?;
                let new_right_alias = fix_starting_gap(&right_alias, offbeat_info.offset_frames)?;
                let position = object.get_layer_frame()?;
                object.delete_object()?;
                let new_right_object = edit.create_object_from_alias(
                    &new_right_alias.to_string(),
                    position.layer,
                    position.start - offbeat_info.offset_frames as usize,
                    0,
                )?;

                object_handle_map.insert(*object_handle_left, new_left_object);
                object_handle_map.insert(offbeat_info.object, new_right_object);
            }
        }

        anyhow::Ok(())
    })??;
    Ok(())
}

fn fix_starting_gap(
    alias: &aviutl2::alias::Table,
    offset_frames: i64,
) -> anyhow::Result<aviutl2::alias::Table> {
    let mut frames: Vec<usize> = alias
        .get_table("Object")
        .context("Object table not found")?
        .parse_value("frame")
        .context("frame column not found")??;
    frames[1..].iter_mut().for_each(|f| {
        *f += offset_frames as usize;
    });
    let mut new_alias = alias.clone();
    new_alias
        .get_table_mut("Object")
        .context("Object table not found")?
        .insert_value(
            "frame",
            frames
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
    Ok(new_alias)
}
fn fix_ending_gap(
    alias: &aviutl2::alias::Table,
    offset_frames: i64,
) -> anyhow::Result<aviutl2::alias::Table> {
    let mut frames: Vec<usize> = alias
        .get_table("Object")
        .context("Object table not found")?
        .parse_value("frame")
        .context("frame column not found")??;
    *frames.last_mut().unwrap() = (*frames.last().unwrap() as i64 - offset_frames) as usize;
    let mut new_alias = alias.clone();
    new_alias
        .get_table_mut("Object")
        .context("Object table not found")?
        .insert_value(
            "frame",
            frames
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
    Ok(new_alias)
}
fn fix_keyframe_gap(
    alias: &aviutl2::alias::Table,
    keyframe_index: usize,
    offset_frames: i64,
) -> anyhow::Result<aviutl2::alias::Table> {
    let mut frames: Vec<usize> = alias
        .get_table("Object")
        .context("Object table not found")?
        .parse_value("frame")
        .context("frame column not found")??;
    frames[keyframe_index + 1] = (frames[keyframe_index + 1] as i64 - offset_frames) as usize;
    let mut new_alias = alias.clone();
    new_alias
        .get_table_mut("Object")
        .context("Object table not found")?
        .insert_value(
            "frame",
            frames
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );

    Ok(new_alias)
}
