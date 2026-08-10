use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use windows::Win32::Graphics::DirectWrite::DWRITE_FACTORY_TYPE_SHARED;
use wintf::com::dwrite::DWriteTextLayoutExt;
use wintf::com::dwrite::dwrite_create_factory;

use super::{DEFAULT_FONT_HEIGHT, DWriteMetrics, ResolvedFont};
use crate::canvas::TextEffects;
use crate::layout::{GlyphMetrics, LayoutEngine, WrapPlan};
use crate::region::TextRegion;
use crate::state::{TextItem, TextLayerConfig};
use crate::writing::WritingMode;
use super::test_support::{default_metrics, empty_font, model_with_font, with_log_cage};

// ── task 6.3 R3.1/R7.3: DrawExecutor——可視窓の全域再描画を自前供給面へ焼き込む ──
//
// 観測可能な完了状態: 可視グリフ数が増えるほど自前供給面の読み戻し結果で非透明
// ピクセルが単調に増加し、Clear 後は全域が透明に戻る。併せて構造契約を檻化する:
// 差分描画でなく全域再描画（残渣なし）・確定行 TextLayout キャッシュは Clear のみ
// 全破棄・合成スケールは描画ターゲットへの一点適用（二重適用/未適用の排除）。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::UI::Composition::Compositor;
use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
use wintf::com::wuc::create_dispatcher_queue_controller;
use wintf::ecs::{GraphicsCore, Visual};

use super::DrawExecutor;
use crate::actor::TextSlotBinding;
use crate::canvas::{
    ContentCanvas, ImageSeam, RegionTransform, Resident, ResidentContent, SurfaceSeam,
};
use crate::layout::VisibleWindow;
use crate::region::ScaleContract;
use crate::surface::TextSurface;

/// テスト用 WUC apartment / dispatcher（surface.rs テストと同一方針:
/// COM 未初期化のテストスレッドでは ASTA 第一候補・NONE 保険）。
fn make_dispatcher_and_compositor() -> (windows::System::DispatcherQueueController, Compositor)
{
    let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
        .or_else(|e_asta| {
            create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta)
        })
        .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
    let compositor = Compositor::new().expect("Compositor::new 失敗");
    (dq, compositor)
}

/// 描画テスト一式（dispatcher/compositor/core/World の寿命を束ねる・headless）。
struct DrawRig {
    _dq: windows::System::DispatcherQueueController,
    compositor: Compositor,
    core: GraphicsCore,
    world: World,
}

