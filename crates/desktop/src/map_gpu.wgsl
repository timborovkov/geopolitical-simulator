struct Camera { transform: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
struct Palette { values: array<vec4<f32>,512> }
@group(0) @binding(1) var<uniform> colors: Palette;
@group(1) @binding(0) var surface: texture_2d<f32>;
@group(1) @binding(1) var filtering: sampler;
struct VertexOutput {
 @builtin(position) position: vec4<f32>,
 @location(0) uv: vec2<f32>,
 @location(1) @interpolate(flat) country: u32,
}
@vertex fn vs(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) country: u32, @builtin(instance_index) copy: u32) -> VertexOutput {
 var o: VertexOutput;
 let wrapped = position + vec2<f32>((f32(copy)-1.0)*360.0,0.0);
 o.position=vec4<f32>(wrapped*camera.transform.xy+camera.transform.zw,0.0,1.0);
 o.uv=uv;o.country=country;return o;
}
@fragment fn fs(v:VertexOutput)->@location(0) vec4<f32> {
 if v.country==0u {
  let tex=textureSample(surface,filtering,v.uv);
  return vec4<f32>(tex.rgb*vec3<f32>(0.78,0.84,0.78),1.0);
 }
 return colors.values[v.country-1u];
}
