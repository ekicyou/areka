#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! areka 本番アプリ骨格
//!
//! wintf を使ったデスクトップマスコット「ぱすたさん」の本番アプリ。骨格は「アプリ起動の器」に徹する:
//! - 構造化ロギング初期化（RUST_LOG フォールバック）・パニックハンドラ設定
//! - 構成入力（ゴースト／バルーンのルートパス）の解決とログ出力（マウントはしない）
//! - UI ランタイム起動（`WinApp::with_exit_policy(ExitPolicy::Explicit)`）・SHIORI 実走デモの env-gate 呼び口
//! - 起動窓シーム（`ghost_session::open_ghost_windows`・本物のゴースト窓を開く。準備が通らなければ
//!   「起動窓を開けない」の告知の上で終了コード 1 で終える）
//! - `main` 自身が所有するメッセージループ（`app.run()`）と終了の指示（`app_exit::quit_app`）での正常終了
//!
//! 座標・配置ロジックは `placement` モジュールが所有し、骨格自身は座標を一切持たない。
//!
//! `main` は `WinApp` 構築／`ghost_session::open_ghost_windows` の後で `ghost_session::boot_ghost` を呼び、
//! 実 sink boot（`wired=true`）／`LogSink`×2 フォールバック boot（`wired=false`）の呼び分けはそこが行う。
//! `run()` 復帰後は `GhostSession::shutdown(CloseReason::User { scope: 0 })`（loop ticker → 終了統括 →
//! seriko join）で終了を総仕上げする。boot 失敗は非致命として扱い骨格起動を止めない。

use bevy_ecs::prelude::*;
use windows::core::Result;
use wintf::*;

/// areka 本体側 `IShioriHost` 実装（単一 sink・突合枠・メールボックス投函）。
/// 脳（`IShiori` 実装）が `Load` で受け取る sink を areka 側で実装する。
mod shiori_host;

/// in-proc アクティベーション経路とリクエスト利用規律（単一 in-flight・遅延完了タイムアウト）。
/// in-proc の `IShiori`（脳）へ到達し `Load` で sink を渡す最小経路と、単一 in-flight・
/// `Unload` 保留取消・設定可能タイムアウトの利用規律を所有する。
mod shiori_session;

/// 製品コード（非テスト）のリファレンス脳＋ファクトリ＋C 入口。`#[implement(IShiori)]`/
/// `#[implement(IShioriFactory)]` 実装＋純粋C コンストラクタ `shiori_factory` を所有する正解見本。
mod reference_brain;

/// 実走デモドライバ。`shiori_factory`→`ShioriSession` で activate→数往復 get→
/// `poll_completions`→raise/notify 観測→drop teardown を駆動し tracing で観測する。
mod shiori_demo;

#[cfg(test)]
mod crate_source_test_support;
/// 配置の crate は本体を使えないので、配置と本体のほかの部分をまたぐテストと判定器はここに置く。
#[cfg(test)]
mod placement_tests;
/// `areka-runtime` は本体を使えないので、本体の部品も使う runtime のテストはここに置く。
#[cfg(test)]
mod runtime_tests;
/// 本体のテストへ emo2 検体を配る入口（crate ごとに 1 つ）。
#[cfg(test)]
mod sample_test_support;

/// tick の門の既定を起動時に上書きする読み口（`AREKA_TICK_GATE=1|0`）。
mod tick_gate_config;

/// 本体に頼る切替・読み直し・起動・終わる相の層。
mod session;

/// 終了の統合操作。全窓を閉じてから終了を指示する `quit_app` と出所の語彙 `ExitOrigin` を持つ。
mod app_exit;
mod mcp;
mod menu;
/// 乗算済み BGRA → 透過 PNG（MCP とオフラインの書き出しが共有）。
mod png_image;
mod readme;
/// OS のセッションの終了（`WM_ENDSESSION`）を窓の手続きの中できれいな終わりにする受け手。
mod session_end;

/// 起こし直しの単位。系の登録の入口 `register_systems` を持つ。
mod ghost_session;

