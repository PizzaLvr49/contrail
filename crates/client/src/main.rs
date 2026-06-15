use bevy::prelude::*;
use bevy_replicon::RepliconPlugins;
use bevy_replicon_renet2::renet2::RenetClientPlugin;
use shared::{
    SharedPlugin,
    steam::{ServerListCallbacks, SteamClient, SteamworksClientPlugin},
};
use std::collections::HashMap;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SteamworksClientPlugin::init().unwrap())
        .add_plugins(RepliconPlugins)
        .add_plugins(RenetClientPlugin)
        .add_plugins(SharedPlugin)
        .add_systems(Startup, try_connect_to_server)
        .run()
}

fn try_connect_to_server(steam_client: Res<SteamClient>) {
    let mut filters = HashMap::new();
    // filters.insert("gametagsand", "contrail");
    let _request = steam_client
        .matchmaking_servers()
        .internet_server_list(
            480,
            &filters,
            ServerListCallbacks::new(
                Box::new(|list, server| {
                    let value = list.lock().unwrap().get_server_details(server);
                    if let Ok(details) = value {
                        info!(
                            "[+] \"{}\"  {}/{}  addr={}:{}",
                            details.server_name,
                            details.players,
                            details.max_players,
                            details.addr,
                            details.connection_port,
                        );
                    }
                }),
                Box::new(|_list, _server| {}),
                Box::new(|list, _response| {
                    let req = list.lock().unwrap();
                    let count = req.get_server_count().unwrap_or(0);
                    if count == 0 {
                        warn!("No servers found.");
                        return;
                    }
                    if let Ok(first) = req.get_server_details(0) {
                        info!(
                            "→ Picked \"{}\" at {}:{}  steam_id={}",
                            first.server_name, first.addr, first.connection_port, first.steamid
                        );
                    }
                }),
            ),
        )
        .unwrap();
}
