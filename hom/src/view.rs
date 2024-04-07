// view.rs      View module
//
// Copyright (c) 2022-2024  Douglas Lau
//
use crate::cube::build_cube;
use bevy::{
    asset::LoadState,
    gltf::Gltf,
    input::mouse::{MouseMotion, MouseWheel},
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::primitives::Aabb,
    scene::InstanceId,
    window::{PrimaryWindow, Window},
};
use std::f32::consts::PI;
use std::path::PathBuf;

/// Path configuration resource for glTF
#[derive(Resource)]
struct PathConfig {
    path: PathBuf,
}

/// Scene state
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum SceneState {
    Loading,
    Spawning,
    SpawnCamera,
    StartAnimation,
    Started,
}

/// Scene resource data
#[derive(Resource)]
struct SceneRes {
    handle: Handle<Gltf>,
    id: Option<InstanceId>,
    animations: Vec<Handle<AnimationClip>>,
    state: SceneState,
}

/// Camera controller component
#[derive(Component)]
struct CameraController {
    focus: Vec3,
    distance: f32,
}

/// Cursor for camera
#[derive(Component)]
struct Cursor;

/// Stage (ground)
#[derive(Component)]
struct Stage;

impl CameraController {
    /// Create a new camera controller
    fn new(pos: Vec3, focus: Vec3) -> Self {
        CameraController {
            focus,
            distance: pos.distance(focus),
        }
    }

    /// Update camera transform
    fn update_transform(&self, xform: &mut Transform) {
        let rot = Mat3::from_quat(xform.rotation);
        xform.translation =
            self.focus + rot.mul_vec3(Vec3::new(0.0, 0.0, self.distance));
    }

    /// Pan camera
    fn pan(&mut self, xform: &mut Transform, motion: Vec2, win_sz: Vec2) {
        let proj = PerspectiveProjection::default(); // FIXME
        let pan =
            motion * Vec2::new(proj.fov * proj.aspect_ratio, proj.fov) / win_sz;
        let right = xform.rotation * Vec3::X * -pan.x;
        let up = xform.rotation * Vec3::Y * pan.y;
        let translation = (right + up) * self.distance;
        self.focus += translation;
        self.update_transform(xform);
    }

    /// Rotate camera
    fn rotate(&mut self, xform: &mut Transform, motion: Vec2, win_sz: Vec2) {
        let delta_x = motion.x / win_sz.x * PI;
        let delta_y = motion.y / win_sz.y * PI;
        xform.rotation = Quat::from_rotation_y(-delta_x * 2.0)
            * xform.rotation
            * Quat::from_rotation_x(-delta_y);
        self.update_transform(xform);
    }

    /// Move camera forward / reverse
    fn forward_reverse(&mut self, xform: &mut Transform, motion: f32) {
        let pos = xform.translation;
        let rot = Mat3::from_quat(xform.rotation);
        let dist = self.distance + motion * self.distance * 0.1;
        self.focus = pos - rot.mul_vec3(Vec3::new(0.0, 0.0, dist));
        self.update_transform(xform);
    }

    /// Zoom camera in or out
    fn zoom(&mut self, xform: &mut Transform, motion: f32) {
        if motion < 0.0 {
            self.distance -= motion * self.distance.max(1.0) * 0.1;
        } else {
            self.distance -= motion * self.distance * 0.1;
        }
        self.update_transform(xform);
    }
}

/// View glTF in an app window
pub fn view_gltf(folder: String, path: PathBuf) {
    let mut app = App::new();
    app.insert_resource(PathConfig { path })
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 500.0,
        })
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: folder,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "homunculus".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(WireframePlugin)
        .add_systems(
            Startup,
            (init_wireframe, init_gizmo, spawn_light, start_loading),
        )
        .add_systems(
            Update,
            (
                spawn_scene,
                check_ready,
                spawn_camera,
                start_animation,
                control_animation,
                draw_cursor,
                pan_rotate_camera,
                zoom_camera,
                update_light_direction,
                toggle_stage,
                toggle_wireframe,
                toggle_help,
            ),
        )
        .run();
}

/// System to initialize wireframe config
fn init_wireframe(mut wireframe_config: ResMut<WireframeConfig>) {
    wireframe_config.global = false;
}

/// System to initialize gizmo config
fn init_gizmo(mut config_store: ResMut<GizmoConfigStore>) {
    for (_, config, _) in config_store.iter_mut() {
        config.line_width = 10.0;
        config.line_perspective = true;
        config.depth_bias = -1.0;
    }
}

/// System to spawn light
fn spawn_light(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    });
}

