// This shader gives blue color to all particles

struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> matrices: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

// @builtin takes into account the aspect ratio of your monitor

// Vertex shader

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = matrices.proj * matrices.view * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 1.0, 1.0);
}

// The @location(0) bit tells WGPU to store the vec4 value returned by 
// this function in the first color target. We'll get into what this is later.
