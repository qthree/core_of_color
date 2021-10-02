use bumpalo::{collections::Vec as BumpVec, Bump};
use colorsys::{Hsl, Rgb};
use glam::DVec2;
use hecs::{Entity, World};
use rand::{prelude::ThreadRng, Rng};
use space::{Neighbour, Neighbours};

mod space;

#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub vec: DVec2,
}

#[derive(Debug, Clone, Copy, Default)]
struct Speed {
    vec: DVec2,
}

#[derive(Debug, Clone)]
pub struct Color {
    hsl: Hsl,
}
impl Color {
    fn bytes(&self) -> [u8; 3] {
        let rgb = Rgb::from(&self.hsl);
        rgb.into()
    }
}
impl From<&Color> for eframe::egui::Color32 {
    fn from(color: &Color) -> Self {
        let rgb = color.bytes();
        eframe::egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    }
}
/*
impl From<Rgb> for Color {
    fn from(rgb: Rgb) -> Self {
        Color {
            r: rgb.red(),
            g: rgb.green(),
            b: rgb.blue(),
        }
    }
}
impl From<Hsl> for Color {
    fn from(hsl: Hsl) -> Self {
        let rgb: Rgb = hsl.into();
        rgb.into()
    }
}
*/
struct IsPlayer(Entity);

struct Player {
    dots: Vec<Entity>,
    size: f64,
    energy: f64,
    rot: f64,
}
impl Player {
    fn new() -> Self {
        Self {
            dots: vec![],
            size: 0.0,
            energy: 0.0,
            rot: 0.0,
        }
    }
    fn update(state: &mut State) {
        for (_, (player, pos /*speed*/)) in state
            .world
            .query::<(&mut Player, &Position /*&mut Speed*/)>()
            .iter()
        {
            for i in 0..player.dots.len() {
                player.update_color_dot(&state.world, i, pos.vec);
            }
        }
    }
    fn rotate(state: &mut State) {
        for (_, player) in state.world.query::<&mut Player>().iter() {
            player.rot += 0.001;
        }
    }
    fn consume_around_dot(world: &World, dot: Entity, dist: f64) -> Option<f64> {
        let neighbours = world.get::<Neighbours>(dot).ok()?;

        let mut energy = 0.0;
        for other in neighbours.slice() {
            if other.dist > dist {
                continue;
            }
            if let Some(query) = world.query_one::<&mut Color>(other.entity).ok() {
                if let Some(color) = query.without::<IsPlayer>().without::<Player>().get() {
                    let sat = color.hsl.saturation() - 0.1;
                    if sat > 0.0 {
                        color.hsl.set_saturation(sat);
                        energy += 0.0001;
                    }
                }
            }
        }
        Some(energy)
    }
    fn consume_energy(state: &mut State) {
        let mut add_dots = vec![];
        for (entity, player) in state.world.query::<&mut Player>().iter() {
            for dot in &player.dots {
                player.energy += Self::consume_around_dot(&state.world, *dot, 0.2).unwrap_or(0.0);
            }
            player.size = player.energy.powf(0.5);
            if player.size > player.dots.len() as f64 {
                add_dots.push(entity);
            }
        }
        for player in add_dots {
            Self::add_dot(&mut state.world, player);
        }
    }
    fn update_color_dot(&self, world: &World, i: usize, pos: DVec2) -> Option<()> {
        let dot = self.dots[i];
        let mut query = world.query_one::<&mut Position>(dot).ok()?;
        let dot_pos = query.get()?;

        let angle = std::f64::consts::TAU * (1.0 / self.dots.len() as f64 * i as f64 + self.rot);
        let pos = pos + DVec2::new(angle.cos(), angle.sin()) * (0.5 + self.size.powf(0.5));
        dot_pos.vec = pos;
        Some(())
    }
    fn add_dot(world: &mut World, player: Entity) -> Option<()> {
        let dots = {
            let mut player = world.get_mut::<Player>(player).ok()?;

            let dot = world.reserve_entity();
            player.dots.push(dot);
            player.dots.clone()
        };

        let color = Color {
            hsl: Hsl::default(),
        };
        let pos = Position::default();
        let is_player = IsPlayer(player);
        let neighbours = Neighbours::default();
        world.spawn_at(*dots.last().unwrap(), (color, pos, is_player, neighbours));

        let angle = (std::f64::consts::TAU / dots.len() as f64).to_degrees();

        for (i, dot) in dots.into_iter().enumerate() {
            Player::update_dot(world, dot, angle * i as f64);
        }
        Some(())
    }
    fn update_dot(world: &mut World, dot: Entity, angle: f64) {
        if let Ok(mut color) = world.get_mut::<Color>(dot) {
            color.hsl = Hsl::new(angle, 100.0, 50.0, None);
        }
    }
}

#[derive(Default)]
pub struct State {
    world: World,
    bump: Bump,
}

