//! 解決表（design.md「Data Models ／ 解決表」）の**全網羅**テスト（タスク 3.3）。
//!
//! ## 章立て（表の行 → 檻の対応）
//!
//! | § | 内容 | 対応する表の行 |
//! |---|---|---|
//! | §1 | 表駆動の全行 × 両軸（[`CASES`]・[`Row`]） | 解決表 10 行すべて |
//! | §2 | 被覆の機械的主張（行・軸・単位の取りこぼしを赤にする） | 同上（主張そのもの） |
//! | §3 | 単位が軸に依らない（`@` 相対側） | `@N`（R1.4） |
//! | §4 | 境界値（`0`・`-0`・小数・`@0`・`@-0`・大きな負値） | `N` / `@N`（R9.4） |
//! | §5 | 正典の記述例 3 つ＋縦書きの記述例 2 つ | 付録 A 逐語（R9.3） |
//! | §6 | ログ件数（解決は無言・警告は分岐とキャラクターの組ごとに 1 件） | 縮退表 全行（R5.2/5.3/5.5） |
//!
//! 縮退表（design.md「Error Handling ／ 縮退表」）の行との対応は §6 の doc に置く。
//! 範囲外記録（縮退表 最終行）と警告一回化の**判断分岐そのもの**は姉妹モジュール
//! `cursor_tag_tests.rs` が持つ（本ファイルは件数の側から重ねて締める）。
//!
//! **期待値は正典の式から書く**——実装が返した値を書き写さない。表の各ケースは
//! 「どの基点に、どの係数を掛けた値を足すか」を[閉包][fn]で持ち、基点 3 種と係数 4 種が
//! いずれも相異なる基点束（[`discriminating_basis`]）でも同じ式が成り立つことまで見るので、
//! 基点・係数のどれを取り違えた実装もどれか 1 本で必ず赤になる。

use super::test_support::{
    CURRENT, FONT_HEIGHT, IMAGE_SIZE, LINE_PITCH, ORIGIN, basis, discriminating_basis,
    out_of_range_region,
};
use super::{
    CursorAxis, CursorBasis, CursorDegrade, CursorWarnGuard, note_out_of_range,
    resolve_cursor_axis, warn_cursor_degrade,
};
use crate::state::{CursorCoord, CursorUnit};
use areka_sakura::contract::ActorKey;
use log_capture_kit::count_levels;

// ── §1 解決表の全行 × 両軸 ──

/// design.md 解決表の**行 ID**（本ファイルの被覆主張の鍵）。
///
/// 表の 10 行と 1 対 1 に対応する。行を増やしたら [`ALL_ROWS`] にも足さない限り §2 が赤になる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    /// `""`（省略）→ `Ok(None)`＝動かさない・無音。
    Omitted,
    /// `N`（数値・負値・小数）→ `origin + N`。
    AbsolutePx,
    /// `Nem` → `origin + N × font_height`。
    AbsoluteEm,
    /// `Nlh` → `origin + N × line_pitch`。
    AbsoluteLh,
    /// `N%` → `origin + N × font_height / 100`。
    AbsolutePercent,
    /// `@N`（単位付き可）→ `current + N × coef`。
    Relative,
    /// `centerx` on X → `image_size.0 / 2`。
    CenterXOnX,
    /// `centery` on Y → `image_size.1 / 2`。
    CenterYOnY,
    /// `centerx` on Y・`centery` on X → `Err(CenterAxisMismatch)`。
    CenterAxisMismatch,
    /// 解釈不能・非有限 → `Err(Unparsable)`。
    Unparsable,
}

/// 解決表の全行（§2 の被覆主張の母集合）。
const ALL_ROWS: &[Row] = &[
    Row::Omitted,
    Row::AbsolutePx,
    Row::AbsoluteEm,
    Row::AbsoluteLh,
    Row::AbsolutePercent,
    Row::Relative,
    Row::CenterXOnX,
    Row::CenterYOnY,
    Row::CenterAxisMismatch,
    Row::Unparsable,
];

/// 両軸に書ける行（`centerx` on X・`centery` on Y は定義から片軸のみで、逆側は
/// [`Row::CenterAxisMismatch`] が受け持つ）。
const AXIS_SYMMETRIC_ROWS: &[Row] = &[
    Row::Omitted,
    Row::AbsolutePx,
    Row::AbsoluteEm,
    Row::AbsoluteLh,
    Row::AbsolutePercent,
    Row::Relative,
    Row::CenterAxisMismatch,
    Row::Unparsable,
];

