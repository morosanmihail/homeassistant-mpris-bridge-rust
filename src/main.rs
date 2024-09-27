use eyre::Result;
use homeassistant::{get_media_players, listen_for_events};
use reqwest::Client;
use tokio::task::JoinSet;

mod homeassistant;

//fn create_mpris_player(name: String) -> Player {
//    // Find or create
//    match PlayerFinder::new().expect("Need it").find_by_name(&name) {
//        Ok(p) => p,
//        Err(err) => Player::new(name).unwrap(),
//    }
//}

async fn send_command_to_home_assistant(
    client: &Client,
    home_assistant_url: &str,
    token: &str,
    entity_id: &str,
    command: &str,
) -> Result<()> {
    let url = format!(
        "{}/api/services/media_player/{}",
        home_assistant_url, command
    );
    let params = serde_json::json!({
        "entity_id": entity_id,
    });

    client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&params)
        .send()
        .await?;

    Ok(())
}

//async fn main_loop(client: &Client, home_assistant_url: &str, token: &str, media_players: Vec<MediaPlayer>) -> Result<()> {
//    for player in media_players {
//        if player.entity_id.starts_with("media_player.") {
//            let name = player.entity_id.replace("media_player.", "");
//            let mplayer = create_mpris_player(name.clone());
//
//            // Set initial playback status based on the state
//            match player.state.as_str() {
//                "playing" => mplayer.set_playback_status(PlaybackStatus::Playing),
//                "paused" => mplayer.set_playback_status(PlaybackStatus::Paused),
//                "stopped" => mplayer.set_playback_status(PlaybackStatus::Stopped),
//                _ => (),
//            }
//
//            println!("Created MPRIS player for: {}", &name);
//
//            let client_clone = client.clone();
//            let home_assistant_url_clone = home_assistant_url.to_string();
//            let token_clone = token.to_string();
//            let entity_id_clone = player.entity_id.clone();
//            let name_clone = name.clone();
//
//            // tokio::spawn(async move {
//            //     let player = PlayerFinder::new().expect("Need it").find_by_name(name_clone).unwrap(); // TODO remove expect and unwrap
//            //     let mut events = player.events().unwrap();
//            //     while let Some(event) = events.next().await {
//            //         match event {
//            //             PlayerEvent::Play => {
//            //                 send_command_to_home_assistant(&client_clone, &home_assistant_url_clone, &token_clone, &entity_id_clone, "media_play").await.unwrap();
//            //             }
//            //             PlayerEvent::Pause => {
//            //                 send_command_to_home_assistant(&client_clone, &home_assistant_url_clone, &token_clone, &entity_id_clone, "media_pause").await.unwrap();
//            //             }
//            //             PlayerEvent::Next => {
//            //                 send_command_to_home_assistant(&client_clone, &home_assistant_url_clone, &token_clone, &entity_id_clone, "media_next_track").await.unwrap();
//            //             }
//            //             _ => {}
//            //         }
//            //     }
//            // });
//        }
//    }
//
//    Ok(())
//}

#[tokio::main]
async fn main() -> Result<()> {
    let home_assistant_url = "http://192.168.1.27:8123";
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiIzYWE5OTY2ZTkyZTc0NTg5ODE0ZDJmYTFkOWUxMTMyOSIsImlhdCI6MTcxODY1MTY1MCwiZXhwIjoyMDM0MDExNjUwfQ._-CbAO3yTtIfuu9hsSvpekb7Oy_VhFps8F4YJHbMdPs";

    let client = Client::new();
    let media_players = get_media_players(&client, home_assistant_url, token)
        .await
        .unwrap();

    for player in media_players {
        //println!("{:?}\n", player);

        // for each configured media player (that we follow)
        // create one loop that listens to mpris events and sends them back to HA
    }

    let mut set = JoinSet::new();

    let _ha_task = set.spawn(listen_for_events(
        "ws://192.168.1.27:8123/api/websocket".to_string(),
        token.to_string(),
    ));

    while let _ = set.join_next().await {}
    Ok(())
}
