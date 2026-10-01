//! 起こし直しの単位（areka-P0-ghost-restart-unit）。
//!
//! ゴーストを 1 体起こすのに要る手順を、**プロセスに 1 回**のもの（系の登録）と
//! **ゴーストごと**のもの（状態の載せ替え）に分けて置く場所である。系はどれも状態
//! （`NonSend`／`Resource`）が無ければ無操作で戻る自己防御を持つので、登録を先に 1 度だけ
//! 済ませ、状態は後から入れ替えても系は新しい状態を見る。
//!
//! 今ここに在るのは登録の入口 [`register_systems`] と、窓を作る側 [`open_ghost_windows`]・
//! 閉じた証を受けて窓を作り直す [`reopen_ghost_windows`]、ゴーストごとの結線 [`boot_ghost`]
//! （fallback へ倒れる）／[`boot_ghost_strict`]（倒れず失敗を返す・切替用）と
//! その 3 ハンドルを降ろす [`GhostSession::shutdown`]、起こしたゴーストの置き場 [`GhostSlot`] と
//! 起動入力の作り口 [`GhostBootInputsSource`] である。登録の呼び手は `fn main` の
//! 1 か所（`WinApp` を作った直後・起動窓の前）。各結線（`wire_*`）は状態の挿入だけを行い、
//! 系を登録しない。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use areka_kanade::{BootOrigin, KanadeMsg, KanadeNotice};
use areka_parsers::package::GhostNames;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use wintf::ecs::FrameFinalize;
use wintf::ecs::widget::bitmap_source::{BoxedCommand, CommandSender, WintfTaskPool};

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
/// - `Update` ← 毎フレームの相 → 停止通知の受け口（受け口 `KanadeNoticeRx` はプロセスに 1 つ
///   なので [`emo2_boot::wire_kanade_stop`] を分割せずそのまま呼ぶ）
/// - `Input` ← 説明書 → 台本の切替要求 → 台本のシェル・バルーンの切替要求 → 中断 → メニュー →
///   バルーンの離脱 → 選択肢の送り → インストールの窓口 → 更新の窓口（[`crate::install::register`]・
///   [`crate::update::register`] は窓口と終了の待ちの門もここで 1 度だけ据え、シェル・バルーンの
///   取り出しの登録は終了の片付けを終了の待ちへ登記する）
/// - `FrameFinalize` ← クリック透過 → OS の閉鎖要求 → 重なり順の対（状態がゴーストごとで
///   ないので `wire_zorder_pair` をそのまま呼ぶ）
///
/// 各系の並び（`before`／`after`・`chain`）は各登録関数が持つ。ここは順に呼ぶだけ。
///
/// 今日との差: `Input` の 9 系と `Update` の毎フレームの相（`emo2_frame_system`）は LogSink の
/// 起動でも登録される。どれも状態（`NonSend`）が無ければ無操作で戻る（記録は `trace!` か無し）
/// ので、見え方は変わらない（インストールと更新の窓口だけは登録と同時に据わるが、依頼が無ければ
/// 何もしない）。バルーンの離脱の系だけは `BalloonWiring` 不在で
/// `error!(balloon_wiring_missing)` の枝を持つが、そこへは `PointerLeave` が立ったときにしか
/// 進まず、LogSink の起動のバルーン窓は `HitTest::none()` のまま（当たり判定の面は結線ありの
/// 起動でだけ装着する）なので `PointerLeave` が立たず届かない。
pub(crate) fn register_systems(world: &mut World, kanade_stop_rx: Receiver<KanadeNotice>) {
    emo2_boot::register_emo2_frame_system(world);
    emo2_boot::wire_kanade_stop(world, kanade_stop_rx);

    readme::register_readme_drain(world);
    emo2_boot::ghost_switch::register_change_drain(world);
    emo2_boot::shell_balloon_switch::register_switch_drain(world);
    input_events::user_break::register_user_break_drain(world);
    menu::register_menu_poll(world);
    input_events::balloon::register_balloon_leave_system(world);
    input_events::choice_drain::register_choice_drain(world);
    crate::install::register(world);
    crate::update::register(world);

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
#[derive(Clone)]
pub(crate) struct StartupDescriptValues {
    /// 採寸 k₀ と attach（`attach_target`）が共有する作者基準 DPI。
    pub(crate) author_dpi: placement::AuthorDpi,
    /// shell descript の `seriko.zorder` の生の値（未指定なら `None`）。解釈は台帳の層が行う。
    pub(crate) zorder_raw: Option<String>,
}

/// 配置の準備（[`prepare_ghost_windows`]）が決めた起動のシェル（areka-P0-shell-balloon-switch
/// 要件 6.4・8.4）。起こす処理の 2 つの入口（[`boot_ghost`]・[`boot_ghost_strict`]）が取り出して
/// 起動の結線（結線ありの腕と LogSink の倒れ先の両方）へ渡す。`Emo2BootInputs`・
/// [`StartupDescriptValues`]・`GhostBootOptions` に欄を足さずに運ぶための置き場である。
#[derive(bevy_ecs::prelude::Resource)]
pub(crate) struct BootShellChoice {
    /// 決めたゴーストの根（起こすゴーストと違えば使わない）。
    pub ghost_root: PathBuf,
    /// 起動のシェルのフォルダ名（`None` は既定のシェル）。
    pub shell: Option<String>,
}

/// 起こすゴーストのシェル: 配置の準備が置いた値を取り出す。無いか根が違えば（準備を経ない
/// 起動・別のゴーストの残り）ここで決める。
fn take_boot_shell(world: &mut World, ghost_root: &Path) -> Option<String> {
    match world.remove_resource::<BootShellChoice>() {
        Some(choice) if choice.ghost_root == ghost_root => choice.shell,
        _ => crate::boot_resolve::decide_boot_shell(ghost_root),
    }
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

/// 窓を作る側（要件 3.3・3.4・window-placement 1.4）: 準備（[`prepare_ghost_windows`]）と
/// 投函（[`commit_ghost_windows`]）を続けて呼ぶ（初回起動の `main` の入口・署名不変）。
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
/// 準備は同期実行し、I/O はそこで完結・Send な値のみを ECS
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
    let prepared = prepare_ghost_windows(world, cfg)?;
    let descript = prepared.descript.clone();
    commit_ghost_windows(world, prepared);
    Ok(descript)
}

/// 準備を終えた窓（design「prepare_ghost_windows / commit_ghost_windows」）: 準備が descript から
/// 1 度だけ読んだ値と、窓を作る閉包（まだ作業プールへ渡していない）。投函
/// （[`commit_ghost_windows`]）するまで窓は 1 枚も生えない。
#[must_use = "準備した窓は投函しないと生えない（commit_ghost_windows へ渡す）"]
pub(crate) struct PreparedWindows {
    /// 準備が descript から 1 度だけ読んだ値（ゴーストごとの結線へ渡す）。
    pub(crate) descript: StartupDescriptValues,
    /// 窓の生成と受け口の装着を行う閉包（`Input` 段で World に適用される）。
    spawn: BoxedCommand,
}

/// 窓の準備（同期）: 作業プールの有無 → 配置の準備 → 監視の 2 源 → 復元 → 窓を作る閉包を組む。
/// 閉包は作業プールへまだ渡さない（渡すのは [`commit_ghost_windows`]）ので、準備だけでは窓は
/// 生えない——切替先の起動に失敗したとき、壊れた切替先の窓が孤児として生えないための分割。
///
/// - 最初に作業プール（[`WintfTaskPool`]）の有無を確かめる。無ければ配置の準備に入らず
///   `error!(event = "task_pool_missing")` の上で [`OpenWindowsError::TaskPoolMissing`]。
/// - 配置の準備の失敗は [`placement::PlacementError`] を包んで返す。
pub(crate) fn prepare_ghost_windows(
    world: &mut World,
    cfg: &ConfigInputs,
) -> Result<PreparedWindows, OpenWindowsError> {
    if !world.contains_resource::<WintfTaskPool>() {
        tracing::error!(
            event = "task_pool_missing",
            ghost_root = %cfg.ghost_root.display(),
            "[prepare_ghost_windows] 作業プールが World に無いので窓を作れません（配置の準備に入らず失敗を返す）"
        );
        return Err(OpenWindowsError::TaskPoolMissing);
    }
    // 起動のシェルはここで 1 度だけ決め（記憶の先が無ければ `warn!` もここの 1 件）、配置の
    // 情報源へ渡したうえで資源に置く。起こす処理の入口（[`boot_ghost`]・[`boot_ghost_strict`]）が
    // 取り出し、同じ値を資産と実行系へ運ぶ
    // （areka-P0-shell-balloon-switch 要件 6.2〜6.4）。
    let shell = crate::boot_resolve::decide_boot_shell(&cfg.ghost_root);
    let prepared = placement::prepare_ghost_windows_for_shell(
        &cfg.ghost_root,
        &cfg.balloon_root,
        shell.as_deref(),
    )?;
    world.insert_resource(BootShellChoice {
        ghost_root: cfg.ghost_root.clone(),
        shell,
    });
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

    // 投函の後、作業プールの async タスク → CommandSender → Input スケジュールで
    // World 適用という既存 ECS コマンド経路で本物窓を組み立てる。
    // 組んだ時点の「窓を閉じた回数」を控え、着いたときに進んでいれば作らない（投函の後に全窓を
    // 閉じた＝この窓はもう要らない・`app_exit::WindowsEpoch`）。
    let epoch_of = |world: &World| {
        world
            .get_resource::<app_exit::WindowsEpoch>()
            .map_or(0, |e| e.0)
    };
    let epoch = epoch_of(world);
    let ghost_root = cfg.ghost_root.clone();
    let spawn: BoxedCommand = Box::new(move |world: &mut World| {
        if epoch_of(world) != epoch {
            tracing::debug!(
                event = "ghost_windows_stale",
                ghost_root = %ghost_root.display(),
                "投函の後に全窓が閉じられたので、着いた窓の閉包は窓を作らない"
            );
            return;
        }
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
        // 投げ込みの受け手をゴースト窓（キャラ窓・バルーン窓）へ差す（areka-P0-file-drop・
        // 要件 1.1・1.2）。起こし直しもこの閉包を通るので、回数によらず新しい窓に差さる。
        input_events::file_drop::attach_file_drop_receivers(world);
        let scopes: Vec<usize> = windows.scopes().collect();
        tracing::info!(
            ?scopes,
            "本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓）"
        );
    });

    Ok(PreparedWindows {
        descript: StartupDescriptValues {
            author_dpi,
            zorder_raw,
        },
        spawn,
    })
}

/// 窓の投函: 準備した閉包を作業プールへ渡す。窓は次の `Input` 段で生える。
///
/// 作業プールが無ければ（準備の後に外された＝配線の誤り）閉包を捨てた上で
/// `error!(event = "task_pool_missing")` を残す。
pub(crate) fn commit_ghost_windows(world: &mut World, prepared: PreparedWindows) {
    let Some(task_pool) = world.get_resource::<WintfTaskPool>() else {
        tracing::error!(
            event = "task_pool_missing",
            "[commit_ghost_windows] 作業プールが World に無いので窓を作れません（準備した窓を捨てる）"
        );
        return;
    };
    let spawn = prepared.spawn;
    task_pool.spawn(|tx: CommandSender| async move {
        let _ = tx.send(spawn);
    });
}

/// 閉じた証を受けて窓を作り直す準備をする（要件 3.4・4.4・4.10）。証（[`WindowsClosed`]）の
/// 唯一の消費先で、手順は 1 度目と同じ [`prepare_ghost_windows`] へ委譲する。窓は投函
/// （[`commit_ghost_windows`]）するまで生えない（呼び手は起動が成功したときだけ投函する）。
// 本番の呼び手は areka-P0-ghost-shell-balloon-switch（ゴーストの切替）。
pub(crate) fn reopen_ghost_windows(
    world: &mut World,
    cfg: &ConfigInputs,
    closed: WindowsClosed,
) -> Result<PreparedWindows, OpenWindowsError> {
    tracing::debug!(
        closed = closed.closed(),
        "[reopen_ghost_windows] 閉じた窓の後に窓を作り直す"
    );
    prepare_ghost_windows(world, cfg)
}

/// ゴーストごとの結線の入力の束（design「boot_ghost」）。
pub(crate) struct GhostBootInputs {
    /// 起動の結線の入力（ゴーストの根・バルーンの根・SHIORI の結線・時計の種類・記憶の置き場）。
    pub wiring: emo2_boot::Emo2BootInputs,
    /// fallback（`LogSink`）の起動が使う 32bit SHIORI helper のパス。
    pub helper_exe: PathBuf,
    /// 停止通知の送出端の写し（結線あり・fallback の両方の起動へ渡る・`shiori-fault-notice`）。
    pub kanade_stop: Sender<KanadeNotice>,
}

impl GhostBootInputs {
    /// 本番の値: SHIORI は `Helper { helper_exe }`・時計は `Real` 既定・記憶の置き場は
    /// `Some(default_app_profile_dir())`・根は構成入力のもの・起動の由来は呼び手が渡したもの。
    pub(crate) fn production(
        cfg: &ConfigInputs,
        helper_exe: PathBuf,
        kanade_stop: Sender<KanadeNotice>,
        boot_origin: BootOrigin,
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
                boot_origin,
            },
            helper_exe,
            kanade_stop,
        }
    }
}

