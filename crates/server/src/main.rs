use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_malek_async::prelude::*;
use sqlx::PgPool;

use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    RenetChannelsExt, RepliconRenetPlugins,
    netcode::{NativeSocket, NetcodeServerTransport, ServerAuthentication, ServerSetupConfig},
    renet2::{ConnectionConfig, RenetServer},
};
use clap::Parser;
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

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
        .add_systems(OnEnter(AppState::InGame), start_replicon_server)
        .add_systems(Update, async_world_sync_point::<DbSyncPoint>)
        .run();
}

struct DbSyncPoint;

#[derive(Resource)]
pub struct DatabasePool(pub PgPool);

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    #[default]
    ConnectingDatabase,
    InGame,
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
                next_state.set(AppState::InGame);
            },
        )
        .await?;

    Ok(())
}

fn start_replicon_server(
    mut commands: Commands,
    channels: Res<RepliconChannels>,
    args: Res<Args>,
) -> Result<()> {
    info!("Starting Replicon server on {}", args.server_addr);

    let server = RenetServer::new(ConnectionConfig::from_channels(
        channels.server_configs(),
        channels.client_configs(),
    ));

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let socket = UdpSocket::bind(args.server_addr)?;

    let server_config = ServerSetupConfig {
        current_time,
        max_clients: args.max_clients,
        protocol_id: args.protocol_id,
        authentication: ServerAuthentication::Unsecure,
        socket_addresses: vec![vec![args.server_addr]],
    };

    let transport = NetcodeServerTransport::new(server_config, NativeSocket::new(socket)?)?;

    commands.insert_resource(server);
    commands.insert_resource(transport);

    Ok(())
}
