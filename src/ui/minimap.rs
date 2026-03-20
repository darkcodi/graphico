use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureUsages};

#[derive(Component)]
pub struct MinimapCamera;

#[derive(Component)]
pub struct MinimapImage;

/// Setup a second camera that renders to a texture, displayed as a small UI image.
pub fn setup_minimap(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    };

    let mut image = Image::default();
    image.resize(size);
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = images.add(image);

    // Minimap camera — renders everything with a very wide view
    commands.spawn((
        MinimapCamera,
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle: image_handle.clone(),
            scale_factor: 1.0,
        }),
        Projection::Orthographic(OrthographicProjection {
            scale: 50.0,
            ..OrthographicProjection::default_2d()
        }),
    ));

    // UI image displaying the minimap
    commands.spawn((
        MinimapImage,
        ImageNode::new(image_handle),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            bottom: Val::Px(10.0),
            width: Val::Px(200.0),
            height: Val::Px(200.0),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
    ));
}

/// Keep the minimap camera centered on the same target as the main camera.
pub fn update_minimap_camera(
    main_camera_q: Query<&Transform, (With<Camera2d>, Without<MinimapCamera>)>,
    mut minimap_camera_q: Query<&mut Transform, With<MinimapCamera>>,
) {
    let Ok(main_tf) = main_camera_q.single() else {
        return;
    };
    let Ok(mut minimap_tf) = minimap_camera_q.single_mut() else {
        return;
    };

    minimap_tf.translation.x = main_tf.translation.x;
    minimap_tf.translation.y = main_tf.translation.y;
}
