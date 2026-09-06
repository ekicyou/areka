//! # kero_menu_capacity_test — 実物バルーン × メニュー 3 台本の容量テスト（task 5.1）
//!
//! 出典 spec: `areka-P0-emo-text-line-height-canon`（要件 **8.1**／**8.2**／**8.4 (a)(c)**／
//! **8.7**／**5.1**〜**5.4**／**6.5**／**3.4**／**4.1**／**4.2**・design.md §4.1 正典表・§4.3）。
//!
//! ## この檻が塞ぐ穴
//!
//! 相方（エモ）側バルーンのダブルクリックメニューで**先頭の選択肢が描かれない**という実機症状は、
//! 行送りが旧式の `ceil(font.height × 1.25)` ＝ 35 で 3 行目の下端（138）が文字描画範囲の下端（133）を
//! 5px あふれ、あふれ判定が 1 行ぶんスクロールを返していたことによる。行送りを正典
//! （`font.height + 行間` ＝ 30）へ直した後、**その症状が二度と起きないこと**を実物の資産で固定する。
//!
//! ## 実物の経路をそのまま通す（in-code の作り物を使わない）
//!
//! | 段 | 使うもの |
//! |---|---|
//! | バルーン記述 | 実 `emo2-kakukaku/descript.txt`（基層）＋`balloonk0s.txt`（面別上書き層）の 2 層マージ |
//! | 領域解決 | `ResolvedBalloonText::resolve`（本番 `present_frame` と同じ入口）を 288×203 で |
//! | 台本 | 実 `ghost/master/dic/menu.pasta` の選択肢 3 台本の本文（`＊メインメニュー選択肢`／
//!   `＊おしゃべり頻度メニュー選択肢`／`＊エモの位置調整選択肢` の見出し直後の行） |
//! | 台本 → cue | 実 `areka_parsers::sakura::parse` → 実 `areka_sakura::compile` |
//! | cue → 状態 | 実 `TextLayerState::apply_cue` |
//! | 配置 | 実 `LayoutEngine::layout`（送り幅は実フォント `Yu Gothic UI` の `DWriteMetrics`） |
//! | あふれ判定 | 実 `LayoutEngine::visible_window` |
//!
//! GPU も実窓も要らない（DirectWrite の factory だけを使う）。
//!
//! ## 実物のジオメトリ（`shipped_fixture_region_test.rs` が別途固定している値）
//!
//! - 文字描画範囲 (24,40)-(240,133)＝幅 216・高さ 93（`balloonk0s.txt` の
//!   `top,40`／`bottom,-70`／`left,24`／`right,-48` を 288×203 へ解決）。
//! - 折返し基準（`wordwrappoint.x`）は基層の `-34` を継いで **254**——描画範囲の右端 **240** の
//!   **外**である（バルーン定義側の粗さ・design §4.3）。ゆえに実効の折返し位置は 240。
//! - フォントは `Yu Gothic UI`・`font.height,28` → 行送り **30**（28 ＋ 行間 2）。
//!
//! ## 行の高さの導出（本ファイルの期待値の根）
//!
//! 行の矩形は「上端＝行送り軸の位置・下端＝上端＋`font.height`」（`layout.rs` の `finish_line`）。
//! 先頭行の上端は書字開始点の 40 ゆえ、行の上端は 40 → 70 → 100、下端は 68 → 98 → **128**。
//! 3 行目の下端 128 は描画範囲の下端 133 の内にあり、あふれ判定は発火しない（先頭可視行 0）。
//! 旧式（`ceil(28 × 1.25)` ＝ 35）だと 3 行目の上端は 40 ＋ 2 × 35 ＝ 110・下端 138 > 133 で
//! あふれる——それが対照テスト（`legacy_pitch_metrics_reproduces_the_dropped_first_line`）である。

use std::path::{Path, PathBuf};

use areka_emo_text::actor::ResolvedBalloonText;
use areka_emo_text::draw::DWriteMetrics;
use areka_emo_text::layout::{GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};
use areka_emo_text::region::TextRegion;
use areka_emo_text::segment::{SegmentPlan, segment_plan};
use areka_emo_text::state::{TextItem, TextLayerConfig, TextLayerState};
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_sakura::compile;
use areka_sakura::contract::{ActorKey, CuePayload, SystemVarSnapshot, TalkCue};
use windows::Win32::Graphics::DirectWrite::{DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory2};
use wintf::com::dwrite::dwrite_create_factory;

// ══ 実物の所在と読み込み（本番と同じ規約） ══════════════════════════════════════════════

