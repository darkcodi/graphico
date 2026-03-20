use bevy::prelude::*;
use std::collections::HashMap;

use crate::graph::model::NodeId;

/// Persistent layout state that survives across frames.
#[derive(Resource)]
pub struct LayoutEngine {
    /// Per-node velocity vectors, accumulated between frames.
    pub velocities: HashMap<NodeId, Vec2>,
    /// Per-node previous force, used for swing/traction calculation.
    pub prev_forces: HashMap<NodeId, Vec2>,
    /// Per-node force accumulator, cleared at the start of each step.
    pub forces: HashMap<NodeId, Vec2>,
    /// ForceAtlas2 global speed factor (adapts automatically).
    pub global_speed: f32,
    /// Sum of per-node swing across last step.
    pub global_swing: f32,
    /// Sum of per-node traction across last step.
    pub global_traction: f32,
    /// True when average displacement fell below the convergence threshold.
    pub is_converged: bool,
    /// Round-robin cursor: index into the node list for work-budget cycling.
    pub round_robin_cursor: usize,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self {
            velocities: HashMap::new(),
            prev_forces: HashMap::new(),
            forces: HashMap::new(),
            global_speed: 1.0,
            global_swing: 0.0,
            global_traction: 0.0,
            is_converged: false,
            round_robin_cursor: 0,
        }
    }
}

impl LayoutEngine {
    /// Register a node with zero velocity.
    pub fn register_node(&mut self, id: NodeId) {
        self.velocities.entry(id).or_insert(Vec2::ZERO);
        self.prev_forces.entry(id).or_insert(Vec2::ZERO);
        self.is_converged = false;
    }

    /// Remove all state for a deleted node.
    pub fn remove_node(&mut self, id: &NodeId) {
        self.velocities.remove(id);
        self.prev_forces.remove(id);
        self.forces.remove(id);
    }

    /// Clear force accumulators for the upcoming computation step.
    pub fn clear_forces(&mut self) {
        for f in self.forces.values_mut() {
            *f = Vec2::ZERO;
        }
    }

    /// Ensure the forces map has an entry for every known node.
    pub fn ensure_force_entries(&mut self) {
        for &id in self.velocities.keys() {
            self.forces.entry(id).or_insert(Vec2::ZERO);
        }
    }

    /// Returns true if `id` is already tracked by the engine.
    pub fn knows_node(&self, id: &NodeId) -> bool {
        self.velocities.contains_key(id)
    }

    /// Adapt the global speed using ForceAtlas2's swing/traction heuristic.
    pub fn adapt_speed(&mut self) {
        if self.global_traction > 0.0 {
            let tolerance = 1.0;
            let target = tolerance * self.global_traction / self.global_swing.max(0.001);
            self.global_speed += (target - self.global_speed) * 0.1;
            self.global_speed = self.global_speed.clamp(0.01, 10.0);
        }
    }
}
