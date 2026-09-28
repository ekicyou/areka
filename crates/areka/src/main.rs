#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! areka 本番アプリ骨格
//!
//! wintf フレームワークを使用したデスクトップマスコット「ぱすたさん」の本番アプリ骨格。
//! この骨格は「アプリ起動の器」に徹する:
//! - 構造化ロギング初期化（RUST_LOG フォールバック）・パニックハンドラ設定
//! - 構成入力（ゴースト／バルーンのルートパス）の解決とログ出力（マウントはしない）
//! - UI ランタイム起動（`WinApp::with_exit_policy(ExitPolicy::Explicit)`）・SHIORI 実走デモの env-gate 呼び口
//! - 起動窓シーム（`ghost_session::open_ghost_windows`・本物のゴースト窓を開く。準備が通らなければ
//!   「起動窓を開けない」の告知の上で終了コード 1 で終える）
//! - `main` 自身が所有するメッセージループ（`app.run()`）と終了の指示（`app_exit::quit_app`）での正常終了
//!
//! 座標・配置ロジックは `placement` モジュール（areka-P0-window-placement）が所有し、
//! 骨格自身は座標を一切持たない。旧モック UI は `examples/mock-shell.rs` へ退避済み。
//!
//! `main` は `WinApp` 構築／`ghost_session::open_ghost_windows` の後で `ghost_session::boot_ghost` を呼び、
//! 実 sink boot（`wired=true`）／既存 `LogSink`×2 フォールバック boot（`wired=false`）の呼び分けはそこが
//! 行う（task 5.2・design.md「エントリポイント / main.rs＋wire_emo2_boot」・DD-7）。`run()` 復帰後は
//! `GhostSession::shutdown(CloseReason::User { scope: 0 })`（DD-10・loop ticker → 終了統括 → seriko join）で
//! 終了を総仕上げする。boot 失敗は非致命として扱い骨格起動を止めない（要件 7.3・8.2）。

use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::core::Result;
use wintf::*;

/// areka 本体側 `IShioriHost` 実装（単一 sink・突合枠・メールボックス投函）。
/// 脳（`IShiori` 実装）が `Load` で受け取る sink を areka 側で実装する（task 4.1）。
mod shiori_host;

/// in-proc アクティベーション経路とリクエスト利用規律（単一 in-flight・遅延完了タイムアウト）。
/// in-proc の `IShiori`（脳）へ到達し `Load` で sink を渡す最小経路と、単一 in-flight・
/// `Unload` 保留取消・設定可能タイムアウトの利用規律を所有する（task 4.2）。
mod shiori_session;

/// 製品コード（非テスト）のリファレンス脳＋ファクトリ＋C 入口。`#[implement(IShiori)]`/
/// `#[implement(IShioriFactory)]` 実装＋純粋C コンストラクタ `shiori_factory` を所有する正解見本。
mod reference_brain;

/// 実走デモドライバ。`shiori_factory`→`ShioriSession` で activate→数往復 get→
/// `poll_completions`→raise/notify 観測→drop teardown を駆動し tracing で観測する。
mod shiori_demo;

/// 窓配置機構（areka-P0-window-placement）。ゴースト定義からキャラ窓・バルーン窓の
/// 初期配置を解決し窓 entity を組み立てる配置パイプライン。`ghost_session::open_ghost_windows`
/// シーム（task 6.2）が `prepare_ghost_windows`→`spawn_ghost_windows` を結線する。
mod placement;

/// tick の門の既定を起動時に上書きする読み口（`AREKA_TICK_GATE=1|0`）。
mod tick_gate_config;

/// emo2 統合結線（areka-P0-emo2-boot）。完成済み 5 トラックのエンジンを束ね、シェル
/// アニメーション側の表示指令を表示層の指令へ変換するアダプタ＋各エンジン結線＋観測を
/// 所有する（`target_map`／`adapter`／`talk_clock`／`assets`／`frame`＋`BootWiringError`・
/// `wire_emo2_boot`）。
mod emo2_boot;

/// UI→kanade のマウス入力配信配線（areka-P0-input-events）。キャラ窓のポインタイベントを
/// 捉え、当たり判定名を resolver で解決し、送出間引き（`throttle`）を通して kanade へ配信する
/// 薄い配線層。現状は `throttle`（送出間引きの純粋判定・task 2.4）のみ。ポインタハンドラ結線と
/// per-scope 状態保持（`MouseWiring`）は task 2.6／2.7 で増設される。
mod input_events;

/// 終了の統合操作（areka-P0-app-lifetime-separation）。全窓を閉じてから終了を指示する
/// `quit_app` と出所の語彙 `ExitOrigin` を持つ。
mod app_exit;
mod menu;
mod readme;
/// OS のセッションの終了（`WM_ENDSESSION`）を窓の手続きの中できれいな終わりにする受け手。
mod session_end;

