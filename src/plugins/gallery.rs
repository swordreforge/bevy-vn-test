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
                update_gallery_scrollbar,
            ).run_if(in_state(AppState::Gallery)))
            .add_systems(OnExit(AppState::Gallery), cleanup_gallery);
    }
}

#[derive(Component)]
struct GalleryScreen;

fn setup_gallery(
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    _gallery_state: Res<GalleryState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    game_font: Res<GameFont>,
    cg_files: Res<AllCgFiles>,
) {
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
    )).with_children(|parent| {
        parent.spawn((
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

        parent.spawn((
            Text::new("CG Gallery"),
            TextFont { font: game_font.0.clone(), font_size: 28.0, ..default() },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
        ));

        parent.spawn((
            Node {
                width: Val::Percent(90.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                ..default()
            },
        )).with_children(|row| {
            row.spawn((
                GalleryScrollArea,
                Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            )).with_children(|scroll| {
                scroll.spawn((
                    GalleryGridContent,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Auto,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        column_gap: Val::Px(12.0),
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                )).with_children(|grid| {
                for file in &cg_files.0 {
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

            row.spawn((
                GalleryScrollbar,
                Node {
                    width: Val::Px(8.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.18, 1.0)),
            )).with_children(|track| {
                track.spawn((
                    GalleryScrollThumb,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.45, 0.45, 0.5, 1.0)),
                ));
            });
        });
    });
}

fn update_gallery_scrollbar(
    scroll_area: Query<(&ScrollPosition, &ComputedNode), With<GalleryScrollArea>>,
    grid_content: Query<&ComputedNode, (With<GalleryGridContent>, Without<GalleryScrollArea>)>,
    scrollbar: Query<&ComputedNode, (With<GalleryScrollbar>, Without<GalleryScrollThumb>)>,
    mut thumb: Query<(&mut Node, &mut BackgroundColor), With<GalleryScrollThumb>>,
) {
    let Ok((scroll_pos, container_node)) = scroll_area.single() else { return };
    let Ok(content_node) = grid_content.single() else { return };
    let Ok(track_node) = scrollbar.single() else { return };
    let Ok((mut thumb_node, _)) = thumb.single_mut() else { return };

    let visible_height = container_node.size().y;
    let content_height = content_node.size().y;

    if content_height <= visible_height || visible_height <= 0.0 {
        thumb_node.display = Display::None;
        return;
    }

    thumb_node.display = Display::Flex;
    let track_height = track_node.size().y;
    if track_height <= 0.0 {
        return;
    }

    let scrollable = content_height - visible_height;
    let ratio = visible_height / content_height;
    let thumb_height = (track_height * ratio).max(16.0);
    let max_top = (track_height - thumb_height).max(0.0);
    let thumb_top = max_top * (scroll_pos.0.y / scrollable);

    thumb_node.height = Val::Px(thumb_height);
    thumb_node.top = Val::Px(thumb_top);
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

fn cleanup_gallery(mut commands: Commands, query: Query<Entity, Or<(With<GalleryRoot>, With<GalleryFullscreen>, With<GalleryScreen>)>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
