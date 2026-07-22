struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) layer: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.tex_coords = input.tex_coords;
    output.color = input.color;
    return output;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Nearest-neighbor sampling for pixel art
    let tex_color = textureSample(sprite_texture, sprite_sampler, input.tex_coords);
    let final_color = tex_color * input.color;

    // Alpha discard for pixel-perfect edges
    if final_color.a < 0.01 {
        discard;
    }

    return final_color;
}