/// 起動入力の作り口（World の NonSend・プロセスに 1 つ）: 構成入力と起動の由来から
/// [`GhostBootInputs`] を組む。切替が相手のゴーストを起こすときに使う（`ShioriWiring::Custom` は
/// 写せず、停止通知の送り口は `main` にしか無いので、作り口ごと World に置く）。本番は
/// [`GhostBootInputs::production`] を helper のパスと停止通知の送り口の写しで閉じたもの、
/// テストは根のフォルダごとに偽の SHIORI を返すものを据える。
pub(crate) struct GhostBootInputsSource(
    pub(crate) Box<dyn Fn(&ConfigInputs, BootOrigin) -> GhostBootInputs>,
);

/// 起こしたゴーストの置き場（World の NonSend・プロセスに 1 つ）。`main` が据え、切替が
/// 入れ替え、`main` が `run()` の後に取り出して降ろす。
pub(crate) struct GhostSlot(pub(crate) Option<GhostSession>);

/// 結線ありの起動が成立しなかった（理由は `wire_emo2_boot` が `warn!`／`error!` で記録済み）。
#[derive(Debug, thiserror::Error)]
#[error("ゴーストの起動の結線が成立しなかった")]
pub(crate) struct BootWiringFailed;

/// 1 体のゴーストの 3 ハンドル（ゴースト実行系・seriko・loop ticker）。
///
/// **落としても何も起きない。降ろすのは必ず [`GhostSession::shutdown`] を呼ぶこと**
/// （`Drop` は終了の理由を持てないので実装しない・要件 1.7）。
pub(crate) struct GhostSession {
    ghost: Option<areka_ghost::GhostRuntime>,
    seriko: Option<areka_actor::ActorHandle>,
    loop_ticker: Option<mpsc::Sender<areka_ghost::ticker::TickerMsg>>,
    /// seriko の送り手の複製（シェル・バルーンの差し替えを seriko へ頼む口）。結線ありの腕だけが
    /// 持ち、テスト用の組み立てと LogSink の腕は `None`。降ろす最初の段で落とす（要件 8.5）。
    seriko_sink: Option<areka_seriko::SerikoSink>,
    /// kanade への送出端の写し（実行系が無ければ `None`）。実行系から毎回引かずに持つのは、
    /// テストが実行系を起こさずに送出端だけを差せるようにするため（[`GhostSession::for_test`]）。
    kanade: Option<Sender<KanadeMsg>>,
    /// 起こしたゴーストの根（`ghost/<フォルダ名>`・起動の結線の入力のもの）。
    ghost_dir: PathBuf,
    /// テスト用の記憶の書き手（実行系を起こさずに差し替えの後始末の記憶を観測する）。
    #[cfg(test)]
    memory_publisher_for_test: Option<areka_sylphya::SylphyaPublisher>,
    /// 窓への結線が成立せず LogSink の起動へ倒れた単位か（倒れた先の成否を問わない・生涯で不変）。
    /// 結線ありの腕・切替の経路・テスト用の組み立ては偽（要件 4.2・4.8・8.3）。
    logsink_fallback: bool,
}

