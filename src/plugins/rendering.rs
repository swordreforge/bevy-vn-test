use bevy::prelude::*;
use crate::components::*;
use crate::state::AppState;
use crate::resources::{BgState, BgCrossFade, CgState, CgFade, CgFadeKind, ObjFileIndex, PendingBg, PendingCg, PendingFg, PendingTextures, QuakeState, SpriteManager, SpriteFade, SpriteFadeKind, SpriteOverlayManager, TextureCache};
use crate::plugins::save_load::SceneRestoreSet;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct RenderChain1;
use crate::script::{FgPosition, Transition};
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage, ScrollBgMessage,
    AnimateSpriteMessage, DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage,
};

include!(concat!(env!("OUT_DIR"), "/game_data.rs"));

pub(crate) fn char_dir(char_id: &str) -> Option<&'static str> {
    let prefix = &char_id[..2];
    match prefix {
        "01" => Some("001_eus"),
        "02" => Some("002_eri"),
        "03" => Some("003_ire"),
        "04" => Some("004_lic"),
        "05" => Some("005_fio"),
        "11" => Some("011_sis"),
        "12" => Some("012_mel"),
        "13" => Some("013_lav"),
        "14" => Some("014_cla"),
        "15" => Some("015_ris"),
        "16" => Some("016_iri"),
        "17" => Some("017_gau"),
        "32" => Some("032_luc"),
        "33" => Some("033_kur"),
        "34" => Some("034_sie"),
        "35" => Some("035_oz"),
        "36" => Some("036_gil"),
        "40" => Some("040_vel"),
        "41" => Some("041_val"),
        "42" => Some("042_kok"),
        "43" => Some("043_lan"),
        "44" => Some("044_nud"),
        _ => None,
    }
}

pub(crate) fn resolve_fg_path(char_id: &str, dir: &str) -> String {
    let base = format!("image/fg/{}/tati_{}.png", dir, char_id);
    if !std::path::Path::new("assets").join(&base).exists() {
        for prefix in &["z", "zz", "zzz"] {
            let alt = format!("image/fg/{}/tati_{}{}.png", dir, prefix, char_id);
            if std::path::Path::new("assets").join(&alt).exists() {
                return alt;
            }
        }
    }
    base
}

#[derive(Resource, Default)]
pub struct RenderingInitialized(pub bool);

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetBgMessage>()
            .add_message::<ShowFgMessage>()
            .add_message::<HideFgMessage>()
            .add_message::<ShowFaceMessage>()
            .add_message::<HideFaceMessage>()
            .add_message::<ShowCgMessage>()
            .add_message::<HideCgMessage>()
            .add_message::<ScrollBgMessage>()
            .add_message::<DrawSpriteMessage>()
            .add_message::<FadeSpriteMessage>()
            .add_message::<MoveSpriteMessage>()
            .add_message::<AnimateSpriteMessage>()
            .init_resource::<BgState>()
            .init_resource::<SpriteManager>()
            .init_resource::<CgState>()
            .init_resource::<TextureCache>()
            .init_resource::<SpriteOverlayManager>()
            .init_resource::<RenderingInitialized>()
            .init_resource::<QuakeState>()
            .init_resource::<PendingTextures>()
            .add_systems(OnEnter(AppState::Gameplay), setup_rendering)
            .add_systems(OnExit(AppState::Gameplay), cleanup_rendering)
            .add_systems(Update, (
                process_pending_textures,
                update_bg_fade,
                update_fg_fade,
                update_cg_fade,
                handle_set_bg,
                handle_show_fg,
                handle_hide_fg,
            ).chain().in_set(RenderChain1).after(SceneRestoreSet).run_if(in_state(AppState::Gameplay)))
            .add_systems(Update, (
                handle_show_face,
                handle_hide_face,
                handle_show_cg,
                handle_hide_cg,
                handle_draw_sprite,
                handle_fade_sprite,
                handle_move_sprite,
                update_sprite_tweens,
                center_sprite_overlays,
                update_overlay_tween,
                quake_update,
                update_sprite_shake,
                update_bg_scroll,
                handle_scroll_bg,
                handle_animate_sprite,
                advance_animated_sprites,
                process_pending_textures,
            ).chain().after(RenderChain1).after(SceneRestoreSet).run_if(in_state(AppState::Gameplay)));
    }
}

fn setup_rendering(
    mut commands: Commands,
    mut bg_state: ResMut<BgState>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut initialized: ResMut<RenderingInitialized>,
) {
    if initialized.0 {
        return;
    }

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
            BackgroundColor(Color::NONE),
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

    commands.spawn((
        ScreenOverlayRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ZIndex((u16::MAX - 1) as i32),
        Visibility::Hidden,
    ));

    initialized.0 = true;
}

