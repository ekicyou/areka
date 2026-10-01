//! シェル・バルーンの切替の入口（spec: areka-P0-shell-balloon-switch 要件 1.1〜1.16・
//! design「ShellBalloonSwitch」）。
//!
//! 切替要求は台本（受け口 `switch_cue.rs` の `SwitchCueSink` → 受信端 [`SwitchRx`] → 入力の段の
//! 取り出しの系 [`drain_switch_requests`]）とメニューの「シェル」「バルーン」枠の 2 つの出どころから、
//! ここの [`request_skin_switch`] 1 本だけを通る（要件 1.1）。入口は kanade へ何か送る前に
//! 進行中・ゴースト切替中・更新中・文脈・切替先を判定し、受理したら背景の資産づくりを起こして
//! kanade へ台詞の切れ目の待ちを頼み、進行中の印 [`SkinSwitchInFlight`] を置く。
//! 名前の解決とインストールの控えは隣の `shell_balloon_resolve.rs`。印の見極めと差し替えは
//! 差し替えの相（`frame/switch.rs`）が受け持つ。
//!
//! Reference の組み立ての規則（名前は `descript.txt` の `name`、無ければフォルダ名・パスは
//! 絶対パス）は [`skin_ref_name`]・[`skin_ref_path`] の 1 か所に置き、差し替えの相の
//! `OnShellChanged`／`OnBalloonChange` も同じものを使う。

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use areka_actor::{ReplyReceiver, reply_channel};
use areka_emo_present::PresentOutcome;
use areka_kanade::{GapRaise, KanadeMsg, MarkedEnd, ShioriMethod, TalkGap};
use areka_parsers::balloon::BalloonModel;
use areka_sylphya::PersistKey;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use super::frame::Emo2Wiring;
use super::ghost_switch::SwitchInFlight;
use super::shell_balloon_resolve::{
    SkinCandidate, balloon_candidates, installed_for, resolve_skin_target, shell_candidates,
};
use super::switch_assets::{SwapBuilt, SwitchBuildError, SwitchBuildRequest, spawn_switch_build};
use crate::boot_config::BootContext;
use crate::boot_resolve::pick_index;
use crate::exit_wait::{self, WorkGate};
use crate::ghost_session::GhostSlot;
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::source::DescriptSource;

/// 切替の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinKind {
    /// シェル（今のゴーストの `shell/` の下）。
    Shell,
    /// バルーン（ベースウェアの根の `balloon/` の下）。
    Balloon,
}

/// 台本の `\![change,shell,名(,--option=raise-event)]`・`\![change,balloon,名]` から届く切替要求
/// （受け口 `SwitchCueSink` が talk スレッドから送る）。名前は台本の字面のまま（無変形）。
///
/// `raise_event` はシェルでだけ真になりうる（バルーンに「切り替え前」のイベントは無い・要件 1.4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkinRequestRaw {
    pub kind: SkinKind,
    pub name: String,
    pub raise_event: bool,
}

/// 切替先の指し方。台本は名前（`descript.txt` の `name` → フォルダ名の順）、メニューはフォルダ名で指す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkinSpec {
    Name(String),
    Folder(String),
}

/// 要求の出どころ（`OnShellChanging` を送るかを決める＝台本の `raise-event` とメニューのシェル）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinOrigin {
    Script { raise_event: bool },
    Menu,
}

/// 入口へ渡す切替要求（要件 1.1: 台本もメニューもこの 1 本を通る）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkinRequest {
    pub kind: SkinKind,
    pub target: SkinSpec,
    pub origin: SkinOrigin,
}

/// 入口の判定（受理以外はどれも `warn!` を 1 件残して何もしない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinVerdict {
    Accepted,
    /// シェル・バルーンの切替が進行中（重ねない・要件 1.12）。
    Busy,
    /// ゴースト切替が進行中（要件 1.12）。
    GhostSwitching,
    /// ネットワーク更新の実行中（要件 1.13）。
    Updating,
    /// 切替先が決まらない（要件 1.7）。
    NotFound,
    /// 起動の文脈・置き場のゴースト・結線・seriko の送り手のどれかが無い（kanade へ送れないときも）。
    NoContext,
}

/// 切替の進行中の印（World の NonSend・高々 1 つ）。在れば次の切替要求は断る（要件 1.12）。
///
/// 置くのは受理のときだけ。消すのは差し替えの相（完了・中止・取りやめ・失敗）と終了の片付け
/// （[`discard_for_exit`]）。
pub(crate) struct SkinSwitchInFlight {
    pub kind: SkinKind,
    pub target: SkinCandidate,
    pub stage: SkinSwitchStage,
}

