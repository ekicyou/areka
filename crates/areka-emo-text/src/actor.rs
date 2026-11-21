//! # actor — UI ドレインとフレーム提示ステップ（結線層）
//!
//! `spawn_emo_text`（UI ドレイン起動）・`TextLayerRuntime`（UI スレッド所有の集約ルート）・
//! `TextSlotBinding`・`present_frame`（毎フレームの注入時刻駆動：リビール進行→レイアウト→描画→装着）を担う。
//!
//! 終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`、全 `UiSender` drop＝drain 正常終了
//! （いずれも error ログなし）。個別メッセージの処理失敗は `Err` 戻し→基盤が `error!`＋継続する。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use areka_emo_compose::{BoxLayout, BoxName, BoxPlacement};
use areka_emo_present::TextSlotView;
use areka_emo_present::balloon_marker::{MarkerImage, MarkerPair};
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_sakura::strict::DetectionPoint;
use bevy_ecs::entity::Entity;
use tracing::debug;

use crate::choice::ResolvedChoiceStyle;
use crate::cursor_tag::CursorWarnGuard;
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, DWriteMetrics, ResolvedFont};
use crate::lookahead::{TalkLookahead, advance_state};
use crate::place::PlaceKey;
use crate::region::{ScaleContract, TextRegion};
use crate::scroll_marker::{PlaceScroll, ResolvedMarkerPlacement, StopReason};
use crate::sink::{EmoTextSink, TextMsg, handle_text_msg_with};
use crate::state::{SpanKind, SurfaceKeyOutcome, TextLayerConfig, TextLayerState};
use crate::surface::TextSurface;
use crate::viewbox_draw::{DrawStats, ViewboxExecutor};
use crate::wrap::WrapMode;
use crate::writing::WritingMode;

/// actor の装着先（結線側が emo-present `TextSlotView` から構築して routing へ登録する）。
///
/// [`crate::surface::TextSurface::attach`] の入力。emo-present は actor を知らない——
/// `ActorKey → TargetId` の対応は結線側が所有し、`text_slot_view(target)` で得た view の値から
/// [`Self::new`] で組む。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSlotBinding {
    /// 予約スロット（`emo-text-layer-slot` Visual entity・emo-present `VisualMount` が予約）。
    pub slot: Entity,
    /// 装着先の窓 entity。
    pub window: Entity,
    /// 合成スケール k（`TextSlotView.scale` 由来・窓 DPI 由来で 1.0 とは限らない）。
    /// 不正値は構築時に [`ScaleContract`] の縮退規約（warn!＋1.0）で正規化済み。
    pub scale: f32,
    /// バルーン surface の物理原寸（TextSurface/swapchain の物理化に使用）。
    pub surface_size: (u32, u32),
    /// 画像座標空間の原寸（負値=反対辺解決・`TextRegion::resolve` の入力）。
    ///
    /// 作者画像空間の原寸そのもの＝k 不変。emo-present の native 原寸
    /// （`TextSlotView::surface_size`・k 適用前）をそのまま透過する。
    /// `TextRegion::resolve` へ物理 px を渡してはならない。
    pub image_size: (u32, u32),
}

impl TextSlotBinding {
    /// `TextSlotView` の読み値（slot/window/scale/物理原寸/native 原寸）から binding を構築する。
    ///
    /// k の正規化（0 以下・非有限→warn!＋1.0 縮退）は [`ScaleContract::new`] に委譲する。
    ///
    /// `image_size` は `round(physical_size / k)` の逆写像で復元せず、presenter が既に正確に持つ
    /// native 原寸を第 5 引数で受けて透過する。逆写像は k<1 で誤差が ±0.5/k 画像 px に増幅され
    /// 1px ずれる（例: k=4/5・native 143 → physical 114 → `round(114/0.8)` = 142）。
    /// k<1 は本番到達可能（`parse_author_dpi` は宣言値を素通しするため、`dpi,120` を宣言した
    /// ゴーストを 100% モニタで表示すれば k=4/5）。
    pub fn new(
        slot: Entity,
        window: Entity,
        scale: f32,
        surface_size: (u32, u32),
        image_size: (u32, u32),
    ) -> Self {
        let contract = ScaleContract::new(scale, None);
        TextSlotBinding {
            slot,
            window,
            scale: contract.scale,
            surface_size,
            image_size,
        }
    }

    /// emo-present の読み取り専用増分 `TextSlotView` からの一点変換。
    ///
    /// [`TextSlotView`] が隣り合わせで公開する native 原寸（`surface_size()`・k 適用前）を `image_size` へ、
    /// 物理寸（`physical_size()`・k 適用後）を `surface_size`（診断・churn 判定用の表示寸）へ写す。
    /// 取り違えるとどちらの向きでも静かに壊れる——`image_size` に物理寸を入れると画像空間が k 倍に膨らみ、
    /// `surface_size` に native を入れると表示寸の記録が k に追随しなくなる。
    pub fn from_view(view: &TextSlotView) -> Self {
        TextSlotBinding::new(
            view.slot(),
            view.window(),
            view.scale(),
            view.physical_size(),
            view.surface_size(),
        )
    }
}

