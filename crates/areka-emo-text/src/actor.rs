//! # actor — UI ドレインとフレーム提示ステップ（結線層）
//!
//! `spawn_emo_text`（`spawn_ui` 結線・UI ドレイン起動）・`TextLayerRuntime`
//! （UI スレッド所有の集約ルート）・`TextSlotBinding`・`present_frame`
//! （毎フレームの注入時刻駆動：リビール進行→レイアウト→描画→装着）を担う。
//!
//! **層規律**: 結線層。終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`、
//! 全 `UiSender` drop＝drain 正常終了（いずれも error ログなし）。個別メッセージの処理失敗は
//! `Err` 戻し→基盤が `error!`＋継続（log-first・ループを殺さない）。

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use areka_actor::UiSpawnError;
use areka_emo_present::TextSlotView;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use tracing::{debug, error, info, warn};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use crate::TextLayerError;
use crate::canvas::ContentCanvas;
use crate::choice::{
    ResolvedChoiceStyle, annotate_lines, decorate_canvas, derive_hit_rows, highlight_band_extent,
    to_window_physical,
};
use crate::draw::{DWriteMetrics, ResolvedFont};
use crate::layout::{CursorWarnGuard, GlyphMetrics, LayoutEngine, WrapPlan};
use crate::region::{ImagePx, ScaleContract, TextRegion};
use crate::segment::segment_plan;
use crate::sink::{EmoTextSink, TextMsg, handle_text_msg};
use crate::state::{TextLayerConfig, TextLayerState};
use crate::surface::TextSurface;
use crate::viewbox_draw::{DrawStats, ViewboxExecutor};
use crate::wrap::WrapMode;
use crate::writing::WritingMode;

/// actor の装着先（結線側が emo-present `TextSlotView` から構築して routing へ登録する）。
///
/// [`crate::surface::TextSurface::attach`] の入力。emo-present は actor を知らない
/// （層純度維持・R9.5）——`ActorKey → TargetId` の対応は結線側（example/emo2-boot）が所有し、
/// `text_slot_view(target)` で得た view の値から本型を [`Self::new`] で組む。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSlotBinding {
    /// 予約スロット（`emo-text-layer-slot` Visual entity・emo-present `VisualMount` が予約）。
    pub slot: Entity,
    /// 装着先の窓 entity。
    pub window: Entity,
    /// 合成スケール k（`TextSlotView.scale` 由来・**窓 DPI 由来で 1.0 とは限らない**）。
    /// 不正値は構築時に [`ScaleContract`] の縮退規約（warn!＋1.0）で正規化済み。
    pub scale: f32,
    /// バルーン surface の物理原寸（TextSurface/swapchain の物理化に使用）。
    pub surface_size: (u32, u32),
    /// 画像座標空間の原寸（負値=反対辺解決・`TextRegion::resolve` の入力）。
    ///
    /// **作者画像空間の原寸そのもの＝k 不変**。emo-present の native 原寸
    /// （`TextSlotView::surface_size`＝k 適用**前**）を**そのまま透過**する。
    /// `TextRegion::resolve` へ物理 px を渡すのはレビューエラー
    /// （2 空間モデルの綻び目をここで構造閉塞——design.md「DPI/スケール契約」）。
    pub image_size: (u32, u32),
}

impl TextSlotBinding {
    /// `TextSlotView` の読み値（slot/window/scale/物理原寸/native 原寸）から binding を構築する。
    ///
    /// k の正規化（0 以下・非有限→warn!＋1.0 縮退）は [`ScaleContract::new`] に委譲する。
    ///
    /// # image px 原寸は**導出しない**（2026-07-30 是正・k<1 の 1px 往復欠陥）
    ///
    /// 旧実装は `image_size = round(physical_size / k)` と**逆写像で復元**していたが、これは
    /// k<1 で厳密な逆写像にならない。順写像は丸め権威 `ScaleRatio::scale_len`（round half away
    /// from zero）で誤差が ±0.5 **物理**px に収まるが、逆写像は k で割るためその誤差が
    /// ±0.5/k **画像**px へ増幅される——k>1 なら 0.5 未満で `round` が厳密に復元するのに対し、
    /// **k<1 では 0.5 を超えて 1px ずれる**（例: k=4/5・native 143 → physical 114 →
    /// `round(114/0.8)` = 142）。k<1 は本番到達可能である（`parse_author_dpi` は任意の宣言値を
    /// 素通しするため、`dpi,120` を宣言したゴーストを 100% モニタで表示すれば k=4/5 になる）。
    ///
    /// 復元しようとしていた値は presenter が**既に正確に持っている**（`TextSlotView::surface_size`
    /// ＝native）。ゆえに逆写像そのものを廃し、native を第 5 引数で受け取って透過する——
    /// 往復が構造ごと消滅し、k に依らず厳密になる。k≥1 では旧実装とバイト同一。
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