/// 全単位（§2 が絶対・相対の双方 × 両軸で存在を要求する）。
const ALL_UNITS: &[CursorUnit] = &[
    CursorUnit::Px,
    CursorUnit::Em,
    CursorUnit::Lh,
    CursorUnit::Percent,
];

/// 表の 1 ケース。`expect` は**式**（基点束から期待値を導く関数）であって定数ではない。
struct Case {
    /// 対応する解決表の行。
    row: Row,
    /// 正典の書き方（`\_l` の当該軸に書かれる文字列）。
    written: &'static str,
    /// 語彙層が転写した結果。
    coord: CursorCoord,
    /// 適用軸。
    axis: CursorAxis,
    /// 期待値を基点束から導く式（design.md 解決表の右辺そのもの）。
    expect: fn(&CursorBasis) -> Result<Option<f32>, CursorDegrade>,
}

/// 絶対の 1 ケースを組む（`written` と値・単位・軸を並べて書けるようにする糖衣）。
macro_rules! absolute_case {
    ($row:expr, $written:literal, $value:expr, $unit:expr, $axis:expr, $expect:expr) => {
        Case {
            row: $row,
            written: $written,
            coord: CursorCoord::Absolute {
                value: $value,
                unit: $unit,
            },
            axis: $axis,
            expect: $expect,
        }
    };
}

/// 相対の 1 ケースを組む。
macro_rules! relative_case {
    ($row:expr, $written:literal, $value:expr, $unit:expr, $axis:expr, $expect:expr) => {
        Case {
            row: $row,
            written: $written,
            coord: CursorCoord::Relative {
                value: $value,
                unit: $unit,
            },
            axis: $axis,
            expect: $expect,
        }
    };
}