/// 切替の段（差し替えの相 `frame/switch.rs` が進める）。
pub(crate) enum SkinSwitchStage {
    /// 台詞の切れ目の返事と、背景の資産づくりの結果を待っている（どちらが先に来てもよい）。
    Waiting {
        /// 切れ目の返事の受け手（資産より先に `Reached` が届いたら、資産がそろったフレームで
        /// 送り直した待ちの受け手に替わる）。
        gap: Option<ReplyReceiver<TalkGap>>,
        /// 資産より先に届いた 1 度目の `Reached`（印の台詞の終わり方を記録に使う）。
        gap_result: Option<TalkGap>,
        /// 背景の資産づくりの結果の受け手（受け取ったら `built` へ移す）。
        build: Receiver<Result<SwapBuilt, SwitchBuildError>>,
        built: Option<SwapBuilt>,
    },
    /// seriko へ差し替えを頼み、置き換えの返信を待っている（全部そろえば差し替えの相の完了の段が
    /// 後始末する）。
    Committed {
        /// 差し替えの世代（置き場の荷物と seriko の合図を結ぶ）。
        epoch: u64,
        /// scope ごとの置き換えの返信の受け手。
        replies: Vec<(u32, ReplyReceiver<PresentOutcome>)>,
        /// 後始末に使う、荷物と seriko の定義に入らなかった残り。
        finish: SwapFinish,
        /// 1 度目の切れ目の返事の印の台詞の終わり方（記録用）。
        marked: Option<MarkedEnd>,
        /// 頼んだ時刻（UI スレッドが相の中で費やした時間 `swap_ms` の起点）。
        committed_at: Instant,
    },
}

/// 資産のうち、荷物（scope ごとの `EmoWorld`・アトラス）と seriko の定義に入らず、完了の後始末で
/// 使う残り。
pub(crate) enum SwapFinish {
    /// シェル: 新しいシェルの配置の値・走っているバルーンの配置の値・位置の記憶。
    Shell {
        source: DescriptSource,
        balloon: BalloonPlacementInputs,
        restored: Vec<(PersistKey, String)>,
    },
    /// バルーン: scope ごとの文字の模型と背景色。
    Balloon {
        scopes: Vec<(u32, BalloonModel, (u8, u8, u8))>,
    },
}

/// 台本からの切替要求の受信端（World の NonSend・ゴーストごとに新品）。
pub(crate) struct SwitchRx(Receiver<SkinRequestRaw>);

/// 受信端を World へ据える（ゴーストごと・系は登録しない＝登録は [`register_switch_drain`]）。
///
/// 呼び手は結線の成立後の `wire_emo2_boot`。前のゴーストの受信端は置き換わって落ちる。
pub(crate) fn wire_switch_rx(world: &mut World, rx: Receiver<SkinRequestRaw>) {
    world.insert_non_send(SwitchRx(rx));
}

/// 取り出しの系を入力の段へ登録し、終了の片付け [`discard_for_exit`] を終了の待ちへ登記する
/// （プロセスに 1 回・持ち物は置かない・呼び手は `ghost_session::register_systems`）。
///
/// 門は背景で書く仕事を持たない（資産づくりはファイルを書かない）ので、片付けを呼ばせるためだけに置く。
pub(crate) fn register_switch_drain(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain_switch_requests.after(dispatch_pointer_events));
    exit_wait::register_gate(
        world,
        "skin_switch",
        Arc::new(WorkGate::default()),
        discard_for_exit,
    );
}

/// 溜まった台本の切替要求を全件取り出し、1 件ごとに入口へ「台本」の出どころで渡す（Input の系）。
///
/// 受信端が無ければ無操作（LogSink の起動・結線の前）。受信端の借用を切ってから入口を呼ぶ。
pub(crate) fn drain_switch_requests(world: &mut World) {
    let Some(rx) = world.get_non_send::<SwitchRx>() else {
        tracing::trace!(
            event = "skin_switch_drain_no_wiring",
            "台本のシェル・バルーンの切替要求の受信端が無い——取り出しは無操作"
        );
        return;
    };
    let pending: Vec<SkinRequestRaw> = rx.0.try_iter().collect();
    for raw in pending {
        request_skin_switch(
            world,
            SkinRequest {
                kind: raw.kind,
                target: SkinSpec::Name(raw.name),
                origin: SkinOrigin::Script {
                    raise_event: raw.raise_event,
                },
            },
        );
    }
}

