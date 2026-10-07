//! シェルの往復の統合テスト（spec: areka-P0-shell-balloon-switch task 11.1・要件 1.8・2.1〜2.6・2.8・
//! 4.2・4.3・11.1・11.3・design「Integration Tests」）。
//!
//! 偽の SHIORI の土台（[`SwitchRig`]・emo2 の複製のゴースト A）に、`R_POST_and_KOMAINU` のシェルを
//! [`SampleRoot::add_shell_copy`] で `second` と名付けて写したものを A の 2 つ目のシェルとして足す。
//! 2 つのシェルは絵の大きさが違うので、差し替えで窓寸が変わる。同じ World に GPU 資源（WARP 可）と、
//! 本物の配置の準備（作業領域だけ合成）で組んだ窓の一式（偽 HWND）を据え、本番の `Input`・`Update` の
//! 段（`emo2_frame_system`）をそのまま回す。台詞は置き場のゴーストの dispatcher へ合成の Tick を
//! 注入して進める（止めれば台詞の時計は進まない）。判定は集めてから 1 回・降ろすのは有界に行う。

use std::collections::BTreeSet;
use std::panic::Location;
use std::path::Path;
use std::time::{Duration, Instant};

use areka_ghost::dispatcher::DispatcherMsg;
use areka_kanade::MonotonicMs;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::Children;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::SampleRoot;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
};
use wintf::ecs::{
    DPI, FrameTime, GraphicsCore, Input, Update, WindowHandle, WindowPos, WucGraphicsResource,
};

use super::GhostSlot;
use crate::boot_resolve::read_last_shell;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::ghost_switch_test_support::{
    BALLOON, FakeShiori, SwitchRig, standard_script,
};
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinOrigin, SkinRequest, SkinSpec, SkinSwitchInFlight, SkinSwitchStage, SkinVerdict,
    request_skin_switch,
};
use crate::emo2_boot::spine::{Progress, RecordedCall, ScriptedShioriBackendBuilder, wait_until};
use crate::emo2_boot::target_map::shell_target;
use crate::input_events::user_break::UserBreakWiring;
use crate::placement::follow::MonitorSnapshot;
use crate::placement::resolver::RectPx;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};

/// 2 つ目のシェルのフォルダ名（`descript.txt` の `name` も同じ）。
pub(super) const SECOND: &str = "second";
/// 合成の作業領域（配置の準備と窓の追従の両方が読む）。
const WORK_AREA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 3840,
    bottom: 2100,
};
/// Tick を注入する実時間の最小間隔（反復ごとに投函して受信箱を溢れさせない）。
const TICK_EVERY: Duration = Duration::from_millis(1);

/// 1 回の待ちの中で注入する Tick（1 回で台詞の時計を進める幅と、注入の回数の上限）。
///
/// 投函した Tick は talk スレッドが遅れると溜まり、後から一度に消化される。台詞の時計を観測より
/// 先へ進めてはならない場面では、幅と回数で 1 回の待ちの進み幅を抑える。
#[derive(Clone, Copy)]
pub(super) struct Ticks {
    step_ms: u64,
    max: usize,
}

/// Tick を注入しない（台詞の時計を止めたまま回す）。
pub(super) const NO_TICKS: Ticks = Ticks { step_ms: 0, max: 0 };
/// 100 ms ずつ・上限なし（進み方を問わない場面）。
pub(super) const UNBOUNDED: Ticks = Ticks {
    step_ms: 100,
    max: usize::MAX,
};
/// 1 ms ずつ・上限なし（台詞の始まりを見るまで。台詞の開始は kanade から中継を経て dispatcher へ
/// 届くので Tick より遅れうる。始まる前の Tick は捨てられ、始まった後に溜まった Tick も 1 回 1 ms
/// しか進めない）。
pub(super) const CREEP: Ticks = Ticks {
    step_ms: 1,
    max: usize::MAX,
};
/// 台詞の始まりを見た後、受理までに注入する Tick（100 ms × 20 回＝台詞の時計で高々 2,000 ms。
/// 命令の位置〔台本の頭から数十 ms〕には届き、命令の後の待ち 5,000 ms は食い切らない）。
pub(super) const ACCEPT: Ticks = Ticks {
    step_ms: 100,
    max: 20,
};
/// 起動のシェル（emo2）の面 0・面 10 の素の大きさ。
const MASTER_SIZES: [(i32, i32); 2] = [(434, 687), (336, 400)];
/// 2 つ目のシェル（`R_POST_and_KOMAINU`）の面 0・面 10 の素の大きさ。
const SECOND_SIZES: [(i32, i32); 2] = [(236, 462), (140, 160)];

