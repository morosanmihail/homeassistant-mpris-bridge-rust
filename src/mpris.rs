use std::sync::Arc;

use mpris_server::{
    zbus::fdo, LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, RootInterface,
    Server, Time, TrackId, Volume,
};
use tokio::sync::Mutex;

use crate::homeassistant::MediaPlayerState;

#[derive(Clone)]
pub struct MyPlayer {
    state: Arc<Mutex<MediaPlayerState>>,
}

impl RootInterface for MyPlayer {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        todo!()
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn set_fullscreen(&self, fullscreen: bool) -> mpris_server::zbus::Result<()> {
        todo!()
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
        todo!()
    }

    async fn previous(&self) -> fdo::Result<()> {
        todo!()
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.state.lock().await.playing = false;
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.state.lock().await.playing = !self.state.lock().await.playing;
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        todo!()
    }

    async fn play(&self) -> fdo::Result<()> {
        self.state.lock().await.playing = true;
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        todo!()
    }

    async fn set_position(&self, track_id: TrackId, position: Time) -> fdo::Result<()> {
        todo!()
    }

    async fn open_uri(&self, uri: String) -> fdo::Result<()> {
        todo!()
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(match self.state.lock().await.playing {
            true => PlaybackStatus::Playing,
            false => PlaybackStatus::Paused,
        })
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::Track)
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::NEG_INFINITY)
    }

    async fn set_rate(&self, rate: PlaybackRate) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn set_shuffle(&self, shuffle: bool) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(Metadata::builder()
            .title("TEST")
            .artist(["TEST ARTIST"])
            .length(Time::from_secs(123))
            .build())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(Volume::MIN)
    }

    async fn set_volume(&self, volume: Volume) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::MIN)
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
        Ok(true)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}

pub async fn new_mpris_player(player: Arc<Mutex<MediaPlayerState>>) -> eyre::Result<()> {
    let _player = Server::new(
        player.lock().await.json_player.entity_id.as_str(),
        MyPlayer {
            state: player.clone(),
        },
    )
    .await?;

    loop {}
}
