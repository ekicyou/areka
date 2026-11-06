//! 合成の時間を測る（audit-emo-compose の指摘 3・8）。決定論のテストではない。
//!
//! 測るもの（どれも公開の口だけを使うので、直す前のコミットにこのファイルを写しても同じに測れる）:
//! - `diamond13`: 13 段の菱形（面 0 … 12 がそれぞれ次の面を 2 か所に置く）を面 0 から合成する。
//!   予算の単位（入る面ごとに「入る 1 ＋その面の行の全部（`surface_rows`）」＋段の積み荷・根の面は数えない）で 65,526 で、
//!   予算 2^16 の内に収まる。ここから命令列の走査の 1 単位あたりの時間を出し、16.667 ms に収まる単位の数を
//!   2 のべき乗に切り下げて示す（予算 `VISIT_BUDGET` の値の測り直しに使う）。重ねる画素の上限（外形の面積×64）で
//!   重ねるのは頭の命令だけなので、時間のほとんどは命令列の走査。
//! - `diamond30_cut`: 30 段の菱形（2^30 の道）を合成する＝今の予算で切れたときの 1 回の時間。
//! - `chain10000`: 10000 段の鎖を合成する。
//! - `emo2_all`: 検体 emo2 の全部の面を 1 つずつ合成した合計（外形の表の効き目を比べる）。
//! - `visible_*`: 見える部品の確かめ（`visible_parts`）を鎖と菱形で回す。
//!
//! 回し方: `cargo run --release -p areka-emo-compose --example compose_timing`。出力は ASCII の行だけ。

use std::path::Path;
use std::time::{Duration, Instant};

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState};
use areka_parsers::shell::{Shell, parse};
use sample_ghost_kit::SampleRoot;

/// 1 フレーム（60 Hz）の長さ（ns）。
const FRAME_NS: f64 = 16_667_000.0;

/// 1 つの場合を回す長さの目安と回数の下限・上限。
const RUN_FOR: Duration = Duration::from_secs(3);
const MIN_REPS: usize = 10;
const MAX_REPS: usize = 2000;

fn on_params() -> AlphaParams {
    AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    }
}

/// 面 0 … `last - 1` がそれぞれ次の面を `places` か所に置き、面 `last` が絵 `s.png` を持つシェル。
fn nested_world(last: u32, places: u32) -> (EmoWorld, AtlasTable) {
    let mut text = String::from("charset,UTF-8\n");
    for i in 0..last {
        text.push_str(&format!("surface{i}\n{{\n"));
        for p in 0..places {
            text.push_str(&format!("element{p},overlay,{},{p},0\n", i + 1));
        }
        text.push_str("}\n");
    }
    text.push_str(&format!(
        "surface{last}\n{{\nelement0,overlay,s.png,0,0\n}}\n"
    ));
    let base = Path::new("shell/master");
    let mut dec = MemoryDecoder::new();
    dec.insert(base.join("s.png"), 1, 1, 4, vec![255; 4], true);
    let image_shell = parse("charset,UTF-8\nsurface0\n{\nelement0,overlay,s.png,0,0\n}\n");
    let atlas = bake_shell(&image_shell, base, &dec);
    let mut world = EmoWorld::build(&parse(&text));
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas)
}

fn bake_shell(shell: &Shell, base: &Path, dec: &MemoryDecoder) -> AtlasTable {
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    bake(std::slice::from_ref(&set), dec, PackConfig::default()).table
}

/// PNG の IHDR から原寸を読む。
fn png_size(path: &Path) -> Option<(u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((w, h))
}

/// emo2 のシェルを、全部の絵を実寸の不透明の画素で焼いて束縛する（golden の外形のテストと同じ組み方）。
fn emo2_world(sample: &SampleRoot) -> (EmoWorld, AtlasTable) {
    let base = sample.folder().join("shell/master");
    let text = std::fs::read_to_string(base.join("surfaces.txt")).expect("emo2 surfaces.txt");
    let shell = parse(&text);
    let mut dec = MemoryDecoder::new();
    let mut seen = std::collections::HashSet::new();
    for s in &shell.surfaces {
        for e in &s.elements {
            let abs = base.join(e.path.as_str());
            if !seen.insert(abs.clone()) {
                continue;
            }
            if let Some((w, h)) = png_size(&abs) {
                dec.insert(abs, w, h, w * 4, vec![255; (w * 4 * h) as usize], true);
            }
        }
    }
    let atlas = bake_shell(&shell, &base, &dec);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas)
}