impl GhostSession {
    /// 告知の場面が使うゴースト名（fallback の起動に失敗して実行系が無ければ `None`）。
    pub(crate) fn ghost_name(&self) -> Option<String> {
        self.names().and_then(|n| n.name.clone())
    }

    /// ゴーストの名前情報（descript の `name`・`sakura.name` ほか）。実行系が無ければ `None`。
    pub(crate) fn names(&self) -> Option<&GhostNames> {
        self.ghost.as_ref().map(|r| &r.mount().names)
    }

    /// ゴーストの実行系（定常到達の記憶を、そのゴーストの記憶の書き手へ投函する）。無ければ `None`。
    pub(crate) fn runtime(&self) -> Option<&areka_ghost::GhostRuntime> {
        self.ghost.as_ref()
    }

    /// 窓への結線が成立せず LogSink の起動へ倒れた単位か（印の判定の材料・要件 4.1・4.2）。
    pub(crate) fn logsink_fallback(&self) -> bool {
        self.logsink_fallback
    }

    /// kanade への送出端（切替の要求を送る）。実行系が無ければ `None`。
    pub(crate) fn kanade(&self) -> Option<&Sender<KanadeMsg>> {
        self.kanade.as_ref()
    }

    /// seriko の送り手の複製（シェル・バルーンの差し替えを頼む）。結線ありの腕でなければ `None`。
    pub(crate) fn seriko_sink(&self) -> Option<&areka_seriko::SerikoSink> {
        self.seriko_sink.as_ref()
    }