/// 起こし直しの単位（areka-P0-ghost-restart-unit）。系の登録の入口 `register_systems` を持つ。
mod ghost_session;

/// アクタースレッドの役割名の宣言（areka-P0-draw-load-parity task 2.3）。
/// `areka-actor` のスレッド開始フックを導入し、生成されるアクタースレッド 1 本ごとに
/// 役割名を宣言して wintf のスレッド名簿へ登録する。
mod thread_roles;

/// スレッド別・プロセス全体の CPU 報告器（areka-P0-draw-load-parity task 2.4）。
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
/// 本ファイルが 1,000 行規約を超えたため切り出した。挙動は不変で、可視性を
/// `pub(crate)` へ広げただけである（詳細はモジュール doc を参照）。
mod boot_config;

pub(crate) use boot_config::{
    ConfigInputs, default_app_profile_dir, default_helper_exe_path, ghost_boot_options,
    is_benign_boot_error, resolve_boot,
};

/// 無いときの告知（根なし／ゴーストなし／バルーンなし／起動窓を開けない）と
/// 告知の抑止（`AREKA_NO_ALERT`）。
mod alert;

/// 起動解決の純粋な判断（ゴースト 6 分岐・バルーン 7 分岐）と既定の定数。
/// 起動前の解決は `boot_config::resolve_boot` が、記憶の書き込みは boot 成功直後の `on_boot_ok` が結線する。
mod boot_resolve;

