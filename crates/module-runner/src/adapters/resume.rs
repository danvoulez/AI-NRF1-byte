//! Resume watcher — polls the permit store for tickets that close as ALLOW,
//! then re-submits the capsule to the runner starting from the step after
//! the one that returned REQUIRE.
//!
//! Design:
//!   - A `ResumeJob` captures the frozen pipeline state at the REQUIRE point.
//!   - `ResumeStore` persists jobs as JSON files in `<state_dir>/resume/<tenant>/`.
//!   - `resume_poll` is an async loop that checks for ALLOW'd tickets and
//!     invokes a callback to re-run the pipeline.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A frozen pipeline state waiting for consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeJob {
    pub job_id: String,
    pub tenant: String,
    pub ticket_id: String,
    pub trace_id: String,
    pub capsule_id_hex: String,
    /// Index of the step that returned REQUIRE (resume from step_index + 1).
    pub resume_after_step: usize,
    /// Frozen env (NRF JSON) at the REQUIRE point.
    pub env_json: serde_json::Value,
    /// Manifest name to re-load.
    pub manifest_name: String,
    /// Accumulated receipts so far.
    pub receipts_hex: Vec<String>,
    pub created_at: i64,
    #[serde(default)]
    pub completed: bool,
}

/// File-backed store for resume jobs.
pub struct ResumeStore {
    state_dir: PathBuf,
}

impl ResumeStore {
    pub fn new(state_dir: impl Into<PathBuf>) -> Self {
        Self {
            state_dir: state_dir.into(),
        }
    }

    fn jobs_dir(&self, tenant: &str) -> PathBuf {
        self.state_dir.join("resume").join(tenant)
    }

    fn job_path(&self, tenant: &str, job_id: &str) -> PathBuf {
        self.jobs_dir(tenant).join(format!("{}.json", job_id))
    }

    /// Save a new resume job.
    pub fn save(&self, job: &ResumeJob) -> anyhow::Result<()> {
        let dir = self.jobs_dir(&job.tenant);
        std::fs::create_dir_all(&dir)?;
        let path = self.job_path(&job.tenant, &job.job_id);
        std::fs::write(&path, serde_json::to_string_pretty(job)?)?;
        Ok(())
    }

    /// Read a resume job.
    pub fn get(&self, tenant: &str, job_id: &str) -> anyhow::Result<Option<ResumeJob>> {
        let path = self.job_path(tenant, job_id);
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)?;
        Ok(Some(serde_json::from_str(&data)?))
    }

    /// List all pending (not completed) resume jobs for a tenant.
    pub fn list_pending(&self, tenant: &str) -> anyhow::Result<Vec<ResumeJob>> {
        let dir = self.jobs_dir(tenant);
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut jobs = vec![];
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let data = std::fs::read_to_string(entry.path())?;
                if let Ok(job) = serde_json::from_str::<ResumeJob>(&data) {
                    if !job.completed {
                        jobs.push(job);
                    }
                }
            }
        }
        Ok(jobs)
    }

    /// Mark a job as completed.
    pub fn mark_completed(&self, tenant: &str, job_id: &str) -> anyhow::Result<()> {
        if let Some(mut job) = self.get(tenant, job_id)? {
            job.completed = true;
            self.save(&job)?;
        }
        Ok(())
    }
}