    /// 実行系の記憶の書き手（差し替えの後始末が `LastShell`／`LastBalloon` を投函する）。
    /// 実行系が無ければ `None`（テスト用の組み立ては [`GhostSession::with_memory_publisher`] の値）。
    pub(crate) fn memory_publisher(&self) -> Option<&areka_sylphya::SylphyaPublisher> {
        #[cfg(test)]
        if let Some(publisher) = &self.memory_publisher_for_test {
            return Some(publisher);
        }
        self.ghost.as_ref().map(|r| r.sylphya_publisher())
    }

    /// 今のシェルのフォルダだけを書き換える（実行系へ委ねる・要件 6.5）。実行系が無ければ偽。
    pub(crate) fn set_shell_dir(&mut self, dir: PathBuf) -> bool {
        match self.ghost.as_mut() {
            Some(runtime) => {
                runtime.set_shell_dir(dir);
                true
            }
            None => false,
        }
    }

    /// テスト用の組み立てに記憶の書き手を持たせる（差し替えの後始末の記憶を観測するため）。
    #[cfg(test)]
    pub(crate) fn with_memory_publisher(
        mut self,
        publisher: areka_sylphya::SylphyaPublisher,
    ) -> Self {
        self.memory_publisher_for_test = Some(publisher);
        self
    }

