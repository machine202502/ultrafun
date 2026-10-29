struct Face {
    view_projection: mat4x4<f32>,
    light_position_range: vec4<f32>,
}

@group(0) @binding(0) var<uniform> face: Face;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = face.view_projection * vec4<f32>(position, 1.0);
    output.world_position = position;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(input.world_position - face.light_position_range.xyz);
    return vec4<f32>(distance / face.light_position_range.w, 0.0, 0.0, 1.0);
}
