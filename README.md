# Media Organizer

一个智能的媒体文件整理工具，使用 AI 解析文件名并从 TMDB 获取元数据，自动重命名和整理电影/电视剧文件。

## ✨ 特性

- 🤖 **AI 驱动的文件名解析** - 使用本地 Ollama + Qwen 2.5 模型智能识别电影/剧集信息
- 🎬 **TMDB 元数据** - 自动获取电影详情、海报、导演、演员等信息
- 📁 **智能重命名** - 按照标准格式重命名文件和文件夹
- 🔄 **安全操作** - 先生成计划，预览后再执行，支持回滚
- 🚀 **GPU 加速** - 支持 NVIDIA GPU 加速 AI 推理
- 📊 **详细日志** - 完整的操作日志和进度显示

## 📋 系统要求

- **操作系统**: Linux (Fedora/Ubuntu/Debian)
- **Rust**: 1.70+
- **Ollama**: 0.13+ (用于 AI 推理)
- **ffprobe**: 用于提取视频技术信息
- **TMDB API Key**: 需要注册 [TMDB](https://www.themoviedb.org/) 获取

### 可选
- **NVIDIA GPU**: 推荐用于加速 AI 推理（需要 CUDA 驱动）

## 🚀 快速开始

### 1. 安装依赖

```bash
# Fedora
sudo dnf install ffmpeg

# Ubuntu/Debian
sudo apt install ffmpeg

# 安装 Ollama
curl -fsSL https://ollama.com/install.sh | sh

# 下载 AI 模型
ollama pull qwen2.5:7b
```

### 2. 配置环境变量

```bash
export TMDB_API_KEY="your_tmdb_api_key"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"
```

### 3. 编译运行

```bash
cd media_organizer
cargo build --release

# 查看帮助
./target/release/media-organizer --help
```

### 4. 整理电影

```bash
# 步骤 1: 生成整理计划
./target/release/media-organizer plan movies /path/to/movies --target /path/to/organized

# 步骤 2: 查看计划
cat plan_*.json

# 步骤 3: 执行计划
./target/release/media-organizer execute plan_*.json

# 如需回滚
./target/release/media-organizer rollback <session_id>
```

## 📖 命令说明

### plan - 生成整理计划

```bash
media-organizer plan movies <SOURCE> [OPTIONS]
media-organizer plan tvshows <SOURCE> [OPTIONS]

Options:
  -t, --target <TARGET>  目标目录
  -v, --verbose          详细输出
  -o, --output <OUTPUT>  计划文件输出路径
      --skip-preflight   跳过预检查
```

### execute - 执行计划

```bash
media-organizer execute <PLAN_FILE> [OPTIONS]

Options:
  --dry-run    仅模拟执行，不实际操作
  --force      跳过确认提示
```

### rollback - 回滚操作

```bash
media-organizer rollback <SESSION_ID>
```

### sessions - 查看会话

```bash
media-organizer sessions          # 列出所有会话
media-organizer sessions <ID>     # 查看会话详情
```

### verify - 验证配置

```bash
media-organizer verify            # 检查所有依赖和配置
```

## 📁 输出格式

### 电影文件夹结构

```
Movies_organized/
└── [电影名称](年份)-ttIMDB_ID-tmdbTMDB_ID/
    ├── [电影名称](年份)-分辨率-格式-编码-位深-音频-声道.mp4
    ├── movie.nfo
    └── poster.jpg
```

### 示例

```
[刺杀小说家2](2025)-tt33095008-tmdb945801/
├── [刺杀小说家2](2025)-2160p-BluRay-hevc-8bit-dts-5.1.mp4
├── movie.nfo
└── poster.jpg
```

## ⚙️ 配置

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `TMDB_API_KEY` | TMDB API 密钥 | (必需) |
| `OLLAMA_BASE_URL` | Ollama 服务地址 | `http://localhost:11434` |
| `OLLAMA_MODEL` | AI 模型名称 | `qwen2.5:7b` |
| `RUST_LOG` | 日志级别 | `info` |

### TMDB API Key

1. 注册 [TMDB 账户](https://www.themoviedb.org/signup)
2. 进入 [API 设置](https://www.themoviedb.org/settings/api)
3. 申请 API Key (v3 auth)
4. 设置环境变量: `export TMDB_API_KEY="your_key"`

## 🔧 GPU 配置

如果你有 NVIDIA GPU，可以启用 GPU 加速以提高 AI 推理速度：

详见 [Ollama GPU 配置指南](docs/04-ollama-gpu-setup.md)

### 快速检查

```bash
# 检查 GPU
nvidia-smi

# 检查 Ollama GPU 状态
ollama serve 2>&1 | grep -i "inference compute"
# 应显示: library=CUDA
```

## 📊 性能

| 模式 | AI 解析时间 (每文件) |
|------|---------------------|
| CPU | 30-60 秒 |
| GPU (RTX 3500) | 1-2 秒 |

## 🐛 故障排除

### AI 解析超时
- 检查 Ollama 是否运行: `pgrep ollama`
- 检查 GPU 是否启用: 查看 Ollama 日志中是否有 `library=CUDA`

### TMDB API 错误
- 检查 API Key 是否正确
- 检查网络连接（可能需要代理）

### 视频信息提取失败
- 确保 ffprobe 已安装: `which ffprobe`

## 📄 文档

- [设计文档](docs/01-design-preparation.md)
- [架构设计](docs/02-architecture-design.md)
- [实现计划](docs/03-implementation-plan.md)
- [GPU 配置指南](docs/04-ollama-gpu-setup.md)

## 📜 许可证

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

