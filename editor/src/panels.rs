use egui::{CentralPanel, SidePanel, TopBottomPanel, TextureId, Color32, Rgba, Frame};
use pixgine_engine::ecs::*;
use pixgine_engine::assets::{ImportSettings, TextureType, TextureFilter};
use crate::viewport::{VP, TransformMode, GizmoAxis};
use std::path::PathBuf;
use std::collections::HashMap;
use bevy_ecs::entity::Entity;

pub struct EP {
    pub tex_id: Option<TextureId>, 
    pub tab: Tab,
    pub asset_path: PathBuf,
    pub show_create_folder_dialog: bool,
    pub show_open_file_dialog: bool,
    pub show_save_file_dialog: bool,
    pub show_open_project_dialog: bool,
    pub show_create_project_dialog: bool,
    pub new_scene_name: String,
    pub new_asset_name: String,
    pub renaming_entity: Option<Entity>,
    pub rename_buffer: String,
    pub renaming_layer: Option<usize>,
    pub layer_rename_buffer: String,
    pub panning: bool,
    pub pan_start: Option<egui::Pos2>,
    pub pan_offset_start: (f32, f32),
    pub gizmo_dragging: bool,
    pub imported_textures: Vec<(u64, String, u32, u32)>,
    pub thumbnails: HashMap<String, egui::ColorImage>,
    pub project_path: Option<PathBuf>,
    pub drag_entity_idx: Option<usize>,
    pub drag_target_idx: Option<usize>,
    pub show_parent_selector: bool,
    pub parent_entity: Option<Entity>,
    pub show_export_dialog: bool,
    pub export_path: PathBuf,
    pub show_build_dialog: bool,
    pub build_message: String,
    pub lua_input: String,
    /// Currently selected texture in the Import tab
    pub selected_import_tex: Option<u64>,
    /// Recently opened scene files
    pub recent_files: Vec<PathBuf>,
    /// Show project settings window
    pub show_project_settings: bool,
    /// Texture ID for the import settings dialog (context menu)
    pub import_settings_tex: Option<u64>,
    /// Temporary import settings being edited in the dialog
    pub edit_import_settings: ImportSettings,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tab { Scene, Tilemap, Anim, Scripts, Import }

impl EP {
    pub fn new() -> Self { 
        Self { 
            tex_id: None, 
            tab: Tab::Scene,
            asset_path: PathBuf::from("assets"),
            show_create_folder_dialog: false,
            show_open_file_dialog: false,
            show_save_file_dialog: false,
            show_open_project_dialog: false,
            show_create_project_dialog: false,
            new_scene_name: String::new(),
            new_asset_name: String::new(),
            renaming_entity: None,
            rename_buffer: String::new(),
            renaming_layer: None,
            layer_rename_buffer: String::new(),
            panning: false,
            pan_start: None,
            pan_offset_start: (0.0, 0.0),
            gizmo_dragging: false,
            imported_textures: Vec::new(),
            thumbnails: HashMap::new(),
            project_path: None,
            drag_entity_idx: None,
            drag_target_idx: None,
            show_parent_selector: false,
            parent_entity: None,
            show_export_dialog: false,
            export_path: PathBuf::from("."),
            show_build_dialog: false,
            build_message: String::new(),
            lua_input: String::new(),
            selected_import_tex: None,
            recent_files: Vec::new(),
            show_project_settings: false,
            import_settings_tex: None,
            edit_import_settings: ImportSettings::default(),
        } 
    }
    pub fn set_tex(&mut self, id: Option<TextureId>) { self.tex_id = id; }

    /// Apply new import settings to a texture and update the spritesheet info.
    /// This does NOT re-upload the GPU texture (that only matters for filter mode);
    /// it just updates the in-memory slicing configuration so the change is
    /// immediately visible in the editor.
    fn apply_import_settings(&mut self, vp: &mut VP, tex_id: u64, settings: ImportSettings) {
        let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((0, 0));
        let (tile_w, tile_h) = match settings.texture_type {
            TextureType::Texture => (tw, th),
            TextureType::Spritesheet => (settings.tile_width, settings.tile_height),
        };
        vp.spritesheet_info.insert(tex_id, crate::viewport::SpritesheetInfo::new(tw, th, tile_w, tile_h));
        vp.import_settings.insert(tex_id, settings);
    }

    /// Ensure a texture is loaded from the given path. If it's already loaded
    /// (matched by name in `imported_textures` or by path in `vp.texture_paths`),
    /// return the existing tex_id. Otherwise, load it and return the new tex_id.
    /// This centralises the repeated load-if-needed pattern used across the
    /// asset browser, double-click handler, and context menus.
    fn ensure_texture_loaded(&mut self, vp: &mut VP, path: &PathBuf, name: &str) -> Option<u64> {
        // Already tracked in imported_textures?
        if let Some(tex_info) = self.imported_textures.iter().find(|(_, n, _, _)| *n == name) {
            return Some(tex_info.0);
        }
        // Already loaded in vp but not yet in imported_textures?
        if let Some((&tex_id, _)) = vp.texture_paths.iter().find(|(_, p)| *p == path) {
            let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((0, 0));
            self.imported_textures.push((tex_id, name.to_string(), tw, th));
            return Some(tex_id);
        }
        // Actually load it
        let dev_queue = vp.ren_ctx.as_ref().map(|c| (c.dev.clone(), c.queue.clone()));
        if let Some((dev, queue)) = dev_queue {
            if let Ok(bytes) = std::fs::read(path) {
                if let Some(tex_id) = vp.import_and_slice_texture(&dev, &queue, &bytes, ImportSettings::default(), Some(path.clone())) {
                    let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((0, 0));
                    self.imported_textures.push((tex_id, name.to_string(), tw, th));
                    return Some(tex_id);
                }
            }
        }
        None
    }

