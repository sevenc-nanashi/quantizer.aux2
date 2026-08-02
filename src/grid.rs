use aviutl2::anyhow::{self, Context};

// Taken from https://github.com/sigma-axis/aviutl2_tl_walkaround2/blob/22a78b355145ed07efd5a37373a5bcd1b8075b66/tl_walkaround2.cpp#L1367
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

struct BpmGridCalc {
    tempo: f64,
    offset: f64,
    rate: f64,
    scale: f64,
}

impl BpmGridCalc {
    fn new(bpm: aviutl2::generic::BpmInfo, rate: f64, scale: f64) -> Self {
        Self {
            tempo: bpm.tempo as f64,
            offset: bpm.start + bpm.offset as f64,
            rate,
            scale,
        }
    }

    fn beat_to_frame(&self, beat_num: f64) -> f64 {
        (60.0 * beat_num + self.tempo * self.offset) * self.rate / (self.tempo * self.scale)
    }

    fn frame_to_beat(&self, frame_num: f64) -> f64 {
        (self.tempo * self.scale * frame_num - self.tempo * self.rate * self.offset)
            / (60.0 * self.rate)
    }
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
    nearest_grid_frame_at_rate(
        bpm_list,
        frame_num,
        *info.fps.numer() as f64,
        *info.fps.denom() as f64,
    )
}

fn nearest_grid_frame_at_rate(
    bpm_list: &[aviutl2::generic::BpmInfo],
    frame_num: f64,
    rate: f64,
    scale: f64,
) -> anyhow::Result<i32> {
    if bpm_list.is_empty() {
        anyhow::bail!("BPM grid is empty");
    }
    if let Some(bpm) = bpm_list.iter().find(|bpm| bpm.tempo <= 0.0) {
        anyhow::bail!("BPM tempo must be positive: {}", bpm.tempo);
    }

    let mut bpm_list = bpm_list.to_vec();
    bpm_list.sort_by(|left, right| left.start.total_cmp(&right.start));

    let mut nearest_frame = None;
    for (index, bpm) in bpm_list.iter().enumerate() {
        let start_frame = (bpm.start * rate / scale).ceil();
        let end_frame = bpm_list
            .get(index + 1)
            .map(|next| (next.start * rate / scale).ceil());
        let bpm_calc = BpmGridCalc::new(*bpm, rate, scale);
        let current_beat = bpm_calc.frame_to_beat(frame_num);
        for beat in [current_beat.floor(), current_beat.ceil()] {
            let candidate = bpm_calc.beat_to_frame(beat).ceil();
            if candidate < start_frame {
                continue;
            }
            if let Some(end_frame) = end_frame
                && candidate >= end_frame
            {
                continue;
            }
            let distance = (candidate - frame_num).abs();
            nearest_frame = Some(match nearest_frame {
                Some((nearest, nearest_distance)) if nearest_distance <= distance => {
                    (nearest, nearest_distance)
                }
                _ => (candidate, distance),
            });
        }
    }

    let nearest_frame = nearest_frame
        .map(|(frame, _)| frame as i32)
        .context("No BPM grid candidate found")?;
    Ok(nearest_frame)
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
    fn nearest_grid_frame_uses_offset_relative_to_each_bpm_start() {
        let bpm_list = [bpm(120.0, 0.0, 0.0), bpm(60.0, 10.0, 0.25)];

        let nearest = nearest_grid_frame_at_rate(&bpm_list, 11.2 * 30.0, 30.0, 1.0).unwrap();

        assert_eq!(nearest, 338);
    }

    #[test]
    fn nearest_grid_frame_does_not_use_previous_segment_after_next_start() {
        let bpm_list = [bpm(120.0, 0.0, 0.0), bpm(60.0, 10.0, 0.25)];

        let nearest = nearest_grid_frame_at_rate(&bpm_list, 10.1 * 30.0, 30.0, 1.0).unwrap();

        assert_eq!(nearest, 308);
    }

    #[test]
    fn nearest_grid_frame_uses_rounded_bpm_segment_boundary() {
        let bpm_list = [bpm(120.0, 0.0, 0.005), bpm(60.0, 10.01, 0.25)];

        let nearest = nearest_grid_frame_at_rate(&bpm_list, 301.0, 30.0, 1.0).unwrap();

        assert_eq!(nearest, 308);
    }
}
