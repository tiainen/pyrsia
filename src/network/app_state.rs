use crate::network::p2p;

pub struct AppState {
    pub p2p_client: p2p::Client,
}
