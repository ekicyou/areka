use areka_parsers::balloon::{Font, FontColor};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
    DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT, DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
    DWRITE_READING_DIRECTION_TOP_TO_BOTTOM, DWRITE_TEXT_ALIGNMENT_LEADING, IDWriteTextFormat,
};
use wintf::com::dwrite::dwrite_create_factory;

use super::test_support::{default_metrics, empty_font, model_with_font, with_log_cage};
use super::{
    DEFAULT_BALLOON_BACKGROUND, DEFAULT_FONT_HEIGHT, DEFAULT_FONT_NAME, DWriteMetrics,
    DirectionRecipe, PROBE_MAX_EXTENT, ResolvedFont, create_text_format,
};
use crate::TextLayerError;
use crate::canvas::TextEffects;
use crate::color::mix_disabled;
use crate::layout::GlyphMetrics;
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;

// ── R4.1/R4.2: フォント解決とフォールバック（純粋部・COM 不要） ──

/// 観測可能な完了状態: フォント名/高さが欠落した balloon 定義に対しても
/// 既定値でレイアウト生成に必要な設定一式が得られる（ukadoc 既定・正常系につき警告なし）。
#[test]
fn missing_font_definition_resolves_to_ukadoc_defaults() {
    let (font, warns, errors) =
        with_log_cage(|| ResolvedFont::resolve(&model_with_font(empty_font())));
    assert_eq!(font.name, DEFAULT_FONT_NAME);
    assert_eq!(
        font.name, "ＭＳ ゴシック",
        "既定フォント名は全角 ＭＳ ゴシック"
    );
    assert_eq!(font.height, DEFAULT_FONT_HEIGHT);
    assert_eq!(
        font.height, 12.0,
        "既定フォント高さは 12（image px・ukadoc 既定）"
    );
    assert_eq!(font.color, (0, 0, 0), "FontColor 欠落→黒");
    assert!(font.fallback_chain.is_empty());
    assert_eq!(
        (warns, errors),
        (0, 0),
        "ukadoc 既定の適用は正常系＝ログなし"
    );
}

#[test]
fn full_font_definition_passes_through() {
    let font = Font::new(
        Some("Meiryo".to_owned()),
        Some(20),
        FontColor::new(Some(10), Some(20), Some(30)),
    );
    let (resolved, warns, _) = with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
    assert_eq!(resolved.name, "Meiryo");
    assert_eq!(resolved.height, 20.0);
    assert_eq!(resolved.color, (10, 20, 30));
    assert!(resolved.fallback_chain.is_empty());
    assert_eq!(warns, 0);
}

/// カンマ区切り複数指定（SSP 拡張）は M1 では先頭のみ採用・残余は型シームに保持。
#[test]
fn comma_separated_names_adopt_first_and_keep_rest_as_seam() {
    let font = Font::new(
        Some("Meiryo, ＭＳ Ｐゴシック ,ＭＳ ゴシック".to_owned()),
        None,
        FontColor::new(None, None, None),
    );
    let resolved = ResolvedFont::resolve(&model_with_font(font));
    assert_eq!(resolved.name, "Meiryo", "先頭名のみ採用（M1）");
    assert_eq!(
        resolved.fallback_chain,
        vec!["ＭＳ Ｐゴシック".to_owned(), "ＭＳ ゴシック".to_owned()],
        "残余は trim 済みでフォールバック連鎖シームに保持（M1 未消費）"
    );
}

/// 宣言はあるが実質空の font.name は縮退（warn＋既定フォント・R4.2 log-first）。
#[test]
fn empty_font_name_falls_back_to_default_with_warn() {
    for raw in ["", "  ", " , "] {
        let font = Font::new(Some(raw.to_owned()), None, FontColor::new(None, None, None));
        let (resolved, warns, _) = with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
        assert_eq!(
            resolved.name, DEFAULT_FONT_NAME,
            "raw {raw:?} は既定フォントへ"
        );
        assert!(resolved.fallback_chain.is_empty());
        assert_eq!(warns, 1, "raw {raw:?} はちょうど 1 回 warn を記録する");
    }
}

/// font.height,0 は DirectWrite fontsize の正値制約を満たせない縮退値（warn＋既定 12）。
#[test]
fn zero_height_falls_back_to_default_with_warn() {
    let font = Font::new(None, Some(0), FontColor::new(None, None, None));
    let (resolved, warns, _) = with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
    assert_eq!(resolved.height, DEFAULT_FONT_HEIGHT);
    assert_eq!(warns, 1);
}

/// font.color は成分独立既定 0（部分欠落→欠落成分のみ 0・ukadoc 既定 0＝正常系）。
#[test]
fn partial_color_channels_default_to_zero() {
    let font = Font::new(None, None, FontColor::new(Some(255), None, Some(7)));
    let (resolved, warns, _) = with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
    assert_eq!(resolved.color, (255, 0, 7));
    assert_eq!(warns, 0);
}

// ── 方向レシピ: writing_mode 解釈結果→DirectWrite 設定の一意導出（design 写像表） ──

