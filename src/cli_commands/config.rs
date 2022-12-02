/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

const CONF_FILE: &str = "pyrsia-cli";
const ENV_VAR_PYRSIA_CONFIG_LOCATION_FOR_TEST: &str = "PYRSIA_CONFIG_LOCATION_FOR_TEST";

struct ConfigLocation {
    config_path: PathBuf,
}

impl ConfigLocation {
    pub fn new() -> Result<Self, confy::ConfyError> {
        let config_path = if cfg!(test) {
            if let Ok(config_path_for_test) = std::env::var(ENV_VAR_PYRSIA_CONFIG_LOCATION_FOR_TEST)
            {
                PathBuf::from(config_path_for_test)
            } else {
                confy::get_configuration_file_path(CONF_FILE, None)?
            }
        } else {
            confy::get_configuration_file_path(CONF_FILE, None)?
        };
        println!("PATH = {}", config_path.display());
        Ok(Self { config_path })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CliConfig {
    pub host: String,
    pub port: String,
    pub disk_allocated: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            host: "localhost".to_string(),
            port: "7888".to_string(),
            disk_allocated: "10 GB".to_string(),
        }
    }
}

impl Display for CliConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config_toml = toml::to_string_pretty(&self).expect("toml format error");
        write!(f, "{}", config_toml)
    }
}

impl PartialEq for CliConfig {
    fn eq(&self, other: &Self) -> bool {
        self.host.as_str() == other.host.as_str()
            && self.port.as_str() == other.port.as_str()
            && self.disk_allocated.as_str() == other.disk_allocated.as_str()
    }
}

pub fn add_config(new_cfg: CliConfig) -> Result<(), confy::ConfyError> {
    let cfg_location = ConfigLocation::new()?;

    let mut cfg: CliConfig = confy::load_path(&cfg_location.config_path)?;
    if !new_cfg.host.is_empty() {
        cfg.host = new_cfg.host
    }

    if !new_cfg.port.is_empty() {
        cfg.port = new_cfg.port
    }
    // need more validation for checking units
    if !new_cfg.disk_allocated.is_empty() {
        cfg.disk_allocated = new_cfg.disk_allocated
    }

    confy::store_path(&cfg_location.config_path, &cfg)?;

    Ok(())
}

pub fn get_config() -> Result<CliConfig, confy::ConfyError> {
    let cfg_location = ConfigLocation::new()?;

    let cfg: CliConfig = confy::load_path(&cfg_location.config_path)?;

    Ok(cfg)
}

pub fn get_config_file_path() -> Result<PathBuf, confy::ConfyError> {
    confy::get_configuration_file_path(CONF_FILE, None)
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_config_file_update() {
        let tmp_dir = tempfile::tempdir()
            .expect("could not create temporary directory")
            .into_path();

        std::env::set_var(
            ENV_VAR_PYRSIA_CONFIG_LOCATION_FOR_TEST,
            tmp_dir.join("pyrsia-cli.config"),
        );

        let cli_config_1 = CliConfig {
            port: "7888".to_string(),
            ..Default::default()
        };
        let cli_config_2 = CliConfig {
            port: "7878".to_string(),
            ..Default::default()
        };

        add_config(cli_config_1.clone()).expect("add_config failed");
        let current_cli_config = get_config().expect("get_config failed");
        assert_eq!(current_cli_config.port, cli_config_1.port);

        add_config(cli_config_2.clone()).expect("add_config failed");
        let current_cli_config = get_config().expect("get_config failed");
        assert_eq!(current_cli_config.port, cli_config_2.port);

        std::env::remove_var(ENV_VAR_PYRSIA_CONFIG_LOCATION_FOR_TEST);
    }

    #[test]
    fn test_get_config_file_path() {
        let config_file_path = get_config_file_path();
        assert!(config_file_path.is_ok());
    }
}
