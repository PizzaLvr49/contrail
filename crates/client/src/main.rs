use bevy::prelude::*;
use shared::steam::SteamworksClientPlugin;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SteamworksClientPlugin::init().unwrap())
        .run()
}