/// 解決表の全行 × 両軸（26 ケース）。
///
/// 単位の係数（1 / 10 / 12 / 0.1）が互いに異なり、係数を掛ける値も行ごとに変えてあるので、
/// 単位を取り違えた実装は当該行で赤になる。
const CASES: &[Case] = &[
    // 省略（両軸）。
    Case {
        row: Row::Omitted,
        written: "",
        coord: CursorCoord::Omitted,
        axis: CursorAxis::X,
        expect: |_| Ok(None),
    },
    Case {
        row: Row::Omitted,
        written: "",
        coord: CursorCoord::Omitted,
        axis: CursorAxis::Y,
        expect: |_| Ok(None),
    },
    // 数値（正の小数・負値を両軸で）。
    absolute_case!(
        Row::AbsolutePx,
        "12.5",
        12.5,
        CursorUnit::Px,
        CursorAxis::X,
        |b| Ok(Some(b.origin.0 + 12.5))
    ),
    absolute_case!(
        Row::AbsolutePx,
        "12.5",
        12.5,
        CursorUnit::Px,
        CursorAxis::Y,
        |b| Ok(Some(b.origin.1 + 12.5))
    ),
    absolute_case!(
        Row::AbsolutePx,
        "-40",
        -40.0,
        CursorUnit::Px,
        CursorAxis::X,
        |b| Ok(Some(b.origin.0 + -40.0))
    ),
    absolute_case!(
        Row::AbsolutePx,
        "-40",
        -40.0,
        CursorUnit::Px,
        CursorAxis::Y,
        |b| Ok(Some(b.origin.1 + -40.0))
    ),
    // 文字高さ（em）。
    absolute_case!(
        Row::AbsoluteEm,
        "2em",
        2.0,
        CursorUnit::Em,
        CursorAxis::X,
        |b| Ok(Some(b.origin.0 + 2.0 * b.font_height))
    ),
    absolute_case!(
        Row::AbsoluteEm,
        "-1.5em",
        -1.5,
        CursorUnit::Em,
        CursorAxis::Y,
        |b| Ok(Some(b.origin.1 + -1.5 * b.font_height))
    ),
    // 行送り（lh）。
    absolute_case!(
        Row::AbsoluteLh,
        "-1lh",
        -1.0,
        CursorUnit::Lh,
        CursorAxis::X,
        // 値を束縛で置くのは「基点 + 値 × 係数」の式の形を崩さずに書くためである。
        |b| {
            let value = -1.0;
            Ok(Some(b.origin.0 + value * b.line_pitch))
        }
    ),
    absolute_case!(
        Row::AbsoluteLh,
        "2lh",
        2.0,
        CursorUnit::Lh,
        CursorAxis::Y,
        |b| Ok(Some(b.origin.1 + 2.0 * b.line_pitch))
    ),
    // 百分率（%）。
    absolute_case!(
        Row::AbsolutePercent,
        "150%",
        150.0,
        CursorUnit::Percent,
        CursorAxis::X,
        |b| { Ok(Some(b.origin.0 + 150.0 * (b.font_height / 100.0))) }
    ),
    absolute_case!(
        Row::AbsolutePercent,
        "-1650%",
        -1650.0,
        CursorUnit::Percent,
        CursorAxis::Y,
        |b| Ok(Some(b.origin.1 + -1650.0 * (b.font_height / 100.0)))
    ),
    // `@` 相対 × 4 単位 × 両軸。
    relative_case!(
        Row::Relative,
        "@3",
        3.0,
        CursorUnit::Px,
        CursorAxis::X,
        |b| Ok(Some(b.current.0 + 3.0))
    ),
    relative_case!(
        Row::Relative,
        "@-100",
        -100.0,
        CursorUnit::Px,
        CursorAxis::Y,
        |b| Ok(Some(b.current.1 + -100.0))
    ),
    relative_case!(
        Row::Relative,
        "@2.5em",
        2.5,
        CursorUnit::Em,
        CursorAxis::X,
        |b| Ok(Some(b.current.0 + 2.5 * b.font_height))
    ),
    relative_case!(
        Row::Relative,
        "@1em",
        1.0,
        CursorUnit::Em,
        CursorAxis::Y,
        |b| Ok(Some(b.current.1 + 1.0 * b.font_height))
    ),
    relative_case!(
        Row::Relative,
        "@-1lh",
        -1.0,
        CursorUnit::Lh,
        CursorAxis::X,
        // 正典の記述例「1 列ぶん左の列の先頭へ」。値の束縛は上の `-1lh` と同じ理由。
        |b| {
            let value = -1.0;
            Ok(Some(b.current.0 + value * b.line_pitch))
        }
    ),
    relative_case!(
        Row::Relative,
        "@2lh",
        2.0,
        CursorUnit::Lh,
        CursorAxis::Y,
        |b| Ok(Some(b.current.1 + 2.0 * b.line_pitch))
    ),
    relative_case!(
        Row::Relative,
        "@-1650%",
        -1650.0,
        CursorUnit::Percent,
        CursorAxis::X,
        |b| Ok(Some(b.current.0 + -1650.0 * (b.font_height / 100.0)))
    ),
    relative_case!(
        Row::Relative,
        "@40%",
        40.0,
        CursorUnit::Percent,
        CursorAxis::Y,
        |b| { Ok(Some(b.current.1 + 40.0 * (b.font_height / 100.0))) }
    ),
    // 中央指定（正しい軸）。基準はバルーン画像そのもの（文字描画開始点でも範囲でもない）。
    Case {
        row: Row::CenterXOnX,
        written: "centerx",
        coord: CursorCoord::CenterX,
        axis: CursorAxis::X,
        expect: |b| Ok(Some(b.image_size.0 / 2.0)),
    },
    Case {
        row: Row::CenterYOnY,
        written: "centery",
        coord: CursorCoord::CenterY,
        axis: CursorAxis::Y,
        expect: |b| Ok(Some(b.image_size.1 / 2.0)),
    },
    // 中央指定の軸取り違え（両向き）。
    Case {
        row: Row::CenterAxisMismatch,
        written: "centerx",
        coord: CursorCoord::CenterX,
        axis: CursorAxis::Y,
        expect: |_| Err(CursorDegrade::CenterAxisMismatch),
    },
    Case {
        row: Row::CenterAxisMismatch,
        written: "centery",
        coord: CursorCoord::CenterY,
        axis: CursorAxis::X,
        expect: |_| Err(CursorDegrade::CenterAxisMismatch),
    },
    // 解釈不能（両軸）。非有限は語彙層で既に `Invalid` へ落ちている。
    Case {
        row: Row::Unparsable,
        written: "3px",
        coord: CursorCoord::Invalid,
        axis: CursorAxis::X,
        expect: |_| Err(CursorDegrade::Unparsable),
    },
    Case {
        row: Row::Unparsable,
        written: "3px",
        coord: CursorCoord::Invalid,
        axis: CursorAxis::Y,
        expect: |_| Err(CursorDegrade::Unparsable),
    },
];

