use super::test_support::{IMAGE, glyphs, inline_positions, model, model_rect};
use super::{CursorWarnGuard, FixedMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;
use areka_sakura::contract::ActorKey;
use log_capture_kit::capture;

// ── 横書きの配線（検証表 H1・H4・H5・H6 と範囲外記録の肯定側） ──
//
// 兄弟ファイルとの分担（重複させない）:
// - `layout_cursor_tests.rs` — 行区切り・軸上書き・末尾蒸発・両軸 no-op・縮退 warn-once、
//   および軸ごと合成（H2）と実効位置（H3）。
// - `layout_cursor_order_tests.rs` — 書かれた順（H3b・H3c）。
// - `layout_cursor_vertical_tests.rs` — 縦書き 2 方向。
// - `cursor_tag_tests.rs`／`cursor_tag_resolve_tests.rs` — 解決そのもの（純関数）。
//
// 本ファイルが見るのは、上のどれにも属さない**配線の残り**である——絶対座標の基点が
// どこから来るか（H1）、正典の記述例が配線を通しても正典の位置に載るか（H4）、軸取り違えの
// 完全無効果（H5）、行数（H6）、そして**範囲外記録の肯定側**（`\_l` が文字描画範囲の外へ
// 出たとき、位置を寄せずに DEBUG を 1 件残すこと）。
//
// 期待値はすべて design.md の検証表と正典逐語（requirements.md 付録 A）から**式で**導く。
// 実装の戻り値を書き写さない。

/// 共通前提 A（既存の横書き檻と同一）: 文字描画開始点 (0, 0)・validrect＝画像全域
/// `0/0/400/224`・折返し閾値 400。font 10（全角 'あ' の advance 10・pitch 13）。
fn region_a() -> TextRegion {
    TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    )
}

