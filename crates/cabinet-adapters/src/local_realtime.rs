use std::collections::{HashMap, HashSet};

use cabinet_domain::realtime::{DocumentRoomId, RoomOwnerKey};
use cabinet_ports::realtime::{
    DocumentRoomOwnerPolicy, JoinDocumentRoomRequest, OperationBroadcastRequest,
    PresenceBroadcastRequest, RealtimeAcknowledgement, RealtimeTransport, RealtimeTransportError,
    ReplayLocalChangesRequest, RoomOwnerPolicyError,
};

pub struct LocalDocumentRoomOwnerPolicy {
    namespace: String,
}

impl LocalDocumentRoomOwnerPolicy {
    pub fn new(namespace: &str) -> Result<Self, RoomOwnerPolicyError> {
        let namespace = namespace.trim();
        if namespace.is_empty() || namespace.chars().any(char::is_control) {
            return Err(RoomOwnerPolicyError::InvalidRoom);
        }
        Ok(Self {
            namespace: namespace.to_string(),
        })
    }
}

impl DocumentRoomOwnerPolicy for LocalDocumentRoomOwnerPolicy {
    fn owner_key(&self, room_id: &DocumentRoomId) -> Result<RoomOwnerKey, RoomOwnerPolicyError> {
        RoomOwnerKey::for_room(&self.namespace, room_id).map_err(RoomOwnerPolicyError::from)
    }
}

#[derive(Default)]
pub struct LocalRealtimeTransport {
    joined_rooms: HashSet<String>,
    operations: HashMap<String, Vec<OperationBroadcastRequest>>,
    presences: HashMap<String, Vec<PresenceBroadcastRequest>>,
    replay_requests: HashMap<String, Vec<ReplayLocalChangesRequest>>,
}

impl LocalRealtimeTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn joined_room_count(&self) -> usize {
        self.joined_rooms.len()
    }

    pub fn recorded_operations(&self, room_id: &DocumentRoomId) -> Vec<OperationBroadcastRequest> {
        self.operations
            .get(&room_key(room_id))
            .cloned()
            .unwrap_or_default()
    }

    pub fn recorded_presences(&self, room_id: &DocumentRoomId) -> Vec<PresenceBroadcastRequest> {
        self.presences
            .get(&room_key(room_id))
            .cloned()
            .unwrap_or_default()
    }

    pub fn recorded_replay_requests(
        &self,
        room_id: &DocumentRoomId,
    ) -> Vec<ReplayLocalChangesRequest> {
        self.replay_requests
            .get(&room_key(room_id))
            .cloned()
            .unwrap_or_default()
    }

    fn ensure_joined(&self, room_id: &DocumentRoomId) -> Result<String, RealtimeTransportError> {
        let key = room_key(room_id);
        if !self.joined_rooms.contains(&key) {
            return Err(RealtimeTransportError::RoomNotJoined);
        }
        Ok(key)
    }
}

impl RealtimeTransport for LocalRealtimeTransport {
    fn join_document_room(
        &mut self,
        request: JoinDocumentRoomRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        self.joined_rooms.insert(room_key(request.room_id()));
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }

    fn broadcast_operation(
        &mut self,
        request: OperationBroadcastRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        let key = self.ensure_joined(request.room_id())?;
        self.operations
            .entry(key)
            .or_default()
            .push(request.clone());
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }

    fn broadcast_presence(
        &mut self,
        request: PresenceBroadcastRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        let key = self.ensure_joined(request.room_id())?;
        self.presences.entry(key).or_default().push(request.clone());
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }

    fn request_replay(
        &mut self,
        request: ReplayLocalChangesRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        let key = self.ensure_joined(request.room_id())?;
        self.replay_requests
            .entry(key)
            .or_default()
            .push(request.clone());
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }
}

fn room_key(room_id: &DocumentRoomId) -> String {
    format!(
        "{}:{}",
        room_id.workspace_id().as_str(),
        room_id.document_id().as_str(),
    )
}
