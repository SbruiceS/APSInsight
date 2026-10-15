//! Deterministic, tenant-isolated job lifecycle primitives.
//!
//! This crate is a functional core: it does not read clocks, spawn workers, persist
//! data or log payloads. Adapters supply time and store the returned [`JobQueue`]
//! state. That makes retry and lease behaviour reproducible in tests and across
//! future storage implementations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

const MAX_PAYLOAD_BYTES: usize = 1_048_576;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JobError {
    #[error("invalid {field}: must match [a-z0-9][a-z0-9_-]{{0,63}}")]
    InvalidSlug { field: &'static str },
    #[error("invalid retry policy: {0}")]
    InvalidRetryPolicy(&'static str),
    #[error("job payload exceeds {MAX_PAYLOAD_BYTES} bytes")]
    PayloadTooLarge,
    #[error("job not found")]
    NotFound,
    #[error("job belongs to another tenant")]
    TenantMismatch,
    #[error("job is not in a state that permits {action}")]
    InvalidState { action: &'static str },
    #[error("job lease belongs to another worker or has expired")]
    LeaseMismatch,
}

pub type Result<T> = std::result::Result<T, JobError>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(String);

impl JobId {
    #[must_use]
    pub fn new() -> Self {
        Self(format!("job_{}", Uuid::now_v7().simple()))
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Includes the first execution attempt. `1` disables retries.
    pub max_attempts: u16,
    pub initial_backoff_seconds: i64,
    pub max_backoff_seconds: i64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 1, initial_backoff_seconds: 0, max_backoff_seconds: 0 }
    }
}

impl RetryPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.max_attempts == 0 || self.max_attempts > 100 {
            return Err(JobError::InvalidRetryPolicy("max_attempts must be 1..=100"));
        }
        if self.initial_backoff_seconds < 0 || self.max_backoff_seconds < self.initial_backoff_seconds {
            return Err(JobError::InvalidRetryPolicy("backoffs must be non-negative and ordered"));
        }
        Ok(())
    }

    fn delay_after(&self, attempt: u16) -> Duration {
        let shift = u32::from(attempt.saturating_sub(1)).min(62);
        let multiplier = 1_i64.checked_shl(shift).unwrap_or(i64::MAX);
        Duration::seconds(self.initial_backoff_seconds.saturating_mul(multiplier).min(self.max_backoff_seconds))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    pub tenant: String,
    pub kind: String,
    pub payload: Value,
    #[serde(default)]
    pub retry: RetryPolicy,
    #[serde(with = "time::serde::rfc3339")]
    pub scheduled_at: OffsetDateTime,
}

impl JobSpec {
    pub fn validate(&self) -> Result<()> {
        validate_slug("tenant", &self.tenant)?;
        validate_slug("job kind", &self.kind)?;
        self.retry.validate()?;
        if serde_json::to_vec(&self.payload).map_err(|_| JobError::PayloadTooLarge)?.len() > MAX_PAYLOAD_BYTES {
            return Err(JobError::PayloadTooLarge);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub worker: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub tenant: String,
    pub kind: String,
    pub payload: Value,
    pub retry: RetryPolicy,
    pub status: JobStatus,
    pub attempts: u16,
    #[serde(with = "time::serde::rfc3339")]
    pub scheduled_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "time::serde::rfc3339::option")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "time::serde::rfc3339::option")]
    pub finished_at: Option<OffsetDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<Lease>,
    /// A bounded, caller-supplied failure category. Never put sensitive details here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JobQueue {
    jobs: BTreeMap<JobId, Job>,
}

impl JobQueue {
    pub fn enqueue(&mut self, spec: JobSpec, now: OffsetDateTime) -> Result<Job> {
        spec.validate()?;
        let job = Job {
            id: JobId::new(),
            tenant: spec.tenant,
            kind: spec.kind,
            payload: spec.payload,
            retry: spec.retry,
            status: JobStatus::Queued,
            attempts: 0,
            scheduled_at: spec.scheduled_at,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
            lease: None,
            failure_code: None,
        };
        self.jobs.insert(job.id.clone(), job.clone());
        Ok(job)
    }

    #[must_use]
    pub fn get(&self, id: &JobId) -> Option<&Job> {
        self.jobs.get(id)
    }

