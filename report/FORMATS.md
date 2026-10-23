# GT7 PS5 资源格式手册（实测）

本文件记录当前已验证的结构与偏移。所有数值均为小端。

## 1. 内容库（contents）

```
contents/<d0>/<d1>/<d5 d4 d3 d2>   = cache_key(FNV-1a(asset_path))
```

- FNV-1a：offset `0x811C9DC5`，prime `16777619`，对路径原始字节。
- base36 字符表 `P6MH85C24L91ZTIYURQ7NKJ0OSBWDAVGX3FE`；
  `d0/d1` 是低位数字（目录），`d6..d2` 是高位数字（文件名），
  `d6` 在 `x >= 0x81BF0FFF` 时取 `6`，否则 `P`。
- 载荷本身已解密；部分为 PackedFile 容器。

## 2. PackedFile

| 偏移 | 类型 | 含义 |
|---|---|---|
| 0x00 | u32 | magic |
| 0x04 | i32 | 负的**解压后**大小（PS5） |
| 0x08 | — | 裸 DEFLATE 流 |

magic：`0xFFF7EEC5`(deflate)、`0xFFF7ED85`(zstd tiny)、
`0xFFF7972F`(zstd regular, 数据在 0x20)、`0xFFF7F32E`(zstd chunk, 0x10000 对齐)。

## 3. RT2 本地化文本

```
0x00  "50TR"（= "RT05" 反写）  0x04 u32 section 数
0x20  每个 section 0x18 字节：{u64 name_off, u64 count, u64 entries_off}
        name = NUL 结尾 ASCII
        entry(0x18) = {u32 id, u16 tag, u16 size, u64 blob_off, u64 payload_off}
        blob = 16 字节头 + payload；payload = 明文 XOR 全局密钥流
        长度 n 的 payload 在 n-1 处以 NUL 结尾
```

密钥流由逐偏移频率分析恢复，**全游戏通用**（us 恢复一次即可解 19 语言）。

## 4. TXS5（贴图集，17 GB / 4,296 个）

| 偏移 | 含义（实测） |
|---|---|
| 0x00 | `"TXS5"` |
| 0x04 | u32 文件大小 |
| 0x18 | 常量 2 |
| 0x20 | u32 头表偏移（常见 0x70） |
| 0x28 | **u32 描述符表偏移**（0xB8 / 0x100 / 0x1D8 / 0x460 …） |
| 0x30 | u32 数据起始（0x1000） |

描述符步长 **0x60**，前 0x20 字节有效：

```
0x00 u64 data_offset
0x08 u32 data_size
0x0C u16 format / tile mode   (0x700, 0xB00, 0x1000, 0x601, 0x1800 …)
0x0E u16 mip / flags
0x10 u16 width
0x12 u16 height
0x14 u32 array count
```

`gt7re txs5-index` 已索引全部 4,296 个文件、7,902 个描述符：

- 0xB00 + 4.0 B/px：2,900 个（RGBA8 或 R16G16）
- 0x700 + 1.0 B/px：1,211 个（BC3/BC7 量级）
- 0x1000 + 4.0 B/px：929 个
- 0x700 + 1.3/1.5/2.0 B/px：带 mip 的变体
- 0x601 + 24 B/px：立方体贴图（6 面 × 4 B/px）

像素数据是**分块 swizzle** 的（按 RGBA8 线性解读会出现条带），
尚未确定 tile 模式，因此暂未导出 DDS/PNG。

补充实测（256x256 / 4 B/px / fmt 0xB00）：按 R16G16 解读时，
通道 0 有 88.3% 的 16 位值落在 0x8000±0x2000，均值 0.542，
通道 1 均值 0.566——这是典型的**双通道法线贴图**分布；
因此 4 B/px 条目应按 R16G16 处理，剩余问题只是 tile swizzle。

1 B/px 条目（fmt 0x700）实测：用 BC7 解码（`texture2ddecoder`）得到接近白色的
贴图，但存在 **16 块行（64 像素）周期的横向条带**；BC3/BC1 解码则是洋红色条带。
说明像素块本身有效，但**块行顺序被重排**。已尝试 identity / transpose /
4·8·16·32 块行反转 / 4·8·16·32 块 Morton，均未消除条带
（`std` 恒为 127.27，横竖梯度随转置互换）。下一步需要确定块级 tile 模式。

