use mpris_server::{
    zbus::fdo, LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property,
    RootInterface, Server, Time, TrackId, Volume,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::homeassistant::{HAEvent, MediaPlayer};

#[derive(Clone)]
pub struct MyPlayer {
    ha_sender: tokio::sync::mpsc::Sender<HAEvent>,
    pub start_state: MediaPlayer,
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
        Ok("AAAAH".to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["uh".to_string()])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["uh".to_string()])
    }
}

impl PlayerInterface for MyPlayer {
    async fn next(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        let _ = self.ha_sender.send(HAEvent::Pause).await;
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        let _ = self.ha_sender.send(HAEvent::Play).await;
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        let _ = self.ha_sender.send(HAEvent::Play).await;
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
        Ok(PlaybackStatus::Paused)
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::Track)
    }

    async fn set_loop_status(&self, _loop_status: LoopStatus) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::NEG_INFINITY)
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
            .unwrap()
            .to_string();
        let artist = self
            .start_state
            .attributes
            .get("media_artist")
            .unwrap()
            .to_string();
        let duration = self
            .start_state
            .attributes
            .get("media_duration")
            .unwrap()
            .as_i64()
            .unwrap();
        Ok(Metadata::builder()
            .title(title.trim_matches(['\"']))
            .artist(vec![artist.trim_matches(['\"'])])
            .length(Time::from_secs(duration))
            .build())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(Volume::MIN)
    }

    async fn set_volume(&self, _volume: Volume) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::ZERO)
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::MIN_POSITIVE)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::MIN_POSITIVE)
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
    mut rx: Receiver<HAEvent>,
    ha_sender: Sender<HAEvent>,
) -> eyre::Result<()> {
    let player = Server::new(
        &entity_id,
        MyPlayer {
            start_state,
            ha_sender,
        },
    )
    .await?;

    loop {
        if let Some(i) = rx.recv().await {
            println!("Got an event! {:?}", i);
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
                HAEvent::MetadataUpdated((title, artist, duration, position)) => {
                    player
                        .properties_changed([
                            Property::CanSeek(false),
                            Property::Metadata(
                                Metadata::builder()
                                    .title(title.trim_matches(['\"']))
                                    .artist(vec![artist.trim_matches(['\"'])])
                                    .length(Time::from_secs(duration))
                                    .build(),
                            ),
                        ])
                        .await?;

                    player
                        .emit(mpris_server::Signal::Seeked {
                            position: Time::from_secs(position),
                        })
                        .await?;
                }
            }
        }
    }
}
