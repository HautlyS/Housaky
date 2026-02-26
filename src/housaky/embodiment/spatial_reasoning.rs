use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point2D) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Point3D) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
            .sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: Point3D,
    pub max: Point3D,
}

impl BoundingBox {
    pub fn new(min: Point3D, max: Point3D) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, p: &Point3D) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    pub fn center(&self) -> Point3D {
        Point3D::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obstacle {
    pub id: String,
    pub bounds: BoundingBox,
    pub is_dynamic: bool,
    pub velocity: Option<Point3D>,
    pub label: String,
    pub confidence: f64,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupancyGrid {
    pub width: usize,
    pub height: usize,
    pub resolution_m: f64,
    pub origin: Point2D,
    pub cells: Vec<f64>,
    pub last_updated: DateTime<Utc>,
}

impl OccupancyGrid {
    pub fn new(width: usize, height: usize, resolution_m: f64, origin: Point2D) -> Self {
        Self {
            width,
            height,
            resolution_m,
            origin,
            cells: vec![0.5; width * height],
            last_updated: Utc::now(),
        }
    }

    pub fn world_to_grid(&self, world: &Point2D) -> Option<(usize, usize)> {
        let gx = ((world.x - self.origin.x) / self.resolution_m) as isize;
        let gy = ((world.y - self.origin.y) / self.resolution_m) as isize;
        if gx >= 0 && (gx as usize) < self.width && gy >= 0 && (gy as usize) < self.height {
            Some((gx as usize, gy as usize))
        } else {
            None
        }
    }

    pub fn grid_to_world(&self, gx: usize, gy: usize) -> Point2D {
        Point2D::new(
            self.origin.x + gx as f64 * self.resolution_m,
            self.origin.y + gy as f64 * self.resolution_m,
        )
    }

    pub fn get_occupancy(&self, gx: usize, gy: usize) -> f64 {
        if gx < self.width && gy < self.height {
            self.cells[gy * self.width + gx]
        } else {
            1.0
        }
    }

    pub fn set_occupancy(&mut self, gx: usize, gy: usize, value: f64) {
        if gx < self.width && gy < self.height {
            let idx = gy * self.width + gx;
            self.cells[idx] = value.clamp(0.0, 1.0);
            self.last_updated = Utc::now();
        }
    }

    pub fn update_from_lidar(&mut self, sensor_pos: &Point2D, hit_pos: &Point2D) {
        // Mark hit cell as occupied
        if let Some((hx, hy)) = self.world_to_grid(hit_pos) {
            self.set_occupancy(hx, hy, 0.9);
        }

        // Bresenham ray tracing to mark free cells
        if let (Some((sx, sy)), Some((ex, ey))) = (
            self.world_to_grid(sensor_pos),
            self.world_to_grid(hit_pos),
        ) {
            for (gx, gy) in bresenham_line(sx as isize, sy as isize, ex as isize, ey as isize)
                .iter()
                .take_while(|&&(x, y)| (x, y) != (ex as isize, ey as isize))
            {
                if *gx >= 0 && *gy >= 0 {
                    let v = self.get_occupancy(*gx as usize, *gy as usize);
                    self.set_occupancy(*gx as usize, *gy as usize, (v * 0.9).max(0.05));
                }
            }
        }
    }

    pub fn is_occupied(&self, gx: usize, gy: usize) -> bool {
        self.get_occupancy(gx, gy) > 0.65
    }
}

fn bresenham_line(x0: isize, y0: isize, x1: isize, y1: isize) -> Vec<(isize, isize)> {
    let mut points = Vec::new();
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx: isize = if x0 < x1 { 1 } else { -1 };
    let sy: isize = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    loop {
        points.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    points
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    pub waypoints: Vec<Point2D>,
    pub total_distance_m: f64,
    pub estimated_time_s: f64,
    pub planning_time_ms: u64,
    pub algorithm: String,
}

impl Path {
    pub fn empty() -> Self {
        Self {
            waypoints: Vec::new(),
            total_distance_m: 0.0,
            estimated_time_s: 0.0,
            planning_time_ms: 0,
            algorithm: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct AStarNode {
    f_cost: ordered_float::OrderedFloat,
    g_cost: ordered_float::OrderedFloat,
    x: usize,
    y: usize,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.f_cost.cmp(&self.f_cost)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct SpatialReasoner {
    pub occupancy_grid: Arc<RwLock<OccupancyGrid>>,
    pub obstacles: Arc<RwLock<HashMap<String, Obstacle>>>,
    pub robot_radius_m: f64,
    pub max_velocity_m_s: f64,
}

impl SpatialReasoner {
    pub fn new(
        grid_width: usize,
        grid_height: usize,
        resolution_m: f64,
        robot_radius_m: f64,
        max_velocity_m_s: f64,
    ) -> Self {
        Self {
            occupancy_grid: Arc::new(RwLock::new(OccupancyGrid::new(
                grid_width,
                grid_height,
                resolution_m,
                Point2D::new(0.0, 0.0),
            ))),
            obstacles: Arc::new(RwLock::new(HashMap::new())),
            robot_radius_m,
            max_velocity_m_s,
        }
    }

    pub async fn update_obstacle(&self, obstacle: Obstacle) {
        self.obstacles
            .write()
            .await
            .insert(obstacle.id.clone(), obstacle);
    }

    pub async fn remove_obstacle(&self, id: &str) {
        self.obstacles.write().await.remove(id);
    }

    /// A* pathfinding on the occupancy grid.
    pub async fn find_path(&self, start: &Point2D, goal: &Point2D) -> Result<Path> {
        let t0 = std::time::Instant::now();
        let grid = self.occupancy_grid.read().await;

        let start_cell = grid.world_to_grid(start).ok_or_else(|| {
            anyhow::anyhow!("Start position ({:.2}, {:.2}) is outside map", start.x, start.y)
        })?;
        let goal_cell = grid.world_to_grid(goal).ok_or_else(|| {
            anyhow::anyhow!("Goal position ({:.2}, {:.2}) is outside map", goal.x, goal.y)
        })?;

        if grid.is_occupied(start_cell.0, start_cell.1) {
            return Err(anyhow::anyhow!("Start position is occupied"));
        }
        if grid.is_occupied(goal_cell.0, goal_cell.1) {
            return Err(anyhow::anyhow!("Goal position is occupied"));
        }

        let mut open_set: BinaryHeap<AStarNode> = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut g_scores: HashMap<(usize, usize), f64> = HashMap::new();
        let mut closed: HashSet<(usize, usize)> = HashSet::new();

        g_scores.insert(start_cell, 0.0);
        let h0 = heuristic(start_cell, goal_cell);
        open_set.push(AStarNode {
            f_cost: ordered_float::OrderedFloat(h0),
            g_cost: ordered_float::OrderedFloat(0.0),
            x: start_cell.0,
            y: start_cell.1,
        });

        let dirs: &[(isize, isize)] = &[
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        while let Some(current) = open_set.pop() {
            let cx = current.x;
            let cy = current.y;

            if (cx, cy) == goal_cell {
                let waypoints = reconstruct_path(&came_from, goal_cell, start_cell)
                    .iter()
                    .map(|&(gx, gy)| grid.grid_to_world(gx, gy))
                    .collect::<Vec<_>>();

                let total_dist: f64 = waypoints
                    .windows(2)
                    .map(|w| w[0].distance_to(&w[1]))
                    .sum();

                return Ok(Path {
                    waypoints,
                    total_distance_m: total_dist,
                    estimated_time_s: total_dist / self.max_velocity_m_s,
                    planning_time_ms: t0.elapsed().as_millis() as u64,
                    algorithm: "A*".to_string(),
                });
            }

            if closed.contains(&(cx, cy)) {
                continue;
            }
            closed.insert((cx, cy));

            let g = *g_scores.get(&(cx, cy)).unwrap_or(&f64::MAX);

            for &(dx, dy) in dirs {
                let nx = cx as isize + dx;
                let ny = cy as isize + dy;
                if nx < 0
                    || ny < 0
                    || nx as usize >= grid.width
                    || ny as usize >= grid.height
                {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);

                if grid.is_occupied(nx, ny) || closed.contains(&(nx, ny)) {
                    continue;
                }

                let step = if dx != 0 && dy != 0 {
                    std::f64::consts::SQRT_2
                } else {
                    1.0
                };
                let ng = g + step * grid.resolution_m;
                let best = *g_scores.get(&(nx, ny)).unwrap_or(&f64::MAX);

                if ng < best {
                    g_scores.insert((nx, ny), ng);
                    came_from.insert((nx, ny), (cx, cy));
                    let h = heuristic((nx, ny), goal_cell);
                    open_set.push(AStarNode {
                        f_cost: ordered_float::OrderedFloat(ng + h),
                        g_cost: ordered_float::OrderedFloat(ng),
                        x: nx,
                        y: ny,
                    });
                }
            }
        }

        Err(anyhow::anyhow!("No path found from start to goal"))
    }

    pub async fn check_collision(&self, position: &Point2D) -> bool {
        let grid = self.occupancy_grid.read().await;
        let clearance_cells =
            (self.robot_radius_m / grid.resolution_m).ceil() as usize;

        if let Some((cx, cy)) = grid.world_to_grid(position) {
            let lo_x = cx.saturating_sub(clearance_cells);
            let hi_x = (cx + clearance_cells).min(grid.width - 1);
            let lo_y = cy.saturating_sub(clearance_cells);
            let hi_y = (cy + clearance_cells).min(grid.height - 1);

            for gx in lo_x..=hi_x {
                for gy in lo_y..=hi_y {
                    if grid.is_occupied(gx, gy) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub async fn get_nearest_obstacle(&self, position: &Point2D) -> Option<(String, f64)> {
        let obstacles = self.obstacles.read().await;
        obstacles
            .values()
            .map(|obs| {
                let center = obs.bounds.center();
                let center2d = Point2D::new(center.x, center.y);
                let dist = position.distance_to(&center2d);
                (obs.id.clone(), dist)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }

    pub async fn smooth_path(&self, path: &mut Path) {
        if path.waypoints.len() < 3 {
            return;
        }

        let grid = self.occupancy_grid.read().await;
        let mut smoothed = vec![path.waypoints[0].clone()];
        let mut i = 0;

        while i < path.waypoints.len() - 1 {
            let mut j = path.waypoints.len() - 1;
            while j > i + 1 {
                if !line_of_sight(&path.waypoints[i], &path.waypoints[j], &grid) {
                    j -= 1;
                    continue;
                }
                break;
            }
            smoothed.push(path.waypoints[j].clone());
            i = j;
        }

        let total_dist: f64 = smoothed.windows(2).map(|w| w[0].distance_to(&w[1])).sum();
        path.waypoints = smoothed;
        path.total_distance_m = total_dist;
        path.estimated_time_s = total_dist / self.max_velocity_m_s;

        debug!(
            "Path smoothed to {} waypoints, dist={:.2}m",
            path.waypoints.len(),
            total_dist
        );
    }

    pub async fn update_map_from_lidar(
        &self,
        sensor_pos: &Point2D,
        hit_positions: &[Point2D],
    ) {
        let mut grid = self.occupancy_grid.write().await;
        for hit in hit_positions {
            grid.update_from_lidar(sensor_pos, hit);
        }
        info!("Updated occupancy map with {} lidar hits", hit_positions.len());
    }
}

fn heuristic(a: (usize, usize), b: (usize, usize)) -> f64 {
    let dx = (a.0 as f64 - b.0 as f64).abs();
    let dy = (a.1 as f64 - b.1 as f64).abs();
    dx.max(dy) + (std::f64::consts::SQRT_2 - 1.0) * dx.min(dy)
}

fn reconstruct_path(
    came_from: &HashMap<(usize, usize), (usize, usize)>,
    mut current: (usize, usize),
    start: (usize, usize),
) -> Vec<(usize, usize)> {
    let mut path = vec![current];
    while let Some(&prev) = came_from.get(&current) {
        current = prev;
        path.push(current);
        if current == start {
            break;
        }
    }
    path.reverse();
    path
}

fn line_of_sight(a: &Point2D, b: &Point2D, grid: &OccupancyGrid) -> bool {
    if let (Some((ax, ay)), Some((bx, by))) = (grid.world_to_grid(a), grid.world_to_grid(b)) {
        for (x, y) in bresenham_line(ax as isize, ay as isize, bx as isize, by as isize) {
            if x >= 0
                && y >= 0
                && grid.is_occupied(x as usize, y as usize)
            {
                return false;
            }
        }
    }
    true
}

mod ordered_float {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct OrderedFloat(pub f64);

    impl Eq for OrderedFloat {}

    impl PartialOrd for OrderedFloat {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for OrderedFloat {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.0
                .partial_cmp(&other.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    }
}

impl Default for SpatialReasoner {
    fn default() -> Self {
        Self::new(200, 200, 0.05, 0.2, 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occupancy_grid_world_to_grid() {
        let grid = OccupancyGrid::new(100, 100, 0.1, Point2D::new(0.0, 0.0));
        let cell = grid.world_to_grid(&Point2D::new(0.5, 0.5));
        assert_eq!(cell, Some((5, 5)));
    }

    #[test]
    fn test_bounding_box_contains() {
        let bb = BoundingBox::new(Point3D::new(0.0, 0.0, 0.0), Point3D::new(1.0, 1.0, 1.0));
        assert!(bb.contains(&Point3D::new(0.5, 0.5, 0.5)));
        assert!(!bb.contains(&Point3D::new(2.0, 0.5, 0.5)));
    }

    #[tokio::test]
    async fn test_pathfinding_simple() {
        let reasoner = SpatialReasoner::new(50, 50, 0.1, 0.1, 0.5);
        let start = Point2D::new(0.1, 0.1);
        let goal = Point2D::new(1.0, 1.0);
        let path = reasoner.find_path(&start, &goal).await.unwrap();
        assert!(!path.waypoints.is_empty());
        assert!(path.total_distance_m > 0.0);
    }

    #[tokio::test]
    async fn test_collision_check_free() {
        let reasoner = SpatialReasoner::new(50, 50, 0.1, 0.1, 0.5);
        let pos = Point2D::new(1.0, 1.0);
        assert!(!reasoner.check_collision(&pos).await);
    }
}