    /// テスト用の組み立てに seriko の送り手を持たせる（切替の入口の文脈の判定を通すため）。
    #[cfg(test)]
    pub(crate) fn with_seriko_sink(mut self, sink: areka_seriko::SerikoSink) -> Self {
        self.seriko_sink = Some(sink);
        self
    }

    /// 実行系を持たない置き場の中身（テスト用）: kanade への送出端と根だけを持つ。
    /// 降ろすもの（実行系・seriko・ticker）が無いので `shutdown` は何もせず `Ok` を返す。
    #[cfg(test)]
    pub(crate) fn for_test(kanade: Option<Sender<KanadeMsg>>, ghost_dir: PathBuf) -> Self {
        Self {
            ghost: None,
            seriko: None,
            loop_ticker: None,
            seriko_sink: None,
            kanade,
            ghost_dir,
            logsink_fallback: false,
            #[cfg(test)]
            memory_publisher_for_test: None,
        }
    }

    /// 台詞を進める Tick の注入口（テスト用・時計を止めて起こした実行系の dispatcher）。
    /// 実行系が無ければ `None`。
    #[cfg(test)]
    pub(crate) fn dispatcher(&self) -> Option<&Sender<areka_ghost::dispatcher::DispatcherMsg>> {
        self.ghost.as_ref().map(|r| r.dispatcher())
    }

    /// 起こしたゴーストの根（実行系が無くても起動に渡した根を返す）。
    pub(crate) fn ghost_dir(&self) -> &Path {
        &self.ghost_dir
    }

    /// 降ろす（要件 1.1・1.3・1.5・1.6）: ① loop ticker の停止（seriko の送り手の複製も落とす）
    /// → ② ゴースト実行系の終了
    /// （`reason` を渡す）→ ③ seriko の join。無い段は飛ばし、②③ の失敗は `error!` の上で
    /// `Err` を返して以降を飛ばす。perf の最終報告はプロセスに 1 回なので呼び手が後で行う。
    pub(crate) fn shutdown(self, reason: areka_kanade::CloseReason) -> windows::core::Result<()> {
        self.shutdown_impl(reason, None).0
    }

