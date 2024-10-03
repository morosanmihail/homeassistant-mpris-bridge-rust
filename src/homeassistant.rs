use std::collections::HashMap;

use eyre::Result;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Deserialize, Debug, Clone)]
pub struct MediaPlayer {
    pub entity_id: String,
    pub state: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MediaPlayerState {
    ha_url: String,
    ha_token: String,
    pub playing: bool,
    pub json_player: MediaPlayer,
}

impl MediaPlayerState {
    pub fn new(player: MediaPlayer, ha_url: String, ha_token: String) -> Result<Self> {
        Ok(Self {
            ha_token,
            ha_url,
            json_player: player,
            playing: false,
        })
    }

    pub async fn send_command_to_home_assistant(
        &self,
        client: &Client,
        command: &str,
    ) -> Result<()> {
        let url = format!("{}/api/services/media_player/{}", self.ha_url, command);
        let params = serde_json::json!({
            "entity_id": self.json_player.entity_id,
        });

        client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.ha_token))
            .json(&params)
            .send()
            .await?;

        Ok(())
    }
}

pub async fn get_media_players(
    client: &Client,
    home_assistant_url: &str,
    token: &str,
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
        .collect();
    Ok(media_players)
}

pub async fn listen_for_events(ha_url: String, access_token: String) -> Result<()> {
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
    while let Some(Ok(message)) = read.next().await {
        if let Message::Text(text) = message {
            let event: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
            //match event["event"]["data"]["entity_id"].as_str() {
            //    Some("media_player.living_room_2") => println!("Heya!"),
            //    _ => println!("Not heya!"),
            //};
            //println!("Received event: {}", event);
        }
    }
    Ok(())
}