/// 解決表の全行 × 両軸を、**2 つの基点束**で通す（R9.1/9.3/9.4）。
///
/// 2 周目の [`discriminating_basis`] は 3 基点が両軸とも相異なるので、共通前提の
/// `image_size.0 / 2 == current.0`（＝200）という偶然の一致に隠れる取り違えを弁別する。
#[test]
fn resolution_table_holds_on_every_row_and_axis() {
    for b in [basis(), discriminating_basis()] {
        for case in CASES {
            assert_eq!(
                resolve_cursor_axis(case.coord, case.axis, &b),
                (case.expect)(&b),
                "解決表 {:?}（`{}` を {:?} 軸に・基点束 {:?}）",
                case.row,
                case.written,
                case.axis,
                b
            );
        }
    }
}

// ── §2 被覆の機械的主張 ──

/// [`Row`] → [`ALL_ROWS`] の添字。
///
/// **網羅 `match`** なので [`Row`] に variant を足すとここがコンパイルエラーになり、
/// [`ALL_ROWS`] への足し忘れが構造で塞がれる（被覆主張の入口が黙って縮まない）。
fn row_index(row: Row) -> usize {
    match row {
        Row::Omitted => 0,
        Row::AbsolutePx => 1,
        Row::AbsoluteEm => 2,
        Row::AbsoluteLh => 3,
        Row::AbsolutePercent => 4,
        Row::Relative => 5,
        Row::CenterXOnX => 6,
        Row::CenterYOnY => 7,
        Row::CenterAxisMismatch => 8,
        Row::Unparsable => 9,
    }
}

/// 表の各行が「1 つ以上のケースを持つ」ことを機械で示す（タスク 3.3 の完了条件）。
///
/// 「全部書いた」で終わらせず、行・軸・単位の 3 方向から取りこぼしを赤にする。
#[test]
fn every_resolution_table_row_is_covered_on_both_axes() {
    // 被覆主張の母集合そのものが縮んでいないこと（`Row` の全 variant が過不足なく並ぶ）。
    assert_eq!(ALL_ROWS.len(), 10, "解決表は 10 行");
    for (i, row) in ALL_ROWS.iter().enumerate() {
        assert_eq!(
            row_index(*row),
            i,
            "ALL_ROWS の並びが `Row` の全 variant と一致しない"
        );
    }
    for row in ALL_ROWS {
        assert!(
            CASES.iter().any(|c| c.row == *row),
            "解決表 {row:?} に対応するケースが無い"
        );
    }
    for row in AXIS_SYMMETRIC_ROWS {
        for axis in [CursorAxis::X, CursorAxis::Y] {
            assert!(
                CASES.iter().any(|c| c.row == *row && c.axis == axis),
                "解決表 {row:?} の {axis:?} 軸のケースが無い"
            );
        }
    }
    // 片軸のみの 2 行は、逆側が軸取り違えとして被覆されていること。
    assert!(
        CASES
            .iter()
            .any(|c| c.coord == CursorCoord::CenterX && c.axis == CursorAxis::Y),
        "`centerx` を Y 軸に書いたケースが無い"
    );
    assert!(
        CASES
            .iter()
            .any(|c| c.coord == CursorCoord::CenterY && c.axis == CursorAxis::X),
        "`centery` を X 軸に書いたケースが無い"
    );
    // 単位は絶対・相対の双方 × 両軸で存在すること（R1.1/1.4）。
    for unit in ALL_UNITS {
        for axis in [CursorAxis::X, CursorAxis::Y] {
            assert!(
                CASES.iter().any(|c| c.axis == axis
                    && matches!(c.coord, CursorCoord::Absolute { unit: u, .. } if u == *unit)),
                "絶対 {unit:?} の {axis:?} 軸のケースが無い"
            );
            assert!(
                CASES.iter().any(|c| c.axis == axis
                    && matches!(c.coord, CursorCoord::Relative { unit: u, .. } if u == *unit)),
                "相対 {unit:?} の {axis:?} 軸のケースが無い"
            );
        }
    }
}