fn cleanup_rendering(
    mut commands: Commands,
    query: Query<Entity, Or<(With<BackgroundRoot>, With<SpriteSlotMarker>, With<CgRoot>, With<SpriteOverlay>, With<ScreenOverlayRoot>)>>,
    mut bg_state: ResMut<BgState>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut cg_state: ResMut<CgState>,
    mut overlay_mgr: ResMut<SpriteOverlayManager>,
    mut cache: ResMut<TextureCache>,
    mut pending: ResMut<PendingTextures>,
    mut initialized: ResMut<RenderingInitialized>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    *bg_state = BgState::default();
    *sprite_mgr = SpriteManager::default();
    *cg_state = CgState::default();
    overlay_mgr.sprites.clear();
    cache.cache.clear();
    *pending = PendingTextures::default();
    initialized.0 = false;
}

fn handle_set_bg(
    mut msg: MessageReader<SetBgMessage>,
    mut bg_state: ResMut<BgState>,
    mut cg_state: ResMut<CgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut pending: ResMut<PendingTextures>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor, &mut Node)>,
    mut commands: Commands,
    obj_index: Res<ObjFileIndex>,
) {
    for msg in msg.read() {
        // Auto-cleanup CG if active (CG covers bg, so changing bg needs CG gone)
        if cg_state.active {
            if let Some(cg_entity) = cg_state.entity.take() {
                commands.entity(cg_entity).despawn();
            }
            cg_state.active = false;
            cg_state.texture = None;
            cg_state.fade = None;
            cg_state.current_file = None;
        }

        // Cancel any pending BG
        pending.bg = None;

        // Complete any in-progress fade instantly
        if bg_state.fade.is_some() {
            let old_active = bg_state.active_idx;
            if let Ok((mut old_img, mut vis, _, _)) = query.get_mut(bg_state.entities[old_active]) {
                *vis = Visibility::Hidden;
                old_img.color.set_alpha(1.0);
            }
            bg_state.active_idx = 1 - bg_state.active_idx;
            bg_state.fade = None;
            // Reset the (now active) buffer that was fading in to full alpha
            if let Ok((mut new_img, _, _, _)) = query.get_mut(bg_state.entities[bg_state.active_idx]) {
                new_img.color.set_alpha(1.0);
            }
        }

        let file = if msg.file.contains('.') { msg.file.clone() } else { format!("{}.jpg", msg.file) };
        let path = format!("image/bg/{}", file);
        let stem = msg.file.trim_end_matches(".png").trim_end_matches(".jpg");
        let resolved = obj_index.0.get(stem).cloned().unwrap_or(path);
        bg_state.current_bg = Some(stem.to_string());
        let handle = cache.cache.entry(resolved.clone()).or_insert_with(|| {
            asset_server.load(&resolved)
        }).clone();

        for &entity in &bg_state.entities {
            commands.entity(entity).remove::<BgScroll>();
        }

        let inactive_idx = 1 - bg_state.active_idx;
        let inactive_entity = bg_state.entities[inactive_idx];

        if let Ok((mut image_node, mut vis, _bg, mut node)) = query.get_mut(inactive_entity) {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
            node.left = Val::Px(0.0);
            node.top = Val::Px(0.0);

            if images.contains(&handle) {
                image_node.image = handle.clone();
                match msg.transition {
                    Some(Transition::Fade) => {
                        let dur = msg.duration.unwrap_or(0.5) as f32;
                        image_node.color.set_alpha(0.0);
                        *vis = Visibility::Visible;
                        bg_state.fade = Some(BgCrossFade {
                            timer: Timer::from_seconds(dur, TimerMode::Once),
                        });
                    }
                    _ => {
                        image_node.color.set_alpha(1.0);
                        *vis = Visibility::Visible;
                        if let Ok((mut old_img, mut old_vis, _, _)) = query.get_mut(bg_state.entities[bg_state.active_idx]) {
                            *old_vis = Visibility::Hidden;
                            old_img.color.set_alpha(1.0);
                        }
                        bg_state.active_idx = inactive_idx;
                        bg_state.fade = None;
                    }
                }
            } else {
                image_node.image = handle.clone();
                pending.bg = Some(PendingBg {
                    handle,
                    entity: inactive_entity,
                    target_idx: inactive_idx,
                    transition: msg.transition.clone(),
                    duration: msg.duration,
                    frames_waited: 0,
                });
            }
        }
    }
}

