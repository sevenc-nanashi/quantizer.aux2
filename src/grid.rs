use aviutl2::anyhow::{self, Context};

// Taken from https://github.com/sigma-axis/aviutl2_tl_walkaround2/blob/8452576855f3bbc81589fc5affbaf92555ca24ff/tl_walkaround2.cpp#L1013
//
// ```
// MIT License
//
// Copyright (c) 2025 sigma-axis
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// ```

pub fn frame_to_second(info: &aviutl2::generic::EditInfo, frame_num: f64) -> f64 {
    frame_num * *info.fps.denom() as f64 / *info.fps.numer() as f64
}

pub fn second_to_frame(info: &aviutl2::generic::EditInfo, second_num: f64) -> f64 {
    second_num * *info.fps.numer() as f64 / *info.fps.denom() as f64
}

pub fn max_frames_per_beat(
    info: &aviutl2::generic::EditInfo,
    bpm_list: &[aviutl2::generic::BpmInfo],
) -> anyhow::Result<f64> {
    if bpm_list.is_empty() {
        anyhow::bail!("BPM grid is empty");
    }
    if let Some(bpm) = bpm_list.iter().find(|bpm| bpm.tempo <= 0.0) {
        anyhow::bail!("BPM tempo must be positive: {}", bpm.tempo);
    }

    let fps = *info.fps.numer() as f64 / *info.fps.denom() as f64;
    let max_seconds_per_beat = bpm_list
        .iter()
        .map(|bpm| 60.0 / bpm.tempo as f64)
        .max_by(f64::total_cmp)
        .expect("bpm_list is not empty");
    Ok(max_seconds_per_beat * fps)
}

pub fn nearest_grid_frame(
    info: &aviutl2::generic::EditInfo,
    bpm_list: &[aviutl2::generic::BpmInfo],
    frame_num: f64,
) -> anyhow::Result<i32> {
    let nearest_second = nearest_grid_second(bpm_list, frame_to_second(info, frame_num))?;
    Ok(second_to_frame(info, nearest_second).ceil() as i32)
}

fn nearest_grid_second(bpm_list: &[aviutl2::generic::BpmInfo], second: f64) -> anyhow::Result<f64> {
    if bpm_list.is_empty() {
        anyhow::bail!("BPM grid is empty");
    }
    if let Some(bpm) = bpm_list.iter().find(|bpm| bpm.tempo <= 0.0) {
        anyhow::bail!("BPM tempo must be positive: {}", bpm.tempo);
    }

    let mut bpm_list = bpm_list.to_vec();
    bpm_list.sort_by(|left, right| left.start.total_cmp(&right.start));

    let mut nearest_second = None;
    for (index, bpm) in bpm_list.iter().enumerate() {
        let end = bpm_list.get(index + 1).map(|next| next.start);
        let current_beat = second_to_beat(*bpm, second);
        for beat in [current_beat.floor(), current_beat.ceil()] {
            let candidate = beat_to_second(*bpm, beat);
            if candidate < bpm.start {
                continue;
            }
            if let Some(end) = end
                && candidate >= end
            {
                continue;
            }
            let distance = (candidate - second).abs();
            nearest_second = Some(match nearest_second {
                Some((nearest, nearest_distance)) if nearest_distance <= distance => {
                    (nearest, nearest_distance)
                }
                _ => (candidate, distance),
            });
        }
    }

    let nearest_second = nearest_second
        .map(|(second, _)| second)
        .context("No BPM grid candidate found")?;
    Ok(nearest_second)
}

fn beat_to_second(bpm: aviutl2::generic::BpmInfo, beat_num: f64) -> f64 {
    bpm.start + (bpm.offset as f64) + 60.0 * beat_num / (bpm.tempo as f64)
}

fn second_to_beat(bpm: aviutl2::generic::BpmInfo, second_num: f64) -> f64 {
    (second_num - bpm.start - (bpm.offset as f64)) * (bpm.tempo as f64) / 60.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bpm(tempo: f32, start: f64, offset: f32) -> aviutl2::generic::BpmInfo {
        aviutl2::generic::BpmInfo {
            tempo,
            beat: 4,
            start,
            offset,
        }
    }

    #[test]
    fn nearest_grid_second_uses_offset_relative_to_each_bpm_start() {
        let bpm_list = [bpm(120.0, 0.0, 0.0), bpm(60.0, 10.0, 0.25)];

        let nearest = nearest_grid_second(&bpm_list, 11.2).unwrap();

        assert_eq!(nearest, 11.25);
    }

    #[test]
    fn nearest_grid_second_does_not_use_previous_segment_after_next_start() {
        let bpm_list = [bpm(120.0, 0.0, 0.0), bpm(60.0, 10.0, 0.25)];

        let nearest = nearest_grid_second(&bpm_list, 10.1).unwrap();

        assert_eq!(nearest, 10.25);
    }
}
