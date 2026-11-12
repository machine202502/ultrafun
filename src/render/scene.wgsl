struct SceneUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    sun_direction: vec4<f32>,
    fog_color_exposure: vec4<f32>,
    fog_ambient_linear: vec4<f32>,
    resolution_snap_dither: vec4<f32>,
    texel_bands_shadow: vec4<f32>,
    ambient_sky_resolution: vec4<f32>,
    ambient_ground_shadows: vec4<f32>,
    bounce_strength: vec4<f32>,
    surface: vec4<f32>,
    atlas_tile_span: vec4<f32>,
}

const MAX_LIGHTS: u32 = 64u;
const MAX_SHADOW_FACES: u32 = 144u;
const GAMMA: f32 = 2.2;
const EMISSIVE_BOOST: f32 = 4.0;
const SHADOW_NORMAL_OFFSET: f32 = 1.5;
const SHADOW_MAX_SLOPE: f32 = 4.0;
const SHADOW_DEPTH_BIAS: f32 = 1.5;
const PANEL_PIXELS: f32 = 16.0;
const SEAM: f32 = 0.65;
const BEVEL_LIGHT: f32 = 1.12;
const BEVEL_DARK: f32 = 0.85;
const PIXEL_NOISE: f32 = 0.24;
const PANEL_NOISE: f32 = 0.2;
const FACE_TOP: f32 = 1.0;
const FACE_BOTTOM: f32 = 0.5;
const FACE_SIDE_X: f32 = 0.8;
const FACE_SIDE_Z: f32 = 0.65;

struct Light {
    position_range: vec4<f32>,
    color_intensity: vec4<f32>,
    shadow: vec4<f32>,
}

struct Lights {
    count: vec4<u32>,
    items: array<Light, MAX_LIGHTS>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var<uniform> lights: Lights;
@group(0) @binding(2) var shadow_maps: texture_2d_array<f32>;
@group(0) @binding(3) var<uniform> shadow_faces: array<mat4x4<f32>, MAX_SHADOW_FACES>;
@group(0) @binding(4) var lightmap: texture_2d<f32>;
@group(0) @binding(5) var lightmap_sampler: sampler;
@group(0) @binding(6) var atlas_texture: texture_2d<f32>;
@group(0) @binding(7) var atlas_sampler: sampler;

const BAYER_4X4: array<f32, 16> = array<f32, 16>(
    0.0, 8.0, 2.0, 10.0,
    12.0, 4.0, 14.0, 6.0,
    3.0, 11.0, 1.0, 9.0,
    15.0, 7.0, 13.0, 5.0,
);

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) lightmap_uv: vec2<f32>,
    @location(4) atlas_uv: vec2<f32>,
    @location(5) atlas_tile: vec4<f32>,
    @location(6) transform_0: vec4<f32>,
    @location(7) transform_1: vec4<f32>,
    @location(8) transform_2: vec4<f32>,
    @location(9) transform_3: vec4<f32>,
    @location(10) normal_0: vec4<f32>,
    @location(11) normal_1: vec4<f32>,
    @location(12) normal_2: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) lightmap_uv: vec2<f32>,
    @location(4) atlas_uv: vec2<f32>,
    @location(5) atlas_tile: vec4<f32>,
}

fn bounced_light(uv: vec2<f32>) -> vec3<f32> {
    if scene.bounce_strength.y < 0.5 || uv.x < 0.0 {
        return vec3<f32>(0.0);
    }
    return textureSampleLevel(lightmap, lightmap_sampler, uv, 0.0).rgb * scene.bounce_strength.x;
}

fn hash2(cell: vec2<f32>) -> f32 {
    return fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.547);
}

fn sample_albedo(atlas_tile: vec4<f32>, atlas_uv: vec2<f32>) -> vec3<f32> {
    let uv = atlas_tile.xy + fract(atlas_uv) * atlas_tile.zw;
    return textureSample(atlas_texture, atlas_sampler, uv).rgb;
}

fn surface_texels(world_position: vec3<f32>, normal: vec3<f32>) -> vec2<f32> {
    let a = abs(normal);
    var uv = world_position.xy;
    if a.x >= a.y && a.x >= a.z {
        uv = world_position.zy;
    } else if a.y >= a.z {
        uv = world_position.xz;
    }
    return uv * scene.surface.y;
}

fn seam_coverage(coordinate: f32, footprint: f32) -> f32 {
    if footprint <= 1.0 {
        let local = floor(coordinate) - floor(coordinate / PANEL_PIXELS) * PANEL_PIXELS;
        return select(0.0, 1.0, local == 0.0);
    }
    let start = coordinate - footprint * 0.5;
    let end = coordinate + footprint * 0.5;
    let seams = floor(end / PANEL_PIXELS) - floor(start / PANEL_PIXELS);
    return clamp(seams / footprint, 0.0, 1.0);
}

