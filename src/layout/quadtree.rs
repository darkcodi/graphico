use bevy::prelude::*;

const EMPTY: u32 = 0;
const MAX_DEPTH: u32 = 40;

/// Arena-allocated Barnes-Hut quadtree for O(n log n) repulsion.
pub struct QuadTree {
    nodes: Vec<QTNode>,
}

struct QTNode {
    center_of_mass: Vec2,
    total_mass: f32,
    bounds_center: Vec2,
    bounds_half: f32,
    /// Indices into `nodes` for NE, NW, SW, SE. 0 = empty slot.
    children: [u32; 4],
    body_count: u32,
    /// Position of the single body when `body_count == 1`.
    body_pos: Vec2,
}

impl QTNode {
    fn new(center: Vec2, half: f32) -> Self {
        Self {
            center_of_mass: Vec2::ZERO,
            total_mass: 0.0,
            bounds_center: center,
            bounds_half: half,
            children: [EMPTY; 4],
            body_count: 0,
            body_pos: Vec2::ZERO,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children == [EMPTY; 4]
    }

    fn quadrant(&self, pos: Vec2) -> usize {
        let dx = if pos.x >= self.bounds_center.x { 0 } else { 1 };
        let dy = if pos.y >= self.bounds_center.y { 0 } else { 1 };
        // NE=0, NW=1, SW=2 (unused combo via +2 trick), SE=3
        // Mapping: (east,north)=0, (west,north)=1, (west,south)=3, (east,south)=2
        dx + dy * 2
    }
}

impl QuadTree {
    /// Build a quadtree from a set of (position, mass) pairs.
    pub fn build(bodies: &[(Vec2, f32)]) -> Self {
        if bodies.is_empty() {
            return Self {
                nodes: vec![QTNode::new(Vec2::ZERO, 1.0)],
            };
        }

        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for &(pos, _) in bodies {
            min = min.min(pos);
            max = max.max(pos);
        }

        let span = (max - min).max_element().max(1.0);
        let half = span * 0.55; // slight padding
        let center = (min + max) * 0.5;

        let capacity = (bodies.len() * 4 + 16).next_power_of_two();
        let mut tree = Self {
            nodes: Vec::with_capacity(capacity),
        };
        tree.nodes.push(QTNode::new(center, half));

        for &(pos, mass) in bodies {
            tree.insert(0, pos, mass, 0);
        }

        tree
    }

    fn insert(&mut self, node_idx: u32, pos: Vec2, mass: f32, depth: u32) {
        if depth > MAX_DEPTH {
            let n = &mut self.nodes[node_idx as usize];
            let tm = n.total_mass + mass;
            if tm > 0.0 {
                n.center_of_mass = (n.center_of_mass * n.total_mass + pos * mass) / tm;
            }
            n.total_mass = tm;
            n.body_count += 1;
            return;
        }

        let bc = self.nodes[node_idx as usize].body_count;

        if bc == 0 && self.nodes[node_idx as usize].is_leaf() {
            let n = &mut self.nodes[node_idx as usize];
            n.body_pos = pos;
            n.center_of_mass = pos;
            n.total_mass = mass;
            n.body_count = 1;
            return;
        }

        if bc == 1 && self.nodes[node_idx as usize].is_leaf() {
            let old_pos = self.nodes[node_idx as usize].body_pos;
            let old_mass = self.nodes[node_idx as usize].total_mass;
            self.push_down(node_idx, old_pos, old_mass, depth);
        }

        // Update aggregate
        {
            let n = &mut self.nodes[node_idx as usize];
            let tm = n.total_mass + mass;
            if tm > 0.0 {
                n.center_of_mass = (n.center_of_mass * n.total_mass + pos * mass) / tm;
            }
            n.total_mass = tm;
            n.body_count += 1;
        }

        let q = self.nodes[node_idx as usize].quadrant(pos);
        let child = self.nodes[node_idx as usize].children[q];
        if child == EMPTY {
            let child_idx = self.alloc_child(node_idx, q);
            let n = &mut self.nodes[child_idx as usize];
            n.body_pos = pos;
            n.center_of_mass = pos;
            n.total_mass = mass;
            n.body_count = 1;
        } else {
            self.insert(child, pos, mass, depth + 1);
        }
    }

