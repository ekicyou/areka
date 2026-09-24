//! 起こし直しの単位（areka-P0-ghost-restart-unit）。
//!
//! ゴーストを 1 体起こすのに要る手順を、**プロセスに 1 回**のもの（系の登録）と
//! **ゴーストごと**のもの（状態の載せ替え）に分けて置く場所である。系はどれも状態
//! （`NonSend`／`Resource`）が無ければ無操作で戻る自己防御を持つので、登録を先に 1 度だけ
//! 済ませ、状態は後から入れ替えても系は新しい状態を見る。
//!
//! 今ここに在るのは登録の入口 [`register_systems`] と、窓を作る側 [`open_ghost_windows`]・
//! 閉じた証を受けて窓を作り直す [`reopen_ghost_windows`]、ゴーストごとの結線 [`boot_ghost`] と
//! その 3 ハンドルを降ろす [`GhostSession::shutdown`] である。登録の呼び手は `fn main` の
//! 1 か所（`WinApp` を作った直後・起動窓の前）。各結線（`wire_*`）は状態の挿入だけを行い、
//! 系を登録しない。

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_kanade::KanadeStopped;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use wintf::ecs::FrameFinalize;
use wintf::ecs::widget::bitmap_source::{CommandSender, WintfTaskPool};

use crate::app_exit::{self, WindowsClosed};
use crate::boot_resolve::{BalloonDecision, GhostDecision};
use crate::emo2_boot;
use crate::input_events;
use crate::menu;
use crate::placement;
use crate::readme;
use crate::{ConfigInputs, default_app_profile_dir, ghost_boot_options, is_benign_boot_error};

/// 系の登録をプロセスに 1 回・1 か所から行う（要件 2.1・2.2・3.3）。
///
/// 載せる系と順序（段ごとに今日の挿入順を保つ）:
/// - `Update` ← 毎フレームの相 → 停止通知の受け口（受け口 `KanadeStopRx` はプロセスに 1 つ
///   なので [`emo2_boot::wire_kanade_stop`] を分割せずそのまま呼ぶ）
/// - `Input` ← 説明書 → 中断 → メニュー → バルーンの離脱 → 選択肢の送り
/// - `FrameFinalize` ← クリック透過 → OS の閉鎖要求 → 重なり順の対（状態がゴーストごとで
///   ないので `wire_zorder_pair` をそのまま呼ぶ）
///
/// 各系の並び（`before`／`after`・`chain`）は各登録関数が持つ。ここは順に呼ぶだけ。
///
/// 今日との差: `Input` の 5 系と `Update` の毎フレームの相（`emo2_frame_system`）は LogSink の
/// 起動でも登録される。どれも状態（`NonSend`）が無ければ無操作で戻る（記録は `trace!` か無し）
/// ので、見え方は変わらない。バルーンの離脱の系だけは `BalloonWiring` 不在で
/// `error!(balloon_wiring_missing)` の枝を持つが、そこへは `PointerLeave` が立ったときにしか
/// 進まず、LogSink の起動のバルーン窓は `HitTest::none()` のまま（当たり判定の面は結線ありの
/// 起動でだけ装着する）なので `PointerLeave` が立たず届かない。
pub(crate) fn register_systems(world: &mut World, kanade_stop_rx: Receiver<KanadeStopped>) {
    emo2_boot::register_emo2_frame_system(world);
    emo2_boot::wire_kanade_stop(world, kanade_stop_rx);

    readme::register_readme_drain(world);
    input_events::user_break::register_user_break_drain(world);
    menu::register_menu_poll(world);
    input_events::balloon::register_balloon_leave_system(world);
    input_events::choice_drain::register_choice_drain(world);

    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        placement::spawn::register_ghost_windows_click_through,
    );
    world
        .resource_mut::<Schedules>()
        .add_systems(FrameFinalize, app_exit::attach_os_close_request);
    placement::spawn::wire_zorder_pair(world);
}

