use std::collections::HashMap;

use eyre::{OptionExt, Result};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Error, Value};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

#[derive(Deserialize, Debug, Clone)]
pub struct MediaPlayer {
    pub entity_id: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub state: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MediaPlayerState {
    ha_url: String,
    ha_token: String,
    pub entity_id: String,
}

#[derive(Debug)]
pub struct MediaPlayerMetadata {
    pub title: String,
    pub artist: String,
    pub duration: i64,
    pub position: i64,
    pub volume: f64,
    pub art_url: String,
    pub playing: bool,
}

#[derive(Debug)]
pub enum HAEvent {
    Play,
    Pause,
    MetadataUpdated(MediaPlayerMetadata),
    Next,
    Previous,
}

pub fn json_to_metadata(
    metadata: HashMap<String, serde_json::Value>,
    playing: bool,
    base_url: String,
) -> Result<MediaPlayerMetadata> {
    Ok(MediaPlayerMetadata {
        title: metadata
            .get("media_title")
            .unwrap_or(&Value::String("".to_string()))
            .to_string()
            .trim_matches(['\"'])
            .to_string(),
        artist: metadata
            .get("media_artist")
            .unwrap_or(&Value::String("".to_string()))
            .to_string()
            .trim_matches(['\"'])
            .to_string(),
        duration: metadata
            .get("media_duration")
            .unwrap_or(&json!(0))
            .as_i64()
            .ok_or_eyre("Could not convert Number to i64")?,
        position: metadata
            .get("media_position")
            .unwrap_or(&json!(0))
            .as_i64()
            .ok_or_eyre("Could not convert Number to i64")?,
        art_url: validate_art_url(
            metadata
                .get("entity_picture")
                .unwrap_or(&Value::String("".to_string()))
                .to_string()
                .trim_matches(['\"'])
                .to_string(),
            &base_url,
        )?
        .to_string(),
        volume: metadata
            .get("volume_level")
            .unwrap_or(&json!(1.0))
            .as_f64()
            .ok_or_eyre("Could not convert Number to f64")?,
        playing,
    })
}

impl MediaPlayerState {
    pub fn new(entity_id: String, ha_url: String, ha_token: String) -> Self {
        Self {
            ha_token,
            ha_url,
            entity_id,
        }
    }

    pub async fn play(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_play").await
    }

    pub async fn pause(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_pause").await
    }

    pub async fn next(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_next_track")
            .await
    }

    pub async fn previous(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_previous_track")
            .await
    }

    pub async fn update_metadata(
        &self,
        metadata: serde_json::value::Value,
        state: String,
    ) -> Result<Vec<HAEvent>> {
        let mut events = vec![];

        if state.eq("\"playing\"") {
            events.push(HAEvent::Play);
        } else {
            events.push(HAEvent::Pause);
        }
        let attribs: HashMap<String, serde_json::Value> =
            if let serde_json::Value::Object(m) = metadata {
                m.into_iter().collect()
            } else {
                eyre::bail!("Oh no");
            };
        events.push(HAEvent::MetadataUpdated(json_to_metadata(
            attribs,
            state.contains("playing"),
            self.ha_url.clone(),
        )?));
        Ok(events)
    }

    pub async fn send_command_to_home_assistant(&self, command: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/services/media_player/{}", self.ha_url, command);
        let params = serde_json::json!({
            "entity_id": self.entity_id,
        });

        client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.ha_token))
            .json(&params)
            .send()
            .await
            .expect("AAAH");

        Ok(())
    }
}

pub async fn get_media_players(
    home_assistant_url: &str,
    token: &str,
    entity_ids: Vec<String>,
) -> Result<Vec<MediaPlayer>> {
    let client = Client::new();
    let url = format!("{}/api/states", home_assistant_url);
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let media_players: Vec<MediaPlayer> = response
        .json::<Vec<MediaPlayer>>()
        .await?
        .into_iter()
        .filter(|m| m.entity_id.starts_with("media_player."))
        .filter(|d| entity_ids.contains(&d.entity_id))
        .collect();

    Ok(media_players)
}

pub async fn listen_for_events(
    ha_url: String,
    access_token: String,
    mut media_players: HashMap<String, MediaPlayerState>,
    channels: HashMap<String, Sender<HAEvent>>,
    mut mpris_rx: Receiver<(String, HAEvent)>,
) -> Result<()> {
    let (ws_stream, _) = connect_async(ha_url).await?;
    let (mut write, mut read) = ws_stream.split();
    let auth_message = json!({
        "type": "auth",
        "access_token": access_token
    });
    write.send(Message::Text(auth_message.to_string())).await?;
    while let Some(Ok(message)) = read.next().await {
        if let Message::Text(text) = message {
            let response: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
            if response["type"] == "auth_ok" {
                println!("Authenticated successfully!");
                break;
            } else if response["type"] == "auth_invalid" {
                eprintln!("Authentication failed: {}", response["message"]);
            }
        }
    }

    let subscribe_message = json!({
        "id": 1,
        "type": "subscribe_events",
        "event_type": "state_changed",
    });

    write
        .send(Message::Text(subscribe_message.to_string()))
        .await
        .expect("Failed to send subscribe message");

    loop {
        tokio::select! {
            event = read.next() => {
                let Some(Ok(Message::Text(text))) = event else { continue };
                let Ok(event): Result<serde_json::Value, Error> = serde_json::from_str(&text) else { continue };
                let Some(entity_id) = event.get("event").and_then(|e| e.get("data")).and_then(|d| d.get("entity_id")).and_then(|e| e.as_str()) else { continue };
                let Some(media_player) = media_players.get_mut(entity_id) else { continue };
                let Some(new_state) = event.get("event").and_then(|e| e.get("data")).and_then(|d| d.get("new_state")) else { continue };
                let Some(attr) = new_state.get("attributes") else { continue };
                let Some(state) = new_state.get("state") else { continue };
                match media_player
                    .update_metadata(
                        attr.clone(),
                        state.to_string().clone(),
                    )
                    .await {
                    Ok(events) => {
                        for e in events {
                            channels
                                .get(entity_id)
                                .unwrap()
                                .send(e)
                                .await?;
                        }
                    },
                    Err(e) => println!("Died during metadata update event with {e}"),
                };
            }
            Some((entity_id, msg)) = mpris_rx.recv() => {
                let media = media_players.get_mut(&entity_id);
                if let Some(mp) = media {
                    match msg {
                        HAEvent::Play=> mp.play().await?,
                        HAEvent::Pause=> mp.pause().await?,
                        HAEvent::Next => mp.next().await?,
                        HAEvent::Previous => mp.previous().await?,
                        _ => {},
                    };
                }
            }
        }
    }
}

fn validate_art_url(art_url: String, base_url: &str) -> eyre::Result<Url> {
    let parsed_url = url::Url::parse(&art_url);

    match parsed_url {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            let base = url::Url::parse(base_url)?;
            base.join(&art_url).map_err(|_e| eyre::eyre!("Oh no"))
        }
        Err(_e) => Err(eyre::eyre!("Oh no")),
    }
}