// ── §3 単位は軸に依らない（R1.4） ──

/// 同じ `@Nlh` を X と Y に与えると、**基点からの移動量が等しい**（R1.4）。
///
/// 絶対側（基点＝`origin`）は姉妹モジュール `cursor_tag_tests.rs` の
/// `unit_coefficient_is_a_scalar_that_does_not_depend_on_the_axis` が持つ。ここは相対側
/// （基点＝`current`）で同じ性質を締める——基点が変わっても係数は軸に依らない。
#[test]
fn relative_line_pitch_moves_the_same_amount_on_both_axes() {
    let b = discriminating_basis();
    let coord = CursorCoord::Relative {
        value: -2.0,
        unit: CursorUnit::Lh,
    };
    let x = resolve_cursor_axis(coord, CursorAxis::X, &b)
        .expect("実導出（縮退しない）")
        .expect("移動が成立する");
    let y = resolve_cursor_axis(coord, CursorAxis::Y, &b)
        .expect("実導出（縮退しない）")
        .expect("移動が成立する");
    assert_eq!(x - b.current.0, -2.0 * b.line_pitch);
    assert_eq!(y - b.current.1, -2.0 * b.line_pitch);
    assert_eq!(x - b.current.0, y - b.current.1);
}

// ── §4 境界値（R9.4） ──

/// `0`・`-0` は基点そのもの、`@0`・`@-0` は現在位置そのもの（両軸）。
///
/// 基点が相異なる [`discriminating_basis`] を使う——共通前提では X の `@0`（＝200）が
/// 画像中央と同値になり、`Relative` を中央指定と取り違えた実装を弁別できない。
#[test]
fn zero_and_negative_zero_land_exactly_on_the_basepoint() {
    let b = discriminating_basis();
    for value in [0.0_f32, -0.0_f32] {
        let absolute = CursorCoord::Absolute {
            value,
            unit: CursorUnit::Px,
        };
        assert_eq!(
            resolve_cursor_axis(absolute, CursorAxis::X, &b),
            Ok(Some(b.origin.0)),
            "絶対 {value} は文字描画開始点そのもの（X）"
        );
        assert_eq!(
            resolve_cursor_axis(absolute, CursorAxis::Y, &b),
            Ok(Some(b.origin.1)),
            "絶対 {value} は文字描画開始点そのもの（Y）"
        );
        let relative = CursorCoord::Relative {
            value,
            unit: CursorUnit::Px,
        };
        assert_eq!(
            resolve_cursor_axis(relative, CursorAxis::X, &b),
            Ok(Some(b.current.0)),
            "相対 @{value} は現在の文字描画位置そのもの（X）"
        );
        assert_eq!(
            resolve_cursor_axis(relative, CursorAxis::Y, &b),
            Ok(Some(b.current.1)),
            "相対 @{value} は現在の文字描画位置そのもの（Y）"
        );
    }
}

/// 小数は丸めずそのまま効く（`0.5` px と `-0.5em`）。
#[test]
fn fractional_values_are_not_rounded() {
    let b = basis();
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: 0.5,
                unit: CursorUnit::Px,
            },
            CursorAxis::Y,
            &b
        ),
        Ok(Some(b.origin.1 + 0.5))
    );
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: -0.5,
                unit: CursorUnit::Em,
            },
            CursorAxis::X,
            &b
        ),
        Ok(Some(b.origin.0 + -0.5 * b.font_height))
    );
}

