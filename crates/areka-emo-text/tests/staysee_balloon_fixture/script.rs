//! 選択肢と遅延座標指定の位置を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.6**・設計 **C2** の H 行・
//! Data Models「Domain Model」）。
//!
//! ## ここで固定するもの
//!
//! 台本 `\_l[60,42]本文\n\q[はい,yes]\n\q[いいえ,no]` を、本番と同じ 4 段
//! （`areka_parsers::sakura::parse` → `areka_sakura::compile` →
//! [`TextLayerState::apply_cue`] → [`LayoutEngine::layout`]）で検体の描画範囲へ流し、
//!
//! - `\_l` の直後のグリフの左上が、描画開始点 (22,20) からの相対 (60,42) ＝ **(82,62)** に置かれること、
//! - 選択肢 2 行のグリフ矩形と、hover の強調帯（[`highlight_band_extent`] が決める丈）が
//!   すべて描画範囲 `[22,309]×[20,158]` の内側に収まること、
//! - 選択肢の見た目（[`ResolvedChoiceStyle`]）が検体の `cursor.*` の宣言から実際に導かれること
//!
//! を固定する。経路は既存の `kero_menu_capacity_test.rs` と同じで、検体だけが違う。
//!
//! ## 「選択肢の目印」は現状 hover の強調帯である
//!
//! areka が選択肢に与える目印は hover 時の帯（塗り色＋文字色）であり、独立した目印の画像は
//! まだ持たない（α 後の `areka-P0-choice-marker-styling` が受け持つ）。ゆえに要件 3.6 の
//! 「選択肢の目印が `validrect` の内側」は、帯の矩形の内包として観測する。帯とクリックを
//! 受ける矩形は同じ 1 つの導出（[`line_bands`] → [`derive_hit_rows`]）から出るので、
//! ここで測った矩形はそのまま描画される帯の矩形でもある。
//!
//! ## 空振りで緑にならないための備え
//!
//! - 「すべて内側」はグリフが 0 個でも真になる。行数・グリフ総数・選択肢の本数・帯の本数を
//!   **先に**固定してから内包を言う。
//! - 帯の丈が 0 でも内包は真になる。帯の丈と寄せ量を**実寸で**固定してから内包を言う。
//! - 遅延座標指定の位置は、値の違う 2 本の台本を同じ経路へ流して**指定どおりに動く**ことを
//!   対にして言う（[`CURSOR_CASES`]）。1 本だけなら「常にこの位置へ置く実装」でも緑になる。
//! - 選択肢の見た目は、`cursor.style` を別の値に宣言した定義で**別の見た目が導かれる**ことを
//!   対照として並べる（[`choice_style_follows_the_cursor_style_declaration`]）。検体の
//!   ファイルは読むだけで 1 バイトも変えず、差し替えるのは読み込んだ文字列の写しである
//!   （要件 2.2・`wrapping.rs` の下辺の対照と同じ手法）。
//!
//! ## 決定論
//!
//! ファイル読み込み・純粋層の解決・DirectWrite の計測のみ。実 GPU・実窓・実ゴーストを
//! 要さず、同一入力に対して常に同一の結果を返す。既定書体が無い環境では
//! [`super::region`] の寸法の門が先に赤で止まる。

use areka_emo_text::actor::ResolvedBalloonText;
use areka_emo_text::choice::{
    ResolvedChoiceStyle, annotate_lines, derive_hit_rows, highlight_band_extent, line_bands,
};
use areka_emo_text::layout::{LayoutEngine, PositionedLine, WrapPlan};
use areka_emo_text::state::{ChoiceSpan, TextItem, TextLayerState};
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::parse_str;
use areka_sakura::compile;
use areka_sakura::contract::{ActorKey, CuePayload, SystemVarSnapshot, TalkCue};

use super::test_support::{
    EXPECTED_ADVANCE_FULL, EXPECTED_FONT_HEIGHT, EXPECTED_LEFT, EXPECTED_LINE_BOX,
    EXPECTED_LINE_PITCH, EXPECTED_RIGHT, EXPECTED_TOP, expected_bottom, read_decoded,
    resolve_staysee, scope_image_size, staysee_metrics,
};

// ── 流し込む台本（設計 C2 の H 行）──────────────────────────────────────────