#[derive(Default)]
struct Random {
    rng: ThreadRng,
}
impl Random {
    fn dvec2(&mut self, amp: f64) -> DVec2 {
        DVec2::new(self.rng.gen_range(-amp..amp), self.rng.gen_range(-amp..amp))
    }
    fn color(&mut self) -> Color {
        let hsl = Hsl::new(
            self.rng.gen_range(0.0..=360.0),
            self.rng.gen_range(70.0..90.0),
            self.rng.gen_range(40.0..60.0),
            None,
        );
        //hsl.into()
        Color { hsl }
    }
}

fn position_speed(state: &mut State) {
    for (_, (pos, speed)) in state.world.query_mut::<(&mut Position, &Speed)>() {
        pos.vec += speed.vec;
    }
}

fn decelerate(state: &mut State) {
    for (_, speed) in state.world.query_mut::<&mut Speed>() {
        speed.vec *= 0.9;
    }
}

fn neighbour_attraction(color: &Color, world: &World, other: &Neighbour) -> Option<DVec2> {
    let normal = other.diff.try_normalize()?;
    if other.dist < 0.3 {
        return Some(-normal / other.dist.max(0.000001) * 0.0001);
    }

    let mut query = world.query_one::<&Color>(other.entity).ok()?;
    let other_color = query.get()?;

    let mut color_diff = (color.hsl.hue() - other_color.hsl.hue()).abs();
    if color_diff > 180.0 {
        color_diff = 360.0 - color_diff;
    }
    color_diff = color_diff + 54.0; //(360.0 / DOTS_NUMBER as f64 * 0.75);
    let force = 1.0 - (color_diff / 90.0);
    let force = force.abs().powf(0.5) * force.signum();

    let sunction = 1.0 - other_color.hsl.lightness() / 50.0;
    let sunction = sunction * sunction * sunction;

    let power = other_color.hsl.saturation() / 200.0 + color.hsl.saturation() / 200.0;
    assert!(power.is_finite());
    assert!(force.is_finite());
    let power = power * power * power;

    //dbg!(force, power);

    let dist = other.dist * other.dist * other.dist;
    let dist = 1.0 / dist;

    Some(normal * 0.01 * dist * (power * force + sunction))
}

fn attract(state: &mut State) {
    for (_, (color, speed, neighbours)) in state
        .world
        .query::<(&Color, &mut Speed, &Neighbours)>()
        .without::<Player>()
        .without::<IsPlayer>()
        .iter()
    {
        for other in neighbours.slice() {
            if let Some(attraction) = neighbour_attraction(color, &state.world, other) {
                speed.vec += attraction;
            }
        }
    }
}

impl State {
    pub fn tick(&mut self) {
        self.bump.reset();
        Player::rotate(self);
        Player::consume_energy(self);
        Player::update(self);
        position_speed(self);
        decelerate(self);
        Neighbours::update(self, 10.0);
        attract(self);
        //std::thread::sleep(Duration::from_micros(1000/60));
    }

    pub fn player_input(&self, player: Entity, dir: DVec2) -> Option<()> {
        let mut query = self
            .world
            .query_one::<&mut Speed>(player)
            .ok()?
            .with::<Player>();
        let speed = query.get()?;
        speed.vec += dir * 0.01;
        Some(())
    }
    pub fn position(&self, entity: Entity) -> Option<Position> {
        let mut query = self.world.query_one::<&Position>(entity).ok()?;
        query.get().copied()
    }

    pub fn batch_spawn_dots(&mut self, n: usize) {
        let mut random = Random::default();

        let to_spawn = (0..n).map(|_| {
            let pos = Position {
                vec: random.dvec2(10.0),
            };
            let speed = Speed {
                vec: random.dvec2(0.001),
            };
            let color = random.color();
            let neighbours = Neighbours::default();

            (pos, speed, color, neighbours)
        });

        self.world.spawn_batch(to_spawn);
    }

    pub fn spawn_player(&mut self) -> Entity {
        //let mut random = Random::default();

        let player = self.world.reserve_entity();
        /*let pos = Position {
            vec: random.dvec2(10.0),
        };*/
        let pos = Position::default();
        let speed = Speed::default();
        let player_component = Player::new();
        let rgb = Rgb::new(255.0, 255.0, 255.0, None);
        let color = Color { hsl: rgb.into() };
        self.world
            .spawn_at(player, (player_component, pos, speed, color));

        Player::add_dot(&mut self.world, player);
        player
    }

    pub fn dots(&self) -> BumpVec<'_, Dot> {
        let mut query = self.world.query::<(&Color, &Position)>();
        let iter = query.iter().map(|(_, (color, &pos))| Dot {
            color: color.clone(),
            pos,
        });
        BumpVec::from_iter_in(iter, &self.bump)
    }
}

#[derive(Debug)]
pub struct Dot {
    pub color: Color,
    pub pos: Position,
}
