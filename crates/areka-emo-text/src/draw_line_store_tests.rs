//! [`LineLayoutStore`] の装飾込み再利用判定（タスク 6.3・要件 11.4／3.6）。
//!
//! 行レイアウトの保持庫は「内容文字列」と「装飾番号の列」の**両方**が同じときだけ
//! 作り直さない。装飾を焼く処理（[`LineLayoutStore::line_layout_decorated`] の
//! `decorate`）は**生成時に 1 度だけ**呼ばれ、再利用のときは呼ばれない。
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | 同じ文字列でも番号列が違えば作り直す／同じなら再利用する | 11.4, 3.6 |
//! | §2 | 焼く処理は生成時に 1 度だけ呼ばれる | 11.4 |
//! | §3 | 装飾なしの入口は空の番号列で委譲し、生成物が従来と同一 | 11.4 |
//! | §4 | 較正（番号列を鍵から外す誤り・焼く処理の回数の誤り） | 11.4 |
//!
//! §4 の較正は実装を実際に摂動して赤になることを確かめてある（実測 2026-09-12・
//! 摂動は取り消し済み）。

use std::cell::Cell;

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory2, IDWriteTextFormat,
};
use windows::core::HSTRING;
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt, dwrite_create_factory};

use super::super::test_support::{empty_font, model_with_font};
use super::super::{PROBE_MAX_EXTENT, ResolvedFont, create_text_format};
use super::LineLayoutStore;
use crate::look::StyleId;
use crate::writing::WritingMode;

/// 判定に使う行の内容（既定フォントで確実に組める短い日本語）。
const LINE: &str = "あいう";
/// 行送り軸の箱寸（既定フォント高さ）。
const BLOCK_EXTENT: f32 = 12.0;

/// 既定フォント（ＭＳ ゴシック 12）の factory ＋ format を組む。
fn factory_and_format() -> (IDWriteFactory2, IDWriteTextFormat) {
    let factory =
        dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory が作れる");
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    let format = create_text_format(&factory, &resolved, WritingMode::HorizontalTb)
        .expect("既定フォントの TextFormat が作れる");
    (factory, format)
}

/// 行のクラスタ送り幅の列（生成物の同一性を字義で突き合わせるための観測量）。
fn advances(layout: &windows::Win32::Graphics::DirectWrite::IDWriteTextLayout) -> Vec<f32> {
    layout
        .get_cluster_metrics()
        .expect("cluster metrics が取れる")
        .iter()
        .map(|c| c.width)
        .collect()
}

// ────────────────────────── §1 再利用の鍵は「文字列」と「番号列」の両方（R11.4）

/// 同じ文字列でも装飾番号の列が違えば作り直す（古い見た目のまま再利用しない）。
#[test]
fn the_same_text_with_a_different_style_id_list_is_rebuilt() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);

    store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &[StyleId::DEFAULT, StyleId::DEFAULT, StyleId::DEFAULT],
            |_| Ok(()),
        )
        .expect("1 度目の生成が成立する");
    assert_eq!(store.creations(), 1, "1 度目は生成される");

    store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &[StyleId::DEFAULT, StyleId(1), StyleId::DEFAULT],
            |_| Ok(()),
        )
        .expect("番号列違いの生成が成立する");
    assert_eq!(
        store.creations(),
        2,
        "文字列が同じでも番号列が違えば作り直す（R11.4）"
    );
}

/// 文字列も番号列も同じなら再利用する（生成回数が増えない）。
#[test]
fn the_same_text_and_the_same_style_id_list_is_reused() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);
    let ids = [StyleId::DEFAULT, StyleId(1), StyleId(2)];

    for _ in 0..3 {
        store
            .line_layout_decorated(
                0,
                LINE,
                &format,
                BLOCK_EXTENT,
                WritingMode::HorizontalTb,
                &ids,
                |_| Ok(()),
            )
            .expect("生成／再利用が成立する");
    }
    assert_eq!(
        store.creations(),
        1,
        "同じ文字列・同じ番号列は 1 度だけ生成される"
    );
}

