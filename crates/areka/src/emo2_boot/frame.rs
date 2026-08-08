//! 毎フレーム結線（attach／dpi／drain／text）の排他 system と NonSend 配線状態。
//!
//! `Emo2Wiring`（NonSend resource・presenter／rx／runtime／clock／assets／attached を保持）と
//! 排他 system `emo2_frame_system(world: &mut World)`（donor パターン: remove→各フェーズ→insert）を
//! 所有する。各フェーズ:
//! - attach: GPU 資源＋`GhostWindows` 到達ゲート→`plan_attachments`（DD-12）→バルーン初回 `ShowSurface`
//!   （面0）→文字層スロット取得→`register_actor_view`（`Option::take` で高々 1 回消費）。**シェルは初回
//!   `ShowSurface` を発行せず**最初のさくらスクリプト `\s` cue まで非表示を保つ（defect #5・実機#5）。
//! - dpi: `Changed<DPI>` の窓を永続 `SystemState` で観測し（`anchor_changed_system` 先例）、当該窓の
//!   target を `refresh_scale` で再スケールして窓寸を reconcile する（emo-dpi-scaling task 4.2・D8）。
//! - drain: attach 完了後のみ `Receiver::try_iter` で `PresentCommand` を FIFO で `presenter.apply` へ適用し、
//!   続けて表示成立点の状態照合報告（`take_pending_resize`）で窓寸を reconcile する（第 2 経路）。
//! - text-scale: 装着済み balloon scope の文字層 binding を presenter の**現適用 k** へ毎フレーム
//!   合わせ直す（`refresh_actor_scale`・emo-dpi-scaling task 7.2・D11-4・Req8）。適用 k の更新点は
//!   1 フレームに 2 つ（dpi 相の `refresh_scale`／drain 相の `apply_show`）あるため、**両者の下流**・
//!   text 相の**上流**に置く。
//! - text: `TalkClock::talk_time` が `Some` のとき `present_frame` を呼ぶ（`Err` は `error!`＋継続）。
//!
//! `plan_attachments`（`GhostWindows::scopes()` を正とする純関数・DD-12）も本モジュールに属する。
//!
//! 本ファイルは task 3 の純関数 `plan_attachments`（＋`AttachPlan`／`PlannedAttach`）、task 4.1 の
//! NonSend 結線資源 `Emo2Wiring` と attach フェーズ（`run_attach_phase`＋補助 `connect_balloon_text`）、
//! そして task 4.2 の drain フェーズ（`run_drain_phase`）・text フェーズ（`run_text_phase`＋純判断
//! `resolve_talk_time`）・排他 system `emo2_frame_system`（remove→3 フェーズ→insert）を実装する。

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Changed, Query};
use bevy_ecs::system::SystemState;
use bevy_ecs::world::World;
use tracing::{debug, error, info, warn};

use areka_emo_present::{EmoPresenter, PresentCommand, TargetId, TextSlotView};
use areka_emo_text::actor::{present_frame, TextLayerRuntime};
use areka_parsers::balloon::BalloonModel;
use areka_sakura::ActorKey;
use wintf::ecs::{FrameTime, GraphicsCore, SizeI, WindowPos, WucGraphicsResource, DPI};

use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
use crate::placement::follow::{resize_window_keep_position, resize_window_to};
use crate::placement::resolver::SizePx;
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker, GhostWindows};

use super::assets::{BalloonScopeAssets, BootAssets, ScopeAssets};
use super::move_cue::{apply_move_directive, MoveDirective};
use super::talk_clock::TalkClock;
use super::target_map::{balloon_target, shell_target};

/// 窓×資産の scope 突き合わせ結果（DD-12・純関数 [`plan_attachments`] の戻り値）。
///
/// `GhostWindows::scopes()`（`usize`・昇順・**正**）を [`BootAssets`] の資産 scope と照合した
/// 装着計画。三分類は排他:
/// - `items`: 窓と資産の双方が揃った scope の装着項目（`window_scopes` の出現順を保つ）。
/// - `missing_assets`: 窓はあるが対応資産が無い scope（呼び手が `warn!`＋skip＝表示なし縮退）。
/// - `unused_assets`: 資産はあるが窓が無い scope（呼び手が `debug!`＋破棄）。
///
/// 純粋・決定論（GPU 不要）。呼び手（attach フェーズ）は `items.len()` が期待窓数と一致することを
/// 積極 assert し、warn+skip 縮退が scope 導出バグを隠さないことを檻に入れる（DD-12・spine S1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachPlan {
    /// 窓と資産の双方が揃った scope の装着項目（`window_scopes` 出現順）。
    pub items: Vec<PlannedAttach>,
    /// 窓はあるが対応資産が無い scope（`usize` のまま・呼び手が `warn!`＋skip 縮退）。
    pub missing_assets: Vec<usize>,
    /// 資産はあるが窓が無い scope（`u32`・呼び手が `debug!`＋破棄）。
    pub unused_assets: Vec<u32>,
}

/// 1 scope 分の装着計画項目（attach フェーズ＝task 4.1 が消費）。
///
/// scope の shell／balloon 表示対象（`target_map` の正本・DD-3 の `2*scope`／`2*scope+1` 採番）と
/// 初期表示 surface id（DD-9・task 2.6 が [`super::assets::ScopeAssets`] へ焼き込み済み）を運ぶ。
/// `static_binds` は [`BootAssets`] 単一共有ゆえ本項目には複製しない（attach フェーズが
/// `assets.static_binds` を直接読む）。
///
/// `shell_index`／`balloon_index` は attach フェーズが非 Clone な `EmoWorld` を
/// `assets.shells`／`assets.balloons` から添字で move 消費するための添字（DD-12 の突き合わせで
/// 自然に確定する）。当該 scope に balloon 資産が無い場合 `balloon_index` は `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAttach {
    /// 対応 scope 番号（`usize`→`u32` 変換済み）。
    pub scope: u32,
    /// シェル表示対象（`shell_target(scope)`＝偶数・DD-3）。
    pub shell_target: TargetId,
    /// バルーン表示対象（`balloon_target(scope)`＝奇数・DD-3）。
    pub balloon_target: TargetId,
    /// 初期表示 surface id（`ScopeAssets.initial_surface_id`・DD-9）。
    ///
    /// **注記（defect #5・2026-07-13 実機#5）**: attach フェーズはこの値でシェル初回表示を駆動しなく
    /// なった（シェルは最初の `\s` cue まで非表示）。planner の突き合わせ・DD-9 の記録として carry する。
    pub initial_surface_id: u32,
    /// `assets.shells` 内の対応添字（attach フェーズの add 消費用）。
    pub shell_index: usize,
    /// `assets.balloons` 内の対応添字（同 scope・無ければ `None`）。
    pub balloon_index: Option<usize>,
}

/// 窓一覧と構築資産を突き合わせ装着計画を返す純関数（DD-12・GPU 不要の決定論単体テスト対象）。
///
/// **`GhostWindows::scopes()`（`window_scopes`・`usize`・正）が唯一の正**（DD-12）。窓一覧を
/// 走査し、各窓 scope を `u32` へ変換（`u32::try_from`）してから `assets.shells` を `scope`
/// フィールドで照合する:
/// - 一致 → `items` へ [`PlannedAttach`] を積む（`window_scopes` の出現順を保つ）。
/// - 不一致、または `u32` に収まらない `usize`（`u32::MAX` 超過＝如何なる資産 scope とも一致し得ない）
///   → 当該窓 scope（`usize`）を `missing_assets` へ。
///
/// 加えて、`assets.shells` の各 scope のうち `window_scopes` に現れないもの（資産あり窓なし）を
/// `unused_assets`（`u32`）へ集める（`u32`→`usize` は無損失ゆえ `as` で比較）。
///
/// # 純粋性
/// 状態・I/O・GPU なし。戻り値は入力順に決定論的（`window_scopes`／`assets.shells` の順序を保つ）。
/// `window_scopes` は `GhostWindows` 契約により一意昇順が期待されるが、本関数は分類の正しさを
/// その前提に依存しない（重複窓は重複項目として忠実に写す）。
///
/// # scope 整合（DD-12）
/// 窓あり資産なしは `warn!`＋skip 縮退、資産あり窓なしは `debug!`＋破棄を呼び手（attach フェーズ）が
/// 行う。呼び手は `items.len()` が期待窓数と一致することを積極 assert し、縮退が scope 導出バグを
/// 隠さないことを担保する（spine S1）。
pub fn plan_attachments(window_scopes: &[usize], assets: &BootAssets) -> AttachPlan {
    let mut items = Vec::new();
    let mut missing_assets = Vec::new();

    // GhostWindows::scopes()（正）を走査し、usize→u32 の吸収をここで一度だけ行う（DD-12）。
    for &window_scope in window_scopes {
        // u32 に収まらない usize は如何なる資産 scope（u32）とも一致し得ない → missing 分類。
        let Ok(scope) = u32::try_from(window_scope) else {
            missing_assets.push(window_scope);
            continue;
        };
        // 資産存在の正は shells の scope フィールド。一致した添字を attach フェーズへ運ぶ。
        match assets.shells.iter().position(|s| s.scope == scope) {
            Some(shell_index) => {
                // balloon 資産は同 scope で引く（build_boot_assets は shell と同 scope 集合で組むが、
                // 万一不揃いなら None として運び、attach フェーズが文字層接続を縮退できるようにする）。
                let balloon_index = assets.balloons.iter().position(|b| b.scope == scope);
                items.push(PlannedAttach {
                    scope,
                    shell_target: shell_target(scope),
                    balloon_target: balloon_target(scope),
                    initial_surface_id: assets.shells[shell_index].initial_surface_id,
                    shell_index,
                    balloon_index,
                });
            }
            // 窓はあるが対応資産が無い → warn!＋skip 縮退の対象（呼び手が観測）。
            None => missing_assets.push(window_scope),
        }
    }

    // 資産あり窓なし: shells の scope で window_scopes に不在のものを集める。
    // u32→usize は無損失（対象プラットフォームの usize は 32bit 以上）ゆえ as で比較する。
    let mut unused_assets = Vec::new();
    for shell in &assets.shells {
        if !window_scopes.contains(&(shell.scope as usize)) {
            unused_assets.push(shell.scope);
        }
    }

    AttachPlan {
        items,
        missing_assets,
        unused_assets,
    }
}

// ---------------------------------------------------------------------------
// Emo2Wiring＋attach フェーズ（tasks.md task 4.1・design「UI 毎フレーム結線 / frame」）
// ---------------------------------------------------------------------------