/// actor 1 人分の layout 入力——`writing_mode`／領域／フォント／折返しモードの解決済み束
/// （`TextLayerRuntime.layout_input` の値型）。
///
/// 解決は既存の一点解決口（[`WritingMode::resolve`]／[`TextRegion::resolve`]／
/// [`ResolvedFont::resolve`]／[`WrapMode::resolve`]）の合成で、本型は束ねるだけ。
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedBalloonText {
    /// 2 層マージ済み `writing_mode` の解釈結果。
    pub mode: WritingMode,
    /// 解決済みテキスト領域（validrect 絶対矩形・描画開始点・折返し閾値——全値 image px）。
    pub region: TextRegion,
    /// 解決済みフォント一式（欠落は ukadoc 既定で充足済み）。
    pub font: ResolvedFont,
    /// 折返しモードの解釈結果（`budoux_newline` 語彙解決・ON 時のみ分かち書き境界を計算）。
    pub wrap: WrapMode,
    /// hover ハイライトスタイルの解決正規形（balloon `cursor.*` モデル＋既定文字色から一点解決）。
    /// 装飾（`decorate_canvas`）が hover 行へ焼く塗り/文字色の源。
    pub choice_style: ResolvedChoiceStyle,
    /// スクロールの矢印の置き場（`arrow0`／`arrow1` の左上・image px）。モデルの矢印の鍵を
    /// 広がり（普通のバルーン＝面の画像・箱＝箱の大きさ）で解いた値。
    pub markers: ResolvedMarkerPlacement,
}

impl ResolvedBalloonText {
    /// balloon model（2 層マージ済み）とバルーン画像原寸（image px・
    /// [`TextSlotBinding::image_size`]）から解決する。物理 px を渡してはならない。
    pub fn resolve(model: &BalloonModel, image_size: (u32, u32)) -> ResolvedBalloonText {
        ResolvedBalloonText::resolve_with_background(model, image_size, DEFAULT_BALLOON_BACKGROUND)
    }
}

/// choice.rs（純粋層）所有のバルーン窓物理 px 矩形を結線層から再輸出する。
/// [`ChoiceHitRow::rect`] の座標系の正本。
pub use crate::choice::HitRectPx;

/// 行ヒットジオメトリ契約。
///
/// 提示フレーム同期スナップショットの 1 行分——1 選択肢セグメントの窓物理 px 矩形に、
/// `ChoiceSelection` 構成材料（`ordinal`/`id`/`label`/`references`）を同梱する。
/// スナップショットは present_actor（`actor_present.rs`）が提示に成功するたびに入れ直す。
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceHitRow {
    /// 元の範囲の種類（選択肢／アンカー・[`crate::state::ChoiceSpan::kind`] の写し）。
    pub kind: SpanKind,
    /// スパンの配送順序数（[`crate::state::ChoiceSpan::ordinal`]・hover 注入／選択解決の主キー）。
    pub ordinal: usize,
    /// ID（`\q`／`\_a` の ID・不透明転写）。
    pub id: String,
    /// 表示文字列（不透明転写）。
    pub label: String,
    /// 付随する引数（`\q` は第 3 引数以降・`\_a` は第 2 引数以降・不透明転写）。
    pub references: Vec<String>,
    /// ヒット矩形（バルーン窓物理 px・スクロール committed 反映済み・[`HitRectPx`]）。
    pub rect: HitRectPx,
}

/// actor 1 人分の描画資源（供給面＋描画実行部＋実測 metrics）。font/mode に束縛されるため
/// actor 別に持つ。
struct ActorRender {
    /// 自前 swapchain 供給面（初回のみ予約スロットへ brush 装着・以降 Present のみ）。
    surface: TextSurface,
    /// viewbox ダーティ矩形スクロールの実行部（保持ピクセルの面内 blit ＋ ダーティ矩形限定描画・
    /// 行 TextLayout キャッシュは actor の行 index に束縛）。
    executor: ViewboxExecutor,
    /// 計測専用 probe 由来の実測 metrics（actor の font/mode に束縛）。
    metrics: DWriteMetrics,
    /// 印の面（スクロールの矢印・窓の子として文字の面の直前）。矢印を出すまでは `None`。
    markers: Option<MarkerSurface>,
    /// 作れなかった印の面の（物理寸, WUC の資源の変わった時刻）（上限越え・その寸での窓の子の作成の
    /// 失敗＝入力で答えが決まる失敗だけ）。同じなら作り直さない（`actor_present.rs` の `present_markers`）。
    markers_failed: Option<((u32, u32), Option<bevy_ecs::change_detection::Tick>)>,
    /// 前のフレームの配置の覚え（鍵と、行・区切り・帯・キャンバス・`actor_present.rs` の
    /// `FrameMemo`）。拡大率と装置の作り直しでは `ActorRender` ごと捨てるので、覚えも一緒に消える。
    frame_memo: Option<present::FrameMemo>,
    /// 届いた字の区切りの覚え（`segment.rs` の `BreakMemo`・`actor_present.rs` が再公開）。
    break_memo: present::BreakMemo,
    /// 配置から装飾までを作り直した回数（テストだけの数）。
    #[cfg(test)]
    frame_rebuilds: u32,
}

impl ActorRender {
    /// 印の面を消してから面を外す。差し込み口が分かれば外し（窓の子なら窓の子ごと消す）、
    /// 分からなければ窓の子だけを消す。
    fn discard(self, world: &mut bevy_ecs::world::World, slot: Option<Entity>) {
        if let Some(markers) = self.markers {
            markers.despawn(world);
        }
        match slot {
            Some(slot) => self.surface.release(world, slot),
            None => self.surface.despawn_window_child(world),
        }
    }
}

/// `\s` の鍵を今のシェルで解決する閉包（結線が渡す・文字の層は seriko の型を名指ししない）。
///
/// 中身は結線の純関数（`resolve_for_text`）で、今のシェルの面の表に無い番号は
/// [`SurfaceKeyOutcome::Unresolved`] を返す。
pub type SurfaceKeyResolver = Box<dyn Fn(&str) -> SurfaceKeyOutcome>;

