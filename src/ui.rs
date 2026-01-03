use crate::layout::{get_pile_position, BASE_VERTICAL_OFFSET, CARD_HEIGHT, CARD_WIDTH};
use crate::models::Card;
use crate::resources::{
    AnimationEventQueue, Difficulty, GameAssets, GameSounds, GameState, PileType, SpiderGame,
    StartAnimationEvent,
};
use crate::systems::{animation_event_dispatcher, AnimationFinishedEvent, DealEvent};
use bevy::prelude::*;
use std::collections::HashMap;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(
                Update,
                handle_menu_interaction.run_if(in_state(GameState::Menu)),
            )
            .add_systems(OnExit(GameState::Menu), cleanup_menu)
            .add_systems(
                Update,
                (
                    start_animation_system.after(animation_event_dispatcher),
                    smooth_movement_system,
                    update_hud_system,
                ),
            )
            .add_systems(Update, animate_moving_cards)
            .add_systems(
                PostUpdate,
                update_game_view.run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(OnEnter(GameState::Won), setup_win_ui)
            .add_systems(OnEnter(GameState::Lost), setup_loss_ui)
            .add_systems(OnEnter(GameState::Menu), cleanup_ui);
    }
}

#[derive(Component)]
struct TransientEntity;
#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct MovesText;
#[derive(Component)]
struct GameUI;
#[derive(Component)]
struct MenuEntity;
#[derive(Component)]
struct QuitButton;
#[derive(Component)]
struct DifficultyButton(Difficulty);
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClickableCard {
    pub pile_type: PileType,
    pub index: usize,
}
#[derive(Component)]
pub struct ClickableEmptyPile {
    pub pile_type: PileType,
}

#[derive(Component)]
pub struct MovingCard {
    pub cards: Vec<Card>,
    pub from: PileType,
    pub to: PileType,
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub progress: f32,
    pub scale: f32,
    pub finished: bool,
    pub delay: f32,
    pub flip_final: bool,
    pub flying_z: f32,
    pub sound_played: bool,
}

#[derive(Component)]
pub struct CardTarget {
    pub translation: Vec3,
    pub scale: f32,
}

fn setup_menu(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(93. / 255., 117. / 255., 87. / 255.)),
            MenuEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Spider Solitaire"),
                TextFont {
                    font: game_assets.font.clone(),
                    font_size: 100.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            let difficulties = [
                (
                    "One Suit",
                    Difficulty::Easy,
                    Color::srgb(142. / 255., 146. / 255., 87. / 255.),
                ),
                (
                    "Two Suits",
                    Difficulty::Medium,
                    Color::srgb(222. / 255., 154. / 255., 40. / 255.),
                ),
                (
                    "Four Suits",
                    Difficulty::Hard,
                    Color::srgb(229. / 255., 93. / 255., 77. / 255.),
                ),
            ];
            for (label, diff, color) in difficulties {
                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(color),
                        DifficultyButton(diff),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Text::new(label),
                            TextFont {
                                font: game_assets.font.clone(),
                                font_size: 25.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            }
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
                    QuitButton,
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Quit"),
                        TextFont {
                            font: game_assets.font.clone(),
                            font_size: 25.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}
fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
fn cleanup_ui(mut commands: Commands, query: Query<Entity, With<GameUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
#[allow(clippy::type_complexity)]
fn handle_menu_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<bevy::app::AppExit>,
    mut deal_writer: MessageWriter<DealEvent>,
    diff_query: Query<
        (&Interaction, &DifficultyButton),
        (Changed<Interaction>, With<DifficultyButton>),
    >,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
) {
    for (interaction, diff_btn) in diff_query.iter() {
        if *interaction == Interaction::Pressed {
            deal_writer.write(DealEvent(diff_btn.0));
            next_state.set(GameState::Playing);
        }
    }
    for interaction in quit_query.iter() {
        if *interaction == Interaction::Pressed {
            app_exit.write(bevy::app::AppExit::Success);
        }
    }
}
fn setup_ui(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(50.0),
                ..default()
            },
            GameUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Score: 500"),
                TextFont {
                    font: game_assets.font.clone(),
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ScoreText,
            ));
            parent.spawn((
                Text::new("Moves: 0"),
                TextFont {
                    font: game_assets.font.clone(),
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                MovesText,
            ));
        });
}
fn setup_win_ui(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(2000),
            GameUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("YOU WIN!"),
                        TextFont {
                            font: game_assets.font.clone(),
                            font_size: 120.0,
                            ..default()
                        },
                        TextColor(Color::srgb(229. / 255., 93. / 255., 77. / 255.)),
                    ));
                });
        });
}