/// 実 emo2 fixture ルート（`crates/pilot/examples/shiori-host-32/fixtures/emo2/`）。
fn emo2_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// 実 fixture ファイルの生バイト列を読む。
///
/// 読めなかったときに「対象 0 件だから緑」にならないよう、失敗と空は明示的に panic する。
fn read_fixture_bytes(rel: &str) -> Vec<u8> {
    let mut path: PathBuf = emo2_fixture_root();
    for seg in rel.split('/') {
        path = path.join(seg);
    }
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "実 emo2 fixture {} の読取に失敗した（本檻はこのファイルの実在が前提）: {e}",
            path.display()
        )
    });
    assert!(
        !bytes.is_empty(),
        "実 emo2 fixture {} が空である",
        path.display()
    );
    bytes
}

/// バルーン記述ファイルを本番と同じ規約で読む。
///
/// 本番経路 `areka_emo_present::balloon::load_scope_balloon_model` は
/// `decode(&bytes, DefaultEncoding::Ansi)`（**既定 Ansi・ファイル内の `charset` 宣言優先**）で
/// 読む。`emo2-kakukaku/descript.txt` は `charset,UTF-8` を宣言しており、面別上書き層は
/// 宣言を持たないが純 ASCII ゆえどちらの既定でも同一である。
fn read_balloon_layer(rel: &str) -> String {
    decode(&read_fixture_bytes(rel), DefaultEncoding::Ansi)
}

/// pasta 辞書を読む。
///
/// 辞書ファイルは `charset` 宣言を持たない（宣言はゴーストの `master/descript.txt` の
/// `charset,UTF-8` が担う）ので、既定を UTF-8 として読む（`emo2_fixture_e2e_test.rs` と同じ規約）。
fn read_pasta(rel: &str) -> String {
    decode(&read_fixture_bytes(rel), DefaultEncoding::Utf8)
}

/// 相方（エモ）側バルーンの面別上書き層（scope 1・`balloonk0.png` 288×203 向け）。
const KERO_OVERLAY: &str = "balloonk0s.txt";
/// 本体（さくら）側バルーンの面別上書き層（scope 0・`balloons0.png` 400×224 向け）。
const SAKURA_OVERLAY: &str = "balloons0s.txt";
/// 相方側バルーン画像の原寸（image px）。
const KERO_IMAGE_SIZE: (u32, u32) = (288, 203);
/// 本体側バルーン画像の原寸（image px）。
const SAKURA_IMAGE_SIZE: (u32, u32) = (400, 224);

/// 相方側の文字描画範囲の下端（image px）＝あふれ判定の境界。
const KERO_BOTTOM: f32 = 133.0;
/// 相方側の文字描画範囲の右端（image px）＝行内軸の絶対上限（hard）。
const KERO_RIGHT: f32 = 240.0;
/// 相方側の折返し基準（image px）＝基層の `wordwrappoint.x,-34` 由来。描画範囲の右端の**外**。
const KERO_SOFT: f32 = 254.0;
/// 実物の `font.height`（image px）。
const FONT_HEIGHT: f32 = 28.0;
/// 正典の行送りピッチ（`font.height + 行間 2`・design §4.1）。
const PITCH: f32 = 30.0;
/// 書字開始点（相方側・image px）。
const KERO_START: (f32, f32) = (24.0, 40.0);

/// 2 層（`descript.txt` 基層＋面別上書き層）をマージした `BalloonModel`（本番と同じ `parse_str`）。
fn merged_model(overlay: &str) -> BalloonModel {
    parse_str(
        &read_balloon_layer("emo2-kakukaku/descript.txt"),
        Some(&read_balloon_layer(&format!("emo2-kakukaku/{overlay}"))),
    )
}

// ══ 実台本（menu.pasta の 3 本の選択肢台本） ═════════════════════════════════════════════

/// 実 `menu.pasta` の選択肢 3 台本を、**見出しの語**で指す（行番号で指さない）。
///
/// pasta の見出しは `＊<名前>` の行で、本文はその直後の
/// `　　　エモ：＠通常　<さくらスクリプト>` 行である。話者接頭辞は最初の ASCII `\`
/// （さくらスクリプトの開始）より前ゆえ、最初の `\` 以降が純さくらスクリプト断片になる。
const MENU_HEADINGS: [&str; 3] = [
    "＊メインメニュー選択肢",
    "＊おしゃべり頻度メニュー選択肢",
    "＊エモの位置調整選択肢",
];

/// 見出しで指した台本の本文（純さくらスクリプト断片）を実 `menu.pasta` から取り出す。
fn menu_script(heading: &str) -> String {
    let pasta = read_pasta("ghost/master/dic/menu.pasta");
    let mut lines = pasta.lines();
    lines
        .find(|l| l.trim_end() == heading)
        .unwrap_or_else(|| panic!("menu.pasta に見出し {heading} が在る"));
    let body = lines
        .find(|l| l.contains('\\'))
        .unwrap_or_else(|| panic!("見出し {heading} の直後にさくらスクリプトを含む本文行が在る"));
    let start = body
        .find('\\')
        .expect("本文行はさくらスクリプト（\\）を含む");
    body[start..].to_string()
}

