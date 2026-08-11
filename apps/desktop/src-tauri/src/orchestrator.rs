use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use uuid::Uuid;

use crate::orchestrator_config::OrchestratorConfig;
use crate::registry::ProfileRegistry;
use crate::resource::{ResourceMonitor, ResourceStatus};

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    LimitExceeded,
    ResourceExhausted,
    CircuitBreakerOpen,
    ProfileNotFound,
    ProfileAlreadyOpen,
    AuthFailed,
    InvalidRequest,
    ProfileCooldown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    Navigate,
    Comment,
    Close,
    CustomScript,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub profile_id: String,
    pub job_type: JobType,
    pub payload: Value,
    #[serde(default = "default_priority")]
    pub priority: u8,
    pub state: JobState,
    pub created_at: String,
    pub started_at: Option<String>,
    pub error: Option<String>,
}

fn default_priority() -> u8 {
    128
}

#[derive(Clone, Debug)]
pub struct CircuitBreakerState {
    pub profile_id: String,
    pub failure_count: u8,
    pub cooldown_until: Option<SystemTime>,
    pub max_failures: u8,
    pub cooldown_secs: u64,
}

impl CircuitBreakerState {
    fn new(profile_id: &str) -> Self {
        Self {
            profile_id: profile_id.to_string(),
            failure_count: 0,
            cooldown_until: None,
            max_failures: 3,
            cooldown_secs: 120,
        }
    }

    pub fn is_open(&self) -> bool {
        if let Some(cd) = self.cooldown_until {
            SystemTime::now() < cd
        } else {
            false
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        if self.failure_count >= self.max_failures {
            self.cooldown_until =
                Some(SystemTime::now() + std::time::Duration::from_secs(self.cooldown_secs));
        }
    }

    pub fn reset(&mut self) {
        self.failure_count = 0;
        self.cooldown_until = None;
    }
}

pub struct JobQueue {
    queues: Mutex<HashMap<String, VecDeque<Job>>>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            queues: Mutex::new(HashMap::new()),
        }
    }

    pub fn enqueue(&self, job: Job) -> usize {
        let mut qs = self.queues.lock().unwrap();
        let entry = qs.entry(job.profile_id.clone()).or_default();
        let pos = entry.len() + 1;
        entry.push_back(job);
        // re-sort by priority (lower number = higher priority)
        entry.make_contiguous().sort_by_key(|j| j.priority);
        pos
    }

    pub fn dequeue(&self, profile_id: &str) -> Option<Job> {
        self.queues
            .lock()
            .unwrap()
            .get_mut(profile_id)
            .and_then(|q| q.pop_front())
    }

    pub fn pending_count(&self) -> usize {
        self.queues
            .lock()
            .unwrap()
            .values()
            .map(|q| q.len())
            .sum()
    }

    pub fn profile_queue_len(&self, profile_id: &str) -> usize {
        self.queues
            .lock()
            .unwrap()
            .get(profile_id)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    pub fn queued_ids(&self) -> Vec<String> {
        self.queues.lock().unwrap().keys().cloned().collect()
    }
}

