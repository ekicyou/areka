//! 折り返しと、描画範囲への内包を拡大率 1.0 と 1.25 で固定する。
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **12.5**・設計「Testing Strategy」の `wrapping.rs`・
//! 「描画範囲の解け方」の表・「文のファイル」の書体）。
//!
//! ## ここで固定するもの
//!
//! 本体側の面 0（`balloons0.png` 326×169）を本番の読み込み（面の系列解決→`descript.txt` に
//! `balloons0s.txt` を重ねたモデル）で解いた範囲と書体（`ＭＳ ゴシック` 12）で、本番と同じ
//! 折り返しの入口（[`LayoutEngine::layout`]・`staysee_balloon_fixture/wrapping.rs` と同じ）へ
//! 3 種類の本文を流し、行数・各行の字数・各行の上端と、すべてのグリフが範囲の内側に収まることを
//! 固定する。
//!
//! | 本文 | 字種 | 字数 | 期待する行の字数 | 1 行目の右端 |
//! |---|---|---|---|---|
//! | 全角だけ | 全角 12 px | 60 | 23・23・14 | 290 |
//! | 半角だけ | 半角 6 px | 120 | 47・47・26 | 296 |
//! | 混ぜた本文 | 全角と半角の交互（全角から） | 90 | 31・31・28 | 296 |
//!
//! ## 導き方（手で書いた値・道具のコードは参照しない）
//!
//! 範囲は左 0・右 326（`validrect.left,0`・`validrect.right,326`）、書き始めは origin の
//! x 14（`balloons0s.txt` の `origin.x,14`）、折り返し基準は 326 − 30 ＝ **296**
//! （`wordwrappoint.x,-30`）。折り返しは「行内位置＋次の字の送り幅 > 296」で起きるので、
//! 1 行に詰められる幅は 296 − 14 ＝ **282** である（遠辺 326 は基準より外なので効かない）。
//!
//! - 全角: 14 + 23×12 ＝ 290 で、次の 1 字が 302 > 296 → 1 行 **23** 字（60 ＝ 23＋23＋14）。
//! - 半角: 14 + 47×6 ＝ 296 で基準**ちょうど**（超えないので入る）、次が 302 > 296 → **47** 字
//!   （120 ＝ 47＋47＋26）。
//! - 混ぜた本文: 全角と半角の 1 組が 18。全角から始まる行は 15 組（270）＋全角 1 字で 282
//!   ちょうど → 31 字で右端 296、次の半角が 302。半角から始まる 2 行目は 15 組＋半角 1 字で
//!   276 → 31 字で右端 290、次の全角が 302。残りが 28 字（90 ＝ 31＋31＋28）。
//!
//! 行の上端は origin の y 11 から行送り 14（`font.height` 12 ＋行間 2）の等差で 11・25・39、
//! 行ボックスの丈は 12（`ＭＳ ゴシック` は `ascent + descent` がちょうど 1em）。3 行目の下端 51 は
//! 範囲の下辺 149（169 − 20）の内である。
//!
//! 相方側（`balloonk0.png` 326×96・範囲 0,0,326,82・origin (14,14)・基準 296）は高さだけが違う。
//! 混ぜた本文 1 本を流すと行の字数は本体側と同じ 31・31・28 で、上端 14・28・42、最後の下端 54 は
//! 下辺 82 の内——下の辺で止まらない（あふれの送りが起きない）ことまで見る。
//!
//! ## 拡大率（`staysee_balloon_fixture/scale.rs` と同じ形）
//!
//! k ＝ 1.0 と 1.25 で本番と同じ構築口 [`TextSlotBinding::new`] から枠の配置を組み、その
//! 画像の寸法で範囲を解いて流す。画像の座標での行の並びは k に依らず同じで、物理の寸法への変換
//! （[`ScaleContract`]）が k 倍になる。k ＝ 1.25 の手計算: 書き始め 14 → 17.5・半角の行の右端
//! 296 → 370・2 行目の上端 25 → 31.25・範囲の下辺 149 → 186.25・行ボックスの丈 12 →
//! `ceil(15.0)` ＝ 15。
//!
//! ## 空振りで緑にならないための備え
//!
//! 内包はグリフが 0 でも真になるので、行数とグリフの総数を先に固定してから 4 辺を言う。
//! 貪欲に詰めたこと（次の行の先頭を足すと基準を超える）は最終行で空回りするので、発火の回数
//! （本文 3 本 × 各 2 回 ＝ 6 回）を数えて固定する。拡大率のループも踏んだ本数を数える。
//!
//! ## 決定論
//!
//! ファイル読み込み・純粋層の解決・DirectWrite の計測だけ。実 GPU・実窓は使わない。