    pub fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.jobs.values()
    }

    /// Recovers jobs whose worker stopped renewing its lease. The caller controls
    /// `now`, so recovery is deterministic and can be transactionally persisted.
    pub fn recover_expired(&mut self, now: OffsetDateTime) {
        for job in self.jobs.values_mut() {
            let expired = job.status == JobStatus::Running && job.lease.as_ref().is_some_and(|lease| lease.expires_at <= now);
            if !expired {
                continue;
            }
            job.lease = None;
            job.updated_at = now;
            if job.attempts < job.retry.max_attempts {
                job.status = JobStatus::Queued;
                job.scheduled_at = now + job.retry.delay_after(job.attempts);
            } else {
                job.status = JobStatus::Failed;
                job.finished_at = Some(now);
                job.failure_code = Some("lease_expired".into());
            }
        }
    }

    /// Claims the earliest eligible job among explicitly authorized tenants.
    pub fn claim(&mut self, worker: &str, tenants: &BTreeSet<String>, now: OffsetDateTime, lease_for: Duration) -> Result<Option<Job>> {
        validate_slug("worker", worker)?;
        if lease_for <= Duration::ZERO {
            return Err(JobError::InvalidRetryPolicy("lease duration must be positive"));
        }
        self.recover_expired(now);
        let id = self
            .jobs
            .values()
            .filter(|job| job.status == JobStatus::Queued && job.scheduled_at <= now && tenants.contains(&job.tenant))
            .min_by_key(|job| (job.scheduled_at, job.created_at, job.id.clone()))
            .map(|job| job.id.clone());
        let Some(id) = id else { return Ok(None) };
        let job = self.jobs.get_mut(&id).expect("selected job exists");
        job.status = JobStatus::Running;
        job.attempts += 1;
        job.updated_at = now;
        job.started_at.get_or_insert(now);
        job.lease = Some(Lease { worker: worker.into(), expires_at: now + lease_for });
        Ok(Some(job.clone()))
    }

    pub fn succeed(&mut self, id: &JobId, worker: &str, now: OffsetDateTime) -> Result<Job> {
        let job = self.active_by_worker(id, worker, now)?;
        job.status = JobStatus::Succeeded;
        job.updated_at = now;
        job.finished_at = Some(now);
        job.lease = None;
        Ok(job.clone())
    }

    pub fn fail(&mut self, id: &JobId, worker: &str, code: &str, now: OffsetDateTime, retryable: bool) -> Result<Job> {
        validate_slug("failure code", code)?;
        let job = self.active_by_worker(id, worker, now)?;
        job.updated_at = now;
        job.lease = None;
        job.failure_code = Some(code.into());
        if retryable && job.attempts < job.retry.max_attempts {
            job.status = JobStatus::Queued;
            job.scheduled_at = now + job.retry.delay_after(job.attempts);
        } else {
            job.status = JobStatus::Failed;
            job.finished_at = Some(now);
        }
        Ok(job.clone())
    }

    pub fn cancel(&mut self, tenant: &str, id: &JobId, now: OffsetDateTime) -> Result<Job> {
        validate_slug("tenant", tenant)?;
        let job = self.jobs.get_mut(id).ok_or(JobError::NotFound)?;
        if job.tenant != tenant {
            return Err(JobError::TenantMismatch);
        }
        if !matches!(job.status, JobStatus::Queued | JobStatus::Running) {
            return Err(JobError::InvalidState { action: "cancellation" });
        }
        job.status = JobStatus::Cancelled;
        job.updated_at = now;
        job.finished_at = Some(now);
        job.lease = None;
        Ok(job.clone())
    }

    fn active_by_worker(&mut self, id: &JobId, worker: &str, now: OffsetDateTime) -> Result<&mut Job> {
        validate_slug("worker", worker)?;
        let job = self.jobs.get_mut(id).ok_or(JobError::NotFound)?;
        let owned = job.status == JobStatus::Running && job.lease.as_ref().is_some_and(|lease| lease.worker == worker && lease.expires_at > now);
        if !owned {
            return Err(JobError::LeaseMismatch);
        }
        Ok(job)
    }
}

