use serde::{Deserialize, Serialize};
use crate::masterhand::ReadableConfig;
use crate::masterhand::WriteableConfig;
use std::fs::File;
use std::{fs, vec};
use parking_lot::RwLock;
use std::io::Write;
mod readable;
mod writeable;
use readable::*;
use writeable::*;

const CONFIG_PATH: &str = "sd:/ultimate/mods/boss/boss.toml";

lazy_static::lazy_static! {
    pub static ref CONFIG: Configuration = Configuration::new();
}

pub struct Configuration(RwLock<Config>);

impl<'rwlock> Configuration {
    pub fn new() -> Self {
        Self(RwLock::new(Config::open().unwrap()))
    }

    pub fn read(&self) -> ReadableConfig<'_> {
        ReadableConfig::new(self.0.read())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub table: Table,
    pub player_1: Player,
    pub player_2: Player,
    pub player_3: Player,
    pub player_4: Player,
    pub player_5: Player,
    pub player_6: Player,
    pub player_7: Player,
    pub player_8: Player,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Table {
    pub MASTERHAND: i32,
    pub CRAZYHAND: i32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Player {
    pub UP: i32,
    pub SIDE_L: i32,
    pub SIDE_R: i32,
    pub DOWN: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            table: Table {
                MASTERHAND: 0,
                CRAZYHAND: 1,
            },
            player_1: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
            player_2: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
            player_3: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
            player_4: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
            player_5: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
            player_6: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
            player_7: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
            player_8: Player {
                UP: -1,
                SIDE_L: -1,
                SIDE_R: -1,
                DOWN: Some(String::from("ARSENE, default & compulsory")),
            },
        }
    }

    pub fn open() -> Result<Config, String> {
        match fs::read_to_string(CONFIG_PATH) {
            Ok(content) => {
                match toml::from_str(&content) {
                    Ok(conf) => conf,
                    Err(error) => {
                        println!("nope1");
                        println!("{}",error);
                        let config = Config::new();
                        config.save().unwrap();
                        Ok(config)
                    }
                }
            }
            Err(_) => {
                println!("nope2");
                let config = Config::new();
                config.save().unwrap();
                Ok(config)
            }
        }
    }

    fn save(&self) -> Result<(), std::io::Error> {
        let config_txt = toml::to_vec(&self).unwrap();
        let mut file = match File::create(CONFIG_PATH) {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        match file.write_all(&config_txt) {
            Ok(_) => {}
            Err(err) => return Err(err),
        }
        println!("[Playable_Bosses::Config] Boss.toml configuration file successfully created!");
        Ok(())
    }
}