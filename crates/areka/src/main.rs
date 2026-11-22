#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! areka 本番アプリ骨格
//!
//! 起動と終了の中身は `areka-session` にあり、`main` は次の順につなぐ「アプリ起動の器」に徹する。
//! 座標・配置は `areka-placement` が持ち、骨格自身は座標を一切持たない。
//! - ログ初期化（`RUST_LOG` が使えなければ info）・パニックハンドラ・起動オプションの読み取り
//! - 起動前の解決（根 → ゴースト → バルーン → 構成入力）と書けるかの確かめ。決まらなければ告知して終了コード 1
//! - UI ランタイム起動（`WinApp::with_exit_policy(ExitPolicy::Explicit)`・失敗は `error!` に残す）
//! - 系の登録（`BODY_SYSTEMS` で本体だけの系を渡す）と起動の文脈の据え付け
//! - 起動窓シーム（`ghost_session::open_ghost_windows`）と最初のゴーストの起動（`ghost_session::boot_ghost`）。
//!   窓を開けない・起動中の印を書けないときは告知して終了コード 1。実 sink の結線が成立しなければ
//!   `LogSink` へ倒れて続ける
//! - `main` 自身が所有するメッセージループ（`app.run()`）。戻るのは `app_exit::quit_app` の終了の指示のとき
//! - `run()` 復帰後の後始末（`finish_after_run`）: 降ろす（loop ticker → 終了統括 → seriko join）→ 起動中の印の始末
//!   → 書いている最中の背景の仕事を待つ → 告知 → CPU 報告の最後の 1 枚。終了コードは最後に決める

use bevy_ecs::prelude::*;
use windows::core::Result;
use wintf::*;

/// 試験用の `IShioriHost` 実装。使い手は `mcp/get_property_tests.rs` のテストだけ。
#[cfg(test)]
mod shiori_host;

/// 配置の crate は本体を使えないので、配置と本体のほかの部分をまたぐテストと判定器はここに置く。
#[cfg(test)]
mod placement_tests;
/// `areka-runtime` は本体を使えないので、本体の部品も使う runtime のテストはここに置く。
#[cfg(test)]
mod runtime_tests;
/// 本体のテストへ emo2 検体を配る入口（crate ごとに 1 つ）。
#[cfg(test)]
mod sample_test_support;
/// 集めた層は本体を使えないので、本体の部品も使う session のテストはここに置く。
#[cfg(test)]
mod session_tests;

/// tick の門の既定を起動時に上書きする読み口（`AREKA_TICK_GATE=1|0`）。
mod tick_gate_config;

mod mcp;
mod menu;
/// 乗算済み BGRA → 透過 PNG（MCP とオフラインの書き出しが共有）。
mod png_image;

/// アクタースレッドの役割名の宣言。
/// `areka-actor` のスレッド開始フックを導入し、生成されるアクタースレッド 1 本ごとに
/// 役割名を宣言して wintf のスレッド名簿へ登録する。
mod thread_roles;

/// スレッド別・プロセス全体の CPU 報告器。
/// target `areka::perf` が点いているときだけ報告スレッドを起こし、名簿を舐めた
/// `perf(thread)` 行と `perf(process)` 行を周期＋終了直前に出す。消灯時は費用 0。
mod perf_thread_report;

/// `.nar` を入れる手続き。
mod install;

/// ゴースト・シェル・バルーンのネットワーク更新。
mod update;

mod file_drop;

/// ベースウェアの根の並びの決め方。
mod roots;

/// 起動オプションの読み方。
mod cli;

/// オフラインの書き出し（`--offline-dump`）。
mod offline_dump;