    /// emo-present の読み取り専用増分 `TextSlotView` からの一点変換（結線の正準口・R9.1/R9.2）。
    ///
    /// # 2 つの寸を**両方**読む（R8.2）
    ///
    /// [`TextSlotView`] は **native 原寸**（`surface_size()`＝k 適用**前**）と**物理寸**
    /// （`physical_size()`＝丸め権威 `scaled_extent` を通した表示寸）を隣り合わせで公開する。
    /// 本メソッドは前者を `image_size` へ、後者を `surface_size` へ写す:
    ///
    /// - `image_size` ← `surface_size()`（native）: 作者画像空間の原寸は**k 不変**でなければ
    ///   ならない（R8.2 の供給面 `ceil(validrect 寸 × k)` が k に比例する前提）。
    /// - `surface_size` ← `physical_size()`（k 適用後）: 診断・churn 判定用の表示寸。
    ///
    /// 取り違えるとどちらの向きでも静かに壊れる——`image_size` に物理寸を入れれば画像空間が
    /// k 倍に膨らみ、`surface_size` に native を入れれば表示寸の記録が k に追随しなくなる
    /// （`presenter.rs` の `physical_size` doc が警告する「消費点での 1 トークンの取り違え」）。
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
/// （design.md `TextLayerRuntime.layout_input` の値型）。
///
/// 解決はすべて既存の一点解決口（[`WritingMode::resolve`]／[`TextRegion::resolve`]／
/// [`ResolvedFont::resolve`]／[`WrapMode::resolve`]）の合成であり、本型は束ねるだけで
/// 独自解釈を持たない。
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
    /// 装飾（`decorate_canvas`）が hover 行へ焼く塗り/文字色の源（design.md RuntimeContract・R4.2/4.3）。
    pub choice_style: ResolvedChoiceStyle,
}

impl ResolvedBalloonText {
    /// balloon model（2 層マージ済み）とバルーン画像原寸（**image px**・
    /// [`TextSlotBinding::image_size`] の一点導出値）から解決する。
    /// 物理 px を渡すのはレビューエラー（2 空間モデル——design.md「DPI/スケール契約」）。
    pub fn resolve(model: &BalloonModel, image_size: (u32, u32)) -> ResolvedBalloonText {
        let mode = WritingMode::resolve(model);
        let font = ResolvedFont::resolve(model);
        // hover ハイライトスタイルはバルーン cursor.* モデル＋解決済み既定文字色から一度だけ解決する
        // （下流 present_actor の装飾は本値を読むだけ・choice.rs へは依存しない・design.md Integration）。
        let choice_style = ResolvedChoiceStyle::resolve(Some(model.cursor()), font.color);
        ResolvedBalloonText {
            mode,
            region: TextRegion::resolve(model, image_size, mode),
            font,
            wrap: WrapMode::resolve(model),
            choice_style,
        }
    }
}

/// choice.rs（純粋層）所有のバルーン窓物理 px 矩形を結線層から再輸出する（design.md RuntimeContract）。
/// 下流（choice-interact）は [`ChoiceHitRow::rect`] を本型で受ける——照会契約の座標系正本。
pub use crate::choice::HitRectPx;

/// 行ヒットジオメトリ契約（本 spec 正本・choice-interact が消費・design.md RuntimeContract）。
///
/// 提示フレーム同期スナップショットの 1 行分——1 選択肢セグメントの窓物理 px 矩形に、
/// 下流 `ChoiceSelection` 構成材料（`ordinal`/`id`/`label`/`references`）を同梱し、契約の
/// 再照会を不要にする（design.md「下流契約」）。スナップショットの population は present_actor
/// （task 8.2）が担い、本 task（8.1）では per-actor スナップショットは空のまま。
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceHitRow {
    /// スパンの配送順序数（[`crate::state::ChoiceSpan::ordinal`]・hover 注入／選択解決の主キー）。
    pub ordinal: usize,
    /// `\q` ID（不透明転写）。
    pub id: String,
    /// 表示文字列（不透明転写）。
    pub label: String,
    /// `\q` 第 3 引数以降（参照列・不透明転写）。
    pub references: Vec<String>,
    /// ヒット矩形（バルーン窓物理 px・スクロール committed 反映済み・[`HitRectPx`]）。
    pub rect: HitRectPx,
}

/// actor 1 人分の描画資源（供給面＋描画実行部＋実測 metrics——font/mode に束縛されるため
/// actor 別に持つ。行 TextLayout キャッシュの index 衝突・format 組み直しの構造回避）。
struct ActorRender {
    /// 自前 swapchain 供給面（初回のみ予約スロットへ brush 装着・以降 Present のみ）。
    surface: TextSurface,
    /// viewbox ダーティ矩形スクロールの実行部（保持ピクセルの面内 blit ＋ ダーティ矩形限定描画・
    /// 行 TextLayout キャッシュは actor の行 index に束縛）。
    executor: ViewboxExecutor,
    /// 計測専用 probe 由来の実測 metrics（actor の font/mode に束縛）。
    metrics: DWriteMetrics,
}

