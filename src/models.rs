use crate::resources::Difficulty;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl Suit {
    #[allow(dead_code)]
    pub fn is_red(&self) -> bool {
        matches!(self, Suit::Hearts | Suit::Diamonds)
    }

    pub fn as_str(&self) -> &str {
        match self {
            Suit::Hearts => "H",
            Suit::Diamonds => "D",
            Suit::Clubs => "C",
            Suit::Spades => "S",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub fn as_str(&self) -> &str {
        match self {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            Rank::Ace => 1,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
        }
    }

    pub fn all() -> [Rank; 13] {
        [
            Rank::Ace,
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
    pub face_up: bool,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self {
            suit,
            rank,
            face_up: false,
        }
    }

    pub fn new_spider_deck(difficulty: Difficulty) -> Vec<Card> {
        let mut deck = Vec::new();
        match difficulty {
            Difficulty::Easy => {
                for _ in 0..8 {
                    for rank in Rank::all() {
                        deck.push(Card::new(Suit::Spades, rank));
                    }
                }
            }
            Difficulty::Medium => {
                for _ in 0..4 {
                    for rank in Rank::all() {
                        deck.push(Card::new(Suit::Spades, rank));
                        deck.push(Card::new(Suit::Hearts, rank));
                    }
                }
            }
            Difficulty::Hard => {
                for _ in 0..2 {
                    deck.extend(Self::new_standard_deck());
                }
            }
        }
        deck
    }

    pub fn new_standard_deck() -> Vec<Card> {
        let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
        let mut deck = Vec::new();
        for suit in suits {
            for rank in Rank::all() {
                deck.push(Card::new(suit, rank));
            }
        }
        deck
    }

    pub fn can_stack_on(&self, other: &Card) -> bool {
        self.rank.value() + 1 == other.rank.value()
    }

    pub fn texture_path(&self) -> String {
        if self.face_up {
            format!("cards/{}{}.png", self.rank.as_str(), self.suit.as_str())
        } else {
            "deck_black.png".to_string()
        }
    }
}
