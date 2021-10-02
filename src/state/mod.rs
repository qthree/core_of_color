use std::time::Duration;

use bumpalo::{collections::Vec as BumpVec, Bump};
use colorsys::{Hsl, Rgb};
use glam::DVec2;
use hecs::{Entity, World};
use rand::{prelude::ThreadRng, Rng};
use space::{Neighbours, Neighbour};

mod space;

const DOTS_NUMBER: usize = 6;

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
    fn bytes(&self) -> [u8;3] {
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
    dots: [Entity; DOTS_NUMBER],
    size: f64,
    rot: f64,
}
impl Player {
    fn update(state: &mut State) {
        for (_, (player, pos, /*speed*/)) in state.world.query::<(&Player, &Position, /*&mut Speed*/)>().iter() {
            for i in 0..DOTS_NUMBER {
                player.update_color_dot(&state.world, i, pos.vec);
                //if let Some(acceleration) = player.take_dot_speed(&state.world, i) {
                //    speed.vec += acceleration.powf(3.0); // / DOTS_NUMBER as f64 * 0.5
                //}
            }
        }
    }
    fn rotate(state: &mut State) {
        for (_, player) in state.world.query::<&mut Player>().iter() {
            player.rot += 0.001;
        }
    }
    fn update_color_dot(&self, world: &World, i: usize, pos: DVec2) -> Option<()> {
        let dot = self.dots[i];
        let mut query = world.query_one::<&mut Position>(dot).ok()?;
        let dot_pos = query.get()?;

        let angle = std::f64::consts::TAU * (1.0 / DOTS_NUMBER as f64 * i as f64 + self.rot);
        let pos = pos + DVec2::new(angle.cos(), angle.sin()) * self.size;
        dot_pos.vec = pos;
        Some(())
    }
    /*fn take_dot_speed(&self, world: &World, i: usize) -> Option<DVec2> {
        let dot = self.dots[i];
        let mut query = world.query_one::<&mut Speed>(dot).ok()?;
        let dot_pos = query.get()?;

        Some(std::mem::take(&mut dot_pos.vec))
    }*/
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
            self.rng.gen_range(40.0..80.0),
            self.rng.gen_range(40.0..60.0),
            None,
        );
        //hsl.into()
        Color{hsl}
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

fn neighbour_attraction(color: &Color, world: &World, other: &Neighbour) -> Option<DVec2>{
    let normal = other.diff.try_normalize()?;
    if other.dist < 0.3 {
        return Some(-normal / other.dist.max(0.000001) * 0.0001);
    }

    let mut query= world.query_one::<&Color>(other.entity).ok()?;
    let other_color = query.get()?;

    let mut color_diff = (color.hsl.hue() - other_color.hsl.hue()).abs();
    if color_diff > 180.0 {
        color_diff = 360.0 - color_diff;
    }
    color_diff = color_diff + 72.0; //(360.0 / DOTS_NUMBER as f64 * 0.75);
    let force = 1.0 - (color_diff / 90.0);
    let force = force.abs().powf(0.5) * force.signum();

    let sunction = 1.0 - other_color.hsl.lightness() / 50.0;
    let sunction = sunction * sunction * sunction;

    let power = other_color.hsl.saturation() / 100.0;
    assert!(power.is_finite());
    assert!(force.is_finite());
    let power = power * power * power;

    //dbg!(force, power);

    let dist = other.dist * other.dist * other.dist;
    let dist = 1.0/dist;

    Some( normal * 0.01 * dist * (power * force + sunction) )
}

fn attract(state: &mut State) {
    for (_, (color, speed, neighbours)) in state.world.query::<(&Color, &mut Speed, &Neighbours)>().iter() {
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
        Player::update(self);
        position_speed(self);
        decelerate(self);
        Neighbours::update(self, 10.0);
        attract(self);
        //std::thread::sleep(Duration::from_micros(1000/60));
    }

    pub fn player_input(&self, player: Entity, dir: DVec2) -> Option<()>{
        let mut query = self.world.query_one::<&mut Speed>(player).ok()?.with::<Player>();
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
    
    fn spawn_player_color(&mut self, player: Entity, angle: f64) -> Entity {
        let color = Color{hsl: Hsl::new(angle, 100.0, 50.0, None)};
        let pos = Position::default();
        let is_player = IsPlayer(player);
        //let speed = Speed::default();
        //let neighbours = Neighbours::default();
        self.world.spawn((color, pos, is_player, /*speed, neighbours*/))
    }
    fn create_player_component(&mut self, player: Entity) -> Player {
        let mut dots = [Entity::from_bits(0); DOTS_NUMBER];
        for i in 0..DOTS_NUMBER {
            let angle = std::f64::consts::TAU * (1.0 / DOTS_NUMBER as f64 * i as f64 + 0.25);
            dots[i] = self.spawn_player_color(player, angle.to_degrees());
        }        
        Player{dots, size: 2.0, rot: 0.25}
    }

    pub fn spawn_player(&mut self) -> Entity {
        //let mut random = Random::default();

        let player = self.world.reserve_entity();
        /*let pos = Position {
            vec: random.dvec2(10.0),
        };*/
        let pos = Position::default();
        let speed = Speed::default();
        let player_component = self.create_player_component(player);
        let rgb = Rgb::new(255.0, 255.0, 255.0, None);
        let color = Color{hsl: rgb.into()};
        self.world.spawn_at(player, (player_component, pos, speed, color));
        player
    }

    pub fn dots(&self) -> BumpVec<'_, Dot> {
        let mut query = self.world.query::<(&Color, &Position)>();
        let iter = query.iter()
        .map(|(_, (color, &pos))| {
            Dot{color: color.clone(), pos}
        });
        BumpVec::from_iter_in(iter, &self.bump)        
    }
}

#[derive(Debug)]
pub struct Dot {
    pub color: Color,
    pub pos: Position,
}