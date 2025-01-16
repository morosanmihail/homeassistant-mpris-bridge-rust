use std::{collections::HashMap, sync::Arc, time::Duration};

use eyre::{OptionExt, Result};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Error, Value};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};
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

#[derive(Debug, Clone)]
pub struct MediaPlayerMetadata {
    pub title: String,
    pub artist: String,
    pub duration: i64,
    pub position: i64,
    pub volume: f64,
    pub art_url: String,
    pub playing: bool,
    pub shuffle: bool,
    pub repeat: HALoopStatus,
}

#[derive(Debug, Clone)]
pub enum HALoopStatus {
    None,
    Track,
    Playlist,
}

#[derive(Debug)]
pub enum HAEvent {
    Play,
    Pause,
    MetadataUpdated(MediaPlayerMetadata),
    Next,
    Previous,
    Volume(f64),
    SetShuffle(bool),
    SetLoop(HALoopStatus),
    Seek(i64),
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
        repeat: match metadata
            .get("repeat")
            .unwrap_or(&json!("off"))
            .to_string()
            .trim_matches(['\"'])
        {
            "one" => HALoopStatus::Track,
            "all" => HALoopStatus::Playlist,
            _ => HALoopStatus::None,
        },
        shuffle: metadata
            .get("shuffle")
            .unwrap_or(&json!(false))
            .as_bool()
            .ok_or_eyre("Could not convert Bool to boolean")?,
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
        self.send_command_to_home_assistant("media_play", None)
            .await
    }

    pub async fn pause(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_pause", None)
            .await
    }

    pub async fn next(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_next_track", None)
            .await
    }

    pub async fn previous(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_previous_track", None)
            .await
    }

    pub async fn set_volume(&self, volume: f64) -> Result<()> {
        let mut extras = serde_json::Map::new();
        extras.insert("volume_level".to_string(), json!(volume));
        self.send_command_to_home_assistant("volume_set", Some(extras))
            .await
    }

    pub async fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        let mut extras = serde_json::Map::new();
        extras.insert("shuffle".to_string(), json!(shuffle));
        self.send_command_to_home_assistant("shuffle_set", Some(extras))
            .await
    }

    pub async fn set_loop(&self, loop_status: HALoopStatus) -> Result<()> {
        let mut extras = serde_json::Map::new();
        extras.insert(
            "repeat".to_string(),
            json!(match loop_status {
                HALoopStatus::None => "off",
                HALoopStatus::Track => "one",
                HALoopStatus::Playlist => "all",
            }),
        );
        self.send_command_to_home_assistant("repeat_set", Some(extras))
            .await
    }

    pub async fn set_seek(&self, position: i64) -> Result<()> {
        let mut extras = serde_json::Map::new();
        extras.insert("seek_position".to_string(), json!(position));
        self.send_command_to_home_assistant("media_seek", Some(extras))
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

    pub async fn send_command_to_home_assistant(
        &self,
        command: &str,
        extra_params: Option<serde_json::Map<String, Value>>,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/services/media_player/{}", self.ha_url, command);

        let mut params = serde_json::Map::new();
        params.insert(
            "entity_id".to_string(),
            Value::String(self.entity_id.clone()),
        );

        if let Some(extras) = extra_params {
            for (k, v) in extras {
                params.insert(k, v);
            }
        }

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
    mpris_rx: Arc<Mutex<Receiver<(String, HAEvent)>>>,
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
                let text = match event {
                    Some(Ok(Message::Text(t))) => t,
                    Some(Ok(Message::Close(_))) => break Err(eyre::eyre!("Channel closed")),
                    None => break Err(eyre::eyre!("Restarting websocket channel due to unknown reason")),
                    _ => continue
                };
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

            result = async {
                let mut guard = mpris_rx.lock().await;
                guard.recv().await
            } => {
                let Some((entity_id, msg)) = result else { continue };
                let media = media_players.get_mut(&entity_id);
                if let Some(mp) = media {
                    match msg {
                        HAEvent::Play=> mp.play().await?,
                        HAEvent::Pause=> mp.pause().await?,
                        HAEvent::Next => mp.next().await?,
                        HAEvent::Previous => mp.previous().await?,
                        HAEvent::Volume(v) => mp.set_volume(v).await?,
                        HAEvent::SetShuffle(s) => mp.set_shuffle(s).await?,
                        HAEvent::SetLoop(l) => mp.set_loop(l).await?,
                        HAEvent::Seek(p) => mp.set_seek(p).await?,
                        _ => {},
                    };
                }
            }

            _ = tokio::time::sleep(Duration::from_secs(60)) => {
                break Err(eyre::eyre!("Timed out"));
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