// ---------------------------------------------------------------- 土台

/// 窓・GPU つきの往復の土台。
pub(super) struct LapRig {
    pub(super) rig: SwitchRig,
    pub(super) windows: GhostWindows,
    /// 注入した Tick の合成の時刻（単調増加）。
    clock_ms: u64,
}

/// A（起動記録あり＝`OnBoot` の台本から始まる）を `script` の偽の SHIORI で、2 つ目のシェル・
/// 窓の一式・GPU 資源つきで起こす。
pub(super) fn lap_rig(script: impl Fn() -> ScriptedShioriBackendBuilder + 'static) -> LapRig {
    lap_rig_of(
        vec![("A", FakeShiori::Scripted(Box::new(script)))],
        &[SECOND],
    )
}

/// [`lap_rig`] の一般形: `ghosts` の偽の SHIORI（A は必ず含む）で土台を組み、A に
/// `R_POST_and_KOMAINU` のシェルを `shells` の各名前（フォルダ名・`name` とも）で写してから、
/// 窓の一式と GPU 資源つきで A を起こす。
pub(super) fn lap_rig_of(ghosts: Vec<(&str, FakeShiori)>, shells: &[&str]) -> LapRig {
    let mut rig = SwitchRig::new(ghosts);
    rig.plant_boot_record("A");
    for shell in shells {
        add_rpost_shell(&rig.root.ghost_dir("A"), shell);
    }
    let windows = spawn_windows(&mut rig);
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    rig.world.insert_resource(core);
    rig.world.insert_resource(wuc);
    rig.boot("A");
    LapRig {
        rig,
        windows,
        clock_ms: 0,
    }
}

/// `R_POST_and_KOMAINU` の `shell/master` を `add_shell_copy` で `folder`（`name` も `folder`）に
/// 写し、それを A の `shell/<folder>` へ複製する。
fn add_rpost_shell(ghost_dir: &Path, folder: &str) {
    let rpost = SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済みの検体");
    rpost
        .add_shell_copy("master", folder, folder)
        .expect("2 つ目のシェルを写す");
    copy_tree(
        &rpost.folder().join("shell").join(folder),
        &ghost_dir.join("shell").join(folder),
    );
}

/// フォルダ `from` を `to` へ再帰で複製する（検体の複製の中だけで使う）。
pub(super) fn copy_tree(from: &Path, to: &Path) {
    let mut stack = vec![(from.to_path_buf(), to.to_path_buf())];
    while let Some((src, dst)) = stack.pop() {
        std::fs::create_dir_all(&dst).expect("複製のフォルダを作る");
        for entry in std::fs::read_dir(&src).expect("写したシェルを走査する") {
            let entry = entry.expect("要素");
            let target = dst.join(entry.file_name());
            if entry.file_type().expect("要素の種別").is_dir() {
                stack.push((entry.path(), target));
            } else {
                std::fs::copy(entry.path(), &target).expect("ファイルを写す");
            }
        }
    }
}

/// A の起動の配置（作業領域だけ合成・作者 DPI 96）で 2 スコープの窓を生やし、偽 HWND と DPI 96 を
/// 持たせる（`WindowPos` への書込の口が通る・実窓には触れない）。
fn spawn_windows(rig: &mut SwitchRig) -> GhostWindows {
    let prepared = crate::placement::prepare_ghost_windows_with_work_area(
        &rig.root.ghost_dir("A"),
        &rig.root.balloon_dir(BALLOON),
        WORK_AREA,
        Some(96),
    )
    .expect("A の配置の準備");
    let world = &mut rig.world;
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![WORK_AREA],
    });
    let windows = spawn_ghost_windows(world, &prepared.placements, &prepared.titles);
    let mut raw = 0x100usize;
    for scope in windows.scopes() {
        for window in [windows.char_window(scope), windows.balloon_window(scope)] {
            world.entity_mut(window.unwrap()).insert(WindowHandle {
                hwnd: HWND(raw as *mut _),
                instance: HINSTANCE::default(),
            });
            raw += 0x10;
        }
    }
    // `WindowHandle` の付与の hook が偽 HWND の DPI を遅延で書くので、流してから置く。
    world.flush();
    for scope in windows.scopes() {
        for window in [windows.char_window(scope), windows.balloon_window(scope)] {
            world
                .entity_mut(window.unwrap())
                .insert(DPI::from_dpi(96, 96));
        }
    }
    windows
}

