//! 実行系の側の起動（design「RuntimeBoot と CueSinkSources」・要件 4.2・13.4）。
//!
//! 起動の結線は「窓の側」（資産・presenter・文字の層・seriko・loop ticker・`Emo2Wiring`）と
//! 「実行系の側」（`GhostRuntime`）の 2 段に分かれる。窓の側が受け口の送出端の束
//! [`CueSinkSources`] を 1 度組み、[`boot_runtime`] が束の複製から 12 本の sink を組んで実行系を
//! 起こす。受信端が窓の側に無い線（readme・中断を禁じる旗・ゴーストの切替・シェル／バルーンの
//! 切替・読み直し）はここで新しく張り、実行系が成立したときだけ World へ据える——失敗したとき
//! World は変わらない。

use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};

use areka_emo_text::sink::EmoTextSink;
use areka_ghost::sink::BootCueSink;
use areka_ghost::{GhostBootError, GhostBootOptions, GhostRuntime, SystemVarWiring};
use areka_kanade::KanadeNotice;
use areka_parsers::charset::DefaultEncoding;
use areka_seriko::SerikoSink;
use bevy_ecs::world::World;
use tracing::{error, warn};

use super::Emo2BootInputs;
use super::change_cue::ChangeCueSink;
use super::ghost_switch::{self, ChangeRequestRaw};
use super::install_cue::InstallCueSink;
use super::move_cue::{MoveCueSink, PositionCue};
use super::readme_cue::ReadmeCueSink;
use super::reload_args::ScriptReload;
use super::reload_cue::{ReloadCueSink, wire_reload_rx};
use super::shell_balloon_switch::{self, SkinRequestRaw};
use super::switch_cue::SwitchCueSink;
use super::talk_clock::{ClockedTextSink, TalkClock};
use super::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};
use super::update_cue::UpdateCueSink;
use super::user_break_cue::{NoUserBreakCueSink, NoUserBreakSignal};
use super::zorder_cue::{ZOrderCueSink, ZOrderDirective};

/// 受け口の送出端の束（窓の側が 1 度作り、実行系を起こすたびに複製して sink を組む）。
#[derive(Clone)]
pub(crate) struct CueSinkSources {
    /// seriko の送り手（Clone）。
    pub surface: SerikoSink,
    /// 文字の層の送り手（Clone）。
    pub text: EmoTextSink,
    /// `ClockedTextSink`／`BalloonLifecycleSink` が共有する台本の時計。
    pub clock: TalkClock,
    /// `\![move]` の線の送出端（受信端は `Emo2Wiring`）。
    pub move_tx: Sender<PositionCue>,
    /// 表示ライフサイクルの線の送出端（受信端は `Emo2Wiring`）。
    pub lifecycle_tx: Sender<TalkLifecycleSignal>,
    /// 重なりの線の送出端（受信端は `Emo2Wiring`）。
    pub zorder_tx: Sender<ZOrderDirective>,
}

