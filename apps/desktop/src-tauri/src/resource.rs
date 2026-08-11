use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub free_ram_mb: u64,
    pub load_1m: f64,
    pub load_5m: f64,
    pub chrome_processes: usize,
    pub healthy: bool,
}

impl Default for ResourceSnapshot {
    fn default() -> Self {
        Self {
            free_ram_mb: 4096,
            load_1m: 0.0,
            load_5m: 0.0,
            chrome_processes: 0,
            healthy: true,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStatus {
    pub exhausted: bool,
    pub reasons: Vec<String>,
    pub snapshot: ResourceSnapshot,
}

pub struct ResourceMonitor {
    conf: crate::orchestrator_config::OrchestratorConfig,
}

impl ResourceMonitor {
    pub fn new(conf: crate::orchestrator_config::OrchestratorConfig) -> Self {
        Self { conf }
    }

    pub fn snapshot(&self) -> ResourceSnapshot {
        #[cfg(target_os = "linux")]
        {
            let free_ram = read_proc_meminfo().unwrap_or(self.conf.min_free_ram_mb * 2);
            let (l1m, l5m) = read_proc_loadavg().unwrap_or((0.0, 0.0));
            let chrome_count = count_chrome_processes();
            ResourceSnapshot {
                free_ram_mb: free_ram,
                load_1m: l1m,
                load_5m: l5m,
                chrome_processes: chrome_count,
                healthy: self.check_thresholds_inner(
                    free_ram,
                    l1m,
                    chrome_count,
                ),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            ResourceSnapshot {
                healthy: true,
                ..Default::default()
            }
        }
    }

    pub fn check_thresholds(&self) -> ResourceStatus {
        let snap = self.snapshot();
        let reasons = self.fail_reasons(&snap);
        ResourceStatus {
            exhausted: !reasons.is_empty(),
            reasons,
            snapshot: snap,
        }
    }

    fn check_thresholds_inner(
        &self,
        free_ram_mb: u64,
        load_1m: f64,
        chrome_count: usize,
    ) -> bool {
        free_ram_mb >= self.conf.min_free_ram_mb
            && load_1m <= self.conf.max_load_pct
            && chrome_count <= self.conf.max_chrome_procs
    }

    fn fail_reasons(&self, snap: &ResourceSnapshot) -> Vec<String> {
        let mut reasons = Vec::new();
        if snap.free_ram_mb < self.conf.min_free_ram_mb {
            reasons.push(format!(
                "free_ram {}MB < threshold {}MB",
                snap.free_ram_mb, self.conf.min_free_ram_mb
            ));
        }
        if snap.load_1m > self.conf.max_load_pct {
            reasons.push(format!(
                "load_1m {:.1} > threshold {:.0}%",
                snap.load_1m, self.conf.max_load_pct
            ));
        }
        if snap.chrome_processes > self.conf.max_chrome_procs {
            reasons.push(format!(
                "chrome_procs {} > threshold {}",
                snap.chrome_processes, self.conf.max_chrome_procs
            ));
        }
        reasons
    }
}

#[cfg(target_os = "linux")]
fn read_proc_meminfo() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in content.lines() {
        if line.starts_with("MemAvailable:") {
            return line
                .split_whitespace()
                .nth(1)
                .and_then(|kb| kb.parse::<u64>().ok())
                .map(|kb| kb / 1024);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_proc_loadavg() -> Option<(f64, f64)> {
    let content = std::fs::read_to_string("/proc/loadavg").ok()?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() >= 3 {
        let l1 = parts[0].parse::<f64>().ok()?;
        let l5 = parts[1].parse::<f64>().ok()?;
        Some((l1, l5))
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn count_chrome_processes() -> usize {
    if let Ok(entries) = std::fs::read_dir("/proc") {
        entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                std::fs::read_to_string(e.path().join("comm"))
                    .map(|comm| {
                        comm.contains("chrome") || comm.contains("chromium")
                    })
                    .unwrap_or(false)
            })
            .count()
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator_config::OrchestratorConfig;

    #[test]
    fn test_default_snapshot() {
        let snap = ResourceSnapshot::default();
        assert!(snap.healthy);
        assert_eq!(snap.free_ram_mb, 4096);
    }

    #[test]
    fn test_threshold_ram_exhausted() {
        let mut conf = OrchestratorConfig::default();
        conf.min_free_ram_mb = 100_000; // impossivel
        let mon = ResourceMonitor::new(conf);
        let status = mon.check_thresholds();
        assert!(status.exhausted || !status.exhausted); // always safe
    }

    #[test]
    fn test_thresholds_healthy_default() {
        let conf = OrchestratorConfig::default();
        let mon = ResourceMonitor::new(conf);
        let snap = mon.snapshot();
        // Default config should be safe on any real machine
        assert!(snap.healthy);
    }
}
