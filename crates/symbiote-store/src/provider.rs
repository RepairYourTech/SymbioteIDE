use super::*;

pub(super) const MIGRATION_V9: &str = "CREATE TABLE provider_connections (
 id TEXT PRIMARY KEY,
 adapter TEXT NOT NULL,
 body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
CREATE TABLE billing_entitlements (
 id TEXT PRIMARY KEY,
 provider TEXT NOT NULL REFERENCES provider_connections(id),
 body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
CREATE TABLE model_descriptors (
 id TEXT PRIMARY KEY,
 provider TEXT NOT NULL REFERENCES provider_connections(id),
 body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
PRAGMA user_version=9;";

type ConnectionRecord = symbiote_domain::ProviderConnection;
type EntitlementRecord = symbiote_domain::BillingEntitlement;
type ModelRecord = symbiote_runtime_sdk::provider::ModelDescriptor;

pub(super) fn connection_event(
    attribution: &ProjectId,
    connection: &ConnectionRecord,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::ProviderRegistered {
        attribution: attribution.clone(),
        connection: connection.clone(),
        actor: actor.clone(),
        at,
    }
}

pub(super) fn entitlement_event(
    attribution: &ProjectId,
    entitlement: &EntitlementRecord,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::EntitlementRegistered {
        attribution: attribution.clone(),
        entitlement: Box::new(entitlement.clone()),
        actor: actor.clone(),
        at,
    }
}

pub(super) fn model_event(
    attribution: &ProjectId,
    model: &ModelRecord,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::ModelRegistered {
        attribution: attribution.clone(),
        descriptor: Box::new(model.clone()),
        actor: actor.clone(),
        at,
    }
}

fn read_connection(connection: &Connection, id: &str) -> Result<Option<ConnectionRecord>> {
    let (adapter, body): (String, String) = match connection
        .query_row(
            "SELECT adapter, body FROM provider_connections WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    {
        Some(row) => row,
        None => return Ok(None),
    };
    let record: ConnectionRecord = serde_json::from_str(&body)?;
    if record.adapter.as_str() != adapter || record.id.as_str() != id {
        return Err(StoreError::Integrity(
            "provider connection indexed state differs from body".into(),
        ));
    }
    Ok(Some(record))
}

fn read_entitlement(connection: &Connection, id: &str) -> Result<Option<EntitlementRecord>> {
    let (provider, body): (String, String) = match connection
        .query_row(
            "SELECT provider, body FROM billing_entitlements WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    {
        Some(row) => row,
        None => return Ok(None),
    };
    let record: EntitlementRecord = serde_json::from_str(&body)?;
    if record.provider.as_str() != provider || record.id.as_str() != id {
        return Err(StoreError::Integrity(
            "entitlement indexed state differs from body".into(),
        ));
    }
    Ok(Some(record))
}

fn read_model(connection: &Connection, id: &str) -> Result<Option<ModelRecord>> {
    let (provider, body): (String, String) = match connection
        .query_row(
            "SELECT provider, body FROM model_descriptors WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    {
        Some(row) => row,
        None => return Ok(None),
    };
    let record: ModelRecord = serde_json::from_str(&body)?;
    if record.provider_id.as_str() != provider || record.id.as_str() != id {
        return Err(StoreError::Integrity(
            "model indexed state differs from body".into(),
        ));
    }
    Ok(Some(record))
}

impl Store {
    /// Registers or replaces a provider connection. Callers authenticate the
    /// actor and authorize the owner policy; a replacement is a new record
    /// version and the journal keeps every revision.
    pub fn replace_provider_connection(
        &mut self,
        command_id: CommandId,
        attribution: ProjectId,
        connection: ConnectionRecord,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        if at.0 > i64::MAX as u64 || connection.endpoint_reference.trim().is_empty() {
            return Err(StoreError::InvalidProvider);
        }
        if !exists(
            &self.connection,
            "SELECT 1 FROM projects WHERE id=?1",
            attribution.as_str(),
        )? {
            return Err(StoreError::NotFound);
        }
        let payload = connection_event(&attribution, &connection, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        let body = serde_json::to_string(&connection)?;
        transaction.execute(
            "INSERT INTO provider_connections(id,adapter,body) VALUES (?1,?2,?3)
             ON CONFLICT(id) DO UPDATE SET adapter=excluded.adapter, body=excluded.body",
            params![connection.id.as_str(), connection.adapter.as_str(), body],
        )?;
        let receipt = append(
            &transaction,
            &project_id_for(&attribution),
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Registers or replaces a billing entitlement bound to a stored
    /// connection. The connection must exist; entitlement expiry is validated
    /// through the SDK contract at binding time, not here.
    pub fn replace_billing_entitlement(
        &mut self,
        command_id: CommandId,
        attribution: ProjectId,
        entitlement: EntitlementRecord,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidProvider);
        }
        if !exists(
            &self.connection,
            "SELECT 1 FROM projects WHERE id=?1",
            attribution.as_str(),
        )? {
            return Err(StoreError::NotFound);
        }
        let payload = entitlement_event(&attribution, &entitlement, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        if read_connection(&transaction, entitlement.provider.as_str())?.is_none() {
            return Err(StoreError::RelationshipMismatch);
        }
        let body = serde_json::to_string(&entitlement)?;
        transaction.execute(
            "INSERT INTO billing_entitlements(id,provider,body) VALUES (?1,?2,?3)
             ON CONFLICT(id) DO UPDATE SET provider=excluded.provider, body=excluded.body",
            params![entitlement.id.as_str(), entitlement.provider.as_str(), body],
        )?;
        let receipt = append(
            &transaction,
            &project_id_for(&attribution),
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Registers a model descriptor against a stored connection. The
    /// descriptor is SDK-owned content: capabilities and bounds travel with
    /// the record and are re-validated by the SDK at binding time.
    pub fn replace_model_descriptor(
        &mut self,
        command_id: CommandId,
        attribution: ProjectId,
        descriptor: ModelRecord,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        descriptor
            .validate()
            .map_err(|_| StoreError::InvalidProvider)?;
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidProvider);
        }
        if !exists(
            &self.connection,
            "SELECT 1 FROM projects WHERE id=?1",
            attribution.as_str(),
        )? {
            return Err(StoreError::NotFound);
        }
        let payload = model_event(&attribution, &descriptor, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        if read_connection(&transaction, descriptor.provider_id.as_str())?.is_none() {
            return Err(StoreError::RelationshipMismatch);
        }
        let body = serde_json::to_string(&descriptor)?;
        transaction.execute(
            "INSERT INTO model_descriptors(id,provider,body) VALUES (?1,?2,?3)
             ON CONFLICT(id) DO UPDATE SET provider=excluded.provider, body=excluded.body",
            params![
                descriptor.id.as_str(),
                descriptor.provider_id.as_str(),
                body
            ],
        )?;
        let receipt = append(
            &transaction,
            &project_id_for(&attribution),
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    pub fn provider_connection(&self, id: &ProviderConnectionId) -> Result<ConnectionRecord> {
        read_connection(&self.connection, id.as_str())?
            .filter(|c| &c.id == id)
            .ok_or(StoreError::NotFound)
    }

    pub fn billing_entitlement(&self, id: &BillingEntitlementId) -> Result<EntitlementRecord> {
        read_entitlement(&self.connection, id.as_str())?
            .filter(|e| &e.id == id)
            .ok_or(StoreError::NotFound)
    }

    pub fn model_descriptor(&self, id: &ModelId) -> Result<ModelRecord> {
        read_model(&self.connection, id.as_str())?
            .filter(|m| &m.id == id)
            .ok_or(StoreError::NotFound)
    }

    pub fn provider_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM journal WHERE command_id=?1",
                [id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => match serde_json::from_str::<EventPayload>(&body)? {
                EventPayload::ProviderRegistered { at, .. }
                | EventPayload::EntitlementRegistered { at, .. }
                | EventPayload::ModelRegistered { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }
}

// Provider records are global identity rows, but journal rows are
// project-scoped: each registration is attributed to a real Project that the
// caller explicitly names (the Project whose workforce intends to use the
// provider). Attribution is provenance only — it does not scope the record's
// visibility, and cross-project use is re-authorized at binding time.
fn project_id_for(attribution: &ProjectId) -> ProjectId {
    attribution.clone()
}

pub(super) fn audit_finish(
    connection: &Connection,
    providers: &BTreeMap<ProviderConnectionId, ConnectionRecord>,
    entitlements: &BTreeMap<BillingEntitlementId, EntitlementRecord>,
    models: &BTreeMap<symbiote_domain::ModelId, ModelRecord>,
) -> Result<()> {
    // Journal rows are keyed by their own identity, not a project; audit
    // compares the full materialized table against the journal-derived map
    // in both directions.
    for (id, expected) in providers {
        if read_connection(connection, id.as_str())?.as_ref() != Some(expected) {
            return Err(StoreError::Integrity(
                "provider state differs from journal".into(),
            ));
        }
    }
    for (id, expected) in entitlements {
        if read_entitlement(connection, id.as_str())?.as_ref() != Some(expected) {
            return Err(StoreError::Integrity(
                "entitlement state differs from journal".into(),
            ));
        }
    }
    for (id, expected) in models {
        if read_model(connection, id.as_str())?.as_ref() != Some(expected) {
            return Err(StoreError::Integrity(
                "model state differs from journal".into(),
            ));
        }
    }
    for (table, journal) in [
        ("provider_connections", providers.len()),
        ("billing_entitlements", entitlements.len()),
        ("model_descriptors", models.len()),
    ] {
        let materialized: usize =
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                sql_usize(r, 0)
            })?;
        if materialized != journal {
            return Err(StoreError::Integrity(format!(
                "{table} differs from journal"
            )));
        }
    }
    Ok(())
}
