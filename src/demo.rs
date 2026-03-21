use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

use crate::render::nodes::estimate_text_size;

const API_BASE: &str = "http://127.0.0.1:3000";
const NODES_PER_TICK: usize = 10;
const MAX_NODES: usize = 1000;
const TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Horizontal gap between adjacent node rectangles.
const H_GAP: f32 = 16.0;
/// Vertical gap between depth levels.
const V_GAP: f32 = 24.0;

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

/// Layered layout that sizes spacing from estimated node rectangles so they never overlap.
fn layered_positions(depth: &[usize], data_strings: &[String]) -> Vec<Vec2> {
    let n = depth.len();
    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let mut levels: Vec<Vec<usize>> = vec![Vec::new(); max_depth + 1];
    for i in 0..n {
        levels[depth[i]].push(i);
    }

    let sizes: Vec<Vec2> = data_strings.iter()
        .map(|d| estimate_text_size(d))
        .collect();

    let max_h = sizes.iter().map(|s| s.y).fold(0f32, f32::max);

    let mut positions = vec![Vec2::ZERO; n];
    for (d, level_nodes) in levels.iter().enumerate() {
        let y = -(d as f32) * (max_h + V_GAP);

        let total_w: f32 = level_nodes.iter().map(|&idx| sizes[idx].x).sum::<f32>()
            + (level_nodes.len().saturating_sub(1) as f32) * H_GAP;

        let mut x = -total_w / 2.0;
        for &node_idx in level_nodes {
            let w = sizes[node_idx].x;
            positions[node_idx] = Vec2::new(x + w / 2.0, y);
            x += w + H_GAP;
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

    let name_strings: Vec<String> = (0..MAX_NODES)
        .map(|i| format!("N{}", i + 1))
        .collect();
    let data_strings: Vec<String> = (0..MAX_NODES)
        .map(|i| format!("Depth: {}\nIndex: {}", depth[i], i))
        .collect();

    let positions = layered_positions(&depth, &name_strings);

    let mut i = 0usize;
    while i < MAX_NODES {
        for _ in 0..NODES_PER_TICK {
            if i >= MAX_NODES {
                break;
            }

            let edges: Vec<&str> = if i == 0 {
                vec![]
            } else {
                vec![created_ids[parent[i]].as_str()]
            };

            let name = &name_strings[i];
            let data = &data_strings[i];
            let pos = positions[i];

            let body = serde_json::json!({
                "name": name,
                "data": data,
                "edges": edges,
                "position": { "x": pos.x, "y": pos.y },
            });

            match agent.post(format!("{API_BASE}/nodes")).send_json(&body) {
                Ok(mut resp) => {
                    if let Ok(parsed) = resp.body_mut().read_json::<serde_json::Value>()
                        && let Some(id) = parsed.get("id").and_then(|v| v.as_str())
                    {
                        created_ids.push(id.to_string());
                    }
                }
                Err(e) => {
                    warn!("demo: POST /nodes failed: {e}");
                    std::thread::sleep(Duration::from_secs(2));
                }
            }

            i += 1;
        }

        std::thread::sleep(TICK_INTERVAL);
    }

    info!("demo: finished generating {MAX_NODES} nodes");
}
