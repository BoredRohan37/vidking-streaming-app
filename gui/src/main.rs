//! MovieBox desktop app: a lightweight, software-rendered GUI for the MovieBox engine.
//! Runs on Windows 7 / 8.1 and up without needing a GPU or OpenGL.

#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use moviebox_gui::app::App;
use moviebox_gui::data::Backend;
use moviebox_gui::raster::Raster;
use moviebox_gui::store::Store;
use moviebox_gui::{demo, theme};
use egui::{pos2, vec2, Rect, ViewportId};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Icon, Window, WindowId};

const ICON_PNG: &[u8] = include_bytes!("../assets/icon-256.png");
const BG_PIXEL: u32 = 0x000A0A0F;

fn window_icon() -> Option<Icon> {
    let img = image::load_from_memory(ICON_PNG).ok()?.to_rgba8();
    let (w, h) = (img.width(), img.height());
    Icon::from_rgba(img.into_raw(), w, h).ok()
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let text = format!("MovieBox crashed: {info}\n");
        if let Some(dir) = moviebox_tui::config::config_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("gui_crash.log"), &text);
        }
        eprintln!("{text}");
    }));
}

/// Run one egui frame and rasterise it into `fb`. Returns the repaint delay requested by egui.
fn render_frame(
    ctx: &egui::Context,
    app: &mut App,
    raster: &mut Raster,
    raw: egui::RawInput,
    fb: &mut [u32],
    w: usize,
    h: usize,
) -> (egui::FullOutput, Duration) {
    let mut full = ctx.run_ui(raw, |ui| app.ui(ui));
    let ppp = full.pixels_per_point;
    let shapes = std::mem::take(&mut full.shapes);
    let prims = ctx.tessellate(shapes, ppp);
    let mut textures = std::mem::take(&mut full.textures_delta);
    raster.set_textures(&textures);
    fb.fill(BG_PIXEL);
    raster.paint(fb, w, h, ppp, &prims);
    raster.free_textures(&textures);
    textures.clear();
    let delay = full
        .viewport_output
        .get(&ViewportId::ROOT)
        .map(|v| v.repaint_delay)
        .unwrap_or(Duration::MAX);
    (full, delay)
}

// ---------------------------------------------------------------- window mode

struct Gfx {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    state: egui_winit::State,
}

struct Runner {
    proxy: EventLoopProxy<()>,
    ctx: egui::Context,
    app: Option<App>,
    gfx: Option<Gfx>,
    raster: Raster,
    next_repaint: Option<Instant>,
}