/// `.nar` を入れる手続き（areka-P0-ghost-install）。中身は後続のタスクで足す。
mod install;

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    human_panic::setup_panic!();

    // tracing-subscriber 初期化（RUST_LOG環境変数対応、デフォルト info）
    // 外部入力の扱い（A1-V）: RUST_LOG が未設定・非UTF-8・不正な構文の場合は
    // try_from_default_env() が Err を返し "info" へフォールバックする（panic 経路なし）。
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // アクタースレッドの役割宣言フックを導入する（draw-load-parity 要件 2.3）。
    // 以後 `spawn_actor` で起きるスレッド（ticker／loop-ticker／各アクター）は走り始めに
    // 自分の役割名を宣言して wintf のスレッド名簿へ載る。**すべての結線より前**に置くのは、
    // これより前に起きたスレッドが名簿から漏れるため。登録以外の挙動は何も変わらない。
    thread_roles::install();

    // スレッド別 CPU の報告器（draw-load-parity 要件 2.3/3.8・task 2.4）。点灯の判定は
    // ここで 1 度だけ行い、`RUST_LOG` に `areka::perf=debug` が無ければ報告スレッドを
    // 起こさない（既定運転の費用 0）。取っ手は終了直前まで持ち回り、最後の 1 枚を出す。
    let perf_report = perf_thread_report::start();

    // 起動前の解決（baseware-root-layout design「起動解決（WinApp 構築の前）」）:
    // 根 → ゴースト → バルーン → 構成入力。決まらなければ告知して終了コード 1 で終える
    // （要件 1.4・4.7・4.8・5.8・6.3）。`warn!` で起動を続ける経路は作らない（要件 6.6）。
    // 決まった経路とフォルダは boot の分岐まで持ち越す（boot 成功直後の記憶の書き込みに使う）。
    let args: Vec<String> = std::env::args().collect();
    // 4 つ目は起動中の印の値＝前回きれいに終わらなかったゴーストの名前（読むだけで消さない）。
    // 初回の起動の由来になる（要件 12.4）。
    let (cfg, ghost_decision, balloon_decision, halted, root) = match resolve_boot(&args) {
        Ok(resolved) => resolved,
        Err(scene) => {
            alert::raise(&scene, alert::suppressed());
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ));
        }
    };

    // 実行ファイル隣接の 32bit SHIORI helper パスを一度だけ解決する（起動の文脈と起動入力の作り口が
    // 持ち、実 sink 結線経路と `LogSink` フォールバック boot 経路の双方が使う・DD-7）。
    let helper_exe = default_helper_exe_path();

    // UI ランタイム起動（COM/DPI 初期化・World 生成・終了の受け口 `AppExit` の据え付け）（R2.4）。
    // DD-7/R7.1: 実 sink 結線（`wire_emo2_boot`）は UI 基盤の後に行うため、`WinApp` 構築を
    // すべての boot より前へ移動した（旧・task 3.3 の boot 先行順序を再編）。
    // `Explicit`: 窓 0 では終了せず、`quit_app` の終了の指示でだけ `run()` が戻る（要件 1.1）。
    let app = WinApp::with_exit_policy(ExitPolicy::Explicit)?;

    // 系の登録（プロセスに 1 回・ここ 1 か所・areka-P0-ghost-restart-unit 要件 2.1／2.2）。
    // 停止通知の channel は 1 本だけ作り、受信端をここで受け口へ据え、送出端の写しを下の起動の
    // 2 経路へ渡す（要件 6.4）。main 自身の送出端は起動の分岐の後で落とす。
    let (kanade_stop_tx, kanade_stop_rx) = std::sync::mpsc::channel();
    ghost_session::register_systems(app.world().borrow_mut().world_mut(), kanade_stop_rx);

    // 起動の文脈と起動入力の作り口（本番版）を据える（切替と右クリックメニューの「ゴースト」枠が読む）。
    install_boot_context(
        app.world().borrow_mut().world_mut(),
        boot_config::BootContext {
            root,
            app_profile_dir: default_app_profile_dir(),
            helper_exe,
            // argv で始まったプロセスか（ここで 1 度だけ決め、以後変えない・要件 12.5）。
            argv_session: ghost_decision.route == boot_resolve::GhostRoute::Argv,
            current: boot_config::CurrentGhost {
                cfg: cfg.clone(),
                ghost: ghost_decision.clone(),
                balloon: balloon_decision.clone(),
            },
        },
        kanade_stop_tx.clone(),
    );

    // tick の門の既定を起動時に一度だけ上書きする（`AREKA_TICK_GATE=1|0`・A/B と安全弁）。
    tick_gate_config::apply_from_env(&mut app.world().borrow_mut());

    // リファレンス脳の実走デモ（要件 5.3/5.4）。環境変数 `AREKA_SHIORI_DEMO` が有効な
    // ときのみ main スレッドで同期駆動する（既定 OFF）。診断目的のため失敗しても通常
    // 起動を中断せず、`app.run()` の UI 立ち上げを阻害しないよう必ずその前に完走させる。
    if let Err(e) = shiori_demo::run_demo_if_enabled() {
        tracing::error!(error = %e, "[main] shiori reference demo failed");
    }

    // 起動窓シーム（window-placement 1.4）: ゴースト定義から本物のゴースト窓
    // （キャラ窓＋バルーン窓）を配置・生成する。準備が通らなければ（モニタ 0 台・起動中の
    // 削除等）「起動窓を開けない」を告知して終了コード 1 で終える（baseware-root-layout
    // 要件 6.4）。窓 0 で居座る経路は作らない（要件 6.6）。
    // 戻り値は配置準備が **1 度だけ**読んだ descript 由来の 2 値（areka-P0-emo-dpi-scaling
    // task 4.3・design Flow 3 手順1）。作者基準 DPI は採寸の k₀ と直下の `wire_emo2_boot`
    // （→`attach_target`）の双方へ渡り、採寸と表示が別々の宣言を見る食い違いを構造的に
    // 排除する。shell の `seriko.zorder` も同じ搬送に乗せる（areka-P0-scope-zorder-pinning
    // 要件 5.1／5.2）——重なりの基底の出所を配置と同じ 1 度の読取に揃えるためである。
    // 配置の失敗と作業プールの欠落はどちらも同じ告知＋終了コード 1。
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

    // env ゲート付き自動 close 機構（CI smoke・task 2.3・R4.1）。
    // `AREKA_APP_SMOKE_EXIT_MS` が有効なミリ秒値のときだけ、VSync relay と同じ
    // `wintf::executor::spawn_local`＋world `Weak` 作法で **一発の** async タスクを投入する
    // （ECS システムではない）。投入は 1 度目の窓の準備が通ったとき（ここ）だけ
    // （areka-P0-ghost-restart-unit 要件 3.5）。env 未設定・不正なら
    // 発火せず、ゴースト窓はメニューの「終了」か OS の閉鎖要求を待ち続ける。
    if let Some(ms) = smoke_exit_ms() {
        // WinApp が strong 所有者を保持するため、この Weak は shutdown まで upgrade 可能。
        let world_weak = std::rc::Rc::downgrade(&app.world());
        tracing::info!(
            env = SMOKE_EXIT_ENV,
            delay_ms = ms,
            "smoke 自動 close ゲート有効 — ゴースト窓を指定 ms 後に despawn します"
        );
        wintf::executor::spawn_local(async move {
            // 指定 ms を async スリープ（async-io は既存依存・tokio 不要）。
            async_io::Timer::after(std::time::Duration::from_millis(ms)).await;
            // shutdown 済みなら strong 所有者は消えており upgrade は None ＝ no-op。
            let Some(world) = world_weak.upgrade() else {
                tracing::debug!("smoke 自動 close: world 既に drop 済み（shutdown）— no-op");
                return;
            };
            // await を跨いで borrow を保持しない TIGHT スコープで全窓を閉じ、終了を指示する。
            {
                let mut ecs = world.borrow_mut();
                let w = ecs.world_mut();
                let count = app_exit::quit_app(w, app_exit::ExitOrigin::Smoke);
                tracing::info!(count, "smoke 自動 close: ゴースト窓を despawn しました");
            }
        });
    }

    // ゴーストごとの結線（areka-P0-ghost-restart-unit・`ghost_session::boot_ghost`）: 実 sink 結線を
    // 試み、成立しなければ `LogSink`×2 の fallback へ倒す。3 ハンドル（ゴースト実行系・seriko・
    // loop ticker）は `GhostSession` に束ねて置き場（`GhostSlot`）へ入れる（切替が入れ替え、
    // `run()` の後にここへ取り出して降ろす）。印の値が在れば由来は「前回落ちた」（要件 12.4）。
    // 起こす前に起動中の印を書く（argv で始まったプロセスは書かない・要件 12.1・12.5）。
    boot_first_ghost(
        app.world().borrow_mut().world_mut(),
        &cfg,
        first_boot_origin(halted),
        &descript,
        &ghost_decision,
        &balloon_decision,
    );

    // main 自身の送出端を落とす（受け口に残る送出端は起動の 2 経路と起動入力の作り口が持つ写しだけ）。
    drop(kanade_stop_tx);

    // `main` 所有のブロッキングメッセージループ（R2.4/R4.1）。窓が 0 になっても戻らず、
    // `app_exit::quit_app`（全窓を閉じてから終了を指示）の終了の指示で `run()` が戻る。
    // 失敗でも後始末は通すので `?` で抜けない（要件 3.2・6.3）。
    let run = app.run();
    // 印の判定の材料（`run` は下で `finish_after_run` へ渡すので、成否をここで控える）。
    let run_ok = run.is_ok();

    // 置き場の単位を取り出し、最初に終了を指示した出所から告知の場面と印の判定の材料を組む
    // （要件 1.1・1.3・1.12・12.2）。窓は `quit_app` が閉じ、残りは `run()` が壊してから戻るので、
    // 告知の背後に窓は無い（要件 1.11）。ゴースト名は降ろす前にここで読む（design「告知の位置」⑴）。
    let AfterRun {
        session,
        scene,
        mark,
        fault,
    } = after_run(app.world().borrow_mut().world_mut());

    // 後始末（降ろす → 印の始末 → 告知 → 降ろした結果 → ④）を成否によらず 1 回通し、
    // 終了コードは最後に決める（要件 3.1・3.2）。告知は降ろした後（design「告知の位置」）。① で
    // loop ticker は止まっており、降ろす結果は告知の後で返すので ②③ が失敗しても告知は出る。
    // 利用者が閉じたらそのまま進む（要件 1.10）。
    finish_after_run(run, fault, move || {
        // 降ろす（① loop ticker Close → ② ghost.shutdown → ③ seriko join・`GhostSession::shutdown`）。
        // 終了理由は全窓 close funnel＝ユーザ操作起点（DD-10）。置き場が空なのは切替の途中の致命か、
        // OS のセッションの終了で降ろし済み（`session_end`）のとき。
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

        // 起動中の印の始末（要件 12.2・12.3）: 降ろして記憶の書き出しが済んだ後に、きれいな終わりなら
        // 消し、そうでなければ理由を残して残す。UI スレッドが App スコープへ直接書くのは実行系が
        // 1 つも動いていない間だけ（ここは降ろした後なので書いてよい）。
        if let Some(mark) = &mark {
            settle_session_mark(
                mark,
                Teardown {
                    run_ok,
                    down_ok: down.is_ok(),
                    shiori_cut: false,
                },
            );
        }

        // SHIORI の失敗の告知（1 プロセスに最大 1 回・抑止なら記録だけ・要件 1.1・1.6・1.12）。
        if let Some(scene) = &scene {
            alert::raise(scene, alert::suppressed());
        }
        down?;

        // ④ スレッド別 CPU の最後のスナップショット（task 2.4）。終了直前に 1 枚出してから
        // 報告スレッドを畳む。②③ の失敗で早く戻る経路では最後の 1 枚が出ない
        // （その場合は周期のスナップショットまでが記録として残る）。消灯時は `None` で何もしない。
        if let Some(handle) = perf_report {
            handle.stop_and_report_final();
        }

        Ok(())
    })
}

