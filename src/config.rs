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

    #[serde(rename = "BOSS_RESPAWN")]
    pub boss_respawn: Option<bool>,

    #[serde(rename = "BOSS_DIFFICULTY")]
    pub boss_difficulty: Option<f32>,

    #[serde(rename = "MASTER_HAND_CSS")]
    pub master_hand_css: Option<bool>,

    #[serde(rename = "CRAZY_HAND_CSS")]
    pub crazy_hand_css: Option<bool>,

    #[serde(rename = "DHARKON_CSS")]
    pub dharkon_css: Option<bool>,

    #[serde(rename = "GALEEM_CSS")]
    pub galeem_css: Option<bool>,

    #[serde(rename = "MARX_CSS")]
    pub marx_css: Option<bool>,

    #[serde(rename = "GIGA_BOWSER_CSS")]
    pub giga_bowser_css: Option<bool>,

    #[serde(rename = "GANON_CSS")]
    pub ganon_css: Option<bool>,

    #[serde(rename = "DRACULA_CSS")]
    pub dracula_css: Option<bool>,

    #[serde(rename = "RATHALOS_CSS")]
    pub rathalos_css: Option<bool>,

    #[serde(rename = "GALLEOM_CSS")]
    pub galleom_css: Option<bool>,

    #[serde(rename = "WOL_MASTER_HAND_CSS")]
    pub wol_master_hand_css: Option<bool>,

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
    let path = "sd:/ultimate/mods/Bosses/config.toml";

    match fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
            show_error(
                0x02,
                "Failed to parse config.toml",
                &format!(
                    "TOML parse error: {}\nCheck formatting at:\nsd:/ultimate/mods/Bosses/config.toml",
                    e
                ),
            );
            exit(0);
        }),
        Err(e) => {
            show_error(
                0x01,
                "Missing or unreadable config.toml for Competitive Playable Bosses",
                &format!(
                    "Error: {}\nExpected at:\nsd:/ultimate/mods/Bosses/config.toml",
                    e
                ),
            );
            exit(0);
        }
    }
}