use areka_emo_present::balloon::{load_scope_balloon_model, resolve_balloon_faces};
use areka_emo_text::actor::{ResolvedBalloonText, TextSlotBinding};
use areka_emo_text::draw::DWriteMetrics;
use areka_emo_text::layout::{LayoutEngine, PositionedLine, WrapPlan};
use areka_emo_text::region::{ImagePx, ScaleContract, TextRegion};
use areka_emo_text::state::{TextItem, TextLayerConfig};
use areka_emo_text::writing::WritingMode;
use bevy_ecs::prelude::World;
use windows::Win32::Graphics::DirectWrite::DWRITE_FACTORY_TYPE_SHARED;
use wintf::com::dwrite::dwrite_create_factory;

use super::test_support::balloon_root;

// ── 期待値（設計「描画範囲の解け方」の表と書体から手で書いた値）──────────────────

/// 観測する拡大率（設計 `wrapping.rs`: 1.0 と 1.25）。
const SCALES: [f32; 2] = [1.0, 1.25];

/// scope ごとの面 0 の期待値。
struct ScopeExpectation {
    scope: u32,
    image_size: (u32, u32),
    /// 範囲（左・上・右・下・image px）。
    region: (f32, f32, f32, f32),
    /// 書き始め（origin の宣言どおり）。
    start: (f32, f32),
    /// 折り返し基準 x（326 − 30）。
    wrap_threshold: f32,
}

const SAKURA: ScopeExpectation = ScopeExpectation {
    scope: 0,
    image_size: (326, 169),
    region: (0.0, 0.0, 326.0, 149.0),
    start: (14.0, 11.0),
    wrap_threshold: 296.0,
};

const KERO: ScopeExpectation = ScopeExpectation {
    scope: 1,
    image_size: (326, 96),
    region: (0.0, 0.0, 326.0, 82.0),
    start: (14.0, 14.0),
    wrap_threshold: 296.0,
};

/// 文字の高さ（`font.height,12`）。
const FONT_HEIGHT: f32 = 12.0;
/// 半角・全角の送り幅（`ＭＳ ゴシック` 12 の 0.5em・1em）。
const ADVANCE_HALF: f32 = 6.0;
const ADVANCE_FULL: f32 = 12.0;
/// 行ボックスの丈と行送り（12 ＋行間 2）。
const LINE_BOX: f32 = 12.0;
const LINE_PITCH: f32 = 14.0;

/// 全角だけの本文（60 字）。
const FULL_WIDTH_BODY: &str = "あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ\
                               あいうえおかきくけこ";
/// 半角だけの本文（120 字）。
const HALF_WIDTH_BODY: &str = "abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij\
                               abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij";
/// 全角と半角を交互に並べた本文（90 字・偶数番目が全角）。
const MIXED_BODY: &str = "あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおeかfきgくhけiこj\
                          あaいbうcえdおe";

/// 本文・各行の字数・各行の右端（導き方はモジュールの説明）。
const BODIES: [(&str, &str, &[usize], &[f32]); 3] = [
    (
        "全角だけ",
        FULL_WIDTH_BODY,
        &[23, 23, 14],
        &[290.0, 290.0, 182.0],
    ),
    (
        "半角だけ",
        HALF_WIDTH_BODY,
        &[47, 47, 26],
        &[296.0, 296.0, 170.0],
    ),
    (
        "混ぜた本文",
        MIXED_BODY,
        &[31, 31, 28],
        &[296.0, 290.0, 266.0],
    ),
];

