// Probe (e): custom GPU viewport embed (MAKE-OR-BREAK for Tailor 3D + photo-editor 2D canvas).
// THROW-AWAY render code (per contract: this is a hostability + model-steering proof, NOT a
// real renderer).
//
// What it proves, for real:
//   1. HOSTABILITY: a custom wgpu scene runs inside an egui paint-callback attached to a pane's
//      rect (egui_wgpu::Callback::new_paint_callback). The callback's prepare() actually executes
//      during egui_kittest's render(), encoding a real wgpu render pass (rotating 3D cube WITH a
//      depth buffer + a painted-texture quad) into an offscreen target. We render offscreen with
//      our own target+depth format so we never depend on egui's shared render-pass format.
//   2. PAINTED 2D TEXTURE: a small texture is created and filled (queue.write_texture) from a
//      brush-color uniform, then sampled by the quad.
//   3. MODEL STEERING (RISK-7 / CONTROL-7): a model-dispatched AccessKit action (node.click() on
//      the "rotate-step" button, i.e. the a11y action channel) increments the rotation angle. The
//      uniform write in prepare() also records that exact angle into a CPU mirror IN THE SAME CODE
//      PATH. We read the mirror before/after the action and assert it changed to the model-driven
//      value. The mirror is only reachable when the GPU uniform-write path runs, so a faked link
//      cannot pass.

use egui_kittest::kittest::Queryable;
use egui_wgpu::wgpu;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

pub struct ProbeResult {
    pub pass: bool,
    pub notes: String,
}

const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const TARGET: u32 = 128;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    angle: f32,
    _pad: [f32; 3],
    brush: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

// 8 cube corners with per-corner colors.
const CUBE_VERTS: &[Vertex] = &[
    Vertex { pos: [-0.5, -0.5, -0.5], color: [1.0, 0.2, 0.3] },
    Vertex { pos: [0.5, -0.5, -0.5], color: [0.2, 1.0, 0.3] },
    Vertex { pos: [0.5, 0.5, -0.5], color: [0.3, 0.2, 1.0] },
    Vertex { pos: [-0.5, 0.5, -0.5], color: [1.0, 1.0, 0.2] },
    Vertex { pos: [-0.5, -0.5, 0.5], color: [1.0, 0.2, 1.0] },
    Vertex { pos: [0.5, -0.5, 0.5], color: [0.2, 1.0, 1.0] },
    Vertex { pos: [0.5, 0.5, 0.5], color: [1.0, 0.6, 0.2] },
    Vertex { pos: [-0.5, 0.5, 0.5], color: [0.6, 0.2, 1.0] },
];
const CUBE_INDICES: &[u16] = &[
    0, 1, 2, 0, 2, 3, // back
    4, 6, 5, 4, 7, 6, // front
    0, 4, 5, 0, 5, 1, // bottom
    3, 2, 6, 3, 6, 7, // top
    0, 3, 7, 0, 7, 4, // left
    1, 5, 6, 1, 6, 2, // right
];

const SHADER: &str = r#"
struct Uniforms { angle: f32, _pad0: f32, _pad1: f32, _pad2: f32, brush: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VOut { @builtin(position) clip: vec4<f32>, @location(0) color: vec3<f32> };

@vertex
fn vs_cube(@location(0) pos: vec3<f32>, @location(1) color: vec3<f32>) -> VOut {
    let c = cos(u.angle);
    let s = sin(u.angle);
    // rotate around Y then a fixed tilt around X
    let ry = mat3x3<f32>(vec3<f32>(c, 0.0, -s), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(s, 0.0, c));
    let tilt = 0.5;
    let ct = cos(tilt); let st = sin(tilt);
    let rx = mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, ct, st), vec3<f32>(0.0, -st, ct));
    let p = rx * (ry * pos) * 0.8;
    var o: VOut;
    o.clip = vec4<f32>(p.x, p.y, p.z * 0.5 + 0.5, 1.0);
    o.color = color;
    return o;
}
@fragment
fn fs_cube(in: VOut) -> @location(0) vec4<f32> { return vec4<f32>(in.color, 1.0); }

struct QOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_quad(@builtin(vertex_index) vi: u32) -> QOut {
    // small quad in the lower-right corner
    var pts = array<vec2<f32>, 6>(
        vec2<f32>(0.2, -0.9), vec2<f32>(0.9, -0.9), vec2<f32>(0.9, -0.2),
        vec2<f32>(0.2, -0.9), vec2<f32>(0.9, -0.2), vec2<f32>(0.2, -0.2));
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 0.0));
    var o: QOut;
    o.clip = vec4<f32>(pts[vi], 0.0, 1.0);
    o.uv = uvs[vi];
    return o;
}
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment
fn fs_quad(in: QOut) -> @location(0) vec4<f32> { return textureSample(tex, samp, in.uv); }
"#;

struct RenderResources {
    cube_pipeline: wgpu::RenderPipeline,
    quad_pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    cube_bind: wgpu::BindGroup,
    quad_bind: wgpu::BindGroup,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    offscreen: wgpu::TextureView,
    depth: wgpu::TextureView,
    painted_once: bool,
    painted_tex: wgpu::Texture,
}

impl RenderResources {
    fn new(device: &wgpu::Device) -> Self {
        use wgpu::util::DeviceExt;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("spike_viewport_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spike_uniform"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cube_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cube_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let cube_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cube_bind"),
            layout: &cube_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let cube_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cube_pl"),
            bind_group_layouts: &[&cube_bgl],
            push_constant_ranges: &[],
        });

