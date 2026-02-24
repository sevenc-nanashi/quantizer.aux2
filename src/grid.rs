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

pub fn beat_to_frame(info: &aviutl2::generic::EditInfo, beat_num: f64) -> f64 {
    second_to_frame(info, beat_to_second(info, beat_num))
}

pub fn beat_to_frame_int(info: &aviutl2::generic::EditInfo, beat_num: f64) -> i32 {
    // rounding is upward.
    beat_to_frame(info, beat_num).ceil() as i32
}

pub fn frame_to_beat(info: &aviutl2::generic::EditInfo, frame_num: f64) -> f64 {
    second_to_beat(info, frame_to_second(info, frame_num))
}

pub fn beat_to_second(info: &aviutl2::generic::EditInfo, beat_num: f64) -> f64 {
    60.0 * beat_num / (info.grid_bpm_tempo as f64) + (info.grid_bpm_offset as f64)
}

pub fn second_to_beat(info: &aviutl2::generic::EditInfo, second_num: f64) -> f64 {
    (second_num - (info.grid_bpm_offset as f64)) * (info.grid_bpm_tempo as f64) / 60.0
}
