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

use super::handlers::*;
use super::HashAlgorithm;
use crate::docker::docker_hub_util::get_docker_hub_auth_token;
use crate::network::app_state::AppState;
use crate::network::p2p;
use crate::util::error_util::{NodeError, NodeErrorType};

use actix_web::{get, HttpResponse, Responder, web};
use bytes::{Buf, Bytes};
use libp2p::PeerId;
use log::{debug, info};
use reqwest::header;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::result::Result;
use std::str;
use uuid::Uuid;

#[get("/library/{name}/blobs/{hash}")]
async fn get_blob(path: web::Path<(String, String)>, data: web::Data<AppState>) -> Result<impl Responder, NodeError> {
    let (name, hash) = path.into_inner();

    debug!("Getting blob with hash : {:?}", hash);
    let blob_content;

    debug!("Step 1: Does {:?} exist in the artifact manager?", hash);
    let decoded_hash = hex::decode(&hash.get(7..).unwrap()).unwrap();
    match get_artifact(&decoded_hash, HashAlgorithm::SHA256) {
        Ok(blob) => {
            debug!("Step 1: YES, {:?} exist in the artifact manager.", hash);
            blob_content = blob;
        }
        Err(_) => {
            debug!(
                "Step 1: NO, {:?} does not exist in the artifact manager.",
                hash
            );

            let blob_stored = get_blob_from_network(data.p2p_client.clone(), &name, &hash).await?;
            if blob_stored {
                blob_content =
                    get_artifact(&decoded_hash, HashAlgorithm::SHA256)?;
            } else {
                return Err(NodeError {
                    error_type: NodeErrorType::Custom("PYRSIA_ARTIFACT_STORAGE_ERROR".to_string()),
                });
            }
        }
    }

    data.p2p_client.clone().provide(String::from(&hash)).await;

    debug!("Final Step: {:?} successfully retrieved!", &hash);
    Ok(HttpResponse::Ok()
        .append_header(("Content-Type", "application/octet-stream"))
        .body(blob_content))
}

pub fn append_to_blob(blob: &str, mut bytes: Bytes) -> std::io::Result<(u64, u64)> {
    debug!("Patching blob: {}", blob);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(blob)?;
    let mut total_bytes_read: u64 = 0;
    let initial_file_length: u64;
    initial_file_length = file.metadata()?.len();
    while bytes.has_remaining() {
        let bytes_remaining = bytes.remaining();
        let bytes_to_read = if bytes_remaining <= 4096 {
            bytes_remaining
        } else {
            4096
        };
        total_bytes_read += bytes_to_read as u64;
        let mut b = vec![0; bytes_to_read];
        bytes.copy_to_slice(&mut b);
        file.write_all(&b)?;
    }

    Ok((initial_file_length, total_bytes_read))
}

fn create_upload_directory(name: &str, id: &str) -> std::io::Result<String> {
    let upload_directory = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    fs::create_dir_all(&upload_directory)?;
    Ok(upload_directory)
}

fn store_blob_in_filesystem(
    name: &str,
    id: &str,
    digest: &str,
    bytes: Bytes,
) -> Result<bool, NodeError> {
    let blob_upload_dest_dir = create_upload_directory(name, &id.to_string())?;
    let mut blob_upload_dest_data = blob_upload_dest_dir.clone();
    blob_upload_dest_data.push_str("/data");
    let append = append_to_blob(&blob_upload_dest_data, bytes)?;

    // check if there is enough local allocated disk space
    let available_space = get_space_available(ARTIFACTS_DIR.as_str())?;
    if append.1 > available_space {
        return Err("Not enough space left to store artifact".into());
    }

    //put blob in artifact manager
    let reader = File::open(blob_upload_dest_data.as_str())?;

    let push_result = put_artifact(
        hex::decode(&digest.get(7..).unwrap()).unwrap().as_ref(),
        Box::new(reader),
        HashAlgorithm::SHA256,
    )?;

    fs::remove_dir_all(&blob_upload_dest_dir)?;

    Ok(push_result)
}

// Request the content of the artifact from the pyrsia network
async fn get_blob_from_network(
    mut p2p_client: p2p::Client,
    name: &str,
    hash: &str,
) -> Result<bool, NodeError> {
    let providers = p2p_client.list_providers(String::from(hash)).await;
    debug!("List of providers for {:?}: {:?}", &hash, providers);
    Ok(match providers.iter().next() {
        Some(peer) => match get_blob_from_other_peer(p2p_client.clone(), peer, name, hash).await {
            true => true,
            false => get_blob_from_docker_hub(name, hash).await?,
        },
        None => get_blob_from_docker_hub(name, hash).await?,
    })
}

// Request the content of the artifact from other peer
async fn get_blob_from_other_peer(
    mut p2p_client: p2p::Client,
    peer_id: &PeerId,
    name: &str,
    hash: &str,
) -> bool {
    info!(
        "Reading blob from Pyrsia Node {}: {}",
        peer_id,
        hash.get(7..).unwrap()
    );
    debug!("Step 2: Does {:?} exist in the Pyrsia network?", hash);
    match p2p_client
        .request_artifact(peer_id, String::from(hash))
        .await
    {
        Ok(artifact) => {
            let id = Uuid::new_v4();
            debug!("Step 2: YES, {:?} exists in the Pyrsia network.", hash);
            match store_blob_in_filesystem(
                name,
                &id.to_string(),
                hash,
                bytes::Bytes::from(artifact),
            ) {
                Ok(stored) => {
                    debug!(
                        "Step 2: {:?} successfully stored locally from Pyrsia network.",
                        hash
                    );
                    stored
                }
                Err(error) => {
                    debug!("Error while storing artifact in filesystem: {}", error);
                    false
                }
            }
        }
        Err(error) => {
            debug!(
                "Step 2: NO, {:?} does not exist in the Pyrsia network.",
                hash
            );
            debug!(
                "Error while fetching artifact from Pyrsia Node, so fetching from dockerhub: {}",
                error
            );
            false
        }
    }
}

async fn get_blob_from_docker_hub(name: &str, hash: &str) -> Result<bool, NodeError> {
    debug!("Step 3: Retrieving {:?} from docker.io", hash);
    let token = get_docker_hub_auth_token(name).await?;

    get_blob_from_docker_hub_with_token(name, hash, token).await
}

async fn get_blob_from_docker_hub_with_token(
    name: &str,
    hash: &str,
    token: String,
) -> Result<bool, NodeError> {
    let url = format!(
        "https://registry-1.docker.io/v2/library/{}/blobs/{}",
        name, hash
    );
    debug!("Reading blob from docker.io with url: {}", url);
    let response = reqwest::Client::new()
        .get(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;

    debug!("Got blob from docker.io with status {}", response.status());
    let bytes = response.bytes().await?;

    let id = Uuid::new_v4();

    store_blob_in_filesystem(name, &id.to_string(), hash, bytes)
}
