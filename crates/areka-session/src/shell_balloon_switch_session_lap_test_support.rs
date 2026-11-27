//! シェルの往復の統合テストの土台（窓・GPU つきの往復の足場と呼出列の読み口）。

use std::cell::Cell;
use std::panic::Location;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use bevy_ecs::world::World;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
};
use wintf::ecs::{DPI, FrameTime, GraphicsCore, Input, Update, WindowHandle, WucGraphicsResource};

use crate::ghost_switch::SwitchInFlight;
use crate::ghost_switch_test_support::{BALLOON, SwitchRig, TICK_EVERY, copy_tree};
use areka_placement::follow::MonitorSnapshot;
use areka_placement::resolver::RectPx;
use areka_placement::spawn::{GhostWindows, spawn_ghost_windows};
use areka_runtime::frame::RuntimeWiring;
use areka_runtime::skin_switch::SkinSwitchInFlight;
use areka_runtime::spine::{GpuPermit, Progress, RecordedCall, wait_until};

/// 2 つ目のシェルのフォルダ名（`descript.txt` の `name` も同じ）。
pub const SECOND: &str = "second";

/// 合成の作業領域（配置の準備と窓の追従の両方が読む）。
const WORK_AREA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 3840,
    bottom: 2100,
};
/// 1 回の待ちの中で注入する Tick（1 回で台詞の時計を進める幅と、注入の回数の上限）。
///
/// 投函した Tick は talk スレッドが遅れると溜まり、後から一度に消化される。台詞の時計を観測より
/// 先へ進めてはならない場面では、幅と回数で 1 回の待ちの進み幅を抑える。
#[derive(Clone, Copy)]
pub struct Ticks {
    step_ms: u64,
    max: usize,
}

/// Tick を注入しない（台詞の時計を止めたまま回す）。
#[cfg(test)]
pub(crate) const NO_TICKS: Ticks = Ticks { step_ms: 0, max: 0 };
/// 100 ms ずつ・上限なし（進み方を問わない場面）。
pub const UNBOUNDED: Ticks = Ticks {
    step_ms: 100,
    max: usize::MAX,
};
/// 1 ms ずつ・上限なし（台詞の始まりを見るまで。台詞の開始は kanade から中継を経て dispatcher へ
/// 届くので Tick より遅れうる。始まる前の Tick は捨てられ、始まった後に溜まった Tick も 1 回 1 ms
/// しか進めない）。
#[cfg(test)]
pub(crate) const CREEP: Ticks = Ticks {
    step_ms: 1,
    max: usize::MAX,
};
/// 台詞の始まりを見た後、受理までに注入する Tick（100 ms × 20 回＝台詞の時計で高々 2,000 ms。
/// 命令の位置〔台本の頭から数十 ms〕には届き、命令の後の待ち 5,000 ms は食い切らない）。
#[cfg(test)]
pub(crate) const ACCEPT: Ticks = Ticks {
    step_ms: 100,
    max: 20,
};

/// 窓・GPU つきの往復の土台。
pub struct LapRig {
    pub rig: SwitchRig,
    pub windows: GhostWindows,
    /// UI の側の要約が巡をまたいで変わった回数（[`LapRig::frames_until`] の進みの目印の片方）。
    ui_moves: Rc<Cell<u64>>,
    /// 前の巡に見た UI の側の要約。
    ui_seen: UiSeen,
    /// GPU の装置の許可（[`GpuPermit`]）。`rig` の World の装置より後に返すので欄の最後。
    _gpu: GpuPermit,
}

/// 呼び手が組んだ土台の A に、`shells_from` の下の `shells` の各フォルダを `shell/<フォルダ>` へ写し、
/// 窓の一式と GPU 資源つきで A を起こす。
pub fn lap_rig_on(mut rig: SwitchRig, shells_from: &Path, shells: &[&str]) -> LapRig {
    rig.plant_boot_record("A");
    for shell in shells {
        copy_tree(
            &shells_from.join(shell),
            &rig.root.ghost_dir("A").join("shell").join(shell),
            None,
        );
    }
    let windows = spawn_windows(&mut rig, 96);
    let gpu = GpuPermit::take();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    rig.world.insert_resource(core);
    rig.world.insert_resource(wuc);
    rig.wire_app_memory();
    rig.boot("A");
    let mut ui_seen = UiSeen::default();
    ui_seen.observe(&rig.world, &Cell::new(0));
    LapRig {
        rig,
        windows,
        ui_moves: Rc::default(),
        ui_seen,
        _gpu: gpu,
    }
}

/// UI の側の進みの要約。どの欄も増えるだけの数にする
/// （行ったり来たりする見た目の状態を入れると、止まっていても進みに数えてしまう）。
#[derive(Debug, Default, Clone, PartialEq)]
struct UiMarks {
    /// 文字の層の普通のバルーンの場所ごとの（消された回数, 載った項目の数）。台詞の起動は冒頭の
    /// 全消去で前者を、台詞の表示は後者を増やす（組は辞書順で増えるだけ）。
    text: Vec<(u64, usize)>,
    /// 切り替えの予約（シェル・ゴースト）が下りた回数。取りやめの記録と定常の知らせ（迎え入れの
    /// 終わり）は予約を下ろす。記録の数を捕まえの途中で読む口が無いので、World の予約で数える。
    released: u64,
}

