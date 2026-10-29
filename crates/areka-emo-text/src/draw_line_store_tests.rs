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
//! | §5 | 列のレシピ（L1 縦書きだけ行間と機能の組・L4 合成の絵文字が崩れない） | 1.7, 2.2, 5.1, 5.5 |
//!
//! §4 の較正は実装を実際に摂動して赤になることを確かめてある（実測 2026-09-12・
//! 摂動は取り消し済み）。

use std::cell::{Cell, RefCell};

use windows::Win32::Foundation::E_INVALIDARG;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_GLYPH_ORIENTATION_ANGLE, DWRITE_GLYPH_RUN,
    DWRITE_GLYPH_RUN_DESCRIPTION, DWRITE_LINE_SPACING_METHOD, DWRITE_LINE_SPACING_METHOD_DEFAULT,
    DWRITE_LINE_SPACING_METHOD_UNIFORM, DWRITE_MATRIX, DWRITE_MEASURING_MODE, DWRITE_STRIKETHROUGH,
    DWRITE_TEXT_RANGE, DWRITE_UNDERLINE, IDWriteFactory2, IDWriteInlineObject,
    IDWritePixelSnapping_Impl, IDWriteTextFormat, IDWriteTextLayout, IDWriteTextRenderer_Impl,
    IDWriteTextRenderer1, IDWriteTextRenderer1_Impl,
};
use windows::core::HSTRING;
use windows_core::{AsImpl, BOOL, IUnknown, Ref, implement};
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt, dwrite_create_factory};

use super::super::test_support::{empty_font, model_with_font};
use super::super::{
    PROBE_MAX_EXTENT, ResolvedFont, VERTICAL_FEATURES, create_line_layout, create_text_format,
};
use super::LineLayoutStore;
use crate::look::StyleId;
use crate::writing::WritingMode;

/// 判定に使う行の内容（既定フォントで確実に組める短い日本語）。
const LINE: &str = "あいう";
/// 行送り軸の箱寸（既定フォント高さ）。
const BLOCK_EXTENT: f32 = 12.0;

/// 既定フォント（ＭＳ ゴシック 12）の factory ＋ format を組む。
fn factory_and_format() -> (IDWriteFactory2, IDWriteTextFormat) {
    factory_and_format_for(WritingMode::HorizontalTb)
}