/// 毎フレーム三相結線の NonSend 状態（design「Emo2Wiring＋emo2_frame_system」・State Management）。
///
/// `EmoPresenter`（`!Send`）・`Receiver`（`!Sync`）・`Rc<RefCell<TextLayerRuntime>>`（`!Send`）を
/// 内包するため NonSend resource として `wire_emo2_boot`（task 5.1）が挿入する。本 task（4.1）は
/// 構築（[`Emo2Wiring::new`]）と attach フェーズ（[`run_attach_phase`]）に加え、drain フェーズ
/// （[`run_drain_phase`]・`rx` を消費）・text フェーズ（[`run_text_phase`]・`clock` を消費）・排他
/// system [`emo2_frame_system`]（3 フェーズを remove→insert で駆動）を所有する（task 4.1／4.2）。
pub struct Emo2Wiring {
    /// 表示層の指令適用ハブ（UI スレッド専有・`!Send`）。
    presenter: EmoPresenter,
    /// worker（seriko 経由 `PresentBridge`）からの表示指令受信端（task 4.2 の drain で消費）。
    rx: Receiver<PresentCommand>,
    /// talk スレッド（`MoveCueSink`）からの `\![move]` 指令受信端（frame 相 drain＝[`run_move_drain_phase`]
    /// が消費）。
    ///
    /// `PresentBridge` の `rx` と同型の配線: `wire_emo2_boot`（task 9.1）が
    /// `mpsc::channel::<MoveDirective>()` の受信端を受け渡し、frame 相の [`emo2_frame_system`] が
    /// [`run_move_drain_phase`] 経由で `try_iter` し `apply_move_directive` へ適用する（task 9.2）。
    move_rx: Receiver<MoveDirective>,
    /// バルーン文字層ランタイム（`register_actor_view`／`present_frame` の所有・`!Send`）。
    runtime: Rc<RefCell<TextLayerRuntime>>,
    /// scope → attach 相で装着に使った [`BalloonModel`]（文字層 k 再追従の再利用源・D11-3・R8.1）。
    ///
    /// `register_actor_view`（装着）と [`TextLayerRuntime::refresh_actor_scale`]（再追従）はいずれも
    /// `&BalloonModel` を要する。装着で使ったモデルをここへ記憶しておき、文字層スケール相
    /// （[`run_text_scale_phase`]）の再追従が
    /// **再パースせず同一モデル**で binding を組み直せるようにする（再パースすれば「装着時と再追従時で
    /// 別モデル」という静かな食い違いの余地が生まれる）。
    ///
    /// 起動時資産は [`BalloonScopeAssets`] が scope 別の定義を保持する（旧 `BootAssets.balloon_model`
    /// の共有 1 本は撤去済み）。ゆえにキーは **scope**——再追従は「どの scope の balloon 窓か」から
    /// 引くのが自然である。attach（[`run_attach_phase`]）は当該 scope の資産から取り出した定義を
    /// **ここと文字層結線（[`connect_balloon_text`]）の双方へ同一値で**挿す（Req 4.1/4.2）。
    balloon_models: HashMap<u32, BalloonModel>,
    /// 文字層 k 追従で `text_slot_view` が `None` だった scope の警告済み集合（R8.6 のエッジガード）。
    ///
    /// [`run_text_scale_phase`] は毎フレーム走る。表示未確立の縮退を素朴に `warn!` すると毎フレーム
    /// 鳴って log を溺れさせるため、**scope ごとに一度だけ**鳴らし、view が取れた時点で除去して
    /// 再武装する（`areka-emo-text` の `unresolved_warned: BTreeSet<ActorKey>` と同型の先例）。
    /// R8.6 が求めるのは縮退が**観測できる**ことであって毎回鳴ることではない。
    text_scale_warned: BTreeSet<u32>,
    /// talk 起点相対秒の時刻源（task 4.2 の text フェーズで `talk_time` を引く）。
    clock: TalkClock,
    /// load-time 構築資産（attach で `take` して高々 1 回消費）。
    assets: Option<BootAssets>,
    /// attach 完了フラグ（高々 1 回のゲート・以降 no-op）。
    attached: bool,
    /// `Changed<DPI>` 観測の**永続** [`SystemState`]（[`run_dpi_phase`]・emo-dpi-scaling task 4.2）。
    ///
    /// `anchor_changed_system` の `Local<Option<SystemState<..>>>` と同じ役割を担う。あちらは bevy の
    /// system 引数として `Local` を受けられるが、本フェーズは排他 system [`emo2_frame_system`] から
    /// 呼ばれる**素の関数**（design の署名 `run_dpi_phase(&mut Emo2Wiring, &mut World)`）ゆえ `Local`
    /// を取れない——run を跨いで `last_run` tick を保つ器がここに要る。毎 run で `SystemState::new`
    /// を作り直すと `last_run` が 0 のままとなり `Changed` が全窓へ誤マッチし続ける（＝毎フレーム
    /// 全窓 refresh の churn）ため、必ず使い回す。
    dpi_state: Option<SystemState<DpiChangedQuery>>,
}

impl Emo2Wiring {
    /// 結線資源を構築する（`wire_emo2_boot`＝task 5.1／9.1 が呼ぶ）。`assets` は `Some` で保持し、
    /// attach フェーズ（[`run_attach_phase`]）が `take` で高々 1 回消費する。`move_rx` は
    /// `MoveCueSink`（talk スレッド）と対の受信端で、frame 相 drain（task 9.2）が消費する。
    pub fn new(
        presenter: EmoPresenter,
        rx: Receiver<PresentCommand>,
        move_rx: Receiver<MoveDirective>,
        runtime: Rc<RefCell<TextLayerRuntime>>,
        clock: TalkClock,
        assets: BootAssets,
    ) -> Self {
        Self {
            presenter,
            rx,
            move_rx,
            runtime,
            // attach 相が装着した scope ごとに埋める（D11-3）。
            balloon_models: HashMap::new(),
            // 縮退警告のエッジガード（初期は未警告＝最初の縮退で 1 回鳴る）。
            text_scale_warned: BTreeSet::new(),
            clock,
            assets: Some(assets),
            attached: false,
            // 初回 [`run_dpi_phase`] で遅延生成する（`SystemState::new` は `&mut World` を要する）。
            dpi_state: None,
        }
    }

    /// 当たり判定 resolver への読み口（design DD-IE-9/DD-IE-10・「Modified Files」mod.rs 行）。
    ///
    /// 内包する [`EmoPresenter`] を読み取り専用で貸し出す。`input-events` の region 解決
    /// （`RegionSource::Presenter`＝task 2.6/2.7）が、この借用を
    /// [`super::hit_region::resolve_hit_region`]`(presenter, scope, x, y)` の第 1 引数
    /// （`&EmoPresenter`）へそのまま渡して当たり判定を解決する（Req 1.3・collision-geometry の
    /// 契約を消費のみ）。所有・可変アクセスは presenter を専有する frame 相（attach/drain/text）に
    /// 閉じたまま、UI 配線層へは read 口のみを開ける（本番表面を最小に保つ）。
    ///
    /// 第一 production 消費者（`input-events`＝roadmap W2・task 2.6/2.7）が生えるまでは呼び出しが
    /// 無く dead_code 警告になる（`areka` は bin crate・baseline は警告皆無）ため明示抑止する。
    #[allow(dead_code)]
    pub(crate) fn presenter(&self) -> &EmoPresenter {
        &self.presenter
    }

    /// 文字層 runtime への共有ハンドル読み口（design「アクセサ（emo2_boot/frame.rs）」・
    /// `Emo2Wiring::runtime()`・Req 4.1）。
    ///
    /// choice-interact のバルーン選択肢対話配線（`super::super::input_events::balloon`）が、この借用を
    /// 経由して `TextLayerRuntime` を読み取り選択肢ハイライト／確定を橋渡しする。既存 [`presenter()`]
    /// アクセサと同型の additive な読み口であり、挙動は一切変えない（`runtime` の所有・可変アクセスは
    /// frame 相の text フェーズに閉じたまま、配線層へは read 口のみを開ける）。上流クレート
    /// （`areka-emo-text`）には一切手を入れない（R8.5）。
    ///
    /// 第一 production 消費者（choice-interact の balloon 配線＝後続 task）が生えるまでは呼び出しが
    /// 無く dead_code 警告になる（`areka` は bin crate・baseline は警告皆無）ため明示抑止する。
    ///
    /// [`presenter()`]: Self::presenter
    #[allow(dead_code)]
    pub(crate) fn runtime(&self) -> &Rc<RefCell<TextLayerRuntime>> {
        &self.runtime
    }

    /// `\![move]` 指令受信端への test-support アクセサ（task 9.1 の存在檻・9.3 の e2e で消費）。
    ///
    /// 本番の frame 相 drain（task 9.2）は `move_rx` を private に閉じて `apply_move_directive` へ
    /// 適用する。9.1 段階では channel 配線の到達性（`MoveCueSink`→`Emo2Wiring` の受信端が届く）を
    /// 決定論に固定するための最小 read 口として `#[cfg(test)]` で開ける（本番表面は増やさない）。
    #[cfg(test)]
    pub(crate) fn drain_move_directives(&self) -> Vec<MoveDirective> {
        self.move_rx.try_iter().collect()
    }

    // ── spine 観測用 test-support アクセサ（tasks.md task 6.2・spine S1/S3/S4） ──────────
    //
    // 本番結線（`wire_emo2_boot`＝task 5.1／`emo2_frame_system`）は `presenter`/`rx` を private に
    // 閉じ、drain/apply/readback は `run_attach_phase`／`run_drain_phase` 内で完結する。決定論 spine
    // （兄弟モジュール `super::spine`・`#[cfg(test)]`）は「受信 `PresentCommand` 列の形状記録」
    // （apply 前に値取り出し）と「apply 後の `read_back` 観測」（R8.2・観測境界をアダプタ記録に
    // 留めない）を行うため、private フィールドへ最小の read/passthrough を要する。以下 3 つは
    // getter/passthrough のみ（本番ロジックは一切変えない）で `#[cfg(test)]` ゲートし本番表面を増やさない。

    /// target の表示中画素（BGRA・`stride=width*4`）を読み戻す（`EmoPresenter::read_back` passthrough・S1/S3/S4）。
    #[cfg(test)]
    pub(crate) fn read_back_target(
        &self,
        target: TargetId,
    ) -> Result<Vec<u8>, areka_emo_present::PresentError> {
        self.presenter.read_back(target)
    }

    /// rx にキュー済みの `PresentCommand` を非ブロックで FIFO 全件取り出す（S3/S4 の受信列記録用）。
    ///
    /// `run_drain_phase` と同じ `Receiver::try_iter` だが、spine は形状記録のため apply 前に**値**として
    /// 取り出す（`PresentCommand` は `reply: Option<ReplySender>` ゆえ非 Clone・move で受ける）。
    #[cfg(test)]
    pub(crate) fn drain_received(&mut self) -> Vec<PresentCommand> {
        self.rx.try_iter().collect()
    }

    /// 1 件の `PresentCommand` を presenter へ適用する（`EmoPresenter::apply` passthrough・S3）。
    ///
    /// `drain_received` で取り出した指令を、形状記録後に実 presenter へ流して実描画→readback まで
    /// 通す（R8.2）ための最小口。本番は同じ `apply` を `run_drain_phase` が呼ぶ。
    #[cfg(test)]
    pub(crate) fn apply_present(&mut self, world: &mut World, cmd: PresentCommand) {
        self.presenter.apply(world, cmd);
    }

    /// 再追従用に記憶している [`BalloonModel`] の scope 集合（昇順・emo-dpi-scaling D11-3 の観測口）。
    ///
    /// 「attach 相が per-scope の model を実際に保持したか」は本番 attach（GPU 資源＋実資産）を
    /// 通さないと観測できないため、spine（in-crate GPU ハーネス）から見えるだけの read を開ける。
    #[cfg(test)]
    pub(crate) fn balloon_model_scopes(&self) -> Vec<u32> {
        let mut scopes: Vec<u32> = self.balloon_models.keys().copied().collect();
        scopes.sort_unstable();
        scopes
    }
}

