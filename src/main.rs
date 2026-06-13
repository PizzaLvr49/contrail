use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_malek_async::prelude::*;
use sqlx::PgPool;
use uuid::Uuid;

fn main() {
    dotenvy::dotenv().ok();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AsyncPlugin)
        .init_state::<AppState>()
        .add_systems(Startup, spawn_db_task)
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
            if let Err(e) = connect_and_seed(async_world).await {
                error!("Database setup failed: {e}");
            }
        })
        .detach();
}

async fn connect_and_seed(
    async_world: AsyncWorld,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = std::env::var("DATABASE_URL").unwrap();

    info!("Connecting to database...");
    let pool = PgPool::connect(&url).await?;
    info!("Connected!");

    sqlx::migrate!().run(&pool).await?;

    let player_id = Uuid::now_v7();
    let username = format!("Pilot_{}", &player_id.to_string()[8..16]);

    sqlx::query!(
        "INSERT INTO players (id, username) VALUES ($1, $2)",
        player_id,
        username
    )
    .execute(&pool)
    .await?;

    info!("Inserted {username}");

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