/// design.md 写像表どおりの一意導出:
/// HorizontalTb→Reading LTR＋Flow TTB／VerticalRl→Reading TTB＋Flow RTL／
/// VerticalLr→Reading TTB＋Flow LTR。
#[test]
fn direction_recipe_maps_three_modes_per_design_table() {
    let h = DirectionRecipe::for_mode(WritingMode::HorizontalTb);
    assert_eq!(h.reading, DWRITE_READING_DIRECTION_LEFT_TO_RIGHT);
    assert_eq!(h.flow, DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM);

    let vrl = DirectionRecipe::for_mode(WritingMode::VerticalRl);
    assert_eq!(vrl.reading, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM);
    assert_eq!(vrl.flow, DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT);

    let vlr = DirectionRecipe::for_mode(WritingMode::VerticalLr);
    assert_eq!(vlr.reading, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM);
    assert_eq!(vlr.flow, DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT);

    // 3 方向は互いに異なるレシピへ写る（一意導出＝単射）。
    assert_ne!(h, vrl);
    assert_ne!(h, vlr);
    assert_ne!(vrl, vlr);
}

/// いずれの方向も Alignment LEADING＋Paragraph NEAR（design 写像表の共通部）。
#[test]
fn all_direction_recipes_share_leading_near_alignment() {
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let recipe = DirectionRecipe::for_mode(mode);
        assert_eq!(recipe.text_alignment, DWRITE_TEXT_ALIGNMENT_LEADING);
        assert_eq!(recipe.paragraph_alignment, DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
    }
}

// ── R4.4/R4.8/R16.4: 無効表示は実体・行単位の装飾は M2 予約のまま ──

/// 無効表示の層が**実体化**し（色が背景との混色になる）、**行単位**の予約型
/// （[`TextEffects`]）は今も 0 バイトのままであることを固定する
/// （要件 4.1／4.2／4.4／4.8／16.4・design「既存テストの改訂」1）。
///
/// 較正（要件 15.6）: 「無効表示の色を混色せず既定の色のままにする」誤りを入れると
/// 混色の断言と `assert_ne!` が、「背景を受け取らず常に白として混ぜる」誤りを
/// 入れると黒背景の 2 つの断言が赤くなる。
#[test]
fn disable_layer_is_materialized_and_row_effects_stay_reserved() {
    // 行単位の装飾は M2 予約のまま（0 バイト＝描画へ影響し得ない構造保証）。
    assert_eq!(std::mem::size_of::<TextEffects>(), 0);

    // ── バルーン定義なし: 既定は ukadoc 既定（黒）・無効表示は白背景との混色 ──
    let plain = ResolvedFont::resolve(&model_with_font(empty_font()));
    assert_eq!(plain.effects, TextEffects::default());
    assert_eq!(plain.looks.default.color, (0, 0, 0));
    assert_eq!(
        plain.looks.disable.color,
        mix_disabled((0, 0, 0), DEFAULT_BALLOON_BACKGROUND),
        "無効表示の色は「既定の文字色と背景色の混色」（要件 4.6）"
    );
    assert_ne!(
        plain.looks.disable.color, plain.looks.default.color,
        "無効表示の色が既定の文字色のまま＝混色していない"
    );
    // 色以外は既定と同じ（正典「ほかは font. 定義群と同じ」・要件 4.5）。
    assert_eq!(plain.looks.disable.name, plain.looks.default.name);
    assert_eq!(plain.looks.disable.height, plain.looks.default.height);
    // 選択肢文字色は既存の選択肢表示の解決結果から取る——未指定バルーンは
    // `ResolvedChoiceStyle::Invert` なので文字色は既定色の反転（黒→白）。
    assert_eq!(
        plain.looks.cursor_text,
        (255, 255, 255),
        "選択肢文字色を既定の文字色で代用している（既存の解決を通していない）"
    );

    // ── バルーン定義あり: 2 層が定義の値から組まれる ──
    let defined = ResolvedFont::resolve(&model_with_font(Font::new(
        Some("Yu Gothic UI,Meiryo".to_owned()),
        Some(20),
        FontColor::new(Some(255), Some(255), Some(255)),
    )));
    // 不変条件⑲: 2 層の既定と ResolvedFont の 4 項目は一致する。
    assert_eq!(
        defined.looks.default.name,
        std::iter::once(defined.name.clone())
            .chain(defined.fallback_chain.iter().cloned())
            .collect::<Vec<String>>(),
        "既定の候補列は「採用名 ＋ 残余名」の順"
    );
    assert_eq!(defined.looks.default.height, defined.height);
    assert_eq!(defined.looks.default.color, defined.color);
    assert_eq!(defined.looks.default.height, 20.0);
    assert_eq!(defined.looks.default.color, (255, 255, 255));

    // ── 背景色を実際に受け取っている（白決め打ちではない） ──
    let white_text = model_with_font(Font::new(
        None,
        None,
        FontColor::new(Some(255), Some(255), Some(255)),
    ));
    let on_black = ResolvedFont::resolve_with_background(&white_text, (0, 0, 0));
    assert_eq!(
        on_black.looks.disable.color,
        mix_disabled((255, 255, 255), (0, 0, 0))
    );
    assert_ne!(
        on_black.looks.disable.color,
        ResolvedFont::resolve(&white_text).looks.disable.color,
        "背景を受け取らず常に白として混ぜている"
    );

    // 既存の構築関数は白の既定へ委譲する。
    assert_eq!(
        ResolvedFont::resolve(&white_text).looks,
        ResolvedFont::resolve_with_background(&white_text, DEFAULT_BALLOON_BACKGROUND).looks
    );
}

