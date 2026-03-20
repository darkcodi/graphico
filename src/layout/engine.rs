use bevy::prelude::*;
use rand::Rng;
use std::collections::{HashMap, HashSet};

use crate::graph::model::{GraphData, NodeId};

use super::params::LayoutParams;
use super::quadtree::QuadTree;

/// Persistent layout state that survives across frames.
///
/// When a topology change is detected, `solve()` runs the full iterative
/// force-directed layout synchronously within a single frame. Between
/// topology changes the engine is completely idle.
#[derive(Resource)]
pub struct LayoutEngine {
    /// Set of node IDs the engine knows about.
    pub known_nodes: HashSet<NodeId>,
    /// Per-node force accumulator, used internally by `solve()`.
    forces: HashMap<NodeId, Vec2>,
    /// Snapshot of the graph's edge count, used to detect edge-only topology changes.
    pub last_edge_count: usize,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self {
            known_nodes: HashSet::new(),
            forces: HashMap::new(),
            last_edge_count: 0,
        }
    }
}

impl LayoutEngine {
    pub fn register_node(&mut self, id: NodeId) {
        self.known_nodes.insert(id);
    }

    pub fn remove_node(&mut self, id: &NodeId) {
        self.known_nodes.remove(id);
        self.forces.remove(id);
    }

    pub fn knows_node(&self, id: &NodeId) -> bool {
        self.known_nodes.contains(id)
    }

    /// Run the full force-directed layout to convergence in one shot.
    /// All iterations execute synchronously within this call.
    pub fn solve(&mut self, graph: &mut GraphData, params: &LayoutParams) {
        let total = params.settling_iterations.max(1);
        let all_ids: Vec<NodeId> = self.known_nodes.iter().copied().collect();
        let node_count = all_ids.len();
        if node_count == 0 {
            return;
        }

        let repulsion_k =
            params.repulsion_strength * params.ideal_edge_length * params.ideal_edge_length;
        let min_spacing = params.min_node_spacing;
        let mut rng = rand::rng();

        for iteration in 0..total {
            let temperature = 1.0 - (iteration as f32 / total as f32);

            let bodies: Vec<(Vec2, f32)> = all_ids
                .iter()
                .filter_map(|id| graph.nodes.get(id))
                .map(|n| (n.position, 1.0))
                .collect();
            let tree = QuadTree::build(&bodies);

            for f in self.forces.values_mut() {
                *f = Vec2::ZERO;
            }
            for &id in &self.known_nodes {
                self.forces.entry(id).or_insert(Vec2::ZERO);
            }

            let centroid: Vec2 = graph.nodes.values().map(|n| n.position).sum::<Vec2>()
                / graph.nodes.len().max(1) as f32;

            for &id in &all_ids {
                let Some(node) = graph.nodes.get(&id) else {
                    continue;
                };
                let pos = node.position;
                let degree = graph.adjacency.get(&id).map_or(0, |a| a.len());
                let mass = 1.0 + degree as f32;

                let mut force = Vec2::ZERO;

                force += tree.compute_repulsion(pos, mass, params.theta, repulsion_k);

                if let Some(edge_ids) = graph.adjacency.get(&id) {
                    for eid in edge_ids {
                        let Some(edge) = graph.edges.get(eid) else {
                            continue;
                        };
                        let neighbor_id = if edge.source == id {
                            edge.target
                        } else {
                            edge.source
                        };
                        let Some(neighbor) = graph.nodes.get(&neighbor_id) else {
                            continue;
                        };
                        let diff = neighbor.position - pos;
                        let dist = diff.length();
                        if dist > 0.001 {
                            force += diff.normalize()
                                * params.attraction_strength
                                * (dist - params.ideal_edge_length);
                        }
                    }
                }

                let to_center = centroid - pos;
                let center_dist = to_center.length();
                if center_dist > 0.001 {
                    force += to_center.normalize() * params.gravity_strength * mass;
                }

                if let Some(f) = self.forces.get_mut(&id) {
                    *f = force;
                }
            }

            // Pairwise overlap separation for nodes closer than min_node_spacing
            for i in 0..all_ids.len() {
                for j in (i + 1)..all_ids.len() {
                    let id_a = all_ids[i];
                    let id_b = all_ids[j];
                    let (pos_a, pos_b) = match (graph.nodes.get(&id_a), graph.nodes.get(&id_b)) {
                        (Some(a), Some(b)) => (a.position, b.position),
                        _ => continue,
                    };
                    let diff = pos_a - pos_b;
                    let dist = diff.length();
                    if dist < min_spacing {
                        let direction = if dist > 0.001 {
                            diff.normalize()
                        } else {
                            let angle = rng.random_range(0.0..std::f32::consts::TAU);
                            Vec2::new(angle.cos(), angle.sin())
                        };
                        let push = direction * (min_spacing - dist) * 0.5;
                        if let Some(f) = self.forces.get_mut(&id_a) {
                            *f += push;
                        }
                        if let Some(f) = self.forces.get_mut(&id_b) {
                            *f -= push;
                        }
                    }
                }
            }

            let force_snapshot: Vec<(NodeId, Vec2)> = self
                .forces
                .iter()
                .filter(|(_, f)| f.length_squared() > 0.0)
                .map(|(&id, &f)| (id, f))
                .collect();

            for (id, force) in force_snapshot {
                let mut displacement = force * temperature;
                let len = displacement.length();
                if len > params.max_displacement {
                    displacement *= params.max_displacement / len;
                }
                if let Some(node) = graph.nodes.get_mut(&id) {
                    node.position += displacement;
                }
            }
        }

        fit_graph_to_screen(graph, params);
    }
}

/// Center the graph at the origin and uniformly scale so its bounding box (including radii)
/// fits inside a square of side `2 * fit_half_extent` minus padding.
fn fit_graph_to_screen(graph: &mut GraphData, params: &LayoutParams) {
    if !params.fit_to_screen || graph.nodes.is_empty() {
        return;
    }

    let half = params.fit_half_extent.max(1.0);
    let pad = params.fit_padding.max(0.0);

    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for n in graph.nodes.values() {
        let r = n.radius;
        min = min.min(n.position - Vec2::splat(r));
        max = max.max(n.position + Vec2::splat(r));
    }

    let center = (min + max) * 0.5;
    let extent = max - min;
    let max_dim = extent.x.max(extent.y).max(1.0e-4);

    let inner = (half * 2.0 - pad * 2.0).max(1.0);
    let scale = inner / max_dim;

    for n in graph.nodes.values_mut() {
        n.position = (n.position - center) * scale;
    }
}