fn handle_scroll_bg(
    mut msg: MessageReader<ScrollBgMessage>,
    mut bg_state: ResMut<BgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Node, &mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
    obj_index: Res<ObjFileIndex>,
) {
    for msg in msg.read() {
        let file = if msg.file.contains('.') { msg.file.clone() } else { format!("{}.jpg", msg.file) };
        let path = format!("image/bg/{}", file);
        let stem = msg.file.trim_end_matches(".png").trim_end_matches(".jpg");
        let resolved = obj_index.0.get(stem).cloned().unwrap_or(path);
        let handle = cache.cache.entry(resolved.clone()).or_insert_with(|| {
            asset_server.load(&resolved)
        }).clone();

        for &entity in &bg_state.entities {
            commands.entity(entity).remove::<BgScroll>();
        }

        let active_idx = bg_state.active_idx;
        let active_entity = bg_state.entities[active_idx];

        if let Ok((entity, mut node, mut image_node, mut vis, _bg)) = query.get_mut(active_entity) {
            image_node.image = handle.clone();
            image_node.color.set_alpha(1.0);
            *vis = Visibility::Visible;

            if let Some(image) = images.get(&handle) {
                let w = image.texture_descriptor.size.width as f32;
                let h = image.texture_descriptor.size.height as f32;
                if w > 0.0 && h > 0.0 {
                    node.width = Val::Px(w);
                    node.height = Val::Px(h);
                }
            }

            node.left = Val::Px(msg.x1);
            node.top = Val::Px(msg.y1);

            if (msg.x1 - msg.x2).abs() > 0.5 || (msg.y1 - msg.y2).abs() > 0.5 {
                let dur = (msg.fade as f32 / 1000.0).max(0.016);
                commands.entity(entity).insert(BgScroll {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    start_x: msg.x1,
                    end_x: msg.x2,
                    start_y: msg.y1,
                    end_y: msg.y2,
                });
            }

            if bg_state.fade.is_some() {
                if let Ok((_, _, _, mut old_vis, _)) = query.get_mut(bg_state.entities[1 - active_idx]) {
                    *old_vis = Visibility::Hidden;
                }
                bg_state.active_idx = 1 - bg_state.active_idx;
                bg_state.fade = None;
            }
        }
    }
}

fn update_bg_scroll(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Node, &mut BgScroll)>,
    mut commands: Commands,
) {
    for (entity, mut node, mut scroll) in &mut query {
        scroll.timer.tick(time.delta());
        let t = scroll.timer.fraction();
        let eased = 1.0 - (1.0 - t) * (1.0 - t);

        node.left = Val::Px(scroll.start_x + (scroll.end_x - scroll.start_x) * eased);
        node.top = Val::Px(scroll.start_y + (scroll.end_y - scroll.start_y) * eased);

        if scroll.timer.just_finished() {
            node.left = Val::Px(scroll.end_x);
            node.top = Val::Px(scroll.end_y);
            commands.entity(entity).remove::<BgScroll>();
        }
    }
}

fn handle_show_fg(
    mut msg: MessageReader<ShowFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut pending: ResMut<PendingTextures>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        let slot = sprite_mgr.slots.get_mut(&msg.position);
        if let Some(slot) = slot {
            let Some(dir) = char_dir(&msg.char_id) else {
                warn!("No FG mapping for char_id: {}", msg.char_id);
                continue;
            };
            let path = resolve_fg_path(&msg.char_id, dir);
            let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
                asset_server.load(&path)
            }).clone();

            slot.char_id = msg.char_id.clone();
            slot.expression = msg.expression.clone();
            slot.texture = Some(handle.clone());

            if let Ok((mut image_node, mut vis, _)) = query.get_mut(slot.entity) {
                if images.contains(&handle) {
                    image_node.image = handle.clone();
                    match msg.transition {
                        Some(Transition::Fade) => {
                            let dur = msg.duration.unwrap_or(0.5) as f32;
                            image_node.color.set_alpha(0.0);
                            *vis = Visibility::Visible;
                            slot.fade = Some(SpriteFade {
                                timer: Timer::from_seconds(dur, TimerMode::Once),
                                kind: SpriteFadeKind::FadeIn,
                            });
                        }
                        _ => {
                            image_node.color.set_alpha(1.0);
                            *vis = Visibility::Visible;
                            slot.fade = None;
                        }
                    }
                } else {
                    pending.fg.insert(msg.position.clone(), PendingFg {
                        handle,
                        entity: slot.entity,
                        transition: msg.transition.clone(),
                        duration: msg.duration,
                        frames_waited: 0,
                    });
                }
            }
        } else {
            warn!("No sprite slot for position: {:?}", msg.position);
        }
    }
}

