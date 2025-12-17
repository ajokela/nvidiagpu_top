/// Parsers for nvidia-smi output formats

// ============================================================================
// DMON Parser (device monitoring)
// ============================================================================
/// Sample output format:
/// # gpu    pwr  gtemp  mtemp     sm    mem    enc    dec    jpg    ofa   mclk   pclk
/// # Idx      W      C      C      %      %      %      %      %      %    MHz    MHz
///     0     69     13      -    100     30      0      0      -      -   3615   1531

/// A single GPU sample from nvidia-smi dmon
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct GpuSample {
    pub gpu_idx: u32,
    pub power_w: Option<u32>,
    pub gpu_temp_c: Option<u32>,
    pub mem_temp_c: Option<u32>,
    pub sm_util: Option<u32>,
    pub mem_util: Option<u32>,
    pub enc_util: Option<u32>,
    pub dec_util: Option<u32>,
    pub jpg_util: Option<u32>,
    pub ofa_util: Option<u32>,
    pub mem_clock_mhz: Option<u32>,
    pub gpu_clock_mhz: Option<u32>,
}

impl GpuSample {
    /// Parse a single value, treating "-" as None
    fn parse_optional(s: &str) -> Option<u32> {
        let trimmed = s.trim();
        if trimmed == "-" || trimmed.is_empty() {
            None
        } else {
            trimmed.parse().ok()
        }
    }