/// UI スレッド所有の集約ルート。
///
/// 純粋状態（[`TextLayerState`]）・binding（結線側が登録）・COM 資源（actor 別の
/// 供給面/描画実行部——World 資源から遅延構築）を束ねる。`Rc<RefCell<_>>` で
/// [`spawn_emo_text`] の UI ドレインとフレーム提示ステップ（[`present_frame`]）が
/// 同一 UI スレッド上で共有する（`!Send`）。
///
/// - cue 適用（[`apply_cue`](Self::apply_cue)）は純粋状態の更新のみで World に触れない。
/// - World への装着・描画はフレーム提示ステップ（[`present_frame`]）が担う。
/// - 時刻は常に注入（`talk_time`）——内部で `Instant::now()` を読まない。
pub struct TextLayerRuntime {
    /// 純粋状態機械（actor 別・cue 列→行/グリフ状態）。
    state: TextLayerState,
    /// 場所 → 装着先（予約スロット）。結線側が [`register_actor`](Self::register_actor) で登録
    /// （スコープで引く口は普通のバルーンの場所 [`PlaceKey::balloon`] を引く）。
    routing: HashMap<PlaceKey, TextSlotBinding>,
    /// 場所 → layout 入力（writing_mode/region/font の解決済み束）。routing と対で登録。
    layout_input: HashMap<PlaceKey, ResolvedBalloonText>,
    /// 普通のバルーンの場所 → 前に解き直した入力（[`attach::ResolveInputs`]）。再追従は入力も結線も
    /// 同じなら解き直さず「変化なし」と答える。在れば、その入力から解いた値が `layout_input` に等しい
    /// （不変条件）。[`register_actor`](Self::register_actor) が消し、装着と再追従が入れ直す。
    /// 箱の場所には持たせない（箱は毎フレーム解き直さない）。
    resolve_inputs: HashMap<PlaceKey, attach::ResolveInputs>,
    /// actor 別の描画資源（遅延生成）。k 再追従（[`TextLayerRuntime::refresh_actor_scale`]）は
    /// 当該エントリだけを破棄して次フレームの再生成に委ね、純粋状態 `state` には触れない。
    surfaces: HashMap<PlaceKey, ActorRender>,
    /// 調整値（行間 `line_gap`）。`DWriteMetrics::new` の行送り算出にのみ使う。
    /// [`TextLayerRuntime::new`] で正規化済み。
    config: TextLayerConfig,
    /// 未解決 actor の warn を actor ごと初回のみに抑える記録（以降は debug!）。
    unresolved_warned: BTreeSet<PlaceKey>,
    /// actor → hover 注入状態（`None`＝ハイライト無し・[`inject_choice_hover`](Self::inject_choice_hover)
    /// で更新・present_actor の装飾が読む・UI スレッド専用）。
    choice_hover: HashMap<PlaceKey, Option<usize>>,
    /// actor → 提示フレーム同期ヒット行スナップショット（[`choice_hit_rows`](Self::choice_hit_rows)
    /// の照会源）。present_actor（`actor_present.rs`）が提示に成功するたびに入れ直す。
    choice_snapshot: HashMap<PlaceKey, Vec<ChoiceHitRow>>,
    /// `\_l` の座標解決の縮退（`CursorDegrade`＝`Unparsable`／`CenterAxisMismatch`）の actor ごと warn-once
    /// 持続状態。present_actor が [`LayoutEngine::layout_styled`] へ `&mut` で渡し、毎フレームの
    /// 重複警告を抑える（`unresolved_warned` と同型）。
    cursor_warn: CursorWarnGuard,
    /// actor → バルーンの背景色（面 0 の原点画素・sRGB 非 premultiplied）。無効表示の見た目の
    /// 色を導くためだけに使う。結線側が装着の前に [`set_balloon_background`](Self::set_balloon_background)
    /// で入れる（未設定は [`DEFAULT_BALLOON_BACKGROUND`]・白）。読み口は
    /// [`background_of`](Self::background_of)（`actor_decoration.rs`）。
    balloon_background: HashMap<ActorKey, (u8, u8, u8)>,
    /// actor → 行の中の印の絵（結線が [`set_inline_marker`](Self::set_inline_marker) で入れる・
    /// 無ければ既定の丸）。装着・再追従・箱の解決の直後に押す。
    inline_marker: HashMap<ActorKey, Arc<MarkerImage>>,
    /// 印の絵が変わった actor（`set_inline_marker` が入れ、次の箱の同期がその actor の箱を
    /// 登録し直して空にする。箱は登録の時にしか印を押さないため）。
    inline_marker_changed: BTreeSet<ActorKey>,
    /// actor → 普通のバルーンの名前（結線が入れるバルーンのフォルダ名）。折り返しの基準と無視した
    /// 書き出し位置の 2 つの警告の名前の欄に出す（未設定は `スコープ{番号}のバルーン`）。
    /// 入れる口は [`set_balloon_label`](Self::set_balloon_label)（`actor_decoration.rs`）。
    balloon_label: HashMap<ActorKey, String>,
    /// `\s` の鍵の解決の閉包（始めは無し）。無いあいだは `Emote` を読まない＝行き先は常に
    /// 普通のバルーン。本番の差し込みは箱の束の受け取り（`actor_box.rs`）が行う。
    surface_resolver: Option<SurfaceKeyResolver>,
    /// 箱の `font.name` のフォントファイルを探す場所の順（シェル → ゴースト）。
    /// 箱の束の受け取りが入れ、フォントファイルを読み込む側が読む。本番の呼び手はまだ無い継ぎ目。
    box_font_dirs: Vec<PathBuf>,
    /// 「箱を隠す印」の立っているスコープ（`actor_box.rs` の `hide_boxes`）。台詞の頭（`ClearAll`）で下ろす。
    hidden_boxes: BTreeSet<ActorKey>,
    /// 今のシェルの箱の束の置き場所の表（箱の束の受け取りが入れ、毎フレームの箱の同期が引く）。
    box_layout: BoxLayout,
    /// 箱の場所 → 登録済みの置き場所（`routing`・`layout_input` と対・`actor_box.rs` の `sync_boxes`）。
    box_sites: HashMap<PlaceKey, BoxPlacement>,
    /// 箱の場所 → 登録したときのシェルの画像の大きさ（native 原寸・`box_sites` と対）。箱の面と印の面を
    /// 「箱とシェルの画像の重なり」で切る（`actor_box.rs` の `image_limit`）。
    box_shell_size: HashMap<PlaceKey, (u32, u32)>,
    /// はみ出しを警告済みの（サーフェス番号, 箱の名前）。箱の束を差し替えると空に戻す。
    box_overflow_warned: BTreeSet<(u32, BoxName)>,
    /// 定義の 2 つの警告（折り返しの基準・無視した書き出し位置）を出した箱の名前。箱の登録には
    /// 前の配置の入力が毎回無いので、名前ごとに 1 度だけにする。箱の束を差し替えると空に戻す。
    box_definition_warned: BTreeSet<BoxName>,
    /// 箱の名前 → 白を混色の相手として解いた値（印は押す前）。箱の束の受け取りが名前ごとに
    /// 1 度だけ解いて入れ、登録は複製して印を押すだけ。解決は拡大率に依らないので、表を替えるまで古びない。
    box_resolved: BTreeMap<BoxName, ResolvedBalloonText>,
    /// いま描画範囲（validrect）が潰れている場所（幅か高さが 0 以下）。登録口
    /// （`actor_attach.rs` の `note_region_collapse`）が状態の変わり目に 1 回だけ記録するための
    /// 記憶で、登録を外しても・箱の束を差し替えても消さない＝隠す・出す・付け直しで記録が
    /// 増えない。
    collapsed_regions: BTreeSet<PlaceKey>,
    /// スコープ → 最後に提示したフレームで文字が 1 字以上見えていた箱の四角（手前から・
    /// `actor_box.rs` の `shown_boxes`）。提示のたびに作り直し、出なくなった箱（`\c`・台詞の頭・
    /// 箱を隠す印・箱の同期が登録を外した箱）はその場で外す。台本の `\s` の受け取りでは外さない
    /// （置き場所は絵の番号に従い、絵が替わった同期で外れる）。
    shown_boxes: HashMap<ActorKey, Vec<ShownBox>>,
    /// 先渡しの空回しで求めた区間の全文の持ち主（[`preview_talk`](Self::preview_talk) が入れ、
    /// 合図の適用が消去と届いた数を数える）。正本ではない（届いた字の正本は `state`）。
    lookahead: TalkLookahead,
    /// 全文の先読みを入れ直した回数（`install`・`reinstall`・`forget_boxes` を呼ぶ 3 か所で 1 つ進める・
    /// 配置の覚えの鍵）。`lookahead.rs` を変えずに入れ直しを見分けるための数。
    lookahead_rev: u64,
    /// このトークが選択待ちに入ったか（文字の線の `ChoiceWaitBegan` で真・`TalkEnded` と次のトークの
    /// 先渡しで偽）。真のときだけ押下の口 [`settle_choices_at`](Self::settle_choices_at) が場所を済ませる。
    /// トークは 1 体のゴーストで同時に 1 本なので、場所ごとでなくランタイムに 1 つ。
    choice_wait_open: bool,
    /// 場所 → 手動の送りの状態。書くのは `actor_scroll.rs`・提示（`actor_present.rs`）・
    /// 本ファイルの消去の腕・箱の登録を外す口だけ。
    scrolls: HashMap<PlaceKey, PlaceScroll>,
    /// スコープ → 普通のバルーンの矢印の絵（`set_balloon_markers`・`actor_scroll.rs`）。
    balloon_markers: HashMap<ActorKey, MarkerPair>,
    /// スコープ → 箱の名前 → 箱の矢印の絵（`set_box_markers`・名前の順はスコープごと）。
    /// 足りない番号はスコープの普通のバルーンの絵で埋める。
    box_markers: BTreeMap<ActorKey, BTreeMap<BoxName, MarkerPair>>,
    /// World の無い所（再追従）で捨てた描き手と、それを付けていた差し込み口。次の [`present_frame`] の頭で
    /// 差し込み口から外し、印の面と窓の子を消す（[`ActorRender::discard`]）。
    retired_renders: Vec<(ActorRender, Entity)>,
    /// 場所 → 作れなかった文字の面の（物理寸, 物理の位置, WUC の資源の変わった時刻, 差し込み口, 窓）。同じなら
    /// 作り直さない（`actor_present.rs`・入力で答えが決まる失敗だけ・装置の作り直しで消す）。
    surface_failed: HashMap<PlaceKey, FailedSurface>,
    /// この文字の層が最後に見た `GraphicsCore` の変化の時刻（`actor_present.rs` の
    /// `drop_renders_after_device_change` の控え・文字の層ごと）。
    seen_graphics: Option<bevy_ecs::change_detection::Tick>,
}