fn hide_slot(
    slot: &mut crate::resources::SpriteSlotInfo,
    query: &mut Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
    transition: Option<Transition>,
    duration: Option<f64>,
) {
    match transition {
        Some(Transition::Fade) => {
            let dur = duration.unwrap_or(0.5) as f32;
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

fn handle_hide_fg(
    mut msg: MessageReader<HideFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        if msg.char_id == "all" {
            for slot in sprite_mgr.slots.values_mut() {
                hide_slot(slot, &mut query, msg.transition.clone(), msg.duration);
            }
        } else if let Some(slot) = sprite_mgr.slots.values_mut()
            .find(|s| s.char_id == msg.char_id)
        {
            hide_slot(slot, &mut query, msg.transition.clone(), msg.duration);
        }
    }
}

fn handle_show_face(
    mut msg: MessageReader<ShowFaceMessage>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Visibility), With<FacePortrait>>,
) {
    for msg in msg.read() {
        let path = format!("image/face/face_{}.png", msg.char_id);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        if let Ok((mut image_node, mut vis)) = query.single_mut() {
            image_node.image = handle;
            *vis = Visibility::Visible;
        }
    }
}

fn handle_hide_face(
    mut msg: MessageReader<HideFaceMessage>,
    mut query: Query<(&mut ImageNode, &mut Visibility), With<FacePortrait>>,
) {
    for _ in msg.read() {
        if let Ok((mut image_node, mut vis)) = query.single_mut() {
            image_node.image = Handle::default();
            *vis = Visibility::Hidden;
        }
    }
}

fn handle_show_cg(
    mut msg: MessageReader<ShowCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut pending: ResMut<PendingTextures>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        pending.cg = None;

        let path = ev_file_path(&msg.file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        let entity = if let Some(entity) = cg_state.entity {
            entity
        } else {
            let e = commands.spawn((
                CgRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    ..default()
                },
                ImageNode {
                    image: Handle::default(),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                    ..default()
                },
                Visibility::Visible,
                ZIndex(2),
            )).id();
            cg_state.entity = Some(e);
            e
        };

        cg_state.active = true;
        cg_state.texture = Some(handle.clone());
        cg_state.current_file = Some(msg.file.clone());

        if images.contains(&handle) {
            if let Ok(mut entry) = commands.get_entity(entity) {
                let has_fade = matches!(msg.transition, Some(Transition::Fade));
                entry.insert(ImageNode {
                    image: handle,
                    color: Color::srgba(1.0, 1.0, 1.0, if has_fade { 0.0 } else { 1.0 }),
                    ..default()
                });
            }
            if matches!(msg.transition, Some(Transition::Fade)) {
                let dur = msg.duration.unwrap_or(0.5) as f32;
                cg_state.fade = Some(CgFade {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    kind: CgFadeKind::FadeIn,
                });
            }
        } else {
            pending.cg = Some(PendingCg {
                handle,
                entity,
                transition: msg.transition.clone(),
                duration: msg.duration,
                frames_waited: 0,
            });
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
                cg_state.current_file = None;
                cg_state.active = false;
                cg_state.texture = None;
            }
        }
    }
}

