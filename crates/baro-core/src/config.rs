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

/// Temperature display unit
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

impl TemperatureUnit {
    /// Convert a Celsius value to this unit
    pub fn convert(self, celsius: f32) -> f32 {
        match self {
            Self::Celsius => celsius,
            Self::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
        }
    }

    /// Display suffix
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Celsius => "C",
            Self::Fahrenheit => "F",
        }
    }

    /// Unit label with degree symbol
    pub const fn unit_label(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
        }
    }
}

/// Device-level configuration that persists to SD card
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceConfig {
    pub home_page_mode: HomePageMode,
    pub temperature_unit: TemperatureUnit,
}
