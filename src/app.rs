use crate::{
    chess::Side,
    config::Config,
    game::{Game, GameData, GameState},
    message::Message,
    ui::UIState,
};

use futures::stream::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use std::sync::mpsc::Sender;

#[derive(Deserialize, Debug, Clone)]
pub struct OwnInfo {
    id: String,
    username: String,
    online: bool,
}

impl OwnInfo {
    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn username(&self) -> &String {
        &self.username
    }

    pub fn online(&self) -> &bool {
        &self.online
    }
}

pub struct App {
    game: Option<Game>,
    own_info: OwnInfo,
    config: Config,
    main_tx: Sender<Message>,
    ui_state: UIState,
}

impl App {
    pub async fn new(main_tx: Sender<Message>) -> Result<Self, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        let config = Config::new().unwrap();

        let token = format!("Bearer {}", config.token());

        let res = client
            .get("https://lichess.org/api/account")
            .header("Authorization", token)
            .send()
            .await?
            .text()
            .await?
            .to_string();

        Ok(Self {
            game: None,
            main_tx,
            config,
            own_info: serde_json::from_str(&res).unwrap(),
            ui_state: UIState::Menu,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn game(&self) -> &Option<Game> {
        &self.game
    }

    pub fn game_mut(&mut self) -> &mut Option<Game> {
        &mut self.game
    }

    pub async fn seek_for_game(&self) {
        let token = format!("Bearer {}", self.config.token());

        tokio::spawn(async move {
            let client = reqwest::Client::new();

            let mut stream = client
                .post("https://lichess.org/api/board/seek?time=10&increment=0")
                .header("Authorization", token)
                .header("Content-Type", "text/plain")
                .send()
                .await
                .unwrap()
                .bytes_stream();

            while let Some(ev) = stream.next().await {
                let e = ev.unwrap();
                if e.len() > 1 {
                    panic!("{:#?}", e);
                }
            }
        });
    }

    pub fn init_new_game<T: ToString>(&mut self, id: T) {
        let id = id.to_string();
        //self.game = Some(Game::new(Board::default(), id.clone()));
        //self.ui_state = UIState::Game;

        let tx = self.main_tx.clone();

        let token = format!("Bearer {}", self.config.token());

        tokio::spawn(async move {
            let path = format!(
                "https://lichess.org/api/board/game/stream/{}",
                id.to_string()
            );

            let client = reqwest::Client::new();

            let mut stream = client
                .get(path)
                .header("Authorization", token)
                .send()
                .await
                .unwrap()
                .bytes_stream();

            while let Some(ev) = stream.next().await {
                let ev = ev.unwrap();
                let data = String::from_utf8(ev.to_vec()).unwrap();

                if data.len() > 1 {
                    let json: Value = serde_json::from_str(&data).unwrap();

                    match json["type"].as_str().unwrap() {
                        "gameFull" => {
                            let state: GameState =
                                serde_json::from_value(json["state"].clone()).unwrap();

                            let data: GameData = serde_json::from_value(json).unwrap();
                            let game = Game::new(id.clone(), data, state);
                            tx.send(Message::GameDataInit(game)).unwrap();
                        }
                        _ => (),
                    }
                }
            }

            // debugging
            /*let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/home/quasar/.game_data")
            .unwrap();

            while let Some(ev) = stream.next().await {
                let ev = ev.unwrap().to_vec();
                file.write_all(&ev).unwrap();
            }*/
        });
    }

    pub fn ui_state(&self) -> &UIState {
        &self.ui_state
    }

    pub fn set_ui_state(&mut self, state: UIState) {
        self.ui_state = state;
    }

    pub fn start_game(&mut self, game: Game) {
        self.game = Some(game);
        self.ui_state = UIState::Game;
    }

    pub fn check_own_side(&self) -> Side {
        let game = self.game().as_ref().unwrap();

        let w = game.data().white();

        if w.id() == self.own_info.id() {
            return Side::White;
        }

        return Side::Black;
    }
}

/*impl Default for App {
    fn default() -> Self {
        Self {
            game: None,
            config: Config::new().unwrap(),
            ui_state: UIState::Menu,
        }
    }
}*/
