use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use std::process::Stdio;
use std::collections::HashSet;

use crate::parser::{GpuSample, ProcessSample, GpuInfo, GpuTopology, ComputeApp, ProcessSystemInfo};

/// Message types from nvidia-smi processes
#[derive(Debug)]
pub enum NvidiaMessage {
    GpuSample(GpuSample),
    ProcessSample(ProcessSample),
    GpuInfo(Vec<GpuInfo>),
    ComputeApps(Vec<ComputeApp>),
    ProcessSystemInfo(Vec<ProcessSystemInfo>),
    Error(String),
    Exited(String),
}

/// Manages all nvidia-smi processes
pub struct NvidiaMonitor {
    #[allow(dead_code)]
    dmon_child: Child,
    #[allow(dead_code)]
    pmon_child: Child,
}

impl NvidiaMonitor {
    pub async fn query_topology() -> Result<GpuTopology> {
        let output = Command::new("nvidia-smi")
            .args(["topo", "-m"])
            .output()
            .await
            .context("Failed to run nvidia-smi topo")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(GpuTopology::parse(&stdout))
    }

    pub async fn query_gpu_info() -> Result<Vec<GpuInfo>> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,uuid,driver_version,memory.total,memory.used,memory.free,power.limit,power.draw,temperature.gpu,temperature.gpu.tlimit,pcie.link.gen.current,pcie.link.gen.max,pcie.link.width.current,pcie.link.width.max,fan.speed,pstate",
                "--format=csv,noheader,nounits"
            ])
            .output()
            .await
            .context("Failed to run nvidia-smi query-gpu")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut gpus = Vec::new();

        for (idx, line) in stdout.lines().enumerate() {
            if let Some(info) = GpuInfo::parse_csv_line(line, idx as u32) {
                gpus.push(info);
            }
        }

        Ok(gpus)
    }

    /// Query per-process VRAM usage
    pub async fn query_compute_apps() -> Result<Vec<ComputeApp>> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-compute-apps=pid,process_name,gpu_uuid,used_memory",
                "--format=csv"
            ])
            .output()
            .await
            .context("Failed to run nvidia-smi query-compute-apps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let apps: Vec<ComputeApp> = stdout
            .lines()
            .filter_map(|line| ComputeApp::parse_csv_line(line))
            .collect();

        Ok(apps)
    }

    /// Query system info for given PIDs via ps
    pub async fn query_process_info(pids: &[u32]) -> Result<Vec<ProcessSystemInfo>> {
        if pids.is_empty() {
            return Ok(Vec::new());
        }

        let pid_str = pids.iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let output = Command::new("ps")
            .args(["-p", &pid_str, "-o", "pid,pcpu,rss,etime,args", "--no-headers"])
            .output()
            .await
            .context("Failed to run ps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let infos: Vec<ProcessSystemInfo> = stdout
            .lines()
            .filter_map(|line| ProcessSystemInfo::parse_ps_line(line))
            .collect();

        Ok(infos)
    }

    pub async fn spawn() -> Result<(Self, mpsc::Receiver<NvidiaMessage>)> {
        let (tx, rx) = mpsc::channel(200);

        // Spawn dmon
        let mut dmon_child = Command::new("nvidia-smi")
            .arg("dmon")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn nvidia-smi dmon")?;

        let dmon_stdout = dmon_child.stdout.take().context("Failed to get dmon stdout")?;
        let tx_dmon = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(dmon_stdout);
            let mut lines = reader.lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if let Some(sample) = GpuSample::parse_line(&line) {
                            if tx_dmon.send(NvidiaMessage::GpuSample(sample)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        let _ = tx_dmon.send(NvidiaMessage::Exited("dmon".into())).await;
                        break;
                    }
                    Err(e) => {
                        let _ = tx_dmon.send(NvidiaMessage::Error(format!("dmon: {}", e))).await;
                        break;
                    }
                }
            }
        });

        // Spawn pmon
        let mut pmon_child = Command::new("nvidia-smi")
            .arg("pmon")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn nvidia-smi pmon")?;

        let pmon_stdout = pmon_child.stdout.take().context("Failed to get pmon stdout")?;
        let tx_pmon = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(pmon_stdout);
            let mut lines = reader.lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if let Some(sample) = ProcessSample::parse_line(&line) {
                            if tx_pmon.send(NvidiaMessage::ProcessSample(sample)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        let _ = tx_pmon.send(NvidiaMessage::Exited("pmon".into())).await;
                        break;
                    }
                    Err(e) => {
                        let _ = tx_pmon.send(NvidiaMessage::Error(format!("pmon: {}", e))).await;
                        break;
                    }
                }
            }
        });

        // Spawn periodic query-gpu task
        let tx_query = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
            loop {
                interval.tick().await;

                // Query GPU info
                if let Ok(info) = Self::query_gpu_info().await {
                    if tx_query.send(NvidiaMessage::GpuInfo(info)).await.is_err() {
                        break;
                    }
                }

                // Query compute apps (VRAM per process)
                if let Ok(apps) = Self::query_compute_apps().await {
                    // Collect unique PIDs
                    let pids: Vec<u32> = apps.iter()
                        .map(|a| a.pid)
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect();

                    // Send compute apps
                    if tx_query.send(NvidiaMessage::ComputeApps(apps)).await.is_err() {
                        break;
                    }

                    // Query system info for these PIDs
                    if let Ok(sys_info) = Self::query_process_info(&pids).await {
                        if tx_query.send(NvidiaMessage::ProcessSystemInfo(sys_info)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok((Self { dmon_child, pmon_child }, rx))
    }
}
