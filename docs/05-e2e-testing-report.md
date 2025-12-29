# E2E 测试报告

**测试日期**: 2025-12-30  
**测试目录**: `/home/johnny/Videos/Media_test/Movies`  
**目标目录**: `/home/johnny/Videos/Media_test/Movies_organized`

---

## 测试概要

| 指标 | 数值 |
|------|------|
| 总文件数 | 12 |
| 成功处理 | 9 (75%) |
| 未处理 | 3 (25%) |
| 匹配有误 | 2 |

---

## 成功处理的电影

| 原文件名 | 整理后文件夹 | 状态 |
|----------|------------|------|
| 买下我.Kupi.menya.2018.HD1080P.mp4 | `[Купи меня][买下我](2017)-tt6964554-tmdb460036` | ✅ |
| 雏菊 导演剪辑版 2006.mp4 | `[데이지][雏菊](2006)-tt0468704-tmdb18426` | ✅ |
| 幸运钥匙.mp4 | `[럭키][幸运钥匙](2016)-tt6175078-tmdb421928` | ✅ |
| J_禁忌童话.2025.1080P.mp4 | `[동화지만 청불입니다][禁忌童话](2025)-tt35303960-tmdb1174128` | ✅ |
| 致命呼叫 2013.rmvb | `[The Call][致命呼叫](2013)-tt1911644-tmdb158011` | ✅ |
| 龙门相.国语中字.2020.1080P.mp4 | `[龙门相](2020)-tt12619080-tmdb880031` | ✅ |
| 破坏不在场证明 特别篇/2024 SP.mp4 | `[アリバイ崩し承りますスペシャル][破坏不在场证明 特别篇](2024)-tt32090414-tmdb1280437` | ✅ |
| 动物农场.mp4 | `[A Day in the Life of a Farmed Animal](2023)-tmdb1183391` | ⚠️ 匹配有误 |
| 破坏不在场证明/01.mp4 | `[Alibi][不在场证明](1929)-tt0019630-tmdb13847` | ⚠️ 匹配有误 |

---

## 未处理的文件

| 文件名 | 路径 | 失败原因 |
|--------|------|----------|
| 杀人者报告 살인자 리포트.2025.1080p.韩语中字.mp4 | 根目录 | AI解析失败或TMDB匹配失败 |
| 特别篇前篇.mp4 | 破坏不在场证明 特别篇 钟表店侦探与祖父的不在场证明/ | 文件名信息不足 |
| 特别篇后篇.mp4 | 破坏不在场证明 特别篇 钟表店侦探与祖父的不在场证明/ | 文件名信息不足 |

---

## 已修复的问题

### 1. NFO 信息不完整
**问题**: 生成的 movie.nfo 文件信息过于简单，缺少演员、导演、剧情简介等

**解决方案**: 
- 扩展 `MovieMetadata` 结构，增加 `genres`, `countries`, `studios`, `writers`, `actor_roles`, `rating`, `votes`, `tagline`, `runtime`, `certification` 等字段
- 修改 TMDB API 调用，使用 `append_to_response=credits,release_dates` 获取完整信息
- 改进 NFO 生成器，输出完整的 Kodi/Emby/Jellyfin 兼容格式

### 2. 续集编号被误识别
**问题**: "刺杀小说家2.4k.mp4" 被解析为 "刺杀小说家"，丢失了续集编号 "2"

**解决方案**: 
- 改进 AI prompt，明确说明续集编号是标题的一部分
- 添加示例："刺杀小说家2.4k.mp4 中，2 是续集编号，4k 才是分辨率"

### 3. 版本信息被包含在标题中
**问题**: "雏菊 导演剪辑版 2006.mp4" 被解析为 "雏菊 导演剪辑版"，导致 TMDB 搜索失败

**解决方案**: 
- 改进 AI prompt，说明版本信息（导演剪辑版、加长版等）不是标题的一部分
- 添加示例："雏菊 导演剪辑版 2006.mp4 的标题是 雏菊，不是 雏菊 导演剪辑版"

### 4. 文件名信息不足时无法解析
**问题**: "2024 SP.mp4" 这样的文件名缺少标题信息

**解决方案**: 
- 添加 `build_parse_input()` 函数，检测文件名是否为 "minimal filename"
- 当文件在有意义的子目录中时，将目录名与文件名组合后传给 AI 解析
- 例如："破坏不在场证明 特别篇/2024 SP.mp4" 会被解析为 "破坏不在场证明 特别篇 - 2024 SP.mp4"

### 5. Plan/Rollback 文件保存位置不合理
**问题**: `plan.json` 和 `rollback.json` 保存在源目录，污染了原始文件夹