/// 実 sakura パイプライン（`parse` → `compile`）で台本を cue 列へ落とし、
/// 文字状態機械が消費する Command cue だけを配送エンベロープへ無変形複写する。
fn command_cues(script: &str) -> Vec<TalkCue> {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = compile(&instructions, &SystemVarSnapshot::default());
    compiled
        .sheet
        .cues()
        .iter()
        .filter_map(|c| match &c.payload {
            CuePayload::Command(cmd) => Some(TalkCue {
                at: c.start_time,
                actor: c.actor.clone(),
                command: cmd.clone(),
                duration: c.duration,
            }),
            _ => None,
        })
        .collect()
}

/// cue 列を実 `TextLayerState` へ載せ、（状態, 唯一の actor）を返す。
fn state_of(cues: &[TalkCue]) -> (TextLayerState, ActorKey) {
    let mut state = TextLayerState::default();
    for cue in cues {
        state.apply_cue(cue);
    }
    let actors: Vec<ActorKey> = state.actors().map(|(k, _)| k.clone()).collect();
    assert_eq!(
        actors.len(),
        1,
        "選択肢台本は 1 スコープぶんの cue 列である: {actors:?}"
    );
    (state, actors[0].clone())
}

// ══ metrics（実フォント）と、実フォント不在の検出 ═══════════════════════════════════════

/// DirectWrite factory（GPU 不要——計測に要るのは factory だけ）。
fn factory() -> IDWriteFactory2 {
    dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DirectWrite factory を生成できる")
}

/// 実フォントの送り幅が縮退していないことを先頭で確かめる（design「新規の決定論テスト」注記）。
///
/// `Yu Gothic UI` はプロポーショナル——仮名の送りは em（28）より**狭い**。この環境に当該フォントが
/// 無いと DirectWrite が等幅の代替（全角＝em）へ落ち、本ファイルの数値の前提（送り ≈ 23）が
/// 崩れたまま緑になり得る。ゆえに縮退を検出したら**赤で止める**。
fn assert_real_font_present(metrics: &dyn GlyphMetrics) {
    let a = metrics.advance('あ', FONT_HEIGHT);
    assert!(
        a < FONT_HEIGHT,
        "実フォント Yu Gothic UI が見つからない（「あ」の送りが {a} ＝ em {FONT_HEIGHT} 以上の等幅値へ縮退している）。\
         本ファイルの期待値はプロポーショナルな実フォントの実測を前提にしているので、\
         代替フォントのまま緑にしない"
    );
}

/// 実物のバルーン記述から解決した metrics（実フォント・正典の行間）。
fn resolved_metrics(factory: &IDWriteFactory2, resolved: &ResolvedBalloonText) -> DWriteMetrics {
    DWriteMetrics::new(
        factory,
        &resolved.font,
        resolved.mode,
        &TextLayerConfig::default(),
    )
    .expect("実 descript のフォントで DWriteMetrics を生成できる")
}

/// 行送りだけを**旧式**（`ceil(font_height × 1.25)`）へ戻すテスト専用の metrics（要件 8.7 の対照）。
///
/// 送り幅・行ボックス丈は実測へそのまま委ねる——差し替えるのは行送りピッチ 1 点だけであり、
/// 「先頭の選択肢が描かれない」症状が行送りの式だけから来ていたことを示す。
/// 製品コードには旧式の口を残さない（design DD-11）。
struct LegacyPitchMetrics<'a> {
    real: &'a DWriteMetrics,
}

impl GlyphMetrics for LegacyPitchMetrics<'_> {
    fn advance(&self, ch: char, font_height: f32) -> f32 {
        self.real.advance(ch, font_height)
    }

    fn line_pitch(&self, font_height: f32) -> f32 {
        (font_height * 1.25).ceil()
    }

    fn line_box_height(&self, font_height: f32) -> f32 {
        self.real.line_box_height(font_height)
    }
}

// ══ 配置（実経路）と観測ヘルパ ═════════════════════════════════════════════════════════

/// 折返し方式の 2 通り（要件 8.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WrapKind {
    /// 1 文字ずつ（`WrapPlan::CharByChar`）。
    CharByChar,
    /// budoux による分節（`WrapPlan::Segmented`）。
    Segmented,
}

