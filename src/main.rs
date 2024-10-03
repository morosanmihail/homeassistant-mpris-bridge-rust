use std::sync::Arc;

use eyre::Result;
use homeassistant::{get_media_players, listen_for_events, MediaPlayerState};
use mpris::new_mpris_player;
use reqwest::Client;
use tokio::{sync::Mutex, task::JoinSet};

mod homeassistant;
mod mpris;

#[tokio::main]
async fn main() -> Result<()> {
    let home_assistant_url = "http://192.168.1.27:8123";
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiIzYWE5OTY2ZTkyZTc0NTg5ODE0ZDJmYTFkOWUxMTMyOSIsImlhdCI6MTcxODY1MTY1MCwiZXhwIjoyMDM0MDExNjUwfQ._-CbAO3yTtIfuu9hsSvpekb7Oy_VhFps8F4YJHbMdPs";

    let client = Client::new();
    let media_players = get_media_players(&client, home_assistant_url, token)
        .await
        .unwrap();

    let mut set = JoinSet::new();
    for player in media_players {
        if player.entity_id.eq("media_player.living_room_2") {
            println!("{:?}\n", player);
            let Ok(player_state) = MediaPlayerState::new(
                player.clone(),
                home_assistant_url.to_string(),
                token.to_string(),
            ) else {
                continue;
            };
            let _mp_task = set.spawn(new_mpris_player(Arc::new(Mutex::new(player_state.clone()))));
        }
    }

    println!("Got media players");

    let _ha_task = set.spawn(listen_for_events(
        "ws://192.168.1.27:8123/api/websocket".to_string(),
        token.to_string(),
    ));

    println!("Spawned threads, now waiting.");

    while (set.join_next().await).is_some() {}
    Ok(())
}