fn validate_slug(field: &'static str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
        && value.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_'));
    if valid { Ok(()) } else { Err(JobError::InvalidSlug { field }) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn now() -> OffsetDateTime {
        datetime!(2026-10-15 10:00 UTC)
    }

    fn spec(tenant: &str) -> JobSpec {
        JobSpec {
            tenant: tenant.into(),
            kind: "ingest_csv".into(),
            payload: serde_json::json!({ "source": "synthetic.csv" }),
            retry: RetryPolicy { max_attempts: 3, initial_backoff_seconds: 10, max_backoff_seconds: 60 },
            scheduled_at: now(),
        }
    }

    #[test]
    fn claims_are_ordered_and_tenant_isolated() {
        let mut queue = JobQueue::default();
        let foreign = queue.enqueue(spec("other"), now()).unwrap();
        let ours = queue.enqueue(spec("demo"), now()).unwrap();
        let mut tenants = BTreeSet::new();
        tenants.insert("demo".into());
        let claimed = queue.claim("worker_1", &tenants, now(), Duration::minutes(1)).unwrap().unwrap();
        assert_eq!(claimed.id, ours.id);
        assert_eq!(queue.get(&foreign.id).unwrap().status, JobStatus::Queued);
    }

    #[test]
    fn retry_is_delayed_and_eventually_fails() {
        let mut queue = JobQueue::default();
        let job = queue.enqueue(spec("demo"), now()).unwrap();
        let tenants = BTreeSet::from(["demo".into()]);
        let claimed = queue.claim("worker_1", &tenants, now(), Duration::minutes(1)).unwrap().unwrap();
        let retry = queue.fail(&claimed.id, "worker_1", "network_error", now(), true).unwrap();
        assert_eq!(retry.status, JobStatus::Queued);
        assert_eq!(retry.scheduled_at, now() + Duration::seconds(10));
        assert!(queue.claim("worker_1", &tenants, now(), Duration::minutes(1)).unwrap().is_none());
        let second = queue.claim("worker_1", &tenants, now() + Duration::seconds(10), Duration::minutes(1)).unwrap().unwrap();
        queue.fail(&second.id, "worker_1", "network_error", now() + Duration::seconds(10), true).unwrap();
        let third = queue.claim("worker_1", &tenants, now() + Duration::seconds(30), Duration::minutes(1)).unwrap().unwrap();
        let failed = queue.fail(&third.id, "worker_1", "network_error", now() + Duration::seconds(30), true).unwrap();
        assert_eq!(failed.status, JobStatus::Failed);
        assert_eq!(failed.id, job.id);
    }

    #[test]
    fn expired_leases_recover_without_cross_worker_completion() {
        let mut queue = JobQueue::default();
        let job = queue.enqueue(spec("demo"), now()).unwrap();
        let tenants = BTreeSet::from(["demo".into()]);
        queue.claim("worker_1", &tenants, now(), Duration::seconds(5)).unwrap();
        assert_eq!(queue.succeed(&job.id, "worker_2", now()), Err(JobError::LeaseMismatch));
        queue.recover_expired(now() + Duration::seconds(5));
        let recovered = queue.get(&job.id).unwrap();
        assert_eq!(recovered.status, JobStatus::Queued);
        assert_eq!(recovered.scheduled_at, now() + Duration::seconds(15));
    }

    #[test]
    fn cancellation_is_tenant_scoped_and_terminal() {
        let mut queue = JobQueue::default();
        let job = queue.enqueue(spec("demo"), now()).unwrap();
        assert_eq!(queue.cancel("other", &job.id, now()), Err(JobError::TenantMismatch));
        let cancelled = queue.cancel("demo", &job.id, now()).unwrap();
        assert_eq!(cancelled.status, JobStatus::Cancelled);
        assert!(matches!(queue.cancel("demo", &job.id, now()), Err(JobError::InvalidState { .. })));
    }

    #[test]
    fn malformed_inputs_and_oversized_payloads_are_rejected() {
        let mut invalid = spec("DEMO");
        assert!(matches!(invalid.validate(), Err(JobError::InvalidSlug { .. })));
        invalid = spec("demo");
        invalid.payload = Value::String("a".repeat(MAX_PAYLOAD_BYTES + 1));
        assert_eq!(invalid.validate(), Err(JobError::PayloadTooLarge));
    }
}
