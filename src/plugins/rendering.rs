use bevy::prelude::*;
use crate::components::*;
use crate::resources::{BgState, BgCrossFade, CgState, CgFade, CgFadeKind, SpriteManager, SpriteFade, SpriteFadeKind, TextureCache};
use crate::script::{FgPosition, Transition};
use crate::state::AppState;
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowCgMessage, HideCgMessage,
};

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetBgMessage>()
            .add_message::<ShowFgMessage>()
            .add_message::<HideFgMessage>()
            .add_message::<ShowCgMessage>()
            .add_message::<HideCgMessage>()
            .init_resource::<BgState>()
            .init_resource::<SpriteManager>()
            .init_resource::<CgState>()
            .init_resource::<TextureCache>()
            .add_systems(OnEnter(AppState::Gameplay), setup_rendering)
            .add_systems(OnExit(AppState::Gameplay), cleanup_rendering)
            .add_systems(Update, (
                update_bg_fade,
                update_fg_fade,
                update_cg_fade,
                handle_set_bg,
                handle_show_fg,
                handle_hide_fg,
                handle_show_cg,
                handle_hide_cg,
            ).chain().run_if(in_state(AppState::Gameplay)));
    }
}

fn setup_rendering(mut commands: Commands, mut bg_state: ResMut<BgState>, mut sprite_mgr: ResMut<SpriteManager>) {
    // Spawn dual-buffer background entities
    let bg_a = commands.spawn((
        BackgroundRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        ImageNode::default(),
        Visibility::Visible,
        ZIndex(0),
    )).id();

    let bg_b = commands.spawn((
        BackgroundRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        ImageNode::default(),
        Visibility::Hidden,
        ZIndex(0),
    )).id();

    bg_state.entities = [bg_a, bg_b];
    bg_state.active_idx = 0;

    // Spawn 3 pooled sprite entities (Left, Center, Right)
    let positions = [
        (FgPosition::Left, Val::Px(0.0)),
        (FgPosition::Center, Val::Px(250.0)),
        (FgPosition::Right, Val::Px(500.0)),
    ];

    for (pos, left_val) in &positions {
        let entity = commands.spawn((
            SpriteSlotMarker(pos.clone()),
            Node {
                width: Val::Px(780.0),
                height: Val::Px(720.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: *left_val,
                ..default()
            },
            ImageNode::default(),
            Visibility::Hidden,
            ZIndex(1),
        )).id();

        sprite_mgr.slots.insert(pos.clone(), crate::resources::SpriteSlotInfo {
            char_id: String::new(),
            expression: String::new(),
            entity,
            texture: None,
            fade: None,
        });
    }
}

fn cleanup_rendering(mut commands: Commands, query: Query<Entity, Or<(With<BackgroundRoot>, With<SpriteSlotMarker>, With<CgRoot>)>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn handle_set_bg(
    mut msg: MessageReader<SetBgMessage>,
    mut bg_state: ResMut<BgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        // Complete any in-progress fade instantly
        if bg_state.fade.is_some() {
            if let Ok((_, mut vis, _)) = query.get_mut(bg_state.entities[bg_state.active_idx]) {
                *vis = Visibility::Hidden;
            }
            bg_state.active_idx = 1 - bg_state.active_idx;
            if let Ok((_, _, mut bg)) = query.get_mut(bg_state.entities[bg_state.active_idx]) {
                bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0);
            }
            bg_state.fade = None;
        }

        let path = format!("images/bg/{}", msg.file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        let inactive_idx = 1 - bg_state.active_idx;
        let inactive_entity = bg_state.entities[inactive_idx];

        if let Ok((mut image_node, mut vis, mut bg)) = query.get_mut(inactive_entity) {
            image_node.image = handle;
            match msg.transition {
                Some(Transition::Fade) => {
                    let dur = msg.duration.unwrap_or(0.5) as f32;
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                    *vis = Visibility::Visible;
                    bg_state.fade = Some(BgCrossFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                    });
                }
                _ => {
                    *vis = Visibility::Visible;
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0);
                    if let Ok((_, mut old_vis, _)) = query.get_mut(bg_state.entities[bg_state.active_idx]) {
                        *old_vis = Visibility::Hidden;
                    }
                    bg_state.active_idx = inactive_idx;
                    bg_state.fade = None;
                }
            }
        }
    }
}

fn handle_show_fg(
    mut msg: MessageReader<ShowFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        let slot = sprite_mgr.slots.get_mut(&msg.position);
        if let Some(slot) = slot {
            let path = format!("images/fg/{}/tati_{}.png", msg.char_id, msg.expression);
            let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
                asset_server.load(&path)
            }).clone();

            slot.char_id = msg.char_id.clone();
            slot.expression = msg.expression.clone();
            slot.texture = Some(handle.clone());

            if let Ok((mut image_node, mut vis, mut bg)) = query.get_mut(slot.entity) {
                image_node.image = handle;
                match msg.transition {
                    Some(Transition::Fade) => {
                        let dur = msg.duration.unwrap_or(0.5) as f32;
                        bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                        *vis = Visibility::Visible;
                        slot.fade = Some(SpriteFade {
                            timer: Timer::from_seconds(dur, TimerMode::Once),
                            kind: SpriteFadeKind::FadeIn,
                        });
                    }
                    _ => {
                        bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0);
                        *vis = Visibility::Visible;
                        slot.fade = None;
                    }
                }
            }
        } else {
            warn!("No sprite slot for position: {:?}", msg.position);
        }
    }
}

