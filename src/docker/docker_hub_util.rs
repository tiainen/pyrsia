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

use crate::util::error_util::NodeError;
use reqwest::get;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Bearer {
    token: String,
    expires_in: u64,
}

pub async fn get_docker_hub_auth_token(name: &str) -> Result<String, NodeError> {
    let auth_url = format!("https://auth.docker.io/token?client_id=Pyrsia&service=registry.docker.io&scope=repository:library/{}:pull", name);

    let token: Bearer = get(auth_url)
        .await?
        .json()
        .await?;

    Ok(token.token)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! async_test {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_get_docker_hub_auth_token() -> Result<(), NodeError> {
        let name = "alpine";
        let result = async_test!(get_docker_hub_auth_token(name));
        check_get_docker_hub_auth_token(result);
        Ok(())
    }

    fn check_get_docker_hub_auth_token(result: Result<String, NodeError>) {
        match result {
            Ok(token) => {
                assert!(token.len() > 0);
            }
            Err(_) => {
                assert!(false)
            }
        };
    }
}
