#!/usr/bin/env python3
"""Build a manifest + master README for the GT7 reverse-engineering output tree."""

import json
from pathlib import Path

ROOT = Path(r"C:\gt7\gt7\_re")

SECTIONS = [
    ("reports/named.txt", "真实资源路径 → store 文件映射"),
    ("reports/store_census_rust.csv", "全库 95,339 文件类型普查"),
    ("reports/mdl5_index.json", "8,149 个 MDL5 模型的头部与标识符"),
    ("reports/txs5_index.json", "4,296 个直接 TXS5 的纹理描述符"),
    ("reports/txs5_index_idp0.json", "14,983 个 IDP0 内嵌 TXS5 描述符"),
    ("extracted/text_decoded", "19 语言本地化文本（JSON/CSV）"),
    ("extracted/textures", "9,097 张 DDS 贴图（42.4 GB）"),
    ("extracted/idp0", "14,983 个 IDP0 载荷（4.99 GB）"),
    ("extracted/audio", "860 个 SNDZ 音频"),
    ("extracted/video", "15 个 MP4/HEVC 影片"),
    ("extracted/tracks/map4", "85 条赛道中心线（点集 + 边表）"),
    ("extracted/tracks/preview", "85 张赛道布局预览 PNG"),
    ("extracted/tracks/rada", "392 个 RADA 分块（向量 + 弧长）"),
    ("extracted/by_kind", "全库按类型硬链接归档（95,339）"),
    ("tree", "12,354 个已命名资源的真实路径镜像"),
    ("rust/gt7re", "Rust 并行解析器源码与二进制"),
    ("report/FORMATS.md", "格式手册"),
    ("report/GT7_PS5_解包逆向_报告.md", "总报告"),
]


def dir_stats(p: Path):
    n = 0
    b = 0
    for f in p.rglob("*"):
        if f.is_file():
            n += 1
            b += f.stat().st_size
    return n, b


def main() -> None:
    rows = []
    for rel, desc in SECTIONS:
        p = ROOT / rel
        if not p.exists():
            rows.append({"path": rel, "desc": desc, "exists": False})
            continue
        if p.is_dir():
            n, b = dir_stats(p)
            rows.append({"path": rel, "desc": desc, "exists": True, "files": n, "bytes": b})
        else:
            rows.append({
                "path": rel,
                "desc": desc,
                "exists": True,
                "files": 1,
                "bytes": p.stat().st_size,
            })

    (ROOT / "manifest.json").write_text(
        json.dumps(rows, ensure_ascii=False, indent=1), encoding="utf-8"
    )

    lines = [
        "# GT7 (PS5) 逆向工程产出索引",
        "",
        f"根目录：`{ROOT}`　（内容库：`C:\\gt7\\gt7\\contents`）",
        "",
        "| 路径 | 内容 | 文件数 | 大小 |",
        "|---|---|---:|---:|",
    ]
    for r in rows:
        if not r["exists"]:
            lines.append(f"| `{r['path']}` | {r['desc']} | — | — |")
            continue
        size = r["bytes"] / 2**30
        lines.append(
            f"| `{r['path']}` | {r['desc']} | {r['files']:,} | {size:.2f} GB |"
        )
    lines += [
        "",
        "## 快速入口",
        "",
        "- 工具：`rust/gt7re/target/release/gt7re.exe`（18 个子命令）",
        "- 格式：`report/FORMATS.md`",
        "- 总报告：`report/GT7_PS5_解包逆向_报告.md`",
        "- 赛道预览：`extracted/tracks/preview/`",
        "- 文本浏览：`extracted/text_decoded/*.csv`",
        "",
    ]
    (ROOT / "README.md").write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines[:16]))
    print(f"\nmanifest -> {ROOT/'manifest.json'}")


if __name__ == "__main__":
    main()