/// System to spawn help text
fn spawn_help(commands: &mut Commands, camera_id: Entity) {
    commands.spawn((
        TargetCamera(camera_id),
        TextBundle::from_section(
            "_____ Mouse _____\n\
             right: pan camera\n\
             middle: rotate camera\n\
             wheel: zoom camera\n\
             /pressed: forward/back\n\
             \n\
             _____ Keys _____\n\
             'Q': toggle help text\n\
             'W': toggle wireframe\n\
             'S': toggle stage\n\
             'D': light direction\n\
             Space: next animation",
            TextStyle {
                font_size: 18.0,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            ..default()
        }),
    ));
}

/// System to start loading scene
fn start_loading(
    mut commands: Commands,
    config: Res<PathConfig>,
    asset_svr: Res<AssetServer>,
) {
    commands.insert_resource(SceneRes {
        handle: asset_svr.load(config.path.clone()),
        id: None,
        animations: Vec::new(),
        state: SceneState::Loading,
    });
}

/// System to spawn the scene
fn spawn_scene(
    mut scene_res: ResMut<SceneRes>,
    asset_svr: Res<AssetServer>,
    gltf_assets: ResMut<Assets<Gltf>>,
    mut spawner: ResMut<SceneSpawner>,
) {
    if scene_res.state != SceneState::Loading {
        return;
    }
    if let Some(LoadState::Loaded) = asset_svr.get_load_state(&scene_res.handle)
    {
        let gltf = gltf_assets.get(&scene_res.handle).unwrap();
        if let Some(scene) = gltf.scenes.first() {
            scene_res.id = Some(spawner.spawn(scene.clone_weak()));
            scene_res.animations = gltf.animations.clone();
            scene_res.state = SceneState::Spawning;
        }
    }
}

/// System to check whether scene is ready (after spawning)
fn check_ready(mut scene_res: ResMut<SceneRes>, spawner: Res<SceneSpawner>) {
    if scene_res.state != SceneState::Spawning {
        return;
    }
    let id = scene_res.id.unwrap();
    if spawner.instance_is_ready(id) {
        scene_res.state = SceneState::SpawnCamera;
    }
}

/// System to spawn camera
fn spawn_camera(
    mut scene_res: ResMut<SceneRes>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&GlobalTransform, &Aabb), With<Handle<Mesh>>>,
) {
    if scene_res.state != SceneState::SpawnCamera {
        return;
    }
    scene_res.state = SceneState::StartAnimation;
    let aabb = bounding_box_meshes(query);
    let (bundle, cam) = camera_bundle(aabb);
    let mut xform = Transform::from_translation(aabb.center.into());
    xform.scale = Vec3::splat(cam.distance * 0.02);
    let id = commands.spawn((bundle, cam)).id();
    spawn_help(&mut commands, id);
    commands.spawn((
        Cursor,
        MaterialMeshBundle {
            mesh: meshes.add(build_cube()),
            material: materials.add(StandardMaterial {
                base_color: Color::FUCHSIA,
                ..Default::default()
            }),
            transform: xform,
            ..Default::default()
        },
    ));

    let min = aabb.min();
    let max = aabb.max();
    let size = (max.x - min.x).max(max.y - min.y).max(max.z - min.z);
    commands.spawn((
        Stage,
        MaterialMeshBundle {
            mesh: meshes
                .add(Mesh::from(Plane3d::default().mesh().size(size, size))),
            material: materials.add(StandardMaterial {
                base_color: Color::DARK_GREEN,
                ..default()
            }),
            visibility: Visibility::Hidden,
            ..Default::default()
        },
    ));
}

/// Get a bounding box containing all meshes
fn bounding_box_meshes(
    query: Query<(&GlobalTransform, &Aabb), With<Handle<Mesh>>>,
) -> Aabb {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for (xform, aabb) in &query {
        min = min.min(xform.transform_point(aabb.min().into()));
        max = max.max(xform.transform_point(aabb.max().into()));
    }
    Aabb::from_min_max(min, max)
}

/// Build camera bundle with controller
fn camera_bundle(aabb: Aabb) -> (Camera3dBundle, CameraController) {
    let look = Vec3::from(aabb.center);
    let pos = look
        + Vec3::new(0.0, 2.0 * aabb.half_extents.y, 4.0 * aabb.half_extents.z);
    (
        Camera3dBundle {
            transform: Transform::from_translation(pos)
                .looking_at(look, Vec3::Y),
            ..Default::default()
        },
        CameraController::new(pos, look),
    )
}

/// System to start the animation player
fn start_animation(
    mut scene_res: ResMut<SceneRes>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if scene_res.state != SceneState::StartAnimation {
        return;
    }
    if let Ok(mut player) = players.get_single_mut() {
        if let Some(animation) = scene_res.animations.first() {
            player.play(animation.clone_weak()).repeat();
            scene_res.state = SceneState::Started;
        }
    }
}