/// 大きな負値は**字義どおり**返り（内側へ寄せない・R2.6）、範囲外として `debug` 1 件が残る。
///
/// 「値を返すこと」と「記録すること」は別の口である——解決は値を歪めず、記録は位置を動かさない。
#[test]
fn large_negative_value_is_returned_literally_and_noted_as_out_of_range() {
    let b = basis();
    let resolved = resolve_cursor_axis(
        CursorCoord::Absolute {
            value: -10_000.0,
            unit: CursorUnit::Px,
        },
        CursorAxis::X,
        &b,
    )
    .expect("実導出（縮退しない）")
    .expect("移動が成立する");
    assert_eq!(resolved, b.origin.0 + -10_000.0, "クランプしない");

    // region は捕捉窓の外で組む（`TextRegion::resolve` の未宣言 origin 通知が混ざらないように）。
    let region = out_of_range_region();
    let ((), counts) = count_levels(|| {
        note_out_of_range(CursorAxis::X, resolved, &region);
    });
    assert_eq!(counts.debug, 1, "範囲外は 1 件記録する");
    assert_eq!(counts.warn, 0, "範囲外は警告ではない");
}

// ── §5 正典の記述例（付録 A 逐語・R9.3） ──

/// 正典の記述例と同じ前提（未宣言バルーン＝文字描画開始点が `(0, 0)`）の基点束。
///
/// 記述例の数値（30・50・35・−70）はこの前提でのみ成り立つので、共通前提の宣言 `origin`
/// `(50, 20)` ではなく `(0, 0)` を使う。現在位置は共通前提のまま `(200, 30)`。
fn canon_basis() -> CursorBasis {
    CursorBasis {
        origin: (0.0, 0.0),
        ..basis()
    }
}

/// 正典 付録 A の記述例 3 つを、解決層の単位（1 軸の解決値）へ翻訳して固定する（R9.3）。
///
/// 期待値は正典の日本語（「座標X=30pixel、座標Y=5文字分高さ」等）から書く。配線（横書きで
/// 実際に文字が落ちる位置）の検証は design.md 検証表 H4 の担当である。
#[test]
fn canonical_examples_resolve_to_the_documented_numbers() {
    let b = canon_basis();

    // `\_l[30,5em]`＝「座標X=30pixel、座標Y=5文字分高さ」。
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: 30.0,
                unit: CursorUnit::Px,
            },
            CursorAxis::X,
            &b
        ),
        Ok(Some(30.0))
    );
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Em,
            },
            CursorAxis::Y,
            &b
        ),
        Ok(Some(5.0 * FONT_HEIGHT)),
        "5em＝文字高さ 5 個ぶん＝50"
    );

    // `\_l[@-1650%,100]`＝「座標X=最後の文字から文字高さ1650%分左、座標Y=100pixel」。
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: -1650.0,
                unit: CursorUnit::Percent,
            },
            CursorAxis::X,
            &b
        ),
        Ok(Some(CURRENT.0 - 165.0)),
        "1650%＝文字高さ 16.5 個ぶん＝165 を現在位置から左へ"
    );
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: 100.0,
                unit: CursorUnit::Px,
            },
            CursorAxis::Y,
            &b
        ),
        Ok(Some(100.0))
    );

    // `\_l[,@-100]`＝「座標X=変更なし、座標Y=最後の文字から100pixel上」。
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Omitted, CursorAxis::X, &b),
        Ok(None),
        "X は変更なし"
    );
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: -100.0,
                unit: CursorUnit::Px,
            },
            CursorAxis::Y,
            &b
        ),
        Ok(Some(CURRENT.1 - 100.0))
    );
}

/// 縦書きの記述例 2 つ（`\_l[@-1lh,0]`／`\_l[,@1em]`・R9.3/3.6）を解決層の単位で固定する。
///
/// 「1 列ぶん左の列の先頭へ」「字送りを 1 文字ぶん進める」という**意味**は配線層（列の
/// 割り当て）で成立する（design.md 検証表 V3／V4）。解決層が負うのは、X が現在位置から
/// 行送り 1 つぶん減り、Y が現在位置から文字高さ 1 つぶん増える、という数値だけである。
#[test]
fn canonical_vertical_examples_resolve_to_one_pitch_and_one_em() {
    let b = canon_basis();
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: -1.0,
                unit: CursorUnit::Lh,
            },
            CursorAxis::X,
            &b
        ),
        Ok(Some(CURRENT.0 - LINE_PITCH)),
        "`@-1lh`＝現在位置から行送り 1 つぶん左"
    );
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: 1.0,
                unit: CursorUnit::Em,
            },
            CursorAxis::Y,
            &b
        ),
        Ok(Some(CURRENT.1 + FONT_HEIGHT)),
        "`@1em`＝現在位置から文字高さ 1 つぶん下"
    );
    // `\_l[@-1lh,0]` の Y 側（絶対 0）は文字描画開始点＝列の先頭。
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: 0.0,
                unit: CursorUnit::Px,
            },
            CursorAxis::Y,
            &b
        ),
        Ok(Some(b.origin.1)),
        "`0`＝文字描画開始点そのもの（列の先頭）"
    );
}