// ── 本番と同じ読み込みと折り返しの経路 ──────────────────────────────────────────

/// 本番の読み込み（面の系列解決→面 0 に `descript.txt` と面別の設定を重ねたモデル）で
/// 枠の配置の画像の寸法から範囲と書体を解く。
fn resolve_scope(exp: &ScopeExpectation, binding: &TextSlotBinding) -> ResolvedBalloonText {
    let root = balloon_root();
    let faces = resolve_balloon_faces(&root, exp.scope)
        .unwrap_or_else(|e| panic!("scope {} の面の系列解決が失敗した: {e}", exp.scope));
    let face0 = faces
        .iter()
        .find(|f| f.surface_id == 0)
        .unwrap_or_else(|| panic!("scope {}: 面 0 が解決されない", exp.scope));
    let model = load_scope_balloon_model(&root, exp.scope, face0);
    ResolvedBalloonText::resolve(&model, binding.image_size)
}

/// 面 0 を拡大率 k で置いたときの枠の配置を本番と同じ構築口で組む（`scale.rs` と同じ形）。
fn binding(world: &mut World, exp: &ScopeExpectation, k: f32) -> TextSlotBinding {
    let (w, h) = exp.image_size;
    let surface = ((w as f32 * k).ceil() as u32, (h as f32 * k).ceil() as u32);
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();
    TextSlotBinding::new(slot, window, k, surface, exp.image_size)
}

/// 本文を 1 字 1 グリフの追記列へ写す。
fn glyph_items(body: &str) -> Vec<TextItem> {
    areka_sakura::cluster::clusters(body)
        .map(TextItem::glyph)
        .collect()
}

/// 範囲が表のとおりであることを確かめてから、本文を全文可視で流す。
fn layout_body(
    exp: &ScopeExpectation,
    binding: &TextSlotBinding,
    body: &str,
) -> (TextRegion, Vec<PositionedLine>) {
    let resolved = resolve_scope(exp, binding);
    let region = resolved.region;
    let scope = exp.scope;
    let k = binding.scale;
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        exp.region,
        "scope {scope} k={k}: 範囲が設計の表と違う（物理の寸法が解決へ渡っている疑い）"
    );
    assert_eq!(region.start(), exp.start, "scope {scope} k={k}: 書き始め");
    assert_eq!(
        region.wrap_threshold(),
        exp.wrap_threshold,
        "scope {scope} k={k}: 折り返し基準"
    );
    assert_eq!(
        resolved.font.height, FONT_HEIGHT,
        "scope {scope}: 文字の高さ（以下の期待値はこの高さが前提）"
    );
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)
        .expect("DirectWrite factory を生成できる（文字の寸法の観測はこれが前提）");
    let metrics = DWriteMetrics::new(
        &factory,
        &resolved.font,
        resolved.mode,
        &TextLayerConfig::default(),
    )
    .expect("areka-balloon の書体で文字の寸法を組める");
    let items = glyph_items(body);
    let lines = LayoutEngine::layout(
        &items,
        items.len(),
        &region,
        resolved.mode,
        resolved.font.height,
        &metrics,
        WrapPlan::CharByChar,
    );
    (region, lines)
}

