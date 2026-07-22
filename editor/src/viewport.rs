use anyhow::Result;
use bevy_ecs::prelude::*;
use pixgine_engine::core::VirtualResolution;
use pixgine_engine::ecs::*;
use pixgine_engine::ecs::{Transform, Sprite, Physics, Animation, Velocity, ParticleEmitter, Particle, AudioSource, Parent, Children, CameraTag};
use pixgine_engine::input::InputManager;
use pixgine_engine::scene::{Scene, spawn_scene, serialize_world, SceneDescriptor, TilemapData, TileLayerData};
use pixgine_engine::physics::PhysicsWorld;
use rapier2d::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use egui_wgpu::Renderer;

const VS: &str = r#"
struct VI { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, };
struct VO { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) color: vec4<f32>, };
struct Cam { matrix: mat4x4<f32>, };
@group(0) @binding(0) var<uniform> cam: Cam;
@vertex fn vmain(v: VI) -> VO {
    var o: VO; o.clip = cam.matrix * vec4<f32>(v.pos, 0.0, 1.0); o.uv = v.uv; o.color = v.color; return o;
}"#;

const FS: &str = r#"
struct VO { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) color: vec4<f32>, };
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@fragment fn fmain(f: VO) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, f.uv) * f.color;
    if c.a < 0.01 { discard; } return c;
}"#;

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SV { pos: [f32; 2], uv: [f32; 2], color: [f32; 4], layer: f32 }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TransformMode { Move, Rotate, Scale }

// Axis handle for gizmo dragging
#[derive(Clone, Copy, PartialEq)]
pub enum GizmoAxis { None, X, Y, XY, Rot }

pub struct VP {
    pub world: World, pub schedule: Schedule,
    pub rt: Option<wgpu::Texture>, pub rtv: Option<wgpu::TextureView>, pub tex_size: (u32, u32),
    pub selected: Option<Entity>, pub scene_path: Option<PathBuf>, pub playing: bool,
    pub entities: Vec<EI>,
    pub ren_ctx: Option<RenCtx>,
    pub texture_map: HashMap<u64, String>,
    pipeline: wgpu::RenderPipeline, ubuf: wgpu::Buffer, bg: wgpu::BindGroup,
    sampler: wgpu::Sampler, bgl: wgpu::BindGroupLayout,
    pub textures: HashMap<u64, (wgpu::TextureView, u32, u32, Option<egui::TextureId>)>, next_tex: u64, fallback_view: wgpu::TextureView,
    pub egui_renderer: Option<Arc<egui::mutex::RwLock<Renderer>>>,
    /// Per-texture spritesheet slicing info (independent of tilemap)
    pub spritesheet_info: HashMap<u64, SpritesheetInfo>,
    pub tile_layers: Vec<TL>, pub sel_tile: usize, pub palette: Vec<(f32,f32,f32,f32)>,
    pub anim_frame: usize, pub anim_timer: f32, pub scripts: Vec<String>,
    pub sel_tex_id: Option<u64>,
    // viewport camera
    pub view_scale: f32, pub view_offset: (f32, f32),
    pub ren_dev_queue: Option<(wgpu::Device, wgpu::Queue)>,
    // transform mode
    pub transform_mode: TransformMode,
    // gizmo dragging
    pub gizmo_axis: GizmoAxis,
    pub gizmo_drag_start_world: Option<(f32, f32)>,
    pub gizmo_drag_start_value: Option<(f32, f32, f32)>,
    // png thumbnails
    pub png_thumbnails: HashMap<String, egui::ColorImage>,
    // spritesheet tile support  
    pub tilesheet_tex_id: Option<u64>,
    pub tilesheet_cols: u32, pub tilesheet_rows: u32,
    pub tilesheet_tile_w: u32, pub tilesheet_tile_h: u32,
    pub tilesheet_path: Option<PathBuf>,
    // physics
    pub physics: PhysicsWorld,
    pub physics_initialized: bool,
    // tilemap painting
    pub painting_tile: bool,
    // lua console
    pub lua_log: Vec<String>,
    // copy/paste
    pub clipboard_entity: Option<bevy_ecs::entity::Entity>,
    // undo/redo
    pub undo_stack: Vec<UndoState>,
    pub redo_stack: Vec<UndoState>,
    pub max_undo: usize,
    // entity name map for persistence
    pub entity_names: HashMap<Entity, String>,
    // egui texture for tilesheet
    pub tilesheet_egui_tex: Option<egui::TextureId>,
    // audio system
    pub audio_manager: Option<pixgine_engine::audio::AudioManager>,
    // drag-and-drop state
    pub drag_tex_id: Option<u64>,
    pub drag_tex_name: Option<String>,
    // build/export
    pub export_path: Option<PathBuf>,
}

/// Snapshot of entire world state for undo/redo
#[derive(Clone)]
pub struct UndoState {
    pub entity_data: Vec<(String, String, serde_json::Value)>, // (name, comp_type, data)
    pub tile_layers: Vec<TL>,
    pub tilesheet_tex_id: Option<u64>,
    pub tilesheet_cols: u32, pub tilesheet_rows: u32,
    pub tilesheet_tile_w: u32, pub tilesheet_tile_h: u32,
    pub palette: Vec<(f32,f32,f32,f32)>,
}

pub struct RenCtx {
    pub dev: wgpu::Device,
    pub queue: wgpu::Queue,
}

#[derive(Clone)] pub struct EI { pub entity: Entity, pub name: String }
#[derive(Clone)] pub struct TL { pub name: String, pub vis: bool, pub tiles: Vec<Vec<u32>>, pub cols: usize, pub rows: usize, pub ts: u32, pub z_index: i32, pub spritesheet_tex_id: Option<u64> }

/// Per-texture spritesheet slicing info
#[derive(Debug, Clone)]
pub struct SpritesheetInfo {
    pub cols: u32,
    pub rows: u32,
    pub tile_w: u32,
    pub tile_h: u32,
    pub tex_w: u32,
    pub tex_h: u32,
}

impl SpritesheetInfo {
    pub fn new(tex_w: u32, tex_h: u32, tile_w: u32, tile_h: u32) -> Self {
        Self {
            cols: if tile_w > 0 { tex_w / tile_w } else { 1 },
            rows: if tile_h > 0 { tex_h / tile_h } else { 1 },
            tile_w,
            tile_h,
            tex_w,
            tex_h,
        }
    }
    pub fn tile_count(&self) -> u32 { self.cols * self.rows }
    pub fn uv_for_tile(&self, index: u32) -> (f32, f32, f32, f32) {
        let col = index % self.cols;
        let row = index / self.cols;
        let u0 = col as f32 / self.cols as f32;
        let v0 = row as f32 / self.rows as f32;
        let u1 = (col + 1) as f32 / self.cols as f32;
        let v1 = (row + 1) as f32 / self.rows as f32;
        (u0, v0, u1, v1)
    }
}