另一个已确认的细节：**纹理按 64 像素对齐填充**。
例如 UI 图标声明 24×24，但 `data_size = 4096`，恰好等于 64×64×1 B
（BC7 表面大小）；因此 `data_size / (width*height)` 会得到 7.11 这类
“异常”bpp。计算真实格式时应使用填充后的尺寸（24→64、48→64、80→128）。

## 5. MDL5（模型，70 GB / 8,149 个）

| 偏移 | 含义 |
|---|---|
| 0x00 | `"MDL5"` |
| 0x04 | u32 版本（当前 37） |
| 0x08 | u32 段大小/偏移（如 0xEC000） |
| 0x0C | u32 另一段偏移（接近文件尾） |
| 0x20 | u32 4 |
| 0x2C..0x60 | 一组计数/偏移（0xF, 0xC6, 0x3A, 0xDB, 0xE, 0x4C, 0x11, 5, 1…） |

标识符区：文件前 2 MB 内含大量 ASCII 名（节点/动画通道/着色器参数），
例如 `beltSwitch`、`steering_rot`、`wiperFront`、`brakeLight`、
`const_baseColorMap0`、`paintBaseColor`、`flake_size0`。
`gt7re mdl5-index` 已导出 8,149 个模型的 448 万条标识符。

## 6. CAR5（车辆参数，333 个已命名）

```
0x00 "CAR5"  0x04 u32 0  0x08 u32 文件大小  0x0C u32 4
0x10 u32 0x60  0x20 u32 0  0x28 u32 0x228  0x30 u32 0x428
```

后续是分段的参数表（偏移/计数对）。尚未映射到具体字段。

## 7. RADA（course_line，赛道走线）

```
0x00 "RADA"   0x04 u32 文件大小
0x10 起：索引表
0x80 起：u64 分块偏移表

每个分块（0x30 字节头）：
  0x00 u32 count
  0x04 u16 flags        （0/1/256/257/512，疑似线条类型）
  0x06 u16 0x2A06       （常量）
  0x08 f32 step         （观测值 0.1）
  0x0C f32 x,y,z        （分块向量 A：起点切向/法线）
  0x18 f32 x,y,z        （分块向量 B：起点坐标）
  0x24 u32 0
  0x28 u64 向量区偏移   （count × 6 字节：3 × i16，模长均值 1.0）
  0x30 u64 弧长区偏移+4 （count × f32 累计弧长）
```

实测：`PK4HF` 8 个分块、4,719 个点；分块弧长 5.8 km。
全库 85 个 `course_line` 共 392 个分块，弧长 0.8 km – 25.7 km
（最长 `PY1Z0` 25.7 km）。

逐点向量按「单位向量 × 弧长增量」可重建折线（尺度自洽：累计长度 = 弧长），
但闭合误差中位数 11%，且包围盒明显小于赛道实际尺寸——说明这些向量
更可能是**相对中心线的偏移/法线**，而不是世界坐标切向；中心线很可能在
`course_map`(MAP4) 中（MAP4 未发现 float 折线数组，可能为量化/分块存储）。
`gt7re rada --polyline` 已导出 393 个 CSV（392 个分块 + summary），
可作为近似视图，但不应视为精确赛道几何。

## 7b. MAP4（course_map，赛道中心线）— 已完全解析

```
0x00 "MAP4"   0x04 u32 版本(4)   0x0C u32 文件大小
0x10 u32 点表偏移(0x80)          0x14 u32 点表结束
0x20 f32 包围盒 min xyz          0x2C f32 max
0x38 f32 单元尺寸(2.5 米)
0x40 u32 次级表偏移              0x44 u32 边表偏移
0x80 u32 点数
0x84 count × 0x18 记录：{f32 x, f32 y, f32 z, u32 a, u32 b, f32 c}
边表：u32 边数，随后 count × (u16, u16) 索引对
```

实测（85 个文件全部验证）：

- 点间距恒为 **2.5 m**，首尾闭合误差 **0.00 m**；
- 长度与同赛道 RADA 弧长吻合（c052_l01：5807.3 m vs 5800.1 m，
  c180_l03：25406.8 m vs 25350.0 m，误差 < 0.3%）；
- 最长赛道 `crs_c180_l03` 25.4 km，`crs_c193_l01` 30.4 km；
- 渲染出的布局是真实赛道形状（发夹弯、连续弯、闭合环线），
  预览图在 `_re/extracted/tracks/preview/`。

因此 **MAP4 = 赛道中心线**；RADA 的逐点向量不是世界坐标几何，
更可能是相对中心线的辅助数据（偏移/法线/曲率），尚待与中心线联合求解。

