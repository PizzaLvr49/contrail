use std::{ops::Deref, sync::Mutex};

use bevy::app::{App, First, Plugin};
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::{
    message::Message,
    prelude::{MessageWriter, Resource},
    schedule::{IntoScheduleConfigs, SystemSet},
    system::Res,
};

pub use steamworks::*;

/// A Bevy-compatible wrapper around various Steamworks events.
#[derive(Message, Debug)]
pub enum SteamworksEvent {
    CallbackResult(CallbackResult),
}

/// A Bevy compatible wrapper around [`steamworks::Client`].
///
/// Automatically dereferences to the client so it can be transparently
/// used.
///
/// For more information on how to use it, see [`steamworks::Client`].
#[derive(Resource, Clone)]
pub struct SteamClient(pub Client);

impl Deref for SteamClient {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A Bevy-compatible wrapper around [`steamworks::Server`].
///
/// Automatically dereferences to the server so it can be transparently used.
#[derive(Resource)]
pub struct SteamServer(pub Server);

impl Deref for SteamServer {
    type Target = Server;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A Bevy [`Plugin`] for adding support for the Steam Client SDK.
pub struct SteamworksClientPlugin {
    steam: Mutex<Option<Client>>,
}

impl SteamworksClientPlugin {
    /// Creates a new `SteamworksPlugin`. The provided `app_id` should correspond
    /// to the Steam app ID provided by Valve.
    /// # Errors
    /// Steam API errors.
    pub fn init_app(app_id: impl Into<AppId>) -> Result<Self, SteamAPIInitError> {
        Ok(Self {
            steam: Mutex::new(Some(Client::init_app(app_id.into())?)),
        })
    }

    /// Creates a new `SteamworksPlugin` using the automatically determined app ID.
    /// If the game isn't being run through steam this can be provided by placing a `steam_appid.txt`
    /// with the ID inside in the current working directory.
    /// Alternatively, you can use `SteamworksPlugin::init_app(<app_id>)` to force a specific app ID.
    /// # Errors
    /// Steam API errors.
    pub fn init() -> Result<Self, SteamAPIInitError> {
        Ok(Self {
            steam: Mutex::new(Some(Client::init()?)),
        })
    }
}

impl From<Client> for SteamworksClientPlugin {
    fn from(client: Client) -> Self {
        Self {
            steam: Mutex::new(Some(client)),
        }
    }
}

impl Plugin for SteamworksClientPlugin {
    fn build(&self, app: &mut App) {
        let client = self
            .steam
            .lock()
            .unwrap()
            .take()
            .expect("The SteamworksPlugin was initialized more than once");

        app.insert_resource(SteamClient(client))
            .add_message::<SteamworksEvent>()
            .configure_sets(First, SteamworksSystem::RunCallbacks)
            .add_systems(
                First,
                run_steam_callbacks
                    .in_set(SteamworksSystem::RunCallbacks)
                    .before(bevy::ecs::message::MessageUpdateSystems),
            );
    }
}

/// A set of [`SystemSet`]s for systems used by [`SteamworksPlugin`]
///
/// [`SystemSet`]: bevy_ecs::schedule::SystemSet
#[derive(Debug, Clone, Copy, Eq, Hash, SystemSet, PartialEq)]
pub enum SteamworksSystem {
    /// A system set that runs the Steam SDK callbacks. Anything dependent on
    /// Steam API results should scheduled after this. This runs in
    /// [`First`].
    RunCallbacks,
}

fn run_steam_callbacks(client: Res<SteamClient>, mut output: MessageWriter<SteamworksEvent>) {
    client.process_callbacks(|callback| {
        output.write(SteamworksEvent::CallbackResult(callback));
    });
}

/// A Bevy [`Plugin`] for adding support for the Steam Server SDK.
///
/// This plugin should be added to your `App` at startup. Its internal callback
/// systems will remain completely idle until `SteamServer` is explicitly inserted
/// into the world resources at runtime.
pub struct SteamworksServerPlugin;

impl Plugin for SteamworksServerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(First, SteamworksServerSystem::RunCallbacks)
            .add_systems(
                First,
                run_steam_server_callbacks
                    .in_set(SteamworksServerSystem::RunCallbacks)
                    .run_if(resource_exists::<SteamServer>),
            );
    }
}

/// A set of [`SystemSet`]s for systems used by [`SteamworksServerPlugin`].
#[derive(Debug, Clone, Copy, Eq, Hash, SystemSet, PartialEq)]
pub enum SteamworksServerSystem {
    /// A system set that runs the Steam Server SDK callbacks.
    RunCallbacks,
}

fn run_steam_server_callbacks(server: Res<SteamServer>) {
    server.run_callbacks();
}