/// アクタースレッドの役割名の宣言。
/// `areka-actor` のスレッド開始フックを導入し、生成されるアクタースレッド 1 本ごとに
/// 役割名を宣言して wintf のスレッド名簿へ登録する。
mod thread_roles;

/// スレッド別・プロセス全体の CPU 報告器。
/// target `areka::perf` が点いているときだけ報告スレッドを起こし、名簿を舐めた
/// `perf(thread)` 行と `perf(process)` 行を周期＋終了直前に出す。消灯時は費用 0。
mod perf_thread_report;

/// 遅延応答と push 経路の end-to-end 結合テスト。
/// モック脳が `SHIORI_S_PENDING`＋token を返し、後で保持 host へ safe `complete`/`raise` を発火する
/// 一連の流れを `ShioriSession` 越しに 1 シナリオで通す（sink/session の単体テストと重複させない）。
#[cfg(test)]
mod shiori_e2e_tests;

/// ライフサイクルと単一 in-flight 規律の end-to-end 結合テスト。
/// 新 ABI の生成〜利用〜teardown（factory create→get→drop teardown・「未ロード状態」は存在しない）と、
/// `Deferred` 保留中の drop 取消→再 activate 後の正常動作を通しシナリオで実証する。
#[cfg(test)]
mod shiori_lifecycle_e2e_tests;

/// 製品 `ReferenceFactory`/`ReferenceBrain` × `ShioriSession` の end-to-end 結合テスト。
/// `shiori_factory` で取得した本物の製品 factory/brain を `ShioriSession` 越しに駆動し、
/// load_dir/shiori_name の貫通（D1）・即時→遅延+complete→raise→notify の数往復・単一 in-flight 拒否・
/// 決定的タイムアウト・stale complete 拒否・drop teardown を実時間 sleep に依存せず検証する。
#[cfg(test)]
mod shiori_reference_e2e_tests;

// ---------------------------------------------------------------------------
// Config Inputs / Ghost Wiring
// ---------------------------------------------------------------------------

/// 起動時の構成入力解決と ghost 結線ヘルパ。
///
/// 本ファイルの 1,000 行規約のため切り出した。可視性は `pub(crate)`。
mod boot_config;

pub(crate) use boot_config::{
    ConfigInputs, default_app_profile_dir, default_helper_exe_path, ghost_boot_options,
    is_benign_boot_error,
};

/// 起動解決の純粋な判断（ゴースト 6 分岐・バルーン 8 分岐）と既定の定数。
/// 起動前の解決は `install::startup::resolve_boot` が、記憶の書き込みは boot 成功直後の `on_boot_ok` が結線する。
mod boot_resolve;

/// `.nar` を入れる手続き。
mod install;

/// ゴースト・シェル・バルーンのネットワーク更新。
mod update;

mod file_drop;

/// ベースウェアの根の並びの決め方。
mod roots;