**解决方案**: 
- 修改 `default_plan_path()` 函数，优先使用目标目录
- 现在这些文件保存在目标目录中

### 6. Confidence 值归一化
**问题**: AI 有时返回 `confidence: 75` 而不是 `0.75`，导致验证失败

**解决方案**: 
- 添加归一化逻辑：如果 confidence > 1.0，则除以 100

### 7. 从文件名解析分辨率
**问题**: 文件名中的 4K、1080p 等分辨率信息未被识别

**解决方案**: 
- 添加 `parse_metadata_from_filename()` 函数，从文件名提取分辨率、格式、编码等信息
- 添加 `merge_metadata()` 函数，合并 ffprobe 和文件名解析的结果

---

## 待解决的问题

### 1. 韩语混合文件名解析失败
**文件**: `杀人者报告 살인자 리포트.2025.1080p.韩语中字.mp4`

**分析**: 
- 文件名包含中韩双语
- TMDB 上存在此电影（id: 1151766）
- 可能是 AI 解析韩语部分失败

**可能的解决方案**:
- 改进 AI prompt，增加韩语解析能力说明
- 或者预处理文件名，提取中文部分

### 2. 长目录名下的短文件名
**文件**: `破坏不在场证明 特别篇 钟表店侦探与祖父的不在场证明/特别篇前篇.mp4`

**分析**: 
- 文件名 "特别篇前篇.mp4" 信息不足
- 目录名包含完整信息，但当前逻辑未能正确使用

**可能的解决方案**:
- 改进 `is_minimal_filename()` 检测逻辑
- 对于 "特别篇" 这类词汇，应该认为是 minimal filename

### 3. TMDB 匹配错误
**问题 1**: "动物农场.mp4" 匹配到了错误的电影

**分析**: 
- 文件名只有 "动物农场"，没有年份
- AI 解析返回英文原标题 "Animal Farm"
- TMDB 搜索返回了错误的版本（2026年的新版本而非经典版）

**可能的解决方案**:
- 当有多个搜索结果时，优先选择评分高或评论数多的版本
- 或者优先选择年份较早的经典版本

**问题 2**: "破坏不在场证明/01.mp4" 匹配到了 1929 年的老电影 "Alibi"

**分析**: 
- 文件名 "01.mp4" 信息极少
- 目录名 "破坏不在场证明" 被传入 AI，但解析结果可能不正确
- TMDB 搜索 "不在场证明" 返回了 1929 年的电影

**可能的解决方案**:
- 对于日剧/韩剧，可能需要特殊处理
- 或者使用更精确的搜索词

---

## 生成的 NFO 示例

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<movie>
  <title>买下我</title>
  <originaltitle>Купи меня</originaltitle>
  <year>2017</year>
  <releasedate>2017-01-01</releasedate>
  <premiered>2017-01-01</premiered>
  <runtime>110</runtime>
  <ratings>
    <rating name="themoviedb" max="10" default="true">
      <value>6.5</value>
      <votes>42</votes>
    </rating>
  </ratings>
  <uniqueid type="tmdb" default="true">460036</uniqueid>
  <uniqueid type="imdb">tt6964554</uniqueid>
  <plot>故事梗概...</plot>
  <genre>剧情</genre>
  <country>Russia</country>
  <studio>制片公司</studio>
  <credits>编剧</credits>
  <director>导演</director>
  <actor>
    <name>演员名</name>
    <role>角色名</role>
    <order>0</order>
  </actor>
  <thumb aspect="poster">https://image.tmdb.org/t/p/w500/xxx.jpg</thumb>
  <fanart>
    <thumb>https://image.tmdb.org/t/p/original/xxx.jpg</thumb>
  </fanart>
</movie>
```

---

## 下一步计划

1. **修复韩语文件名解析**: 改进 AI prompt 或添加预处理
2. **改进 minimal filename 检测**: 识别更多需要使用目录名的场景
3. **优化 TMDB 搜索结果选择**: 当有多个匹配时，使用更智能的选择逻辑
4. **添加手动确认模式**: 对于低置信度的匹配，提示用户确认
5. **支持 TV Shows 测试**: 目前只测试了 Movies，需要测试电视剧

---

## 相关代码修改记录

| 文件 | 修改内容 |
|------|----------|
| `src/models/media.rs` | 扩展 MovieMetadata 结构 |
| `src/services/tmdb.rs` | 添加 append_to_response 支持 |
| `src/generators/nfo.rs` | 增强 NFO 生成 |
| `src/core/parser.rs` | AI prompt 改进、confidence 归一化 |
| `src/core/planner.rs` | 添加 build_parse_input、merge_metadata |
| `src/services/ffprobe.rs` | 添加文件名元数据解析 |
| `src/cli/commands/plan.rs` | 修改默认输出路径 |