/// 共通前提 B（**弁別用**）: `origin` 未宣言・validrect を画像の**部分矩形**
/// `left 40 / top 20 / right 360 / bottom 200` にしたバルーン。
///
/// 未宣言なので文字描画開始点は書字開始角へ縮退し、横書きでは `(left, top) = (40, 20)` になる。
/// 4 つの候補（画像左上 `(0, 0)`／画像中央 `(200, 112)`／validrect 右下 `(360, 200)`／
/// 書字開始角 `(40, 20)`）が**すべて相異なる**ので、基点を取り違えた実装はどれも赤になる
/// ——既定の共通前提 A は 4 候補のうち 2 つが `(0, 0)` と重なるため、この弁別ができない。
fn region_b_undeclared_offset() -> TextRegion {
    let region = TextRegion::resolve(
        &model_rect((None, None), (Some(20), Some(200), Some(40), Some(360))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 檻の前提を檻にする（fixture が意図した矩形・開始点になっていることの確認）。
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (40.0, 20.0, 360.0, 200.0)
    );
    assert_eq!(
        region.start(),
        (40.0, 20.0),
        "未宣言の origin 成分は書字開始角（横書き＝validrect の左上）へ縮退する"
    );
    region
}

/// 共通前提 C: 前提 B と同じ validrect に、文字描画開始点を**角と異なる位置**
/// `(100, 60)` として宣言したバルーン（H1 の弁別用の対照）。
fn region_c_declared_origin() -> TextRegion {
    let region = TextRegion::resolve(
        &model_rect(
            (Some(100), Some(60)),
            (Some(20), Some(200), Some(40), Some(360)),
        ),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        region.start(),
        (100.0, 60.0),
        "宣言された origin 成分は字義どおり（角へ寄せない）"
    );
    region
}

/// 横書き・共通前提を指定して layout を回す（visible は指定数）。
fn layout_h(items: &[TextItem], visible: usize, region: &TextRegion) -> Vec<PositionedLine> {
    LayoutEngine::layout(
        items,
        visible,
        region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// 全角グリフ 1 個（'あ'・advance 10）。
fn glyph() -> TextItem {
    TextItem::Glyph { ch: 'あ' }
}

/// `\_l[x,y]`（両軸とも絶対 px）。
fn cursor_px(x: f32, y: f32) -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Absolute {
            value: x,
            unit: CursorUnit::Px,
        },
        y: CursorCoord::Absolute {
            value: y,
            unit: CursorUnit::Px,
        },
    }
}

// ── H1: 未宣言バルーンでの既存表示結果の不変（Requirement 2.7・9.6） ──

/// **H1（性質の檻）**: `origin` 未宣言のバルーンでは、`\_l` の絶対座標が
/// **書字開始角**（横書き＝validrect の左上）から測られる。
///
/// 検証表 H1 は「既存 13 本の期待値が未宣言バルーンで不変」だが、既存 13 本の共通前提は
/// validrect＝画像全域・`origin` 宣言 `(0, 0)` なので、**基点の候補が軒並み `(0, 0)` に潰れて
/// いる**——期待値が現在の値と一致することを確かめても「不変」の主張にはならない（基点を
/// 画像左上へ取り違えた実装も同じ値を返す）。そこで本檻は「不変」を**性質**として書く:
///
/// > 未宣言バルーンの絶対座標の基点は、validrect の左上（＝書字開始角）である。
///
/// 前提 B は 4 つの候補（画像左上 (0,0)／画像中央 (200,112)／validrect 右下 (360,200)／
/// 書字開始角 (40,20)）を相異ならせてあるので、どれを取り違えても赤になる。
///
/// 手計算（`start = (40, 20)`・font 10・pitch 13。式は design.md 解決表
/// `位置 = 基点 + 値 × 係数`）:
/// - `\_l[0,0]` → `(40 + 0, 20 + 0) = (40, 20)`
/// - `\_l[100,50]` → `(40 + 100, 20 + 50) = (140, 70)`（正典「数値＝ピクセル単位座標」）
/// - `\_l[2em,3lh]` → `(40 + 2×10, 20 + 3×13) = (60, 59)`（正典「1em＝文字高さ」「1lh＝1em＋行間」）
///
/// 3 形はいずれも R2.7 が「表示結果を変えない」と定める既存実導出形（非負の数値・`em`・`lh`）
/// である。原点の切替（タスク 1.2）で横書きの着地が変わらなかったのは、未宣言成分が
/// 書字開始角へ縮退した結果 `TextRegion::start()` が validrect の左上と一致するからであって、
/// 偶然ではない——この檻はその**理由**を固定する。
#[test]
fn undeclared_balloon_measures_absolute_cursor_from_the_write_start_corner() {
    let region = region_b_undeclared_offset();

    let lines = layout_h(&[cursor_px(0.0, 0.0), glyph()], 1, &region);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        inline_positions(&lines[0]),
        vec![40.0],
        "\\_l[0,0] の X は書字開始角 40（画像左上 0 でも validrect 右辺 360 でもない）"
    );
    assert_eq!(
        lines[0].rect.top, 20.0,
        "\\_l[0,0] の Y は書字開始角 20（画像上辺 0 でも画像中央 112 でもない）"
    );

    let lines = layout_h(&[cursor_px(100.0, 50.0), glyph()], 1, &region);
    assert_eq!(inline_positions(&lines[0]), vec![140.0], "40 + 100");
    assert_eq!(lines[0].rect.top, 70.0, "20 + 50");

    let items = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 2.0,
                unit: CursorUnit::Em,
            },
            y: CursorCoord::Absolute {
                value: 3.0,
                unit: CursorUnit::Lh,
            },
        },
        glyph(),
    ];
    let lines = layout_h(&items, 1, &region);
    assert_eq!(
        inline_positions(&lines[0]),
        vec![60.0],
        "40 + 2 × font_height(10)"
    );
    assert_eq!(lines[0].rect.top, 59.0, "20 + 3 × line_pitch(13)");
}

/// **H1 の弁別対照**: 文字描画開始点を角と異なる位置に**宣言**したバルーンでは、絶対座標が
/// その宣言値から測られる（Requirement 2.9）。
///
/// 上の檻だけだと「基点を validrect の左上に決め打ちした実装」が素通りしてしまう
/// （未宣言バルーンでは両者が一致するため）。宣言バルーンを 1 本並べると、基点が本当に
/// `TextRegion::start()`（宣言は字義・未宣言のみ角へ縮退）から来ていることが確かめられる。
///
/// 手計算: `start = (100, 60)` なので `\_l[0,0]` → `(100, 60)`。
///
/// **本檻は横書き 1 方向の対照にとどめる**——検証項目 V7（3 書字方向 × 宣言 `origin`）は
/// タスク 5.2 の所管である。
#[test]
fn declared_origin_moves_the_absolute_basis_off_the_write_start_corner() {
    let region = region_c_declared_origin();
    let lines = layout_h(&[cursor_px(0.0, 0.0), glyph()], 1, &region);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        inline_positions(&lines[0]),
        vec![100.0],
        "宣言された origin.x(100) が絶対座標の基点になる（validrect 左辺 40 ではない）"
    );
    assert_eq!(
        lines[0].rect.top, 60.0,
        "宣言された origin.y(60) が絶対座標の基点になる（validrect 上辺 20 ではない）"
    );
}

