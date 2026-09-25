// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Declarative TOML UI Viewer & Visual Form Runner in AORUI
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use ui_core::UiEvent;
use ui_gpu::GpuRenderer;
use ui_layout::{AvailableSpace, Size};
use ui_widgets::declarative::{DeclarativeUiDoc, EventRouter};
use ui_widgets::{FontFamily, InteractionState, Theme, ThemeWatcher, WidgetTree};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

struct AppState {
    log_message: String,
    counter: usize,
}

struct DeclarativeApp {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    resources: ui_gpu::ResourceTable,
    theme: Theme,
    theme_shared: Arc<RwLock<Theme>>,
    _theme_watcher: Option<ThemeWatcher>,

    _ui_path: PathBuf,
    ui_doc: DeclarativeUiDoc,
    event_router: EventRouter<AppState>,
    app_state: AppState,

    last_tree: Option<WidgetTree>,
    last_root: Option<ui_layout::NodeId>,
    last_mouse_pos: Option<(f32, f32)>,
}

impl DeclarativeApp {
    pub fn new(ui_path: PathBuf) -> Self {
        let theme_path = PathBuf::from("themes/studio_pro.toml");
        let initial_theme = Theme::from_file(&theme_path).unwrap_or_else(|_| Theme::studio_pro());
        let theme_shared = Arc::new(RwLock::new(initial_theme.clone()));

        let watcher_target = theme_shared.clone();
        let _theme_watcher = ThemeWatcher::watch_file(&theme_path, watcher_target, |_new_theme| {
            println!("⚡ [ThemeWatcher] Rechargement à chaud du thème réussi !");
        }).ok();

        let ui_doc = DeclarativeUiDoc::from_file(&ui_path)
            .unwrap_or_else(|e| {
                eprintln!("⚠️ Impossible de charger {:?}: {} — Utilisation d'un document par défaut", ui_path, e);
                DeclarativeUiDoc::default()
            });

        let mut event_router = EventRouter::new();

        // 1. Câblage des événements applicatifs déclarés dans le TOML
        event_router.on("studio:save_toml", |state: &mut AppState, _id| {
            state.log_message = "💾 Fichier d'interface TOML exporté et enregistré avec succès !".to_string();
            println!("{}", state.log_message);
        });

        event_router.on("studio:export_code", |state: &mut AppState, _id| {
            state.log_message = "📄 Code Rust et Rhai pour la gestion des événements généré !".to_string();
            println!("{}", state.log_message);
        });

        event_router.on("studio:run_preview", |state: &mut AppState, _id| {
            state.counter += 1;
            state.log_message = format!("▶ Mode Exécution déclenché (Itération #{})", state.counter);
            println!("{}", state.log_message);
        });

        event_router.on("auth:submit", |state: &mut AppState, _id| {
            state.log_message = "🔐 Tentative d'authentification réussie !".to_string();
            println!("{}", state.log_message);
        });

        event_router.on("tool:add_button", |state: &mut AppState, _id| {
            state.log_message = "➕ Nouveau Widget Bouton ajouté au Canvas !".to_string();
            println!("{}", state.log_message);
        });

        Self {
            window: None,
            renderer: None,
            resources: ui_gpu::ResourceTable::new(),
            theme: initial_theme,
            theme_shared,
            _theme_watcher,
            _ui_path: ui_path,
            ui_doc,
            event_router,
            app_state: AppState {
                log_message: "Prêt.".to_string(),
                counter: 0,
            },
            last_tree: None,
            last_root: None,
            last_mouse_pos: None,
        }
    }

