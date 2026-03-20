use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Holds the shared circle texture used for all node sprites.
#[derive(Resource)]
pub struct NodeCircleTexture(pub Handle<Image>);

/// Create a small white circle texture at startup.
pub fn create_circle_texture(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = 64u32;
    let center = size as f32 / 2.0;
    let radius = center - 1.0;

    let mut data = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center + 0.5;
            let dy = y as f32 - center + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();

            let alpha = if dist <= radius - 1.0 {
                255
            } else if dist <= radius {
                ((radius - dist) * 255.0) as u8
            } else {
                0
            };

            let idx = ((y * size + x) * 4) as usize;
            data[idx] = 255; // R
            data[idx + 1] = 255; // G
            data[idx + 2] = 255; // B
            data[idx + 3] = alpha; // A
        }
    }

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
    commands.insert_resource(NodeCircleTexture(handle));
}
