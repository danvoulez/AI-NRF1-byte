//! Permit (consent) store — read, approve, deny, expire tickets.
//!
//! Tickets are stored as JSON files in `<state_dir>/permit-tickets/<tenant>/<ticket_id>.json`.
//! This module provides the core logic that both CLI and HTTP hooks consume.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted consent ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub ticket_id: String,
    pub tenant: String,
    pub status: TicketStatus,
    pub expires_at: i64,
    pub required_roles: Vec<String>,
    pub k: u8,
    pub n: u8,
    pub approvals: Vec<Approval>,
    pub created_at: i64,
    #[serde(default)]
    pub closed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TicketStatus {
    Pending,
    Allow,
    Deny,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub role: String,
    pub ts: i64,
    #[serde(default)]
    pub sig_hex: Option<String>,
}

/// Result of an approve/deny operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermitOutcome {
    /// Approval recorded but quorum not yet met.
    Pending { approvals: u8, needed: u8 },
    /// Quorum met — ticket closed as ALLOW.
    Closed(TicketStatus),
    /// Error: ticket not found, expired, already closed, or invalid role.
    Rejected(String),
}

/// File-backed permit store.
pub struct PermitStore {
    state_dir: PathBuf,
}

impl PermitStore {
    pub fn new(state_dir: impl Into<PathBuf>) -> Self {
        Self {
            state_dir: state_dir.into(),
        }
    }

    fn ticket_path(&self, tenant: &str, ticket_id: &str) -> PathBuf {
        self.state_dir
            .join("permit-tickets")
            .join(tenant)
            .join(format!("{}.json", ticket_id))
    }

    fn tickets_dir(&self, tenant: &str) -> PathBuf {
        self.state_dir.join("permit-tickets").join(tenant)
    }

