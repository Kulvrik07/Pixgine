mod panels; mod viewport;
use std::sync::Arc; use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::*; use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey; use winit::window::{Window, WindowId};
use egui::{ViewportId, Visuals, Stroke, Color32};
use egui_wgpu::winit::Painter; use egui_winit::State as EWS;
use panels::EP; use viewport::VP;

struct App { w: Option<Arc<Window>>, es: Option<EWS>, p: Option<Painter>, ctx: Option<egui::Context>, vp: Option<VP>, tex: Option<egui::TextureId>, ep: EP }

/// Build a polished dark theme for the editor UI.
fn setup_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(220, 225, 235));
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.panel_fill = Color32::from_rgb(22, 25, 32);
    visuals.window_fill = Color32::from_rgb(26, 30, 38);
    visuals.button_frame = false;
    // Widget colors
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(55, 60, 70));
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(80, 140, 210));
    ctx.set_visuals(visuals);

    // Text styles — slightly larger for readability
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::new(13.0, egui::FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::new(11.0, egui::FontFamily::Proportional));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::new(12.0, egui::FontFamily::Proportional));
    ctx.set_style(style);
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        el.set_control_flow(winit::event_loop::ControlFlow::Poll);
        if self.w.is_some() { return; }
        log::info!("Creating editor");
        let wa = Window::default_attributes().with_title("Pixgine Editor").with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));
        let win = Arc::new(el.create_window(wa).unwrap());
        let ctx = egui::Context::default(); ctx.set_pixels_per_point(1.0);
        setup_theme(&ctx);
        let es = EWS::new(ctx.clone(), ViewportId::ROOT, &win, None, None, None);
        let mut p = pollster::block_on(Painter::new(ctx.clone(), egui_wgpu::WgpuConfiguration::default(), 1, None, false, false));
        pollster::block_on(p.set_window(ViewportId::ROOT, Some(win.clone()))).unwrap();
        let rs = p.render_state().unwrap();
        let mut vp = VP::new(&rs.device, &rs.queue).unwrap(); vp.add();
        let egui_renderer = rs.renderer.clone();
        vp.set_renderer(rs.device.clone(), rs.queue.clone(), egui_renderer);
        let tex = vp.rtv.as_ref().map(|tv| { let mut r = rs.renderer.write(); r.register_native_texture(&rs.device, tv, wgpu::FilterMode::Nearest) });
        self.ep.set_tex(tex);
        self.w = Some(win); self.es = Some(es); self.p = Some(p); self.ctx = Some(ctx); self.vp = Some(vp); self.tex = tex;
        log::info!("Editor ready");
    }
    fn about_to_wait(&mut self, _: &ActiveEventLoop) { if let Some(w) = &self.w { w.request_redraw(); } }
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        if let (Some(es), Some(w)) = (&mut self.es, &self.w) { let _ = es.on_window_event(w, &ev); }
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => { if let Some(p) = &mut self.p { p.on_window_resized(ViewportId::ROOT, std::num::NonZeroU32::new(s.width.max(1)).unwrap(), std::num::NonZeroU32::new(s.height.max(1)).unwrap()); } }
            WindowEvent::RedrawRequested => {
                let (Some(es), Some(p), Some(w), Some(ctx)) = (&mut self.es, &mut self.p, &self.w, &self.ctx) else { return; };
                if let Some(rs) = p.render_state() { if let Some(ref mut vp) = self.vp { vp.render(&rs.device, &rs.queue); } }
                let ri = es.take_egui_input(w);
                let fo = ctx.run(ri, |egui_ctx| { if let Some(ref mut vp) = self.vp { self.ep.show(egui_ctx, vp); } });
                let c = ctx.tessellate(fo.shapes, ctx.pixels_per_point());
                p.paint_and_update_textures(ViewportId::ROOT, ctx.pixels_per_point(), [0.08,0.09,0.12,1.0], &c, &fo.textures_delta, vec![]);
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(k), state, .. }, .. } => {
                if state == ElementState::Pressed {
                    // Check Ctrl modifier from egui context before borrowing vp
                    let ctrl = self.ctx.as_ref()
                        .map(|c| c.input(|i| i.modifiers.contains(egui::Modifiers::CTRL)))
                        .unwrap_or(false);
                    if let Some(ref mut vp) = self.vp {
                        match k {
                            winit::keyboard::KeyCode::KeyW => {
                                vp.transform_mode = viewport::TransformMode::Move;
                            }
                            winit::keyboard::KeyCode::KeyE => {
                                vp.transform_mode = viewport::TransformMode::Rotate;
                            }
                            winit::keyboard::KeyCode::KeyR => {
                                vp.transform_mode = viewport::TransformMode::Scale;
                            }
                            winit::keyboard::KeyCode::KeyC => {
                                vp.copy_selected();
                            }
                            winit::keyboard::KeyCode::KeyV => {
                                vp.paste_entity();
                            }
                            winit::keyboard::KeyCode::KeyZ => {
                                if ctrl { vp.restore_undo(); }
                            }
                            winit::keyboard::KeyCode::KeyY => {
                                if ctrl { vp.restore_redo(); }
                            }
                            winit::keyboard::KeyCode::KeyS => {
                                if ctrl {
                                    if let Some(p) = vp.scene_path.clone() { let _ = vp.save_scene(&p); }
                                }
                            }
                            winit::keyboard::KeyCode::KeyO => {
                                if ctrl { self.ep.show_open_file_dialog = true; }
                            }
                            winit::keyboard::KeyCode::Equal => {
                                // Zoom in
                                if vp.pixel_perfect {
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
                            winit::keyboard::KeyCode::Minus => {
                                // Zoom out
                                if vp.pixel_perfect {
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
                            winit::keyboard::KeyCode::Digit0 => {
                                if ctrl {
                                    vp.view_scale = 1.0; vp.view_offset = (0.0, 0.0);
                                    vp.target_view_scale = 1.0; vp.target_view_offset = (0.0, 0.0);
                                }
                            }
                            winit::keyboard::KeyCode::Delete | winit::keyboard::KeyCode::Backspace => {
                                if let Some(e) = vp.selected { vp.remove(e); }
                            }
                            _ => {}
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                // Entity selection in viewport handled via egui in panels
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    EventLoop::new()?.run_app(&mut App { w: None, es: None, p: None, ctx: None, vp: None, tex: None, ep: EP::new() })?;
    Ok(())
}
