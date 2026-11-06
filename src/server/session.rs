use axum::extract::ws::Message;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

pub type ClientId = String;

#[derive(Clone)]
pub struct ClientSession {
    pub id: ClientId,
    pub server_id: String,
    pub sender: mpsc::Sender<Message>,
}

#[derive(Default)]
pub struct SessionRegistry {
    /// Maps ClientId -> ClientSession
    pub clients: DashMap<ClientId, ClientSession>,
    /// Maps InternalRequestId -> (ClientId, OriginalRawId)
    pub pending_requests: DashMap<String, (ClientId, Value)>,
}

impl SessionRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn register_client(&self, session: ClientSession) {
        self.clients.insert(session.id.clone(), session);
    }

    pub fn unregister_client(&self, client_id: &str) {
        self.clients.remove(client_id);
        // Clean up pending requests associated with this disconnected client
        let to_remove: Vec<String> = self
            .pending_requests
            .iter()
            .filter(|entry| entry.value().0 == client_id)
            .map(|entry| entry.key().clone())
            .collect();

        for key in to_remove {
            self.pending_requests.remove(&key);
        }
    }

    pub fn client_count_for_server(&self, server_id: &str) -> usize {
        self.clients
            .iter()
            .filter(|entry| entry.value().server_id == server_id)
            .count()
    }

    pub fn total_clients(&self) -> usize {
        self.clients.len()
    }
}
