//! Persistent GPU geometry and bounded, asynchronously decoded terrain tiles.
use crate::reference_atlas::{Atlas, Manifest, TileKey, checked_read};
use eframe::{egui, egui_wgpu};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    country: u32,
}
struct Geometry {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    sections: Vec<(std::ops::Range<u32>, [f32; 4])>,
}
impl Geometry {
    fn new(device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        Self {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Atlas vertices"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Atlas indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            count: indices.len() as u32,
            sections: Vec::new(),
        }
    }
    fn draw_visible(&self, pass: &mut wgpu::RenderPass<'static>, transform: [f32; 4]) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        for copy in 0..3 {
            let shift = (copy as f32 - 1.0) * 360.0;
            let mut batch: Option<std::ops::Range<u32>> = None;
            for (range, bounds) in &self.sections {
                let x0 = (bounds[0] + shift) * transform[0] + transform[2];
                let x1 = (bounds[2] + shift) * transform[0] + transform[2];
                let y0 = bounds[1] * transform[1] + transform[3];
                let y1 = bounds[3] * transform[1] + transform[3];
                if x1 < -1.0 || x0 > 1.0 || y1 < -1.0 || y0 > 1.0 {
                    continue;
                }
                if let Some(pending) = &mut batch
                    && pending.end == range.start
                {
                    pending.end = range.end;
                } else {
                    if let Some(pending) = batch.take() {
                        pass.draw_indexed(pending, 0, copy..copy + 1);
                    }
                    batch = Some(range.clone());
                }
            }
            if let Some(pending) = batch {
                pass.draw_indexed(pending, 0, copy..copy + 1);
            }
        }
    }
    fn draw(&self, pass: &mut wgpu::RenderPass<'static>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.count, 0, 0..3);
    }
}
struct Tile {
    bind: wgpu::BindGroup,
    geometry: Geometry,
    used: u64,
}
struct Decoded {
    key: TileKey,
    result: Result<Vec<image::RgbaImage>, String>,
}
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    colors: wgpu::Buffer,
    bind: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::BindGroup,
    geometry: Vec<Geometry>,
    tiles: BTreeMap<TileKey, Tile>,
    pending: BTreeSet<TileKey>,
    failed: BTreeSet<TileKey>,
    requests: mpsc::SyncSender<TileKey>,
    decoded: Mutex<mpsc::Receiver<Decoded>>,
    tick: u64,
    pub diagnostic: Arc<Mutex<Option<String>>>,
}
fn texture_bind(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    levels: &[image::RgbaImage],
) -> wgpu::BindGroup {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Atlas tile"),
        size: wgpu::Extent3d {
            width: levels[0].width(),
            height: levels[0].height(),
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (mip, img) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            img.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(img.width() * 4),
                rows_per_image: Some(img.height()),
            },
            wgpu::Extent3d {
                width: img.width(),
                height: img.height(),
                depth_or_array_layers: 1,
            },
        );
    }
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Atlas texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&Default::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
impl Renderer {
    pub fn install(
        state: &egui_wgpu::RenderState,
        ctx: egui::Context,
        atlas: &Atlas,
        manifest: Arc<Manifest>,
        root: PathBuf,
    ) -> Arc<Mutex<Option<String>>> {
        let device = &state.device;
        if std::env::var_os("STATECRAFT_MAP_BENCHMARK").is_some() {
            eprintln!("MAP_ADAPTER {:?}", state.adapter.get_info());
        }
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Atlas camera"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let colors = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Atlas country colors"),
            size: 512 * 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: colors.as_entire_binding(),
                },
            ],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let white = texture_bind(
            device,
            &state.queue,
            &texture_layout,
            &sampler,
            &[image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4]))],
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Atlas shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("map_gpu.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout, &texture_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Reference atlas"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Uint32],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let geometry = (0..3)
            .map(|lod| {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let mut sections = Vec::new();
                for (i, c) in atlas.countries.iter().enumerate() {
                    for mesh in &c.lods[lod] {
                        let base = vertices.len() as u32;
                        vertices.extend(mesh.points.iter().map(|p| Vertex {
                            position: *p,
                            uv: [0.0; 2],
                            country: i as u32 + 1,
                        }));
                        let start = indices.len() as u32;
                        indices.extend(mesh.triangles.iter().map(|n| base + n));
                        sections.push((start..indices.len() as u32, mesh.bounds));
                    }
                }
                let mut geometry = Geometry::new(device, &vertices, &indices);
                geometry.sections = sections;
                geometry
            })
            .collect();
        let (requests, rx) = mpsc::sync_channel::<TileKey>(64);
        let (tx, decoded) = mpsc::sync_channel(4);
        std::thread::spawn(move || {
            while let Ok(key) = rx.recv() {
                let result = checked_read(&root, &key.file(), &manifest)
                    .and_then(|b| {
                        image::load_from_memory_with_format(&b, image::ImageFormat::Jpeg)
                            .map_err(|e| e.to_string())
                    })
                    .and_then(|img| {
                        if img.width() != 514 || img.height() != 514 {
                            return Err("Invalid terrain tile size".into());
                        }
                        let mut levels = vec![img.into_rgba8()];
                        while levels.last().unwrap().width() > 1 {
                            let last = levels.last().unwrap();
                            levels.push(image::imageops::resize(
                                last,
                                (last.width() / 2).max(1),
                                (last.height() / 2).max(1),
                                image::imageops::FilterType::Triangle,
                            ));
                        }
                        Ok(levels)
                    });
                if tx.send(Decoded { key, result }).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        let diagnostic = Arc::new(Mutex::new(None));
        state.renderer.write().callback_resources.insert(Self {
            pipeline,
            uniform,
            colors,
            bind,
            texture_layout,
            sampler,
            white,
            geometry,
            tiles: BTreeMap::new(),
            pending: BTreeSet::new(),
            failed: BTreeSet::new(),
            requests,
            decoded: Mutex::new(decoded),
            tick: 0,
            diagnostic: diagnostic.clone(),
        });
        diagnostic
    }
}
pub struct Draw {
    pub transform: [f32; 4],
    pub colors: Vec<[f32; 4]>,
    pub lod: usize,
    pub tiles: Vec<TileKey>,
}
impl egui_wgpu::CallbackTrait for Draw {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _: &egui_wgpu::ScreenDescriptor,
        _: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(r) = resources.get_mut::<Renderer>() else {
            return vec![];
        };
        r.tick += 1;
        queue.write_buffer(&r.uniform, 0, bytemuck::cast_slice(&self.transform));
        queue.write_buffer(&r.colors, 0, bytemuck::cast_slice(&self.colors));
        // 88 × 514² × 4 × 4/3 < 128 MiB. Bounded decoded channel + worker/upload < 10 MiB.
        while let Ok(decoded) = r.decoded.lock().unwrap().try_recv() {
            r.pending.remove(&decoded.key);
            match decoded.result {
                Ok(levels) => {
                    if r.tiles.len() >= 88
                        && let Some(key) = r
                            .tiles
                            .iter()
                            .filter(|(k, _)| k.level > 0)
                            .min_by_key(|(_, v)| v.used)
                            .map(|(k, _)| *k)
                    {
                        r.tiles.remove(&key);
                    }
                    let [x0, y0, x1, y1] = decoded.key.bounds();
                    let a = 1.0 / 514.0;
                    let b = 513.0 / 514.0;
                    let vertices = [
                        Vertex {
                            position: [x0, y1],
                            uv: [a, a],
                            country: 0,
                        },
                        Vertex {
                            position: [x1, y1],
                            uv: [b, a],
                            country: 0,
                        },
                        Vertex {
                            position: [x1, y0],
                            uv: [b, b],
                            country: 0,
                        },
                        Vertex {
                            position: [x0, y0],
                            uv: [a, b],
                            country: 0,
                        },
                    ];
                    r.tiles.insert(
                        decoded.key,
                        Tile {
                            bind: texture_bind(
                                device,
                                queue,
                                &r.texture_layout,
                                &r.sampler,
                                &levels,
                            ),
                            geometry: Geometry::new(device, &vertices, &[0, 1, 2, 0, 2, 3]),
                            used: r.tick,
                        },
                    );
                }
                Err(e) => {
                    r.failed.insert(decoded.key);
                    *r.diagnostic.lock().unwrap() = Some(e);
                }
            }
        }
        for key in &self.tiles {
            if let Some(tile) = r.tiles.get_mut(key) {
                tile.used = r.tick;
            } else if !r.pending.contains(key)
                && !r.failed.contains(key)
                && r.requests.try_send(*key).is_ok()
            {
                r.pending.insert(*key);
            }
        }
        if r.tick.is_multiple_of(60) && std::env::var_os("STATECRAFT_MAP_BENCHMARK").is_some() {
            eprintln!(
                "MAP_CACHE frame={} resident_tiles={} image_bytes_upper_bound={} pending={}",
                r.tick,
                r.tiles.len(),
                r.tiles.len() * 514 * 514 * 4 * 4 / 3,
                r.pending.len()
            );
        }
        vec![]
    }
    fn paint(
        &self,
        _: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(r) = resources.get::<Renderer>() else {
            return;
        };
        pass.set_pipeline(&r.pipeline);
        pass.set_bind_group(0, &r.bind, &[]);
        for key in &self.tiles {
            // Do not shade an entire parent beneath fully resident visible children.
            // A partial/missing child retains its parent as a seamless fallback.
            let children: Vec<_> = self
                .tiles
                .iter()
                .filter(|child| {
                    child.level == key.level + 1 && child.x / 2 == key.x && child.y / 2 == key.y
                })
                .collect();
            if !children.is_empty() && children.iter().all(|child| r.tiles.contains_key(child)) {
                continue;
            }
            if let Some(tile) = r.tiles.get(key) {
                pass.set_bind_group(1, &tile.bind, &[]);
                tile.geometry.draw(pass);
            }
        }
        pass.set_bind_group(1, &r.white, &[]);
        r.geometry[self.lod].draw_visible(pass, self.transform);
    }
}