/// attach フェーズ（高々 1 回・design「フェーズ①（attach）」）をテスト駆動口として実装する。
///
/// ゲート（`GhostWindows` Resource ＋ `GraphicsCore` ＋ `WucGraphicsResource::is_valid()`）成立
/// フレームで純関数 [`plan_attachments`]（DD-12）を確定し、計画項目ごとに shell／balloon target を
/// 装着する。**バルーンのみ**初回表示（面0）を駆動して文字層スロットを取得し、**シェルは初回表示を
/// 発行せず**最初のさくらスクリプト `\s` cue まで非表示を保つ（defect #5・2026-07-13 実機#5）。
/// 資産は `Option::take` で高々 1 回消費し、ゲート不成立では消費せず
/// 次フレーム再試行へ委ねる（表示なし縮退・hang しない）。窓あり資産なしは `warn!`＋skip、資産あり
/// 窓なしは `debug!`＋破棄で log-first に観測し、計画件数と実装着件数を `info!` に列挙する（spine が
/// 件数一致を積極 assert・DD-12）。個別の attach 失敗・窓欠落は `error!`／`warn!`＋継続であり
/// panic しない（log-first・R7.3）——1 scope の失敗は他 scope を巻き込まない。
///
/// donor（`examples/emo-present.rs::boot_present_system`）の attach 駆動を、複数 scope × `GhostWindows`
/// 由来の窓解決へ一般化したもの。`apply` は同期実行のため balloon の `text_slot_view` は同一フレームで
/// `Some` になるのが正常経路（DD-4）。万一 `None`（上流の遅延化）なら接続せず次フレーム再試行に委ねる
/// （R4.2・[`connect_balloon_text`]）。
pub fn run_attach_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    // 高々 1 回: 装着済みなら以降 no-op（装着後の remove/insert churn を避ける donor 慣行）。
    if wiring.attached {
        return;
    }

    // ゲート: GhostWindows Resource ＋ GPU 資源（GraphicsCore ＋ WucGraphicsResource::is_valid）。
    // いずれか欠ける間は資産を消費せず attached も立てず、次フレーム再試行へ委ねる（hang しない）。
    let gate_ready = world.get_resource::<GhostWindows>().is_some()
        && world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !gate_ready {
        return;
    }

    // GhostWindows は Clone（小さな Entity 写像）。attach_target/apply が `&mut World` を要するため、
    // 窓写像を先に clone して world の不変借用をループへ跨がせない（借用衝突回避）。
    let ghost_windows = world
        .get_resource::<GhostWindows>()
        .expect("ゲートで存在確認済み")
        .clone();
    let window_scopes: Vec<usize> = ghost_windows.scopes().collect();

    // 資産は高々 1 回消費（ゲート成立後にのみ take）。既に None（二重 attach の異常）なら log-first で
    // 観測して打ち切る（panic しない・attached を立てて以降の空回りを止める）。
    let Some(assets) = wiring.assets.take() else {
        warn!("emo2 attach: ゲート成立だが BootAssets が既に消費済み（想定外）→ 装着せず打ち切り");
        wiring.attached = true;
        return;
    };

    // DD-12: 窓一覧（正）× 資産の突き合わせ（純関数・GPU 不要）。
    let plan = plan_attachments(&window_scopes, &assets);

    // log-first の縮退観測: 窓あり資産なし＝warn!（表示なし縮退・skip）・資産あり窓なし＝debug!（破棄）。
    if !plan.missing_assets.is_empty() {
        warn!(
            missing_scopes = ?plan.missing_assets,
            "emo2 attach: 窓はあるが対応資産が無い scope（表示なし縮退・skip）"
        );
    }
    if !plan.unused_assets.is_empty() {
        debug!(
            unused_scopes = ?plan.unused_assets,
            "emo2 attach: 資産はあるが窓が無い scope（破棄）"
        );
    }

    // 非 Clone な `EmoWorld` を計画の添字で個別に move 消費するため Option 包みにする（take で 1 回）。
    let BootAssets {
        shells,
        balloons,
        // resolver は attach では未使用（seriko へは wire_emo2_boot=task 5.1 が手渡す）。
        resolver: _,
        // static_binds は attach では未使用（defect #5・2026-07-13 実機#5）: シェル初回表示を attach で
        // 焼き付けなくなったため。起動時オンの bindgroup default は seriko が保持し（spawn_seriko へ
        // 手渡し済み）、最初の `\s` cue が駆動する Show{shell,id,binds=static_binds} に載って表示層へ届く。
        static_binds: _,
        // bind_resolver は attach では未使用（seriko の actor へは task 7.2 が手渡す）。
        bind_resolver: _,
        // loop_tables は attach では未使用（SERIKO ループ表は spawn_seriko の actor 構築＝task 9.2 が
        // 手渡す）。attach 相はループを駆動しないため破棄する。
        loop_tables: _,
        // author_dpi（D1・Req1.1）: descript 宣言由来の原稿 DPI を attach 時の target 政策として
        // 供給する（emo-dpi-scaling task 4.2）。shell と balloon で別宣言ゆえ引き当てを取り違え
        // ないよう [`AuthorDpis`] へ束ねる（下の `attach_target` 呼び 2 箇所が `for_target` で引く）。
        shell_author_dpi,
        balloon_author_dpi,
    } = assets;
    let author_dpis = AuthorDpis {
        shell: shell_author_dpi,
        balloon: balloon_author_dpi,
    };
    let mut shells: Vec<_> = shells.into_iter().map(Some).collect();
    // 文字層へ渡すバルーン定義は各 scope 自身の [`BalloonScopeAssets::model`]（scope 別 2 層マージ
    // 済み・Req 2.1）。World／アトラスと**同一の資産 1 件から**取り出すため、ある scope のバルーンが
    // 別 scope の系列由来の定義で駆動される取り違えが構造的に起こり得ない（Req 4.1）。
    let mut balloons: Vec<_> = balloons.into_iter().map(Some).collect();

    let planned_count = plan.items.len();
    let mut attached_count = 0usize;

    for item in &plan.items {
        let scope = item.scope;

        // --- shell target: char_window → attach_target（EmoWorld を move）。初回 ShowSurface は
        //     発行しない（defect #5）: シェルは最初の `\s` cue まで非表示・target のみ生成する。 ---
        let Some(shell_window) = ghost_windows.char_window(scope as usize) else {
            error!(scope, "emo2 attach: char_window が無い（GhostWindows 不整合）→ この scope を skip");
            continue;
        };
        let Some(shell_assets) = shells.get_mut(item.shell_index).and_then(|s| s.take()) else {
            error!(
                scope,
                shell_index = item.shell_index,
                "emo2 attach: shell 資産の添字が空（二重消費？）→ skip"
            );
            continue;
        };
        let ScopeAssets {
            emo_world: shell_world,
            atlas: shell_atlas,
            ..
        } = shell_assets;

        if let Err(e) = wiring.presenter.attach_target(
            world,
            item.shell_target,
            shell_window,
            shell_world,
            shell_atlas,
            // author_dpi は attach 対象 target と同じ式で引く（`item.shell_target` を両方に書く）＝
            // shell/balloon の取り違えが 1 行の中で目視可能になる（両者 u16 で型は守ってくれない）。
            author_dpis.for_target(item, item.shell_target),
        ) {
            error!(scope, error = %e, "emo2 attach: シェル target の attach に失敗（log-first・継続）");
            continue;
        }
        // シェルは初回 ShowSurface を attach で発行しない（defect #5・2026-07-13 実機#5）。SSP 互換の
        // 既定は「シェル表示なし（surface -1）」であり、attach 時に surface0/surface10 を焼き付けると
        // ゴースト起動の一瞬に規定面がちらつく（実機#5 の欠陥）。初回シェル表示は、最初のさくら
        // スクリプト `\s[N]` cue が seriko→PresentBridge→drain 経路で運ぶ ShowSurface が駆動する
        // （起動時オンの bindgroup default は seriko 保持の static_binds が Show に載る）。上の
        // attach_target で target 自体は生成済みゆえ、後続の `\s`-driven ShowSurface はこの
        // shell_target へ適用できる（emo2 murasaki は `\s[1000]`／kero は `\s[通常]` を OnBoot で
        // 発行するため、talk 開始直後にシェルは表示される）。
        // シェル target の装着成功を計上（DD-12 の planned==attached 積極 assert 用・balloon と対で 1 scope）。
        attached_count += 1;

        // --- balloon target（同 scope の資産がある場合）: attach → 初回 ShowSurface（面0・default）
        //     → text_slot_view → register_actor_view ---
        let Some(balloon_index) = item.balloon_index else {
            warn!(scope, "emo2 attach: 同 scope の balloon 資産が無い（DD-12 balloon_index None）→ 文字層接続なし");
            continue;
        };
        let Some(balloon_window) = ghost_windows.balloon_window(scope as usize) else {
            warn!(scope, "emo2 attach: balloon_window が無い（GhostWindows 不整合）→ バルーン装着を skip");
            continue;
        };
        // 当該 scope の資産 1 件を take で消費する（World／アトラス／定義は同一資産から取り出す
        // ＝別 scope の系列由来の定義が混ざり得ない・Req 4.1）。
        let Some(BalloonScopeAssets {
            emo_world: balloon_world,
            atlas: balloon_atlas,
            model: balloon_model,
            ..
        }) = balloons.get_mut(balloon_index).and_then(|b| b.take())
        else {
            error!(
                scope,
                balloon_index,
                "emo2 attach: balloon 資産の添字が空（二重消費？）→ バルーン装着を skip"
            );
            continue;
        };
        if let Err(e) = wiring.presenter.attach_target(
            world,
            item.balloon_target,
            balloon_window,
            balloon_world,
            balloon_atlas,
            // shell 側と同型: attach 対象 target と同じ式（`item.balloon_target`）で引き当てる。
            author_dpis.for_target(item, item.balloon_target),
        ) {
            error!(scope, error = %e, "emo2 attach: バルーン target の attach に失敗（log-first・継続）");
            continue;
        }
        // バルーン初回表示は面 0・bind なし・pattern なし（DD-9・R4.1 の「初回サーフェス表示＝
        // バルーン枠表示」）。初回枠は SERIKO ループ非駆動ゆえ空 pattern＝拡張前と観測等価（R5.4）。
        wiring.presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: item.balloon_target,
                surface_id: 0,
                binds: areka_emo_compose::BindSet::default(),
                pattern: areka_emo_compose::PatternState::default(),
                reply: None,
            },
        );
        // apply は同期ゆえ同一フレームで text_slot_view が Some になるのが正常経路（DD-4）。
        // None（上流の遅延化）は接続せず次フレーム再試行へ委ねる（R4.2）。
        let view = wiring.presenter.text_slot_view(item.balloon_target);
        connect_balloon_text(
            &wiring.runtime,
            view,
            // 再追従（[`run_text_scale_phase`]）は**同一の写像**で actor を引く——
            // ここと式が食い違うと、再追従が別 actor を作って文字だけ旧 k のまま残る。
            ActorKey::from(scope.to_string()),
            &balloon_model,
        );
        // 文字層 k 再追従（D11-3・R8.1）の再利用源: **いま文字層へ渡したのと同一の**モデルを
        // scope キーで記憶する（借用→move の 1 値ゆえ二つの供給先が別値になり得ない——別値になると
        // 「装着時と再追従時で別定義」という静かな食い違いが生まれる・R4.1/4.2）。文字層スケール相
        // （[`run_text_scale_phase`]）はこれを再利用して binding を組み直す（再パースしない）。
        // 装着が `text_slot_view` None で次フレームへ委ねられた場合でもモデル自体は有効ゆえ、
        // 接続成否に関わらず記憶する（再追従は未登録 actor を静穏 skip する・7.1 の契約）。
        wiring.balloon_models.insert(scope, balloon_model);
    }

    info!(
        planned = planned_count,
        attached = attached_count,
        missing = plan.missing_assets.len(),
        unused = plan.unused_assets.len(),
        "emo2 attach: 装着計画を実行（planned＝計画件数・attached＝実装着件数）"
    );

    // ゲートを通過した attach 試行の完了。以降は no-op（高々 1 回）。
    wiring.attached = true;
}