/// 切替要求の唯一の入口（要件 1.1）。乱数は本番の `pick_index`。
pub(crate) fn request_skin_switch(world: &mut World, req: SkinRequest) -> SkinVerdict {
    request_skin_switch_with(world, req, pick_index)
}

/// 入口の中身。乱数を外から受ける（決定論テスト用・本番は [`request_skin_switch`]）。
///
/// 判定は 進行中 → ゴースト切替中 → 更新中 → 文脈なし（置き場・根・結線・kanade の送出端・
/// seriko の送り手）→ 該当なし の順で、断りはどれも `warn!` を 1 件残し kanade へ何も送らない
/// （要件 1.7・1.12・1.13）。受理したら `info!(skin_switch_requested)` → 背景の資産づくり →
/// 待ちの依頼（メニューのシェルと `raise-event` 付きの台本のシェルだけ `OnShellChanging` の印つき）→
/// 進行中の印、の順。送れなければ `error!(skin_switch_send_failed)` で印を置かない。
/// 定常でない kanade での無視（要件 1.14）は、待ちの返事で差し替えの相が判定する。
pub(crate) fn request_skin_switch_with(
    world: &mut World,
    req: SkinRequest,
    pick: impl FnOnce(usize) -> usize,
) -> SkinVerdict {
    if world.get_non_send::<SkinSwitchInFlight>().is_some() {
        tracing::warn!(
            event = "skin_switch_busy",
            kind = ?req.kind,
            spec = ?req.target,
            "シェル・バルーンの切替の途中に新しい切替要求が届いた——重ねずに無視する"
        );
        return SkinVerdict::Busy;
    }
    if world.get_non_send::<SwitchInFlight>().is_some() {
        tracing::warn!(
            event = "skin_switch_ghost_switching",
            kind = ?req.kind,
            spec = ?req.target,
            "ゴースト切替の途中にシェル・バルーンの切替要求が届いた——無視する"
        );
        return SkinVerdict::GhostSwitching;
    }
    if crate::update::desk::is_busy(world) {
        tracing::warn!(
            event = "skin_switch_updating",
            kind = ?req.kind,
            spec = ?req.target,
            "ネットワーク更新の実行中にシェル・バルーンの切替要求が届いた——無視する"
        );
        return SkinVerdict::Updating;
    }

    let Some(ctx) = SwitchContext::read(world) else {
        return SkinVerdict::NoContext;
    };
    let (candidates, current) = match req.kind {
        SkinKind::Shell => (shell_candidates(&ctx.ghost_dir), ctx.current_shell.clone()),
        SkinKind::Balloon => (balloon_candidates(&ctx.root), ctx.current_balloon.clone()),
    };
    let installed = installed_for(world, req.kind, &ctx.ghost_dir);
    let target = match resolve_skin_target(
        &candidates,
        &req.target,
        current.as_deref(),
        installed.as_deref().map_err(|e| *e),
        pick,
    ) {
        Ok(target) => target,
        Err(reason) => {
            tracing::warn!(
                event = "skin_switch_unknown",
                kind = ?req.kind,
                reason = ?reason,
                spec = ?req.target,
                "切替先が候補のどれにも一致しない——切替を無視する（イベントも送らない）"
            );
            return SkinVerdict::NotFound;
        }
    };

    tracing::info!(
        event = "skin_switch_requested",
        kind = ?req.kind,
        to = %target.folder,
        origin = ?req.origin,
        "シェル・バルーンの切替要求を受けた"
    );
    let build = spawn_switch_build(match req.kind {
        SkinKind::Shell => SwitchBuildRequest::Shell {
            ghost_root: ctx.ghost_dir.clone(),
            folder: target.folder.clone(),
            balloon_dir: ctx.balloon_dir.clone(),
        },
        SkinKind::Balloon => SwitchBuildRequest::Balloon {
            dir: target.dir.clone(),
        },
    });
    let marked = req.kind == SkinKind::Shell
        && matches!(
            req.origin,
            SkinOrigin::Menu | SkinOrigin::Script { raise_event: true }
        );
    // Ref1＝今のシェルの名前（目録の `name`、無ければフォルダ名・今のシェルが分からなければ空）。
    let raise = marked.then(|| {
        let current_name = current.as_deref().map_or_else(String::new, |folder| {
            candidates
                .iter()
                .find(|c| c.folder == folder)
                .map_or_else(|| folder.to_owned(), skin_ref_name)
        });
        GapRaise {
            id: "OnShellChanging".to_owned(),
            references: vec![
                skin_ref_name(&target),
                current_name,
                skin_ref_path(&target.dir),
            ],
            method: ShioriMethod::Get,
        }
    });
    let (reply, gap) = reply_channel();
    if ctx
        .kanade
        .send(KanadeMsg::AwaitTalkGap { raise, reply })
        .is_err()
    {
        tracing::error!(
            event = "skin_switch_send_failed",
            kind = ?req.kind,
            to = %target.folder,
            "kanade へ台詞の切れ目の待ちを頼めなかった（kanade は止まっている）——切替を行わない"
        );
        return SkinVerdict::NoContext;
    }
    world.insert_non_send(SkinSwitchInFlight {
        kind: req.kind,
        target,
        stage: SkinSwitchStage::Waiting {
            gap: Some(gap),
            gap_result: None,
            build,
            built: None,
        },
    });
    SkinVerdict::Accepted
}