/// System to control animations
fn control_animation(
    scene_res: Res<SceneRes>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<&mut AnimationPlayer>,
    mut animation_idx: Local<usize>,
    mut is_changing: Local<bool>,
) {
    if scene_res.state != SceneState::Started {
        return;
    }
    let mut player = players.get_single_mut().unwrap();
    if keyboard.pressed(KeyCode::Space) {
        player.pause();
        *is_changing = true;
    } else if *is_changing {
        *animation_idx = (*animation_idx + 1) % scene_res.animations.len();
        player
            .start(scene_res.animations[*animation_idx].clone_weak())
            .repeat();
        player.resume();
        *is_changing = false;
    }
}

/// System to draw cursor gizmo
fn draw_cursor(mut gizmos: Gizmos, query: Query<&Transform, With<Cursor>>) {
    for xform in &query {
        gizmos.cuboid(*xform, Color::FUCHSIA);
    }
}

/// System to pan/rotate the camera
#[allow(clippy::type_complexity)]
fn pan_rotate_camera(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut queries: ParamSet<(
        Query<(&mut CameraController, &mut Transform)>,
        Query<&mut Transform, With<Cursor>>,
    )>,
) {
    if !mouse.pressed(MouseButton::Right) && !mouse.pressed(MouseButton::Middle)
    {
        ev_motion.clear();
        return;
    }

    let mut motion = Vec2::ZERO;
    for ev in ev_motion.read() {
        motion += ev.delta;
    }
    if motion.length_squared() > 0.0 {
        let win_sz = primary_window_size(windows);
        if let Ok((mut cam, mut xform)) = queries.p0().get_single_mut() {
            if mouse.pressed(MouseButton::Right) {
                cam.pan(&mut xform, motion, win_sz);
                let focus = cam.focus;
                if let Ok(mut xform) = queries.p1().get_single_mut() {
                    xform.translation = focus;
                };
            } else {
                cam.rotate(&mut xform, motion, win_sz);
            }
        }
    }
}

/// Get the size of the primary window
fn primary_window_size(windows: Query<&Window, With<PrimaryWindow>>) -> Vec2 {
    let window = windows.get_single().unwrap();
    Vec2::new(window.width(), window.height())
}

/// System to zoom the camera
#[allow(clippy::type_complexity)]
fn zoom_camera(
    mouse: Res<ButtonInput<MouseButton>>,
    mut ev_scroll: EventReader<MouseWheel>,
    mut queries: ParamSet<(
        Query<(&mut CameraController, &mut Transform)>,
        Query<&mut Transform, With<Cursor>>,
    )>,
) {
    let mut motion = 0.0;
    for ev in ev_scroll.read() {
        motion += ev.y;
    }
    if motion.abs() > 0.0 {
        let mut focus = Vec3::default();
        let mut scale = 1.0;
        if let Ok((mut cam, mut xform)) = queries.p0().get_single_mut() {
            if mouse.pressed(MouseButton::Middle) {
                cam.forward_reverse(&mut xform, motion);
            } else {
                cam.zoom(&mut xform, motion);
            };
            focus = cam.focus;
            scale = cam.distance;
        }
        if let Ok(mut xform) = queries.p1().get_single_mut() {
            xform.translation = focus;
            xform.scale = Vec3::splat(scale * 0.02);
        };
    }
}

/// System to update the directional light
#[allow(clippy::type_complexity)]
fn update_light_direction(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut queries: ParamSet<(
        Query<&Transform, With<CameraController>>,
        Query<&mut Transform, With<DirectionalLight>>,
    )>,
) {
    if keyboard.just_pressed(KeyCode::KeyD) {
        let cam_rot = queries.p0().get_single().unwrap().rotation;
        for mut xform in &mut queries.p1() {
            xform.rotation = cam_rot;
        }
    }
}

/// System to toggle stage
fn toggle_stage(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Visibility, With<Stage>>,
) {
    if keyboard.just_pressed(KeyCode::KeyS) {
        let mut vis = query.single_mut();
        *vis = if *vis == Visibility::Hidden {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// System to toggle wireframe
fn toggle_wireframe(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut wireframe_config: ResMut<WireframeConfig>,
) {
    if keyboard.just_pressed(KeyCode::KeyW) {
        wireframe_config.global = !wireframe_config.global;
    }
}

/// System to toggle help text
fn toggle_help(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Visibility, With<Text>>,
) {
    if keyboard.just_pressed(KeyCode::KeyQ) {
        for mut vis in &mut query {
            *vis = if *vis == Visibility::Hidden {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}