    /// Read a ticket by ID.
    pub fn get(&self, tenant: &str, ticket_id: &str) -> anyhow::Result<Option<Ticket>> {
        let path = self.ticket_path(tenant, ticket_id);
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)?;
        let ticket: Ticket = serde_json::from_str(&data)?;
        Ok(Some(ticket))
    }

    /// List all tickets for a tenant (optionally filtered by status).
    pub fn list(&self, tenant: &str, status: Option<&TicketStatus>) -> anyhow::Result<Vec<Ticket>> {
        let dir = self.tickets_dir(tenant);
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut tickets = vec![];
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let data = std::fs::read_to_string(entry.path())?;
                if let Ok(t) = serde_json::from_str::<Ticket>(&data) {
                    if status.is_none() || status == Some(&t.status) {
                        tickets.push(t);
                    }
                }
            }
        }
        Ok(tickets)
    }

    /// Save a ticket to disk.
    fn save(&self, ticket: &Ticket) -> anyhow::Result<()> {
        let path = self.ticket_path(&ticket.tenant, &ticket.ticket_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(ticket)?)?;
        Ok(())
    }

    /// Approve a ticket with a given role. Returns outcome.
    pub fn approve(
        &self,
        tenant: &str,
        ticket_id: &str,
        role: &str,
        now_nanos: i64,
        sig_hex: Option<String>,
    ) -> anyhow::Result<PermitOutcome> {
        let Some(mut ticket) = self.get(tenant, ticket_id)? else {
            return Ok(PermitOutcome::Rejected("ticket not found".into()));
        };

        // Check status
        if ticket.status != TicketStatus::Pending {
            return Ok(PermitOutcome::Rejected(format!(
                "ticket already closed: {:?}",
                ticket.status
            )));
        }

        // Check expiration
        if now_nanos > ticket.expires_at {
            ticket.status = TicketStatus::Expired;
            ticket.closed_at = Some(now_nanos);
            self.save(&ticket)?;
            return Ok(PermitOutcome::Closed(TicketStatus::Expired));
        }

        // Check role is valid
        if !ticket.required_roles.contains(&role.to_string()) {
            return Ok(PermitOutcome::Rejected(format!(
                "role '{}' not in required_roles",
                role
            )));
        }

        // Check role not already used
        if ticket.approvals.iter().any(|a| a.role == role) {
            return Ok(PermitOutcome::Rejected(format!(
                "role '{}' already approved",
                role
            )));
        }

        // Add approval
        ticket.approvals.push(Approval {
            role: role.into(),
            ts: now_nanos,
            sig_hex,
        });

        // Check quorum
        let count = ticket.approvals.len() as u8;
        if count >= ticket.k {
            ticket.status = TicketStatus::Allow;
            ticket.closed_at = Some(now_nanos);
            self.save(&ticket)?;
            Ok(PermitOutcome::Closed(TicketStatus::Allow))
        } else {
            self.save(&ticket)?;
            Ok(PermitOutcome::Pending {
                approvals: count,
                needed: ticket.k,
            })
        }
    }

    /// Deny a ticket explicitly.
    pub fn deny(
        &self,
        tenant: &str,
        ticket_id: &str,
        role: &str,
        now_nanos: i64,
    ) -> anyhow::Result<PermitOutcome> {
        let Some(mut ticket) = self.get(tenant, ticket_id)? else {
            return Ok(PermitOutcome::Rejected("ticket not found".into()));
        };

        if ticket.status != TicketStatus::Pending {
            return Ok(PermitOutcome::Rejected(format!(
                "ticket already closed: {:?}",
                ticket.status
            )));
        }

        if !ticket.required_roles.contains(&role.to_string()) {
            return Ok(PermitOutcome::Rejected(format!(
                "role '{}' not in required_roles",
                role
            )));
        }

        ticket.status = TicketStatus::Deny;
        ticket.closed_at = Some(now_nanos);
        self.save(&ticket)?;
        Ok(PermitOutcome::Closed(TicketStatus::Deny))
    }

    /// Expire all pending tickets past their TTL.
    pub fn expire_stale(&self, tenant: &str, now_nanos: i64) -> anyhow::Result<Vec<String>> {
        let mut expired = vec![];
        for mut ticket in self.list(tenant, Some(&TicketStatus::Pending))? {
            if now_nanos > ticket.expires_at {
                ticket.status = TicketStatus::Expired;
                ticket.closed_at = Some(now_nanos);
                self.save(&ticket)?;
                expired.push(ticket.ticket_id.clone());
            }
        }
        Ok(expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> (PermitStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "ai-nrf1-permit-test-{}-{:?}",
            name,
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        (PermitStore::new(&dir), dir)
    }

    fn seed_ticket(store: &PermitStore) -> Ticket {
        let ticket = Ticket {
            ticket_id: "t-001".into(),
            tenant: "acme".into(),
            status: TicketStatus::Pending,
            expires_at: i64::MAX,
            required_roles: vec!["ops".into(), "risk".into(), "legal".into()],
            k: 2,
            n: 3,
            approvals: vec![],
            created_at: 1_000_000,
            closed_at: None,
        };
        store.save(&ticket).unwrap();
        ticket
    }

    #[test]
    fn get_nonexistent() {
        let (store, dir) = temp_store("get_nonexistent");
        assert!(store.get("acme", "nope").unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn approve_quorum_2_of_3() {
        let (store, dir) = temp_store("approve_quorum");
        seed_ticket(&store);

        // First approval — pending
        let r = store
            .approve("acme", "t-001", "ops", 2_000_000, None)
            .unwrap();
        assert_eq!(
            r,
            PermitOutcome::Pending {
                approvals: 1,
                needed: 2
            }
        );

        // Second approval — quorum met
        let r = store
            .approve("acme", "t-001", "risk", 3_000_000, None)
            .unwrap();
        assert_eq!(r, PermitOutcome::Closed(TicketStatus::Allow));

        // Verify persisted
        let t = store.get("acme", "t-001").unwrap().unwrap();
        assert_eq!(t.status, TicketStatus::Allow);
        assert_eq!(t.approvals.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_role_rejected() {
        let (store, dir) = temp_store("dup_role");
        seed_ticket(&store);

        store
            .approve("acme", "t-001", "ops", 2_000_000, None)
            .unwrap();
        let r = store
            .approve("acme", "t-001", "ops", 3_000_000, None)
            .unwrap();
        assert!(matches!(r, PermitOutcome::Rejected(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_role_rejected() {
        let (store, dir) = temp_store("invalid_role");
        seed_ticket(&store);

        let r = store
            .approve("acme", "t-001", "janitor", 2_000_000, None)
            .unwrap();
        assert!(matches!(r, PermitOutcome::Rejected(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deny_closes_ticket() {
        let (store, dir) = temp_store("deny");
        seed_ticket(&store);

        let r = store.deny("acme", "t-001", "ops", 2_000_000).unwrap();
        assert_eq!(r, PermitOutcome::Closed(TicketStatus::Deny));

        let t = store.get("acme", "t-001").unwrap().unwrap();
        assert_eq!(t.status, TicketStatus::Deny);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn approve_after_deny_rejected() {
        let (store, dir) = temp_store("approve_after_deny");
        seed_ticket(&store);

        store.deny("acme", "t-001", "ops", 2_000_000).unwrap();
        let r = store
            .approve("acme", "t-001", "risk", 3_000_000, None)
            .unwrap();
        assert!(matches!(r, PermitOutcome::Rejected(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expire_stale_tickets() {
        let (store, dir) = temp_store("expire_stale");
        let mut ticket = seed_ticket(&store);
        ticket.expires_at = 1_500_000; // already expired
        store.save(&ticket).unwrap();

        let expired = store.expire_stale("acme", 2_000_000).unwrap();
        assert_eq!(expired, vec!["t-001"]);

        let t = store.get("acme", "t-001").unwrap().unwrap();
        assert_eq!(t.status, TicketStatus::Expired);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn approve_expired_ticket() {
        let (store, dir) = temp_store("approve_expired");
        let mut ticket = seed_ticket(&store);
        ticket.expires_at = 1_500_000;
        store.save(&ticket).unwrap();

        let r = store
            .approve("acme", "t-001", "ops", 2_000_000, None)
            .unwrap();
        assert_eq!(r, PermitOutcome::Closed(TicketStatus::Expired));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_by_status() {
        let (store, dir) = temp_store("list_status");
        seed_ticket(&store);

        let pending = store.list("acme", Some(&TicketStatus::Pending)).unwrap();
        assert_eq!(pending.len(), 1);

        let allowed = store.list("acme", Some(&TicketStatus::Allow)).unwrap();
        assert_eq!(allowed.len(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
