# gt7re — Gran Turismo 7 (PS5) 并行解析器

Rust 实现，用来完成此前 Python 阶段的全部工作：内容库索引、资源命名还原、
类型普查、批量解包/提取、`.rt2` 本地化文本解密，以及 GT7 UDP 遥测包解析。

## 快速开始

```powershell
cd C:\gt7\gt7\_re\rust\gt7re
cargo build --release
$exe = ".\target\release\gt7re.exe"

# 资源路径 -> 磁盘文件
& $exe resolve "common/render.txs" "rtext/common/us.rt2"

# 全库类型普查
& $exe census --out C:\gt7\gt7\_re\reports\census.csv --report C:\gt7\gt7\_re\reports\summary.txt

# 按类型拆清单 / 按类型提取（copy 解包 PackedFile，link 建硬链接镜像）
& $exe catalog --csv C:\gt7\gt7\_re\reports\census.csv --out C:\gt7\gt7\_re\catalog
& $exe extract-kind --csv C:\gt7\gt7\_re\reports\census.csv --kind "sound (SNDZ)" `
    --out C:\gt7\gt7\_re\extracted\audio --mode copy

# 把全库 95,339 个文件按类型硬链接成可浏览目录（零额外空间）
& $exe link-all --csv C:\gt7\gt7\_re\reports\census.csv --out C:\gt7\gt7\_re\extracted\by_kind

# MDL5 模型元数据索引（头部字段 + 骨骼/材质/着色器参数名）
& $exe mdl5-index --csv C:\gt7\gt7\_re\reports\census.csv `
    --out C:\gt7\gt7\_re\reports\mdl5_index.json --head-mb 2 --max-strings 800

# RADA (course_line) 赛道走线分块导出
& $exe rada --named C:\gt7\gt7\_re\reports\named.txt `
    --out C:\gt7\gt7\_re\extracted\tracks\rada --polyline

# MAP4 (course_map) 赛道中心线导出（点集 + 边表 + summary）
& $exe map4 --named C:\gt7\gt7\_re\reports\named.txt `
    --out C:\gt7\gt7\_re\extracted\tracks\map4

# TXS5 贴图集描述符索引
& $exe txs5-index --csv C:\gt7\gt7\_re\reports\census.csv `
    --out C:\gt7\gt7\_re\reports\txs5_index.json

# 解包 IDP0 容器（内嵌 TXS5），再索引
& $exe idp0-extract --csv C:\gt7\gt7\_re\reports\census.csv `
    --out C:\gt7\gt7\_re\extracted\idp0
& $exe txs5-index --dir C:\gt7\gt7\_re\extracted\idp0 `
    --out C:\gt7\gt7\_re\reports\txs5_index_idp0.json

# 任意类型文件的字符串索引（例：92 个 UI 工程 -> 18.9 万条字符串）
& $exe strings --kind "UI project (MPRJ)" `
    --out C:\gt7\gt7\_re\reports\mprj_strings.json --max 3000

# 从 UI 工程 (.gpb) 内嵌的 IDP0 抽 UI 贴图（.txs）
& $exe gpb-extract --kind "unknown:34627067" `
    --out C:\gt7\gt7\_re\extracted\ui_txs

# 已命名资源镜像成真实目录树（不占额外空间）
& $exe extract --list C:\gt7\gt7\_re\reports\named.txt --out C:\gt7\gt7\_re\tree --mode link

# 本地化文本：先从一个拉丁语系文件恢复密钥流，再复用到全部语言
& $exe rt2 decode C:\gt7\gt7\_re\extracted\text\rtext\common\us.rt2 `
    --out C:\gt7\gt7\_re\extracted\text_decoded --keydir C:\gt7\gt7\_re\keys
$files = Get-ChildItem C:\gt7\gt7\_re\extracted\text\rtext\common\*.rt2 | % FullName
& $exe rt2 decode @files --out C:\gt7\gt7\_re\extracted\text_decoded `
    --keystream C:\gt7\gt7\_re\keys\us.ks.bin

