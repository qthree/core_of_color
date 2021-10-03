use crate::state::{self, Color};
use eframe::{
    egui::{self, Color32, Frame},
    epi,
};
use glam::DVec2;
use hecs::Entity;

pub struct App {
    state: state::State,
    player: Entity,
    tick: f64,
}

impl App {
    pub fn new() -> Self {
        let mut state = state::State::default();
        state.batch_spawn_dots(1000);
        let player = state.spawn_player();
        Self { state, player, tick: 0.0 }
    }
    fn input_dir(ctx: &egui::CtxRef) -> DVec2 {
        use egui::Key;
        let input = ctx.input();

        if input.pointer.any_down() {
            if let Some(pos) = input.pointer.interact_pos() {
                let rect = ctx.available_rect();
                let x = pos.x - rect.width() / 2.0;
                let y = pos.y - rect.height() / 2.0;
                return DVec2::new(x as f64, -y as f64).normalize_or_zero();
            }
        }

        let left = input.key_down(Key::A) || input.key_down(Key::ArrowLeft);
        let right = input.key_down(Key::D) || input.key_down(Key::ArrowRight);
        let up = input.key_down(Key::W) || input.key_down(Key::ArrowUp);
        let down = input.key_down(Key::S) || input.key_down(Key::ArrowDown);

        let left_xor_right = left ^ right;
        let up_xor_down = up ^ down;

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
        "Core of Color"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        let input_dir = Self::input_dir(ctx);
        self.state.player_input(self.player, input_dir);
        self.state.tick();
        let player_pos = self.state.position(self.player).unwrap().vec;
        let player_size = self.state.size(self.player).unwrap().0;
        let mut misc_dots = player_size.floor();
        let scale = 40.0;

        egui::CentralPanel::default()
            .frame(Frame::dark_canvas(&*ctx.style()))
            .show(&ctx, |ui| {
                let transform = |pos: DVec2| {
                    let scale = DVec2::new(scale, -scale);
                    let transition = DVec2::new(
                        ui.available_width() as f64 * 0.5,
                        ui.available_height() as f64 * 0.5,
                    ) - player_pos * scale;
                    let pos = pos * scale + transition;
                    egui::pos2(pos.x as f32, pos.y as f32)
                };
                let circle = egui::Shape::circle_stroke(
                    transform(player_pos),
                    (scale * player_size.powf(0.5)) as f32,
                    (1.0, Color32::from_rgba_premultiplied(10, 10, 10, 10)),
                );
                let mut shapes = vec![circle];
                while misc_dots >= 1.0 {
                    let frac = std::f64::consts::TAU/3.0;
                    let transform = |vec| transform(player_pos + vec);
                    shapes.push(misc_dot(misc_dots, self.tick, transform));
                    shapes.push(misc_dot(misc_dots, self.tick+frac, transform));
                    shapes.push(misc_dot(misc_dots, self.tick+frac*2.0, transform));
                    misc_dots -= 1.0;
                }
                let dots = self.state.dots();
                shapes.extend(dots.iter().map(|dot| {
                    let center = transform(dot.pos.vec);
                    let color = Color32::from(&dot.color);
                    let radius = dot.size * 3.0;
                    if dot.hollow {
                        egui::Shape::Circle{center, radius, fill: Color32::BLACK, stroke: (0.5, color).into()}
                    } else {
                        egui::Shape::circle_filled(center, radius, color)
                    }
                }));
                ui.painter().extend(shapes);
            });
        self.tick += 0.1;
        ctx.request_repaint();
    }
}

fn misc_dot(misc_dots: f64, tick: f64, transform: impl Fn(DVec2) -> egui::Pos2) -> egui::Shape {
    let pos = DVec2::new((- tick + misc_dots * 0.5).cos() * misc_dots * 0.5, (- tick + misc_dots * 0.5).sin() * misc_dots * 0.25);
    let center = transform(pos*0.5);
    let color = Color32::from(Color32::WHITE);
    egui::Shape::circle_filled(center, 1.0, color)
}