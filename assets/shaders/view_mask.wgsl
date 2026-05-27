#define_import_path bevy_vn::view_mask

#import bevy_ui::ui_vertex_output UiVertexOutput

@group(1) @binding(0)
var name_texture: texture_2d<f32>;
@group(1) @binding(1)
var name_sampler: sampler;
@group(1) @binding(2)
var mask_texture: texture_2d<f32>;
@group(1) @binding(3)
var mask_sampler: sampler;
@group(1) @binding(4)
var<uniform> progress: f32;
@group(1) @binding(5)
var<uniform> name_left: f32;
@group(1) @binding(6)
var<uniform> name_top: f32;

const SCREEN_W: f32 = 1280.0;
const SCREEN_H: f32 = 720.0;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let name_color = textureSample(name_texture, name_sampler, in.uv);
    if name_color.a < 0.01 {
        return vec4<f32>(0.0);
    }
    let mask_uv = vec2<f32>(
        (name_left + in.uv.x * in.size.x) / SCREEN_W,
        (name_top + in.uv.y * in.size.y) / SCREEN_H,
    );
    let mask_val = textureSample(mask_texture, mask_sampler, mask_uv).r;
    let threshold = 1.0 - progress;
    let opacity = smoothstep(threshold - 0.01, threshold + 0.01, mask_val);
    return vec4<f32>(name_color.rgb, name_color.a * opacity);
}