/// スコープごとに既定の着せ替えと名前が分かれる検体（試験専用・画面と書き出しの試験が共有）。
#[cfg(test)]
mod scope_bind_fixture_test_support;

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    use areka_session::{
        alert,
        boot_config::{self, default_app_profile_dir, default_helper_exe_path},
        boot_request, boot_resolve, exit_wait, ghost_session, log_history, readonly,
        session_mark::{Teardown, settle_session_mark},
        write_failure,
    };
    human_panic::setup_panic!();

    // RUST_LOG が未設定・非UTF-8・不正な構文なら "info" へフォールバックする（panic 経路なし）。
    log_history::init();

    // 起動オプションを 1 度だけ読む（記録はここで出る）。`--help` は起動の解決・書けるかの確かめ・
    // MCP・アプリの窓のどれよりも前に説明書を開いて戻る（開けなければ 0 でない終了）。
    let opts = cli::read();
    if opts.help {
        return cli::help::run();
    }
    // `--offline-dump` は書き出しだけ走らせ、その終了コードで終える（役目の登録・起動の解決・MCP・窓より前）。
    if let Some(req) = &opts.offline {
        std::process::exit(offline_dump::run(req));
    }
    // 起動の依頼の持ち物（`--install` と exe の隣の initial_install.nar）は早い戻りの後に作る。
    let mut startup_install = install::startup::StartupInstall::new(opts.install);
    // 読み取りだけかを 1 度だけ立てる（置き場の形と受け口が決まる前）。
    readonly::engage(opts.readonly);
    if opts.modules == boot_request::ModulePolicy::Skip {
        tracing::info!(
            event = "shiori_skipped",
            "[main] --option dontloadmodule により、SHIORI を読まずに起動する"
        );
    }

    // アクタースレッドの役割宣言フックを導入する。以後 `spawn_actor` で起きるスレッドは走り始めに
    // 自分の役割名を宣言して wintf のスレッド名簿へ載る。**すべての結線より前**に置くのは、
    // これより前に起きたスレッドが名簿から漏れるため。
    thread_roles::install();

    // スレッド別 CPU の報告器。点灯の判定はここで 1 度だけ行い、`RUST_LOG` に `areka::perf=debug` が
    // 無ければ報告スレッドを起こさない（既定運転の費用 0）。取っ手は終了直前まで持ち回り、最後の 1 枚を出す。
    let perf_report = perf_thread_report::start();

    // 起動前の解決（`WinApp` 構築の前）: 根 → ゴースト → バルーン → 構成入力。決まらなければ告知して
    // 終了コード 1 で終える（`warn!` で起動を続ける経路は作らない）。決まった経路とフォルダは boot の
    // 分岐まで持ち越す（boot 成功直後の記憶の書き込みに使う）。4 つ目は起動中の印の値＝前回きれいに
    // 終わらなかったゴーストの名前（読むだけで消さない）で、初回の起動の由来になる。
    let (cfg, ghost_decision, balloon_decision, halted, resolved_roots) =
        match install::startup::resolve_boot(&opts.boot, &mut startup_install) {
            Ok(resolved) => resolved,
            Err(scene) => {
                alert::raise(&scene, alert::suppressed());
                return Err(windows::core::Error::from_hresult(
                    windows::Win32::Foundation::E_FAIL,
                ));
            }
        };
    // 書けるかの確かめ（argv の起動も同じ道・写しの後の置き場を見る）。
    write_failure::check_boot(readonly::is_on(), &cfg, &default_app_profile_dir())
        .map_err(write_failure::stop_boot)?;

    // MCP の待受。畳むのは取っ手の `Drop` で、`down?` の早い戻りを含む `main` のどの出口でも閉じる。
    // `app` より先に宣言する＝`app` の後に落ちる。待受の失敗は `error!` に残すだけで終了コードには
    // 響かない。待受の候補は `--sstp-listen` の指定で決まる。登録表の受け口は `register_systems` の
    // 後で World へ置く。3 つ目は help の Claude Desktop の設定例に載せる中継の置き場＝起動した exe の
    // 本当の場所の隣（補助 exe と同じ決まり。場所が取れなければ「なし」）。
    let (mcp_tools, mcp_inbox) = areka_mcp::tools::all_entrances(areka_mcp::tools::REPLY_WAIT);
    let _mcp = cli::listen::start_mcp(
        &opts.listen,
        mcp_tools,
        boot_config::exe_location().map(|exe| exe.with_file_name(areka_mcp::BRIDGE_EXE_FILE_NAME)),
    );

    // 実行ファイル隣接の 32bit SHIORI helper パスを一度だけ解決する。実 sink 結線経路と `LogSink`
    // フォールバック boot 経路の双方が使う。
    let helper_exe = default_helper_exe_path();

    // UI ランタイム起動（COM/DPI 初期化・World 生成・終了の受け口 `AppExit` の据え付け）。実 sink 結線
    // （`wire_runtime`）は UI 基盤の後なので、`WinApp` 構築はすべての boot より前に置く。
    // `Explicit`: 窓 0 では終了せず、`quit_app` の終了の指示でだけ `run()` が戻る。
    let app = WinApp::with_exit_policy(ExitPolicy::Explicit).inspect_err(|e| {
        tracing::error!(event = "winapp_init_failed", error = %e, "[main] UI の基盤を初期化できませんでした")
    })?;

    // 系の登録（プロセスに 1 回・ここ 1 か所）。停止通知の channel は 1 本だけ作り、受信端をここで
    // 受け口へ据え、送出端の写しを下の起動の 2 経路へ渡す。main 自身の送出端は起動の分岐の後で落とす。
    let (kanade_stop_tx, kanade_stop_rx) = std::sync::mpsc::channel();
    ghost_session::register_systems(
        app.world().borrow_mut().world_mut(),
        kanade_stop_rx,
        BODY_SYSTEMS,
    );
    mcp::install(app.world().borrow_mut().world_mut(), mcp_inbox);
    roots::install(app.world().borrow_mut().world_mut());
    readonly::install(app.world().borrow_mut().world_mut());
    install::startup::install(app.world().borrow_mut().world_mut(), startup_install);

    // 起動の文脈と起動入力の作り口（本番版）を据える（切替と右クリックメニューの「ゴースト」枠が読む）。
    install_boot_context(
        app.world().borrow_mut().world_mut(),
        boot_config::BootContext {
            roots: resolved_roots.roots,
            roots_origin: resolved_roots.origin,
            app_profile_dir: default_app_profile_dir(),
            helper_exe,
            // argv で始まったプロセスか（ここで 1 度だけ決め、以後変えない）。
            argv_session: ghost_decision.route == boot_resolve::GhostRoute::Argv,
            current: boot_config::CurrentGhost {
                cfg: cfg.clone(),
                ghost: ghost_decision.clone(),
                balloon: balloon_decision.clone(),
            },
        },
        kanade_stop_tx.clone(),
        opts.modules,
    );

    // tick の門の既定を起動時に一度だけ上書きする（`AREKA_TICK_GATE=1|0`・A/B と安全弁）。
    tick_gate_config::apply_from_env(&mut app.world().borrow_mut());

    // 起動窓シーム: ゴースト定義から本物のゴースト窓（キャラ窓＋バルーン窓）を配置・生成する。
    // 準備が通らなければ（モニタ 0 台・起動中の削除等）「起動窓を開けない」を告知して終了コード 1 で
    // 終える（窓 0 で居座る経路は作らない。配置の失敗と作業プールの欠落は同じ告知）。戻り値は配置準備が
    // **1 度だけ**読んだ descript 由来の値で、作者基準 DPI は採寸の k₀ と直下の `wire_runtime`
    // （→`attach_target`）の双方へ渡る＝採寸と表示が別々の宣言を見る食い違いを排除する。shell の
    // `seriko.zorder` も同じ搬送に乗せ、重なりの基底の出所を配置と同じ 1 度の読取に揃える。
    let opened = ghost_session::open_ghost_windows(app.world().borrow_mut().world_mut(), &cfg);
    let descript = match opened {
        Ok(startup) => startup,
        Err(err) => {
            alert::raise(
                &alert::AlertScene::StartupWindow {
                    reason: err.to_string(),
                },
                alert::suppressed(),
            );
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ));
        }
    };

    areka_session::smoke_exit::arm(&app);

    // ゴーストごとの結線（`ghost_session::boot_ghost`）: 実 sink 結線を試み、成立しなければ `LogSink`×2 の
    // fallback へ倒す。3 ハンドル（ゴースト実行系・seriko・loop ticker）は `GhostSession` に束ねて置き場
    // （`GhostSlot`）へ入れる（切替が入れ替え、`run()` の後にここへ取り出して降ろす）。印の値が在れば
    // 由来は「前回落ちた」。起こす前に起動中の印を書く（argv で始まったプロセスは書かない）。
    boot_first_ghost(
        app.world().borrow_mut().world_mut(),
        &cfg,
        first_boot_origin(halted),
        &descript,
        &ghost_decision,
        &balloon_decision,
    )
    .map_err(write_failure::stop_boot)?;

    drop(kanade_stop_tx);

    // `main` 所有のブロッキングメッセージループ。窓が 0 になっても戻らず、`app_exit::quit_app` の
    // 終了の指示で `run()` が戻る。失敗でも後始末は通すので `?` で抜けない。
    let run = app.run();
    // 溜まった要求を即座に "shutting down" で返すため、`begin_close` より前に外す。
    mcp::close(app.world().borrow_mut().world_mut());
    let run_ok = run.is_ok();
    // 終了を始める: 背景の仕事の門を閉じ、待っている依頼を捨てる。書いている最中の仕事は、降ろして
    // 印の始末を済ませた後・告知の前に、ここから数えて上限まで待つ。
    let exit_budget = areka_kanade::WaitBudget {
        started: std::time::Instant::now(),
        limit: exit_wait::EXIT_WAIT_LIMIT,
    };
    let closing = exit_wait::begin_close(app.world().borrow_mut().world_mut());

    // 置き場の単位を取り出し、最初に終了を指示した出所から告知の場面と印の判定の材料を組む。窓は
    // `quit_app` が閉じ、残りは `run()` が壊してから戻るので、告知の背後に窓は無い。ゴースト名は
    // 降ろす前にここで読む。
    let AfterRun {
        session,
        scene,
        mark,
        fault,
    } = after_run(app.world().borrow_mut().world_mut());
    // 降ろす途中の書き込みの失敗の受け手（OS のセッションの終了で処理済みなら `None`）。
    let late = write_failure::take_receiver(app.world().borrow_mut().world_mut());

    // 後始末（降ろす → 印の始末 → 告知 → 降ろした結果 → ④）を成否によらず 1 回通し、終了コードは最後に
    // 決める。告知は降ろした後。① で loop ticker は止まっており、降ろす結果は告知の後で返すので ②③ が
    // 失敗しても告知は出る。利用者が閉じたらそのまま進む。
    finish_after_run(run, fault, move || {
        // 降ろす（① loop ticker Close → ② ghost.shutdown → ③ seriko join）。終了理由は全窓 close funnel
        // ＝ユーザ操作起点。置き場が空なのは切替の途中の致命か、OS のセッションの終了で降ろし済み
        // （`session_end`）のとき。
        let down = match session {
            Some(session) => session.shutdown(areka_kanade::CloseReason::User { scope: 0 }),
            None => {
                tracing::info!(
                    event = "ghost_slot_empty",
                    "[main] 置き場が空（切替の途中の致命か、OS のセッションの終了で降ろし済み）——降ろすゴーストは無い"
                );
                Ok(())
            }
        };

        // 起動中の印の始末: 降ろして記憶の書き出しが済んだ後に、きれいな終わりなら消し、そうでなければ
        // 理由を残して残す。UI スレッドが App スコープへ直接書くのは実行系が 1 つも動いていない間だけ
        // （ここは降ろした後なので書いてよい）。
        let clear_failed = mark.as_ref().and_then(|mark| {
            settle_session_mark(
                mark,
                Teardown {
                    run_ok,
                    down_ok: down.is_ok(),
                    shiori_cut: false,
                },
            )
            .1
        });

        // 書いている最中の背景の仕事を待つ（印の始末の後＝待つ間に終わらされても印は済んでいる）。
        closing.wait(exit_budget);

        // 告知を 1 つだけ選ぶ（出所の場面 → 印を消せない → 降ろす途中の失敗・抑止なら記録だけ）。
        let told = write_failure::tell_at_close(scene, clear_failed, late, alert::suppressed());
        down?;

        // ④ スレッド別 CPU の最後のスナップショット。終了直前に 1 枚出してから報告スレッドを畳む。
        // ②③ の失敗で早く戻る経路では最後の 1 枚が出ない（周期のスナップショットまでが残る）。
        // 消灯時は `None` で何もしない。
        if let Some(handle) = perf_report {
            handle.stop_and_report_final();
        }

        told
    })
}