    fn font_family(family: &FontFamily) -> glyphon::Family<'_> {
        match family {
            FontFamily::SansSerif => glyphon::Family::SansSerif,
            FontFamily::Serif => glyphon::Family::Serif,
            FontFamily::Monospace => glyphon::Family::Monospace,
            FontFamily::Named(name) => glyphon::Family::Name(name.as_str()),
        }
    }

    fn redraw(&mut self) {
        if let Ok(lock) = self.theme_shared.read() {
            self.theme = lock.clone();
        }

        let (width_f, height_f) = if let Some(r) = &self.renderer {
            let (w, h) = r.window_size();
            (w as f32, h as f32)
        } else {
            return;
        };

        let mut tree = WidgetTree::new();
        let root = match self.ui_doc.build_tree(&mut tree) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Erreur lors de la construction de l'arbre déclaratif: {}", e);
                return;
            }
        };

        let Some(renderer) = self.renderer.as_mut() else { return; };

        let available = Size {
            width: AvailableSpace::Definite(width_f),
            height: AvailableSpace::Definite(height_f),
        };
        let _ = tree.compute(root, available);

        let measure = renderer.text_measure();
        let interaction = InteractionState {
            hovered: None,
            pressed: None,
            measure: Some(&measure),
        };

        if let Ok(frame) = tree.build_frame(root, &self.theme, interaction) {
            let family = Self::font_family(&self.theme.typography.family);
            let text_runs: Vec<_> = frame
                .texts
                .iter()
                .map(|spec| {
                    let align = match spec.align {
                        ui_widgets::TextAlign::Left => glyphon::cosmic_text::Align::Left,
                        ui_widgets::TextAlign::Center => glyphon::cosmic_text::Align::Center,
                        ui_widgets::TextAlign::Right => glyphon::cosmic_text::Align::Right,
                    };
                    let weight = match spec.weight {
                        ui_widgets::FontWeight::Normal => glyphon::Weight::NORMAL,
                        ui_widgets::FontWeight::Bold => glyphon::Weight::BOLD,
                    };
                    renderer.make_text_run(
                        &spec.text,
                        spec.bounds,
                        spec.font_size,
                        spec.color,
                        align,
                        family,
                        weight,
                        spec.clip,
                    )
                })
                .collect();

            let layer = ui_gpu::RenderLayer {
                instances: &frame.instances,
                texts: &text_runs,
            };

            let clear_r = (self.theme.glass_bg[0] * 0.7).clamp(0.0, 1.0) as f64;
            let clear_g = (self.theme.glass_bg[1] * 0.7).clamp(0.0, 1.0) as f64;
            let clear_b = (self.theme.glass_bg[2] * 0.7).clamp(0.0, 1.0) as f64;

            let _ = renderer.render_layers(
                wgpu::Color { r: clear_r, g: clear_g, b: clear_b, a: 1.0 },
                &[layer],
                &[],
                &self.resources,
            );
        }

        self.last_tree = Some(tree);
        self.last_root = Some(root);
    }
}

impl ApplicationHandler for DeclarativeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let w = self.ui_doc.window.width;
        let h = self.ui_doc.window.height;
        let title = &self.ui_doc.window.title;

        let window_attrs = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(w, h))
            .with_min_inner_size(winit::dpi::LogicalSize::new(800.0, 500.0));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create declarative app window"),
        );

        let size = window.inner_size();
        let mut renderer = GpuRenderer::new(window.clone());
        renderer.resize(size);

        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size);
                }
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.last_mouse_pos = Some((position.x as f32, position.y as f32));
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if let (Some(tree), Some(root), Some(pos)) = (&self.last_tree, self.last_root, self.last_mouse_pos) {
                    if let Ok(Some(UiEvent::ButtonClicked { widget_id })) = tree.dispatch_click(root, pos) {
                        println!("🖱️ Clic sur widget : {}", widget_id);
                        self.ui_doc.dispatch_click(&self.event_router, &mut self.app_state, &widget_id);
                        if let Some(win) = &self.window {
                            win.request_redraw();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    println!("============================================================");
    println!("🌌 AORUI DECLARATIVE UI RUNNER & FORM BUILDER DEMO");
    println!("📄 Chargement de l'interface depuis 'ui/form_designer_demo.toml'");
    println!("============================================================");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = DeclarativeApp::new(PathBuf::from("ui/form_designer_demo.toml"));
    let _ = event_loop.run_app(&mut app);
}