/// バルーン文字層スロットの接続判断（R4.2 の None 分岐を headless に切り出した補助・DD-4）。
///
/// `text_slot_view` が `Some` なら [`TextLayerRuntime::register_actor_view`] で actor を登録し
/// `true` を返す。`None`（初回 `ShowSurface` 未合流＝上流の遅延化・Revalidation Trigger）なら
/// 登録せず `warn!` して `false` を返し、接続を次フレーム再試行へ委ねる（panic しない・R4.2）。
/// 登録判断を純結線として切り出すことで、GPU 不要の headless 単体テストが None 経路を檻に入れられる。
fn connect_balloon_text(
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    view: Option<TextSlotView>,
    actor: ActorKey,
    model: &BalloonModel,
) -> bool {
    match view {
        Some(view) => {
            runtime.borrow_mut().register_actor_view(actor, &view, model);
            true
        }
        None => {
            warn!(
                actor = %actor.as_str(),
                "emo2 attach: text_slot_view が None（初回 ShowSurface 未合流）→ 文字層接続を次フレームへ委ねる（R4.2）"
            );
            false
        }
    }
}

// ---------------------------------------------------------------------------
// DPI 追従フェーズ（areka-P0-emo-dpi-scaling task 4.2・design「areka / emo2_boot（DPI 追従
// フェーズ）> run_dpi_phase（frame.rs）」・Flow 2／Flow 3 手順 5・D8・Req3.1/4.1/4.2/4.3/4.4）
// ---------------------------------------------------------------------------

/// 装着時に各 target へ渡す原稿 DPI の対（shell 宣言＋balloon 宣言・D1・Req1.1）。
///
/// shell は `seriko.dpi`・balloon は `dpi` と**別宣言**であり、どちらも `u16` ゆえ取り違えても
/// コンパイルは通る——通ったまま「シェルだけバルーンの縮尺で描かれる」という静かな誤表示になる。
/// ゆえに呼び手に「どちらの `u16` か」を選ばせず、**装着対象 target の同一性**で引き当てる
/// （[`Self::for_target`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AuthorDpis {
    /// shell descript の `seriko.dpi`（無宣言・不正は上流 source.rs が 96 へ正規化済み）。
    shell: u16,
    /// balloon descript の `dpi`（同上）。
    balloon: u16,
}

impl AuthorDpis {
    /// 装着対象 `target` に対応する author_dpi を引く（shell target＝shell 宣言・balloon target＝
    /// balloon 宣言）。
    ///
    /// `item` の 2 target のいずれでもない値は到達＝結線バグゆえ `warn!`＋既定 96 へ縮退する
    /// （表示を失わない縮退・panic しない・log-first）。
    fn for_target(self, item: &PlannedAttach, target: TargetId) -> u16 {
        if target == item.shell_target {
            self.shell
        } else if target == item.balloon_target {
            self.balloon
        } else {
            warn!(
                ?target,
                scope = item.scope,
                "emo2 attach: 当該 scope の shell/balloon いずれの target でもない → author_dpi 既定 96 へ縮退"
            );
            96
        }
    }
}

/// ゴースト窓の種別（窓寸 reconcile の**反映口**の選択・`spawn.rs` の marker 由来）。
///
/// キャラ窓とバルーン窓は寸法変更時の位置の決め方が異なる（D8）——キャラ窓はアンカー辺へ
/// 釘付けされた接地点を保つため [`resize_window_to`]、バルーン窓は位置がキャラ窓追従で決まる
/// 従属量ゆえ [`resize_window_keep_position`]（同フレームで二重に位置が動かない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GhostWindowKind {
    /// キャラ窓（`CharWindowMarker`）→ アンカー保存リサイズ。
    Char,
    /// バルーン窓（`BalloonWindowMarker`）→ 位置維持リサイズ。
    Balloon,
}

/// 窓 entity の marker 照合結果（[`classify_ghost_window`] の 3 分類・純判断）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GhostWindowClass {
    /// ゴースト窓（scope＋種別が確定）。
    Ghost(usize, GhostWindowKind),
    /// ゴースト窓でない（`GhostWindowMarker` 系を持たない他の窓）＝本フェーズの対象外。
    NotGhost,
    /// char/balloon の marker が同居している（`spawn.rs` の排他付与に反する結線バグ）。
    Ambiguous,
}

/// 窓に付いた marker（scope 付き）から scope と種別を判定する純関数（GPU/World 不要）。
///
/// `spawn_ghost_windows` は 1 窓へ `CharWindowMarker`／`BalloonWindowMarker` の**どちらか一方**
/// だけを付ける。ゆえに両方あり＝結線バグ（[`GhostWindowClass::Ambiguous`]）、どちらも無し＝
/// ゴースト窓ではない（[`GhostWindowClass::NotGhost`]）として呼び手が縮退できるよう 3 分類で返す。
fn classify_ghost_window(
    char_scope: Option<usize>,
    balloon_scope: Option<usize>,
) -> GhostWindowClass {
    match (char_scope, balloon_scope) {
        (Some(scope), None) => GhostWindowClass::Ghost(scope, GhostWindowKind::Char),
        (None, Some(scope)) => GhostWindowClass::Ghost(scope, GhostWindowKind::Balloon),
        (None, None) => GhostWindowClass::NotGhost,
        (Some(_), Some(_)) => GhostWindowClass::Ambiguous,
    }
}

/// 報告された新物理寸を窓 client へ反映する（D8 の振り分け・二経路の共通末端）。
///
/// 種別ごとの反映口（char＝[`resize_window_to`]／balloon＝[`resize_window_keep_position`]）へ
/// 振り分ける。戻り値は**書込が起きたか**であり、`false` は失敗とは限らない——同寸のべき等
/// skip（振動しない・Req4.2）も `false` を返す（ゆえに `false` を error として鳴らさない。
/// 実失敗＝`WindowHandle` 未付与等は各反映口が `warn!` 済み）。
///
/// 物理寸は `u32`（表示バッファ外形）で報告されるが窓寸は `i32` 通貨ゆえ、ここで変換し
/// 超過・0 を弾く（log-first・panic しない・Req3.4 と同流儀の二重防波堤）。
///
/// # `route` 引数（Req 1.2・design D13・task 1.4）
///
/// **本関数は共通末端であって経路ではない**。呼出元は 2 つあり、両者は消失診断の上で
/// 別物である:
///
/// - [`dpi_phase_with`]（`Changed<DPI>` エッジ由来）→ [`PlacementRoute::DpiReproject`]
/// - [`reconcile_reported_sizes`]（drain 相の報告回収・**初回表示の k₀ 補正を含み**
///   `Changed<DPI>` 非依存）→ [`PlacementRoute::ReportedSizeReconcile`]
///
/// ゆえに route を本関数の内部で決め打ちしてはならない——決め打つと DPI 変化ゼロの起動でも
/// 「DPI 由来」の偽レコードが毎回出て、要件 1.9 の受理回数突合（セッション②＝ドラッグ禁止・
/// OS 側 DPI 変更のみ）が汚染される。バルーン窓は位置据置きリサイズ
/// （[`PlacementRoute::KeepPositionResize`]）へ落ちるため route を消費しない。
fn reconcile_window_size(
    world: &mut World,
    window: Entity,
    kind: GhostWindowKind,
    new_size: (u32, u32),
    route: PlacementRoute,
) -> bool {
    let (Ok(w), Ok(h)) = (i32::try_from(new_size.0), i32::try_from(new_size.1)) else {
        error!(
            entity = ?window,
            ?kind,
            ?new_size,
            "dpi reconcile: 報告された物理寸が i32 域を超える → 窓寸を変えない（前寸維持・log-first）"
        );
        return false;
    };
    if w == 0 || h == 0 {
        warn!(
            entity = ?window,
            ?kind,
            ?new_size,
            "dpi reconcile: 報告された物理寸に 0 軸がある → 窓寸を変えない（前寸維持）"
        );
        return false;
    }
    let new_size = SizePx { w, h };
    match kind {
        // キャラ窓: アンカー射影 T を再適用して接地点（下端中央）を保つ。
        // 経路タグは呼出元が渡した route を透過させる（本関数は共通末端であって経路ではない
        // ＝DPI 相と drain 相を 1 語で名乗らせない・Req 1.2／D13）。
        GhostWindowKind::Char => resize_window_to(world, window, new_size, route),
        // バルーン窓: 位置は追従で決まる従属量ゆえ据え置き、寸だけ差し替える
        // （経路語彙は関数名そのもの＝KeepPositionResize ゆえ route を消費しない）。
        GhostWindowKind::Balloon => resize_window_keep_position(world, window, new_size),
    }
}

/// 再スケール報告の供給源（本番実装は [`EmoPresenter`]）。
///
/// frame 側の結線が presenter へ求めるのは 2 つの報告だけである——(1) `Changed<DPI>` エッジで
/// 駆動する再表示の結果（[`EmoPresenter::refresh_scale`]）、(2) 表示成立点の状態照合が積んだ
/// 未消費の窓寸要求（[`EmoPresenter::take_pending_resize`]）。両者の**責任分界**（再表示が
/// 成立したら (1) が要求を自ら消費し、ゲート不成立なら一切触れず (2) が拾う）は presenter 側の
/// 契約であり、そちらの檻は emo-present in-crate が所有する。
///
/// この最小トレイトは、**frame 側の結線**（毎フレーム両経路を呼ぶ・`Some` のみ reconcile する・
/// 種別で反映口を振り分ける）を GPU 無しで決定論の檻へ入れるためのシームである（D9 の
/// 振り分け基準 (a)＝判断分岐は in-crate 純テスト、GPU readback は emo-present 別プロセス）。
trait ScaleReportSource {
    /// 窓 DPI から k を再導出し、再表示が成立して物理寸が変わったならその新物理寸を返す。
    fn refresh_scale_report(&mut self, world: &mut World, target: TargetId) -> Option<(u32, u32)>;
    /// 表示成立点の状態照合が積んだ未消費の窓寸 reconcile 要求を取り出す（取り出しで消える）。
    fn take_scale_report(&mut self, target: TargetId) -> Option<(u32, u32)>;
}

impl ScaleReportSource for EmoPresenter {
    fn refresh_scale_report(&mut self, world: &mut World, target: TargetId) -> Option<(u32, u32)> {
        self.refresh_scale(world, target)
    }

    fn take_scale_report(&mut self, target: TargetId) -> Option<(u32, u32)> {
        self.take_pending_resize(target)
    }
}

/// `Changed<DPI>` の窓と、その窓の scope/種別を引くための marker を一度に取る query 型。
///
/// `Changed<DPI>` フィルタは `DPI` component を持つ窓のみへ効く（本番は窓生成時に必ず付与され、
/// `WM_DPICHANGED` と生成時の `GetDpiForWindow` 実値補正の双方で変化が発火する）。marker は
/// `Option` で取り、ゴースト窓でない窓（どちらも `None`）を静穏に読み飛ばす。
type DpiChangedQuery = Query<
    'static,
    'static,
    (
        Entity,
        Option<&'static CharWindowMarker>,
        Option<&'static BalloonWindowMarker>,
    ),
    Changed<DPI>,
>;