/// 既定フォント（ＭＳ ゴシック 12）の factory ＋ 書字方向 `mode` の format を組む。
fn factory_and_format_for(mode: WritingMode) -> (IDWriteFactory2, IDWriteTextFormat) {
    let factory =
        dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory が作れる");
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    let format =
        create_text_format(&factory, &resolved, mode).expect("既定フォントの TextFormat が作れる");
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

/// 装飾なしの入口の生成物は TextLayout の唯一の生成口（[`create_line_layout`]）で直に組んだ行と
/// 同一——クラスタ送り幅もはみ出しも一致し、箱寸は `(PROBE_MAX_EXTENT, font_height)`。
///
/// 突き合わせ先は保持庫を通らない生成口の直呼び（保持庫の再利用・焼く処理・はみ出しの合わせが
/// 生成物へ触っていないことを見る）。箱寸は字義で別に判定する（生成口の場合分けの取り違えを見る）。
#[test]
fn line_layout_produces_the_same_line_as_a_bare_factory_call() {
    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);

    let reference = create_line_layout(
        &factory,
        LINE,
        &format,
        BLOCK_EXTENT,
        WritingMode::HorizontalTb,
    )
    .expect("生成口の直呼びで行が組める");
    let expected_advances = advances(&reference);
    let expected_overhang = super::measure_line_overhang(&reference).expect("はみ出しが測れる");

    let produced = store
        .line_layout(0, LINE, &format, BLOCK_EXTENT, WritingMode::HorizontalTb)
        .expect("装飾なしの生成が成立する");

    assert_eq!(
        advances(&produced),
        expected_advances,
        "装飾なしの入口の生成物は生成口の直呼びと同じ送り幅"
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

// ────────────────────────── §5 列のレシピ（L1・L4・要件 1.7／2.2／5.1／5.5）

/// 装飾のある行の箱の幅（行内最大 em）——既定の高さ 12 と別の値にして、行間が呼び手の
/// `block_extent` から来ることを見分ける。
const DECORATED_EXTENT: f32 = 20.0;

/// 機能の組の期待値（名前から組む＝定数の中身の取り違えも赤にする）。
const EXPECTED_FEATURES: [&[u8; 4]; 10] = [
    b"ccmp", b"locl", b"mark", b"mkmk", b"rlig", b"calt", b"clig", b"liga", b"kern", b"rclt",
];

/// 装飾のある行の番号列（2 字目だけ装飾）。
const DECORATED_IDS: [StyleId; 3] = [StyleId::DEFAULT, StyleId(1), StyleId::DEFAULT];

/// 行の行間の指定（方式・行の幅・基線）。
fn line_spacing(layout: &IDWriteTextLayout) -> (DWRITE_LINE_SPACING_METHOD, f32, f32) {
    let (mut method, mut spacing, mut baseline) = (DWRITE_LINE_SPACING_METHOD_DEFAULT, 0.0, 0.0);
    unsafe { layout.GetLineSpacing(&mut method, &mut spacing, &mut baseline) }
        .expect("行間が読める");
    (method, spacing, baseline)
}

/// 先頭の字に当たる typography の機能の組（札・値）。typography 無しは `None`。
fn features_at_start(layout: &IDWriteTextLayout) -> Option<Vec<(u32, u32)>> {
    let mut typography = None;
    unsafe { layout.GetTypography(0, &mut typography, None) }.expect("typography が読める");
    typography.map(|t| {
        (0..unsafe { t.GetFontFeatureCount() })
            .map(|i| {
                let f = unsafe { t.GetFontFeature(i) }.expect("機能が読める");
                (f.nameTag.0, f.parameter)
            })
            .collect()
    })
}

/// 装飾のある行を作る（2 字目の大きさを行内最大 em へ焼く＝生成の後の範囲指定）。
fn decorated_line(
    store: &mut LineLayoutStore,
    format: &IDWriteTextFormat,
    mode: WritingMode,
) -> IDWriteTextLayout {
    store
        .line_layout_decorated(
            1,
            LINE,
            format,
            DECORATED_EXTENT,
            mode,
            &DECORATED_IDS,
            |layout| {
                let range = DWRITE_TEXT_RANGE {
                    startPosition: 1,
                    length: 1,
                };
                unsafe { layout.SetFontSize(DECORATED_EXTENT, range) }
                    .map_err(super::super::device_err("SetFontSize(test)"))
            },
        )
        .expect("装飾のある行が組める")
}

/// L1: 縦書き 2 方向の行（装飾なし・装飾あり）は行間が（UNIFORM・箱の幅・その半分）で、先頭の字に
/// 10 個の機能（値 1・`vpal`・`vert`・`vkrn` を含まない）を持つ。横書きの行は typography 無し・行間は既定。
#[test]
fn only_vertical_lines_carry_the_column_recipe() {
    let expected: Vec<(u32, u32)> = EXPECTED_FEATURES
        .iter()
        .map(|name| (u32::from_le_bytes(**name), 1))
        .collect();
    let excluded = [b"vpal", b"vert", b"vkrn"].map(|name| u32::from_le_bytes(*name));
    assert!(
        VERTICAL_FEATURES
            .iter()
            .all(|tag| !excluded.contains(&tag.0)),
        "機能の組は vpal・vert・vkrn を含まない"
    );

    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let (factory, format) = factory_and_format_for(mode);
        let mut store = LineLayoutStore::new(&factory);
        let plain = store
            .line_layout(0, LINE, &format, BLOCK_EXTENT, mode)
            .expect("縦書きの行が組める");
        let decorated = decorated_line(&mut store, &format, mode);

        for (layout, extent, what) in [
            (&plain, BLOCK_EXTENT, "装飾なし"),
            (&decorated, DECORATED_EXTENT, "装飾あり"),
        ] {
            assert_eq!(
                line_spacing(layout),
                (DWRITE_LINE_SPACING_METHOD_UNIFORM, extent, extent / 2.0),
                "{mode:?} {what}: 行間は UNIFORM・行の箱の幅・その半分"
            );
            assert_eq!(
                features_at_start(layout).as_deref(),
                Some(expected.as_slice()),
                "{mode:?} {what}: 先頭の字に 10 個の機能が値 1 で当たる"
            );
        }
    }

    let (factory, format) = factory_and_format();
    let mut store = LineLayoutStore::new(&factory);
    let plain = store
        .line_layout(0, LINE, &format, BLOCK_EXTENT, WritingMode::HorizontalTb)
        .expect("横書きの行が組める");
    let decorated = decorated_line(&mut store, &format, WritingMode::HorizontalTb);
    for (layout, what) in [(&plain, "装飾なし"), (&decorated, "装飾あり")] {
        assert_eq!(
            features_at_start(layout),
            None,
            "横書き {what}: typography 無し"
        );
        assert_eq!(
            line_spacing(layout).0,
            DWRITE_LINE_SPACING_METHOD_DEFAULT,
            "横書き {what}: 行間は既定"
        );
    }
}

/// 行の字形の並び（字形番号の列・送り幅の列）を 1 並びずつ集める受け手（L4 の観測用）。
///
/// 縦書きの行は `IDWriteTextRenderer1` を持たない受け手を断るので 2 つの口を持ち、どちらも同じ記録へ流す。
#[implement(IDWriteTextRenderer1)]
struct GlyphRecorder {
    runs: RefCell<Vec<(Vec<u16>, Vec<f32>)>>,
}

impl GlyphRecorder {
    fn record(&self, run: *const DWRITE_GLYPH_RUN) {
        // SAFETY: DirectWrite が描く間だけ有効な並び（null でない・個数は glyphCount）。
        let run = unsafe { &*run };
        let n = run.glyphCount as usize;
        let indices = unsafe { std::slice::from_raw_parts(run.glyphIndices, n) }.to_vec();
        let advances = unsafe { std::slice::from_raw_parts(run.glyphAdvances, n) }.to_vec();
        self.runs.borrow_mut().push((indices, advances));
    }
}

impl IDWritePixelSnapping_Impl for GlyphRecorder_Impl {
    fn IsPixelSnappingDisabled(&self, _c: *const core::ffi::c_void) -> windows_core::Result<BOOL> {
        Ok(BOOL(1))
    }

    fn GetCurrentTransform(
        &self,
        _c: *const core::ffi::c_void,
        transform: *mut DWRITE_MATRIX,
    ) -> windows_core::Result<()> {
        if transform.is_null() {
            return Err(E_INVALIDARG.into());
        }
        // SAFETY: DirectWrite が渡す書き込み先（null は上で弾いた）。
        unsafe {
            transform.write(DWRITE_MATRIX {
                m11: 1.0,
                m22: 1.0,
                ..Default::default()
            })
        };
        Ok(())
    }

    fn GetPixelsPerDip(&self, _c: *const core::ffi::c_void) -> windows_core::Result<f32> {
        Ok(1.0)
    }
}

impl IDWriteTextRenderer_Impl for GlyphRecorder_Impl {
    fn DrawGlyphRun(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _mode: DWRITE_MEASURING_MODE,
        run: *const DWRITE_GLYPH_RUN,
        _d: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        self.record(run);
        Ok(())
    }

    fn DrawUnderline(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _u: *const DWRITE_UNDERLINE,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawStrikethrough(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _s: *const DWRITE_STRIKETHROUGH,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawInlineObject(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _o: Ref<IDWriteInlineObject>,
        _sideways: BOOL,
        _rtl: BOOL,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }
}

impl IDWriteTextRenderer1_Impl for GlyphRecorder_Impl {
    fn DrawGlyphRun(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _mode: DWRITE_MEASURING_MODE,
        run: *const DWRITE_GLYPH_RUN,
        _d: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        self.record(run);
        Ok(())
    }

    fn DrawUnderline(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _u: *const DWRITE_UNDERLINE,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawStrikethrough(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _s: *const DWRITE_STRIKETHROUGH,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn DrawInlineObject(
        &self,
        _c: *const core::ffi::c_void,
        _x: f32,
        _y: f32,
        _angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _o: Ref<IDWriteInlineObject>,
        _sideways: BOOL,
        _rtl: BOOL,
        _e: Ref<IUnknown>,
    ) -> windows_core::Result<()> {
        Ok(())
    }
}

/// 行を描いて字形の並びを集める。
fn glyph_runs(layout: &IDWriteTextLayout) -> Vec<(Vec<u16>, Vec<f32>)> {
    let recorder: IDWriteTextRenderer1 = GlyphRecorder {
        runs: RefCell::new(Vec::new()),
    }
    .into();
    unsafe { layout.Draw(None, &recorder, 0.0, 0.0) }.expect("行が描ける");
    // SAFETY: 上で作った受け手そのもの（GlyphRecorder の実装）。
    let recorded: &GlyphRecorder = unsafe { recorder.as_impl() };
    recorded.runs.borrow().clone()
}

/// L4: 縦書きで、絵文字の ZWJ 列と国旗の字形番号と送り幅が、typography 無しの既定（生成口を通らない
/// factory の直呼び）と列のレシピで一致する（明示した 10 個の機能で合成の絵文字が崩れない）。
#[test]
fn the_column_recipe_keeps_composed_emoji_glyphs() {
    const EMOJI: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{1F1EF}\u{1F1F5}";
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let (factory, format) = factory_and_format_for(mode);
        let recipe = create_line_layout(&factory, EMOJI, &format, BLOCK_EXTENT, mode)
            .expect("列のレシピの行が組める");
        assert!(
            features_at_start(&recipe).is_some(),
            "{mode:?}: 比べる側に列のレシピが当たっている（恒真の突き合わせにしない）"
        );
        let bare = factory
            .create_text_layout(
                &HSTRING::from(EMOJI),
                &format,
                BLOCK_EXTENT,
                PROBE_MAX_EXTENT,
            )
            .expect("typography 無しの行が組める");
        let expected = glyph_runs(&bare);
        // 合成の度合い（字形の数）は OS の代替フォント次第なので固定せず、既定と同じことだけを見る
        // （実測 2026-10-29: 縦横とも 3＋2 字形）。
        assert!(
            expected.iter().any(|(g, _)| !g.is_empty()),
            "{mode:?}: 既定の行に字形が在る（観測の前提）"
        );
        assert_eq!(
            glyph_runs(&recipe),
            expected,
            "{mode:?}: 字形番号と送り幅が typography 無しと一致"
        );
    }
}
