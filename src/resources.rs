use crate::models::Card;
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    Menu,
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PileType {
    Tableau(usize),
    Stock,
    Foundation(usize),
}

#[derive(Debug, Clone)]
pub struct SelectedCard {
    pub card: Card,
    pub from: PileType,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    #[default]
    Easy = 1,
    Medium = 2,
    Hard = 4,
}

#[derive(Clone, Debug, Message)]
pub struct StartAnimationEvent {
    pub cards: Vec<Card>,
    pub from: PileType,
    pub to: PileType,
    pub from_index: usize,
    pub visual_start_pos: Option<Vec2>,
    pub delay: f32,
    pub flip_final: bool,
    pub original_pile_len: Option<usize>,
    pub target_index_override: Option<usize>,
    pub fly_z_override: Option<f32>,
}

#[derive(Resource, Clone)]
pub struct SpiderGame {
    pub tableau: [Vec<Card>; 10],
    pub stock: Vec<Card>,
    pub foundations: Vec<Card>,
    pub selected: Option<SelectedCard>,
    pub drag_pos: Option<Vec2>,
    pub drag_offset: Option<Vec2>,
    pub move_count: u32,
    pub score: i32,
    pub auto_move_to: Option<(Vec2, PileType)>,
    pub difficulty: Difficulty,
    pub visual_pile_locks: [Option<usize>; 10],
}

#[derive(Resource, Default)]
pub struct GameHistory {
    pub stack: Vec<SpiderGame>,
}

#[derive(Resource, Default)]
pub struct SequenceCheckRequest {
    pub pending: bool,
}

#[derive(Resource)]
pub struct AnimationEventQueue {
    pub timer: Timer,
    pub queue: Vec<StartAnimationEvent>,
}

impl Default for AnimationEventQueue {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.06, TimerMode::Repeating),
            queue: Vec::new(),
        }
    }
}

#[derive(Resource)]
pub struct GameSounds {
    pub deal: Handle<AudioSource>,
    pub drop: Handle<AudioSource>,
}

#[derive(Resource, Default)]
pub struct GameAssets {
    pub back_texture: Handle<Image>,
    pub blank_texture: Handle<Image>,
    pub font: Handle<Font>,
}

#[derive(Resource, Default)]
pub struct WarmUpState {
    pub frames: u32,
}

impl Default for SpiderGame {
    fn default() -> Self {
        Self {
            tableau: Default::default(),
            stock: Vec::new(),
            foundations: Vec::new(),
            selected: None,
            drag_pos: None,
            drag_offset: None,
            move_count: 0,
            score: 500,
            auto_move_to: None,
            difficulty: Difficulty::Easy,
            visual_pile_locks: [None; 10],
        }
    }
}

impl SpiderGame {
    pub fn get_tableau_top(&self, index: usize) -> Option<&Card> {
        self.tableau.get(index)?.last()
    }

    pub fn is_won(&self) -> bool {
        self.foundations.len() == 8
    }
}
