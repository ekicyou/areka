//! 毎フレーム三相結線（attach／drain／text）の排他 system と NonSend 配線状態。
//!
//! `Emo2Wiring`（NonSend resource・presenter／rx／runtime／clock／assets／attached を保持）と
//! 排他 system `emo2_frame_system(world: &mut World)`（donor パターン: remove→3 フェーズ→insert）を
//! 所有する。三フェーズ:
//! - attach: GPU 資源＋`GhostWindows` 到達ゲート→`plan_attachments`（DD-12）→初回 `ShowSurface`→
//!   文字層スロット取得→`register_actor_view`（`Option::take` で高々 1 回消費）。
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

use bevy_ecs::world::World;
use tracing::{debug, error, info, warn};

use areka_emo_present::{EmoPresenter, PresentCommand, TargetId, TextSlotView};
use areka_emo_text::actor::{present_frame, TextLayerRuntime};
use areka_parsers::balloon::BalloonModel;
use areka_sakura::ActorKey;
use wintf::ecs::{FrameTime, GraphicsCore, WucGraphicsResource};

use crate::placement::spawn::GhostWindows;

use super::assets::{BootAssets, ScopeAssets};
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
    /// 結線資源を構築する（`wire_emo2_boot`＝task 5.1 が呼ぶ）。`assets` は `Some` で保持し、
    /// attach フェーズ（[`run_attach_phase`]）が `take` で高々 1 回消費する。
    pub fn new(
        presenter: EmoPresenter,
        rx: Receiver<PresentCommand>,
        runtime: Rc<RefCell<TextLayerRuntime>>,
        clock: TalkClock,
        assets: BootAssets,
    ) -> Self {
        Self {
            presenter,
            rx,
            runtime,
            clock,
            assets: Some(assets),
            attached: false,
        }
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
/// 装着して初回表示を駆動する。資産は `Option::take` で高々 1 回消費し、ゲート不成立では消費せず
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
        static_binds,
    } = assets;
    let mut shells: Vec<_> = shells.into_iter().map(Some).collect();
    let mut balloons: Vec<_> = balloons.into_iter().map(Some).collect();

    let planned_count = plan.items.len();
    let mut attached_count = 0usize;

    for item in &plan.items {
        let scope = item.scope;

        // --- shell target: char_window → attach_target（EmoWorld を move）→ 初回 ShowSurface ---
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
        // 初回 ShowSurface（初期面 initial_surface_id・起動時オンの static_binds・DD-8/DD-9）。
        wiring.presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: item.shell_target,
                surface_id: item.initial_surface_id,
                binds: static_binds.clone(),
                reply: None,
            },
        );
        // シェル装着＋初回表示が済んだ scope を計上（DD-12 の planned==attached 積極 assert 用）。
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
}
