// all warp routes can be here
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

use super::handlers::blobs::get_blob;
use super::handlers::manifests::get_manifest;

use actix_web::{get, HttpResponse, Responder, Scope, web};

#[get("/")]
async fn base() -> impl Responder {
    HttpResponse::Ok().body("{}")
}

pub fn docker_service() -> Scope {
    web::scope("v2")
        .service(base)
        .service(get_blob)
        .service(get_manifest)
}