// ── COM 検証（headless DWrite・デバイス非依存・窓不要） ──

/// TextFormat の実設定を読み戻す（family 名・fontsize・4 方向設定）。
fn read_family_name(format: &IDWriteTextFormat) -> String {
    unsafe {
        let len = format.GetFontFamilyNameLength() as usize;
        let mut buf = vec![0u16; len + 1];
        format
            .GetFontFamilyName(&mut buf)
            .expect("GetFontFamilyName");
        String::from_utf16_lossy(&buf[..len])
    }
}

/// 観測可能な完了状態（COM 側）: 欠落 balloon 定義→既定値一式で実 TextFormat が
/// 生成でき、3 方向それぞれでレシピどおりの設定が焼き込まれている。
#[test]
fn text_format_from_missing_definition_carries_defaults_and_recipe() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)
        .expect("DWriteCreateFactory（デバイス非依存・headless 可）");
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let recipe = DirectionRecipe::for_mode(mode);
        let format = create_text_format(&factory, &resolved, mode)
            .expect("既定値一式で TextFormat 生成が成立する");
        assert_eq!(read_family_name(&format), "ＭＳ ゴシック");
        unsafe {
            assert_eq!(format.GetFontSize(), 12.0);
            assert_eq!(format.GetReadingDirection(), recipe.reading, "{mode:?}");
            assert_eq!(format.GetFlowDirection(), recipe.flow, "{mode:?}");
            assert_eq!(format.GetTextAlignment(), recipe.text_alignment);
            assert_eq!(format.GetParagraphAlignment(), recipe.paragraph_alignment);
        }
    }
}

/// 明示定義（名前・高さ）はそのまま TextFormat へ写る（fontsize＝font.height 素通し）。
#[test]
fn text_format_honors_explicit_name_and_height() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let font = Font::new(
        Some("ＭＳ Ｐゴシック".to_owned()),
        Some(20),
        FontColor::new(None, None, None),
    );
    let resolved = ResolvedFont::resolve(&model_with_font(font));
    let format = create_text_format(&factory, &resolved, WritingMode::HorizontalTb)
        .expect("明示定義で TextFormat 生成が成立する");
    assert_eq!(read_family_name(&format), "ＭＳ Ｐゴシック");
    unsafe {
        assert_eq!(format.GetFontSize(), 20.0);
    }
}

/// フォント生成失敗経路（R4.2・Error Categories）: 生成失敗→warn＋既定フォント再試行→
/// なお失敗は error!＋Device Err（panic しない）。fontsize 非正値は DirectWrite が
/// 決定論的に拒否するため、再試行でも失敗する入力として経路全体を檻化する。
#[test]
fn unusable_format_surfaces_device_error_after_default_retry() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    // resolve は正値を保証するため、失敗経路は手組みの縮退値で叩く（crate 内テスト特権）。
    let mut resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    resolved.height = 0.0;
    let (result, warns, errors) =
        with_log_cage(|| create_text_format(&factory, &resolved, WritingMode::HorizontalTb));
    match result {
        Err(TextLayerError::Device { context, .. }) => {
            assert_eq!(context, "CreateTextFormat");
        }
        other => panic!("Device エラーを期待したが {other:?}"),
    }
    assert_eq!(
        warns, 1,
        "初回失敗→既定フォント再試行の warn がちょうど 1 回"
    );
    assert_eq!(errors, 1, "再試行失敗→error! がちょうど 1 回");
}

// ── task 6.2 R4.5: DWriteMetrics——計測専用 probe TextLayout（probe 規約） ──
//
// probe 規約（design discussion #1 裁定）: 未折返し（折返し無効寸）の測定専用
// TextLayout を、描画と同一の create_text_format 経路（フォント・サイズ・
// writing_mode 写像設定込み）で折返し決定の前に生成し、cluster metrics から
// advance を得る（鶏卵の構造的切断）。probe はキャッシュ可（確定内容の metrics 不変）。

use wintf::com::dwrite::DWriteTextLayoutExt;

/// テスト側の手組み probe（実装と独立に同一規約で測る参照値）。
fn manual_probe_advance(
    factory: &windows::Win32::Graphics::DirectWrite::IDWriteFactory2,
    font: &ResolvedFont,
    mode: WritingMode,
    ch: char,
) -> f32 {
    let format = create_text_format(factory, font, mode).expect("参照 format");
    let layout = wintf::com::dwrite::DWriteFactoryExt::create_text_layout(
        factory,
        &windows::core::HSTRING::from(ch.to_string()),
        &format,
        PROBE_MAX_EXTENT,
        PROBE_MAX_EXTENT,
    )
    .expect("参照 probe layout");
    layout
        .get_cluster_metrics()
        .expect("参照 cluster metrics")
        .iter()
        .map(|c| c.width)
        .sum()
}

/// 観測可能な完了状態（task 6.2）: 実測送り幅は、描画と同一の format 経路で
/// 生成した未折返し probe layout の cluster metrics と一致する。
#[test]
fn dwrite_metrics_advance_matches_manual_probe_layout() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
    for ch in ['あ', 'a', '漢', 'W', '。'] {
        let expected = manual_probe_advance(&factory, &resolved, WritingMode::HorizontalTb, ch);
        assert_eq!(
            metrics.advance(ch, DEFAULT_FONT_HEIGHT),
            expected,
            "{ch:?} の実測 advance が probe 参照値と一致する"
        );
        assert!(expected > 0.0, "{ch:?} の advance は正値");
    }
}

