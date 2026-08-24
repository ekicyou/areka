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
    DEFAULT_FONT_HEIGHT, DEFAULT_FONT_NAME, DWriteMetrics, DirectionRecipe, FontDisableSeam,
    PROBE_MAX_EXTENT, RESERVED_KEY_DISABLE_FONT_PREFIX, ResolvedFont, create_text_format,
};
use crate::TextLayerError;
use crate::canvas::TextEffects;
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

// ── R10.3: 文字装飾／disable.font.* は型シームのみ（実挙動なし） ──

/// 装飾（TextEffects）と disable.font.* シームは型のみ＝データを一切持たない
/// （zero-sized・M1 で描画へ影響し得ない構造保証）。
#[test]
fn decoration_and_disable_seams_are_type_only() {
    assert_eq!(RESERVED_KEY_DISABLE_FONT_PREFIX, "disable.font.");
    assert_eq!(std::mem::size_of::<TextEffects>(), 0);
    assert_eq!(std::mem::size_of::<FontDisableSeam>(), 0);
    // ResolvedFont はシームを保持するが Default 生成のみ（実挙動なし）。
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    assert_eq!(resolved.effects, TextEffects::default());
    assert_eq!(resolved.disable, FontDisableSeam::default());
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

/// line_pitch は M1 正準式 ceil(font_height × TextLayerConfig::line_pitch_factor)
/// ——FixedMetrics と同じ正本（trait doc）に従う。
#[test]
fn dwrite_metrics_line_pitch_follows_config_canon() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
    assert_eq!(metrics.line_pitch(12.0), 15.0);
    assert_eq!(metrics.line_pitch(10.0), 13.0, "12.5 → ceil 13");
    // 係数は config が正本——非既定係数も反映される。
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    let config = TextLayerConfig {
        line_pitch_factor: 2.0,
        ..TextLayerConfig::default()
    };
    let doubled = DWriteMetrics::new(&factory, &resolved, WritingMode::HorizontalTb, &config)
        .expect("非既定係数でも生成が成立する");
    assert_eq!(doubled.line_pitch(10.0), 20.0);
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
