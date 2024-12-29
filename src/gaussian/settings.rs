use bevy::prelude::*;
use bevy_args::{
    Deserialize,
    Serialize,
    ValueEnum,
};

use crate::sort::SortMode;


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Reflect,
)]
pub enum DrawMode {
    #[default]
    All,
    Selected,
    HighlightSelected,
}


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Reflect,
    Serialize,
    Deserialize,
    ValueEnum,
)]
pub enum GaussianMode {
    Gaussian2d,
    #[default]
    Gaussian3d,
    Gaussian4d,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Reflect,
)]
pub enum PlaybackMode {
    #[default]
    Forward,
    Reverse,
    Still,
}


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Reflect,
)]
pub enum RasterizeMode {
    #[default]
    Color,
    Depth,
    Normal,
}


// TODO: breakdown into components
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct CloudSettings {
    pub aabb: bool,
    pub global_opacity: f32,
    pub global_scale: f32,
    pub opacity_adaptive_radius: bool,
    pub visualize_bounding_box: bool,
    pub sort_mode: SortMode,
    pub draw_mode: DrawMode,
    pub gaussian_mode: GaussianMode,
    pub playback_mode: PlaybackMode,
    pub rasterize_mode: RasterizeMode,
    pub time: f32,
    pub time_scale: f32,
    pub time_start: f32,
    pub time_stop: f32,
}

impl Default for CloudSettings {
    fn default() -> Self {
        Self {
            aabb: false,
            global_opacity: 1.0,
            global_scale: 1.0,
            opacity_adaptive_radius: true,
            visualize_bounding_box: false,
            sort_mode: SortMode::default(),
            draw_mode: DrawMode::default(),
            gaussian_mode: GaussianMode::default(),
            rasterize_mode: RasterizeMode::default(),
            playback_mode: PlaybackMode::default(),
            time: 0.0,
            time_scale: 1.0,
            time_start: 0.0,
            time_stop: 1.0,
        }
    }
}
