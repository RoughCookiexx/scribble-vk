use cgmath::{Deg, Matrix4, Vector3, vec3};
use std::time::Instant;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::config::DemoConfig;

pub struct DemoController {
    pub model_count: usize,
    pub max_models: usize,
    pub enable_spawning: bool,
    pub enable_rotation: bool,
    start_time: Instant,
}

impl DemoController {
    pub fn new(config: &DemoConfig) -> Self {
        Self {
            model_count: config.initial_model_count,
            max_models: config.max_models,
            enable_spawning: config.enable_model_spawning,
            enable_rotation: config.enable_rotation,
            start_time: Instant::now(),
        }
    }

    /// Handle keyboard input - returns true if state changed
    pub fn handle_key(&mut self, key: PhysicalKey) -> bool {
        if !self.enable_spawning {
            return false;
        }

        match key {
            PhysicalKey::Code(KeyCode::ArrowLeft) if self.model_count > 1 => {
                self.model_count -= 1;
                true
            }
            PhysicalKey::Code(KeyCode::ArrowRight) if self.model_count < self.max_models => {
                self.model_count += 1;
                true
            }
            _ => false,
        }
    }

    /// Calculate model transforms - pure math, no Vulkan
    pub fn get_model_transforms(&self) -> Vec<ModelTransform> {
        (0..self.model_count)
            .map(|i| {
                let y = (((i % 2) as f32) * 2.5) - 1.25;
                let z = (((i / 2) as f32) * -2.0) + 1.0;

                let rotation = if self.enable_rotation {
                    Deg(90.0) * self.start_time.elapsed().as_secs_f32()
                } else {
                    Deg(0.0)
                };

                let opacity = (i + 1) as f32 * 0.25;

                ModelTransform {
                    position: vec3(0.0, y, z),
                    rotation,
                    opacity,
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct ModelTransform {
    pub position: Vector3<f32>,
    pub rotation: Deg<f32>,
    pub opacity: f32,
}

impl ModelTransform {
    pub fn to_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_translation(self.position)
            * Matrix4::from_axis_angle(vec3(0.0, 0.0, 1.0), self.rotation)
    }
}