/// 起動窓の準備が descript から**1 度だけ**読み取った値のうち、呼び手（`main`）が下流へ
/// 配るもの（areka-P0-emo-dpi-scaling task 4.3・areka-P0-scope-zorder-pinning 要件 5.1／5.2）。
///
/// どちらも `wire_emo2_boot` へ渡る搬送値であり、[`open_ghost_windows`] 自身は解釈しない。
/// 戻り口を 2 つに増やさず 1 つの型へまとめるのは、「同じ 1 度の読取から来た」という出所を
/// 型で示すためである——別々に読み直す余地を残すと、配置と重なりが違う宣言を見る日が来る。
pub(crate) struct StartupDescriptValues {
    /// 採寸 k₀ と attach（`attach_target`）が共有する作者基準 DPI。
    pub(crate) author_dpi: placement::AuthorDpi,
    /// shell descript の `seriko.zorder` の生の値（未指定なら `None`）。解釈は台帳の層が行う。
    pub(crate) zorder_raw: Option<String>,
}

/// 窓を作る側が失敗する理由（design「open_ghost_windows / reopen_ghost_windows」）。
#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenWindowsError {
    /// 配置の準備が通らない（モニタ 0 台・起動中の削除等）。
    #[error(transparent)]
    Placement(#[from] placement::PlacementError),
    /// 窓を積む作業プールが World に無い（本番では `EcsWorld::new` が必ず挿す＝配線の誤り）。
    #[error("窓を作るための作業プールが World に無い")]
    TaskPoolMissing,
}