/// `f` を暖めてから回し、中央値と最小（ns）と回数を返す。
fn time(mut f: impl FnMut()) -> (f64, f64, usize) {
    f();
    let start = Instant::now();
    let mut samples = Vec::new();
    while samples.len() < MIN_REPS || (start.elapsed() < RUN_FOR && samples.len() < MAX_REPS) {
        let t = Instant::now();
        f();
        samples.push(t.elapsed().as_nanos() as f64);
    }
    samples.sort_by(f64::total_cmp);
    (samples[samples.len() / 2], samples[0], samples.len())
}

fn report(case: &str, extra: &str, (median, min, reps): (f64, f64, usize)) {
    println!(
        "case={case} {extra} median_us={:.1} min_us={:.1} reps={reps}",
        median / 1000.0,
        min / 1000.0
    );
}

/// 面 0 を合成する場合を測り、時間の中央値（ナノ秒）を返す。
fn compose_case(case: &str, world: &EmoWorld, atlas: &AtlasTable) -> f64 {
    let mut composer = Composer::new();
    let mut out = ComposedSurface::new(0, 0);
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let mut f = || {
        composer
            .compose_into(&mut out, world, atlas, 0, &binds, &pattern)
            .expect("合成できる");
    };
    let r = time(&mut f);
    let px = out.width() as u64 * out.height() as u64;
    report(
        case,
        &format!("extent={}x{} px={px}", out.width(), out.height()),
        r,
    );
    r.0
}

fn visible_case(case: &str, world: &EmoWorld) {
    let table = world.nest_table();
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let mut parts = Vec::new();
    let r = time(|| table.visible_parts(0, &binds, &pattern, &mut parts));
    report(case, &format!("parts={}", parts.len()), r);
}

fn main() {
    // 菱形 13 段: 根の下の中の面（段 1〜12・各 2^d 回）は入る 1＋行 2＋子へ入る印 2＝5、葉（段 13・2^13 回）は
    // 入る 1＋行 1＋命令 1＝3（予算の単位・`budget.rs`）。
    const DIAMOND_LEVELS: u32 = 13;
    let units = 5 * ((1u64 << DIAMOND_LEVELS) - 2) + 3 * (1u64 << DIAMOND_LEVELS);
    let (world, atlas) = nested_world(DIAMOND_LEVELS, 2);
    let median = compose_case("diamond13", &world, &atlas);
    let per_unit = median / units as f64;
    let fit = FRAME_NS / per_unit;
    let pow = fit.log2().floor() as u32;
    println!(
        "per_unit_ns={per_unit:.2} units={units} fit_in_frame={fit:.0} budget_pow2=2^{pow}={}",
        1u64 << pow
    );
    visible_case("visible_diamond13", &world);

    let (world, atlas) = nested_world(30, 2);
    compose_case("diamond30_cut", &world, &atlas);

    let (world, atlas) = nested_world(10_000, 1);
    compose_case("chain10000", &world, &atlas);
    visible_case("visible_chain10000", &world);

    let sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let (world, atlas) = emo2_world(&sample);
    let ids: Vec<u32> = world.surface_ids().collect();
    let mut composer = Composer::new();
    let mut outs: Vec<ComposedSurface> = ids.iter().map(|_| ComposedSurface::new(0, 0)).collect();
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let mut ok = 0;
    let r = time(|| {
        ok = 0;
        for (id, out) in ids.iter().zip(outs.iter_mut()) {
            if composer
                .compose_into(out, &world, &atlas, *id, &binds, &pattern)
                .is_ok()
            {
                ok += 1;
            }
        }
    });
    report(
        "emo2_all",
        &format!("surfaces={} composed={ok}", ids.len()),
        r,
    );
}
