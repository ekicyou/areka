//! 毎フレーム三相結線（attach／drain／text）の排他 system と NonSend 配線状態。
//!
//! `Emo2Wiring`（NonSend resource・presenter／rx／runtime／clock／assets／attached を保持）と
//! 排他 system `emo2_frame_system(world: &mut World)`（donor パターン: remove→3 フェーズ→insert）を
//! 所有する。三フェーズ:
//! - attach: GPU 資源＋`GhostWindows` 到達ゲート→`plan_attachments`（DD-12）→バルーン初回 `ShowSurface`
//!   （面0）→文字層スロット取得→`register_actor_view`（`Option::take` で高々 1 回消費）。**シェルは初回
//!   `ShowSurface` を発行せず**最初のさくらスクリプト `\s` cue まで非表示を保つ（defect #5・実機#5）。
//! - drain: attach 完了後のみ `Receiver::try_iter` で `PresentCommand` を FIFO で `presenter.apply` へ適用。
//! - text: `TalkClock::talk_time` が `Some` のとき `present_frame` を呼ぶ（`Err` は `error!`＋継続）。
//!
//! `plan_attachments`（`GhostWindows::scopes()` を正とする純関数・DD-12）も本モジュールに属する。
//!
//! 本ファイルは task 3 の純関数 `plan_attachments`（＋`AttachPlan`／`PlannedAttach`）、task 4.1 の
//! NonSend 結線資源 `Emo2Wiring` と attach フェーズ（`run_attach_phase`＋補助 `connect_balloon_text`）、
//! そして task 4.2 の drain フェーズ（`run_drain_phase`）・text フェーズ（`run_text_phase`＋純判断
//! `resolve_talk_time`）・排他 system `emo2_frame_system`（remove→3 フェーズ→insert）を実装する。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::{debug, error, info, warn};

use areka_emo_present::{EmoPresenter, PresentCommand, TargetId, TextSlotView};
use areka_emo_text::actor::{present_frame, TextLayerRuntime};
use areka_parsers::balloon::BalloonModel;
use areka_sakura::ActorKey;
use wintf::ecs::{FrameTime, GraphicsCore, SizeI, WindowPos, WucGraphicsResource};

use crate::placement::follow::resize_window_to;
use crate::placement::resolver::SizePx;
use crate::placement::spawn::GhostWindows;

