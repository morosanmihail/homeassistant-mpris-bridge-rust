use std::collections::HashMap;

use eyre::{OptionExt, Result};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Deserialize, Debug, Clone)]
pub struct MediaPlayer {
    pub entity_id: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MediaPlayerState {
    ha_url: String,
    ha_token: String,
    pub entity_id: String,
}

#[derive(Debug)]
pub enum HAEvent {
    Play,
    Pause,
    MetadataUpdated((String, String, i64, i64)),
}

impl MediaPlayerState {
    pub fn new(entity_id: String, ha_url: String, ha_token: String) -> Result<Self> {
        Ok(Self {
            ha_token,
            ha_url,
            entity_id,
        })
    }

    pub async fn play(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_play").await
    }

    pub async fn pause(&self) -> Result<()> {
        self.send_command_to_home_assistant("media_pause").await
    }

    pub async fn update_metadata(
        &mut self,
        metadata: serde_json::value::Value,
        state: String,
    ) -> Result<Vec<HAEvent>> {
        let attribs: HashMap<String, serde_json::Value> =
            if let serde_json::Value::Object(m) = metadata {
                m.into_iter().collect()
            } else {
                return Ok(vec![]);
            };

        let mut events = vec![];

        if state.eq("\"playing\"") {
            events.push(HAEvent::Play);
        } else {
            events.push(HAEvent::Pause);
        }
        events.push(HAEvent::MetadataUpdated((
            attribs
                .get("media_title")
                .unwrap_or(&Value::String("".to_string()))
                .to_string(),
            attribs
                .get("media_artist")
                .unwrap_or(&Value::String("".to_string()))
                .to_string(),
            attribs
                .get("media_duration")
                .unwrap_or(&Value::Number(serde_json::Number::from_f64(0.0).unwrap()))
                .as_i64()
                .ok_or_eyre("Could not convert Number to i64")?,
            attribs
                .get("media_position")
                .unwrap_or(&Value::Number(serde_json::Number::from_f64(0.0).unwrap()))
                .as_i64()
                .ok_or_eyre("Could not convert Number to i64")?,
        )));
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
    client: &Client,
    home_assistant_url: &str,
    token: &str,
    entity_ids: Vec<String>,
) -> Result<Vec<MediaPlayer>> {
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
    mut mpris_rx: Receiver<HAEvent>,
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
            Some(Ok(message)) = read.next() => {
                if let Message::Text(text) = message {
                    let event: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
                    let entity = event.get("event").and_then(|e| e.get("data")).and_then(|d| d.get("entity_id")).and_then(|e| e.as_str());
                    if let Some(entity_id) = entity
                    {
                        if let Some(media_player) = media_players.get_mut(entity_id) {

                        let events = media_player
                            .update_metadata(
                                // TODO: fix these to now error
                                event["event"]["data"]["new_state"]["attributes"].clone(),
                                event["event"]["data"]["new_state"]["state"]
                                    .to_string()
                                    .clone(),
                            )
                            .await?;

                        for e in events {
                            channels
                                .get(entity_id)
                                .unwrap()
                                .send(e)
                                .await?;
                        }
                        }
                    }
                }
            }
            Some(msg) = mpris_rx.recv() => {
                match msg {
        HAEvent::Play => media_players.get_mut("media_player.living_room_2").unwrap().play().await?,
        HAEvent::Pause => media_players.get_mut("media_player.living_room_2").unwrap().pause().await?,
        HAEvent::MetadataUpdated(_) => todo!(),
                };
            }
        }
    }
}