fn pixel_texture(texels: vec2<f32>, footprint: vec2<f32>) -> f32 {
    let strength = scene.surface.x;
    if strength <= 0.0 {
        return 1.0;
    }
    let pixel = floor(texels + 0.001);
    let panel = floor(pixel / PANEL_PIXELS);
    let local = pixel - panel * PANEL_PIXELS;
    let sharp = clamp(2.0 - max(footprint.x, footprint.y), 0.0, 1.0);
    var value = 1.0;
    if local.x == 1.0 || local.y == 1.0 {
        value = mix(1.0, BEVEL_LIGHT, sharp);
    } else if local.x == PANEL_PIXELS - 1.0 || local.y == PANEL_PIXELS - 1.0 {
        value = mix(1.0, BEVEL_DARK, sharp);
    }
    let seam = max(seam_coverage(texels.x, footprint.x), seam_coverage(texels.y, footprint.y));
    value = mix(value, SEAM, seam);
    let cell = exp2(max(ceil(log2(max(footprint, vec2<f32>(1.0)))), vec2<f32>(0.0)));
    let noise = hash2(floor(pixel / cell) * cell) - 0.5;
    value *= 1.0 + PIXEL_NOISE * noise / sqrt(cell.x * cell.y);
    let panel_noise = hash2(panel + vec2<f32>(17.0, 31.0)) - 0.5;
    let panel_cell = max(footprint.x, footprint.y) / PANEL_PIXELS;
    value *= 1.0 + PANEL_NOISE * panel_noise * clamp(2.0 - panel_cell, 0.0, 1.0);
    return mix(1.0, value, strength);
}

fn face_shade(normal: vec3<f32>) -> f32 {
    let vertical = select(FACE_BOTTOM, FACE_TOP, normal.y > 0.0);
    let shade = normal.y * normal.y * vertical + normal.x * normal.x * FACE_SIDE_X + normal.z * normal.z * FACE_SIDE_Z;
    return mix(1.0, shade, scene.surface.z);
}

fn saturate_color(color: vec3<f32>) -> vec3<f32> {
    let luma = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    return max(mix(vec3<f32>(luma), color, scene.surface.w), vec3<f32>(0.0));
}

fn snap_to_pixels(clip: vec4<f32>) -> vec4<f32> {
    if scene.resolution_snap_dither.z < 0.5 || clip.w <= 0.0 {
        return clip;
    }
    let half_resolution = scene.resolution_snap_dither.xy * 0.5;
    let snapped = round(clip.xy / clip.w * half_resolution) / half_resolution;
    return vec4<f32>(snapped * clip.w, clip.zw);
}

fn quantize(color: vec3<f32>, pixel: vec2<f32>) -> vec3<f32> {
    if scene.resolution_snap_dither.w < 0.5 {
        return color;
    }
    let cell = vec2<u32>(pixel) % vec2<u32>(4u, 4u);
    var bayer = BAYER_4X4;
    let threshold = (bayer[cell.y * 4u + cell.x] + 0.5) / 16.0;
    let levels = scene.resolution_snap_dither.w;
    return floor(color * levels + threshold) / levels;
}

fn to_linear(color: vec3<f32>) -> vec3<f32> {
    return pow(max(color, vec3<f32>(0.0)), vec3<f32>(GAMMA));
}

fn to_display(color: vec3<f32>) -> vec3<f32> {
    return pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / GAMMA));
}