impl LapRig {
    /// 本番の `Input`・`Update` の段を `done` が真になるまで有界に回す（期限切れは `false`）。
    /// この呼び出しの中で置き場のゴーストの dispatcher へ合成の Tick を `ticks` のとおり注入して
    /// 台詞を進め、回数を使い切った後は Tick なしでフレームだけを回す。巡ごとに本番の巡と同じく
    /// `FrameTime` を置き、スレッドのメッセージを配る（文字の層の cue の適用とバルーンの表示が進む）。
    ///
    /// 条件の中で足場を書き換えるので、World を借りない進みの目印（`progress_probe`）を先に取ってから
    /// 芯の待ちへ渡す（areka-P0-ghost-session-test-load-flake 要件 2.1・2.2）。打ち切りは目印で決め、
    /// 打ち切ったら呼び出しの場所を添えた文言を標準エラーへ 1 行出して `false`。時刻の送り先は
    /// dispatcher の `DispatcherMsg::Tick` だけ（足場の時刻の注入と同じ決まり・要件 2.6）。
    #[track_caller]
    pub(super) fn frames_until(
        &mut self,
        ticks: Ticks,
        mut done: impl FnMut(&SwitchRig) -> bool,
    ) -> bool {
        let Self { rig, clock_ms, .. } = self;
        let mut last_tick: Option<Instant> = None;
        let mut sent = 0;
        let probe = rig.progress_probe();
        let caller = Location::caller();
        let waited = wait_until(
            &format!("{}:{}", caller.file(), caller.line()),
            Progress::Count(&probe),
            || {
                if sent < ticks.max && last_tick.is_none_or(|at| at.elapsed() >= TICK_EVERY) {
                    sent += 1;
                    last_tick = Some(Instant::now());
                    *clock_ms += ticks.step_ms;
                    if let Some(dispatcher) = rig
                        .world
                        .get_non_send::<GhostSlot>()
                        .and_then(|slot| slot.0.as_ref())
                        .and_then(|session| session.dispatcher())
                    {
                        let _ = dispatcher.send(DispatcherMsg::Tick {
                            now: MonotonicMs(*clock_ms),
                        });
                    }
                }
                // 本番の巡と同じく、巡ごとに `FrameTime` を dola の時計で置く（台詞の文字の現れと
                // バルーンの表示を UI が決める時刻・`TalkClock` と同じ時計）。
                rig.world
                    .insert_resource(FrameTime(dola::runtime::clock::now()));
                pump_messages();
                rig.world.run_schedule(Input);
                rig.world.run_schedule(Update);
                done(rig)
            },
        );
        if let Err(failure) = &waited {
            eprintln!("{failure}");
        }
        waited.is_ok()
    }
}

/// 1 巡で配るメッセージの上限（起こし直しの投函が続いても巡を終わらせる）。
const PUMP_MAX: usize = 1024;

/// 今のスレッドのキューに溜まったメッセージを配る（本番のメッセージループの 1 巡ぶん）。文字の層の
/// cue の適用（`spawn_ui` の受け手）は executor の窓へのメッセージで走るので、配らないと台詞の文字が
/// 文字の層へ載らず、バルーンも現れない。
fn pump_messages() {
    let mut msg = MSG::default();
    for _ in 0..PUMP_MAX {
        // SAFETY: 今のスレッドのキューから 1 件取り出して配るだけ（待たない）。
        let got = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) };
        if !got.as_bool() {
            break;
        }
        // SAFETY: いま取り出したメッセージを同じスレッドで配る。
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// キャラ窓ごとの見え方。
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Look {
    children: Vec<BTreeSet<Entity>>,
    sizes: Vec<Option<(i32, i32)>>,
    /// キャラ窓の下端（位置の y＋高さ）。
    bottoms: Vec<Option<i32>>,
    visible: Vec<Option<bool>>,
}