// ── §6 ログ件数（縮退表の行 → 件数） ──
//
// 縮退表（design.md「Error Handling ／ 縮退表」）の各行との対応:
//
// | 縮退表の行 | 本ファイルの檻 |
// |---|---|
// | 軸省略＝当該軸不動・ログなし | `resolution_is_silent`・`degrade_warns_once_per_branch_and_actor` |
// | 両軸省略／両軸縮退＝完全無効果（`debug`） | 解決層は両軸とも「動かさない」を返すことまで（`CASES` の省略・縮退行）。`debug` は配線層が出す |
// | 負値絶対＝実導出・ログなし | `CASES` の `-40`／`-1.5em`／`-1lh` ＋ `resolution_is_silent` |
// | `%`／`@` 相対＝実導出・ログなし | `CASES` の `%`／`@` 8 行 ＋ `resolution_is_silent` |
// | `centerx`／`centery`（正しい軸）＝実導出・ログなし | `CASES` の 2 行 ＋ `resolution_is_silent` |
// | 軸取り違え＝当該軸不動・`warn`（初回のみ） | `degrade_warns_once_per_branch_and_actor` |
// | 解釈不能＝当該軸不動・`warn`（初回のみ） | 同上 |
// | 範囲外＝`debug`（一回化しない） | `large_negative_value_is_returned_literally_and_noted_as_out_of_range`＋姉妹モジュール |

/// **解決そのものは 1 件もログを出さない**（省略・実導出・縮退のいずれでも）。
///
/// 記録は別の 2 口（`warn_cursor_degrade`／`note_out_of_range`）の責務である、という
/// 役割分担を件数で固定する。解決の中に記録が紛れ込むと、走査のたびに同じ行が積もる。
#[test]
fn resolution_is_silent() {
    let b = basis();
    let ((), counts) = count_levels(|| {
        for case in CASES {
            let _ = resolve_cursor_axis(case.coord, case.axis, &b);
        }
    });
    assert_eq!(counts.error, 0, "縮退は致命ではない（R5.1）");
    assert_eq!(counts.warn, 0, "解決は警告しない（記録は別の口）");
    assert_eq!(counts.debug, 0, "解決は DEBUG も出さない");
}

/// 配線層の契約（`Err` のときだけ警告する）を 1 ケースぶん再現する。
///
/// 配線そのもの（`layout.rs`）はタスク 4.1 の担当なので、ここでは契約だけを写して件数を測る。
fn warn_if_degraded(case: &Case, b: &CursorBasis, actor: &ActorKey, guard: &mut CursorWarnGuard) {
    if let Err(degrade) = resolve_cursor_axis(case.coord, case.axis, b) {
        warn_cursor_degrade(actor, case.axis, case.coord, degrade, guard);
    }
}

