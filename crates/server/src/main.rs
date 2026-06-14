use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_malek_async::prelude::*;
use sqlx::PgPool;
use steamworks::{Client, Server, ServerMode};

use bevy_renet2::steam::{AccessPermission, SteamServerConfig, SteamServerTransport};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    RenetChannelsExt, RepliconRenetPlugins,
    renet2::{ConnectionConfig, RenetServer},
};
use clap::Parser;
use std::net::{IpAddr, SocketAddr};

#[derive(Parser, Debug, Resource)]
#[command(author, version, about)]
struct Args {
    /// Address to bind the game server to.
    #[arg(long, env = "SERVER_ADDR", default_value = "127.0.0.1:5000")]
    server_addr: SocketAddr,

    /// Database connection URL.
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Maximum number of connected clients.
    #[arg(long, env = "MAX_CLIENTS", default_value_t = 60)]
    max_clients: usize,

    /// Protocol ID for Renet.
    #[arg(long, env = "PROTOCOL_ID", default_value_t = 0)]
    protocol_id: u64,
}

fn main() {
    dotenvy::dotenv().ok();

    App::new()
        .insert_resource(Args::parse())
        .add_plugins(DefaultPlugins)
        .add_plugins(AsyncPlugin)
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)
        .init_state::<AppState>()
        .add_systems(Startup, spawn_db_task)
        .add_systems(
            Update,
            (
                async_world_sync_point::<DbSyncPoint>,
                run_steam_callbacks.run_if(in_state(AppState::Ready)),
            ),
        )
        .add_systems(OnEnter(AppState::ConnectingSteam), init_steam_server)
        .add_systems(
            OnEnter(AppState::ConfiguringTransport),
            init_transport_server,
        )
        .run();
}

struct DbSyncPoint;

#[derive(Resource)]
pub struct DatabasePool(pub PgPool);

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    #[default]
    ConnectingDatabase,
    ConnectingSteam,
    ConfiguringTransport,
    Ready,
}

fn spawn_db_task(async_world: Res<AsyncWorld>, args: Res<Args>) {
    let async_world = async_world.clone();
    let database_url = args.database_url.clone();

    IoTaskPool::get()
        .spawn(async move {
            if let Err(e) = setup_db(async_world, database_url).await {
                error!("Database setup failed: {e}");
            }
        })
        .detach();
}

async fn setup_db(async_world: AsyncWorld, database_url: String) -> Result<()> {
    info!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;
    info!("Connected!");

    sqlx::migrate!("../../migrations").run(&pool).await?;

    async_world
        .bridge(
            DbSyncPoint,
            |mut commands: Commands, mut next_state: ResMut<NextState<AppState>>| {
                commands.insert_resource(DatabasePool(pool));
                next_state.set(AppState::ConnectingSteam);
            },
        )
        .await?;

    Ok(())
}

#[derive(Resource)]
pub struct SteamServerInstance {
    pub server: Server,
    pub client: Client,
}

fn init_steam_server(mut commands: Commands, args: Res<Args>) {
    info!("Starting Steamworks...");

    let IpAddr::V4(ipv4_addr) = args.server_addr.ip() else {
        error!("Steamworks requires an IPv4 address, but an IPv6 address was provided.");
        return;
    };

    match Server::init(
        ipv4_addr,
        args.server_addr.port(),
        args.server_addr.port() + 1,
        ServerMode::NoAuthentication,
        "0",
    ) {
        Ok((server, client)) => {
            server.log_on_anonymous();
            server.enable_heartbeats(true);
            server.set_max_players(args.max_clients as i32);

            commands.insert_resource(SteamServerInstance { server, client });
            info!("Steamworks initialized successfully.");
        }
        Err(e) => {
            error!("Steam init failed: {:?}", e);
        }
    }
}

fn init_transport_server(
    mut commands: Commands,
    channels: Res<RepliconChannels>,
    args: Res<Args>,
    mut state: ResMut<NextState<AppState>>,
    instance: Res<SteamServerInstance>,
) {
    instance.server.run_callbacks();
    info!("Initializing Replicon and Renet2 Transport...");

    let renet_server = RenetServer::new(ConnectionConfig::from_channels(
        channels.server_configs(),
        channels.client_configs(),
    ));

    let steam_config = SteamServerConfig {
        max_clients: args.max_clients,
        access_permission: AccessPermission::Public,
    };

    match SteamServerTransport::new(&instance.client, steam_config) {
        Ok(transport) => {
            commands.insert_resource(renet_server);
            commands.insert_resource(transport);

            state.set(AppState::Ready);
            info!("Server is Ready!");
        }
        Err(e) => {
            error!("Steam transport failed: {:?}", e);
        }
    }
}

fn run_steam_callbacks(server_instance: Res<SteamServerInstance>) {
    server_instance.server.run_callbacks();
}
