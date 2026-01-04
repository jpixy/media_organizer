# 06 - Ollama GPU Setup Guide

This document covers configuring Ollama to use NVIDIA GPU on Fedora Linux.

---

## 1. Problem Description

### Symptoms

1. Ollama shows `entering low vram mode` and `total vram="0 B"` at startup
2. Inference is extremely slow, often timing out (>180 seconds)
3. Logs show `initial_count=0`, indicating no GPU detected
4. `nvidia-smi` command doesn't exist or shows `nouveau` driver

### Example Logs

```
time=2025-12-29T16:48:22.901+08:00 level=INFO source=types.go:60 msg="inference compute" id=cpu library=cpu
time=2025-12-29T16:48:22.901+08:00 level=INFO source=routes.go:1648 msg="entering low vram mode" "total vram"="0 B"
```

---

## 2. Environment Info

| Item | Version/Info |
|------|--------------|
| **OS** | Fedora 42 (Linux 6.17.13) |
| **GPU** | NVIDIA RTX 3500 Ada Generation Laptop GPU |
| **VRAM** | 12GB |
| **Ollama Version** | 0.13.5 |
| **CUDA Version** | 13.0 |
| **Driver Version** | 580.119.02 |

---

## 3. Diagnostic Steps

### Step 1: Check GPU Hardware

```bash
lspci | grep -i nvidia
```

Expected output:
```
01:00.0 3D controller: NVIDIA Corporation AD106M [GeForce RTX 3500 Ada Generation Laptop GPU]
```

### Step 2: Check Current Driver

```bash
lsmod | grep -E "nvidia|nouveau"
```

- If shows `nouveau`: Open source driver, need to install NVIDIA proprietary driver
- If shows `nvidia`: Proprietary driver is loaded

### Step 3: Check nvidia-smi

```bash
nvidia-smi
```

If command doesn't exist or errors, NVIDIA driver is not properly installed.

### Step 4: Check NVML Library

```bash
ldconfig -p | grep nvidia-ml
```

If no output, need to install CUDA library packages.

### Step 5: Check Ollama CUDA Libraries

```bash
ls -la /usr/lib/ollama/cuda_v12/
ls -la /usr/lib/ollama/cuda_v13/
```

---

## 4. Solutions

### Issue 1: NVIDIA Driver Not Installed

**Symptom**: `nvidia-smi` doesn't exist, `lsmod` shows `nouveau`

**Solution**:

```bash
# 1. Enable RPM Fusion repository
sudo dnf install \
  https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm \
  https://download1.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm

# 2. Install NVIDIA driver and CUDA libraries
sudo dnf install akmod-nvidia xorg-x11-drv-nvidia-cuda xorg-x11-drv-nvidia-cuda-libs

# 3. Wait for akmods to compile kernel module (~5-10 minutes)
sudo akmods --force

# 4. Reboot
sudo reboot
```

### Issue 2: libnvidia-ml Not in ldconfig Cache

**Solution**:

```bash
sudo ldconfig
ldconfig -p | grep nvidia-ml
```

### Issue 3: Ollama CUDA Libraries Missing

**Solution**:

```bash
# Method 1: Download from GitHub and install
wget -O /tmp/ollama.tgz \
  "https://github.com/ollama/ollama/releases/download/v0.13.5/ollama-linux-amd64.tgz"
sudo tar -C /usr -xzf /tmp/ollama.tgz

# Method 2: Use official install script
curl -fsSL https://ollama.com/install.sh | sh
```

### Issue 4: Ollama Using Wrong Library Path

**Solution**:

```bash
# Create symlink
sudo rm -rf /usr/local/lib/ollama
sudo ln -sf /usr/lib/ollama /usr/local/lib/ollama

# Verify
ls -la /usr/local/lib/ollama/cuda_v12/
```

---

## 5. Verification Steps

### Step 1: Verify Driver Loaded

```bash
nvidia-smi
```

Expected output:
```
+-----------------------------------------------------------------------------------------+
| NVIDIA-SMI 580.119.02             Driver Version: 580.119.02     CUDA Version: 13.0     |
+-----------------------------------------+------------------------+----------------------+
| GPU  Name                 Persistence-M | Bus-Id          Disp.A | Volatile Uncorr. ECC |
|   0  NVIDIA RTX 3500 Ada Gene...    Off |   00000000:01:00.0 Off |                  Off |
+-----------------------------------------+------------------------+----------------------+
```

### Step 2: Verify Ollama GPU Detection

```bash
ollama serve 2>&1 | head -30
```

Expected log (GPU detected):
```
msg="inference compute" id=GPU-xxx library=CUDA compute=8.9 name=CUDA0 
description="NVIDIA RTX 3500 Ada Generation Laptop GPU" 
total="12.0 GiB" available="11.6 GiB"
```

### Step 3: Verify GPU Inference

```bash
# Send test request
curl -s http://localhost:11434/api/generate \
  -d '{"model":"qwen2.5:7b","prompt":"Hello","stream":false}' &

# Check GPU usage
sleep 5
nvidia-smi
```

Expected output (GPU process exists):
```
| Processes:                                                                              |
|    0   N/A  N/A   xxxxx      C   /usr/local/bin/ollama                  4866MiB |
```

### Step 4: Performance Comparison

| Mode | First Load Time | Inference Time (Simple Prompt) |
|------|-----------------|-------------------------------|
| CPU | >180s (timeout) | >60s |
| GPU | ~30s | ~5s |

---

## 6. FAQ

### Q1: GPU Stops Working After Reboot

Check if kernel module is loaded:
```bash
lsmod | grep nvidia
```
If not, kernel module may need recompiling after kernel update:
```bash
sudo akmods --force
sudo reboot
```

### Q2: Ollama Shows "entering low vram mode"

This is normal when GPU VRAM < 20GB. As long as logs show `library=CUDA`, GPU is being used.

### Q3: How to Check Which CUDA Version Ollama Uses?

Check the `libdirs` field in logs:
```
libdirs=ollama,cuda_v13  # Using CUDA 13
libdirs=ollama,cuda_v12  # Using CUDA 12
```

### Q4: What Do [GIN] Logs Mean?

Ollama uses Go's Gin framework for HTTP server. `[GIN]` logs are HTTP request access logs:
```
[GIN] 2025/12/29 - 17:36:45 | 200 | 40.227045034s | 127.0.0.1 | POST "/api/generate"
```
- `200`: HTTP status code
- `40.227045034s`: Request processing time
- `POST "/api/generate"`: API endpoint

---

## 7. Quick Diagnostic Script

```bash
#!/bin/bash
echo "=== GPU Hardware ==="
lspci | grep -i nvidia

echo ""
echo "=== Driver Module ==="
lsmod | grep -E "nvidia|nouveau"

echo ""
echo "=== NVIDIA Driver ==="
nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv 2>/dev/null || echo "nvidia-smi unavailable"

echo ""
echo "=== NVML Library ==="
ldconfig -p | grep nvidia-ml

echo ""
echo "=== Ollama Libraries ==="
ls -la /usr/lib/ollama/cuda_v* 2>/dev/null || ls -la /usr/local/lib/ollama/cuda_v* 2>/dev/null

echo ""
echo "=== Ollama Status ==="
pgrep -x ollama && echo "Ollama is running" || echo "Ollama is not running"
```

---

## 8. References

- [Ollama Documentation](https://ollama.com/docs)
- [RPM Fusion NVIDIA Driver Guide](https://rpmfusion.org/Howto/NVIDIA)
- [NVIDIA CUDA Toolkit Documentation](https://developer.nvidia.com/cuda-toolkit)