/// 実台本を実経路で配置し、行列を返す（全グリフ可視）。
fn layout_script(
    state: &TextLayerState,
    actor: &ActorKey,
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
    metrics: &dyn GlyphMetrics,
    kind: WrapKind,
) -> Vec<PositionedLine> {
    let items = state
        .actor_state(actor)
        .expect("apply_cue 済みの actor 状態が在る")
        .items();
    // 全グリフ可視（リビール完了後の定常状態＝メニューが読める状態）。
    let visible = items
        .iter()
        .filter(|it| matches!(it, TextItem::Glyph { .. }))
        .count();
    let plan: SegmentPlan;
    let wrap = match kind {
        WrapKind::CharByChar => WrapPlan::CharByChar,
        WrapKind::Segmented => {
            plan = segment_plan(items);
            WrapPlan::Segmented(&plan)
        }
    };
    LayoutEngine::layout(items, visible, region, mode, font_height, metrics, wrap)
}

/// 行列を「グリフ通し番号 → 行 index」へ写す。
///
/// 配置はグリフを追記順にそのまま並べる（可視 prefix を落とすだけで並べ替えない）ため、
/// 行を跨いで数えた i 番目のグリフが、`ChoiceSpan::glyph_range` の序数 i に対応する。
fn glyph_line_index(lines: &[PositionedLine]) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        out.extend(std::iter::repeat_n(i, line.glyphs.len()));
    }
    out
}

/// 行の上端の列（image px）。
fn line_tops(lines: &[PositionedLine]) -> Vec<f32> {
    lines.iter().map(|l| l.rect.top).collect()
}

/// 行の下端の列（image px）。
fn line_bottoms(lines: &[PositionedLine]) -> Vec<f32> {
    lines.iter().map(|l| l.rect.bottom).collect()
}

// ══ テスト 1: 3 台本 × 2 方式で全選択肢が収まる（8.1／8.2／5.1〜5.4） ══════════════════

/// 実物の相方側バルーンで、実 `menu.pasta` の選択肢 3 台本すべてが
/// (a) スクロールを起こさず（先頭可視行 0）、(b) どの選択肢の行も下端が描画範囲の下端 133 を
/// 超えず、(c) 折返し方式 2 通りで同一の結果になることを固定する。
///
/// 行の上端／下端の期待値（design §4.1 の行送り 30 からの導出）:
///
/// | 台本 | 行の上端 | 行の下端 | 備考 |
/// |---|---|---|---|
/// | メインメニュー | 40 / 70 / 100 | 68 / 98 / **128** | 3 項目・`\_l[5em,2lh]` の「閉じる」が 3 行目 |
/// | おしゃべり頻度 | 40 / 70 / 100 / 100 | 68 / 98 / 128 / 128 | 4 項目 3 段——「もどる」は「たまーに」と同じ段の右 |
/// | 位置調整 | 40 / 100 | 68 / 128 | 2 項目・2 段目（上端 70）は文字が無く**行を作らない** |
#[test]
fn three_menu_scripts_fit_without_scrolling_in_both_wrap_plans() {
    let factory = factory();
    let model = merged_model(KERO_OVERLAY);
    let resolved = ResolvedBalloonText::resolve(&model, KERO_IMAGE_SIZE);
    let metrics = resolved_metrics(&factory, &resolved);
    assert_real_font_present(&metrics);

    // 前提の実物ジオメトリ（値が変われば以下の期待値の導出ごと崩れるので先に固定する）。
    assert_eq!(resolved.mode, WritingMode::HorizontalTb, "相方側は横書き");
    assert_eq!(
        resolved.font.height, FONT_HEIGHT,
        "実 descript の font.height"
    );
    assert_eq!(
        (
            resolved.region.left(),
            resolved.region.top(),
            resolved.region.right(),
            resolved.region.bottom()
        ),
        (KERO_START.0, KERO_START.1, KERO_RIGHT, KERO_BOTTOM),
        "相方側の文字描画範囲 (24,40)-(240,133)"
    );
    assert_eq!(
        resolved.region.wrap_threshold(),
        KERO_SOFT,
        "折返し基準は基層の wordwrappoint.x,-34 由来の 254（描画範囲の右端の外）"
    );
    assert_eq!(
        resolved.region.inline_limit(),
        KERO_RIGHT,
        "行内軸の絶対上限は描画範囲の右端 240"
    );
    assert_eq!(
        metrics.line_pitch(FONT_HEIGHT),
        PITCH,
        "行送りは 28 + 2 = 30"
    );

    // 台本ごとの期待（行の上端・選択肢の数）。
    let expected: [(&str, &[f32], usize); 3] = [
        (MENU_HEADINGS[0], &[40.0, 70.0, 100.0], 3),
        (MENU_HEADINGS[1], &[40.0, 70.0, 100.0, 100.0], 4),
        (MENU_HEADINGS[2], &[40.0, 100.0], 2),
    ];

    for (heading, want_tops, want_choices) in expected {
        let cues = command_cues(&menu_script(heading));
        let (state, actor) = state_of(&cues);
        let choices = state
            .actor_state(&actor)
            .expect("状態が在る")
            .choices()
            .to_vec();
        assert_eq!(choices.len(), want_choices, "{heading}: 実台本の選択肢の数");

        let mut both: Vec<Vec<PositionedLine>> = Vec::new();
        for kind in [WrapKind::CharByChar, WrapKind::Segmented] {
            let lines = layout_script(
                &state,
                &actor,
                &resolved.region,
                resolved.mode,
                resolved.font.height,
                &metrics,
                kind,
            );
            let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);

            // (a) スクロールしない（要件 5.1〜5.3・8.1）。
            assert_eq!(
                window.first_visible_line,
                0,
                "{heading}／{kind:?}: 先頭可視行が 0 でない＝先頭の選択肢が描かれない症状の再来。行の下端={:?}",
                line_bottoms(&lines)
            );
            assert_eq!(
                window.block_offset, 0.0,
                "{heading}／{kind:?}: スクロールしないのでオフセットは 0"
            );

            // 行の数と上端が正典の行送り（30）どおりに並ぶ。
            assert_eq!(
                line_tops(&lines),
                want_tops.to_vec(),
                "{heading}／{kind:?}: 行の上端（先頭 40・以降 +30・カーソル指定は 40+2×30=100）"
            );
            // 行の下端＝上端＋font.height（`layout.rs` の `finish_line`）——68 / 98 / 128。
            assert_eq!(
                line_bottoms(&lines),
                want_tops
                    .iter()
                    .map(|t| t + FONT_HEIGHT)
                    .collect::<Vec<f32>>(),
                "{heading}／{kind:?}: 行の下端（上端＋28）"
            );

            // (b) すべての選択肢の行の下端が描画範囲の下端の内（要件 5.4）。
            let index_of = glyph_line_index(&lines);
            for span in &choices {
                assert!(
                    !span.glyph_range.is_empty(),
                    "{heading}: 選択肢 {} は文字を持つ",
                    span.label
                );
                for g in span.glyph_range.clone() {
                    let line = &lines[index_of[g]];
                    assert!(
                        line.rect.bottom <= KERO_BOTTOM,
                        "{heading}／{kind:?}: 選択肢「{}」の行の下端 {} が描画範囲の下端 {KERO_BOTTOM} を超える",
                        span.label,
                        line.rect.bottom
                    );
                }
            }
            both.push(lines);
        }

        // (c) 折返し方式 2 通りで結果が同一（要件 8.2）。
        assert_eq!(
            both[0], both[1],
            "{heading}: 1 文字ずつ／分節の折返しで行の配置が食い違う"
        );
    }
}