fn process_pending_textures(
    images: Res<Assets<Image>>,
    mut pending: ResMut<PendingTextures>,
    mut bg_state: ResMut<BgState>,
    mut cg_state: ResMut<CgState>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut bg_query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor, &mut Node), With<BackgroundRoot>>,
    mut sprite_query: Query<(&mut ImageNode, &mut Visibility), Without<BackgroundRoot>>,
) {
    // ── Process pending BG ──
    if let Some(p) = &mut pending.bg {
        p.frames_waited += 1;
        if images.contains(&p.handle) {
            if let Ok((mut image_node, mut vis, _bg, mut node)) = bg_query.get_mut(p.entity) {
                image_node.image = p.handle.clone();
                node.width = Val::Percent(100.0);
                node.height = Val::Percent(100.0);
                node.left = Val::Px(0.0);
                node.top = Val::Px(0.0);
                match p.transition {
                    Some(Transition::Fade) => {
                        let dur = p.duration.unwrap_or(0.5) as f32;
                        image_node.color.set_alpha(0.0);
                        *vis = Visibility::Visible;
                        bg_state.fade = Some(BgCrossFade {
                            timer: Timer::from_seconds(dur, TimerMode::Once),
                        });
                    }
                    _ => {
                        image_node.color.set_alpha(1.0);
                        *vis = Visibility::Visible;
                        let old_active = 1 - p.target_idx;
                        if let Ok((mut old_img, mut old_vis, _, _)) = bg_query.get_mut(bg_state.entities[old_active]) {
                            *old_vis = Visibility::Hidden;
                            old_img.color.set_alpha(1.0);
                        }
                        bg_state.active_idx = p.target_idx;
                        bg_state.fade = None;
                    }
                }
            }
            pending.bg = None;
        }
    }

    // ── Process pending FG + CG (both lack BackgroundRoot) ──
    pending.fg.retain(|_pos, p| {
        p.frames_waited += 1;
        if images.contains(&p.handle) {
            if let Ok((mut image_node, mut vis)) = sprite_query.get_mut(p.entity) {
                image_node.image = p.handle.clone();
                match p.transition {
                    Some(Transition::Fade) => {
                        let dur = p.duration.unwrap_or(0.5) as f32;
                        image_node.color.set_alpha(0.0);
                        *vis = Visibility::Visible;
                        if let Some(slot) = sprite_mgr.slots.values_mut().find(|s| s.entity == p.entity) {
                            slot.fade = Some(SpriteFade {
                                timer: Timer::from_seconds(dur, TimerMode::Once),
                                kind: SpriteFadeKind::FadeIn,
                            });
                        }
                    }
                    _ => {
                        image_node.color.set_alpha(1.0);
                        *vis = Visibility::Visible;
                    }
                }
            }
            false
        } else {
            true
        }
    });

    if let Some(p) = &mut pending.cg {
        p.frames_waited += 1;
        if images.contains(&p.handle) {
            if let Ok((mut image_node, _vis)) = sprite_query.get_mut(p.entity) {
                if matches!(p.transition, Some(Transition::Fade)) {
                    image_node.image = p.handle.clone();
                    image_node.color.set_alpha(0.0);
                    let dur = p.duration.unwrap_or(0.5) as f32;
                    cg_state.fade = Some(CgFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                        kind: CgFadeKind::FadeIn,
                    });
                } else {
                    image_node.image = p.handle.clone();
                    image_node.color.set_alpha(1.0);
                }
            }
            pending.cg = None;
        }
    }
}