/// 窓を作る側（要件 3.3・3.4・window-placement 1.4）: 配置の準備 → 監視の 2 源 → 復元 →
/// 窓の生成と受け口の装着を作業プールへ積む。
///
/// - 最初に作業プール（[`WintfTaskPool`]）の有無を確かめる。無ければ配置の準備に入らず
///   `error!(event = "task_pool_missing")` の上で [`OpenWindowsError::TaskPoolMissing`]
///   （`EcsWorld::spawn` のように黙って何もしない形は取らない）。
/// - 成功時: 既存 ECS コマンド経路（作業プールの async タスク → `CommandSender` → Input
///   スケジュールで World 適用）で窓を組み立てる（`FrameFinalize` への系の結線は
///   [`register_systems`] が先に済ませる）。smoke の自動終了はここでは仕掛けない（呼び手の
///   `fn main` が 1 度目の成功の直後に 1 度だけ仕掛ける・要件 3.5）。
/// - 失敗時（モニタ 0 台・起動中の削除等）: [`placement::PlacementError`] を包んで返す。
///   呼び手（`main`）が「起動窓を開けない」を告知して終了コード 1 で終える
///   （baseware-root-layout 要件 6.4・窓を開かずに居座らない＝要件 6.6）。
///
/// 準備（`prepare_ghost_windows`）は同期実行し、I/O はここで完結・Send な値のみを ECS
/// コマンドへ運ぶ。呼び出しスレッドは `WinApp` 構築済みの MTA UI スレッド＝COM
/// 初期化済み（measure の WIC 前提を満たす）。
///
/// 戻り値は準備が descript から**1 度だけ**読んだ値の組（[`StartupDescriptValues`]）。
/// 作者基準 DPI は (a) 採寸の k₀ と (b) 呼び手（`main`）経由で
/// `wire_emo2_boot`→`attach_target` の双方へ配られ、shell の `seriko.zorder` は同じ経路で
/// 重なりの台帳へ配られる（design Flow 3 手順1「1 度だけ読む」）。
pub(crate) fn open_ghost_windows(
    world: &mut World,
    cfg: &ConfigInputs,
) -> Result<StartupDescriptValues, OpenWindowsError> {
    let Some(task_pool) = world.get_resource::<WintfTaskPool>() else {
        tracing::error!(
            event = "task_pool_missing",
            ghost_root = %cfg.ghost_root.display(),
            "[open_ghost_windows] 作業プールが World に無いので窓を作れません（配置の準備に入らず失敗を返す）"
        );
        return Err(OpenWindowsError::TaskPoolMissing);
    };
    let prepared = placement::prepare_ghost_windows(&cfg.ghost_root, &cfg.balloon_root)?;
    // モニタ 2 源（task 8.1・atom task 5.1）: 起動時の実モニタから忠実転写した
    // 作業領域源とモニタ別拡大率表（物理 px・Send な純粋データ）。bottom 吸着ドラッグ
    // （4.7・task 8.2）と拡大率の相が消費する。
    // **セッション内固定ではない**（DD15 撤回・atom 要件 5.1）——毎フレーム先頭の
    // 同期段（`emo2_boot::frame::work_area_sync`）が実行時のモニタ表から作り直す。
    // ここが作るのは起動時の初期値であり、構築関数は同期段と同一である。
    // 構築と同時に全モニタの観測を 1 回出す（areka-P0-dpi-window-vanish 要件 1.1 の
    // **正典出力点**・D12）。既定 OFF・診断 `RUST_LOG` でのみ点灯する。
    let sources = crate::boot_monitor_snapshot(&wintf::ecs::window::monitor::enumerate_monitors());

    // clickthrough 登録・OS の閉鎖要求の受け手・重なり順の対の `FrameFinalize` への結線は
    // `register_systems` が済ませてある（どれも `Added<WindowHandle>` 起点ゆえ、
    // 窓 spawn より先に結線しても取りこぼさない）。

    // 復元マージ（design C4・要件 1.4）: snapshot 構築直後・spawn closure へ渡す前に、
    // 永続先読み（load_restored_state）→ 純関数 merge（apply_restored_placements）で
    // 保存位置を反映した placements を得る。`prepared` を placements/titles へ分解し、
    // merge 済み placements（value 渡し）と titles を closure へ move する
    // （default_encoding は boot 結線・source.rs と同一の Ansi＝mount 解決の一貫性）。
    // 作者基準 DPI は `prepared` 分解の前に取り出して呼び手へ返す（`Copy` 値の転記）。
    let author_dpi = prepared.author_dpi;
    // 重なりの基底の生の値も同じ読取から取り出す（解釈は結線の先＝台帳の層が行う・
    // areka-P0-scope-zorder-pinning 要件 5.2）。`prepared` を分解する前に写す。
    let zorder_raw = prepared.zorder_raw.clone();
    // 復元は**起動時の作業領域源を 1 度だけ**読む（atom 要件 5.7）。以後の同期段は
    // この判定へ効かせない——拡大率をまたぐ保存位置の追従は行わない裁定
    // （`windowposition-limit` の開発者裁定）を踏襲する。
    let (placements, restored_scopes) = crate::restore_merged_placements(
        &cfg.ghost_root,
        prepared.placements,
        &sources.snapshot,
        areka_parsers::charset::DefaultEncoding::Ansi,
    );
    let titles = prepared.titles;

    // 作業プールの async タスク → CommandSender → Input スケジュールで
    // World 適用という既存 ECS コマンド経路で本物窓を組み立てる。
    task_pool.spawn(|tx: CommandSender| async move {
        let _ = tx.send(Box::new(move |world: &mut World| {
            // 2 源は同時に挿す（片方だけ古い運転を作らない・atom C6）。
            world.insert_resource(sources.snapshot);
            world.insert_resource(sources.dpi_table);
            let windows = placement::spawn::spawn_ghost_windows(world, &placements, &titles);
            // 保存位置が復元されたスコープは既定配置ではない（scg 7.3）。台帳の
            // 既定位置を落として連鎖の再解決から常に除外する——さもないと次回起動で
            // 利用者のドラッグ位置が隣接位置へ引き戻される。spawn が Resource として
            // 挿した実体を直接標す（戻り値の clone を触っても Resource へは効かない）。
            if !restored_scopes.is_empty()
                && let Some(mut gw) = world.get_resource_mut::<placement::spawn::GhostWindows>()
            {
                for scope in &restored_scopes {
                    gw.clear_default_char_pos(*scope);
                }
            }
            // マウス入力ハンドラ装着（areka-P0-input-events・依存方向 input_events→
            // placement）: placement は `crate::` パスを持てない（example の `#[path]`
            // include で成立させるため）ゆえ、キャラ窓へのポインタハンドラ結線は
            // input_events 側が担う。spawn 直後の同一 World-mutation クロージャ内で
            // 同期実行するため、キャラ窓は既に存在し async race はない。
            input_events::attach_char_pointer_handlers(world);
            // 右クリックメニューの解放ハンドラも同じ場所で付ける。このクロージャが動くのは
            // `app.run()` の中＝`menu::wire_menu` より後で、結線の無い起動では解放を無視するだけ。
            menu::attach_release_handlers(world);
            // バルーン窓へポインタハンドラを装着（task 6.2・`attach_char_pointer_handlers`
            // 直後・R4.3/5.5）: `BalloonWindowMarker` 窓へ `OnPointerMoved`／`OnPointerPressed`
            // を post-spawn 挿入する（標的はバルーン窓のみ＝キャラ窓配線の非退行・R4.3）。同一
            // `&mut World` クロージャ内で同期実行するためバルーン窓は既に存在し async race は
            // ない（キャラ窓ハンドラ装着と同型のタイミング契約）。
            input_events::balloon::attach_balloon_pointer_handlers(world);
            let scopes: Vec<usize> = windows.scopes().collect();
            tracing::info!(
                ?scopes,
                "本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓）"
            );
        }));
    });

    Ok(StartupDescriptValues {
        author_dpi,
        zorder_raw,
    })
}