/// UI スレッド所有の集約ルート（NonSend・design.md「TextLayerRuntime」正本）。
///
/// 純粋状態（[`TextLayerState`]）・binding（結線側が登録）・COM 資源（actor 別の
/// 供給面/描画実行部——World 資源から遅延構築）を束ねる。`Rc<RefCell<_>>` で
/// [`spawn_emo_text`] の UI ドレインとフレーム提示ステップ（[`present_frame`]）が
/// 同一 UI スレッド上で共有する（`!Send`——スレッドを跨がない）。
///
/// - cue 適用（[`apply_cue`](Self::apply_cue)）は純粋状態の更新のみで World に触れない。
/// - World への装着・描画はフレーム提示ステップ（[`present_frame`]）が担う。
/// - 時刻は常に注入（`talk_time`）——内部で `Instant::now()` を読まない（R3.3）。
pub struct TextLayerRuntime {
    /// 純粋状態機械（actor 別・cue 列→行/グリフ状態）。
    state: TextLayerState,
    /// actor → 装着先（予約スロット）。結線側が [`register_actor`](Self::register_actor) で登録。
    routing: HashMap<ActorKey, TextSlotBinding>,
    /// actor → layout 入力（writing_mode/region/font の解決済み束）。routing と対で登録。
    layout_input: HashMap<ActorKey, ResolvedBalloonText>,
    /// actor 別の描画資源（遅延生成——`ActorRender` 不在の解決フレームで World 資源から構築・装着。
    /// k 再追従（[`TextLayerRuntime::refresh_actor_scale`]）は本 map の当該エントリだけを破棄し、
    /// 次フレームの再生成へ委ねる——純粋状態 `state` には触れない・R8.2/R8.3）。
    surfaces: HashMap<ActorKey, ActorRender>,
    /// 調整値（line_pitch 係数）。reveal ペースは配送 duration 由来ゆえ char_wait は持たない
    /// （本 config は `DWriteMetrics::new` の行送り算出にのみ使う）。
    config: TextLayerConfig,
    /// 未解決 actor の warn を actor ごと初回のみに抑える記録（以降は debug!——
    /// design Error Handling「未 binding actor の cue」）。
    unresolved_warned: BTreeSet<ActorKey>,
    /// actor → hover 注入状態（`None`＝ハイライト無し・[`inject_choice_hover`](Self::inject_choice_hover)
    /// で更新・present_actor の装飾（task 8.2）が読む・UI スレッド専用）。
    choice_hover: HashMap<ActorKey, Option<usize>>,
    /// actor → 提示フレーム同期ヒット行スナップショット（[`choice_hit_rows`](Self::choice_hit_rows)
    /// の照会源）。population は present_actor（task 8.2）が present 成功時に行う——本 task では空のまま。
    choice_snapshot: HashMap<ActorKey, Vec<ChoiceHitRow>>,
    /// `\_l` カーソル換算縮退（6.5）の actor ごと warn-once 檻。present_actor が
    /// [`LayoutEngine::layout_with_cursor_warn`] へ `&mut` で渡す持続 guard——per-frame layout 呼出での
    /// 重複警告を走査を跨いで抑止する（`unresolved_warned` と同型・行出力へは影響しない）。
    cursor_warn: CursorWarnGuard,
}

impl TextLayerRuntime {
    /// 空のランタイムを構築する（COM 資源は初回解決フレームで World 資源から遅延構築）。
    pub fn new(config: TextLayerConfig) -> TextLayerRuntime {
        TextLayerRuntime {
            state: TextLayerState::default(),
            routing: HashMap::new(),
            layout_input: HashMap::new(),
            surfaces: HashMap::new(),
            config,
            unresolved_warned: BTreeSet::new(),
            choice_hover: HashMap::new(),
            choice_snapshot: HashMap::new(),
            cursor_warn: CursorWarnGuard::default(),
        }
    }

    /// actor と装着先（予約スロット）＋layout 入力の対応を登録する（結線側の口——
    /// `ActorKey → TargetId` の対応は結線側が所有し、emo-present `TextSlotView` の読み値
    /// から [`TextSlotBinding::new`]／[`ResolvedBalloonText::resolve`] で組んで渡す・R9.5）。
    ///
    /// 未解決のまま蓄積していた actor は次の [`present_frame`] で再試行され装着される。
    pub fn register_actor(
        &mut self,
        actor: ActorKey,
        binding: TextSlotBinding,
        resolved: ResolvedBalloonText,
    ) {
        debug!(actor = %actor, slot = ?binding.slot, "actor の装着先（予約スロット）を登録した");
        self.routing.insert(actor.clone(), binding);
        self.layout_input.insert(actor, resolved);
    }

    /// 統合配線の一点口（R9.5・task 8）: `ActorKey → TargetId` 対応の解決結果
    /// （`EmoPresenter::text_slot_view(target)` の view——対応関係の所有は結線側・
    /// example/emo2-boot）と 2 層マージ済み balloon model から、binding
    /// （[`TextSlotBinding::from_view`]）と layout 入力（[`ResolvedBalloonText::resolve`]・
    /// 入力は必ず binding の `image_size`＝**image px**）を導出して
    /// [`register_actor`](Self::register_actor) へ登録する。
    ///
    /// 物理 px を領域解決へ渡す誤配線をこの口で構造閉塞する（2 空間モデル——design.md
    /// 「DPI/スケール契約」）。行レイアウト（`PositionedLine`）・クリック可能範囲の
    /// choice-render 再利用シーム（R9.4）には手を触れない——本口は入力の供給のみで、
    /// レイアウト出力の形を変えない。
    ///
    /// `text_slot_view` は初回 `ShowSurface` 前は `None` を返すため、呼び手は表示確立後に
    /// 本口で登録する（未登録の間に届いた cue は蓄積され、登録後の次フレームで装着・描画される）。
    pub fn register_actor_view(
        &mut self,
        actor: ActorKey,
        view: &TextSlotView,
        model: &BalloonModel,
    ) {
        self.register_actor_binding(actor, TextSlotBinding::from_view(view), model);
    }

    /// **単一構築経路の内側**（binding 直渡し・装着側）: view からの読み取りは呼び手側の
    /// [`TextSlotBinding::from_view`] 一点で完結し、以降の導出——`binding.image_size` を入力と
    /// する [`ResolvedBalloonText::resolve`] と [`register_actor`](Self::register_actor) への
    /// 登録——は装着でも再追従でも完全に同一である（第 2 の構築流儀を作らない・R4.3/R8.1）。
    ///
    /// 再追従側（[`refresh_actor_binding`](Self::refresh_actor_binding)）は判定キーに解決済み
    /// 領域を含む都合で `resolve` を**判定前に自分で 1 回だけ**呼び、その値のまま
    /// `register_actor` へ合流する（二重 resolve の回避——導出そのものは本メソッドと同一）。
    fn register_actor_binding(
        &mut self,
        actor: ActorKey,
        binding: TextSlotBinding,
        model: &BalloonModel,
    ) {
        let resolved = ResolvedBalloonText::resolve(model, binding.image_size);
        self.register_actor(actor, binding, resolved);
    }