fn update_bg_fade(
    time: Res<Time>,
    mut bg_state: ResMut<BgState>,
    mut query: Query<(&mut ImageNode, &mut Visibility)>,
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

        if let Ok((mut image, _)) = query.get_mut(active_entity) {
            image.color.set_alpha(1.0 - t);
        }
        if let Ok((mut image, _)) = query.get_mut(inactive_entity) {
            image.color.set_alpha(t);
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
    mut query: Query<(&mut ImageNode, &mut Visibility)>,
) {
    for (_position, slot) in sprite_mgr.slots.iter_mut() {
        let finished = {
            let fade = match &mut slot.fade {
                Some(f) => f,
                None => continue,
            };

            fade.timer.tick(time.delta());
            let t = fade.timer.fraction();

            if let Ok((mut image, _)) = query.get_mut(slot.entity) {
                let alpha = match fade.kind {
                    SpriteFadeKind::FadeIn => t,
                    SpriteFadeKind::FadeOut => 1.0 - t,
                };
                image.color.set_alpha(alpha);
            }

            let finished = fade.timer.just_finished();
            if finished && matches!(fade.kind, SpriteFadeKind::FadeOut) {
                if let Ok((_, mut vis)) = query.get_mut(slot.entity) {
                    *vis = Visibility::Hidden;
                }
                slot.char_id.clear();
                slot.expression.clear();
                slot.texture = None;
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
    mut commands: Commands,
    mut query: Query<(&mut ImageNode, &mut Visibility)>,
) {
    let Some(ref mut fade) = cg_state.fade else {
        return;
    };

    fade.timer.tick(time.delta());
    let t = fade.timer.fraction();
    let kind = fade.kind;
    let finished = fade.timer.just_finished();
    let entity = cg_state.entity;
    if let Some(entity) = entity {
        if let Ok((mut image, _)) = query.get_mut(entity) {
            let alpha = match kind {
                CgFadeKind::FadeIn => t,
                CgFadeKind::FadeOut => 1.0 - t,
            };
            image.color.set_alpha(alpha);
        }
    }

    if finished {
        if kind == CgFadeKind::FadeOut {
            if let Some(entity) = entity {
                commands.entity(entity).despawn();
            }
            cg_state.active = false;
            cg_state.texture = None;
            cg_state.entity = None;
            cg_state.current_file = None;
        } else {
            if let Some(entity) = entity {
                if let Ok((mut image, _)) = query.get_mut(entity) {
                    image.color.set_alpha(1.0);
                }
            }
        }
        cg_state.fade = None;
    }
}

fn sprite_depth_scale(z: i32) -> f32 {
    if z == 0 { 1.0 }
    else { 1.0 / (1.0 + z.abs() as f32 * 0.001) }
}

fn handle_draw_sprite(
    mut msg: MessageReader<DrawSpriteMessage>,
    mut overlay_mgr: ResMut<SpriteOverlayManager>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    obj_index: Res<ObjFileIndex>,
) {
    for msg in msg.read() {
        let stem = msg.file.trim_end_matches(".png").trim_end_matches(".jpg");
        let full_path = obj_index.0.get(stem).cloned().unwrap_or_else(|| {
            let path = if msg.file.contains('.') {
                msg.file.clone()
            } else {
                format!("{}.png", msg.file)
            };
            format!("image/obj/{}", path)
        });
        let handle = cache.cache.entry(full_path.clone()).or_insert_with(|| {
            asset_server.load(&full_path)
        }).clone();

        let alpha = (msg.alpha as f32 / 255.0).clamp(0.0, 1.0);
        let scale = sprite_depth_scale(msg.z);
        let rot_rad = msg.rotation.to_radians();
        let has_fade_in = msg.time > 0;

        if msg.file.contains("_tx") {
            for (_, entity) in overlay_mgr.sprites.drain() {
                commands.entity(entity).despawn();
            }
        }

        let is_tx = msg.file.contains("_tx");
        let (anchor_x, anchor_y, target_x, target_y, initial_left, initial_top) = if is_tx {
            (0.5_f32, 0.5_f32, 640.0_f32, 360.0_f32, 0.0_f32, 0.0_f32)
        } else {
            (msg.anchor_x, msg.anchor_y, msg.x, msg.y, msg.x, msg.y)
        };

        if let Some(&entity) = overlay_mgr.sprites.get(&msg.id) {
            if let Ok(mut entry) = commands.get_entity(entity) {
                entry.insert(ImageNode {
                    image: handle.clone(),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..default()
                });
                entry.insert(Transform::from_scale(Vec3::splat(scale)).with_rotation(Quat::from_rotation_z(rot_rad)));
                entry.insert(SpriteAnchor {
                    anchor_x,
                    anchor_y,
                    target_x,
                    target_y,
                });
            }
        } else {
            let blend = match msg.blend_mode {
                1 => SpriteBlendMode::Add,
                2 => SpriteBlendMode::Multiply,
                3 => SpriteBlendMode::Screen,
                _ => SpriteBlendMode::Normal,
            };
            let mut spawn = commands.spawn((
                SpriteOverlay { id: msg.id.clone(), blend_mode: blend },
                Node {
                    width: Val::Auto,
                    height: Val::Auto,
                    position_type: PositionType::Absolute,
                    left: Val::Px(initial_left),
                    top: Val::Px(initial_top),
                    ..default()
                },
                ImageNode {
                    image: handle.clone(),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..default()
                },
                SpriteAnchor {
                    anchor_x,
                    anchor_y,
                    target_x,
                    target_y,
                },
                Transform::from_scale(Vec3::splat(scale)).with_rotation(Quat::from_rotation_z(rot_rad)),
                Visibility::Visible,
                ZIndex((1 + msg.priority.max(0) as i32).min(2)),
            ));
            if has_fade_in {
                let dur = (msg.time as f32 / 1000.0).max(0.016);
                spawn.insert(SpriteTween {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    start_x: initial_left, end_x: initial_left,
                    start_y: initial_top, end_y: initial_top,
                    start_alpha: alpha, end_alpha: 1.0,
                    start_scale: scale, end_scale: scale,
                    kind: TweenKind::FadeIn,
                });
            }
            let entity = spawn.id();
            overlay_mgr.sprites.insert(msg.id.clone(), entity);
        }
    }
}

fn handle_fade_sprite(
    mut msg: MessageReader<FadeSpriteMessage>,
    overlay_mgr: Res<SpriteOverlayManager>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        if let Some(&entity) = overlay_mgr.sprites.get(&msg.id) {
            let dur = (msg.time as f32 / 1000.0).max(0.016);
            commands.entity(entity).insert(SpriteTween {
                timer: Timer::from_seconds(dur, TimerMode::Once),
                start_x: 0.0,
                end_x: 0.0,
                start_y: 0.0,
                end_y: 0.0,
                start_alpha: 1.0,
                end_alpha: 0.0,
                start_scale: 1.0,
                end_scale: 1.0,
                kind: TweenKind::FadeOut,
            });
        }
    }
}

fn handle_move_sprite(
    mut msg: MessageReader<MoveSpriteMessage>,
    overlay_mgr: Res<SpriteOverlayManager>,
    mut commands: Commands,
    query: Query<(&Node, &ImageNode, Option<&Transform>), With<SpriteOverlay>>,
) {
    for msg in msg.read() {
        if let Some(&entity) = overlay_mgr.sprites.get(&msg.id) {
            let dur = (msg.time as f32 / 1000.0).max(0.016);
            let (start_x, start_y, start_alpha, start_scale) = query.get(entity).map(|(node, image, tf)| {
                let x = match node.left { Val::Px(v) => v, _ => 0.0 };
                let y = match node.top { Val::Px(v) => v, _ => 0.0 };
                let a = image.color.alpha();
                let s = tf.and_then(|t| Some(t.scale.x)).unwrap_or(1.0);
                (x, y, a, s)
            }).unwrap_or((0.0, 0.0, 1.0, 1.0));
            let target_alpha = (msg.alpha as f32 / 255.0).clamp(0.0, 1.0);
            let end_scale = sprite_depth_scale(msg.z);
            commands.entity(entity).insert(SpriteTween {
                timer: Timer::from_seconds(dur, TimerMode::Once),
                start_x,
                end_x: msg.x,
                start_y,
                end_y: msg.y,
                start_alpha,
                end_alpha: target_alpha,
                start_scale,
                end_scale,
                kind: TweenKind::Move,
            });
        }
    }
}

fn update_sprite_tweens(
    time: Res<Time>,
    mut commands: Commands,
    mut overlay_mgr: ResMut<SpriteOverlayManager>,
    mut query: Query<(Entity, &mut SpriteTween, &mut Node, &mut ImageNode, &mut Transform, Option<&SpriteOverlay>)>,
) {
    for (entity, mut tween, mut node, mut image, mut tf, overlay) in &mut query {
        tween.timer.tick(time.delta());
        let t = tween.timer.fraction();
        let eased = 1.0 - (1.0 - t) * (1.0 - t); // ease-out quad

        node.left = Val::Px(tween.start_x + (tween.end_x - tween.start_x) * eased);
        node.top = Val::Px(tween.start_y + (tween.end_y - tween.start_y) * eased);
        let alpha = tween.start_alpha + (tween.end_alpha - tween.start_alpha) * eased;
        image.color.set_alpha(alpha);
        let s = tween.start_scale + (tween.end_scale - tween.start_scale) * eased;
        tf.scale = Vec3::splat(s);

        if tween.timer.just_finished() {
            match tween.kind {
                TweenKind::FadeOut => {
                    if let Some(overlay) = overlay {
                        overlay_mgr.sprites.remove(&overlay.id);
                    }
                    commands.entity(entity).despawn();
                }
                TweenKind::FadeIn | TweenKind::Move => {
                    node.left = Val::Px(tween.end_x);
                    node.top = Val::Px(tween.end_y);
                    image.color.set_alpha(tween.end_alpha);
                    tf.scale = Vec3::splat(tween.end_scale);
                    commands.entity(entity).remove::<SpriteTween>();
                }
            }
        }
    }
}

fn center_sprite_overlays(
    mut query: Query<(Entity, &mut Node, &SpriteAnchor, &ImageNode, Option<&mut SpriteTween>)>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
) {
    for (entity, mut node, anchor, image_node, tween) in &mut query {
        let Some(image) = images.get(&image_node.image) else { continue; };
        let w = image.texture_descriptor.size.width as f32;
        let h = image.texture_descriptor.size.height as f32;
        if w > 0.0 && h > 0.0 {
            let left = anchor.target_x - anchor.anchor_x * w;
            let top = anchor.target_y - anchor.anchor_y * h;
            node.left = Val::Px(left);
            node.top = Val::Px(top);
            if let Some(mut tween) = tween {
                tween.start_x = left;
                tween.end_x = left;
                tween.start_y = top;
                tween.end_y = top;
            }
            commands.entity(entity).remove::<SpriteAnchor>();
        }
    }
}

fn update_overlay_tween(
    time: Res<Time>,
    mut query: Query<(Entity, &mut BackgroundColor, &mut OverlayTween), With<ScreenOverlayRoot>>,
    mut commands: Commands,
) {
    for (entity, mut bg, mut tween) in query.iter_mut() {
        tween.timer.tick(time.delta());
        let progress = tween.timer.fraction().min(1.0);
        let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
        let alpha = tween.start_alpha + (tween.end_alpha - tween.start_alpha) * eased;
        let mut color = bg.0;
        color.set_alpha(alpha.clamp(0.0, 1.0));
        *bg = BackgroundColor(color);
        if tween.timer.just_finished() {
            if tween.end_alpha <= 0.0 {
                commands.entity(entity).insert(Visibility::Hidden);
            }
            commands.entity(entity).remove::<OverlayTween>();
        }
    }
}

fn handle_animate_sprite(
    mut msg: MessageReader<AnimateSpriteMessage>,
    mut overlay_mgr: ResMut<SpriteOverlayManager>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        if msg.max == 0 {
            continue;
        }
        if let Some(&entity) = overlay_mgr.sprites.get(&msg.id) {
            commands.entity(entity).despawn();
            overlay_mgr.sprites.remove(&msg.id);
        }

        let blend = match msg.draw {
            1 => SpriteBlendMode::Add,
            2 => SpriteBlendMode::Multiply,
            3 => SpriteBlendMode::Screen,
            _ => SpriteBlendMode::Normal,
        };

        let alpha = (msg.alpha as f32 / 255.0).clamp(0.0, 1.0);
        let scale = sprite_depth_scale(msg.z);
        let rot_rad = msg.rotation.to_radians();

        let mut frames = Vec::with_capacity(msg.max as usize);
        for i in 0..msg.max {
            let path = format!("image/anime/{}_{:02}.png", msg.file, i + 1);
            let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
                asset_server.load(&path)
            }).clone();
            frames.push(handle);
        }

        let frame_secs = (msg.frame_time as f32 / 1000.0).max(0.016);
        let timer = Timer::from_seconds(frame_secs, TimerMode::Repeating);

        let entity = commands.spawn((
            SpriteOverlay { id: msg.id.clone(), blend_mode: blend },
            Node {
                width: Val::Auto,
                height: Val::Auto,
                position_type: PositionType::Absolute,
                left: Val::Px(msg.x),
                top: Val::Px(msg.y),
                ..default()
            },
            ImageNode {
                image: frames[0].clone(),
                color: Color::srgba(1.0, 1.0, 1.0, alpha),
                ..default()
            },
            SpriteAnchor {
                anchor_x: msg.anchor_x,
                anchor_y: msg.anchor_y,
                target_x: msg.x,
                target_y: msg.y,
            },
            Transform::from_scale(Vec3::splat(scale)).with_rotation(Quat::from_rotation_z(rot_rad)),
            Visibility::Visible,
            ZIndex((1 + msg.priority.max(0) as i32).min(2)),
            AnimatedSprite {
                frames,
                current_frame: 0,
                timer,
                max_frames: msg.max as usize,
                finished: false,
            },
        )).id();
        overlay_mgr.sprites.insert(msg.id.clone(), entity);
    }
}