/// 入口が受理に要る文脈（World から借りずに写し取ったもの）。
struct SwitchContext {
    root: areka_ghost::BasewareRoot,
    ghost_dir: std::path::PathBuf,
    kanade: std::sync::mpsc::Sender<KanadeMsg>,
    /// 今のシェルのフォルダ名（実行系のマウントの末尾・実行系が無ければ無し）。
    current_shell: Option<String>,
    /// 今のバルーンのフォルダ名（起動の文脈の写し）。
    current_balloon: Option<String>,
    /// 今のバルーンのフォルダ（起動の文脈の写し・シェルの差し替えで配置の値を読む先）。
    balloon_dir: std::path::PathBuf,
}

impl SwitchContext {
    /// 置き場のゴースト・起動の文脈・結線・kanade の送出端・seriko の送り手をそろえる。欠けていれば
    /// `warn!(skin_switch_no_context)` を `reason` つきで 1 件残して `None`。
    fn read(world: &World) -> Option<Self> {
        let missing = |reason: &'static str| {
            tracing::warn!(
                event = "skin_switch_no_context",
                reason,
                "切替に要る文脈が無いのでシェル・バルーンの切替要求を無視する"
            );
        };
        let Some(session) = world
            .get_non_send::<GhostSlot>()
            .and_then(|slot| slot.0.as_ref())
        else {
            missing("ghost_slot");
            return None;
        };
        let Some(ctx) = world.get_resource::<BootContext>() else {
            missing("boot_context");
            return None;
        };
        if world.get_non_send::<Emo2Wiring>().is_none() {
            missing("wiring");
            return None;
        }
        let Some(kanade) = session.kanade().cloned() else {
            missing("kanade");
            return None;
        };
        if session.seriko_sink().is_none() {
            missing("seriko_sink");
            return None;
        }
        Some(Self {
            root: ctx.root.clone(),
            ghost_dir: session.ghost_dir().to_path_buf(),
            kanade,
            current_shell: session.current_shell_folder(),
            current_balloon: ctx.current.balloon.folder.clone(),
            balloon_dir: ctx.current.balloon.dir.clone(),
        })
    }
}

/// 終了が始まった（`exit_wait::begin_close` が呼ぶ）: 進行中の印があれば
/// `info!(skin_switch_dropped, reason=exit)` を残して消す（要件 5.7）。背景の資産づくりは
/// 受け手が落ちたことで結果を捨てる。
pub(crate) fn discard_for_exit(world: &mut World) {
    if let Some(dropped) = world.remove_non_send::<SkinSwitchInFlight>() {
        tracing::info!(
            event = "skin_switch_dropped",
            reason = "exit",
            kind = ?dropped.kind,
            to = %dropped.target.folder,
            "終了が始まったのでシェル・バルーンの切替を取りやめた"
        );
    }
}

/// Reference の名前（`descript.txt` の `name`、無ければフォルダ名・要件 2.1・2.4・3.2）。
pub(crate) fn skin_ref_name(candidate: &SkinCandidate) -> String {
    candidate
        .name
        .clone()
        .unwrap_or_else(|| candidate.folder.clone())
}

/// Reference のパス（絶対パスの文字列・絶対化できなければそのまま・要件 2.1・2.4・3.2）。
pub(crate) fn skin_ref_path(dir: &Path) -> String {
    std::path::absolute(dir)
        .unwrap_or_else(|_| dir.to_path_buf())
        .display()
        .to_string()
}

#[cfg(test)]
#[path = "shell_balloon_switch_tests.rs"]
mod tests;
