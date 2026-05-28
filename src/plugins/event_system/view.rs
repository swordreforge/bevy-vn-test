use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;
use crate::plugins::event_system::view_data::{self, ViewEntry, ViewTweenEntry};
use crate::plugins::event_system::view_material::ViewMaskMaterial;
use crate::resources::{Settings, ViewBlocking};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewPhase {
    FadeOut,
    PrepareScene,
    FadeIn,
    PenTween,
    PenWait,
    RevealName,
    DisplayWait,
    FadeOutScene,
    SetWindowColor,
    Done,
}

#[derive(Component)]
pub struct ViewState {
    pub char_id: String,
    pub phase: ViewPhase,
    pub timer: Timer,
    pub step_idx: usize,
    pub pen_entity: Option<Entity>,
    pub name_entity: Option<Entity>,
    pub mask_material: Option<Handle<ViewMaskMaterial>>,
    pub scene_entities: Vec<Entity>,
    pub entry: &'static ViewEntry,
    pub tween_entry: &'static ViewTweenEntry,
}

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewBlocking>()
            .add_systems(Update, advance_view);
    }
}

fn advance_view(
    mut commands: Commands,
    time: Res<Time>,
    mut view_query: Query<(Entity, &mut ViewState)>,
    mut overlay_query: Query<(
        Entity,
        &mut BackgroundColor,
        &mut Visibility,
    ), With<crate::components::ScreenOverlayRoot>>,
    asset_server: Res<AssetServer>,
    mut view_blocking: ResMut<ViewBlocking>,
    mut materials: ResMut<Assets<ViewMaskMaterial>>,
) {
    let Ok((view_entity, mut view)) = view_query.single_mut() else {
        if view_blocking.0 {
            view_blocking.0 = false;
        }
        return;
    };
    view_blocking.0 = true;

    match view.phase {
        ViewPhase::FadeOut => {
            view.timer.tick(time.delta());
            for (_, mut bg, mut vis) in overlay_query.iter_mut() {
                let progress = (view.timer.elapsed_secs() / view.timer.duration().as_secs_f32()).min(1.0);
                bg.0 = Color::srgba(0.0, 0.0, 0.0, progress);
                *vis = Visibility::Visible;
            }
            if view.timer.just_finished() {
                view.phase = ViewPhase::PrepareScene;
            }
        }
        ViewPhase::PrepareScene => {
            let entry = view.entry;
            let prefix = view_data::VIEW_PATH_PREFIX;
            let mut entities = Vec::new();

            let base = commands.spawn((
                ImageNode {
                    image: asset_server.load(format!("{}{}.png", prefix, entry.base_file)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                ViewSprite,
            )).id();
            entities.push(base);

            let name_x = entry.name_x as f32;
            let name_texture: Handle<Image> = asset_server.load(format!("{}{}.png", prefix, entry.name_file));
            let mask_path = format!("images/rule/{}.png", entry.mask_file);
            let mask_texture: Handle<Image> = asset_server.load(&mask_path);
            let mat_handle = materials.add(ViewMaskMaterial {
                name_texture,
                mask_texture,
                progress: 0.0,
                name_left: name_x,
                name_top: 393.0,
            });
            view.mask_material = Some(mat_handle.clone());

            let name = commands.spawn((
                MaterialNode(mat_handle),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(name_x),
                    top: Val::Px(393.0),
                    ..default()
                },
                ViewSprite,
            )).id();
            entities.push(name);
            view.name_entity = Some(name);

            let pen = commands.spawn((
                ImageNode {
                    image: asset_server.load(format!("{}view_pen.png", prefix)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(640.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                ViewSprite,
                ViewPen,
            )).id();
            entities.push(pen);
            view.pen_entity = Some(pen);

            view.scene_entities = entities;
            view.phase = ViewPhase::FadeIn;
            view.timer = Timer::from_seconds(0.2, TimerMode::Once);
        }
        ViewPhase::FadeIn => {
            view.timer.tick(time.delta());
            for (_, mut bg, mut vis) in overlay_query.iter_mut() {
                let progress = 1.0 - (view.timer.elapsed_secs() / view.timer.duration().as_secs_f32()).min(1.0);
                bg.0 = Color::srgba(0.0, 0.0, 0.0, progress);
                if view.timer.just_finished() {
                    *vis = Visibility::Hidden;
                }
            }
            if view.timer.just_finished() {
                view.phase = ViewPhase::PenTween;
                view.step_idx = 0;
                view.timer = Timer::from_seconds(
                    view.tween_entry.step_wait_ms as f32 / 1000.0,
                    TimerMode::Once,
                );
            }
        }
        ViewPhase::PenTween => {
            view.timer.tick(time.delta());
            if view.timer.just_finished() {
                let tween = view.tween_entry;
                let waypoints = tween.waypoints;
                if view.step_idx < waypoints.len() {
                    if let Some(pen_e) = view.pen_entity {
                        if let Ok(mut entity_cmd) = commands.get_entity(pen_e) {
                            let (x, y) = waypoints[view.step_idx];
                            entity_cmd.insert(Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(x),
                                top: Val::Px(y),
                                ..default()
                            });
                        }
                    }
                    view.step_idx += 1;
                }
                if view.step_idx < waypoints.len() {
                    view.timer = Timer::from_seconds(
                        tween.step_duration_ms as f32 / 1000.0,
                        TimerMode::Once,
                    );
                } else {
                    view.phase = ViewPhase::PenWait;
                    view.timer = Timer::from_seconds(
                        tween.step_wait_ms as f32 / 1000.0,
                        TimerMode::Once,
                    );
                }
            }
        }
        ViewPhase::PenWait => {
            view.timer.tick(time.delta());
            if view.timer.just_finished() {
                view.phase = ViewPhase::RevealName;
                view.timer = Timer::from_seconds(
                    view.tween_entry.reveal_time_ms as f32 / 1000.0,
                    TimerMode::Once,
                );
            }
        }
        ViewPhase::RevealName => {
            view.timer.tick(time.delta());
            let progress = (view.timer.elapsed_secs() / view.timer.duration().as_secs_f32()).min(1.0);
            if let Some(mat) = &view.mask_material {
                if let Some(material) = materials.get_mut(mat) {
                    material.progress = progress;
                }
            }
            if view.timer.just_finished() {
                view.phase = ViewPhase::DisplayWait;
                view.timer = Timer::from_seconds(1.0, TimerMode::Once);
            }
        }
        ViewPhase::DisplayWait => {
            view.timer.tick(time.delta());
            if view.timer.just_finished() {
                view.phase = ViewPhase::FadeOutScene;
                view.timer = Timer::from_seconds(0.25, TimerMode::Once);
            }
        }
        ViewPhase::FadeOutScene => {
            view.timer.tick(time.delta());
            for (_, mut bg, mut vis) in overlay_query.iter_mut() {
                let progress = (view.timer.elapsed_secs() / view.timer.duration().as_secs_f32()).min(1.0);
                bg.0 = Color::srgba(0.0, 0.0, 0.0, progress);
                *vis = Visibility::Visible;
            }
            if view.timer.just_finished() {
                for entity in &view.scene_entities {
                    if let Ok(mut e) = commands.get_entity(*entity) {
                        e.despawn();
                    }
                }
                view.scene_entities.clear();
                view.phase = ViewPhase::SetWindowColor;
            }
        }
        ViewPhase::SetWindowColor => {
            let wc = view.entry.window_color as i32;
            commands.queue(move |world: &mut World| {
                let mut settings = world.resource_mut::<Settings>();
                settings.window_color_idx = wc;
            });
            view.phase = ViewPhase::Done;
        }
        ViewPhase::Done => {
            for (_, mut bg, mut vis) in overlay_query.iter_mut() {
                bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                *vis = Visibility::Hidden;
            }
            view_blocking.0 = false;
            commands.entity(view_entity).despawn();
        }
    }
}

#[derive(Component)]
struct ViewSprite;

#[derive(Component)]
struct ViewPen;
