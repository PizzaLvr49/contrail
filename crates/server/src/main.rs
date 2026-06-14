use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_malek_async::prelude::*;
use sqlx::PgPool;

use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    RenetChannelsExt, RepliconRenetPlugins,
    netcode::{NativeSocket, NetcodeServerTransport, ServerAuthentication, ServerSetupConfig},
    renet2::{ConnectionConfig, RenetServer},
};
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

fn main() {
    dotenvy::dotenv().ok();

    App::new()
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

fn spawn_db_task(async_world: Res<AsyncWorld>) {
    let async_world = async_world.clone();
    IoTaskPool::get()
        .spawn(async move {
            if let Err(e) = setup_db(async_world).await {
                error!("Database setup failed: {e}");
            }
        })
        .detach();
}

async fn setup_db(async_world: AsyncWorld) -> Result<()> {
    let url = std::env::var("DATABASE_URL")?;

    info!("Connecting to database...");
    let pool = PgPool::connect(&url).await?;
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

fn start_replicon_server(mut commands: Commands, channels: Res<RepliconChannels>) -> Result<()> {
    let bind_addr: SocketAddr = "127.0.0.1:5000".parse()?;

    info!("Starting Replicon server on {}", bind_addr);

    let server = RenetServer::new(ConnectionConfig::from_channels(
        channels.server_configs(),
        channels.client_configs(),
    ));

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let socket = UdpSocket::bind(bind_addr)?;
    let server_config = ServerSetupConfig {
        current_time,
        max_clients: 60,
        protocol_id: 0,
        authentication: ServerAuthentication::Unsecure,
        socket_addresses: vec![vec![bind_addr]],
    };
    let transport = NetcodeServerTransport::new(server_config, NativeSocket::new(socket)?)?;

    commands.insert_resource(server);
    commands.insert_resource(transport);

    Ok(())
}
