use std::{collections::HashMap, io::Write, path::PathBuf, sync::Arc, time::Duration};

use eyre::{OptionExt, Result};
use homeassistant::{get_media_players, listen_for_events, MediaPlayerState};
use mpris::new_mpris_player;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinSet,
};

mod homeassistant;
mod mpris;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    home_assistant_url: String,
    home_assistant_token: String,
    entity_ids: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            home_assistant_url: "YOUR_HA_URL_HERE".to_string(),
            home_assistant_token: "YOUR_HA_TOKEN_HERE".to_string(),
            entity_ids: vec!["YOUR_MEDIA".to_string(), "PLAYERS_HERE".to_string()],
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = get_config()?;
    let media_players = get_media_players(
        &config.home_assistant_url,
        &config.home_assistant_token,
        config.entity_ids,
    )
    .await
    .unwrap();

    let parsed_url = url::Url::parse(&config.home_assistant_url)?;
    let websocket_url = format!(
        "ws://{}{}/api/websocket",
        parsed_url
            .host_str()
            .ok_or_eyre("Can not get host from HA URL")?,
        match parsed_url.port() {
            Some(v) => format!(":{}", v),
            None => "".to_string(),
        },
    );

    // Channel to handle events from HA to MPRIS
    let mut channels = HashMap::new();

    // Channel to handle events from MPRIS to HA
    let (mpris_tx, mpris_rx) = mpsc::channel(100);
    let mpris_rx = Arc::new(Mutex::new(mpris_rx));
    let mut set = JoinSet::new();

    let mut media_player_states = HashMap::new();

    for player in &media_players {
        let (ha_tx, ha_rx) = mpsc::channel(100);
        channels.insert(player.entity_id.clone(), ha_tx);

        let _mp_task = set.spawn(new_mpris_player(
            player.entity_id.clone(),
            player.clone(),
            config.home_assistant_url.clone(),
            ha_rx,
            mpris_tx.clone(),
        ));

        media_player_states.insert(
            player.entity_id.clone(),
            MediaPlayerState::new(
                player.entity_id.clone(),
                config.home_assistant_url.to_string(),
                config.home_assistant_token.to_string(),
            ),
        );
    }

    println!("Connected to {}", websocket_url);
    let _ha_task = set.spawn(async move {
        loop {
            let websocket_url = websocket_url.clone();
            let media_player_states = media_player_states.clone();
            let channels = channels.clone();
            if let Err(e) = listen_for_events(
                websocket_url,
                config.home_assistant_token.to_string(),
                media_player_states,
                channels,
                mpris_rx.clone(),
            )
            .await
            {
                println!("WebSocket connection lost. {e}. Retrying...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    });

    while (set.join_next().await).is_some() {}
    Ok(())
}

fn get_config() -> eyre::Result<Config> {
    let home_dir = dirs::home_dir().ok_or_eyre("Could not find home directory")?;
    let config: PathBuf = home_dir.join(".config/ha_mpris_bridge/config.toml");

    if let Some(parent_dir) = config.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    if !config.exists() {
        let default_config = Config::default();
        let toml_content = toml::to_string_pretty(&default_config)?;

        let mut file = std::fs::File::create(&config)?;
        file.write_all(toml_content.as_bytes())?;
    }

    let config = std::fs::read_to_string(&config)?;

    let config: Config = toml::from_str(&config)?;
    Ok(config)
}