/// 本体だけのモジュールの系の登録（`register_systems` へ渡す）。
pub(crate) const BODY_SYSTEMS: areka_session::body_systems::BodySystems =
    areka_session::body_systems::BodySystems {
        menu_poll: menu::register_menu_poll,
        after_input: register_body_after_input,
        after_char_pointer: menu::attach_release_handlers,
        after_balloon_pointer: attach_body_receivers,
        after_runtime_wired: wire_body_after_runtime,
        on_switch_end: update::desk::on_switch_end,
        on_steady: tell_desks_steady,
        overwrite_between: install::desk::run_overwrite_between,
        update_busy: update::desk::is_busy,
        install_busy: install::desk::is_busy,
        cue_sinks: body_cue_sinks,
    };

fn register_body_after_input(world: &mut World) {
    install::register(world);
    update::register(world);
    mcp::register(world);
}

fn attach_body_receivers(world: &mut World) {
    // 投げ込みの受け手をゴースト窓（キャラ窓・バルーン窓）へ差す。起こし直しもこの閉包を通るので、
    // 回数によらず新しい窓に差さる。
    file_drop::attach_file_drop_receivers(world);
    // 位置初期化の近道の受け手も同じ窓へ差す。
    // 窓がキーの入力先のときだけ届くので、ほかのアプリの同じキーは横取りしない。
    menu::shortcut::attach_shortcut_receivers(world);
}

