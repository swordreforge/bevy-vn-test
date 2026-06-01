use crate::audio_messages::{PlayBgmMessage, StopBgmMessage};
use crate::components::*;
use crate::resources::{
    load_unlock_state, save_unlock_state, AllCgFiles, GalleryMode, GalleryState, GameFont, SafeMode,
    TextureCache, UnlockState,
};
use std::collections::HashMap;

#[derive(serde::Deserialize)]
struct BgmEntry {
    id: String,
    title: String,
}

include!(concat!(env!("OUT_DIR"), "/game_data.rs"));
use crate::state::AppState;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct DebugUnlockAll(pub bool);

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load_unlock_state())
            .init_resource::<GalleryState>()
            .init_resource::<TextureCache>()
            .init_resource::<AllCgFiles>()
            .init_resource::<SafeMode>()
            .init_resource::<DebugUnlockAll>()
            .add_systems(OnEnter(AppState::Gallery), setup_gallery)
            .add_systems(
                Update,
                detect_tap_unlock_all
                    .before(handle_thumbnail_click)
                    .before(handle_bgm_card_click)
                    .run_if(in_state(AppState::Gallery)),
            )
            .add_systems(
                Update,
                (
                    handle_thumbnail_click,
                    handle_back_button,
                    handle_fullscreen_click,
                    handle_gallery_escape,
                    handle_gallery_page_nav,
                    handle_debug_unlock_all,
                    handle_safe_mode_toggle,
                )
                    .run_if(in_state(AppState::Gallery)),
            )
            .add_systems(
                Update,
                (
                    handle_mode_toggle,
                    sync_gallery_mode_ui.after(handle_mode_toggle),
                    handle_clear_mode_toggle,
                    handle_bgm_card_click,
                )
                    .run_if(in_state(AppState::Gallery)),
            )
            .add_systems(OnExit(AppState::Gallery), cleanup_gallery);
    }
}

#[derive(Component)]
struct GalleryScreen;

#[derive(Component)]
struct GalleryPageText;

#[derive(Component)]
struct GalleryPageLeftBtn;

#[derive(Component)]
struct GalleryPageRightBtn;

#[derive(Component)]
struct GalleryTitleText;

#[derive(Component)]
struct GalleryModeBtn(GalleryMode);

#[derive(Component)]
struct GalleryClearModeBtn;

#[derive(Component)]
struct ClearModeLabel;



const CGS_PER_PAGE: usize = 9;
const BGMS_PER_PAGE: usize = 12;

fn load_bgm_title_map() -> HashMap<String, String> {
    let content = include_str!("../../assets/scripts/bgm_index.ron");
    ron::from_str::<Vec<BgmEntry>>(content)
        .ok()
        .map(|v| v.into_iter().map(|e| (e.id, e.title)).collect())
        .unwrap_or_default()
}

fn filtered_cg_files<'a>(cg_files: &'a [String], safe_mode: bool) -> Vec<&'a String> {
    if safe_mode {
        cg_files.iter().filter(|f| !f.starts_with("hcg")).collect()
    } else {
        cg_files.iter().collect()
    }
}

fn cg_total_pages(filtered_count: usize) -> usize {
    (filtered_count + CGS_PER_PAGE - 1) / CGS_PER_PAGE
}

fn bgm_total_pages(clear_mode: bool) -> usize {
    let ids = filtered_bgm_ids(clear_mode);
    (ids.len() + BGMS_PER_PAGE - 1) / BGMS_PER_PAGE
}

fn filtered_bgm_ids(clear_mode: bool) -> Vec<&'static str> {
    let ids = all_bgm_ids();
    if !clear_mode {
        return ids;
    }
    let title_map = load_bgm_title_map();
    ids.into_iter().filter(|id| title_map.contains_key(*id)).collect()
}