/// 遅延座標指定の値だけが違う 2 本の台本と、その値。
///
/// 1 本目が設計 H 行の台本そのもの。2 本目は指定の値を変えた対で、
/// 「指定どおりに動く」ことを言うために置く（1 本だけでは位置を決め打ちした実装でも緑になる）。
const CURSOR_CASES: [(&str, f32, f32); 2] = [
    (
        "\\_l[60,42]本文\\n\\q[はい,yes]\\n\\q[いいえ,no]",
        60.0,
        42.0,
    ),
    (
        "\\_l[100,14]本文\\n\\q[はい,yes]\\n\\q[いいえ,no]",
        100.0,
        14.0,
    ),
];

/// 設計 H 行の台本（[`CURSOR_CASES`] の 1 本目）。
const SCRIPT: &str = CURSOR_CASES[0].0;

/// 台本が載せる選択肢（配送順に `(ID, 表示文字列, 文字数)`）。
const EXPECTED_CHOICES: [(&str, &str, usize); 2] = [("yes", "はい", 2), ("no", "いいえ", 3)];

/// 台本が載せるグリフの総数（`本文` 2 ＋ `はい` 2 ＋ `いいえ` 3）。
const EXPECTED_GLYPHS: usize = 7;

/// 台本が作る行（`\_l` の直後の本文・選択肢 2 行）。
const EXPECTED_LINES: usize = 3;

// ── 本番と同じ 4 段（`kero_menu_capacity_test.rs` と同じ経路）──────────────────

/// 台本を実 sakura パイプライン（`parse` → `compile`）で cue 列へ落とし、
/// 文字状態機械が消費する Command cue だけを配送エンベロープへ無変形複写する。
fn command_cues(script: &str) -> Vec<TalkCue> {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = compile(&instructions, &SystemVarSnapshot::default());
    let cues: Vec<TalkCue> = compiled
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
        .collect();
    assert!(
        !cues.is_empty(),
        "台本 `{script}` から Command cue が 1 つも出なかった（cue 0 件では以降の観測が空振りになる）"
    );
    cues
}

/// cue 列を実 [`TextLayerState`] へ載せ、（状態, 唯一の actor）を返す。
fn state_of(script: &str) -> (TextLayerState, ActorKey) {
    let mut state = TextLayerState::default();
    for cue in command_cues(script) {
        state.apply_cue(&cue);
    }
    let actors: Vec<ActorKey> = state.actors().map(|(k, _)| k.clone()).collect();
    assert_eq!(
        actors.len(),
        1,
        "台本 `{script}` は 1 スコープぶんの cue 列のはず（実測の actor: {actors:?}）"
    );
    (state, actors[0].clone())
}

/// 配置の 1 回ぶん（描画範囲・行列・選択肢スパン）。
struct Placed {
    resolved: ResolvedBalloonText,
    lines: Vec<PositionedLine>,
    choices: Vec<ChoiceSpan>,
}

/// 台本を本体側（scope 0）の描画範囲・既定書体で配置する（全文可視）。
fn place(script: &str) -> Placed {
    let (state, actor) = state_of(script);
    let resolved = resolve_staysee(0);
    assert_eq!(
        resolved.font.height, EXPECTED_FONT_HEIGHT,
        "文字の高さが {EXPECTED_FONT_HEIGHT} ではなく {}（以下の期待値はこの高さが前提）",
        resolved.font.height
    );
    assert_eq!(
        resolved.mode,
        WritingMode::HorizontalTb,
        "検体は横書き（`vertical` 未宣言）のはずだが {:?} に解決された",
        resolved.mode
    );
    let actor_state = state
        .actor_state(&actor)
        .expect("apply_cue 済みの actor 状態が在る");
    let items = actor_state.items();
    let visible = items
        .iter()
        .filter(|it| matches!(it, TextItem::Glyph { .. }))
        .count();
    assert_eq!(
        visible, EXPECTED_GLYPHS,
        "台本 `{script}` が載せたグリフが {EXPECTED_GLYPHS} 個ではなく {visible} 個"
    );
    let metrics = staysee_metrics(&resolved);
    let lines = LayoutEngine::layout(
        items,
        visible,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &metrics,
        WrapPlan::CharByChar,
    );
    let choices = actor_state.choices().to_vec();
    Placed {
        resolved,
        lines,
        choices,
    }
}