    /// 期限つきで降ろす（OS のセッションの終了の受け手だけが呼ぶ・要件 1.1・2.1）。手順は
    /// [`GhostSession::shutdown`] と同じで、② だけを見張りつきの `GhostRuntime::shutdown_within` に
    /// 替える。戻りの 2 つ目は上限で SHIORI の待ちを打ち切ったときだけ `Some`（実行系が無ければ `None`）。
    pub(crate) fn shutdown_within(
        self,
        reason: areka_kanade::CloseReason,
        budget: areka_kanade::WaitBudget,
    ) -> (windows::core::Result<()>, Option<areka_kanade::ShioriCut>) {
        self.shutdown_impl(reason, Some(budget))
    }

    /// 降ろす手順の本体（`budget` が在れば ② を見張りつきで走らせる）。
    fn shutdown_impl(
        self,
        reason: areka_kanade::CloseReason,
        budget: Option<areka_kanade::WaitBudget>,
    ) -> (windows::core::Result<()>, Option<areka_kanade::ShioriCut>) {
        // ① loop ticker Close（本ブロック）: SERIKO ループ ticker の worker スレッドは closure 内へ
        // `SerikoSink` クローン（tick_sink）を握る。これを先に停止させないと seriko inbox が ticker 経由で
        // 生き続け、③ の join が「全 Sender drop」を永遠に待って hang する。停止端 Sender へ
        // `TickerMsg::Close` を送ると worker は `recv_timeout` から `Ok(Close)` で return し、その closure＝
        // tick_sink が drop される（inbox 切断の 1 本が外れる。セッションの複製はこの段の末尾、
        // ghost 側 SerikoSink は ② が外す）。
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
        // セッションが持つ seriko の送り手の複製（差し替えの口）も ① で落とす。残すと self の残りの欄
        // として関数の終わりまで生き、③ の join が「全 Sender drop」を待ち続けて hang する（要件 8.5）。
        drop(self.seriko_sink);

        // ② 終了握手（task 5.2・design「終了握手（R6）」・DD-10）: boot 済み（`Some`）のときのみ
        // `shutdown` を呼ぶ。終了理由は呼び手が渡す（`fn main` は `CloseReason::User { scope: 0 }`＝
        // 全窓 close funnel はユーザ操作起点）。OnClose 応答の再生完了待ちは kanade の `ForceQuit`
        // 終了系列内で処理される（本仕様は `shutdown` を呼ぶだけ・不改変・R6.2）。失敗は `error!` の上で
        // 呼び手へ `Err` を返す（genuine な失敗を黙って exit 0 にしない・R6.3）。
        let mut cut = None;
        if let Some(runtime) = self.ghost {
            let result = match budget {
                Some(budget) => {
                    let (result, fired) = runtime.shutdown_within(reason, budget);
                    cut = fired;
                    result
                }
                None => runtime.shutdown(reason),
            };
            if let Err(err) = result {
                tracing::error!(error = %err, "ghost 結線層の終了統括に失敗しました");
                return (
                    Err(windows::core::Error::from_hresult(
                        windows::Win32::Foundation::E_FAIL,
                    )),
                    cut,
                );
            }
        }

        // ③ seriko アクターの join（design「終了握手（R6）」・R6.3）。seriko inbox への送信端は 3 本ある:
        // (a) ghost 側の `SerikoSink`（surface_sink・②の `shutdown` が drop）と (b) loop ticker closure の
        // `tick_sink`（①の Close→worker return で drop）と (c) セッションの複製（`seriko_sink`・① で drop）。
        // ①②で全端が drop されて inbox が切断され、seriko worker は自然終了する。複製は `GhostSession`
        // だけが持ち ① で落とす（それ以外の sink は `wire_emo2_boot` が boot／ticker へ move 済み）ため、
        // この `join` は全端 drop 完了（＝ticker worker 終端）まで block したうえで速やかに戻る（①で ticker
        // を先に Close し複製を落としたことが hang 回避の要）。join 失敗（worker
        // panic）は握り潰さず `error!`＋`Err` 伝播する（genuine な失敗を隠さない）。
        if let Some(seriko) = self.seriko {
            if let Err(err) = seriko.join() {
                tracing::error!(error = %err, "seriko アクターの join に失敗しました");
                return (
                    Err(windows::core::Error::from_hresult(
                        windows::Win32::Foundation::E_FAIL,
                    )),
                    cut,
                );
            }
        }

        (Ok(()), cut)
    }
}

