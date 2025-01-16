use std::sync::Arc;

use mpris_server::{
    zbus::fdo, LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property,
    RootInterface, Server, Time, TrackId, Volume,
};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use crate::homeassistant::{
    json_to_metadata, HAEvent, HALoopStatus, MediaPlayer, MediaPlayerMetadata,
};

#[derive(Clone)]
pub struct MyPlayer {
    entity_id: String,
    ha_sender: tokio::sync::mpsc::Sender<(String, HAEvent)>,
    metadata: Arc<Mutex<MediaPlayerMetadata>>,
}

impl RootInterface for MyPlayer {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
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
        Ok(false)
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
            .send((
                self.entity_id.clone(),
                if self.metadata.lock().await.playing {
                    HAEvent::Pause
                } else {
                    HAEvent::Play
                },
            ))
            .await;
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Pause))
            .await;
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Play))
            .await;
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Seek(offset.as_secs())))
            .await;
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Seek(position.as_secs())))
            .await;
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Ok(())
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        if self.metadata.lock().await.playing {
            Ok(PlaybackStatus::Playing)
        } else {
            Ok(PlaybackStatus::Paused)
        }
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(match self.metadata.lock().await.repeat {
            HALoopStatus::None => LoopStatus::None,
            HALoopStatus::Track => LoopStatus::Track,
            HALoopStatus::Playlist => LoopStatus::Playlist,
        })
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> mpris_server::zbus::Result<()> {
        let _ = self
            .ha_sender
            .send((
                self.entity_id.clone(),
                HAEvent::SetLoop(match loop_status {
                    LoopStatus::None => HALoopStatus::None,
                    LoopStatus::Track => HALoopStatus::Track,
                    LoopStatus::Playlist => HALoopStatus::Playlist,
                }),
            ))
            .await;

        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(self.metadata.lock().await.shuffle)
    }

    async fn set_shuffle(&self, shuffle: bool) -> mpris_server::zbus::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::SetShuffle(shuffle)))
            .await;
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        let metadata = self.metadata.lock().await;
        Ok(Metadata::builder()
            .title(metadata.title.clone())
            .artist(vec![metadata.artist.clone()])
            .length(Time::from_secs(metadata.duration))
            .art_url(metadata.art_url.clone())
            .build())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(self.metadata.lock().await.volume)
    }

    async fn set_volume(&self, volume: Volume) -> mpris_server::zbus::Result<()> {
        let _ = self
            .ha_sender
            .send((self.entity_id.clone(), HAEvent::Volume(volume)))
            .await;
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_secs(self.metadata.lock().await.position))
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
        Ok(true)
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
    let metadata = json_to_metadata(
        start_state.attributes,
        start_state.state.contains("playing"),
        base_url.clone(),
    )?;

    let metadata_lock = Arc::new(Mutex::new(metadata));
    let media_player = MyPlayer {
        entity_id: entity_id.clone(),
        ha_sender,
        metadata: metadata_lock.clone(),
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
                HAEvent::MetadataUpdated(metadata_update) => {
                    player
                        .properties_changed([
                            Property::Metadata(
                                Metadata::builder()
                                    .title(&metadata_update.title)
                                    .artist(vec![&metadata_update.artist])
                                    .length(Time::from_secs(metadata_update.duration))
                                    .art_url(
                                        metadata_update.art_url.trim_matches(['\"']).to_string(),
                                    )
                                    .build(),
                            ),
                            Property::CanSeek(true),
                            Property::LoopStatus(match metadata_update.repeat {
                                HALoopStatus::None => LoopStatus::None,
                                HALoopStatus::Playlist => LoopStatus::Playlist,
                                HALoopStatus::Track => LoopStatus::Track,
                            }),
                            Property::Shuffle(metadata_update.shuffle),
                        ])
                        .await?;
                    {
                        let mut metadata = metadata_lock.lock().await;
                        *metadata = metadata_update.clone();
                    }
                }
                _ => {}
            }
        }
    }
}
