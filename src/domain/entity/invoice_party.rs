use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::InvoicePartyRole;
use super::InvoicePartyType;
use super::AuditMetadata;

/// Strongly-typed ID for InvoiceParty
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvoicePartyId(pub Uuid);

impl InvoicePartyId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for InvoicePartyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for InvoicePartyId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for InvoicePartyId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<InvoicePartyId> for Uuid {
    fn from(id: InvoicePartyId) -> Self { id.0 }
}

impl AsRef<Uuid> for InvoicePartyId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for InvoicePartyId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceParty {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub role: InvoicePartyRole,
    pub party_type: InvoicePartyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_id: Option<Uuid>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl InvoiceParty {
    /// Create a builder for InvoiceParty
    pub fn builder() -> InvoicePartyBuilder {
        InvoicePartyBuilder::default()
    }

    /// Create a new InvoiceParty with required fields
    pub fn new(invoice_id: Uuid, role: InvoicePartyRole, party_type: InvoicePartyType, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            invoice_id,
            role,
            party_type,
            party_id: None,
            name,
            email: None,
            address: None,
            tax_id: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> InvoicePartyId {
        InvoicePartyId(self.id)
    }

    /// Get when this entity was created
    pub fn created_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.created_at.as_ref()
    }

    /// Get when this entity was last updated
    pub fn updated_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.updated_at.as_ref()
    }

    /// Check if this entity is soft deleted
    pub fn is_deleted(&self) -> bool {
        self.metadata.deleted_at.is_some()
    }

    /// Check if this entity is active (not deleted)
    pub fn is_active(&self) -> bool {
        self.metadata.deleted_at.is_none()
    }

    /// Get when this entity was deleted
    pub fn deleted_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.deleted_at.as_ref()
    }

    /// Get who created this entity
    pub fn created_by(&self) -> Option<&Uuid> {
        self.metadata.created_by.as_ref()
    }

    /// Get who last updated this entity
    pub fn updated_by(&self) -> Option<&Uuid> {
        self.metadata.updated_by.as_ref()
    }

    /// Get who deleted this entity
    pub fn deleted_by(&self) -> Option<&Uuid> {
        self.metadata.deleted_by.as_ref()
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the party_id field (chainable)
    pub fn with_party_id(mut self, value: Uuid) -> Self {
        self.party_id = Some(value);
        self
    }

    /// Set the email field (chainable)
    pub fn with_email(mut self, value: String) -> Self {
        self.email = Some(value);
        self
    }

    /// Set the address field (chainable)
    pub fn with_address(mut self, value: String) -> Self {
        self.address = Some(value);
        self
    }

    /// Set the tax_id field (chainable)
    pub fn with_tax_id(mut self, value: String) -> Self {
        self.tax_id = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "invoice_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoice_id = v; }
                }
                "role" => {
                    if let Ok(v) = serde_json::from_value(value) { self.role = v; }
                }
                "party_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.party_type = v; }
                }
                "party_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.party_id = v; }
                }
                "name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.name = v; }
                }
                "email" => {
                    if let Ok(v) = serde_json::from_value(value) { self.email = v; }
                }
                "address" => {
                    if let Ok(v) = serde_json::from_value(value) { self.address = v; }
                }
                "tax_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.tax_id = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for InvoiceParty {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "InvoiceParty"
    }
}

impl backbone_core::PersistentEntity for InvoiceParty {
    fn entity_id(&self) -> String {
        self.id.to_string()
    }
    fn set_entity_id(&mut self, id: String) {
        if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            self.id = uuid;
        }
    }
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.created_at
    }
    fn set_created_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.created_at = Some(ts);
    }
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.updated_at
    }
    fn set_updated_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.updated_at = Some(ts);
    }
    fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.deleted_at
    }
    fn set_deleted_at(&mut self, ts: Option<chrono::DateTime<chrono::Utc>>) {
        self.metadata.deleted_at = ts;
    }
}

impl backbone_orm::EntityRepoMeta for InvoiceParty {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("invoice_id".to_string(), "uuid".to_string());
        m.insert("party_id".to_string(), "uuid".to_string());
        m.insert("role".to_string(), "invoice_party_role".to_string());
        m.insert("party_type".to_string(), "invoice_party_type".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["name"]
    }
}

/// Builder for InvoiceParty entity
///
/// Provides a fluent API for constructing InvoiceParty instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct InvoicePartyBuilder {
    invoice_id: Option<Uuid>,
    role: Option<InvoicePartyRole>,
    party_type: Option<InvoicePartyType>,
    party_id: Option<Uuid>,
    name: Option<String>,
    email: Option<String>,
    address: Option<String>,
    tax_id: Option<String>,
}

impl InvoicePartyBuilder {
    /// Set the invoice_id field (required)
    pub fn invoice_id(mut self, value: Uuid) -> Self {
        self.invoice_id = Some(value);
        self
    }

    /// Set the role field (required)
    pub fn role(mut self, value: InvoicePartyRole) -> Self {
        self.role = Some(value);
        self
    }

    /// Set the party_type field (required)
    pub fn party_type(mut self, value: InvoicePartyType) -> Self {
        self.party_type = Some(value);
        self
    }

    /// Set the party_id field (optional)
    pub fn party_id(mut self, value: Uuid) -> Self {
        self.party_id = Some(value);
        self
    }

    /// Set the name field (required)
    pub fn name(mut self, value: String) -> Self {
        self.name = Some(value);
        self
    }

    /// Set the email field (optional)
    pub fn email(mut self, value: String) -> Self {
        self.email = Some(value);
        self
    }

    /// Set the address field (optional)
    pub fn address(mut self, value: String) -> Self {
        self.address = Some(value);
        self
    }

    /// Set the tax_id field (optional)
    pub fn tax_id(mut self, value: String) -> Self {
        self.tax_id = Some(value);
        self
    }

    /// Build the InvoiceParty entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<InvoiceParty, String> {
        let invoice_id = self.invoice_id.ok_or_else(|| "invoice_id is required".to_string())?;
        let role = self.role.ok_or_else(|| "role is required".to_string())?;
        let party_type = self.party_type.ok_or_else(|| "party_type is required".to_string())?;
        let name = self.name.ok_or_else(|| "name is required".to_string())?;

        Ok(InvoiceParty {
            id: Uuid::new_v4(),
            invoice_id,
            role,
            party_type,
            party_id: self.party_id,
            name,
            email: self.email,
            address: self.address,
            tax_id: self.tax_id,
            metadata: AuditMetadata::default(),
        })
    }
}