fn wire_body_after_runtime(world: &mut World, runtime: &areka_ghost::GhostRuntime) {
    // 右クリックメニューの結線: 終了の項目が先に済んだ入力の結線（`wire_mouse_input`）を使う。
    menu::wire_menu(world, runtime.kanade().clone());
    // 枠の登記: `wire_menu` が登記の口を新品にするので、起こすたびにやり直す。
    menu::ghost_frame::register(world);
    menu::shell_frame::register(world);
    menu::balloon_frame::register(world);
    menu::install_frame::register(world);
    menu::roots_frame::register(world);
    menu::update_frame::register(world);
    menu::reset_pos_frame::register(world);
    menu::reset_balloon_pos_frame::register(world);
    // 置換語 %lastghostname・%lastobjectname の載せ直し（記憶の置き場は起こすたびに新しくなる）。
    install::names::reseed(world, runtime);
}

fn tell_desks_steady(world: &mut World) {
    // インストールの窓口へ（送り直しの頼みは定常到達の回数で見直す）。
    install::desk::on_steady(world);
    // 更新の窓口へ（`homeurl` の写しを消して照会し直す）。
    update::desk::on_steady(world);
}

/// 受け口の列の 9・10 番目（インストール → 更新の順）。
fn body_cue_sinks(world: &World) -> [Box<dyn areka_ghost::sink::BootCueSink>; 2] {
    use install::install_cue::InstallCueSink;
    use update::update_cue::UpdateCueSink;
    // 送出端はプロセスに 1 つの窓口から借りる（窓口が無ければ受信端の無い送出端）。
    [
        Box::new(InstallCueSink::new(install::desk::raw_sender(world))),
        Box::new(UpdateCueSink::new(update::desk::raw_sender(world))),
    ]
}