impl Look {
    /// どのキャラ窓にも装着の子が 2 つ（面と文字層スロット）在る。
    fn full(&self) -> bool {
        !self.children.is_empty() && self.children.iter().all(|c| c.len() == 2)
    }

    fn entities(&self) -> BTreeSet<Entity> {
        self.children.iter().flatten().copied().collect()
    }
}

/// フレームごとの見え方の記録（全部の窓に子がそろった後だけ・続けて同じものは 1 つにまとめる）。
#[derive(Default)]
pub(super) struct LookLog {
    pub(super) distinct: Vec<Look>,
    /// 一度そろった後に子の欠けたフレームが在ったか（窓が空になった）。
    pub(super) blank_after_full: bool,
}

impl LookLog {
    pub(super) fn record(&mut self, look: Look) {
        if !look.full() {
            self.blank_after_full |= !self.distinct.is_empty();
            return;
        }
        if self.distinct.last() != Some(&look) {
            self.distinct.push(look);
        }
    }

    /// 記録した見え方（窓寸・シェルの表示）と、装着の子が見え方ごとに互いに素か、キャラ窓の下端が
    /// 見え方をまたいで最初と同じか（窓寸が変わっても下端揃えのまま置き直された）。
    #[allow(clippy::type_complexity)]
    fn summary(
        &self,
    ) -> (
        Vec<(Vec<Option<(i32, i32)>>, Vec<Option<bool>>)>,
        bool,
        bool,
    ) {
        let shapes = self
            .distinct
            .iter()
            .map(|l| (l.sizes.clone(), l.visible.clone()))
            .collect();
        let sets: Vec<BTreeSet<Entity>> = self.distinct.iter().map(Look::entities).collect();
        let disjoint = sets
            .iter()
            .enumerate()
            .all(|(i, a)| sets[i + 1..].iter().all(|b| a.is_disjoint(b)));
        let anchored = self.distinct.first().is_some_and(|first| {
            first.bottoms.iter().all(Option::is_some)
                && self.distinct.iter().all(|l| l.bottoms == first.bottoms)
        });
        (shapes, disjoint, anchored)
    }
}

/// 窓寸の並び（scope 0・1）を見え方の形へ。
fn sizes(of: [(i32, i32); 2]) -> Vec<Option<(i32, i32)>> {
    of.iter().copied().map(Some).collect()
}

/// 呼出列の種別（`GET OnBoot` の形）。
pub(super) fn kinds(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|c| match c {
            RecordedCall::Get { id, .. } => format!("GET {id}"),
            RecordedCall::Notify { id, .. } => format!("NOTIFY {id}"),
            RecordedCall::Unload => "UNLOAD".to_owned(),
            RecordedCall::Status => "STATUS".to_owned(),
        })
        .collect()
}

/// 呼出列から `id` の GET の Reference を順に取る。
pub(super) fn refs_of(calls: &[RecordedCall], id: &str) -> Vec<Vec<String>> {
    calls
        .iter()
        .filter_map(|c| match c {
            RecordedCall::Get {
                id: got,
                references,
            } if got == id => Some(references.clone()),
            _ => None,
        })
        .collect()
}

/// A の最初の起動の呼出列。
pub(super) fn calls_a(rig: &SwitchRig) -> Vec<RecordedCall> {
    rig.calls("A").into_iter().next().unwrap_or_default()
}

/// `id` の GET が `n` 件以上届いたか。
pub(super) fn got(rig: &SwitchRig, id: &str, n: usize) -> bool {
    refs_of(&calls_a(rig), id).len() >= n
}

/// 置き場のゴーストの記憶の書き手へ柵を掛けてから、A の `LastShell` を実 fs から読む。
pub(super) fn last_shell(rig: &SwitchRig) -> Option<String> {
    let fenced = rig
        .world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime())
        .is_some_and(|runtime| runtime.sylphya_publisher().barrier().is_ok());
    assert!(fenced, "記憶の書き手へ柵を掛けられない");
    read_last_shell(&rig.root.ghost_dir("A"))
}

