use bevy::prelude::*;

#[derive(Resource)]
pub struct LayoutParams {
    /// Repulsive force coefficient (ForceAtlas2 k_r).
    pub repulsion_strength: f32,
    /// Attractive force coefficient along edges.
    pub attraction_strength: f32,
    /// Gravity pulling nodes toward the graph centroid.
    pub gravity_strength: f32,
    /// Rest length for edge springs.
    pub ideal_edge_length: f32,
    /// Maximum displacement per node per step.
    pub max_displacement: f32,
    /// Barnes-Hut opening angle threshold. Higher = faster but less accurate.
    pub theta: f32,
    /// Number of force iterations to run when solving the layout.
    pub settling_iterations: u32,
    /// Random offset radius when placing new nodes near neighbors.
    pub jitter_radius: f32,
    /// Master switch to pause/resume layout.
    pub enabled: bool,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            repulsion_strength: 1.0,
            attraction_strength: 1.0,
            gravity_strength: 1.0,
            ideal_edge_length: 100.0,
            max_displacement: 50.0,
            theta: 1.2,
            settling_iterations: 300,
            jitter_radius: 10.0,
            enabled: true,
        }
    }
}