/// 行を「その行に並んだ文字」の文字列へ写す（どの行に何が置かれたかを失敗文言に出すため）。
fn line_text(line: &PositionedLine) -> String {
    line.glyphs.iter().map(|g| g.ch).collect()
}

// ── 遅延座標指定の着地（設計 C2 の H 行・要件 3.6）────────────────────────────

/// `\_l[x,y]` の直後のグリフの左上が、描画開始点からの相対 (x,y) に置かれる。
///
/// 検体の描画開始点は (22,20) なので、設計 H 行の `\_l[60,42]` なら (82,62) である。
/// 値の違う 2 本目（`\_l[100,14]` → (122,34)）を同じ経路へ流し、**指定どおりに動く**ことを
/// 対にして言う（1 本だけなら位置を決め打ちした実装でも緑になる）。
#[test]
fn the_cursor_tag_places_the_next_glyph_at_the_declared_offset() {
    for (script, dx, dy) in CURSOR_CASES {
        let placed = place(script);
        let start = placed.resolved.region.start();
        assert_eq!(
            start,
            (EXPECTED_LEFT, EXPECTED_TOP),
            "描画開始点が ({EXPECTED_LEFT},{EXPECTED_TOP}) ではなく {start:?}\
             （遅延座標指定の基点が変わると以下の期待値の意味ごと変わる）"
        );
        assert_eq!(
            placed.lines.len(),
            EXPECTED_LINES,
            "台本 `{script}`: 行数が {EXPECTED_LINES} ではなく {}（各行の文字 {:?}）",
            placed.lines.len(),
            placed.lines.iter().map(line_text).collect::<Vec<_>>()
        );

        let head = &placed.lines[0];
        assert_eq!(
            line_text(head),
            "本文",
            "台本 `{script}`: 遅延座標指定の直後の行が「本文」ではなく「{}」",
            line_text(head)
        );
        let glyph = head
            .glyphs
            .first()
            .unwrap_or_else(|| panic!("台本 `{script}`: 先頭の行に文字が 1 つも無い"));

        let want = (start.0 + dx, start.1 + dy);
        assert_eq!(
            (glyph.inline_pos, head.rect.top),
            want,
            "台本 `{script}`: 遅延座標指定の直後のグリフ `{}` の左上が {want:?} ではなく {:?}\
             （描画開始点 {start:?} からの相対 ({dx},{dy}) に置かれるはず）",
            glyph.ch,
            (glyph.inline_pos, head.rect.top)
        );
        assert_eq!(
            head.rect.left, want.0,
            "台本 `{script}`: 行の矩形の近端が {} ではなく {}（描画とクリックの位置が食い違う）",
            want.0, head.rect.left
        );
    }
}

/// 遅延座標指定で送った行の**次の行**から、行送りは通常どおり 14 px ずつ進む。
///
/// 選択肢 2 行の上端の期待値（[`choice_glyphs_and_bands_stay_inside_the_drawing_range`]）は
/// この等差からの導出なので、導出の前提の側も固定する。
#[test]
fn the_rows_after_the_cursor_tag_advance_by_the_canonical_line_pitch() {
    let placed = place(SCRIPT);
    let tops: Vec<f32> = placed.lines.iter().map(|l| l.rect.top).collect();
    let want: Vec<f32> = (0..EXPECTED_LINES)
        .map(|n| EXPECTED_TOP + CURSOR_CASES[0].2 + EXPECTED_LINE_PITCH * n as f32)
        .collect();
    assert_eq!(
        tops,
        want,
        "行の上端が {want:?} ではなく {tops:?}（遅延座標指定で送った {} 以降は行送り {EXPECTED_LINE_PITCH} の等差）",
        EXPECTED_TOP + CURSOR_CASES[0].2
    );
    for (n, line) in placed.lines.iter().enumerate() {
        assert_eq!(
            line.rect.bottom - line.rect.top,
            EXPECTED_LINE_BOX,
            "{} 行目の行ボックスの丈が {EXPECTED_LINE_BOX} ではなく {}",
            n + 1,
            line.rect.bottom - line.rect.top
        );
    }
}

// ── 選択肢のグリフと強調帯の内包（設計 C2 の H 行・要件 3.6）──────────────────

