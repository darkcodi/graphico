use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

const API_BASE: &str = "http://127.0.0.1:3000";
const NODES_PER_TICK: usize = 10;
const MAX_NODES: usize = 1000;
const TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Vertical gap between depth levels (world units).
const V_SPACING: f32 = 120.0;
/// Preferred horizontal gap when the level is not crowded.
const H_SPACING_MAX: f32 = 72.0;
/// Cap total width per level so a star-shaped tree does not span absurdly far.
const LEVEL_WIDTH_BUDGET: f32 = 2800.0;

pub struct DemoPlugin;

impl Plugin for DemoPlugin {
    fn build(&self, _app: &mut App) {
        std::thread::spawn(demo_loop);
    }
}

/// Uniform random recursive tree: `parent[i]` in `0..i` for `i >= 1`.
fn random_tree_parents(rng: &mut impl Rng, n: usize) -> Vec<usize> {
    let mut parent = vec![0usize; n];
    for i in 1..n {
        parent[i] = rng.random_range(0..i);
    }
    parent
}

fn depths_from_parents(parent: &[usize]) -> Vec<usize> {
    let n = parent.len();
    let mut depth = vec![0usize; n];
    for i in 1..n {
        depth[i] = depth[parent[i]] + 1;
    }
    depth
}

/// Layered layout: nodes at depth `d` share `y`, spread `x` by index within the level.
fn layered_positions(depth: &[usize]) -> Vec<Vec2> {
    let n = depth.len();
    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let mut levels: Vec<Vec<usize>> = vec![Vec::new(); max_depth + 1];
    for i in 0..n {
        levels[depth[i]].push(i);
    }

    let mut positions = vec![Vec2::ZERO; n];
    for (d, level_nodes) in levels.iter().enumerate() {
        let count = level_nodes.len();
        let h = (LEVEL_WIDTH_BUDGET / count.max(1) as f32).min(H_SPACING_MAX);
        for (j, &node_idx) in level_nodes.iter().enumerate() {
            let x = (j as f32 - (count as f32 - 1.0) / 2.0) * h;
            let y = -(d as f32) * V_SPACING;
            positions[node_idx] = Vec2::new(x, y);
        }
    }
    positions
}

fn demo_loop() {
    std::thread::sleep(Duration::from_secs(1));

    let agent = ureq::Agent::new_with_defaults();
    let mut created_ids: Vec<String> = Vec::new();
    let mut rng = rand::rng();

    let parent = random_tree_parents(&mut rng, MAX_NODES);
    let depth = depths_from_parents(&parent);
    let positions = layered_positions(&depth);

    let mut i = 0usize;
    while i < MAX_NODES {
        for _ in 0..NODES_PER_TICK {
            if i >= MAX_NODES {
                break;
            }

            let hue: f32 = rng.random_range(0.0..360.0);
            let color = hsl_to_hex(hue, 0.7, 0.6);

            let edges: Vec<&str> = if i == 0 {
                vec![]
            } else {
                vec![created_ids[parent[i]].as_str()]
            };

            let data = format!("N{}\nDepth: {}", i + 1, depth[i]);
            let pos = positions[i];

            let body = serde_json::json!({
                "data": data,
                "color": color,
                "edges": edges,
                "position": { "x": pos.x, "y": pos.y },
            });

            match agent.post(format!("{API_BASE}/node")).send_json(&body) {
                Ok(mut resp) => {
                    if let Ok(parsed) = resp.body_mut().read_json::<serde_json::Value>()
                        && let Some(id) = parsed.get("id").and_then(|v| v.as_str())
                    {
                        created_ids.push(id.to_string());
                    }
                }
                Err(e) => {
                    warn!("demo: POST /node failed: {e}");
                    std::thread::sleep(Duration::from_secs(2));
                }
            }

            i += 1;
        }

        std::thread::sleep(TICK_INTERVAL);
    }

    info!("demo: finished generating {MAX_NODES} nodes");
}

fn hsl_to_hex(h: f32, s: f32, l: f32) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h2 = h / 60.0;
    let x = c * (1.0 - (h2 % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h2 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let r = ((r1 + m) * 255.0) as u8;
    let g = ((g1 + m) * 255.0) as u8;
    let b = ((b1 + m) * 255.0) as u8;
    format!("#{r:02X}{g:02X}{b:02X}")
}
