use bevy::{
    asset::{AssetServerSettings, LoadState},
    gltf::Gltf,
    input::mouse::{MouseMotion, MouseWheel},
    math::Vec3A,
    prelude::*,
    render::primitives::{Aabb, Sphere},
    scene::InstanceId,
};
use std::f32::consts::PI;
use std::path::PathBuf;

struct PathConfig {
    path: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum SceneState {
    Loading,
    Spawning,
    Ready,
    Setup,
    Started,
}

struct SceneRes {
    handle: Handle<Gltf>,
    id: Option<InstanceId>,
    animations: Vec<Handle<AnimationClip>>,
    state: SceneState,
}

#[derive(Component)]
struct CameraController {
    focus: Vec3,
    radius: f32,
}

impl CameraController {
    fn new(pos: Vec3, focus: Vec3) -> Self {
        CameraController {
            focus,
            radius: pos.distance(focus),
        }
    }

    fn update_transform(&self, transform: &mut Transform) {
        let rot = Mat3::from_quat(transform.rotation);
        transform.translation =
            self.focus + rot.mul_vec3(Vec3::new(0.0, 0.0, self.radius));
    }

    fn pan(&mut self, transform: &mut Transform, motion: Vec2, win_sz: Vec2) {
        let proj = PerspectiveProjection::default(); // FIXME
        let pan =
            motion * Vec2::new(proj.fov * proj.aspect_ratio, proj.fov) / win_sz;
        let right = transform.rotation * Vec3::X * -pan.x;
        let up = transform.rotation * Vec3::Y * pan.y;
        let translation = (right + up) * self.radius;
        self.focus += translation;
        self.update_transform(transform);
    }

    fn rotate(
        &mut self,
        transform: &mut Transform,
        motion: Vec2,
        win_sz: Vec2,
    ) {
        let delta_x = motion.x / win_sz.x * PI;
        let delta_y = motion.y / win_sz.y * PI;
        transform.rotation = Quat::from_rotation_y(-delta_x * 2.0)
            * transform.rotation
            * Quat::from_rotation_x(-delta_y);
        self.update_transform(transform);
    }

    fn zoom(&mut self, transform: &mut Transform, motion: f32) {
        self.radius -= motion * self.radius * 0.2;
        self.radius = self.radius.max(0.1);
        self.update_transform(transform);
    }
}

pub fn view_gltf(folder: String, path: PathBuf) {
    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        title: "homunculus".to_string(),
        ..default()
    })
    .insert_resource(PathConfig { path })
    .insert_resource(AssetServerSettings {
        asset_folder: folder,
        watch_for_changes: true,
    })
    .insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    })
    .add_plugins(DefaultPlugins)
    .add_startup_system(start_loading)
    .add_system(check_loaded)
    .add_system(check_ready)
    .add_system(setup_camera_and_light)
    .add_system(start_animation)
    .add_system(control_animation)
    .add_system(update_camera)
    .add_system(update_light_direction)
    .run();
}

fn start_loading(
    mut commands: Commands,
    config: Res<PathConfig>,
    asset_svr: Res<AssetServer>,
) {
    commands.insert_resource(SceneRes {
        handle: asset_svr.load(&*config.path),
        id: None,
        animations: vec![],
        state: SceneState::Loading,
    });
}

fn check_loaded(
    mut scene_res: ResMut<SceneRes>,
    asset_svr: Res<AssetServer>,
    gltf_assets: ResMut<Assets<Gltf>>,
    mut spawner: ResMut<SceneSpawner>,
) {
    if scene_res.state != SceneState::Loading {
        return;
    }
    if asset_svr.get_load_state(&scene_res.handle) == LoadState::Loaded {
        let gltf = gltf_assets.get(&scene_res.handle).unwrap();
        if let Some(scene) = gltf.scenes.first() {
            scene_res.id = Some(spawner.spawn(scene.clone_weak()));
            scene_res.animations = gltf.animations.clone();
            scene_res.state = SceneState::Spawning;
        }
    }
}

fn check_ready(mut scene_res: ResMut<SceneRes>, spawner: Res<SceneSpawner>) {
    if scene_res.state != SceneState::Spawning {
        return;
    }
    let id = scene_res.id.unwrap();
    if spawner.instance_is_ready(id) {
        scene_res.state = SceneState::Ready;
    }
}