    /// Add a scene path to the recent files list (deduplicated, max 10 entries).
    fn add_recent_file(&mut self, path: &PathBuf) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.clone());
        if self.recent_files.len() > 10 {
            self.recent_files.pop();
        }
    }

    fn check_entity(&self, vp: &VP, e: bevy_ecs::entity::Entity) -> bool {
        vp.world.get::<Transform>(e).is_some() || vp.world.get::<Sprite>(e).is_some() || vp.world.get::<Physics>(e).is_some()
    }

    /// Convert screen to world coords - corrected Y-axis: world Y increases downward
    fn screen_to_world(&self, screen: egui::Pos2, vr: &egui::Rect, vp: &VP) -> (f32, f32) {
        let a = vp.tex_size.0 as f32 / vp.tex_size.1 as f32;
        let (dw, dh) = if vr.width()/vr.height() > a { (vr.height()*a, vr.height()) } else { (vr.width(), vr.width()/a) };
        let left = vr.left() + (vr.width() - dw) / 2.0;
        let top = vr.top() + (vr.height() - dh) / 2.0;
        let w = vp.tex_size.0 as f32;
        let h = vp.tex_size.1 as f32;
        let vs = vp.view_scale;
        let (ox, oy) = vp.view_offset;
        // Camera: clip_y = wy*2*vs/h + (1-2*oy/h) → world Y=0 at TOP of screen, Y increases downward
        // ny = (clip_y+1)/2 = (wy*vs - oy)/h → ny=0 is TOP of viewport
        let nx = (screen.x - left) / dw;
        let ny = (screen.y - top) / dh;
        let wx = (nx * w - ox) / vs;
        let wy = (ny * h + oy) / vs;
        (wx, wy)
    }

    /// Get world position of selected entity (for gizmo center)
    fn get_gizmo_center(&self, vp: &VP) -> Option<(f32, f32)> {
        vp.selected.and_then(|e| vp.world.get::<Transform>(e).map(|t| (t.x, t.y)))
    }

    /// Convert world coords to egui screen position in the viewport
    fn world_to_screen(&self, wx: f32, wy: f32, vr: &egui::Rect, vp: &VP) -> egui::Pos2 {
        let a = vp.tex_size.0 as f32 / vp.tex_size.1 as f32;
        let (dw, dh) = if vr.width()/vr.height() > a { (vr.height()*a, vr.height()) } else { (vr.width(), vr.width()/a) };
        let left = vr.left() + (vr.width() - dw) / 2.0;
        let top = vr.top() + (vr.height() - dh) / 2.0;
        let w = vp.tex_size.0 as f32;
        let h = vp.tex_size.1 as f32;
        let vs = vp.view_scale;
        let (ox, oy) = vp.view_offset;
        // Camera: clip_y = wy*2*vs/h + (1-2*oy/h) → wy=0 at TOP of screen, Y increases downward
        let nx = (wx * vs + ox) / w;
        let ny = (wy * vs - oy) / h;
        egui::pos2(left + nx * dw, top + ny * dh)
    }

    pub fn show(&mut self, ctx: &egui::Context, vp: &mut VP) {
        // Validate selection — clear stale entities
        if let Some(e) = vp.selected {
            if !self.check_entity(vp, e) {
                vp.selected = None;
            }
        }

        // Menu bar
        TopBottomPanel::top("menu").show(ctx, |ui| { egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("🆕 New Project...").clicked() { self.show_create_project_dialog = true; self.new_scene_name.clear(); ui.close_menu(); }
                if ui.button("📂 Open Project...").clicked() { self.show_open_project_dialog = true; ui.close_menu(); }
                ui.separator();
                if ui.button("📂 Open Scene...").clicked() { self.show_open_file_dialog = true; ui.close_menu(); }
                if ui.button("💾 Save Scene").clicked() { 
                    if let Some(p) = vp.scene_path.clone() { let _ = vp.save_scene(&p); }
                    else { self.show_save_file_dialog = true; }
                    ui.close_menu(); 
                }
                if ui.button("💾 Save Scene As...").clicked() { self.show_save_file_dialog = true; ui.close_menu(); }
                ui.separator();
                if ui.button("🔄 New Scene").clicked() { 
                    for e in vp.world.query::<bevy_ecs::entity::Entity>().iter(&vp.world).collect::<Vec<_>>() { let _ = vp.world.despawn(e); } 
                    vp.scene_path = None; vp.refresh(); vp.selected = None; ui.close_menu(); 
                }
                ui.separator();
                // Recent files
                if !self.recent_files.is_empty() {
                    ui.label("📄 Recent Scenes:");
                    let mut open_recent: Option<PathBuf> = None;
                    for rf in &self.recent_files {
                        let label = rf.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if ui.button(&label).clicked() {
                            open_recent = Some(rf.clone());
                        }
                    }
                    if let Some(rf) = open_recent {
                        let _ = vp.load_scene(&rf);
                        ui.close_menu();
                    }
                    ui.separator();
                }
                if ui.button("🏗 Build & Export...").clicked() { self.show_build_dialog = true; ui.close_menu(); }
                if ui.button("📦 Export Project...").clicked() { self.show_export_dialog = true; ui.close_menu(); }
                ui.separator(); if ui.button("Exit").clicked() { std::process::exit(0); }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("↩ Undo (Ctrl+Z)").clicked() { vp.restore_undo(); ui.close_menu(); }
                if ui.button("↪ Redo (Ctrl+Shift+Z)").clicked() { vp.restore_redo(); ui.close_menu(); }
                ui.separator();
                if ui.button("📋 Copy (C)").clicked() { vp.copy_selected(); ui.close_menu(); }
                if ui.button("📋 Paste (V)").clicked() { vp.paste_entity(); ui.close_menu(); }
            });
            ui.menu_button("Tools", |ui| {
                if ui.button("Tilemap Editor").clicked() { self.tab = Tab::Tilemap; ui.close_menu(); }
                if ui.button("Animation Editor").clicked() { self.tab = Tab::Anim; ui.close_menu(); }
                if ui.button("Script Editor").clicked() { self.tab = Tab::Scripts; ui.close_menu(); }
                if ui.button("Scene View").clicked() { self.tab = Tab::Scene; ui.close_menu(); }
                if ui.button("Import Settings").clicked() { self.tab = Tab::Import; ui.close_menu(); }
                ui.separator();
                if ui.button("⚙️ Project Settings").clicked() { self.show_project_settings = true; ui.close_menu(); }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("About Pixgine").clicked() { log::info!("Pixgine v0.1"); ui.close_menu(); }
            });
        }); });

        // Toolbar
        TopBottomPanel::top("tool").show(ctx, |ui| { ui.horizontal(|ui| {
            ui.label("🎮 Pixgine"); ui.separator();
            if !vp.playing { 
                if ui.button("▶ Play").clicked() { vp.playing = true; vp.schedule = pixgine_engine::ecs::build_core_schedule(); }
            } else { 
                if ui.button("⏹ Stop").clicked() { vp.playing = false; }
            }
            ui.separator();
            if ui.button("↶").on_hover_text("Undo (Ctrl+Z)").clicked() { vp.restore_undo(); }
            if ui.button("↷").on_hover_text("Redo (Ctrl+Shift+Z)").clicked() { vp.restore_redo(); }
            ui.separator();
            // Transform mode buttons
            if ui.selectable_label(vp.transform_mode == TransformMode::Move, "✚ Move (W)").clicked() { vp.transform_mode = TransformMode::Move; }
            if ui.selectable_label(vp.transform_mode == TransformMode::Rotate, "⟳ Rot (E)").clicked() { vp.transform_mode = TransformMode::Rotate; }
            if ui.selectable_label(vp.transform_mode == TransformMode::Scale, "⤡ Scale (R)").clicked() { vp.transform_mode = TransformMode::Scale; }
            ui.separator();
            // Grid & Snap controls
            if ui.selectable_label(vp.show_grid, "🌐 Grid").clicked() { vp.show_grid = !vp.show_grid; }
            if ui.selectable_label(vp.snap_to_grid, "🧲 Snap").clicked() { vp.snap_to_grid = !vp.snap_to_grid; }
            if ui.selectable_label(vp.pixel_perfect, "👾 Pixel").on_hover_text("Pixel Perfect Crisp Rendering").clicked() { vp.pixel_perfect = !vp.pixel_perfect; }
            
            egui::ComboBox::from_id_salt("grid_size_combo")
                .selected_text(format!("{}px", vp.grid_size))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut vp.grid_size, 8, "8x8");
                    ui.selectable_value(&mut vp.grid_size, 16, "16x16");
                    ui.selectable_value(&mut vp.grid_size, 32, "32x32");
                    ui.selectable_value(&mut vp.grid_size, 64, "64x64");
                });

            ui.separator();
            // View controls
            ui.label(format!("🔍 {:.0}%", vp.view_scale * 100.0));
            if ui.button("100%").clicked() { vp.view_scale = 1.0; vp.view_offset = (0.0, 0.0); vp.target_view_scale = 1.0; vp.target_view_offset = (0.0, 0.0); }
            if ui.button("200%").clicked() { vp.view_scale = 2.0; vp.view_offset = (0.0, 0.0); vp.target_view_scale = 2.0; vp.target_view_offset = (0.0, 0.0); }
            if ui.button("300%").clicked() { vp.view_scale = 3.0; vp.view_offset = (0.0, 0.0); vp.target_view_scale = 3.0; vp.target_view_offset = (0.0, 0.0); }
            if ui.button("400%").clicked() { vp.view_scale = 4.0; vp.view_offset = (0.0, 0.0); vp.target_view_scale = 4.0; vp.target_view_offset = (0.0, 0.0); }
            if ui.button("800%").clicked() { vp.view_scale = 8.0; vp.view_offset = (0.0, 0.0); vp.target_view_scale = 8.0; vp.target_view_offset = (0.0, 0.0); }
            if ui.button("➕").clicked() {
                if vp.pixel_perfect {
                    // Step to next pixel-perfect scale (integers up, 1/n down)
                    let old = vp.view_scale;
                    if old >= 1.0 {
                        vp.view_scale = (old + 1.0).min(20.0);
                    } else {
                        let n = (1.0 / old).round().max(2.0);
                        vp.view_scale = (1.0 / (n - 1.0)).max(1.0);
                    }
                    vp.target_view_scale = vp.view_scale;
                } else {
                    vp.view_scale = (vp.view_scale * 1.25).min(20.0);
                    vp.target_view_scale = vp.view_scale;
                }
                vp.view_offset = (vp.view_offset.0.round(), vp.view_offset.1.round());
                vp.target_view_offset = vp.view_offset;
            }
            if ui.button("➖").clicked() {
                if vp.pixel_perfect {
                    // Step to next pixel-perfect scale (1/n fractions down)
                    let old = vp.view_scale;
                    if old <= 1.0 {
                        let n = (1.0 / old).round().max(1.0);
                        vp.view_scale = (1.0 / (n + 1.0)).max(0.05);
                    } else {
                        vp.view_scale = (old - 1.0).max(1.0);
                    }
                    vp.target_view_scale = vp.view_scale;
                } else {
                    vp.view_scale = (vp.view_scale / 1.25).max(0.1);
                    vp.target_view_scale = vp.view_scale;
                }
                vp.view_offset = (vp.view_offset.0.round(), vp.view_offset.1.round());
                vp.target_view_offset = vp.view_offset;
            }
            if ui.button("⟲ Reset").clicked() { vp.view_scale = 1.0; vp.view_offset = (0.0, 0.0); vp.target_view_scale = 1.0; vp.target_view_offset = (0.0, 0.0); }
            ui.separator();
            ui.label("🖱 Right-drag=Pan | Scroll=Zoom");
            ui.separator(); 
            ui.label(format!("📐 {}x{}", vp.tex_size.0, vp.tex_size.1));
            if !vp.entities.is_empty() { ui.label(format!("📦 {} ents", vp.entities.len())); }
            if let Some(p) = &vp.scene_path { 
                ui.separator(); 
                let name = p.file_stem().unwrap_or_default().to_string_lossy();
                ui.label(format!("📄 {}", name)); 
            }
            if let Some(pp) = &self.project_path {
                ui.separator();
                ui.label(format!("🏗 {}", pp.file_name().unwrap_or_default().to_string_lossy()));
            }
        }); });

        // Tabs
        TopBottomPanel::top("tabs").show(ctx, |ui| { ui.horizontal(|ui| {
            let tabs: [(&str, Tab, &str); 5] = [
                ("Scene", Tab::Scene, "📋"),
                ("Tilemap", Tab::Tilemap, "🧱"),
                ("Anim", Tab::Anim, "🎬"),
                ("Scripts", Tab::Scripts, "📜"),
                ("Import", Tab::Import, "📥"),
            ];
            for (l, t, icon) in tabs.iter() {
                if ui.selectable_label(self.tab == *t, format!("{} {}", icon, l)).clicked() { self.tab = *t; }
            }
        }); });

        // Left panel: Scene Tree / Tilemap / etc
        SidePanel::left("left").resizable(true).default_width(220.0).min_width(120.0).max_width(450.0).show(ctx, |ui| {
            match self.tab {
                Tab::Scene => {
                    ui.horizontal(|ui| {
                        ui.heading("📋 Scene");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("➕").on_hover_text("Add Entity").clicked() { vp.add(); }
                            if ui.button("🗑").on_hover_text("Delete Selected").clicked() { if let Some(e) = vp.selected { vp.remove(e); } }
                        });
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui, |ui| {
                        let ents = vp.entities.clone();
                        let _delete_target: Option<Entity> = None;
                        let mut move_up_target: Option<usize> = None;
                        let mut move_down_target: Option<usize> = None;
                        for (idx, info) in ents.iter().enumerate() {
                            let is_sel = vp.selected == Some(info.entity);
                            
                            // Check parent-child relationship for indentation
                            let indent = if vp.world.get::<Parent>(info.entity).is_some() { 12.0 } else { 0.0 };
                            
                            ui.horizontal(|ui| {
                                ui.add_space(indent);
                                
                                if self.renaming_entity == Some(info.entity) {
                                    ui.label("📝");
                                    ui.text_edit_singleline(&mut self.rename_buffer);
                                    let done = ui.button("✓").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    if done {
                                        if let Some(ei) = vp.entities.iter_mut().find(|e| e.entity == info.entity) {
                                            if !self.rename_buffer.is_empty() { 
                                                ei.name = self.rename_buffer.clone();
                                                vp.entity_names.insert(info.entity, self.rename_buffer.clone());
                                            }
                                        }
                                        self.renaming_entity = None;
                                    }
                                } else {
                                    let icon = if vp.world.get::<CameraTag>(info.entity).is_some() { "📷" }
                                               else if vp.world.get::<ParticleEmitter>(info.entity).is_some() { "✨" }
                                               else if vp.world.get::<AudioSource>(info.entity).is_some() { "🔊" }
                                               else if vp.world.get::<Sprite>(info.entity).is_some() { "🟦" } 
                                               else if vp.world.get::<Physics>(info.entity).is_some() { "⚙️" }
                                               else { "⬜" };
                                    let label = format!("{} {}", icon, info.name);
                                    let resp = ui.selectable_label(is_sel, &label);
                                    if resp.clicked() {
                                        vp.selected = Some(info.entity);
                                    }
                                    // Drag-to-reorder: track drag source
                                    if resp.is_pointer_button_down_on() {
                                        self.drag_entity_idx = Some(idx);
                                    }
                                    
                                    if is_sel && ui.ui_contains_pointer() && ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                                        self.renaming_entity = Some(info.entity);
                                        self.rename_buffer = info.name.clone();
                                    }
                                    
                                    // Up/Down arrows for reordering
                                    if idx > 0 && ui.button("▲").on_hover_text("Move Up").clicked() {
                                        move_up_target = Some(idx);
                                    }
                                    if idx + 1 < ents.len() && ui.button("▼").on_hover_text("Move Down").clicked() {
                                        move_down_target = Some(idx);
                                    }
                                    // Parent button
                                    if ui.button("🔗").on_hover_text("Set Parent").clicked() {
                                        self.show_parent_selector = true;
                                        self.parent_entity = Some(info.entity);
                                    }
                                    // Remove parent button if has parent
                                    if vp.world.get::<Parent>(info.entity).is_some() {
                                        if ui.button("🔓").on_hover_text("Remove Parent").clicked() {
                                            vp.set_parent(info.entity, None);
                                        }
                                    }
                                }
                            });
                        }
                        // Apply reordering
                        if let Some(idx) = move_up_target { vp.move_entity_up(idx); }
                        if let Some(idx) = move_down_target { vp.move_entity_down(idx); }
                        // Handle drag drop for reordering
                        if let Some(src_idx) = self.drag_entity_idx {
                            if let Some(dst_idx) = self.drag_target_idx {
                                if src_idx != dst_idx {
                                    // Move entity in list
                                    // (drag-drop simplified - just use buttons for now)
                                }
                            }
                        }
                        if !ui.ui_contains_pointer() || !ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) {
                            self.drag_entity_idx = None;
                            self.drag_target_idx = None;
                        }
                    });
                    ui.add_space(4.0);
                    if ui.button("➕ Empty Entity").clicked() { vp.save_undo(); vp.add(); }
                    if ui.button("📷 Camera Entity").clicked() { 
                        vp.save_undo();
                        let e = vp.world.spawn((Transform::new(160.0, 90.0), CameraTag)).id();
                        vp.entity_names.insert(e, format!("Camera_{}", vp.entities.len()));
                        vp.refresh(); vp.selected = Some(e);
                    }
                    if ui.button("✨ Particle Emitter").clicked() { 
                        vp.save_undo();
                        let e = vp.world.spawn((Transform::new(160.0, 90.0), ParticleEmitter::default())).id();
                        vp.entity_names.insert(e, format!("Emitter_{}", vp.entities.len()));
                        vp.refresh(); vp.selected = Some(e);
                    }
                    if ui.button("🔊 Audio Source").clicked() { 
                        vp.save_undo();
                        let e = vp.world.spawn((Transform::new(160.0, 90.0), AudioSource::default())).id();
                        vp.entity_names.insert(e, format!("Audio_{}", vp.entities.len()));
                        vp.refresh(); vp.selected = Some(e);
                    }
                    if ui.button("🏃 Moving Entity").clicked() { 
                        vp.save_undo();
                        let e = vp.world.spawn((Transform::new(160.0, 90.0), Sprite::default(), Velocity::default())).id();
                        vp.entity_names.insert(e, format!("Moving_{}", vp.entities.len()));
                        vp.refresh(); vp.selected = Some(e);
                    }
                    if ui.button("🎮 Player Entity").clicked() { 
                        vp.save_undo();
                        let e = vp.world.spawn((Transform::new(160.0, 90.0), Sprite::default(), Player)).id();
                        vp.entity_names.insert(e, format!("Player_{}", vp.entities.len()));
                        vp.refresh(); vp.selected = Some(e);
                    }
                }
                Tab::Tilemap => {
                    ui.horizontal(|ui| {
                        ui.heading("🧱 Tiles");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🖌").on_hover_text("Paint").clicked() { vp.sel_tile = vp.sel_tile.max(1); }
                            if ui.button("🗑").on_hover_text("Erase").clicked() { vp.sel_tile = 0; }
                        });
                    });
                    ui.separator();
                    // --- Global tilesheet picker (Fix 1) ---
                    // This is the fallback tilesheet used when layers don't have their own
                    let tex_list: Vec<(u64, u32, u32)> = vp.textures.iter().map(|(id, (_, w, h, _))| (*id, *w, *h)).collect();
                    ui.horizontal(|ui| {
                        ui.label("Tilesheet:");
                        let cur_label = vp.tilesheet_tex_id.map(|id| format!("Tex{}", id)).unwrap_or_else(|| "none".into());
                        egui::ComboBox::from_id_salt("global_tilesheet_tex")
                            .selected_text(&cur_label)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(false, "none").clicked() {
                                    vp.tilesheet_tex_id = None;
                                }
                                for &(id, tw, th) in &tex_list {
                                    let label = format!("Tex{} ({}x{})", id, tw, th);
                                    if ui.selectable_label(false, &label).clicked() {
                                        vp.tilesheet_tex_id = Some(id);
                                        let auto_tile = crate::viewport::auto_tile_size(tw, th);
                                        vp.tilesheet_cols = tw / auto_tile;
                                        vp.tilesheet_rows = th / auto_tile;
                                        vp.tilesheet_tile_w = auto_tile;
                                        vp.tilesheet_tile_h = auto_tile;
                                    }
                                }
                            });
                    });
                    // --- END Fix 1 ---
                    // Show tilesheet tiles with actual images or color palette
                    if let Some(tid) = vp.tilesheet_tex_id {
                        if let Some((ref _tv, tw, th, egui_tex_opt)) = vp.textures.get(&tid) {
                            // Show BOTH tile size and columns/rows controls
                            ui.horizontal(|ui| {
                                ui.label("Cols:");
                                ui.add(egui::DragValue::new(&mut vp.tilesheet_cols).range(1..=1024).speed(1));
                                ui.label("Rows:");
                                ui.add(egui::DragValue::new(&mut vp.tilesheet_rows).range(1..=1024).speed(1));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Tile W(px):");
                                ui.add(egui::DragValue::new(&mut vp.tilesheet_tile_w).range(1..=512).speed(1));
                                ui.label("H(px):");
                                ui.add(egui::DragValue::new(&mut vp.tilesheet_tile_h).range(1..=512).speed(1));
                            });
                            if ui.button("🔄 Re-slice from size").clicked() {
                                vp.tilesheet_cols = tw / vp.tilesheet_tile_w;
                                vp.tilesheet_rows = th / vp.tilesheet_tile_h;
                            }
                            // Show all tiles - scrollable if needed
                            let cell_size = 32.0;
                            let cols_avail = ((ui.available_width() - 4.0) / (cell_size + 4.0)).max(1.0) as usize;
                            let ncols = vp.tilesheet_cols.min(cols_avail as u32);
                            
                            // Scrollable tilesheet palette with actual tile images
                            egui::ScrollArea::both().auto_shrink([false;2]).max_height(300.0).show(ui, |ui| {
                                egui::Grid::new("tilesheet_grid2").min_col_width(cell_size+4.0).show(ui, |ui| {
                                    for row in 0..vp.tilesheet_rows {
                                        for col in 0..vp.tilesheet_cols {
                                            let idx = row * vp.tilesheet_cols + col;
                                            let is_sel = vp.sel_tile == (idx + 1) as usize;
                                            let (u0, v0, u1, v1) = crate::viewport::SpritesheetInfo::new(*tw, *th, vp.tilesheet_tile_w, vp.tilesheet_tile_h).uv_for_tile(idx);
                                            let r = ui.add_sized(egui::vec2(cell_size, cell_size), egui::Button::new("").fill(if is_sel { egui::Color32::YELLOW } else { egui::Color32::DARK_GRAY }));
                                            if r.clicked() { vp.sel_tile = (idx + 1) as usize; }
                                            // Draw the tile thumbnail using the registered egui texture
                                            if let Some(egui_tex) = egui_tex_opt {
                                                let uv_rect = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
                                                ui.painter().image(*egui_tex, r.rect.shrink(1.0), uv_rect, egui::Color32::WHITE);
                                            }
                                            if (col + 1) % ncols == 0 { ui.end_row(); }
                                        }
                                    }
                                });
                            });
                        } else {
                            ui.label("Tilesheet loaded but texture missing");
                        }
                        if ui.button("❌ Clear Tilesheet").clicked() {
                            vp.tilesheet_tex_id = None;
                            vp.tilesheet_cols = 1;
                            vp.tilesheet_rows = 1;
                            vp.tilesheet_tile_w = 16;
                            vp.tilesheet_tile_h = 16;
                        }
                    } else {
                    // Color palette (scrollable)
                        let tile_size = 32.0;
                        let cols_avail = (ui.available_width() / (tile_size + 4.0)).max(1.0) as usize;
                        egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui, |ui| {
                            egui::Grid::new("palette_grid").min_col_width(tile_size + 4.0).show(ui, |ui| {
                                for (i, c) in vp.palette.iter().enumerate() {
                                    let col: Color32 = Rgba::from_rgba_unmultiplied(c.0, c.1, c.2, c.3).into();
                                    let is_sel = vp.sel_tile == i + 1;
                                    let bg = if is_sel { Color32::WHITE } else { Color32::DARK_GRAY };
                                    let r = ui.add_sized(egui::vec2(tile_size, tile_size), egui::Button::new(""));
                                    if r.clicked() { vp.sel_tile = i + 1; }
                                    ui.painter().rect_filled(r.rect, 0.0, col);
                                    if (i + 1) % cols_avail == 0 { ui.end_row(); }
                                }
                            });
                        });
                    }
                    ui.add_space(8.0); 
                    ui.label("Layers:");
                    let mut del_idx: Option<usize> = None;
                    for i in 0..vp.tile_layers.len() {
                        ui.horizontal(|ui| {
                            let mut v = vp.tile_layers[i].vis;
                            ui.checkbox(&mut v, "");
                            vp.tile_layers[i].vis = v;
                            if self.renaming_layer == Some(i) {
                                ui.text_edit_singleline(&mut self.layer_rename_buffer);
                                if ui.button("✓").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    if !self.layer_rename_buffer.is_empty() { vp.tile_layers[i].name = self.layer_rename_buffer.clone(); }
                                    self.renaming_layer = None;
                                }
                            } else {
                                if ui.label(&vp.tile_layers[i].name).double_clicked() {
                                    self.renaming_layer = Some(i);
                                    self.layer_rename_buffer = vp.tile_layers[i].name.clone();
                                }
                            }
                            // Spritesheet selection for this layer
                            let layer_tex_id = vp.tile_layers[i].spritesheet_tex_id;
                            let tex_list: Vec<(u64, u32, u32)> = vp.textures.iter().map(|(id, (_, w, h, _))| (*id, *w, *h)).collect();
                            ui.horizontal(|ui| {
                                ui.label("Tex:");
                                let cur_label = layer_tex_id.map(|id| format!("Tex{}", id)).unwrap_or_else(|| "none".into());
                                egui::ComboBox::from_id_salt(format!("layer_tex_{}", i))
                                    .selected_text(&cur_label)
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(false, "none").clicked() {
                                            vp.tile_layers[i].spritesheet_tex_id = None;
                                        }
                                        for &(id, _, _) in &tex_list {
                                            let label = format!("Tex{}", id);
                                            if ui.selectable_label(false, &label).clicked() {
                                                vp.tile_layers[i].spritesheet_tex_id = Some(id);
                                            }
                                        }
                                    });
                                // Z-index control
                                ui.label("Z:");
                                ui.add(egui::DragValue::new(&mut vp.tile_layers[i].z_index).speed(1));
                            });
                            // Tile picker with actual texture previews for this layer
                            // Use layer's own spritesheet if set, otherwise fall back to global tilesheet
                            let effective_tex_id = layer_tex_id.or(vp.tilesheet_tex_id);
                            if let Some(tex_id) = effective_tex_id {
                                if let Some(info) = vp.spritesheet_info.get(&tex_id) {
                                    if info.tile_count() > 1 {
                                        let egui_tex_opt = vp.textures.get(&tex_id).and_then(|(_, _, _, opt)| *opt);
                                        ui.horizontal(|ui| {
                                            ui.label("Tiles:");
                                            let cell_sz = 20.0f32;
                                            let cols_avail = ((ui.available_width() - 4.0) / (cell_sz + 2.0)).max(1.0) as u32;
                                            let display_cols = info.cols.min(cols_avail).min(10);
                                            let display_rows = info.rows.min(4);
                                            let max_show = (display_cols * display_rows).min(info.tile_count());
                                            egui::Grid::new(format!("layer_tile_grid_{}", i)).min_col_width(cell_sz + 2.0).show(ui, |ui| {
                                                let mut idx = 0u32;
                                                for _ in 0..display_rows {
                                                    for _ in 0..display_cols {
                                                        if idx >= max_show { break; }
                                                        let is_sel = vp.sel_tile == (idx + 1) as usize;
                                                        let (u0, v0, u1, v1) = info.uv_for_tile(idx);
                                                        let btn = ui.add_sized(egui::vec2(cell_sz, cell_sz), 
                                                            egui::Button::new("").fill(if is_sel { egui::Color32::YELLOW } else { egui::Color32::DARK_GRAY }));
                                                        if btn.clicked() {
                                                            vp.sel_tile = (idx + 1) as usize;
                                                        }
                                                        // Draw tile thumbnail
                                                        if let Some(egui_tex) = egui_tex_opt {
                                                            let uv_rect = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
                                                            ui.painter().image(egui_tex, btn.rect.shrink(1.0), uv_rect, egui::Color32::WHITE);
                                                        }
                                                        idx += 1;
                                                        if idx % display_cols == 0 { ui.end_row(); }
                                                    }
                                                    if idx >= max_show { break; }
                                                }
                                                // Show all tiles - no "+4 more" message
                                            });
                                        });
                                    }
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✏️").on_hover_text("Rename").clicked() {
                                    self.renaming_layer = Some(i);
                                    self.layer_rename_buffer = vp.tile_layers[i].name.clone();
                                }
                                if ui.button("❌").on_hover_text("Delete").clicked() && vp.tile_layers.len() > 1 { del_idx = Some(i); }
                            });
                        });
                    }
                    if let Some(idx) = del_idx { vp.tile_layers.remove(idx); }
                    if ui.button("➕ Layer").clicked() { 
                        vp.tile_layers.push(crate::viewport::TL { 
                            name: format!("Layer {}", vp.tile_layers.len()), vis: true, 
                            tiles: vec![vec![0;20];11], cols: 20, rows: 11, ts: 16, z_index: 0, spritesheet_tex_id: None 
                        }); 
                    }
                }
                Tab::Anim => { 
                    ui.heading("🎬 Animation"); ui.separator(); 
                    ui.label("Frame:"); ui.add(egui::Slider::new(&mut vp.anim_frame, 0..=60)); 
                    ui.horizontal(|ui| {
                        if ui.button("▶").clicked() { vp.anim_timer = 0.0; }
                        if ui.button("⏹").clicked() { vp.anim_timer = -1.0; }
                    });
                }
                Tab::Scripts => { 
                    ui.heading("📜 Scripts"); ui.separator(); 
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let scripts = vp.scripts.clone();
                        for s in &scripts { 
                            ui.horizontal(|ui| {
                                if ui.button("▶").clicked() { log::info!("Run script: {}", s); }
                                ui.label(s); 
                                if ui.button("❌").clicked() { vp.scripts.retain(|x| x != s); }
                            });
                        }
                    });
                    if ui.button("➕ Load Script").clicked() { vp.scripts.push("main.lua".into()); }
                    ui.separator();
                    ui.label("🖥️ Lua Console");
                    let console_height = ui.available_height().min(200.0).max(40.0);
                    egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui, |ui| {
                        ui.set_min_height(console_height);
                        let log = &vp.lua_log;
                        if log.is_empty() {
                            ui.weak("(console output appears here)");
                        } else {
                            for line in log.iter().rev().take(50) {
                                ui.label(line);
                            }
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let resp = ui.text_edit_singleline(&mut self.lua_input);
                        if ui.button("⏎").clicked() || resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let cmd = self.lua_input.trim().to_string();
                            if !cmd.is_empty() {
                                vp.lua_log.push(format!("> {}", cmd));
                                // For now, just echo - actual Lua exec would use ScriptEngine
                                vp.lua_log.push(format!("[info] Lua execution not available in editor"));
                                self.lua_input.clear();
                            }
                        }
                    });
                }
                Tab::Import => {
                    ui.heading("📥 Import Settings"); ui.separator();
                    ui.label("Configure how image assets in this folder are imported:");
                    ui.add_space(4.0);
                    // Scan the asset directory for image files so that PNGs
                    // that haven't been loaded yet still appear in the list.
                    let mut image_files: Vec<(String, PathBuf)> = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(&self.asset_path) {
                        for entry in entries.flatten() {
                            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                                let name = entry.file_name().to_string_lossy().to_string();
                                if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") {
                                    image_files.push((name, entry.path()));
                                }
                            }
                        }
                    }
                    image_files.sort_by(|a, b| a.0.cmp(&b.0));
                    if image_files.is_empty() {
                        ui.weak("(No image files found in the current folder)");
                        ui.add_space(8.0);
                        ui.label("💡 Tip: Add images to the asset folder, then select");
                        ui.label("   them here to configure import settings (Texture / Spritesheet).");
                    } else {
                        egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui, |ui| {
                            for (name, path) in &image_files {
                                // Check if this image is already loaded as a texture
                                let loaded_tex_id = self.imported_textures.iter()
                                    .find(|(_, n, _, _)| *n == *name)
                                    .map(|(id, _, _, _)| *id);
                                let is_sel = self.selected_import_tex == loaded_tex_id;
                                let (tw, th) = loaded_tex_id
                                    .and_then(|id| vp.textures.get(&id))
                                    .map(|(_, w, h, _)| (*w, *h))
                                    .unwrap_or((0, 0));
                                let settings = loaded_tex_id
                                    .and_then(|id| vp.import_settings.get(&id))
                                    .cloned()
                                    .unwrap_or_else(ImportSettings::default);
                                let tile_info = if loaded_tex_id.is_some() && settings.texture_type == TextureType::Spritesheet {
                                    format!(" | {}x{} tiles", settings.cols(tw), settings.rows(th))
                                } else {
                                    String::new()
                                };
                                let status = if loaded_tex_id.is_some() {
                                    format!(" ({}x{}){}", tw, th, tile_info)
                                } else {
                                    " (not imported — click to load)".to_string()
                                };
                                let label = format!("{}{}", name, status);
                                let r = ui.selectable_label(is_sel, &label);
                                if r.clicked() {
                                    if loaded_tex_id.is_none() {
                                        // Load the texture on demand
                                        if let Some(tex_id) = self.ensure_texture_loaded(vp, path, name) {
                                            self.selected_import_tex = Some(tex_id);
                                        }
                                    } else {
                                        self.selected_import_tex = loaded_tex_id;
                                    }
                                }
                                r.context_menu(|ui| {
                                    if ui.button("⚙️ Import Settings").clicked() {
                                        if let Some(tex_id) = self.ensure_texture_loaded(vp, path, name) {
                                            self.import_settings_tex = Some(tex_id);
                                            self.edit_import_settings = vp.import_settings.get(&tex_id).cloned()
                                                .unwrap_or_else(ImportSettings::default);
                                            self.selected_import_tex = Some(tex_id);
                                        }
                                        ui.close_menu();
                                    }
                                    if ui.button("🔄 Re-import").clicked() {
                                        if let Some(tex_id) = self.ensure_texture_loaded(vp, path, name) {
                                            let s = vp.import_settings.get(&tex_id).cloned()
                                                .unwrap_or_else(ImportSettings::default);
                                            let _ = vp.reimport_texture(tex_id, s);
                                        }
                                        ui.close_menu();
                                    }
                                });
                            }
                        });
                    }
                    ui.add_space(8.0);
                    // Show detailed settings for selected texture
                    if let Some(tex_id) = self.selected_import_tex {
                        // Clone all data we need from vp before the closure to avoid borrow conflicts
                        let tex_data = vp.textures.get(&tex_id).map(|(_, w, h, egui_tex)| (*w, *h, *egui_tex));
                        let settings = vp.import_settings.get(&tex_id).cloned()
                            .unwrap_or_else(ImportSettings::default);
                        if let Some((tw, th, egui_tex_opt)) = tex_data {
                            Frame::group(ui.style()).show(ui, |ui| {
                                ui.label("🔧 Import Settings");
                                ui.separator();
                                // Texture preview thumbnail
                                let max_size = 120.0;
                                let scale = (tw as f32 / max_size).max(th as f32 / max_size).max(1.0);
                                let display_w = (tw as f32 / scale).min(max_size);
                                let display_h = (th as f32 / scale).min(max_size);
                                if let Some(egui_tex) = egui_tex_opt {
                                    ui.add_sized(egui::vec2(display_w, display_h), egui::Image::new(egui::load::SizedTexture::new(egui_tex, egui::vec2(display_w, display_h))));
                                }
                                ui.add_space(4.0);
                                // Texture type selector
                                ui.label("Texture Type:");
                                egui::ComboBox::from_id_salt("import_type_combo")
                                    .selected_text(settings.texture_type.as_str())
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(false, "Texture").clicked() {
                                            let mut new_settings = settings.clone();
                                            new_settings.texture_type = TextureType::Texture;
                                            self.apply_import_settings(vp, tex_id, new_settings);
                                        }
                                        if ui.selectable_label(false, "Spritesheet").clicked() {
                                            let mut new_settings = settings.clone();
                                            new_settings.texture_type = TextureType::Spritesheet;
                                            if new_settings.tile_width == 0 || new_settings.tile_height == 0 {
                                                let auto = ImportSettings::auto_tile_size(tw, th);
                                                new_settings.tile_width = auto;
                                                new_settings.tile_height = auto;
                                            }
                                            self.apply_import_settings(vp, tex_id, new_settings);
                                        }
                                    });
                                ui.add_space(4.0);
                                // Tile dimensions (only for spritesheet)
                                if settings.texture_type == TextureType::Spritesheet {
                                    ui.label("Tile Size:");
                                    ui.horizontal(|ui| {
                                        ui.label("W:");
                                        let mut tw_val = settings.tile_width;
                                        ui.add(egui::DragValue::new(&mut tw_val).range(1..=512).speed(1));
                                        ui.label("px");
                                        ui.label("H:");
                                        let mut th_val = settings.tile_height;
                                        ui.add(egui::DragValue::new(&mut th_val).range(1..=512).speed(1));
                                        ui.label("px");
                                        if tw_val != settings.tile_width || th_val != settings.tile_height {
                                            let mut new_settings = settings.clone();
                                            new_settings.tile_width = tw_val;
                                            new_settings.tile_height = th_val;
                                            self.apply_import_settings(vp, tex_id, new_settings);
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Grid: {}x{} ({} tiles)", settings.cols(tw), settings.rows(th), settings.tile_count(tw, th)));
                                        if ui.button("🔧 Auto-detect").clicked() {
                                            let auto = ImportSettings::auto_tile_size(tw, th);
                                            let mut new_settings = settings.clone();
                                            new_settings.tile_width = auto;
                                            new_settings.tile_height = auto;
                                            self.apply_import_settings(vp, tex_id, new_settings);
                                        }
                                    });
                                }
                                ui.add_space(4.0);
                                // Filter mode
                                ui.label("Filter:");
                                egui::ComboBox::from_id_salt("import_filter_combo")
                                    .selected_text(match settings.filter {
                                        TextureFilter::Nearest => "Nearest (pixel art)",
                                        TextureFilter::Linear => "Linear (smooth)",
                                    })
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(false, "Nearest (pixel art)").clicked() {
                                            let mut new_settings = settings.clone();
                                            new_settings.filter = TextureFilter::Nearest;
                                            self.apply_import_settings(vp, tex_id, new_settings);
                                        }
                                        if ui.selectable_label(false, "Linear (smooth)").clicked() {
                                            let mut new_settings = settings.clone();
                                            new_settings.filter = TextureFilter::Linear;
                                            self.apply_import_settings(vp, tex_id, new_settings);
                                        }
                                    });
                                ui.add_space(4.0);
                                if ui.button("🔄 Re-import").clicked() {
                                    let _ = vp.reimport_texture(tex_id, settings.clone());
                                }
                            });
                        }
                    }
                }
            }
        });
        SidePanel::right("right").resizable(true).default_width(280.0).min_width(140.0).max_width(550.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("🔍 Inspector");
                    if let Some(e) = vp.selected {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑").on_hover_text("Delete Entity").clicked() { vp.remove(e); }
                        });
                    }
                });
                ui.separator();
                if let Some(e) = vp.selected {
                    let info_idx = vp.entities.iter().position(|i| i.entity == e);
                    if self.renaming_entity == Some(e) {
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.rename_buffer);
                            if ui.button("✓").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if let Some(idx) = info_idx {
                                    if !self.rename_buffer.is_empty() { vp.entities[idx].name = self.rename_buffer.clone(); }
                                }
                                self.renaming_entity = None;
                            }
                        });
                    } else {
                        let ent_name = info_idx.and_then(|idx| vp.entities.get(idx)).map(|i| i.name.clone()).unwrap_or_else(|| "<unknown>".to_string());
                        ui.horizontal(|ui| {
                            ui.label(format!("Entity: {}", ent_name));
                            if ui.button("✏️").clicked() {
                                self.renaming_entity = Some(e);
                                self.rename_buffer = vp.entities.get(info_idx.unwrap_or(0)).map(|i| i.name.clone()).unwrap_or_default();
                            }
                        });
                    }
                    ui.label(format!("ID: {:?}", e));
                    ui.add_space(6.0);

                    // Transform
                    if let Some(t) = vp.world.get::<Transform>(e) {
                        let mut tv = (t.x, t.y, t.rotation, t.scale_x, t.scale_y);
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("📐 Transform");
                            ui.horizontal(|ui| { ui.label("X:"); ui.add(egui::DragValue::new(&mut tv.0).speed(1.0).range(-10000.0..=10000.0)); 
                                                  ui.label("Y:"); ui.add(egui::DragValue::new(&mut tv.1).speed(1.0).range(-10000.0..=10000.0)); });
                            ui.horizontal(|ui| { ui.label("Rot:"); ui.add(egui::DragValue::new(&mut tv.2).speed(0.05).range(-6.28..=6.28).suffix(" rad")); 
                                                  if ui.button("⟲ 0").clicked() { tv.2 = 0.0; } });
                            ui.horizontal(|ui| { ui.label("SX:"); ui.add(egui::DragValue::new(&mut tv.3).speed(0.1).range(0.01..=100.0)); 
                                                  ui.label("SY:"); ui.add(egui::DragValue::new(&mut tv.4).speed(0.1).range(0.01..=100.0)); });
                        });
                        if let Some(mut w) = vp.world.get_mut::<Transform>(e) { w.x = tv.0; w.y = tv.1; w.rotation = tv.2; w.scale_x = tv.3; w.scale_y = tv.4; }
                    } else {
                        if ui.button("➕ Add Transform").clicked() { vp.world.entity_mut(e).insert(Transform::default()); }
                    }
                    ui.add_space(4.0);

                    // Sprite
                    let has_sprite = vp.world.get::<Sprite>(e).is_some();
                    let sprite_visible = vp.world.get::<Sprite>(e).map(|s| s.visible).unwrap_or(false);
                    let sprite_layer = vp.world.get::<Sprite>(e).map(|s| s.layer).unwrap_or(0);
                    if has_sprite {
                        let cur_tex_id = vp.world.get::<Sprite>(e).and_then(|s| s.texture_id);
                        let cur_sx = vp.world.get::<Sprite>(e).map(|s| s.source_x).unwrap_or(0);
                        let cur_sy = vp.world.get::<Sprite>(e).map(|s| s.source_y).unwrap_or(0);
                        // Clone texture list to avoid borrow conflicts
                        let tex_list: Vec<(u64, u32, u32)> = vp.textures.iter().map(|(id, (_, w, h, _))| (*id, *w, *h)).collect();
                        let sprite_info = vp.spritesheet_info.clone();
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🟦 Sprite");
                            let mut vis = sprite_visible;
                            ui.checkbox(&mut vis, "Visible");
                            if vis != sprite_visible { if let Some(mut s) = vp.world.get_mut::<Sprite>(e) { s.visible = vis; } }
                            let mut sl = sprite_layer;
                            ui.horizontal(|ui| { ui.label("Layer:"); ui.add(egui::DragValue::new(&mut sl).speed(1)); });
                            if sl != sprite_layer { if let Some(mut s) = vp.world.get_mut::<Sprite>(e) { s.layer = sl; } }
                            if !tex_list.is_empty() {
                                // Texture picker
                                ui.horizontal(|ui| {
                                    ui.label("Tex:");
                                    let cur_label = cur_tex_id.map(|id| format!("Tex{}", id)).unwrap_or_else(|| "none".into());
                                    egui::ComboBox::from_id_salt("tex_combo")
                                        .selected_text(&cur_label)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_label(false, "none").clicked() {
                                                if let Some(mut s) = vp.world.get_mut::<Sprite>(e) { s.texture_id = None; }
                                            }
                                            for &(id, tw, th) in &tex_list {
                                                let info = sprite_info.get(&id).cloned().unwrap_or_else(|| crate::viewport::SpritesheetInfo::new(tw, th, tw, th));
                                                let label = if info.tile_count() > 1 {
                                                    format!("Tex{} ({}x{}, {} tiles)", id, tw, th, info.tile_count())
                                                } else {
                                                    format!("Tex{} ({}x{})", id, tw, th)
                                                };
                                                if ui.selectable_label(false, &label).clicked() {
                                                    if let Some(mut s) = vp.world.get_mut::<Sprite>(e) {
                                                        s.texture_id = Some(id);
                                                        // Use the import settings' spritesheet info
                                                        let info = vp.spritesheet_info.get(&id).cloned()
                                                            .unwrap_or_else(|| crate::viewport::SpritesheetInfo::new(tw, th, tw, th));
                                                        s.source_width = info.tile_w;
                                                        s.source_height = info.tile_h;
                                                        s.source_x = 0;
                                                        s.source_y = 0;
                                                    }
                                                }
                                            }
                                        });
                                });
                                // Tile picker (only if a texture is selected AND it has multiple tiles)
                                if let Some(tex_id) = cur_tex_id {
                                    if let Some(info) = sprite_info.get(&tex_id) {
                                        if info.tile_count() > 1 {
                                            ui.horizontal(|ui| {
                                                ui.label("Tile:");
                                                // Determine current tile index from source_x/source_y
                                                let cur_tile = if info.tile_w > 0 && info.tile_h > 0 {
                                                    let col = cur_sx / info.tile_w;
                                                    let row = cur_sy / info.tile_h;
                                                    (row * info.cols + col) as usize
                                                } else { 0 };
                                                let mut tile_idx = cur_tile.min(info.tile_count().saturating_sub(1) as usize);
                                                ui.add(egui::DragValue::new(&mut tile_idx).range(0..=info.tile_count().saturating_sub(1) as usize).speed(1));
                                                ui.label(format!("{}/{}", tile_idx + 1, info.tile_count()));
                                                if tile_idx != cur_tile {
                                                    if let Some(mut s) = vp.world.get_mut::<Sprite>(e) {
                                                        let col = tile_idx as u32 % info.cols;
                                                        let row = tile_idx as u32 / info.cols;
                                                        s.source_x = col * info.tile_w;
                                                        s.source_y = row * info.tile_h;
                                                        s.source_width = info.tile_w;
                                                        s.source_height = info.tile_h;
                                                    }
                                                }
                                            });
                                            // Show a quick mini-grid of tiles to pick from (with actual images)
                                            // Scrollable grid showing ALL tiles
                                            let cell_sz = 24.0f32;
                                            let cols_avail = ((ui.available_width() - 4.0) / (cell_sz + 2.0)).max(1.0) as u32;
                                            let display_cols = info.cols.min(cols_avail).min(8);
                                            let display_rows = (info.tile_count() + display_cols - 1) / display_cols; // Show all rows
                                            let egui_tex_opt = vp.textures.get(&tex_id).and_then(|(_, _, _, opt)| *opt);
                                            
                                            egui::ScrollArea::vertical().auto_shrink([false;2]).max_height(200.0).show(ui, |ui| {
                                                egui::Grid::new("sprite_tile_grid").min_col_width(cell_sz + 2.0).show(ui, |ui| {
                                                    for row in 0..info.rows {
                                                        for col in 0..info.cols {
                                                            let idx = row * info.cols + col;
                                                            let (u0, v0, u1, v1) = info.uv_for_tile(idx);
                                                            let is_cur = cur_sx == (idx % info.cols) * info.tile_w && cur_sy == (idx / info.cols) * info.tile_h;
                                                            let r = ui.add_sized(egui::vec2(cell_sz, cell_sz), egui::Button::new("").fill(if is_cur { egui::Color32::YELLOW } else { egui::Color32::DARK_GRAY }));
                                                            if r.clicked() {
                                                                if let Some(mut s) = vp.world.get_mut::<Sprite>(e) {
                                                                    s.source_x = (idx % info.cols) * info.tile_w;
                                                                    s.source_y = (idx / info.cols) * info.tile_h;
                                                                    s.source_width = info.tile_w;
                                                                    s.source_height = info.tile_h;
                                                                }
                                                            }
                                                            // Draw tile thumbnail
                                                            if let Some(egui_tex) = egui_tex_opt {
                                                                let uv_rect = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
                                                                ui.painter().image(egui_tex, r.rect.shrink(1.0), uv_rect, egui::Color32::WHITE);
                                                            }
                                                            if (col + 1) % display_cols == 0 { ui.end_row(); }
                                                        }
                                                    }
                                                });
                                            });
                                        }
                                    }
                                }
                            }
                            let mut fx = vp.world.get::<Sprite>(e).map(|s| s.flip_x).unwrap_or(false);
                            let mut fy = vp.world.get::<Sprite>(e).map(|s| s.flip_y).unwrap_or(false);
                            ui.checkbox(&mut fx, "Flip X");
                            ui.checkbox(&mut fy, "Flip Y");
                            if let Some(mut s) = vp.world.get_mut::<Sprite>(e) { s.flip_x = fx; s.flip_y = fy; }
                        });
                    } else {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🟦 Sprite");
                            if ui.button("➕ Add Sprite").clicked() { vp.world.entity_mut(e).insert(Sprite::default()); }
                        });
                    }
                    ui.add_space(4.0);

                    // Spritesheet Menu
                    let cur_tex_id = vp.world.get::<Sprite>(e).and_then(|s| s.texture_id);
                    if let Some(tex_id) = cur_tex_id {
                        let sprite_info = vp.spritesheet_info.clone();
                        if let Some(info) = sprite_info.get(&tex_id) {
                            Frame::group(ui.style()).show(ui, |ui| {
                                ui.label("🖼️ Spritesheet");
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}x{} tiles", info.cols, info.rows));
                                    ui.label(format!("{}x{} px", info.tile_w, info.tile_h));
                                });
                                if ui.button("🔧 Auto-slice").clicked() {
                                    let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((0, 0));
                                    let auto_tile = ImportSettings::auto_tile_size(tw, th);
                                    let new_settings = ImportSettings::spritesheet(auto_tile, auto_tile);
                                    let new_info = crate::viewport::SpritesheetInfo::new(tw, th, auto_tile, auto_tile);
                                    vp.spritesheet_info.insert(tex_id, new_info.clone());
                                    vp.import_settings.insert(tex_id, new_settings);
                                    if let Some(mut s) = vp.world.get_mut::<Sprite>(e) {
                                        s.source_width = auto_tile;
                                        s.source_height = auto_tile;
                                        s.source_x = 0;
                                        s.source_y = 0;
                                    }
                                }
                                if ui.button("❌ Remove").clicked() {
                                    vp.spritesheet_info.remove(&tex_id);
                                }
                            });
                        } else {
                            Frame::group(ui.style()).show(ui, |ui| {
                                ui.label("🖼️ Spritesheet");
                                if ui.button("🔧 Auto-slice").clicked() {
                                    let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((0, 0));
                                    let auto_tile = ImportSettings::auto_tile_size(tw, th);
                                    let new_settings = ImportSettings::spritesheet(auto_tile, auto_tile);
                                    let new_info = crate::viewport::SpritesheetInfo::new(tw, th, auto_tile, auto_tile);
                                    vp.spritesheet_info.insert(tex_id, new_info.clone());
                                    vp.import_settings.insert(tex_id, new_settings);
                                    if let Some(mut s) = vp.world.get_mut::<Sprite>(e) {
                                        s.source_width = auto_tile;
                                        s.source_height = auto_tile;
                                        s.source_x = 0;
                                        s.source_y = 0;
                                    }
                                }
                            });
                        }
                    }
                    ui.add_space(4.0);

                    // Physics
                    let has_physics = vp.world.get::<Physics>(e).is_some();
                    if has_physics {
                        let ph_clone = vp.world.get::<Physics>(e).cloned().unwrap_or_default();
                        let mut ph = ph_clone.clone();
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("⚙️ Physics");
                            egui::ComboBox::from_id_salt("phys_type")
                                .selected_text(format!("{:?}", ph.body_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut ph.body_type, PhysicsBodyType::Static, "Static");
                                    ui.selectable_value(&mut ph.body_type, PhysicsBodyType::Dynamic, "Dynamic");
                                    ui.selectable_value(&mut ph.body_type, PhysicsBodyType::Kinematic, "Kinematic");
                                });
                            ui.horizontal(|ui| { ui.label("Mass:"); ui.add(egui::DragValue::new(&mut ph.mass).speed(0.1).range(0.0..=1000.0)); });
                            ui.horizontal(|ui| { ui.label("Friction:"); ui.add(egui::DragValue::new(&mut ph.friction).speed(0.05).range(0.0..=1.0)); });
                            ui.horizontal(|ui| { ui.label("Restitution:"); ui.add(egui::DragValue::new(&mut ph.restitution).speed(0.05).range(0.0..=1.0)); });
                            ui.horizontal(|ui| { ui.label("Col W:"); ui.add(egui::DragValue::new(&mut ph.collider_width).speed(1.0).range(1.0..=512.0)); 
                                                  ui.label("H:"); ui.add(egui::DragValue::new(&mut ph.collider_height).speed(1.0).range(1.0..=512.0)); });
                            ui.checkbox(&mut ph.is_trigger, "Trigger");
                            if ph != ph_clone { if let Some(mut w) = vp.world.get_mut::<Physics>(e) { *w = ph; } }
                            if ui.button("❌ Remove Physics").clicked() { vp.world.entity_mut(e).remove::<Physics>(); }
                        });
                    } else {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("⚙️ Physics");
                            if ui.button("➕ Add Physics").clicked() { vp.world.entity_mut(e).insert(Physics::default()); }
                        });
                    }
                    ui.add_space(4.0);

                    // Animation
                    let has_anim = vp.world.get::<Animation>(e).is_some();
                    if has_anim {
                        let anim = vp.world.get::<Animation>(e).cloned().unwrap_or_default();
                        let mut a = anim.clone();
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🎬 Animation");
                            ui.horizontal(|ui| { ui.label("Name:"); ui.text_edit_singleline(&mut a.name); });
                            ui.checkbox(&mut a.looping, "Looping");
                            ui.checkbox(&mut a.playing, "Playing");
                            if a != anim { if let Some(mut w) = vp.world.get_mut::<Animation>(e) { *w = a; } }
                            if ui.button("❌ Remove Anim").clicked() { vp.world.entity_mut(e).remove::<Animation>(); }
                        });
                    } else {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🎬 Animation");
                            if ui.button("➕ Add Animation").clicked() { vp.world.entity_mut(e).insert(Animation { 
                                name: "idle".into(), current_frame: 0, frame_durations: vec![0.1], 
                                elapsed: 0.0, looping: true, playing: true 
                            }); }
                        });
                    }
                    ui.add_space(4.0);

                    // Velocity
                    let has_vel = vp.world.get::<Velocity>(e).is_some();
                    if has_vel {
                        let vel = vp.world.get::<Velocity>(e).cloned().unwrap_or_default();
                        let mut v = vel.clone();
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🏃 Velocity");
                            ui.horizontal(|ui| { ui.label("VX:"); ui.add(egui::DragValue::new(&mut v.x).speed(1.0)); });
                            ui.horizontal(|ui| { ui.label("VY:"); ui.add(egui::DragValue::new(&mut v.y).speed(1.0)); });
                            if v != vel { if let Some(mut w) = vp.world.get_mut::<Velocity>(e) { *w = v; } }
                            if ui.button("❌").clicked() { vp.world.entity_mut(e).remove::<Velocity>(); }
                        });
                    } else {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🏃 Velocity");
                            if ui.button("➕ Add Velocity").clicked() { vp.world.entity_mut(e).insert(Velocity::default()); }
                        });
                    }
                    ui.add_space(4.0);

                    // Particle Emitter
                    let has_particle = vp.world.get::<ParticleEmitter>(e).is_some();
                    if has_particle {
                        let pe = vp.world.get::<ParticleEmitter>(e).cloned().unwrap_or_default();
                        let mut p = pe.clone();
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("✨ Particle Emitter");
                            ui.checkbox(&mut p.emitting, "Emitting");
                            ui.horizontal(|ui| { ui.label("Rate:"); ui.add(egui::DragValue::new(&mut p.spawn_rate).speed(0.5).range(0.0..=500.0)); });
                            ui.horizontal(|ui| { ui.label("Max:"); ui.add(egui::DragValue::new(&mut p.max_particles).speed(1).range(1..=10000)); });
                            ui.horizontal(|ui| { ui.label("Lifetime:"); ui.add(egui::DragValue::new(&mut p.lifetime).speed(0.1).range(0.1..=30.0)); });
                            ui.horizontal(|ui| { ui.label("Speed:"); ui.add(egui::DragValue::new(&mut p.speed).speed(1.0).range(0.0..=1000.0)); });
                            ui.horizontal(|ui| { ui.label("Gravity:"); ui.add(egui::DragValue::new(&mut p.gravity).speed(1.0).range(-500.0..=500.0)); });
                            ui.horizontal(|ui| { ui.label("Start Sz:"); ui.add(egui::DragValue::new(&mut p.start_size).speed(0.5).range(0.1..=100.0)); 
                                                  ui.label("End Sz:"); ui.add(egui::DragValue::new(&mut p.end_size).speed(0.5).range(0.1..=100.0)); });
                            if p != pe { if let Some(mut w) = vp.world.get_mut::<ParticleEmitter>(e) { *w = p; } }
                            if ui.button("❌ Remove Emitter").clicked() { vp.world.entity_mut(e).remove::<ParticleEmitter>(); }
                        });
                    } else {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("✨ Particle Emitter");
                            if ui.button("➕ Add Emitter").clicked() { vp.world.entity_mut(e).insert(ParticleEmitter::default()); }
                        });
                    }
                    ui.add_space(4.0);

                    // Audio Source
                    let has_audio = vp.world.get::<AudioSource>(e).is_some();
                    if has_audio {
                        let au = vp.world.get::<AudioSource>(e).cloned().unwrap_or_default();
                        let mut a = au.clone();
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🔊 Audio Source");
                            ui.horizontal(|ui| { ui.label("Path:"); ui.text_edit_singleline(&mut a.path); });
                            ui.checkbox(&mut a.looping, "Looping");
                            ui.checkbox(&mut a.playing, "Playing");
                            ui.checkbox(&mut a.is_music, "Background Music");
                            ui.horizontal(|ui| { ui.label("Volume:"); ui.add(egui::Slider::new(&mut a.volume, 0.0..=1.0)); });
                            if a.path != au.path || a.playing != au.playing {
                                // Audio playback would happen via the audio manager
                                vp.lua_log.push(format!("[audio] would play: {}", a.path));
                            }
                            if a != au { if let Some(mut w) = vp.world.get_mut::<AudioSource>(e) { *w = a; } }
                            if ui.button("❌ Remove Audio").clicked() { vp.world.entity_mut(e).remove::<AudioSource>(); }
                        });
                    } else {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("🔊 Audio Source");
                            if ui.button("➕ Add Audio").clicked() { vp.world.entity_mut(e).insert(AudioSource::default()); }
                        });
                    }
                    ui.add_space(4.0);

                    // Script component
                    let has_script = vp.world.get::<Script>(e).is_some();
                    if has_script {
                        let sc = vp.world.get::<Script>(e).cloned().unwrap();
                        let mut s = sc.clone();
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("📜 Script");
                            ui.horizontal(|ui| { ui.label("Path:"); ui.text_edit_singleline(&mut s.path); });
                            if s != sc { if let Some(mut w) = vp.world.get_mut::<Script>(e) { *w = s; } }
                            if ui.button("❌ Remove Script").clicked() { vp.world.entity_mut(e).remove::<Script>(); }
                        });
                    } else {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("📜 Script");
                            if ui.button("➕ Add Script").clicked() { vp.world.entity_mut(e).insert(Script { path: "".into(), source: "".into() }); }
                        });
                    }
                    ui.add_space(4.0);

                    // Camera Tag
                    let has_camera = vp.world.get::<CameraTag>(e).is_some();
                    if !has_camera {
                        Frame::group(ui.style()).show(ui, |ui| {
                            ui.label("📷 Camera");
                            if ui.button("➕ Make Camera").clicked() { vp.world.entity_mut(e).insert(CameraTag); }
                        });
                    } else {
                        ui.label("📷 This entity is the active camera");
                        if ui.button("❌ Remove Camera Tag").clicked() { vp.world.entity_mut(e).remove::<CameraTag>(); }
                    }
                    ui.add_space(4.0);
                    
                    // Parent/child info
                    if let Some(parent) = vp.world.get::<Parent>(e) {
                        let parent_name = vp.entity_names.get(&parent.0).cloned().unwrap_or_else(|| format!("{:?}", parent.0));
                        ui.label(format!("🔗 Parent: {}", parent_name));
                    }
                    if let Some(children) = vp.world.get::<Children>(e) {
                        if !children.0.is_empty() {
                            ui.label(format!("👶 Children: {}", children.0.len()));
                        }
                    }
                    
                    ui.add_space(4.0);
                    ui.separator();
                    if ui.button("🗑 Delete Entity").clicked() { vp.remove(e); }
                } else { 
                    ui.weak("(Select an entity from the Scene panel)"); 
                }
            });
        });

        // Center: Viewport with gizmos
        CentralPanel::default().show(ctx, |ui| {
            let r = ui.available_rect_before_wrap();
            let (vr_id, vr_rect) = ui.allocate_space(r.size());
            let _vr_response = ui.interact(vr_rect, vr_id, egui::Sense::click_and_drag());
            
            let mouse_pos = ctx.input(|i| i.pointer.interact_pos().unwrap_or(egui::Pos2::ZERO));
            let in_viewport = vr_rect.contains(mouse_pos);
            
            // Cursor-centered zoom
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 && in_viewport {
                let old_scale = vp.view_scale;

                // Compute the new scale
                let new_scale = if vp.pixel_perfect {
                    // Pixel-perfect: step between nice values — integers for
                    // zoom-in (1×, 2×, 3× …) and 1/n fractions for zoom-out
                    // (½, ⅓, ¼ …).  Every pixel stays crisp at any level.
                    if scroll > 0.0 {
                        if old_scale >= 1.0 {
                            (old_scale + 1.0).min(20.0)
                        } else {
                            let n = (1.0 / old_scale).round().max(2.0);
                            (1.0 / (n - 1.0)).max(1.0)
                        }
                    } else {
                        if old_scale <= 1.0 {
                            let n = (1.0 / old_scale).round().max(1.0);
                            (1.0 / (n + 1.0)).max(0.05)
                        } else {
                            (old_scale - 1.0).max(1.0)
                        }
                    }
                } else {
                    let zoom_factor = if scroll > 0.0 { 1.15 } else { 1.0 / 1.15 };
                    (old_scale * zoom_factor).clamp(0.1, 20.0)
                };

                let (mouse_wx, mouse_wy) = self.screen_to_world(mouse_pos, &vr_rect, vp);

                let a = vp.tex_size.0 as f32 / vp.tex_size.1 as f32;
                let (dw, dh) = if vr_rect.width()/vr_rect.height() > a { (vr_rect.height()*a, vr_rect.height()) } else { (vr_rect.width(), vr_rect.width()/a) };
                let left = vr_rect.left() + (vr_rect.width() - dw) / 2.0;
                let top = vr_rect.top() + (vr_rect.height() - dh) / 2.0;
                let nx = (mouse_pos.x - left) / dw;
                let ny = (mouse_pos.y - top) / dh;

                let new_ox = nx * vp.tex_size.0 as f32 - mouse_wx * new_scale;
                let new_oy = mouse_wy * new_scale - ny * vp.tex_size.1 as f32;

                if vp.pixel_perfect {
                    // Instant snap for pixel-perfect mode
                    vp.view_scale = new_scale;
                    vp.view_offset = (new_ox.round(), new_oy.round());
                    vp.target_view_scale = vp.view_scale;
                    vp.target_view_offset = vp.view_offset;
                } else {
                    // Smooth interpolation: set targets, render loop lerps
                    vp.target_view_scale = new_scale;
                    vp.target_view_offset = (new_ox, new_oy);
                }
            }

            // Right-click pan
            let rc_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Secondary));
            let rc_clicked = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary));
            if rc_clicked && in_viewport {
                self.panning = true;
                self.pan_start = Some(mouse_pos);
                // Sync target to actual so there's no jump when starting a pan
                vp.target_view_offset = vp.view_offset;
                self.pan_offset_start = vp.view_offset;
            }
            if rc_down && self.panning {
                if let Some(start) = self.pan_start {
                    let a = vp.tex_size.0 as f32 / vp.tex_size.1 as f32;
                    let (dw, dh) = if vr_rect.width()/vr_rect.height() > a { (vr_rect.height()*a, vr_rect.height()) } else { (vr_rect.width(), vr_rect.width()/a) };
                    let dx = (mouse_pos.x - start.x) * (vp.tex_size.0 as f32 / dw);
                    let dy = (mouse_pos.y - start.y) * (vp.tex_size.1 as f32 / dh);
                    let new_ox = self.pan_offset_start.0 + dx;
                    let new_oy = self.pan_offset_start.1 - dy;
                    if vp.pixel_perfect {
                        vp.view_offset = (new_ox.round(), new_oy.round());
                        vp.target_view_offset = vp.view_offset;
                    } else {
                        vp.target_view_offset = (new_ox, new_oy);
                    }
                }
            }
            if !rc_down { self.panning = false; }

            // Left click: select entity or gizmo drag, or paint tiles
            let lc = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
            let ld = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
            
            // Tilemap painting mode
            if self.tab == Tab::Tilemap && in_viewport {
                if ld {
                    let (wx, wy) = self.screen_to_world(mouse_pos, &vr_rect, vp);
                    let ts = vp.tile_layers.first().map(|l| l.ts as f32).unwrap_or(16.0);
                    let col = (wx / ts).floor() as i32;
                    let row = (wy / ts).floor() as i32;
                    if col >= 0 && row >= 0 {
                        if let Some(layer) = vp.tile_layers.iter_mut().find(|l| l.vis) {
                            if (row as usize) < layer.rows && (col as usize) < layer.cols {
                                layer.tiles[row as usize][col as usize] = vp.sel_tile as u32;
                            }
                        }
                    }
                }
            } else if lc && in_viewport {
                let (wx, wy) = self.screen_to_world(mouse_pos, &vr_rect, vp);
                let mut hit_gizmo = false;
                if let Some(sel) = vp.selected {
                    if let Some((cx, cy)) = self.get_gizmo_center(vp) {
                        let gizmo_scale = vp.view_scale.max(0.5);
                        let axis = vp.hit_test_gizmo(wx, wy, cx, cy, gizmo_scale);
                        if axis != GizmoAxis::None {
                            vp.gizmo_axis = axis;
                            vp.gizmo_drag_start_world = Some((wx, wy));
                            if let Some(t) = vp.world.get::<Transform>(sel) {
                                vp.gizmo_drag_start_value = Some((t.x, t.y, t.rotation));
                            }
                            self.gizmo_dragging = true;
                            hit_gizmo = true;
                        }
                    }
                }
                if !hit_gizmo {
                    if let Some(entity) = vp.hit_test(wx, wy) {
                        vp.selected = Some(entity);
                    } else {
                        vp.selected = None;
                    }
                }
            }
            // Gizmo drag
            if ld && self.gizmo_dragging && vp.gizmo_axis != GizmoAxis::None && in_viewport {
                if let Some(sel) = vp.selected {
                    let (wx, wy) = self.screen_to_world(mouse_pos, &vr_rect, vp);
                    let (start_wx, start_wy) = vp.gizmo_drag_start_world.unwrap_or((wx, wy));
                    let (start_x, start_y, start_rot) = vp.gizmo_drag_start_value.unwrap_or((0.0,0.0,0.0));
                    let mut dx = wx - start_wx;
                    let mut dy = wy - start_wy;
                    
                    if vp.snap_to_grid {
                        let gs = vp.grid_size.max(1) as f32;
                        dx = (dx / gs).round() * gs;
                        dy = (dy / gs).round() * gs;
                    }
                    
                    if let Some(mut t) = vp.world.get_mut::<Transform>(sel) {
                        match vp.transform_mode {
                            TransformMode::Move => {
                                match vp.gizmo_axis {
                                    GizmoAxis::X => { t.x = start_x + dx; }
                                    GizmoAxis::Y => { t.y = start_y + dy; }
                                    GizmoAxis::XY => { t.x = start_x + dx; t.y = start_y + dy; }
                                    _ => {}
                                }
                            }
                            TransformMode::Scale => {
                                match vp.gizmo_axis {
                                    GizmoAxis::X => { t.scale_x = (t.scale_x + dx * 0.05).max(0.01); }
                                    GizmoAxis::Y => { t.scale_y = (t.scale_y + dy * 0.05).max(0.01); }
                                    _ => {}
                                }
                            }
                            TransformMode::Rotate => {
                                if vp.gizmo_axis == GizmoAxis::Rot {
                                    t.rotation = start_rot + dx * 0.02;
                                }
                            }
                        }
                    }
                }
            }
            if !ld { self.gizmo_dragging = false; vp.gizmo_axis = GizmoAxis::None; }

            // Render viewport
            if let Some(tid) = self.tex_id {
                let a = vp.tex_size.0 as f32 / vp.tex_size.1 as f32;
                let (dw, dh) = if vr_rect.width()/vr_rect.height() > a { (vr_rect.height()*a, vr_rect.height()) } else { (vr_rect.width(), vr_rect.width()/a) };
                let dr = egui::Rect::from_min_size(egui::pos2(vr_rect.left()+(vr_rect.width()-dw)/2.0, vr_rect.top()+(vr_rect.height()-dh)/2.0), egui::vec2(dw, dh));
                
                // Subtle checkerboard background (shows transparency)
                let bg_c1 = Color32::from_rgb(18, 20, 26);
                let bg_c2 = Color32::from_rgb(24, 27, 33);
                let checker_size = 8.0;
                let cols = ((vr_rect.width() / checker_size).ceil() as i32) + 1;
                let rows = ((vr_rect.height() / checker_size).ceil() as i32) + 1;
                for ry in 0..rows {
                    for rx in 0..cols {
                        let x = vr_rect.left() + rx as f32 * checker_size;
                        let y = vr_rect.top() + ry as f32 * checker_size;
                        let c = if (rx + ry) % 2 == 0 { bg_c1 } else { bg_c2 };
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(checker_size, checker_size)),
                            0.0, c,
                        );
                    }
                }
                // Viewport border
                ui.painter().rect_stroke(vr_rect, 1.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 70, 90)), egui::StrokeKind::Outside);
                ui.painter().image(tid, dr, egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), Color32::WHITE);
                
                // Drag-and-drop textures
                if let Some(dropped_tex) = vp.drag_tex_id {
                    // Skip drag-and-drop if this is a double-click — double-clicking
                    // a texture in the asset browser selects it in the Import tab
                    // (for import configuration), not creates a sprite entity.
                    // The viewport is rendered before the asset browser, so we must
                    // check for double-click here to avoid processing the drag from
                    // the first click of the double-click.
                    let double_clicked = ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
                    if in_viewport && lc && !double_clicked {
                        let (mouse_wx, mouse_wy) = self.screen_to_world(mouse_pos, &vr_rect, vp);
                        if let Some(sel) = vp.selected {
                            // Assign texture to selected entity
                            vp.save_undo();
                            if let Some(mut sp) = vp.world.get_mut::<Sprite>(sel) {
                                sp.texture_id = Some(dropped_tex);
                                if let Some((_, tw, th, _)) = vp.textures.get(&dropped_tex) {
                                    // Use the import settings' spritesheet info (already set during import)
                                    let info = vp.spritesheet_info.get(&dropped_tex).cloned()
                                        .unwrap_or_else(|| crate::viewport::SpritesheetInfo::new(*tw, *th, *tw, *th));
                                    sp.source_width = info.tile_w;
                                    sp.source_height = info.tile_h;
                                    sp.source_x = 0;
                                    sp.source_y = 0;
                                }
                                vp.lua_log.push("[asset] assigned texture to entity".to_string());
                            }
                        } else {
                            // No entity selected — create a new sprite entity with this texture
                            vp.save_undo();
                            let (tw, th) = vp.textures.get(&dropped_tex)
                                .map(|(_, w, h, _)| (*w, *h))
                                .unwrap_or((16, 16));
                            let info = vp.spritesheet_info.get(&dropped_tex).cloned()
                                .unwrap_or_else(|| crate::viewport::SpritesheetInfo::new(tw, th, tw, th));
                            let mut sp = Sprite::default();
                            sp.texture_id = Some(dropped_tex);
                            sp.source_width = info.tile_w;
                            sp.source_height = info.tile_h;
                            sp.source_x = 0;
                            sp.source_y = 0;
                            let e = vp.world.spawn((
                                Transform::new(mouse_wx, mouse_wy),
                                sp,
                            )).id();
                            let name = vp.drag_tex_name.clone()
                                .unwrap_or_else(|| format!("Sprite_{}", vp.entities.len()));
                            vp.entity_names.insert(e, name);
                            vp.refresh();
                            vp.selected = Some(e);
                            vp.lua_log.push(format!("[asset] created sprite entity with texture", ));
                        }
                        vp.drag_tex_id = None;
                        vp.drag_tex_name = None;
                    }
                }
                
                // Canvas painter clipped to texture display area
                let canvas_painter = ui.painter_at(dr.intersect(vr_rect));

                // 1. GRID OVERLAY — major lines every 4 cells, minor lines between
                let grid_size = if vp.tile_layers.first().map(|l| l.ts).unwrap_or(0) > 0 {
                    vp.tile_layers.first().unwrap().ts as f32
                } else {
                    vp.grid_size.max(4) as f32
                };

                if vp.show_grid {
                    let minor_color = Color32::from_rgba_unmultiplied(120, 140, 160, 35);
                    let major_color = Color32::from_rgba_unmultiplied(120, 140, 160, 70);
                    let (w_left, w_top) = self.screen_to_world(dr.min, &vr_rect, vp);
                    let (w_right, w_bottom) = self.screen_to_world(dr.max, &vr_rect, vp);

                    let start_x = (w_left / grid_size).floor() * grid_size;
                    let mut gx = start_x;
                    let mut gx_i = 0i32;
                    while gx <= w_right + grid_size {
                        let sp = self.world_to_screen(gx, 0.0, &vr_rect, vp);
                        if sp.x >= dr.left() && sp.x <= dr.right() {
                            let is_major = gx_i % 4 == 0;
                            canvas_painter.line_segment(
                                [egui::pos2(sp.x, dr.top()), egui::pos2(sp.x, dr.bottom())],
                                (if is_major { 1.0 } else { 0.5 }, if is_major { major_color } else { minor_color }),
                            );
                        }
                        gx += grid_size;
                        gx_i += 1;
                    }

                    let start_y = (w_top / grid_size).floor() * grid_size;
                    let mut gy = start_y;
                    let mut gy_i = 0i32;
                    while gy <= w_bottom + grid_size {
                        let sp = self.world_to_screen(0.0, gy, &vr_rect, vp);
                        if sp.y >= dr.top() && sp.y <= dr.bottom() {
                            let is_major = gy_i % 4 == 0;
                            canvas_painter.line_segment(
                                [egui::pos2(dr.left(), sp.y), egui::pos2(dr.right(), sp.y)],
                                (if is_major { 1.0 } else { 0.5 }, if is_major { major_color } else { minor_color }),
                            );
                        }
                        gy += grid_size;
                        gy_i += 1;
                    }
                }

                // 2. VISIBLE X AND Y AXES — clean anti-aliased with glow
                let origin_sp = self.world_to_screen(0.0, 0.0, &vr_rect, vp);

                // Horizontal X-Axis (Y=0)
                if origin_sp.y >= dr.top() && origin_sp.y <= dr.bottom() {
                    canvas_painter.line_segment(
                        [egui::pos2(dr.left(), origin_sp.y), egui::pos2(dr.right(), origin_sp.y)],
                        (4.0, Color32::from_black_alpha(180)),
                    );
                    canvas_painter.line_segment(
                        [egui::pos2(dr.left(), origin_sp.y), egui::pos2(dr.right(), origin_sp.y)],
                        (2.0, Color32::from_rgb(255, 68, 68)),
                    );
                    let arrow_x = dr.right() - 25.0;
                    canvas_painter.text(
                        egui::pos2(arrow_x, origin_sp.y - 12.0),
                        egui::Align2::RIGHT_BOTTOM,
                        "+X ▶",
                        egui::TextStyle::Small.resolve(ui.style()),
                        Color32::from_rgb(255, 140, 140),
                    );
                }

                // Vertical Y-Axis (X=0)
                if origin_sp.x >= dr.left() && origin_sp.x <= dr.right() {
                    canvas_painter.line_segment(
                        [egui::pos2(origin_sp.x, dr.top()), egui::pos2(origin_sp.x, dr.bottom())],
                        (4.0, Color32::from_black_alpha(180)),
                    );
                    canvas_painter.line_segment(
                        [egui::pos2(origin_sp.x, dr.top()), egui::pos2(origin_sp.x, dr.bottom())],
                        (2.0, Color32::from_rgb(68, 255, 68)),
                    );
                    let arrow_y = dr.bottom() - 25.0;
                    canvas_painter.text(
                        egui::pos2(origin_sp.x + 8.0, arrow_y),
                        egui::Align2::LEFT_BOTTOM,
                        "+Y ▼",
                        egui::TextStyle::Small.resolve(ui.style()),
                        Color32::from_rgb(140, 255, 140),
                    );
                }

                // Origin Marker (0,0) — polished badge
                if origin_sp.x >= dr.left() && origin_sp.x <= dr.right() &&
                   origin_sp.y >= dr.top() && origin_sp.y <= dr.bottom() {
                    canvas_painter.circle_filled(origin_sp, 7.0, Color32::from_rgb(15, 17, 22));
                    canvas_painter.circle_filled(origin_sp, 5.0, Color32::from_rgb(255, 220, 0));
                    canvas_painter.circle_filled(origin_sp, 2.2, Color32::from_rgb(15, 17, 22));

                    let badge_rect = egui::Rect::from_min_size(
                        egui::pos2(origin_sp.x + 10.0, origin_sp.y - 20.0),
                        egui::vec2(40.0, 18.0),
                    );
                    canvas_painter.rect_filled(badge_rect, 4.0, Color32::from_black_alpha(210));
                    canvas_painter.rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 220, 0)), egui::StrokeKind::Outside);
                    canvas_painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "(0,0)",
                        egui::TextStyle::Small.resolve(ui.style()),
                        Color32::WHITE,
                    );
                }

                // 3. CORNER VIEWPORT ORIENTATION GIZMO (Bottom-Right corner)
                let corner_center = egui::pos2(dr.right() - 44.0, dr.bottom() - 44.0);
                let gizmo_bg = egui::Rect::from_center_size(corner_center, egui::vec2(64.0, 64.0));
                canvas_painter.rect_filled(gizmo_bg, 10.0, Color32::from_black_alpha(200));
                canvas_painter.rect_stroke(gizmo_bg, 10.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(70, 80, 100)), egui::StrokeKind::Outside);

                let axis_len = 19.0;
                let x_end = egui::pos2(corner_center.x + axis_len, corner_center.y);
                let y_end = egui::pos2(corner_center.x, corner_center.y + axis_len);

                // Red X vector
                canvas_painter.line_segment([corner_center, x_end], (2.5, Color32::from_rgb(255, 75, 75)));
                canvas_painter.circle_filled(x_end, 3.5, Color32::from_rgb(255, 75, 75));
                canvas_painter.text(egui::pos2(x_end.x + 6.0, x_end.y), egui::Align2::LEFT_CENTER, "X", egui::TextStyle::Small.resolve(ui.style()), Color32::from_rgb(255, 150, 150));

                // Green Y vector
                canvas_painter.line_segment([corner_center, y_end], (2.5, Color32::from_rgb(75, 255, 75)));
                canvas_painter.circle_filled(y_end, 3.5, Color32::from_rgb(75, 255, 75));
                canvas_painter.text(egui::pos2(y_end.x, y_end.y + 6.0), egui::Align2::CENTER_TOP, "Y", egui::TextStyle::Small.resolve(ui.style()), Color32::from_rgb(150, 255, 150));

                canvas_painter.circle_filled(corner_center, 3.5, Color32::from_rgb(220, 225, 235));
                
                // Draw transform gizmo overlay on selected entity
                if let Some((cx, cy)) = self.get_gizmo_center(vp) {
                    let gizmo_scale = vp.view_scale.max(0.5);
                    let len = 20.0 * gizmo_scale;
                    let c = self.world_to_screen(cx, cy, &vr_rect, vp);
                    let ex = self.world_to_screen(cx + len / vp.view_scale, cy, &vr_rect, vp);
                    let ey = self.world_to_screen(cx, cy + len / vp.view_scale, &vr_rect, vp);
                    match vp.transform_mode {
                        TransformMode::Move => {
                            canvas_painter.line_segment([c, ex], (2.0, Color32::RED));
                            canvas_painter.line_segment([c, ey], (2.0, Color32::GREEN));
                            canvas_painter.circle_filled(ex, 3.0, Color32::RED);
                            canvas_painter.circle_filled(ey, 3.0, Color32::GREEN);
                            canvas_painter.rect_filled(egui::Rect::from_center_size(c, egui::vec2(6.0, 6.0)), 0.0, Color32::from_rgb(200, 200, 200));
                        }
                        TransformMode::Scale => {
                            canvas_painter.line_segment([c, ex], (2.0, Color32::RED));
                            canvas_painter.line_segment([c, ey], (2.0, Color32::GREEN));
                            canvas_painter.rect_filled(egui::Rect::from_center_size(ex, egui::vec2(6.0, 6.0)), 0.0, Color32::RED);
                            canvas_painter.rect_filled(egui::Rect::from_center_size(ey, egui::vec2(6.0, 6.0)), 0.0, Color32::GREEN);
                        }
                        TransformMode::Rotate => {
                            let ex2 = self.world_to_screen(cx + len / vp.view_scale, cy, &vr_rect, vp);
                            let n = 24;
                            let pts: Vec<egui::Pos2> = (0..=n).map(|i| {
                                let ang = (i as f32 / n as f32) * std::f32::consts::TAU;
                                self.world_to_screen(cx + len/vp.view_scale * ang.cos(), cy + len/vp.view_scale * ang.sin(), &vr_rect, vp)
                            }).collect();
                            for w in pts.windows(2) {
                                canvas_painter.line_segment([w[0], w[1]], (2.0, Color32::from_rgb(100, 200, 255)));
                            }
                            canvas_painter.circle_filled(ex2, 4.0, Color32::from_rgb(100, 200, 255));
                        }
                    }
                }

                // Info overlay — polished badge
                let info = format!("{}x{} @ {:.0}% | {:?}", vp.tex_size.0, vp.tex_size.1, (dw / vp.tex_size.0 as f32) * 100.0, vp.transform_mode);
                let info_rect = egui::Rect::from_min_size(
                    egui::pos2(dr.left()+6.0, dr.top()+6.0),
                    egui::vec2(180.0, 22.0),
                );
                canvas_painter.rect_filled(info_rect, 4.0, Color32::from_black_alpha(190));
                canvas_painter.rect_stroke(info_rect, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 70, 90)), egui::StrokeKind::Outside);
                canvas_painter.text(
                    egui::pos2(dr.left()+14.0, dr.top()+10.0),
                    egui::Align2::LEFT_CENTER,
                    info,
                    egui::TextStyle::Small.resolve(ui.style()),
                    Color32::from_rgb(220, 225, 235),
                );
            } else { 
                ui.painter().rect_filled(vr_rect, 0.0, Color32::from_rgb(20,20,30));
                ui.weak("(No render target)");
            }
        });

        // Asset Browser
        TopBottomPanel::bottom("assets").resizable(true).default_height(200.0).min_height(100.0).max_height(600.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("📁 Assets");
                ui.separator();
                ui.label("📥 Switch to the Import tab to configure texture import settings (Texture / Spritesheet)");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄").on_hover_text("Refresh").clicked() { self.thumbnails.clear(); }
                    if ui.button("📁").on_hover_text("Create Folder").clicked() { 
                        self.new_asset_name.clear();
                        self.show_create_folder_dialog = true;
                    }
                    if ui.button("📄").on_hover_text("Create Script").clicked() {
                        let path = self.asset_path.join("new_script.lua");
                        let _ = std::fs::write(&path, "-- Pixgine Script\nfunction init() end\nfunction update(dt) end\n");
                    }
                    if ui.button("📌").on_hover_text("Create Scene").clicked() {
                        let path = self.asset_path.join("new_scene.json");
                        let _ = std::fs::write(&path, "{\"entities\":[]}");
                    }
                    if ui.button("🖼️").on_hover_text("Import Image...").clicked() {
                        let rfd = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp"])
                            .pick_file();
                        if let Some(src) = rfd {
                            let dst = self.asset_path.join(src.file_name().unwrap_or_default());
                            let _ = std::fs::copy(&src, &dst);
                            let name = src.file_name().unwrap_or_default().to_string_lossy().to_string();
                            if let Some(tex_id) = self.ensure_texture_loaded(vp, &dst, &name) {
                                self.selected_import_tex = Some(tex_id);
                                self.tab = Tab::Import;
                                self.asset_path = dst.parent().unwrap_or(&self.asset_path).to_path_buf();
                            }
                        }
                    }
                });
            });
            ui.separator();

            // Path breadcrumbs
            ui.horizontal(|ui| {
                if ui.button("📁").clicked() { self.asset_path = PathBuf::from("assets"); }
                let mut parts: Vec<String> = Vec::new();
                let mut built = String::new();
                for comp in self.asset_path.components() {
                    let s = comp.as_os_str().to_string_lossy().to_string();
                    parts.push(s.clone());
                    built.push('/');
                    built.push_str(&s);
                }
                built = built.trim_start_matches('/').to_string();
                let mut running = String::new();
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 { running.push('/'); }
                    running.push_str(part);
                    if ui.button(part).clicked() {
                        self.asset_path = PathBuf::from(&running);
                    }
                }
            });
            ui.separator();

            // Asset grid
            egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui, |ui| {
                let mut dirs: Vec<_> = std::fs::read_dir(&self.asset_path).ok().into_iter().flatten()
                    .filter_map(|e| e.ok()).filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).collect();
                dirs.sort_by(|a, b| a.path().cmp(&b.path()));
                let mut files: Vec<_> = std::fs::read_dir(&self.asset_path).ok().into_iter().flatten()
                    .filter_map(|e| e.ok()).filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false)).collect();
                files.sort_by(|a, b| a.path().cmp(&b.path()));

                let cell_w = 80.0;
                let cols = ((ui.available_width() / cell_w) as usize).max(1);
                egui::Grid::new("asset_grid3").min_col_width(cell_w).max_col_width(cell_w).show(ui, |ui| {
                                        for entry in &dirs {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let path = entry.path();
                        let r = ui.button(format!("📁\n{}", name));
                        if r.double_clicked() {
                            self.asset_path = path.clone();
                        }
                        r.context_menu(|ui| {
                            if ui.button("Open").clicked() { self.asset_path = entry.path(); ui.close_menu(); }
                            if ui.button("Delete").clicked() { let _ = std::fs::remove_dir_all(entry.path()); ui.close_menu(); }
                        });
                    }
                    for entry in &files {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let path = entry.path();
                        if name.ends_with(".png") && !self.thumbnails.contains_key(&name) {
                            if let Some(thumb) = vp.load_png_thumbnail(&path) {
                                self.thumbnails.insert(name.clone(), thumb);
                            }
                        }
                        // Build button text with icon
                        let icon = if name.ends_with(".png") { "🖼️" } else if name.ends_with(".json") { "📄" } else if name.ends_with(".lua") { "📜" } else { "📄" };
                        let r = ui.button(format!("{}\n{}", icon, name));
                        // Draw thumbnail overlay if PNG
                        if name.ends_with(".png") {
                            if let Some(thumb) = self.thumbnails.get(&name) {
                                let tex_id_str = format!("thumb_{}", name);
                                let tex = ui.ctx().load_texture(&tex_id_str, thumb.clone(), egui::TextureOptions::NEAREST);
                                let img_size = egui::vec2(20.0, 20.0);
                                let img_pos = egui::pos2(r.rect.left() + 4.0, r.rect.top() + 4.0);
                                ui.painter().image(tex.id(), egui::Rect::from_min_size(img_pos, img_size), egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), Color32::WHITE);
                            }
                            // Drag source: clicking a texture sets it as draggable
                            if r.is_pointer_button_down_on() && ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) {
                                // Ensure the texture is loaded, then set up drag state
                                if let Some(tex_id) = self.ensure_texture_loaded(vp, &path, &name) {
                                    vp.drag_tex_id = Some(tex_id);
                                    vp.drag_tex_name = Some(name.clone());
                                    // Also select in Import tab so settings are visible
                                    self.selected_import_tex = Some(tex_id);
                                }
                            }
                        }
                        if r.double_clicked() {
                            if name.ends_with(".json") { let _ = vp.load_scene(&path); self.add_recent_file(&path); }
                            else if name.ends_with(".lua") { vp.scripts.push(name.clone()); }
                            else if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") {
                                // Clear any pending drag state
                                vp.drag_tex_id = None;
                                vp.drag_tex_name = None;
                                // Double-clicking a PNG/JPG loads the texture (if not already
                                // loaded) and selects it in the Import tab so the user can
                                // configure whether it's a single texture or a spritesheet.
                                // It does NOT create a sprite entity — use the context menu
                                // "➕ Sprite Entity" to add it to the scene.
                                if let Some(tex_id) = self.ensure_texture_loaded(vp, &path, &name) {
                                    self.selected_import_tex = Some(tex_id);
                                    self.tab = Tab::Import;
                                    let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((0,0));
                                    vp.lua_log.push(format!("[asset] selected texture for import: {} ({}x{})", name, tw, th));
                                }
                            }
                        }
                        r.context_menu(|ui| {
                            let p = path.clone();
                            if name.ends_with(".json") { if ui.button("Open Scene").clicked() { let _ = vp.load_scene(&p); self.add_recent_file(&p); ui.close_menu(); } }
                            if name.ends_with(".lua") { if ui.button("Add Script").clicked() { vp.scripts.push(name.clone()); ui.close_menu(); } }
                            if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") {
                                if ui.button("➕ Sprite Entity").clicked() {
                                    if let Some(tex_id) = self.ensure_texture_loaded(vp, &p, &name) {
                                        let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((16,16));
                                        let info = vp.spritesheet_info.get(&tex_id).cloned()
                                            .unwrap_or_else(|| crate::viewport::SpritesheetInfo::new(tw, th, tw, th));
                                        let mut sp = Sprite::default();
                                        sp.texture_id = Some(tex_id);
                                        sp.source_width = info.tile_w;
                                        sp.source_height = info.tile_h;
                                        if !vp.spritesheet_info.contains_key(&tex_id) {
                                            vp.spritesheet_info.insert(tex_id, info);
                                        }
                                        let e = vp.world.spawn((Transform::new(160.0, 90.0), sp)).id();
                                        vp.entity_names.insert(e, name.clone());
                                        vp.refresh();
                                        vp.selected = Some(e);
                                        vp.save_undo();
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("⚙️ Import Settings").clicked() {
                                    // Ensure texture is loaded, then open import settings
                                    if let Some(tex_id) = self.ensure_texture_loaded(vp, &p, &name) {
                                        self.import_settings_tex = Some(tex_id);
                                        self.edit_import_settings = vp.import_settings.get(&tex_id).cloned()
                                            .unwrap_or_else(ImportSettings::default);
                                        self.selected_import_tex = Some(tex_id);
                                    }
                                    ui.close_menu();
                                }
                            }
                            if ui.button("Delete").clicked() { let _ = std::fs::remove_file(&p); ui.close_menu(); }
                        });
                    }
                    let remaining = cols.saturating_sub(dirs.len() + files.len());
                    for _ in 0..remaining { ui.label(""); }
                });
            });
        });

        // Create Project dialog
        if self.show_create_project_dialog {
            egui::Window::new("Create Project")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Project Name:");
                    ui.text_edit_singleline(&mut self.new_scene_name);
                    if ui.button("📁 Choose Parent Folder").clicked() {
                        let folder = rfd::FileDialog::new().pick_folder();
                        if let Some(f) = folder {
                            self.project_path = Some(f);
                        }
                    }
                    if let Some(ref pp) = self.project_path {
                        ui.label(format!("Location: {}", pp.display()));
                    } else {
                        // Default to current dir
                        self.project_path = Some(std::env::current_dir().unwrap_or_default());
                        ui.label(format!("Location: {}", self.project_path.as_ref().unwrap().display()));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            let name = if self.new_scene_name.is_empty() { "untitled".to_string() } else { self.new_scene_name.clone() };
                            let base = self.project_path.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_default()).join(&name);
                            let _ = std::fs::create_dir_all(base.join("assets").join("scenes"));
                            let _ = std::fs::create_dir_all(base.join("assets").join("textures"));
                            let _ = std::fs::create_dir_all(base.join("assets").join("scripts"));
                            let _ = std::fs::create_dir_all(base.join("assets").join("tilesets"));
                            let meta = serde_json::json!({"name": name, "version": "0.1.0"});
                            let _ = std::fs::write(base.join("project.pixgine"), serde_json::to_string_pretty(&meta).unwrap());
                            for e in vp.world.query::<bevy_ecs::entity::Entity>().iter(&vp.world).collect::<Vec<_>>() { let _ = vp.world.despawn(e); }
                            vp.scene_path = Some(base.join("assets").join("scenes").join("scene.json"));
                            vp.refresh();
                            vp.selected = None;
                            self.asset_path = base.join("assets");
                            self.project_path = Some(base.clone());
                            // Save initial scene
                            let _ = vp.save_scene(&base.join("assets").join("scenes").join("scene.json"));
                            self.show_create_project_dialog = false;
                        }
                        if ui.button("Cancel").clicked() { self.show_create_project_dialog = false; }
                    });
                });
        }

        // Open Project dialog
        if self.show_open_project_dialog {
            let rfd = rfd::FileDialog::new()
                .add_filter("Pixgine Project", &["pixgine"])
                .set_title("Open Project")
                .pick_file();
            self.show_open_project_dialog = false;
            if let Some(path) = rfd {
                let project_dir = path.parent().unwrap_or(&PathBuf::from(".")).to_path_buf();
                let assets_dir = project_dir.join("assets");
                if assets_dir.exists() {
                    self.asset_path = assets_dir;
                    self.project_path = Some(project_dir.clone());
                    // Try to load last scene
                    let scene = project_dir.join("assets").join("scenes").join("scene.json");
                    if scene.exists() {
                        let _ = vp.load_scene(&scene);
                        self.add_recent_file(&scene);
                    }
                    log::info!("Opened project: {:?}", project_dir);
                }
            }
        }

        // Create folder dialog
        if self.show_create_folder_dialog {
            egui::Window::new("Create Folder")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Folder Name:");
                    ui.text_edit_singleline(&mut self.new_asset_name);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            let path = self.asset_path.join(&self.new_asset_name);
                            let _ = std::fs::create_dir_all(&path);
                            self.new_asset_name.clear();
                            self.show_create_folder_dialog = false;
                        }
                        if ui.button("Cancel").clicked() { self.show_create_folder_dialog = false; }
                    });
                });
        }

        // Open file dialog (native)
        if self.show_open_file_dialog {
            let rfd = rfd::FileDialog::new()
                .add_filter("Scene Files", &["json"])
                .set_title("Open Scene")
                .pick_file();
            self.show_open_file_dialog = false;
            if let Some(path) = rfd {
                let _ = vp.load_scene(&path);
                self.add_recent_file(&path);
                self.asset_path = path.parent().unwrap_or(&self.asset_path).to_path_buf();
            }
        }

        // Save file dialog (native)
        if self.show_save_file_dialog {
            let rfd = rfd::FileDialog::new()
                .add_filter("Scene Files", &["json"])
                .set_title("Save Scene As")
                .set_file_name("scene.json")
                .save_file();
            self.show_save_file_dialog = false;
            if let Some(path) = rfd {
                let _ = vp.save_scene(&path);
            }
        }

        // Parent selector
        if self.show_parent_selector {
            if let Some(child) = self.parent_entity {
                let ents = vp.entities.clone();
                egui::Window::new("Select Parent Entity")
                    .collapsible(false)
                    .resizable(true)
                    .default_size([250.0, 200.0])
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label("Choose a parent for this entity:");
                        ui.separator();
                        for info in &ents {
                            if info.entity == child { continue; }
                            let is_current_parent = vp.world.get::<Parent>(child).map(|p| p.0 == info.entity).unwrap_or(false);
                            let label = if is_current_parent { format!("✓ {}", info.name) } else { info.name.clone() };
                            if ui.selectable_label(false, &label).clicked() {
                                vp.set_parent(child, Some(info.entity));
                                self.show_parent_selector = false;
                            }
                        }
                        ui.separator();
                        if ui.button("❌ Clear Parent").clicked() {
                            vp.set_parent(child, None);
                            self.show_parent_selector = false;
                        }
                        if ui.button("Cancel").clicked() { self.show_parent_selector = false; }
                    });
            } else {
                self.show_parent_selector = false;
            }
        }

        // Build dialog
        if self.show_build_dialog {
            egui::Window::new("Build & Export")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Build the game into a standalone binary.");
                    ui.label("Game will be exported with all assets.");
                    ui.separator();
                    if ui.button("📁 Choose Export Folder").clicked() {
                        let folder = rfd::FileDialog::new().pick_folder();
                        if let Some(f) = folder {
                            self.export_path = f;
                        }
                    }
                    if self.export_path != PathBuf::from(".") {
                        ui.label(format!("Export to: {:?}", self.export_path));
                    } else {
                        self.export_path = PathBuf::from("export");
                        ui.label("Export to: 'export/'");
                    }
                    ui.separator();
                    if !self.build_message.is_empty() {
                        ui.label(&self.build_message);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("🏗 Build & Export").clicked() {
                            self.build_message = "Building...".to_string();
                            match vp.build_export(&self.export_path) {
                                Ok(()) => { self.build_message = "✅ Build successful!".to_string(); }
                                Err(e) => { self.build_message = format!("❌ Build failed: {}", e); }
                            }
                        }
                        if ui.button("Close").clicked() { self.show_build_dialog = false; self.build_message.clear(); }
                    });
                });
        }

        // Export project dialog
        if self.show_export_dialog {
            egui::Window::new("Export Project")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Export the entire project folder.");
                    ui.label("This copies all assets and scene files.");
                    ui.separator();
                    if ui.button("📁 Choose Export Folder").clicked() {
                        let folder = rfd::FileDialog::new().pick_folder();
                        if let Some(f) = folder {
                            self.export_path = f;
                        }
                    }
                    if self.export_path != PathBuf::from(".") {
                        ui.label(format!("Export to: {:?}", self.export_path));
                    }
                    if ui.button("📦 Export").clicked() {
                        // Save current scene first
                        if let Some(p) = vp.scene_path.clone() {
                            let _ = vp.save_scene(&p);
                        }
                        // Copy assets folder recursively
                        let src = PathBuf::from("assets");
                        if src.exists() {
                            let dst = self.export_path.join("assets");
                            let _ = std::fs::create_dir_all(&dst);
                            if let Ok(entries) = std::fs::read_dir(&src) {
                                for entry in entries.flatten() {
                                    let name = entry.file_name();
                                    let _ = std::fs::copy(entry.path(), dst.join(&name));
                                }
                            }
                        }
                        // Copy scene
                        if let Some(ref p) = vp.scene_path {
                            let _ = std::fs::copy(p, self.export_path.join("scene.json"));
                        }
                        log::info!("Project exported to {:?}", self.export_path);
                        self.show_export_dialog = false;
                    }
                    if ui.button("Cancel").clicked() { self.show_export_dialog = false; }
                });
        }

        // Texture preview window — removed.
        // Double-clicking a PNG now selects it in the Import tab instead.
        // The Import tab shows a thumbnail preview and full import settings
        // (Texture / Spritesheet, tile size, columns, rows, filter, re-import).

        // Import Settings dialog (opened from context menu or Import tab)
        if let Some(tex_id) = self.import_settings_tex {
            let tex_name = vp.texture_paths.get(&tex_id)
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("Tex{}", tex_id));
            let (tw, th) = vp.textures.get(&tex_id).map(|(_, w, h, _)| (*w, *h)).unwrap_or((0, 0));
            let settings = self.edit_import_settings.clone();
            egui::Window::new(format!("⚙️ Import Settings — {}", tex_name))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("**{}** ({}x{})", tex_name, tw, th));
                    ui.separator();

                    // Texture type selector
                    ui.label("Texture Type:");
                    egui::ComboBox::from_id_salt("import_settings_type")
                        .selected_text(settings.texture_type.as_str())
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(false, "Texture").clicked() {
                                self.edit_import_settings.texture_type = TextureType::Texture;
                            }
                            if ui.selectable_label(false, "Spritesheet").clicked() {
                                self.edit_import_settings.texture_type = TextureType::Spritesheet;
                                if self.edit_import_settings.tile_width == 0 || self.edit_import_settings.tile_height == 0 {
                                    let auto = ImportSettings::auto_tile_size(tw, th);
                                    self.edit_import_settings.tile_width = auto;
                                    self.edit_import_settings.tile_height = auto;
                                }
                            }
                        });

                    ui.add_space(8.0);

                    // Tile dimensions (only for spritesheet)
                    if self.edit_import_settings.texture_type == TextureType::Spritesheet {
                        ui.label("Tile Size:");
                        ui.horizontal(|ui| {
                            ui.label("W:");
                            ui.add(egui::DragValue::new(&mut self.edit_import_settings.tile_width).range(1..=512).speed(1));
                            ui.label("px");
                            ui.label("H:");
                            ui.add(egui::DragValue::new(&mut self.edit_import_settings.tile_height).range(1..=512).speed(1));
                            ui.label("px");
                        });
                        ui.horizontal(|ui| {
                            ui.label(format!("Grid: {}x{} ({} tiles)",
                                self.edit_import_settings.cols(tw),
                                self.edit_import_settings.rows(th),
                                self.edit_import_settings.tile_count(tw, th)));
                            if ui.button("🔧 Auto-detect").clicked() {
                                let auto = ImportSettings::auto_tile_size(tw, th);
                                self.edit_import_settings.tile_width = auto;
                                self.edit_import_settings.tile_height = auto;
                            }
                        });
                    }

                    ui.add_space(8.0);

                    // Filter mode
                    ui.label("Filter:");
                    egui::ComboBox::from_id_salt("import_settings_filter")
                        .selected_text(match settings.filter {
                            TextureFilter::Nearest => "Nearest (pixel art)",
                            TextureFilter::Linear => "Linear (smooth)",
                        })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(false, "Nearest (pixel art)").clicked() {
                                self.edit_import_settings.filter = TextureFilter::Nearest;
                            }
                            if ui.selectable_label(false, "Linear (smooth)").clicked() {
                                self.edit_import_settings.filter = TextureFilter::Linear;
                            }
                        });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui.button("🔄 Re-import").clicked() {
                            let new_settings = self.edit_import_settings.clone();
                            let _ = vp.reimport_texture(tex_id, new_settings.clone());
                            self.apply_import_settings(vp, tex_id, new_settings);
                        }
                        if ui.button("Apply").clicked() {
                            let new_settings = self.edit_import_settings.clone();
                            self.apply_import_settings(vp, tex_id, new_settings);
                        }
                        if ui.button("✕ Close").clicked() {
                            self.import_settings_tex = None;
                        }
                    });
                });
        }

        // Project Settings window
        if self.show_project_settings {
            egui::Window::new("⚙️ Project Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("📐 Display");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Virtual Width:");
                        let mut vw = vp.tex_size.0;
                        ui.add(egui::DragValue::new(&mut vw).range(64..=4096).speed(8));
                        if vw != vp.tex_size.0 {
                            if let Some(rc) = vp.ren_ctx.clone() {
                                vp.resize(&rc.dev, vw, vp.tex_size.1);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Virtual Height:");
                        let mut vh = vp.tex_size.1;
                        ui.add(egui::DragValue::new(&mut vh).range(64..=4096).speed(8));
                        if vh != vp.tex_size.1 {
                            if let Some(rc) = vp.ren_ctx.clone() {
                                vp.resize(&rc.dev, vp.tex_size.0, vh);
                            }
                        }
                    });
                    ui.checkbox(&mut vp.pixel_perfect, "Pixel Perfect Rendering");
                    ui.add_space(8.0);

                    ui.label("🧱 Grid");
                    ui.separator();
                    ui.checkbox(&mut vp.show_grid, "Show Grid");
                    ui.checkbox(&mut vp.snap_to_grid, "Snap to Grid");
                    ui.horizontal(|ui| {
                        ui.label("Grid Size:");
                        ui.add(egui::DragValue::new(&mut vp.grid_size).range(1..=128).speed(1));
                        ui.label("px");
                    });
                    ui.add_space(8.0);

                    ui.label("🎮 Controls");
                    ui.separator();
                    ui.label("W — Move tool");
                    ui.label("E — Rotate tool");
                    ui.label("R — Scale tool");
                    ui.label("C/V — Copy/Paste");
                    ui.label("Ctrl+Z/Y — Undo/Redo");
                    ui.label("Ctrl+S — Save Scene");
                    ui.label("Ctrl+O — Open Scene");
                    ui.label("Right-drag — Pan viewport");
                    ui.label("Scroll — Zoom viewport");
                    ui.add_space(8.0);

                    if ui.button("✕ Close").clicked() {
                        self.show_project_settings = false;
                    }
                });
        }
    }
}