/// 台本が載せた選択肢が、配送順・ID・表示文字列・文字数のとおりに記録される。
///
/// 選択肢が 0 本でも「すべて内側」は真になるので、内包を言う前にここで本数を固定する。
#[test]
fn the_script_records_both_choices_with_their_ids_and_labels() {
    let placed = place(SCRIPT);
    assert_eq!(
        placed.choices.len(),
        EXPECTED_CHOICES.len(),
        "選択肢が {} 本ではなく {} 本（実測 {:?}）",
        EXPECTED_CHOICES.len(),
        placed.choices.len(),
        placed
            .choices
            .iter()
            .map(|c| c.label.clone())
            .collect::<Vec<_>>()
    );
    for (n, (id, label, len)) in EXPECTED_CHOICES.iter().enumerate() {
        let span = &placed.choices[n];
        assert_eq!(
            (span.ordinal, span.id.as_str(), span.label.as_str()),
            (n, *id, *label),
            "{} 本目の選択肢が (序数 {n}, ID `{id}`, 表示 `{label}`) ではなく (序数 {}, ID `{}`, 表示 `{}`)",
            n + 1,
            span.ordinal,
            span.id,
            span.label
        );
        assert_eq!(
            span.glyph_range.len(),
            *len,
            "選択肢「{label}」のグリフ範囲が {len} 文字ではなく {} 文字（{:?}）",
            span.glyph_range.len(),
            span.glyph_range
        );
    }
}

