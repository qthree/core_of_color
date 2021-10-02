use crate::state;
use eframe::{egui::{self, Color32, Frame}, epi};
use glam::DVec2;
use hecs::Entity;

pub struct App {
    state: state::State,
    player: Entity,
}

impl App {
    pub fn new() -> Self {
        let mut state = state::State::default();
        state.batch_spawn_dots(1000);
        let player = state.spawn_player();
        Self{state, player}
    }
    fn input_dir(ctx: &egui::CtxRef) -> DVec2 {
        use egui::Key;
        let input = ctx.input();
        
        let left = input.key_down(Key::ArrowLeft);
        let right = input.key_down(Key::ArrowRight);
        let up = input.key_down(Key::ArrowUp);
        let down = input.key_down(Key::ArrowDown);

        let left_xor_right = left^right;
        let up_xor_down = up^down;

        let mul = if left_xor_right ^ up_xor_down {
            1.0
        } else if left_xor_right && up_xor_down {
            0.7
        } else {
            return DVec2::new(0.0, 0.0);
        };
        let x = if left_xor_right {
            if left {
                -mul
            } else {
                mul
            }
        } else {
            0.0
        };
        let y = if up_xor_down {
            if down {
                -mul
            } else {
                mul
            }
        } else {
            0.0
        };
        DVec2::new(x, y)
    }
}

impl epi::App for App {
    
    fn name(&self) -> &str {
        "Red-Green-Blue"
    }
    /*
    #[allow(unused_variables)]
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        storage: Option<&dyn epi::Storage>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn max_size_points(&self) -> egui::Vec2 {
        self.backend_panel.max_size_points_active
    }

    fn clear_color(&self) -> egui::Rgba {
        egui::Rgba::TRANSPARENT // we set a `CentralPanel` fill color in `demo_windows.rs`
    }

    fn warm_up_enabled(&self) -> bool {
        // The example windows use a lot of emojis. Pre-cache them by running one frame where everything is open
        cfg!(not(debug_assertions))
    }
    */

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        
        let input_dir = Self::input_dir(ctx);
        self.state.player_input(self.player, input_dir);
        self.state.tick();
        let player_pos = self.state.position(self.player).unwrap().vec;

        /*egui::Window::new("Dots").show(ctx, |ui|{
            
        });*/
        
        egui::CentralPanel::default().frame(Frame::dark_canvas(&*ctx.style())).show(&ctx, |ui| {
            let dots = self.state.dots();
            let shapes: Vec<_> = dots.iter().map(|dot| {
                let scale = DVec2::new(40.0, -40.0);
                let transition = DVec2::new(ui.available_width() as f64 * 0.5, ui.available_height() as f64 * 0.5) - player_pos * scale;
                let pos = dot.pos.vec * scale + transition;
                let center = egui::pos2(pos.x as f32, pos.y as f32);
                let color = Color32::from(&dot.color);
                egui::Shape::circle_filled(center, 3.0, color)
            }).collect();
            ui.painter().extend(shapes);
        });
        ctx.request_repaint();
        
        /*if let Some(web_info) = frame.info().web_info.as_ref() {
            if let Some(anchor) = web_info.web_location_hash.strip_prefix('#') {
                self.selected_anchor = anchor.to_owned();
            }
        }

        if self.selected_anchor.is_empty() {
            self.selected_anchor = self.apps.iter_mut().next().unwrap().0.to_owned();
        }

        egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            egui::trace!(ui);
            self.bar_contents(ui, frame);
        });

        self.backend_panel.update(ctx, frame);

        if self.backend_panel.open || ctx.memory().everything_is_visible() {
            egui::SidePanel::left("backend_panel").show(ctx, |ui| {
                self.backend_panel.ui(ui, frame);
            });
        }

        for (anchor, app) in self.apps.iter_mut() {
            if anchor == self.selected_anchor || ctx.memory().everything_is_visible() {
                app.update(ctx, frame);
            }
        }

        self.backend_panel.end_of_frame(ctx);

        self.ui_file_drag_and_drop(ctx);*/
    }
}