/// 行数とグリフの総数を先に固定し、すべてのグリフの矩形が範囲の内側にあることを言う。
fn assert_contained(label: &str, region: &TextRegion, lines: &[PositionedLine], total: usize) {
    let placed: usize = lines.iter().map(|l| l.glyphs.len()).sum();
    assert_eq!(
        placed, total,
        "{label}: 置かれたグリフが {placed} 個（流したのは {total} 個・内包は 0 個でも真になる）"
    );
    for (n, line) in lines.iter().enumerate() {
        for (i, g) in line.glyphs.iter().enumerate() {
            let at = format!("{label}: {} 行目 {} 字目 `{}`", n + 1, i + 1, g.text);
            let (l, r) = (g.inline_pos, g.inline_pos + g.advance);
            assert!(l >= region.left(), "{at} の左端 {l} が範囲の左辺の外");
            assert!(r <= region.right(), "{at} の右端 {r} が範囲の右辺の外");
            assert!(
                line.rect.top >= region.top(),
                "{at} の上端 {} が範囲の上辺の外",
                line.rect.top
            );
            assert!(
                line.rect.bottom <= region.bottom(),
                "{at} の下端 {} が範囲の下辺 {} の外",
                line.rect.bottom,
                region.bottom()
            );
        }
    }
}

// ── 本体側: 行数・字数・上端・右端と内包（拡大率 1.0 と 1.25）───────────────────

/// 3 本文の行の字数・上端・右端が導いた値どおりで、すべてのグリフが範囲の内側にある。
#[test]
fn sakura_bodies_wrap_into_the_derived_lines_and_stay_inside() {
    let mut world = World::new();
    let mut visited = 0usize;
    for k in SCALES {
        let b = binding(&mut world, &SAKURA, k);
        for (label, body, lens, ends) in BODIES {
            let label = format!("{label} k={k}");
            let (region, lines) = layout_body(&SAKURA, &b, body);
            let actual: Vec<usize> = lines.iter().map(|l| l.glyphs.len()).collect();
            assert_eq!(actual, lens, "{label}: 各行の字数");
            assert_contained(&label, &region, &lines, lens.iter().sum());

            for (n, line) in lines.iter().enumerate() {
                let top = SAKURA.start.1 + LINE_PITCH * n as f32;
                assert_eq!(line.rect.top, top, "{label}: {} 行目の上端", n + 1);
                assert_eq!(
                    line.rect.bottom - line.rect.top,
                    LINE_BOX,
                    "{label}: {} 行目の行ボックスの丈",
                    n + 1
                );
                let head = &line.glyphs[0];
                assert_eq!(
                    head.inline_pos,
                    SAKURA.start.0,
                    "{label}: {} 行目の書き始め",
                    n + 1
                );
                let tail = line.glyphs.last().expect("空の行は無い");
                assert_eq!(
                    tail.inline_pos + tail.advance,
                    ends[n],
                    "{label}: {} 行目の右端",
                    n + 1
                );
            }
        }
        visited += 1;
    }
    assert_eq!(visited, SCALES.len(), "踏んだ拡大率の本数");
}

/// 折り返しが貪欲であること——次の行の先頭の 1 字を足すと基準 296 を超える。
///
/// 送り幅も字種ごとに 6／12 であることを同じ走査で確かめる（字数の導出の前提）。
#[test]
fn sakura_wrapping_fills_each_line_up_to_the_threshold() {
    let mut world = World::new();
    let b = binding(&mut world, &SAKURA, 1.0);
    let mut checks = 0usize;
    for (label, body, _, _) in BODIES {
        let (_, lines) = layout_body(&SAKURA, &b, body);
        for (n, line) in lines.iter().enumerate() {
            for g in &line.glyphs {
                let want = if g.text.is_ascii() {
                    ADVANCE_HALF
                } else {
                    ADVANCE_FULL
                };
                assert_eq!(g.advance, want, "{label}: `{}` の送り幅", g.text);
            }
            let Some(next) = lines.get(n + 1).and_then(|l| l.glyphs.first()) else {
                continue;
            };
            let tail = line.glyphs.last().expect("空の行は無い");
            let end = tail.inline_pos + tail.advance;
            assert!(
                end + next.advance > SAKURA.wrap_threshold,
                "{label}: {} 行目は右端 {end} で、次の `{}` を足しても基準 {} を超えない（早すぎる折り返し）",
                n + 1,
                next.text,
                SAKURA.wrap_threshold
            );
            checks += 1;
        }
    }
    assert_eq!(checks, 6, "貪欲の検査の発火の回数（本文 3 本 × 各 2 回）");
}