/// probe は writing_mode 写像設定込みの同一 format で生成される——縦書き
/// （vertical_rl）の実測は縦書き format の probe 参照値と一致する（横書き format
/// の値ではない）。
#[test]
fn dwrite_metrics_probe_carries_writing_mode_recipe() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let metrics = default_metrics(&factory, mode);
        for ch in ['あ', 'a', '、'] {
            assert_eq!(
                metrics.advance(ch, DEFAULT_FONT_HEIGHT),
                manual_probe_advance(&factory, &resolved, mode, ch),
                "{mode:?} {ch:?}: probe は当該 writing_mode の format で測られる"
            );
        }
    }
}

/// 等幅（ＭＳ ゴシック）: 全角＝半角×2 の実測。プロポーショナル
/// （ＭＳ Ｐゴシック）: 'i' と 'W' の送り幅が異なる実測——FixedMetrics の
/// 仮想値では出ない差が実測で得られることの檻。
#[test]
fn dwrite_metrics_measures_fixed_pitch_and_proportional_distinctly() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    // 等幅: 既定 ＭＳ ゴシック——全角は半角のちょうど 2 倍。
    let gothic = default_metrics(&factory, WritingMode::HorizontalTb);
    let full = gothic.advance('あ', DEFAULT_FONT_HEIGHT);
    let half = gothic.advance('a', DEFAULT_FONT_HEIGHT);
    assert!(full > 0.0 && half > 0.0);
    assert_eq!(full, half * 2.0, "等幅フォントの全角＝半角×2");
    // プロポーショナル: ＭＳ Ｐゴシック——'i' は 'W' より狭い。
    let p_font = Font::new(
        Some("ＭＳ Ｐゴシック".to_owned()),
        Some(12),
        FontColor::new(None, None, None),
    );
    let p_resolved = ResolvedFont::resolve(&model_with_font(p_font));
    let p_metrics = DWriteMetrics::new(
        &factory,
        &p_resolved,
        WritingMode::HorizontalTb,
        &TextLayerConfig::default(),
    )
    .expect("プロポーショナルで DWriteMetrics 生成が成立する");
    let narrow = p_metrics.advance('i', 12.0);
    let wide = p_metrics.advance('W', 12.0);
    assert!(
        narrow < wide,
        "プロポーショナルの実測: 'i'({narrow}) < 'W'({wide})"
    );
}

/// 決定論: 同一フォント・同一文字→同一送り幅（同一インスタンスの再計測も
/// 別インスタンスも完全一致・R2.5 系/R11.6 の COM 側檻）。
#[test]
fn dwrite_metrics_is_deterministic_across_calls_and_instances() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let first = default_metrics(&factory, WritingMode::VerticalRl);
    let second = default_metrics(&factory, WritingMode::VerticalRl);
    for ch in ['あ', 'x', '！'] {
        let a = first.advance(ch, DEFAULT_FONT_HEIGHT);
        let b = first.advance(ch, DEFAULT_FONT_HEIGHT);
        let c = second.advance(ch, DEFAULT_FONT_HEIGHT);
        assert_eq!(a, b, "{ch:?}: 再計測（キャッシュ経路）も同値");
        assert_eq!(a, c, "{ch:?}: 別インスタンスも同値");
    }
}

/// キャッシュ規約: 同一文字の probe は 1 回だけ生成され、以後はキャッシュから
/// 返る（値は同一・probe 規約「確定内容の metrics は不変ゆえキャッシュ可」）。
#[test]
fn dwrite_metrics_caches_probed_advances() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
    assert_eq!(metrics.cached_probe_count(), 0);
    let first = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
    assert_eq!(metrics.cached_probe_count(), 1);
    let again = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
    assert_eq!(
        metrics.cached_probe_count(),
        1,
        "同一文字の再計測は probe を増やさない"
    );
    assert_eq!(first, again);
    metrics.advance('a', DEFAULT_FONT_HEIGHT);
    assert_eq!(metrics.cached_probe_count(), 2);
}

/// line_pitch は正典式 font_height + TextLayerConfig::line_gap
/// ——FixedMetrics と同じ正本（trait doc）に従う。
#[test]
fn dwrite_metrics_line_pitch_follows_config_canon() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
    assert_eq!(metrics.line_pitch(12.0), 14.0, "12 + 2 = 14");
    assert_eq!(metrics.line_pitch(10.0), 12.0, "10 + 2 = 12");
    // 行間は config が正本——既定（2）以外の行間も反映される。
    // 既定と同じ 2 を入れると差が出ず検査が空振りになるため、非既定の 5 を使う。
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    let config = TextLayerConfig {
        line_gap: 5.0,
        ..TextLayerConfig::default()
    };
    let widened = DWriteMetrics::new(&factory, &resolved, WritingMode::HorizontalTb, &config)
        .expect("非既定の行間でも生成が成立する");
    assert_eq!(
        widened.line_pitch(10.0),
        15.0,
        "非既定の行間 5: 10 + 5 = 15（既定 2 なら 12 で赤）"
    );
}

