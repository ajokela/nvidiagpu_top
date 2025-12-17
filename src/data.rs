use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::parser::{GpuSample, ProcessSample, GpuInfo, GpuTopology, ComputeApp, ProcessSystemInfo};

/// A timestamped GPU sample
#[derive(Debug, Clone)]
pub struct TimestampedSample {
    pub sample: GpuSample,
    pub timestamp: Instant,
}

/// Ring buffer for storing historical GPU data
#[derive(Debug)]
pub struct GpuHistory {
    samples: VecDeque<TimestampedSample>,
    max_samples: usize,
}

impl GpuHistory {
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    pub fn push(&mut self, sample: GpuSample) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(TimestampedSample {
            sample,
            timestamp: Instant::now(),
        });
    }

    pub fn latest(&self) -> Option<&GpuSample> {
        self.samples.back().map(|ts| &ts.sample)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn recent_values<F>(&self, count: usize, extractor: F) -> Vec<f64>
    where
        F: Fn(&GpuSample) -> Option<u32>,
    {
        self.samples
            .iter()
            .rev()
            .take(count)
            .filter_map(|ts| extractor(&ts.sample).map(|v| v as f64))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn chart_data<F>(&self, extractor: F) -> Vec<(f64, f64)>
    where
        F: Fn(&GpuSample) -> Option<u32>,
    {
        let now = Instant::now();
        self.samples
            .iter()
            .filter_map(|ts| {
                extractor(&ts.sample).map(|v| {
                    let secs_ago = now.duration_since(ts.timestamp).as_secs_f64();
                    (-secs_ago, v as f64)
                })
            })
            .collect()
    }
}

/// Process info with timestamp for cleanup
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub sample: ProcessSample,
    pub last_seen: Instant,
}

/// Combined process data from multiple sources
#[derive(Debug, Clone)]
pub struct EnrichedProcess {
    pub pid: u32,
    pub command: String,
    pub gpu_idx: u32,
    pub vram_mib: u64,          // From compute-apps
    pub sm_util: Option<u32>,   // From pmon (instantaneous)
    pub cpu_percent: f32,       // From ps
    pub rss_mb: u64,            // System RAM from ps
    pub elapsed: String,        // Runtime
}

/// Data store for all GPUs
#[derive(Debug)]
pub struct DataStore {
    // Historical samples from dmon
    gpus: HashMap<u32, GpuHistory>,
    max_samples: usize,
    total_samples: u64,
    start_time: Instant,

    // Process monitoring from pmon
    processes: HashMap<(u32, u32), ProcessInfo>, // (gpu_idx, pid) -> info

    // Compute apps (VRAM per process) - key is (gpu_uuid, pid)
    compute_apps: Vec<ComputeApp>,

    // System info per process
    process_sys_info: HashMap<u32, ProcessSystemInfo>, // pid -> info

    // Static GPU info from query-gpu
    gpu_info: HashMap<u32, GpuInfo>,

    // Topology
    topology: Option<GpuTopology>,
}

impl DataStore {
    pub fn new(history_seconds: u64) -> Self {
        let max_samples = history_seconds as usize;
        Self {
            gpus: HashMap::new(),
            max_samples,
            total_samples: 0,
            start_time: Instant::now(),
            processes: HashMap::new(),
            compute_apps: Vec::new(),
            process_sys_info: HashMap::new(),
            gpu_info: HashMap::new(),
            topology: None,
        }
    }

    // ========== DMON data ==========
    pub fn add_sample(&mut self, sample: GpuSample) {
        let gpu_idx = sample.gpu_idx;
        self.gpus
            .entry(gpu_idx)
            .or_insert_with(|| GpuHistory::new(self.max_samples))
            .push(sample);
        self.total_samples += 1;
    }

    pub fn get_gpu(&self, idx: u32) -> Option<&GpuHistory> {
        self.gpus.get(&idx)
    }

