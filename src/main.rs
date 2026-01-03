mod input;
mod layout;
mod models;
mod resources;
mod systems;
mod ui;

use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode};
use models::{Rank, Suit};
use resources::{
    AnimationEventQueue, GameAssets, GameHistory, GameSounds, GameState, SequenceCheckRequest,
    SpiderGame, StartAnimationEvent, WarmUpState,
};
use systems::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Spider Solitaire".to_string(),
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(ui::UiPlugin)
        .add_plugins(input::InputPlugin)
        .init_state::<GameState>()
        .add_message::<DealEvent>()
        .add_message::<DealStockEvent>()
        .add_message::<StartAnimationEvent>()
        .add_message::<AnimationFinishedEvent>()
        .add_message::<UndoEvent>()
        .insert_resource(SpiderGame::default())
        .insert_resource(AnimationEventQueue::default())
        .insert_resource(GameHistory::default())
        .insert_resource(GameAssets::default())
        .insert_resource(WarmUpState { frames: 0 })
        .insert_resource(SequenceCheckRequest::default())
        .insert_resource(ClearColor(Color::srgb(
            93.0 / 255.0,
            117.0 / 255.0,
            87.0 / 255.0,
        )))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                check_assets_ready.run_if(in_state(GameState::Loading)),
                animation_event_dispatcher.run_if(in_state(GameState::Playing)),
                deal_system,
                deal_stock_system,
                finish_animation_system,
                auto_move_system,
                undo_system,
                stability_check_system.run_if(in_state(GameState::Playing)),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut game_assets: ResMut<GameAssets>,
) {
    commands.spawn(Camera2d);

    let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
    for suit in suits {
        for rank in Rank::all() {
            let _ =
                asset_server.load::<Image>(format!("cards/{}{}.png", rank.as_str(), suit.as_str()));
        }
    }

    game_assets.back_texture = asset_server.load("deck_black.png");
    game_assets.blank_texture = asset_server.load("card_blank.png");

    game_assets.font = asset_server.load("pixeloid.sans.ttf");

    commands.insert_resource(GameSounds {
        deal: asset_server.load("sounds/deal.ogg"),
        drop: asset_server.load("sounds/drop.ogg"),
    });
}

fn check_assets_ready(
    mut next_state: ResMut<NextState<GameState>>,
    asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    game_sounds: Res<GameSounds>,
    mut warm_up: ResMut<WarmUpState>,
) {
    let mut all_ready = true;

    if !matches!(
        asset_server.get_load_state(&game_assets.back_texture),
        Some(LoadState::Loaded)
    ) {
        all_ready = false;
    }
    if !matches!(
        asset_server.get_load_state(&game_assets.blank_texture),
        Some(LoadState::Loaded)
    ) {
        all_ready = false;
    }
    if !matches!(
        asset_server.get_load_state(&game_sounds.deal),
        Some(LoadState::Loaded)
    ) {
        all_ready = false;
    }

    if !matches!(
        asset_server.get_load_state(&game_assets.font),
        Some(LoadState::Loaded)
    ) {
        all_ready = false;
    }

    if all_ready {
        if warm_up.frames == 0 {
            warm_up.frames = 1;
        } else if warm_up.frames < 10 {
            warm_up.frames += 1;
        } else {
            next_state.set(GameState::Menu);
        }
    }
}