/// 自動 close の仕掛け（`AREKA_APP_SMOKE_EXIT_MS`・CI smoke）。
mod smoke_exit;

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
    use session::{alert, exit_wait, log_history, readonly, write_failure};
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
    if opts.modules == cli::ModulePolicy::Skip {
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
    let app = WinApp::with_exit_policy(ExitPolicy::Explicit)?;

    // 系の登録（プロセスに 1 回・ここ 1 か所）。停止通知の channel は 1 本だけ作り、受信端をここで
    // 受け口へ据え、送出端の写しを下の起動の 2 経路へ渡す。main 自身の送出端は起動の分岐の後で落とす。
    let (kanade_stop_tx, kanade_stop_rx) = std::sync::mpsc::channel();
    ghost_session::register_systems(app.world().borrow_mut().world_mut(), kanade_stop_rx);
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

    // リファレンス脳の実走デモ。`AREKA_SHIORI_DEMO` が有効なときのみ main スレッドで同期駆動する
    // （既定 OFF）。診断目的なので失敗しても起動を中断せず、`app.run()` より前に完走させる。
    if let Err(e) = shiori_demo::run_demo_if_enabled() {
        tracing::error!(error = %e, "[main] shiori reference demo failed");
    }

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

    smoke_exit::arm(&app);

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

    // main 自身の送出端を落とす（受け口に残る送出端は起動の 2 経路と起動入力の作り口が持つ写しだけ）。
    drop(kanade_stop_tx);

    // `main` 所有のブロッキングメッセージループ。窓が 0 になっても戻らず、`app_exit::quit_app` の
    // 終了の指示で `run()` が戻る。失敗でも後始末は通すので `?` で抜けない。
    let run = app.run();
    // MCP の受け口を外す（溜まった要求は即座に "shutting down"・`begin_close` より前）。
    mcp::close(app.world().borrow_mut().world_mut());
    // `run` は下で `finish_after_run` へ渡すので、成否をここで控える。
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
/// 作り口は本番の入力の組み立て（[`ghost_session::GhostBootInputs::production`]）を文脈の helper の
/// パスと停止通知の送出端の写しと文脈の記憶の置き場で閉じたもの（切替が相手のゴーストを起こすときにも同じ入力になる）。
fn install_boot_context(
    world: &mut World,
    ctx: boot_config::BootContext,
    kanade_stop: std::sync::mpsc::Sender<areka_kanade::KanadeNotice>,
    modules: cli::ModulePolicy,
) {
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
    cfg: &ConfigInputs,
    origin: areka_kanade::BootOrigin,
    descript: &ghost_session::StartupDescriptValues,
    ghost: &boot_resolve::GhostDecision,
    balloon: &boot_resolve::BalloonDecision,
) -> std::result::Result<(), areka_sylphya::PersistFailure> {
    let argv_session = world
        .get_resource::<boot_config::BootContext>()
        .is_some_and(|ctx| ctx.argv_session);
    match world.get_resource::<boot_config::BootContext>() {
        Some(ctx) if !ctx.argv_session => boot_resolve::write_session_mark(
            &ctx.app_profile_dir,
            &boot_resolve::running_name(ghost),
        )?, // 書けなければゴーストを起こさない（`main` が起動を止める）
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
    session: Option<ghost_session::GhostSession>,
    /// 告知の場面（最初の出所が Fault か記憶を書けなかったもので、OS のセッションの終了を処理していないときだけ）。
    scene: Option<session::alert::AlertScene>,
    /// 起動中の印の判定の材料（起動の文脈が無いか、OS のセッションの終了で済んでいれば `None`＝
    /// 印に触れない）。判定は降ろした後。
    mark: Option<MarkInputs>,
    /// 終了コードを 1 にするか（最初の出所に失敗の中身がある＝`fault_of`／`write_failure_of`・告知の場面の有無とは独立）。
    fault: bool,
}

/// 起動中の印の判定の材料（`run()` の後に組み、降ろした後に [`settle_session_mark`] へ渡す）。
struct MarkInputs {
    /// 記憶の置き場（App スコープ）。
    app_profile_dir: std::path::PathBuf,
    /// 起動の文脈の argv の旗。
    argv_session: bool,
    /// 最初に終了を指示した出所（無ければ `None`）。
    first: Option<app_exit::ExitOrigin>,
    /// 最初の起動が LogSink へ倒れたか（倒れた先の成否を問わない）。
    logsink_fallback: bool,
}

/// 降ろした結果（`fn main` の後始末は `shiori_cut=false`・OS のセッションの終了は見張りの結果）。
#[derive(Debug, Clone, Copy)]
struct Teardown {
    /// メッセージループ（`run()`）が成功したか。
    run_ok: bool,
    /// 降ろす処理が成功したか。
    down_ok: bool,
    /// OS のセッションの終了で SHIORI の待ちを上限で打ち切ったか。
    shiori_cut: bool,
}

/// 置き場の単位を取り出し、最初に終了を指示した出所から告知の場面と印の判定の材料を組む。
///
/// 告知のゴーストは置き場の単位の名前と文脈の今のゴーストの根（切替後の今のゴースト）。
/// 切替の途中の致命（既定へ戻せなかった）では置き場が空で、文脈の今のゴーストは最後に起きた
/// ゴーストのまま残るので、起こそうとしていた既定ゴーストの名前（フォルダ名）と場所で組む
/// （致命になるのは既定ゴーストを起こせなかったときだけ＝`ghost_switch::fatal` の呼び手）。
fn after_run(world: &mut World) -> AfterRun {
    use session::alert;
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

/// きれいな終わりの判定の結論。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkVerdict {
    /// きれいに終わった＝印を消す。
    Clear,
    /// きれいに終わらなかった＝印を残す（理由は記録の `reason`）。
    Keep(&'static str),
    /// argv で始まったプロセス＝印に触れない。
    Untouched,
}

/// きれいな終わりの判定（後始末と OS のセッションの終了が共有する唯一の判定）。
///
/// 時系列で最初に起きた理由を採る。上から順に: argv で始まったプロセスは触らない → 最初の起動が
/// LogSink へ倒れた → 最初の出所が無い → 出所が失敗（SHIORI の失敗・既定へ戻せない致命）→ `run()` の
/// 失敗 → OS のセッションの終了で SHIORI の待ちを上限で打ち切った → 降ろす処理の失敗 → それ以外は
/// 消す。出所は網羅の match で見るので、出所を足すとこの判定の漏れがコンパイルで止まる。
/// 名前は見ないので、倒れたのが既定ゴースト自身でも同じく残す。
fn session_mark_verdict(
    first: Option<&app_exit::ExitOrigin>,
    argv_session: bool,
    logsink_fallback: bool,
    end: Teardown,
) -> MarkVerdict {
    use app_exit::ExitOrigin;
    use areka_kanade::KanadeStopCause;
    if argv_session {
        return MarkVerdict::Untouched;
    }
    if logsink_fallback {
        return MarkVerdict::Keep("logsink_fallback");
    }
    let Some(first) = first else {
        return MarkVerdict::Keep("no_exit_origin");
    };
    match first {
        ExitOrigin::KanadeStopped(KanadeStopCause::Fault(_)) => return MarkVerdict::Keep("fault"),
        ExitOrigin::GhostFallbackFailed(_) => return MarkVerdict::Keep("switch_fatal"),
        ExitOrigin::KanadeStopped(
            KanadeStopCause::Quit
            | KanadeStopCause::Forced
            | KanadeStopCause::CloseSilent
            | KanadeStopCause::DeadlineExceeded,
        )
        | ExitOrigin::Escape
        | ExitOrigin::Smoke
        | ExitOrigin::OsClose
        | ExitOrigin::SessionEnd
        | ExitOrigin::MemoryWriteFailed(_) => {}
    }
    if !end.run_ok {
        return MarkVerdict::Keep("run_failed");
    }
    if end.shiori_cut {
        return MarkVerdict::Keep("session_end_deadline");
    }
    if !end.down_ok {
        return MarkVerdict::Keep("down_failed");
    }
    MarkVerdict::Clear
}

/// 降ろした後の印の始末: 判定して、消すなら消し（`info!(session_mark_cleared)`）、残すなら理由つきの
/// `info!(session_mark_kept)`（`boot_resolve::keep_session_mark`）、argv なら `debug!` だけ。
/// 呼ぶのはゴーストの実行系が 0 の時点だけ。印を消せなかった失敗は組の 2 つ目で返す（記録済み）。
fn settle_session_mark(
    mark: &MarkInputs,
    end: Teardown,
) -> (MarkVerdict, Option<areka_sylphya::PersistFailure>) {
    let verdict = session_mark_verdict(
        mark.first.as_ref(),
        mark.argv_session,
        mark.logsink_fallback,
        end,
    );
    let failed = match verdict {
        MarkVerdict::Clear => boot_resolve::clear_session_mark(&mark.app_profile_dir).err(),
        MarkVerdict::Keep(reason) => {
            boot_resolve::keep_session_mark(&mark.app_profile_dir, reason, mark.first.as_ref());
            None
        }
        MarkVerdict::Untouched => {
            tracing::debug!(
                event = "session_mark_untouched_argv",
                "[main] argv で始まったプロセスなので起動中の印に触れません"
            );
            None
        }
    };
    (verdict, failed)
}

// ---------------------------------------------------------------------------
// Startup Window
// ---------------------------------------------------------------------------

/// 復元マージのシーム。
///
/// `ghost_session::open_ghost_windows` が `spawn_ghost_windows` へ渡す placements を、起動時に
/// 先読みした永続 entries で差し替える（保存位置優先・毎起動 live 再射影）。`open_ghost_windows` は
/// 実モニタの列挙と作業プール（`WintfTaskPool`）を要してテスト困難なので、純粋シーム（load→apply）を
/// 本ヘルパへ抽出して単体で檻に入れる（IO は [`areka_placement::persist::load_restored_state`] の
/// 1 点のみ・merge は純関数）。
fn restore_merged_placements(
    ghost_root: &std::path::Path,
    placements: Vec<areka_placement::resolver::ScopePlacement>,
    snapshot: &areka_placement::follow::MonitorSnapshot,
) -> (
    Vec<areka_placement::resolver::ScopePlacement>,
    std::collections::BTreeMap<usize, Option<areka_placement::chain_finalize::AdjacencyMark>>,
    bool,
) {
    // 唯一の IO 点: mount 解決 → Ghost スコープ永続 entries 先読み。
    let entries = areka_placement::persist::load_restored_state(ghost_root);
    // 純関数 merge（永続不書込・保存位置優先 → project_restore → balloon 導出）と、記憶の位置を
    // 寄せて表示したかの判定（窓を作る閉包が `RestoredPositionsClamped` へ据える）。
    let (merged, clamped) =
        areka_placement::persist::restore_and_judge(placements, &entries, snapshot);
    // 起動時関門: 経路①（spawn 初期値）と経路②（復元 merge）はどちらもこの合流点の出力を消費するため、
    // ここ 1 点で両方が被覆される。merge **後**に置くのは保存値優先の合流規則を変えないためで、補正は
    // `balloon_pos`（表示位置）だけに作用する——`balloon_offset`（論理相対位置）は生値のまま
    // （補正を焼き付けない）。
    let merged = areka_placement::balloon_limit::apply_balloon_limit(merged, snapshot);
    // 記憶に位置（x・y の両方）があるスコープ集合。起動の並べ直しは覚えた印で、拡大率の変化の後は
    // 変わる直前の隣り合いで決める。値の違いでなく記憶の有無で決める＝記憶の値がたまたま既定と同じでも
    // 記憶として扱う（判定は merge と同じ読み取り `has_saved_char_pos`）。値は覚えた隣り合いの印
    // （無ければ None）で、窓を作る閉包が窓の一式へ据える。
    let restored = areka_placement::persist::restored_scope_marks(&entries, &merged);
    (merged, restored, clamped)
}

/// `PersistWiring`（NonSend）を、ゴースト窓を保持する同一 World へ挿入するシーム。
///
/// wired 経路（実 sink 結線成立）と fallback boot 経路（`LogSink`×2）の**両方**が、生きた ghost
/// runtime が存在するときにこのヘルパで `runtime.sylphya_publisher().clone()` を World の NonSend
/// リソースとして挿入する。以降、follow.rs の DragEnd 観測点が
/// [`areka_placement::persist::persist_entries`] 経由でこの publisher の clone 送信端から保存 entries を
/// write-through 投函できる。
///
/// 生きた runtime が無い経路（wired の `None` ghost・fallback の `Err`）では挿入しない＝永続結線なし
/// （`persist_entries` は `PersistWiring` 不在で debug!＋no-op へ縮退）。挿入は純粋な World 変異ゆえ
/// headless 単体テスト可能。
/// を薄く包み `#[cfg(test)]` の檻に入れる）。
fn insert_persist_wiring(world: &mut World, publisher: areka_sylphya::SylphyaPublisher) {
    world.insert_non_send(areka_placement::persist::PersistWiring { publisher });
}

/// boot が `Ok` を返した直後の結線（wired／fallback の両アームが呼ぶ 1 か所）。
///
/// 位置永続の導管を挿入し（[`insert_persist_wiring`]）、続けて起動成功時の記憶を書く
/// （[`record_last_used_at_boot`]）。
fn on_boot_ok(
    world: &mut World,
    runtime: &areka_ghost::GhostRuntime,
    ghost: &boot_resolve::GhostDecision,
    balloon: &boot_resolve::BalloonDecision,
) {
    insert_persist_wiring(world, runtime.sylphya_publisher().clone());
    record_last_used_at_boot(
        runtime.sylphya_publisher(),
        &runtime.mount().shell.dir,
        ghost,
        balloon,
    );
}

/// 起動の呼び出しが返った時点の記憶（初回の起動・既定への戻しはここで書く）。
///
/// 経路が切替なら書かない: 壊れた切替先の名前を最後のゴーストに一瞬も載せないため、切替先の記憶は
/// 定常到達で `ghost_switch::record_steady_memory` が同じ [`record_last_used`] で書く。
fn record_last_used_at_boot(
    publisher: &areka_sylphya::SylphyaPublisher,
    shell_dir: &std::path::Path,
    ghost: &boot_resolve::GhostDecision,
    balloon: &boot_resolve::BalloonDecision,
) {
    if ghost.route == boot_resolve::GhostRoute::Switched {
        tracing::debug!(
            event = "last_used_deferred",
            ghost = ?ghost.folder,
            "[main] 切替で起こしたゴーストの記憶は定常到達まで書きません"
        );
        return;
    }
    record_last_used(publisher, shell_dir, ghost, balloon);
}

/// 最後に使ったもの（ゴースト・バルーン・シェル）を、動いているゴーストの記憶の書き手へ投函する。
/// 書き込みは投函だけで待たない（反映は `shutdown` の barrier に任せる）。シェルのフォルダ名は
/// `shell_dir`（`mount().shell.dir`）の末尾（非 UTF-8 は `to_string_lossy` で写す）。呼び手は起動の直後と
/// 切替の定常到達（`ghost_switch::record_steady_memory`）。
pub(crate) fn record_last_used(
    publisher: &areka_sylphya::SylphyaPublisher,
    shell_dir: &std::path::Path,
    ghost: &boot_resolve::GhostDecision,
    balloon: &boot_resolve::BalloonDecision,
) {
    let Some(shell_folder) = shell_dir.file_name().map(|n| n.to_string_lossy()) else {
        // `<ゴースト>/shell/<名>` の形で末尾が無いことは起きない。来たら書かずに残す。
        tracing::error!(
            event = "last_used_shell_folder_missing",
            shell_dir = %shell_dir.display(),
            "[main] シェルのフォルダ名が取れないので記憶を書きません"
        );
        return;
    };
    boot_resolve::LastUsed {
        ghost,
        balloon,
        shell_folder: &shell_folder,
    }
    .record(publisher);
}

/// 起動時モニタスナップショットの構築＋出力シーム。
///
/// [`areka_placement::follow::MonitorSnapshot`] は **placement の全判断が読む権威**（work area 解決・
/// アンカー射影・可視性判定がすべてこの Resource を引く）なので、正典の出力点はこの構築点にする。
/// ここで観測した値だけが「以後の判断が実際に見た値」であり、他所で列挙し直した値ではない。
///
/// 出力は共有ヘルパ [`areka_placement::diag::log_monitor_snapshot`] 1 本で、呼出点タグ
/// [`areka_placement::MONITOR_SNAPSHOT_CONTEXT`] を名乗る。`prepare_ghost_windows` の列挙点も
/// 同じヘルパを別タグで呼ぶため、語彙は共有したままログ上で出所を弁別でき、両者の食い違いは
/// grep 突合で検出できる。
///
/// 観測を足すだけで snapshot の中身は変えない。実モニタを列挙する `ghost_session::open_ghost_windows`
/// から切り出したシームゆえ、合成モニタで headless 檻に入る。
///
/// # 2 源を同じ構築関数で作る
///
/// 返すのは作業領域源と**モニタ別拡大率表**の組である。実行時の同期段
/// （`areka_runtime::frame::work_area_sync`）も同じ [`areka_placement::follow::MonitorSources::from_monitors`]
/// を通る——起動時だけが別の作り方をすると、同期が入った後も起動時の値だけが違う形になり得る。
fn boot_monitor_snapshot(
    monitors: &[wintf::ecs::window::monitor::Monitor],
) -> areka_placement::follow::MonitorSources {
    areka_placement::diag::log_monitor_snapshot(
        &areka_placement::monitor_records(monitors),
        areka_placement::MONITOR_SNAPSHOT_CONTEXT,
    );
    areka_placement::follow::MonitorSources::from_monitors(monitors)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "main_config_input_tests.rs"]
mod config_input_tests;

/// ghost 結線ヘルパの headless 単体テスト。
///
/// `areka_ghost::boot`／`GhostRuntime::shutdown` 自体は実 I/O・実スレッドを伴うため、
/// ここでは純粋な組み立て・分類ロジック（`default_helper_exe_path`／`ghost_boot_options`／
/// `is_benign_boot_error`）だけを headless に検証する。実際の boot→shutdown 一巡は
/// 既存の実プロセス smoke テスト（`tests/smoke_boot_loop_exit.rs`）が証明する。
#[cfg(test)]
#[path = "main_ghost_wiring_tests.rs"]
mod ghost_wiring_tests;

/// 復元マージシームの headless 単体テスト。
///
/// `restore_merged_placements`（`load_restored_state`→`apply_restored_placements`）を檻に入れ、
/// 植えた sylphya.toml の保存位置が既定位置に優先して merge 済み placements の char_pos へ載ること／
/// 永続不在なら既定 placement に恒等であることを証明する（＝spawn される窓の初期位置が保存位置に
/// なる結線の証明）。
#[cfg(test)]
#[path = "main_restore_seam_tests.rs"]
mod restore_seam_tests;

/// `PersistWiring` 挿入シームの headless 単体テスト。
///
/// wired／fallback 両経路が使う挿入ヘルパ `insert_persist_wiring` を檻に入れる。実 publisher
/// （`spawn_sylphya`＋共有 fake IO）を headless World へ挿入し、(a) NonSend リソース `PersistWiring` が
/// 存在すること、(b) その World 越しの `persist_entries` 投函が barrier 後に別ハンドルの `load_scope` で
/// 読み戻せる（＝World レベルの配線導管が正しく確立されている）ことを証明する。
#[cfg(test)]
#[path = "main_persist_wiring_seam_tests.rs"]
mod persist_wiring_seam_tests;

/// 起動時モニタスナップショット出力シームの headless 単体テスト。
///
/// [`boot_monitor_snapshot`] を合成モニタ（混在 DPI・負座標・3200 超）で檻に入れる。
/// 実モニタも実 GPU も要さない。
#[cfg(test)]
#[path = "main_monitor_snapshot_seam_tests.rs"]
mod monitor_snapshot_seam_tests;

/// `finish_after_run` の単体テスト。
#[cfg(test)]
#[path = "main_finish_after_run_tests.rs"]
mod finish_after_run_tests;

/// SHIORI の読み方の指定から結線を決める口と倒れ先の表。
#[cfg(test)]
#[path = "main_module_policy_tests.rs"]
mod module_policy_tests;
/// 据え付けと `run()` の後・起動中の印のテスト。
#[cfg(test)]
#[path = "main_session_mark_tests.rs"]
mod session_mark_tests;
#[cfg(test)]
#[path = "main_session_mark_write_tests.rs"]
mod session_mark_write_tests;
