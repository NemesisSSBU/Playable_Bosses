use serde::Deserialize;
use std::fs;
use std::process::exit;
use skyline::error::show_error;
use once_cell::sync::Lazy;

#[derive(Deserialize, Debug)]
pub struct Options {
    #[serde(rename = "FULL_STUN_DURATION")]
    pub full_stun_duration: Option<bool>,
    #[serde(rename = "GIGA_BOWSER_NORMAL")]
    pub giga_bowser_normal: Option<bool>,
    #[serde(rename = "WOL_MASTER_HAND_NORMAL")]
    pub wol_master_hand_normal: Option<bool>,
    #[serde(rename = "CUSTOM_CSS")]
    pub custom_css: Option<bool>,
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

    #[serde(rename = "MARX_STAGE")]
    pub marx_stage: Option<bool>,
    #[serde(rename = "GANON_STAGE")]
    pub ganon_stage: Option<bool>,
    #[serde(rename = "DRACULA_STAGE")]
    pub dracula_stage: Option<bool>,
    #[serde(rename = "GALLEOM_STAGE")]
    pub galleom_stage: Option<bool>,
    #[serde(rename = "RATHALOS_STAGE")]
    pub rathalos_stage: Option<bool>,
    #[serde(rename = "FINAL2_STAGE")]
    pub final2_stage: Option<bool>,
    #[serde(rename = "FINAL3_STAGE")]
    pub final3_stage: Option<bool>,

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

fn find_config_path() -> Option<String> {
    let default_path = "sd:/ultimate/mods/Bosses/config.toml";
    if fs::metadata(default_path).is_ok() {
        return Some(default_path.to_string());
    }
    let mods_root = "sd:/ultimate/mods";
    let mut preferred = Vec::new();
    let mut others = Vec::new();
    if let Ok(entries) = fs::read_dir(mods_root) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                let candidate = format!("{}/config.toml", entry.path().to_string_lossy());

                if fs::metadata(&candidate).is_ok() {
                    if dir_name.contains("boss") || dir_name.contains("comp_boss") {
                        preferred.push(candidate.clone());
                    } else {
                        others.push(candidate.clone());
                    }
                }
            }
        }
    }
    let try_parse = |p: &str| -> Option<String> {
        if let Ok(contents) = fs::read_to_string(p) {
            if toml::from_str::<Config>(&contents).is_ok() {
                return Some(p.to_string());
            }
        }
        None
    };
    for p in preferred.iter().chain(others.iter()) {
        if let Some(ok) = try_parse(p) {
            return Some(ok);
        }
    }
    let fallbacks = [
        "sd:/ultimate/comp_boss/config.toml",
        "sd:/config/comp_boss/config.toml",
        "sd:/comp_boss/config.toml",
    ];
    for p in &fallbacks {
        if let Some(ok) = try_parse(p) {
            return Some(ok);
        }
    }
    None
}

pub fn load_config() -> Config {
    let searched_hint = "\
- sd:/ultimate/mods/Bosses/config.toml
- sd:/ultimate/mods/*/config.toml   (boss-like folder names preferred)
- sd:/ultimate/comp_boss/config.toml
- sd:/config/comp_boss/config.toml
";
    let path = match find_config_path() {
        Some(p) => p,
        None => {
            show_error(
                0x01,
                "Missing config.toml for Competitive Playable Bosses",
                &format!(
                    "I couldn’t find a config.toml for this mod.\nSearched:\n{}\n\
                     Put your config.toml inside your mod’s folder (any name), or one of the fallback locations above.",
                    searched_hint
                ),
            );
            exit(0);
        }
    };
    match fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
            show_error(
                0x02,
                "Failed to parse config.toml",
                &format!(
                    "TOML parse error: {}\nCheck formatting at:\n{}",
                    e, path
                ),
            );
            exit(0);
        }),
        Err(e) => {
            show_error(
                0x01,
                "Unreadable config.toml for Competitive Playable Bosses",
                &format!(
                    "Error: {}\nTried to read:\n{}",
                    e, path
                ),
            );
            exit(0);
        }
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(load_config);