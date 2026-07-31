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
    /// 引くのが自然である。attach が実際に scope 別の定義をここへ挿すのは tasks.md task 3.3 が担い、
    /// それまでは先頭 scope の定義 1 本を全 scope へ配る暫定形である。
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
    // 文字層へ渡すバルーン定義は、この時点ではまだ **先頭 scope の定義 1 本を全 scope へ配る**
    // （撤去した `BootAssets.balloon_model` と観測等価——emo2 では先頭＝scope 0 の 2 層マージ結果）。
    // 各 scope が自身の [`BalloonScopeAssets::model`] を受け取る per-scope 供給は tasks.md task 3.3 が
    // 担い、そこで本スタンドインは消える。
    let shared_balloon_model = balloons.first().map(|b| b.model.clone());
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
        // per-scope 供給（task 3.3）までの暫定: 先頭 scope の定義を全 scope へ配る。資産を
        // take する前に引き当てる（不在なら資産を消費せず skip する）。`balloon_index` が
        // 取れている以上 balloon 資産は非空ゆえ、この縮退は到達しない防衛である。
        let Some(balloon_model) = shared_balloon_model.as_ref() else {
            error!(
                scope,
                "emo2 attach: バルーン定義が 1 件も無い（balloon 資産ゼロ）→ 文字層接続なし"
            );
            continue;
        };
        let Some(BalloonScopeAssets {
            emo_world: balloon_world,
            atlas: balloon_atlas,
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
        // 文字層 k 再追従（D11-3・R8.1）の再利用源: 装着に使うモデルを scope キーで記憶する。
        // 文字層スケール相（[`run_text_scale_phase`]）はこれを再利用して binding を組み直す（再パースしない）。
        // 装着が `text_slot_view` None で次フレームへ委ねられた場合でもモデル自体は有効ゆえ、
        // 接続成否に関わらず記憶する（再追従は未登録 actor を静穏 skip する・7.1 の契約）。
        wiring
            .balloon_models
            .insert(scope, (*balloon_model).clone());
        // apply は同期ゆえ同一フレームで text_slot_view が Some になるのが正常経路（DD-4）。
        // None（上流の遅延化）は接続せず次フレーム再試行へ委ねる（R4.2）。
        let view = wiring.presenter.text_slot_view(item.balloon_target);
        connect_balloon_text(
            &wiring.runtime,
            view,
            // 再追従（[`run_text_scale_phase`]）は**同一の写像**で actor を引く——
            // ここと式が食い違うと、再追従が別 actor を作って文字だけ旧 k のまま残る。
            ActorKey::from(scope.to_string()),
            balloon_model,
        );
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
fn reconcile_window_size(
    world: &mut World,
    window: Entity,
    kind: GhostWindowKind,
    new_size: (u32, u32),
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
        GhostWindowKind::Char => resize_window_to(world, window, new_size),
        // バルーン窓: 位置は追従で決まる従属量ゆえ据え置き、寸だけ差し替える。
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
/// 各ゴースト窓について対応 target の [`ScaleReportSource::refresh_scale_report`] を呼び、
/// `Some(新物理寸)` のときだけ [`reconcile_window_size`] で窓 client を合わせる。
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
/// `None` で**窓寸を一切触らない**（前寸維持）。panic しない。
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
        // 再導出→（差分があれば）再表示。`None` は「窓寸を触らない」が正しい（前表示・前寸の維持・
        // Req4.4）。なお `None` は k 不変とは同義でない——不可視・未表示・失敗に加え、**k は変わった
        // が丸め後の物理寸が同じ**場合も `None` である（`refresh_scale` の doc が明記）。ゆえに
        // 文字層 k 追従の判断材料にはこの戻り値を使わない（[`run_text_scale_phase`] を参照）。
        if let Some(new_size) = source.refresh_scale_report(world, target) {
            reconcile_window_size(world, window, kind, new_size);
        }
    }
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
/// # なぜ「イベント駆動」ではなく毎フレーム走査なのか（D11-4 の意図＝k 変化の検出）
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
/// ゆえに検出点は「**balloon target の適用 k が文字層 binding の k と食い違っているか**」であり、
/// それを判定できる唯一の権威は [`TextLayerRuntime::refresh_actor_binding`]（task 7.1）である。
/// 本フェーズは判定を自前で複製せず（第 2 のガードは本家と乖離し得る）、毎フレーム
/// [`TextLayerRuntime::refresh_actor_scale`] へ委ねる——同値 k・未登録 actor はあちらが
/// **再構築せず `false`** を返す（churn ガード・R8.5）。費用は balloon 1 枚あたり map 1 引きと
/// `ScaleRatio` 1 比較。
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
/// 保持する）ため、**不可視は本縮退経路に落ちない**——不可視の間は同値 k の no-op が続き、`Show`
/// で `applied` が跳ねた次の走査が再追従する。
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
            reconcile_window_size(world, window, kind, new_size);
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
        resize_window_to(world, char_window, shown_size);
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
fn resnap_with<S: PhysicalSizeSource + ?Sized>(source: &S, world: &mut World) {
    // scope 識別は GhostWindows 経由（Req4.5）。未挿入は shell 寸を引く対象が無い＝no-op。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return;
    };
    // presenter 借用を解いてから resnap_from_sizes（&mut World）を呼ぶため、先に collect する。
    let mut sizes: Vec<(usize, SizePx)> = Vec::new();
    for scope in ghost_windows.scopes() {
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
mod tests {
    use std::collections::BTreeMap;
    use std::sync::mpsc::Receiver;
    use std::sync::{mpsc, Arc, Mutex};

    use areka_emo_atlas::AtlasTable;
    use areka_emo_compose::{BindSet, EmoWorld};
    use areka_emo_text::state::TextLayerConfig;
    use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    use super::*;
    use crate::emo2_boot::assets::{BalloonScopeAssets, BootAssets, LoopTables, ScopeAssets};

    /// throwaway な `EmoWorld`（空 shell から build・`plan_attachments` は emo_world を読まない）。
    ///
    /// `EmoWorld::build` は空 `Shell` に対し entity ゼロの空 World を返す寛容契約（COM/GPU 不要）。
    fn empty_world() -> EmoWorld {
        EmoWorld::build(&areka_parsers::shell::parse(""))
    }

    /// 空アトラス（headless 構築・`plan_attachments` は atlas を読まない）。
    fn empty_atlas() -> AtlasTable {
        AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
    }

    /// 合成 `BootAssets`（shell scope 集合と同一の balloon scope 集合を持つ標準形）。
    ///
    /// `plan_attachments` が実際に読むのは `shells[*].scope`／`shells[*].initial_surface_id`／
    /// `balloons[*].scope` のみ。残りのフィールド（emo_world／atlas／model／resolver／
    /// static_binds）は最小の headless 値で埋める（COM/GPU/fixture 不要の純合成）。
    fn synth_assets(shells: &[(u32, u32)]) -> BootAssets {
        let balloon_scopes: Vec<u32> = shells.iter().map(|&(scope, _)| scope).collect();
        synth_assets_with_balloons(shells, &balloon_scopes)
    }

    /// balloon scope 集合を shell と独立に指定できる版（`balloon_index` の `None` 経路検証用）。
    fn synth_assets_with_balloons(shells: &[(u32, u32)], balloon_scopes: &[u32]) -> BootAssets {
        BootAssets {
            shells: shells
                .iter()
                .map(|&(scope, initial_surface_id)| ScopeAssets {
                    scope,
                    emo_world: empty_world(),
                    atlas: empty_atlas(),
                    initial_surface_id,
                })
                .collect(),
            balloons: balloon_scopes
                .iter()
                .map(|&scope| BalloonScopeAssets {
                    scope,
                    emo_world: empty_world(),
                    atlas: empty_atlas(),
                    model: areka_parsers::balloon::parse_str("", None),
                })
                .collect(),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: BindSet::default(),
            // plan_attachments は bind_resolver を読まない（headless 純合成）＝空表で十分。
            bind_resolver: BindResolver::empty(),
            // plan_attachments は loop_tables を読まない（headless 純合成）＝空表で十分。
            loop_tables: LoopTables {
                shell: AnimationTable::empty(),
                balloon: BTreeMap::new(),
            },
            shell_author_dpi: 96,
            balloon_author_dpi: 96,
        }
    }

    /// DD-12 完全一致: `window_scopes == 資産 scope` → 計画件数＝窓数・missing/unused 空。
    ///
    /// 各項目の shell/balloon target（DD-3 採番）と初期面（DD-9: scope0→0／scope1→10）・添字も検証する。
    #[test]
    fn plan_attachments_exact_match_plans_all_windows() {
        // 資産 scope [0,1]（DD-9: scope0→初期 surface 0／scope1→初期 surface 10）。
        let assets = synth_assets(&[(0, 0), (1, 10)]);
        let window_scopes = [0usize, 1];

        let plan = plan_attachments(&window_scopes, &assets);

        // 計画件数＝窓数（DD-12 の積極 assert・完全一致の核）。
        assert_eq!(
            plan.items.len(),
            window_scopes.len(),
            "完全一致では計画件数＝窓数"
        );
        assert!(plan.missing_assets.is_empty(), "窓あり資産なしは無い");
        assert!(plan.unused_assets.is_empty(), "資産あり窓なしは無い");

        // scope0 の項目: shell=TargetId(0)・balloon=TargetId(1)・初期面 0（DD-9）。
        assert_eq!(plan.items[0].scope, 0);
        assert_eq!(plan.items[0].shell_target, shell_target(0));
        assert_eq!(plan.items[0].balloon_target, balloon_target(0));
        assert_eq!(plan.items[0].initial_surface_id, 0, "scope0 初期面 0（DD-9）");
        assert_eq!(plan.items[0].shell_index, 0);
        assert_eq!(plan.items[0].balloon_index, Some(0));

        // scope1 の項目: shell=TargetId(2)・balloon=TargetId(3)・初期面 10（DD-9）。
        assert_eq!(plan.items[1].scope, 1);
        assert_eq!(plan.items[1].shell_target, shell_target(1));
        assert_eq!(plan.items[1].balloon_target, balloon_target(1));
        assert_eq!(plan.items[1].initial_surface_id, 10, "scope1 初期面 10（DD-9）");
        assert_eq!(plan.items[1].shell_index, 1);
        assert_eq!(plan.items[1].balloon_index, Some(1));
    }

    /// DD-12 窓あり資産なし: 窓 scope に対応資産が無ければ `missing_assets` へ（`items` には載らない）。
    #[test]
    fn plan_attachments_window_without_asset_goes_to_missing() {
        // 資産 [0,1]・窓 [0,1,2] → 窓 scope 2 は資産なし。
        let assets = synth_assets(&[(0, 0), (1, 10)]);
        let window_scopes = [0usize, 1, 2];

        let plan = plan_attachments(&window_scopes, &assets);

        assert_eq!(plan.missing_assets, vec![2usize], "窓あり資産なしは missing 検出");
        assert_eq!(plan.items.len(), 2, "資産のある 0,1 のみ装着計画に載る");
        assert!(
            plan.items.iter().all(|it| it.scope != 2),
            "scope2 は items に載らない"
        );
        assert!(plan.unused_assets.is_empty(), "全資産に窓がある");
    }

    /// DD-12 資産あり窓なし: 窓を持たない資産 scope は `unused_assets`（`u32`）へ（`items` には載らない）。
    #[test]
    fn plan_attachments_asset_without_window_goes_to_unused() {
        // 資産 [0,1]・窓 [0] のみ → 資産 scope 1 は窓なし。
        let assets = synth_assets(&[(0, 0), (1, 10)]);
        let window_scopes = [0usize];

        let plan = plan_attachments(&window_scopes, &assets);

        assert_eq!(plan.unused_assets, vec![1u32], "資産あり窓なしは unused 検出（u32）");
        assert_eq!(plan.items.len(), 1, "窓のある scope0 のみ装着計画に載る");
        assert_eq!(plan.items[0].scope, 0);
        assert!(plan.missing_assets.is_empty(), "全窓に資産がある");
    }

    /// DD-12 `usize`→`u32` 変換境界: 小 usize scope は u32 資産 scope と一致・`u32::MAX` 超過は missing。
    #[test]
    fn plan_attachments_usize_to_u32_conversion_boundary() {
        let assets = synth_assets(&[(0, 0), (1, 10)]);

        // 小さい usize scope (0,1) は u32 資産 scope と正しく一致する。
        let plan = plan_attachments(&[0usize, 1], &assets);
        assert_eq!(plan.items.len(), 2, "小 usize scope は u32 資産と一致");
        assert_eq!(plan.items[0].scope, 0);
        assert_eq!(plan.items[1].scope, 1);

        // usize > u32::MAX は如何なる u32 資産 scope とも一致し得ず missing 分類（64bit 環境でのみ表現可能）。
        #[cfg(target_pointer_width = "64")]
        {
            let overflow = (u32::MAX as usize) + 1; // u32 に収まらない境界超過値
            let plan = plan_attachments(&[0usize, overflow], &assets);
            assert_eq!(plan.items.len(), 1, "overflow scope は装着計画に載らない");
            assert_eq!(plan.items[0].scope, 0, "収まる 0 のみ計画に載る");
            assert_eq!(
                plan.missing_assets,
                vec![overflow],
                "u32 超過 usize は missing 分類（変換不能）"
            );
        }
    }

    /// `balloon_index` の `None` 経路: shell 資産はあるが同 scope の balloon 資産が無い場合。
    #[test]
    fn plan_attachments_marks_missing_balloon_index_none() {
        // shell scope [0,1]・balloon scope [0] のみ → scope1 は shell だけで balloon なし。
        let assets = synth_assets_with_balloons(&[(0, 0), (1, 10)], &[0]);
        let plan = plan_attachments(&[0usize, 1], &assets);

        assert_eq!(plan.items.len(), 2, "shell 資産の揃う 0,1 が計画に載る");
        assert_eq!(plan.items[0].balloon_index, Some(0), "scope0 は balloon あり");
        assert_eq!(plan.items[1].balloon_index, None, "scope1 は balloon なし → None");
    }

    /// R4.2 headless: `text_slot_view` が `None`（初回 ShowSurface 未合流）の経路では文字層を
    /// 接続せず `false` を返し、次フレーム再試行へ委ねる（panic しない・register も呼ばない）。
    ///
    /// 実 `EmoPresenter::new()` は何も装着していないため `text_slot_view(any) == None`。この実源の
    /// `None` を `connect_balloon_text` に与え、接続が起きない（`false`）ことを GPU 不要で檻に入れる。
    #[test]
    fn connect_balloon_text_skips_when_view_none() {
        // 初回 ShowSurface 前の presenter は text_slot_view が None（R4.2 の遅延化を実源で再現）。
        let presenter = EmoPresenter::new();
        let view = presenter.text_slot_view(TargetId(1));
        assert!(view.is_none(), "初回 ShowSurface 前の text_slot_view は None（前提）");

        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        let model = areka_parsers::balloon::parse_str("", None);

        // None 経路: 文字層を接続せず false（次フレーム再試行へ委ねる）。panic しない。
        let registered = connect_balloon_text(&runtime, view, ActorKey::from("0"), &model);
        assert!(
            !registered,
            "text_slot_view None では文字層を接続せず false を返す（R4.2）"
        );

        // register_actor_view 未呼出の担保: runtime は無改変で再借用可能（lingering borrow / poison なし）。
        assert!(
            runtime.try_borrow_mut().is_ok(),
            "None 経路は runtime に触れない（借用/poison を残さない）"
        );
    }

    /// ゲート（GPU 資源）不成立の World では装着しない・panic しない・資産を消費しない（R1.3/4.1）。
    ///
    /// `GraphicsCore`／`WucGraphicsResource`／`GhostWindows` を一切持たない `World` に対し
    /// `run_attach_phase` を複数回呼んでも、`attached` は false のまま・`assets` は `Some` のまま
    /// （高々 1 回消費の `take` はゲート成立後にのみ行うため未消費）で、panic しないことを headless に固定する。
    #[test]
    fn run_attach_phase_without_gpu_does_not_attach_or_consume_assets() {
        let mut wiring = Emo2Wiring::new(
            EmoPresenter::new(),
            mpsc::channel::<PresentCommand>().1,
            mpsc::channel::<MoveDirective>().1,
            Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
            TalkClock::new(Arc::new(|| 0.0)),
            synth_assets(&[(0, 0), (1, 10)]),
        );
        // GraphicsCore/WucGraphicsResource/GhostWindows を一切持たない素の World（ゲート不成立）。
        let mut world = World::new();

        run_attach_phase(&mut wiring, &mut world);
        assert!(!wiring.attached, "GPU 資源なしでは装着しない（attached=false）");
        assert!(
            wiring.assets.is_some(),
            "ゲート不成立では assets を take しない（次フレーム再試行のため保持）"
        );
        assert!(
            wiring.balloon_model_scopes().is_empty(),
            "ゲート不成立では per-scope BalloonModel も記憶しない（attach 相の副作用ゼロ・D11-3）"
        );

        // 冪等: 再試行してもゲート不成立なら未装着のまま・panic しない・資産も保持する。
        run_attach_phase(&mut wiring, &mut world);
        assert!(!wiring.attached, "再試行でもゲート不成立なら未装着");
        assert!(wiring.assets.is_some(), "再試行でも assets を保持");
    }

    /// task 2.5（Req 1.3）: UI 配線層（`Emo2Wiring`）の presenter 読み口から collision-geometry の
    /// `resolve_hit_region` を呼べることを固定する。`input-events` の第一消費者（task 2.6/2.7 の
    /// `RegionSource::Presenter`）が `Emo2Wiring::presenter()` を借りて resolver を叩く経路の縮図。
    ///
    /// 未装着 `EmoPresenter`（現サーフェス無し）では collision-geometry の documented degrade により
    /// `region: None` へ正常縮退する（hit_region.rs 4.4/5.3）。GPU/表示なしで決定論的に成立する。
    #[test]
    fn presenter_accessor_feeds_resolve_hit_region() {
        use crate::emo2_boot::hit_region::{resolve_hit_region, HitRegion};

        let wiring = Emo2Wiring::new(
            EmoPresenter::new(),
            mpsc::channel::<PresentCommand>().1,
            mpsc::channel::<MoveDirective>().1,
            Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
            TalkClock::new(Arc::new(|| 0.0)),
            synth_assets(&[(0, 0), (1, 10)]),
        );

        // UI 配線層の読み口から resolver を呼べる（Req 1.3・型が resolve_hit_region の
        // 第 1 引数 &EmoPresenter に一致することをコンパイル時にも固定）。未装着ゆえ region None。
        let got = resolve_hit_region(wiring.presenter(), 0, 100, 100);
        assert_eq!(
            got,
            HitRegion {
                scope: 0,
                region: None,
            },
            "未装着 presenter は region None（collision-geometry の正常縮退・Req1.3）"
        );
    }

    /// task 9.1 存在檻: `MoveCueSink`（送出端）→ `Emo2Wiring`（受信端 `move_rx`）の channel 配線が
    /// 到達可能であること（`wire_emo2_boot` が `mpsc::channel::<MoveDirective>()` を生成し送出端を
    /// sinks 第 3 要素の `MoveCueSink` へ、受信端を `Emo2Wiring` へ渡す配線の縮図）。
    ///
    /// 送出端を持つ `MoveCueSink` に `\![move]` キャリア cue を `emit` すると、`Emo2Wiring` が保持する
    /// 受信端から同一 `MoveDirective` が drain できる（frame 相 drain＝task 9.2 の適用は本檻の範囲外）。
    #[test]
    fn move_cue_sink_reaches_emo2_wiring_receiver() {
        use super::super::move_cue::MoveCueSink;
        use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};

        // wire_emo2_boot 手順4 と同型: 単一 channel の送出端を sink へ、受信端を Emo2Wiring へ。
        let (move_tx, move_rx) = mpsc::channel::<MoveDirective>();
        let mut move_sink = MoveCueSink::new(move_tx);
        let wiring = Emo2Wiring::new(
            EmoPresenter::new(),
            mpsc::channel::<PresentCommand>().1,
            move_rx,
            Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
            TalkClock::new(Arc::new(|| 0.0)),
            synth_assets(&[(0, 0)]),
        );

        // sink（talk スレッド相当）へ `\![move]` キャリアを emit → 受信端（Emo2Wiring）へ届く。
        move_sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("1"),
            command: CueCommand::command_carrier(
                "move",
                ["-353", "", "", "0", "base", "base"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ),
            duration: 0.0,
        });

        let drained = wiring.drain_move_directives();
        assert_eq!(drained.len(), 1, "sink→Emo2Wiring の受信端へちょうど 1 件届く");
        assert_eq!(drained[0].scope, 1, "scope は cue.actor（\\1）由来");
        assert_eq!(
            drained[0].base,
            crate::emo2_boot::move_cue::MoveBase::Scope(0),
            "base=scope0（fixture 形）"
        );
    }

    // ── task 4.2: drain／text フェーズ＋emo2_frame_system の檻 ──────────────────────
    //
    // ログ発火を目視でなく実行テストで決定論的に檻へ入れるため、adapter.rs（task 2.5）と同じ
    // スレッドローカル capture subscriber を単一ファイル境界（frame.rs）内へ最小インライン複製する。
    // `EmoPresenter::apply` は未装着 target への `Hide`（reply: None）で `error!(?target_id, ...)` を
    // 発火するため（presenter.rs `apply_hide`・reply-less でも log-first）、この ERROR を観測して
    // drain の apply 到着順（FIFO）を強く檻に入れられる。

    /// イベントの `level`＋各フィールドを 1 行文字列へ整形して共有 Vec へ push する最小 Layer。
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            ev: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let meta = ev.metadata();
            let mut line = format!("level={} target={}", meta.level(), meta.target());
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut line));
            self.0.lock().unwrap().push(line);
        }
    }

    /// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
    fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
        let cap = Capture::default();
        let logs = cap.0.clone();
        let subscriber = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(subscriber, f);
        let guard = logs.lock().unwrap();
        guard.clone()
    }

    /// 捕捉行のうち指定 level（例 `"ERROR"`）の件数を数える。
    fn count_level(logs: &[String], level: &str) -> usize {
        let needle = format!("level={level}");
        logs.iter().filter(|l| l.contains(&needle)).count()
    }

    /// テスト用の可制御クロック: 返り値の `Arc<Mutex<f64>>` に「壁時刻」を書けば、`TalkClock` の
    /// クロックがその時刻を返す（決定論・talk_clock.rs のテストと同型の注入クロック）。
    fn controllable_clock() -> (Arc<Mutex<f64>>, TalkClock) {
        let now = Arc::new(Mutex::new(0.0f64));
        let now_for_clock = Arc::clone(&now);
        let clock: Arc<dyn Fn() -> f64 + Send + Sync> =
            Arc::new(move || *now_for_clock.lock().expect("test clock mutex poisoned"));
        (now, TalkClock::new(clock))
    }

    /// 可制御クロックの「壁時刻」を書き換える。
    fn set_now(now: &Arc<Mutex<f64>>, wall: f64) {
        *now.lock().expect("test clock mutex poisoned") = wall;
    }

    /// epoch を確立しない固定クロック（drain 系テストは talk_time を使わない）。
    fn zero_clock() -> TalkClock {
        TalkClock::new(Arc::new(|| 0.0))
    }

    /// headless な `Emo2Wiring`（実 `EmoPresenter`／空 `TextLayerRuntime`／合成 `BootAssets`）を
    /// 注入 `rx`／`clock` で組む（task 4.1 のゲートテストと同型・COM/GPU 不要）。
    fn headless_wiring_with(rx: Receiver<PresentCommand>, clock: TalkClock) -> Emo2Wiring {
        Emo2Wiring::new(
            EmoPresenter::new(),
            rx,
            mpsc::channel::<MoveDirective>().1,
            Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
            clock,
            synth_assets(&[(0, 0)]),
        )
    }

    /// R2.2/DD-1 drain: attach 前は drain せず（保留＝取りこぼしなし）、attach 後に FIFO 到着順で
    /// **全件** `presenter.apply` へ適用し切る。
    ///
    /// 未装着 target への `Hide`（reply: None）は `EmoPresenter::apply_hide` が
    /// `error!(?target_id, "apply(Hide): 未装着ターゲット")` を発火する（reply-less でも log-first・
    /// panic しない）。この ERROR を capture subscriber で観測し、(a) attach 前は 0 件（gate 閉＝
    /// チャネル未 drain）、(b) attach 後は送信順 `TargetId(0)→(1)→(2)` でちょうど 3 件（drain-all＋
    /// FIFO 到着順）、(c) 2 度目の drain は 0 件（空チャネル・二重適用なし）を決定論的に反証する。
    #[test]
    fn run_drain_phase_gates_on_attach_then_drains_all_in_fifo_order() {
        let (tx, rx) = mpsc::channel::<PresentCommand>();
        let mut wiring = headless_wiring_with(rx, zero_clock());
        // GhostWindows/GPU を持たない素の World（drain は presenter.apply のみ・GPU 不要）。
        let mut world = World::new();

        // FIFO で 3 件送る（未装着 target ゆえ apply は error!＋return の no-op-with-log・panic しない）。
        for t in [0u32, 1, 2] {
            tx.send(PresentCommand::Hide {
                target: TargetId(t),
                reply: None,
            })
            .expect("送信は成功する（受信端 rx は wiring が保持）");
        }

        // (a) attach 前（gate 閉）: drain しない → apply 未呼出 → ERROR ログ 0 件。
        assert!(!wiring.attached, "前提: 未装着（run_attach_phase 未実行）");
        let logs_gated = capture_logs(|| run_drain_phase(&mut wiring, &mut world));
        assert_eq!(
            count_level(&logs_gated, "ERROR"),
            0,
            "attach 前は drain せず apply も呼ばない（チャネルが保留バッファ・取りこぼしなし・DD-1）: {logs_gated:?}"
        );

        // attach 完了フラグを立てる（本番は run_attach_phase が立てる・test では直接）。
        wiring.attached = true;

        // (b) gate 開: 現時点キュー済みを FIFO で全件 apply → 未装着 target ゆえ ERROR がちょうど 3 件、
        //     かつ target_id が送信順（0,1,2）で並ぶ（apply が到着順に呼ばれた実証）。
        let logs_drained = capture_logs(|| run_drain_phase(&mut wiring, &mut world));
        let errs: Vec<&String> = logs_drained
            .iter()
            .filter(|l| l.contains("level=ERROR"))
            .collect();
        assert_eq!(
            errs.len(),
            3,
            "gate 開後は 3 件全て apply（drain-all）: {logs_drained:?}"
        );
        for (i, expected) in [0u32, 1, 2].iter().enumerate() {
            assert!(
                errs[i].contains(&format!("TargetId({expected})")),
                "apply は FIFO 到着順（{i} 番目は TargetId({expected})）: {}",
                errs[i]
            );
        }

        // (c) 二度目の drain: チャネルは空 → 何も再適用しない（ERROR 0・二重適用なし）。
        let logs_empty = capture_logs(|| run_drain_phase(&mut wiring, &mut world));
        assert_eq!(
            count_level(&logs_empty, "ERROR"),
            0,
            "drain 済みチャネルは空・再適用しない: {logs_empty:?}"
        );
    }

    /// R2.2/R2.3 text 判断: `resolve_talk_time` は override 優先→`clock.talk_time`→`None` を返す。
    ///
    /// GPU/時刻 I/O 抜きの純関数として 4 経路を決定論檻へ入れる: override 勝ち（frame_now 無視）・
    /// override 無し×epoch 確立×frame_now 有り＝差分・frame_now 不在＝None・epoch 未確立＝None。
    #[test]
    fn resolve_talk_time_override_wins_else_clock_else_none() {
        // epoch 未確立の固定クロック。
        let clock_unset = zero_clock();

        // override=Some → そのまま（テスト注入経路が最優先・frame_now/clock は無視）。
        assert_eq!(
            resolve_talk_time(Some(5.0), Some(999.0), &clock_unset),
            Some(5.0),
            "override は最優先で採用（frame_now は無視）"
        );
        assert_eq!(
            resolve_talk_time(Some(5.0), None, &clock_unset),
            Some(5.0),
            "override は frame_now 不在でも採用"
        );

        // override=None, frame_now=Some, epoch 確立 → clock.talk_time(frame_now)。
        let (now, clock) = controllable_clock();
        set_now(&now, 100.0);
        clock.observe_cue(0.0); // epoch = 100.0 - 0.0 = 100.0
        assert_eq!(
            resolve_talk_time(None, Some(105.0), &clock),
            Some(5.0),
            "override 無しは clock.talk_time(frame_now)（105-100=5）"
        );

        // override=None, frame_now=None → None（FrameTime 資源不在＝headless）。
        assert_eq!(
            resolve_talk_time(None, None, &clock),
            None,
            "frame_now 不在は None（present_frame を呼ばない）"
        );

        // override=None, epoch 未確立 → None（talk 未到達＝描くものがない）。
        assert_eq!(
            resolve_talk_time(None, Some(105.0), &clock_unset),
            None,
            "epoch 未確立は None（talk 未到達）"
        );
    }

    /// R2.3 text smoke（no panic）: `run_text_phase` は override で `present_frame` へ到達し、
    /// override 無し×`FrameTime` 不在では skip する（いずれも panic しない）。
    ///
    /// 登録 actor の無い空 `TextLayerRuntime` に対し `present_frame` は `Ok(())` で即復帰する
    /// （GPU 不要・upstream 契約）。override=Some(2.0) で present_frame を踏み、override=None かつ
    /// `FrameTime` 資源なしで skip することを、panic なし＋ runtime 再借用可で担保する。
    #[test]
    fn run_text_phase_override_reaches_present_frame_without_panic() {
        let (_tx, rx) = mpsc::channel::<PresentCommand>();
        let mut wiring = headless_wiring_with(rx, zero_clock());
        // FrameTime 資源を持たない素の World（override 経路と skip 経路の双方を踏む）。
        let mut world = World::new();

        // override=Some(2.0)・空 runtime（登録 actor 無し）→ present_frame は Ok(()) で即復帰・panic しない。
        run_text_phase(&mut wiring, &mut world, Some(2.0));

        // override=None・FrameTime 資源なし → talk_time 解決不能で present_frame を呼ばず skip・panic しない。
        run_text_phase(&mut wiring, &mut world, None);

        // present_frame は borrow を残さない（RefCell を再借用できる＝lingering borrow / poison なし）。
        assert!(
            wiring.runtime.try_borrow_mut().is_ok(),
            "present_frame 後に runtime を再借用できる（借用を残さない）"
        );
    }

    /// 排他 system の疎通（DD-1/DD-4）: `emo2_frame_system` は NonSend `Emo2Wiring` を remove→3 フェーズ
    /// →insert で駆動して**必ず戻す**、かつ未挿入 World では安全に no-op（panic しない）。
    ///
    /// GPU/GhostWindows を持たない World ではゲート不成立で attach は起きず、drain は attach 前ゆえ
    /// 走らず、text は FrameTime 不在で skip する（＝実質 no-op）。それでも system が wiring を取り出して
    /// 戻す配線（remove→insert）が働くことを、実行後に NonSend resource が再取得できることで反証する。
    #[test]
    fn emo2_frame_system_removes_runs_and_reinserts_wiring() {
        let (_tx, rx) = mpsc::channel::<PresentCommand>();
        let wiring = headless_wiring_with(rx, zero_clock());
        let mut world = World::new();
        world.insert_non_send_resource(wiring);

        // remove→attach/drain/text（いずれもゲート不成立の no-op）→ re-insert。panic しない。
        emo2_frame_system(&mut world);
        assert!(
            world.get_non_send_resource::<Emo2Wiring>().is_some(),
            "emo2_frame_system は wiring を取り出して駆動後に必ず戻す（配線の疎通）"
        );

        // 冪等: もう一度呼んでも remove→insert で wiring を保つ（panic しない）。
        emo2_frame_system(&mut world);
        assert!(
            world.get_non_send_resource::<Emo2Wiring>().is_some(),
            "再実行でも wiring を保つ（remove→insert の冪等）"
        );

        // 資源が無い World でも安全に no-op（wire_emo2_boot 前・LogSink フォールバック boot 経路）。
        let mut empty_world = World::new();
        emo2_frame_system(&mut empty_world); // panic しない
        assert!(
            empty_world.get_non_send_resource::<Emo2Wiring>().is_none(),
            "未挿入なら no-op（何も挿入しない）"
        );
    }

    // ── task 3.2: resnap シーム（resnap_from_sizes／resnap_shell_targets）の檻 ────────
    //
    // drain 後の shell サーフェス寸法変化検知を GPU 不要で headless に固定する。
    // spawn_ghost_windows で 2 スコープの char/balloon 窓＋GhostWindows を組み（char 窓は
    // Anchored 付き）、各窓へ偽 WindowHandle を注入し MonitorSnapshot を挿入した World 上で、
    // 合成 (scope, SizePx) を resnap_from_sizes へ直接注入して観測する（Req1.3/3.1/3.4/4.5）。

    use bevy_ecs::prelude::Entity;
    use windows::Win32::Foundation::{HINSTANCE, HWND};

    use crate::placement::follow::MonitorSnapshot;
    use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
    use crate::placement::source::GhostTitles;
    use crate::placement::spawn::spawn_ghost_windows;
    use wintf::ecs::{Point, SizeI, WindowHandle, WindowPos};

    /// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム・follow.rs の fake_handle 相当）。
    fn fake_handle(raw: usize) -> WindowHandle {
        WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        }
    }

    /// resnap 檻の 2 スコープ解決済み配置（both Bottom・初期位置は work_area 下端に整合＝
    /// bottom 不変量を満たす: scope0 y=1444−687=757／scope1 y=1444−357=1087）。
    fn resnap_placements() -> Vec<ScopePlacement> {
        vec![
            ScopePlacement {
                scope: 0,
                char_pos: PointPx { x: 1483, y: 757 },
                char_size: SizePx { w: 434, h: 687 },
                balloon_pos: PointPx { x: 1071, y: 732 },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: -412, y: -25 },
                anchor: Anchor::Bottom,
            },
            ScopePlacement {
                scope: 1,
                char_pos: PointPx { x: 1049, y: 1087 },
                char_size: SizePx { w: 278, h: 357 },
                balloon_pos: PointPx { x: 1334, y: 1068 },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: 285, y: -19 },
                anchor: Anchor::Bottom,
            },
        ]
    }

    /// 実 work area（原点非 (0,0)・96 非倍数の合成値・bottom=1444＝配置と整合）。
    fn resnap_work_area() -> RectPx {
        RectPx {
            left: 31,
            top: 17,
            right: 2574,
            bottom: 1444,
        }
    }

    /// spawn_ghost_windows で 2 スコープの窓を組み、各窓へ偽 WindowHandle を付与し
    /// MonitorSnapshot を挿入した World を返す（char 窓は spawn が Anchored/WindowPos を付ける）。
    fn resnap_world() -> (World, GhostWindows) {
        let placements = resnap_placements();
        let mut world = World::new();
        let gw = spawn_ghost_windows(
            &mut world,
            &placements,
            &GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]),
        );
        // 偽 WindowHandle 付与（enqueue_window_set_pos が WindowPos を書けるように＝
        // resize_window_to の反映口が成立する条件）。
        let mut raw = 0x100usize;
        for scope in gw.scopes().collect::<Vec<_>>() {
            for e in [
                gw.char_window(scope).unwrap(),
                gw.balloon_window(scope).unwrap(),
            ] {
                world.entity_mut(e).insert(fake_handle(raw));
                raw += 0x10;
            }
        }
        // MonitorSnapshot（project_anchor Bottom が下端 live 算出に用いる）。
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![resnap_work_area()],
        });
        (world, gw)
    }

    fn size_of(world: &World, e: Entity) -> Option<SizeI> {
        world.get::<WindowPos>(e).and_then(|wp| wp.size)
    }
    fn pos_of(world: &World, e: Entity) -> Option<Point> {
        world.get::<WindowPos>(e).and_then(|wp| wp.position)
    }

    /// 1.3/3.1: 異寸→resize＋re-snap。shown_size が現 WindowPos.size と異なると、当該 char 窓の
    /// size が新寸・position が Anchored(Bottom) に沿って再射影される（y=下端−h'・x 保持）。
    #[test]
    fn resnap_from_sizes_drives_resize_and_resnap_on_size_change() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();
        assert_eq!(size_of(&world, char0), Some(SizeI::new(434, 687)), "前提: 初期寸");
        assert_eq!(
            pos_of(&world, char0),
            Some(Point { x: 1483, y: 757 }),
            "前提: 初期位置（bottom 不変量を満たす）"
        );

        // h 687→700 の異寸を注入。
        resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 700 })].into_iter());

        // 新寸へ更新され、Bottom 再射影で下端固定（y=1444−700=744・x 保持）。
        assert_eq!(size_of(&world, char0), Some(SizeI::new(434, 700)), "新寸へ更新");
        assert_eq!(
            pos_of(&world, char0),
            Some(Point { x: 1483, y: 744 }),
            "Bottom 再射影: y=work_area.bottom−h'（x 保持）"
        );
    }

    /// 3.1: 同寸→no-op。shown_size が現 WindowPos.size と同一なら resize は駆動されず窓状態不変。
    #[test]
    fn resnap_from_sizes_is_noop_on_same_size() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();
        let size_before = size_of(&world, char0);
        let pos_before = pos_of(&world, char0);

        // 現寸と同一（434×687）→ 冗長駆動を避ける（非発火）。
        resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 687 })].into_iter());

        assert_eq!(size_of(&world, char0), size_before, "同寸は size 不変");
        assert_eq!(pos_of(&world, char0), pos_before, "同寸は position 不変（非発火）");
    }

    /// 3.4: 非正/変換失敗→skip。非正寸（0・負）は resnap_from_sizes が弾き窓状態不変（二重防波堤）。
    #[test]
    fn resnap_from_sizes_skips_non_positive_sizes() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();
        let size_before = size_of(&world, char0);
        let pos_before = pos_of(&world, char0);

        for bad in [
            SizePx { w: 0, h: 687 },
            SizePx { w: 434, h: 0 },
            SizePx { w: -5, h: 687 },
            SizePx { w: 434, h: -5 },
        ] {
            resnap_from_sizes(&mut world, [(0usize, bad)].into_iter());
        }

        assert_eq!(size_of(&world, char0), size_before, "非正寸は skip（size 不変）");
        assert_eq!(pos_of(&world, char0), pos_before, "非正寸は skip（position 不変）");
    }

    /// 4.5: balloon で駆動しない。char 窓が resize されても同 scope の balloon 窓の
    /// WindowPos.size は不変（resnap_from_sizes は scope→char_window のみ写像し balloon に触れない）。
    #[test]
    fn resnap_from_sizes_never_resizes_balloon_window() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();
        let balloon0 = gw.balloon_window(0).unwrap();
        let balloon_size_before = size_of(&world, balloon0);
        assert_eq!(
            balloon_size_before,
            Some(SizeI::new(223, 158)),
            "前提: balloon 初期寸"
        );

        // char0 を異寸で駆動（balloon の寸へ仮に写せば 500×720 になるはずの値）。
        resnap_from_sizes(&mut world, [(0usize, SizePx { w: 500, h: 720 })].into_iter());

        // char は新寸へ（駆動された証拠）。
        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(500, 720)),
            "char は resize される"
        );
        // balloon の寸は不変（balloon を resize 対象にしていない・Req4.5）。
        assert_eq!(
            size_of(&world, balloon0),
            balloon_size_before,
            "balloon 窓の size は resnap で不変（scope→char_window のみ写像）"
        );
    }

    /// resnap が引く物理寸を target ごとに作り分ける fake（[`PhysicalSizeSource`] の檻用実装）。
    ///
    /// shell（偶数 id）と balloon（奇数 id）で**異なる寸**を返し、問い合わせられた `TargetId` を
    /// 記録する。これにより「resnap がどちらを読んだか」が窓ジオメトリと問い合わせ記録の
    /// 二重の観測面で判別できる（実 `EmoPresenter` は装着＋`ShowSurface` 完了＝GPU が要り、
    /// 未装着だと全 target が `None` に潰れて判別不能になる——それが変異生存の穴だった）。
    struct FakeSizes {
        shell: (u32, u32),
        balloon: (u32, u32),
        queried: std::cell::RefCell<Vec<u32>>,
    }

    impl FakeSizes {
        fn new(shell: (u32, u32), balloon: (u32, u32)) -> Self {
            FakeSizes {
                shell,
                balloon,
                queried: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl PhysicalSizeSource for FakeSizes {
        fn physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
            self.queried.borrow_mut().push(target.0);
            // target_map: shell=2*scope（偶数）／balloon=2*scope+1（奇数）。
            if target.0 % 2 == 0 {
                Some(self.shell)
            } else {
                Some(self.balloon)
            }
        }
    }

    /// 4.5（変異檻・2026-07-30 新設）: resnap は **shell target の物理寸**で char 窓を駆動し、
    /// balloon target のジオメトリは読まない。
    ///
    /// `shell_target(scope)` → `balloon_target(scope)` の 1 トークン変異を**排他的に殺す**。
    /// shell/balloon で寸を変えてあるため、変異すると char 窓が balloon 寸（223×158）へ
    /// 縮み、Bottom 再射影で y も 1444−158=1286 へ跳ぶ——寸法・位置の両方で判別できる。
    #[test]
    fn resnap_reads_shell_targets_only_and_ignores_balloon_geometry() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();
        let char1 = gw.char_window(1).unwrap();
        let balloon0 = gw.balloon_window(0).unwrap();
        let balloon_size_before = size_of(&world, balloon0);

        // shell=434×700（scope0 の初期寸 434×687 と異なる＝駆動される）／
        // balloon=223×158（fixture の balloon 実寸・変異したらこちらが char へ写る）。
        let fake = FakeSizes::new((434, 700), (223, 158));
        resnap_with(&fake, &mut world);

        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(434, 700)),
            "char0 は shell target の物理寸へ揃う（balloon 寸 223×158 なら変異）"
        );
        assert_eq!(
            pos_of(&world, char0),
            Some(Point { x: 1483, y: 744 }),
            "Bottom 再射影は shell 寸基準: y=1444−700（balloon 寸なら 1444−158=1286）"
        );
        assert_eq!(
            size_of(&world, char1),
            Some(SizeI::new(434, 700)),
            "char1 も shell target の物理寸で駆動される（scope 横断で同一判断）"
        );
        assert_eq!(
            size_of(&world, balloon0),
            balloon_size_before,
            "balloon 窓自体は書かれない（scope→char_window のみ写像・Req4.5）"
        );
    }

    /// 4.5（変異檻・2026-07-30 新設）: 問い合わせた `TargetId` 集合が shell だけであること。
    ///
    /// 上のジオメトリ檻と観測面を分ける——寸が偶然一致しても読み口の取り違えを捕まえる
    /// （兄弟の `dpi_phase_first_run_matches_all_windows_without_churn` と同じ技法）。
    #[test]
    fn resnap_queries_shell_targets_only() {
        let (mut world, _gw) = resnap_world();

        let fake = FakeSizes::new((434, 700), (223, 158));
        resnap_with(&fake, &mut world);

        let mut queried = fake.queried.borrow().clone();
        queried.sort_unstable();
        assert_eq!(
            queried,
            vec![shell_target(0).0, shell_target(1).0],
            "resnap が引く target は shell のみ（balloon_target {:?}/{:?} は一度も引かない）",
            balloon_target(0),
            balloon_target(1)
        );
    }

    /// アダプタ存在チェック: resnap_shell_targets を target 未装着の EmoPresenter::new()
    /// （target_physical_size 全 None）で呼ぶと全 scope skip の no-op（panic しない）・
    /// GhostWindows 未挿入でも安全。
    ///
    /// **注意**: 未装着 presenter は全 target が `None` ゆえ shell/balloon を判別できない
    /// （本テストは変異を殺さない）。読み口の取り違えは
    /// `resnap_reads_shell_targets_only_and_ignores_balloon_geometry` と
    /// `resnap_queries_shell_targets_only` が担う。
    #[test]
    fn resnap_shell_targets_is_noop_with_unattached_presenter() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();
        let size_before = size_of(&world, char0);
        let pos_before = pos_of(&world, char0);

        // 未装着 presenter＝text_slot_view 全 None → 全 scope skip（窓状態不変・panic しない）。
        let presenter = EmoPresenter::new();
        resnap_shell_targets(&presenter, &mut world);
        assert_eq!(size_of(&world, char0), size_before, "未装着は全 scope skip（size 不変）");
        assert_eq!(pos_of(&world, char0), pos_before, "未装着は全 scope skip（position 不変）");

        // GhostWindows 未挿入の素の World でも安全（no-op・panic しない）。
        let mut empty = World::new();
        resnap_shell_targets(&presenter, &mut empty);
    }

    // ── task 4.1: 検知→反映の一連のべき等（回帰檻・Req1.5/3.1） ──────────────────────
    //
    // 既存の移動専用経路（enqueue_window_set_pos／move_window_to／on_char_drag）は本 task で
    // 一切改変せず（follow.rs の move 系統合テスト群が無改変で緑＝単一ライター一般化の無影響）、
    // ここでは「寸法検知（差分判定）→窓反映（resize_window_to のべき等 skip）」の一連の流れが
    // 多重には効かないことを端から端まで固定する（Req3.1 の冗長回避・design「System Flows」
    // 同寸同アンカー非発火／「resnap_from_sizes」Postconditions 同寸 no-op）。

    /// 1.5/3.1（一連のべき等）: 寸法検知→窓反映の一連の流れが多重には効かないことを端から端まで
    /// 固定する。まず現寸と**異なる** shown_size で 1 回駆動して resize 発火（size 新寸・position
    /// Bottom 再射影）を確立し、続けて**同一**の shown_size を 2・3 回繰り返し駆動しても、1 回目
    /// 適用後の position・size が**一切変化しない**（冗長な再配置・再書込が起きない）ことを反証する。
    /// 空虚一致でないよう「1 回目で実際に size/position が変化した」ことも先に assert する。
    /// 96 非倍数の work area 辺・寸法（bottom=1444／h=700）で dpi/96 再スケール混入の檻とする。
    #[test]
    fn resnap_from_sizes_is_idempotent_across_repeats_after_a_size_change() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();

        // 前提: 初期寸・初期位置（bottom 不変量を満たす・96 非倍数 work area 由来）。
        let size_initial = size_of(&world, char0);
        let pos_initial = pos_of(&world, char0);
        assert_eq!(size_initial, Some(SizeI::new(434, 687)), "前提: 初期寸");
        assert_eq!(pos_initial, Some(Point { x: 1483, y: 757 }), "前提: 初期位置");

        // (1) 現寸と異なる h=700 を 1 回駆動 → resize 発火（前提の確立）。
        resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 700 })].into_iter());
        let size_after_first = size_of(&world, char0);
        let pos_after_first = pos_of(&world, char0);

        // 空虚一致でないことの担保: 1 回目で size・position が実際に変化した（新寸＋Bottom 再射影）。
        assert_eq!(size_after_first, Some(SizeI::new(434, 700)), "1 回目で新寸へ更新");
        assert_eq!(
            pos_after_first,
            Some(Point { x: 1483, y: 744 }),
            "1 回目で Bottom 再射影（y=1444−700=744・x 保持）"
        );
        assert_ne!(size_after_first, size_initial, "1 回目は実際に size が変化した（空虚でない）");
        assert_ne!(pos_after_first, pos_initial, "1 回目は実際に position が変化した（空虚でない）");

        // (2) 同一 shown_size を 2 回・3 回繰り返し駆動 → 窓の position・size が 1 回目適用後から
        //     一切変化しない（検知→反映の一連が多重には効かない＝冗長な再配置・再書込なし・Req3.1）。
        for repeat in 2..=3 {
            resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 700 })].into_iter());
            assert_eq!(
                size_of(&world, char0),
                size_after_first,
                "同寸 {repeat} 回目: size は 1 回目適用後から不変（多重には効かない）"
            );
            assert_eq!(
                pos_of(&world, char0),
                pos_after_first,
                "同寸 {repeat} 回目: position は 1 回目適用後から不変（非発火）"
            );
        }
    }

    /// 3.1（純べき等）: 最初から現寸と**同一**の shown_size を反復駆動しても、一度も窓状態が
    /// 変化しない（検知段の同寸 skip が毎回効く・冗長駆動ゼロ）。size・position の**両方**が不変
    /// であることを毎回見る。
    #[test]
    fn resnap_from_sizes_same_size_repeats_never_change_window_state() {
        let (mut world, gw) = resnap_world();
        let char0 = gw.char_window(0).unwrap();
        let size_before = size_of(&world, char0);
        let pos_before = pos_of(&world, char0);

        // 現寸（434×687）と同一を 3 回反復 → 毎回 no-op（窓状態不変・冗長駆動なし・Req3.1）。
        for repeat in 1..=3 {
            resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 687 })].into_iter());
            assert_eq!(size_of(&world, char0), size_before, "同寸反復 {repeat}: size 不変");
            assert_eq!(pos_of(&world, char0), pos_before, "同寸反復 {repeat}: position 不変");
        }
    }

    // ── task 9.2: run_move_drain_phase（frame 相 move drain→apply）の存在＋ゲート檻 ──────
    //
    // 9.1 の channel 到達（`move_cue_sink_reaches_emo2_wiring_receiver`）と 7.4 の apply 単体
    // （move_cue.rs `apply_move_tests`）を frame 相 drain で接ぐ結線の存在チェック。full spine
    // （cue→CueSheet→dispatch→sink→channel→frame）は task 9.3 が所有する。

    /// fixture `\1\![move,-353,,,0,base,base]` の `MoveDirective`（scope1・base scope0）。
    fn fixture_move_directive() -> MoveDirective {
        crate::emo2_boot::move_cue::parse_move_directive(
            1,
            &["-353", "", "", "0", "base", "base"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("fixture move は Ok")
    }

    /// 9.2 ゲート檻: `GhostWindows` 未挿入の間は `move_rx` を drain せず保留する（取りこぼしなし）。
    ///
    /// 素の `World`（`GhostWindows` なし）で `run_move_drain_phase` を呼んでも、送出済みの
    /// `MoveDirective` はチャネルに残る（後から test-support `drain_move_directives` で取り出せる＝
    /// gate 閉で未消費の実証）。move はキャラ窓生成後に一括適用され OnFirstBoot 移動を取りこぼさない。
    #[test]
    fn run_move_drain_phase_buffers_until_ghost_windows_present() {
        let (tx, rx) = mpsc::channel::<MoveDirective>();
        let wiring = Emo2Wiring::new(
            EmoPresenter::new(),
            mpsc::channel::<PresentCommand>().1,
            rx,
            Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
            zero_clock(),
            synth_assets(&[(0, 0)]),
        );
        tx.send(fixture_move_directive()).expect("送出は成功する（受信端は wiring 保持）");

        // GhostWindows 未挿入の素の World → drain せず保留（try_iter を呼ばない）。
        let mut world = World::new();
        run_move_drain_phase(&wiring, &mut world);

        // gate 閉ゆえ未消費: 送出した 1 件がチャネルに残る（保留＝取りこぼしなし）。
        let remaining = wiring.drain_move_directives();
        assert_eq!(
            remaining.len(),
            1,
            "GhostWindows 未挿入では drain せず保留する（取りこぼしなし）"
        );
        assert_eq!(remaining[0].scope, 1, "保留された directive は fixture（scope1）");
    }

    /// 9.2 apply 檻: `GhostWindows` 存在下で `move_rx` を drain すると `apply_move_directive` が
    /// 対象窓を fixture 検算位置へ即時移動する（channel→frame 相 drain→apply→窓移動の結線存在）。
    ///
    /// base scope0 (1483,757,434,687)・target scope1 (1049,1087,278,357)・x=Px(-353)・y=Fix:
    /// x' = 1483 + 434/2 − 353 − 278/2 = 1208・y は現状維持 1087（`resolve_move_target_position` 検算）。
    #[test]
    fn run_move_drain_phase_applies_directive_when_ghost_windows_present() {
        let (mut world, gw) = resnap_world();
        let target = gw.char_window(1).unwrap();
        assert_eq!(
            pos_of(&world, target),
            Some(Point { x: 1049, y: 1087 }),
            "前提: 移動前の scope1 初期位置"
        );

        let (tx, rx) = mpsc::channel::<MoveDirective>();
        let wiring = Emo2Wiring::new(
            EmoPresenter::new(),
            mpsc::channel::<PresentCommand>().1,
            rx,
            Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
            zero_clock(),
            synth_assets(&[(0, 0)]),
        );
        tx.send(fixture_move_directive()).expect("送出は成功する");

        run_move_drain_phase(&wiring, &mut world);

        // channel→drain→apply→move_window_to で対象窓が fixture 検算位置へ即時移動する。
        assert_eq!(
            pos_of(&world, target),
            Some(Point { x: 1208, y: 1087 }),
            "x'=1483+217−353−139=1208・y=Fix は現状維持（channel→frame drain→apply）"
        );
        // drain 済みチャネルは空（二重適用なし・FIFO 全件消費）。
        assert_eq!(
            wiring.drain_move_directives().len(),
            0,
            "drain 後チャネルは空（全件消費・二重適用なし）"
        );
    }

    // ── task 4.2: DPI 追従フェーズ（run_dpi_phase／窓寸 reconcile 二経路）の檻 ──────────
    //
    // 判断分岐（窓種別の判定・物理寸の算出・反映口の振り分け・エッジ観測の永続性・二経路の
    // 責任分界）を GPU 不要で決定論に固定する（design「Testing Strategy」振り分け基準 (a)・D9）。
    // GPU readback 檻（実 k 倍表示の寸法・バイト）は emo-present in-crate＝別プロセス側の領分
    // （R5.1/R5.3）ゆえここでは組まない——本ファイルへ 2 個目の Compositor を持ち込まない。
    //
    // 「書込ゼロ」の観測境界は follow.rs task 2.2 の檻と同一手法を用いる: `SetWindowPosCommand`
    // の TLS キューは wintf 私有で件数を覗く API が無く `flush()` は偽 HWND へ実 Win32 を撃つため
    // 使えない。代わりに **`Arrangement.offset` 同期**（`enqueue_window_set_pos` 内で enqueue と
    // 不可分に対で走る）を witness とし、sentinel が据え置かれたまま＝単一ライター経路を一度も
    // 通っていない＝窓書込 0 件の決定論的証拠とする。

    use areka_emo_compose::ScaleRatio;
    use wintf::ecs::layout::{Arrangement, Offset};
    use wintf::ecs::DPI;

    /// 単一ライター経路を通ったか否かの witness 用 sentinel（実位置と重ならない値）。
    const WRITER_WITNESS: Offset = Offset { x: -1.0, y: -1.0 };

    /// spawn 時 offset 付きの `Arrangement`（実 pipeline の spawn 位置を模す・follow.rs 檻と同型）。
    fn arrangement_at(x: f32, y: f32) -> Arrangement {
        Arrangement {
            offset: Offset { x, y },
            ..Default::default()
        }
    }

    /// entity の `Arrangement.offset` を読む（未付与は panic で検出）。
    fn arrangement_offset_of(world: &World, entity: Entity) -> Offset {
        world
            .get::<Arrangement>(entity)
            .expect("Arrangement があるはず")
            .offset
    }

    /// 単一ライター経路を通っていない＝窓書込ゼロ（sentinel が据え置かれている）。
    fn assert_no_write(world: &World, entity: Entity, what: &str) {
        assert_eq!(
            arrangement_offset_of(world, entity),
            WRITER_WITNESS,
            "{what}: 単一ライター経路を通った痕跡がある（書込ゼロのはず）"
        );
    }

    /// 全窓の書込 witness を sentinel へ戻す（フェーズ境界で「以降の書込」だけを見るため）。
    fn reset_write_witness(world: &mut World, gw: &GhostWindows) {
        for scope in gw.scopes().collect::<Vec<_>>() {
            for e in [
                gw.char_window(scope).expect("char 窓がある"),
                gw.balloon_window(scope).expect("balloon 窓がある"),
            ] {
                world
                    .entity_mut(e)
                    .insert(arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y));
            }
        }
    }

    /// resnap 檻の World（2 スコープ・偽 HWND・MonitorSnapshot）へ、書込 witness の
    /// `Arrangement`（sentinel）と `DPI`（96＝author_dpi 既定と等倍）を全窓へ付与した DPI 相の檻。
    fn dpi_world() -> (World, GhostWindows) {
        let (mut world, gw) = resnap_world();
        for scope in gw.scopes().collect::<Vec<_>>() {
            for e in [
                gw.char_window(scope).expect("char 窓がある"),
                gw.balloon_window(scope).expect("balloon 窓がある"),
            ] {
                world.entity_mut(e).insert((
                    arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
                    DPI::from_dpi(96, 96),
                ));
            }
        }
        (world, gw)
    }

    /// 決定論 fake の再スケール報告源（GPU 不要で**二経路の結線**を檻に入れる）。
    ///
    /// `EmoPresenter` の消費規約（`refresh_scale` が再表示成立時に自ら `pending_resize` を
    /// take して返す／ゲート不成立なら一切触れない）を写した最小の fake。実 presenter 側の
    /// 規約そのものは emo-present in-crate テストが所有し、ここでは **frame 側の結線**
    /// （両経路を毎フレーム呼ぶ・`Some` のみ reconcile する）だけを見る。
    #[derive(Default)]
    struct FakeReports {
        /// `refresh_scale_report` が返す報告（target 番号→物理寸・取り出しで消える）。
        refresh: BTreeMap<u32, (u32, u32)>,
        /// `take_scale_report` が返す未消費報告（同上）。
        pending: BTreeMap<u32, (u32, u32)>,
        /// 呼出記録（`("refresh"|"take", target 番号)`・呼ばれたこと自体の非空虚性検査用）。
        calls: Vec<(&'static str, u32)>,
    }

    impl FakeReports {
        /// 指定種別の呼出だけを target 番号の列として取り出す。
        fn calls_of(&self, kind: &str) -> Vec<u32> {
            self.calls
                .iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, t)| *t)
                .collect()
        }
    }

    impl ScaleReportSource for FakeReports {
        fn refresh_scale_report(
            &mut self,
            _world: &mut World,
            target: TargetId,
        ) -> Option<(u32, u32)> {
            self.calls.push(("refresh", target.0));
            let report = self.refresh.remove(&target.0);
            if report.is_some() {
                // presenter 規約: 再表示が成立したなら状態照合が積んだ要求は本メソッドが消費する
                // （同一フレームの drain が二度目を拾わない）。
                self.pending.remove(&target.0);
            }
            report
        }

        fn take_scale_report(&mut self, target: TargetId) -> Option<(u32, u32)> {
            self.calls.push(("take", target.0));
            self.pending.remove(&target.0)
        }
    }

    /// 窓種別の判定（`spawn.rs` の marker から・純関数）: char のみ／balloon のみ／どちらでもない／
    /// 両方同居（結線バグ）の 4 分岐を全網羅する。
    #[test]
    fn classify_ghost_window_covers_all_marker_combinations() {
        assert_eq!(
            classify_ghost_window(Some(3), None),
            GhostWindowClass::Ghost(3, GhostWindowKind::Char),
            "CharWindowMarker のみ → キャラ窓（scope 保持）"
        );
        assert_eq!(
            classify_ghost_window(None, Some(7)),
            GhostWindowClass::Ghost(7, GhostWindowKind::Balloon),
            "BalloonWindowMarker のみ → バルーン窓（scope 保持）"
        );
        assert_eq!(
            classify_ghost_window(None, None),
            GhostWindowClass::NotGhost,
            "どちらの marker も無い窓は DPI 相の対象外"
        );
        assert_eq!(
            classify_ghost_window(Some(0), Some(0)),
            GhostWindowClass::Ambiguous,
            "両 marker 同居は spawn の排他付与に反する結線バグ（縮退させる）"
        );
    }

    /// 反映口の振り分け（D8）: **char 窓は `resize_window_to`**（アンカー保存＝Bottom 再射影で
    /// 位置が動く）・**balloon 窓は `resize_window_keep_position`**（位置維持）。観測可能な差
    /// （position が動く／動かない）で振り分けを反証する。
    #[test]
    fn reconcile_window_size_routes_char_to_anchor_resize_and_balloon_to_keep_position() {
        let (mut world, gw) = dpi_world();
        let char0 = gw.char_window(0).expect("char 窓がある");
        let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");

        // --- balloon を先に見る（char の resize は BalloonFollow で balloon を動かすため）---
        assert_eq!(
            pos_of(&world, balloon0),
            Some(Point { x: 1071, y: 732 }),
            "前提: balloon 初期位置"
        );
        assert!(
            reconcile_window_size(&mut world, balloon0, GhostWindowKind::Balloon, (446, 316)),
            "balloon: 異寸ゆえ書込が成立する"
        );
        assert_eq!(
            size_of(&world, balloon0),
            Some(SizeI::new(446, 316)),
            "balloon: 新物理寸へ更新"
        );
        assert_eq!(
            pos_of(&world, balloon0),
            Some(Point { x: 1071, y: 732 }),
            "balloon: 位置は維持される（resize_window_keep_position＝アンカー再射影しない）"
        );

        // --- char: アンカー保存リサイズ（Bottom 再射影で y と中央 x が動く）---
        assert_eq!(
            pos_of(&world, char0),
            Some(Point { x: 1483, y: 757 }),
            "前提: char 初期位置"
        );
        assert!(
            reconcile_window_size(&mut world, char0, GhostWindowKind::Char, (868, 1374)),
            "char: 異寸ゆえ書込が成立する"
        );
        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(868, 1374)),
            "char: 新物理寸へ更新"
        );
        assert_eq!(
            pos_of(&world, char0),
            Some(Point { x: 1266, y: 70 }),
            "char: Bottom 再射影（y=1444−1374=70）＋下端中央保存（x=1483+217−434=1266）"
        );
    }

    /// 縮退（log-first・panic しない）: i32 域超過・0 寸は窓へ書かない。同寸はべき等 skip で
    /// 書込ゼロ（`false` は失敗ではない）。
    #[test]
    fn reconcile_window_size_guards_and_idempotent_skip_write_nothing() {
        let (mut world, gw) = dpi_world();
        let char0 = gw.char_window(0).expect("char 窓がある");
        let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");

        // i32 域超過（u32 なら表現できるが窓寸に渡せない）→ 書かない。
        assert!(!reconcile_window_size(
            &mut world,
            char0,
            GhostWindowKind::Char,
            (u32::MAX, 687)
        ));
        // 0 寸（native 0 由来の退化）→ 書かない。
        assert!(!reconcile_window_size(
            &mut world,
            char0,
            GhostWindowKind::Char,
            (0, 687)
        ));
        assert!(!reconcile_window_size(
            &mut world,
            balloon0,
            GhostWindowKind::Balloon,
            (446, 0)
        ));
        // 同寸（k 不変で丸め後も同寸）→ べき等 skip（false は失敗でなく「書かなかった」）。
        assert!(!reconcile_window_size(
            &mut world,
            char0,
            GhostWindowKind::Char,
            (434, 687)
        ));
        assert!(!reconcile_window_size(
            &mut world,
            balloon0,
            GhostWindowKind::Balloon,
            (223, 158)
        ));

        assert_eq!(size_of(&world, char0), Some(SizeI::new(434, 687)), "char 寸不変");
        assert_eq!(
            size_of(&world, balloon0),
            Some(SizeI::new(223, 158)),
            "balloon 寸不変"
        );
        assert_no_write(&world, char0, "縮退・べき等 skip");
        assert_no_write(&world, balloon0, "縮退・べき等 skip");
    }

    /// **本 task の到達判定（tasks.md 4.2）**: 窓 DPI を差し替えた次のフェーズ実行で、当該窓の
    /// client が `scaled_extent(applied, native)` と一致する。
    ///
    /// 96→192（k=2/1）へ `DPI` を差し替え、presenter が報告する新物理寸として
    /// `ScaleRatio::scaled_extent(native)` を与える（実 presenter の報告値はこの丸め権威で
    /// 作られる——emo-present in-crate が所有する契約）。`dpi_phase_with` 一回で char 窓の
    /// `WindowPos.size` が同一の `scaled_extent` に一致することを反証する（Req3.1/4.1/4.2）。
    #[test]
    fn dpi_phase_reconciles_changed_window_to_scaled_extent() {
        let (mut world, gw) = dpi_world();
        let char0 = gw.char_window(0).expect("char 窓がある");
        let native = (434u32, 687u32);
        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(native.0 as i32, native.1 as i32)),
            "前提: 窓 client は k=1 相当の native 寸"
        );

        // 窓 DPI 192（k = 192/96 = 2/1）へ差し替え → Changed<DPI> 発火。
        world.entity_mut(char0).insert(DPI::from_dpi(192, 192));
        let k = ScaleRatio::new(192, 96).expect("非ゼロ比");
        let scaled = k.scaled_extent(native.0, native.1);

        let mut source = FakeReports::default();
        source.refresh.insert(shell_target(0).0, scaled);
        let mut state = None;
        dpi_phase_with(&mut source, &mut state, &mut world);

        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(scaled.0 as i32, scaled.1 as i32)),
            "DPI 差替後の同一フレームで窓 client＝scaled_extent(applied, native)"
        );
        assert_eq!(scaled, (868, 1374), "k=2/1・native 434×687 の検算値");
        assert!(
            source.calls_of("refresh").contains(&shell_target(0).0),
            "非空虚性: 当該窓の shell target に対し refresh が呼ばれた"
        );
    }

    /// 二経路の責任分界 (1)（**二重 resize しない**）: `refresh_scale` が再表示に成立して報告を
    /// 返した場合、その要求は presenter 自身が消費済みであり、同一フレームの drain 相 reconcile は
    /// **窓へ一切書かない**（sentinel をフェーズ境界で戻して「以降の書込」だけを見る）。
    #[test]
    fn drain_reconcile_writes_nothing_when_refresh_already_consumed_the_report() {
        let (mut world, gw) = dpi_world();
        let char0 = gw.char_window(0).expect("char 窓がある");
        world.entity_mut(char0).insert(DPI::from_dpi(192, 192));

        let mut source = FakeReports::default();
        // 状態照合が積んだ要求（pending）と、再表示成立で返る報告（refresh）は**同一の 1 件**。
        source.refresh.insert(shell_target(0).0, (868, 1374));
        source.pending.insert(shell_target(0).0, (868, 1374));

        let mut state = None;
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(868, 1374)),
            "DPI 相で reconcile 済み（非空虚性の前提）"
        );

        // フェーズ境界: witness を戻し、以降（drain 相）の書込だけを観測する。
        reset_write_witness(&mut world, &gw);
        reconcile_reported_sizes(&mut source, &mut world);

        assert!(
            source.calls_of("take").contains(&shell_target(0).0),
            "非空虚性: drain 相は take を実際に呼んでいる（呼んだ上で None だった）"
        );
        assert_no_write(&world, char0, "drain 相の二重 resize");
        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(868, 1374)),
            "窓寸は DPI 相の結果のまま（二重適用なし）"
        );
    }

    /// 二経路の責任分界 (2)（**取りこぼさない**・design Flow 3 手順 5）: `refresh_scale` の
    /// ゲートが不成立で報告が返らなくても、表示成立点が積んだ未消費要求（初回表示の k₀ 補正）は
    /// drain 相の reconcile が同一フレーム内で拾って窓寸へ反映する。
    #[test]
    fn drain_reconcile_applies_undrained_report_when_refresh_gate_fails() {
        let (mut world, gw) = dpi_world();
        let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");

        let mut source = FakeReports::default();
        // refresh は空（k 不変等でゲート不成立＝報告なし）・pending のみ未消費で残る。
        source.pending.insert(balloon_target(0).0, (279, 198));

        let mut state = None;
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert_no_write(&world, balloon0, "DPI 相はゲート不成立ゆえ書かない");

        reconcile_reported_sizes(&mut source, &mut world);
        assert_eq!(
            size_of(&world, balloon0),
            Some(SizeI::new(279, 198)),
            "未消費の要求を drain 相が拾って窓 client へ反映（取りこぼしなし）"
        );
        assert_eq!(
            pos_of(&world, balloon0),
            Some(Point { x: 1071, y: 732 }),
            "balloon は位置維持（resize_window_keep_position 経路）"
        );
    }

    /// 初回 run の全窓マッチ（`SystemState::new` 仕様）は churn を生まない: 報告が無ければ
    /// 窓書込ゼロ。ただし**全窓に対し refresh が実際に呼ばれている**ことも同時に見る（空虚な
    /// 「何も起きなかった」で通さない）。
    #[test]
    fn dpi_phase_first_run_matches_all_windows_without_churn() {
        let (mut world, gw) = dpi_world();
        let mut source = FakeReports::default(); // 報告なし＝k 差分なし相当
        let mut state = None;

        dpi_phase_with(&mut source, &mut state, &mut world);

        // 初回 run は全窓（2 スコープ×char/balloon＝4 target）へマッチする（非空虚性）。
        let mut refreshed = source.calls_of("refresh");
        refreshed.sort_unstable();
        assert_eq!(
            refreshed,
            vec![
                shell_target(0).0,
                balloon_target(0).0,
                shell_target(1).0,
                balloon_target(1).0
            ]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>(),
            "初回 run は全ゴースト窓へマッチする（SystemState::new 仕様）"
        );
        // 報告が無ければ窓へは一切書かない（べき等 skip と合わせて churn ゼロ）。
        for scope in [0usize, 1] {
            assert_no_write(&world, gw.char_window(scope).unwrap(), "初回 run churn");
            assert_no_write(&world, gw.balloon_window(scope).unwrap(), "初回 run churn");
        }
    }

    /// `Changed<DPI>` が無いフレームは**仕事をしない**: 2 回目の run では refresh を一度も呼ばず
    /// 窓書込もゼロ（永続 `SystemState` が `last_run` を跨いで保つ＝毎フレーム全マッチしない）。
    #[test]
    fn dpi_phase_without_dpi_change_does_no_work() {
        let (mut world, gw) = dpi_world();
        let mut source = FakeReports::default();
        let mut state = None;

        // 1 回目（初回 run の全マッチを消費）。
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert!(
            !source.calls_of("refresh").is_empty(),
            "非空虚性: 1 回目は実際にマッチしている"
        );

        // 2 回目: DPI を一切触っていない → マッチ 0 件＝refresh 呼出ゼロ・窓書込ゼロ。
        source.calls.clear();
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert!(
            source.calls_of("refresh").is_empty(),
            "Changed<DPI> 無しのフレームは refresh を呼ばない（実質 no-op）: {:?}",
            source.calls
        );
        for scope in [0usize, 1] {
            assert_no_write(&world, gw.char_window(scope).unwrap(), "変化なしフレーム");
            assert_no_write(
                &world,
                gw.balloon_window(scope).unwrap(),
                "変化なしフレーム",
            );
        }

        // 3 回目: 1 窓だけ DPI を差し替える → その窓だけがマッチする（検知が生きている証拠）。
        let char1 = gw.char_window(1).expect("char 窓がある");
        world.entity_mut(char1).insert(DPI::from_dpi(144, 144));
        source.calls.clear();
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert_eq!(
            source.calls_of("refresh"),
            vec![shell_target(1).0],
            "変化した窓の target だけが refresh 対象"
        );
    }

    /// author_dpi の引き当て（取り違え防止）: shell target には shell 宣言・balloon target には
    /// balloon 宣言が渡る。両者 `u16` ゆえ取り違えてもコンパイルは通る——**異なる値**で引き当てを
    /// 反証する（入れ替えれば必ず落ちる）。未知 target は既定 96 へ縮退する（panic しない）。
    #[test]
    fn author_dpis_pairs_shell_and_balloon_declarations() {
        let assets = synth_assets(&[(0, 0)]);
        let plan = plan_attachments(&[0usize], &assets);
        let item = &plan.items[0];
        // shell=120（125% 原稿）・balloon=72（意図的に異なる値・入れ替え検出用）。
        let dpis = AuthorDpis {
            shell: 120,
            balloon: 72,
        };

        assert_eq!(
            dpis.for_target(item, item.shell_target),
            120,
            "shell target には shell_author_dpi が渡る"
        );
        assert_eq!(
            dpis.for_target(item, item.balloon_target),
            72,
            "balloon target には balloon_author_dpi が渡る"
        );
        assert_eq!(
            dpis.for_target(item, TargetId(9999)),
            96,
            "当該 scope のいずれの target でもない＝結線バグ → 既定 96 へ縮退（panic しない）"
        );
    }

    // ── task 7.2: 文字層 k 追従フェーズ（run_text_scale_phase・D11-3/D11-4・R8.1/8.5/8.6） ──
    //
    // 本番関数をそのまま駆動する（シームを噛ませない）。`Some(view)` の適用そのもの（binding
    // 再構築・供給面破棄・churn ガード）は GPU 装着を要するため spine（in-crate GPU ハーネス）が
    // 実経路で檻に入れる。ここでは GPU 不要で観測できる 2 点——(a) 走査対象が balloon 装着 scope に
    // 限られること、(b) 表示未確立の縮退が **scope ごとに一度だけ** 鳴ること——を固定する。

    /// 素の結線資源（GPU/資産不要・`run_text_scale_phase` の headless 駆動用）。
    fn headless_wiring() -> Emo2Wiring {
        headless_wiring_with(mpsc::channel::<PresentCommand>().1, zero_clock())
    }

    /// 捕捉行のうち `level` かつ本文に `needle` を含む件数（他フェーズの警告と混ざらないよう絞る）。
    fn count_level_containing(logs: &[String], level: &str, needle: &str) -> usize {
        let lv = format!("level={level}");
        logs.iter()
            .filter(|l| l.contains(&lv) && l.contains(needle))
            .count()
    }

    /// balloon 未装着（`balloon_models` 空）では走査対象がゼロ＝完全 no-op（警告も出さない）。
    ///
    /// 「毎フレーム走査」が attach 前のフレームで鳴き続けないこと（起動直後の log 汚染の禁止）と、
    /// shell しか無い状況で文字層へ触れないことを同時に固定する。
    #[test]
    fn text_scale_phase_without_balloon_models_is_silent_noop() {
        let mut wiring = headless_wiring();

        let logs = capture_logs(|| {
            assert!(
                run_text_scale_phase(&mut wiring).is_empty(),
                "balloon 未装着では再構築 scope なし"
            );
        });

        assert_eq!(count_level(&logs, "WARN"), 0, "attach 前は何も鳴らさない: {logs:?}");
        assert_eq!(count_level(&logs, "ERROR"), 0, "attach 前は何も鳴らさない: {logs:?}");
    }

    /// R8.6 縮退（log-first だが log spam にしない）: `text_slot_view` が `None`（表示未確立）の
    /// scope は再追従せず skip し、**警告は scope ごとに一度だけ**鳴る（毎フレーム走査ゆえ素朴な
    /// `warn!` は毎フレーム鳴ってしまう）。
    ///
    /// 実源（何も装着していない `EmoPresenter`＝`text_slot_view` が常に `None`）へ、attach 相と同じ
    /// 形で per-scope の [`BalloonModel`] を記憶させた状態を作り、3 フレーム相当を走らせる。
    #[test]
    fn text_scale_phase_warns_once_per_scope_when_view_unavailable() {
        let mut wiring = headless_wiring();
        // attach 相が記憶するのと同じ形（scope→model）。presenter は未装着ゆえ view は常に None。
        wiring
            .balloon_models
            .insert(0, areka_parsers::balloon::parse_str("", None));
        wiring
            .balloon_models
            .insert(1, areka_parsers::balloon::parse_str("", None));

        let first = capture_logs(|| {
            assert!(
                run_text_scale_phase(&mut wiring).is_empty(),
                "view None では再構築しない（縮退 skip・R8.6）"
            );
        });
        assert_eq!(
            count_level(&first, "WARN"),
            2,
            "初回は縮退した scope ごとに 1 回ずつ鳴る（0 と 1 の 2 件・R8.6 の観測可能性）: {first:?}"
        );

        // 2・3 フレーム目: 状態が変わっていない以上、同じ警告を鳴らし直さない（log spam の禁止）。
        let rest = capture_logs(|| {
            run_text_scale_phase(&mut wiring);
            run_text_scale_phase(&mut wiring);
        });
        assert_eq!(
            count_level(&rest, "WARN"),
            0,
            "同一状態が続く間は再度鳴らさない（エッジガード）: {rest:?}"
        );
        assert_eq!(count_level(&rest, "ERROR"), 0, "縮退は失敗ではない: {rest:?}");

        // 借用/poison を残さない（None 経路は runtime に触れない）。
        assert!(wiring.runtime.try_borrow_mut().is_ok(), "runtime を汚さない");
    }

    /// **排他 system への組み込み**（call-site の檻）: [`emo2_frame_system`] は毎フレーム
    /// [`run_text_scale_phase`] を駆動する。
    ///
    /// 関数が正しくても system から呼ばれていなければ本番では何も起きない——その 1 行の欠落を
    /// 検出する。未装着 presenter（`text_slot_view` が常に `None`）＋ attach 相と同形の記憶済み
    /// [`BalloonModel`] を持つ World で 1 フレーム回すと R8.6 の縮退警告がちょうど 1 回鳴り、
    /// 2 フレーム目は鳴らない（＝呼ばれている、かつエッジガードが system 越しに効いている）。
    #[test]
    fn emo2_frame_system_drives_text_scale_phase_every_frame() {
        let (mut world, _gw) = dpi_world();
        let (_tx, rx) = mpsc::channel::<PresentCommand>();
        let mut wiring = headless_wiring_with(rx, zero_clock());
        // attach 相が記憶するのと同じ形（scope→model）。GPU 資源が無いため attach 相自体は空回りする。
        wiring
            .balloon_models
            .insert(0, areka_parsers::balloon::parse_str("", None));
        world.insert_non_send_resource(wiring);

        let first = capture_logs(|| emo2_frame_system(&mut world));
        assert_eq!(
            count_level_containing(&first, "WARN", "text-scale"),
            1,
            "1 フレーム目で文字層 k 追従フェーズが駆動され縮退が 1 回鳴る（system 組み込みの証跡）: {first:?}"
        );

        let second = capture_logs(|| emo2_frame_system(&mut world));
        assert_eq!(
            count_level_containing(&second, "WARN", "text-scale"),
            0,
            "2 フレーム目は同一状態ゆえ鳴らない（エッジガードが system 越しに効く）: {second:?}"
        );
        assert!(
            world.get_non_send_resource::<Emo2Wiring>().is_some(),
            "wiring は remove→insert で必ず戻る"
        );
    }

    /// **本番経路**での `SystemState` 永続性（Flow 2 キー決定 (b)・churn 禁止）: `run_dpi_phase` は
    /// 観測器を `Emo2Wiring.dpi_state` へ**保持**し、run を跨いで `last_run` を進める。
    ///
    /// `dpi_phase_with` へテスト側の state を渡す檻では「本番が wiring のフィールドを使っている」
    /// ことを一切見ないため、ここでは `run_dpi_phase(&mut wiring, ..)` だけを叩き、その**副作用**
    /// （`wiring.dpi_state` の生成と `last_run` の前進）を private フィールド越しに観測する。
    ///
    /// 非空虚性の核: 同一 World で**新規** `SystemState` を作ると全窓（4 窓）へマッチする——
    /// すなわち「毎 run 作り直す実装」は毎フレーム全窓を refresh する churn になる。本番の
    /// 永続観測器が 0 件であることと対にして、永続性が実際に効いていることを弁別する。
    #[test]
    fn run_dpi_phase_persists_system_state_across_frames_in_production_path() {
        let (mut world, gw) = dpi_world();
        let (_tx, rx) = mpsc::channel::<PresentCommand>();
        let mut wiring = headless_wiring_with(rx, zero_clock());
        assert!(
            wiring.dpi_state.is_none(),
            "前提: 観測器は初回 run まで未生成（SystemState::new は &mut World を要する）"
        );

        // 1 フレーム目（本番経路）: 初回 run の全窓マッチをここで消費する。
        run_dpi_phase(&mut wiring, &mut world);
        assert!(
            wiring.dpi_state.is_some(),
            "run_dpi_phase は観測器を wiring へ保持しなければならない（毎 run 作り直せば churn）"
        );

        // 1 フレーム目の直後: DPI を一切触っていないので、永続観測器のマッチは 0 件。
        let matched_after_first = wiring
            .dpi_state
            .as_mut()
            .expect("生成済み")
            .get(&world)
            .iter()
            .count();
        assert_eq!(
            matched_after_first, 0,
            "永続観測器は初回 run で Changed を消費済み＝以降はマッチしない"
        );

        // 非空虚性: 同一 World で新規 SystemState を作れば全 4 窓へマッチする（＝作り直し実装の churn）。
        let mut fresh: SystemState<DpiChangedQuery> = SystemState::new(&mut world);
        assert_eq!(
            fresh.get(&world).iter().count(),
            4,
            "新規 SystemState は全窓へマッチする（この差が永続性の効果そのもの）"
        );

        // 永続観測器は「変化しなくなった」のではなく、実変化はきちんと拾う（恒久的な盲目でない）。
        world
            .entity_mut(gw.char_window(1).expect("char 窓がある"))
            .insert(DPI::from_dpi(144, 144));
        let matched_after_change = wiring
            .dpi_state
            .as_mut()
            .expect("生成済み")
            .get(&world)
            .iter()
            .count();
        assert_eq!(
            matched_after_change, 1,
            "実際に DPI が変わった 1 窓だけを拾う（検知が生きている）"
        );
    }

    /// 結線の疎通（run_dpi_phase／emo2_frame_system）: 実 `EmoPresenter`（未装着）と `Changed<DPI>`
    /// のある World で `emo2_frame_system` を回しても、報告源が何も返さないため窓書込はゼロで
    /// panic しない。フェーズが排他 system へ組み込まれていること自体を固定する。
    #[test]
    fn emo2_frame_system_runs_dpi_phase_without_writes_when_unattached() {
        let (mut world, gw) = dpi_world();
        let (_tx, rx) = mpsc::channel::<PresentCommand>();
        world.insert_non_send_resource(headless_wiring_with(rx, zero_clock()));

        // DPI 差替（Changed 発火）→ 排他 system を 2 フレーム回す。
        world
            .entity_mut(gw.char_window(0).unwrap())
            .insert(DPI::from_dpi(192, 192));
        emo2_frame_system(&mut world);
        emo2_frame_system(&mut world);

        assert!(
            world.get_non_send_resource::<Emo2Wiring>().is_some(),
            "wiring は remove→insert で必ず戻る"
        );
        for scope in [0usize, 1] {
            assert_no_write(&world, gw.char_window(scope).unwrap(), "未装着 presenter");
            assert_no_write(
                &world,
                gw.balloon_window(scope).unwrap(),
                "未装着 presenter",
            );
        }
    }
}
