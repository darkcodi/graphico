use bevy::prelude::*;

#[derive(Resource)]
pub struct LayoutParams {
    /// Repulsive force coefficient (Fruchterman-Reingold style).
    pub repulsion_strength: f32,
    /// Attractive force coefficient along edges.
    pub attraction_strength: f32,
    /// Gravity pulling all nodes toward the graph centroid each iteration.
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
    /// Minimum distance between node centers; overlap prevention kicks in below this.
    pub min_node_spacing: f32,
    /// After `solve()`, scale and center the graph so its axis-aligned bounds fit in
    /// `[-fit_half_extent, fit_half_extent]` (both axes), preventing unbounded spread.
    pub fit_to_screen: bool,
    /// Half-size of the target bounding square in world units (see `fit_to_screen`).
    pub fit_half_extent: f32,
    /// Inset from the fit box edge (world units) so nodes are not flush with the bounds.
    pub fit_padding: f32,
    /// Master switch to pause/resume layout.
    pub enabled: bool,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            repulsion_strength: 1.0,
            // Stronger springs + shorter rest length keep connected components tight.
            attraction_strength: 2.5,
            gravity_strength: 0.5,
            ideal_edge_length: 55.0,
            max_displacement: 50.0,
            theta: 1.2,
            settling_iterations: 300,
            jitter_radius: 10.0,
            min_node_spacing: 20.0,
            fit_to_screen: true,
            fit_half_extent: 420.0,
            fit_padding: 24.0,
            enabled: true,
        }
    }
}