/// `run()` の結果を受けて後始末へ進む判断。
///
/// 後始末は成否によらず必ず 1 回通す。`run` の失敗・後始末の失敗・Fault のどれか 1 つでも
/// あれば `Err`（終了コード 1）、なければ `Ok`（0）。後始末の失敗は後始末の中で、Fault は
/// `quit_app` の `app_exit` で記録済みなので、ここで記録するのは `run` の失敗だけ。
fn finish_after_run(
    run: Result<()>,
    fault: bool,
    cleanup: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if let Err(err) = &run {
        tracing::error!(error = %err, "メッセージループが失敗で戻りました（後始末は続けます）");
    }
    let cleaned = cleanup();
    run?;
    cleaned?;
    if fault {
        return Err(windows::core::Error::from_hresult(
            windows::Win32::Foundation::E_FAIL,
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 置き場と起動の文脈
// ---------------------------------------------------------------------------

/// 起動の文脈と起動入力の作り口（本番版）を World へ据える（系の登録の直後に 1 回）。
/// 作り口は本番の入力の組み立て（[`areka_session::ghost_session::GhostBootInputs::production`]）を文脈の helper の
/// パスと停止通知の送出端の写しと文脈の記憶の置き場で閉じたもの（切替が相手のゴーストを起こすときにも同じ入力になる）。
fn install_boot_context(
    world: &mut World,
    ctx: areka_session::boot_config::BootContext,
    kanade_stop: std::sync::mpsc::Sender<areka_kanade::KanadeNotice>,
    modules: areka_session::boot_request::ModulePolicy,
) {
    use areka_session::ghost_session;
    let helper_exe = ctx.helper_exe.clone();
    // 記憶の置き場は文脈のもの（本番は `default_app_profile_dir()` と同じ値）。
    let app_profile_dir = ctx.app_profile_dir.clone();
    world.insert_resource(ctx);
    world.insert_non_send(ghost_session::GhostBootInputsSource(Box::new(
        move |cfg, origin| {
            ghost_session::GhostBootInputs::production(
                cfg,
                helper_exe.clone(),
                kanade_stop.clone(),
                origin,
                Some(app_profile_dir.clone()),
                modules,
            )
        },
    )));
}

/// 初回の起動の由来: 起動中の印の値（前回きれいに終わらなかったゴーストの名前）が在れば
/// 「前回落ちた」（`OnBoot` の Ref6/7）、無ければ「ふつう」。
fn first_boot_origin(halted: Option<String>) -> areka_kanade::BootOrigin {
    match halted {
        Some(ghost_name) => areka_kanade::BootOrigin::Halted { ghost_name },
        None => areka_kanade::BootOrigin::Plain,
    }
}

/// 初回の起動: 起動入力の作り口で入力を組んで起こし（fallback へ倒れる `boot_ghost`）、
/// 置き場（`GhostSlot`）へ入れる。作り口は [`install_boot_context`] が先に据えている。
///
/// 起こす前（ゴーストの実行系がまだ 1 つも無い時点）に、argv で始まったプロセスでなければ
/// 起動中の印＝起こすゴーストの名前を書く。UI スレッドが App スコープへ直接書くのは実行系が 1 つも
/// 動いていない間だけ（動いている間はそのゴーストの記憶の書き手を通す＝保存は「読んで重ねて書く」
/// ので、2 つの書き手が同時に書くと片方が消えうる）。
fn boot_first_ghost(
    world: &mut World,
    cfg: &areka_session::boot_config::ConfigInputs,
    origin: areka_kanade::BootOrigin,
    descript: &areka_session::ghost_session::StartupDescriptValues,
    ghost: &areka_session::boot_resolve::GhostDecision,
    balloon: &areka_session::boot_resolve::BalloonDecision,
) -> std::result::Result<(), areka_sylphya::PersistFailure> {
    use areka_session::{boot_config, boot_resolve, ghost_session};
    let (argv_session, app_profile_dir) = match world.get_resource::<boot_config::BootContext>() {
        Some(ctx) => (ctx.argv_session, Some(ctx.app_profile_dir.clone())),
        None => (false, None),
    };
    match app_profile_dir {
        Some(dir) if !argv_session => {
            boot_resolve::write_session_mark(&dir, &boot_resolve::running_name(ghost))?
        } // 書けなければゴーストを起こさない（`main` が起動を止める）
        Some(_) => tracing::debug!(
            event = "session_mark_untouched_argv",
            "[main] argv で始まったプロセスなので起動中の印を書きません"
        ),
        None => tracing::warn!(
            event = "boot_context_missing",
            "[main] 起動の文脈が無いので起動中の印を書けません（このゴーストが落ちても次の起動は前回落ちたと気付きません）"
        ),
    }
    let inputs = (world.non_send::<ghost_session::GhostBootInputsSource>().0)(cfg, origin);
    let session = ghost_session::boot_ghost(world, inputs, descript, ghost, balloon);
    // LogSink へ倒れたら、このプロセスは終わり方によらず印を残す。帰結を 1 件だけ記録する（倒れた先の
    // 成否は `boot_ghost` が記録済み）。argv なら印に触れないので `debug!`。
    if session.logsink_fallback() {
        if argv_session {
            tracing::debug!(
                event = "session_mark_pinned_by_fallback",
                "[main] 起動が LogSink へ倒れた——argv で始まったプロセスなので起動中の印には触れません"
            );
        } else {
            tracing::warn!(
                event = "session_mark_pinned_by_fallback",
                "[main] 起動が LogSink へ倒れた——このプロセスは終わり方によらず起動中の印を残す（次の起動は既定のゴーストで Ref6/7 付き）"
            );
        }
    }
    world.insert_non_send(ghost_session::GhostSlot(Some(session)));
    Ok(())
}

/// `run()` の後に置き場と起動の文脈から組むもの。
struct AfterRun {
    /// 降ろす単位（置き場が空＝切替の途中の致命か、OS のセッションの終了で降ろし済みなら `None`）。
    session: Option<areka_session::ghost_session::GhostSession>,
    /// 告知の場面（最初の出所が Fault か記憶を書けなかったもので、OS のセッションの終了を処理していないときだけ）。
    scene: Option<areka_session::alert::AlertScene>,
    /// 起動中の印の判定の材料（起動の文脈が無いか、OS のセッションの終了で済んでいれば `None`＝
    /// 印に触れない）。判定は降ろした後。
    mark: Option<areka_session::session_mark::MarkInputs>,
    /// 終了コードを 1 にするか（最初の出所に失敗の中身がある＝`fault_of`／`write_failure_of`・告知の場面の有無とは独立）。
    fault: bool,
}

/// 置き場の単位を取り出し、最初に終了を指示した出所から告知の場面と印の判定の材料を組む。
///
/// 告知のゴーストは置き場の単位の名前と文脈の今のゴーストの根（切替後の今のゴースト）。
/// 切替の途中の致命（既定へ戻せなかった）では置き場が空で、文脈の今のゴーストは最後に起きた
/// ゴーストのまま残るので、起こそうとしていた既定ゴーストの名前（フォルダ名）と場所で組む
/// （致命になるのは既定ゴーストを起こせなかったときだけ＝`ghost_switch::fatal` の呼び手）。
fn after_run(world: &mut World) -> AfterRun {
    use areka_session::{
        alert, app_exit, boot_config, boot_resolve, ghost_session, session_end,
        session_mark::MarkInputs,
    };
    let session = world
        .get_non_send_mut::<ghost_session::GhostSlot>()
        .and_then(|mut slot| slot.0.take());
    let first = world
        .get_resource::<app_exit::FirstExit>()
        .map(|first| first.0.clone());
    let fault = first.as_ref().and_then(app_exit::fault_of).cloned();
    let write = first.as_ref().and_then(app_exit::write_failure_of).cloned();
    let fault_exit = fault.is_some() || write.is_some();
    if world.contains_resource::<session_end::SessionEnded>() {
        // OS のセッションの終了の中で降ろして印も始末した。告知は OS の終了を塞ぐので出さない。
        tracing::info!(
            event = "session_end_already_handled",
            first = ?first,
            "[main] OS のセッションの終了で後始末は済んでいる——告知を出さず、起動中の印にも触れません"
        );
        return AfterRun {
            session,
            scene: None,
            mark: None,
            fault: fault_exit,
        };
    }
    let Some(ctx) = world.get_resource::<boot_config::BootContext>() else {
        // `main` が系の登録の直後に据える。無ければ配線の誤りで、告知の場所も記憶の置き場も組めない。
        tracing::error!(
            event = "boot_context_missing",
            "[main] 起動の文脈が無い——告知を組めず、起動中の印も消せません（次の起動は前回落ちたとして Ref6/7 付きになります）"
        );
        return AfterRun {
            session,
            scene: None,
            mark: None,
            fault: fault_exit,
        };
    };
    let (ghost_name, ghost_root) = match first {
        Some(app_exit::ExitOrigin::GhostFallbackFailed(_)) => (
            Some(boot_resolve::DEFAULT_GHOST_FOLDER.to_owned()),
            // 既定ゴーストの場所は和から引く（どの根にも無ければ並びの先頭の根の下）。
            areka_ghost::catalog_roots::find_ghost(&ctx.roots, boot_resolve::DEFAULT_GHOST_FOLDER)
                .map(|e| e.dir)
                .unwrap_or_else(|| {
                    ctx.roots
                        .roots()
                        .first()
                        .map(|r| r.ghost_dir(boot_resolve::DEFAULT_GHOST_FOLDER))
                        .unwrap_or_default()
                }),
        ),
        _ => (
            session
                .as_ref()
                .and_then(ghost_session::GhostSession::ghost_name)
                .or_else(|| ctx.current.ghost.folder.clone()),
            ctx.current.cfg.ghost_root.clone(),
        ),
    };
    let mark = MarkInputs {
        app_profile_dir: ctx.app_profile_dir.clone(),
        argv_session: ctx.argv_session,
        first,
        logsink_fallback: session
            .as_ref()
            .is_some_and(ghost_session::GhostSession::logsink_fallback),
    };
    let scene = fault
        .map(|fault| alert::AlertScene::ShioriFault {
            ghost_name,
            // argv で相対パスを渡されても告知には絶対パスを載せる。
            ghost_root: std::path::absolute(&ghost_root).unwrap_or(ghost_root),
            fault,
        })
        .or_else(|| {
            write.map(|failure| alert::AlertScene::MemoryWrite {
                failure,
                moment: alert::WriteMoment::Running,
            })
        });
    AfterRun {
        session,
        scene,
        mark: Some(mark),
        fault: fault_exit,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// `finish_after_run` の単体テスト。
#[cfg(test)]
#[path = "main_finish_after_run_tests.rs"]
mod finish_after_run_tests;

/// 据え付けと `run()` の後・起動中の印のテスト。
#[cfg(test)]
#[path = "main_session_mark_tests.rs"]
mod session_mark_tests;

/// UI の基盤の初期化の失敗が記録に残ることの本文の確かめ。
#[cfg(test)]
#[path = "main_winapp_log_tests.rs"]
mod main_winapp_log_tests;
