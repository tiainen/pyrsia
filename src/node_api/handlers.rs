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

use crate::network::app_state::AppState;
use crate::node_manager::{handlers::*, model::cli::Status};
use crate::util::error_util::NodeError;

use actix_web::{get, HttpResponse, Responder, web};
use log::debug;

#[get("/peers")]
async fn peers(data: web::Data<AppState>) -> impl Responder {
    let p2p_peers = data.p2p_client.clone().list_peers().await;
    debug!("Got received_peers: {:?}", p2p_peers);

    let str_peers: Vec<String> = p2p_peers.into_iter().map(|p| p.to_string()).collect();
    let str_peers_as_json = serde_json::to_string(&str_peers).unwrap();

    HttpResponse::Ok().body(str_peers_as_json)
}

#[get("/status")]
async fn status(data: web::Data<AppState>) -> Result<impl Responder, NodeError> {
    let p2p_peers = data.p2p_client.clone().list_peers().await;
    debug!("Got received_peers: {:?}", p2p_peers);

    let art_count_result = get_arts_count()?;

    let disk_space_result = disk_usage(ARTIFACTS_DIR.as_str())?;

    let status = Status {
        artifact_count: art_count_result,
        peers_count: p2p_peers.len(),
        disk_allocated: String::from(ALLOCATED_SPACE_FOR_ARTIFACTS),
        disk_usage: format!("{:.4}", disk_space_result),
    };

    let status_as_json = serde_json::to_string(&status)?;

    Ok(HttpResponse::Ok().body(status_as_json))
}
