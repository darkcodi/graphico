use bevy::camera::primitives::Aabb;
use bevy::prelude::*;

use super::grid::{SpatialGrid, CHUNK_SIZE};
use crate::graph::components::ChunkEntity;
use crate::graph::model::GraphData;
use crate::render::edges::build_edge_mesh;

/// Rebuild dirty chunk meshes and ensure chunk entities exist.
pub fn rebuild_dirty_chunks(
    mut commands: Commands,
    mut grid: ResMut<SpatialGrid>,
    graph: Res<GraphData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut chunk_query: Query<(Entity, &mut Mesh2d), With<ChunkEntity>>,
) {
    let mut rebuilt = 0;
    let max_rebuilds_per_frame = 10;

    for (coord, chunk_data) in grid.cells.iter_mut() {
        if !chunk_data.dirty {
            continue;
        }
        if rebuilt >= max_rebuilds_per_frame {
            break;
        }

        let mesh = build_edge_mesh(chunk_data, &graph);

        if let Some(entity) = chunk_data.entity {
            if let Ok((_ent, mut mesh_handle)) = chunk_query.get_mut(entity) {
                mesh_handle.0 = meshes.add(mesh);
            }
        } else {
            let center = Vec2::new(
                coord.x as f32 * CHUNK_SIZE + CHUNK_SIZE * 0.5,
                coord.y as f32 * CHUNK_SIZE + CHUNK_SIZE * 0.5,
            );
            let half = CHUNK_SIZE * 0.5;

            let entity = commands
                .spawn((
                    ChunkEntity,
                    Mesh2d(meshes.add(mesh)),
                    MeshMaterial2d(materials.add(ColorMaterial::default())),
                    Transform::IDENTITY,
                    Aabb::from_min_max(
                        Vec3::new(center.x - half, center.y - half, -1.0),
                        Vec3::new(center.x + half, center.y + half, 1.0),
                    ),
                ))
                .id();
            chunk_data.entity = Some(entity);
        }

        chunk_data.dirty = false;
        rebuilt += 1;
    }
}