    pub fn gpu_indices(&self) -> Vec<u32> {
        let mut indices: Vec<_> = self.gpus.keys().copied().collect();
        indices.sort();
        indices
    }

    pub fn total_samples(&self) -> u64 {
        self.total_samples
    }

    pub fn uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    // ========== PMON data ==========
    pub fn add_process_sample(&mut self, sample: ProcessSample) {
        let key = (sample.gpu_idx, sample.pid);
        self.processes.insert(key, ProcessInfo {
            sample,
            last_seen: Instant::now(),
        });

        let cutoff = Instant::now() - std::time::Duration::from_secs(5);
        self.processes.retain(|_, v| v.last_seen > cutoff);
    }

    #[allow(dead_code)]
    pub fn get_processes(&self) -> Vec<&ProcessInfo> {
        let mut procs: Vec<_> = self.processes.values().collect();
        procs.sort_by_key(|p| (p.sample.gpu_idx, p.sample.pid));
        procs
    }

    // ========== Compute Apps ==========
    pub fn update_compute_apps(&mut self, apps: Vec<ComputeApp>) {
        self.compute_apps = apps;
    }

    // ========== Process System Info ==========
    pub fn update_process_sys_info(&mut self, infos: Vec<ProcessSystemInfo>) {
        self.process_sys_info.clear();
        for info in infos {
            self.process_sys_info.insert(info.pid, info);
        }
    }

    // ========== Enriched Process View ==========
    /// Get enriched process data combining all sources
    pub fn get_enriched_processes(&self) -> Vec<EnrichedProcess> {
        let mut result = Vec::new();

        // Build GPU index lookup from UUID
        let uuid_to_idx: HashMap<&str, u32> = self.gpu_info
            .values()
            .map(|g| (g.uuid.as_str(), g.index))
            .collect();

        // Group compute apps by (pid, gpu_idx)
        for app in &self.compute_apps {
            let gpu_idx = uuid_to_idx.get(app.gpu_uuid.as_str()).copied().unwrap_or(0);

            // Get pmon data if available
            let pmon = self.processes.get(&(gpu_idx, app.pid));

            // Get system info if available
            let sys_info = self.process_sys_info.get(&app.pid);

            let enriched = EnrichedProcess {
                pid: app.pid,
                command: app.name.split('/').last().unwrap_or(&app.name).to_string(),
                gpu_idx,
                vram_mib: app.vram_used_mib,
                sm_util: pmon.and_then(|p| p.sample.sm_util),
                cpu_percent: sys_info.map(|s| s.cpu_percent).unwrap_or(0.0),
                rss_mb: sys_info.map(|s| s.rss_kb / 1024).unwrap_or(0),
                elapsed: sys_info.map(|s| s.elapsed.clone()).unwrap_or_default(),
            };

            result.push(enriched);
        }

        // Sort by GPU then by VRAM usage (descending)
        result.sort_by(|a, b| {
            a.gpu_idx.cmp(&b.gpu_idx)
                .then(b.vram_mib.cmp(&a.vram_mib))
        });

        result
    }

    // ========== Query GPU data ==========
    pub fn update_gpu_info(&mut self, info: Vec<GpuInfo>) {
        for gpu in info {
            self.gpu_info.insert(gpu.index, gpu);
        }
    }

    pub fn get_gpu_info(&self, idx: u32) -> Option<&GpuInfo> {
        self.gpu_info.get(&idx)
    }

    pub fn all_gpu_info(&self) -> Vec<&GpuInfo> {
        let mut infos: Vec<_> = self.gpu_info.values().collect();
        infos.sort_by_key(|i| i.index);
        infos
    }

    // ========== Topology ==========
    pub fn set_topology(&mut self, topology: GpuTopology) {
        self.topology = Some(topology);
    }

    pub fn get_topology(&self) -> Option<&GpuTopology> {
        self.topology.as_ref()
    }
}