// ══ テスト 2: `\_l[5em,2lh]` の着地（4.1／4.2） ═════════════════════════════════════════

/// 実台本の `\_l[5em,2lh]` で置いた選択肢が、
/// (a) 行送り軸は「改行 2 回で送った段」と同じ高さ（40 ＋ 2 × 30 ＝ 100）に、
/// (b) 行内軸は `font.height` の 5 倍（24 ＋ 5 × 28 ＝ 164）に着地することを、
/// 同じ台本の実経路で確かめる。
///
/// (b) は本仕様の前後で変わらない値である（`em` の係数は `font.height` のまま・要件 4.2）。
/// (a) だけが新しい行送りへ追随する（要件 4.1）。
#[test]
fn cursor_tag_lands_on_the_same_row_as_two_newlines_and_keeps_the_em_column() {
    let factory = factory();
    let model = merged_model(KERO_OVERLAY);
    let resolved = ResolvedBalloonText::resolve(&model, KERO_IMAGE_SIZE);
    let metrics = resolved_metrics(&factory, &resolved);
    assert_real_font_present(&metrics);

    let script = menu_script(MENU_HEADINGS[0]);
    assert!(
        script.contains("\\_l[5em,2lh]"),
        "実台本は \\_l[5em,2lh] を含む: {script}"
    );
    let cues = command_cues(&script);
    let (state, actor) = state_of(&cues);
    let lines = layout_script(
        &state,
        &actor,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &metrics,
        WrapKind::CharByChar,
    );

    // 「閉じる」の行を選択肢スパンから引く（in-code の行番号決め打ちをしない）。
    let index_of = glyph_line_index(&lines);
    let close = state
        .actor_state(&actor)
        .expect("状態が在る")
        .choices()
        .iter()
        .find(|s| s.label == "閉じる")
        .expect("実台本に「閉じる」が在る")
        .clone();
    let close_line = &lines[index_of[close.glyph_range.start]];

    // (a) 行送り軸: 改行 2 回で送った段と同じ高さ。改行 2 回ぶんの高さは
    //     書字開始点 40 ＋ 2 × 行送り 30 ＝ 100（旧式 35 なら 110 で赤）。
    let two_newlines_top = KERO_START.1 + 2.0 * metrics.line_pitch(FONT_HEIGHT);
    assert_eq!(
        two_newlines_top, 100.0,
        "改行 2 回ぶんの段の上端（40 + 2 × 30）"
    );
    assert_eq!(
        close_line.rect.top, two_newlines_top,
        "\\_l[…,2lh] の「閉じる」は改行 2 回で送った段と同じ高さに着地する"
    );
    assert_eq!(
        close_line.rect.bottom,
        two_newlines_top + FONT_HEIGHT,
        "行の下端は上端＋font.height（128 ≤ 133）"
    );

    // (b) 行内軸: font.height の 5 倍（em の係数は不変）。
    let em_column = KERO_START.0 + 5.0 * FONT_HEIGHT;
    assert_eq!(em_column, 164.0, "5em の桁（24 + 5 × 28）");
    assert_eq!(
        close_line.glyphs[0].inline_pos, em_column,
        "\\_l[5em,…] の行内位置は従来と同じ 164（em の係数は font.height のまま）"
    );
    assert_eq!(
        close_line.rect.left, em_column,
        "行の矩形の近端も字下げを反映する（描画とヒットの整合）"
    );
}

