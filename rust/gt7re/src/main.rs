use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use gt7re::census;
use gt7re::discover;
use gt7re::extract::{self, Mode};
use gt7re::gpb;
use gt7re::idp0;
use gt7re::mdl5;
use gt7re::map4;
use gt7re::packed;
use gt7re::rada;
use gt7re::research;
use gt7re::rt2;
use gt7re::store::Store;
use gt7re::txs5;
use gt7re::udp;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gt7re", version, about = "Gran Turismo 7 (PS5) parallel content-store parser")]
struct Cli {
    /// Path to the game's contents directory
    #[arg(long, global = true, default_value = r"C:\gt7\gt7\contents")]
    store: PathBuf,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Resolve asset path(s) to store files
    Resolve { paths: Vec<String> },
    /// Show store statistics
    Stats,
    /// Classify every store file and write a CSV census
    Census {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        out: PathBuf,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Grow the asset-name map with parallel candidate testing
    Discover {
        #[arg(long = "list", value_name = "FILE")]
        lists: Vec<PathBuf>,
        #[arg(long, default_value = "named.txt")]
        named: PathBuf,
        #[arg(long)]
        mine: bool,
        #[arg(long)]
        expand: bool,
        #[arg(long)]
        depth2: bool,
        #[arg(long, default_value_t = 2)]
        rounds: usize,
    },
    /// Extract (or hard-link) named assets into a real directory tree
    Extract {
        #[arg(long)]
        list: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "copy")]
        mode: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// RT2 localisation container tools
    Rt2 {
        #[command(subcommand)]
        cmd: Rt2Cmd,
    },
    /// Build dealer descriptions and exact GT Cafe scenes from DMSB + RT2
    Research {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\extracted\text\rtext\common")]
        rt2_dir: PathBuf,
        #[arg(long, default_value = r"C:\gt7\gt7\_re\extracted\dms_prpys\dmsdata")]
        dmsb_dir: PathBuf,
        #[arg(long, default_value = r"C:\gt7\gt7\_re\keys\rt2_rust")]
        key_dir: PathBuf,
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\research_rust.json")]
        out: PathBuf,
        /// Optional JSON array of CarDetail ids for the prior 253-row audit
        #[arg(long)]
        legacy_review: Option<PathBuf>,
    },
    /// Unpack a Polyphony PackedFile
    Packed {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// GT7 UDP telemetry tools
    Udp {
        #[command(subcommand)]
        cmd: UdpCmd,
    },
    /// Split a census CSV into per-type catalog lists
    Catalog {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long, default_value = "catalog")]
        out: PathBuf,
    },
    /// Extract every store file of one census kind (by rel path)
    ExtractKind {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "copy")]
        mode: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Hard-link every store file into `<out>/<kind>/<rel>` (no extra space)
    LinkAll {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Index MDL5 model metadata (header + identifier strings)
    Mdl5Index {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 2)]
        head_mb: u64,
        #[arg(long, default_value_t = 2000)]
        max_strings: usize,
    },
    /// Parse RADA (`course_line`) sections and export their vectors
    Rada {
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        named: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        /// Also write one polyline CSV per section plus a summary CSV
        #[arg(long)]
        polyline: bool,
    },
    /// Parse MAP4 course maps (centreline points + edge table)
    Map4 {
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        named: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Index TXS5 texture-set descriptors (format / size / dimensions)
    Txs5Index {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Index files from an extracted directory instead of the census
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Extract the payloads of all IDP0 containers (normally TXS5 textures)
    Idp0Extract {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
        /// Append an extension to each extracted payload (e.g. ".txs")
        #[arg(long)]
        ext: Option<String>,
    },
    /// Extract printable strings from every file of one census kind
    Strings {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 2000)]
        max: usize,
    },
    /// Extract embedded IDP0/TXS5 assets from UI projects (.gpb / .mproject)
    GpbExtract {
        #[arg(long, default_value = r"C:\gt7\gt7\_re\reports\store_census_rust.csv")]
        csv: PathBuf,
        #[arg(long, default_value = "UI project (MPRJ)")]
        kind: String,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum Rt2Cmd {
    /// Recover keystream and decode .rt2 file(s) to JSON + CSV
    Decode {
        files: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        keydir: Option<PathBuf>,
        /// Use an existing keystream file instead of recovering one per file
        #[arg(long)]
        keystream: Option<PathBuf>,
    },
    /// Print container metadata
    Info { file: PathBuf },
}

#[derive(Subcommand)]
enum UdpCmd {
    /// Parse a captured packet (A/B/T/C)
    Parse { file: PathBuf },
    /// Print the packet layout table
    Layout,
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Resolve { paths } => {
            if paths.is_empty() {
                bail!("provide at least one asset path");
            }
            let store = Store::scan(&cli.store)?;
            for p in paths {
                match store.resolve(&p) {
                    Some(rel) => println!("{p} -> {rel}"),
                    None => println!("{p} -> NOT FOUND"),
                }
            }
        }
        Cmd::Stats => {
            let store = Store::scan(&cli.store)?;
            let (files, bytes) = store.stats();
            println!("store files: {files}");
            println!("store size : {:.2} GB", bytes as f64 / 2f64.powi(30));
        }
        Cmd::Census { out, report } => {
            let store = Store::scan(&cli.store)?;
            eprintln!("scanning {} files ...", store.rels.len());
            let s = census::write_census(&store, &out)?;
            println!(
                "files: {}  size: {:.2} GB  -> {}",
                s.files,
                s.bytes as f64 / 2f64.powi(30),
                out.display()
            );
            let mut v: Vec<_> = s.by_kind.iter().collect();
            v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
            for (k, (n, bytes)) in v.iter().take(40) {
                println!("{:<40} {:>9} {:>10.2} GB", k, n, *bytes as f64 / 2f64.powi(30));
            }
            if let Some(r) = report {
                census::write_report(&s, &r)?;
                println!("report -> {}", r.display());
            }
        }
        Cmd::Discover { lists, named, mine, expand, depth2, rounds } => {
            let store = Store::scan(&cli.store)?;
            let mut map = discover::load_named(&named);
            println!("start: {} named files", map.len());

            let mut cands: Vec<String> = Vec::new();
            for list in &lists {
                let text = fs::read_to_string(list)
                    .with_context(|| format!("reading {}", list.display()))?;
                for line in text.lines() {
                    let s = line.trim().trim_matches('"').trim_end_matches(',');
                    if !s.is_empty() {
                        cands.push(s.to_string());
                        cands.push(s.to_lowercase());
                    }
                }
            }
            let mut seen: HashSet<String> = HashSet::new();
            cands.retain(|c| seen.insert(c.clone()));
            println!("list candidates: {}", cands.len());
            let mut added = 0usize;
            for (c, rel) in discover::test_candidates(&store, &cands) {
                if map.insert(c, rel).is_none() {
                    added += 1;
                }
            }
            discover::save_named(&named, &map)?;
            println!("after lists: +{added} -> {}", map.len());

            if mine {
                let tokens = discover::mine_named_files(&store, &map);
                println!("mined tokens: {}", tokens.len());
                let mut all = Vec::with_capacity(tokens.len() * 2);
                for t in tokens {
                    all.push(t.clone());
                    all.push(t.to_lowercase());
                }
                let mut added = 0usize;
                for (c, rel) in discover::test_candidates(&store, &all) {
                    if map.insert(c, rel).is_none() {
                        added += 1;
                    }
                }
                discover::save_named(&named, &map)?;
                println!("after mine: +{added} -> {}", map.len());
            }
            if expand {
                for r in 0..rounds {
                    let cands = discover::expand_candidates(&map, depth2);
                    println!("round {}: probing {} candidates ...", r + 1, cands.len());
                    let mut added = 0usize;
                    for (c, rel) in discover::test_candidates(&store, &cands) {
                        if map.insert(c, rel).is_none() {
                            added += 1;
                        }
                    }
                    discover::save_named(&named, &map)?;
                    println!("round {}: +{} -> {}", r + 1, added, map.len());
                    if added == 0 {
                        break;
                    }
                }
            }
            discover::save_named(&named, &map)?;
            println!("final: {} named -> {}", map.len(), named.display());
        }
        Cmd::Extract { list, out, mode, limit } => {
            let store = Store::scan(&cli.store)?;
            let entries = extract::load_named(&list)?;
            let mode = match mode.as_str() {
                "copy" => Mode::Copy,
                "link" => Mode::Link,
                other => bail!("unknown mode {other} (copy|link)"),
            };
            println!("entries: {}  mode: {mode:?}", entries.len());
            let st = extract::extract_named(&store, &entries, &out, mode, limit)?;
            println!(
                "extracted {}  failed {}  bytes {:.2} GB -> {}",
                st.extracted,
                st.failed,
                st.bytes as f64 / 2f64.powi(30),
                out.display()
            );
        }
        Cmd::Rt2 { cmd } => match cmd {
            Rt2Cmd::Decode { files, out, keydir, keystream } => {
                let shared = match &keystream {
                    Some(p) => Some(fs::read(p)?),
                    None => None,
                };
                for f in files {
                    if let Some(ks) = &shared {
                        let n = rt2::decode_with_key(&f, ks, &out)?;
                        println!("{}: {n} strings (shared keystream)", f.display());
                    } else {
                        let (ks, n) = rt2::auto_decode(&f, &out, keydir.as_deref())?;
                        println!("{}: {n} strings, keystream {ks} bytes", f.display());
                    }
                }
            }
            Rt2Cmd::Info { file } => {
                let rt = rt2::Rt2File::parse(&file)?;
                println!("sections: {}", rt.sections.len());
                for s in rt.sections.iter().take(20) {
                    println!("  {:<36} entries={}", s.name, s.entries.len());
                }
                println!("strings: {}", rt.payloads().len());
                println!("max payload: {}", rt.max_payload_len());
            }
        },
        Cmd::Research {
            rt2_dir,
            dmsb_dir,
            key_dir,
            out,
            legacy_review,
        } => {
            let started = std::time::Instant::now();
            let report = research::build(&research::Options {
                rt2_dir,
                dmsb_dir,
                key_dir,
                out: out.clone(),
                legacy_review,
            })?;
            println!(
                "{} -> {} ({:.2}s)",
                serde_json::to_string(&report.stats)?,
                out.display(),
                started.elapsed().as_secs_f64()
            );
        }
        Cmd::Packed { file, out } => {
            let data = fs::read(&file)?;
            let (payload, kind) = packed::unpack(&data)?;
            fs::write(&out, &payload)?;
            println!("{kind}: {} -> {} ({} bytes)", data.len(), out.display(), payload.len());
        }
        Cmd::Udp { cmd } => match cmd {
            UdpCmd::Parse { file } => {
                let data = fs::read(&file)?;
                let pkt = if data.len() > 368 {
                    &data[..368]
                } else {
                    &data[..]
                };
                println!("{}", serde_json::to_string_pretty(&udp::parse(pkt)?)?);
            }
            UdpCmd::Layout => println!("{}", serde_json::to_string_pretty(&udp::layout())?),
        },
        Cmd::Catalog { csv, out } => {
            let rows = census::read_catalog(&csv)?;
            let summary = census::write_catalog(&rows, &out)?;
            for (kind, n, bytes) in summary.iter().take(50) {
                println!("{:<40} {:>9} {:>10.2} GB", kind, n, *bytes as f64 / 2f64.powi(30));
            }
        }
        Cmd::ExtractKind { csv, kind, out, mode, limit } => {
            let store = Store::scan(&cli.store)?;
            let rows = census::read_catalog(&csv)?;
            let entries: Vec<extract::Named> = rows
                .into_iter()
                .filter(|(_, _, k)| k == &kind)
                .map(|(rel, _, _)| extract::Named {
                    asset: format!("{}/{}", census::sanitize_kind(&kind), rel),
                    rel,
                })
                .collect();
            if entries.is_empty() {
                bail!("no files of kind {kind}");
            }
            let mode = match mode.as_str() {
                "copy" => Mode::Copy,
                "link" => Mode::Link,
                other => bail!("unknown mode {other} (copy|link)"),
            };
            println!("{} files of kind {kind}", entries.len());
            let st = extract::extract_named(&store, &entries, &out, mode, limit)?;
            println!(
                "extracted {}  failed {}  bytes {:.2} GB -> {}",
                st.extracted,
                st.failed,
                st.bytes as f64 / 2f64.powi(30),
                out.display()
            );
        }
        Cmd::LinkAll { csv, out } => {
            let store = Store::scan(&cli.store)?;
            let rows = census::read_catalog(&csv)?;
            let entries: Vec<extract::Named> = rows
                .into_iter()
                .map(|(rel, _, kind)| extract::Named {
                    asset: format!("{}/{}", census::sanitize_kind(&kind), rel),
                    rel,
                })
                .collect();
            println!("linking {} files ...", entries.len());
            let st = extract::extract_named(&store, &entries, &out, Mode::Link, None)?;
            println!(
                "linked {}  failed {}  bytes {:.2} GB -> {}",
                st.extracted,
                st.failed,
                st.bytes as f64 / 2f64.powi(30),
                out.display()
            );
        }
        Cmd::Mdl5Index { csv, out, head_mb, max_strings } => {
            let rows = census::read_catalog(&csv)?;
            let entries: Vec<(String, u64)> = rows
                .into_iter()
                .filter(|(_, _, k)| k == "model-3d (MDL5)")
                .map(|(rel, size, _)| (rel, size))
                .collect();
            println!("indexing {} MDL5 files ...", entries.len());
            let infos = mdl5::index_many(&entries, &cli.store, head_mb << 20, max_strings);
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?;
            }
            fs::write(&out, serde_json::to_vec(&infos)?)?;
            let with = infos.iter().filter(|i| !i.strings.is_empty()).count();
            let total: usize = infos.iter().map(|i| i.strings.len()).sum();
            println!(
                "indexed {} models ({} with identifiers, {} strings) -> {}",
                infos.len(),
                with,
                total,
                out.display()
            );
        }
        Cmd::Rada { file, named, out, polyline } => {
            if let Some(f) = file {
                let r = rada::parse(&f)?;
                if let Some(p) = out.parent() {
                    fs::create_dir_all(p)?;
                }
                fs::write(&out, serde_json::to_vec_pretty(&r)?)?;
                if polyline {
                    let dir = out.with_extension("");
                    fs::create_dir_all(&dir)?;
                    for (i, sec) in r.sections.iter().enumerate() {
                        rada::write_polyline_csv(sec, &dir.join(format!("{i:02}.csv")))?;
                    }
                }
                println!(
                    "{}: {} sections, {} points -> {}",
                    f.display(),
                    r.sections.len(),
                    r.sections.iter().map(|s| s.points.len()).sum::<usize>(),
                    out.display()
                );
            } else if let Some(n) = named {
                let store = Store::scan(&cli.store)?;
                let text = fs::read_to_string(&n)?;
                fs::create_dir_all(&out)?;
                let mut done = 0usize;
                let mut summary = String::from("asset,section,flags,count,length_m,closure_m,px,pz\n");
                for line in text.lines() {
                    let Some((asset, rel)) = line.split_once('\t') else { continue };
                    if !asset.contains("course_line") {
                        continue;
                    }
                    let p = store.path_of_rel(rel.trim());
                    match rada::parse(&p) {
                        Ok(r) => {
                            let stem = asset.replace('/', "_");
                            fs::write(
                                out.join(format!("{stem}.json")),
                                serde_json::to_vec(&r)?,
                            )?;
                            if polyline {
                                let dir = out.join(&stem);
                                fs::create_dir_all(&dir)?;
                                for (i, sec) in r.sections.iter().enumerate() {
                                    rada::write_polyline_csv(sec, &dir.join(format!("{i:02}.csv")))?;
                                    let pts = rada::reconstruct(sec);
                                    let length = sec
                                        .distances
                                        .iter()
                                        .cloned()
                                        .fold(f32::MIN, f32::max);
                                    let closure = pts
                                        .first()
                                        .zip(pts.last())
                                        .map(|(a, b)| {
                                            ((a[0] - b[0]).powi(2)
                                                + (a[1] - b[1]).powi(2)
                                                + (a[2] - b[2]).powi(2))
                                            .sqrt()
                                        })
                                        .unwrap_or(0.0);
                                    let px = pts.iter().map(|p| p[0]).fold(f32::MIN, f32::max)
                                        - pts.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
                                    let pz = pts.iter().map(|p| p[2]).fold(f32::MIN, f32::max)
                                        - pts.iter().map(|p| p[2]).fold(f32::MAX, f32::min);
                                    summary.push_str(&format!(
                                        "{stem},{i},{},{},{length:.1},{closure:.1},{px:.1},{pz:.1}\n",
                                        sec.flags,
                                        sec.count
                                    ));
                                }
                            }
                            done += 1;
                        }
                        Err(e) => eprintln!("skip {asset}: {e}"),
                    }
                }
                if polyline {
                    fs::write(out.join("summary.csv"), summary)?;
                }
                println!("parsed {done} course_line files -> {}", out.display());
            } else {
                bail!("provide --file or --named");
            }
        }
        Cmd::Map4 { file, named, out } => {
            if let Some(f) = file {
                let m = map4::parse(&f)?;
                if let Some(p) = out.parent() {
                    fs::create_dir_all(p)?;
                }
                fs::write(&out, serde_json::to_vec_pretty(&m)?)?;
                let csv = out.with_extension("csv");
                map4::write_csv(&m, &csv)?;
                println!(
                    "{}: {} points, {} edges, {:.1} m, closure {:.2} m -> {}",
                    f.display(),
                    m.point_count,
                    m.edge_count,
                    m.length_m,
                    m.closure_m,
                    out.display()
                );
            } else if let Some(n) = named {
                let store = Store::scan(&cli.store)?;
                let text = fs::read_to_string(&n)?;
                fs::create_dir_all(&out)?;
                let mut summary = String::from("asset,points,edges,length_m,closure_m,cell\n");
                let mut done = 0usize;
                for line in text.lines() {
                    let Some((asset, rel)) = line.split_once('\t') else { continue };
                    if !asset.contains("course_map") {
                        continue;
                    }
                    let p = store.path_of_rel(rel.trim());
                    match map4::parse(&p) {
                        Ok(m) => {
                            let stem = asset.replace('/', "_");
                            let json = out.join(format!("{stem}.json"));
                            fs::write(&json, serde_json::to_vec(&m)?)?;
                            map4::write_csv(&m, &out.join(format!("{stem}.csv")))?;
                            summary.push_str(&format!(
                                "{stem},{},{},{:.1},{:.2},{}\n",
                                m.point_count, m.edge_count, m.length_m, m.closure_m, m.cell_size
                            ));
                            done += 1;
                        }
                        Err(e) => eprintln!("skip {asset}: {e}"),
                    }
                }
                fs::write(out.join("summary.csv"), summary)?;
                println!("parsed {done} course_map files -> {}", out.display());
            } else {
                bail!("provide --file or --named");
            }
        }
        Cmd::Txs5Index { csv, out, dir } => {
            let store = Store::scan(&cli.store)?;
            if let Some(d) = dir {
                let mut entries = Vec::new();
                for e in walkdir::WalkDir::new(&d).into_iter().filter_map(|e| e.ok()) {
                    if e.file_type().is_file() {
                        entries.push(e.path().to_path_buf());
                    }
                }
                println!("indexing {} files under {} ...", entries.len(), d.display());
                let files: Vec<txs5::Txs5File> = entries
                    .par_iter()
                    .filter_map(|p| {
                        let rel = p.strip_prefix(&d).ok()?.to_string_lossy().to_string();
                        txs5::parse(&rel, p).ok()
                    })
                    .collect();
                if let Some(p) = out.parent() {
                    fs::create_dir_all(p)?;
                }
                fs::write(&out, serde_json::to_vec(&files)?)?;
                let total_entries: usize = files.iter().map(|f| f.entries.len()).sum();
                println!("indexed {} files ({} descriptors)", files.len(), total_entries);
                return Ok(());
            }
            let rows = census::read_catalog(&csv)?;
            let entries: Vec<String> = rows
                .into_iter()
                .filter(|(_, _, k)| k == "TXS5 texture-set")
                .map(|(rel, _, _)| rel)
                .collect();
            println!("indexing {} TXS5 files ...", entries.len());
            let files: Vec<txs5::Txs5File> = entries
                .par_iter()
                .filter_map(|rel| txs5::parse(rel, &store.path_of_rel(rel)).ok())
                .collect();
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?;
            }
            fs::write(&out, serde_json::to_vec(&files)?)?;
            let total_entries: usize = files.iter().map(|f| f.entries.len()).sum();
            let texels: u64 = files
                .iter()
                .flat_map(|f| f.entries.iter())
                .map(|e| e.width as u64 * e.height as u64 * e.count.max(1) as u64)
                .sum();
            println!(
                "indexed {} files ({} descriptors, {} texels) -> {}",
                files.len(),
                total_entries,
                texels,
                out.display()
            );
        }
        Cmd::Idp0Extract { csv, out, limit, ext } => {
            let store = Store::scan(&cli.store)?;
            let rows = census::read_catalog(&csv)?;
            let mut entries: Vec<String> = rows
                .into_iter()
                .filter(|(_, _, k)| k == "IDP0 container")
                .map(|(rel, _, _)| rel)
                .collect();
            if let Some(n) = limit {
                entries.truncate(n);
            }
            println!("extracting {} IDP0 payloads ...", entries.len());
            let results: Vec<(String, u64, String)> = entries
                .par_iter()
                .filter_map(|rel| {
                    let src = store.path_of_rel(rel);
                    let mut dest = out.join(rel);
                    if let Some(e) = &ext {
                        dest.set_extension(e.trim_start_matches('.'));
                    }
                    match idp0::extract(&src, &dest) {
                        Ok(h) => {
                            let magic = fs::read(&dest)
                                .ok()
                                .map(|d| {
                                    d.get(0..4)
                                        .map(|b| {
                                            b.iter()
                                                .map(|c| {
                                                    if (0x20..0x7F).contains(c) {
                                                        *c as char
                                                    } else {
                                                        '.'
                                                    }
                                                })
                                                .collect::<String>()
                                        })
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                            Some((rel.clone(), h.payload_size, magic))
                        }
                        Err(e) => {
                            eprintln!("skip {rel}: {e}");
                            None
                        }
                    }
                })
                .collect();
            let bytes: u64 = results.iter().map(|r| r.1).sum();
            let mut kinds: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for (_, _, magic) in &results {
                *kinds.entry(magic.as_str()).or_insert(0) += 1;
            }
            println!(
                "extracted {} payloads, {:.2} GB -> {}",
                results.len(),
                bytes as f64 / 2f64.powi(30),
                out.display()
            );
            let mut kv: Vec<_> = kinds.into_iter().collect();
            kv.sort_by(|a, b| b.1.cmp(&a.1));
            for (m, n) in kv.iter().take(8) {
                println!("  inner magic {m:?}: {n}");
            }
        }
        Cmd::Strings { csv, kind, out, max } => {
            let store = Store::scan(&cli.store)?;
            let rows = census::read_catalog(&csv)?;
            let entries: Vec<String> = rows
                .into_iter()
                .filter(|(_, _, k)| k == &kind)
                .map(|(rel, _, _)| rel)
                .collect();
            println!("scanning {} files of kind {kind} ...", entries.len());
            let result: Vec<(String, Vec<String>)> = entries
                .par_iter()
                .map(|rel| {
                    let mut out = Vec::new();
                    let mut seen = std::collections::HashSet::new();
                    if let Ok(data) = fs::read(store.path_of_rel(rel)) {
                        let mut run = Vec::new();
                        for &b in data.iter().chain(std::iter::once(&0u8)) {
                            if (0x20..0x7F).contains(&b) {
                                run.push(b);
                            } else {
                                if run.len() >= 4 {
                                    let s = String::from_utf8_lossy(&run).trim().to_string();
                                    if s.len() <= 200 && seen.insert(s.clone()) {
                                        out.push(s);
                                        if out.len() >= max {
                                            break;
                                        }
                                    }
                                }
                                run.clear();
                            }
                        }
                    }
                    (rel.clone(), out)
                })
                .collect();
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?;
            }
            fs::write(&out, serde_json::to_vec(&result)?)?;
            let total: usize = result.iter().map(|r| r.1.len()).sum();
            println!("{} files, {} strings -> {}", result.len(), total, out.display());
        }
        Cmd::GpbExtract { csv, kind, out } => {
            let store = Store::scan(&cli.store)?;
            let rows = census::read_catalog(&csv)?;
            let entries: Vec<String> = rows
                .into_iter()
                .filter(|(_, _, k)| k == &kind)
                .map(|(rel, _, _)| rel)
                .collect();
            println!("scanning {} files of kind {kind} ...", entries.len());
            let counts: Vec<(String, usize)> = entries
                .par_iter()
                .map(|rel| {
                    let stem = rel.replace('/', "_");
                    let n = gpb::extract_file(&store.path_of_rel(rel), &stem, &out).unwrap_or(0);
                    (rel.clone(), n)
                })
                .collect();
            let total: usize = counts.iter().map(|c| c.1).sum();
            let nonempty = counts.iter().filter(|c| c.1 > 0).count();
            println!(
                "{} files, {} embedded assets, {} files with assets -> {}",
                counts.len(),
                total,
                nonempty,
                out.display()
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
