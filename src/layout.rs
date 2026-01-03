use crate::resources::PileType;
use bevy::prelude::*;

const BASE_CARD_WIDTH: f32 = 120.0;
pub const BASE_VERTICAL_OFFSET: f32 = 35.0;

pub const CARD_WIDTH: f32 = BASE_CARD_WIDTH;
pub const CARD_HEIGHT: f32 = 168.0;

pub fn get_pile_position(
    pile_type: PileType,
    index: usize,
    window_size: Vec2,
    total_cards: usize,
) -> (Vec3, f32) {
    let w = window_size.x;
    let h = window_size.y;

    let max_game_width = 3000.0;
    let game_width = w.min(max_game_width) * 0.95;

    let spacing_x = game_width / 10.0;

    let target_card_width = spacing_x * 0.90;

    let scale_w = target_card_width / BASE_CARD_WIDTH;
    let scale_h = h / 900.0;

    let scale = scale_w.min(scale_h).clamp(0.4, 2.5);

    let standard_offset = BASE_VERTICAL_OFFSET * scale;
    let tableau_top_y = h / 2.0 - (100.0 * scale).max(60.0);

    let mut current_offset = standard_offset;

    if let PileType::Tableau(_) = pile_type {
        if total_cards > 1 {
            let card_visual_height = CARD_HEIGHT * scale;

            let bottom_margin = card_visual_height;

            let bottom_y = -h / 2.0 + bottom_margin;

            let available_height = tableau_top_y - bottom_y;

            let needed_height = (total_cards as f32 - 1.0) * standard_offset + card_visual_height;

            if needed_height > available_height {
                let compressed =
                    (available_height - card_visual_height) / (total_cards as f32 - 1.0);
                current_offset = compressed.max(10.0 * scale);
            }
        }
    }

    let start_x = -(9.0 * spacing_x) / 2.0;

    let pos = match pile_type {
        PileType::Tableau(col) => {
            let x = start_x + (col as f32 * spacing_x);
            let y = tableau_top_y - (index as f32 * current_offset);
            Vec3::new(x, y, index as f32 + 1.0)
        }
        PileType::Stock => {
            let x = start_x + (9.0 * spacing_x);
            let y = -h / 2.0 + (CARD_HEIGHT * scale / 1.5) + 20.0;
            Vec3::new(x, y, 0.0)
        }
        PileType::Foundation(i) => {
            let base_foundation_x = start_x;
            let offset_x = i as f32 * (spacing_x * 0.2);
            let x = base_foundation_x + offset_x;
            let y = -h / 2.0 + (CARD_HEIGHT * scale / 1.5) + 20.0;
            Vec3::new(x, y, i as f32)
        }
    };

    (pos, scale)
}
