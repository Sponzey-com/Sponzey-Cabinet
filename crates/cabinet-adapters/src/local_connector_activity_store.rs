use cabinet_domain::connector::ConnectorActivity;
use cabinet_ports::connector::{ConnectorActivityStorePort, ConnectorPortError};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LocalConnectorActivityStore {
    activities: Vec<ConnectorActivity>,
}

impl LocalConnectorActivityStore {
    pub fn activities(&self) -> &[ConnectorActivity] {
        &self.activities
    }
}

impl ConnectorActivityStorePort for LocalConnectorActivityStore {
    fn record_activity(&mut self, activity: ConnectorActivity) -> Result<(), ConnectorPortError> {
        self.activities.push(activity);
        Ok(())
    }

    fn list_activities(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<ConnectorActivity>, ConnectorPortError> {
        Ok(self
            .activities
            .iter()
            .filter(|activity| activity.workspace_id_hash() == workspace_id_hash)
            .cloned()
            .collect())
    }
}