/// 番号列が同じでも文字列が変われば作り直す（従来の鍵の側が生きている）。
#[test]
fn a_changed_text_with_the_same_style_id_list_is_rebuilt() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);
    let ids = [StyleId(1)];

    store
        .line_layout_decorated(
            0,
            "あ",
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &ids,
            |_| Ok(()),
        )
        .expect("1 度目の生成が成立する");
    store
        .line_layout_decorated(
            0,
            "あい",
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &ids,
            |_| Ok(()),
        )
        .expect("文字列違いの生成が成立する");
    assert_eq!(store.creations(), 2, "文字列が変われば作り直す");
}

// ────────────────────────── §2 焼く処理は生成時に 1 度だけ（R11.4）

/// 焼く処理は生成の度に 1 度だけ呼ばれ、再利用のときは呼ばれない。
#[test]
fn the_decorate_step_runs_exactly_once_per_generation() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);
    let calls = Cell::new(0u32);
    let ids = [StyleId(1)];

    let mut draw = |ids: &[StyleId]| {
        store
            .line_layout_decorated(
                0,
                LINE,
                &format,
                BLOCK_EXTENT,
                WritingMode::HorizontalTb,
                ids,
                |_| {
                    calls.set(calls.get() + 1);
                    Ok(())
                },
            )
            .expect("生成／再利用が成立する");
    };

    draw(&ids);
    assert_eq!(calls.get(), 1, "生成時に 1 度だけ焼く");
    draw(&ids);
    draw(&ids);
    assert_eq!(calls.get(), 1, "再利用では焼かない");
    draw(&[StyleId(2)]);
    assert_eq!(calls.get(), 2, "作り直しでもう 1 度だけ焼く");
    assert_eq!(store.creations(), 2, "焼いた回数＝生成回数");
}

/// 焼く処理の失敗はそのまま呼び手へ返り、失敗した行は保持庫に残らない
/// （次の呼出しで作り直される＝古い見た目を掴まない）。
#[test]
fn a_failing_decorate_step_is_propagated_and_leaves_nothing_cached() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);
    let ids = [StyleId(1)];

    let failed = store.line_layout_decorated(
        0,
        LINE,
        &format,
        BLOCK_EXTENT,
        WritingMode::HorizontalTb,
        &ids,
        |_| {
            Err(crate::TextLayerError::Device {
                hresult: -1,
                context: "テスト用の失敗",
            })
        },
    );
    assert!(failed.is_err(), "焼く処理の失敗は呼び手へ返る");
    assert_eq!(store.overhang(0), None, "失敗した行は保持庫に残らない");

    store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &ids,
            |_| Ok(()),
        )
        .expect("次の呼出しで作り直される");
    assert_eq!(store.creations(), 2, "失敗ぶんも生成回数には数える");
}

// ────────────────────────── §3 装飾なしの入口は空の番号列で委譲（R11.4）

/// `line_layout` は空の番号列で保持庫へ入る——だから空の番号列の
/// `line_layout_decorated` はそれを再利用し、番号列が付けば作り直す。
#[test]
fn line_layout_stores_the_line_under_an_empty_style_id_list() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);

    store
        .line_layout(0, LINE, &format, BLOCK_EXTENT, WritingMode::HorizontalTb)
        .expect("装飾なしの生成が成立する");
    assert_eq!(store.creations(), 1);

    store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &[],
            |_| Ok(()),
        )
        .expect("空の番号列で再利用される");
    assert_eq!(
        store.creations(),
        1,
        "装飾なしの入口が置いた鍵の番号列は空（空同士は再利用）"
    );

    store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &[StyleId(1)],
            |_| Ok(()),
        )
        .expect("番号列が付けば作り直す");
    assert_eq!(store.creations(), 2, "空でない番号列は別の鍵");
}