/// `LastShell` だけを書いた記録（`last_shell_recorded`）のシェルの並び（投函の順）。
pub(super) fn recorded_shells(events: &[CapturedEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("last_shell_recorded"))
        .map(|e| e.field_str("shell").map(str::to_owned))
        .collect()
}

/// 完了の記録（`skin_switch_done`）ごとの印の台詞の終わり方（`marked` の欄の字面）。
pub(super) fn done_marks(events: &[CapturedEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("skin_switch_done"))
        .map(|e| e.field("marked").map(str::to_owned))
        .collect()
}

/// 切替の進行中の印が無い。
pub(super) fn idle(rig: &SwitchRig) -> bool {
    rig.world.get_non_send::<SkinSwitchInFlight>().is_none()
}

pub(super) fn abs(dir: &Path) -> String {
    std::path::absolute(dir).unwrap().display().to_string()
}

/// A の `shell/<folder>` の `descript.txt` の `name`（隠しも含む目録から）。
fn shell_name(rig: &SwitchRig, folder: &str) -> String {
    areka_ghost::catalog::list_all_shells(&rig.root.ghost_dir("A"))
        .into_iter()
        .find(|e| e.identity.folder == folder)
        .and_then(|e| e.identity.name)
        .unwrap_or_else(|| panic!("シェル {folder} に name が在る"))
}

/// 中断の旗の持ち物が生きた送出端につながっているか（会話の状態の持ち物が作り直されていない）。
fn user_break_live(rig: &mut SwitchRig) -> bool {
    rig.world
        .get_non_send_mut::<UserBreakWiring>()
        .is_some_and(|mut w| w.flag_source_connected())
}

// ---------------------------------------------------------------- 往復

/// A の `OnBoot`: 両スコープの面を出し、台詞の終わりで `raise-event` 付きの `second` への切替を命じる。
pub(super) const BOOT_TO_SECOND: &str =
    "\\0\\s[0]\\1\\s[10]\\0A\\![change,shell,second,--option=raise-event]\\e";
/// 1 度目の `OnShellChanging` の台詞（印の台詞）。
const CHANGING_1: &str = "\\0着替えます\\e";
/// 1 度目の `OnShellChanged` の台詞: `master` へ戻る切替を命じる（フォルダ名で引く）。
const CHANGED_1: &str = "\\0\\![change,shell,master,--option=raise-event]\\e";
/// 2 度目の `OnShellChanging` の台詞。
const CHANGING_2: &str = "\\0戻ります\\e";