/// line_box_height は**実 font face metrics**（`ascent + descent`）由来——
/// 既定 ＭＳ ゴシックは比ちょうど 1.0（em ボックス丈と一致）だが、Yu Gothic UI は
/// 1.33 倍（28px で 37.2px）へ伸びる。**この差が「hover 文字の下が切れる」不具合の量**であり、
/// 既定フォントだけを見ていると観測できない（既定フォント盲点）。
#[test]
fn dwrite_metrics_line_box_height_comes_from_real_font_face_metrics() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    // ＭＳ ゴシック（既定・upem 256／ascent 220／descent 36）＝ちょうど 1.0em。
    let gothic = default_metrics(&factory, WritingMode::HorizontalTb);
    let box_12 = gothic.line_box_height(12.0);
    assert!(
        (box_12 - 12.0).abs() < 0.01,
        "ＭＳ ゴシックの行ボックス丈は em ボックス丈と一致（実測 1.0em）: {box_12}"
    );
    // Yu Gothic UI（upem 2048／ascent 2210／descent 514）＝1.3301em。
    let yu_font = Font::new(
        Some("Yu Gothic UI".to_owned()),
        Some(28),
        FontColor::new(None, None, None),
    );
    let yu = DWriteMetrics::new(
        &factory,
        &ResolvedFont::resolve(&model_with_font(yu_font)),
        WritingMode::HorizontalTb,
        &TextLayerConfig::default(),
    )
    .expect("Yu Gothic UI で DWriteMetrics 生成が成立する");
    let box_28 = yu.line_box_height(28.0);
    assert!(
        (box_28 - 37.24).abs() < 0.1,
        "Yu Gothic UI 28px の行ボックス丈は 37.24px（(2210+514)/2048×28）: {box_28}"
    );
    assert!(
        box_28 > 28.0,
        "em ボックス丈（28）より高い＝帯を font_height で切ると descent がはみ出す"
    );
    // 比例（font_height 非依存の設計値）: 高さ 2 倍で丈も 2 倍。
    assert!((yu.line_box_height(56.0) - box_28 * 2.0).abs() < 0.01);
}

/// 契約檻: advance へ束縛フォントと異なる font_height が渡されたら warn（縮退継続・
/// 値は束縛 format の実測のまま——probe は描画と同一 format が正準ゆえ）。
#[test]
fn dwrite_metrics_warns_on_font_height_mismatch() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
    let bound = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
    let (mismatched, warns, errors) = with_log_cage(|| metrics.advance('あ', 99.0));
    assert_eq!(
        warns, 1,
        "束縛高さと異なる font_height はちょうど 1 回 warn"
    );
    assert_eq!(errors, 0);
    assert_eq!(
        mismatched, bound,
        "値は束縛 format の実測のまま（縮退継続）"
    );
}

// ── C5 フォント縦書き等価の構造檻（spec areka-P0-balloon-vertical-canon 要件 6.1〜6.4／6.6） ──
//
// **何を守っているか**（同 spec 裁定 4・design.md C5）: areka は SSP の
// 「フォント名の頭に `@` を付けて縦書き異体を選び、異体が無ければ環境の標準ゴシックへ
// 自動差し替える」機構を**模倣しない**。DirectWrite のネイティブ縦組みをそのまま用い、
// バルーン定義の `font.name` は縦書きでも差し替えない。したがって要件 6.2／6.3 は
// 「その経路が**存在しない**」という不在の主張であり、値を突き合わせる普通のテストでは
// 書けない。以下は不在を本番ソース `draw.rs` の字面と構造で固定する檻である。要件 6.4
// （計測と描画が同一の format 工場を通る）も「呼び手がここ以外に増えていない」という
// 構造の主張なので同じ手段で固定する。
//
// **既存が固定済みのものは重ねない**: 要件 6.1／6.6（3 モードの reading／flow 写像）は
// 本ファイルの `direction_recipe_maps_three_modes_per_design_table`（写像表そのもの・
// 3 モード全て）と `all_direction_recipes_share_leading_near_alignment`（共通部）が既に
// 固定している。COM 側の焼き込みも
// `text_format_from_missing_definition_carries_defaults_and_recipe` が 3 モードで
// TextFormat から読み戻して確認済み。よってここでは写像の再宣言を作らない。
//
// **空振りの危険と更新義務**（design.md C5 Risks の明示要求）: 字面検査はリファクタで
// 名前や書式が変わると、赤くならないまま守備範囲だけが消える。この檻は `draw.rs` の
// 次の名前と形に依存している——`create_text_format`／`try_create_format`／
// `DirectionRecipe::for_mode`／`DEFAULT_FONT_NAME`、および `cargo fmt` が保証する
// トップレベル項目の列 0 閉じ括弧。**これらを改名・改形したときは、この檻も同時に
// 更新すること。** 各檻は自分自身に陽性対照（既知の違反文字列を同じ検査関数へ通して
// 検出できることの確認）を持たせてあり、検査関数が壊れたときは陽性対照が先に落ちる。
//
// 本節の要件番号はすべて spec `areka-P0-balloon-vertical-canon` のもの。`draw.rs` 本文の
// `R4.2` 等は完了 spec `areka-P0-emo-text-layer` の番号であり別物。

/// 本番ソース `draw.rs` の本文。`#[path]` で取り込まれる本テストファイルの本文は
/// 含まれない（`include_str!` はディスク上の `draw.rs` 単体を読む）。
const DRAW_RS: &str = include_str!("draw.rs");