/// 閉じた証を受けて窓を作り直す（要件 3.4・4.4）。証（[`WindowsClosed`]）の唯一の消費先で、
/// 手順は 1 度目と同じ [`open_ghost_windows`] へ委譲する。
// 本番の呼び手は #13（ゴーストの切替）。それまでは test からだけ呼ぶ。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn reopen_ghost_windows(
    world: &mut World,
    cfg: &ConfigInputs,
    closed: WindowsClosed,
) -> Result<StartupDescriptValues, OpenWindowsError> {
    tracing::debug!(
        closed = closed.closed(),
        "[reopen_ghost_windows] 閉じた窓の後に窓を作り直す"
    );
    open_ghost_windows(world, cfg)
}

/// ゴーストごとの結線の入力の束（design「boot_ghost」）。
pub(crate) struct GhostBootInputs {
    /// 起動の結線の入力（ゴーストの根・バルーンの根・SHIORI の結線・時計の種類・記憶の置き場）。
    pub wiring: emo2_boot::Emo2BootInputs,
    /// fallback（`LogSink`）の起動が使う 32bit SHIORI helper のパス。
    pub helper_exe: PathBuf,
    /// 停止通知の送出端の写し（結線あり・fallback の両方の起動へ渡る・#55）。
    pub kanade_stop: Sender<KanadeStopped>,
}

impl GhostBootInputs {
    /// 本番の値: SHIORI は `Helper { helper_exe }`・時計は `Real` 既定・記憶の置き場は
    /// `Some(default_app_profile_dir())`・根は構成入力のもの。
    pub(crate) fn production(
        cfg: &ConfigInputs,
        helper_exe: PathBuf,
        kanade_stop: Sender<KanadeStopped>,
    ) -> Self {
        Self {
            wiring: emo2_boot::Emo2BootInputs {
                ghost_root: cfg.ghost_root.clone(),
                balloon_root: cfg.balloon_root.clone(),
                shiori: areka_ghost::ShioriWiring::Helper {
                    helper_exe: helper_exe.clone(),
                },
                ticker: areka_ghost::TickerMode::Real(Default::default()),
                app_profile_dir: Some(default_app_profile_dir()),
            },
            helper_exe,
            kanade_stop,
        }
    }
}

/// 1 体のゴーストの 3 ハンドル（ゴースト実行系・seriko・loop ticker）。
///
/// **落としても何も起きない。降ろすのは必ず [`GhostSession::shutdown`] を呼ぶこと**
/// （`Drop` は終了の理由を持てないので実装しない・要件 1.7）。
pub(crate) struct GhostSession {
    ghost: Option<areka_ghost::GhostRuntime>,
    seriko: Option<areka_actor::ActorHandle>,
    loop_ticker: Option<mpsc::Sender<areka_ghost::ticker::TickerMsg>>,
}