/// 往復（要件 11.1）: A（`master`）→ 台本の `\![change,shell,second,--option=raise-event]` →
/// `OnShellChanging`（Ref0＝`second`・Ref1＝`master` の名前・Ref2＝`second` の絶対パス・台本あり）→
/// その台詞の再生の後に差し替え → `OnShellChanged`（Ref0＝`second`・Ref1＝ゴースト名 `A`・Ref2）→
/// `LastShell`＝`second` → その台詞の `master` への切替で同じ手順を逆向きに 1 周。集めて 1 回で判定:
/// ⑴ 呼出列（SHIORI を降ろさず起こし直さない＝`OnClose`・`OnBoot` は最初の起動の 1 件だけ・起動 1 回）
/// と Reference ⑵ 記憶 ⑶ キャラ窓の装着の子は見え方ごとに新しい分だけ（互いに素・子の欠けたフレーム
/// なし）⑷ 窓寸が新しいシェルの面の大きさへ・キャラ窓は下端揃えのまま置き直す・シェルは表示のまま
/// （窓寸の要求と表示の引き継ぎ・配置の入れ直し）⑸ 終了の指示なし・中断の旗の持ち物が生きている（会話の状態を保つ）。
///
/// # 非空虚性
/// 差し替えの後に実行系の今のシェルを書き換えないと、2 度目の `OnShellChanging` の Ref1 が `master` の
/// 名前のままで赤。置き換えが古い子を消さないと ⑶ の互いに素か子の数 2 が崩れて赤。
#[test]
fn script_shell_switch_round_trips_with_raise_event() {
    let mut lap = lap_rig(|| {
        standard_script(BOOT_TO_SECOND)
            .get("OnShellChanging", Ok(Some(CHANGING_1.to_owned())))
            .get("OnShellChanged", Ok(Some(CHANGED_1.to_owned())))
            .get("OnShellChanging", Ok(Some(CHANGING_2.to_owned())))
            .get("OnShellChanged", Ok(None))
    });
    let master_name = shell_name(&lap.rig, "master");
    let ghost_dir = lap.rig.root.ghost_dir("A");
    let steady = lap.rig.wait_steady();

    let mut log = LookLog::default();
    let windows = lap.windows.clone();
    // 1 度目の `OnShellChanged` の台詞はすぐ次の切替を命じるので、「1 度目が届いて印が無い」瞬間は
    // フレームの境目に来るとは限らない。往復を 1 本の待ちで回し、途中の記憶は投函の記録の並びで見る。
    let ((round_trip, last), events) = capture(|| {
        let round_trip = lap.frames_until(UNBOUNDED, |rig| {
            log.record(look_of(rig, &windows));
            got(rig, "OnShellChanged", 2) && idle(rig)
        });
        (round_trip, last_shell(&lap.rig))
    });

    let calls = calls_a(&lap.rig);
    let boots = lap.rig.calls("A").len();
    let exit_requested = lap.rig.exit_requested();
    let break_live = user_break_live(&mut lap.rig);
    let shutdown_ok = lap.rig.shutdown();
    let path = |folder: &str| abs(&ghost_dir.join("shell").join(folder));

    assert_eq!(
        (
            (steady, round_trip, boots, kinds(&calls)),
            (
                refs_of(&calls, "OnShellChanging"),
                refs_of(&calls, "OnShellChanged")
            ),
            (recorded_shells(&events), last),
            done_marks(&events),
            log.summary(),
            log.blank_after_full,
            (exit_requested, break_live, shutdown_ok),
        ),
        (
            (
                true,
                true,
                1,
                vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnBoot".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                    "GET OnShellChanging".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "GET OnShellChanged".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "GET OnShellChanging".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "GET OnShellChanged".to_owned(),
                ]
            ),
            (
                vec![
                    vec![SECOND.to_owned(), master_name.clone(), path(SECOND)],
                    vec![master_name.clone(), SECOND.to_owned(), path("master")],
                ],
                vec![
                    vec![SECOND.to_owned(), "A".to_owned(), path(SECOND)],
                    vec![master_name.clone(), "A".to_owned(), path("master")],
                ],
            ),
            (
                vec![Some(SECOND.to_owned()), Some("master".to_owned())],
                Some("master".to_owned())
            ),
            vec![Some("Some(Completed)".to_owned()); 2],
            (
                vec![
                    (sizes(MASTER_SIZES), vec![Some(true); 2]),
                    (sizes(SECOND_SIZES), vec![Some(true); 2]),
                    (sizes(MASTER_SIZES), vec![Some(true); 2]),
                ],
                true,
                true
            ),
            false,
            (false, true, true),
        ),
        "((定常, 往復が終わった, 起動の回数, 呼出列), (OnShellChanging, OnShellChanged の Reference), \
         (LastShell の投函の並び, 実 fs の LastShell), 完了の記録の印の台詞の終わり方, ((見え方ごとの窓寸・シェルの表示), \
         装着の子が互いに素, 下端が保たれた), 子の欠けたフレーム, (終了の指示, 中断の旗, 降ろせた))"
    );
}

/// 今の見え方（キャラ窓ごとの装着の子・窓寸・シェルの表示）。
pub(super) fn look_of(rig: &SwitchRig, windows: &GhostWindows) -> Look {
    let world = &rig.world;
    let presenter = world.get_non_send::<Emo2Wiring>().map(|w| w.presenter());
    let mut look = Look::default();
    for scope in windows.scopes() {
        let window = windows.char_window(scope).unwrap();
        look.children.push(
            world
                .get::<Children>(window)
                .map(|c| c.iter().copied().collect())
                .unwrap_or_default(),
        );
        let pos = world.get::<WindowPos>(window);
        look.sizes
            .push(pos.and_then(|p| p.size).map(|s| (s.width, s.height)));
        look.bottoms
            .push(pos.and_then(|p| Some(p.position?.y + p.size?.height)));
        look.visible
            .push(presenter.and_then(|p| p.target_visible(shell_target(scope as u32))));
    }
    look
}