    /// balloon target の**再追従シーム**（適用 k・面実寸・文字描画領域の変化に追従する・
    /// R4.3〜R4.7／R8.1/8.2/8.3/8.5/8.7・design D11/D3）。
    ///
    /// 窓 DPI 変化（モニタ跨ぎ移動・表示スケール変更）で emo-present の適用 k が変わったとき、
    /// あるいは同じ k のままバルーン面の実寸や当該 scope の `validrect` が変わったとき、
    /// 結線側（`emo2_boot` の DPI フェーズ）が新しい [`TextSlotView`] を携えて呼ぶ。view から
    /// binding を再構築し（[`register_actor_view`](Self::register_actor_view) と同一の導出）、
    /// 当該 actor の `ActorRender`（供給面・描画実行部・実測 metrics）を破棄する。次の
    /// [`present_frame`] の初回解決分岐が**新 k の物理寸**（`ceil(validrect 寸 × k)`／
    /// `validrect 原点 × k`）で再生成する——既存の生成式をそのまま再利用し、旧寸供給面は
    /// 再利用しない（R8.2）。
    ///
    /// # リビール状態は保存される（R8.3・`Clear`/`ClearAll` と別物）
    ///
    /// 破棄するのは描画資源だけで、純粋状態（[`TextLayerState`]——typewriter 進行・確定行）には
    /// 触れない（[`register_actor`](Self::register_actor) が `routing`＋`layout_input` しか
    /// 上書きしない既存構造がこれを担保する）。確定行 TextLayout キャッシュは `ActorRender` に
    /// 宿るため、破棄→再生成で次フレームは保存済み状態から**全再描画**される（R8.4）。
    ///
    /// # 戻り値（churn ガード・R4.5/R8.5）
    ///
    /// 再追従を行ったとき `true`、行わなかったとき `false`。判定キー——binding 全体
    /// （k・物理寸・image 原寸・slot・window）と `model`×`image_size` から解き直した
    /// [`ResolvedBalloonText`]——が**すべて同値**（identity 再導出を含む）なら
    /// **no-op で `false`**：毎フレーム再結線・再生成を構造的に禁じる。逆に k が同値でも
    /// 面実寸や文字描画領域が違えば再構築する（R4.4——k の同値のみを根拠に省略しない）。
    /// 未登録 actor（まだ装着されていない）も `false`——装着は `register_actor_view` の
    /// 領分であり、本口が第 2 の装着経路にならない（R4.6）。
    pub fn refresh_actor_scale(
        &mut self,
        actor: &ActorKey,
        view: &TextSlotView,
        model: &BalloonModel,
    ) -> bool {
        self.refresh_actor_binding(actor, TextSlotBinding::from_view(view), model)
    }

    /// [`refresh_actor_scale`](Self::refresh_actor_scale) の内側（binding 直渡し・判断分岐の本体）。
    /// `TextSlotView` は emo-present 私有フィールド型ゆえ in-crate 檻から構築できないため、
    /// 判断分岐をこの層で檻に入れられるよう分けてある（公開口との差は view 読み取りの有無のみ）。
    fn refresh_actor_binding(
        &mut self,
        actor: &ActorKey,
        binding: TextSlotBinding,
        model: &BalloonModel,
    ) -> bool {
        let Some(&current) = self.routing.get(actor) else {
            // 未装着 actor（`text_slot_view` が None のまま等）——再構築すべき binding が無い。
            // 失敗ではなく「対象なし」の静穏 skip（装着は register_actor_view の領分・R4.6）。
            debug!(
                actor = %actor,
                k = binding.scale,
                "文字層の再追従: 未登録 actor のため何もしない（装着は register_actor_view の領分）"
            );
            return false;
        };
        let (k_old, k_new) = (current.scale, binding.scale);
        // 判定キー＝binding 全体（k・物理寸・image 原寸・slot・window）と、その image 原寸で
        // model から解き直した文字描画領域の**連言**（D3・R4.4）。k の同値のみを根拠に再追従を
        // 省略すると、同 k のまま面実寸や scope 別 `validrect` が変わったときに旧寸の文字層が
        // 残る。k は双方とも ScaleContract 正規化済み（TextSlotBinding::new 経由）の同一表現
        // ゆえ、（derive した `PartialEq` 経由の）厳密一致で「変化なし」を判定してよい
        // （f32 は出口ビュー——ここでは比較にのみ使い、寸法演算には一切用いない・D4）。
        //
        // 再解決は純関数ゆえ判定前にここで **1 回だけ**行い、再構築側でもこの値をそのまま使う
        // （二重 resolve・第 2 の構築流儀を作らない・R4.3）。
        let resolved = ResolvedBalloonText::resolve(model, binding.image_size);
        if current == binding && self.layout_input.get(actor) == Some(&resolved) {
            debug!(
                actor = %actor,
                k = k_new,
                "文字層の再追従: 適用 k・面実寸・文字描画領域がいずれも同値のため再結線・再生成を行わない（churn ガード・R4.5）"
            );
            return false;
        }

        // 装着と同一の導出（`ResolvedBalloonText::resolve` → `register_actor`）で binding／
        // layout 入力を再構築する（単一構築経路・R4.3）。純粋状態（TextLayerState）には
        // 触れない＝リビール進行・確定行は保存される（R4.7）。
        self.register_actor(actor.clone(), binding, resolved);
        // 描画資源だけを破棄する。次 present_frame の初回解決分岐が新しい k・面実寸の物理寸で
        // 再生成し、空の行 TextLayout キャッシュから保存済み状態を全再描画する（R4.3/R8.4）。
        self.surfaces.remove(actor);
        info!(
            actor = %actor,
            k_old,
            k_new,
            image_size = ?binding.image_size,
            surface_size = ?binding.surface_size,
            "文字層の再追従: binding と文字描画領域を再構築し描画資源を破棄した（次フレームで新しい物理寸へ再生成・リビール状態は保存）"
        );
        true
    }

