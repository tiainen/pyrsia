use std::fmt;

#[derive(Debug)]
pub enum NodeErrorType {
    Anyhow(anyhow::Error),
    Io(std::io::Error),
    SerdeJson(serde_json::Error),
    Reqwest(reqwest::Error),
    ManifestUnknown(String),
    Custom(String),
}

#[derive(Debug)]
pub struct NodeError {
    pub error_type: NodeErrorType,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match &self.error_type {
            NodeErrorType::Anyhow(e) => format!("anyhow::Error: {}", e),
            NodeErrorType::Io(e) => format!("std::io::Error: {}", e),
            NodeErrorType::SerdeJson(e) => format!("serde_json::Error: {}", e),
            NodeErrorType::Reqwest(e) => format!("reqwest::Error: {}", e),
            NodeErrorType::ManifestUnknown(manifest) => format!("PyrsiaNodeError: Manifest Unknown: {}", manifest),
            NodeErrorType::Custom(msg) => format!("PyrsiaNodeError: {}", msg),
        };
        write!(f, "{}", printable)
    }
}

impl actix_web::error::ResponseError for NodeError {
}

impl From<anyhow::Error> for NodeError {
    fn from(err: anyhow::Error) -> NodeError {
        NodeError {
            error_type: NodeErrorType::Anyhow(err)
        }
    }
}

impl From<std::io::Error> for NodeError {
    fn from(err: std::io::Error) -> NodeError {
        NodeError {
            error_type: NodeErrorType::Io(err)
        }
    }
}

impl From<serde_json::Error> for NodeError {
    fn from(err: serde_json::Error) -> NodeError {
        NodeError {
            error_type: NodeErrorType::SerdeJson(err)
        }
    }
}

impl From<reqwest::Error> for NodeError {
    fn from(err: reqwest::Error) -> NodeError {
        NodeError {
            error_type: NodeErrorType::Reqwest(err)
        }
    }
}

impl From<&str> for NodeError {
    fn from(msg: &str) -> NodeError {
        NodeError {
            error_type: NodeErrorType::Custom(msg.to_string())
        }
    }
}