// ---------------------------------------------------------------- 印の無い台本・メニュー相当

/// 段の名前（`None`＝印が無い）。
fn stage(rig: &SwitchRig) -> Option<&'static str> {
    rig.world
        .get_non_send::<SkinSwitchInFlight>()
        .map(|f| match &f.stage {
            SkinSwitchStage::Waiting { built: None, .. } => "waiting",
            SkinSwitchStage::Waiting { built: Some(_), .. } => "built",
            SkinSwitchStage::Committed { .. } => "committed",
        })
}

/// A の `OnBoot`: 両スコープの面を出し、`raise-event` 無しで `second` への切替を命じてから、
/// 台詞をまだ 5 秒続ける（命令の位置では台本は終わっていない）。
pub(super) const BOOT_TO_SECOND_PLAIN: &str =
    r"\0\s[0]\1\s[10]\0A\![change,shell,second]\_w[5000]\e";

/// `raise-event` 無しの台本の切替（要件 1.2・2.3・11.3）: `OnShellChanging` は 0 件。命令の後も台本が
/// 続いている間（台詞の時計を止め、資産がそろった後もフレームを回す）は差し替わらず、台詞の時計を
/// 進めて台本が終わると差し替わり、`OnShellChanged`（Ref0〜2）と `LastShell`＝`second`。
///
/// # 非空虚性
/// 待ちの依頼が台本の終わりを待たずに切れ目を返すと、時計を止めた間に差し替わって「止めている間の段」が
/// `committed` か印なしになり赤。
#[test]
fn plain_script_shell_switch_skips_changing_and_waits_for_the_script_end() {
    let mut lap = lap_rig(|| standard_script(BOOT_TO_SECOND_PLAIN).get("OnShellChanged", Ok(None)));
    let ghost_dir = lap.rig.root.ghost_dir("A");
    let steady = lap.rig.wait_steady();
    let mut log = LookLog::default();
    let windows = lap.windows.clone();

    // 台詞の始まり（両スコープの面が出た）を 1 ms ずつの Tick で見てから、受理までの台詞の時計を
    // ACCEPT の上限までしか進めない（Tick が溜まっても命令の後の待ちを食い切らない）。
    let started = lap.frames_until(CREEP, |rig| {
        log.record(look_of(rig, &windows));
        !log.distinct.is_empty()
    });
    let requested = lap.frames_until(ACCEPT, |rig| {
        log.record(look_of(rig, &windows));
        !idle(rig)
    });
    // 台詞の時計を止めたまま、資産がそろうまで回し、そろった後もさらに 20 フレーム回す。
    let built = lap.frames_until(NO_TICKS, |rig| matches!(stage(rig), Some("built") | None));
    let mut frames = 0;
    lap.frames_until(NO_TICKS, |_| {
        frames += 1;
        frames >= 20
    });
    let held = (stage(&lap.rig), got(&lap.rig, "OnShellChanged", 1));
    let (done, events) = capture(|| {
        lap.frames_until(UNBOUNDED, |rig| {
            log.record(look_of(rig, &windows));
            got(rig, "OnShellChanged", 1) && idle(rig)
        })
    });
    let last = last_shell(&lap.rig);

    let calls = calls_a(&lap.rig);
    let shutdown_ok = lap.rig.shutdown();
    assert_eq!(
        (
            (steady, started, requested, built, held, done),
            kinds(&calls),
            refs_of(&calls, "OnShellChanged"),
            last,
            done_marks(&events),
            log.summary(),
            log.blank_after_full,
            shutdown_ok,
        ),
        (
            (true, true, true, true, (Some("built"), false), true),
            vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "GET OnTranslate".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "GET OnShellChanged".to_owned(),
            ],
            vec![vec![
                SECOND.to_owned(),
                "A".to_owned(),
                abs(&ghost_dir.join("shell").join(SECOND)),
            ]],
            Some(SECOND.to_owned()),
            vec![Some("None".to_owned())],
            (
                vec![
                    (sizes(MASTER_SIZES), vec![Some(true); 2]),
                    (sizes(SECOND_SIZES), vec![Some(true); 2]),
                ],
                true,
                true
            ),
            false,
            true,
        ),
        "((定常, 台詞が始まった, 受理, 資産がそろった, 時計を止めた間の (段, OnShellChanged), 差し替わった), 呼出列, \
         OnShellChanged の Reference, LastShell, 完了の記録の印の台詞の終わり方, (見え方, 装着の子が\
         互いに素, 下端が保たれた), 子の欠けたフレーム, 降ろせた)"
    );
}