/// 作れなかった文字の面の覚えの中身（[`TextLayerRuntime`] の `surface_failed`）。
type FailedSurface = (
    (u32, u32),
    (f32, f32),
    Option<bevy_ecs::change_detection::Tick>,
    (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity),
);

impl TextLayerRuntime {
    /// 空のランタイムを構築する（COM 資源は初回解決フレームで World 資源から遅延構築）。
    ///
    /// 調整値はここで 1 度だけ正規化する（[`TextLayerConfig::normalized`]）——非有限・負の
    /// 行間の縮退警告は構築時の 1 件だけで、以降は正規化済みの値を配る。
    pub fn new(config: TextLayerConfig) -> TextLayerRuntime {
        TextLayerRuntime {
            state: TextLayerState::default(),
            routing: HashMap::new(),
            layout_input: HashMap::new(),
            resolve_inputs: HashMap::new(),
            surfaces: HashMap::new(),
            config: config.normalized(),
            unresolved_warned: BTreeSet::new(),
            choice_hover: HashMap::new(),
            choice_snapshot: HashMap::new(),
            cursor_warn: CursorWarnGuard::default(),
            balloon_background: HashMap::new(),
            inline_marker: HashMap::new(),
            inline_marker_changed: BTreeSet::new(),
            balloon_label: HashMap::new(),
            surface_resolver: None,
            box_font_dirs: Vec::new(),
            hidden_boxes: BTreeSet::new(),
            box_layout: BoxLayout::default(),
            box_sites: HashMap::new(),
            box_shell_size: HashMap::new(),
            box_overflow_warned: BTreeSet::new(),
            box_definition_warned: BTreeSet::new(),
            box_resolved: BTreeMap::new(),
            collapsed_regions: BTreeSet::new(),
            shown_boxes: HashMap::new(),
            lookahead: TalkLookahead::default(),
            lookahead_rev: 0,
            choice_wait_open: false,
            scrolls: HashMap::new(),
            balloon_markers: HashMap::new(),
            box_markers: BTreeMap::new(),
            retired_renders: Vec::new(),
            surface_failed: HashMap::new(),
            seen_graphics: None,
        }
    }

