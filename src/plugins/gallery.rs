use bevy::prelude::*;
use crate::state::AppState;
use crate::resources::{GameFont, GalleryState, TextureCache, UnlockState, AllCgFiles};
use crate::components::*;

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnlockState>()
            .init_resource::<GalleryState>()
            .init_resource::<TextureCache>()
            .init_resource::<AllCgFiles>()
            .add_systems(OnEnter(AppState::Gallery), setup_gallery)
            .add_systems(Update, (
                handle_thumbnail_click,
                handle_back_button,
                handle_fullscreen_click,
                handle_gallery_escape,
                handle_gallery_page_nav,
                handle_debug_unlock_all,
            ).run_if(in_state(AppState::Gallery)))
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

const CGS_PER_PAGE: usize = 9;

fn setup_gallery(
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    gallery_state: Res<GalleryState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    game_font: Res<GameFont>,
    cg_files: Res<AllCgFiles>,
) {
    let total_pages = ((cg_files.0.len() + CGS_PER_PAGE - 1) / CGS_PER_PAGE).max(1);

    commands.spawn((
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
    )).with_children(|root| {
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
        )).with_child((
            Text::new("← Back"),
            TextFont { font: game_font.0.clone(), font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));

        root.spawn((
            Text::new("CG Gallery"),
            TextFont { font: game_font.0.clone(), font_size: 28.0, ..default() },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
        ));

        root.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(16.0),
                margin: UiRect::vertical(Val::Px(6.0)),
                ..default()
            },
        )).with_children(|nav| {
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
            )).with_child((
                Text::new("◀"),
                TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
                TextColor(Color::WHITE),
            ));

            nav.spawn((
                GalleryPageText,
                Text::new(format!("Page {}/{}", gallery_state.page + 1, total_pages)),
                TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
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
            )).with_child((
                Text::new("▶"),
                TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });

        root.spawn((
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
        )).with_children(|grid| {
            let start = gallery_state.page * CGS_PER_PAGE;
            let end = (start + CGS_PER_PAGE).min(cg_files.0.len());
            for i in start..end {
                let file = &cg_files.0[i];
                if unlock_state.cg_unlocked.contains(file) {
                    let path = format!("images/ev/{}", file);
                    let handle = cache.cache.entry(path.clone())
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
                    )).with_child((
                        Text::new("[ LOCKED ]"),
                        TextFont { font: game_font.0.clone(), font_size: 16.0, ..default() },
                        TextColor(Color::srgb(0.3, 0.3, 0.4)),
                    ));
                }
            }
        });
    });
}

