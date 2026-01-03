use bevy::prelude::*;

use crate::layout::{get_pile_position, CARD_HEIGHT, CARD_WIDTH};
use crate::resources::{GameHistory, GameSounds, GameState, PileType, SelectedCard, SpiderGame};
use crate::systems::{DealStockEvent, UndoEvent};
use crate::ui::{ClickableCard, ClickableEmptyPile};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_mouse_input, handle_keyboard_input));
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_mouse_input(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    card_query: Query<(&Transform, &ClickableCard)>,
    empty_query: Query<(&Transform, &ClickableEmptyPile)>,
    mut game: ResMut<SpiderGame>,
    mut history: ResMut<GameHistory>,
    state: Res<State<GameState>>,
    mut deal_stock_writer: MessageWriter<DealStockEvent>,
    game_sounds: Option<Res<GameSounds>>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    if game.auto_move_to.is_some() {
        return;
    }

    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    let window_size = Vec2::new(window.width(), window.height());

    if mouse_button.pressed(MouseButton::Left)
        && game.selected.is_some()
        && game.drag_offset.is_some()
    {
        let offset = game.drag_offset.unwrap();
        game.drag_pos = Some(world_pos + offset);
    }

    if mouse_button.just_pressed(MouseButton::Left) {
        let mut cards_with_distance: Vec<_> = card_query
            .iter()
            .map(|(t, c)| {
                let scale = t.scale.x;
                (
                    t.translation.z,
                    c,
                    (world_pos.x - t.translation.truncate().x).abs(),
                    (world_pos.y - t.translation.truncate().y).abs(),
                    t.translation.truncate(),
                    scale,
                )
            })
            .filter(|(_, _, dx, dy, _, scale)| {
                let actual_width = CARD_WIDTH * scale;
                let actual_height = CARD_HEIGHT * scale;
                *dx < actual_width / 2.0 && *dy < actual_height / 2.0
            })
            .collect();

        cards_with_distance.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        if let Some((_, clickable, _, _, card_pos, _)) = cards_with_distance.first() {
            if clickable.pile_type == PileType::Stock {
                if !game.stock.is_empty() {
                    deal_stock_writer.write(DealStockEvent);
                }
                return;
            }

            let card = match clickable.pile_type {
                PileType::Tableau(col) => {
                    if clickable.index < game.tableau[col].len() {
                        Some(game.tableau[col][clickable.index])
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(c) = card {
                if c.face_up {
                    let mut valid_sequence = true;
                    if let PileType::Tableau(col) = clickable.pile_type {
                        let pile = &game.tableau[col];
                        for i in clickable.index..pile.len() - 1 {
                            let current = pile[i];
                            let next = pile[i + 1];
                            if current.suit != next.suit
                                || current.rank.value() != next.rank.value() + 1
                            {
                                valid_sequence = false;
                                break;
                            }
                        }
                    }

                    if valid_sequence {
                        let offset = *card_pos - world_pos;
                        game.drag_offset = Some(offset);
                        game.selected = Some(SelectedCard {
                            card: c,
                            from: clickable.pile_type,
                            index: clickable.index,
                        });
                        game.drag_pos = Some(world_pos + offset);
                    }
                }
            }
        }
    }

    if mouse_button.just_released(MouseButton::Left) {
        if let Some(selected) = &game.selected {
            let mut target_pile = None;
            let mut min_dist = f32::MAX;

            for (transform, card) in card_query.iter() {
                if card.pile_type == selected.from {
                    continue;
                }

                let is_top_card = match card.pile_type {
                    PileType::Tableau(col) => {
                        !game.tableau[col].is_empty() && card.index == game.tableau[col].len() - 1
                    }
                    _ => false,
                };

                if is_top_card {
                    let scale = transform.scale.x;
                    let half_w = (CARD_WIDTH * scale) / 2.0;
                    let half_h = (CARD_HEIGHT * scale) / 2.0;

                    let dx = (world_pos.x - transform.translation.x).abs();
                    let dy = (world_pos.y - transform.translation.y).abs();

                    if dx < half_w && dy < half_h {
                        let dist = transform.translation.xy().distance(world_pos);
                        if dist < min_dist {
                            if let Some(target_card) = game.get_tableau_top(match card.pile_type {
                                PileType::Tableau(c) => c,
                                _ => 0,
                            }) {
                                if target_card.rank.value() == selected.card.rank.value() + 1 {
                                    min_dist = dist;
                                    target_pile = Some(card.pile_type);
                                }
                            }
                        }
                    }
                }
            }

            if target_pile.is_none() {
                for (transform, empty) in empty_query.iter() {
                    let scale = transform.scale.x;
                    let half_w = (CARD_WIDTH * scale) / 2.0;
                    let half_h = (CARD_HEIGHT * scale) / 2.0;

                    let dx = (world_pos.x - transform.translation.x).abs();
                    let dy = (world_pos.y - transform.translation.y).abs();

                    if dx < half_w && dy < half_h {
                        let dist = transform.translation.xy().distance(world_pos);
                        if dist < min_dist {
                            min_dist = dist;
                            target_pile = Some(empty.pile_type);
                        }
                    }
                }
            }

            if let Some(to_pile) = target_pile {
                let mut valid_move = false;
                if let PileType::Tableau(to_col) = to_pile {
                    if let Some(top) = game.get_tableau_top(to_col) {
                        if selected.card.can_stack_on(top) {
                            valid_move = true;
                        }
                    } else {
                        valid_move = true;
                    }
                }

                if valid_move {
                    let mut snapshot = (*game).clone();
                    snapshot.selected = None;
                    snapshot.drag_pos = None;
                    snapshot.drag_offset = None;
                    snapshot.auto_move_to = None;
                    history.stack.push(snapshot);

                    let target_idx = match to_pile {
                        PileType::Tableau(col) => game.tableau[col].len(),
                        _ => 0,
                    };

                    let total_cards_for_drop = match to_pile {
                        PileType::Tableau(_) => target_idx + 1,
                        _ => 1,
                    };

                    let (target_pos_3d, _) =
                        get_pile_position(to_pile, target_idx, window_size, total_cards_for_drop);

                    game.auto_move_to =
                        Some((Vec2::new(target_pos_3d.x, target_pos_3d.y), to_pile));

                    if let Some(sounds) = game_sounds {
                        commands
                            .spawn((AudioPlayer(sounds.drop.clone()), PlaybackSettings::DESPAWN));
                    }
                } else {
                    game.selected = None;
                    game.drag_pos = None;
                }
            } else {
                game.selected = None;
                game.drag_pos = None;
            }
            game.drag_offset = None;
        }
    }
}

fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut undo_writer: MessageWriter<UndoEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyZ) && *state.get() == GameState::Playing {
        undo_writer.write(UndoEvent);
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Menu);
    }
}
