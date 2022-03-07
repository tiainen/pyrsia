
use super::handlers::{peers, status};

use actix_web::{Scope, web};

pub fn node_service() -> Scope {
    web::scope("node")
        .service(peers)
        .service(status)
}