// ══ テスト 3: 「閉じる」「もどる」が描画範囲の右端の内に収まる（8.4(a)／6.5／3.4） ═══════

/// 折返し基準（254）が描画範囲の右端（240）の**外**にある実物のバルーンで、
/// `\_l[5em,…]`（x164 起点）に置かれた「閉じる」「もどる」の**全グリフの遠端**が
/// 描画範囲の右端 240 の内に収まり、行が増えない（折り返されない）ことを固定する。
///
/// brief の見込み「x164..248（1 文字 28px）」は**全角＝em という机上の仮定**による値だった。
/// `Yu Gothic UI` はプロポーショナルで仮名の送りは em より狭い。実測（2026-09-06・本ファイル）:
///
/// | 選択肢 | 送りの内訳 | 3 文字の合計 | 遠端（起点 164） |
/// |---|---|---|---|
/// | 「閉じる」 | 閉 28.00 ＋ じ 19.88 ＋ る 21.34 | 69.22 | **233.22** ≤ 240 |
/// | 「もどる」 | も 21.34 ＋ ど 20.22 ＋ る 21.34 | 62.90 | **226.90** ≤ 240 |
///
/// 送りの実測値そのものは環境のフォント版で動きうるので、値を逐語で固定はせず、
/// 収まり（240 の内）と「縮退した等幅値ではない」（200 より遠い）の 2 つの境で挟む。
#[test]
fn close_and_back_choices_stay_inside_the_drawing_range_without_wrapping() {
    let factory = factory();
    let model = merged_model(KERO_OVERLAY);
    let resolved = ResolvedBalloonText::resolve(&model, KERO_IMAGE_SIZE);
    let metrics = resolved_metrics(&factory, &resolved);
    assert_real_font_present(&metrics);

    // 机上値がなぜ外れたか（要件 6.5 の記録）: 全角＝em を仮定すると 3 文字で 84px ＝ 遠端 248 に
    // なり描画範囲の右端 240 を超える。実フォントの送りは em より狭いので実際には超えない。
    assert!(
        KERO_START.0 + 5.0 * FONT_HEIGHT + 3.0 * FONT_HEIGHT > KERO_RIGHT,
        "brief の机上値（全角＝em の仮定）なら 164 + 3 × 28 = 248 で右端 240 を超えていた"
    );

    // 「閉じる」＝メインメニュー・「もどる」＝おしゃべり頻度／位置調整の 2 台本に在る。
    let targets: [(&str, &str); 3] = [
        (MENU_HEADINGS[0], "閉じる"),
        (MENU_HEADINGS[1], "もどる"),
        (MENU_HEADINGS[2], "もどる"),
    ];

    for (heading, label) in targets {
        let cues = command_cues(&menu_script(heading));
        let (state, actor) = state_of(&cues);
        for kind in [WrapKind::CharByChar, WrapKind::Segmented] {
            let lines = layout_script(
                &state,
                &actor,
                &resolved.region,
                resolved.mode,
                resolved.font.height,
                &metrics,
                kind,
            );
            let index_of = glyph_line_index(&lines);
            let span = state
                .actor_state(&actor)
                .expect("状態が在る")
                .choices()
                .iter()
                .find(|s| s.label == label)
                .unwrap_or_else(|| panic!("{heading} に「{label}」が在る"))
                .clone();

            // 折り返されない＝当該選択肢の全グリフが 1 つの行に在る。
            let rows: Vec<usize> = span.glyph_range.clone().map(|g| index_of[g]).collect();
            assert!(
                rows.windows(2).all(|w| w[0] == w[1]),
                "{heading}／{kind:?}: 「{label}」が折り返されて複数の行に割れた: rows={rows:?}"
            );

            // 全グリフの遠端が描画範囲の右端の内。
            let line = &lines[rows[0]];
            let base = first_glyph_of(&lines, rows[0]);
            let far = span
                .glyph_range
                .clone()
                .map(|g| {
                    let glyph = &line.glyphs[g - base];
                    glyph.inline_pos + glyph.advance
                })
                .fold(f32::MIN, f32::max);
            assert!(
                far <= KERO_RIGHT,
                "{heading}／{kind:?}: 「{label}」の遠端 {far} が描画範囲の右端 {KERO_RIGHT} を超える"
            );
            assert!(
                far > 200.0,
                "{heading}／{kind:?}: 「{label}」の遠端 {far} が小さすぎる＝送りが縮退している疑い（実測の見込みは ≈ 233）"
            );
        }
    }
}

