use anyhow::Result;
use log::*;
use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

#[derive(Debug, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
    pub vulkan: VulkanConfig,
    pub shaders: ShaderConfig,
}

#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct VulkanConfig {
    pub validation_enabled: bool,
    pub max_frames_in_flight: usize,
    pub max_vertices: u32,
    pub staging_buffer_vertex_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct ShaderConfig {
    pub vertex: PathBuf,
    pub fragment: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_str = std::fs::read_to_string("config.toml").unwrap_or_else(|_| {
            warn!("config.toml not found, using embedded defaults");
            DEFAULT_CONFIG.to_string()
        });

        Ok(toml::from_str(&config_str)?)
    }
}