// ── H4: 正典の記述例 3 つ（Requirement 9.3・付録 A「記述例」） ──

/// **H4-1**: 正典の記述例 `\_l[30,5em]`＝「座標 X=30pixel、座標 Y=5 文字分高さ」。
///
/// 手計算（前提 A・`start = (0, 0)`・font 10）: `(0 + 30, 0 + 5×10) = (30, 50)`。
#[test]
fn canon_example_absolute_px_and_em() {
    let region = region_a();
    let items = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 30.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Em,
            },
        },
        glyph(),
    ];
    let lines = layout_h(&items, 1, &region);
    assert_eq!(lines.len(), 1);
    assert_eq!(inline_positions(&lines[0]), vec![30.0], "X = 0 + 30");
    assert_eq!(lines[0].rect.top, 50.0, "Y = 0 + 5 × font_height(10)");
}

/// **H4-2**: 正典の記述例 `\_l[@-1650%,100]`＝「座標 X=最後の文字から文字高さ 1650%分左、
/// 座標 Y=100pixel」。`@` 相対と `%` の共存（Requirement 3.2）がそのまま乗る。
///
/// 手計算（前提 A・font 10・`%` の係数＝`font_height / 100 = 0.1`）:
/// 20 個の全角グリフを置くと次の文字が置かれる位置（実効位置）は `(200, 0)` なので
/// - X ＝ `current.x + (−1650) × 0.1 = 200 − 165 = 35`
/// - Y ＝ `origin.y + 100 = 0 + 100 = 100`
///
/// グリフ数を 20 にしてあるのは、正典例の移動量 165 に対して**着地が文字描画範囲に残る**
/// ようにするためである（範囲外へ出すと本檻の主張に範囲外記録が混ざる。範囲外そのものは
/// 専用の檻が見る）。
#[test]
fn canon_example_relative_percent_and_absolute_px() {
    let region = region_a();
    let mut items = glyphs(20);
    items.push(TextItem::CursorMove {
        x: CursorCoord::Relative {
            value: -1650.0,
            unit: CursorUnit::Percent,
        },
        y: CursorCoord::Absolute {
            value: 100.0,
            unit: CursorUnit::Px,
        },
    });
    items.push(glyph());
    let lines = layout_h(&items, 21, &region);
    assert_eq!(lines.len(), 2, "移動が成立するので \\_l は行の分割点になる");
    assert_eq!(
        inline_positions(&lines[1]),
        vec![35.0],
        "X = 実効位置 200 + (−1650) × font_height(10)/100 = 200 − 165"
    );
    assert_eq!(lines[1].rect.top, 100.0, "Y = 0 + 100（絶対・基点は原点）");
}

/// **H4-3**: 正典の記述例 `\_l[,@-100]`＝「座標 X=変更なし、座標 Y=最後の文字から 100pixel 上」。
///
/// 手計算（前提 A・font 10）:
/// - `あ` を置いて `\_l[,150]` で Y を 150 へ送り、次の `あ` を置く（行内位置は 10 → 20）。
/// - `\_l[,@-100]` の実効位置は `(20, 150)`。X は省略＝**動かさない**ので行内位置 20 のまま
///   （行頭 0 へ戻らないことが「変更なし」の観測点そのもの）、Y ＝ `150 − 100 = 50`。
///
/// X の省略が「先に置かれた文字の続き」を保つ点が正典「(省略): 移動しない」の逐語であり、
/// 改行のような行内リセットとは別物である（Requirement 1.2・1.5・3.1）。
#[test]
fn canon_example_relative_negative_y_keeps_x_unchanged() {
    let region = region_a();
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Absolute {
                value: 150.0,
                unit: CursorUnit::Px,
            },
        },
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Relative {
                value: -100.0,
                unit: CursorUnit::Px,
            },
        },
        glyph(),
    ];
    let lines = layout_h(&items, 3, &region);
    assert_eq!(lines.len(), 3);
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "2 個目は X 据え置き"
    );
    assert_eq!(lines[1].rect.top, 150.0, "\\_l[,150] で Y = 0 + 150");
    assert_eq!(
        inline_positions(&lines[2]),
        vec![20.0],
        "X＝変更なし（直前グリフの送り終端 20 のまま・行頭 0 へ戻らない）"
    );
    assert_eq!(lines[2].rect.top, 50.0, "Y = 実効位置 150 − 100");
}