/// 実行系の側の起動の失敗（分類した記録は [`boot_runtime`] が残す）。
#[derive(Debug, thiserror::Error)]
pub(crate) enum RuntimeBootError {
    /// `areka_ghost::boot_with_origin` の失敗。
    #[error("実行系の起動に失敗")]
    Boot(#[source] GhostBootError),
}

/// 受信端が窓の側に無い線の手元の端（実行系の成立後にだけ World へ据える）。
pub(super) struct RuntimeLines {
    readme_rx: Receiver<crate::readme::ReadmeRequest>,
    no_user_break_rx: Receiver<NoUserBreakSignal>,
    /// 中断を受け入れた UI が同じ線へ「中断された」を送る送出端の複製（areka-P0-balloon-break task 3.3）。
    user_break_lifecycle_tx: Sender<TalkLifecycleSignal>,
    change_rx: Receiver<ChangeRequestRaw>,
    switch_rx: Receiver<SkinRequestRaw>,
    pub(super) reload_rx: Receiver<ScriptReload>,
}

/// 束の複製から 12 本の sink を組む（並びは旗の順序の保証を担うので変えない）。
///
/// 1. surface（seriko）2. 時刻付きの文字 3. `\![move]` 4. 表示ライフサイクル（文字より**後**——
/// 起床旗 PRESENT が文字 cue より先に倒れないため・draw-load-parity 3.5）5. 重なり
/// 6. 開く系（readme）7. 中断を禁じる旗 8. ゴーストの切替 9. インストール 10. 更新
/// 11. シェル／バルーンの切替 12. 読み直し（areka-P0-mcp-reload）。5 本目から後ろはどれも
/// 「名前＋第 1 引数」で自分宛だけを拾い、文字 cue に依存しないため末尾に足して構わない。
/// インストールと更新の送出端はプロセスに 1 つの窓口から借りる（窓口が無ければ受信端の無い送出端）。
pub(super) fn runtime_sinks(
    world: &World,
    sources: &CueSinkSources,
) -> (Vec<Box<dyn BootCueSink>>, RuntimeLines) {
    let CueSinkSources {
        surface,
        text,
        clock,
        move_tx,
        lifecycle_tx,
        zorder_tx,
    } = sources.clone();
    let (readme_tx, readme_rx) = std::sync::mpsc::channel();
    let (no_user_break_tx, no_user_break_rx) = std::sync::mpsc::channel();
    let (change_tx, change_rx) = std::sync::mpsc::channel();
    let (switch_tx, switch_rx) = std::sync::mpsc::channel();
    let (reload_tx, reload_rx) = std::sync::mpsc::channel();
    let user_break_lifecycle_tx = lifecycle_tx.clone();
    let sinks: Vec<Box<dyn BootCueSink>> = vec![
        Box::new(surface),
        Box::new(ClockedTextSink::new(text, clock.clone())),
        Box::new(MoveCueSink::new(move_tx)),
        Box::new(BalloonLifecycleSink::new(lifecycle_tx, clock)),
        Box::new(ZOrderCueSink::new(zorder_tx)),
        Box::new(ReadmeCueSink::new(readme_tx)),
        Box::new(NoUserBreakCueSink::new(no_user_break_tx)),
        Box::new(ChangeCueSink::new(change_tx)),
        Box::new(InstallCueSink::new(crate::install::desk::raw_sender(world))),
        Box::new(UpdateCueSink::new(crate::update::desk::raw_sender(world))),
        Box::new(SwitchCueSink::new(switch_tx)),
        Box::new(ReloadCueSink::new(reload_tx)),
    ];
    let lines = RuntimeLines {
        readme_rx,
        no_user_break_rx,
        user_break_lifecycle_tx,
        change_rx,
        switch_rx,
        reload_rx,
    };
    (sinks, lines)
}

/// 実行系の側の起動: 束の複製から 12 本の sink を組んで `boot_with_origin` を呼び、成立したら
/// 受信端を World へ据える。`Err` のとき World は変わっていない。
///
/// 停止通知の送出端は引数 `kanade_stop` をそのまま kanade へ渡す（ここでは channel を作らない）。
/// 起動の由来は kanade の起動の根（`OnBoot`／`OnGhostChanged`／読み直し）の選択へ渡る。
pub(crate) fn boot_runtime(
    world: &mut World,
    inputs: Emo2BootInputs,
    sources: &CueSinkSources,
    kanade_stop: Sender<KanadeNotice>,
    shell: Option<&str>,
) -> Result<GhostRuntime, RuntimeBootError> {
    let Emo2BootInputs {
        ghost_root,
        balloon_root: _,
        shiori,
        ticker,
        app_profile_dir,
        boot_origin,
    } = inputs;
    let (sinks, lines) = runtime_sinks(world, sources);
    let boot_options = GhostBootOptions {
        strict_judges: None,
        ghost_root: ghost_root.clone(),
        default_encoding: DefaultEncoding::Ansi,
        shiori,
        sinks,
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir,
        ticker,
    };
    let runtime = match areka_ghost::boot_with_origin(
        boot_options,
        Some(kanade_stop),
        boot_origin,
        shell,
        crate::write_failure::sender(world),
    ) {
        Ok(runtime) => runtime,
        Err(err) => {
            // R7.4: 既存 main と同一方針で分類（起点不在＝良性 warn・他＝error）。
            if crate::is_benign_boot_error(&err) {
                warn!(
                    error = %err,
                    "emo2-boot: ghost boot の起点が見つかりません（実 sink 結線は不成立・帰結は呼び手が記録・R7.4）"
                );
            } else {
                error!(
                    error = %err,
                    "emo2-boot: ghost boot に失敗（実 sink 結線は不成立・帰結は呼び手が記録・R7.4）"
                );
            }
            return Err(RuntimeBootError::Boot(err));
        }
    };
    install_lines(world, &ghost_root, &runtime, lines);
    Ok(runtime)
}

/// 実行系の成立後に受信端を World へ据える（どれもゴーストごとに新品・前の受信端は置き換わって落ちる）。
/// 取り出しの系の登録は `ghost_session::register_systems` がプロセスに 1 度だけ行う。
fn install_lines(
    world: &mut World,
    ghost_root: &Path,
    runtime: &GhostRuntime,
    lines: RuntimeLines,
) {
    let RuntimeLines {
        readme_rx,
        no_user_break_rx,
        user_break_lifecycle_tx,
        change_rx,
        switch_rx,
        reload_rx,
    } = lines;
    // 説明書の置き場はゴーストの根と定義の `readme` キーから起動時に 1 度だけ決まる（要件 4.5）。
    let readme_path = crate::readme::resolve_path(ghost_root, runtime.mount().readme.as_deref());
    crate::readme::wire_readme(world, readme_path, readme_rx);
    ghost_switch::wire_change_rx(world, change_rx);
    shell_balloon_switch::wire_switch_rx(world, switch_rx);
    // 中断の持ち物は運行（kanade）への送出端を新しい実行系から複製する（「最後に届けた旗」は初期値から）。
    crate::input_events::user_break::wire_user_break(
        world,
        no_user_break_rx,
        user_break_lifecycle_tx,
        runtime.kanade().clone(),
    );
    wire_reload_rx(world, reload_rx);
}

#[cfg(test)]
#[path = "runtime_boot_tests.rs"]
mod tests;