/// ファサード `draw.rs` の**本番の子モジュール**の本文（ファイル名付き）。
///
/// 分割（タスク 1.1）と実体化（タスク 6.2〜6.4）でファサードの本番コードは
/// 4 ファイルへ分かれた。`draw.rs` 単体だけを走査する檻は、分かれた先に同じ違反が
/// 入っても赤くならない——`@` 前置の禁止はこの一覧の**全ファイル**を走査する。
/// **ファサードへ本番の子を足したらここへも足すこと。**
const DRAW_FACADE_SOURCES: &[(&str, &str)] = &[
    ("draw.rs", DRAW_RS),
    ("draw_metrics.rs", include_str!("draw_metrics.rs")),
    ("draw_line_store.rs", include_str!("draw_line_store.rs")),
    ("draw_catalog.rs", include_str!("draw_catalog.rs")),
];

/// 手保守の [`DRAW_FACADE_SOURCES`] が「新設したのに載せない」を塞げない穴を閉じる
/// （タスク 9.4）。
///
/// ファサード群の外延は機械で決まる——`src/draw*.rs` のうち兄弟テスト（`*_tests.rs`）と
/// 支援（`*_test_support.rs`）を除いたものが本番ファイルである。実ファイル集合を実行時に
/// 読んで一覧と突き合わせるので、`draw_*.rs` を新設して一覧へ載せ忘れると赤になる
/// （母数の `assert_eq!` は「黙って減る」しか塞げない）。
#[test]
fn draw_facade_sources_cover_every_draw_production_file() {
    use std::collections::BTreeSet;

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let actual: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("src ディレクトリが読めない")
        .map(|entry| {
            entry
                .expect("src の項目が読めない")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| {
            name.starts_with("draw")
                && name.ends_with(".rs")
                && !name.ends_with("_tests.rs")
                && !name.ends_with("_test_support.rs")
        })
        .collect();
    let listed: BTreeSet<String> = DRAW_FACADE_SOURCES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(
        actual, listed,
        "`draw*.rs` の本番ファイル集合と DRAW_FACADE_SOURCES が食い違う——\
         新設したファイルを一覧へ足すこと"
    );
    // 空振り防止: どちらも空なら上の等値は恒真になる。
    assert_eq!(listed.len(), 4, "走査するファサードの母数");
}

/// 改行を LF へ正規化した `draw.rs` 本文。ワークツリーは `core.autocrlf` により CRLF で
/// 展開されるため、行末に依存する検査（列 0 閉じ括弧・行単位の走査）は必ずこれを使う。
fn draw_rs() -> String {
    DRAW_RS.replace('\r', "")
}

/// 部分文字列の非重複出現数。
fn count(src: &str, needle: &str) -> usize {
    src.matches(needle).count()
}

/// トップレベル関数 1 本の本文（署名の先頭から列 0 の閉じ括弧の直前まで）を切り出す。
///
/// 列 0 の `}` に依拠できるのは、ワークスペースが `cargo fmt --all -- --check` を常時
/// ゲートしており、トップレベル項目の閉じ括弧が必ず列 0 に来るため。
fn top_level_fn_source<'a>(src: &'a str, signature_head: &str) -> &'a str {
    let start = src.find(signature_head).unwrap_or_else(|| {
        panic!("draw.rs に `{signature_head}` が無い——改名したなら本檻も更新すること")
    });
    let rest = &src[start..];
    let end = rest
        .find("\n}\n")
        .expect("トップレベル関数の列 0 閉じ括弧が見つからない（rustfmt 前提が崩れている）");
    &rest[..end]
}