impl DrawRig {
    fn new() -> DrawRig {
        let (_dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
        DrawRig {
            _dq,
            compositor,
            core,
            world: World::new(),
        }
    }

    /// 予約スロット（emo-present VisualMount 同型）を組み、image px 原寸と k から
    /// TextSurface を装着する（物理寸＝ceil(image × k)・offset (0,0)）。
    fn attach(&mut self, image_size: (u32, u32), k: f32) -> TextSurface {
        let window = self.world.spawn_empty().id();
        let slot = self
            .world
            .spawn((
                Name::new("emo-text-layer-slot"),
                Visual::default(),
                ChildOf(window),
            ))
            .id();
        self.world.flush();
        let physical = (
            (image_size.0 as f32 * k).ceil() as u32,
            (image_size.1 as f32 * k).ceil() as u32,
        );
        let binding = TextSlotBinding::new(slot, window, k, physical, image_size);
        TextSurface::attach(
            &mut self.world,
            &binding,
            &self.compositor,
            &self.core,
            physical,
            (0.0, 0.0),
        )
        .expect("TextSurface::attach 失敗")
    }
}

/// テスト用 BalloonModel（幾何のみ・font 未指定＝既定 ＭＳ ゴシック）。
fn geo_model(origin: (Option<i32>, Option<i32>), font_height: Option<u32>) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(origin.0, origin.1),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, font_height, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// 文字列→グリフ item 列。
fn glyph_items(s: &str) -> Vec<TextItem> {
    s.chars().map(|ch| TextItem::Glyph { ch }).collect()
}

/// layout→canvas→visible_window→render→read_back の通し（テスト用最短経路）。
#[allow(clippy::too_many_arguments)]
fn render_items(
    executor: &mut DrawExecutor,
    surface: &mut TextSurface,
    items: &[TextItem],
    visible: usize,
    region: &TextRegion,
    mode: WritingMode,
    font: &ResolvedFont,
    metrics: &DWriteMetrics,
    contract: &ScaleContract,
) -> Vec<u8> {
    let lines = LayoutEngine::layout(
        items,
        visible,
        region,
        mode,
        font.height,
        metrics,
        WrapPlan::CharByChar,
    );
    let canvas = crate::canvas::ContentCanvas::from_layout(&lines, region, mode);
    let window = LayoutEngine::visible_window(&lines, region, mode);
    executor
        .render(&canvas, &window, font, mode, contract, surface)
        .expect("DrawExecutor::render 失敗");
    surface.read_back().expect("read_back 失敗")
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0）。
fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 非透明ピクセルの外接範囲 (min_x, min_y)（インクなしは None）。
fn ink_min(bytes: &[u8], width: u32) -> Option<(u32, u32)> {
    let mut min: Option<(u32, u32)> = None;
    for (i, px) in bytes.chunks_exact(4).enumerate() {
        if px[3] != 0 {
            let (x, y) = (i as u32 % width, i as u32 / width);
            min = Some(match min {
                None => (x, y),
                Some((mx, my)) => (mx.min(x), my.min(y)),
            });
        }
    }
    min
}

/// 観測可能な完了状態（task 6.3）: 可視グリフ数が増えるほど自前供給面の読み戻しで
/// 非透明ピクセルが単調に増加し（R3.1 typewriter 進行の描画側）、Clear（状態全消去＋
/// 確定行キャッシュ全破棄）後は全域が透明に戻る。
#[test]
fn render_grows_opaque_pixels_monotonically_and_clear_restores_transparency() {
    let mut rig = DrawRig::new();
    let image = (120u32, 60u32);
    let mut surface = rig.attach(image, 1.0);
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
    let mode = WritingMode::HorizontalTb;
    let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成失敗");
    let region = TextRegion::resolve(&geo_model((Some(0), Some(0)), None), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

    let items = glyph_items("あいうえお");
    let mut counts = Vec::new();
    for visible in 0..=items.len() {
        let bytes = render_items(
            &mut executor,
            &mut surface,
            &items,
            visible,
            &region,
            mode,
            &font,
            &metrics,
            &contract,
        );
        counts.push(opaque_count(&bytes));
    }
    assert_eq!(counts[0], 0, "可視 0 グリフ＝全透明");
    for i in 1..counts.len() {
        assert!(
            counts[i] > counts[i - 1],
            "非透明ピクセルは可視グリフ数とともに単調増加する: {counts:?}"
        );
    }

    // Clear: 未リビール分含む全消去（空 canvas）＋確定行キャッシュの全破棄。
    executor.clear_cache();
    let bytes = render_items(
        &mut executor,
        &mut surface,
        &[],
        0,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert!(
        bytes.iter().all(|&b| b == 0),
        "Clear 後は全域が透明（premultiplied 全 0）へ戻る"
    );
}

/// 全域再描画の構造檻（R7.3）: 可視内容を減らした再描画で以前のインクが残らない
/// （差分描画なら 5 グリフ分の残渣が出る）。同一入力の再描画はバイト一致（決定論）。
#[test]
fn render_is_full_redraw_without_residue() {
    let mut rig = DrawRig::new();
    let image = (120u32, 60u32);
    let mut surface = rig.attach(image, 1.0);
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
    let mode = WritingMode::HorizontalTb;
    let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成失敗");
    let region = TextRegion::resolve(&geo_model((Some(0), Some(0)), None), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

    let items = glyph_items("あいうえお");
    let two_first = render_items(
        &mut executor,
        &mut surface,
        &items,
        2,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    let five = render_items(
        &mut executor,
        &mut surface,
        &items,
        5,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    let two_again = render_items(
        &mut executor,
        &mut surface,
        &items,
        2,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert!(opaque_count(&five) > opaque_count(&two_first));
    assert_eq!(
        two_first, two_again,
        "全域再描画: 5 グリフ描画後に 2 グリフへ戻すと以前の描画とバイト一致（残渣なし）"
    );
}

/// 確定行キャッシュの檻（task 6.3・design「確定行は再生成しない・Clear で全破棄」）:
/// 内容不変の行（確定行）は TextLayout を再生成せず、リビール中の行のみ都度更新。
/// clear_cache（Clear 適用点）だけが全破棄する。
#[test]
fn confirmed_line_layouts_regenerate_only_on_clear() {
    let mut rig = DrawRig::new();
    let image = (120u32, 60u32);
    let mut surface = rig.attach(image, 1.0);
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
    let mode = WritingMode::HorizontalTb;
    let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成失敗");
    let region = TextRegion::resolve(&geo_model((Some(0), Some(0)), None), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

    // 行 0 確定（"あい"）＋行 1 リビール中（"う"）。
    let mut items = glyph_items("あい");
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.push(TextItem::Glyph { ch: 'う' });

    render_items(
        &mut executor,
        &mut surface,
        &items,
        3,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert_eq!(executor.line_layout_creations(), 2, "初回は 2 行分を生成");

    render_items(
        &mut executor,
        &mut surface,
        &items,
        3,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert_eq!(
        executor.line_layout_creations(),
        2,
        "内容不変の再描画は確定行・現行行とも再生成しない（キャッシュ再利用）"
    );

    // リビール進行: 行 1 が "う"→"うえ" へ——行 1 のみ都度更新（行 0 は確定キャッシュ）。
    items.push(TextItem::Glyph { ch: 'え' });
    render_items(
        &mut executor,
        &mut surface,
        &items,
        4,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert_eq!(
        executor.line_layout_creations(),
        3,
        "リビール中の行のみ再生成（確定行 0 はキャッシュ維持）"
    );

    // Clear 適用点: 全破棄→次描画は全行を再生成する。
    executor.clear_cache();
    render_items(
        &mut executor,
        &mut surface,
        &items,
        4,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert_eq!(
        executor.line_layout_creations(),
        5,
        "Clear のみが確定行キャッシュを全破棄する（再描画で 2 行分を再生成）"
    );
}

/// スケール一点適用の構造檻（task 6.3・DPI/スケール契約）: k=2 のインク開始位置は
/// 画像座標 ×2 の近傍に現れる。二重適用（×4）でも未適用（×1）でもないことを
/// 範囲判定で排除する（origin (20,20)・正解 ≈40〜・二重 80〜・未適用 ≈20）。
#[test]
fn scale_applies_exactly_once_at_draw_target() {
    let mut rig = DrawRig::new();
    let image = (120u32, 60u32);
    let k = 2.0f32;
    let mut surface = rig.attach(image, k);
    assert_eq!(surface.size(), (240, 120), "物理寸 = ceil(image × k)");
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let model = geo_model((Some(20), Some(20)), None);
    let font = ResolvedFont::resolve(&model);
    let mode = WritingMode::HorizontalTb;
    let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成失敗");
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(k, None);
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

    // '■'（全角・インクがほぼ em ボックスを満たす）1 グリフを (20,20) へ。
    let items = glyph_items("■");
    let bytes = render_items(
        &mut executor,
        &mut surface,
        &items,
        1,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    let (min_x, min_y) = ink_min(&bytes, 240).expect("インクが描かれる");
    // 正解: 画像座標 (20 + ベアリング数 px) × 2 ≈ [40, 40+2×font]。
    // 二重適用なら x ≥ 80・未適用なら x ≈ 20——いずれも範囲外。
    assert!(
        (38..=70).contains(&min_x),
        "インク開始 x={min_x} は一点適用の範囲 [38,70]（未適用≈20・二重適用≥80 を排除）"
    );
    assert!(
        (38..=70).contains(&min_y),
        "インク開始 y={min_y} は一点適用の範囲 [38,70]（未適用≈20・二重適用≥80 を排除）"
    );
}

/// スクロール可視窓の描画: あふれ発火後は先頭行が供給面から消え（全域再描画）、
/// 可視窓の行が領域先頭へ詰めて描かれる（同一入力の再描画はバイト一致・決定論）。
#[test]
fn scroll_overflow_drops_oldest_line_via_full_redraw() {
    let mut rig = DrawRig::new();
    let image = (60u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    // font 10 → pitch 13・行下端 10/23/36/49——validrect.bottom 40 で 4 行目があふれる。
    let model = geo_model((Some(0), Some(0)), Some(10));
    let font = ResolvedFont::resolve(&model);
    let mode = WritingMode::HorizontalTb;
    let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成失敗");
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

    // 行 0 = ■■■（3 グリフ）・行 1〜3 = ■ 各 1 グリフ。
    let mut items3 = glyph_items("■■■");
    for _ in 0..2 {
        items3.push(TextItem::LineBreak { ratio: 1.0 });
        items3.push(TextItem::Glyph { ch: '■' });
    }
    let mut items4 = items3.clone();
    items4.push(TextItem::LineBreak { ratio: 1.0 });
    items4.push(TextItem::Glyph { ch: '■' });

    // 3 行（収まる・可視 ■×5）→ 4 行（あふれ・可視窓は行 1〜3 ＝ ■×3）。
    let before = render_items(
        &mut executor,
        &mut surface,
        &items3,
        5,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    let after = render_items(
        &mut executor,
        &mut surface,
        &items4,
        6,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert!(
        opaque_count(&after) < opaque_count(&before),
        "スクロール後は行 0（■×3）が全域再描画で消える: before={} after={}",
        opaque_count(&before),
        opaque_count(&after)
    );
    let (_, min_y) = ink_min(&after, 60).expect("スクロール後もインクが描かれる");
    assert!(
        min_y < 10,
        "可視窓先頭行（行 1）は block_offset で領域先頭へ詰めて描かれる（min_y={min_y}）"
    );
    // 決定論: 同一入力の再描画はバイト一致（差分累積なし）。
    let again = render_items(
        &mut executor,
        &mut surface,
        &items4,
        6,
        &region,
        mode,
        &font,
        &metrics,
        &contract,
    );
    assert_eq!(after, again, "同一入力→同一ピクセル（全域再描画の決定論）");
}

/// R8.5: Image/Surface 住人は M1 型シーム——描画は warn!＋skip（インクなし・panic なし）。
/// warn は executor ごと初回のみ（ログスパム抑制）。
#[test]
fn image_and_surface_seam_residents_warn_and_skip() {
    let mut rig = DrawRig::new();
    let image = (120u32, 60u32);
    let mut surface = rig.attach(image, 1.0);
    let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
    let mode = WritingMode::HorizontalTb;
    let contract = ScaleContract::new(1.0, None);
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

    let canvas = ContentCanvas {
        residents: vec![
            Resident {
                content: ResidentContent::Image(ImageSeam::default()),
                transform: RegionTransform::translation(5.0, 5.0),
                effects: TextEffects::default(),
            },
            Resident {
                content: ResidentContent::Surface(SurfaceSeam::default()),
                transform: RegionTransform::identity(),
                effects: TextEffects::default(),
            },
        ],
        size: (120.0, 60.0),
    };
    let window = VisibleWindow {
        first_visible_line: 0,
        block_offset: 0.0,
    };

    let (result, warns, errors) = with_log_cage(|| {
        executor.render(&canvas, &window, &font, mode, &contract, &mut surface)
    });
    result.expect("シーム住人があっても render は成功する（skip 継続）");
    assert!(warns >= 1, "シーム住人の描画要求は warn を記録する");
    assert_eq!(errors, 0);
    let bytes = surface.read_back().expect("read_back 失敗");
    assert!(
        bytes.iter().all(|&b| b == 0),
        "シーム住人は実挙動なし＝インクを一切描かない（R8.5）"
    );

    let (result, warns2, _) = with_log_cage(|| {
        executor.render(&canvas, &window, &font, mode, &contract, &mut surface)
    });
    result.expect("2 回目の render も成功する");
    assert_eq!(
        warns2, 0,
        "シーム warn は executor ごと初回のみ（スパム抑制）"
    );
}

/// 観測可能な完了状態（task 6.2）: LayoutEngine の外部注入点（&dyn GlyphMetrics）へ
/// 実測値ベースの送り幅を提供でき、折返しが実測 advance で決まる——probe（計測）が
/// 折返し決定より前＝鶏卵にならない順序で機能する単体確認。
#[test]
fn layout_engine_wraps_using_measured_advances() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
    let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
    let full = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
    // 折返し閾値＝実測全角 1.5 個分: 2 文字目で「行内位置＋次グリフ幅 > 閾値」が成立。
    let threshold = (full * 1.5).round() as i32;
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(Some(threshold), None),
        ValidRect::new(None, None, None, None),
        empty_font(),
        None,
        None,
    );
    let region = TextRegion::resolve(&model, (400, 224), WritingMode::HorizontalTb);
    let items = [TextItem::Glyph { ch: 'あ' }, TextItem::Glyph { ch: 'あ' }];
    let lines = LayoutEngine::layout(
        &items,
        2,
        &region,
        WritingMode::HorizontalTb,
        DEFAULT_FONT_HEIGHT,
        &metrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "実測 advance が折返し判定を駆動する");
    assert_eq!(lines[0].glyphs.len(), 1);
    assert_eq!(
        lines[0].glyphs[0].advance, full,
        "配置グリフの advance は実測値"
    );
    assert_eq!(
        lines[1].glyphs[0].inline_pos, 0.0,
        "折返し行は行内開始へ戻る"
    );
}

// ── task 6.4 R4.5/R6.1–6.3/R7.5: probe/描画行 TextLayout の送り幅一致 invariant ──
//
// design Testing Strategy Integration #5（正典）: 同一 TextFormat・同一テキストで
// probe layout（DWriteMetrics＝per-char 計測）と描画行 TextLayout
// （DrawExecutor::line_layout＝render が DrawTextLayout へ渡す実物）の cluster
// advance が**同値**であることを、等幅（ＭＳ ゴシック）＋プロポーショナル欧文混在
// （ＭＳ Ｐゴシック）の両方で檻化する。
//
// 許容誤差: なし（f32 完全一致）。同一 factory・同一 create_text_format 経路・
// 同一テキストに対する DirectWrite 計測は決定論であり、design の「同値」が正準
// ——epsilon を挟むと per-char probe と行文脈（カーニング等）の乖離
// （6.2 申し送りの検出責務）を握りつぶすため導入しない。

/// テキストごとの invariant 検証ケース（フォント名 None＝既定 ＭＳ ゴシック）。
const INVARIANT_CASES: [(Option<&str>, &str); 2] = [
    // 等幅: 既定 ＭＳ ゴシック・全角/半角混在。
    (None, "あiWa。漢！x"),
    // プロポーショナル欧文混在: ＭＳ Ｐゴシック・'i'/'W' 等の可変幅＋全角同居。
    (Some("ＭＳ Ｐゴシック"), "iWMjlあ。W"),
];

/// ケースの ResolvedFont（height 12 固定・color 既定）。
fn invariant_font(name: Option<&str>) -> ResolvedFont {
    let font = Font::new(
        name.map(str::to_owned),
        Some(12),
        FontColor::new(None, None, None),
    );
    ResolvedFont::resolve(&model_with_font(font))
}

/// DrawExecutor の実描画経路（ensure_format→line_layout）で行 TextLayout を組み、
/// cluster metrics の幅列を返す——render が DrawTextLayout へ渡すのと同一の layout
/// （検証用の別組みではない）から実描画送り幅を読む。
fn drawn_line_cluster_widths(
    executor: &mut DrawExecutor,
    text: &str,
    font: &ResolvedFont,
    mode: WritingMode,
) -> Vec<f32> {
    let format = executor
        .ensure_format(font, mode)
        .expect("ensure_format 失敗");
    let layout = executor
        .line_layout(0, text, &format, font.height, mode)
        .expect("line_layout 失敗");
    layout
        .get_cluster_metrics()
        .expect("GetClusterMetrics(描画行) 失敗")
        .iter()
        .map(|c| c.width)
        .collect()
}

/// 観測可能な完了状態（task 6.4）: 同一フォント設定・同一テキストで、計測専用
/// probe の per-char advance と描画行 TextLayout の per-cluster advance が完全一致
/// する（等幅＋プロポーショナル欧文混在 × 3 方向）。プロポーショナルの行文脈調整
/// （カーニング等）で per-char probe と行計測が乖離するなら、本檻がその文字で
/// 赤くなる（6.2 申し送りの検出責務・クリップでは隠れない metrics 述語）。
#[test]
fn probe_advances_match_drawn_line_cluster_advances() {
    let rig = DrawRig::new();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");
    for (name, text) in INVARIANT_CASES {
        let font = invariant_font(name);
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let metrics =
                DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
                    .expect("DWriteMetrics 生成失敗");
            let widths = drawn_line_cluster_widths(&mut executor, text, &font, mode);
            assert_eq!(
                widths.len(),
                text.chars().count(),
                "{} {mode:?}: 検証テキストは 1 文字=1 cluster の前提",
                font.name
            );
            for (ch, width) in text.chars().zip(&widths) {
                let probe = metrics.advance(ch, font.height);
                assert!(probe > 0.0, "{} {mode:?} {ch:?}: probe は正値", font.name);
                assert_eq!(
                    probe, *width,
                    "{} {mode:?} {ch:?}: probe advance（計測専用）と描画行 cluster \
                     advance（実描画）の同値 invariant",
                    font.name
                );
            }
        }
    }
}

/// 乖離の検出形式（task 6.4「クリップで隠さない」）: probe 駆動で折返した各行に
/// ついて、描画行 TextLayout の送り終端（行内開始＋cluster advance の同順逐次加算
/// ——LayoutEngine の inline_pos 累積と同じ f32 結合順）が行矩形の行内終端と完全
/// 一致し、かつ折返し閾値を超えない。probe と実描画の送り幅が乖離すれば、行終端の
/// 不一致＝**折返し位置のズレ**としてここで赤くなる（供給面クリップに依存しない
/// metrics 述語——ピクセル述語は 9.2/10.2 の領分）。
#[test]
fn advance_divergence_would_surface_as_wrap_position_drift() {
    let rig = DrawRig::new();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");
    for (name, text) in INVARIANT_CASES {
        let font = invariant_font(name);
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let metrics =
                DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
                    .expect("DWriteMetrics 生成失敗");
            // 折返し閾値＝先頭 4 文字の probe 送り終端の floor——実測値が閾値と
            // 折返し位置の両方を駆動する形（metrics が違えば折返しもズレる）。
            let cum4: f32 = text
                .chars()
                .take(4)
                .map(|ch| metrics.advance(ch, font.height))
                .sum();
            let threshold = cum4.floor() as i32;
            assert!(threshold >= 1, "{} {mode:?}: 閾値は正値", font.name);
            // 幾何モデル: 横書き＝origin(0,0)＋wordwrappoint.x・縦書き＝origin 既定
            // （書字開始角）＋wordwrappoint.y（軸読み替え正準表）。
            let (origin, wordwrap) = match mode {
                WritingMode::HorizontalTb => (
                    Origin::new(Some(0), Some(0)),
                    WordWrapPoint::new(Some(threshold), None),
                ),
                WritingMode::VerticalRl | WritingMode::VerticalLr => (
                    Origin::new(None, None),
                    WordWrapPoint::new(None, Some(threshold)),
                ),
            };
            let model = BalloonModel::new(
                WindowPosition::new(None, None),
                origin,
                wordwrap,
                ValidRect::new(None, None, None, None),
                empty_font(),
                None,
                None,
            );
            let region = TextRegion::resolve(&model, (400, 224), mode);
            let items = glyph_items(text);
            let lines =
                LayoutEngine::layout(
                    &items,
                    items.len(),
                    &region,
                    mode,
                    font.height,
                    &metrics,
                    WrapPlan::CharByChar,
                );
            assert!(
                lines.len() >= 2,
                "{} {mode:?}: 実測駆動の折返しが実際に発生する構成",
                font.name
            );
            let placed: usize = lines.iter().map(|l| l.glyphs.len()).sum();
            assert_eq!(placed, text.chars().count(), "折返しでグリフを失わない");
            for (i, line) in lines.iter().enumerate() {
                let line_text: String = line.glyphs.iter().map(|g| g.ch).collect();
                let widths = drawn_line_cluster_widths(&mut executor, &line_text, &font, mode);
                let (inline_start, inline_end) = match mode {
                    WritingMode::HorizontalTb => (line.rect.left, line.rect.right),
                    WritingMode::VerticalRl | WritingMode::VerticalLr => {
                        (line.rect.top, line.rect.bottom)
                    }
                };
                // LayoutEngine の inline_pos 累積と同じ逐次加算（f32 結合順まで一致）。
                let mut drawn_end = inline_start;
                for w in &widths {
                    drawn_end += *w;
                }
                assert_eq!(
                    drawn_end, inline_end,
                    "{} {mode:?} 行 {i} ({line_text:?}): 実描画の送り終端＝計測 \
                     レイアウトの行内終端（不一致＝折返し位置のズレとして検出）",
                    font.name
                );
                assert!(
                    inline_end <= threshold as f32 || line.glyphs.len() == 1,
                    "{} {mode:?} 行 {i}: 行終端 {inline_end} は折返し閾値 {threshold} 内 \
                     （行頭 1 グリフ縮退を除く）",
                    font.name
                );
            }
        }
    }
}
