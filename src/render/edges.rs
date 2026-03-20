use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;

use crate::graph::model::GraphData;
use crate::spatial::grid::ChunkData;

/// Build a LineList mesh for all edges within a chunk.
/// Positions are in world space (chunk entity is at origin).
pub fn build_edge_mesh(chunk_data: &ChunkData, graph: &GraphData) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();

    let process_edge = |edge_id, positions: &mut Vec<[f32; 3]>, colors: &mut Vec<[f32; 4]>| {
        if let Some(edge) = graph.edges.get(edge_id) {
            let source = graph.nodes.get(&edge.source);
            let target = graph.nodes.get(&edge.target);
            if let (Some(src), Some(tgt)) = (source, target) {
                let color_arr = edge.color.to_linear().to_f32_array();
                positions.push([src.position.x, src.position.y, 0.0]);
                positions.push([tgt.position.x, tgt.position.y, 0.0]);
                colors.push(color_arr);
                colors.push(color_arr);
            }
        }
    };

    for edge_id in &chunk_data.edge_ids {
        process_edge(edge_id, &mut positions, &mut colors);
    }
    for edge_id in &chunk_data.cross_edges {
        process_edge(edge_id, &mut positions, &mut colors);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, default());

    if positions.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    } else {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    mesh
}