/// DPI フェーズの本体（報告源を抽象化した中核・[`run_dpi_phase`] が本番の presenter を渡す）。
///
/// 永続 `state` で `Changed<DPI>` を観測し（`anchor_changed_system` 先例と同じ流儀）、変化した
/// 各ゴースト窓について対応 target の [`ScaleReportSource::refresh_scale_report`] を呼ぶ。
///
/// # 位置の権威と寸の権威の分離（S2 是正・design D7・Req 4.1／4.2／4.5／4.6）
///
/// **窓寸**を合わせるのは報告が `Some(新物理寸)` のときだけ（[`reconcile_window_size`]）だが、
/// **位置**はその成否に条件付けない——`None` のキャラ窓は
/// [`reproject_char_window_at_current_size`] で**現寸のまま**射影 T を一度通す。位置の再射影が
/// [`resize_window_to`] の内部にしかない以上、`Some` ゲートの下流に置くと「再導出結果が得られ
/// ない経路で位置の再射影ごと欠落する」（診断レポート §1.2 の S2）。正常系は同寸・同 work area
/// ゆえべき等 skip で書込ゼロ＝ Req 4.5 はそのまま成立する。バルーン窓の `None` は位置据置き
/// （位置は従属量ゆえキャラ窓確定後の追従が随伴させる）。
///
/// # べき等・churn なし（Flow 2 キー決定 (b)）
///
/// 初回 run は `SystemState::new` の仕様で全窓へマッチするが、k 差分が無ければ報告は `None`
/// （presenter のゲート）であり、仮に報告があっても同寸なら反映口がべき等 skip する。ゆえに
/// 初回全マッチは無害に吸収される。`Changed<DPI>` が無いフレームは query が空＝実質 no-op。
///
/// # 失敗（Req4.4）
///
/// 再導出・再表示の失敗は presenter が `error!`＋`None` で前 k・前表示を維持する。本関数は
/// `None` で**窓寸を一切触らない**（前寸維持）——位置だけは上記のとおり現寸で射影を通す
/// （寸が古いまま位置だけ正す瞬間は同一フレームの drain 相 reconcile が閉じる・D7）。
/// panic しない。
fn dpi_phase_with<S: ScaleReportSource>(
    source: &mut S,
    state: &mut Option<SystemState<DpiChangedQuery>>,
    world: &mut World,
) {
    // 永続シーム: run を跨いで同一 SystemState を使い回し `last_run` を保つ（毎 run 新規生成は
    // `last_run` が 0 のままとなり全窓へ誤マッチし続ける＝毎フレーム再表示の churn）。
    let state = state.get_or_insert_with(|| SystemState::new(world));
    // 変化窓を collect して World の不変借用を即解放してから `&mut World` のループへ入る
    // （`anchor_changed_system` と同じ collect→release→&mut ループ）。
    let changed: Vec<(Entity, Option<usize>, Option<usize>)> = state
        .get(world)
        .iter()
        .map(|(entity, char_marker, balloon_marker)| {
            (
                entity,
                char_marker.map(|m| m.scope),
                balloon_marker.map(|m| m.scope),
            )
        })
        .collect();

    for (window, char_scope, balloon_scope) in changed {
        let (scope, kind) = match classify_ghost_window(char_scope, balloon_scope) {
            GhostWindowClass::Ghost(scope, kind) => (scope, kind),
            // ゴースト窓でない窓の DPI 変化は本フェーズの対象外（正常・静穏に読み飛ばす）。
            GhostWindowClass::NotGhost => continue,
            GhostWindowClass::Ambiguous => {
                error!(
                    entity = ?window,
                    "dpi: char/balloon marker が同居する窓（spawn の排他付与に反する）→ 再スケールを skip"
                );
                continue;
            }
        };
        // target 採番（DD-3: shell=2*scope／balloon=2*scope+1）は u32 域。収まらない scope は
        // 如何なる target とも対応しない（plan_attachments の usize→u32 境界と同じ扱い）。
        let Ok(scope) = u32::try_from(scope) else {
            error!(
                entity = ?window,
                scope,
                "dpi: scope が u32 に収まらず target を採番できない → 再スケールを skip"
            );
            continue;
        };
        let target = match kind {
            GhostWindowKind::Char => shell_target(scope),
            GhostWindowKind::Balloon => balloon_target(scope),
        };
        // 再導出→（差分があれば）再表示。`None` は「**窓寸**を触らない」が正しい（前表示・前寸の
        // 維持・Req4.4）。なお `None` は k 不変とは同義でない——不可視・未表示・失敗に加え、**k は
        // 変わったが丸め後の物理寸が同じ**場合も `None` である（`refresh_scale` の doc が明記）。
        // ゆえに文字層 k 追従の判断材料にはこの戻り値を使わない（[`run_text_scale_phase`] を参照）。
        //
        // **位置**は寸の再導出結果に条件付けない（S2 是正・D7・下の `None` 腕）。
        match source.refresh_scale_report(world, target) {
            // 経路タグ: 本フェーズは `Changed<DPI>` エッジ駆動＝真に DPI 由来（Req 1.2・D13）。
            Some(new_size) => {
                reconcile_window_size(world, window, kind, new_size, PlacementRoute::DpiReproject);
            }
            // 再導出結果なし: 寸は触らないが、**位置は現寸のまま射影 T を一度通す**。
            // バルーン窓は位置据置きのまま（位置は従属量ゆえ、キャラ窓確定後の
            // [`follow_balloon`]＝`resize_window_to` 手順 6/7 が随伴させる）。
            None => match kind {
                GhostWindowKind::Char => {
                    reproject_char_window_at_current_size(world, window);
                }
                GhostWindowKind::Balloon => {}
            },
        }
    }
}

/// 窓寸の再導出結果が得られなかったキャラ窓を、**現在の寸のまま**射影 T へ通す（S2 是正・
/// design D7・Req 4.1／4.2／4.5／4.6・診断レポート §1.2）。
///
/// # なぜ必要か（位置の権威と寸の権威の分離）
///
/// 位置の再射影は [`resize_window_to`] の**内部**にしかないため、`refresh_scale_report` の
/// `Some` ゲートの下流に置くと「再導出結果が得られない経路では位置の再射影ごと欠落する」。
/// `None` を返す経路には**不可視**・**未表示**（いずれも Req 4.6 が名指しで扱う状況）と
/// 「k は変わったが丸め後の物理寸が同じ」（正常系で日常的に起こる）が含まれ、いずれも
/// **窓の DPI は変わっている＝接地すべき work area が変わっている**。ゆえに寸の成否に
/// 関わらず射影を一度通す。
///
/// # Req 4.5（現状維持）との関係——正常系は書込ゼロで抜ける
///
/// 現寸をそのまま渡すので手順 3b（下端中央の付け替え）は恒等、[`project_anchor`] が Y を
/// **変化後の** work area 下端から再導出する。同寸・同 work area なら導出値が現在値と一致し
/// [`resize_window_to`] のべき等 skip が書込ゼロで抜ける。書込が起きるのは**現位置が接地点
/// 規約に違反しているとき**だけであり、それは Req 4.1／4.2 が要求する保全そのものである
/// （design「dpi_phase 位置/寸分離 > Risks / Req 4.5 との整合」＝矛盾ではなく優先順位）。
///
/// # 縮退（log-first・要件 6.2/6.3 の区別を踏襲）
///
/// 窓寸が引けない場合は現状維持のまま打ち切る。水準は 2 分される（[`reconcile_reported_sizes`]
/// と同じ区別・混ぜると終了時ログの良性ノイズが本物の異常を埋める）:
///
/// - **entity 破棄済み**: 終了処理の**正常終了系**ゆえ `debug!`（[`DESPAWNED_SKIP_TAG`]）。
/// - **実在するが `WindowPos.size` 不在**（窓生成前）: 真の異常ゆえ `warn!`。
///
/// 戻り値は**窓へ書込が起きたか**（`false` はべき等 skip・縮退の双方を含み、失敗とは限らない
/// ＝[`reconcile_window_size`] と同じ流儀）。panic しない。
fn reproject_char_window_at_current_size(world: &mut World, window: Entity) -> bool {
    let Some(current) = world.get::<WindowPos>(window).and_then(|wp| wp.size) else {
        if world.get_entity(window).is_err() {
            debug!(
                entity = ?window,
                "{DESPAWNED_SKIP_TAG} dpi reproject: 窓 entity が破棄済み（despawn）→ 位置再射影を正常系として打ち切り"
            );
        } else {
            warn!(
                entity = ?window,
                "dpi reproject: WindowPos.size 未確定（窓生成前）のため現寸を読めず、位置を再射影せず現状維持"
            );
        }
        return false;
    };
    resize_window_to(
        world,
        window,
        SizePx {
            w: current.width,
            h: current.height,
        },
        PlacementRoute::DpiReproject,
    )
}

/// DPI 追従フェーズ（design「run_dpi_phase（frame.rs）」・D8・Req3.1/4.1/4.2/4.3）。
///
/// `Changed<DPI>` の窓を永続 [`SystemState`]（`anchor_changed_system` 先例）で観測し、当該窓に
/// 対応する target の `refresh_scale` を呼ぶ。`Some(新物理寸)` なら char 窓は
/// [`resize_window_to`]（アンカー保存）・balloon 窓は [`resize_window_keep_position`] で窓 client を
/// 新物理寸へ reconcile する——**同一フレーム・同一 UI スレッド呼出**で完結するため、フェーズ
/// 終了時点で照会値（`applied_scale`）・表示寸・窓 client が揃う（Req4.2）。
///
/// 進行中の talk 再生・SERIKO ループは presenter の**外**に状態を持つため、再表示はキャッシュ
/// ミス 1 回のコストで済み挙動を失わない（Req4.3）。本フェーズは target 状態を一切リセットしない。
///
/// # 窓寸 reconcile の第 2 経路
///
/// エッジ（`Changed<DPI>`）観測は「再表示のトリガ」に徹する。窓寸の整合そのものは**表示が成立
/// したという状態**に紐づき、[`run_drain_phase`] 末尾の [`reconcile_reported_sizes`] が同一
/// フレーム内で拾う（初回表示の k₀ 補正＝Flow 3 手順 5 はこちらの経路で landing する）。両者は
/// presenter の消費規約により二重にも取りこぼしにもならない。
///
/// # 文字層の k 追従は本フェーズが担わない（task 7.2・D11-4・Req8）
///
/// バルーンの**文字**も新 k へ追従させる必要があるが、その反映点は本フェーズではなく
/// [`run_text_scale_phase`] である——適用 k の更新点は 1 フレームに 2 つ（本フェーズの
/// `refresh_scale` と drain 相の `apply_show`）あり、本フェーズの戻り値だけを見ると後者を取り
/// こぼすためである（詳細は [`run_text_scale_phase`] の doc）。
pub fn run_dpi_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    // presenter（報告源）と dpi_state（永続観測器）は互いに素なフィールドゆえ同時に借りられる。
    let Emo2Wiring {
        presenter,
        dpi_state,
        ..
    } = wiring;
    dpi_phase_with(presenter, dpi_state, world);
}

// ---------------------------------------------------------------------------
// 文字層 k 追従フェーズ（areka-P0-emo-dpi-scaling task 7.2・design D11-3/D11-4・Req8.1/8.5/8.6）
// ---------------------------------------------------------------------------