fn setup_loss_ui(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(2000),
            GameUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("YOU LOSE!"),
                        TextFont {
                            font: game_assets.font.clone(),
                            font_size: 120.0,
                            ..default()
                        },
                        TextColor(Color::srgb(142. / 255., 146. / 255., 87. / 255.)),
                    ));
                });
        });
}

fn update_hud_system(
    game: Res<SpiderGame>,
    mut q_score: Query<&mut Text, (With<ScoreText>, Without<MovesText>)>,
    mut q_moves: Query<&mut Text, (With<MovesText>, Without<ScoreText>)>,
) {
    if game.is_changed() {
        for mut text in q_score.iter_mut() {
            **text = format!("Score: {}", game.score);
        }
        for mut text in q_moves.iter_mut() {
            **text = format!("Moves: {}", game.move_count);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn start_animation_system(
    mut commands: Commands,
    mut animation_events: MessageReader<StartAnimationEvent>,
    game: Res<SpiderGame>,
    _queue_res: Res<AnimationEventQueue>,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window>,
    moving_cards: Query<(&Transform, &MovingCard)>,
    card_query: Query<(&ClickableCard, &Transform)>,
) {
    let Some(window) = window_query.iter().next() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    let mut current_z = 900.0;
    for (t, _) in moving_cards.iter() {
        if t.translation.z <= current_z {
            current_z = t.translation.z - 1.0;
        }
    }

    for ev in animation_events.read() {
        let found_visual_pos = card_query
            .iter()
            .find(|(c, _)| c.pile_type == ev.from && c.index == ev.from_index)
            .map(|(_, t)| t.translation);

        let start_pos_xy = if let Some(real_pos) = found_visual_pos {
            Vec3::new(real_pos.x, real_pos.y, 0.0)
        } else if let Some(drag_pos) = ev.visual_start_pos {
            Vec3::new(drag_pos.x, drag_pos.y, 0.0)
        } else {
            let total_cards_for_layout = ev.original_pile_len.unwrap_or_else(|| match ev.from {
                PileType::Tableau(c) => game.tableau[c].len().max(ev.from_index + 1),
                _ => 1,
            });
            let (log_start, _) =
                get_pile_position(ev.from, ev.from_index, window_size, total_cards_for_layout);
            Vec3::new(log_start.x, log_start.y, 0.0)
        };

        let total_for_scale = match ev.from {
            PileType::Tableau(c) => game.tableau[c].len().max(ev.from_index + 1),
            _ => 1,
        };
        let (_, scale) = get_pile_position(ev.from, ev.from_index, window_size, total_for_scale);

        let start_pos_z = if found_visual_pos.is_some() {
            let (p, _) = get_pile_position(ev.from, ev.from_index, window_size, total_for_scale);
            p.z
        } else if ev.visual_start_pos.is_some() {
            current_z
        } else {
            let (p, _) = get_pile_position(ev.from, ev.from_index, window_size, total_for_scale);
            p.z
        };

        let mut actual_start_x = start_pos_xy.x;
        if ev.from == PileType::Stock {
            let mut cards_in_stock = game.stock.len();
            cards_in_stock += ev.cards.len();
            let deals_left = cards_in_stock.div_ceil(10);
            if deals_left > 0 {
                let offset_factor = (deals_left - 1) as f32;
                let offset_x = -(offset_factor * 20.0 * scale);
                actual_start_x += offset_x;
            }
        }
        let actual_start_pos = Vec3::new(actual_start_x, start_pos_xy.y, start_pos_z);

        let target_idx = if let Some(idx) = ev.target_index_override {
            idx
        } else {
            match ev.to {
                PileType::Tableau(col) => game.tableau[col].len(),
                PileType::Foundation(_) => 0,
                _ => 0,
            }
        };

        let to_total = target_idx + 1;
        let (end_pos_logic, _) = get_pile_position(ev.to, target_idx, window_size, to_total);

        let flying_z = if let Some(z_override) = ev.fly_z_override {
            900.0 + z_override
        } else {
            900.0 + ev.from_index as f32
        };

        let actual_end_pos = Vec3::new(end_pos_logic.x, end_pos_logic.y, flying_z);

        commands
            .spawn((
                Transform {
                    translation: actual_start_pos,
                    scale: Vec3::splat(scale),
                    ..default()
                },
                GlobalTransform::default(),
                Visibility::default(),
                MovingCard {
                    cards: ev.cards.clone(),
                    from: ev.from,
                    to: ev.to,
                    start_pos: actual_start_pos,
                    end_pos: actual_end_pos,
                    progress: 0.0,
                    scale,
                    finished: false,
                    delay: ev.delay,
                    flip_final: ev.flip_final,
                    flying_z,
                    sound_played: false,
                },
            ))
            .with_children(|parent| {
                for (i, card) in ev.cards.iter().enumerate() {
                    let texture: Handle<Image> = asset_server.load(card.texture_path());
                    parent.spawn((
                        Sprite {
                            image: texture,
                            custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                            ..default()
                        },
                        Transform {
                            translation: Vec3::new(
                                0.0,
                                -(i as f32 * BASE_VERTICAL_OFFSET),
                                0.1 + i as f32 * 0.01,
                            ),
                            ..default()
                        },
                    ));
                }
            });
    }
}

fn animate_moving_cards(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut MovingCard)>,
    time: Res<Time>,
    mut finished_writer: MessageWriter<AnimationFinishedEvent>,
    game_sounds: Option<Res<GameSounds>>,
) {
    let speed = 3.5;
    for (entity, mut transform, mut moving) in query.iter_mut() {
        if moving.finished {
            commands.entity(entity).despawn();
            continue;
        }

        if moving.delay > 0.0 {
            moving.delay -= time.delta_secs();
            transform.translation = moving.start_pos;
            continue;
        }

        if !moving.sound_played {
            if let Some(sounds) = &game_sounds {
                commands.spawn((AudioPlayer(sounds.deal.clone()), PlaybackSettings::DESPAWN));
            }
            moving.sound_played = true;
        }

        moving.progress += time.delta_secs() * speed;
        let t = moving.progress.min(1.0);
        let eased_t = 1.0 - (1.0 - t) * (1.0 - t);

        let current_xy = moving.start_pos.xy().lerp(moving.end_pos.xy(), eased_t);
        let current_z = moving.flying_z;

        transform.translation = Vec3::new(current_xy.x, current_xy.y, current_z);
        transform.scale = Vec3::splat(moving.scale);

        if moving.progress >= 1.0 {
            finished_writer.write(AnimationFinishedEvent {
                cards: moving.cards.clone(),
                from: moving.from,
                to: moving.to,
                flip_final: moving.flip_final,
            });
            moving.finished = true;
        }
    }
}

fn smooth_movement_system(mut query: Query<(&mut Transform, &CardTarget)>, time: Res<Time>) {
    let dt = time.delta_secs();
    let speed = 8.0;

    for (mut transform, target) in query.iter_mut() {
        let current_scale = transform.scale.x;
        let new_scale = current_scale + (target.scale - current_scale) * speed * dt;
        transform.scale = Vec3::splat(new_scale);

        let current_pos = transform.translation;
        let target_pos = target.translation;

        let new_x = current_pos.x + (target_pos.x - current_pos.x) * speed * dt;
        let new_y = current_pos.y + (target_pos.y - current_pos.y) * speed * dt;

        transform.translation = Vec3::new(new_x, new_y, target_pos.z);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_game_view(
    mut commands: Commands,
    game: Res<SpiderGame>,
    queue_res: Res<AnimationEventQueue>,
    _state: Res<State<GameState>>,
    asset_server: Res<AssetServer>,
    clickable_cards: Query<(Entity, &ClickableCard)>,
    clickable_empty: Query<(Entity, &ClickableEmptyPile)>,
    transients: Query<Entity, With<TransientEntity>>,
    window_query: Query<&Window>,
) {
    for entity in transients.iter() {
        commands.entity(entity).despawn();
    }

    let Some(window) = window_query.iter().next() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    let blank_texture: Handle<Image> = asset_server.load("card_blank.png");
    let back_texture: Handle<Image> = asset_server.load("deck_black.png");
    let slot_color = Color::srgba(1.0, 1.0, 1.0, 0.3);

    let mut existing_cards: HashMap<(PileType, usize), Entity> = HashMap::new();
    for (e, c) in clickable_cards.iter() {
        existing_cards.insert((c.pile_type, c.index), e);
    }

    let mut existing_empty: HashMap<PileType, Entity> = HashMap::new();
    for (e, c) in clickable_empty.iter() {
        existing_empty.insert(c.pile_type, e);
    }

    let is_being_dragged = |pile_type: PileType, index: usize| -> bool {
        if let Some(selected) = &game.selected {
            if game.drag_pos.is_some() && selected.from == pile_type {
                if let PileType::Tableau(_) = pile_type {
                    return index >= selected.index;
                }
                return index == selected.index;
            }
        }
        false
    };

    let (stock_pos, scale) = get_pile_position(PileType::Stock, 0, window_size, 1);
    let scale_vec = Vec3::splat(scale);
    let mut cards_in_stock = game.stock.len();
    cards_in_stock += queue_res
        .queue
        .iter()
        .filter(|ev| ev.from == PileType::Stock)
        .map(|ev| ev.cards.len())
        .sum::<usize>();
    if cards_in_stock > 0 {
        let max_deals = 5;
        let actual_deals_left = cards_in_stock.div_ceil(10);
        let visual_deals_left = actual_deals_left.min(max_deals);
        let start_index = max_deals - visual_deals_left;
        for i in start_index..max_deals {
            let offset_factor = i as f32 - (max_deals as f32 - 1.0);
            let offset_x = offset_factor * 20.0 * scale;
            let z_offset = max_deals as f32 - i as f32;
            let pos = Vec3::new(stock_pos.x + offset_x, stock_pos.y, stock_pos.z + z_offset);
            let key = (PileType::Stock, i);
            if let Some(entity) = existing_cards.remove(&key) {
                commands.entity(entity).insert(CardTarget {
                    translation: pos,
                    scale,
                });
            } else {
                commands.spawn((
                    Sprite {
                        image: back_texture.clone(),
                        custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                        ..default()
                    },
                    Transform {
                        translation: pos,
                        scale: scale_vec,
                        ..default()
                    },
                    ClickableCard {
                        pile_type: PileType::Stock,
                        index: i,
                    },
                    CardTarget {
                        translation: pos,
                        scale,
                    },
                ));
            }
        }
    }

    for (i, card) in game.foundations.iter().enumerate() {
        let pile = PileType::Foundation(i);
        let idx = 0;
        if !is_being_dragged(pile, idx) {
            let (pos, scale) = get_pile_position(pile, idx, window_size, 1);
            if let Some(entity) = existing_cards.remove(&(pile, idx)) {
                commands.entity(entity).insert(CardTarget {
                    translation: pos,
                    scale,
                });
                let texture_path = card.texture_path();
                commands.entity(entity).insert(Sprite {
                    image: asset_server.load(texture_path),
                    custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                    ..default()
                });
            } else {
                let texture_path = card.texture_path();
                commands.spawn((
                    Sprite {
                        image: asset_server.load(texture_path),
                        custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                        ..default()
                    },
                    Transform {
                        translation: pos,
                        scale: Vec3::splat(scale),
                        ..default()
                    },
                    ClickableCard {
                        pile_type: pile,
                        index: idx,
                    },
                    CardTarget {
                        translation: pos,
                        scale,
                    },
                ));
            }
        }
    }

    for (i, col) in game.tableau.iter().enumerate() {
        let pile_type = PileType::Tableau(i);
        let (base_pos, _) = get_pile_position(pile_type, 0, window_size, 1);

        if let Some(entity) = existing_empty.remove(&pile_type) {
            commands.entity(entity).insert(CardTarget {
                translation: Vec3::new(base_pos.x, base_pos.y, 0.0),
                scale,
            });
        } else {
            commands.spawn((
                Sprite {
                    image: blank_texture.clone(),
                    custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                    color: slot_color,
                    ..default()
                },
                Transform {
                    translation: Vec3::new(base_pos.x, base_pos.y, 0.0),
                    scale: scale_vec,
                    ..default()
                },
                ClickableEmptyPile { pile_type },
                CardTarget {
                    translation: Vec3::new(base_pos.x, base_pos.y, 0.0),
                    scale,
                },
            ));
        }

        let total_in_pile_visual = if let Some(locked_len) = game.visual_pile_locks[i] {
            locked_len
        } else {
            col.len()
        };

        for (card_idx, card) in col.iter().enumerate() {
            if !is_being_dragged(pile_type, card_idx) {
                let (target_pos, scale) =
                    get_pile_position(pile_type, card_idx, window_size, total_in_pile_visual);

                if let Some(entity) = existing_cards.remove(&(pile_type, card_idx)) {
                    commands.entity(entity).insert(CardTarget {
                        translation: target_pos,
                        scale,
                    });
                    let texture_path = if card.face_up {
                        card.texture_path()
                    } else {
                        "deck_black.png".to_string()
                    };
                    commands.entity(entity).insert(Sprite {
                        image: asset_server.load(texture_path),
                        custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                        ..default()
                    });
                } else {
                    let (loose_pos, _) =
                        get_pile_position(pile_type, card_idx, window_size, card_idx + 1);

                    let texture_path = if card.face_up {
                        card.texture_path()
                    } else {
                        "deck_black.png".to_string()
                    };
                    commands.spawn((
                        Sprite {
                            image: asset_server.load(texture_path),
                            custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                            ..default()
                        },
                        Transform {
                            translation: loose_pos,
                            scale: Vec3::splat(scale),
                            ..default()
                        },
                        ClickableCard {
                            pile_type,
                            index: card_idx,
                        },
                        CardTarget {
                            translation: target_pos,
                            scale,
                        },
                    ));
                }
            }
        }
    }

    for (_, entity) in existing_cards {
        commands.entity(entity).despawn();
    }
    for (_, entity) in existing_empty {
        commands.entity(entity).despawn();
    }

    if let Some(selected) = &game.selected {
        if let Some(drag_pos) = game.drag_pos {
            let cards_to_render: Vec<(usize, &Card)> = match selected.from {
                PileType::Tableau(col) => game.tableau[col]
                    .iter()
                    .enumerate()
                    .skip(selected.index)
                    .collect(),
                _ => vec![],
            };
            for (i, (_, card)) in cards_to_render.iter().enumerate() {
                let texture: Handle<Image> = asset_server.load(card.texture_path());
                commands.spawn((
                    Sprite {
                        image: texture,
                        custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                        ..default()
                    },
                    Transform {
                        translation: Vec3::new(
                            drag_pos.x,
                            drag_pos.y - (i as f32 * BASE_VERTICAL_OFFSET * scale),
                            500.0 + i as f32,
                        ),
                        scale: scale_vec,
                        ..default()
                    },
                    TransientEntity,
                ));
            }
        }
    }

    for ev in queue_res.queue.iter() {
        if let PileType::Tableau(col) = ev.from {
            let total_estimate = ev
                .original_pile_len
                .unwrap_or(ev.from_index + ev.cards.len());

            for (i, card) in ev.cards.iter().enumerate() {
                let pile_idx = ev.from_index + i;
                let (pos, _) = get_pile_position(
                    PileType::Tableau(col),
                    pile_idx,
                    window_size,
                    total_estimate,
                );
                let texture: Handle<Image> = asset_server.load(card.texture_path());
                commands.spawn((
                    Sprite {
                        image: texture,
                        custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                        ..default()
                    },
                    Transform {
                        translation: pos,
                        scale: scale_vec,
                        ..default()
                    },
                    TransientEntity,
                ));
            }
        }
    }
}
