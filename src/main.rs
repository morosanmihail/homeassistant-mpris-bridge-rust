use std::collections::HashMap;

use eyre::Result;
use homeassistant::{get_media_players, listen_for_events, MediaPlayerState};
use mpris::new_mpris_player;
use reqwest::Client;
use tokio::{sync::mpsc, task::JoinSet};

mod homeassistant;
mod mpris;

#[tokio::main]
async fn main() -> Result<()> {
    let home_assistant_url = "http://192.168.1.27:8123";
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiIzYWE5OTY2ZTkyZTc0NTg5ODE0ZDJmYTFkOWUxMTMyOSIsImlhdCI6MTcxODY1MTY1MCwiZXhwIjoyMDM0MDExNjUwfQ._-CbAO3yTtIfuu9hsSvpekb7Oy_VhFps8F4YJHbMdPs";
    let entity_ids = vec!["media_player.living_room_2".to_string()];

    let client = Client::new();
    let media_players = get_media_players(&client, home_assistant_url, token, entity_ids)
        .await
        .unwrap();

    // Channel to handle events from HA to MPRIS
    let mut channels = HashMap::new();

    // Channel to handle events from MPRIS to HA
    let (mpris_tx, mpris_rx) = mpsc::channel(100);
    let mut set = JoinSet::new();

    for player in &media_players {
        println!("{:?}\n", player);
        let (ha_tx, ha_rx) = mpsc::channel(100);
        channels.insert(player.entity_id.clone(), ha_tx);

        let _mp_task = set.spawn(new_mpris_player(
            player.entity_id.clone(),
            player.clone(),
            ha_rx,
            mpris_tx.clone(),
        ));
    }

    let media_players: HashMap<_, _> = media_players
        .iter()
        .map(|d| {
            (
                d.entity_id.clone(),
                MediaPlayerState::new(
                    d.entity_id.clone(),
                    home_assistant_url.to_string(),
                    token.to_string(),
                )
                .unwrap(),
            )
        })
        .collect();

    println!("THERE ARE {}", media_players.len());

    let _ha_task = set.spawn(listen_for_events(
        "ws://192.168.1.27:8123/api/websocket".to_string(),
        token.to_string(),
        media_players,
        channels,
        mpris_rx,
    ));

    println!("Spawned threads, now waiting.");

    while (set.join_next().await).is_some() {}
    Ok(())
}