    /// cue を actor 別の純粋状態機械へ適用する（UI ドレインの適用点・World に触れない）。
    ///
    /// `Clear`／`ClearAll` は描画実行部の全域リセット要求点でもある（planner 初期化＋確定行
    /// TextLayout キャッシュの全破棄——design「Clear で全破棄」・破棄はこの口だけ・次フレームは
    /// `FramePlan::FullClear`＝全域透明・R4.3）。純粋状態（[`TextLayerState::apply_cue`]）を
    /// 空にするだけでは既描画サーフェスに古いピクセルが残留するため（#6 欠陥）、提示層の
    /// 描画実行部にも同じ消去を伝える必要がある:
    ///
    /// - `Clear`＝**対象スコープのみ**（`cue.actor` の描画実行部だけをクリア・R6.4/R7.4）。
    /// - `ClearAll`＝**全スコープ**（装着済み全 actor の描画実行部をクリア・#6 の冒頭全消し・
    ///   R6.4/R7.4）。上流は残存スコープを列挙できないため、全消しは本ランタイムが自己完結して
    ///   行う（`state.rs::apply_cue` の全 `actor_states` 消去と対）。
    pub fn apply_cue(&mut self, cue: &TalkCue) {
        // catch-all を置かず variant を明示し、将来 dola が clear 系 variant を追加した際に
        // コンパイラへ描画実行部側の再検討を強制する（no-catch-all 規律）。
        match &cue.command {
            CueCommand::Clear => {
                if let Some(render) = self.surfaces.get_mut(&cue.actor) {
                    render.executor.request_clear();
                }
                // 選択肢ライフサイクルの原子的無効化（R5.1/5.2/5.4）: 当該 actor の hover を None へ
                // リセットし、ヒット行スナップショットを純粋状態の選択肢消去と**同時**に無効化する
                // （表示と hit の片方だけが古い状態に残らない——present を待たず `choice_hit_rows` が空・
                // `choice_active` が false へ揃う）。スパン初期化は下段 `state.apply_cue(Clear)` が
                // items と同一ライフサイクルで担う。snapshot を明示除去するのは、`choice_active` が
                // span 由来で即時に false へ倒れる一方、snapshot は present まで stale 行を保持しうる
                // 隙間を塞ぎ、5.2 の原子性を照会時点で成立させるため（次 present の空再導出と冪等）。
                self.choice_hover.remove(&cue.actor);
                self.choice_snapshot.remove(&cue.actor);
            }
            CueCommand::ClearAll => {
                for render in self.surfaces.values_mut() {
                    render.executor.request_clear();
                }
                // 全スコープの原子的無効化（R5.1/5.2/5.4・#6 冒頭全消し）: 保持する**全** actor の
                // hover／ヒット行スナップショットを一括初期化する（上流は残存スコープを列挙できない——
                // `state.rs::apply_cue(ClearAll)` の全 actor_states 消去と対）。cue が名指ししない
                // actor の stale hover／snapshot も同時に消え、片方だけ古い状態が残らない（5.2）。
                self.choice_hover.clear();
                self.choice_snapshot.clear();
            }
            // 他コマンドは描画実行部への全域クリアを要さない（グリフ更新は present_frame が
            // リビール進行として描き、非担当コマンドは reveal を汚さない）。`Cursor` の
            // warn-once 良性スキップ・記録は純粋層 `state.apply_cue` が担う（本口は clear 要否のみ）。
            CueCommand::Text(_)
            | CueCommand::Emote { .. }
            | CueCommand::Choice { .. }
            | CueCommand::EntityRef(_)
            | CueCommand::Custom { .. }
            | CueCommand::NewLine { .. }
            | CueCommand::Cursor { .. }
            | CueCommand::BalloonSurface { .. }
            | CueCommand::Wait => {}
        }
        self.state.apply_cue(cue);
    }

    /// 純粋状態機械（可視グリフ数・actor 状態の読み取り口）。
    pub fn state(&self) -> &TextLayerState {
        &self.state
    }

    /// 調整値（line_pitch 係数）。
    pub fn config(&self) -> &TextLayerConfig {
        &self.config
    }

    /// actor の供給面が予約スロットへ装着済みか。
    pub fn is_attached(&self, actor: &ActorKey) -> bool {
        self.surfaces.contains_key(actor)
    }

    /// 装着済み actor の供給面（readback 等の観測口・未装着は `None`）。
    pub fn surface(&self, actor: &ActorKey) -> Option<&TextSurface> {
        self.surfaces.get(actor).map(|render| &render.surface)
    }

    /// 装着済み actor の決定論観測統計（[`ViewboxExecutor::stats`]・未装着は `None`）。
    ///
    /// [`surface`](Self::surface) と同型の additive アクセサ（R9.2 非抵触——emo2-boot 消費経路の
    /// 再定義ではない）。example／統合テストがこの口から actor 別の [`DrawStats`]
    /// （blit・`DrawTextLayout` 実行回数・行 TextLayout 生成回数・FullClear 回数）を読み、
    /// 「可視窓のみ移動フレームで確定 content の再描画が起きない」等を決定論的に観測する
    /// （R3.5/R10.3・目視非依存）。`ViewboxExecutor::stats()` は runtime 内部の `ActorRender` に
    /// 抱えられており、この読み口がないと example から R10.3 checkpoint が成立しない。
    pub fn draw_stats(&self, actor: &ActorKey) -> Option<DrawStats> {
        self.surfaces
            .get(actor)
            .map(|render| render.executor.stats())
    }