    /// Parse a line of nvidia-smi dmon output
    /// Returns None if this is a header line (starts with #) or invalid
    pub fn parse_line(line: &str) -> Option<Self> {
        let line = line.trim();

        // Skip header lines
        if line.starts_with('#') || line.is_empty() {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        // We expect at least 12 fields
        if parts.len() < 12 {
            return None;
        }

        Some(Self {
            gpu_idx: parts[0].parse().ok()?,
            power_w: Self::parse_optional(parts[1]),
            gpu_temp_c: Self::parse_optional(parts[2]),
            mem_temp_c: Self::parse_optional(parts[3]),
            sm_util: Self::parse_optional(parts[4]),
            mem_util: Self::parse_optional(parts[5]),
            enc_util: Self::parse_optional(parts[6]),
            dec_util: Self::parse_optional(parts[7]),
            jpg_util: Self::parse_optional(parts[8]),
            ofa_util: Self::parse_optional(parts[9]),
            mem_clock_mhz: Self::parse_optional(parts[10]),
            gpu_clock_mhz: Self::parse_optional(parts[11]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_line() {
        let line = "    0     69     13      -    100     30      0      0      -      -   3615   1531";
        let sample = GpuSample::parse_line(line).unwrap();

        assert_eq!(sample.gpu_idx, 0);
        assert_eq!(sample.power_w, Some(69));
        assert_eq!(sample.gpu_temp_c, Some(13));
        assert_eq!(sample.mem_temp_c, None);
        assert_eq!(sample.sm_util, Some(100));
        assert_eq!(sample.mem_util, Some(30));
        assert_eq!(sample.enc_util, Some(0));
        assert_eq!(sample.dec_util, Some(0));
        assert_eq!(sample.jpg_util, None);
        assert_eq!(sample.ofa_util, None);
        assert_eq!(sample.mem_clock_mhz, Some(3615));
        assert_eq!(sample.gpu_clock_mhz, Some(1531));
    }

    #[test]
    fn test_skip_header_lines() {
        assert!(GpuSample::parse_line("# gpu    pwr  gtemp  mtemp").is_none());
        assert!(GpuSample::parse_line("# Idx      W      C      C").is_none());
    }

    #[test]
    fn test_skip_empty_lines() {
        assert!(GpuSample::parse_line("").is_none());
        assert!(GpuSample::parse_line("   ").is_none());
    }
}

// ============================================================================
// PMON Parser (process monitoring)
// ============================================================================
/// Sample output format:
/// # gpu    pid   type     sm    mem    enc    dec    jpg    ofa    command
/// # Idx      #    C/G      %      %      %      %      %      %    name
///     0  21093     C      -      -      -      -      -      -    llama-server
///     1  27581     C     99     14      -      -      -      -    python

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProcessSample {
    pub gpu_idx: u32,
    pub pid: u32,
    pub process_type: String,  // C = Compute, G = Graphics
    pub sm_util: Option<u32>,
    pub mem_util: Option<u32>,
    pub enc_util: Option<u32>,
    pub dec_util: Option<u32>,
    pub command: String,
}

// ============================================================================
// Compute Apps Parser (per-process VRAM usage)
// ============================================================================
/// From: nvidia-smi --query-compute-apps=pid,name,gpu_uuid,used_memory --format=csv

#[derive(Debug, Clone)]
pub struct ComputeApp {
    pub pid: u32,
    pub name: String,
    pub gpu_uuid: String,
    pub vram_used_mib: u64,
}

impl ComputeApp {
    pub fn parse_csv_line(line: &str) -> Option<Self> {
        // Skip header
        if line.starts_with("pid") {
            return None;
        }

        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            return None;
        }

        let vram_str = parts[3].replace(" MiB", "").replace("[N/A]", "0");

        Some(Self {
            pid: parts[0].parse().ok()?,
            name: parts[1].to_string(),
            gpu_uuid: parts[2].to_string(),
            vram_used_mib: vram_str.trim().parse().unwrap_or(0),
        })
    }
}

// ============================================================================
// Process System Info (from /proc via ps)
// ============================================================================
#[derive(Debug, Clone, Default)]
pub struct ProcessSystemInfo {
    pub pid: u32,
    pub cpu_percent: f32,
    pub rss_kb: u64,        // System RAM in KB
    pub elapsed: String,    // Runtime
}

impl ProcessSystemInfo {
    /// Parse output from: ps -p <pids> -o pid,pcpu,rss,etime --no-headers
    pub fn parse_ps_line(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let mut parts = line.split_whitespace();
        let pid: u32 = parts.next()?.parse().ok()?;
        let cpu_str = parts.next()?;
        let rss_str = parts.next()?;
        let elapsed = parts.next()?.to_string();

        Some(Self {
            pid,
            cpu_percent: cpu_str.parse().unwrap_or(0.0),
            rss_kb: rss_str.parse().unwrap_or(0),
            elapsed,
        })
    }
}

impl ProcessSample {
    fn parse_optional(s: &str) -> Option<u32> {
        let trimmed = s.trim();
        if trimmed == "-" || trimmed.is_empty() {
            None
        } else {
            trimmed.parse().ok()
        }
    }

    pub fn parse_line(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            return None;
        }

        Some(Self {
            gpu_idx: parts[0].parse().ok()?,
            pid: parts[1].parse().ok()?,
            process_type: parts[2].to_string(),
            sm_util: Self::parse_optional(parts[3]),
            mem_util: Self::parse_optional(parts[4]),
            enc_util: Self::parse_optional(parts[5]),
            dec_util: Self::parse_optional(parts[6]),
            // Skip jpg (7) and ofa (8)
            command: parts[9..].join(" "),
        })
    }
}

// ============================================================================
// Query GPU Parser (static and memory info)
// ============================================================================
/// Parsed from: nvidia-smi --query-gpu=... --format=csv,noheader,nounits

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub uuid: String,
    pub driver_version: String,
    pub memory_total_mib: u64,
    pub memory_used_mib: u64,
    pub memory_free_mib: u64,
    pub power_limit_w: Option<f32>,
    pub power_draw_w: Option<f32>,
    pub temperature_c: Option<u32>,
    pub temperature_limit_c: Option<u32>,
    pub pcie_gen_current: Option<u32>,
    pub pcie_gen_max: Option<u32>,
    pub pcie_width_current: Option<u32>,
    pub pcie_width_max: Option<u32>,
    pub fan_speed_pct: Option<u32>,
    pub pstate: String,
    pub throttle_reasons: Vec<String>,
}

impl GpuInfo {
    /// Parse CSV output from nvidia-smi --query-gpu
    pub fn parse_csv_line(line: &str, index: u32) -> Option<Self> {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 16 {
            return None;
        }

        let parse_u32 = |s: &str| -> Option<u32> {
            s.trim().replace("[Not Supported]", "").replace("[N/A]", "").parse().ok()
        };
        let parse_u64 = |s: &str| -> Option<u64> {
            s.trim().replace("[Not Supported]", "").replace("[N/A]", "").parse().ok()
        };
        let parse_f32 = |s: &str| -> Option<f32> {
            s.trim().replace("[Not Supported]", "").replace("[N/A]", "").parse().ok()
        };

        Some(Self {
            index,
            name: parts[0].to_string(),
            uuid: parts[1].to_string(),
            driver_version: parts[2].to_string(),
            memory_total_mib: parse_u64(parts[3]).unwrap_or(0),
            memory_used_mib: parse_u64(parts[4]).unwrap_or(0),
            memory_free_mib: parse_u64(parts[5]).unwrap_or(0),
            power_limit_w: parse_f32(parts[6]),
            power_draw_w: parse_f32(parts[7]),
            temperature_c: parse_u32(parts[8]),
            temperature_limit_c: parse_u32(parts[9]),
            pcie_gen_current: parse_u32(parts[10]),
            pcie_gen_max: parse_u32(parts[11]),
            pcie_width_current: parse_u32(parts[12]),
            pcie_width_max: parse_u32(parts[13]),
            fan_speed_pct: parse_u32(parts[14]),
            pstate: parts[15].to_string(),
            throttle_reasons: Vec::new(),
        })
    }
}

// ============================================================================
// Topology Parser
// ============================================================================
/// GPU interconnect types
#[derive(Debug, Clone, PartialEq)]
pub enum GpuLink {
    Self_,        // X - same GPU
    PIX,          // Single PCIe bridge
    PXB,          // Multiple PCIe bridges
    PHB,          // PCIe Host Bridge
    NODE,         // Same NUMA node
    SYS,          // Cross NUMA (QPI/UPI)
    NVLink(u32),  // NVLink with count
}

#[allow(dead_code)]
impl GpuLink {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "X" => Some(Self::Self_),
            "PIX" => Some(Self::PIX),
            "PXB" => Some(Self::PXB),
            "PHB" => Some(Self::PHB),
            "NODE" => Some(Self::NODE),
            "SYS" => Some(Self::SYS),
            s if s.starts_with("NV") => {
                s[2..].parse().ok().map(Self::NVLink)
            }
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Self_ => "Self",
            Self::PIX => "Single PCIe bridge (fast)",
            Self::PXB => "Multiple PCIe bridges",
            Self::PHB => "PCIe Host Bridge",
            Self::NODE => "Same NUMA node",
            Self::SYS => "Cross NUMA (slow)",
            Self::NVLink(n) => match n {
                1 => "NVLink x1",
                2 => "NVLink x2",
                _ => "NVLink",
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GpuTopology {
    pub gpu_count: usize,
    pub matrix: Vec<Vec<Option<GpuLink>>>,
    pub cpu_affinity: Vec<String>,
    pub numa_affinity: Vec<String>,
}

impl GpuTopology {
    /// Parse nvidia-smi topo -m output
    pub fn parse(output: &str) -> Self {
        let lines: Vec<&str> = output.lines().collect();
        let mut topo = Self::default();

        // Find the header line and data lines
        for (i, line) in lines.iter().enumerate() {
            // Skip until we find a line starting with GPU0
            if line.trim().starts_with("GPU0") {
                // Parse data lines
                for data_line in &lines[i..] {
                    if data_line.trim().is_empty() || data_line.starts_with("Legend") {
                        break;
                    }
                    if data_line.trim().starts_with("GPU") {
                        topo.parse_topo_line(data_line);
                    }
                }
                break;
            }
        }

        topo
    }

    fn parse_topo_line(&mut self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() || !parts[0].starts_with("GPU") {
            return;
        }

        let mut row = Vec::new();
        for (i, part) in parts.iter().enumerate().skip(1) {
            if let Some(link) = GpuLink::from_str(part) {
                row.push(Some(link));
            } else if i <= self.gpu_count + 1 {
                row.push(None);
            } else {
                // CPU/NUMA affinity columns
                if self.cpu_affinity.len() < self.matrix.len() + 1 {
                    self.cpu_affinity.push(part.to_string());
                } else if self.numa_affinity.len() < self.matrix.len() + 1 {
                    self.numa_affinity.push(part.to_string());
                }
            }
        }

        if !row.is_empty() {
            self.gpu_count = self.gpu_count.max(row.len());
            self.matrix.push(row);
        }
    }
}