/// 選択肢 2 行のグリフ矩形と強調帯が、すべて描画範囲の内側に収まる。
///
/// 内包は対象が 0 件でも真になり、帯の丈が 0 でも真になる。そこで
///
/// 1. 行数・選択肢の本数・帯の本数、
/// 2. 帯の丈（[`highlight_band_extent`] の実寸）と寄せ量、
/// 3. 選択肢ごとの帯の矩形（逐語）、
///
/// をこの順で固定してから 4 辺の内包を言う。帯の矩形は描画（塗り）とクリックの**共通の源**
/// （[`derive_hit_rows`]）から採るので、ここで内側なら描かれる帯も内側である。
#[test]
fn choice_glyphs_and_bands_stay_inside_the_drawing_range() {
    let placed = place(SCRIPT);
    let region = &placed.resolved.region;
    let bottom = expected_bottom(0);
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (EXPECTED_LEFT, EXPECTED_TOP, EXPECTED_RIGHT, bottom),
        "内包を測る描画範囲が期待値と違う（実測 {:?}）",
        (region.left(), region.top(), region.right(), region.bottom())
    );
    assert_eq!(
        placed.lines.len(),
        EXPECTED_LINES,
        "行数が {EXPECTED_LINES} ではなく {}（内包はグリフが 0 でも真になるので先に数える）",
        placed.lines.len()
    );
    assert_eq!(
        placed.choices.len(),
        EXPECTED_CHOICES.len(),
        "選択肢が {} 本ではなく {} 本",
        EXPECTED_CHOICES.len(),
        placed.choices.len()
    );

    // ⑵ 帯の実寸——丈 0 の帯や「帯が 1 本も無い」状態で内包が真になるのを防ぐ。
    let metrics = staysee_metrics(&placed.resolved);
    let bands = line_bands(&placed.lines, placed.resolved.mode, &metrics);
    let want_extent =
        highlight_band_extent(EXPECTED_FONT_HEIGHT, EXPECTED_LINE_BOX, EXPECTED_LINE_PITCH);
    assert_eq!(
        want_extent, EXPECTED_LINE_BOX,
        "既定書体の帯の丈は行ボックスの丈 {EXPECTED_LINE_BOX} と同じはずだが {want_extent}"
    );
    assert!(
        want_extent > 0.0,
        "帯の丈が {want_extent} ＝ 0 以下では内包を測っても何も示さない"
    );
    assert_eq!(
        bands.len(),
        placed.lines.len(),
        "帯が行数 {} 本ではなく {} 本",
        placed.lines.len(),
        bands.len()
    );
    for (n, band) in bands.iter().enumerate() {
        assert_eq!(
            (band.extent, band.offset),
            (want_extent, 0.0),
            "{} 行目の帯が (丈 {want_extent}, 寄せ 0) ではなく (丈 {}, 寄せ {})",
            n + 1,
            band.extent,
            band.offset
        );
    }

    // ⑶ 選択肢ごとの帯の矩形（描画とクリックの共通の源から採る・原点を戻して絶対 image px へ）。
    let segments = annotate_lines(&placed.lines, &placed.choices);
    assert_eq!(
        segments.len(),
        EXPECTED_CHOICES.len(),
        "選択肢の行別注釈が {} 件ではなく {} 件（折返しが起きていなければ選択肢 1 本＝1 件）",
        EXPECTED_CHOICES.len(),
        segments.len()
    );
    let rows = derive_hit_rows(
        &placed.lines,
        &segments,
        placed.resolved.mode,
        region,
        &bands,
    );
    assert_eq!(
        rows.len(),
        EXPECTED_CHOICES.len(),
        "帯の矩形が {} 件ではなく {} 件",
        EXPECTED_CHOICES.len(),
        rows.len()
    );

    for (n, row) in rows.iter().enumerate() {
        let (_, label, len) = EXPECTED_CHOICES[n];
        // 選択肢の行は本文の行の次から順に 1 行ずつ。
        let line = &placed.lines[n + 1];
        assert_eq!(
            line_text(line),
            label,
            "{} 行目に置かれた文字が「{label}」ではなく「{}」",
            n + 2,
            line_text(line)
        );
        // canvas 局所座標から絶対 image px へ戻す（原点は描画範囲の左上）。
        let abs = (
            row.rect.left + region.left(),
            row.rect.top + region.top(),
            row.rect.right + region.left(),
            row.rect.bottom + region.top(),
        );
        let want = (
            EXPECTED_LEFT,
            line.rect.top,
            EXPECTED_LEFT + EXPECTED_ADVANCE_FULL * len as f32,
            line.rect.top + want_extent,
        );
        assert_eq!(
            abs, want,
            "選択肢「{label}」の帯が {want:?} ではなく {abs:?}\
             （行内軸は文字幅 全角 {EXPECTED_ADVANCE_FULL} × {len} 字・ブロック軸は行の上端から丈 {want_extent}）"
        );
    }

    // ⑷ 4 辺の内包——グリフ矩形と帯の両方を、1 画素も超えないことで言う。
    let mut checked = 0usize;
    for (n, row) in rows.iter().enumerate() {
        let label = EXPECTED_CHOICES[n].1;
        let line = &placed.lines[n + 1];
        for (i, glyph) in line.glyphs.iter().enumerate() {
            let where_ = format!("選択肢「{label}」の {} 文字目 `{}`", i + 1, glyph.ch);
            assert_containment(
                &where_,
                (
                    glyph.inline_pos,
                    line.rect.top,
                    glyph.inline_pos + glyph.advance,
                    line.rect.bottom,
                ),
                bottom,
            );
            checked += 1;
        }
        assert_containment(
            &format!("選択肢「{label}」の強調帯"),
            (
                row.rect.left + region.left(),
                row.rect.top + region.top(),
                row.rect.right + region.left(),
                row.rect.bottom + region.top(),
            ),
            bottom,
        );
        checked += 1;
    }
    let want_checks: usize = EXPECTED_CHOICES.iter().map(|(_, _, len)| len + 1).sum();
    assert_eq!(
        checked,
        want_checks,
        "内包を測った矩形が {want_checks} 個（選択肢の文字 {} 個＋帯 {} 本）ではなく {checked} 個",
        want_checks - EXPECTED_CHOICES.len(),
        EXPECTED_CHOICES.len()
    );
}

/// 矩形 `(左, 上, 右, 下)` が描画範囲の内側に収まることを 4 辺それぞれで言う。
fn assert_containment(where_: &str, rect: (f32, f32, f32, f32), bottom: f32) {
    let (left, top, right, low) = rect;
    assert!(
        left >= EXPECTED_LEFT,
        "{where_} の左端 {left} が描画範囲の左辺 {EXPECTED_LEFT} より外に出た（超過 {} px）",
        EXPECTED_LEFT - left
    );
    assert!(
        right <= EXPECTED_RIGHT,
        "{where_} の右端 {right} が描画範囲の右辺 {EXPECTED_RIGHT} を超えた（超過 {} px）",
        right - EXPECTED_RIGHT
    );
    assert!(
        top >= EXPECTED_TOP,
        "{where_} の上端 {top} が描画範囲の上辺 {EXPECTED_TOP} より外に出た（超過 {} px）",
        EXPECTED_TOP - top
    );
    assert!(
        low <= bottom,
        "{where_} の下端 {low} が描画範囲の下辺 {bottom} を超えた（超過 {} px）",
        low - bottom
    );
}

