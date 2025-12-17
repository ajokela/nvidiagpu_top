# nvidiagpu_top

A terminal UI for monitoring NVIDIA GPU metrics in real-time.

## Features

- Real-time GPU metrics (power, temperature, utilization, clocks)
- Memory usage with visual bars
- Process monitoring with VRAM allocation, SM utilization, CPU%, and runtime
- Historical charts for GPU metrics
- GPU topology view (NVLink, PCIe interconnects)
- Detailed GPU info overlay

## Note on Data Availability

Unlike AMD's open-source drivers, which expose detailed GPU internals and enable feature-rich tools like [amdgpu_top](https://github.com/Umio-Yasuno/amdgpu_top), NVIDIA's proprietary drivers provide limited access to GPU metrics. This tool works within those constraints, using `nvidia-smi` to surface what data is available.

## Requirements

- Linux with NVIDIA drivers
- `nvidia-smi` in PATH

## Building

```bash
cargo build --release
```

## Usage

```bash
./target/release/nvidiagpu_top
```

### Options

- `-h, --history <SECS>` - History retention in seconds (default: 300)

### Keybindings

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit |
| `Tab` | Switch between Dashboard and Charts |
| `1` / `2` | Jump to Dashboard / Charts |
| `j` / `k` or arrows | Select GPU |
| `i` | Toggle GPU info overlay |
| `t` | Toggle topology overlay |

## License

BSD 3-Clause License. See [LICENSE](LICENSE) for details.