fn populate_cg_grid(
    grid: &mut ChildSpawnerCommands,
    page: usize,
    cg_files: &[String],
    unlock_state: &UnlockState,
    debug_all_unlocked: bool,
    safe_mode: bool,
    asset_server: &AssetServer,
    cache: &mut TextureCache,
    game_font: &GameFont,
) {
    let filtered = filtered_cg_files(cg_files, safe_mode);
    let start = page * CGS_PER_PAGE;
    let end = (start + CGS_PER_PAGE).min(filtered.len());
    for i in start..end {
        let file = filtered[i];
        if debug_all_unlocked || unlock_state.cg_unlocked.contains(file.as_str()) {
            let path = ev_file_path(file);
            let handle = cache
                .cache
                .entry(path.clone())
                .or_insert_with(|| asset_server.load(&path))
                .clone();
            grid.spawn((
                GalleryThumbnail(file.clone()),
                Button,
                Node {
                    width: Val::Px(360.0),
                    height: Val::Px(200.0),
                    ..default()
                },
                ImageNode::new(handle),
                ZIndex(5),
            ));
        } else {
            grid.spawn((
                GalleryThumbnail(file.clone()),
                GalleryLocked,
                Node {
                    width: Val::Px(360.0),
                    height: Val::Px(200.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 1.0)),
                ZIndex(5),
            ))
            .with_child((
                Text::new("[ LOCKED ]"),
                TextFont {
                    font: game_font.0.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.3, 0.3, 0.4)),
            ));
        }
    }
}

