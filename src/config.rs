use serde::Deserialize;
use std::fs;
use std::process::exit;
use skyline::error::show_error;

#[derive(Deserialize, Debug)]
pub struct Options {
    #[serde(rename = "GAME_VERSION")]
    pub game_version: Option<String>,

    #[serde(rename = "FULL_STUN_DURATION")]
    pub full_stun_duration: Option<bool>,

    #[serde(rename = "GIGA_BOWSER_NORMAL")]
    pub giga_bowser_normal: Option<bool>,

    #[serde(rename = "CUSTOM_CSS")]
    pub custom_css: Option<bool>,

    #[serde(rename = "NO_BOSS_STAGES")]
    pub no_boss_stages: Option<bool>,

    #[serde(rename = "NO_FINAL_BOSS_STAGES")]
    pub no_final_boss_stages: Option<bool>,

    #[serde(rename = "BOSS_DIFFICULTY")]
    pub boss_difficulty: Option<f32>,

    #[serde(rename = "MASTER_HAND_HP")]
    pub master_hand_hp: Option<f32>,

    #[serde(rename = "CRAZY_HAND_HP")]
    pub crazy_hand_hp: Option<f32>,

    #[serde(rename = "DHARKON_HP")]
    pub dharkon_hp: Option<f32>,

    #[serde(rename = "GALEEM_HP")]
    pub galeem_hp: Option<f32>,

    #[serde(rename = "MARX_HP")]
    pub marx_hp: Option<f32>,

    #[serde(rename = "GIGA_BOWSER_HP")]
    pub giga_bowser_hp: Option<f32>,

    #[serde(rename = "GANON_HP")]
    pub ganon_hp: Option<f32>,

    #[serde(rename = "DRACULA_PHASE_1_HP")]
    pub dracula_phase_1_hp: Option<f32>,

    #[serde(rename = "DRACULA_PHASE_2_HP")]
    pub dracula_phase_2_hp: Option<f32>,

    #[serde(rename = "RATHALOS_HP")]
    pub rathalos_hp: Option<f32>,

    #[serde(rename = "GALLEOM_HP")]
    pub galleom_hp: Option<f32>,

    #[serde(rename = "WOL_MASTER_HAND_HP")]
    pub wol_master_hand_hp: Option<f32>,

    #[serde(rename = "DHARKON_RAGE_HP")]
    pub dharkon_rage_hp: Option<f32>,

    #[serde(rename = "GALEEM_RAGE_HP")]
    pub galeem_rage_hp: Option<f32>,

    #[serde(rename = "GALLEOM_RAGE_HP")]
    pub galleom_rage_hp: Option<f32>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub options: Options,
}

pub fn load_config() -> Config {
    let default = Config {
        options: Options {
            game_version: Some("13.0.4".to_string()),
            full_stun_duration: Some(false),
            giga_bowser_normal: Some(false),
            custom_css: Some(false),
            no_boss_stages: Some(false),
            no_final_boss_stages: Some(false),
            boss_difficulty: Some(10.0),
            master_hand_hp: Some(400.0),
            crazy_hand_hp: Some(400.0),
            dharkon_hp: Some(400.0),
            galeem_hp: Some(400.0),
            marx_hp: Some(400.0),
            giga_bowser_hp: Some(600.0),
            ganon_hp: Some(600.0),
            dracula_phase_1_hp: Some(160.0),
            dracula_phase_2_hp: Some(500.0),
            rathalos_hp: Some(600.0),
            galleom_hp: Some(700.0),
            wol_master_hand_hp: Some(400.0),
            dharkon_rage_hp: Some(220.0),
            galeem_rage_hp: Some(220.0),
            galleom_rage_hp: Some(220.0),
        },
    };

    let path = "sd:/ultimate/mods/Bosses/config.toml";

    match fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or(default),
        Err(e) => {
            show_error(
                0x01,
                "Missing or unreadable config.toml for Competitive Playable Bosses",
                &format!("Error: {}\nExpected at:\nsd:/ultimate/mods/Bosses/config.toml", e),
            );
            exit(0);
        }
    }
}
