use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

const API_BASE: &str = "http://127.0.0.1:3000";
const NODES_PER_TICK: usize = 10;
const MAX_NODES: usize = 50;
const TICK_INTERVAL: Duration = Duration::from_millis(50);

pub struct DemoPlugin;

impl Plugin for DemoPlugin {
    fn build(&self, _app: &mut App) {
        std::thread::spawn(demo_loop);
    }
}

fn demo_loop() {
    std::thread::sleep(Duration::from_secs(1));

    let agent = ureq::Agent::new_with_defaults();
    let mut created_ids: Vec<String> = Vec::new();
    let mut rng = rand::rng();
    let mut total = 0usize;

    while total < MAX_NODES {
        for _ in 0..NODES_PER_TICK {
            let hue: f32 = rng.random_range(0.0..360.0);
            let color = hsl_to_hex(hue, 0.7, 0.6);

            let edges: Vec<&str> = if created_ids.len() >= 2 && rng.random_bool(0.5) {
                let idx = rng.random_range(0..created_ids.len());
                vec![created_ids[idx].as_str()]
            } else {
                vec![]
            };

            total += 1;
            let name = format!("N{}", total);

            let body = serde_json::json!({
                "name": name,
                "color": color,
                "edges": edges,
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
