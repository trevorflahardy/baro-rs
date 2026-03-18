use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct Config<'a> {
    pub internet: InternetConfig<'a>,
    pub device: DeviceConfig,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InternetConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
}

/// Which home page style to use
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HomePageMode {
    /// Status-first dashboard (banner + sorted sensor rows) for outdoor/backpack use
    #[default]
    Outdoor,
    /// 2x2 mini-graph grid with auto-cycling for stationary indoor use
    Home,
}

/// Device-level configuration that persists to SD card
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceConfig {
    pub home_page_mode: HomePageMode,
}