    /// hover 状態注入（契約正本・R4.1）。`None`＝ハイライト無し。UI スレッド専用（runtime は `!Send`）。
    ///
    /// 注入値は actor ごとに保持し、次の提示フレームの装飾（task 8.2）が読む。`ordinal` が現存
    /// 選択肢スパンに無い場合も **panic せず**そのまま保持し、描画時に「ハイライト無し」として
    /// 縮退する（stale ordinal——`decorate_canvas` が hover 印を付けない・design.md RuntimeContract）。
    /// 縮退検出時は `debug!` を一件出す（log-first・ループを殺さない）。
    pub fn inject_choice_hover(&mut self, actor: &ActorKey, hover: Option<usize>) {
        // 現存スパンに無い ordinal は縮退（ハイライト無し）——検出を debug ログに残す（panic しない）。
        if let Some(ordinal) = hover {
            let exists = self
                .state
                .actor_state(actor)
                .is_some_and(|s| s.choices().iter().any(|span| span.ordinal == ordinal));
            if !exists {
                debug!(
                    actor = %actor,
                    ordinal,
                    "inject_choice_hover: 現存選択肢スパンに無い ordinal——ハイライト無しとして縮退（保持のみ・panic なし）"
                );
            }
        }
        self.choice_hover.insert(actor.clone(), hover);
    }

    /// 行ヒットジオメトリ照会（契約正本・R3.2）。
    ///
    /// **鮮度契約**: 最後に提示（present）したフレームの導出値＝表示と同一 layout からの単一導出
    /// （R3.3/5.2・population は present_actor＝task 8.2）。未装着・選択肢なし・スナップショット未
    /// population は空 slice。
    pub fn choice_hit_rows(&self, actor: &ActorKey) -> &[ChoiceHitRow] {
        self.choice_snapshot
            .get(actor)
            .map_or(&[], Vec::as_slice)
    }

    /// 「選択肢表示中」照会（R1.3・照会のみ＝バリア解決はしない）。
    ///
    /// **表示層自身**の選択肢スパン集合（[`ActorTextState::choices`](crate::state::ActorTextState::choices)）が
    /// 非空であることを表す（DD-6——供給側 `CuePlayerState::WaitingForChoice` バリアの真実源とは別）。
    /// 未知 actor・スパン空は `false`。
    pub fn choice_active(&self, actor: &ActorKey) -> bool {
        self.state
            .actor_state(actor)
            .is_some_and(|s| !s.choices().is_empty())
    }
}

/// 結線 API（UI スレッド＝pump スレッドから呼ぶ・design.md「TextLayerActor」正本）:
/// `spawn_ui` で UI ドレインを起動し、受信口 [`EmoTextSink`] と drain の join ハンドルを返す。
///
/// handler は `runtime` の `Rc` clone を捕捉し（`!Send` handler・基盤許容）、
/// [`handle_text_msg`]（終了規律の正準写像）へ委譲して cue を純粋状態へ適用する。
/// 終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`・全 `UiSender`
/// （＝全 [`EmoTextSink`] クローン）drop＝drain 正常終了（R1.4・error ログなし）。
/// 個別 cue の適用失敗（runtime 借用競合など）は `Err` 戻し→基盤が `error!`＋継続する
/// （R1.5——失敗は終了経路ではない・panic しない）。
///
/// # 前提
///
/// UI（pump）スレッドから呼ぶこと。誤用は呼出時検出不能（基盤既知リスク——`spawn_ui` の
/// Risks 参照・spawn 時 debug! 診断で緩和）。
pub fn spawn_emo_text(
    runtime: Rc<RefCell<TextLayerRuntime>>,
) -> Result<(EmoTextSink, wintf_winmsg_executor::JoinHandle<()>), UiSpawnError> {
    let (tx, handle) = areka_actor::spawn_ui("emo-text", move |msg: TextMsg| {
        handle_text_msg(msg, |cue| match runtime.try_borrow_mut() {
            Ok(mut rt) => {
                rt.apply_cue(&cue);
                Ok(())
            }
            // 借用競合（UI スレッド上の別処理が runtime を保持中）——panic せず Err 戻しで
            // 基盤の error!＋継続に乗せる（当該 cue は失われるが後続の受理は破壊しない・R1.5）。
            Err(_) => Err(format!(
                "TextLayerRuntime が借用中のため cue を適用できない（actor={}, at={}）——当該 cue は失われる",
                cue.actor, cue.at
            )),
        })
    })?;
    Ok((EmoTextSink::new(tx), handle))
}