/// 文字層 k 追従フェーズ（毎フレーム・R8.1/8.5/8.6・design D11-4）: 装着済み balloon scope の
/// 文字層 binding を presenter の**現適用 k** へ合わせ直す。戻り値は実際に binding を再構築した
/// scope（昇順・観測用。本番＝[`emo2_frame_system`] は捨てる）。
///
/// バルーンの**文字**は emo-text の binding（装着時の k を焼き付ける）に載るため、窓とバルーン画像
/// だけを再スケールすると文字だけが旧 k の寸法に取り残される（6.5 一次実走で実測した欠陥）。本
/// フェーズはその取り残しを構造的に消す。
///
/// # なぜ「イベント駆動」ではなく毎フレーム走査なのか（D11-4 の意図＝binding 変化の検出）
///
/// 素朴には「[`run_dpi_phase`] の `refresh_scale` が `Some` を返した balloon 窓へ伝搬する」と書け
/// るが、**`Some` と「適用 k が変わった」は同値ではない**——`refresh_scale` の doc が明記するとおり
/// 次の 2 つで乖離する:
///
/// - **不可視のとき**: `refresh_scale` は再表示せず `applied` も更新せずに `None` を返す。適用 k は
///   その後の `Show`（`apply_show`＝drain 相）で新 k へ跳ぶ——エッジは既に消費済みで二度と来ない
///   （`\b[-1]`→`\b[0]` は本番の通常列であり、バルーンは大半の時間が不可視である）。
/// - **k は変わったが丸め後の物理寸が同じとき**: `refresh_scale` は再表示に成功しても
///   `take_pending_resize` が `None` ゆえ `None` を返す。文字層の供給面は
///   `ceil(validrect 寸 × k)`（AC 8.2）と別の丸めで決まるため、こちらは寸が変わり得る。
///
/// ゆえに検出点は「**presenter の現状態から組み直した文字層 binding が、当該 actor の現 binding と
/// 食い違っているか**」であり、それを判定できる唯一の権威は
/// [`TextLayerRuntime::refresh_actor_binding`] である。本フェーズは判定を自前で複製せず（第 2 の
/// ガードは本家と乖離し得る）、毎フレーム [`TextLayerRuntime::refresh_actor_scale`] へ委ねる。
///
/// あちらの判定キーは **binding 全体（k・物理寸・image 原寸・slot・window）と、その image 原寸で
/// モデルから解き直した `ResolvedBalloonText`（文字描画領域を含む）の連言**であり、すべて同値なら
/// **再構築せず `false`** を返す（churn ガード・R4.5/R8.5）。未登録 actor も `false`（装着は
/// `register_actor_view` の領分）。**k が同値でも面実寸や当該 scope の `validrect` が違えば再構築
/// する**——k の同値のみを根拠に省略しない（R4.4。scope 別バルーン定義が当事者であり、旧契約
/// 「同値 k なら再構築しない」では相方側の領域変化を取りこぼす）。費用は balloon 1 枚あたり
/// `ResolvedBalloonText` の再解決 1 回と、binding／解決済み領域の 2 構造体比較。
///
/// # 呼ぶ位置（[`emo2_frame_system`] 内）
///
/// 適用 k の更新点は 1 フレームに 2 つ——[`run_dpi_phase`] の `refresh_scale` と
/// [`run_drain_phase`] の `apply_show`。本フェーズは**両者の下流**かつ [`run_text_phase`]
/// （`present_frame`）の**上流**に置く。こうすると、どちらの経路で k が跳ねても同一フレーム内で
/// binding が組み直され、その直後の描画が新 k の物理寸で走る（1 フレームの旧寸残りが生じない）。
///
/// # 縮退（R8.6・log-first だが log spam にしない）
///
/// `text_slot_view` が `None`（初回 `ShowSurface` が成立していない＝表示未確立）なら再追従できず
/// skip する。毎フレーム走査ゆえ素朴に `warn!` すると毎フレーム鳴るため、**scope ごとに一度だけ**
/// 警告し（`text_scale_warned`・emo-text の `unresolved_warned` と同型のエッジガード）、view が
/// 取れるようになった時点で再武装する（再度落ちれば再び 1 回鳴る）。なお `Hide` は
/// `text_slot_view` を `None` にしない（`apply_hide` は mount／chain／`applied`／`native_size` を
/// 保持する）ため、**不可視は本縮退経路に落ちない**——不可視の間は判定キーが同値のまま no-op が
/// 続き、`Show` で `applied` が跳ねた次の走査が再追従する。
///
/// [`BalloonModel`] は attach 時に記憶した per-scope の同一モデルを再利用する（再パースしない・
/// D11-3）。actor は attach と同一写像 `ActorKey::from(scope.to_string())`。shell target は emo2 で
/// 文字スロットを持たないため走査対象に入らない（`balloon_models` が balloon 装着 scope のみを持つ）。
/// panic しない。
pub fn run_text_scale_phase(wiring: &mut Emo2Wiring) -> Vec<u32> {
    // presenter（view 供給）／runtime（適用先）／balloon_models（再利用モデル）／warn ガードは
    // 互いに素なフィールドゆえ同時に借りられる。
    let Emo2Wiring {
        presenter,
        runtime,
        balloon_models,
        text_scale_warned,
        ..
    } = wiring;

    let mut refreshed = Vec::new();
    // BTreeMap ではなく HashMap ゆえ列挙順は不定。観測（戻り値）と warn 順を決定論にするため昇順化する。
    let mut scopes: Vec<u32> = balloon_models.keys().copied().collect();
    scopes.sort_unstable();

    for scope in scopes {
        let target = balloon_target(scope);
        // actor 引き当ては attach（`run_attach_phase` の `connect_balloon_text` 呼び）と**同一の写像**。
        // 別式で組むと存在しない actor を指し、7.1 の未登録 skip で静かに何も起きなくなる。
        let actor = ActorKey::from(scope.to_string());
        let Some(view) = presenter.text_slot_view(target) else {
            // 表示未確立（初回 ShowSurface が成立していない）。毎フレーム走査ゆえ scope ごとに 1 回だけ鳴らす。
            if text_scale_warned.insert(scope) {
                warn!(
                    scope,
                    ?target,
                    actor = %actor.as_str(),
                    "text-scale: text_slot_view が None（表示未確立）→ 文字層 k 追従を skip し次機会へ委ねる（本 scope の警告は復帰まで抑止・R8.6）"
                );
            }
            continue;
        };
        // 復帰＝次に落ちたときは再び 1 回鳴らす（エッジの再武装）。
        text_scale_warned.remove(&scope);
        // 判定（k 変化・未登録）は 7.1 の権威へ委ねる（本フェーズは第 2 のガードを持たない・R8.5）。
        let model = &balloon_models[&scope];
        if runtime.borrow_mut().refresh_actor_scale(&actor, &view, model) {
            refreshed.push(scope);
        }
    }
    refreshed
}