// ── 拡大率: 画像の座標は同じ・物理の寸法は k 倍 ──────────────────────────────────

/// 拡大率 1.25 で画像の座標の行の並びが 1.0 と同じで、物理の寸法への変換が k 倍になる。
#[test]
fn scale_keeps_image_lines_and_multiplies_physical_sizes() {
    let mut world = World::new();
    let unit = binding(&mut world, &SAKURA, 1.0);
    let scaled = binding(&mut world, &SAKURA, 1.25);
    assert_eq!(scaled.scale, 1.25, "枠の配置が保つ k");
    let contract = ScaleContract::new(scaled.scale, None);

    for (label, body, _, _) in BODIES {
        let (_, base) = layout_body(&SAKURA, &unit, body);
        let (region, lines) = layout_body(&SAKURA, &scaled, body);
        assert_eq!(
            lines, base,
            "{label}: k=1.25 の画像の座標の行の並びが k=1.0 と違う"
        );
        assert_eq!(
            contract.to_physical(ImagePx(region.bottom())).0,
            186.25,
            "{label}: 範囲の下辺 149 の物理座標"
        );
        for line in &lines {
            for g in &line.glyphs {
                let l = contract.to_physical(ImagePx(g.inline_pos)).0;
                assert_eq!(l, g.inline_pos * 1.25, "{label}: `{}` の物理の左端", g.text);
            }
        }
    }

    // 手計算の値（モジュールの説明）。
    let (_, half) = layout_body(&SAKURA, &scaled, HALF_WIDTH_BODY);
    let tail = half[0].glyphs.last().expect("空の行は無い");
    assert_eq!(
        contract
            .to_physical(ImagePx(half[0].glyphs[0].inline_pos))
            .0,
        17.5,
        "書き始め 14 の物理座標"
    );
    assert_eq!(
        contract
            .to_physical(ImagePx(tail.inline_pos + tail.advance))
            .0,
        370.0,
        "半角の行の右端 296 の物理座標"
    );
    assert_eq!(
        contract.to_physical(ImagePx(half[1].rect.top)).0,
        31.25,
        "2 行目の上端 25 の物理座標"
    );
    assert_eq!(
        contract.physical_extent(ImagePx(half[0].rect.bottom - half[0].rect.top)),
        15,
        "行ボックスの丈 12 の物理寸法"
    );
}

// ── 相方側: 下の辺で止まらない長さの本文 1 本 ────────────────────────────────────

/// 相方側へ混ぜた本文 1 本を流すと、行の字数は本体側と同じで、上端は 14 から 14 刻み、
/// すべてのグリフが範囲の内側にあり、あふれの送りは起きない（拡大率 1.0 と 1.25）。
#[test]
fn kero_body_stays_inside_without_reaching_the_bottom() {
    let mut world = World::new();
    let mut visited = 0usize;
    for k in SCALES {
        let b = binding(&mut world, &KERO, k);
        let label = format!("相方側 k={k}");
        let (region, lines) = layout_body(&KERO, &b, MIXED_BODY);
        let actual: Vec<usize> = lines.iter().map(|l| l.glyphs.len()).collect();
        assert_eq!(actual, [31, 31, 28], "{label}: 各行の字数");
        let tops: Vec<f32> = lines.iter().map(|l| l.rect.top).collect();
        assert_eq!(tops, [14.0, 28.0, 42.0], "{label}: 各行の上端");
        assert_contained(&label, &region, &lines, 90);

        let window = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
        assert_eq!(
            (window.first_visible_line, window.block_offset),
            (0, 0.0),
            "{label}: 下の辺で止まった（あふれの送りが起きた）"
        );
        visited += 1;
    }
    assert_eq!(visited, SCALES.len(), "踏んだ拡大率の本数");
}