/// メニュー相当の要求（出どころ＝メニュー・フォルダ名で指す）で今のシェル自身を選ぶ（要件 1.5・1.8・
/// 11.3）: `OnShellChanging` が 1 件（Ref0・Ref1 とも今のシェルの名前・`204`＝台詞なし）届き、無視せず
/// 資産を作り直して差し替え（装着の子が新しくなり窓寸は同じ）、`OnShellChanged` と `LastShell`＝`master`。
///
/// # 非空虚性
/// 解決が今のシェルを除外するか「同じなら何もしない」に倒れると、判定が `NotFound` か差し替え 0 で赤。
#[test]
fn menu_switch_to_the_current_shell_rebuilds_and_raises_both_events() {
    let mut lap = lap_rig(|| {
        standard_script(r"\0\s[0]\1\s[10]\0A\e")
            .get("OnShellChanging", Ok(None))
            .get("OnShellChanged", Ok(None))
    });
    let master_name = shell_name(&lap.rig, "master");
    let path = abs(&lap.rig.root.ghost_dir("A").join("shell").join("master"));
    let steady = lap.rig.wait_steady();
    let mut log = LookLog::default();
    let windows = lap.windows.clone();

    let shown = lap.frames_until(UNBOUNDED, |rig| {
        log.record(look_of(rig, &windows));
        !log.distinct.is_empty()
    });
    let verdict = request_skin_switch(
        &mut lap.rig.world,
        SkinRequest {
            kind: SkinKind::Shell,
            target: SkinSpec::Folder("master".to_owned()),
            origin: SkinOrigin::Menu,
        },
    );
    let (done, events) = capture(|| {
        lap.frames_until(UNBOUNDED, |rig| {
            log.record(look_of(rig, &windows));
            got(rig, "OnShellChanged", 1) && idle(rig)
        })
    });
    let last = last_shell(&lap.rig);

    let calls = calls_a(&lap.rig);
    let shutdown_ok = lap.rig.shutdown();
    assert_eq!(
        (
            (steady, shown, verdict, done),
            kinds(&calls),
            (
                refs_of(&calls, "OnShellChanging"),
                refs_of(&calls, "OnShellChanged")
            ),
            last,
            done_marks(&events),
            log.summary(),
            log.blank_after_full,
            shutdown_ok,
        ),
        (
            (true, true, SkinVerdict::Accepted, true),
            vec![
                "NOTIFY OnInitialize".to_owned(),
                "GET username".to_owned(),
                "GET OnBoot".to_owned(),
                "GET OnTranslate".to_owned(),
                "NOTIFY basewareversion".to_owned(),
                "GET OnShellChanging".to_owned(),
                "GET OnShellChanged".to_owned(),
            ],
            (
                vec![vec![master_name.clone(), master_name.clone(), path.clone()]],
                vec![vec![master_name, "A".to_owned(), path]],
            ),
            Some("master".to_owned()),
            vec![Some("Some(NoTalk)".to_owned())],
            (
                vec![
                    (sizes(MASTER_SIZES), vec![Some(true); 2]),
                    (sizes(MASTER_SIZES), vec![Some(true); 2]),
                ],
                true,
                true
            ),
            false,
            true,
        ),
        "((定常, 面が出た, 判定, 差し替わった), 呼出列, (OnShellChanging, OnShellChanged の Reference), \
         LastShell, 完了の記録の印の台詞の終わり方, (見え方, 装着の子が互いに素, 下端が保たれた), 子の欠けたフレーム, 降ろせた)"
    );
}