/// 窓寸 reconcile の第 2 経路（状態照合・design Flow 2 キー決定 (d)／Flow 3 手順 5）。
///
/// [`GhostWindows`] の各 scope について shell／balloon 両 target の
/// [`ScaleReportSource::take_scale_report`] を引き、`Some(新物理寸)` を
/// [`reconcile_window_size`] で窓 client へ反映する（char＝アンカー保存／balloon＝位置維持）。
///
/// 報告は「表示が成立して物理寸が前回適用寸から変わった（**初回表示を含む**）」ことを表す状態で
/// あり、`Changed<DPI>` エッジの消費順序に依存しない。ゆえに (a) エッジが初回表示より前に消費
/// されても k₀ と実窓 DPI の差分は残置されず、(b) 既に [`run_dpi_phase`] が再表示して報告を消費
/// 済みなら取り出しは `None` となり二重に窓を書かない。
///
/// [`GhostWindows`] 未挿入（窓生成前）は no-op。窓 entity が引けない scope は `warn!`＋skip
/// （報告は既に取り出し済み＝次フレームへ持ち越さない——窓が無い以上反映先が無い）。panic しない。
///
/// # 破棄済み窓の打ち切り（要件 6.2/6.3・design D8 消費側）
///
/// 「登録は在るが**指す先の entity が既に despawn 済み**」は終了処理の**正常系**であり、
/// `debug!`（[`DESPAWNED_SKIP_TAG`]）で当該 target を打ち切って**他 scope の処理を継続**する
/// （警告以上を出さない＝要件 6.2）。上段の「登録が無い」`warn!` とは別事象である。
fn reconcile_reported_sizes<S: ScaleReportSource>(source: &mut S, world: &mut World) {
    // GhostWindows は小さな Entity 写像（Clone）。target/窓の解決へ world の不変借用を跨がせない。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>().cloned() else {
        return;
    };
    for scope in ghost_windows.scopes() {
        let Ok(scope32) = u32::try_from(scope) else {
            error!(
                scope,
                "dpi reconcile: scope が u32 に収まらず target を採番できない → skip"
            );
            continue;
        };
        for (target, window, kind) in [
            (
                shell_target(scope32),
                ghost_windows.char_window(scope),
                GhostWindowKind::Char,
            ),
            (
                balloon_target(scope32),
                ghost_windows.balloon_window(scope),
                GhostWindowKind::Balloon,
            ),
        ] {
            // 報告が無い（＝物理寸が変わっていない／未表示／既に消費済み）なら何もしない。
            let Some(new_size) = source.take_scale_report(target) else {
                continue;
            };
            let Some(window) = window else {
                warn!(
                    scope,
                    ?target,
                    ?new_size,
                    "dpi reconcile: 窓 entity が無い（GhostWindows 不整合）→ 反映先が無く skip"
                );
                continue;
            };
            // 存在確認（要件 6.2/6.3・design D8 消費側）: レジストリが指す窓が既に
            // despawn 済み（終了処理でゴースト窓が破棄された後のフレーム）なら、**正常終了系**
            // として debug で打ち切り、**他の scope／target の処理は続ける**。報告は上で
            // 取り出し済みのまま持ち越さない（窓が無い以上、次フレームでも反映先は無い）。
            // 上の `None` 腕（レジストリ不整合＝warn）とは別物である——あちらは「登録が無い」、
            // こちらは「登録はあるが指す先が消えた」で、後者だけが終了処理の正常系。
            if world.get_entity(window).is_err() {
                debug!(
                    scope,
                    ?target,
                    entity = ?window,
                    "{DESPAWNED_SKIP_TAG} dpi reconcile: 窓 entity が破棄済み（despawn）→ 本 target を正常系として打ち切り（他 scope は継続）"
                );
                continue;
            }
            // 経路タグ: 本経路は「表示が成立して物理寸が変わった」状態に紐づき `Changed<DPI>`
            // に**依存しない**（初回表示の k₀ 補正もここで landing する）。DPI 由来と名乗らせ
            // ないため DpiReproject とは別語を貼る（Req 1.2・D13）。
            reconcile_window_size(
                world,
                window,
                kind,
                new_size,
                PlacementRoute::ReportedSizeReconcile,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// drain・text フェーズ＋排他 system（tasks.md task 4.2・design「UI 毎フレーム結線 / frame」の
// Responsibilities フェーズ②（drain）／③（text）・Service Interface・DD-1）
// ---------------------------------------------------------------------------

/// フェーズ②（drain・design「フェーズ②（drain）」・DD-1）: attach 完了後のみ受信済み
/// `PresentCommand` を FIFO で全件 `presenter.apply` へ適用する。
///
/// attach 前はチャネルが保留バッファを兼ねる（取りこぼしなし・FIFO）ため **`attached` が立つまで
/// drain しない**。装着後は [`Receiver::try_iter`] で**現時点でキュー済みの指令を非ブロックで
/// FIFO 全件**取り出し、到着順に `presenter.apply(world, cmd)` する。`apply` は `()` を返し、失敗は
/// `cmd.reply`（本経路は常に `None`＝撃ちっぱなし）経由で、未装着等の異常は presenter 内部で
/// `error!` 済み（log-first）。本フェーズは panic しない（`SurfaceOutput`→UI の非ブロック配送契約）。
///
/// drain は attach 後にのみ走るため `TargetNotAttached` は原理上発生しない（発生＝結線バグとして
/// presenter が `error!`・design「適用時（UI）」）。`try_iter` はチャネルが空になるか送信端が全て
/// drop されると尽きる（ブロックしない）。
pub fn run_drain_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    // attach 前はチャネルが保留バッファを兼ねる（取りこぼしなし・FIFO）。装着後のみ drain する（DD-1）。
    if !wiring.attached {
        return;
    }
    // try_iter: 現時点でキュー済みの指令を非ブロックで FIFO 全件取り出す（空・全送信端 drop で尽きる）。
    // wiring.rx（受信端＝shared 借用）と wiring.presenter（mut 借用）は互いに素なフィールドゆえ両立する。
    for cmd in wiring.rx.try_iter() {
        // apply は () を返し、失敗は cmd.reply（本経路は常に None）経由。未装着等の異常は presenter 内部で
        // error! 済み（log-first）。撃ちっぱなしの非ブロック配送契約ゆえ本フェーズは panic しない。
        wiring.presenter.apply(world, cmd);
    }
    // 窓寸 reconcile の第 2 経路（emo-dpi-scaling task 4.2・design Flow 2 キー決定 (d)／Flow 3 手順 5）:
    // 本フレームの全 apply が済んだ**後**に、表示成立点の状態照合が積んだ未消費の窓寸要求を取り出して
    // 窓 client へ反映する（同一フレーム内完結・エッジ消費順序に依存しない）。attach 相の初回
    // ShowSurface が積む k₀ 補正もここで landing する。
    reconcile_reported_sizes(&mut wiring.presenter, world);
}

/// move drain フェーズ（`\![move]` の末端結線・design「frame 相で drain→`apply_move_directive`」・
/// R5.1/5.3/5.5/R6・task 9.2）: talk スレッド（`MoveCueSink`）から mpsc で届いた [`MoveDirective`]
/// を非ブロックで FIFO 全件 drain し、UI スレッド上で [`apply_move_directive`] へ適用する。
///
/// `PresentBridge`（[`run_drain_phase`]）と同型の跨ぎパターンだが、ゲートは `attached`（GPU）でなく
/// **`GhostWindows` の存在**である——move は GPU 表示層でなくキャラ窓 entity（`GhostWindows` が spawn 時
/// に生成）へ作用するため、GPU attach を待つ必要がない。`GhostWindows` 未挿入の間はチャネルが保留
/// バッファを兼ね（[`Receiver::try_iter`] を呼ばず取りこぼさない）、窓が生成された最初のフレームで
/// 一括適用する（OnFirstBoot の位置調整を早期に取りこぼさないための buffering・present drain の
/// 「attach 前は保留」と同じ意図）。
///
/// 各 directive の適用は [`apply_move_directive`] が完結させる: 非スコープ基準・窓/`WindowPos` 不在・
/// 座標算出不能はいずれも同関数内で `warn!`＋`false`（log-first・非 panic・R5.5）ゆえ、本フェーズは
/// 戻り値を捨てて次 directive へ進む（1 件の縮退が他 directive・talk を巻き込まない）。`try_iter` は
/// チャネルが空か全送信端 drop で尽きる（ブロックしない・empty/disconnected でも panic しない）。
pub fn run_move_drain_phase(wiring: &Emo2Wiring, world: &mut World) {
    // GhostWindows 未挿入の間はチャネルが保留バッファを兼ねる（try_iter を呼ばず取りこぼさない）。
    // 窓生成後の最初のフレームで一括適用する（OnFirstBoot 移動の早期取りこぼし防止・present drain と同意図）。
    if world.get_resource::<GhostWindows>().is_none() {
        return;
    }
    // try_iter: 現時点でキュー済みの MoveDirective を非ブロックで FIFO 全件取り出す（空・全送信端 drop で尽きる）。
    // wiring.move_rx（shared 借用）と world（mut 借用・別オブジェクト）は互いに素ゆえ両立する。
    for directive in wiring.move_rx.try_iter() {
        // 適用の全縮退（非スコープ基準・窓不在・算出不能）は apply_move_directive 内で warn!＋false 済み
        // （log-first・R5.5）。戻り値は捨てて次 directive へ進む（1 件の縮退で talk を殺さない・非 panic）。
        apply_move_directive(world, &directive);
    }
}

// ---------------------------------------------------------------------------
// resnap シーム（tasks.md task 3.2・design「統合シーム（emo2_boot frame.rs）>
// resnap_shell_targets / resnap_from_sizes」・Req1.3/3.1/3.2/4.1/4.3/4.5・DD-2/DD-5）
// ---------------------------------------------------------------------------

/// 合成寸法列を受け、shell サーフェス寸が変わった scope の char 窓のみ [`resize_window_to`] を
/// 駆動する純粋判定部（headless テスト対象・GPU 不要・design「resnap_from_sizes」・
/// Req1.3/3.1/3.4/4.5）。
///
/// [`GhostWindows`] Resource を world から取得（未挿入は no-op＝Preconditions）。各
/// `(scope, shown_size)` について:
/// - `char_window(scope)` が `None`（未知 scope）→ skip（再適用対象の char 窓が無い）。
/// - **非正寸**（`w <= 0 || h <= 0`）→ skip（Req3.4 の防御・[`resize_window_to`] と二重防波堤）。
/// - char 窓 `WindowPos.size` と `SizeI::new(w, h)` が**異なるときのみ** [`resize_window_to`] を
///   呼ぶ（同寸は no-op＝冗長駆動回避・Req3.1 べき等）。
///
/// **balloon 窓には一切触れない**（scope→`char_window` 写像のみ・Req4.5/DD-5）。判定・反映は
/// World 操作に閉じ GPU を要しない（GPU 結合は薄い [`resnap_shell_targets`] が担う）。
fn resnap_from_sizes(world: &mut World, sizes: impl Iterator<Item = (usize, SizePx)>) {
    // GhostWindows（scope→窓 entity の正本）。未挿入は no-op（Preconditions・Req4.5）。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return;
    };
    // scope→char_window を先に解決して collect し、world の不変借用を後段の &mut ループへ跨がせない
    // （借用衝突回避）。未知 scope・非正寸はここで弾く（Req3.4・二重防波堤）。
    let mut targets: Vec<(Entity, SizePx)> = Vec::new();
    for (scope, shown_size) in sizes {
        let Some(char_window) = ghost_windows.char_window(scope) else {
            // 未知 scope（GhostWindows に無い）→ skip（char 窓が無ければ再適用対象なし）。
            continue;
        };
        if shown_size.w <= 0 || shown_size.h <= 0 {
            debug!(scope, ?shown_size, "resnap: 非正寸のため skip（Req3.4・二重防波堤）");
            continue;
        }
        targets.push((char_window, shown_size));
    }
    // 反映: char 窓 WindowPos.size と異なるときのみ resize_window_to を駆動（同寸は非発火・Req3.1）。
    for (char_window, shown_size) in targets {
        let current = world.get::<WindowPos>(char_window).and_then(|wp| wp.size);
        if current == Some(SizeI::new(shown_size.w, shown_size.h)) {
            // 同寸＝冗長駆動を避ける（Req3.1 べき等・正常系ゆえ静穏に skip）。
            continue;
        }
        // 異寸のみ: 新寸で T 再適用→一度書き→随伴（resize_window_to が単一ライター・Req1.3）。
        // 経路タグは Resnap（毎フレーム再スナップ・Req 1.2／task 1.4）。
        resize_window_to(world, char_window, shown_size, PlacementRoute::Resnap);
    }
}

/// drain 後に shell サーフェス寸法の変化を検知し、変化した char 窓のみアンカー再適用を駆動する
/// 薄いアダプタ（GPU 結合の thin wiring・`presenter` を read-only 消費・design
/// 「resnap_shell_targets」・Req3.2/4.1/4.5）。
///
/// [`GhostWindows`] を取得し `scopes()` を回す。各 scope について
/// **`presenter.text_slot_view(shell_target(scope))`**（**`balloon_target` は読まない**＝shell
/// 限定駆動・Req4.5/DD-5）を引き、`None`（初回 `ShowSurface` 前＝未表示）は skip。
/// `surface_size() -> (u32, u32)`（emo-present 適用点の実寸・Req4.1）を `i32::try_from` で
/// [`SizePx`] 化し、**変換失敗・0** は skip（Req3.4。`try_from(0)=Ok(0)` ゆえ 0 を明示的に弾く）。
/// 得た `(scope, SizePx)` 列を [`resnap_from_sizes`] へ渡す——**presenter 借用を解いてから**
/// （先に `Vec` へ collect してから world を mut 借用・借用衝突回避）。
///
/// 未表示 target・未装着 presenter は全 scope skip（no-op・panic しない）。`GhostWindows` 未挿入
/// でも安全（`resnap_from_sizes` が no-op）。
fn resnap_shell_targets(presenter: &EmoPresenter, world: &mut World) {
    resnap_with(presenter, world)
}

/// 表示中 target の**物理寸**（k 倍後）だけを引く最小シーム（[`EmoPresenter::target_physical_size`]
/// の抽象）。
///
/// `EmoPresenter` から `Some` を得るには実 GPU で `ShowSurface` を完了させた装着済み target が
/// 要る。ゆえに「resnap が **どの `TargetId` を読むか**」（shell か balloon か）は、素の
/// `EmoPresenter::new()` を渡す存在チェックでは**全 target が `None` に潰れて観測できない**——
/// `shell_target`→`balloon_target` の 1 トークン変異が檻をすり抜けていた実際の穴である
/// （2026-07-30 是正。それ以前は「コードレビューで足りる」と散文で断っていた）。
///
/// 本トレイトは兄弟の [`ScaleReportSource`] と同型の意図を持つ: **frame 側の結線**を GPU 無しの
/// 決定論檻へ入れるためのシーム（D9 の振り分け基準 (a)＝判断分岐は in-crate 純テスト）。
trait PhysicalSizeSource {
    /// 表示中なら適用済み k を掛けた物理寸を返す。未装着・未表示は `None`。
    fn physical_size(&self, target: TargetId) -> Option<(u32, u32)>;
}

impl PhysicalSizeSource for EmoPresenter {
    fn physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
        self.target_physical_size(target)
    }
}