同目录还有 `course_map`(MAP4)、`enemy_spline`(ADSP)、`replay`(GCM7)、`pack`(PACL)。

## 8. SNDZ（音频，860 个）

```
0x00 "SNDZ"  0x04 u32 0x2A  0x0C u32 0x00010001
0x10 u32 0xE460（示例）  0x18/0x1C 相同的大数（样本数？）
0x80 起：0x0C 开头的条目表
```

载荷非 RIFF/OggS/AT9 标准容器。实测为**带标签的分块容器**，
0x80 起依次出现：

```
HTBS  WAVS  RNDS  BRNS  BLNS  CHNS  TSKS  SMXS  REQS
WAVD(0x1E0)  REQD(0x22C)  WAVS(0x248)
```

每块头 = `4 字节 tag + u32 长度`（如 `WAVD 40`、`REQD 18`）。
`WAVD` 内含 300 / 184 / 289 / 38481 / **48000** / 20279 / 38480 / 9728 / 1024
等参数，`48000` 即采样率；`REQD` 段是包络/曲线数据。
因此 SNDZ 是**合成器/音序定义**（波形参数 + 包络 + 任务），
不是原始 PCM/AT9 音频；要还原声音需要实现对应合成器。

## 9. 其它已识别容器

| magic | 观察 |
|---|---|
| `SCPS` | 含 `sys`、`diffuse_en` 等明文键，疑似着色器/材质参数集 |
| `IDP0` | 0x10/0x40/0x50/0x20/0x44 等偏移表 |
| `STRB` | 字符串库（`common/crs_info.strb` 5 MB） |
| `MPRJ` | UI 工程（`.mproject`） |
| `GTEB`/`GTED`/`EBAF`/`EVSF`/`SDEF`/`SCNB`/`MAP4`/`ADSP`/`GCM7` | 尚未解析 |

### GPB / MPRJ（UI 工程）

`.gpb` 以 `4bpg` 开头，内含 UTF-8 资源路径字符串，例如
`/image/4k/cursor/arrow.png`、`/image/4k/icon/ps_icon/ps_square.svg`、
`/image/cursor/cross.dds`，以及内嵌的 `IDP0` 数据块与 SVG 文本。
这些是运行时虚拟路径，直接按 FNV-1a 命名不命中存储库，需通过资源表解析。

`.gpb` 内嵌的 `IDP0` 块可以直接按标准 0x70 头解析，载荷全部是 TXS5。
`gt7re gpb-extract` 已从 859 个 `4bpg` 文件中抽出 **6,058 个 UI 贴图**
（1.42 GB，单文件平均 0.24 MB），全部为 `fmt=0x700`，
尺寸以 80×80、48×48、24×24、600×240、1200×480 等 UI 资源为主。

`gt7re strings --kind "UI project (MPRJ)"` 已导出 92 个 UI 工程的
189,277 条字符串（其中 129 条为干净资源路径）。

## 8b. IDP0（通用包装，14,983 个 / 5 GB）

```
0x00 "IDP0"   0x04 u32 版本(2)
0x08..0x30 u64 偏移表
0x30 u64 载荷偏移（0x70）
0x38 u64 载荷大小
0x40 4 字节 tag（"ecp"）
0x44 4 字节 tag（"txs"）
0x70 起：载荷（实测全部为 TXS5 贴图集）
```

`gt7re idp0-extract` 已解出 14,983 个载荷（4.99 GB），全部可被
`txs5-index --dir` 识别。至此贴图集总数为
**19,279 个文件 / 22,885 个描述符 / 约 41 亿 texel**。

社区工具链（已验证文档）：

- `oxmaulmike2581/IMG2SXT3` 的做法与本实现一致：去掉 0x70 头，输出 `.txs`；
- 其 README 指向 id-daemon 的 **GTS_mdl** 工具：把 `.txs` 拖上去即可生成 DDS(DX10)。
  这是目前绕过 TXS5 swizzle 的最快路径。

因此另提供 `.txs` 扩展名输出，便于直接喂给 GTS_mdl：

```powershell
& gt7re.exe idp0-extract --csv census.csv --out extracted\idp0_txs --ext .txs
```

已生成 `_re/extracted/idp0_txs/`（14,983 个 `.txs`）。

## 10. UDP 遥测

包大小 A=296、B=316、T=344、C=368 字节；`gt7re udp layout` 输出完整字段表，
`gt7re udp parse <bin>` 输出 JSON。
