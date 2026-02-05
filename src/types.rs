use std::mem::size_of;

use vulkanalia::prelude::v1_0::*;

pub type Vec2 = cgmath::Vector2<f32>;
pub type Vec3 = cgmath::Vector3<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;

#[rustfmt::skip]
pub const RECT: &[f32] = &[
    0., 0., 
    1.1, -1.,
    1.1, 1.,
    -1.1, 1.,
    -1.1, -1.,
];

#[rustfmt::skip]
pub const RECT_INDICES: &[u16] = &[
    0, 1, 2, 
    0, 2, 3,
    0, 3, 4,
    0, 4, 1
];

#[derive(Debug, Default, Clone)]
pub struct Lines(Vec<Line>);

impl Lines {
    fn new_gpu_backed(max_lines_num: usize) -> Self {
        Lines(Vec::with_capacity(max_lines_num))
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn add(&mut self, line: Line) {
        self.0.push(line);
    }

    pub fn extend(&mut self, segments: &Lines) {
        self.0.extend(segments.0.iter())
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Line {
    pub position: Vec2,
    pub dir: Vec2,
}

impl Line {
    pub fn new(from: Vec2, to: Vec2) -> Self {
        let dir = to - from;
        Line {
            position: (from + to) / 2.,
            dir,
        }
    }
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Line>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }
}
