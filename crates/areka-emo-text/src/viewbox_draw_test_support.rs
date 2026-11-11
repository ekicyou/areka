use super::ViewboxExecutor;
use crate::actor::test_support::{make_dispatcher_and_compositor, spawn_reserved_slot};
use crate::actor::{ResolvedBalloonText, TextSlotBinding};
use crate::canvas::{ChoiceLineContent, ContentCanvas, GlyphRunContent, Resident, ResidentContent};
use crate::draw::test_support::PASTA_FONT_NAMES;
use crate::draw::{DrawExecutor, FontCatalog, ResolvedFont};
use crate::layout::{
    GlyphMetrics, LayoutEngine, PositionedGlyph, PositionedLine, VisibleWindow, WrapPlan,
};
use crate::look::{StyleId, StyleTable, TextLook};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{TextItem, TextLayerState};
use crate::surface::TextSurface;
use crate::viewbox::test_support::layout_canvas;
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::cluster::{cluster_count, clusters};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::prelude::World;
use sample_ghost_kit::SampleRoot;
use std::path::PathBuf;
use std::sync::LazyLock;
use windows::UI::Composition::Compositor;
use wintf::ecs::GraphicsCore;

/// emo2 検体。段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。
/// 保持を本ファイル 1 か所に束ねてあるので、段 ③ の複製も lib テストバイナリあたり 1 回で済む。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// emo2 が同時にインストールするバルーンのフォルダ（自分でパスを継ぎ足さない）。
pub(super) fn emo2_balloon_root() -> PathBuf {
    EMO2.balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}

/// 描画の検証リグ（dispatcher/compositor/core/World の寿命を束ねる・headless）。
/// 見本（`DrawExecutor`）の檻と viewbox の檻が同じものを使う。
pub(crate) struct Rig {
    pub(crate) _dq: windows::System::DispatcherQueueController,
    pub(crate) compositor: Compositor,
    pub(crate) core: GraphicsCore,
    pub(crate) world: World,
}

