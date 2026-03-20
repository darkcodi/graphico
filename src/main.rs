use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use graphico::GraphicoPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Graphico — Large-Scale Graph Visualizer".into(),
                    resolution: (1600u32, 900u32).into(),
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            GraphicoPlugin,
        ))
        .add_systems(Update, diagnostics_overlay)
        .run();
}

#[derive(Component)]
struct DiagnosticsText;

fn diagnostics_overlay(
    mut commands: Commands,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    graph: Res<graphico::graph::model::GraphData>,
    grid: Res<graphico::spatial::grid::SpatialGrid>,
    mut text_q: Query<&mut Text, With<DiagnosticsText>>,
    camera_q: Query<&Projection, With<graphico::camera::components::CameraState>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let entity_count = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.value())
        .unwrap_or(0.0) as u64;

    let zoom = camera_q
        .single()
        .map(|p| match p {
            Projection::Orthographic(ortho) => ortho.scale,
            _ => 1.0,
        })
        .unwrap_or(1.0);

    let chunks = grid.cells.len();

    let info = format!(
        "FPS: {:.0} | Nodes: {} | Edges: {} | Chunks: {} | Entities: {} | Zoom: {:.2}",
        fps,
        graph.node_count(),
        graph.edge_count(),
        chunks,
        entity_count,
        zoom,
    );

    if let Ok(mut text) = text_q.single_mut() {
        **text = info;
    } else {
        commands.spawn((
            DiagnosticsText,
            Text::new(info),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.0, 1.0, 0.0)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
        ));
    }
}