    /// cue を actor 別の純粋状態機械へ適用する（UI ドレインの適用点・World に触れない）。
    ///
    /// `Clear`／`ClearAll` は描画実行部の全域リセット要求点でもある（planner 初期化＋確定行
    /// TextLayout キャッシュの全破棄——破棄はこの口だけ・次フレームは `FramePlan::FullClear`）。
    /// 純粋状態（[`TextLayerState::apply_cue`]）を空にするだけでは既描画サーフェスに古いピクセルが
    /// 残るため、描画実行部にも同じ消去を伝える:
    ///
    /// - `Clear`＝対象スコープのみ（`cue.actor` の描画実行部だけをクリア）。
    /// - `ClearAll`＝全スコープ（装着済み全 actor の描画実行部をクリア）。上流は残存スコープを
    ///   列挙できないため、全消しは本ランタイムが自己完結して行う（`state.rs::apply_cue` の
    ///   全 `actor_states` 消去と対）。
    ///
    /// 戻り値は名前の形の `\b[名前]` の箱が無かったか（[`TextLayerState::apply_cue`] の答え）。
    /// 戻り値は名前の形の `\b[名前]` の箱が無かったか（[`TextLayerState::apply_cue`] の答え）。
    pub fn apply_cue(&mut self, cue: &TalkCue) -> bool {
        // 状態を進める前の行き先（`\c` が消すのは今の行き先の場所だけ）。先渡しの区間の番号も
        // 空回しと同じく、この行き先で消去を数える。
        let place = PlaceKey {
            actor: cue.actor.clone(),
            place: self.state.destination(&cue.actor),
        };
        self.lookahead.note_cue(cue, &place);
        // catch-all を置かず variant を明示し、clear 系 variant の追加時にコンパイラへ再検討を強制する。
        match &cue.command {
            CueCommand::Clear => {
                if let Some(render) = self.surfaces.get_mut(&place) {
                    render.executor.request_clear();
                }
                // hover とヒット行スナップショットを純粋状態の選択肢消去と同時に無効化する。
                // snapshot は present まで stale 行を保持しうるので、明示除去して照会時点で表示と hit を揃える。
                self.choice_hover.remove(&place);
                self.choice_snapshot.remove(&place);
                self.forget_manual_at(&place, StopReason::Cleared);
            }
            CueCommand::ClearAll => {
                for render in self.surfaces.values_mut() {
                    render.executor.request_clear();
                }
                // 全 actor の hover／ヒット行スナップショットを一括初期化する。
                self.choice_hover.clear();
                self.choice_snapshot.clear();
                // 箱を隠す印は次の台詞の頭まで。
                self.hidden_boxes.clear();
                self.forget_all_manual(StopReason::Cleared);
            }
            // 他コマンドは全域クリアを要さない。`Cursor` の warn-once は純粋層 `state.apply_cue` が担い、
            // `\s` の行き先への受け渡しは下段の `advance_state` が担う。
            CueCommand::Emote { .. }
            | CueCommand::Text(_)
            | CueCommand::Choice { .. }
            | CueCommand::Custom { .. }
            | CueCommand::NewLine { .. }
            | CueCommand::Cursor { .. }
            | CueCommand::BalloonSurface { .. }
            | CueCommand::AnchorBegin { .. }
            | CueCommand::AnchorEnd
            | CueCommand::Wait => {}
        }
        let box_missing = advance_state(&mut self.state, self.surface_resolver.as_deref(), cue);
        // 全消しの後だけ、作りたてと同じで面を持たない場所を外す（面を持つ場所は次のフレームで
        // 画面を消すので残す・空回しの写しでは呼ばない）。
        if matches!(cue.command, CueCommand::ClearAll) {
            let surfaces = &self.surfaces;
            self.state
                .forget_pristine_places(|place| surfaces.contains_key(place));
        }
        // `\c`・台詞の頭で文字が無くなった箱を、提示を待たずに写しから外す。`\s` の受け取りは
        // 行き先だけを替え、箱の置き場所は絵の番号に従う（次の箱の同期が決める）ので外さない。
        self.prune_shown_boxes();
        box_missing
    }

    /// 先渡し（再生の前に渡された、これから届く合図の全部）を受け取る: 今の状態の写しで空回しし、
    /// 区間の全文を入れ替える。受け取りを `debug!` 1 行（合図の数・求めた区間の数）で残す。
    fn preview_talk(&mut self, upcoming: &[TalkCue]) {
        let sections =
            self.lookahead
                .install(&self.state, self.surface_resolver.as_deref(), upcoming);
        self.lookahead_rev += 1;
        debug!(
            cues = upcoming.len(),
            sections, "先渡しを受け取った——空回しで区間の全文を求めた"
        );
    }

