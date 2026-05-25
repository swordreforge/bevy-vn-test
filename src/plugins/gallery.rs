use bevy::prelude::*;
use crate::state::AppState;
use crate::resources::{UnlockState, GalleryState, TextureCache};
use crate::components::*;

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnlockState>()
            .init_resource::<GalleryState>()
            .init_resource::<TextureCache>()
            .add_systems(OnEnter(AppState::Gallery), setup_gallery)
            .add_systems(Update, (
                handle_thumbnail_click,
                handle_back_button,
                handle_fullscreen_click,
                handle_gallery_escape,
            ).run_if(in_state(AppState::Gallery)))
            .add_systems(OnExit(AppState::Gallery), cleanup_gallery);
    }
}

const ALL_CG_FILES: &[&str] = &["eve_010101.png"];

#[derive(Component)]
struct GalleryScreen;

fn setup_gallery(
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    _gallery_state: Res<GalleryState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
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
            GalleryScreen,
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
            GalleryScreen,
            Text::new("← Back"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));

        parent.spawn((
            GalleryScreen,
            Text::new("CG Gallery"),
            TextFont { font_size: 28.0, ..default() },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
        ));

        parent.spawn((
            GalleryScreen,
            Node {
                width: Val::Percent(90.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(12.0),
                margin: UiRect::top(Val::Px(20.0)),
                ..default()
            },
        )).with_children(|grid| {
            for file in ALL_CG_FILES {
                if unlock_state.cg_unlocked.contains(*file) {
                    let path = format!("images/ev/{}", file);
                    let handle = cache.cache.entry(path.clone())
                        .or_insert_with(|| asset_server.load(&path))
                        .clone();
                    grid.spawn((
                        GalleryThumbnail(file.to_string()),
                        GalleryScreen,
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
                        GalleryThumbnail(file.to_string()),
                        GalleryLocked,
                        GalleryScreen,
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
                        GalleryScreen,
                        Text::new("🔒"),
                        TextFont { font_size: 32.0, ..default() },
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
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
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