# UDP 遥测
& $exe udp layout
& $exe udp parse capture.bin
```

## 原理

### 1. 内容库命名

PS5 版把每个资源存成

```
contents/<cache_key(FNV-1a(asset_path))>
```

- `FNV-1a`：32 位，offset `0x811C9DC5`，prime `16777619`
- `cache_key`：7 位 base36，字符表 `P6MH85C24L91ZTIYURQ7NKJ0OSBWDAVGX3FE`；
  低两位做目录，高五位做文件名

因此任意候选路径都能 O(1) 验真，不需要解密 `gt.idx`。

### 2. PackedFile 容器

| magic (LE) | 格式 |
|---|---|
| `0xFFF7EEC5` | zlib |
| `0xFFF7ED85` | zstd（tiny） |
| `0xFFF7972F` | zstd（regular） |
| `0xFFF7F32E` | zstd（0x10000 对齐分块） |

PS5 的 `0xFFF7EEC5` 实际是**裸 DEFLATE**（没有 zlib 头），并且偏移 0x04 的
字段是**解压后大小**；实现里先试 zlib、失败再退回 raw DEFLATE。
该类型共 9,097 个文件，全部是 DDS 贴图，解压后 42.4 GB。

### 3. `.rt2` 文本

每个 payload 是 `plaintext XOR keystream`，**整个游戏共用一条密钥流**
（同一字符串在 us/gb 中密文完全相同即可证明）。
按字节偏移做频率分析即可恢复；字符串末尾的 NUL 单独计分，首字节用
「首字符模型」（大写字母 / `<` / 数字）估计。

19 种语言的坏行率：拉丁语系约 0.8%–1.1%，中日韩 0.8%–1.8%，
俄文 3.2%，泰文 11%（组合字符较多）。长文本（品牌/车辆简介、剧情对白、
UI 提示）基本完全正确。

### 4. UDP 遥测

包大小 A=296 / B=316 / T=344 / C=368 字节；`udp parse` 输出 JSON，
`udp layout` 输出完整字段偏移表。

## 性能（本机）

| 操作 | 规模 | 耗时 |
|---|---|---|
| 扫描内容库 | 95,339 文件 / 282 GB | 3.8 s |
| 全库类型普查 | 95,339 文件 | 6.2 s |
| 已命名资源硬链接镜像 | 12,354 文件 | 3.3 s |
| RT2 单语言解码 | 42,638 字符串 | 0.18 s |
| RT2 19 语言（共享密钥流） | 810,122 字符串 | 1.9 s |
| 全库硬链接归档 | 95,339 文件 | 14.8 s |
| 解包 DDS 贴图 | 9,097 文件 / 42.4 GB | 约 2 min |
| MDL5 元数据索引 | 8,149 模型 / 4.48M 标识符 | 8.3 s |

并行基于 `rayon`；哈希/命名/普查/提取均为数据并行。

## 模块

```
src/store.rs     FNV-1a、cache_key、内容库扫描与解析
src/packed.rs    PackedFile 解包
src/census.rs    类型识别、普查 CSV、分类清单
src/discover.rs  候选路径生成、并行验真、字符串挖掘
src/extract.rs   并行复制/硬链接提取
src/rt2.rs       rt2 解析、密钥流恢复、JSON/CSV 导出
src/udp.rs       GT7 UDP 遥测解析
src/mdl5.rs      MDL5 头部与标识符（骨骼/材质/着色器参数名）索引
src/rada.rs      RADA (course_line) 分块、逐点向量与弧长解析
src/map4.rs      MAP4 (course_map) 赛道中心线点集与边表解析
src/txs5.rs      TXS5 贴图集描述符（格式/宽高/偏移/数组数）索引
src/idp0.rs      IDP0 通用包装（0x70 头 + 内嵌 TXS5）
src/gpb.rs       GPB 内嵌 IDP0/TXS5 提取
src/main.rs      CLI（strings 子命令为通用字符串索引）
```

## 测试

```powershell
cargo test --release
```

包含 2 个单元测试（共 4 组已知路径的 FNV-1a 与 cache-key 向量）。

## 已知限制

- `eboot.bin` 是加密的 PS5 SELF，代码逻辑与在线请求实现需要主机密钥。
- `MDL5` 已能索引元数据（节点名、着色器参数名）；完整几何/材质解包尚未实现。
- `TXS5` / `.gpb` / `.adc` 目前只做识别与提取，尚未转成通用格式。
- `packed:zstd-regular` 等变体沿用公开结构实现，PS5 上未发现可用于验证的样本。
- `.rt2` 短字符串尾部的填充字节偶尔会被解成多余字符（例如 `Dirt/`）。
- RADA 折线重建为近似结果（闭合误差中位数 11%），逐点向量疑似相对中心线的
  偏移/法线；精确赛道几何需要解析 `course_map`(MAP4)。