// ── H5: 中央指定の軸取り違え（Requirement 1.5・5.3・5.4） ──

/// **H5**: `\_l[centery,centerx]`（**両軸とも取り違え**）は両軸不動・行を分割せず、
/// `warn` が **1 件**（分岐は `CenterAxisMismatch`。軸が違っても同一キャラクターでは 1 回
/// ＝一回化の鍵が `(actor, degrade)` であって軸を含まないこと）、`debug` が **1 件**
/// （完全無効果）になる。
///
/// 手計算（前提 A・font 10）: `[あ, \_l[centery,centerx], あ]` は `\_l` が完全無効果なので
/// 1 行 `[あ@0, あ@10]`——`\_l` を書かなかった場合と 1 ビットも変わらない。
///
/// 件数だけでなく**どれが出たか**を固定する: `warn` は `axis = X`（先に解決される軸）・
/// `coord = CenterY`（X 軸に書かれた `centery`）・`degrade = CenterAxisMismatch`。
/// `debug` は配線の完全無効化（`\_l` を素通しした旨）であって範囲外記録ではない。
///
/// `TextRegion` は**捕捉窓の外**で組む——`TextRegion::resolve` は validrect や origin の縮退で
/// `debug!` を出すので、窓の中で組むと件数に混ざる。
#[test]
fn center_axis_mismatch_on_both_axes_is_a_complete_noop_with_one_warn_and_one_debug() {
    let region = region_a();
    let actor = ActorKey::from("0");
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::CenterY,
            y: CursorCoord::CenterX,
        },
        glyph(),
    ];

    let (lines, events) = capture(|| {
        let mut guard = CursorWarnGuard::default();
        LayoutEngine::layout_with_cursor_warn(
            &items,
            2,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
            &actor,
            &mut guard,
        )
    });

    // 挙動: 両軸不動＝行を分割せず、`\_l` を書かなかった場合と同じ配置。
    assert_eq!(
        lines.len(),
        1,
        "両軸とも移動が成立しない \\_l は行を分割しない（R6.2）"
    );
    assert_eq!(
        inline_positions(&lines[0]),
        vec![0.0, 10.0],
        "両軸不動＝カーソルは動かない（R1.5）"
    );

    // 警告: 1 件だけ・分岐は CenterAxisMismatch（軸は鍵に含まれない＝2 軸で 1 件）。
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(
        warns.len(),
        1,
        "軸が違っても同一キャラクター・同一分岐なので警告は 1 件（鍵は (actor, degrade)）"
    );
    assert_eq!(warns[0].field("degrade"), Some("CenterAxisMismatch"));
    assert_eq!(
        warns[0].field("axis"),
        Some("X"),
        "先に解決される X 軸の分が残る（Y 軸は同一分岐なので沈黙）"
    );
    assert_eq!(
        warns[0].field("coord"),
        Some("CenterY"),
        "X 軸に書かれた `centery` が原因として載る"
    );

    // 完全無効果の DEBUG: 1 件だけ（範囲外記録は 1 件も無い＝移動が成立していないため）。
    let debugs: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::DEBUG)
        .collect();
    assert_eq!(debugs.len(), 1, "完全無効果の DEBUG は 1 件");
    assert!(
        debugs[0].message().contains("完全 no-op"),
        "出たのは完全無効果の記録である（実際: {}）",
        debugs[0].message()
    );
}

// ── H6: 行構造（Requirement 6.1・6.2・6.3） ──