/// 行 `index` の先頭グリフの通し番号（グリフ通し番号 → 行内 index の変換に使う）。
fn first_glyph_of(lines: &[PositionedLine], index: usize) -> usize {
    lines[..index].iter().map(|l| l.glyphs.len()).sum()
}

// ══ テスト 4: 本体側の折返し位置は描画範囲の判定を持たない参照と一致する（8.4(c)／6.4） ══

/// 本体（さくら）側バルーン `balloons0s.txt` は折返し基準（351）を描画範囲の右端（356）の
/// **内**に自ら上書きしている。この場合、行内軸の絶対上限（hard）の判定は一度も発火せず、
/// 折返しの位置は「折返し基準だけを見る参照」と一致しなければならない（本仕様の前後で
/// 折返しの位置を変えない・要件 6.4）。
///
/// 参照は本ファイルの中に置いた素朴な実装（送りを足し込み、基準を超えたら折る）である。
///
/// この検査が空振りでないことの較正（2026-09-06・本ファイルで実施）: 参照の基準を折返し基準
/// 351 から描画範囲の右端 356 へ差し替えると、参照の折返し位置は [14, 28] になり実配置の
/// [13, 26, 39] と食い違って**赤になる**。すなわち本検査は「折返しが基準 351 に従っている」ことを
/// 実際に区別している（両者が同じ値になるような緩い比較ではない）。
#[test]
fn sakura_side_wrap_positions_match_a_soft_only_reference() {
    let factory = factory();
    let model = merged_model(SAKURA_OVERLAY);
    let resolved = ResolvedBalloonText::resolve(&model, SAKURA_IMAGE_SIZE);
    let metrics = resolved_metrics(&factory, &resolved);
    assert_real_font_present(&metrics);

    let soft = resolved.region.wrap_threshold();
    let hard = resolved.region.inline_limit();
    assert_eq!(
        (soft, hard),
        (351.0, 356.0),
        "本体側は折返し基準 351 を描画範囲の右端 356 の内に自ら上書きしている"
    );

    // 折返し基準を確実に何度も超える長さの 1 行（40 文字・改行もカーソル指定も無い）。
    // 「あ」の送りは実測 22.85 ゆえ 1 行に 13 文字（36 ＋ 13 × 22.85 ＝ 333.0・
    // 14 文字目の遠端 355.9 > 351）で、40 文字なら折返しが 3 度起きる。
    let text = "あ".repeat(40);
    let mut state = TextLayerState::default();
    let actor = ActorKey::from("0");
    state.apply_cue(&TalkCue {
        at: 0.0,
        actor: actor.clone(),
        command: areka_sakura::contract::CueCommand::Text(text.clone()),
        duration: 0.0,
    });

    for kind in [WrapKind::CharByChar, WrapKind::Segmented] {
        let lines = layout_script(
            &state,
            &actor,
            &resolved.region,
            resolved.mode,
            resolved.font.height,
            &metrics,
            kind,
        );
        // 実際に折り返されていること（折返しが 1 度も起きないと本検査は空振りになる）。
        assert!(
            lines.len() >= 3,
            "{kind:?}: 40 文字は本体側の行幅を超えて何度も折り返されるはず: 行数 {}",
            lines.len()
        );

        // 参照: 折返し基準だけを見る素朴な実装（描画範囲の判定を持たない）。
        let mut want_breaks: Vec<usize> = Vec::new();
        let mut pos = resolved.region.start().0;
        let mut count_in_line = 0usize;
        for (i, ch) in text.chars().enumerate() {
            let adv = metrics.advance(ch, resolved.font.height);
            if count_in_line > 0 && pos + adv > soft {
                want_breaks.push(i);
                pos = resolved.region.start().0;
                count_in_line = 0;
            }
            pos += adv;
            count_in_line += 1;
        }

        // 実配置の折返し位置（各行の先頭グリフの通し番号・先頭行を除く）。
        let mut got_breaks: Vec<usize> = Vec::new();
        let mut acc = 0usize;
        for line in &lines {
            if acc > 0 {
                got_breaks.push(acc);
            }
            acc += line.glyphs.len();
        }
        assert_eq!(
            got_breaks, want_breaks,
            "{kind:?}: 本体側（折返し基準が描画範囲の内）の折返し位置が、描画範囲の判定を持たない参照と食い違う"
        );
    }
}

