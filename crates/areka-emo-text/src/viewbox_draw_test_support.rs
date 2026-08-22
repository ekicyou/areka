use crate::actor::TextSlotBinding;
use crate::canvas::ContentCanvas;
use crate::layout::{FixedMetrics, LayoutEngine, VisibleWindow, WrapPlan};
use crate::region::TextRegion;
use crate::state::TextItem;
use crate::surface::TextSurface;
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::UI::Composition::Compositor;
use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
use wintf::com::wuc::create_dispatcher_queue_controller;
use wintf::ecs::{GraphicsCore, Visual};

/// テスト用 WUC apartment/dispatcher（surface.rs/draw.rs テストと同一方針:
/// COM 未初期化のテストスレッドでは ASTA 第一候補・NONE 保険）。
pub(super) fn make_dispatcher_and_compositor()
-> (windows::System::DispatcherQueueController, Compositor) {
    let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
        .or_else(|e_asta| create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta))
        .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
    let compositor = Compositor::new().expect("Compositor::new 失敗");
    (dq, compositor)
}

/// ViewboxExecutor 検証リグ（dispatcher/compositor/core/World の寿命を束ねる・headless）。
pub(super) struct Rig {
    pub(super) _dq: windows::System::DispatcherQueueController,
    pub(super) compositor: Compositor,
    pub(super) core: GraphicsCore,
    pub(super) world: World,
}

impl Rig {
    pub(super) fn new() -> Rig {
        let (_dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
        Rig {
            _dq,
            compositor,
            core,
            world: World::new(),
        }
    }

    /// 予約スロット（emo-present VisualMount 同型）を組み、image px 原寸と k から
    /// TextSurface を装着する（物理寸＝ceil(image × k)・offset (0,0)）。
    pub(super) fn attach(&mut self, image_size: (u32, u32), k: f32) -> TextSurface {
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

/// テスト用 BalloonModel（幾何のみ・origin (0,0)・font 高さ指定可・validrect 全域）。
pub(super) fn geo_model(font_height: Option<u32>) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, font_height, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// live-diff 用 BalloonModel（**origin 未指定**＝mode ごとの書字開始角へ寄せる・font 高さ
/// 指定可・validrect 全域）。origin (0,0) を明示すると vertical_rl では validrect 内の
/// 左上に留まり列が面外（負の x）へ描かれてしまうため、origin は None にして
/// クランプ正準（horizontal/vertical_lr＝左上・vertical_rl＝右上）へ委ねる。
/// フォント名＋高さを明示した live-diff 用モデル（既定フォントは `name=None`＝ＭＳ ゴシック・
/// G4——非 default フォント/大サイズで AA こぼれガード `DIRTY_GUARD_IMG_PX` の実効性を byte
/// 等価で検証するため）。
pub(super) fn live_diff_model_font(name: Option<&str>, font_height: Option<u32>) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(
            name.map(str::to_owned),
            font_height,
            FontColor::new(None, None, None),
        ),
        None,
        None,
    )
}

/// 文字列→グリフ item 列。
pub(super) fn glyph_items(s: &str) -> Vec<TextItem> {
    s.chars().map(|ch| TextItem::Glyph { ch }).collect()
}

/// items→(canvas, visible_window)（純粋レイアウト・FixedMetrics・visible は全量）。
pub(super) fn build(
    items: &[TextItem],
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
) -> (ContentCanvas, VisibleWindow) {
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    let lines = LayoutEngine::layout(
        items,
        visible,
        &region,
        mode,
        font_height,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    let canvas = ContentCanvas::from_layout(&lines, &region, mode);
    let window = LayoutEngine::visible_window(&lines, &region, mode);
    (canvas, window)
}

/// 非透明ピクセル数（BGRA 密配列の α≠0）。
pub(super) fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// ブロック軸（行送り軸）方向のインク範囲 `(near, far)`（物理 px・両端含む・インクなしは `None`）。
/// horizontal_tb＝y（上端〜下端）・vertical_rl/lr＝x（左端〜右端）——スクロールで content が
/// 動く軸。k≠1.0 の許容差比較（G2）で oracle と viewbox のインク位置差を測る。
pub(super) fn block_axis_ink_span(
    bytes: &[u8],
    w: u32,
    h: u32,
    mode: WritingMode,
) -> Option<(u32, u32)> {
    let mut lo: Option<u32> = None;
    let mut hi: u32 = 0;
    for y in 0..h {
        for x in 0..w {
            if bytes[((y * w + x) * 4 + 3) as usize] != 0 {
                let b = match mode {
                    WritingMode::HorizontalTb => y,
                    WritingMode::VerticalRl | WritingMode::VerticalLr => x,
                };
                lo = Some(lo.map_or(b, |v| v.min(b)));
                hi = hi.max(b);
            }
        }
    }
    lo.map(|l| (l, hi))
}