/// 前の巡に見た UI の側の要約と、切り替えの予約（シェル・ゴースト）が在ったか。
#[derive(Default)]
pub(crate) struct UiSeen {
    marks: UiMarks,
    held: [bool; 2],
}

impl UiSeen {
    /// 今の巡の観測（文字の層の要約・予約の有無）を畳み、要約が前の巡と違えば `moves` を 1 増やす。
    pub(crate) fn note(&mut self, text: Vec<(u64, usize)>, held: [bool; 2], moves: &Cell<u64>) {
        let released = self
            .held
            .iter()
            .zip(&held)
            .filter(|(was, now)| **was && !**now);
        let marks = UiMarks {
            text,
            released: self.marks.released + released.count() as u64,
        };
        self.held = held;
        if marks != self.marks {
            self.marks = marks;
            moves.set(moves.get() + 1);
        }
    }

    /// World から今の巡の観測を読んで [`UiSeen::note`] へ渡す。
    fn observe(&mut self, world: &World, moves: &Cell<u64>) {
        let text = world
            .get_non_send::<RuntimeWiring>()
            .map(|wiring| {
                let runtime = wiring.runtime().borrow();
                let state = runtime.state();
                state
                    .actors()
                    .map(|(actor, place)| (state.clear_count(actor), place.items().len()))
                    .collect()
            })
            .unwrap_or_default();
        let held = [
            world.get_non_send::<SkinSwitchInFlight>().is_some(),
            world.get_non_send::<SwitchInFlight>().is_some(),
        ];
        self.note(text, held, moves);
    }
}

/// A の起動の配置（作業領域だけ合成・作者 DPI 96）で 2 スコープの窓を生やし、偽 HWND と `dpi` を
/// 持たせる（`WindowPos` への書込の口が通る・実窓には触れない）。
pub fn spawn_windows(rig: &mut SwitchRig, dpi: u16) -> GhostWindows {
    let prepared = areka_placement::prepare_ghost_windows_with_work_area(
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
                .insert(DPI::from_dpi(dpi, dpi));
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
    /// 芯の待ちへ渡す。打ち切りは目印で決め、打ち切ったら呼び出しの場所を添えた文言を標準エラーへ
    /// 1 行出して `false`。合成の時刻の送り先は dispatcher の `DispatcherMsg::Tick` だけ。
    ///
    /// 目印は SHIORI の側の数（`SwitchRig::progress_probe`）＋UI の側の要約が変わった回数
    /// （`ui_moves`・巡を回した後に読む）。台詞の起動・表示・取りやめ・定常の知らせのように SHIORI を
    /// 呼ばずに進む仕事も進みに数え、負荷で遅いだけでは `［止まった］` で打ち切らない。
    #[track_caller]
    pub fn frames_until(&mut self, ticks: Ticks, mut done: impl FnMut(&SwitchRig) -> bool) -> bool {
        let Self {
            rig,
            ui_moves,
            ui_seen,
            ..
        } = self;
        let mut last_tick: Option<Instant> = None;
        let mut sent = 0;
        let (shiori, moves) = (rig.progress_probe(), Rc::clone(ui_moves));
        let probe = move || shiori() + moves.get();
        let caller = Location::caller();
        let waited = wait_until(
            &format!("{}:{}", caller.file(), caller.line()),
            Progress::Count(&probe),
            || {
                if sent < ticks.max && last_tick.is_none_or(|at| at.elapsed() >= TICK_EVERY) {
                    sent += 1;
                    last_tick = Some(Instant::now());
                    rig.inject_talk_tick(ticks.step_ms);
                }
                // 台詞の文字の現れとバルーンの表示を UI が決める時刻。
                let now = rig.frame_time();
                rig.world.insert_resource(FrameTime(now));
                pump_messages();
                rig.world.run_schedule(Input);
                rig.world.run_schedule(Update);
                ui_seen.observe(&rig.world, ui_moves);
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
pub fn pump_messages() {
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

/// 呼出列の種別（`GET OnBoot` の形）。
pub fn kinds(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|c| match c {
            RecordedCall::Get { id, .. } => format!("GET {id}"),
            RecordedCall::Notify { id, .. } => format!("NOTIFY {id}"),
            RecordedCall::Unload => "UNLOAD".to_owned(),
        })
        .collect()
}

/// 呼出列から `id` の GET の Reference を順に取る。
pub fn refs_of(calls: &[RecordedCall], id: &str) -> Vec<Vec<String>> {
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
pub fn calls_a(rig: &SwitchRig) -> Vec<RecordedCall> {
    rig.calls("A").into_iter().next().unwrap_or_default()
}

/// `id` の GET が `n` 件以上届いたか。
pub fn got(rig: &SwitchRig, id: &str, n: usize) -> bool {
    refs_of(&calls_a(rig), id).len() >= n
}

/// 置き場のゴーストの今のシェルのフォルダ名。
pub fn current_shell(rig: &SwitchRig) -> Option<String> {
    rig.world
        .get_non_send::<crate::ghost_session::GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.current_shell_folder())
}

/// 切替の進行中の印が無い。
pub fn idle(rig: &SwitchRig) -> bool {
    rig.world.get_non_send::<SkinSwitchInFlight>().is_none()
}
