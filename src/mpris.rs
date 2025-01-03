use std::sync::Arc;

use eyre::OptionExt;
use mpris_server::{
    zbus::fdo, LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property,
    RootInterface, Server, Time, TrackId, Volume,
};
use serde_json::json;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};
use url::Url;

use crate::homeassistant::{HAEvent, MediaPlayer};

#[derive(Clone)]
pub struct MyPlayer {
    base_url: String,
    entity_id: String,
    ha_sender: tokio::sync::mpsc::Sender<(String, HAEvent)>,
    pub start_state: MediaPlayer,
    position: Arc<Mutex<i64>>,
}

impl RootInterface for MyPlayer {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok("MyPlayer".into())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("HomeAssistantPlayer".to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl PlayerInterface for MyPlayer {
    async fn next(&self) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Next))
            .await;
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Previous))
            .await;
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Pause))
            .await;
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Play))
            .await;
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Play))
            .await;
        Ok(())
    }

    async fn seek(&self, _offset: Time) -> fdo::Result<()> {
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, _position: Time) -> fdo::Result<()> {
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Ok(())
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        if self.start_state.state.contains("playing") {
            Ok(PlaybackStatus::Playing)
        } else {
            Ok(PlaybackStatus::Paused)
        }
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::Track)
    }

    async fn set_loop_status(&self, _loop_status: LoopStatus) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn set_shuffle(&self, _shuffle: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        let title = self
            .start_state
            .attributes
            .get("media_title")
            .unwrap_or(&json!(""))
            .to_string();
        let artist = self
            .start_state
            .attributes
            .get("media_artist")
            .unwrap_or(&json!(""))
            .to_string();
        let duration = self
            .start_state
            .attributes
            .get("media_duration")
            .unwrap_or(&json!(0))
            .as_i64()
            .unwrap();
        let art = self
            .start_state
            .attributes
            .get("entity_picture")
            .unwrap_or(&json!(""))
            .to_string();
        Ok(Metadata::builder()
            .title(title.trim_matches(['\"']))
            .artist(vec![artist.trim_matches(['\"'])])
            .length(Time::from_secs(duration))
            .art_url(
                validate_art_url(art.trim_matches(['\"']).to_string(), self.base_url.clone())
                    .unwrap_or(
                        Url::parse("http://example.com").expect("Default URL is always valid"),
                    )
                    .to_string(),
            )
            .build())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(Volume::MIN)
    }

    async fn set_volume(&self, _volume: Volume) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_secs(*self.position.lock().await))
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}

pub async fn new_mpris_player(
    entity_id: String,
    start_state: MediaPlayer,
    base_url: String,
    mut rx: Receiver<HAEvent>,
    ha_sender: Sender<(String, HAEvent)>,
) -> eyre::Result<()> {
    let duration = start_state
        .attributes
        .get("media_position")
        .unwrap_or(&json!(0))
        .as_i64()
        .ok_or_eyre("Could not convert Number to i64")?;
    let position_lock = Arc::new(Mutex::new(duration));
    let media_player = MyPlayer {
        base_url: base_url.clone(),
        entity_id: entity_id.clone(),
        start_state,
        ha_sender,
        position: position_lock.clone(),
    };
    let player = Server::new(&entity_id.clone(), media_player).await?;

    loop {
        if let Some(i) = rx.recv().await {
            match i {
                HAEvent::Play => {
                    player
                        .properties_changed([Property::PlaybackStatus(PlaybackStatus::Playing)])
                        .await?;
                }
                HAEvent::Pause => {
                    player
                        .properties_changed([Property::PlaybackStatus(PlaybackStatus::Paused)])
                        .await?;
                }
                HAEvent::MetadataUpdated((title, artist, duration, position, art_url)) => {
                    player
                        .properties_changed([
                            Property::CanSeek(false),
                            Property::Metadata(
                                Metadata::builder()
                                    .title(title.trim_matches(['\"']))
                                    .artist(vec![artist.trim_matches(['\"'])])
                                    .length(Time::from_secs(duration))
                                    .art_url(
                                        validate_art_url(
                                            art_url.trim_matches(['\"']).to_string(),
                                            base_url.clone(),
                                        )?
                                        .to_string(),
                                    )
                                    .build(),
                            ),
                        ])
                        .await?;
                    {
                        let mut pos = position_lock.lock().await;
                        *pos = position;
                    }
                    player
                        .emit(mpris_server::Signal::Seeked {
                            position: Time::from_secs(position),
                        })
                        .await?;
                }
                _ => {}
            }
        }
    }
}

fn validate_art_url(art_url: String, base_url: String) -> eyre::Result<Url> {
    let parsed_url = url::Url::parse(&art_url);

    match parsed_url {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            let base = url::Url::parse(&base_url)?;
            base.join(&art_url).map_err(|_e| eyre::eyre!("Oh no"))
        }
        Err(_e) => Err(eyre::eyre!("Oh no")),
    }
}