/// 配線の契約を表の全ケースで通したときの件数（R5.2/5.3/5.5）。
///
/// 表の 26 ケースのうち縮退は 4 件（軸取り違え 2・解釈不能 2）だが、一回化の鍵が
/// `(キャラクター, 分岐)` なので同一キャラクターでは **2 件**にまとまる。別キャラクターでは
/// 再び 2 件。省略と実導出（22 ケース）は 1 件も出さない。
///
/// ケースの仕分けは**期待値の表**（`case.expect`）で行う——実装の戻り値で仕分けると
/// 「実装が縮退と呼んだものだけを縮退と数える」同義反復になる。
#[test]
fn degrade_warns_once_per_branch_and_actor() {
    let a0 = ActorKey::from("0");
    let a1 = ActorKey::from("1");
    let b = basis();
    let mut guard = CursorWarnGuard::default();

    let derived: Vec<&Case> = CASES.iter().filter(|c| (c.expect)(&b).is_ok()).collect();
    let degraded: Vec<&Case> = CASES.iter().filter(|c| (c.expect)(&b).is_err()).collect();
    assert_eq!(derived.len(), 22, "省略と実導出のケース数（表の仕分け）");
    assert_eq!(
        degraded.len(),
        4,
        "縮退のケース数（軸取り違え 2・解釈不能 2）"
    );

    // 省略と実導出だけを通す＝0 件（R5.2/5.5）。
    let ((), counts) = count_levels(|| {
        for case in &derived {
            warn_if_degraded(case, &b, &a0, &mut guard);
        }
    });
    assert_eq!(counts.warn, 0, "省略と実導出は警告しない");

    // 縮退 4 ケースを通す＝分岐 2 つぶんの 2 件（軸違い・重複は沈黙）。
    let ((), counts) = count_levels(|| {
        for case in &degraded {
            warn_if_degraded(case, &b, &a0, &mut guard);
        }
    });
    assert_eq!(
        counts.warn, 2,
        "同一キャラクターでは分岐（解釈不能・軸取り違え）ごとに 1 件"
    );

    // 同じキャラクターで表を丸ごと再走査＝沈黙（走査を跨いで持続する）。
    let ((), counts) = count_levels(|| {
        for case in CASES {
            warn_if_degraded(case, &b, &a0, &mut guard);
        }
    });
    assert_eq!(counts.warn, 0, "再走査では追加の警告を出さない");

    // 別キャラクター＝再び 2 件。
    let ((), counts) = count_levels(|| {
        for case in CASES {
            warn_if_degraded(case, &b, &a1, &mut guard);
        }
    });
    assert_eq!(counts.warn, 2, "別キャラクターでは再び分岐ごとに 1 件");
}

/// 共通前提の定数が design.md「Unit Tests」の逐語どおりであること（檻の前提を檻にする）。
///
/// ここが黙って変わると、上の全ケースの期待値が「実装に合わせて動く」ようになる。
#[test]
fn shared_premises_match_the_design_document() {
    assert_eq!(FONT_HEIGHT, 10.0, "font_height = 10");
    assert_eq!(LINE_PITCH, 12.0, "line_pitch = 12");
    assert_eq!(IMAGE_SIZE, (400.0, 224.0), "image_size = (400, 224)");
    assert_eq!(ORIGIN, (50.0, 20.0), "origin の宣言例 = (50, 20)");
    assert_eq!(CURRENT, (200.0, 30.0), "current = (200, 30)");
}

/// 弁別用の基点束は**3 基点が両軸とも相異なる**こと（`discriminating_basis` の存在理由）。
///
/// 共通前提は `image_size.0 / 2 == current.0`（＝200）という偶然の一致を持つので、その値の
/// ままでは「画像中央」と「現在の文字描画位置」を取り違えた実装が X 軸で素通りする。
/// `discriminating_basis` はこの一致を外すためだけに在るのだが、その**性質そのもの**を
/// 主張しないと、画像原寸を共通前提と同値へ戻すだけで、これに依る 5 本の檻
/// （`resolution_table_holds_on_every_row_and_axis`・
/// `relative_line_pitch_moves_the_same_amount_on_both_axes`・
/// `zero_and_negative_zero_land_exactly_on_the_basepoint`・姉妹モジュールの
/// `centerx_on_x_resolves_to_half_the_image_width`／`centery_on_y_resolves_to_half_the_image_height`）
/// が黙って弁別力を失う。
///
/// 値の逐語固定ではなく**性質**を書くのは、「なぜその値なのか」を檻に残すためである。
#[test]
fn the_discriminating_basis_keeps_all_three_basepoints_apart() {
    let d = discriminating_basis();
    for (axis, origin, current, half) in [
        ("X", d.origin.0, d.current.0, d.image_size.0 / 2.0),
        ("Y", d.origin.1, d.current.1, d.image_size.1 / 2.0),
    ] {
        assert!(
            origin != current && current != half && origin != half,
            "弁別用の基点束は 3 基点が相異なること（{axis} 軸: 文字描画開始点 {origin}・             現在の文字描画位置 {current}・画像中央 {half}）"
        );
    }
    // 共通前提の側は一致を**持っている**こと（上の主張が空回りしていない対照）。
    assert_eq!(
        IMAGE_SIZE.0 / 2.0,
        CURRENT.0,
        "共通前提は X 軸で画像中央と現在位置が一致する——だから弁別用の基点束が要る"
    );
}
