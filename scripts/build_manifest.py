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
    ("reports/gt7_menu_scripts_diss.zip", "1,906 个 Adhoc 菜单脚本反汇编"),
    ("reports/menu_disassembly_manifest.csv", "菜单反汇编逐文件 SHA-256 清单"),
    ("reports/research_rust.json", "Rust DMSB + RT2 + GT Cafe 研究结果"),
    ("reports/face_atlases_rust", "19 位角色的游戏内真实头像图集输出"),
    ("site/public/avatars/extracted", "站点使用的 19 张真实角色头像"),
    ("site/public/data", "研究站点 JSON/CSV/Markdown 数据"),
    ("report/FINAL_STATUS.md", "逐项完成度、产物和格式限制"),
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

    for row in rows:
        if not row["exists"]:
            print(f"{row['path']}: MISSING")
            continue
        print(
            f"{row['path']}: {row['files']:,} files, "
            f"{row['bytes'] / 2**30:.2f} GiB"
        )
    print(f"\nmanifest -> {ROOT/'manifest.json'}")


if __name__ == "__main__":
    main()