/// `patterns` のうち `src` に現れたものを列挙する（禁止パターン検査の本体）。
fn hits<'p>(src: &str, patterns: &'p [&'p str]) -> Vec<&'p str> {
    patterns
        .iter()
        .copied()
        .filter(|pat| src.contains(pat))
        .collect()
}

/// 「フォント名の頭へ `@` を付ける」生成の字面パターン。`"@` は `"@"`／`"@{}"`／
/// `format!("@{name}")` を、`'@'` は char 経路（`push('@')`／`insert(0, '@')`）を捕まえる。
/// Rust の束縛パターン（`seam @ (..)`）は素の `@` なのでどちらにも当たらない＝誤検出しない。
const AT_PREFIX_PATTERNS: &[&str] = &["\"@", "'@'"];

/// 要件 6.2: 本番ソースに「フォント名の頭へ `@` を付ける」生成が存在しない。
///
/// 走査面は `draw.rs` 単体ではなく**ファサードの本番の子を含む全ファイル**
/// （[`DRAW_FACADE_SOURCES`]）——家族名が DirectWrite へ渡る入口が `draw.rs` の
/// `try_create_format` と `draw_metrics.rs` の `probe_format_for` の 2 つになったため、
/// 片方だけを見る檻では違反が素通りする（タスク 1.1 の申し送り）。
#[test]
fn at_prefixed_font_name_generation_is_absent_from_production_source() {
    // 空振り防止: draw.rs には素の `@`（束縛パターン）が現に在る。`@` が 1 個も無いから
    // 緑、という無意味な緑ではないことを先に示す。
    assert!(
        draw_rs().contains('@'),
        "draw.rs に `@` が 1 個も無い——この檻は空振りしている可能性がある"
    );
    for (name, raw) in DRAW_FACADE_SOURCES {
        let src = raw.replace('\r', "");
        assert_eq!(
            hits(&src, AT_PREFIX_PATTERNS),
            Vec::<&str>::new(),
            "SSP の `@` フォント機構（縦書き異体名の生成）が {name} に現れた。\
             areka は裁定 4 によりこれを模倣しない"
        );
    }
    // 陽性対照: 同じ検査関数が既知の違反を確かに検出する。
    for planted in [
        r#"    let name = format!("@{}", font.name);"#,
        r#"    let name = "@".to_owned() + &font.name;"#,
        r#"    let mut name = font.name.clone(); name.insert(0, '@');"#,
    ] {
        assert!(
            !hits(planted, AT_PREFIX_PATTERNS).is_empty(),
            "陽性対照を検出できない＝禁止パターンが空振りしている: {planted}"
        );
    }
    // 陰性対照: Rust の束縛パターンの素の `@` では発火しない（誤検出しない）。
    assert!(
        hits(
            "                seam @ (ResidentContent::Image(_)) => {}",
            AT_PREFIX_PATTERNS
        )
        .is_empty(),
        "束縛パターンの `@` で発火する＝誤検出する檻になっている"
    );
}

/// 要件 6.3: DirectWrite へ渡るフォント family 名は「台本／バルーン定義が書いた名前」か
/// 「既定フォントへの戻し」の 2 つだけ——縦書き専用の差し替え先は存在しない。
///
/// **家族名の入口は 2 つある**（タスク 6.4）:
///
/// 1. `draw.rs::try_create_format`（描画・計測共用の束縛書式）——バルーン定義の
///    `font.name` をそのまま渡すか、生成失敗時に `DEFAULT_FONT_NAME` で再試行するか。
/// 2. `draw_metrics.rs::probe_format_for`（計測鍵ごとの試験用書式）——計測鍵の候補列を
///    `FontCatalog::family_for` で解決した名前か、全滅時の `DEFAULT_FONT_NAME` か。
///
/// どちらの入口も「書かれた名前」か「既定名」しか通さないので、`@` 前置のような
/// 別名生成が入り込む余地が無い。**入口を増やしたらこの檻も広げること。**
#[test]
fn font_family_reaches_directwrite_only_as_author_name_or_default_retry() {
    let src = draw_rs();
    // family 名が DirectWrite へ渡る唯一の窓口は try_create_format。定義 1＋呼出 2 で 3。
    assert_eq!(
        count(&src, "try_create_format"),
        3,
        "try_create_format の出現が 1 定義＋2 呼出から動いた——family 名の入口が増減した疑い"
    );
    let body = top_level_fn_source(&src, "pub fn create_text_format(");
    // 切り出しが本物であることの確認（範囲取得が壊れていたらここで落ちる）。
    assert!(
        body.contains("DirectionRecipe::for_mode"),
        "create_text_format の本文切り出しが失敗している: {body}"
    );
    assert_eq!(
        count(body, "try_create_format("),
        2,
        "呼出 2 箇所はいずれも create_text_format の内側にある"
    );
    assert!(
        body.contains("try_create_format(factory, &font.name, font.height)"),
        "1 回目はバルーン定義の font.name をそのまま渡す（別名へ差し替えない）"
    );
    assert!(
        body.contains("try_create_format(factory, DEFAULT_FONT_NAME, font.height)"),
        "2 回目は生成失敗時の既定フォント再試行のみ（書字方向とは無関係な経路）"
    );
    // 既定フォント名リテラルは宣言 1 箇所だけ——縦書き専用の差し替え先が別に
    // 埋め込まれていないこと。ログ文中の ＭＳ ゴシックは引用符に挟まれないので当たらず、
    // doc コメントの Yu Gothic UI 実測例も対象外＝誤検出しない。
    assert!(
        src.contains("pub const DEFAULT_FONT_NAME: &str = \"ＭＳ ゴシック\";"),
        "既定フォント名の宣言が見つからない——改名したなら本檻も更新すること"
    );
    assert_eq!(
        count(&src, "\"ＭＳ ゴシック\""),
        1,
        "フォント名リテラルが DEFAULT_FONT_NAME の宣言以外にも現れた"
    );

    // ── 第 2 の入口: draw_metrics.rs::probe_format_for（計測鍵ごとの試験用書式）
    let metrics_src = include_str!("draw_metrics.rs").replace('\r', "");
    assert!(
        metrics_src.contains("fn probe_format_for(&self, key: &FontKey)"),
        "probe_format_for が見つからない——改名したなら本檻も更新すること"
    );
    assert_eq!(
        count(&metrics_src, ".create_text_format("),
        1,
        "draw_metrics.rs から DirectWrite へ family 名を渡す呼出が 1 か所から動いた"
    );
    assert!(
        metrics_src.contains("&HSTRING::from(family.as_str()),"),
        "試験用書式へ渡す family 名が束縛 `family` 以外から来ている"
    );
    // その `family` の唯一の出所——候補列の解決結果か、全滅時の既定名。
    assert!(
        metrics_src.contains(
            "        let family = self\n            .fonts\n            .family_for(&key.name)\n\
             \x20           .unwrap_or_else(|| DEFAULT_FONT_NAME.to_owned());"
        ),
        "family の導出が「計測鍵の候補列を FontCatalog で解決した名前か既定名」から動いた"
    );
    assert_eq!(
        count(&metrics_src, "\"ＭＳ ゴシック\""),
        0,
        "draw_metrics.rs にフォント名リテラルが現れた（既定名は DEFAULT_FONT_NAME 経由だけ）"
    );
}

/// 要件 6.2／6.3: `create_text_format` の中で書字方向が効くのは方向レシピの 1 行だけ
/// ——フォント名の選択は writing_mode に一切依存しない。
#[test]
fn writing_mode_selects_only_the_direction_recipe_not_the_font_name() {
    let src = draw_rs();
    let body = top_level_fn_source(&src, "pub fn create_text_format(");
    let mode_lines: Vec<&str> = body.lines().filter(|line| line.contains("mode")).collect();
    assert_eq!(
        mode_lines.len(),
        2,
        "書字方向に触れる行が署名＋方向レシピの 2 行から増えた＝縦書き分岐が混入した疑い: \
         {mode_lines:?}"
    );
    assert!(
        mode_lines[0].contains("mode: WritingMode"),
        "1 行目は署名の引数: {:?}",
        mode_lines[0]
    );
    assert!(
        mode_lines[1].contains("DirectionRecipe::for_mode(mode)"),
        "2 行目は方向レシピの適用のみ: {:?}",
        mode_lines[1]
    );
    // 陽性対照: 縦書き分岐でフォント名を差し替える版を同じ手順に通すと 3 行になる。
    let planted = "pub fn create_text_format(\n    mode: WritingMode,\n) -> R {\n    \
        let name = if mode.is_vertical() { at_name } else { &font.name };\n    \
        DirectionRecipe::for_mode(mode).apply(&format)?;\n}\n";
    let planted_body = top_level_fn_source(planted, "pub fn create_text_format(");
    assert_eq!(
        planted_body
            .lines()
            .filter(|line| line.contains("mode"))
            .count(),
        3,
        "陽性対照（縦書き分岐でフォント名を選ぶ版）を検出できていない"
    );
}

/// 要件 6.4: `DirectionRecipe::for_mode` の本番呼出は `create_text_format` の内側 1 箇所
/// だけ——計測（`DWriteMetrics`）も描画（`viewbox_draw.rs`）も同じ format 工場を通る構造
/// の証跡。呼び手が増えたらここが赤くなる。
///
/// **数え方**: `draw.rs` 本文に現れる識別子 `for_mode` の**全出現**を数える。現状は
/// 定義（`pub fn for_mode(`）と呼出（`create_text_format` 内）の 2 件のみで、doc
/// コメント中の言及も無い。`DirectionRecipe::for_mode(` という呼出形だけを数えれば
/// たまたま 1 になるが、それでは `Self::for_mode(..)` や `use` 経由の裸呼出を見逃す。
/// 全出現を数えて内訳を突き合わせる形にしてあるので、doc への言及が増えただけでも赤に
/// なり、人が内訳を読み直すことになる（それが意図した厳しさ）。
#[test]
fn direction_recipe_factory_has_exactly_one_call_site_inside_create_text_format() {
    let src = draw_rs();
    assert_eq!(
        count(&src, "for_mode"),
        2,
        "draw.rs の `for_mode` 出現が 2（定義 1＋呼出 1）から動いた——内訳を読み直すこと"
    );
    assert_eq!(
        count(&src, "pub fn for_mode("),
        1,
        "内訳のうち 1 件は定義そのもの"
    );
    let body = top_level_fn_source(&src, "pub fn create_text_format(");
    assert!(
        body.contains("try_create_format("),
        "create_text_format の本文切り出しが失敗している: {body}"
    );
    assert_eq!(
        count(body, "for_mode"),
        1,
        "残る 1 件は create_text_format の内側の呼出でなければならない"
    );
    // 陽性対照: 工場の外に呼び手が増えた版を同じ数え方に通すと 3 になる。
    let planted = "pub fn for_mode(mode: WritingMode) {}\n\
        pub fn create_text_format() {\n    DirectionRecipe::for_mode(mode);\n}\n\
        pub fn elsewhere() {\n    DirectionRecipe::for_mode(other);\n}\n";
    assert_eq!(
        count(planted, "for_mode"),
        3,
        "陽性対照（工場の外に呼び手が増えた版）を検出できていない"
    );
}

/// 要件 6.2／6.3 の観測面: バルーン定義の明示フォント名は 3 モードとも TextFormat へ
/// そのまま焼かれる——縦書きでも `@` は付かず、標準ゴシックへも差し替わらない。
/// 字面檻がリファクタで空振りしても、この実 DirectWrite 読み戻しは挙動を捕まえる。
#[test]
fn explicit_font_name_is_carried_into_vertical_modes_unchanged() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let font = Font::new(
        Some("ＭＳ Ｐゴシック".to_owned()),
        Some(16),
        FontColor::new(None, None, None),
    );
    let resolved = ResolvedFont::resolve(&model_with_font(font));
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let format =
            create_text_format(&factory, &resolved, mode).expect("明示定義で TextFormat 生成");
        let family = read_family_name(&format);
        assert_eq!(
            family, "ＭＳ Ｐゴシック",
            "{mode:?}: 作者指定のフォント名が差し替わっている"
        );
        assert!(
            !family.starts_with('@'),
            "{mode:?}: 縦書き異体の `@` 接頭辞が付いている: {family}"
        );
    }
}