    /// 純粋状態機械（可視グリフ数・actor 状態の読み取り口）。
    pub fn state(&self) -> &TextLayerState {
        &self.state
    }

    /// actor の供給面が予約スロットへ装着済みか。
    pub fn is_attached(&self, actor: &ActorKey) -> bool {
        self.surfaces.contains_key(&PlaceKey::balloon(actor))
    }

    /// 装着済み actor の供給面（readback 等の観測口・未装着は `None`）。
    pub fn surface(&self, actor: &ActorKey) -> Option<&TextSurface> {
        self.surface_at(&PlaceKey::balloon(actor))
    }

    /// 場所（普通のバルーンか箱）の供給面（readback 等の観測口・面が無ければ `None`）。
    pub fn surface_at(&self, place: &PlaceKey) -> Option<&TextSurface> {
        self.surfaces.get(place).map(|render| &render.surface)
    }

    /// 装着済み actor の決定論観測統計（[`ViewboxExecutor::stats`]・未装着は `None`）。
    ///
    /// example／統合テストがこの口から actor 別の [`DrawStats`]（blit・`DrawTextLayout` 実行回数・
    /// 行 TextLayout 生成回数・FullClear 回数）を読み、「可視窓のみ移動フレームで確定 content の
    /// 再描画が起きない」等を決定論的に観測する。
    pub fn draw_stats(&self, actor: &ActorKey) -> Option<DrawStats> {
        self.surfaces
            .get(&PlaceKey::balloon(actor))
            .map(|render| render.executor.stats())
    }

    /// hover 状態注入。`None`＝ハイライト無し。UI スレッド専用（runtime は `!Send`）。
    ///
    /// 注入値は actor ごとに保持し、次の提示フレームの装飾が読む。`ordinal` が現存選択肢スパンに
    /// 無い場合も panic せずそのまま保持し、描画時に「ハイライト無し」として縮退する
    /// （`decorate_canvas` が hover 印を付けない）。縮退検出時は `debug!` を一件出す。
    pub fn inject_choice_hover(&mut self, actor: &ActorKey, hover: Option<usize>) {
        self.inject_choice_hover_at(&PlaceKey::balloon(actor), hover);
    }

    /// 場所を指す hover 状態注入（箱の選択肢の強調）。意味は
    /// [`inject_choice_hover`](Self::inject_choice_hover) と同じで、宛先が場所の鍵になる。
    pub fn inject_choice_hover_at(&mut self, place: &PlaceKey, hover: Option<usize>) {
        // 現存スパンに無い ordinal は縮退（ハイライト無し）。検出を debug ログに残す。
        if let Some(ordinal) = hover {
            let exists = self
                .state
                .place_state(place)
                .is_some_and(|s| s.choices().iter().any(|span| span.ordinal == ordinal));
            if !exists {
                debug!(
                    actor = %place.actor,
                    ordinal,
                    "inject_choice_hover: 現存選択肢スパンに無い ordinal——ハイライト無しとして縮退（保持のみ・panic なし）"
                );
            }
        }
        self.choice_hover.insert(place.clone(), hover);
    }

    /// 行ヒットジオメトリ照会。
    ///
    /// 鮮度契約: 最後に提示（present）したフレームの導出値＝表示と同一 layout からの単一導出
    /// （population は present_actor）。未装着・選択肢なし・スナップショット未 population は空 slice。
    pub fn choice_hit_rows(&self, actor: &ActorKey) -> &[ChoiceHitRow] {
        self.choice_hit_rows_at(&PlaceKey::balloon(actor))
    }

    /// 場所を指す行ヒットジオメトリ照会。箱の場所の矩形は**シェルの窓**の物理 px
    /// （箱の位置を足し済み）。鮮度契約は [`choice_hit_rows`](Self::choice_hit_rows) と同じ。
    pub fn choice_hit_rows_at(&self, place: &PlaceKey) -> &[ChoiceHitRow] {
        self.choice_snapshot.get(place).map_or(&[], Vec::as_slice)
    }

    /// 「済んでいない選択肢があるか」照会（照会のみ＝バリア解決はしない）。
    ///
    /// 表示層自身の範囲の記録（[`ActorTextState::choices`](crate::state::ActorTextState::choices)）に、
    /// 種類が選択肢で済みの境目より後のもの（[`ActorTextState::has_live_choice`](crate::state::ActorTextState::has_live_choice)）
    /// があることを表す（供給側 `CuePlayerState::WaitingForChoice` バリアの真実源とは別）。
    /// スコープのどの場所（普通のバルーン・箱）に済んでいない選択肢があっても真（kanade の
    /// 選択待ちの単位がスコープだから）。アンカーの範囲は数えない。バルーンの時間切れの観測は
    /// これを読まない（選択肢つきのトークはトークの終わりの時刻から測る）。未知 actor・選択肢なしは `false`。
    pub fn choice_active(&self, actor: &ActorKey) -> bool {
        self.state
            .places()
            .any(|(key, s)| key.actor == *actor && s.has_live_choice())
    }

    /// 押下の口: 選択待ちに入った後なら、場所の選択肢を済みにし、当たり行の写しと強調から
    /// 済んだ選択肢をその場で外す（提示を待たない・`Clear` と同じ原子性）。
    /// 選択待ちに入る前は何もしない（kanade が捨てる押下なので、押し直せるまま残す）。
    pub fn settle_choices_at(&mut self, place: &PlaceKey) {
        if !self.choice_wait_open {
            debug!(actor = %place.actor, "選択待ちの前の押下——選択肢を済みにしない（押し直せるまま）");
            return;
        }
        if self.state.settle_choices_at(place) {
            self.drop_settled_choices(place);
        }
    }

    /// 文字の線の `ChoiceWaitBegan`: このトークが選択待ちに入ったと覚える。
    fn note_choice_wait_began(&mut self) {
        self.choice_wait_open = true;
    }

