use std::{collections::HashMap, io::Write, path::PathBuf};

use eyre::{OptionExt, Result};
use homeassistant::{get_media_players, listen_for_events, MediaPlayerState};
use mpris::new_mpris_player;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, task::JoinSet};

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

    let client = Client::new();
    let media_players = get_media_players(
        &client,
        &config.home_assistant_url,
        &config.home_assistant_token,
        config.entity_ids,
    )
    .await
    .unwrap();

    // Channel to handle events from HA to MPRIS
    let mut channels = HashMap::new();

    // Channel to handle events from MPRIS to HA
    let (mpris_tx, mpris_rx) = mpsc::channel(100);
    let mut set = JoinSet::new();

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
    }

    let media_players: HashMap<_, _> = media_players
        .iter()
        .map(|d| {
            (
                d.entity_id.clone(),
                MediaPlayerState::new(
                    d.entity_id.clone(),
                    config.home_assistant_url.to_string(),
                    config.home_assistant_token.to_string(),
                )
                .unwrap(),
            )
        })
        .collect();

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
    println!("Connected to {}", websocket_url);
    let _ha_task = set.spawn(listen_for_events(
        websocket_url,
        config.home_assistant_token.to_string(),
        media_players,
        channels,
        mpris_rx,
    ));

    while (set.join_next().await).is_some() {}
    Ok(())
}
