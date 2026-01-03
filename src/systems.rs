use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::models::{Card, Rank};
use crate::resources::{
    AnimationEventQueue, Difficulty, GameHistory, GameState, PileType, SequenceCheckRequest,
    SpiderGame, StartAnimationEvent,
};
use crate::ui::MovingCard;
use crate::ui::{CardTarget, ClickableCard};

#[derive(Message)]
pub struct DealEvent(pub Difficulty);
#[derive(Message)]
pub struct DealStockEvent;
#[derive(Message)]
pub struct UndoEvent;

#[derive(Message)]
pub struct AnimationFinishedEvent {
    pub cards: Vec<Card>,
    pub from: PileType,
    pub to: PileType,
    pub flip_final: bool,
}

pub fn animation_event_dispatcher(
    time: Res<Time>,
    mut queue_res: ResMut<AnimationEventQueue>,
    mut event_writer: MessageWriter<StartAnimationEvent>,
) {
    queue_res.timer.tick(time.delta());
    if queue_res.timer.just_finished() && !queue_res.queue.is_empty() {
        let event = queue_res.queue.pop().unwrap();
        event_writer.write(event);
    }
}

pub fn deal_system(
    mut commands: Commands,
    mut game: ResMut<SpiderGame>,
    mut deal_events: MessageReader<DealEvent>,
    mut queue_res: ResMut<AnimationEventQueue>,
    mut history: ResMut<GameHistory>,
    moving_cards: Query<Entity, With<MovingCard>>,
) {
    for ev in deal_events.read() {
        for entity in moving_cards.iter() {
            commands.entity(entity).despawn();
        }

        history.stack.clear();
        let current_difficulty = ev.0;
        *game = SpiderGame {
            difficulty: current_difficulty,
            score: 500,
            move_count: 0,
            ..SpiderGame::default()
        };

        let mut deck = Card::new_spider_deck(game.difficulty);
        deck.shuffle(&mut rand::thread_rng());

        queue_res.queue.clear();
        let mut events = Vec::new();

        let mut col_counts = [0; 10];

        for i in 0..54 {
            let column = i % 10;
            let mut card = deck.pop().unwrap();
            card.face_up = false;
            let is_last = if column < 4 { i >= 50 } else { i >= 44 };

            let target_idx = col_counts[column];
            col_counts[column] += 1;

            events.push(StartAnimationEvent {
                cards: vec![card],
                from: PileType::Stock,
                to: PileType::Tableau(column),
                from_index: 0,
                visual_start_pos: None,
                delay: 0.0,
                flip_final: is_last,
                original_pile_len: None,
                target_index_override: Some(target_idx),
                fly_z_override: Some(i as f32),
            });
        }
        events.reverse();
        queue_res.queue = events;
        game.stock = deck;
    }
}

pub fn deal_stock_system(
    mut game: ResMut<SpiderGame>,
    mut deal_stock_events: MessageReader<DealStockEvent>,
    mut queue_res: ResMut<AnimationEventQueue>,
    mut history: ResMut<GameHistory>,
) {
    for _ in deal_stock_events.read() {
        if game.stock.is_empty() {
            continue;
        }
        history.stack.push((*game).clone());
        let mut events = Vec::new();
        for i in 0..10 {
            if let Some(mut card) = game.stock.pop() {
                card.face_up = true;

                let target_idx = game.tableau[i].len();

                events.push(StartAnimationEvent {
                    cards: vec![card],
                    from: PileType::Stock,
                    to: PileType::Tableau(i),
                    from_index: 0,
                    visual_start_pos: None,
                    delay: 0.0,
                    flip_final: false,
                    original_pile_len: None,
                    target_index_override: Some(target_idx),
                    fly_z_override: Some(i as f32),
                });
            }
        }
        events.reverse();
        queue_res.queue.extend(events);
    }
}

pub fn finish_animation_system(
    mut game: ResMut<SpiderGame>,
    mut events: MessageReader<AnimationFinishedEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    _queue_res: ResMut<AnimationEventQueue>,
    mut check_request: ResMut<SequenceCheckRequest>,
) {
    let mut any_finished = false;
    for ev in events.read() {
        any_finished = true;
        match ev.to {
            PileType::Tableau(col) => {
                game.tableau[col].extend(ev.cards.iter().cloned());
                if ev.flip_final {
                    if let Some(last) = game.tableau[col].last_mut() {
                        last.face_up = true;
                    }
                }
            }
            PileType::Foundation(idx) => {
                if let Some(card) = ev.cards.first() {
                    if idx >= game.foundations.len() {
                        game.foundations.push(*card);
                    } else {
                        game.foundations[idx] = *card;
                    }
                }
                if ev.flip_final {
                    game.score += 100;
                    if let PileType::Tableau(from_col) = ev.from {
                        if let Some(last) = game.tableau[from_col].last_mut() {
                            last.face_up = true;
                        }
                        game.visual_pile_locks[from_col] = None;
                    }
                    if game.is_won() {
                        next_state.set(GameState::Won);
                    }
                }
            }
            _ => {}
        }
    }
    if any_finished {
        check_request.pending = true;
    }
}