/// **H6**: 移動が成立するときだけ `\_l` が行の分割点になる。
/// `あ\_l[10,]あ` は **2 行**・`あ\_l[,]あ` は **1 行**（`Vec<PositionedLine>` の行数）。
///
/// 前者は行内位置が偶然 10（＝1 個目の送り終端）と同じになるので、**着地座標だけを見ても
/// 2 つのケースを区別できない**——区別できるのは行数だけである。`\c[line]` の「行」判定が
/// これに乗る（Requirement 6.3・本仕様は `\c[line]` を実装しない）。
///
/// 手計算（前提 A・font 10）:
/// - `[あ, \_l[10,], あ]`: X ＝ `0 + 10 = 10`・Y は省略＝据え置き 0。実体化で 1 行目
///   `[あ@0]` が閉じ、2 行目 `[あ@10]`（`top = 0`）が開く → 2 行。
/// - `[あ, \_l[,], あ]`: 両軸省略＝完全無効果。行は閉じない → 1 行 `[あ@0, あ@10]`。
#[test]
fn line_count_splits_only_when_a_move_succeeds() {
    let region = region_a();

    let moved = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        glyph(),
    ];
    let lines = layout_h(&moved, 2, &region);
    assert_eq!(
        lines.len(),
        2,
        "一方の軸で移動が成立する \\_l は行の分割点（R6.1）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(inline_positions(&lines[1]), vec![10.0], "X = 0 + 10");
    assert_eq!(lines[1].rect.top, 0.0, "Y は省略＝据え置き");

    let omitted = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Omitted,
        },
        glyph(),
    ];
    let lines = layout_h(&omitted, 2, &region);
    assert_eq!(
        lines.len(),
        1,
        "両軸省略の \\_l は行を分割しない（R6.2）——着地座標は上のケースと同じなので、\
         2 つを弁別できる観測点は行数だけである"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
}

// ── 範囲外記録の肯定側（Requirement 2.6・design 縮退表の最終行） ──

/// **範囲外記録の肯定側**: `\_l` の解決値が文字描画範囲の外へ出たとき、位置は**寄せられず**
/// （字義どおり）、DEBUG が **1 件**記録される。
///
/// 手計算（前提 B・`start = (40, 20)`・validrect `[40, 360] × [20, 200]`・画像 400×224）:
/// - 範囲外: `\_l[340,]` → X ＝ `40 + 340 = 380`。validrect の右辺 360 の**外**だが、
///   バルーン画像の右辺 400 の**内**である——判定に画像の辺を使った実装は「範囲内」と見て
///   0 件になるので、この 1 件が「validrect の辺で判定していること」の証跡にもなる。
///   着地は 380 のまま（validrect 右辺 360 へ寄せない・R2.6「内側への自動的な寄せを行わない」）。
/// - 範囲内の対照: `\_l[300,]` → X ＝ `40 + 300 = 340` ∈ `[40, 360]` → 0 件・着地 340。
///
/// 対照を隣に置くのは、**同じ観測点で 1 件を出せることを示すため**である（0 件の主張が
/// 経路ごと素通りして静かに緑になる形を塞ぐ）。
///
/// Y は省略なので解決が成立するのは X 軸だけ＝件数は厳密に 1 件になる。`TextRegion` は
/// **捕捉窓の外**で組む（`TextRegion::resolve` 自身が縮退の `debug!` を出すため）。
#[test]
fn out_of_range_cursor_lands_literally_and_records_one_debug() {
    let region = region_b_undeclared_offset();
    let out_of_range = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 340.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        glyph(),
    ];
    let in_range = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 300.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        glyph(),
    ];

    // 肯定側: 範囲外へ出す。
    let (lines, events) = capture(|| layout_h(&out_of_range, 1, &region));
    assert_eq!(lines.len(), 1);
    assert_eq!(
        inline_positions(&lines[0]),
        vec![380.0],
        "解決値 380 が字義どおり着地する（validrect 右辺 360 へ寄せない・R2.6）"
    );
    let notes: Vec<_> = events
        .iter()
        .filter(|e| e.message().starts_with("[note_out_of_range]"))
        .collect();
    assert_eq!(
        notes.len(),
        1,
        "範囲外の解決値は DEBUG を 1 件残す（配線が記録の口を呼んでいることの証跡）"
    );
    assert_eq!(notes[0].level, tracing::Level::DEBUG);
    assert_eq!(notes[0].field("axis"), Some("X"));
    assert_eq!(notes[0].field("value"), Some("380.0"));
    assert_eq!(
        notes[0].field("range_min"),
        Some("40.0"),
        "範囲は validrect の辺（画像の辺 0 ではない）"
    );
    assert_eq!(
        notes[0].field("range_max"),
        Some("360.0"),
        "範囲は validrect の辺（画像の辺 400 ではない）"
    );

    // 対照: 同じ観測点で範囲内なら 0 件（かつ着地は指定どおり）。
    let (lines, events) = capture(|| layout_h(&in_range, 1, &region));
    assert_eq!(inline_positions(&lines[0]), vec![340.0], "40 + 300");
    assert_eq!(
        events
            .iter()
            .filter(|e| e.message().starts_with("[note_out_of_range]"))
            .count(),
        0,
        "範囲内（境界含む）は記録しない"
    );
}
