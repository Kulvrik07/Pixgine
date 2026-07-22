mod panels; mod viewport;
use std::sync::Arc; use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::*; use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey; use winit::window::{Window, WindowId};
use egui::ViewportId; use egui_wgpu::winit::Painter; use egui_winit::State as EWS;
use panels::EP; use viewport::VP;

struct App { w: Option<Arc<Window>>, es: Option<EWS>, p: Option<Painter>, ctx: Option<egui::Context>, vp: Option<VP>, tex: Option<egui::TextureId>, ep: EP }

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        el.set_control_flow(winit::event_loop::ControlFlow::Poll);
        if self.w.is_some() { return; }
        log::info!("Creating editor");
        let wa = Window::default_attributes().with_title("Pixgine Editor").with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));
        let win = Arc::new(el.create_window(wa).unwrap());
        let ctx = egui::Context::default(); ctx.set_pixels_per_point(1.0);
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
                p.paint_and_update_textures(ViewportId::ROOT, ctx.pixels_per_point(), [0.15,0.15,0.15,1.0], &c, &fo.textures_delta, vec![]);
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(k), state, .. }, .. } => {
                if state == ElementState::Pressed {
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
                                // Undo/Redo with Ctrl
                                // We check if Ctrl is held via another method
                                vp.restore_undo();
                            }
                            winit::keyboard::KeyCode::KeyY => {
                                vp.restore_redo();
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