fn handle_thumbnail_click(
    interaction_query: Query<(&Interaction, &GalleryThumbnail), (Changed<Interaction>, With<Button>)>,
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
                let path = format!("images/ev/{}", file);
                let handle = cache.cache.entry(path.clone())
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
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
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

fn handle_gallery_page_nav(
    keys: Res<ButtonInput<KeyCode>>,
    mut gallery_state: ResMut<GalleryState>,
    grid_query: Query<Entity, With<GalleryGridContent>>,
    children_query: Query<&Children, With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    left_btn_query: Query<&Interaction, (Changed<Interaction>, With<GalleryPageLeftBtn>)>,
    right_btn_query: Query<&Interaction, (Changed<Interaction>, With<GalleryPageRightBtn>)>,
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    cg_files: Res<AllCgFiles>,
    game_font: Res<GameFont>,
) {
    if gallery_state.fullscreen.is_some() {
        return;
    }

    let total_pages = ((cg_files.0.len() + CGS_PER_PAGE - 1) / CGS_PER_PAGE).max(1);
    let old_page = gallery_state.page;

    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::ArrowUp) {
        gallery_state.page = if gallery_state.page == 0 {
            total_pages - 1
        } else {
            gallery_state.page - 1
        };
    }
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::ArrowDown) {
        gallery_state.page = (gallery_state.page + 1) % total_pages;
    }

    for interaction in &left_btn_query {
        if *interaction == Interaction::Pressed {
            gallery_state.page = if gallery_state.page == 0 {
                total_pages - 1
            } else {
                gallery_state.page - 1
            };
        }
    }
    for interaction in &right_btn_query {
        if *interaction == Interaction::Pressed {
            gallery_state.page = (gallery_state.page + 1) % total_pages;
        }
    }

    if gallery_state.page != old_page {
        for children in &children_query {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        for entity in &grid_query {
            commands.entity(entity).with_children(|grid| {
                let start = gallery_state.page * CGS_PER_PAGE;
                let end = (start + CGS_PER_PAGE).min(cg_files.0.len());
                for i in start..end {
                    let file = &cg_files.0[i];
                    if unlock_state.cg_unlocked.contains(file) {
                        let path = format!("images/ev/{}", file);
                        let handle = cache.cache.entry(path.clone())
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
                        )).with_child((
                            Text::new("[ LOCKED ]"),
                            TextFont { font: game_font.0.clone(), font_size: 16.0, ..default() },
                            TextColor(Color::srgb(0.3, 0.3, 0.4)),
                        ));
                    }
                }
            });
        }

        for entity in &page_text_query {
            commands.entity(entity).insert(Text::new(
                format!("Page {}/{}", gallery_state.page + 1, total_pages),
            ));
        }
    }
}

fn handle_gallery_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut gallery_state: ResMut<GalleryState>,
    mut commands: Commands,
    fullscreen_query: Query<Entity, With<GalleryFullscreen>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if gallery_state.fullscreen.is_some() {
            for entity in &fullscreen_query {
                commands.entity(entity).despawn();
            }
            gallery_state.fullscreen = None;
        } else {
            next_state.set(AppState::Menu);
        }
    }
}

fn handle_debug_unlock_all(
    keys: Res<ButtonInput<KeyCode>>,
    mut unlock_state: ResMut<UnlockState>,
    cg_files: Res<AllCgFiles>,
    gallery_state: Res<GalleryState>,
    grid_query: Query<Entity, With<GalleryGridContent>>,
    children_query: Query<&Children, With<GalleryGridContent>>,
    page_text_query: Query<Entity, With<GalleryPageText>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    game_font: Res<GameFont>,
) {
    if !keys.just_pressed(KeyCode::KeyU) {
        return;
    }

    for file in &cg_files.0 {
        unlock_state.cg_unlocked.insert(file.clone());
    }
    let new_count = unlock_state.cg_unlocked.len();

    for children in &children_query {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let total_pages = ((cg_files.0.len() + CGS_PER_PAGE - 1) / CGS_PER_PAGE).max(1);

    for entity in &grid_query {
        commands.entity(entity).with_children(|grid| {
            let start = gallery_state.page * CGS_PER_PAGE;
            let end = (start + CGS_PER_PAGE).min(cg_files.0.len());
            for i in start..end {
                let file = &cg_files.0[i];
                if unlock_state.cg_unlocked.contains(file) {
                    let path = format!("images/ev/{}", file);
                    let handle = cache.cache.entry(path.clone())
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
                    )).with_child((
                        Text::new("[ LOCKED ]"),
                        TextFont { font: game_font.0.clone(), font_size: 16.0, ..default() },
                        TextColor(Color::srgb(0.3, 0.3, 0.4)),
                    ));
                }
            }
        });
    }

    for entity in &page_text_query {
        commands.entity(entity).insert(Text::new(
            format!("Page {}/{} (Unlocked: {}/{})", gallery_state.page + 1, total_pages, new_count, cg_files.0.len()),
        ));
    }
}

fn cleanup_gallery(mut commands: Commands, query: Query<Entity, Or<(With<GalleryRoot>, With<GalleryFullscreen>, With<GalleryScreen>)>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
