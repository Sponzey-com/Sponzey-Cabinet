use cabinet_domain::connector::{
    ConnectorDefinition, ConnectorId, ConnectorKind, ConnectorScopeSet,
};
use cabinet_ports::connector::{ConnectorDefinitionRegistryPort, ConnectorPortError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticConnectorDefinitionRegistry {
    definitions: Vec<ConnectorDefinition>,
}

impl StaticConnectorDefinitionRegistry {
    pub fn phase005_baseline() -> Self {
        Self {
            definitions: vec![
                ConnectorDefinition::new(
                    ConnectorId::new("connector-slack").expect("static connector id"),
                    ConnectorKind::Slack,
                    "Slack",
                    ConnectorScopeSet::read_write(),
                )
                .expect("static Slack descriptor"),
                ConnectorDefinition::new(
                    ConnectorId::new("connector-teams").expect("static connector id"),
                    ConnectorKind::Teams,
                    "Microsoft Teams",
                    ConnectorScopeSet::read_only(),
                )
                .expect("static Teams descriptor"),
                ConnectorDefinition::new(
                    ConnectorId::new("connector-jira").expect("static connector id"),
                    ConnectorKind::Jira,
                    "Jira",
                    ConnectorScopeSet::read_write(),
                )
                .expect("static Jira descriptor"),
            ],
        }
    }
}

impl ConnectorDefinitionRegistryPort for StaticConnectorDefinitionRegistry {
    fn list_definitions(&self) -> Result<Vec<ConnectorDefinition>, ConnectorPortError> {
        Ok(self.definitions.clone())
    }

    fn find_definition(
        &self,
        id: &ConnectorId,
    ) -> Result<Option<ConnectorDefinition>, ConnectorPortError> {
        Ok(self
            .definitions
            .iter()
            .find(|definition| definition.id() == id)
            .cloned())
    }
}