/// フレーム提示ステップ（毎フレーム UI スレッドで呼ぶ・example/emo2-boot が駆動）:
/// `talk_time` は注入時刻（talk 起点相対秒・実時間 sleep 不使用・R3.3）。
///
/// actor ごとに「リビール進行の解決（純粋）→ レイアウト決定（純粋）→ viewbox ダーティ矩形
/// スクロール描画（COM）→ 変化ありのフレームだけ供給面を提示（Present のみ）」を駆動する:
///
/// - **未解決 actor**（binding 未登録）: 状態は蓄積のみ・描画スキップ・次フレーム再試行
///   （actor ごと初回 `warn!`＋以降 `debug!`——frame の `Err` にはしない）。
/// - **初回解決フレーム**: World 資源（[`GraphicsCore`]／[`WucGraphicsResource`]）から
///   供給面/描画実行部を構築し予約スロットへ装着する（`ActorRender` 不在時のみ・`info!`）。
///   通常は actor ごと初回の 1 回だが、k 再追従（[`TextLayerRuntime::refresh_actor_scale`]）で
///   `ActorRender` を破棄した直後のフレームでも再発火する（新 k の物理寸で再生成・R8.2）。
/// - **装着済み actor のグリフ更新**: viewbox ダーティ矩形スクロール描画→（変化ありのフレームだけ）
///   swapchain Present で完結し、バルーン surface 本体の再合成（emo-compose 再駆動）を要求しない（R9.3）。
/// - **デバイス失敗**: 失敗源で `error!` 済み（log-first）。当該 actor の当該フレーム提示を
///   skip して他 actor の処理は継続し、最初の失敗を `Err` として返す（次フレーム再試行）。
pub fn present_frame(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    talk_time: f64,
) -> Result<(), TextLayerError> {
    // 状態を持つ actor だけが提示対象（binding 登録済みでも cue が無ければ描くものがない）。
    let actors: Vec<ActorKey> = runtime.state.actors().map(|(key, _)| key.clone()).collect();
    let mut first_err: Option<TextLayerError> = None;
    for actor in &actors {
        match present_actor(runtime, world, actor, talk_time) {
            Ok(()) => {}
            // 未解決 actor: 蓄積継続・描画スキップ・次フレーム再試行（正常経路・Err にしない）。
            Err(TextLayerError::SlotNotAttached { .. }) => {
                if runtime.unresolved_warned.insert(actor.clone()) {
                    warn!(
                        actor = %actor,
                        talk_time,
                        "actor の装着先（予約スロット）が未解決——状態を蓄積し描画をスキップして次フレームで再試行する"
                    );
                } else {
                    debug!(
                        actor = %actor,
                        talk_time,
                        "actor の装着先が未解決のまま——蓄積継続・描画スキップ・次フレーム再試行"
                    );
                }
            }
            // デバイス失敗: 失敗源で error! 済み。他 actor は継続し、最初の失敗を返す。
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// 1 actor 分のフレーム提示（[`present_frame`] の内訳・log-first）。
///
/// binding 未解決は [`TextLayerError::SlotNotAttached`]（呼び手が skip 写像）。
fn present_actor(
    runtime: &mut TextLayerRuntime,
    world: &mut World,
    actor: &ActorKey,
    talk_time: f64,
) -> Result<(), TextLayerError> {
    // ── 解決確認: binding＋layout 入力が揃うまでは蓄積のみ（描画スキップ） ──
    let (Some(binding), Some(resolved)) = (
        runtime.routing.get(actor).copied(),
        runtime.layout_input.get(actor).cloned(),
    ) else {
        return Err(TextLayerError::SlotNotAttached {
            actor: actor.to_string(),
        });
    };

    let contract = ScaleContract::new(binding.scale, None);

    // ── 初回解決フレーム: World 資源から描画資源を構築し予約スロットへ装着（初回のみ） ──
    if !runtime.surfaces.contains_key(actor) {
        let region = &resolved.region;
        // 物理寸＝ceil(validrect 寸 × k)・offset＝validrect 原点 × k（DPI/スケール契約）。
        let physical_size = (
            contract.physical_extent(ImagePx(region.right() - region.left())),
            contract.physical_extent(ImagePx(region.bottom() - region.top())),
        );
        let physical_offset = (
            contract.to_physical(ImagePx(region.left())).0,
            contract.to_physical(ImagePx(region.top())).0,
        );

        // Compositor は所有クローンで取り出し、以後の &mut World 装着と借用衝突しない
        // ようにする（emo-present presenter.rs と同じ規律）。
        let Some(compositor) = world
            .get_resource::<WucGraphicsResource>()
            .and_then(|resource| resource.compositor().cloned())
        else {
            error!(
                actor = %actor,
                "present_frame: WucGraphicsResource/Compositor 不在（供給面を生成できない）"
            );
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "WucGraphicsResource::compositor",
            });
        };
        if !world.contains_resource::<GraphicsCore>() {
            error!(
                actor = %actor,
                "present_frame: GraphicsCore 不在（供給面を生成できない）"
            );
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "GraphicsCore resource",
            });
        }
        // GraphicsCore は resource_scope で一時取り外し——attach の &mut World と
        // 資源借用の衝突を構造回避する。
        let config = runtime.config;
        let render = world.resource_scope(
            |world, core: bevy_ecs::world::Mut<GraphicsCore>| -> Result<ActorRender, TextLayerError> {
                let surface = TextSurface::attach(
                    world,
                    &binding,
                    &compositor,
                    &core,
                    physical_size,
                    physical_offset,
                )?;
                let executor = ViewboxExecutor::new(&core)?;
                let Some(factory) = core.dwrite_factory() else {
                    error!(actor = %actor, "present_frame: dwrite_factory 不在（metrics を構築できない）");
                    return Err(TextLayerError::Device {
                        hresult: 0,
                        context: "GraphicsCore::dwrite_factory",
                    });
                };
                let metrics = DWriteMetrics::new(factory, &resolved.font, resolved.mode, &config)?;
                Ok(ActorRender {
                    surface,
                    executor,
                    metrics,
                })
            },
        )?;
        runtime.surfaces.insert(actor.clone(), render);
        info!(
            actor = %actor,
            slot = ?binding.slot,
            ?physical_size,
            wrap = ?resolved.wrap,
            "テキスト供給面を予約スロットへ装着した（ActorRender 不在時のみ・以降は Present のみ）"
        );
    }

    // ── リビール進行（純粋）→ レイアウト決定（純粋）→ viewbox ダーティ矩形描画 → 提示（Present のみ） ──
    let Some(render) = runtime.surfaces.get_mut(actor) else {
        // 直前の insert 直後に到達するため構造上起こらない——防御（panic 禁止・log-first）。
        error!(actor = %actor, "present_frame: 装着済み描画資源の引き当てに失敗（構造不変の破れ）");
        return Err(TextLayerError::Device {
            hresult: 0,
            context: "ActorRender missing after attach",
        });
    };
    let Some(actor_state) = runtime.state.actor_state(actor) else {
        // present_frame は state.actors() 由来の actor だけを渡す——防御的に空フレーム扱い。
        return Ok(());
    };
    let visible = runtime.state.visible_glyphs(actor, talk_time);
    // 折返し計画: ON（BudouxWordWrap）のときだけ分かち書き境界を全 items から計算し
    // layout へ供給する。OFF（CharByChar）は plan を計算すらしない（R4.2 の構造保証——
    // segment_plan を呼ぶのは Segmented アームだけ）。`plan` は ON アームでのみ束縛され、
    // 借用 `&plan` が layout 呼出まで生存するよう遅延初期化パターンで宣言する。
    let plan;
    let wrap = match resolved.wrap {
        WrapMode::CharByChar => WrapPlan::CharByChar,
        WrapMode::BudouxWordWrap => {
            plan = segment_plan(actor_state.items());
            WrapPlan::Segmented(&plan)
        }
    };
    // `\_l` 換算縮退（6.5）の warn-once を production で有効化する持続 guard を渡す
    // （純挙動は `layout` と完全同一——差は縮退ログの有無のみ・task 4.2 が本配線へ委譲）。
    let lines = LayoutEngine::layout_with_cursor_warn(
        actor_state.items(),
        visible,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &render.metrics,
        wrap,
        actor,
        &mut runtime.cursor_warn,
    );
    let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);

    // ── 選択肢パイプライン: 同一 lines を単一の源に 注釈→装飾→描画（表示とヒットの単一導出・R3.3/5.2） ──
    // 注釈は layout 直後の同一 lines を消費する（可視窓調整後の行へ再適用しない——design Precondition）。
    let spans = actor_state.choices();
    let segments = annotate_lines(&lines, spans);
    // ハイライト帯／ヒット帯のブロック軸寸（**単一の源**・R3.3）: 実 font metrics の行ボックス丈
    // （descent 込み）を行送りピッチで頭打ちにした値を 1 度だけ決め、装飾（描画帯）とヒット導出
    // （照会帯）の両方へ同一値を配る。em ボックス丈（font.height）で切ると和文フォントの descent
    // インクが帯の外へ出る（実機不具合「選択肢の文字の下が切れる」の真因）。
    let band_extent = highlight_band_extent(
        resolved.font.height,
        render.metrics.line_box_height(resolved.font.height),
        render.metrics.line_pitch(resolved.font.height),
    );
    // hover 印は per-actor 保持値（未注入＝None＝ハイライト無し・8.1）。
    let hover = runtime.choice_hover.get(actor).copied().flatten();
    // 装飾: hover 行へ塗り/文字色を焼く。セグメント空（選択肢無し）は decorate が恒等＝canvas 無変更（非退行）。
    let canvas = ContentCanvas::from_layout(&lines, &resolved.region, resolved.mode);
    let canvas = decorate_canvas(
        canvas,
        &segments,
        hover,
        resolved.choice_style,
        resolved.font.color,
        &resolved.region,
        resolved.mode,
        band_extent,
    );
    let changed = render.executor.render(
        &canvas,
        &window,
        &resolved.font,
        resolved.mode,
        &contract,
        &mut render.surface,
    )?;
    // 装着済み actor のグリフ更新は供給面の提示のみで完結（emo-compose 再駆動なし・R9.3）。
    // 変化ありのフレームだけ提示する（`FramePlan::NoChange` は blit も描画も present も省く——
    // readback は front を読むため観測述語に影響しない・R1.1/R3.1）。
    if changed {
        // ── ヒット行スナップショット更新（present 成功時のみ・表示と同一 lines/segments の単一導出・5.2） ──
        // committed は既存 visible_window→executor が確定した面反映済みスクロールをそのまま消費する
        // （新規のスクロール可視判定は追加しない・6.3）。NoChange フレームはこの更新を丸ごと省き
        // 直前スナップショットを不変のまま保つ。
        let committed = render.executor.scroll_state().committed;
        // 帯は装飾（描画）へ渡したのと**同一の band_extent**——描画とヒットの座標整合（R3.3）。
        let hit_rows = derive_hit_rows(
            &lines,
            &segments,
            resolved.mode,
            &resolved.region,
            band_extent,
        );
        // 各ヒット行を配送順序数で対応スパンへ突き合わせ、窓物理 px 矩形＋下流構成材料を同梱する。
        let snapshot: Vec<ChoiceHitRow> = hit_rows
            .iter()
            .filter_map(|row| {
                spans
                    .iter()
                    .find(|span| span.ordinal == row.ordinal)
                    .map(|span| ChoiceHitRow {
                        ordinal: row.ordinal,
                        id: span.id.clone(),
                        label: span.label.clone(),
                        references: span.references.clone(),
                        rect: to_window_physical(
                            row,
                            &resolved.region,
                            resolved.mode,
                            committed,
                            &contract,
                        ),
                    })
            })
            .collect();
        runtime.choice_snapshot.insert(actor.clone(), snapshot);
        render.surface.present()?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "actor_test_support.rs"]
mod test_support;

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
#[path = "actor_scale_refresh_tests.rs"]
mod scale_refresh_tests;