pub fn auto_move_system(
    mut game: ResMut<SpiderGame>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    _queue_res: ResMut<AnimationEventQueue>,
    mut history: ResMut<GameHistory>,
    mut check_request: ResMut<SequenceCheckRequest>,
) {
    if let Some((target_pos, to_pile)) = game.auto_move_to {
        if let Some(current_drag_pos) = game.drag_pos {
            let speed = 25.0;
            let delta = time.delta_secs() * speed;
            let new_pos = current_drag_pos.lerp(target_pos, delta);
            game.drag_pos = Some(new_pos);

            if new_pos.distance(target_pos) < 2.0 {
                let mut clean_state = (*game).clone();
                clean_state.selected = None;
                clean_state.drag_pos = None;
                clean_state.auto_move_to = None;
                history.stack.push(clean_state);

                if let Some(selected) = game.selected.take() {
                    let cards_to_move: Vec<Card> = match selected.from {
                        PileType::Tableau(from_col) => {
                            let drained: Vec<Card> =
                                game.tableau[from_col].drain(selected.index..).collect();
                            if let Some(new_top) = game.tableau[from_col].last_mut() {
                                new_top.face_up = true;
                            }
                            drained
                        }
                        _ => vec![],
                    };

                    if let PileType::Tableau(col) = to_pile {
                        game.tableau[col].extend(cards_to_move);
                        if let Some(last) = game.tableau[col].last_mut() {
                            last.face_up = true;
                        }
                    }

                    game.move_count += 1;
                    game.score -= 1;

                    if game.score <= 0 {
                        next_state.set(GameState::Lost);
                    } else if game.is_won() {
                        next_state.set(GameState::Won);
                    }

                    check_request.pending = true;

                    if game.is_won() {
                        next_state.set(GameState::Won);
                    }
                }

                game.auto_move_to = None;
                game.drag_pos = None;
                game.selected = None;
            }
        }
    }
}

pub fn stability_check_system(
    mut check_request: ResMut<SequenceCheckRequest>,
    mut game: ResMut<SpiderGame>,
    mut queue_res: ResMut<AnimationEventQueue>,
    card_query: Query<(&ClickableCard, &Transform, &CardTarget)>,
) {
    if !check_request.pending {
        return;
    }

    let logical_count: usize = game.tableau.iter().map(|col| col.len()).sum();
    let visual_count = card_query
        .iter()
        .filter(|(c, _, _)| matches!(c.pile_type, PileType::Tableau(_)))
        .count();

    if logical_count != visual_count {
        return;
    }

    let mut all_stable = true;
    let threshold = 1.0;

    for (_, transform, target) in card_query.iter() {
        if transform.translation.distance(target.translation) > threshold {
            all_stable = false;
            break;
        }
        if (transform.scale.x - target.scale).abs() > 0.01 {
            all_stable = false;
            break;
        }
    }

    if all_stable {
        check_completed_sequences(&mut game, &mut queue_res);
        check_request.pending = false;
    }
}

pub fn undo_system(
    mut commands: Commands,
    mut game: ResMut<SpiderGame>,
    mut history: ResMut<GameHistory>,
    mut undo_events: MessageReader<UndoEvent>,
    mut queue_res: ResMut<AnimationEventQueue>,
    mut check_request: ResMut<SequenceCheckRequest>,
    moving_cards: Query<Entity, With<MovingCard>>,
) {
    for _ in undo_events.read() {
        if game.auto_move_to.is_some() {
            continue;
        }
        if let Some(prev_state) = history.stack.pop() {
            let current_diff = game.difficulty;
            *game = prev_state;
            game.difficulty = current_diff;

            game.visual_pile_locks = [None; 10];

            queue_res.queue.clear();
            for entity in moving_cards.iter() {
                commands.entity(entity).despawn();
            }
            check_request.pending = false;
        }
    }
}

fn check_completed_sequences(game: &mut SpiderGame, queue_res: &mut ResMut<AnimationEventQueue>) {
    for col in 0..10 {
        let current_len = game.tableau[col].len();
        if current_len < 13 {
            continue;
        }
        let start_idx = current_len - 13;
        let potential_sequence = &game.tableau[col][start_idx..];

        if potential_sequence[0].rank != Rank::King || potential_sequence[12].rank != Rank::Ace {
            continue;
        }

        let suit = potential_sequence[0].suit;
        let mut is_sequence = true;
        for i in 0..12 {
            if potential_sequence[i].suit != suit
                || potential_sequence[i + 1].suit != suit
                || potential_sequence[i].rank.value() != potential_sequence[i + 1].rank.value() + 1
            {
                is_sequence = false;
                break;
            }
        }

        if is_sequence {
            game.visual_pile_locks[col] = Some(current_len);

            let completed_cards: Vec<Card> = game.tableau[col].drain(start_idx..).collect();
            let mut events = Vec::new();
            let total_cards = completed_cards.len();

            for (i, card) in completed_cards.iter().rev().enumerate() {
                let delay = i as f32 * 0.15;
                let is_bottom_card = i == total_cards - 1;
                events.push(StartAnimationEvent {
                    cards: vec![*card],
                    from: PileType::Tableau(col),
                    to: PileType::Foundation(game.foundations.len()),
                    from_index: start_idx + (12 - i),
                    visual_start_pos: None,
                    delay,
                    flip_final: is_bottom_card,
                    original_pile_len: Some(current_len),
                    target_index_override: None,
                    fly_z_override: Some(i as f32),
                });
            }
            events.reverse();
            queue_res.queue.extend(events);
        }
    }
}