/// Check which pending resume jobs have their tickets resolved as ALLOW.
/// Returns the jobs that are ready to resume.
pub fn check_resumable(
    resume_store: &ResumeStore,
    permit_store: &crate::adapters::permit::PermitStore,
    tenant: &str,
) -> anyhow::Result<Vec<ResumeJob>> {
    let pending = resume_store.list_pending(tenant)?;
    let mut ready = vec![];
    for job in pending {
        if let Some(ticket) = permit_store.get(tenant, &job.ticket_id)? {
            if ticket.status == crate::adapters::permit::TicketStatus::Allow {
                ready.push(job);
            }
        }
    }
    Ok(ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> (ResumeStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "ai-nrf1-resume-test-{}-{:?}",
            name,
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        (ResumeStore::new(&dir), dir)
    }

    fn sample_job() -> ResumeJob {
        ResumeJob {
            job_id: "j-001".into(),
            tenant: "acme".into(),
            ticket_id: "t-001".into(),
            trace_id: "abc123".into(),
            capsule_id_hex: "deadbeef".into(),
            resume_after_step: 2,
            env_json: serde_json::json!({"amount": 5000}),
            manifest_name: "api-receipt-gateway".into(),
            receipts_hex: vec!["aabb".into(), "ccdd".into()],
            created_at: 1_000_000,
            completed: false,
        }
    }

    #[test]
    fn save_and_get() {
        let (store, dir) = temp_store("save_get");
        let job = sample_job();
        store.save(&job).unwrap();

        let got = store.get("acme", "j-001").unwrap().unwrap();
        assert_eq!(got.job_id, "j-001");
        assert_eq!(got.resume_after_step, 2);
        assert!(!got.completed);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_pending_excludes_completed() {
        let (store, dir) = temp_store("list_pending");
        let job = sample_job();
        store.save(&job).unwrap();

        let pending = store.list_pending("acme").unwrap();
        assert_eq!(pending.len(), 1);

        store.mark_completed("acme", "j-001").unwrap();
        let pending = store.list_pending("acme").unwrap();
        assert_eq!(pending.len(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_nonexistent() {
        let (store, dir) = temp_store("get_none");
        assert!(store.get("acme", "nope").unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_resumable_with_permit() {
        let (resume_store, dir) = temp_store("resumable");
        let permit_dir = dir.join("permit-state");

        // Create resume job
        let job = sample_job();
        resume_store.save(&job).unwrap();

        // Create permit store with an ALLOW'd ticket
        let permit_store = crate::adapters::permit::PermitStore::new(&permit_dir);
        let ticket = crate::adapters::permit::Ticket {
            ticket_id: "t-001".into(),
            tenant: "acme".into(),
            status: crate::adapters::permit::TicketStatus::Allow,
            expires_at: i64::MAX,
            required_roles: vec!["ops".into()],
            k: 1,
            n: 1,
            approvals: vec![crate::adapters::permit::Approval {
                role: "ops".into(),
                ts: 2_000_000,
                sig_hex: None,
            }],
            created_at: 1_000_000,
            closed_at: Some(2_000_000),
        };
        // Save ticket directly
        let ticket_dir = permit_dir.join("permit-tickets").join("acme");
        std::fs::create_dir_all(&ticket_dir).unwrap();
        std::fs::write(
            ticket_dir.join("t-001.json"),
            serde_json::to_string_pretty(&ticket).unwrap(),
        )
        .unwrap();

        let ready = check_resumable(&resume_store, &permit_store, "acme").unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].job_id, "j-001");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_resumable_pending_ticket_not_ready() {
        let (resume_store, dir) = temp_store("not_ready");
        let permit_dir = dir.join("permit-state");

        let job = sample_job();
        resume_store.save(&job).unwrap();

        let permit_store = crate::adapters::permit::PermitStore::new(&permit_dir);
        let ticket = crate::adapters::permit::Ticket {
            ticket_id: "t-001".into(),
            tenant: "acme".into(),
            status: crate::adapters::permit::TicketStatus::Pending,
            expires_at: i64::MAX,
            required_roles: vec!["ops".into()],
            k: 1,
            n: 1,
            approvals: vec![],
            created_at: 1_000_000,
            closed_at: None,
        };
        let ticket_dir = permit_dir.join("permit-tickets").join("acme");
        std::fs::create_dir_all(&ticket_dir).unwrap();
        std::fs::write(
            ticket_dir.join("t-001.json"),
            serde_json::to_string_pretty(&ticket).unwrap(),
        )
        .unwrap();

        let ready = check_resumable(&resume_store, &permit_store, "acme").unwrap();
        assert_eq!(ready.len(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
