use serde::{Deserialize, Serialize};

use crate::{WorldPoint, WorldPt};

pub const EPSILON: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub const fn from_world(point: WorldPoint) -> Self {
        Self {
            x: point.x.value(),
            y: point.y.value(),
        }
    }

    pub const fn world(self) -> WorldPoint {
        WorldPoint::new(WorldPt(self.x), WorldPt(self.y))
    }

    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    pub fn translated(self, vector: Vector) -> Self {
        Self {
            x: self.x + vector.x,
            y: self.y + vector.y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

impl Vector {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn normalized(self) -> Self {
        let length = self.length();
        if length <= EPSILON {
            return Self { x: 1.0, y: 0.0 };
        }
        Self {
            x: self.x / length,
            y: self.y / length,
        }
    }

    pub fn scaled(self, value: f64) -> Self {
        Self {
            x: self.x * value,
            y: self.y * value,
        }
    }
}

pub fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub fn normalize_angle(degrees: f64) -> f64 {
    let mut value = degrees % 360.0;
    if value < 0.0 {
        value += 360.0;
    }
    value
}

pub fn angle_between(from: Point, to: Point) -> f64 {
    normalize_angle((to.y - from.y).atan2(to.x - from.x).to_degrees())
}

pub fn direction_from_angle(degrees: f64) -> Vector {
    let radians = normalize_angle(degrees).to_radians();
    Vector {
        x: radians.cos(),
        y: radians.sin(),
    }
}

pub fn angular_distance(a: f64, b: f64) -> f64 {
    let diff = (normalize_angle(a) - normalize_angle(b)).abs();
    diff.min(360.0 - diff)
}

pub fn angle_in_clockwise_arc(angle: f64, start: f64, end: f64) -> bool {
    let span = normalize_angle(end - start);
    let offset = normalize_angle(angle - start);
    offset <= span + EPSILON
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AngularGap {
    pub start: f64,
    pub end: f64,
    pub size: f64,
    pub center: f64,
}

pub fn largest_angular_gap(directions: &[f64]) -> AngularGap {
    if directions.is_empty() {
        return AngularGap {
            start: 0.0,
            end: 360.0,
            size: 360.0,
            center: 0.0,
        };
    }

    let mut sorted: Vec<f64> = directions
        .iter()
        .map(|angle| (normalize_angle(*angle) * 1000.0).round() / 1000.0)
        .collect();
    sorted.sort_by(f64::total_cmp);
    sorted.dedup_by(|a, b| (*a - *b).abs() < 0.001);

    let mut best = AngularGap {
        start: sorted[0],
        end: sorted[0],
        size: 0.0,
        center: sorted[0],
    };

    for index in 0..sorted.len() {
        let start = sorted[index];
        let end = if index == sorted.len() - 1 {
            sorted[0] + 360.0
        } else {
            sorted[index + 1]
        };
        let size = end - start;
        if size > best.size {
            best = AngularGap {
                start,
                end,
                size,
                center: normalize_angle(start + size / 2.0),
            };
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_largest_gap_center() {
        let gap = largest_angular_gap(&[0.0, 120.0]);
        assert_eq!(gap.center, 240.0);
    }

    #[test]
    fn angular_distance_wraps() {
        assert_eq!(angular_distance(350.0, 10.0), 20.0);
    }
}
