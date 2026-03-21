use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Holds the shared rectangle texture used for all node sprites.
#[derive(Resource)]
pub struct NodeRectTexture(pub Handle<Image>);

/// Create a small solid white texture at startup (stretches to any rectangle size).
pub fn create_rect_texture(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = 4u32;
    let data = vec![255u8; (size * size * 4) as usize];

    let image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );

    let handle = images.add(image);
    commands.insert_resource(NodeRectTexture(handle));
}

/// Estimate node rectangle size from multi-line text content.
pub fn estimate_text_size(data: &str) -> Vec2 {
    const CHAR_WIDTH: f32 = 7.0;
    const LINE_HEIGHT: f32 = 16.0;
    const PADDING_H: f32 = 20.0;
    const PADDING_V: f32 = 14.0;
    const MIN_WIDTH: f32 = 40.0;
    const MIN_HEIGHT: f32 = 24.0;

    if data.is_empty() {
        return Vec2::new(MIN_WIDTH, MIN_HEIGHT);
    }

    let lines: Vec<&str> = data.lines().collect();
    let num_lines = lines.len().max(1) as f32;
    let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(1) as f32;

    let width = (max_line_len * CHAR_WIDTH + PADDING_H).max(MIN_WIDTH);
    let height = (num_lines * LINE_HEIGHT + PADDING_V).max(MIN_HEIGHT);

    Vec2::new(width, height)
}