        let cube_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube_pipeline"),
            layout: Some(&cube_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_cube"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_cube"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: OFFSCREEN_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // painted texture + sampler for the 2D quad
        let painted_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("painted_tex"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let painted_view = painted_tex.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let quad_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quad_bgl"),
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
        let quad_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad_bind"),
            layout: &quad_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&painted_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let quad_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad_pl"),
            bind_group_layouts: &[&quad_bgl],
            push_constant_ranges: &[],
        });
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad_pipeline"),
            layout: Some(&quad_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_quad"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_quad"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: OFFSCREEN_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube_vbuf"),
            contents: bytemuck::cast_slice(CUBE_VERTS),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube_ibuf"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let offscreen = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("offscreen"),
                size: wgpu::Extent3d { width: TARGET, height: TARGET, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OFFSCREEN_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: TARGET, height: TARGET, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());

        Self {
            cube_pipeline,
            quad_pipeline,
            uniform_buf,
            cube_bind,
            quad_bind,
            vbuf,
            ibuf,
            offscreen,
            depth,
            painted_once: false,
            painted_tex,
        }
    }
}

struct ViewportCallback {
    angle: f32,
    brush: [f32; 4],
    written_angle: Arc<AtomicU32>,
    rendered: Arc<AtomicBool>,
}

impl egui_wgpu::CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _sd: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if resources.get::<RenderResources>().is_none() {
            resources.insert(RenderResources::new(device));
        }
        let res: &mut RenderResources = resources.get_mut().unwrap();

        // paint the 2D texture from the brush color (once).
        if !res.painted_once {
            let px = [
                (self.brush[0] * 255.0) as u8,
                (self.brush[1] * 255.0) as u8,
                (self.brush[2] * 255.0) as u8,
                255u8,
            ];
            let data: Vec<u8> = std::iter::repeat(px).take(16).flatten().collect();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &res.painted_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(16), rows_per_image: Some(4) },
                wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            );
            res.painted_once = true;
        }

        // Write the rotation uniform (the model-driven value) AND mirror it in the SAME path.
        let u = Uniforms { angle: self.angle, _pad: [0.0; 3], brush: self.brush };
        queue.write_buffer(&res.uniform_buf, 0, bytemuck::bytes_of(&u));
        self.written_angle.store(self.angle.to_bits(), Ordering::SeqCst);

        // Encode a real offscreen render pass: rotating 3D cube (with depth) + painted-texture quad.
        {
            let mut pass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("spike_offscreen_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res.offscreen,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.05, b: 0.08, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &res.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&res.cube_pipeline);
            pass.set_bind_group(0, &res.cube_bind, &[]);
            pass.set_vertex_buffer(0, res.vbuf.slice(..));
            pass.set_index_buffer(res.ibuf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..CUBE_INDICES.len() as u32, 0, 0..1);
            pass.set_pipeline(&res.quad_pipeline);
            pass.set_bind_group(0, &res.quad_bind, &[]);
            pass.draw(0..6, 0..1);
        }
        self.rendered.store(true, Ordering::SeqCst);
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::epaint::PaintCallbackInfo,
        _render_pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        // No-op: the custom scene is rendered offscreen in prepare(); compositing into the egui
        // pane is deferred to the real implementation. This probe proves hostability + steering.
    }
}

#[derive(Default)]
struct ViewportState {
    angle: f32,
    written: Arc<AtomicU32>,
    rendered: Arc<AtomicBool>,
}

pub fn run() -> ProbeResult {
    let mut harness = egui_kittest::Harness::builder().wgpu().build_ui_state(
        |ui, st: &mut ViewportState| {
            // model-steerable control: a button whose AccessKit Click increments the angle.
            if ui.button("rotate-step").clicked() {
                st.angle += 1.0;
            }
            let (rect, _r) = ui.allocate_exact_size(egui::vec2(128.0, 128.0), egui::Sense::hover());
            let cb = ViewportCallback {
                angle: st.angle,
                brush: [0.9, 0.3, 0.55, 1.0],
                written_angle: st.written.clone(),
                rendered: st.rendered.clone(),
            };
            ui.painter().add(egui_wgpu::Callback::new_paint_callback(rect, cb));
        },
        ViewportState::default(),
    );

    // Frame 1: lay out + render the custom scene (prepare() runs the wgpu pass).
    harness.run();
    let render1 = harness.render();
    let hosted = render1.is_ok();
    let rendered_flag = harness.state().rendered.load(Ordering::SeqCst);
    let before = f32::from_bits(harness.state().written.load(Ordering::SeqCst));

    // Model action through the AccessKit channel: dispatch Click on the button node.
    harness.get_by_label("rotate-step").click();
    harness.run();
    let render2 = harness.render();
    let after = f32::from_bits(harness.state().written.load(Ordering::SeqCst));

    let steered = (after - before).abs() > 0.5 && (after - 1.0).abs() < 1e-3;
    let pass = hosted && rendered_flag && render2.is_ok() && steered;

    ProbeResult {
        pass,
        notes: format!(
            "hosted(render_ok)={} offscreen_pass_submitted={} steering: uniform angle before={} after={} (model AccessKit Click on 'rotate-step'); custom wgpu offscreen pass (3D cube w/depth + painted-texture quad) built + submitted via the harness's real device/queue inside an egui pane paint-callback, no device loss; PIXEL OUTPUT NOT READ BACK -> proves hostability + model-steering, NOT pixel-correctness (readback deferred to MT-002)",
            hosted, rendered_flag, before, after
        ),
    }
}