/// 装飾なしの入口の生成物は従来と同一——箱寸 `(PROBE_MAX_EXTENT, font_height)` で
/// factory から直に組んだ行と、クラスタ送り幅もはみ出しも一致する。
///
/// 期待値は委譲先の関数からではなく **factory の直呼び**から採る（同じ関数へ委譲すると
/// 突き合わせが恒真になるため・Implementation Notes 5.1）。
#[test]
fn line_layout_produces_the_same_line_as_a_bare_factory_call() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);

    let reference = factory
        .create_text_layout(
            &HSTRING::from(LINE),
            &format,
            PROBE_MAX_EXTENT,
            BLOCK_EXTENT,
        )
        .expect("直呼びの行が組める");
    let expected_advances = advances(&reference);
    let expected_overhang = super::measure_line_overhang(&reference).expect("はみ出しが測れる");

    let produced = store
        .line_layout(0, LINE, &format, BLOCK_EXTENT, WritingMode::HorizontalTb)
        .expect("装飾なしの生成が成立する");

    assert_eq!(
        advances(&produced),
        expected_advances,
        "装飾なしの入口の生成物は箱寸 (PROBE_MAX_EXTENT, font_height) の直呼びと同じ送り幅"
    );
    assert_eq!(expected_advances.len(), 3, "3 文字ぶんのクラスタが在る");
    assert_eq!(
        store.overhang(0),
        Some(expected_overhang),
        "はみ出しも直呼びと同じ（焼く処理が生成物へ触っていない）"
    );
    // 箱寸そのものを字義で判定する（送り幅とはみ出しだけでは行送り軸の箱寸の取り違えを
    // 見逃す——実測で `font_height * 2.0` の摂動が両者を素通りした・2026-09-12）。
    assert_eq!(
        (unsafe { produced.GetMaxWidth() }, unsafe {
            produced.GetMaxHeight()
        }),
        (PROBE_MAX_EXTENT, BLOCK_EXTENT),
        "横書きの箱寸は（折返し無効寸, font_height）"
    );
}

/// 装飾なしの入口の縦書きも従来と同一——箱寸は軸が入れ替わる。
#[test]
fn line_layout_keeps_the_vertical_box_extents() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);

    let produced = store
        .line_layout(0, LINE, &format, BLOCK_EXTENT, WritingMode::VerticalRl)
        .expect("装飾なしの生成が成立する");

    assert_eq!(
        (unsafe { produced.GetMaxWidth() }, unsafe {
            produced.GetMaxHeight()
        }),
        (BLOCK_EXTENT, PROBE_MAX_EXTENT),
        "縦書きの箱寸は（font_height, 折返し無効寸）"
    );
}

// ────────────────────────── §4 較正（R11.4）

/// 較正の対照: 番号列だけが違う 2 本は**別の行**として生成されるので、
/// 生成回数は呼出回数と同数になる（鍵から番号列を外すと 1 になり赤）。
#[test]
fn every_distinct_style_id_list_costs_one_generation() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);

    for id in 0..4u32 {
        store
            .line_layout_decorated(
                0,
                LINE,
                &format,
                BLOCK_EXTENT,
                WritingMode::HorizontalTb,
                &[StyleId(id)],
                |_| Ok(()),
            )
            .expect("生成が成立する");
    }
    assert_eq!(
        store.creations(),
        4,
        "番号列が 4 通りなら生成も 4 回（文字列だけで再利用すると 1 になる）"
    );
}

/// 較正の対照: 番号列の**長さだけ**が違う場合も別の鍵（前方一致で済ませる誤りを赤にする）。
#[test]
fn a_longer_style_id_list_with_the_same_prefix_is_a_different_key() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);

    store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &[StyleId(1)],
            |_| Ok(()),
        )
        .expect("生成が成立する");
    store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            BLOCK_EXTENT,
            WritingMode::HorizontalTb,
            &[StyleId(1), StyleId(1)],
            |_| Ok(()),
        )
        .expect("生成が成立する");
    assert_eq!(store.creations(), 2, "長さが違えば別の鍵");
}
