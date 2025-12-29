# 快速开始指南

本指南帮助你在 5 分钟内开始使用 Media Organizer。

## 前提条件

- Linux 系统 (Fedora/Ubuntu/Debian)
- Rust 开发环境
- 网络代理（如果在中国大陆，TMDB 需要代理访问）

## 第一步：安装依赖 (2分钟)

```bash
# 1. 安装 ffmpeg（包含 ffprobe）
# Fedora:
sudo dnf install ffmpeg

# Ubuntu/Debian:
sudo apt install ffmpeg

# 2. 安装 Ollama
curl -fsSL https://ollama.com/install.sh | sh

# 3. 启动 Ollama 并下载模型
ollama serve &
ollama pull qwen2.5:7b
```

## 第二步：获取 TMDB API Key (1分钟)

1. 访问 https://www.themoviedb.org/signup 注册账户
2. 登录后访问 https://www.themoviedb.org/settings/api
3. 点击 "Create" 创建 API Key
4. 复制 "API Key (v3 auth)"

## 第三步：编译项目 (1分钟)

```bash
cd media_organizer
cargo build --release
```

## 第四步：整理你的电影 (1分钟)

```bash
# 设置环境变量
export TMDB_API_KEY="你的API密钥"

# 生成整理计划
./target/release/media-organizer plan movies ~/Videos/Movies --target ~/Videos/Movies_Organized -v

# 查看生成的计划
cat plan_*.json

# 执行计划（移动文件、下载海报、生成 NFO）
./target/release/media-organizer execute plan_*.json
```

## 完整示例

```bash
# 假设你的电影在 ~/Downloads/Movies
# 你想整理到 ~/Videos/Movies

# 1. 设置环境变量
export TMDB_API_KEY="b04ce72868d0071b09650ab99df1d3d0"
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="qwen2.5:7b"

# 2. 确保 Ollama 运行中
pgrep ollama || ollama serve &

# 3. 生成计划
./target/release/media-organizer plan movies \
  ~/Downloads/Movies \
  --target ~/Videos/Movies \
  --verbose

# 4. 查看计划内容
less plan_*.json

# 5. 执行计划
./target/release/media-organizer execute plan_*.json

# 6. 查看结果
ls -la ~/Videos/Movies/
```

## 命令速查表

| 操作 | 命令 |
|------|------|
| 整理电影 | `media-organizer plan movies <源目录> -t <目标目录>` |
| 整理电视剧 | `media-organizer plan tvshows <源目录> -t <目标目录>` |
| 执行计划 | `media-organizer execute <plan.json>` |
| 模拟执行 | `media-organizer execute <plan.json> --dry-run` |
| 回滚操作 | `media-organizer rollback <session_id>` |
| 查看会话 | `media-organizer sessions` |
| 验证配置 | `media-organizer verify` |
| 帮助信息 | `media-organizer --help` |

## 输出示例

执行后，你的电影将被整理为：

```
Movies/
├── [刺杀小说家2](2025)-tt33095008-tmdb945801/
│   ├── [刺杀小说家2](2025)-2160p-BluRay-hevc-8bit-dts-5.1.mp4
│   ├── movie.nfo
│   └── poster.jpg
├── [幸运钥匙](2016)-tt5719388-tmdb416620/
│   ├── [幸运钥匙](2016)-1080p-WEB-DL-h264-aac-stereo.mp4
│   ├── movie.nfo
│   └── poster.jpg
└── ...
```

## 常见问题

### Q: AI 解析很慢？
A: 检查是否启用了 GPU。运行 `nvidia-smi` 确认 GPU 可用，查看 [GPU 配置指南](docs/04-ollama-gpu-setup.md)。

### Q: TMDB 连接超时？
A: 在中国大陆需要代理才能访问 TMDB。确保你的代理已开启。

### Q: 找不到电影信息？
A: 可能是文件名太复杂或电影在 TMDB 中不存在。检查 `plan.json` 中的 `unknown` 部分。

### Q: 如何回滚？
A: 运行 `media-organizer sessions` 查看会话 ID，然后 `media-organizer rollback <session_id>`。

## 下一步

- 查看 [README.md](README.md) 了解完整功能
- 查看 [GPU 配置指南](docs/04-ollama-gpu-setup.md) 加速 AI 推理
- 查看 [设计文档](docs/01-design-preparation.md) 了解架构设计