fn setup_camera_and_light(
    mut scene_res: ResMut<SceneRes>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&GlobalTransform, &Aabb), With<Handle<Mesh>>>,
) {
    if scene_res.state != SceneState::Ready {
        return;
    }
    scene_res.state = SceneState::Setup;
    let aabb = bounding_box_meshes(query);
    let (bundle, controller) = bundle_controller(aabb.clone());
    commands.spawn_bundle(bundle).insert(controller);
    let min = aabb.min();
    let max = aabb.max();
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: min.x,
                right: max.x,
                bottom: min.y,
                top: max.y,
                near: min.z,
                far: 2.0 * max.z,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
    let size = (max.x - min.x).max(max.z - min.z);
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size })),
        material: materials.add(StandardMaterial {
            base_color: Color::DARK_GREEN,
            perceptual_roughness: 1.0,
            ..default()
        }),
        ..default()
    });
}

/// Get a bounding box containing all meshes
fn bounding_box_meshes(
    query: Query<(&GlobalTransform, &Aabb), With<Handle<Mesh>>>,
) -> Aabb {
    let mut min = Vec3A::splat(f32::MAX);
    let mut max = Vec3A::splat(f32::MIN);
    for (xform, aabb) in &query {
        let aabb = Aabb::from(Sphere {
            center: Vec3A::from(xform.mul_vec3(aabb.center.into())),
            radius: xform.radius_vec3a(aabb.half_extents),
        });
        min = min.min(aabb.min());
        max = max.max(aabb.max());
    }
    let aabb = Aabb::from_min_max(Vec3::from(min), Vec3::from(max));
    Aabb::from(Sphere {
        center: aabb.center,
        radius: aabb.half_extents.length(),
    })
}

fn bundle_controller(aabb: Aabb) -> (Camera3dBundle, CameraController) {
    let look = Vec3::from(aabb.center);
    let pos = Vec3::from(aabb.center + aabb.half_extents * 0.6);
    let bundle = Camera3dBundle {
        transform: Transform::from_translation(pos).looking_at(look, Vec3::Y),
        ..Default::default()
    };
    let controller = CameraController::new(pos, look);
    (bundle, controller)
}

fn start_animation(
    mut scene_res: ResMut<SceneRes>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if scene_res.state != SceneState::Setup {
        return;
    }
    if let Ok(mut player) = players.get_single_mut() {
        if let Some(animation) = scene_res.animations.first() {
            player.play(animation.clone_weak()).repeat();
            scene_res.state = SceneState::Started;
        }
    }
}

fn control_animation(
    scene_res: Res<SceneRes>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut players: Query<&mut AnimationPlayer>,
    mut animation_idx: Local<usize>,
    mut is_changing: Local<bool>,
) {
    if scene_res.state != SceneState::Started {
        return;
    }
    let mut player = players.get_single_mut().unwrap();
    if mouse.pressed(MouseButton::Right) {
        player.pause();
        *is_changing = true;
    } else if *is_changing {
        *animation_idx = (*animation_idx + 1) % scene_res.animations.len();
        player
            .play(scene_res.animations[*animation_idx].clone_weak())
            .repeat();
        *is_changing = false;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        if player.is_paused() {
            player.resume();
        } else {
            player.pause();
        }
    }
}

fn update_camera(
    windows: Res<Windows>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    mouse: Res<Input<MouseButton>>,
    input: Res<Input<KeyCode>>,
    mut query: Query<(&mut CameraController, &mut Transform)>,
) {
    let (mut controller, mut transform) = match query.get_single_mut() {
        Ok((controller, transform)) => (controller, transform),
        Err(_) => return,
    };

    let shift =
        input.pressed(KeyCode::LShift) || input.pressed(KeyCode::RShift);
    if mouse.pressed(MouseButton::Middle) {
        let mut motion = Vec2::ZERO;
        for ev in ev_motion.iter() {
            motion += ev.delta;
        }
        if motion.length_squared() > 0.0 {
            let win_sz = primary_window_size(&windows);
            if shift {
                controller.pan(&mut transform, motion, win_sz);
            } else {
                controller.rotate(&mut transform, motion, win_sz);
            }
        }
    } else {
        let mut motion = 0.0;
        for ev in ev_scroll.iter() {
            motion += ev.y;
        }
        if motion.abs() > 0.0 {
            controller.zoom(&mut transform, motion);
        }
    }
}

fn primary_window_size(windows: &Res<Windows>) -> Vec2 {
    let window = windows.get_primary().unwrap();
    Vec2::new(window.width() as f32, window.height() as f32)
}

fn update_light_direction(
    mouse: Res<Input<MouseButton>>,
    mut queries: ParamSet<(
        Query<&Transform, With<CameraController>>,
        Query<&mut Transform, With<DirectionalLight>>,
    )>,
) {
    if mouse.pressed(MouseButton::Left) {
        let cam_rot = queries.p0().get_single().unwrap().rotation;
        for mut xform in &mut queries.p1() {
            xform.rotation = cam_rot;
        }
    }
}