/// Auto-detect tile size: find largest power-of-2 factor that divides both dimensions
pub fn auto_tile_size(tw: u32, th: u32) -> u32 {
    for &size in &[64, 32, 16, 8] {
        if tw >= size && th >= size && tw % size == 0 && th % size == 0 {
            return size;
        }
    }
    16.min(tw).min(th)
}

impl VP {
    pub fn new(dev: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let vr = VirtualResolution::default();
        let rt = dev.create_texture(&wgpu::TextureDescriptor {
            label: Some("vp_rt"), size: wgpu::Extent3d { width: vr.width, height: vr.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[],
        });
        let rtv = rt.create_view(&wgpu::TextureViewDescriptor::default());

        let vsm = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(VS.into()) });
        let fsm = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(FS.into()) });

        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor { label: None, size: 64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let ft = dev.create_texture(&wgpu::TextureDescriptor {
            label: None, size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &ft, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[255u8,255,255,255], wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        let fv = ft.create_view(&wgpu::TextureViewDescriptor::default());
        let sam = dev.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, mipmap_filter: wgpu::FilterMode::Nearest, ..Default::default() });

        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ]});
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sam) },
        ]});
        let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bgl], push_constant_ranges: &[] });
        let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&pl),
            vertex: wgpu::VertexState { module: &vsm, entry_point: Some("vmain"), buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SV>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
                ],
            }], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module: &fsm, entry_point: Some("fmain"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });

        let mut world = World::new(); let schedule = Schedule::default();
        world.insert_resource(TimeResource::default());
        world.insert_resource(InputResource { manager: InputManager::new() });

        Ok(Self {
            world, schedule, rt: Some(rt), rtv: Some(rtv), tex_size: (vr.width, vr.height),
            selected: None, scene_path: None, playing: false, entities: Vec::new(),
            ren_ctx: None, texture_map: HashMap::new(),
            pipeline, ubuf, bg, sampler: sam, bgl,
            textures: HashMap::new(), spritesheet_info: HashMap::new(), next_tex: 1, fallback_view: fv,
            tile_layers: vec![TL { name: "Ground".into(), vis: true, tiles: vec![vec![0;20];11], cols: 20, rows: 11, ts: 16, z_index: 0, spritesheet_tex_id: None }],
            sel_tile: 0, palette: vec![(0.3,0.6,0.3,1.0),(0.4,0.4,0.3,1.0),(0.5,0.5,0.6,1.0),(0.6,0.3,0.3,1.0)],
            anim_frame: 0, anim_timer: 0.0, scripts: Vec::new(),
            sel_tex_id: None, view_scale: 1.0, view_offset: (0.0,0.0),
            ren_dev_queue: None, transform_mode: TransformMode::Move,
            gizmo_axis: GizmoAxis::None, gizmo_drag_start_world: None, gizmo_drag_start_value: None,
            png_thumbnails: HashMap::new(),
            tilesheet_tex_id: None, tilesheet_cols: 1, tilesheet_rows: 1,
            tilesheet_tile_w: 16, tilesheet_tile_h: 16,
            physics: PhysicsWorld::new(), physics_initialized: false,
            painting_tile: false,
            lua_log: Vec::new(),
            clipboard_entity: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 50,
            entity_names: HashMap::new(),
            tilesheet_egui_tex: None,
            audio_manager: None,
            drag_tex_id: None,
            drag_tex_name: None,
            export_path: None,
            tilesheet_path: None,
            egui_renderer: None,
        })
    }

    /// Save undo snapshot
    pub fn save_undo(&mut self) {
        let mut entity_data = Vec::new();
        for (e, _) in self.world.query::<(Entity, Option<&Transform>)>().iter(&self.world) {
            let name = self.entity_names.get(&e).cloned().unwrap_or_else(|| format!("Ent_{:?}", e));
            // Snapshot all components as JSON
            let mut components: HashMap<String, serde_json::Value> = HashMap::new();
            if let Some(t) = self.world.get::<Transform>(e) { components.insert("Transform".into(), serde_json::to_value(t).unwrap()); }
            if let Some(s) = self.world.get::<Sprite>(e) { components.insert("Sprite".into(), serde_json::to_value(s).unwrap()); }
            if let Some(p) = self.world.get::<Physics>(e) { components.insert("Physics".into(), serde_json::to_value(p).unwrap()); }
            if let Some(a) = self.world.get::<Animation>(e) { components.insert("Animation".into(), serde_json::to_value(a).unwrap()); }
            if let Some(v) = self.world.get::<Velocity>(e) { components.insert("Velocity".into(), serde_json::to_value(v).unwrap()); }
            if let Some(pe) = self.world.get::<ParticleEmitter>(e) { components.insert("ParticleEmitter".into(), serde_json::to_value(pe).unwrap()); }
            if let Some(au) = self.world.get::<AudioSource>(e) { components.insert("AudioSource".into(), serde_json::to_value(au).unwrap()); }
            if let Some(s) = self.world.get::<Script>(e) { components.insert("Script".into(), serde_json::json!({"path": s.path, "source": s.source})); }
            if self.world.get::<CameraTag>(e).is_some() { components.insert("CameraTag".into(), serde_json::json!({})); }
            if self.world.get::<Player>(e).is_some() { components.insert("Player".into(), serde_json::json!({})); }
            if let Some(p) = self.world.get::<Parent>(e) {
                if let Some(pname) = self.entity_names.get(&p.0) {
                    components.insert("Parent".into(), serde_json::json!({"parent_name": pname}));
                }
            }
            entity_data.push((name, format!("{:?}", e), serde_json::to_value(&components).unwrap_or_default()));
        }
        let state = UndoState {
            entity_data,
            tile_layers: self.tile_layers.clone(),
            tilesheet_tex_id: self.tilesheet_tex_id,
            tilesheet_cols: self.tilesheet_cols,
            tilesheet_rows: self.tilesheet_rows,
            tilesheet_tile_w: self.tilesheet_tile_w,
            tilesheet_tile_h: self.tilesheet_tile_h,
            palette: self.palette.clone(),
        };
        self.undo_stack.push(state);
        if self.undo_stack.len() > self.max_undo { self.undo_stack.remove(0); }
        self.redo_stack.clear();
    }

    /// Restore undo state
    pub fn restore_undo(&mut self) {
        let Some(state) = self.undo_stack.pop() else { return };
        // Save current state to redo
        let mut entity_data = Vec::new();
        for (e, _) in self.world.query::<(Entity, Option<&Transform>)>().iter(&self.world) {
            let name = self.entity_names.get(&e).cloned().unwrap_or_else(|| format!("Ent_{:?}", e));
            let mut components: HashMap<String, serde_json::Value> = HashMap::new();
            if let Some(t) = self.world.get::<Transform>(e) { components.insert("Transform".into(), serde_json::to_value(t).unwrap()); }
            if let Some(s) = self.world.get::<Sprite>(e) { components.insert("Sprite".into(), serde_json::to_value(s).unwrap()); }
            if let Some(p) = self.world.get::<Physics>(e) { components.insert("Physics".into(), serde_json::to_value(p).unwrap()); }
            if let Some(a) = self.world.get::<Animation>(e) { components.insert("Animation".into(), serde_json::to_value(a).unwrap()); }
            if let Some(v) = self.world.get::<Velocity>(e) { components.insert("Velocity".into(), serde_json::to_value(v).unwrap()); }
            if let Some(pe) = self.world.get::<ParticleEmitter>(e) { components.insert("ParticleEmitter".into(), serde_json::to_value(pe).unwrap()); }
            if let Some(au) = self.world.get::<AudioSource>(e) { components.insert("AudioSource".into(), serde_json::to_value(au).unwrap()); }
            if let Some(s) = self.world.get::<Script>(e) { components.insert("Script".into(), serde_json::json!({"path": s.path, "source": s.source})); }
            if self.world.get::<CameraTag>(e).is_some() { components.insert("CameraTag".into(), serde_json::json!({})); }
            if self.world.get::<Player>(e).is_some() { components.insert("Player".into(), serde_json::json!({})); }
            entity_data.push((name, format!("{:?}", e), serde_json::to_value(&components).unwrap_or_default()));
        }
        self.redo_stack.push(UndoState {
            entity_data,
            tile_layers: self.tile_layers.clone(),
            tilesheet_tex_id: self.tilesheet_tex_id,
            tilesheet_cols: self.tilesheet_cols,
            tilesheet_rows: self.tilesheet_rows,
            tilesheet_tile_w: self.tilesheet_tile_w,
            tilesheet_tile_h: self.tilesheet_tile_h,
            palette: self.palette.clone(),
        });
        self.apply_undo_state(state);
    }

    /// Restore redo state
    pub fn restore_redo(&mut self) {
        let Some(state) = self.redo_stack.pop() else { return };
        let mut entity_data = Vec::new();
        for (e, _) in self.world.query::<(Entity, Option<&Transform>)>().iter(&self.world) {
            let name = self.entity_names.get(&e).cloned().unwrap_or_else(|| format!("Ent_{:?}", e));
            let mut components: HashMap<String, serde_json::Value> = HashMap::new();
            if let Some(t) = self.world.get::<Transform>(e) { components.insert("Transform".into(), serde_json::to_value(t).unwrap()); }
            if let Some(s) = self.world.get::<Sprite>(e) { components.insert("Sprite".into(), serde_json::to_value(s).unwrap()); }
            if let Some(p) = self.world.get::<Physics>(e) { components.insert("Physics".into(), serde_json::to_value(p).unwrap()); }
            if let Some(a) = self.world.get::<Animation>(e) { components.insert("Animation".into(), serde_json::to_value(a).unwrap()); }
            if let Some(v) = self.world.get::<Velocity>(e) { components.insert("Velocity".into(), serde_json::to_value(v).unwrap()); }
            if let Some(pe) = self.world.get::<ParticleEmitter>(e) { components.insert("ParticleEmitter".into(), serde_json::to_value(pe).unwrap()); }
            if let Some(au) = self.world.get::<AudioSource>(e) { components.insert("AudioSource".into(), serde_json::to_value(au).unwrap()); }
            if let Some(s) = self.world.get::<Script>(e) { components.insert("Script".into(), serde_json::json!({"path": s.path, "source": s.source})); }
            if self.world.get::<CameraTag>(e).is_some() { components.insert("CameraTag".into(), serde_json::json!({})); }
            if self.world.get::<Player>(e).is_some() { components.insert("Player".into(), serde_json::json!({})); }
            entity_data.push((name, format!("{:?}", e), serde_json::to_value(&components).unwrap_or_default()));
        }
        self.undo_stack.push(UndoState {
            entity_data,
            tile_layers: self.tile_layers.clone(),
            tilesheet_tex_id: self.tilesheet_tex_id,
            tilesheet_cols: self.tilesheet_cols,
            tilesheet_rows: self.tilesheet_rows,
            tilesheet_tile_w: self.tilesheet_tile_w,
            tilesheet_tile_h: self.tilesheet_tile_h,
            palette: self.palette.clone(),
        });
        self.apply_undo_state(state);
    }

    fn apply_undo_state(&mut self, state: UndoState) {
        // Despawn all entities
        for e in self.world.query::<Entity>().iter(&self.world).collect::<Vec<_>>() {
            let _ = self.world.despawn(e);
        }
        self.entity_names.clear();

        // Rebuild entity names from data
        let mut name_order: Vec<String> = Vec::new();
        let mut entity_data_map: HashMap<String, (String, serde_json::Value)> = HashMap::new();
        for (name, id_str, comps_json) in &state.entity_data {
            name_order.push(name.clone());
            entity_data_map.insert(name.clone(), (id_str.clone(), comps_json.clone()));
        }

        // Spawn entities in order
        for name in &name_order {
            if let Some((_id, comps_json)) = entity_data_map.get(name) {
                if let Some(comps) = comps_json.as_object() {
                    let mut entity = self.world.spawn_empty();
                    self.entity_names.insert(entity.id(), name.clone());
                    for (comp_type, comp_data) in comps {
                        match comp_type.as_str() {
                            "Transform" => { if let Ok(t) = serde_json::from_value::<Transform>(comp_data.clone()) { entity.insert(t); } }
                            "Sprite" => { if let Ok(s) = serde_json::from_value::<Sprite>(comp_data.clone()) { entity.insert(s); } }
                            "Physics" => { if let Ok(p) = serde_json::from_value::<Physics>(comp_data.clone()) { entity.insert(p); } }
                            "Animation" => { if let Ok(a) = serde_json::from_value::<Animation>(comp_data.clone()) { entity.insert(a); } }
                            "Velocity" => { if let Ok(v) = serde_json::from_value::<Velocity>(comp_data.clone()) { entity.insert(v); } }
                            "ParticleEmitter" => { if let Ok(pe) = serde_json::from_value::<ParticleEmitter>(comp_data.clone()) { entity.insert(pe); } }
                            "AudioSource" => { if let Ok(au) = serde_json::from_value::<AudioSource>(comp_data.clone()) { entity.insert(au); } }
                            "Script" => { if let Some(path) = comp_data.get("path").and_then(|v| v.as_str()) { entity.insert(Script { path: path.to_string(), source: comp_data.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string() }); } }
                            "CameraTag" => { entity.insert(CameraTag); }
                            "Player" => { entity.insert(Player); }
                            "Parent" => {
                                // Will be wired in pass 2
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Pass 2: wire parent/child
        for name in &name_order {
            if let Some((_id, comps_json)) = entity_data_map.get(name) {
                if let Some(comps) = comps_json.as_object() {
                    if let Some(parent_data) = comps.get("Parent") {
                        if let Some(pname) = parent_data.get("parent_name").and_then(|v| v.as_str()) {
                            // Find child entity by name
                            if let Some(child_entity) = self.entity_names.iter().find(|(_, n)| *n == name).map(|(e, _)| *e) {
                                if let Some(parent_entity) = self.entity_names.iter().find(|(_, n)| *n == pname).map(|(e, _)| *e) {
                                    self.world.entity_mut(child_entity).insert(Parent(parent_entity));
                                    if let Some(mut children) = self.world.get_mut::<Children>(parent_entity) {
                                        children.0.push(child_entity);
                                    } else {
                                        self.world.entity_mut(parent_entity).insert(Children(vec![child_entity]));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.tile_layers = state.tile_layers;
        self.tilesheet_tex_id = state.tilesheet_tex_id;
        self.tilesheet_cols = state.tilesheet_cols;
        self.tilesheet_rows = state.tilesheet_rows;
        self.tilesheet_tile_w = state.tilesheet_tile_w;
        self.tilesheet_tile_h = state.tilesheet_tile_h;
        self.palette = state.palette;
        self.refresh();
        self.selected = None;
    }

    pub fn set_renderer(&mut self, dev: wgpu::Device, queue: wgpu::Queue, egui_renderer: Arc<egui::mutex::RwLock<Renderer>>) {
        self.ren_ctx = Some(RenCtx { dev, queue });
        self.egui_renderer = Some(egui_renderer);
    }

    /// Central helper: create a wgpu texture, register it with egui,
    /// insert into textures map, and store default spritesheet info.
    /// Returns the texture id.
    pub fn import_and_slice_texture(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], auto_slice: bool) -> Option<u64> {
        let img = image::load_from_memory(bytes).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = (img.width(), img.height());
        let t = dev.create_texture(&wgpu::TextureDescriptor {
            label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &t, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &rgba, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4*w), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let view = t.create_view(&wgpu::TextureViewDescriptor::default());
        let id = self.next_tex; self.next_tex += 1;

        // Register with egui_wgpu renderer for native texture thumbnails
        let egui_tex_id = self.egui_renderer.as_ref().map(|r| {
            let mut renderer = r.write();
            renderer.register_native_texture(dev, &view, wgpu::FilterMode::Nearest)
        });
        self.textures.insert(id, (view, w, h, egui_tex_id));

        // Always populate spritesheet_info — default = full-texture single tile
        let (tile_w, tile_h) = if auto_slice {
            let s = auto_tile_size(w, h);
            (s, s)
        } else {
            (w, h)
        };
        self.spritesheet_info.insert(id, SpritesheetInfo::new(w, h, tile_w, tile_h));
        Some(id)
    }

    /// Legacy load_tex — forwards to import_and_slice_texture without auto-slice
    pub fn load_tex(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) -> Option<u64> {
        self.import_and_slice_texture(dev, queue, bytes, false)
    }

    pub fn refresh(&mut self) {
        self.entities.clear();
        for (e,_) in self.world.query::<(Entity, Option<&Transform>)>().iter(&self.world) {
            let name = self.entity_names.get(&e).cloned().unwrap_or_else(|| format!("Ent_{:?}", e));
            self.entities.push(EI { entity: e, name });
        }
    }

    pub fn add(&mut self) {
        self.save_undo();
        let e = self.world.spawn((
            Transform::new(160.0, 90.0),
            Sprite { layer: 0, color: [1.0,1.0,1.0,1.0], visible: true, ..Default::default() },
        )).id();
        self.entity_names.insert(e, format!("Entity_{}", self.entities.len()));
        self.refresh(); self.selected = Some(e);
    }

    pub fn remove(&mut self, e: Entity) {
        self.save_undo();
        // Remove from parent if needed
        if let Some(parent) = self.world.get::<Parent>(e).copied() {
            if let Some(mut children) = self.world.get_mut::<Children>(parent.0) {
                children.0.retain(|c| *c != e);
            }
        }
        // Remove children
        let children: Vec<Entity> = self.world.get::<Children>(e).map(|c| c.0.clone()).unwrap_or_default();
        for child in &children {
            let _ = self.world.despawn(*child);
            self.entity_names.remove(child);
        }
        self.entity_names.remove(&e);
        if self.selected == Some(e) { self.selected = None; }
        let _ = self.world.despawn(e); self.refresh();
    }

    /// Reorder entity in the list (drag up/down for render order)
    pub fn move_entity_up(&mut self, idx: usize) {
        if idx == 0 || idx >= self.entities.len() { return; }
        self.save_undo();
        self.entities.swap(idx, idx - 1);
        self.refresh();
    }
    pub fn move_entity_down(&mut self, idx: usize) {
        if idx + 1 >= self.entities.len() { return; }
        self.save_undo();
        self.entities.swap(idx, idx + 1);
        self.refresh();
    }

    /// Set parent entity
    pub fn set_parent(&mut self, child: Entity, parent: Option<Entity>) {
        self.save_undo();
        // Remove old parent
        if let Some(old_parent) = self.world.get::<Parent>(child).copied() {
            if let Some(mut children) = self.world.get_mut::<Children>(old_parent.0) {
                children.0.retain(|c| *c != child);
            }
            self.world.entity_mut(child).remove::<Parent>();
        }
        if let Some(p) = parent {
            self.world.entity_mut(child).insert(Parent(p));
            if let Some(mut children) = self.world.get_mut::<Children>(p) {
                children.0.push(child);
            } else {
                self.world.entity_mut(p).insert(Children(vec![child]));
            }
        }
    }

    pub fn copy_selected(&mut self) {
        self.clipboard_entity = self.selected;
    }

    pub fn paste_entity(&mut self) {
        self.save_undo();
        if let Some(src) = self.clipboard_entity {
            let tr = self.world.get::<Transform>(src).cloned().unwrap_or(Transform::new(160.0, 90.0));
            let sp = self.world.get::<Sprite>(src).cloned();
            let ph = self.world.get::<Physics>(src).cloned();
            let an = self.world.get::<Animation>(src).cloned();
            let vel = self.world.get::<Velocity>(src).cloned();
            let pe = self.world.get::<ParticleEmitter>(src).cloned();
            let au = self.world.get::<AudioSource>(src).cloned();
            let mut new_tr = tr.clone();
            new_tr.x += 10.0;
            new_tr.y += 10.0;
            let mut cmd = self.world.spawn(new_tr);
            if let Some(s) = sp { cmd.insert(s); }
            if let Some(p) = ph { cmd.insert(p); }
            if let Some(a) = an { cmd.insert(a); }
            if let Some(v) = vel { cmd.insert(v); }
            if let Some(p) = pe { cmd.insert(p); }
            if let Some(a) = au { cmd.insert(a); }
            let e = cmd.id();
            let name = self.entity_names.get(&src).cloned().unwrap_or_else(|| "Pasted".to_string());
            self.entity_names.insert(e, format!("{}_copy", name));
            self.refresh();
            self.selected = Some(e);
        }
    }

    pub fn load_scene(&mut self, p: &PathBuf) -> Result<()> {
        self.save_undo();
        for id in self.world.query::<Entity>().iter(&self.world).collect::<Vec<_>>() { let _ = self.world.despawn(id); }
        self.entity_names.clear();
        let descriptor = Scene::load_from_file(p)?;
        let _ = spawn_scene(&mut self.world, descriptor.clone())?;
        
        // Restore tilemap data
        let tilesheet_path = p.parent().unwrap_or(p).join("..").join("tilesets");
        // Rebuild entity names from descriptor
        for (i, ed) in descriptor.entities.iter().enumerate() {
            // Find the spawned entity for this name
            // We need to recreate names
        }
        self.refresh();
        
        // Name entities from the descriptor
        let mut ei = self.entities.clone();
        for (i, ed) in descriptor.entities.iter().enumerate() {
            if i < ei.len() {
                self.entity_names.insert(ei[i].entity, ed.name.clone());
            }
        }
        self.refresh();
        
        // Restore tilemap
        if !descriptor.tilemap.layers.is_empty() {
            self.tile_layers = descriptor.tilemap.layers.iter().map(|l| TL {
                name: l.name.clone(),
                vis: l.visible,
                tiles: l.tiles.clone(),
                cols: l.cols,
                rows: l.rows,
                ts: l.tile_size,
                z_index: l.z_index,
                spritesheet_tex_id: l.spritesheet_tex_id,
            }).collect();
            self.tilesheet_cols = descriptor.tilemap.tilesheet_cols;
            self.tilesheet_rows = descriptor.tilemap.tilesheet_rows;
            self.tilesheet_tile_w = descriptor.tilemap.tilesheet_tile_w;
            self.tilesheet_tile_h = descriptor.tilemap.tilesheet_tile_h;
            self.palette = descriptor.tilemap.palette.iter().map(|c| (c[0], c[1], c[2], c[3])).collect();
        }
        
        self.scene_path = Some(p.clone()); self.refresh(); Ok(())
    }

    pub fn save_scene(&mut self, p: &PathBuf) -> Result<()> {
        let entities = serialize_world(&mut self.world);
        // Merge with tilemap data
        let tilemap = TilemapData {
            layers: self.tile_layers.iter().map(|l| TileLayerData {
                name: l.name.clone(),
                visible: l.vis,
                tiles: l.tiles.clone(),
                cols: l.cols,
                rows: l.rows,
                tile_size: l.ts,
                z_index: l.z_index,
                spritesheet_tex_id: l.spritesheet_tex_id,
            }).collect(),
            tilesheet_tex_id: self.tilesheet_tex_id,
            tilesheet_cols: self.tilesheet_cols,
            tilesheet_rows: self.tilesheet_rows,
            tilesheet_tile_w: self.tilesheet_tile_w,
            tilesheet_tile_h: self.tilesheet_tile_h,
            palette: self.palette.iter().map(|c| [c.0, c.1, c.2, c.3]).collect(),
        };
        let desc = SceneDescriptor {
            name: entities.name,
            entities: entities.entities,
            tilemap,
        };
        std::fs::write(p, serde_json::to_string_pretty(&desc)?)?;
        self.scene_path = Some(p.clone()); Ok(())
    }

    /// Hit test: find entity at world position, return (entity, distance^2)
    pub fn hit_test(&mut self, wx: f32, wy: f32) -> Option<Entity> {
        let mut best: Option<(Entity, f32)> = None;
        for (e, tr, sp) in self.world.query::<(Entity, &Transform, &Sprite)>().iter(&self.world) {
            if !sp.visible { continue; }
            let sw = (if sp.source_width > 0 { sp.source_width as f32 } else { 16.0 }) * tr.scale_x;
            let sh = (if sp.source_height > 0 { sp.source_height as f32 } else { 16.0 }) * tr.scale_y;
            let hw = sw / 2.0;
            let hh = sh / 2.0;
            if wx >= tr.x - hw && wx <= tr.x + hw && wy >= tr.y - hh && wy <= tr.y + hh {
                let d = (wx - tr.x).powi(2) + (wy - tr.y).powi(2);
                if best.is_none() || d < best.unwrap().1 { best = Some((e, d)); }
            }
        }
        best.map(|(e, _)| e)
    }

    /// Convert screen pixel to world position (given viewport rect)
    /// FIXED: world Y increases downward, same as screen Y
    pub fn _screen_to_world(&self, screen_px: (f32, f32), viewport_rect: &egui::Rect) -> (f32, f32) {
        let vw = viewport_rect.width();
        let vh = viewport_rect.height();
        let tw = self.tex_size.0 as f32;
        let th = self.tex_size.1 as f32;
        let aspect = tw / th;
        let (dw, dh) = if vw / vh > aspect { (vh * aspect, vh) } else { (vw, vw / aspect) };
        let left = viewport_rect.left() + (vw - dw) / 2.0;
        let top = viewport_rect.top() + (vh - dh) / 2.0;
        let nx = (screen_px.0 - left) / dw;
        let ny = (screen_px.1 - top) / dh;
        // FIXED: wy increases downward (ny=0 is top of screen → wy=0 at top)
        let wx = (nx * tw - self.view_offset.0) / self.view_scale;
        let wy = (ny * th + self.view_offset.1) / self.view_scale;
        (wx, wy)
    }

    /// Load a PNG and return its egui color image for thumbnail display
    pub fn load_png_thumbnail(&mut self, path: &PathBuf) -> Option<egui::ColorImage> {
        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let size = w.min(h).min(64) as usize; // max 64px thumbnail
        let thumb = image::imageops::resize(&rgba, size as u32, size as u32, image::imageops::FilterType::Nearest);
        let pixels: Vec<egui::Color32> = thumb.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])).collect();
        Some(egui::ColorImage { size: [size, size], pixels })
    }

    /// Get the world-space extent of the gizmo handles
    fn get_gizmo_handles(&self, cx: f32, cy: f32, scale: f32) -> Vec<(GizmoAxis, [[f32;2];2], f32)> {
        let len = 20.0 * scale;
        match self.transform_mode {
            TransformMode::Move => vec![
                (GizmoAxis::X, [[cx, cy], [cx+len, cy]], 0.0),
                (GizmoAxis::Y, [[cx, cy], [cx, cy+len]], 0.0),
                (GizmoAxis::XY, [[cx+len*0.5, cy+len*0.5], [cx+len*0.5, cy+len*0.5]], 0.0),
            ],
            TransformMode::Scale => vec![
                (GizmoAxis::X, [[cx+len*0.3, cy], [cx+len, cy]], 0.0),
                (GizmoAxis::Y, [[cx, cy+len*0.3], [cx, cy+len]], 0.0),
            ],
            TransformMode::Rotate => vec![
                (GizmoAxis::Rot, [[cx+len*0.7, cy], [cx+len*0.7, cy]], 0.0),
            ],
        }
    }

    /// Check if a world pos hits a gizmo handle. Returns (axis, center_x, center_y)
    pub fn hit_test_gizmo(&self, wx: f32, wy: f32, cx: f32, cy: f32, scale: f32) -> GizmoAxis {
        let threshold = 8.0 * scale;
        let handles = self.get_gizmo_handles(cx, cy, scale);
        for (axis, pts, _) in &handles {
            for &p in pts {
                let dx = wx - p[0];
                let dy = wy - p[1];
                if dx*dx + dy*dy < threshold*threshold {
                    return *axis;
                }
            }
        }
        GizmoAxis::None
    }

    pub fn render(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(ref tv) = self.rtv else { return };
        let mut enc = dev.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        if self.playing {
            if !self.physics_initialized {
                self.physics.gravity = Vector::new(0.0, 200.0);
                for (e, tr, ph) in self.world.query::<(Entity, &Transform, &Physics)>().iter(&self.world) {
                    self.physics.create_body(e, tr, ph);
                }
                // Generate physics colliders for tilemap layers
                for l in &self.tile_layers {
                    if !l.vis { continue; }
                    let ts = l.ts as f32;
                    for (r, row) in l.tiles.iter().enumerate() {
                        for (c, &tid) in row.iter().enumerate() {
                            if tid == 0 { continue; }
                            // Create a static rigid body for each tile
                            let x = c as f32 * ts + ts / 2.0;
                            let y = r as f32 * ts + ts / 2.0;
                            let body = RigidBodyBuilder::new(RigidBodyType::Fixed)
                                .translation(vector![x, y])
                                .build();
                            let handle = self.physics.rigid_body_set.insert(body);
                            let collider = ColliderBuilder::cuboid(ts / 2.0, ts / 2.0).build();
                            self.physics.collider_set.insert_with_parent(collider, handle, &mut self.physics.rigid_body_set);
                        }
                    }
                }
                self.physics_initialized = true;
            }
            self.physics.step();
            for (e, mut tr) in self.world.query::<(Entity, &mut Transform)>().iter_mut(&mut self.world) {
                if let Some(handle) = self.physics.entity_map.get(&e) {
                    if let Some(rb) = self.physics.rigid_body_set.get(*handle) {
                        let pos = rb.translation();
                        tr.x = pos.x;
                        tr.y = pos.y;
                        tr.rotation = rb.rotation().angle();
                    }
                }
            }
            self.schedule.run(&mut self.world);
        }

        let (w, h) = (self.tex_size.0 as f32, self.tex_size.1 as f32);
        let (ox, oy) = self.view_offset;
        let vs = self.view_scale;
        // Pixel-perfect: compute sub-pixel jitter so world origin maps to
        // an exact integer pixel center (eliminates shimmering at any zoom).
        // Under the base matrix:
        //   pixel_x = vs*wx + ox
        //   pixel_y = vs*wy - oy
        // We want pixel(0,0) to land on an integer pixel, so we compute the
        // fractional part of the screen position of world origin and apply
        // inverse jitter to the projection matrix.
        let screen_ox = ox;           // screen pixel x of world (0,0)
        let screen_oy = -oy;          // screen pixel y of world (0,0), positive = down
        // frac() gives [0, 1), we want to snap to nearest integer
        // so we need to subtract the fractional part
        let frac_x = screen_ox - screen_ox.floor();
        let frac_y = screen_oy - screen_oy.floor();
        // Jitter in NDC: subtract the fractional offset so pixel(0,0) snaps
        let jitter_ndc_x = -frac_x * 2.0 / w;
        let jitter_ndc_y = -frac_y * 2.0 / h;
        queue.write_buffer(&self.ubuf, 0, bytemuck::cast_slice(&[[
            [2.0*vs/w,0.0,0.0,0.0],
            [0.0,-2.0*vs/h,0.0,0.0],
            [0.0,0.0,1.0,0.0],
            [-1.0+ox*2.0/w+jitter_ndc_x, 1.0+oy*2.0/h+jitter_ndc_y, 0.0, 1.0]
        ]]));

        // Batch vertices by texture ID (0 = fallback/white texture)
        #[derive(Default)]
        struct Batch { verts: Vec<SV>, idxs: Vec<u32> }
        let mut batches: HashMap<u64, Batch> = HashMap::new();

        // Tilemap - use per-layer spritesheets if available, otherwise use global tilesheet
        for l in &self.tile_layers {
            if !l.vis { continue; }
            let layer_tilesheet_id = l.spritesheet_tex_id.or(self.tilesheet_tex_id).unwrap_or(0);
            for (r, row) in l.tiles.iter().enumerate() {
                for (c, &tid) in row.iter().enumerate() {
                    if tid == 0 { continue; }
                    let (x, y, ts) = (c as f32 * l.ts as f32, r as f32 * l.ts as f32, l.ts as f32);
                    let b = batches.entry(layer_tilesheet_id).or_default();
                    let i = b.verts.len() as u32;
                    if layer_tilesheet_id != 0 {
                        // Get spritesheet info for UV calculation
                        let info = self.spritesheet_info.get(&layer_tilesheet_id).cloned()
                            .unwrap_or_else(|| SpritesheetInfo::new(l.ts, l.ts, l.ts, l.ts));
                        let tile_idx = tid as u32 - 1;
                        let (u0, v0, u1, v1) = info.uv_for_tile(tile_idx);
                        b.verts.extend(&[
                            SV { pos: [x,y], uv: [u0,v0], color: [1.0,1.0,1.0,1.0], layer: l.z_index as f32 },
                            SV { pos: [x+ts,y], uv: [u1,v0], color: [1.0,1.0,1.0,1.0], layer: l.z_index as f32 },
                            SV { pos: [x+ts,y+ts], uv: [u1,v1], color: [1.0,1.0,1.0,1.0], layer: l.z_index as f32 },
                            SV { pos: [x,y+ts], uv: [u0,v1], color: [1.0,1.0,1.0,1.0], layer: l.z_index as f32 }]);
                    } else {
                        let pal_idx = ((tid as usize).max(1).saturating_sub(1)).min(self.palette.len().saturating_sub(1));
                        let col = self.palette[pal_idx];
                        b.verts.extend(&[SV { pos: [x,y], uv: [0.0,0.0], color: [col.0,col.1,col.2,col.3], layer: l.z_index as f32 },
                            SV { pos: [x+ts,y], uv: [1.0,0.0], color: [col.0,col.1,col.2,col.3], layer: l.z_index as f32 },
                            SV { pos: [x+ts,y+ts], uv: [1.0,1.0], color: [col.0,col.1,col.2,col.3], layer: l.z_index as f32 },
                            SV { pos: [x,y+ts], uv: [0.0,1.0], color: [col.0,col.1,col.2,col.3], layer: l.z_index as f32 }]);
                    }
                    b.idxs.extend(&[i,i+1,i+2,i,i+2,i+3]);
                }
            }
        }

        // ECS sprites - group by texture_id
        for (tr, sp) in self.world.query::<(&Transform, &Sprite)>().iter(&self.world) {
            if !sp.visible { continue; }
            let tid = sp.texture_id.unwrap_or(0);
            let b = batches.entry(tid).or_default();
            let (sw, sh) = (if sp.source_width > 0 { sp.source_width as f32 } else { 16.0 } * tr.scale_x,
                            if sp.source_height > 0 { sp.source_height as f32 } else { 16.0 } * tr.scale_y);
            let cos = tr.rotation.cos();
            let sin = tr.rotation.sin();
            let hw = sw/2.0; let hh = sh/2.0;
            let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];
            let uvs = if sp.flip_x { [(1.0,0.0),(0.0,0.0),(0.0,1.0),(1.0,1.0)] } else { [(0.0,0.0),(1.0,0.0),(1.0,1.0),(0.0,1.0)] };
            let uvs = if sp.flip_y { [(uvs[0].0,1.0-uvs[0].1),(uvs[1].0,1.0-uvs[1].1),(uvs[2].0,1.0-uvs[2].1),(uvs[3].0,1.0-uvs[3].1)] } else { uvs };
            let i = b.verts.len() as u32;
            for (j, &(lx, ly)) in corners.iter().enumerate() {
                let rx = lx*cos - ly*sin;
                let ry = lx*sin + ly*cos;
                b.verts.push(SV { pos: [tr.x+rx, tr.y+ry], uv: [uvs[j].0, uvs[j].1], color: sp.color, layer: sp.layer as f32 });
            }
            b.idxs.extend(&[i,i+1,i+2,i,i+2,i+3]);
        }

        // Selection outline on selected entity (only if it has a visible sprite)
        // Draws a steady 2px-thick border using thin quads along each edge
        if let Some(sel) = self.selected {
            if let Some(tr) = self.world.get::<Transform>(sel) {
                if let Some(sp) = self.world.get::<Sprite>(sel) {
                    if sp.visible {
                        let (sw, sh) = (if sp.source_width > 0 { sp.source_width as f32 } else { 16.0 } * tr.scale_x,
                                        if sp.source_height > 0 { sp.source_height as f32 } else { 16.0 } * tr.scale_y);
                        let b = batches.entry(0).or_default();
                        let cos = tr.rotation.cos(); let sin = tr.rotation.sin();
                        let half_w = sw/2.0; let half_h = sh/2.0;
                        let corners = [(-half_w, -half_h), (half_w, -half_h), (half_w, half_h), (-half_w, half_h)];
                        let outline_width = 2.0 / self.view_scale.max(0.1); // 2px thickness in world units
                        // Rotate each corner
                        let rc: Vec<[f32;2]> = corners.iter().map(|&(lx, ly)| {
                            [tr.x + lx*cos - ly*sin, tr.y + lx*sin + ly*cos]
                        }).collect();
                        // Build 4 thin quads for each edge of the bounding box
                        let edges = [(0,1), (1,2), (2,3), (3,0)];
                        let ow = outline_width;
                        for &(a, b_idx) in &edges {
                            let (ax, ay) = (rc[a][0], rc[a][1]);
                            let (bx, by) = (rc[b_idx][0], rc[b_idx][1]);
                            // Edge direction
                            let dx = bx - ax;
                            let dy = by - ay;
                            let len = (dx*dx + dy*dy).sqrt().max(0.001);
                            let nx = -dy / len * ow * 0.5;
                            let ny = dx / len * ow * 0.5;
                            let i = b.verts.len() as u32;
                            b.verts.extend(&[
                                SV { pos: [ax - nx, ay - ny], uv: [0.0,0.0], color: [1.0,1.0,0.0,1.0], layer: 9999.0 },
                                SV { pos: [ax + nx, ay + ny], uv: [0.0,0.0], color: [1.0,1.0,0.0,1.0], layer: 9999.0 },
                                SV { pos: [bx + nx, by + ny], uv: [0.0,0.0], color: [1.0,1.0,0.0,1.0], layer: 9999.0 },
                                SV { pos: [bx - nx, by - ny], uv: [0.0,0.0], color: [1.0,1.0,0.0,1.0], layer: 9999.0 },
                            ]);
                            b.idxs.extend(&[i, i+1, i+2, i, i+2, i+3]);
                        }
                    }
                }
            }
        }

        // Physics debug
        for (tr, ph) in self.world.query::<(&Transform, &Physics)>().iter(&self.world) {
            let b = batches.entry(0).or_default();
            let (x, y) = (tr.x - ph.collider_width/2.0, tr.y - ph.collider_height/2.0);
            let c = if ph.is_trigger { [0.0,1.0,0.3,0.5] } else { [1.0,0.3,0.3,0.5] };
            let i = b.verts.len() as u32;
            b.verts.extend(&[SV { pos: [x,y], uv: [0.0,0.0], color: c, layer: 999.0 },
                SV { pos: [x+ph.collider_width,y], uv: [1.0,0.0], color: c, layer: 999.0 },
                SV { pos: [x+ph.collider_width,y+ph.collider_height], uv: [1.0,1.0], color: c, layer: 999.0 },
                SV { pos: [x,y+ph.collider_height], uv: [0.0,1.0], color: c, layer: 999.0 }]);
            b.idxs.extend(&[i,i+1,i+2,i,i+2,i+3]);
        }

        // Particle rendering - render each particle as a small colored quad
        for (emitter, tr) in self.world.query::<(&ParticleEmitter, &Transform)>().iter(&self.world) {
            let b = batches.entry(0).or_default();
            for p in &emitter.particles {
                let sz = p.size;
                let half = sz / 2.0;
                let i = b.verts.len() as u32;
                b.verts.extend(&[
                    SV { pos: [p.x - half, p.y - half], uv: [0.0,0.0], color: p.color, layer: 9998.0 },
                    SV { pos: [p.x + half, p.y - half], uv: [1.0,0.0], color: p.color, layer: 9998.0 },
                    SV { pos: [p.x + half, p.y + half], uv: [1.0,1.0], color: p.color, layer: 9998.0 },
                    SV { pos: [p.x - half, p.y + half], uv: [0.0,1.0], color: p.color, layer: 9998.0 },
                ]);
                b.idxs.extend(&[i,i+1,i+2,i,i+2,i+3]);
            }
        }

        // Tilemap collision generation (auto physics for tile layers)
        if self.playing {
            for l in &self.tile_layers {
                if !l.vis { continue; }
                let ts = l.ts as f32;
                for (r, row) in l.tiles.iter().enumerate() {
                    for (c, &tid) in row.iter().enumerate() {
                        if tid == 0 { continue; }
                        let b = batches.entry(0).or_default();
                        let (x, y) = (c as f32 * ts, r as f32 * ts);
                        let i = b.verts.len() as u32;
                        b.verts.extend(&[SV { pos: [x,y], uv: [0.0,0.0], color: [1.0,0.5,0.0,0.3], layer: 998.0 },
                            SV { pos: [x+ts,y], uv: [0.0,0.0], color: [1.0,0.5,0.0,0.3], layer: 998.0 },
                            SV { pos: [x+ts,y+ts], uv: [0.0,0.0], color: [1.0,0.5,0.0,0.3], layer: 998.0 },
                            SV { pos: [x,y+ts], uv: [0.0,0.0], color: [1.0,0.5,0.0,0.3], layer: 998.0 }]);
                        b.idxs.extend(&[i,i+1,i+2,i,i+2,i+3]);
                    }
                }
            }
        }

        // Sort batches by their minimum layer value for correct z-ordering
        let mut sorted_batches: Vec<(u64, &Batch)> = batches.iter().map(|(k, b)| (*k, b)).collect();
        sorted_batches.sort_by(|a, b| {
            let a_layer = a.1.verts.first().map(|v| v.layer).unwrap_or(0.0);
            let b_layer = b.1.verts.first().map(|v| v.layer).unwrap_or(0.0);
            a_layer.partial_cmp(&b_layer).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Upload and draw each batch with its texture
        let bg_c = if self.playing { wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 } } else { wgpu::Color { r: 0.2, g: 0.3, b: 0.6, a: 1.0 } };
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None, color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: tv, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(bg_c), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
        });
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, &self.bg, &[]);
        
        let sam = &self.sampler;
        for (tex_id, batch) in &sorted_batches {
            if batch.verts.is_empty() { continue; }
            // Create bind group for this texture (or use fallback)
            let tex_view = if *tex_id == 0 { &self.fallback_view } else {
                self.textures.get(tex_id).map(|(tv,_,_,_)| tv).unwrap_or(&self.fallback_view)
            };
            let tex_bg = dev.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.ubuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(tex_view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sam) },
                ],
            });
            rp.set_bind_group(0, &tex_bg, &[]);
            
            let vbuf = dev.create_buffer(&wgpu::BufferDescriptor {
                label: None, size: (batch.verts.len() * std::mem::size_of::<SV>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            });
            queue.write_buffer(&vbuf, 0, bytemuck::cast_slice(&batch.verts));
            let ibuf = dev.create_buffer(&wgpu::BufferDescriptor {
                label: None, size: (batch.idxs.len() * 4) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            });
            queue.write_buffer(&ibuf, 0, bytemuck::cast_slice(&batch.idxs));
            rp.set_vertex_buffer(0, vbuf.slice(..));
            rp.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..batch.idxs.len() as u32, 0, 0..1);
        }
        drop(rp);
        queue.submit(std::iter::once(enc.finish()));
    }

    #[allow(unused)]
    pub fn resize(&mut self, dev: &wgpu::Device, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        self.rt = Some(dev.create_texture(&wgpu::TextureDescriptor {
            label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[],
        }));
        self.rtv = Some(self.rt.as_ref().unwrap().create_view(&wgpu::TextureViewDescriptor::default()));
        self.tex_size = (w, h);
    }

    /// Upload a tilesheet texture to egui for actual image preview
    pub fn _upload_tilesheet_to_egui(&mut self, ctx: &egui::Context) {
        if let Some(tid) = self.tilesheet_tex_id {
            if let Some((ref tv, tw, th, _egui_id)) = self.textures.get(&tid) {
                // Read back texture pixels for egui
                if let Some(ref rc) = self.ren_ctx {
                    // We need to create a buffer to read the texture back
                    let buf_size = (tw * th * 4) as u64;
                    let buf = rc.dev.create_buffer(&wgpu::BufferDescriptor {
                        label: None, size: buf_size,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    });
                    let mut enc = rc.dev.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    let (_tex, _tw, _th, _egui_id) = self.textures.get(&tid).unwrap();
                    // We need the actual texture, not the view
                    // For simplicity, just use numbered tiles for now and load from file
                    // This is a known limitation - we show numbers instead of actual images
                    // The actual image preview would need a separate readback path
                    log::info!("Tilesheet {}: {}x{} with {}x{} tiles ({}x{})", tid, tw, th, 
                        self.tilesheet_cols, self.tilesheet_rows,
                        self.tilesheet_tile_w, self.tilesheet_tile_h);
                }
            }
        }
    }

    /// Build & Export the game as a standalone binary
    pub fn build_export(&mut self, out_dir: &PathBuf) -> Result<()> {
        log::info!("Exporting game to: {:?}", out_dir);
        // Save current scene
        let scene_path = out_dir.join("assets").join("scenes").join("scene.json");
        std::fs::create_dir_all(out_dir.join("assets").join("scenes"))?;
        std::fs::create_dir_all(out_dir.join("assets").join("textures"))?;
        std::fs::create_dir_all(out_dir.join("assets").join("scripts"))?;
        std::fs::create_dir_all(out_dir.join("assets").join("tilesets"))?;
        
        // Save scene
        let _ = self.save_scene(&scene_path);
        
        // Copy all texture files
        let assets_dir = std::path::Path::new("assets");
        if assets_dir.exists() {
            for entry in walkdir::WalkDir::new(assets_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let rel = entry.path().strip_prefix("assets").unwrap_or(entry.path());
                    let dst = out_dir.join("assets").join(rel);
                    if let Some(parent) = dst.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::copy(entry.path(), &dst);
                }
            }
        }
        
        // Try to compile the game binary (requires cargo)
        log::info!("Building game binary...");
        let status = std::process::Command::new("cargo")
            .args(["build", "--bin", "pixgine-game", "--release"])
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to run cargo: {}", e))?;
        
        if status.success() {
            // Copy the binary
            let src = std::path::Path::new("target").join("release").join("pixgine-game");
            if src.exists() {
                let dst = out_dir.join("pixgine-game");
                let _ = std::fs::copy(&src, &dst);
                log::info!("Game exported to: {:?}", dst);
            }
            // Also copy assets alongside the binary
            let _ = std::fs::write(out_dir.join("run.sh"), "#!/bin/bash\n./pixgine-game\n");
            #[cfg(unix)]
            let _ = std::process::Command::new("chmod").args(["+x", out_dir.join("run.sh").to_str().unwrap_or("")]).status();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Build failed"))
        }
    }
}