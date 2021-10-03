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
pub struct Size(pub f64);

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
    energy: f64,
    rot: f64,
}
impl Player {
    fn new() -> Self {
        Self {
            dots: vec![],
            //energy: 8.7f64.powf(2.0),
            energy: 1.0,
            rot: 0.0,
        }
    }
    fn update(state: &mut State) {
        for (_, (player, pos, size)) in state
            .world
            .query::<(&mut Player, &Position, &Size)>()
            .iter()
        {
            let size = size.0.powf(0.5);
            for i in 0..player.dots.len() {
                player.update_dot_position(&state.world, i, pos.vec, size);
            }
        }
    }
    fn rotate(state: &mut State) {
        for (_, player) in state.world.query::<&mut Player>().iter() {
            player.rot += 0.001;
        }
    }
    fn consume_around_dot(&self, world: &World, dot: Entity, dist: f64) -> Option<f64> {
        let neighbours = world.get::<Neighbours>(dot).ok()?;
        let energy_size = self.energy_size();
        let rate = if Player::is_blackhole(energy_size) {
            energy_size
        } else {
            1.0
        };

        let mut energy = 0.0;
        for other in neighbours.slice() {
            if other.dist > dist * rate {
                continue;
            }
            if let Some(query) = world.query_one::<&mut Color>(other.entity).ok() {
                if let Some(color) = query.without::<IsPlayer>().without::<Player>().get() {
                    let sat = color.hsl.saturation();
                    if sat > 0.0 {
                        let sat = (sat - 0.01 * rate * rate).max(0.0);
                        color.hsl.set_saturation(sat);
                        energy += 0.0003 * rate;
                    }
                }
            }
        }
        Some(energy)
    }
    fn consume_energy(state: &mut State) {
        for (_entity, (player,)) in state.world.query::<(&mut Player,)>().iter() {
            for dot in &player.dots {
                player.energy += player
                    .consume_around_dot(&state.world, *dot, 0.2)
                    .unwrap_or(0.0);
            }
        }
    }
    fn update_dot_position(&self, world: &World, i: usize, pos: DVec2, size: f64) -> Option<()> {
        let dot = self.dots[i];
        let mut query = world.query_one::<&mut Position>(dot).ok()?;
        let dot_pos = query.get()?;

        let angle = std::f64::consts::TAU * (1.0 / self.dots.len() as f64 * i as f64 + self.rot);
        let pos = pos + DVec2::new(angle.cos(), angle.sin()) * size;
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
    fn energy_size(&self) -> f64 {
        self.energy.powf(0.5)
    }
    fn is_blackhole(size: f64) -> bool {
        size > 9.0
    }
    fn grow(state: &mut State) {
        let mut add_dots = vec![];
        for (entity, (player, size, color)) in state
            .world
            .query::<(&mut Player, &mut Size, &mut Color)>()
            .iter()
        {
            let new_size = player.energy_size();
            if Player::is_blackhole(new_size) {
                size.0 = (size.0 - 0.1).max(0.01);
                color.hsl.set_lightness((size.0 - 1.0) / 8.0 * 100.0);
                player.set_dots_lightness(&state.world, color.hsl.lightness());
            } else {
                if new_size > size.0 {
                    size.0 = new_size;
                }
                if new_size >= (player.dots.len() + 1) as f64 {
                    add_dots.push(entity);
                }
            }
        }
        for player in add_dots {
            Self::add_dot(&mut state.world, player);
        }
    }
    fn set_dots_lightness(&self, world: &World, lightness: f64) {
        for dot in &self.dots {
            if let Ok(mut color) = world.get_mut::<Color>(*dot) {
                color.hsl.set_lightness(lightness);
            }
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

fn global_gravity(state: &mut State) {
    for (_, (pos, speed)) in state.world.query_mut::<(&Position, &mut Speed)>() {
        let dist = pos.vec.length();
        speed.vec -= pos.vec.normalize_or_zero() * (dist * 0.001).powf(2.0);
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

    Some(normal * 0.01 * (dist * (power * force) + sunction * 0.3))
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

fn heat_death(state: &mut State) {
    let mut despawn = BumpVec::new_in(&state.bump);
    let count = state
        .world
        .query::<(&Color,)>()
        .without::<Player>()
        .without::<IsPlayer>()
        .iter()
        .map(|(entity, (color,))| {
            if color.hsl.saturation() <= 0.1 {
                despawn.push(entity);
            }
        })
        .count();
    for entity in despawn {
        let _ = state.world.despawn(entity);
    }
    if count < 10 {
        restart(state);
    }
}

fn restart(state: &mut State) {
    {
        let mut despawn = BumpVec::new_in(&state.bump);
        for (dot, _) in state.world.query::<&IsPlayer>().iter() {
            despawn.push(dot);
        }
        for entity in despawn {
            let _ = state.world.despawn(entity);
        }
    }
    {
        let mut players_to_respawn = BumpVec::new_in(&state.bump);
        for (player, _) in state.world.query::<&Player>().iter() {
            players_to_respawn.push(player);
        }
        for entity in players_to_respawn {
            state.respawn_player(entity);
        }
    }
    state.batch_spawn_dots(1000);
}

impl State {
    pub fn tick(&mut self) {
        self.bump.reset();
        heat_death(self);
        global_gravity(self);
        position_speed(self);
        Player::rotate(self);
        Player::consume_energy(self);
        Player::grow(self);
        Player::update(self);
        decelerate(self);
        let count = self.world.query::<&Neighbours>().iter().count() as f64;
        Neighbours::update(self, (10000.0 / count).clamp(10.0, 10000.0));
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
        self.world.get::<Position>(entity).ok().as_deref().copied()
    }
    pub fn size(&self, entity: Entity) -> Option<Size> {
        self.world.get::<Size>(entity).ok().as_deref().copied()
    }

    pub fn batch_spawn_dots(&mut self, n: usize) {
        let mut random = Random::default();

        let to_spawn = (0..n).map(|_| {
            let pos = Position {
                vec: random.dvec2(10.0),
            };
            let speed = Speed {
                vec: pos.vec,
            };
            let color = random.color();
            let neighbours = Neighbours::default();

            (pos, speed, color, neighbours)
        });

        self.world.spawn_batch(to_spawn);
    }

    pub fn respawn_player(&mut self, player: Entity) {
        let pos = Position::default();
        let speed = Speed::default();
        let size = Size(1.0);
        let player_component = Player::new();
        let rgb = Rgb::new(255.0, 255.0, 255.0, None);
        let color = Color { hsl: rgb.into() };
        self.world
            .spawn_at(player, (player_component, pos, speed, size, color));

        Player::add_dot(&mut self.world, player);
    }

    pub fn spawn_player(&mut self) -> Entity {
        let player = self.world.reserve_entity();
        self.respawn_player(player);
        player
    }

    pub fn dots(&self) -> BumpVec<'_, Dot> {
        let mut query = self.world.query::<(&Color, &Position, Option<&Size>)>();
        let iter = query.iter().map(|(_, (color, &pos, size))| Dot {
            color: color.clone(),
            pos,
            size: size.unwrap_or(&Size(1.0)).0.powf(0.5) as f32,
        });
        BumpVec::from_iter_in(iter, &self.bump)
    }
}

#[derive(Debug)]
pub struct Dot {
    pub color: Color,
    pub pos: Position,
    pub size: f32,
}