    fn push_down(&mut self, node_idx: u32, pos: Vec2, mass: f32, depth: u32) {
        let q = self.nodes[node_idx as usize].quadrant(pos);
        let child = self.nodes[node_idx as usize].children[q];
        if child == EMPTY {
            let child_idx = self.alloc_child(node_idx, q);
            let n = &mut self.nodes[child_idx as usize];
            n.body_pos = pos;
            n.center_of_mass = pos;
            n.total_mass = mass;
            n.body_count = 1;
        } else {
            self.insert(child, pos, mass, depth + 1);
        }
    }

    fn alloc_child(&mut self, parent: u32, quadrant: usize) -> u32 {
        let p = &self.nodes[parent as usize];
        let qh = p.bounds_half * 0.5;
        let offsets = [
            Vec2::new(qh, qh),   // NE
            Vec2::new(-qh, qh),  // NW
            Vec2::new(-qh, -qh), // SW
            Vec2::new(qh, -qh),  // SE
        ];
        let child_center = p.bounds_center + offsets[quadrant];

        let idx = self.nodes.len() as u32;
        self.nodes.push(QTNode::new(child_center, qh));
        self.nodes[parent as usize].children[quadrant] = idx;
        idx
    }

    /// Compute net repulsive force on a body at `pos` with `mass`.
    /// Uses ForceAtlas2-style linear repulsion: `k_r * mass_a * mass_b / dist`.
    pub fn compute_repulsion(&self, pos: Vec2, mass: f32, theta: f32, k_r: f32) -> Vec2 {
        if self.nodes.is_empty() {
            return Vec2::ZERO;
        }
        self.walk_repulsion(0, pos, mass, theta, k_r)
    }

    fn walk_repulsion(
        &self,
        idx: u32,
        pos: Vec2,
        mass: f32,
        theta: f32,
        k_r: f32,
    ) -> Vec2 {
        let node = &self.nodes[idx as usize];
        if node.body_count == 0 {
            return Vec2::ZERO;
        }

        let diff = pos - node.center_of_mass;
        let dist = diff.length();

        let size = node.bounds_half * 2.0;

        if node.is_leaf() || (size / dist.max(0.001)) < theta {
            if dist < 0.001 {
                return Vec2::ZERO;
            }
            let strength = k_r * mass * node.total_mass / dist;
            return diff.normalize() * strength;
        }

        let mut force = Vec2::ZERO;
        for &child in &node.children {
            if child != EMPTY {
                force += self.walk_repulsion(child, pos, mass, theta, k_r);
            }
        }
        force
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree() {
        let tree = QuadTree::build(&[]);
        let f = tree.compute_repulsion(Vec2::ZERO, 1.0, 1.0, 1.0);
        assert_eq!(f, Vec2::ZERO);
    }

    #[test]
    fn two_bodies_repel() {
        let bodies = vec![(Vec2::new(0.0, 0.0), 1.0), (Vec2::new(10.0, 0.0), 1.0)];
        let tree = QuadTree::build(&bodies);

        let f = tree.compute_repulsion(Vec2::new(0.0, 0.0), 1.0, 0.0, 1.0);
        assert!(f.x < 0.0, "body at origin should be pushed left (away from body at x=10)");
    }

    #[test]
    fn single_body_no_self_force() {
        let bodies = vec![(Vec2::new(5.0, 5.0), 1.0)];
        let tree = QuadTree::build(&bodies);
        let f = tree.compute_repulsion(Vec2::new(5.0, 5.0), 1.0, 1.0, 1.0);
        assert_eq!(f, Vec2::ZERO);
    }
}