pub struct Orchestrator {
    pub config: OrchestratorConfig,
    pub resource_monitor: ResourceMonitor,
    pub job_queue: JobQueue,
    pub circuit_breakers: Mutex<HashMap<String, CircuitBreakerState>>,
    pub registry: Arc<ProfileRegistry>,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig, registry: Arc<ProfileRegistry>) -> Self {
        let resource_monitor = ResourceMonitor::new(config.clone());
        Self {
            config,
            resource_monitor,
            job_queue: JobQueue::new(),
            circuit_breakers: Mutex::new(HashMap::new()),
            registry,
        }
    }

    pub fn check_can_open(&self, profile_id: &str) -> Result<(), (ReasonCode, String)> {
        {
            let cbs = self.circuit_breakers.lock().unwrap();
            if let Some(cb) = cbs.get(profile_id) {
                if cb.is_open() {
                    return Err((
                        ReasonCode::CircuitBreakerOpen,
                        format!("profile {} is in cooldown", profile_id),
                    ));
                }
            }
        }

        let active_count = self.registry.len();
        if active_count >= self.config.max_active_profiles {
            let active_ids = self.registry.ids();
            return Err((
                ReasonCode::LimitExceeded,
                format!(
                    "max_active_profiles {}/{}",
                    active_count, self.config.max_active_profiles
                ),
            ));
        }

        let status = self.resource_monitor.check_thresholds();
        if status.exhausted {
            return Err((ReasonCode::ResourceExhausted, status.reasons.join("; ")));
        }

        Ok(())
    }

    pub fn record_failure(&self, profile_id: &str) {
        let mut cbs = self.circuit_breakers.lock().unwrap();
        let entry = cbs
            .entry(profile_id.to_string())
            .or_insert_with(|| CircuitBreakerState::new(profile_id));
        entry.record_failure();
    }

    pub fn record_success(&self, profile_id: &str) {
        let mut cbs = self.circuit_breakers.lock().unwrap();
        if let Some(cb) = cbs.get_mut(profile_id) {
            cb.reset();
        }
    }

    pub fn enqueue_open_request(&self, profile_id: &str) -> Option<usize> {
        if !self.config.queue_enabled {
            return None;
        }
        let job = Job {
            id: Uuid::new_v4().to_string(),
            profile_id: profile_id.to_string(),
            job_type: JobType::Close,
            payload: Value::Null,
            priority: 128,
            state: JobState::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
            started_at: None,
            error: None,
        };
        Some(self.job_queue.enqueue(job))
    }

    /// Returns list of profile IDs that would be drained from queue
    pub fn try_drain_slots(&self) -> Vec<String> {
        let active = self.registry.len();
        let slots = self.config.max_active_profiles.saturating_sub(active);
        self.job_queue.queued_ids().into_iter().take(slots).collect()
    }

    pub fn status(&self) -> OrchestratorStatus {
        let snap = self.resource_monitor.snapshot();
        let cbs: Vec<CircuitBreakerStatus> = {
            let cbs = self.circuit_breakers.lock().unwrap();
            cbs.iter()
                .map(|(id, cb)| CircuitBreakerStatus {
                    profile_id: id.clone(),
                    state: if cb.is_open() {
                        "open".to_string()
                    } else {
                        "closed".to_string()
                    },
                    cooldown_until: cb.cooldown_until.and_then(|t| {
                        t.duration_since(SystemTime::UNIX_EPOCH)
                            .ok()
                            .map(|d| {
                                chrono::DateTime::from_timestamp(
                                    d.as_secs() as i64,
                                    0,
                                )
                                .unwrap_or_default()
                                .to_rfc3339()
                            })
                    }),
                })
                .collect()
        };

        OrchestratorStatus {
            active_profiles: self.registry.len(),
            max_active_profiles: self.config.max_active_profiles,
            queue_depth: self.job_queue.pending_count(),
            resources: snap,
            circuit_breakers: cbs,
        }
    }

    pub fn snapshot_resources(&self) -> ResourceStatus {
        self.resource_monitor.check_thresholds()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorStatus {
    pub active_profiles: usize,
    pub max_active_profiles: usize,
    pub queue_depth: usize,
    pub resources: crate::resource::ResourceSnapshot,
    pub circuit_breakers: Vec<CircuitBreakerStatus>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerStatus {
    pub profile_id: String,
    pub state: String,
    pub cooldown_until: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator_config::OrchestratorConfig;

    fn test_registry() -> Arc<ProfileRegistry> {
        Arc::new(ProfileRegistry::new())
    }

    fn test_orchestrator() -> Orchestrator {
        Orchestrator::new(OrchestratorConfig::default(), test_registry())
    }

    #[test]
    fn test_can_open_when_empty() {
        let orch = test_orchestrator();
        assert!(orch.check_can_open("p1").is_ok());
    }

    #[test]
    fn test_limit_enforced_at_max() {
        let config = OrchestratorConfig::default();
        let registry = test_registry();
        registry.insert("p1", 9222, None);
        registry.insert("p2", 9223, None);
        registry.insert("p3", 9224, None);
        let orch = Orchestrator::new(config, registry);
        let err = orch.check_can_open("p4").unwrap_err();
        assert_eq!(err.0, ReasonCode::LimitExceeded);
    }

    #[test]
    fn test_limit_not_reached() {
        let config = OrchestratorConfig::default();
        let registry = test_registry();
        registry.insert("p1", 9222, None);
        registry.insert("p2", 9223, None);
        let orch = Orchestrator::new(config, registry);
        assert!(orch.check_can_open("p3").is_ok());
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let orch = test_orchestrator();
        orch.record_failure("p1");
        orch.record_failure("p1");
        orch.record_failure("p1");
        let err = orch.check_can_open("p1").unwrap_err();
        assert_eq!(err.0, ReasonCode::CircuitBreakerOpen);
    }

    #[test]
    fn test_circuit_breaker_not_open_early() {
        let orch = test_orchestrator();
        orch.record_failure("p1");
        orch.record_failure("p1");
        assert!(orch.check_can_open("p1").is_ok());
    }

    #[test]
    fn test_circuit_breaker_reset_on_success() {
        let orch = test_orchestrator();
        orch.record_failure("p1");
        orch.record_failure("p1");
        orch.record_success("p1");
        // After reset, should be back to 0 failures
        orch.record_failure("p1");
        orch.record_failure("p1");
        assert!(orch.check_can_open("p1").is_ok());
        orch.record_failure("p1");
        assert!(orch.check_can_open("p1").is_err());
    }

    #[test]
    fn test_job_queue_fifo() {
        let q = JobQueue::new();
        q.enqueue(Job {
            id: "j1".into(),
            profile_id: "p1".into(),
            job_type: JobType::Comment,
            payload: Value::String("hello".into()),
            priority: 128,
            state: JobState::Pending,
            created_at: "t1".into(),
            started_at: None,
            error: None,
        });
        q.enqueue(Job {
            id: "j2".into(),
            profile_id: "p1".into(),
            job_type: JobType::Navigate,
            payload: Value::String("url".into()),
            priority: 128,
            state: JobState::Pending,
            created_at: "t2".into(),
            started_at: None,
            error: None,
        });
        assert_eq!(q.dequeue("p1").unwrap().id, "j1");
        assert_eq!(q.dequeue("p1").unwrap().id, "j2");
        assert!(q.dequeue("p1").is_none());
    }

    #[test]
    fn test_job_queue_priority() {
        let q = JobQueue::new();
        q.enqueue(Job {
            id: "low".into(),
            profile_id: "p1".into(),
            job_type: JobType::Comment,
            payload: Value::Null,
            priority: 200,
            state: JobState::Pending,
            created_at: "t1".into(),
            started_at: None,
            error: None,
        });
        q.enqueue(Job {
            id: "high".into(),
            profile_id: "p1".into(),
            job_type: JobType::Comment,
            payload: Value::Null,
            priority: 0,
            state: JobState::Pending,
            created_at: "t2".into(),
            started_at: None,
            error: None,
        });
        // Higher priority (lower number) should come first after sorting
        let first = q.dequeue("p1").unwrap();
        assert_eq!(first.id, "high");
    }

    #[test]
    fn test_queue_pending_count() {
        let q = JobQueue::new();
        assert_eq!(q.pending_count(), 0);
        q.enqueue(Job {
            id: "j1".into(),
            profile_id: "p1".into(),
            job_type: JobType::Comment,
            payload: Value::Null,
            priority: 128,
            state: JobState::Pending,
            created_at: "t1".into(),
            started_at: None,
            error: None,
        });
        q.enqueue(Job {
            id: "j2".into(),
            profile_id: "p2".into(),
            job_type: JobType::Navigate,
            payload: Value::Null,
            priority: 128,
            state: JobState::Pending,
            created_at: "t2".into(),
            started_at: None,
            error: None,
        });
        assert_eq!(q.pending_count(), 2);
    }

    #[test]
    fn test_orchestrator_status() {
        let orch = test_orchestrator();
        orch.registry.insert("p1", 9222, None);
        let status = orch.status();
        assert_eq!(status.active_profiles, 1);
        assert_eq!(status.max_active_profiles, 3);
        assert_eq!(status.queue_depth, 0);
    }

    #[test]
    fn test_snapshot_resources() {
        let orch = test_orchestrator();
        let rs = orch.snapshot_resources();
        // Should not panic
        let _ = rs.exhausted;
    }
}