// ══ テスト 5: 旧式の行送りへ戻すと症状が再現する（8.7 の対照） ═══════════════════════════

/// 判定が生きていることの対照。行送りだけを旧式（`ceil(28 × 1.25)` ＝ 35）へ戻すと、
/// メインメニューの 3 行目（上端 40 ＋ 2 × 35 ＝ 110・下端 138）が描画範囲の下端 133 を
/// 超え、あふれ判定が 1 行ぶんのスクロールを返す＝**先頭の選択肢が描かれなくなる**。
///
/// これが本ファイルの容量テストの赤くなり方であり、同じ入力・同じ経路で行送りの値 1 点だけが
/// 違う。旧式の口は製品コードに残さない（テスト専用の metrics で注入する）。
#[test]
fn legacy_pitch_metrics_reproduces_the_dropped_first_line() {
    let factory = factory();
    let model = merged_model(KERO_OVERLAY);
    let resolved = ResolvedBalloonText::resolve(&model, KERO_IMAGE_SIZE);
    let real = resolved_metrics(&factory, &resolved);
    assert_real_font_present(&real);
    let legacy = LegacyPitchMetrics { real: &real };
    assert_eq!(
        legacy.line_pitch(FONT_HEIGHT),
        35.0,
        "旧式の行送りは ceil(28 × 1.25) = 35"
    );

    let cues = command_cues(&menu_script(MENU_HEADINGS[0]));
    let (state, actor) = state_of(&cues);
    let lines = layout_script(
        &state,
        &actor,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &legacy,
        WrapKind::CharByChar,
    );
    assert_eq!(
        line_tops(&lines),
        vec![40.0, 75.0, 110.0],
        "旧式では 3 行目の上端が 110（40 + 2 × 35）"
    );
    assert_eq!(
        line_bottoms(&lines)[2],
        138.0,
        "旧式では 3 行目の下端が 138 で描画範囲の下端 133 を 5px 超える"
    );

    let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);
    assert_eq!(
        window.first_visible_line, 1,
        "旧式ではあふれ判定が 1 行ぶんスクロールを返す＝先頭の選択肢「おしゃべり頻度」が描かれない"
    );

    // 同じ入力を正典の行送りで通すとスクロールしない（差は行送りの値 1 点だけ）。
    let canon = layout_script(
        &state,
        &actor,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &real,
        WrapKind::CharByChar,
    );
    assert_eq!(
        LayoutEngine::visible_window(&canon, &resolved.region, resolved.mode).first_visible_line,
        0,
        "正典の行送り（30）ではスクロールしない"
    );
}

// ══ 実物の所在の生存確認 ═══════════════════════════════════════════════════════════════

/// 本ファイルが指す実物の 3 ファイルが実在することを単独で確かめる。
/// パスが腐って「読めないから対象 0 件で緑」になる形を作らないための錨。
#[test]
fn the_shipped_fixture_files_this_cage_points_at_exist() {
    let root = emo2_fixture_root();
    for rel in [
        "emo2-kakukaku/descript.txt",
        &format!("emo2-kakukaku/{KERO_OVERLAY}"),
        &format!("emo2-kakukaku/{SAKURA_OVERLAY}"),
        "ghost/master/dic/menu.pasta",
    ] {
        let mut path: PathBuf = root.clone();
        for seg in rel.split('/') {
            path = path.join(seg);
        }
        assert!(
            Path::new(&path).is_file(),
            "実物 {} が見つからない",
            path.display()
        );
    }
    for heading in MENU_HEADINGS {
        let script = menu_script(heading);
        assert!(
            script.contains("\\q["),
            "{heading} の本文は \\q の選択肢を含む: {script}"
        );
    }
}