impl GhostSession {
    /// 告知の場面が使うゴースト名（fallback の起動に失敗して実行系が無ければ `None`）。
    pub(crate) fn ghost_name(&self) -> Option<String> {
        self.ghost
            .as_ref()
            .and_then(|r| r.mount().names.name.clone())
    }

    /// 降ろす（要件 1.1・1.3・1.5・1.6）: ① loop ticker の停止 → ② ゴースト実行系の終了
    /// （`reason` を渡す）→ ③ seriko の join。無い段は飛ばし、②③ の失敗は `error!` の上で
    /// `Err` を返して以降を飛ばす。perf の最終報告はプロセスに 1 回なので呼び手が後で行う。
    pub(crate) fn shutdown(self, reason: areka_kanade::CloseReason) -> windows::core::Result<()> {
        // ① loop ticker Close（本ブロック）: SERIKO ループ ticker の worker スレッドは closure 内へ
        // `SerikoSink` クローン（tick_sink）を握る。これを先に停止させないと seriko inbox が ticker 経由で
        // 生き続け、③ の join が「全 Sender drop」を永遠に待って hang する。停止端 Sender へ
        // `TickerMsg::Close` を送ると worker は `recv_timeout` から `Ok(Close)` で return し、その closure＝
        // tick_sink が drop される（inbox 切断の片翼が外れる。残る片翼＝ghost 側 SerikoSink は ② が外す）。
        // 呼び手は ticker の JoinHandle を持たない（`wire_emo2_boot` が保持せず drop 済み）ため直接 join でき
        // ないが、③ の seriko join が全 Sender drop まで block するため実質 ticker worker の終端を待つ形に
        // なり hang しない（Close 未達で worker が既に終端していても drop で disconnected 経路へ倒れる）。
        // 失敗（既に終端済み）は shutdown 期待事象ゆえ `debug!`（silent failure 禁止・非致命・R7.5）。
        if let Some(ticker) = self.loop_ticker {
            match ticker.send(areka_ghost::ticker::TickerMsg::Close) {
                Ok(()) => tracing::info!(
                    "seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止）"
                ),
                Err(_) => tracing::debug!(
                    "seriko: loop ticker は既に終端済み（Close 送信先なし・shutdown 期待事象）"
                ),
            }
            // 送信の成否に依らず停止端 Sender をここで drop し、確実に制御チャンネルを disconnected にする。
            drop(ticker);
        }

        // ② 終了握手（task 5.2・design「終了握手（R6）」・DD-10）: boot 済み（`Some`）のときのみ
        // `shutdown` を呼ぶ。終了理由は呼び手が渡す（`fn main` は `CloseReason::User { scope: 0 }`＝
        // 全窓 close funnel はユーザ操作起点）。OnClose 応答の再生完了待ちは kanade の `ForceQuit`
        // 終了系列内で処理される（本仕様は `shutdown` を呼ぶだけ・不改変・R6.2）。失敗は `error!` の上で
        // 呼び手へ `Err` を返す（genuine な失敗を黙って exit 0 にしない・R6.3）。
        if let Some(runtime) = self.ghost {
            if let Err(err) = runtime.shutdown(reason) {
                tracing::error!(error = %err, "ghost 結線層の終了統括に失敗しました");
                return Err(windows::core::Error::from_hresult(
                    windows::Win32::Foundation::E_FAIL,
                ));
            }
        }

        // ③ seriko アクターの join（design「終了握手（R6）」・R6.3）。seriko inbox への送信端は 2 本ある:
        // (a) ghost 側の `SerikoSink`（surface_sink・②の `shutdown` が drop）と (b) loop ticker closure の
        // `tick_sink`（①の Close→worker return で drop）。①②で両端が drop されて inbox が切断され、seriko
        // worker は自然終了する。呼び手は自前の `SerikoSink` クローンを保持しない（sink は `wire_emo2_boot`
        // が boot／ticker へ move 済み）ため、この `join` は両端 drop 完了（＝ticker worker 終端）まで block
        // したうえで速やかに戻る（①で ticker を先に Close したことが hang 回避の要）。join 失敗（worker
        // panic）は握り潰さず `error!`＋`Err` 伝播する（genuine な失敗を隠さない）。
        if let Some(seriko) = self.seriko {
            if let Err(err) = seriko.join() {
                tracing::error!(error = %err, "seriko アクターの join に失敗しました");
                return Err(windows::core::Error::from_hresult(
                    windows::Win32::Foundation::E_FAIL,
                ));
            }
        }

        Ok(())
    }
}

