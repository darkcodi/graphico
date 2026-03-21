use bevy::prelude::*;
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

        let repulsion_k = params.repulsion_strength * (1.0 + node_count as f32).ln();

        for iteration in 0..total {
            let temperature = 1.0 - (iteration as f32 / total as f32);

            let bodies: Vec<(Vec2, f32)> = graph
                .nodes
                .values()
                .map(|n| (n.position, 1.0))
                .collect();
            let tree = QuadTree::build(&bodies);

            // Clear forces
            for f in self.forces.values_mut() {
                *f = Vec2::ZERO;
            }
            for &id in &self.known_nodes {
                self.forces.entry(id).or_insert(Vec2::ZERO);
            }

            let centroid: Vec2 = graph.nodes.values().map(|n| n.position).sum::<Vec2>()
                / graph.nodes.len().max(1) as f32;

            // Compute forces for every node
            for &id in &all_ids {
                let Some(node) = graph.nodes.get(&id) else {
                    continue;
                };
                let pos = node.position;
                let mass =
                    1.0 + graph.adjacency.get(&id).map_or(0, |a| a.len()) as f32;

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
                    force += to_center.normalize()
                        * params.gravity_strength
                        * mass
                        * center_dist.ln().max(0.0);
                }

                if let Some(f) = self.forces.get_mut(&id) {
                    *f = force;
                }
            }

            // Apply displacement scaled by temperature
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
    }
}