    /// 選択待ちに入ったかの欄を戻す（`TalkEnded` と次のトークの先渡し）。
    fn close_choice_wait(&mut self) {
        self.choice_wait_open = false;
    }

    /// 文字の線のトークの終わり: 全部の場所の選択肢を済みにし、押下の口と同じく写しと強調を揃える。
    fn settle_all_choices(&mut self) {
        self.state.settle_all_choices();
        let places: Vec<PlaceKey> = self.state.places().map(|(key, _)| key.clone()).collect();
        for place in &places {
            self.drop_settled_choices(place);
        }
    }

    /// 場所の当たり行の写しから済んだ選択肢の行を外し（アンカーの行は残る）、済んだ選択肢を指す
    /// 強調を `None` にする。
    fn drop_settled_choices(&mut self, place: &PlaceKey) {
        let Some(state) = self.state.place_state(place) else {
            return;
        };
        if let Some(rows) = self.choice_snapshot.get_mut(place) {
            rows.retain(|row| !is_settled_ordinal(state, row.ordinal));
        }
        if let Some(hover) = self.choice_hover.get_mut(place)
            && hover.is_some_and(|ordinal| is_settled_ordinal(state, ordinal))
        {
            *hover = None;
        }
    }

    /// 「押せる範囲があるか」の照会（選択肢かアンカー・種類を問わない）。
    ///
    /// スコープの**どの場所か**（普通のバルーン・箱）に範囲の記録があれば真。ホバーと押下の
    /// 前段が見る。バルーンの時間切れの待ちを止める条件には使わない（済んでいない選択肢の
    /// 有無は [`choice_active`](Self::choice_active)）。未知 actor・範囲なしは `false`。
    pub fn hit_active(&self, actor: &ActorKey) -> bool {
        self.state
            .places()
            .any(|(key, s)| key.actor == *actor && !s.choices().is_empty())
    }

    /// `\s` の解決の閉包だけを差し込む（テスト専用の口・本番は箱の束の受け取り
    /// `set_box_layout` が表と一緒にこの欄へ入れる）。
    #[cfg(test)]
    pub(super) fn set_surface_resolver(&mut self, resolve: SurfaceKeyResolver) {
        self.surface_resolver = Some(resolve);
        self.rehearse_again();
    }

    /// 空回しをやり直す: 空回しの出発点（状態か `\s` の解決の閉包）を合図と
    /// 無関係に変えた口の直後に呼ぶ。今の状態と解決の閉包で `reinstall` を呼ぶだけで、先渡しを
    /// 受け取っていなければ何も起きない。
    fn rehearse_again(&mut self) {
        self.lookahead
            .reinstall(&self.state, self.surface_resolver.as_deref());
        self.lookahead_rev += 1;
    }

    /// その場所の描き手が配置から装飾までを作り直した回数（テストだけの読み口）。
    /// 描き手が無ければ `None`。
    #[cfg(test)]
    pub(super) fn frame_rebuilds(&self, place: &PlaceKey) -> Option<u32> {
        self.surfaces.get(place).map(|render| render.frame_rebuilds)
    }

    /// 検査用の読み口: その場所の配置の入力を引いて [`present::arrange_lines`] を呼ぶだけ
    /// （`present_actor` が冒頭でしている引き当てと同じ）。配置の入力か状態が無ければ `None`。
    /// 字幅は呼び手が渡す（決まった字幅で GPU の資源なしに本番と同じ手順の行の列を取る）。
    #[cfg(test)]
    pub(super) fn arrange_for_test(
        &mut self,
        place: &PlaceKey,
        metrics: &dyn crate::layout::GlyphMetrics,
        talk_time: f64,
    ) -> Option<Vec<crate::layout::PositionedLine>> {
        let resolved = self.layout_input.get(place)?;
        present::arrange_lines(
            &self.state,
            &mut self.lookahead,
            &mut self.cursor_warn,
            place,
            resolved,
            metrics,
            &mut present::BreakMemo::default(),
            talk_time,
        )
    }
}