// ── 選択肢の見た目は宣言から導かれる（設計 C2 の H 行・要件 3.6）───────────────

/// 検体の `cursor.style,square` から、四角い塗りの見た目が**色ごと**導かれる。
///
/// 塗り色は `cursor.brush.color`（192,214,242）・hover の文字色は `cursor.font.color`
/// （0,20,50）であり、どちらも検体の宣言そのものである。定数を書き写した比較にならないよう、
/// 宣言の値を読み出して突き合わせる。
#[test]
fn the_choice_style_is_derived_from_the_cursor_declaration() {
    let text = read_decoded("descript.txt");
    let want_fill = declared_color(&text, "cursor.brush.color");
    let want_text = declared_color(&text, "cursor.font.color");
    assert_eq!(
        (want_fill, want_text),
        ((192, 214, 242), (0, 20, 50)),
        "検体の宣言から読んだ色が ((192,214,242),(0,20,50)) ではなく ({want_fill:?},{want_text:?})"
    );

    let style = resolve_staysee(0).choice_style;
    assert_eq!(
        style,
        ResolvedChoiceStyle::SquareFill {
            fill: want_fill,
            text: want_text,
        },
        "`cursor.style,square` から導かれる見た目が 四角い塗り（塗り {want_fill:?}・文字 {want_text:?}）\
         ではなく {style:?}"
    );
}

/// `cursor.style` を別の値に宣言すると、導かれる見た目も別のものになる。
///
/// [`the_choice_style_is_derived_from_the_cursor_declaration`] の比較が
/// 「どんな定義でも同じ既定を返す」実装でも緑になるものではないことの対照である。
/// 検体の `descript.txt` は読むだけで、差し替えるのは読み込んだ**文字列の写し**であり、
/// 差し替えが当たったことをその場で確かめる（要件 2.2）。
#[test]
fn choice_style_follows_the_cursor_style_declaration() {
    const DECLARED: &str = "cursor.style,square";
    let text = read_decoded("descript.txt");
    assert!(
        text.contains(DECLARED),
        "検体の `descript.txt` に `{DECLARED}` の行が無い（対照の前提が崩れている）"
    );

    // `none` を宣言すると目印そのものが無くなる（塗りも hover 文字色も出ない）。
    let none = resolve_variant(&text.replace(DECLARED, "cursor.style,none"));
    assert_eq!(
        none,
        ResolvedChoiceStyle::NoMarker,
        "`cursor.style,none` から導かれる見た目が 目印なし ではなく {none:?}"
    );

    // 塗り色の宣言を変えると、導かれる塗り色も追随する（色が焼き付けられていない）。
    let repainted =
        resolve_variant(&text.replace("cursor.brush.color.r,192", "cursor.brush.color.r,7"));
    let want_text = declared_color(&text, "cursor.font.color");
    assert_eq!(
        repainted,
        ResolvedChoiceStyle::SquareFill {
            fill: (7, 214, 242),
            text: want_text,
        },
        "塗り色の赤成分を 192 から 7 へ宣言し直したのに、導かれた見た目が {repainted:?}"
    );
}

/// 定義の文字列の写しから、本体側（scope 0）の選択肢の見た目を解く（対照専用）。
fn resolve_variant(text: &str) -> ResolvedChoiceStyle {
    ResolvedBalloonText::resolve(&parse_str(text, None), scope_image_size(0)).choice_style
}

/// 定義の文字列から `<接頭辞>.r/g/b` の 3 成分を読む（宣言そのものを期待値の源にする）。
fn declared_color(text: &str, prefix: &str) -> (u8, u8, u8) {
    let read = |component: char| -> u8 {
        let key = format!("{prefix}.{component},");
        text.lines()
            .find_map(|line| line.trim().strip_prefix(&key))
            .unwrap_or_else(|| panic!("検体の `descript.txt` に `{key}` の行が無い"))
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("`{key}` の値が 0〜255 の数ではない: {e}"))
    };
    (read('r'), read('g'), read('b'))
}
