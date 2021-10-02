use super::{Position, State};
use bumpalo::{collections::Vec as BumpVec, Bump};
use float_ord::FloatOrd;
use glam::DVec2;
use hecs::Entity;

#[derive(Debug, Clone, Copy)]
struct Element {
    entity: Entity,
    pos: Position,
}
struct Field<'a> {
    mean: Option<DVec2>,
    elements: BumpVec<'a, Element>,
}

impl<'a> Field<'a> {
    fn new(bump: &'a Bump) -> Self {
        Self {
            mean: None,
            elements: BumpVec::new_in(bump),
        }
    }
    fn with_capacity_in(capacity: usize, bump: &'a Bump) -> Self {
        Self {
            mean: None,
            elements: BumpVec::with_capacity_in(capacity, bump),
        }
    }
    fn extend(&mut self, iter: impl Iterator<Item = Element>) {
        self.elements.extend(iter);
        self.mean = None;
    }
    fn get_mean_or_compute(&mut self) -> DVec2 {
        let Self {
            ref mut mean,
            ref elements,
        } = self;
        *mean.get_or_insert_with(|| Self::compute_mean(elements))
    }
    fn compute_mean(vec: &BumpVec<'a, Element>) -> DVec2 {
        let sum: DVec2 = vec.iter().map(|e| &e.pos.vec).sum();
        sum / vec.len() as f64
    }
    fn split(&mut self, bump: &'a Bump, by_x: bool, mean: f64) -> Field<'a> {
        let mut new = Field::with_capacity_in(self.elements.len(), bump);
        self.elements.retain(|e| {
            let split = if by_x {
                e.pos.vec.x > mean
            } else {
                e.pos.vec.y > mean
            };
            if split {
                new.elements.push(*e);
            }
            !split
        });
        self.mean = None;
        if self.elements.is_empty() {
            std::mem::swap(self, &mut new);
        }
        new
    }

    fn rect(self) -> Rect<'a> {
        let min_x = self
            .elements
            .iter()
            .map(|e| FloatOrd(e.pos.vec.x))
            .min()
            .unwrap();
        let max_x = self
            .elements
            .iter()
            .map(|e| FloatOrd(e.pos.vec.x))
            .max()
            .unwrap();
        let min_y = self
            .elements
            .iter()
            .map(|e| FloatOrd(e.pos.vec.y))
            .min()
            .unwrap();
        let max_y = self
            .elements
            .iter()
            .map(|e| FloatOrd(e.pos.vec.y))
            .max()
            .unwrap();
        Rect {
            min: DVec2::new(min_x.0, min_y.0),
            max: DVec2::new(max_x.0, max_y.0),
            elements: self.elements,
        }
    }
}

struct Partition<'a> {
    fields: BumpVec<'a, Field<'a>>,
    bump: &'a Bump,
}
impl<'a> Partition<'a> {
    fn from_iter_in(iter: impl Iterator<Item = Element>, bump: &'a Bump) -> Self {
        let mut fields = BumpVec::new_in(&bump);
        let mut field = Field::new(bump);
        field.extend(iter);
        fields.push(field);
        Self { fields, bump }
    }
    fn partition_all_once<'b: 'a>(
        &mut self,
        bump: &'b Bump,
        partitioned: &mut BumpVec<'a, Field<'a>>,
        max_elements: usize,
    ) -> bool {
        let done = self.fields.iter_mut().any(|field| {
            if field.elements.len() > max_elements {
                let mean = field.get_mean_or_compute();
                let mut ab = field.split(bump, false, mean.y);
                if !ab.elements.is_empty() {
                    let b = ab.split(bump, true, mean.x);
                    let a = ab;
                    if !a.elements.is_empty() {
                        partitioned.push(a);
                    }
                    if !b.elements.is_empty() {
                        partitioned.push(b);
                    }
                }
                let d = field.split(bump, true, mean.x);
                if !d.elements.is_empty() {
                    partitioned.push(d);
                }
                true
            } else {
                false
            }
        });
        self.fields.append(partitioned);
        done
    }
    fn partition(mut self, max_elements: usize) -> Space<'a> {
        let mut partitioned = BumpVec::new_in(&self.bump);
        while self.partition_all_once(&self.bump, &mut partitioned, max_elements) {}
        let mut rects = BumpVec::with_capacity_in(self.fields.len(), &self.bump);
        rects.extend(self.fields.drain(..).map(|field| field.rect()));
        Space { rects }
    }
}

struct Rect<'a> {
    min: DVec2,
    max: DVec2,
    elements: BumpVec<'a, Element>,
}
impl<'a> Rect<'a> {
    fn distance(&self, pos: Position) -> f64 {
        let x = pos.vec.x.clamp(self.min.x, self.max.x);
        let y = pos.vec.y.clamp(self.min.y, self.max.y);
        DVec2::new(x, y).distance(pos.vec)
    }
    fn neighbours(&'a self, pos: Position, dist: f64) -> impl 'a + Iterator<Item = Neighbour> {
        self.elements
            .iter()
            .map(move |el| {
                let diff = el.pos.vec - pos.vec;
                let dist = diff.length();
                Neighbour {
                    entity: el.entity,
                    diff,
                    dist,
                }
            })
            .filter(move |neighbour| neighbour.dist <= dist)
    }
}

struct Space<'a> {
    rects: BumpVec<'a, Rect<'a>>,
}
impl<'a> Space<'a> {
    fn neighbours(&'a self, pos: Position, dist: f64) -> impl 'a + Iterator<Item = Neighbour> {
        self.rects
            .iter()
            .filter(move |rect| rect.distance(pos) <= dist)
            .flat_map(move |rect| rect.neighbours(pos, dist))
    }
}

#[derive(Debug)]
pub struct Neighbour {
    pub entity: Entity,
    pub diff: DVec2,
    pub dist: f64,
}

#[derive(Debug, Default)]
pub struct Neighbours {
    pub vec: Vec<Neighbour>,
}

impl Neighbours {
    pub fn slice(&self) -> &[Neighbour] {
        &self.vec
    }
    pub fn update(state: &mut State, dist: f64) {
        {
            let partition = {
                let mut query = state.world.query::<&Position>();
                let iter = query.iter().map(|(entity, &pos)| Element { entity, pos });
                Partition::from_iter_in(iter, &state.bump)
            };
            let space = partition.partition(32);
            for (_, (pos, neighbours)) in state.world.query_mut::<(&Position, &mut Neighbours)>() {
                neighbours.vec.clear();
                neighbours.vec.extend(space.neighbours(*pos, dist));
            }
        }
        state.bump.reset();
    }
}
