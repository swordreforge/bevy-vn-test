use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;
use bevy::ui_render::prelude::*;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ViewMaskMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub name_texture: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub mask_texture: Handle<Image>,
    #[uniform(4)]
    pub progress: f32,
    #[uniform(5)]
    pub name_left: f32,
    #[uniform(6)]
    pub name_top: f32,
}

impl UiMaterial for ViewMaskMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/view_mask.wgsl".into()
    }
}
