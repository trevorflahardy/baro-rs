use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct Config<'a> {
    pub internet: InternetConfig<'a>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InternetConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
}