fn advance_animated_sprites(
    time: Res<Time>,
    mut query: Query<(&mut AnimatedSprite, &mut ImageNode)>,
) {
    for (mut anim, mut image) in query.iter_mut() {
        if anim.finished || anim.max_frames <= 1 {
            continue;
        }

        anim.timer.tick(time.delta());
        while anim.timer.just_finished() && !anim.finished {
            anim.current_frame += 1;
            if anim.current_frame >= anim.max_frames {
                anim.finished = true;
            } else {
                image.image = anim.frames[anim.current_frame].clone();
            }
        }
    }
}

fn quake_update(
    time: Res<Time>,
    mut quake: ResMut<QuakeState>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let Some(ref mut timer) = quake.timer else {
        return;
    };
    timer.tick(time.delta());
    let progress = timer.fraction();
    let decay = 1.0 - progress;
    let intensity = quake.intensity * decay;

    if let Ok(mut transform) = camera_query.single_mut() {
        if intensity > 0.5 {
            let offset_x = (rand::random::<f32>() - 0.5) * 2.0 * intensity;
            let offset_y = (rand::random::<f32>() - 0.5) * 2.0 * intensity;
            transform.translation.x = offset_x;
            transform.translation.y = offset_y;
        } else {
            transform.translation.x = 0.0;
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
            quake.timer = None;
        }
    }
}

fn update_sprite_shake(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpriteShake, &mut Node)>,
) {
    for (entity, mut shake, mut node) in query.iter_mut() {
        if !shake.initialized {
            shake.base_x = match node.left { Val::Px(v) => v, _ => 0.0 };
            shake.base_y = match node.top { Val::Px(v) => v, _ => 0.0 };
            shake.initialized = true;
        }

        shake.timer.tick(time.delta());
        let decay = 1.0 - shake.timer.fraction();
        let intensity = shake.intensity * decay;

        if intensity > 0.5 {
            let offset_x = (rand::random::<f32>() - 0.5) * 2.0 * intensity;
            let offset_y = (rand::random::<f32>() - 0.5) * 2.0 * intensity;
            node.left = Val::Px(shake.base_x + offset_x);
            node.top = Val::Px(shake.base_y + offset_y);
        } else {
            node.left = Val::Px(shake.base_x);
            node.top = Val::Px(shake.base_y);
            commands.entity(entity).remove::<SpriteShake>();
        }
    }
}