/// ゴーストごとの結線と状態の載せ替え（要件 2.3・2.4・2.6・design「boot_ghost」）。今日どおりの入口。
///
/// 起動の結線が成立すれば結線ありの腕（[`boot_wired`]）を、成立しなければ `LogSink`×2 の
/// fallback の腕を通す（初回起動の非致命）。各状態は `insert_non_send` で置き換わるので n 回
/// 呼べる（呼び手は先に [`GhostSession::shutdown`] で前のゴーストを降ろしておくこと）。証は
/// 取らない（証は [`reopen_ghost_windows`] が消費する）。
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
    // fallback の腕が使う根と App スコープの置き場（結線の入力は `boot_wired` へ move するので先に写す）。
    // 置き場は本番では `GhostBootInputs::production` の `Some(default_app_profile_dir())`＝
    // `ghost_boot_options` の既定と同じ値（振る舞いは不変・要件 4.7）。テストは土台の置き場か `None`。
    let ghost_root = wiring.ghost_root.clone();
    let app_profile_dir = wiring.app_profile_dir.clone();
    // 起動のシェルは結線ありの腕と倒れ先で同じ値を使う（取り出すのはここの 1 回だけ・要件 6.4）。
    let shell = take_boot_shell(world, &ghost_root);

    // `wired=false`（asset 組立失敗・boot 失敗等）は現行の `LogSink`×2 フォールバック boot へ
    // 倒し、既存 smoke 前提・非致命 boot 意味論を温存する（R7.1/7.3・DD-7）。
    if let Ok(session) = boot_wired(
        world,
        wiring,
        &kanade_stop,
        descript,
        ghost,
        balloon,
        shell.as_deref(),
    ) {
        return session;
    }

    // フォールバック（R7.3・DD-7）: 現行の `LogSink`×2 boot を UI 基盤・起動窓の後へ
    // relocate したもの。失敗は非致命——起動前の解決が `ghost/master/descript.txt` の実在を
    // 確かめているので、ここでの `MountError::StartPointMissing` は解決後の消失（起動中の削除等）
    // に限られ、`warn!` の上で `None` として骨格起動を継続する（要件 8.2）。それ以外の予期しない
    // 失敗（読取不能・shell 不在等）は `error!`（`is_benign_boot_error` の分類は不変・R7.4）。
    let ghost_options = areka_ghost::GhostBootOptions {
        app_profile_dir,
        ..ghost_boot_options(ghost_root.clone(), helper_exe)
    };
    // 停止通知の送出端つきで起動する（SHIORI の失敗で kanade が止まったら終了相へ届く・要件 6.4）。
    // 倒れ先も起動のシェルでマウントする（areka-P0-shell-balloon-switch 要件 6.4）。
    let runtime = match areka_ghost::boot_with_origin(
        ghost_options,
        Some(kanade_stop),
        BootOrigin::Plain,
        shell.as_deref(),
    ) {
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
    // 倒れたことは倒れた先の成否によらず単位に残す（印の判定の材料・要件 4.1・4.2）。
    GhostSession {
        kanade: runtime.as_ref().map(|r| r.kanade().clone()),
        ghost: runtime,
        seriko: None,
        loop_ticker: None,
        seriko_sink: None,
        ghost_dir: ghost_root,
        logsink_fallback: true,
        #[cfg(test)]
        memory_publisher_for_test: None,
    }
}

/// 切替用の入口（要件 6.1）: 結線が成立しなければ fallback の骨格へ倒れず
/// `Err(BootWiringFailed)` を返す（呼び手が既定ゴーストへ戻すか致命で終える）。成立したときの
/// 状態の載せ替えは [`boot_ghost`] の結線ありの腕と同じ（[`boot_wired`]）。
pub(crate) fn boot_ghost_strict(
    world: &mut World,
    inputs: GhostBootInputs,
    descript: &StartupDescriptValues,
    ghost: &GhostDecision,
    balloon: &BalloonDecision,
) -> Result<GhostSession, BootWiringFailed> {
    // helper のパスは fallback の腕だけが使う（厳格な入口は fallback を持たない）。
    let GhostBootInputs {
        wiring,
        helper_exe: _,
        kanade_stop,
    } = inputs;
    let shell = take_boot_shell(world, &wiring.ghost_root);
    boot_wired(
        world,
        wiring,
        &kanade_stop,
        descript,
        ghost,
        balloon,
        shell.as_deref(),
    )
}