fn handle_hide_fg(
    mut msg: MessageReader<HideFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        let slot = sprite_mgr.slots.values_mut()
            .find(|s| s.char_id == msg.char_id);

        if let Some(slot) = slot {
            match msg.transition {
                Some(Transition::Fade) => {
                    let dur = msg.duration.unwrap_or(0.5) as f32;
                    slot.fade = Some(SpriteFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                        kind: SpriteFadeKind::FadeOut,
                    });
                }
                _ => {
                    slot.char_id.clear();
                    slot.expression.clear();
                    slot.texture = None;
                    if let Ok((mut image_node, mut vis, _)) = query.get_mut(slot.entity) {
                        image_node.image = Handle::default();
                        *vis = Visibility::Hidden;
                    }
                }
            }
        }
    }
}

fn handle_show_cg(
    mut msg: MessageReader<ShowCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        if let Some(entity) = cg_state.entity.take() {
            commands.entity(entity).despawn();
        }

        let path = format!("images/ev/{}", msg.file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        let initial_alpha = match msg.transition {
            Some(Transition::Fade) => 0.0,
            _ => 1.0,
        };

        let entity = commands.spawn((
            CgRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                ..default()
            },
            ImageNode::new(handle.clone()),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, initial_alpha)),
            Visibility::Visible,
            ZIndex(2),
        )).id();

        cg_state.active = true;
        cg_state.entity = Some(entity);
        cg_state.texture = Some(handle);

        match msg.transition {
            Some(Transition::Fade) => {
                let dur = msg.duration.unwrap_or(0.5) as f32;
                cg_state.fade = Some(CgFade {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    kind: CgFadeKind::FadeIn,
                });
            }
            _ => {}
        }
    }
}

fn handle_hide_cg(
    mut msg: MessageReader<HideCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        match msg.transition {
            Some(Transition::Fade) => {
                if cg_state.entity.is_some() {
                    let dur = msg.duration.unwrap_or(0.5) as f32;
                    cg_state.fade = Some(CgFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                        kind: CgFadeKind::FadeOut,
                    });
                }
            }
            _ => {
                if let Some(entity) = cg_state.entity.take() {
                    commands.entity(entity).despawn();
                }
                cg_state.active = false;
                cg_state.texture = None;
            }
        }
    }
}

fn update_bg_fade(
    time: Res<Time>,
    mut bg_state: ResMut<BgState>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    if bg_state.fade.is_none() {
        return;
    }

    let active_idx = bg_state.active_idx;
    let entities = bg_state.entities;
    let active_entity = entities[active_idx];
    let inactive_entity = entities[1 - active_idx];

    let finished = {
        let fade = bg_state.fade.as_mut().unwrap();
        fade.timer.tick(time.delta());
        let t = fade.timer.fraction();

        if let Ok((mut bg, _)) = query.get_mut(active_entity) {
            bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0 - t);
        }
        if let Ok((mut bg, _)) = query.get_mut(inactive_entity) {
            bg.0 = Color::srgba(0.0, 0.0, 0.0, t);
        }

        let finished = fade.timer.just_finished();
        if finished {
            if let Ok((_, mut vis)) = query.get_mut(active_entity) {
                *vis = Visibility::Hidden;
            }
        }
        finished
    };

    if finished {
        bg_state.active_idx = 1 - bg_state.active_idx;
        bg_state.fade = None;
    }
}

fn update_fg_fade(
    time: Res<Time>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    for (_position, slot) in sprite_mgr.slots.iter_mut() {
        let finished = {
            let fade = match &mut slot.fade {
                Some(f) => f,
                None => continue,
            };

            fade.timer.tick(time.delta());
            let t = fade.timer.fraction();

            if let Ok((mut bg, _)) = query.get_mut(slot.entity) {
                let alpha = match fade.kind {
                    SpriteFadeKind::FadeIn => t,
                    SpriteFadeKind::FadeOut => 1.0 - t,
                };
                bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
            }

            let finished = fade.timer.just_finished();
            if finished && matches!(fade.kind, SpriteFadeKind::FadeOut) {
                if let Ok((_, mut vis)) = query.get_mut(slot.entity) {
                    *vis = Visibility::Hidden;
                }
            }
            finished
        };

        if finished {
            slot.fade = None;
        }
    }
}

fn update_cg_fade(
    time: Res<Time>,
    mut cg_state: ResMut<CgState>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    let entity = cg_state.entity;
    let finished = {
        let fade = match &mut cg_state.fade {
            Some(f) => f,
            None => return,
        };

        fade.timer.tick(time.delta());
        let t = fade.timer.fraction();

        if let Some(entity) = entity {
            if let Ok((mut bg, _)) = query.get_mut(entity) {
                let alpha = match fade.kind {
                    CgFadeKind::FadeIn => t,
                    CgFadeKind::FadeOut => 1.0 - t,
                };
                bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
            }

            let finished = fade.timer.just_finished();
            if finished && matches!(fade.kind, CgFadeKind::FadeOut) {
                if let Ok((_, mut vis)) = query.get_mut(entity) {
                    *vis = Visibility::Hidden;
                }
            }
            finished
        } else {
            true
        }
    };

    if finished {
        cg_state.fade = None;
    }
}
