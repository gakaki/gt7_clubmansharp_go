# Gran Turismo 7 (PS5) 解包 / 逆向报告

工作目录 C:\gt7\gt7　产出目录 C:\gt7\gt7\_re

---

## 0. 完成度矩阵（对照最初需求）

| 需求 | 状态 | 证据 / 产物 |
|---|---|---|
| 内容库解包 | ✅ | 95,339 文件 / 282 GB，按类型硬链接归档 |
| 文件名还原 | ✅ | FNV-1a + cache key，12,355 条映射 |
| 音频 | ◐ | 860 个 SNDZ 已导出并索引段标签；`.szd/.szd3/.irf` 未解码 |
| 视频 | ✅ | 15 个 MP4/HEVC，5.25 GB |
| 3D 素材 | ◐ | 8,149 个 MDL5、59,498 个子模型记录、纹理/着色器偏移与 621 万标识符已索引；几何未导出 |
| 贴图 | ◐ | 9,097 张 DDS 已解包（42.4 GB）；19,279 个 TXS5 已索引，像素 swizzle 未解 |
| 图片 | ◐ | JPEG/PNG 可直接用；4,074 个 JXL 已解包并建立盒/EXIF/codestream 索引，像素解码仍不支持 |
| UI | ◐ | 92 个 UI 工程/189,277 条字符串；6,058 个内嵌 UI 贴图已抽出；GPB 结构未完整解析 |
| 代码文件 | ⛔ | eboot.bin 为加密 PS5 SELF；`.adc` 为编译脚本 |
| 代码主体逻辑 | ⛔ | 同上，需要主机密钥或 SELF 解密 |
| 网络 / UDP | ◐ | UDP 遥测解析器 + 字段表（A/B/T/C）；在线 HTTP 未解 |
| 赛道数据 | ✅ | 85 条赛道中心线（2.5 m 采样、闭合、长度校验） |
| 赛道走线 race line | ◐ | 392 个 RADA 分块（向量+弧长）已导出；位置重建为近似 |
| UI 文本 / 国际文本 | ✅ | 19 语言 × 42,638 条，坏行率 0.8%–3.2% |
| 对话 | ✅ | rt2 `BuddyQuest` 剧情对白已解出 |
| 车辆数据 | ◐ | 商店 JSON、CAR5 几何数据已提取；SPECDB 未定位 |
| 全库数据普查 | ✅ | `store_census_rust.csv` 95,339 行 |
| Rust 并行解析器 | ✅ | 19 个子命令，`cargo test --release` 通过 |

图例：✅ 完成　◐ 部分完成　⛔ 受外部条件阻塞

### 解除阻塞所需条件

| 阻塞项 | 需要什么 |
|---|---|
| `eboot.bin` 代码 / 主逻辑 / 在线协议 | PS5 主机密钥，或可由越狱机导出的已解密 `eboot.elf` |
| TXS5 像素（19,279 个纹理集） | id-daemon 的 **GTS_mdl** 工具，或一份可用的 PS5/GNF tile swizzle 参考实现 |
| JPEG XL（4,074 张） | 支持当前容器的 libjxl 解码器（`pillow-jxl-plugin` 与 `imagecodecs` 均失败） |
| MDL5 几何 | MDL5 顶点声明 / 索引格式资料，或可解析的参考工具 |
| SNDZ 声音 | Polyphony 合成器规范（文件是波形+包络+任务定义，不是 PCM） |
| GPB 完整结构 | GPB 节点/属性格式资料（内嵌 IDP0 已能提取） |

---

## 1. 目标识别

| 项目 | 值 |
|---|---|
| 游戏 | Gran Turismo 7（跑车浪漫旅 7） |
| 平台 | PlayStation 5 |
| Title ID | PPSA01317（US，UP9000-PPSA01317_00-GRANTURISMO70000） |
| 版本 | contentVersion 01.710.000 |
| SDK / 要求系统 | 0x1300000000000000 / 0x1360000000000000 |
| 发行参数 | sce_sys/param.json（含 31 种语言标题名） |

Dump 内容：