impl Rig {
    pub(crate) fn new() -> Rig {
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
    pub(crate) fn attach(&mut self, image_size: (u32, u32), k: f32) -> TextSurface {
        let (window, slot) = spawn_reserved_slot(&mut self.world);
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
pub(crate) fn geo_model(font_height: Option<u32>) -> BalloonModel {
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
/// 指定可・validrect 全域）。このモデルは validrect を面の全域（左上 (0,0) から右下まで）に
/// するので、(0,0) は範囲の手前端そのもの＝範囲内である（両端を含む）。そのため origin (0,0)
/// を明示すれば宣言がそのまま用いられて面の左上から書き始まり、vertical_rl は列送りが −x
/// ゆえ列が面外（負の x）へ描かれてしまう。
/// そのため origin は None にして、未宣言時の縮退＝書字開始角
/// （horizontal_tb/vertical_lr＝validrect 左上・vertical_rl＝右上）へ委ねる
/// （spec `areka-P0-balloon-vertical-canon` の要件 3.10／3.11）。
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

/// 文字列→グリフ item 列（クラスタの定義点で切る＝本番の cue の適用と同じ切り方）。
pub(crate) fn glyph_items(s: &str) -> Vec<TextItem> {
    clusters(s).map(TextItem::glyph).collect()
}

/// items→(canvas, visible_window)（純粋レイアウト・FixedMetrics・visible は全量）。
pub(super) fn build(
    items: &[TextItem],
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
) -> (ContentCanvas, VisibleWindow) {
    let (lines, canvas) = layout_canvas(items, region, mode, font_height);
    let window = LayoutEngine::visible_window(&lines, region, mode);
    (canvas, window)
}

/// 非透明ピクセル数（BGRA 密配列の α≠0）。
pub(crate) fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 「文字色（黒）でも背景（透明）でもない色」の画素数: α≠0 かつ B/G/R のいずれか＞0
/// （premultiplied の黒は 0,0,0 なので、黒の文字の縁の半透明も数えない）。
pub(super) fn colored_count(bytes: &[u8]) -> usize {
    bytes
        .chunks_exact(4)
        .filter(|px| px[3] != 0 && (px[0] > 0 || px[1] > 0 || px[2] > 0))
        .count()
}

/// Segoe UI Emoji が引けなければ理由を添えて失敗にする（飛ばして緑にしない・要件 6.7）。
/// 色の判定は代替フォントの色つきの字形を前提にするので、色を判定するテストの先頭で呼ぶ。
pub(super) fn require_segoe_ui_emoji(fonts: &FontCatalog) {
    if fonts.family_for(&["Segoe UI Emoji".to_string()]).is_none() {
        panic!(
            "Segoe UI Emoji が見つからない——Windows 10/11 の同梱フォントなので、無いのは環境の異常か\
             家族名の綴りの誤り（調査 2026-09-29 では `Some(\"Segoe UI Emoji\")`）"
        );
    }
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

/// emo2 の同梱バルーン（`descript.txt`＋`balloons0s.txt`）を実ファイルから読み、balloon 画像
/// 原寸（surface0＝400×224）で解いた文字の設定（Yu Gothic UI・validrect 320×122）。
pub(super) fn emo2_balloon_text() -> ResolvedBalloonText {
    let fixture_dir = emo2_balloon_root();
    let read_dec = |name: &str| -> String {
        let bytes = std::fs::read(fixture_dir.join(name))
            .unwrap_or_else(|e| panic!("fixture {name} 読取失敗: {e}"));
        areka_parsers::charset::decode(&bytes, areka_parsers::charset::DefaultEncoding::Utf8)
    };
    let model = areka_parsers::balloon::parse_str(
        &read_dec("descript.txt"),
        Some(&read_dec("balloons0s.txt")),
    );
    ResolvedBalloonText::resolve(&model, (400, 224))
}

/// 描画範囲（validrect）の寸を整数へ切り上げたもの（k=1 の供給面の物理寸）。
pub(super) fn region_extent(region: &TextRegion) -> (u32, u32) {
    (
        (region.right() - region.left()).ceil() as u32,
        (region.bottom() - region.top()).ceil() as u32,
    )
}

/// example の共有の台本（行 1「おっはよー！」・行 2「めっちゃええ朝やん！」・行 3「今日もいくでー！」・
/// あふれを起こす短い行「ほな」9 本）を `state` へ積む。`at` は行 1・行 2・行 3・短い行の各 cue の
/// 時刻（行の前の改行も同じ時刻）。reveal は配送 duration 由来（interval = duration / N）なので、
/// Text へ字数 × `interval` を焼き込むと 1 字ずつ `interval` の間隔で見える。
pub(super) fn apply_fixture_script(
    state: &mut TextLayerState,
    actor: &ActorKey,
    at: [f64; 4],
    interval: f64,
) {
    let cue = |at: f64, command: CueCommand| TalkCue {
        at,
        actor: actor.clone(),
        duration: match &command {
            CueCommand::Text(t) => cluster_count(t) as f64 * interval,
            _ => 0.0,
        },
        command,
    };
    let newline = || CueCommand::NewLine { ratio: 1.0 };
    state.apply_cue(&cue(at[0], CueCommand::Text("おっはよー！".into())));
    state.apply_cue(&cue(at[1], newline()));
    state.apply_cue(&cue(at[1], CueCommand::Text("めっちゃええ朝やん！".into())));
    state.apply_cue(&cue(at[2], newline()));
    state.apply_cue(&cue(at[2], CueCommand::Text("今日もいくでー！".into())));
    for _ in 0..9 {
        state.apply_cue(&cue(at[3], newline()));
        state.apply_cue(&cue(at[3], CueCommand::Text("ほな".into())));
    }
}

/// items の先頭 `visible` 字を配置し、(行, canvas, 可視窓) を返す（描画の檻の 1 フレームの組み立て）。
pub(crate) fn layout_frame(
    items: &[TextItem],
    visible: usize,
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
    metrics: &dyn GlyphMetrics,
    wrap: WrapPlan<'_>,
) -> (Vec<PositionedLine>, ContentCanvas, VisibleWindow) {
    let lines = LayoutEngine::layout(items, visible, region, mode, font_height, metrics, wrap);
    let canvas = ContentCanvas::from_layout(&lines, region, mode);
    let window = LayoutEngine::visible_window(&lines, region, mode);
    (lines, canvas, window)
}

/// `state` の `actor` を時刻 `t` まで見せた 1 フレーム（文字単位の折返し）。
/// 返り値は (見せた字の数, 行, canvas, 可視窓)。
pub(super) fn state_frame(
    state: &TextLayerState,
    actor: &ActorKey,
    t: f64,
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
    metrics: &dyn GlyphMetrics,
) -> (usize, Vec<PositionedLine>, ContentCanvas, VisibleWindow) {
    let items: Vec<TextItem> = state
        .actor_state(actor)
        .map(|s| s.items().to_vec())
        .unwrap_or_default();
    let visible = state.visible_glyphs(actor, t);
    let (lines, canvas, window) = layout_frame(
        &items,
        visible,
        region,
        mode,
        font_height,
        metrics,
        WrapPlan::CharByChar,
    );
    (visible, lines, canvas, window)
}

/// 見本（全域再描画の [`DrawExecutor`]・front へ焼く）と viewbox（ダーティ描画・back へ焼いて
/// flip）を同じ寸の 2 面へ装着した対。どちらも read_back は front を読むので直接比べられる。
pub(super) struct RenderPair {
    /// 見本の供給面。
    pub(super) oracle_surface: TextSurface,
    /// viewbox の供給面。
    pub(super) viewbox_surface: TextSurface,
    /// 比較専用の見本。
    pub(super) oracle: DrawExecutor,
    /// 本番の実行部。
    pub(super) viewbox: ViewboxExecutor,
}

impl RenderPair {
    /// `rig` へ同じ寸（ceil(image × k)）の 2 面を装着し、両実行部を作る。
    pub(super) fn attach(rig: &mut Rig, image: (u32, u32), k: f32) -> RenderPair {
        RenderPair {
            oracle_surface: rig.attach(image, k),
            viewbox_surface: rig.attach(image, k),
            oracle: DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗"),
            viewbox: ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗"),
        }
    }

    /// 同じ canvas・可視窓を両方へ描き、読み戻し `(見本, viewbox)` を返す。
    pub(super) fn render(
        &mut self,
        label: &str,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        font: &ResolvedFont,
        mode: WritingMode,
        contract: &ScaleContract,
    ) -> (Vec<u8>, Vec<u8>) {
        self.oracle
            .render(
                canvas,
                window,
                font,
                mode,
                contract,
                &mut self.oracle_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: オラクル render 失敗: {e:?}"));
        self.viewbox
            .render(
                canvas,
                window,
                font,
                mode,
                contract,
                &mut self.viewbox_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: viewbox render 失敗: {e:?}"));
        let ob = self
            .oracle_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: オラクル read_back 失敗: {e:?}"));
        let vb = self
            .viewbox_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: viewbox read_back 失敗: {e:?}"));
        (ob, vb)
    }
}

/// 同じ寸（`w`×`h`）の 2 つの読み戻しで食い違う行の y の一覧。
pub(super) fn diff_rows(a: &[u8], b: &[u8], w: u32, h: u32) -> Vec<u32> {
    let row = |y: u32| ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
    (0..h).filter(|&y| a[row(y)] != b[row(y)]).collect()
}

/// canvas の GlyphRun 住人だけを `choice(run)` の Choice 住人へ写す（transform・effects と
/// ほかの住人はそのまま）。選択肢の帯の檻が GlyphRun の canvas から Choice の canvas を作る。
pub(super) fn choice_canvas_from(
    canvas: &ContentCanvas,
    choice: impl Fn(&GlyphRunContent) -> ChoiceLineContent,
) -> ContentCanvas {
    let residents = canvas
        .residents
        .iter()
        .map(|r| match &r.content {
            ResidentContent::GlyphRun(run) => Resident {
                content: ResidentContent::Choice(choice(run)),
                transform: r.transform,
                effects: r.effects,
            },
            _ => r.clone(),
        })
        .collect();
    ContentCanvas {
        residents,
        size: canvas.size,
    }
}

/// 塗りの色（premultiplied BGRA・α=255）＝ `SquareFill { fill: (105, 25, 25) }`。
pub(super) const FILL_BGRA: [u8; 4] = [25, 25, 105, 255];

/// 列 `x0..x1`（全 y）の画素のうち `pred` が真のものを数える（BGRA 密配列・幅 `w`）。
pub(super) fn count_in_x_band(
    bytes: &[u8],
    w: u32,
    x0: u32,
    x1: u32,
    pred: impl Fn(&[u8]) -> bool,
) -> usize {
    bytes
        .chunks_exact(4)
        .enumerate()
        .filter(|(i, px)| {
            let x = *i as u32 % w;
            x >= x0 && x < x1 && pred(px)
        })
        .count()
}

/// premultiplied BGRA 密配列の全域から、指定 BGRA を持つ画素の x 範囲（左端, 右端）を返す。
pub(super) fn bgra_x_span(bytes: &[u8], w: u32, h: u32, target: [u8; 4]) -> Option<(u32, u32)> {
    let mut span: Option<(u32, u32)> = None;
    for x in 0..w {
        let hit = (0..h).any(|y| {
            let o = ((y * w + x) * 4) as usize;
            bytes[o..o + 4] == target
        });
        if hit {
            span = Some(match span {
                None => (x, x),
                Some((l, _)) => (l, x),
            });
        }
    }
    span
}

/// premultiplied BGRA 密配列の全域から、指定 BGRA を持つ画素の y 範囲（上端, 下端）を返す。
pub(super) fn bgra_y_span(bytes: &[u8], w: u32, h: u32, target: [u8; 4]) -> Option<(u32, u32)> {
    let rows: Vec<u32> = (0..h)
        .filter(|&y| {
            (0..w).any(|x| {
                let o = ((y * w + x) * 4) as usize;
                bytes[o..o + 4] == target
            })
        })
        .collect();
    Some((*rows.first()?, *rows.last()?))
}

/// 帯の読み戻しの書体（本番と同じ解決に通す）と、引けるはずの書体の名前（違えば赤）。ぱすたの
/// `pasta_tategaki` の `font.name` は候補列のまま書き、`FontCatalog::pick` で先頭の「游ゴシック Medium」が
/// GDI の名前として引け、その家族名「Yu Gothic」になる（emo-text-dwrite-fidelity 要件 3.1・タスク 1.2）。
pub(super) const R_FONTS: [(&str, &str); 2] = [
    ("Yu Gothic UI", "Yu Gothic UI"),
    (PASTA_FONT_NAMES, "Yu Gothic"),
];
/// ぱすたの測りと同じ 4 つの選択肢（列ごと）。
pub(super) const R_CHOICES: [&str; 4] = ["晴れてる", "雨だよ", "曇ってる", "ないしょ"];
/// 記録だけの横倒しの欧文の列（選択肢にしない・合否に入れない）。
pub(super) const R_LATIN: &str = "Ag";

/// 番号だけを与えたグリフ（位置と送りは区間の切り出しに効かないので 0）。
pub(super) fn glyph(text: &str, style: StyleId) -> PositionedGlyph {
    PositionedGlyph {
        text: text.into(),
        inline_pos: 0.0,
        advance: 0.0,
        style,
    }
}

/// 番号 1 に `look` を積んだ表を返す（`intern` 経由＝本番と同じ畳み込み規則）。
pub(super) fn table_with(look: &TextLook, default: &TextLook) -> (StyleTable, StyleId) {
    let mut table = StyleTable::default();
    let id = table.intern(look, default);
    assert_ne!(
        id,
        StyleId::DEFAULT,
        "検査用の見た目は既定と異なる（番号 0 に畳み込まれると檻が空振りする）"
    );
    (table, id)
}