use super::assets::{BootAssets, ScopeAssets};
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
                let balloon_index = assets.balloons.iter().position(|b| b.0 == scope);
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
    /// talk 起点相対秒の時刻源（task 4.2 の text フェーズで `talk_time` を引く）。
    clock: TalkClock,
    /// load-time 構築資産（attach で `take` して高々 1 回消費）。
    assets: Option<BootAssets>,
    /// attach 完了フラグ（高々 1 回のゲート・以降 no-op）。
    attached: bool,
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
            clock,
            assets: Some(assets),
            attached: false,
        }
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
        balloon_model,
        // resolver は attach では未使用（seriko へは wire_emo2_boot=task 5.1 が手渡す）。
        resolver: _,
        // static_binds は attach では未使用（defect #5・2026-07-13 実機#5）: シェル初回表示を attach で
        // 焼き付けなくなったため。起動時オンの bindgroup default は seriko が保持し（spawn_seriko へ
        // 手渡し済み）、最初の `\s` cue が駆動する Show{shell,id,binds=static_binds} に載って表示層へ届く。
        static_binds: _,
    } = assets;
    let mut shells: Vec<_> = shells.into_iter().map(Some).collect();
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
        let Some((_, balloon_world, balloon_atlas)) =
            balloons.get_mut(balloon_index).and_then(|b| b.take())
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
        ) {
            error!(scope, error = %e, "emo2 attach: バルーン target の attach に失敗（log-first・継続）");
            continue;
        }
        // バルーン初回表示は面 0・bind なし（DD-9・R4.1 の「初回サーフェス表示＝バルーン枠表示」）。
        wiring.presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: item.balloon_target,
                surface_id: 0,
                binds: areka_emo_compose::BindSet::default(),
                reply: None,
            },
        );
        // apply は同期ゆえ同一フレームで text_slot_view が Some になるのが正常経路（DD-4）。
        // None（上流の遅延化）は接続せず次フレーム再試行へ委ねる（R4.2）。
        let view = wiring.presenter.text_slot_view(item.balloon_target);
        connect_balloon_text(
            &wiring.runtime,
            view,
            ActorKey::from(scope.to_string()),
            &balloon_model,
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
    // scope 識別は GhostWindows 経由（Req4.5）。未挿入は shell 寸を引く対象が無い＝no-op。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return;
    };
    // presenter 借用を解いてから resnap_from_sizes（&mut World）を呼ぶため、先に collect する。
    let mut sizes: Vec<(usize, SizePx)> = Vec::new();
    for scope in ghost_windows.scopes() {
        // shell target（偶数=2*scope）のみを読む（balloon_target は読まない＝shell 限定・Req4.5）。
        let Some(view) = presenter.text_slot_view(shell_target(scope as u32)) else {
            // 初回 ShowSurface 前＝未表示 → skip（no-op・遅延化への防御）。
            continue;
        };
        let (w, h) = view.surface_size(); // emo-present 適用点の実寸（Req4.1・古い寸で駆動しない）。
        // (u32,u32)→i32 変換失敗は skip（Req3.4）。
        let (Ok(w), Ok(h)) = (i32::try_from(w), i32::try_from(h)) else {
            debug!(scope, w, h, "resnap: 実寸の i32 変換に失敗 → skip（Req3.4）");
            continue;
        };
        // 0 は skip（try_from(0)=Ok(0) ゆえ明示的に弾く・Req3.4）。負値は u32 起点ゆえ生じない。
        if w == 0 || h == 0 {
            debug!(scope, "resnap: 実寸が 0 → skip（Req3.4・try_from(0)=Ok を明示的に弾く）");
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
    // donor 慣行: remove して &mut World を各フェーズへ排他に渡し、3 フェーズ駆動後に必ず戻す。
    run_attach_phase(&mut wiring, world);
    run_drain_phase(&mut wiring, world);
    // `\![move]` の末端結線: talk スレッドの MoveCueSink から届いた MoveDirective を drain し
    // apply_move_directive で実窓へ即時反映する（GhostWindows ゲート・R5・task 9.2）。present drain
    // とは独立で、GPU attach でなく GhostWindows 存在を待つ（move はキャラ窓 entity へ作用するため）。
    run_move_drain_phase(&wiring, world);
    // drain（全 PresentCommand 適用）後に shell サーフェス寸法の変化を検知し、変化した char 窓のみ
    // アンカー再適用を駆動する（適用後の実寸を読むため drain の**後**・同一 World・同一 tick 内の
    // 直接呼び・Req4.1/4.3/1.3）。text の前後とは機能的に無関係だが drain の後であることが必須。
    resnap_shell_targets(&wiring.presenter, world);
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
    use areka_seriko::SurfaceResolver;
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    use super::*;
    use crate::emo2_boot::assets::{BootAssets, ScopeAssets};

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
    /// `balloons[*].0` のみ。残りのフィールド（emo_world／atlas／balloon_model／resolver／
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
                .map(|&scope| (scope, empty_world(), empty_atlas()))
                .collect(),
            balloon_model: areka_parsers::balloon::parse_str("", None),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: BindSet::default(),
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

        // 冪等: 再試行してもゲート不成立なら未装着のまま・panic しない・資産も保持する。
        run_attach_phase(&mut wiring, &mut world);
        assert!(!wiring.attached, "再試行でもゲート不成立なら未装着");
        assert!(wiring.assets.is_some(), "再試行でも assets を保持");
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

    /// アダプタ存在チェック: resnap_shell_targets を target 未装着の EmoPresenter::new()
    /// （text_slot_view 全 None）で呼ぶと全 scope skip の no-op（panic しない）・GhostWindows
    /// 未挿入でも安全。shell_target のみ読む配線は本存在チェック＋コードレビューで足りる。
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
}