/// ゴーストごとの結線と状態の載せ替え（要件 2.3・2.4・2.6・design「boot_ghost」）。
///
/// 起動の結線（`wire_emo2_boot`）が成立すれば実 sink の腕（入力・メニュー・記憶・バルーンの
/// 選択・選択肢の送り）を、成立しなければ `LogSink`×2 の fallback の腕を通す。各状態は
/// `insert_non_send` で置き換わるので n 回呼べる（呼び手は先に [`GhostSession::shutdown`] で
/// 前のゴーストを降ろしておくこと）。証は取らない（証は [`reopen_ghost_windows`] が消費する）。
pub(crate) fn boot_ghost(
    world: &mut World,
    inputs: GhostBootInputs,
    descript: &StartupDescriptValues,
    ghost: &GhostDecision,
    balloon: &BalloonDecision,
) -> GhostSession {
    let GhostBootInputs {
        wiring,
        helper_exe,
        kanade_stop,
    } = inputs;
    // fallback の腕が使う根（結線の入力は `wire_emo2_boot` へ move するので先に写す）。
    let ghost_root = wiring.ghost_root.clone();

    // emo2 統合結線（task 5.2・design「エントリポイント / main.rs＋wire_emo2_boot」・DD-7）:
    // UI 基盤・起動窓の後で完成済み 5 トラック（seriko／sakura／emo-present／emo-text／actor）を
    // 束ねる実 sink 結線を試みる。`wired=true` なら実 sink boot が成立し、ghost／seriko ハンドルを
    // 終了処理へ運ぶ。`wired=false`（asset 組立失敗・boot 失敗等）は現行の `LogSink`×2 フォール
    // バック boot へ倒し、既存 smoke 前提・非致命 boot 意味論を温存する（R7.1/7.3・DD-7）。
    let outcome = emo2_boot::wire_emo2_boot(
        world,
        wiring,
        descript.author_dpi,
        descript.zorder_raw.as_deref(),
        kanade_stop.clone(),
    );
    if outcome.wired {
        tracing::info!(
            "実 sink 結線で起動しました（emo2-boot wire 成立・SERIKO ループ ticker 稼働）"
        );
        // マウス配信資源を World へ結線（task 3.1・design「main.rs＋wire_mouse_input」・
        // DD-IE-9）: kanade Sender クローンで MouseWiring（NonSend・Presenter）を挿入する。
        // 挿入は wire_emo2_boot 成功後＝Emo2Wiring 挿入済みゆえ presenter 経由の region 解決が
        // 成立する（Emo2Wiring 挿入と同位置・同型・self-gating）。窓へのハンドラ登録は task 3.2。
        if let Some(runtime) = outcome.ghost.as_ref() {
            let sender = runtime.kanade().clone();
            input_events::wire_mouse_input(world, sender);
            // 右クリックメニューの結線（areka-P0-popup-menu-minimal）: 終了の項目が上の入力の結線を使う。
            menu::wire_menu(world, runtime.kanade().clone());
            // 位置永続の World 結線（task 6.2・design C4/C5・要件 1.9）: wire_mouse_input とは
            // 別行の additive 挿入。ゴースト窓を保持する同一 World（`wire_mouse_input` と同経路）へ
            // sylphya publisher clone を持つ PersistWiring（NonSend）を差し、DragEnd→persist_entries の
            // write-through 導管を確立する。続けて起動成功時の記憶を書く（baseware-root-layout 要件 3）。
            crate::on_boot_ok(world, runtime, ghost, balloon);
        }
        // バルーン選択肢対話配線を World へ結線（task 6.2・design「main.rs＋wire_balloon_choice」・
        // R4.3/5.5/8.1）: mpsc チャネル生成＋`BalloonWiring`／`ChoiceSelectionInbox`（NonSend）挿入。
        // 離脱の系の登録は [`register_systems`] が済ませてある。ハンドラ装着は [`open_ghost_windows`] の
        // spawn 直後（`attach_balloon_pointer_handlers`）が担う。balloon ハンドラは `Emo2Wiring` を
        // self-gate するため wired 経路でのみ意味を持つ（`wire_mouse_input` と同じ gating・DD-IE-9 前例）。
        input_events::balloon::wire_balloon_choice(world);
        // 選択確定通知の受信結線（areka-P0-choice-select-events task 5・design C1 ChoiceDrain・
        // Req1.1/1.2/1.5/1.6/3.7）: 直上 `wire_balloon_choice` が挿入した `ChoiceSelectionInbox`
        // （precondition）を毎フレーム drain し kanade へ全件転送する系（登録は [`register_systems`]）の
        // 送り口を置く。位置・様式は `wire_mouse_input`（上方の同 boot スロット）と同型——kanade Sender
        // クローンを持つ NonSend 資源挿入。
        if let Some(runtime) = outcome.ghost.as_ref() {
            input_events::choice_drain::wire_choice_drain(world, runtime.kanade().clone());
        }
        GhostSession {
            ghost: outcome.ghost,
            seriko: outcome.seriko,
            loop_ticker: outcome.loop_ticker,
        }
    } else {
        // フォールバック（R7.3・DD-7）: 現行の `LogSink`×2 boot を UI 基盤・起動窓の後へ
        // relocate したもの。失敗は非致命——起動前の解決が `ghost/master/descript.txt` の実在を
        // 確かめているので、ここでの `MountError::StartPointMissing` は解決後の消失（起動中の削除等）
        // に限られ、`warn!` の上で `None` として骨格起動を継続する（要件 8.2）。それ以外の予期しない
        // 失敗（読取不能・shell 不在等）は `error!`（`is_benign_boot_error` の分類は不変・R7.4）。
        let ghost_options = ghost_boot_options(ghost_root, helper_exe);
        // 停止通知の送出端つきで起動する（SHIORI の失敗で kanade が止まったら終了相へ届く・要件 6.4）。
        let runtime = match areka_ghost::boot_with_kanade_stop(ghost_options, Some(kanade_stop)) {
            Ok(runtime) => {
                tracing::info!("LogSink フォールバックで起動しました（emo2-boot wire 不成立）");
                // 位置永続の World 結線（task 6.2・design C4/C5・要件 1.9）: fallback boot でも
                // 生きた runtime があれば wired 経路と同型に PersistWiring（NonSend）を同一 World へ
                // 挿入する（両経路で DragEnd→persist_entries の write-through 導管を確立）。
                // 起動成功時の記憶の書き込みも wired 経路と同じ 1 か所で行う（baseware-root-layout 要件 3）。
                crate::on_boot_ok(world, &runtime, ghost, balloon);
                Some(runtime)
            }
            Err(err) => {
                if is_benign_boot_error(&err) {
                    tracing::warn!(
                        error = %err,
                        "ghost 結線層の起動起点が見つかりません（決定のみで継続・骨格起動は阻害しません）"
                    );
                } else {
                    tracing::error!(
                        error = %err,
                        "ghost 結線層の起動に失敗しました（継続・骨格起動は阻害しません）"
                    );
                }
                None
            }
        };
        // フォールバック経路に seriko アクター・loop ticker はない（実 sink 結線が成立していない）。
        GhostSession {
            ghost: runtime,
            seriko: None,
            loop_ticker: None,
        }
    }
}

#[cfg(test)]
#[path = "ghost_session_restart_tests.rs"]
mod restart_tests;