/// `run()` の結果を受けて後始末へ進む判断（ゴーストを降ろす手順そのものは `ghost-restart-unit` で `GhostSession::shutdown` へ括り出した）。
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
// 置き場と起動の文脈（areka-P0-ghost-shell-balloon-switch task 8.5）
// ---------------------------------------------------------------------------

/// 起動の文脈と起動入力の作り口（本番版）を World へ据える（系の登録の直後に 1 回）。
/// 作り口は本番の入力の組み立て（[`ghost_session::GhostBootInputs::production`]）を文脈の helper の
/// パスと停止通知の送出端の写しで閉じたもの（切替が相手のゴーストを起こすときにも同じ入力になる）。
fn install_boot_context(
    world: &mut World,
    ctx: boot_config::BootContext,
    kanade_stop: std::sync::mpsc::Sender<areka_kanade::KanadeNotice>,
) {
    let helper_exe = ctx.helper_exe.clone();
    world.insert_resource(ctx);
    world.insert_non_send(ghost_session::GhostBootInputsSource(Box::new(
        move |cfg, origin| {
            ghost_session::GhostBootInputs::production(
                cfg,
                helper_exe.clone(),
                kanade_stop.clone(),
                origin,
            )
        },
    )));
}

/// 初回の起動の由来: 起動中の印の値（前回きれいに終わらなかったゴーストの名前）が在れば
/// 「前回落ちた」（`OnBoot` の Ref6/7）、無ければ「ふつう」（要件 12.4）。
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
/// 起動中の印＝起こすゴーストの名前を書く（要件 12.1・12.5）。UI スレッドが App スコープへ直接
/// 書くのは実行系が 1 つも動いていない間だけ（動いている間はそのゴーストの記憶の書き手を通す＝
/// 保存は「読んで重ねて書く」ので、2 つの書き手が同時に書くと片方が消えうる）。
fn boot_first_ghost(
    world: &mut World,
    cfg: &ConfigInputs,
    origin: areka_kanade::BootOrigin,
    descript: &ghost_session::StartupDescriptValues,
    ghost: &boot_resolve::GhostDecision,
    balloon: &boot_resolve::BalloonDecision,
) {
    let argv_session = world
        .get_resource::<boot_config::BootContext>()
        .is_some_and(|ctx| ctx.argv_session);
    match world.get_resource::<boot_config::BootContext>() {
        Some(ctx) if !ctx.argv_session => boot_resolve::write_session_mark(
            &ctx.app_profile_dir,
            &boot_resolve::running_name(&ctx.root, ghost),
        ),
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
    // LogSink へ倒れたら、このプロセスは終わり方によらず印を残す（要件 4.1・4.3）。帰結を 1 件だけ
    // 記録する（倒れた先の成否は `boot_ghost` が今日どおり記録済み）。argv なら印に触れないので `debug!`。
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
}

/// `run()` の後に置き場と起動の文脈から組むもの。
struct AfterRun {
    /// 降ろす単位（置き場が空＝切替の途中の致命か、OS のセッションの終了で降ろし済みなら `None`）。
    session: Option<ghost_session::GhostSession>,
    /// SHIORI の失敗の告知の場面（最初の出所が Fault で、OS のセッションの終了を処理していないときだけ）。
    scene: Option<alert::AlertScene>,
    /// 起動中の印の判定の材料（起動の文脈が無いか、OS のセッションの終了で済んでいれば `None`＝
    /// 印に触れない）。判定は降ろした後。
    mark: Option<MarkInputs>,
    /// 終了コードを 1 にするか（最初の出所に失敗の中身がある＝`fault_of`・告知の場面の有無とは独立）。
    fault: bool,
}

/// 起動中の印の判定の材料（`run()` の後に組み、降ろした後に [`settle_session_mark`] へ渡す）。
struct MarkInputs {
    /// 記憶の置き場（App スコープ）。
    app_profile_dir: std::path::PathBuf,
    /// 起動の文脈の argv の旗（要件 12.5）。
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
    let session = world
        .get_non_send_mut::<ghost_session::GhostSlot>()
        .and_then(|mut slot| slot.0.take());
    let first = world
        .get_resource::<app_exit::FirstExit>()
        .map(|first| first.0.clone());
    let fault = first.as_ref().and_then(app_exit::fault_of).cloned();
    let fault_exit = fault.is_some();
    if world.contains_resource::<session_end::SessionEnded>() {
        // OS のセッションの終了の中で降ろして印も始末した（要件 12.11）。告知は OS の終了を塞ぐので出さない。
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
            ctx.root.ghost_dir(boot_resolve::DEFAULT_GHOST_FOLDER),
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
    let scene = fault.map(|fault| alert::AlertScene::ShioriFault {
        ghost_name,
        // argv で相対パスを渡されても告知には絶対パスを載せる（要件 1.3）。
        ghost_root: std::path::absolute(&ghost_root).unwrap_or(ghost_root),
        fault,
    });
    AfterRun {
        session,
        scene,
        mark: Some(mark),
        fault: fault_exit,
    }
}

/// きれいな終わりの判定（要件 12.2・12.3・12.5）の結論。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkVerdict {
    /// きれいに終わった＝印を消す。
    Clear,
    /// きれいに終わらなかった＝印を残す（理由は記録の `reason`）。
    Keep(&'static str),
    /// argv で始まったプロセス＝印に触れない。
    Untouched,
}

/// きれいな終わりの判定（後始末と OS のセッションの終了が共有する唯一の判定・要件 12.2・12.3）。
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
        | ExitOrigin::SessionEnd => {}
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
/// `info!(session_mark_kept)`、argv なら `debug!` だけ。呼ぶのはゴーストの実行系が 0 の時点だけ。
fn settle_session_mark(mark: &MarkInputs, end: Teardown) -> MarkVerdict {
    let verdict = session_mark_verdict(
        mark.first.as_ref(),
        mark.argv_session,
        mark.logsink_fallback,
        end,
    );
    match verdict {
        MarkVerdict::Clear => boot_resolve::clear_session_mark(&mark.app_profile_dir),
        MarkVerdict::Keep(reason) => tracing::info!(
            event = "session_mark_kept",
            reason,
            first = ?mark.first,
            "[main] きれいに終わらなかったので起動中の印を残します（次の起動は既定のゴーストで Ref6/7 付き）"
        ),
        MarkVerdict::Untouched => tracing::debug!(
            event = "session_mark_untouched_argv",
            "[main] argv で始まったプロセスなので起動中の印に触れません"
        ),
    }
    verdict
}

// ---------------------------------------------------------------------------
// Startup Window
// ---------------------------------------------------------------------------

/// 復元マージのシーム抽出（task 6.1・design C4・要件 1.4/1.5/5.1/6.1）。
///
/// `ghost_session::open_ghost_windows` が `spawn_ghost_windows` へ渡す placements を、起動時に
/// 先読みした永続 entries で差し替える（保存位置優先・毎起動 live 再射影）。
/// `ghost_session::open_ghost_windows` は実モニタの列挙と作業プール（`WintfTaskPool`）を要して
/// テスト困難ゆえ、純粋シーム（load→apply）を本ヘルパへ抽出し単体で
/// 檻に入れる（IO は [`placement::persist::load_restored_state`] の 1 点のみ・merge は純関数）。
///
/// `default_encoding` は boot 結線・`source.rs` と同一の [`areka_parsers::charset::DefaultEncoding`]
/// を渡すこと（mount 解決の一貫性のため）。呼び出し側は `DefaultEncoding::Ansi` を渡す。
fn restore_merged_placements(
    ghost_root: &std::path::Path,
    placements: Vec<placement::resolver::ScopePlacement>,
    snapshot: &placement::follow::MonitorSnapshot,
    default_encoding: areka_parsers::charset::DefaultEncoding,
) -> (
    Vec<placement::resolver::ScopePlacement>,
    std::collections::BTreeSet<usize>,
) {
    // 唯一の IO 点（design C1・A1 シーム）: mount 解決 → Ghost スコープ永続 entries 先読み。
    let entries = placement::persist::load_restored_state(ghost_root, default_encoding);
    // resolver 既定のキャラ位置を merge 前に控える（scg 7.3 の「既定配置か否か」判定の基準）。
    let defaults: Vec<(usize, placement::resolver::PointPx)> =
        placements.iter().map(|p| (p.scope, p.char_pos)).collect();
    // 純関数 merge（永続不書込・保存位置優先 → project_restore → balloon 導出）。
    let merged = placement::persist::apply_restored_placements(placements, &entries, snapshot);
    // 起動時関門（areka-P0-windowposition-limit design C6・要件 2.2/4.7/4.9/5.5/6.1）:
    // 経路①（spawn 初期値）と経路②（復元 merge）はどちらもこの合流点の出力を消費するため、
    // ここ 1 点で両方が被覆される。merge **後**に置くのは保存値優先の合流規則（4.7）を
    // 一切変えないためであり、補正は `balloon_pos`（表示位置）だけに作用する
    // ——`balloon_offset`（論理相対位置）は生値のまま（DD6・補正を焼き付けない）。
    let merged = placement::balloon_limit::apply_balloon_limit(merged, snapshot);
    // 保存位置が採用された（＝resolver 既定から動いた）スコープ集合。これらは**利用者の意思に
    // よる配置**であって既定配置ではないため、連鎖の再解決から常に除外される（scg 7.3）。
    // 保存値がたまたま既定と同値だった場合は差が出ないが、その位置は既定そのものゆえ
    // 既定配置として扱って差し支えない。
    let restored: std::collections::BTreeSet<usize> = merged
        .iter()
        .zip(defaults.iter())
        .filter(|(m, (_, d))| m.char_pos != *d)
        .map(|(m, _)| m.scope)
        .collect();
    (merged, restored)
}

/// `PersistWiring`（NonSend）を、ゴースト窓を保持する同一 World へ挿入するシーム抽出
/// （task 6.2・design C4/C5・要件 1.9）。
///
/// wired 経路（実 sink 結線成立）と fallback boot 経路（`LogSink`×2）の**両方**が、生きた
/// ghost runtime が存在するときにこのヘルパで `runtime.sylphya_publisher().clone()` を World の
/// NonSend リソースとして挿入する（`MouseWiring`／`Emo2Wiring` の NonSend 先例に倣う）。以降、
/// follow.rs の DragEnd 観測点が [`placement::persist::persist_entries`] 経由でこの publisher の
/// clone 送信端から保存 entries を write-through 投函できる（World レベルの配線導管）。
///
/// 生きた runtime が無い経路（wired の `None` ghost・fallback の `Err`）
/// では挿入しない＝従来どおり永続結線なし（`persist_entries` は `PersistWiring` 不在で debug!＋
/// no-op へ縮退・6.2）。挿入は純粋な World 変異ゆえ headless 単体テスト可能（`insert_non_send`
/// を薄く包み `#[cfg(test)]` の檻に入れる）。
fn insert_persist_wiring(world: &mut World, publisher: areka_sylphya::SylphyaPublisher) {
    world.insert_non_send(placement::persist::PersistWiring { publisher });
}

/// boot が `Ok` を返した直後の結線（wired／fallback の両アームが呼ぶ 1 か所）。
///
/// 位置永続の導管を挿入し（[`insert_persist_wiring`]）、続けて起動成功時の記憶を書く
/// （[`record_last_used_at_boot`]・baseware-root-layout 要件 3.2〜3.5）。
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

/// 起動の呼び出しが返った時点の記憶（初回の起動・既定への戻しは今日どおりここで書く）。
///
/// 経路が切替なら書かない（要件 4.6・12.6）: 壊れた切替先の名前を最後のゴーストに一瞬も載せない
/// ため、切替先の記憶は定常到達で `ghost_switch::record_steady_memory` が同じ [`record_last_used`] で書く。
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

/// 最後に使ったもの（ゴースト・バルーン・シェル）を、動いているゴーストの記憶の書き手へ投函する
/// （baseware-root-layout 要件 3.2〜3.5）。書き込みは投函だけで待たない（反映は `shutdown` の
/// barrier に任せる＝design R1）。シェルのフォルダ名は `shell_dir`（`mount().shell.dir`）の末尾
/// （非 UTF-8 は `to_string_lossy` で写す）。呼び手は起動の直後と切替の定常到達
/// （`ghost_switch::record_steady_memory`）。
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

/// 起動時モニタスナップショットの構築＋出力シーム（areka-P0-dpi-window-vanish task 1.2・
/// 要件 1.1・design D12「areka 構築点を正典」）。
///
/// [`placement::follow::MonitorSnapshot`] は **placement の全判断が読む権威**（work area
/// 解決・アンカー射影・可視性判定がすべてこの Resource を引く）である。したがって
/// 要件 1.1 の正典出力点はこの構築点——ここで観測した値だけが「以後の判断が実際に見た値」
/// であり、他所で列挙し直した値ではない（D12）。
///
/// 出力は共有ヘルパ [`placement::diag::log_monitor_snapshot`] 1 本で、呼出点タグ
/// [`placement::MONITOR_SNAPSHOT_CONTEXT`] を名乗る。`prepare_ghost_windows` の列挙点も
/// 同じヘルパを別タグで呼ぶため、語彙は共有したままログ上で出所を弁別でき、
/// 両者の食い違いは grep 突合で検出できる（D12: 専用の突合機構は新設しない）。
///
/// 観測を足すだけで snapshot の中身は一切変えない（D2: 観測増設は Req 2.7 の
/// 「変更」に数えない）。実モニタを列挙する `ghost_session::open_ghost_windows` から切り出した
/// シームゆえ、合成モニタで headless 檻に入る。
///
/// # 2 源を同じ構築関数で作る（atom task 5.1・要件 5.1）
///
/// 返すのは作業領域源と**モニタ別拡大率表**の組である。実行時の同期段
/// （`emo2_boot::frame::work_area_sync`）も同じ [`placement::follow::MonitorSources::from_monitors`]
/// を通る——起動時だけが別の作り方をすると、同期が入った後も起動時の値だけが違う形になり得る。
fn boot_monitor_snapshot(
    monitors: &[wintf::ecs::window::monitor::Monitor],
) -> placement::follow::MonitorSources {
    placement::diag::log_monitor_snapshot(
        &placement::monitor_records(monitors),
        placement::MONITOR_SNAPSHOT_CONTEXT,
    );
    placement::follow::MonitorSources::from_monitors(monitors)
}

/// 自動 close ゲートを有効化する環境変数名（`AREKA_` 冠規約・記憶 areka-runtime-env-naming）。
const SMOKE_EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";

/// 与えられた値から自動 close の遅延ミリ秒を解釈する純粋ヘルパ（env アクセスなし・単体テスト可能）。
///
/// - `None`／空／空白のみ／非数値／負値／`u64` 溢れ → `None`（ゲート OFF＝タスク不投入）。
/// - 周辺空白をトリムした非負整数 → `Some(ms)`（`"0"` は 0ms＝即時発火として受理）。
///
/// `u64::from_str` は負号・小数点・非数字を弾き、範囲外を `Err` にするため、負値・溢れは
/// 自然に `None` へ落ちる（不正入力はゲート OFF に倒す）。
fn smoke_exit_ms_from(value: Option<&str>) -> Option<u64> {
    let trimmed = value.map(str::trim)?;
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// 環境変数 [`SMOKE_EXIT_ENV`] から自動 close の遅延ミリ秒を読む。
fn smoke_exit_ms() -> Option<u64> {
    smoke_exit_ms_from(std::env::var(SMOKE_EXIT_ENV).ok().as_deref())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// smoke の自動終了ゲート（task 2.3）の headless 単体テスト。
///
/// ランタイム結線（`fn main` の自動終了の仕掛け）は生きた `WinApp` を要し headless では駆動できないため、
/// 判断を持つ `smoke_exit_ms_from` だけを純粋・env 非依存で検証する（結線は実プロセス smoke が担う）。
#[cfg(test)]
#[path = "main_startup_window_tests.rs"]
mod startup_window_tests;

#[cfg(test)]
#[path = "main_config_input_tests.rs"]
mod config_input_tests;

/// ghost 結線ヘルパ（task 3.3）の headless 単体テスト。
///
/// `areka_ghost::boot`／`GhostRuntime::shutdown` 自体は実 I/O・実スレッドを伴うため、
/// ここでは純粋な組み立て・分類ロジック（`default_helper_exe_path`／`ghost_boot_options`／
/// `is_benign_boot_error`）だけを headless に検証する。実際の boot→shutdown 一巡は
/// 既存の実プロセス smoke テスト（`tests/smoke_boot_loop_exit.rs`）が証明する。
#[cfg(test)]
#[path = "main_ghost_wiring_tests.rs"]
mod ghost_wiring_tests;

/// 復元マージシーム（task 6.1・design C4・要件 1.4/1.5）の headless 単体テスト。
///
/// `ghost_session::open_ghost_windows` は実モニタの列挙と作業プールを要するためテスト困難ゆえ、
/// それが `spawn_ghost_windows` へ渡す placements を作る純粋シーム
/// （`restore_merged_placements`＝`load_restored_state`→`apply_restored_placements`）を
/// 抽出して檻に入れる。植えた sylphya.toml の保存位置が既定位置に優先して merge 済み
/// placements の char_pos へ載ること（1.4）／永続不在なら既定 placement に恒等（1.5）を
/// 証明する（＝spawn される窓の初期位置が保存位置になる結線の証明）。
#[cfg(test)]
#[path = "main_restore_seam_tests.rs"]
mod restore_seam_tests;

/// `PersistWiring` 挿入シーム（task 6.2・design C4/C5・要件 1.9）の headless 単体テスト。
///
/// wired／fallback 両経路が使う挿入ヘルパ `insert_persist_wiring` を檻に入れる。
/// シーム結線そのもの（`ghost_session::boot_ghost` の boot 経路分岐）は実 boot を伴うため、TDD は
/// headless で駆動可能な挿入ヘルパで回す。実 publisher（`spawn_sylphya`＋共有 fake IO）を
/// headless World へ挿入し、(a) NonSend リソース `PersistWiring` が存在すること、(b) その
/// World 越しの `persist_entries` 投函が barrier 後に別ハンドルの `load_scope` で読み戻せる
/// （＝World レベルの配線導管が正しく確立され DragEnd→file の World シームが成立している）
/// ことを証明する。DragEnd→file の完全な end-to-end は task 8.2 が担う。
#[cfg(test)]
#[path = "main_persist_wiring_seam_tests.rs"]
mod persist_wiring_seam_tests;

/// 起動時モニタスナップショット出力シーム（areka-P0-dpi-window-vanish task 1.2・
/// 要件 1.1・design D12「areka 構築点を正典」）の headless 単体テスト。
///
/// `ghost_session::open_ghost_windows` は実モニタを列挙してテスト困難ゆえ、それが
/// `MonitorSnapshot` を組む点＝**placement の全判断が読む権威の構築点**を
/// [`boot_monitor_snapshot`] へ抽出し、合成モニタ（混在 DPI・負座標・3200 超）で檻に入れる。
/// 実モニタも実 GPU も要さない。
#[cfg(test)]
#[path = "main_monitor_snapshot_seam_tests.rs"]
mod monitor_snapshot_seam_tests;

/// `finish_after_run`（task 4.1・要件 3.1・3.2・6.3）の単体テスト。
#[cfg(test)]
#[path = "main_finish_after_run_tests.rs"]
mod finish_after_run_tests;

/// 据え付けと `run()` の後・起動中の印（task 8.5・11.1・要件 8.1・8.2・12.1〜12.5・12.12）のテスト。
#[cfg(test)]
#[path = "main_session_mark_tests.rs"]
mod session_mark_tests;