/// [`resnap_shell_targets`] の本体（[`PhysicalSizeSource`] 越しに寸を引く形へ一般化したもの）。
///
/// 本番経路は `resnap_shell_targets` が**本体を持たずここへ委譲する**だけである——実装を 2 つに
/// 割らないことが要点で、fake 相手の檻が「本番も同じ判断をしている」ことを担保する
/// （実装が分岐していると fake は緑のまま本番だけ壊れ得る）。
///
/// # 破棄済み窓の打ち切り（要件 6.2/6.3・design D8 消費側）
///
/// scope ループの**冒頭**で char 窓 entity の存在を確認し、既に despawn 済みなら
/// `debug!`（[`DESPAWNED_SKIP_TAG`]）で当該 scope を打ち切って**他 scope は処理し切る**
/// （終了処理の正常系ゆえ警告以上を出さない）。寸の問い合わせより手前に置くのは、
/// 破棄済み窓のために表示側へ問い合わせる意味が無いためである。
fn resnap_with<S: PhysicalSizeSource + ?Sized>(source: &S, world: &mut World) {
    // scope 識別は GhostWindows 経由（Req4.5）。未挿入は shell 寸を引く対象が無い＝no-op。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return;
    };
    // presenter 借用を解いてから resnap_from_sizes（&mut World）を呼ぶため、先に collect する。
    let mut sizes: Vec<(usize, SizePx)> = Vec::new();
    for scope in ghost_windows.scopes() {
        // 存在確認（要件 6.2/6.3・design D8 消費側）: レジストリが指す char 窓が既に
        // despawn 済みなら **正常終了系**として debug で打ち切り、**他 scope は処理し切る**。
        // 寸の問い合わせより手前に置く——破棄済みの窓のために表示側へ問い合わせる意味が
        // 無いうえ、素通りさせると下流 `resize_window_to` が破棄済み窓ぶん呼ばれる。
        if let Some(char_window) = ghost_windows.char_window(scope)
            && world.get_entity(char_window).is_err()
        {
            debug!(
                scope,
                entity = ?char_window,
                "{DESPAWNED_SKIP_TAG} resnap: char 窓 entity が破棄済み（despawn）→ 本 scope を正常系として打ち切り（他 scope は継続）"
            );
            continue;
        }
        // shell target（偶数=2*scope）のみを読む（balloon_target は読まない＝shell 限定・Req4.5）。
        // 窓 client に合わせるべき寸は **物理寸**（k 倍後）であって native 原寸ではない。両者を
        // 選べる `text_slot_view()`（`surface_size()`／`physical_size()` が隣り合う）ではなく、
        // **物理寸だけを返す** `target_physical_size` を引く——消費点に取り違えの選択肢を残さない
        // （native で駆動すると k≠1 で DPI 相 reconcile と同一フレーム内で綱引きになり窓が原寸へ
        // 引き戻される）。丸めは presenter 側が権威 `scaled_extent` で確定済みゆえ通貨変換のみ行う。
        // 未表示（初回 ShowSurface 前）・未装着は `None` → skip（no-op・遅延化への防御）。
        // shell/balloon の取り違えは `resnap_reads_shell_targets_only_and_ignores_balloon_geometry`
        // と `resnap_queries_shell_targets_only` が排他的に殺す（2026-07-30 実測）。
        let Some((w, h)) = source.physical_size(shell_target(scope as u32)) else {
            continue;
        };
        // (u32,u32)→i32 変換失敗は skip（Req3.4）。
        let (Ok(w), Ok(h)) = (i32::try_from(w), i32::try_from(h)) else {
            debug!(
                scope,
                w, h, "resnap: 物理寸の i32 変換に失敗 → skip（Req3.4）"
            );
            continue;
        };
        // 0 は skip（try_from(0)=Ok(0) ゆえ明示的に弾く・Req3.4）。負値は u32 起点ゆえ生じない。
        if w == 0 || h == 0 {
            debug!(
                scope,
                "resnap: 物理寸が 0 → skip（Req3.4・try_from(0)=Ok を明示的に弾く）"
            );
            continue;
        }
        sizes.push((scope, SizePx { w, h }));
    }
    // ここで presenter／ghost_windows 借用は終わり、world を mut 借用して判定・反映へ渡す。
    resnap_from_sizes(world, sizes.into_iter());
}

/// `talk_time` 解決の純判断（override 優先→`clock.talk_time(frame_now)`→`None`）。
///
/// [`run_text_phase`] の分岐条件を GPU/時刻 I/O 抜きの決定論檻へ切り出した純関数:
/// - `override_` が `Some(t)`（テスト注入経路）→ `Some(t)`（`frame_now`／`clock` は無視・最優先）。
/// - `override_` が `None`（本番経路）→ `frame_now` が `Some(now)` なら `clock.talk_time(now)`、
///   `frame_now` が `None`（`FrameTime` 資源不在＝headless）なら `None`。
/// - いずれも `clock` の epoch 未確立（talk 未到達）なら `talk_time` が `None` を返すため `None`。
///
/// 戻り値 `None` は「今フレームは描くものがない（`present_frame` を呼ばない）」を意味する。
fn resolve_talk_time(
    override_: Option<f64>,
    frame_now: Option<f64>,
    clock: &TalkClock,
) -> Option<f64> {
    match override_ {
        // テスト注入経路: override が最優先（frame_now／clock は無視）。
        Some(t) => Some(t),
        // 本番経路: FrameTime（frame_now）→ TalkClock。frame_now 不在／epoch 未確立は None。
        None => frame_now.and_then(|now| clock.talk_time(now)),
    }
}

/// フェーズ③（text・design「フェーズ③（text）」・R2.3）: `talk_time` が定まるフレームでのみ
/// `present_frame` を駆動する（`Err` は `error!`＋継続＝次フレーム再試行）。
///
/// `talk_time` の解決は [`resolve_talk_time`] に委ねる（`talk_time_override` が `Some` ならそれを、
/// なければ `FrameTime` 資源（`wintf::ecs::FrameTime`・`.0: f64`）を読んで
/// [`TalkClock::talk_time`]）。解決が `Some(t)` のときのみ
/// `present_frame(&mut runtime.borrow_mut(), world, t)` を呼ぶ。`Err(e)` は `error!`（`present_frame`
/// 側で失敗源を log 済み・first error 返却）＋継続で、他 actor を巻き込まず次フレーム再試行へ委ねる
/// （R2.3・emo-text 既存契約）。解決が `None`（epoch 未確立／`FrameTime` 不在かつ override なし）なら
/// `present_frame` を呼ばず skip する（描くものがない・hang しない）。
pub fn run_text_phase(wiring: &mut Emo2Wiring, world: &mut World, talk_time_override: Option<f64>) {
    // 本番の frame 時刻源（headless では不在）。override が Some なら resolve_talk_time が優先採用する。
    let frame_now = world.get_resource::<FrameTime>().map(|ft| ft.0);
    let Some(talk_time) = resolve_talk_time(talk_time_override, frame_now, &wiring.clock) else {
        // epoch 未確立（talk 未到達）または FrameTime 不在かつ override なし → 描くものがない・skip。
        return;
    };
    // present_frame は失敗源で log 済み（first error 返却）。frame は error!＋継続で、他 actor を
    // 巻き込まず次フレーム再試行へ委ねる（R2.3・emo-text 既存契約）。
    let mut runtime = wiring.runtime.borrow_mut();
    if let Err(e) = present_frame(&mut runtime, world, talk_time) {
        error!(
            error = %e,
            talk_time,
            "emo2 text: present_frame が失敗（他 actor 非破壊・次フレーム再試行・R2.3）"
        );
    }
    // 実機サインオフ用 hover 注入導線（HoverInjectConduit・8.2/8.4/8.6）: present_frame の**後**に
    // 駆動し、`choice_active`／`choice_hit_rows` が当該フレームの提示を反映した状態で env ゲート
    // （`AREKA_CHOICE_HOVER_INJECT`）駆動の周期巡回注入を行う。env 未設定/無効なら完全 no-op
    // （`inject_choice_hover` を一度も呼ばない・本番既定）。`talk_time` は同じ frame clock 時刻源。
    super::hover_inject::drive(&mut runtime, talk_time);
}

/// `FrameFinalize` 登録の排他 system（donor パターン: remove→3 フェーズ→insert・DD-1/DD-4）。
///
/// `Emo2Wiring`（NonSend）を [`World::remove_non_send_resource`] で取り出してから
/// attach→drain→text の 3 フェーズを順に駆動し、[`World::insert_non_send_resource`] で戻す。
/// remove→insert は `&mut World` を各フェーズへ排他に渡すための donor 慣行（借用衝突回避・
/// `examples/emo-present.rs::boot_present_system` と同型）。本番の text フェーズは override 無し
/// （`FrameTime`＋`TalkClock` で `talk_time` を解決）。
///
/// `Emo2Wiring` 未挿入（`wire_emo2_boot`＝task 5.1 前・フォールバック boot 経路）なら早期 return の
/// no-op（安全・panic しない）。schedule への登録（`add_systems(FrameFinalize, emo2_frame_system)`）は
/// `wire_emo2_boot`（task 5.1）が行い、本関数はここでは定義のみ（登録しない）。
pub fn emo2_frame_system(world: &mut World) {
    // Emo2Wiring 未挿入（wire_emo2_boot=task 5.1 前・LogSink フォールバック boot 経路）なら no-op。
    let Some(mut wiring) = world.remove_non_send_resource::<Emo2Wiring>() else {
        return;
    };
    // donor 慣行: remove して &mut World を各フェーズへ排他に渡し、全フェーズ駆動後に必ず戻す。
    run_attach_phase(&mut wiring, world);
    // DPI 追従（attach → dpi → drain …の順・design「run_dpi_phase（frame.rs）」）: attach の**後**に
    // 置く——装着前は再スケール対象の target が無く、attach と同一フレームで窓が生えた直後の
    // `Changed<DPI>`（生成時 GetDpiForWindow 実値補正）を同フレームで拾えるようにするため。drain の
    // **前**に置く理由は責任分界であり順序依存ではない: エッジ駆動の再表示を先に済ませ、残った
    // 未消費要求（初回表示の k₀ 補正）を drain 末尾の状態照合経路が拾う（両経路は presenter の
    // 消費規約により二重にも取りこぼしにもならない＝どちらの順でも整合する）。
    run_dpi_phase(&mut wiring, world);
    run_drain_phase(&mut wiring, world);
    // `\![move]` の末端結線: talk スレッドの MoveCueSink から届いた MoveDirective を drain し
    // apply_move_directive で実窓へ即時反映する（GhostWindows ゲート・R5・task 9.2）。present drain
    // とは独立で、GPU attach でなく GhostWindows 存在を待つ（move はキャラ窓 entity へ作用するため）。
    run_move_drain_phase(&wiring, world);
    // drain（全 PresentCommand 適用）後に shell サーフェス寸法の変化を検知し、変化した char 窓のみ
    // アンカー再適用を駆動する（適用後の実寸を読むため drain の**後**・同一 World・同一 tick 内の
    // 直接呼び・Req4.1/4.3/1.3）。text の前後とは機能的に無関係だが drain の後であることが必須。
    resnap_shell_targets(&wiring.presenter, world);
    // 文字層 k 追従（emo-dpi-scaling task 7.2・D11-4・Req8）: 適用 k の更新点（dpi 相の `refresh_scale`
    // ／drain 相の `apply_show`）の**両方の下流**、かつ `present_frame` の**上流**に置く。こうすると
    // どちらの経路で k が跳ねても同一フレーム内で binding が新 k へ組み直され、直後の描画が新しい物理寸
    // で走る（旧寸の文字が 1 フレーム残らない）。戻り値（再構築 scope）は観測用ゆえ本番は捨てる。
    let _ = run_text_scale_phase(&mut wiring);
    run_text_phase(&mut wiring, world, None); // 本番: override なし（FrameTime＋clock で解決）。
    world.insert_non_send_resource(wiring);
}

#[cfg(test)]
#[path = "frame_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "frame_attach_tests.rs"]
mod attach_tests;

#[cfg(test)]
#[path = "frame_drain_text_tests.rs"]
mod drain_text_tests;

#[cfg(test)]
#[path = "frame_resnap_tests.rs"]
mod resnap_tests;

#[cfg(test)]
#[path = "frame_dpi_tests.rs"]
mod dpi_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_tests.rs"]
mod dpi_reproject_tests;

#[cfg(test)]
#[path = "frame_dpi_reproject_none_tests.rs"]
mod dpi_reproject_none_tests;

#[cfg(test)]
#[path = "frame_diag_route_tests.rs"]
mod diag_route_tests;

#[cfg(test)]
#[path = "frame_text_scale_tests.rs"]
mod text_scale_tests;