fn tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = color * (2.51 * color + 0.03);
    let b = color * (2.43 * color + 0.59) + 0.14;
    return clamp(a / b, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn snap_to_light_texel(world_position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let texel = scene.texel_bands_shadow.x;
    if texel <= 0.0 {
        return world_position;
    }
    let snapped = (floor(world_position / texel) + 0.5) * texel;
    return snapped - normal * dot(snapped - world_position, normal);
}

fn band(light: vec3<f32>) -> vec3<f32> {
    let bands = scene.texel_bands_shadow.y;
    if bands < 1.0 {
        return light;
    }
    return floor(light * bands + 0.5) / bands;
}

fn cube_face(direction: vec3<f32>) -> u32 {
    let a = abs(direction);
    if a.x >= a.y && a.x >= a.z {
        return select(1u, 0u, direction.x > 0.0);
    }
    if a.y >= a.z {
        return select(3u, 2u, direction.y > 0.0);
    }
    return select(5u, 4u, direction.z > 0.0);
}

fn stored_distance(layer: u32, texel: vec2<i32>, resolution: i32) -> f32 {
    let clamped = clamp(texel, vec2<i32>(0), vec2<i32>(resolution - 1));
    return textureLoad(shadow_maps, clamped, i32(layer), 0).r;
}

fn shadow_visibility(light: Light, world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let index = light.shadow.x;
    if index < 0.0 || scene.ambient_ground_shadows.w < 0.5 {
        return 1.0;
    }
    let light_position = light.position_range.xyz;
    let range = light.position_range.w;
    let resolution = i32(scene.ambient_sky_resolution.w);
    let texel_world = 2.0 * length(world_position - light_position) / f32(resolution);
    let cos_angle = clamp(dot(normal, normalize(light_position - world_position)), 0.05, 1.0);
    let slope = min(sqrt(1.0 - cos_angle * cos_angle) / cos_angle, SHADOW_MAX_SLOPE);
    let sample_position = world_position + normal * max(texel_world * (SHADOW_NORMAL_OFFSET + slope), 0.03);
    let to_sample = sample_position - light_position;
    let layer = u32(index) * 6u + cube_face(to_sample);
    let clip = shadow_faces[layer] * vec4<f32>(sample_position, 1.0);
    let ndc = clip.xy / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let center = vec2<i32>(floor(uv * f32(resolution)));
    let distance = length(to_sample) / range - texel_world * SHADOW_DEPTH_BIAS / range;

    var lit = 0.0;
    var samples = 0.0;
    let radius = select(0, 1, scene.texel_bands_shadow.w > 0.5);
    for (var y = -radius; y <= radius; y++) {
        for (var x = -radius; x <= radius; x++) {
            let stored = stored_distance(layer, center + vec2<i32>(x, y), resolution);
            lit += select(0.0, 1.0, distance <= stored);
            samples += 1.0;
        }
    }
    let visibility = lit / samples;
    let strength = scene.texel_bands_shadow.z;
    return mix(1.0 - strength, 1.0, visibility);
}

fn point_lights(world_position: vec3<f32>, normal: vec3<f32>, linear: bool) -> vec3<f32> {
    var total = vec3<f32>(0.0);
    let count = min(lights.count.x, MAX_LIGHTS);
    for (var index = 0u; index < count; index++) {
        let light = lights.items[index];
        let to_light = light.position_range.xyz - world_position;
        let distance = length(to_light);
        let range = light.position_range.w;
        if distance >= range {
            continue;
        }
        let facing = max(dot(normal, to_light / max(distance, 0.001)), 0.0);
        if facing <= 0.0 {
            continue;
        }
        let edge = clamp(1.0 - pow(distance / range, 4.0), 0.0, 1.0);
        let falloff = edge * edge / (distance * distance + 1.0);
        var color = light.color_intensity.rgb;
        if linear {
            color = to_linear(color);
        }
        let visibility = shadow_visibility(light, world_position, normal);
        total += color * light.color_intensity.a * facing * falloff * visibility;
    }
    return total;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let transform = mat4x4<f32>(input.transform_0, input.transform_1, input.transform_2, input.transform_3);
    let normal = mat3x3<f32>(input.normal_0.xyz, input.normal_1.xyz, input.normal_2.xyz);
    let position = transform * vec4<f32>(input.position, 1.0);
    output.clip_position = snap_to_pixels(scene.view_projection * position);
    output.world_position = position.xyz;
    output.normal = normalize(normal * input.normal);
    output.color = input.color;
    output.lightmap_uv = input.lightmap_uv;
    output.atlas_uv = input.atlas_uv;
    output.atlas_tile = input.atlas_tile;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let linear = scene.fog_ambient_linear.w > 0.5;
    let fog_start = scene.fog_ambient_linear.x;
    let fog_end = scene.fog_ambient_linear.y;
    let ambient_level = scene.fog_ambient_linear.z;
    let distance = length(input.world_position - scene.camera_position.xyz);
    let fog = clamp((distance - fog_start) / (fog_end - fog_start), 0.0, 1.0);
    let fog_color = scene.fog_color_exposure.rgb;
    let texels = surface_texels(input.world_position, input.normal);
    let texel_footprint = max(abs(dpdx(texels)), abs(dpdy(texels)));

    var lit: vec3<f32>;
    if input.color.a > 0.5 {
        lit = input.color.rgb * sample_albedo(input.atlas_tile, input.atlas_uv);
        if linear {
            lit = to_display(tonemap(to_linear(lit) * EMISSIVE_BOOST * scene.fog_color_exposure.w));
        }
    } else {
        let normal = normalize(input.normal);
        var base = input.color.rgb * sample_albedo(input.atlas_tile, input.atlas_uv);
        var sky = scene.ambient_sky_resolution.rgb;
        var ground = scene.ambient_ground_shadows.rgb;
        if linear {
            base = to_linear(base);
            sky = to_linear(sky);
            ground = to_linear(ground);
        }
        let ambient = mix(ground, sky, normal.y * 0.5 + 0.5) * ambient_level;
        let sunlight = max(dot(normal, -scene.sun_direction.xyz), 0.0) * scene.sun_direction.w;
        let light_position = snap_to_light_texel(input.world_position, normal);
        let direct = vec3<f32>(sunlight) + point_lights(light_position, normal, linear);
        lit = base * (ambient + band(direct) + bounced_light(input.lightmap_uv)) * face_shade(normal);
        if linear {
            lit = to_display(tonemap(lit * scene.fog_color_exposure.w));
        }
    }
    let color = mix(saturate_color(lit), fog_color, fog);
    return vec4<f32>(quantize(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), input.clip_position.xy), 1.0);
}