impl ApplicationHandler<()> for Runner {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("MovieBox")
            .with_inner_size(LogicalSize::new(1360.0, 860.0))
            .with_min_inner_size(LogicalSize::new(1040.0, 640.0))
            .with_window_icon(window_icon());
        let window = match el.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("Could not create a window: {e}");
                el.exit();
                return;
            }
        };
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Could not start the display: {e}");
                el.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Could not create the drawing surface: {e}");
                el.exit();
                return;
            }
        };
        let state = egui_winit::State::new(
            self.ctx.clone(),
            ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        let proxy = self.proxy.clone();
        let (tx, rx) = mpsc::channel();
        let be = Backend::new(
            tx,
            Arc::new(move || {
                let _ = proxy.send_event(());
            }),
            true,
        );
        self.app = Some(App::new(self.ctx.clone(), be, rx, Store::load()));
        window.request_redraw();
        self.gfx = Some(Gfx { window, surface, state });
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, _event: ()) {
        if let Some(g) = &self.gfx {
            g.window.request_redraw();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(g) = self.gfx.as_mut() else { return };
        let resp = g.state.on_window_event(&g.window, &event);
        match event {
            WindowEvent::CloseRequested => {
                if let Some(app) = &self.app {
                    app.persist();
                }
                el.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => g.window.request_redraw(),
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
        if resp.repaint {
            if let Some(g) = &self.gfx {
                g.window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        match self.next_repaint {
            Some(t) if t <= Instant::now() => {
                self.next_repaint = None;
                if let Some(g) = &self.gfx {
                    g.window.request_redraw();
                }
                el.set_control_flow(ControlFlow::Wait);
            }
            Some(t) => el.set_control_flow(ControlFlow::WaitUntil(t)),
            None => el.set_control_flow(ControlFlow::Wait),
        }
    }
}

impl Runner {
    fn redraw(&mut self) {
        let (Some(g), Some(app)) = (self.gfx.as_mut(), self.app.as_mut()) else { return };
        let size = g.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if g.surface.resize(w, h).is_err() {
            return;
        }
        let raw = g.state.take_egui_input(&g.window);
        let Ok(mut buffer) = g.surface.buffer_mut() else { return };
        let (full, delay) = render_frame(&self.ctx, app, &mut self.raster, raw, &mut buffer, size.width as usize, size.height as usize);
        let _ = buffer.present();
        g.state.handle_platform_output(&g.window, full.platform_output);
        if delay.is_zero() {
            g.window.request_redraw();
        } else if delay < Duration::from_secs(3600) {
            self.next_repaint = Some(Instant::now() + delay);
        }
    }
}

fn run_window() {
    let event_loop = match EventLoop::<()>::with_user_event().build() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Could not start the event loop: {e}");
            return;
        }
    };
    let ctx = egui::Context::default();
    theme::apply(&ctx);
    let mut runner = Runner {
        proxy: event_loop.create_proxy(),
        ctx,
        app: None,
        gfx: None,
        raster: Raster::new(),
        next_repaint: None,
    };
    if let Err(e) = event_loop.run_app(&mut runner) {
        eprintln!("Event loop error: {e}");
    }
}

// ---------------------------------------------------------------- screenshot mode

/// `moviebox-gui --screenshot out.png [page] [WxH] [hover=x,y] [scroll=N]`
fn screenshot(args: &[String]) {
    let out = args.first().cloned().unwrap_or_else(|| "shot.png".to_string());
    let page = args.get(1).cloned().unwrap_or_else(|| "home".to_string());
    let size = args.get(2).cloned().unwrap_or_else(|| "1440x900".to_string());
    let (w, h) = size
        .split_once('x')
        .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?)))
        .unwrap_or((1440, 900));
    let mut hover: Option<(f32, f32)> = None;
    let mut scroll = 0.0f32;
    let mut ppp = 1.0f32;
    let mut touch = false;
    for a in &args[3.min(args.len())..] {
        if a == "touch" {
            touch = true;
        }
        if let Some(v) = a.strip_prefix("ppp=") {
            ppp = v.parse().unwrap_or(1.0);
        }
        if let Some(v) = a.strip_prefix("hover=") {
            if let Some((x, y)) = v.split_once(',') {
                hover = Some((x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0)));
            }
        }
        if let Some(v) = a.strip_prefix("scroll=") {
            scroll = v.parse().unwrap_or(0.0);
        }
    }

    let ctx = egui::Context::default();
    theme::apply(&ctx);
    let (tx, rx) = mpsc::channel();
    let be = Backend::new(tx, Arc::new(|| {}), false);
    let mut app = App::new(ctx.clone(), be, rx, Store::default());
    app.touch = touch;
    demo::seed(&mut app, &page);
    let mut raster = Raster::new();
    let (pw, ph) = ((w as f32 * ppp) as usize, (h as f32 * ppp) as usize);
    let mut fb = vec![BG_PIXEL; pw * ph];
    let frames = 14;
    for i in 0..frames {
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w as f32, h as f32))),
            time: Some(20.0 + i as f64 * 0.1),
            ..Default::default()
        };
        raw.viewports.entry(ViewportId::ROOT).or_default().native_pixels_per_point = Some(ppp);
        if let Some((x, y)) = hover {
            raw.events.push(egui::Event::PointerMoved(pos2(x, y)));
        }
        if scroll > 0.0 && i == 3 {
            raw.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -scroll),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: egui::Modifiers::NONE,
            });
        }
        render_frame(&ctx, &mut app, &mut raster, raw, &mut fb, pw, ph);
    }
    let mut img = image::RgbImage::new(pw as u32, ph as u32);
    for (i, px) in fb.iter().enumerate() {
        let (x, y) = ((i % pw) as u32, (i / pw) as u32);
        img.put_pixel(x, y, image::Rgb([(px >> 16) as u8, (px >> 8) as u8, *px as u8]));
    }
    match img.save(&out) {
        Ok(()) => println!("saved {out}"),
        Err(e) => eprintln!("could not save {out}: {e}"),
    }
}

fn main() {
    install_panic_hook();
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--screenshot") {
        screenshot(&args[i + 1..]);
        return;
    }
    run_window();
}