/// 結線ありの腕（2 つの入口が共有・私有）: 起動の結線（`wire_emo2_boot`）を試み、成立すれば
/// 窓ごとの状態（マウス・メニュー・記憶・バルーンの選択・選択肢の送り）を `insert_non_send` で
/// 新品へ置き換えて [`GhostSession`] を返す（要件 4.8）。成立しなければ `Err(BootWiringFailed)`
/// （理由は `wire_emo2_boot` が記録済み）で、World の状態には触れない。`shell` は入口が
/// 取り出した起動のシェル（[`take_boot_shell`]）で、結線へそのまま渡す（要件 6.4）。
fn boot_wired(
    world: &mut World,
    wiring: emo2_boot::Emo2BootInputs,
    kanade_stop: &Sender<KanadeNotice>,
    descript: &StartupDescriptValues,
    ghost: &GhostDecision,
    balloon: &BalloonDecision,
    shell: Option<&str>,
) -> Result<GhostSession, BootWiringFailed> {
    let ghost_dir = wiring.ghost_root.clone();

    // emo2 統合結線（task 5.2・design「エントリポイント / main.rs＋wire_emo2_boot」・DD-7）:
    // UI 基盤・起動窓の後で完成済み 5 トラック（seriko／sakura／emo-present／emo-text／actor）を
    // 束ねる実 sink 結線を試みる。`wired=true` なら実 sink boot が成立し、ghost／seriko ハンドルを
    // 終了処理へ運ぶ。
    let outcome = emo2_boot::wire_emo2_boot(
        world,
        wiring,
        descript.author_dpi,
        descript.zorder_raw.as_deref(),
        kanade_stop.clone(),
        shell,
    );
    if !outcome.wired {
        return Err(BootWiringFailed);
    }
    tracing::info!("実 sink 結線で起動しました（emo2-boot wire 成立・SERIKO ループ ticker 稼働）");
    // マウス配信資源を World へ結線（task 3.1・design「main.rs＋wire_mouse_input」・
    // DD-IE-9）: kanade Sender クローンで MouseWiring（NonSend・Presenter）を挿入する。
    // 挿入は wire_emo2_boot 成功後＝Emo2Wiring 挿入済みゆえ presenter 経由の region 解決が
    // 成立する（Emo2Wiring 挿入と同位置・同型・self-gating）。窓へのハンドラ登録は task 3.2。
    if let Some(runtime) = outcome.ghost.as_ref() {
        let sender = runtime.kanade().clone();
        input_events::wire_mouse_input(world, sender);
        // 右クリックメニューの結線（areka-P0-popup-menu-minimal）: 終了の項目が上の入力の結線を使う。
        menu::wire_menu(world, runtime.kanade().clone());
        // 「ゴースト」枠の登記（要件 1.12）: `wire_menu` が登記の口を新品にするので、起こすたびにやり直す。
        menu::ghost_frame::register(world);
        // 「インストール」枠の登記（ghost-install 要件 1.8）: 同じ理由で起こすたびにやり直す。
        menu::install_frame::register(world);
        // 「ネットワーク更新」枠の登記（network-update 要件 1.2）: 同じ理由で起こすたびにやり直す。
        menu::update_frame::register(world);
        // 置換語 %lastghostname・%lastobjectname の載せ直し（記憶の置き場は起こすたびに新しくなる）。
        crate::install::names::reseed(world, runtime);
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
    Ok(GhostSession {
        kanade: outcome.ghost.as_ref().map(|r| r.kanade().clone()),
        ghost: outcome.ghost,
        seriko: outcome.seriko,
        loop_ticker: outcome.loop_ticker,
        seriko_sink: outcome.seriko_sink,
        ghost_dir,
        logsink_fallback: false,
        #[cfg(test)]
        memory_publisher_for_test: None,
    })
}

#[cfg(test)]
#[path = "ghost_session_restart_tests.rs"]
mod restart_tests;

#[cfg(test)]
#[path = "ghost_session_strict_tests.rs"]
mod strict_tests;

#[cfg(test)]
#[path = "ghost_session_switch_tests.rs"]
mod switch_tests;

#[cfg(test)]
#[path = "boot_shell_tests.rs"]
mod boot_shell_tests;

#[cfg(test)]
#[path = "shell_balloon_switch_session_tests.rs"]
mod shell_balloon_switch_session_tests;