- eboot.bin（110,576,035 字节）= PS5 SELF，magic 0xEEF51454，仍为加密态。
  解密需要主机密钥（未获得），因此主程序机器码本期无法反编译。
- fakelib/*.sprx、sce_module/、sce_sys/、playlgo*.dat = 标准 fPKG 转储结构。
- contents/ = 游戏资源主体：95,339 个文件 / 282.03 GB。
- 根目录杂项：cs（JSON，二手车商店数据）、2468c.com.txt（散播站点水印）、
  watermark_img.png、k9.psp、ampr_emu.index、libSieRdNNabla.prx（GT Sophy 神经网络库）。

---

## 2. 关键突破 1：contents 存储格式与文件名还原算法

contents/ 下是 gt.idx 加两段 base36 目录：

```
contents/gt.idx                    (3,083,792 字节，全加密)
contents/<d0>/<d1>/<d5 d4 d3 d2>   (5 字符文件名)
```

已实测验证的规则：

```
磁盘文件名 = compute_cache_key( FNV-1a(asset_path) )
```

- FNV-1a：32 位，offset 0x811C9DC5，prime 16777619，对 asset path 原始大小写计算。
- compute_cache_key(x)：把 32 位哈希写成 7 位 base36，字符表为
  P6MH85C24L91ZTIYURQ7NKJ0OSBWDAVGX3FE
  - 目录 = 低位数字 d0/d1
  - 文件名 = d6 d5 d4 d3 d2，其中 d6 取 TABLE[0]（字符 P，当 x < 0x81BF0FFF）
    或 TABLE[1]（字符 6，当 x >= 0x81BF0FFF）

验证结果：flatz gt7tool 的 105 条已知路径 105/105 全部命中；
GT Sport 的 66,935 条路径列表命中 9,282 条（13.9%，覆盖 99 GB 体积）。

意义：任意候选路径都能以微秒级验真（算哈希、查表），因此不需要解密 gt.idx
也能反推文件名；反过来，只要路径字典足够全，就能命名全部资源。

gt.idx 用 flatz 提供的 ChaCha20 key/IV 解密失败（PS5 版换了 index key），
但目前不需要它：命名走 FNV-1a 加 cache key 路线。

---

## 3. 关键突破 2：本地化文本 rtext/*.rt2 解密

rtext/common/<lang>.rt2 共 19 种语言（us, gb, jp, zh, cn, tw, kr, ru, de, fr, it, es,
pt, nl, pl, cz, tr, ar, th），每个文件 285 个 section / 42,638 条字符串。

容器结构（已解析）：

```
0x00  magic 50TR（= RT05 反写），count = 285
0x20  285 x 24 字节 section 表 {u64 name_off, u64 entry_count, u64 entries_off}
      之后是 section 名字符串区（明文 ASCII，如 ABARTH_DESCRIPTION）
      每条 entry = 24 字节 {u32 id, u16 tag, u16 payload_size, u64 blob_off, u64 payload_off}
      blob = 16 字节头 + payload（payload 高熵，无已知容器魔数）
```

payload 是固定密钥流 XOR 加密：同一条目在 us 与 gb 之间密文一致
（例如 TireName 54 条中 51 条 us/gb 密文完全相同），说明密钥流与语言无关、
对每个字节偏移固定复用。因此可用逐位置频率分析恢复密钥流：

```
cipher[offset] = plaintext[offset] XOR keystream[offset]
keystream[offset] = argmax_k  SUM_b count[b] * log P_model(b XOR k)
```

第二轮修正：**密钥流是全局的**，不随语言变化。用 us.rt2 恢复一次后直接套用到
jp/zh/ru/th/pl 均得到正确文本，因此只需恢复一次；短标签表（TireName/Unit/
CATEGORY 等）也随之解出。字符串末尾的 NUL 终止符单独计分，首字节用
「首字符模型」（大写字母 / `<` / 数字）估计。

结果与质量（Rust 版重新导出）：

- 19 种语言全部 42,638 条字符串导出成功；含替换字符 U+FFFD 的行占比：
  us/gb/de/fr/es/it/nl/pt/cz/pl/tr 约 0.8%–1.1%，jp/zh/cn/tw/kr 约 0.8%–1.8%，
  ru 3.2%，th 11.0%（泰文组合字符多）。
- 长文本 section（品牌/车辆简介、RaceMessage、PhotoToolTip、PartsCopyLine、
  剧情对白 <sseq ...>）基本完全正确，例如：
  - us: "In March 31, 1949, Carlo Abarth founded Abarth & C in Bologna, Italy..."
  - jp: "カルロ・アバルト自身が設計した記念すべき第1号車、アバルト 204 Aが誕生。..."
  - zh: "公司推出了初创车型——由卡洛·阿巴斯亲自设计的阿巴斯204 A。..."
- TireName 已解出：Wet / Dirt / Snow / Sports / Racing / Comfort。
- 零星瑕疵：短字符串尾部填充字节偶尔被解成多余字符（如 `Dirt/`）。
- 已导出 19 语言到 _re/extracted/text_decoded/<lang>.json 与 <lang>.csv
  （utf-8-sig，Excel 可直接打开）；原始容器在 _re/extracted/text/rtext/common/。

---

## 4. 已命名覆盖

| 指标 | 数值 |
|---|---|
| store 文件总数 | 95,339 |
| 已命名文件 | 12,279（12.9%） |
| store 总体积 | 282.03 GB |
| 已命名体积 | 107.27 GB（38.0%） |

已命名顶层分组（前 20）：

```
car 3443 / livery 3112 / scapes 1265 / carparts 1205 / piece 1142 / crs 934 /
tire 486 / character 201 / carshop 150 / font 109 / crowd 108 / sky 58 /
adtex 46 / rtext 24 / carsound 21 / projects 14 / common 12 / sound_gt 8 /
scripts 5 / develop 3
```

映射表：_re/reports/named.txt（格式 asset_path TAB contents相对路径）。

---

## 5. 全库类型普查

全量清单：_re/reports/store_census.csv（95,339 行：相对路径、大小、类型、首 32 字节）。

| 类型 | 数量 | 体积 |
|---|---:|---:|
| MDL5 模型（3D） | 8,149 | 70.39 GB |
| PACC（Polyphony 归档） | 47 | 73.82 GB |
| PACS | 47 | 0.97 GB |
| PACL 归档 | 144 | 9.83 GB |
| TXS5 贴图集 | 4,296 | 17.09 GB |
| IDP0（内嵌 TXS5） | 14,983 | 5.00 GB |
| GTEB | 495 | 8.25 GB |
| SCPS | 6,666 | 8.83 GB |
| IDP0 | 14,983 | 5.00 GB |
| JPEG 图片 | 16,823 | 3.37 GB |
| packed:zlib（Polyphony PackedFile） | 9,097 | 1.40 GB |
| SNDZ 音频 | 860 | 0.62 GB |
| CAR5 | 587 | 少量 |
| EVSF / EBAF / STRB / SCNB / SDEF 等 | 1,600+ | 少量 |
| MPRJ（.mproject，UI 工程） | 92 | 少量 |
| PNG | 797 | 0.21 GB |
| JSON / 纯文本 | 1,433 + 12,468 | 0.36 GB |
| MAP4 / ADSP / GCM7 / RCMD / EXPD / RT05 / OTTO / SHD5 等 | 各数十至上百 | 少量 |

补充修正：

- 00 00 00 0C 4A 58 4C 20（4,074 个）= JPEG XL，carshop/*.json 实际是 JXL 预览图。
- 15 个 MP4 / ISO-BMFF（ftyp @ 0x04，hvc1/HEVC），共 5.25 GB，最大单个 1.9 GB
  （X/H/6H9GN），即过场动画/影片资源。
- 01 00 00 00（4,798 个）为带版本头的容器族（含 .img 图像资源）。
- 0xFFF7EEC5 / 0xFFF7ED85 / 0xFFF7972F / 0xFFF7F32E = Polyphony PackedFile
  （zlib / zstd-tiny / zstd-regular / zstd-chunked），gt7store.py 已实现自动解包。
  实测 PS5 上 0xFFF7EEC5 为**裸 DEFLATE**（非 zlib 封装），偏移 0x04 是解压后大小；
  9,097 个该类文件全部是 **DDS 贴图**，已全部解包为 42.4 GB（`_re/extracted/textures`）。
- 全库 95,339 个文件已按类型硬链接到 `_re/extracted/by_kind/<kind>/`（零额外空间）。

---

## 6. 已产出物

| 路径 | 内容 |
|---|---|
| _re/reports/named.txt | 12,279 条 真实路径 到 store 文件 的映射 |
| _re/reports/store_census.csv | 95,339 行全库类型普查 |
| _re/reports/candidates_templates.txt | 1,406 万条候选路径（由 GTToolsSharp 模板生成） |
| _re/extracted/text/rtext/common/*.rt2 | 19 语言原始文本容器 |
| _re/extracted/text_decoded/*.json, *.csv | 19 语言解密文本（每种 285 section、42,638 条） |
| _re/extracted/named/ | 2,512 个已命名数据文件（JSON/DAT/SVG/GPB/STRB 等，含商店数据） |
| _re/extracted/probe/ | 首批验证文件（render.txs、main.adc、fontset_jp.gpb 等） |
| _re/keys/rt2_keystream.bin、_re/keys/rt2/*.ks.bin | 各语言 rt2 恢复出的密钥流 |
| _re/report/refs/GT7_UDP_telemetry_README.md | GT7 UDP 遥测协议完整文档（社区版） |
| _re/repos/ | 4 个上游工具仓库（见第 7 节） |
| _re/rust/gt7re/ | **Rust 并行解析器**（源码、CLI、单元测试、release 二进制） |
| _re/tree/ | 12,354 个已命名资源的硬链接镜像，可直接按真实目录浏览 |
| _re/catalog/ | 按类型拆分的 1,190 份清单（每个 magic 一份） |
| _re/extracted/audio/ | 860 个 SNDZ 音频文件（669 MB） |
| _re/extracted/textures/ | 9,097 张 DDS 贴图（42.4 GB，已解压） |
| _re/extracted/video/ | 15 个 MP4/HEVC 影片（5.25 GB） |
| _re/extracted/by_kind/ | 全库 95,339 文件按类型硬链接归档 |
| _re/reports/mdl5_index.json | 8,149 个 MDL5、59,498 个子模型记录、纹理/着色器偏移与 621 万标识符 |
| _re/reports/mdl5_summary.txt | 模型专属词汇统计（着色器参数 / 节点名） |
| _re/reports/txs5_index.json | 4,296 个 TXS5 / 7,902 个纹理描述符（格式、宽高、偏移） |
| _re/reports/txs5_index_idp0.json | IDP0 内嵌的 14,983 个 TXS5 描述符 |
| _re/reports/txs5_catalog.csv | 22,885 个纹理条目的可筛选清单（尺寸/格式/偏移） |
| _re/reports/mprj_strings.json | 92 个 UI 工程的 189,277 条字符串（含 129 条资源路径） |
| _re/extracted/idp0/ | 14,983 个 IDP0 载荷（4.99 GB，全部为 TXS5） |
| _re/extracted/idp0_txs/ | 同上，带 `.txs` 扩展名，可直接用 GTS_mdl 转 DDS |
| _re/extracted/ui_txs/ | 6,058 个从 GPB 内嵌 IDP0 抽出的 UI 贴图（1.42 GB） |
| _re/README.md | 全部产出的导航索引（文件数/体积） |
| _re/manifest.json | 机器可读的产出清单 |
| _re/extracted/tracks/rada/ | 85 个 course_line 的 392 个分块（向量 + 弧长，JSON） |
| _re/extracted/tracks/map4/ | **85 条赛道中心线**（点集 + 边表 + 长度，JSON/CSV） |
| _re/extracted/tracks/preview/ | 85 张赛道布局预览 PNG |

---

## 7. 工具链与用法

脚本目录 C:\gt7\gt7\_re\scripts\，Python 环境 C:\gt7\gt7\_re\venv

```powershell
$py = "C:\gt7\gt7\_re\venv\Scripts\python.exe"
$S  = "C:\gt7\gt7\_re\scripts"

# 路径 到 store 文件
& $py $S\gt7store.py resolve "common/render.txs"

# 全库类型普查
& $py $S\gt7store.py census --out C:\gt7\gt7\_re\reports\store_census.csv

# 按清单批量提取（自动解 PackedFile）
& $py $S\gt7store.py extract-hits C:\gt7\gt7\_re\reports\named.txt -o D:\out

# 大规模命名发现（numpy 向量化，千万级候选每分钟）
& $py $S\gtfast.py --lists <候选文件> --mine --expand --rounds 3 --static

# 本地化文本：恢复密钥流并导出 JSON/CSV
& $py $S\rt2_decode.py auto <rt2 文件...> -o <输出目录>

# 二进制结构查看与字符串提取
& $py $S\binview.py <file> --offset 0x40 --length 0x200
& $py $S\binview.py <file> --utf16 --limit 30
```

上游工具仓库（_re/repos/）：

| 仓库 | 用途 |
|---|---|
| flatz/gt7tool | GT7 PDIPFS 与 index 解析、PackedFile、ChaCha20 与 crc32 解密参考 |
| Nenkai/GTToolsSharp | GT 系列 volume 解包与重打包，含 MPHFileListBruteforcer（GT7 路径模板库）与 SpecDB/menudb 密钥 |
| Nenkai/PDTools | PDTools.RText（rt2）、PDTools.SpecDB、模型解析、SimulatorInterface（UDP） |
| MacManley/gt7-udp | GT7 UDP 遥测包格式（Packet A/B/T/S 字段表） |

另外已按要求安装 GitHub 技能 agent-reach（Panniantong/Agent-Reach）到
C:\Users\hxmwi\.codex\skills\agent-reach，下一轮对话生效。

### Rust 并行解析器（最终交付）

源码 `_re/rust/gt7re`，二进制 `_re/rust/gt7re/target/release/gt7re.exe`。
它复刻并取代了此前全部 Python 脚本，且为数据并行实现（rayon）：

```powershell
$exe = "C:\gt7\gt7\_re\rust\gt7re\target\release\gt7re.exe"
& $exe resolve "common/render.txs"          # 路径 -> 文件
& $exe stats                                 # 库统计
& $exe census --out census.csv               # 全库类型普查
& $exe catalog --csv census.csv --out catalog
& $exe discover --list gts_files.txt --named named.txt --mine --expand
& $exe extract --list named.txt --out tree --mode link
& $exe extract-kind --csv census.csv --kind "sound (SNDZ)" --out audio
& $exe rt2 decode us.rt2 --out text --keydir keys      # 恢复密钥流
& $exe rt2 decode *.rt2 --out text --keystream keys\us.ks.bin
& $exe packed some.packed --out out.bin
& $exe mdl5-index --csv census.csv --out mdl5_index.json
& $exe map4 --named named.txt --out tracks\map4
& $exe rada --named named.txt --out tracks\rada --polyline
& $exe txs5-index --csv census.csv --out txs5_index.json
& $exe idp0-extract --csv census.csv --out extracted\idp0
& $exe udp layout ; & $exe udp parse capture.bin
```

实测性能（本机）：扫描 95,339 文件 3.8 s；全库普查 6.2 s；12,354 文件硬链接
镜像 3.3 s；RT2 单语言 0.18 s；19 语言合计 1.9 s。`cargo test --release` 通过
（2 个单元测试，含 4 组已知路径的 FNV-1a / cache-key 向量）。

---

## 8. 游戏侧已确认可读的数据

- 车辆数据：cs（二手车与传奇车商店 JSON，含 car_label、价格、稀有度、variation）；
  carshop/used_car_dealer、legend_car_dealer、brand_central_dealer 共 150 个文件。
- 品牌与车辆文本：ABARTH_DESCRIPTION、TOYOTA_DESCRIPTION、CAR_HISTORY_DESCRIPTION、
  MUSEUM 等 section（19 语言）。
- UI 文本：RaceMessage、QuickMenu、PhotoToolTip、PartsCopyLine、GriefReport 等 section。
- UI 工程：products/gt7 下的 .mproject、.adc、.gpb（普查中 MPRJ 92 个）。
- 赛道：crs/*（934 个已命名）。每个赛道布局 `crs/<course>/l0X/` 下是分开的资源：
  `course_line`（RADA）、`course_map`（MAP4）、`enemy_spline`（ADSP）、
  `replay`（GCM7）、`pack`（PACL）、`tex_stream`、`auto_drive`。
  RADA 已解析：每个分块 = {count, flags, step, 起点切向, 起点坐标,
  向量区偏移, 弧长区偏移}；向量区是 count × 6 字节（3×i16），
  弧长区是 count × f32 累计距离。全库 85 个 `course_line` 共 392 个分块，
  弧长 0.8 km – 25.7 km（最长 25.7 km），已导出到 `_re/extracted/tracks/rada/`。
  逐点向量按「单位向量 × 弧长增量」可重建折线，但闭合误差中位数 11%，
  且包围盒偏小，说明向量更可能是相对中心线的偏移/法线；
  已导出 393 个 CSV（含 summary.csv）作为近似视图，精确几何仍待 MAP4 解析。
  **MAP4 已完全解析**：`course_map` 是 2.5 m 等距采样的赛道中心线，
  85 条赛道全部闭合（误差 0.00 m），长度与 RADA 弧长误差 < 0.3%
  （c052_l01 5807.3 m，c180_l03 25406.8 m，最长 c193_l01 30.4 km），
  已导出点集/边表并渲染 85 张布局预览图。
- 角色与人群：character/*（201）、crowd/pos（108）。
- 音频：SNDZ 860 个，sound_gt 下 .szd、.szd3、.irf，以及 carsound/。
- 贴图与图像：TXS5 4,296、JPG 16,823、PNG 797、JXL 4,074、.img。

---

## 9. 尚未完成与下一步

1. eboot.bin 反编译：PS5 SELF 为加密态，需要主机密钥或社区 SELF 解密工具，
   这是拿到代码主体逻辑的前提。
2. ~~scripts/gt7/*.adc 反编译~~：已完成——`scriptsmenu.adc` 中的 1,906 个
   Adhoc 菜单脚本已全部反汇编为 `.diss`，共 1,325,950 行；归档为
   `reports/gt7_menu_scripts_diss.zip`，逐文件 SHA-256 见
   `reports/menu_disassembly_manifest.csv`。
3. ~~rtext 短标签补全~~：已完成——密钥流全局复用后 19 语言坏行率 0.8%–3.2%
   （泰文 11%），长文本全部可读。
4. gt.idx 的 index key：PS5 版本与 PS4 不同，如需 100% 官方文件名，
   需要从 eboot 或社区新 key 取得。
5. 3D 与贴图集：DDS 贴图已全部解包（9,097 张 / 42.4 GB）。
    MDL5 已完成元数据索引（8,149 个文件、59,498 个子模型记录、621 万标识符，
   含 baseColorMap/clearcoatMap/flake_size/steering_rot 等），
   但完整几何（顶点/索引/材质绑定）尚未导出为 OBJ/glTF；
   TXS5（17 GB）仍是自定义容器，字段已定位（尺寸/格式/数据偏移）；
   人头像使用的 GNM mode 9 + BC7 已成功解码，其余 tile 组合仍需逐格式验证。
6. 音频解码：SNDZ 已按类型导出（860 个）；.szd/.szd3（Sony SCEE 变体）、
   .irf 仍需容器解析。
7. 视频：已定位 15 个 MP4（5.25 GB，HEVC）；可直接用 ffmpeg 处理。
8. 网络层：UDP 遥测协议已实现（Rust `udp parse`）；如需游戏自身在线服务请求格式，
   需要先完成第 1 项。
9. 全量按名导出：已完成 12,354 文件硬链接镜像；剩余未命名文件可用
   `catalog` + `extract-kind` 按类型取出。


