use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering as CmpOrdering;
use crate::constants::WALL_DIAMETER;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Node { pub cost: i32, pub pos: (i32, i32) }
impl Ord for Node { fn cmp(&self, other: &Self) -> CmpOrdering { other.cost.cmp(&self.cost) } }
impl PartialOrd for Node { fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> { Some(self.cmp(other)) } }

pub struct Pathfinder { pub walls: HashSet<(i32, i32)>, pub width: i32, pub height: i32 }
impl Pathfinder {
    pub fn new(width: i32, height: i32, walls: HashSet<(i32, i32)>) -> Self { Self { width, height, walls } }
    pub fn find_path(&self, start_world: [f32; 2], end_world: [f32; 2]) -> Vec<[f32; 2]> {
        let sx = (start_world[0] / WALL_DIAMETER).floor() as i32; let sy = (start_world[1] / WALL_DIAMETER).floor() as i32;
        let ex = (end_world[0] / WALL_DIAMETER).floor() as i32; let ey = (end_world[1] / WALL_DIAMETER).floor() as i32;
        if sx == ex && sy == ey { return vec![end_world]; };
        let start = (sx, sy); let goal = (ex, ey);
        let mut dist: HashMap<(i32, i32), i32> = HashMap::new(); let mut heap = BinaryHeap::new(); let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        dist.insert(start, 0); let h_start = (start.0 - goal.0).abs() + (start.1 - goal.1).abs(); heap.push(Node { cost: h_start, pos: start });
        while let Some(Node { cost: _, pos }) = heap.pop() {
            if pos == goal {
                let mut path = Vec::new(); let mut curr = goal;
                let end_center_x = (curr.0 as f32 * WALL_DIAMETER) + (WALL_DIAMETER / 2.0); let end_center_y = (curr.1 as f32 * WALL_DIAMETER) + (WALL_DIAMETER / 2.0);
                path.push([end_center_x, end_center_y]);
                while let Some(prev) = came_from.get(&curr) { if *prev == start { break; }; curr = *prev; let wx = (curr.0 as f32 * WALL_DIAMETER) + (WALL_DIAMETER / 2.0); let wy = (curr.1 as f32 * WALL_DIAMETER) + (WALL_DIAMETER / 2.0); path.push([wx, wy]); }
                return path;
            }
            let neighbors = [ (pos.0 + 1, pos.1), (pos.0 - 1, pos.1), (pos.0, pos.1 + 1), (pos.0, pos.1 - 1) ];
            for next in neighbors {
                if next.0 < 0 || next.0 >= self.width || next.1 < 0 || next.1 >= self.height { continue; };
                if self.walls.contains(&next) { continue; };
                let g_score = dist[&pos] + 1;
                if g_score < *dist.get(&next).unwrap_or(&i32::MAX) { dist.insert(next, g_score); let h = (next.0 - goal.0).abs() + (next.1 - goal.1).abs(); heap.push(Node { cost: g_score + h, pos: next }); came_from.insert(next, pos); }
            }
        }
        vec![end_world] 
    }
}