/// 結線 API（UI スレッド＝pump スレッドから呼ぶ）:
/// `spawn_ui` で UI ドレインを起動し、受信口 [`EmoTextSink`] と drain の join ハンドルを返す。
///
/// handler は `runtime` の `Rc` clone を捕捉し（`!Send` handler・基盤許容）、
/// [`handle_text_msg_with`]（終了規律の正準写像）へ委譲して cue を純粋状態へ適用する。
/// 先渡し（`TextMsg::Upcoming`）は `preview_talk` へ渡す。選択待ちの始まり
/// （`TextMsg::ChoiceWaitBegan`）は押下の口を開き、トークの終わり（`TextMsg::TalkEnded`）は
/// 全部の場所の選択肢を済みにして口を閉じる（先渡しも口を閉じる）。
/// 終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`・全 `UiSender`
/// （＝全 [`EmoTextSink`] クローン）drop＝drain 正常終了（error ログなし）。
/// 個別 cue の適用失敗（runtime 借用競合など）は `Err` 戻し→基盤が `error!`＋継続する。
///
/// # 前提
///
/// UI（pump）スレッドから呼ぶこと。誤用は呼出時検出不能（`spawn_ui` の Risks 参照・
/// spawn 時 debug! 診断で緩和）。
pub fn spawn_emo_text(
    runtime: Rc<RefCell<TextLayerRuntime>>,
) -> (EmoTextSink, wintf_winmsg_executor::JoinHandle<()>) {
    let (tx, handle) = areka_actor::spawn_ui("emo-text", move |msg: TextMsg| {
        handle_text_msg_with(
            msg,
            |cue, probe| match runtime.try_borrow_mut() {
                Ok(mut rt) => {
                    let box_missing = rt.apply_cue(&cue);
                    drop(rt);
                    // 名前の形の `\b` の箱が無い＝strict の台本へ 1 回知らせる。
                    if let (true, Some(probe)) = (box_missing, probe) {
                        probe.report(DetectionPoint::BalloonBoxMissing.finding());
                    }
                    Ok(())
                }
                // 借用競合（UI スレッド上の別処理が runtime を保持中）——panic せず Err 戻しで
                // 基盤の error!＋継続に乗せる（当該 cue は失われる）。
                Err(_) => Err(format!(
                    "TextLayerRuntime が借用中のため cue を適用できない（actor={}, at={}）——当該 cue は失われる",
                    cue.actor, cue.at
                )),
            },
            |upcoming| match runtime.try_borrow_mut() {
                Ok(mut rt) => {
                    // 次のトークの先渡し＝前のトークの選択待ちは終わっている。
                    rt.close_choice_wait();
                    rt.preview_talk(&upcoming);
                    Ok(())
                }
                // 借用競合——cue と同じく Err 戻しで基盤の error!＋継続に乗せる（そのトークは先渡し無し・
                // 文節の折り返しでは warn が出る）。
                Err(_) => Err(format!(
                    "TextLayerRuntime が借用中のため先渡しを受け取れない（cues={}）——そのトークは先渡し無し",
                    upcoming.len()
                )),
            },
            |mark| match runtime.try_borrow_mut() {
                Ok(mut rt) => {
                    if mark == TextMsg::ChoiceWaitBegan {
                        rt.note_choice_wait_began();
                    } else {
                        // トークの終わり: 全部の場所の選択肢を済みにし、押下の口を閉じる。
                        rt.settle_all_choices();
                        rt.close_choice_wait();
                    }
                    Ok(())
                }
                // 借用競合——cue と同じく Err 戻しで基盤の error!＋継続に乗せる（終わりなら、その
                // トークの選択肢は次の台詞の頭で消える）。
                Err(_) => Err(format!(
                    "TextLayerRuntime が借用中のためトークの印を適用できない（{mark:?}）"
                )),
            },
        )
    });
    (EmoTextSink::new(tx), handle)
}

#[path = "actor_attach.rs"]
mod attach;

#[path = "actor_box.rs"]
mod boxes;

#[path = "actor_present.rs"]
mod present;

#[path = "actor_scroll.rs"]
mod scroll;

#[path = "marker_surface.rs"]
mod marker;
pub(crate) use marker::marker_bitmap;
pub use marker::{MarkerSurface, PlacedMarker};

/// その序数の範囲が済んだ選択肢か（範囲が無い序数は済んでいない扱い）。押下の口・トークの終わりの口と
/// 提示（`actor_present.rs`）が同じ判定で写しと強調から外す。
fn is_settled_ordinal(state: &crate::state::ActorTextState, ordinal: usize) -> bool {
    state
        .choices()
        .iter()
        .any(|span| span.ordinal == ordinal && state.is_settled_choice(span))
}

pub use boxes::ShownBox;
#[cfg(test)]
pub(crate) use present::choice_ranges;
pub use present::present_frame;

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "actor_test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "actor_runtime_frame_tests.rs"]
mod runtime_frame_tests;

#[cfg(test)]
#[path = "actor_choice_contract_tests.rs"]
mod choice_contract_tests;

#[cfg(test)]
#[path = "actor_clear_atomicity_tests.rs"]
mod clear_atomicity_tests;

#[cfg(test)]
#[path = "actor_choice_settle_tests.rs"]
mod choice_settle_tests;

#[cfg(test)]
#[path = "actor_scale_refresh_tests.rs"]
mod scale_refresh_tests;

#[cfg(test)]
#[path = "actor_region_warn_tests.rs"]
mod region_warn_tests;

#[cfg(test)]
#[path = "actor_writing_warn_tests.rs"]
mod writing_warn_tests;

#[cfg(test)]
#[path = "actor_region_collapse_tests.rs"]
mod region_collapse_tests;

#[cfg(test)]
#[path = "actor_region_collapse_present_tests.rs"]
mod region_collapse_present_tests;

#[cfg(test)]
#[path = "actor_underline_present_tests.rs"]
mod underline_present_tests;

#[cfg(test)]
#[path = "actor_scroll_retain_tests.rs"]
mod scroll_retain_tests;

#[cfg(test)]
#[path = "actor_route_tests.rs"]
mod route_tests;

#[cfg(test)]
#[path = "actor_lookahead_tests.rs"]
mod lookahead_tests;

#[cfg(test)]
#[path = "actor_lookahead_shapes_tests.rs"]
mod lookahead_shapes_tests;

#[cfg(test)]
#[path = "actor_box_tests.rs"]
mod box_tests;

#[cfg(test)]
#[path = "actor_box_sync_tests.rs"]
mod box_sync_tests;

#[cfg(test)]
#[path = "actor_box_present_tests.rs"]
mod box_present_tests;

#[cfg(test)]
#[path = "actor_scroll_tests.rs"]
mod scroll_tests;

#[cfg(test)]
#[path = "actor_scroll_present_tests.rs"]
mod scroll_present_tests;

#[cfg(test)]
#[path = "actor_surface_limit_tests.rs"]
mod surface_limit_tests;

#[cfg(test)]
#[path = "actor_frame_memo_tests.rs"]
mod frame_memo_tests;

/// バルーン背景色の受け口。
#[path = "actor_decoration.rs"]
mod decoration;

#[cfg(test)]
#[path = "actor_decoration_tests.rs"]
mod decoration_tests;

#[cfg(test)]
#[path = "actor_decoration_frame_tests.rs"]
mod decoration_frame_tests;

#[cfg(test)]
#[path = "actor_inline_marker_tests.rs"]
mod inline_marker_tests;

#[cfg(test)]
#[path = "sink_probe_tests.rs"]
mod sink_probe_tests;
