pub mod steam;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_client_event::<Ping>(Channel::Ordered);
    }
}

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct Ping;
