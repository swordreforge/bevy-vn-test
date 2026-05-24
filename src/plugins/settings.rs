use bevy::prelude::*;
use crate::components::*;
use crate::resources::Settings;
use crate::state::AppState;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Settings>()
            .add_systems(OnEnter(AppState::Settings), setup_settings_ui)
            .add_systems(OnExit(AppState::Settings), cleanup_settings)
            .add_systems(Update, (
                handle_slider_clicks,
                handle_toggle_clicks,
                handle_back_click,
                update_slider_visuals,
                update_toggle_visuals,
            ).run_if(in_state(AppState::Settings)));
    }
}

#[derive(Component)]
struct SettingsScreen;

fn color_for_filled(filled: bool) -> Color {
    if filled {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(0.25, 0.25, 0.3)
    }
}

fn setup_settings_ui(mut commands: Commands, settings: Res<Settings>) {
    let slider_defs: [(&str, SliderSetting, f32); 5] = [
        ("BGM Volume", SliderSetting::BgmVolume, settings.bgm_volume * 100.0),
        ("SE Volume", SliderSetting::SeVolume, settings.se_volume * 100.0),
        ("Voice Volume", SliderSetting::VoiceVolume, settings.voice_volume * 100.0),
        ("Text Speed", SliderSetting::TextSpeed, settings.text_speed as f32),
        ("Msg Opacity", SliderSetting::MsgOpacity, settings.message_window_opacity as f32),
    ];

    commands.spawn((
        SettingsScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.15, 0.95)),
        ZIndex(5),
    )).with_children(|parent| {
        // Back button
        parent.spawn((
            SettingsBackButton,
            Text::new("← Back"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::srgb(0.6, 0.6, 0.8)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Px(20.0),
                ..default()
            },
        ));

        // Title
        parent.spawn((
            Text::new("Settings"),
            TextFont { font_size: 36.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(30.0)), ..default() },
        ));

        // Sliders
        for (label, setting, initial) in &slider_defs {
            let initial_val = *initial;
            let setting_copy = *setting;
            parent.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
            )).with_children(|row| {
                // Label
                row.spawn((
                    Text::new(*label),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    Node { width: Val::Px(150.0), ..default() },
                ));

                // Track with 10 segments
                for i in 0..10 {
                    let seg_val = (i as f32) * 10.0;
                    let filled = seg_val <= initial_val;
                    row.spawn((
                        SliderSegment(i),
                        setting_copy,
                        Node {
                            width: Val::Px(20.0),
                            height: Val::Px(22.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(color_for_filled(filled)),
                    ));
                }

                // Value text
                row.spawn((
                    SliderValueText,
                    Text::new(format!("{:>3.0}", initial_val)),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    Node {
                        width: Val::Px(50.0),
                        ..default()
                    },
                ));
            });
        }

        // Spacing before toggles
        parent.spawn((Node { height: Val::Px(20.0), ..default() },));

        // Toggles
        let toggle_defs: [(&str, &str, bool); 2] = [
            ("Auto Mode", "auto", settings.auto_mode),
            ("Skip Mode", "skip", settings.skip_mode),
        ];

        for (label, group, initial_val) in &toggle_defs {
            parent.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
            )).with_children(|row| {
                row.spawn((
                    Text::new(*label),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    Node { width: Val::Px(150.0), ..default() },
                ));

                // ON button
                let on_active = *initial_val;
                row.spawn((
                    ToggleOption { group: group.to_string(), value: true },
                    Text::new("ON"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(if on_active { Color::WHITE } else { Color::srgb(0.4, 0.4, 0.5) }),
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if on_active { Color::srgb(0.15, 0.3, 0.6) } else { Color::srgb(0.12, 0.12, 0.18) }),
                ));

                // OFF button
                let off_active = !*initial_val;
                row.spawn((
                    ToggleOption { group: group.to_string(), value: false },
                    Text::new("OFF"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(if off_active { Color::WHITE } else { Color::srgb(0.4, 0.4, 0.5) }),
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if off_active { Color::srgb(0.3, 0.12, 0.12) } else { Color::srgb(0.12, 0.12, 0.18) }),
                ));
            });
        }
    });
}

fn handle_slider_clicks(
    mut settings: ResMut<Settings>,
    query: Query<(&SliderSegment, &SliderSetting, &Interaction), Changed<Interaction>>,
) {
    for (segment, slider_setting, interaction) in query.iter() {
        if *interaction == Interaction::Pressed {
            let val = (segment.0 as f32) * 10.0;
            match slider_setting {
                SliderSetting::BgmVolume => settings.bgm_volume = val / 100.0,
                SliderSetting::SeVolume => settings.se_volume = val / 100.0,
                SliderSetting::VoiceVolume => settings.voice_volume = val / 100.0,
                SliderSetting::TextSpeed => settings.text_speed = val as u32,
                SliderSetting::MsgOpacity => settings.message_window_opacity = val as u8,
            }
        }
    }
}

fn handle_toggle_clicks(
    mut settings: ResMut<Settings>,
    query: Query<(&ToggleOption, &Interaction), Changed<Interaction>>,
) {
    for (option, interaction) in query.iter() {
        if *interaction == Interaction::Pressed {
            match option.group.as_str() {
                "auto" => settings.auto_mode = option.value,
                "skip" => settings.skip_mode = option.value,
                _ => warn!("Unknown toggle group: {}", option.group),
            }
        }
    }
}

fn handle_back_click(
    mut next_state: ResMut<NextState<AppState>>,
    query: Query<&Interaction, (With<SettingsBackButton>, Changed<Interaction>)>,
) {
    for interaction in query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

fn update_slider_visuals(
    settings: Res<Settings>,
    mut query: Query<(&SliderSegment, &SliderSetting, &mut BackgroundColor)>,
) {
    for (segment, slider_setting, mut bg) in query.iter_mut() {
        let current = match slider_setting {
            SliderSetting::BgmVolume => settings.bgm_volume * 100.0,
            SliderSetting::SeVolume => settings.se_volume * 100.0,
            SliderSetting::VoiceVolume => settings.voice_volume * 100.0,
            SliderSetting::TextSpeed => settings.text_speed as f32,
            SliderSetting::MsgOpacity => settings.message_window_opacity as f32,
        };
        let seg_val = (segment.0 as f32) * 10.0;
        *bg = BackgroundColor(color_for_filled(seg_val <= current));
    }
}

fn update_toggle_visuals(
    settings: Res<Settings>,
    mut query: Query<(&ToggleOption, &mut TextColor, &mut BackgroundColor)>,
) {
    for (option, mut text_color, mut bg) in query.iter_mut() {
        let active = match option.group.as_str() {
            "auto" => settings.auto_mode == option.value,
            "skip" => settings.skip_mode == option.value,
            _ => false,
        };
        if active {
            *text_color = TextColor(Color::WHITE);
            *bg = BackgroundColor(if option.value { Color::srgb(0.15, 0.3, 0.6) } else { Color::srgb(0.3, 0.12, 0.12) });
        } else {
            *text_color = TextColor(Color::srgb(0.4, 0.4, 0.5));
            *bg = BackgroundColor(Color::srgb(0.12, 0.12, 0.18));
        }
    }
}

fn cleanup_settings(
    mut commands: Commands,
    query: Query<Entity, With<SettingsScreen>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