fn populate_bgm_grid(
    grid: &mut ChildSpawnerCommands,
    page: usize,
    unlock_state: &UnlockState,
    debug_all_unlocked: bool,
    game_font: &GameFont,
    playing_bgm: &Option<String>,
    clear_mode: bool,
) {
    let bgm_ids = filtered_bgm_ids(clear_mode);
    let start = page * BGMS_PER_PAGE;
    let end = (start + BGMS_PER_PAGE).min(bgm_ids.len());
    let title_map = load_bgm_title_map();
    for i in start..end {
        let id = bgm_ids[i];
        let unlocked = debug_all_unlocked || unlock_state.bgm_unlocked.contains(id);
        let playing = playing_bgm.as_deref() == Some(id);
        let title = title_map.get(id).map(|s| s.as_str()).unwrap_or(id);

        grid.spawn((
            BgmCard(id.to_string()),
            Button,
            Node {
                width: Val::Px(280.0),
                height: Val::Px(70.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(if playing {
                Color::srgba(0.3, 0.5, 0.3, 0.9)
            } else if unlocked {
                Color::srgba(0.2, 0.2, 0.3, 0.8)
            } else {
                Color::srgba(0.12, 0.12, 0.16, 0.8)
            }),
            ZIndex(5),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(if unlocked { title } else { "[ LOCKED ]" }),
                TextFont {
                    font: game_font.0.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(if !unlocked {
                    Color::srgb(0.3, 0.3, 0.4)
                } else if playing {
                    Color::srgb(0.6, 1.0, 0.6)
                } else {
                    Color::srgb(0.9, 0.9, 0.95)
                }),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
            card.spawn((
                Text::new(if playing { "■" } else { "▶" }),
                TextFont {
                    font: game_font.0.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(if unlocked {
                    Color::srgb(0.6, 0.8, 0.6)
                } else {
                    Color::srgb(0.3, 0.3, 0.4)
                }),
            ));
        });
    }
}

fn setup_gallery(
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    gallery_state: Res<GalleryState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    game_font: Res<GameFont>,
    cg_files: Res<AllCgFiles>,
    safe_mode: Res<SafeMode>,
    debug_all: Res<DebugUnlockAll>,
) {
    let filtered_count = filtered_cg_files(&cg_files.0, safe_mode.0).len();
    let cg_total = cg_total_pages(filtered_count);
    let bgm_total = bgm_total_pages(gallery_state.clear_mode);
    let is_cg = gallery_state.mode == GalleryMode::Cg;
    let current_page = if is_cg {
        gallery_state.cg_page
    } else {
        gallery_state.bgm_page
    };
    let total_pages = if is_cg { cg_total } else { bgm_total };

    commands
        .spawn((
            GalleryRoot,
            GalleryScreen,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
            ZIndex(5),
        ))
        .with_children(|root| {
            root.spawn((
                GalleryBackButton,
                Button,
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(36.0),
                    position_type: PositionType::Absolute,
                    top: Val::Px(8.0),
                    left: Val::Px(8.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
            ))
            .with_child((
                Text::new("← Back"),
                TextFont {
                    font: game_font.0.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            root.spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(24.0),
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },))
                .with_children(|title_row| {
                    title_row.spawn((
                        GalleryTitleText,
                        Text::new(if is_cg { "CG Gallery" } else { "BGM Gallery" }),
                        TextFont {
                            font: game_font.0.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    title_row.spawn((
                        GalleryModeToggle,
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(0.0),
                            ..default()
                        },
                    ))
                    .with_children(|toggle| {
                        for &(mode, label) in
                            &[(GalleryMode::Cg, "CG"), (GalleryMode::Bgm, "BGM")]
                        {
                            let active = gallery_state.mode == mode;
                            toggle.spawn((
                                GalleryModeBtn(mode),
                                Button,
                                Text::new(label),
                                TextFont {
                                    font: game_font.0.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(if active {
                                    Color::WHITE
                                } else {
                                    Color::srgb(0.5, 0.5, 0.6)
                                }),
                                Node {
                                    width: Val::Px(50.0),
                                    height: Val::Px(28.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(if active {
                                    Color::srgba(0.3, 0.5, 0.3, 0.9)
                                } else {
                                    Color::srgba(0.15, 0.15, 0.2, 0.8)
                                }),
                            ));
                        }
                    });
                });

            root.spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(16.0),
                margin: UiRect::vertical(Val::Px(6.0)),
                ..default()
            },))
                .with_children(|nav| {
                    nav.spawn((
                        GalleryPageLeftBtn,
                        Button,
                        Node {
                            width: Val::Px(36.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                    ))
                    .with_child((
                        Text::new("◀"),
                        TextFont {
                            font: game_font.0.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    nav.spawn((
                        GalleryPageText,
                        Text::new(format!(
                            "Page {}/{}",
                            current_page + 1,
                            total_pages.max(1)
                        )),
                        TextFont {
                            font: game_font.0.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.8)),
                    ));

                    nav.spawn((
                        GalleryPageRightBtn,
                        Button,
                        Node {
                            width: Val::Px(36.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                    ))
                    .with_child((
                        Text::new("▶"),
                        TextFont {
                            font: game_font.0.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            root.spawn((
                GalleryModeUi,
                Node {
                    display: if is_cg { Display::Flex } else { Display::None },
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ))
                .with_children(|row| {
                    row.spawn((
                        GallerySafeModeBtn,
                        Button,
                        Node {
                            width: Val::Px(140.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                    ))
                    .with_child((
                        Text::new(if safe_mode.0 {
                            "[x] Safe Mode"
                        } else {
                            "[ ] Safe Mode"
                        }),
                        TextFont {
                            font: game_font.0.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.8)),
                        SafeModeLabel,
                    ));
                });

            root.spawn((
                GalleryModeUi,
                Node {
                    display: if is_cg { Display::None } else { Display::Flex },
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ))
                .with_children(|row| {
                    row.spawn((
                        GalleryClearModeBtn,
                        Button,
                        Node {
                            width: Val::Px(150.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                    ))
                    .with_child((
                        Text::new(if gallery_state.clear_mode {
                            "[x] Clear Mode"
                        } else {
                            "[ ] Clear Mode"
                        }),
                        TextFont {
                            font: game_font.0.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.8)),
                        ClearModeLabel,
                    ));
                });

            if is_cg {
                root.spawn((
                    GalleryCgGrid,
                    GalleryGridContent,
                    Node {
                        width: Val::Percent(90.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        align_content: AlignContent::FlexStart,
                        column_gap: Val::Px(12.0),
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                ))
                .with_children(|grid| {
                    populate_cg_grid(
                        grid,
                        gallery_state.cg_page,
                        &cg_files.0,
                        &*unlock_state,
                        debug_all.0,
                        safe_mode.0,
                        &*asset_server,
                        &mut *cache,
                        &*game_font,
                    );
                });
            } else {
                root.spawn((
                    GalleryBgmGrid,
                    GalleryGridContent,
                    Node {
                        width: Val::Percent(90.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        align_content: AlignContent::FlexStart,
                        column_gap: Val::Px(16.0),
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                ))
                .with_children(|grid| {
                    populate_bgm_grid(
                        grid,
                        gallery_state.bgm_page,
                        &*unlock_state,
                        debug_all.0,
                        &*game_font,
                        &gallery_state.playing_bgm,
                        gallery_state.clear_mode,
                    );
                });
            }
        });
}

fn handle_thumbnail_click(
    interaction_query: Query<
        (&Interaction, &GalleryThumbnail),
        (Changed<Interaction>, With<Button>),
    >,
    mut gallery_state: ResMut<GalleryState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    unlock_state: Res<UnlockState>,
) {
    for (interaction, thumbnail) in &interaction_query {
        if *interaction == Interaction::Pressed {
            if gallery_state.fullscreen.is_some() {
                return;
            }
            let file = &thumbnail.0;
            if unlock_state.cg_unlocked.contains(file) {
                gallery_state.fullscreen = Some(file.clone());
                let path = ev_file_path(file);
                let handle = cache
                    .cache
                    .entry(path.clone())
                    .or_insert_with(|| asset_server.load(&path))
                    .clone();
                commands.spawn((
                    GalleryFullscreen,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        ..default()
                    },
                    ImageNode::new(handle),
                    BackgroundColor(Color::BLACK),
                    Button,
                    ZIndex(6),
                ));
            }
        }
    }
}

fn handle_back_button(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<GalleryBackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    dialogue: Res<crate::resources::DialogueState>,
    mut stop_bgm: MessageWriter<StopBgmMessage>,
    mut gallery_state: ResMut<GalleryState>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            stop_bgm.write(StopBgmMessage {
                id: None,
                fade_out: None,
            });
            gallery_state.playing_bgm = None;
            let target = if dialogue.current_text.is_empty() {
                AppState::Title
            } else {
                AppState::Menu
            };
            next_state.set(target);
        }
    }
}

fn handle_fullscreen_click(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<GalleryFullscreen>)>,
    mut gallery_state: ResMut<GalleryState>,
    mut commands: Commands,
    fullscreen_query: Query<Entity, With<GalleryFullscreen>>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            for entity in &fullscreen_query {
                commands.entity(entity).despawn();
            }
            gallery_state.fullscreen = None;
        }
    }
}

fn repopulate_grid(
    mode: GalleryMode,
    bgm_page: usize,
    cg_page: usize,
    grid_query: &Query<Entity, With<GalleryGridContent>>,
    children_query: &Query<&Children, With<GalleryGridContent>>,
    page_text_query: &Query<Entity, With<GalleryPageText>>,
    title_text_query: &Query<Entity, With<GalleryTitleText>>,
    commands: &mut Commands,
    unlock_state: &UnlockState,
    debug_all: bool,
    safe_mode: &SafeMode,
    asset_server: &AssetServer,
    cache: &mut TextureCache,
    cg_files: &AllCgFiles,
    game_font: &GameFont,
    playing_bgm: &Option<String>,
    clear_mode: bool,
) {
    let is_cg = mode == GalleryMode::Cg;
    let current_page = if is_cg { cg_page } else { bgm_page };

    let total_pages = if is_cg {
        let filtered = filtered_cg_files(&cg_files.0, safe_mode.0);
        cg_total_pages(filtered.len())
    } else {
        bgm_total_pages(clear_mode)
    };

    for children in children_query {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    for entity in grid_query {
        commands.entity(entity).with_children(|grid| {
            if is_cg {
                populate_cg_grid(
                    grid,
                    cg_page,
                    &cg_files.0,
                    unlock_state,
                    debug_all,
                    safe_mode.0,
                    asset_server,
                    cache,
                    game_font,
                );
            } else {
                populate_bgm_grid(
                    grid,
                    bgm_page,
                    unlock_state,
                    debug_all,
                    game_font,
                    playing_bgm,
                    clear_mode,
                );
            }
        });
    }

    for entity in page_text_query {
        commands.entity(entity).insert(Text::new(format!(
            "Page {}/{}",
            current_page + 1,
            total_pages.max(1)
        )));
    }

    for entity in title_text_query {
        commands.entity(entity).insert(Text::new(if is_cg {
            "CG Gallery"
        } else {
            "BGM Gallery"
        }));
    }
}

fn handle_gallery_page_nav(
    keys: Res<ButtonInput<KeyCode>>,
    mut gallery_state: ResMut<GalleryState>,
    grid_query: Query<Entity, With<GalleryGridContent>>,
    children_query: Query<&Children, With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    title_text_query: Query<Entity, With<GalleryTitleText>>,
    left_btn_query: Query<&Interaction, (Changed<Interaction>, With<GalleryPageLeftBtn>)>,
    right_btn_query: Query<&Interaction, (Changed<Interaction>, With<GalleryPageRightBtn>)>,
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    cg_files: Res<AllCgFiles>,
    game_font: Res<GameFont>,
    safe_mode: Res<SafeMode>,
    debug_all: Res<DebugUnlockAll>,
) {
    if gallery_state.fullscreen.is_some() {
        return;
    }

    let is_cg = gallery_state.mode == GalleryMode::Cg;
    let old_page = if is_cg {
        gallery_state.cg_page
    } else {
        gallery_state.bgm_page
    };

    let total_pages = if is_cg {
        let filtered = filtered_cg_files(&cg_files.0, safe_mode.0);
        cg_total_pages(filtered.len())
    } else {
        bgm_total_pages(gallery_state.clear_mode)
    };

    let mut new_page = old_page;

    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::ArrowUp) {
        new_page = if old_page == 0 {
            total_pages.saturating_sub(1)
        } else {
            old_page - 1
        };
    }
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::ArrowDown) {
        new_page = (old_page + 1) % total_pages.max(1);
    }

    for interaction in &left_btn_query {
        if *interaction == Interaction::Pressed {
            new_page = if old_page == 0 {
                total_pages.saturating_sub(1)
            } else {
                old_page - 1
            };
        }
    }
    for interaction in &right_btn_query {
        if *interaction == Interaction::Pressed {
            new_page = (old_page + 1) % total_pages.max(1);
        }
    }

    if new_page != old_page {
        if is_cg {
            gallery_state.cg_page = new_page;
        } else {
            gallery_state.bgm_page = new_page;
        }

        repopulate_grid(
            gallery_state.mode,
            gallery_state.bgm_page,
            gallery_state.cg_page,
            &grid_query,
            &children_query,
            &page_text_query,
            &title_text_query,
            &mut commands,
            &*unlock_state,
            debug_all.0,
            &*safe_mode,
            &*asset_server,
            &mut *cache,
            &*cg_files,
            &*game_font,
            &gallery_state.playing_bgm,
            gallery_state.clear_mode,
        );
    }
}

fn handle_gallery_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut gallery_state: ResMut<GalleryState>,
    mut commands: Commands,
    fullscreen_query: Query<Entity, With<GalleryFullscreen>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut stop_bgm: MessageWriter<StopBgmMessage>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if gallery_state.fullscreen.is_some() {
            for entity in &fullscreen_query {
                commands.entity(entity).despawn();
            }
            gallery_state.fullscreen = None;
        } else {
            stop_bgm.write(StopBgmMessage {
                id: None,
                fade_out: None,
            });
            gallery_state.playing_bgm = None;
            next_state.set(AppState::Menu);
        }
    }
}

fn detect_tap_unlock_all(
    cg_query: Query<&Interaction, (Changed<Interaction>, With<GalleryThumbnail>, With<Button>)>,
    bgm_query: Query<&Interaction, (Changed<Interaction>, With<BgmCard>, With<Button>)>,
    mut debug_all: ResMut<DebugUnlockAll>,
    mut tap_times: Local<Vec<f64>>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let tapped = cg_query.iter().any(|i| *i == Interaction::Pressed)
        || bgm_query.iter().any(|i| *i == Interaction::Pressed);
    if !tapped {
        return;
    }
    tap_times.push(now);
    tap_times.retain(|t| now - *t <= 1.5);
    if tap_times.len() >= 7 {
        debug_all.0 = !debug_all.0;
        info!("Tap 7x unlock all: {}", debug_all.0);
        tap_times.clear();
    }
}

fn handle_debug_unlock_all(
    keys: Res<ButtonInput<KeyCode>>,
    mut debug_all: ResMut<DebugUnlockAll>,
    mut gallery_state: ResMut<GalleryState>,
    grid_query: Query<Entity, With<GalleryGridContent>>,
    children_query: Query<&Children, With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    title_text_query: Query<Entity, With<GalleryTitleText>>,
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    cg_files: Res<AllCgFiles>,
    game_font: Res<GameFont>,
    safe_mode: Res<SafeMode>,
) {
    if !keys.just_pressed(KeyCode::KeyU) {
        return;
    }

    debug_all.0 = !debug_all.0;
    info!("Debug unlock all: {}", debug_all.0);

    let is_cg = gallery_state.mode == GalleryMode::Cg;
    if is_cg {
        let filtered = filtered_cg_files(&cg_files.0, safe_mode.0);
        let total_pages = cg_total_pages(filtered.len());
        if gallery_state.cg_page >= total_pages && total_pages > 0 {
            gallery_state.cg_page = total_pages - 1;
        }
    } else {
        let total_pages = bgm_total_pages(gallery_state.clear_mode);
        if gallery_state.bgm_page >= total_pages && total_pages > 0 {
            gallery_state.bgm_page = total_pages - 1;
        }
    }

    repopulate_grid(
        gallery_state.mode,
        gallery_state.bgm_page,
        gallery_state.cg_page,
        &grid_query,
        &children_query,
        &page_text_query,
        &title_text_query,
        &mut commands,
        &*unlock_state,
        debug_all.0,
        &*safe_mode,
        &*asset_server,
        &mut *cache,
        &*cg_files,
        &*game_font,
        &gallery_state.playing_bgm,
        gallery_state.clear_mode,
    );
}

fn handle_safe_mode_toggle(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<GallerySafeModeBtn>)>,
    mut safe_mode: ResMut<SafeMode>,
    mut gallery_state: ResMut<GalleryState>,
    grid_query: Query<Entity, With<GalleryGridContent>>,
    children_query: Query<&Children, With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    title_text_query: Query<Entity, With<GalleryTitleText>>,
    mut label_query: Query<&mut Text, With<SafeModeLabel>>,
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    cg_files: Res<AllCgFiles>,
    game_font: Res<GameFont>,
    debug_all: Res<DebugUnlockAll>,
) {
    let toggled = interaction_query.iter().any(|i| *i == Interaction::Pressed);
    if !toggled {
        return;
    }

    safe_mode.0 = !safe_mode.0;
    gallery_state.cg_page = 0;

    let filtered = filtered_cg_files(&cg_files.0, safe_mode.0);
    let total_pages = cg_total_pages(filtered.len());

    for children in &children_query {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    for entity in &grid_query {
        commands.entity(entity).with_children(|grid| {
            populate_cg_grid(
                grid,
                gallery_state.cg_page,
                &cg_files.0,
                &*unlock_state,
                debug_all.0,
                safe_mode.0,
                &*asset_server,
                &mut *cache,
                &*game_font,
            );
        });
    }

    for entity in &page_text_query {
        commands.entity(entity).insert(Text::new(format!(
            "Page {}/{}",
            gallery_state.cg_page + 1,
            total_pages.max(1)
        )));
    }

    for mut text in &mut label_query {
        text.0 = if safe_mode.0 {
            "[x] Safe Mode".to_string()
        } else {
            "[ ] Safe Mode".to_string()
        };
    }

    for entity in &title_text_query {
        commands.entity(entity).insert(Text::new("CG Gallery"));
    }
}

fn handle_mode_toggle(
    interaction_query: Query<(&GalleryModeBtn, &Interaction), Changed<Interaction>>,
    mut gallery_state: ResMut<GalleryState>,
    mut stop_bgm: MessageWriter<StopBgmMessage>,
) {
    for (btn_mode, interaction) in &interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if gallery_state.mode == btn_mode.0 {
            continue;
        }
        stop_bgm.write(StopBgmMessage {
            id: None,
            fade_out: None,
        });
        gallery_state.playing_bgm = None;
        gallery_state.mode = btn_mode.0;
    }
}

fn sync_gallery_mode_ui(
    mut prev_mode: Local<Option<GalleryMode>>,
    gallery_state: Res<GalleryState>,
    grid_query: Query<(Entity, &Children), With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    title_text_query: Query<Entity, With<GalleryTitleText>>,
    mode_btn_query: Query<(Entity, &GalleryModeBtn)>,
    safe_btn_query: Query<&ChildOf, With<GallerySafeModeBtn>>,
    clear_btn_query: Query<&ChildOf, With<GalleryClearModeBtn>>,
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    cg_files: Res<AllCgFiles>,
    game_font: Res<GameFont>,
    safe_mode: Res<SafeMode>,
    debug_all: Res<DebugUnlockAll>,
) {
    match *prev_mode {
        None => *prev_mode = Some(gallery_state.mode),
        Some(prev) if prev == gallery_state.mode => return,
        _ => {}
    }
    *prev_mode = Some(gallery_state.mode);

    let is_cg = gallery_state.mode == GalleryMode::Cg;
    let current_page = if is_cg {
        gallery_state.cg_page
    } else {
        gallery_state.bgm_page
    };
    let total_pages = if is_cg {
        let filtered = filtered_cg_files(&cg_files.0, safe_mode.0);
        cg_total_pages(filtered.len())
    } else {
        bgm_total_pages(gallery_state.clear_mode)
    };

    // Despawn children + repopulate grid
    for (_, children) in &grid_query {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
    for (entity, _) in &grid_query {
        commands.entity(entity).with_children(|grid| {
            if is_cg {
                populate_cg_grid(
                    grid,
                    gallery_state.cg_page,
                    &cg_files.0,
                    &*unlock_state,
                    debug_all.0,
                    safe_mode.0,
                    &*asset_server,
                    &mut *cache,
                    &*game_font,
                );
            } else {
                populate_bgm_grid(
                    grid,
                    gallery_state.bgm_page,
                    &*unlock_state,
                    debug_all.0,
                    &*game_font,
                    &gallery_state.playing_bgm,
                    gallery_state.clear_mode,
                );
            }
        });
    }

    // Update page text and title
    for entity in &page_text_query {
        commands.entity(entity).insert(Text::new(format!(
            "Page {}/{}",
            current_page + 1,
            total_pages.max(1)
        )));
    }
    for entity in &title_text_query {
        commands.entity(entity).insert(Text::new(if is_cg {
            "CG Gallery"
        } else {
            "BGM Gallery"
        }));
    }

    // Update toggle button visuals
    for (entity, mode) in &mode_btn_query {
        let active = gallery_state.mode == mode.0;
        commands.entity(entity).insert((
            BackgroundColor(if active {
                Color::srgba(0.3, 0.5, 0.3, 0.9)
            } else {
                Color::srgba(0.15, 0.15, 0.2, 0.8)
            }),
            TextColor(if active {
                Color::WHITE
            } else {
                Color::srgb(0.5, 0.5, 0.6)
            }),
        ));
    }

    // Toggle safe/clear mode UI visibility
    for parent in &safe_btn_query {
        commands.entity(parent.0).insert(Node {
            display: if is_cg { Display::Flex } else { Display::None },
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            margin: UiRect::bottom(Val::Px(4.0)),
            ..default()
        });
    }
    for parent in &clear_btn_query {
        commands.entity(parent.0).insert(Node {
            display: if is_cg { Display::None } else { Display::Flex },
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            margin: UiRect::bottom(Val::Px(4.0)),
            ..default()
        });
    }
}

fn handle_clear_mode_toggle(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<GalleryClearModeBtn>)>,
    mut gallery_state: ResMut<GalleryState>,
    grid_query: Query<Entity, With<GalleryGridContent>>,
    children_query: Query<&Children, With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    title_text_query: Query<Entity, With<GalleryTitleText>>,
    mut label_query: Query<&mut Text, With<ClearModeLabel>>,
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    game_font: Res<GameFont>,
    debug_all: Res<DebugUnlockAll>,
) {
    let toggled = interaction_query.iter().any(|i| *i == Interaction::Pressed);
    if !toggled {
        return;
    }

    gallery_state.clear_mode = !gallery_state.clear_mode;
    gallery_state.bgm_page = 0;

    let total_pages = bgm_total_pages(gallery_state.clear_mode);
    if gallery_state.bgm_page >= total_pages && total_pages > 0 {
        gallery_state.bgm_page = total_pages - 1;
    }

    for children in &children_query {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    for entity in &grid_query {
        commands.entity(entity).with_children(|grid| {
            populate_bgm_grid(
                grid,
                gallery_state.bgm_page,
                &*unlock_state,
                debug_all.0,
                &*game_font,
                &gallery_state.playing_bgm,
                gallery_state.clear_mode,
            );
        });
    }

    for entity in &page_text_query {
        commands.entity(entity).insert(Text::new(format!(
            "Page {}/{}",
            gallery_state.bgm_page + 1,
            total_pages.max(1)
        )));
    }

    for entity in &title_text_query {
        commands.entity(entity).insert(Text::new("BGM Gallery"));
    }

    for mut text in &mut label_query {
        text.0 = if gallery_state.clear_mode {
            "[x] Clear Mode".to_string()
        } else {
            "[ ] Clear Mode".to_string()
        };
    }
}

fn handle_bgm_card_click(
    interaction_query: Query<
        (&BgmCard, &Interaction),
        (Changed<Interaction>, With<Button>),
    >,
    mut gallery_state: ResMut<GalleryState>,
    unlock_state: Res<UnlockState>,
    debug_all: Res<DebugUnlockAll>,
    mut play_bgm: MessageWriter<PlayBgmMessage>,
    mut stop_bgm: MessageWriter<StopBgmMessage>,
    grid_query: Query<Entity, With<GalleryGridContent>>,
    children_query: Query<&Children, With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    title_text_query: Query<Entity, With<GalleryTitleText>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    cg_files: Res<AllCgFiles>,
    game_font: Res<GameFont>,
    safe_mode: Res<SafeMode>,
) {
    if gallery_state.fullscreen.is_some() {
        return;
    }

    for (card, interaction) in &interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let id = &card.0;
        let unlocked = debug_all.0 || unlock_state.bgm_unlocked.contains(id.as_str());
        if !unlocked {
            continue;
        }

        if gallery_state.playing_bgm.as_deref() == Some(id.as_str()) {
            stop_bgm.write(StopBgmMessage {
                id: Some(id.clone()),
                fade_out: None,
            });
            gallery_state.playing_bgm = None;
        } else {
            play_bgm.write(PlayBgmMessage {
                id: id.clone(),
                volume: None,
                fade_in: None,
            });
            gallery_state.playing_bgm = Some(id.clone());
        }

        repopulate_grid(
            gallery_state.mode,
            gallery_state.bgm_page,
            gallery_state.cg_page,
            &grid_query,
            &children_query,
            &page_text_query,
            &title_text_query,
            &mut commands,
            &*unlock_state,
            debug_all.0,
            &*safe_mode,
            &*asset_server,
            &mut *cache,
            &*cg_files,
            &*game_font,
            &gallery_state.playing_bgm,
            gallery_state.clear_mode,
        );
    }
}

fn cleanup_gallery(
    mut commands: Commands,
    query: Query<
        Entity,
        Or<(
            With<GalleryRoot>,
            With<GalleryFullscreen>,
            With<GalleryScreen>,
        )>,
    >,
    unlock_state: Res<UnlockState>,
    mut stop_bgm: MessageWriter<StopBgmMessage>,
    mut gallery_state: ResMut<GalleryState>,
) {
    stop_bgm.write(StopBgmMessage {
        id: None,
        fade_out: None,
    });
    gallery_state.playing_bgm = None;
    save_unlock_state(&unlock_state);
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
