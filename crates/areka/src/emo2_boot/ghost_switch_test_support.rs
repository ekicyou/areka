//! 偽の SHIORI を持つゴーストを同じ World で起こす統合テストの土台
//! （areka-P0-ghost-shell-balloon-switch・design「Integration Tests」）。
//!
//! 一時の根（emo2 検体の複製を根にして `ghost/A`・`ghost/B` を足したもの）に
//! `ghost/A`・`ghost/B`・`ghost/emo2` と既定のバルーン `balloon/emo2-kakukaku` を並べ、
//! ゴーストのフォルダ名ごとに偽の SHIORI（台本は起こすたびに新品）を選ぶ起動入力の作り口
//! （[`GhostBootInputsSource`]）を World に据える。World は系の登録を 1 回だけ済ませ、
//! 起動の文脈（[`BootContext`]）と終了の指示の受け口（`AppExit`）を持つ。フレームは回さず、
//! 通知の相と台本の切替要求の取り出しを [`SwitchRig::pump_until`] で有界に回す。台本の中の
//! `\![change,ghost,…]` から切替を通すときは [`SwitchRig::pump_talking_until`] が、置き場の
//! ゴーストの dispatcher へ合成の Tick を注入して台詞を進め、`Input` の段（登録済みの取り出しの系）を回す。
//! 足場の待ちはどれも進みの目印（[`SwitchRig::progress_probe`]）を待ちの芯へ渡し、打ち切りを「相手が
//! 進まなかった時間」で決める（areka-P0-ghost-session-test-load-flake）。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::Location;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::{Duration, Instant};

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{BasewareRoot, ShioriWiring, TickerMode};
use areka_kanade::{
    BootOrigin, CloseReason, KanadeNotice, MonotonicMs, ShioriBackend, ShioriUnblock,
};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use sample_ghost_kit::SampleRoot;
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::AppExit;
use wintf::ecs::Input;

use super::Emo2BootInputs;
use super::frame::{KanadeNoticeRx, run_ghost_quit_phase};
use super::ghost_switch::drain_change_requests;
use super::sample_test_support::acquire_emo2;
use super::spine::{
    HOMEURL_RESOURCE, Progress, RecordedCall, RigPermit, ScriptedShioriBackend,
    ScriptedShioriBackendBuilder, ScriptedShioriHandle, WaitFailure, run_bounded_watching,
    wait_recv, wait_until, wait_until_with,
};
use crate::ConfigInputs;
use crate::boot_config::{BootContext, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{
    GhostBootInputs, GhostBootInputsSource, GhostSlot, StartupDescriptValues, boot_ghost,
    register_systems,
};
use crate::placement::AuthorDpi;

/// 既定のバルーン（emo2 の同梱）。
pub(crate) const BALLOON: &str = "emo2-kakukaku";

/// 接続に失敗する偽の SHIORI の理由。
pub(crate) const CONNECT_ERR: &str = "ghost-switch-rig simulated connect failure";

/// ゴーストごとの偽の SHIORI。
pub(crate) enum FakeShiori {
    /// 台本の組み立て（起こすたびに呼んで新品の台本を作る）。
    Scripted(Box<dyn Fn() -> ScriptedShioriBackendBuilder>),
    /// 接続に失敗する（kanade は `Fault` で止まる）。
    ConnectFail,
    /// 最初の起動だけ台本で起き、2 回目からは接続に失敗する（同じゴーストへの切替の失敗を作る）。
    ScriptedThenConnectFail(Box<dyn Fn() -> ScriptedShioriBackendBuilder>),
    /// 起動の結線が同期で成立しない（結線の入力の根が実在しない＝`boot_ghost_strict` は `Err`）。
    /// 窓の準備は構成入力の本物の根で通る。`boot_ghost` では LogSink の腕へ倒れ、倒れた先も
    /// 起点が無くて失敗する（実行系なし）。
    WiringFail,
    /// 結線は成立しないが LogSink の起動は成功する（バルーンの根が実在しない・ゴーストの根は本物）。
    /// `boot_ghost` は LogSink の腕へ倒れ、mount が通って実行系が起きる（SHIORI は使わない helper の
    /// 経路で接続に失敗し、kanade は `Fault` で止まる）。[`FakeShiori::WiringFail`] と対で、倒れた先の
    /// 成功と失敗の両方を作る。
    BalloonMissing,
    /// 台本どおりに答えるが、[`Stall`] の GET に入ったところで解かれるまで止まる。解いた後は台本どおりに
    /// 答える（`hold_at` と違って失敗を返さない＝呼び出しの並びは止めない場合と同じになる）。
    ScriptedStalled(Box<dyn Fn() -> ScriptedShioriBackendBuilder>, Stall),
}

/// 偽の SHIORI を 1 つの GET の入口で止める栓（相手が進めない状態を、時間でなく固まりで作る）。
/// 解く手 [`Stall::release`] は一度呼べば下ろさない（以後の同じ GET は止まらない）。
#[derive(Clone)]
pub(crate) struct Stall {
    at: &'static str,
    /// （入ったか・解かれたか）と、解かれたことを知らせる条件変数。
    state: Arc<(Mutex<(bool, bool)>, Condvar)>,
}

impl Stall {
    /// `id` の GET で止まる栓。
    pub(crate) fn at(id: &'static str) -> Self {
        Self {
            at: id,
            state: Arc::default(),
        }
    }

    /// 止まる GET に一度でも入ったか（後戻りしない観測）。
    pub(crate) fn entered(&self) -> bool {
        self.state.0.lock().expect("stall poisoned").0
    }

    /// 解く（止まっている GET を通し、以後は止めない）。
    pub(crate) fn release(&self) {
        self.state.0.lock().expect("stall poisoned").1 = true;
        self.state.1.notify_all();
    }

    /// GET の入口（SHIORI のアクターのスレッド）。`id` が栓の GET なら解かれるまで待つ。
    fn pass(&self, id: &str) {
        if id != self.at {
            return;
        }
        let (lock, released) = &*self.state;
        let mut state = lock.lock().expect("stall poisoned");
        state.0 = true;
        let _released = released
            .wait_while(state, |s| !s.1)
            .expect("stall poisoned");
    }
}

/// [`Stall`] を前に置いた台本の偽の SHIORI（止まるのは GET の入口だけ・ほかは台本へそのまま渡す）。
struct StalledBackend {
    inner: ScriptedShioriBackend,
    stall: Stall,
}

impl ShioriBackend for StalledBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.stall.pass(id);
        self.inner.get(id, references, status)
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.inner.notify(id, references, status)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.inner.unload()
    }

    fn status(&mut self) -> HelperStatus {
        self.inner.status()
    }

    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        self.inner.unblock_handle()
    }
}

/// 標準の台本: 起動系列（`OnInitialize`・`OnFirstBoot` 204・`OnBoot` は `on_boot`・
/// `basewareversion`）と降ろすときの `OnClose`・`Unload`。切替の握手の応答は呼び手が足す。
/// 起動記録があって `OnFirstBoot` が呼ばれなくても、使われない応答は残るだけで害は無い。
pub(crate) fn standard_script(on_boot: &str) -> ScriptedShioriBackendBuilder {
    ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(on_boot.to_owned())))
        .notify("basewareversion", Ok(()))
        .notify("OnClose", Ok(()))
        .unload(Ok(ExitKind::Clean))
}

/// 注入する Tick 1 回で台詞の時計を進める幅（合成の ms）。
const TICK_STEP_MS: u64 = 100;
/// Tick を注入する実時間の最小間隔（spin の反復ごとに投函して受信箱を溢れさせない）。
const TICK_EVERY: Duration = Duration::from_millis(1);

/// 起こしたゴースト 1 回分の記録（フォルダ名と偽の SHIORI の観測口）。
type BootLedger = Rc<RefCell<Vec<(String, ScriptedShioriHandle)>>>;

/// 2 体以上の偽ゴーストを同じ World で起こす土台。
pub(crate) struct SwitchRig {
    pub(crate) world: World,
    pub(crate) root: BasewareRoot,
    boots: BootLedger,
    /// ゴーストの作り口（[`GhostBootInputsSource`] の閉包）が呼ばれた回数（進みの目印の片方）。
    /// 台帳 `boots` の長さでは足りない: 台帳に載るのは台本つきの回だけで、`ConnectFail`・`WiringFail`・
    /// `BalloonMissing` で起こした回（切替の失敗から既定ゴーストへ戻るテスト）が入らない。
    factory_calls: Rc<Cell<u64>>,
    /// 起こす実行系に App スコープの置き場（[`SwitchRig::app_dir`]）を渡すか（既定は渡さない＝
    /// 最後に使ったゴーストと印を実行系の記憶の書き手が書かない）。[`SwitchRig::wire_app_memory`] で立てる。
    app_memory: Rc<Cell<bool>>,
    /// 注入した Tick の合成の時刻（単調増加・ゴーストをまたいでも戻さない）。
    talk_clock_ms: u64,
    /// 根の木の寿命（捨てると木が消えるので、許可の前に落ちる欄に置く）。
    sample: SampleRoot,
    /// 足場の同時の数の許可（[`RigPermit`]）。ほかの欄（World・ゴーストのスレッド・根の木）が
    /// 落ちた後に返すので欄の最後（巻き戻りでも同じ）。
    _permit: RigPermit,
}

impl SwitchRig {
    /// 根を組み、World を組み立てる（系の登録 1 回・作り口・終了の指示の受け口）。
    /// `scripts` はフォルダ名ごとの偽の SHIORI（無いフォルダを起こすと panic）。
    pub(crate) fn new(scripts: Vec<(&str, FakeShiori)>) -> Self {
        // スレッドを持つものを作る前に、待ちの関数の外で取る（許可を待つ時間は待ちの時間に数えない）。
        let permit = RigPermit::take();
        // SAFETY: 資産の焼き込み（WIC）に要る COM 初期化（既初期化の S_FALSE 等は無視）。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let sample = acquire_emo2();
        for folder in ["A", "B"] {
            copy_ghost(
                sample.folder(),
                &sample.root().join("ghost").join(folder),
                folder,
            );
        }
        let root = BasewareRoot::new(sample.root().to_path_buf());

        let mut world = World::new();
        world.init_resource::<Schedules>();
        world.insert_non_send(AppExit::new());
        let (notice_tx, notice_rx) = mpsc::channel::<KanadeNotice>();
        register_systems(&mut world, notice_rx);

        let boots: BootLedger = Rc::default();
        let scripts: HashMap<String, FakeShiori> = scripts
            .into_iter()
            .map(|(folder, fake)| (folder.to_owned(), fake))
            .collect();
        let ledger = Rc::clone(&boots);
        let factory_calls = Rc::new(Cell::new(0u64));
        let calls = Rc::clone(&factory_calls);
        let app_memory = Rc::new(Cell::new(false));
        let app_dir = app_dir_of(sample.root());
        let wired_app = Rc::clone(&app_memory);
        world.insert_non_send(GhostBootInputsSource(Box::new(
            move |cfg: &ConfigInputs, origin: BootOrigin| {
                calls.set(calls.get() + 1);
                let folder = folder_of(&cfg.ghost_root);
                let mut ghost_root = cfg.ghost_root.clone();
                let mut balloon_root = cfg.balloon_root.clone();
                let booted_before = ledger.borrow().iter().any(|(f, _)| *f == folder);
                let shiori = match scripts.get(&folder) {
                    Some(FakeShiori::ScriptedThenConnectFail(_)) if booted_before => {
                        ShioriWiring::Custom(Box::new(|| Err(CONNECT_ERR.to_owned())))
                    }
                    Some(
                        FakeShiori::Scripted(script) | FakeShiori::ScriptedThenConnectFail(script),
                    ) => {
                        let (backend, handle) = script().build();
                        ledger.borrow_mut().push((folder, handle));
                        ShioriWiring::Custom(Box::new(move || {
                            Ok(Box::new(backend) as Box<dyn areka_kanade::ShioriBackend>)
                        }))
                    }
                    Some(FakeShiori::ScriptedStalled(script, stall)) => {
                        let (backend, handle) = script().build();
                        ledger.borrow_mut().push((folder, handle));
                        let stall = stall.clone();
                        ShioriWiring::Custom(Box::new(move || {
                            Ok(Box::new(StalledBackend {
                                inner: backend,
                                stall,
                            }) as Box<dyn ShioriBackend>)
                        }))
                    }
                    Some(FakeShiori::ConnectFail) => {
                        ShioriWiring::Custom(Box::new(|| Err(CONNECT_ERR.to_owned())))
                    }
                    Some(FakeShiori::WiringFail) => {
                        ghost_root = PathBuf::from("ghost_switch_test_support/無い/ghost");
                        ShioriWiring::Custom(Box::new(|| Err(CONNECT_ERR.to_owned())))
                    }
                    Some(FakeShiori::BalloonMissing) => {
                        balloon_root = PathBuf::from("ghost_switch_test_support/無い/balloon");
                        ShioriWiring::Custom(Box::new(|| Err(CONNECT_ERR.to_owned())))
                    }
                    None => panic!("偽の SHIORI の台本が無いゴースト: {folder}"),
                };
                GhostBootInputs {
                    wiring: Emo2BootInputs {
                        ghost_root,
                        balloon_root,
                        shiori,
                        ticker: TickerMode::Disabled,
                        app_profile_dir: wired_app.get().then(|| app_dir.clone()),
                        boot_origin: origin,
                    },
                    // 結線ありの腕を通す形では、fallback に落ちると記録が残らず判定が赤になる。WiringFail／BalloonMissing はわざと倒れ、倒れた先の SHIORI はこの helper で接続に失敗する。
                    helper_exe: PathBuf::from("ghost_switch_test_support/使わない/helper.exe"),
                    kanade_stop: notice_tx.clone(),
                }
            },
        )));

        Self {
            world,
            root,
            boots,
            factory_calls,
            app_memory,
            talk_clock_ms: 0,
            sample,
            _permit: permit,
        }
    }

    /// `folder` に起動記録（`[boot] count`）を先に書く（次の起動は `OnFirstBoot` を飛ばす）。
    pub(crate) fn plant_boot_record(&self, folder: &str) {
        let dir = self
            .root
            .ghost_dir(folder)
            .join("ghost")
            .join("master")
            .join("profile")
            .join("areka");
        std::fs::create_dir_all(&dir).expect("profile/areka を組む");
        std::fs::write(
            dir.join("sylphya.toml"),
            "format-version = 1\n[boot]\ncount = \"1\"\n",
        )
        .expect("起動記録を書く");
    }

    /// 起動の文脈に据える App スコープの記憶の置き場（最後に使ったゴースト・起動中の印）。
    pub(crate) fn app_dir(&self) -> PathBuf {
        app_dir_of(self.sample.root())
    }

    /// 以後に起こす実行系へ App スコープの置き場（[`SwitchRig::app_dir`]＝起動の文脈と同じ場所）を
    /// 渡す（本番の `default_app_profile_dir()` の代わり）。起動の直後と定常到達の記憶の投函が実 fs に
    /// 届く。立てる前に起こした実行系には効かない。
    pub(crate) fn wire_app_memory(&self) {
        self.app_memory.set(true);
    }

    /// 根に既定のバルーンの複製 `balloon/<folder>` を足し、その descript.txt の `name` を `name` に
    /// 書き換える（2 つ目のバルーン）。
    pub(crate) fn add_balloon_copy(&self, folder: &str, name: &str) {
        let to = self.root.balloon_dir(folder);
        copy_tree(&self.root.balloon_dir(BALLOON), &to);
        rewrite_lines(&to.join("descript.txt"), |line| {
            if line.starts_with("name,") {
                format!("name,{name}")
            } else {
                line.to_owned()
            }
        });
    }

    /// 構成入力（ゴーストの根と既定のバルーン）。
    pub(crate) fn cfg(&self, folder: &str) -> ConfigInputs {
        ConfigInputs {
            ghost_root: self.root.ghost_dir(folder),
            balloon_root: self.root.balloon_dir(BALLOON),
        }
    }

    /// `folder` を今のゴーストとして起こす（起動の文脈を据え、作り口の入力で `boot_ghost` し、
    /// 置き場へ入れる＝`fn main` の据え付けと同じ形）。
    pub(crate) fn boot(&mut self, folder: &str) {
        let cfg = self.cfg(folder);
        let ghost = GhostDecision {
            route: GhostRoute::Memory,
            dir: cfg.ghost_root.clone(),
            folder: Some(folder.to_owned()),
        };
        let balloon = BalloonDecision {
            route: BalloonRoute::Companion,
            dir: cfg.balloon_root.clone(),
            folder: Some(BALLOON.to_owned()),
        };
        let inputs = (self.world.non_send::<GhostBootInputsSource>().0)(&cfg, BootOrigin::Plain);
        self.world.insert_resource(BootContext {
            root: self.root.clone(),
            app_profile_dir: self.app_dir(),
            helper_exe: inputs.helper_exe.clone(),
            argv_session: false,
            current: CurrentGhost {
                cfg,
                ghost: ghost.clone(),
                balloon: balloon.clone(),
            },
        });
        let descript = StartupDescriptValues {
            author_dpi: AuthorDpi::DEFAULT,
            zorder_raw: None,
        };
        let session = boot_ghost(&mut self.world, inputs, &descript, &ghost, &balloon);
        self.world.insert_non_send(GhostSlot(Some(session)));
    }

    /// `folder` を起こした回ごとの呼出列（状態の問い合わせと、定常到達で更新の窓口が飛ばす
    /// `homeurl` の照会を除く・起こした順）。照会は UI の定常到達の処理から非同期に届くので、
    /// 観測した時点で記録に載っているかが決まらない。
    pub(crate) fn calls(&self, folder: &str) -> Vec<Vec<RecordedCall>> {
        self.boots
            .borrow()
            .iter()
            .filter(|(f, _)| f == folder)
            .map(|(_, handle)| {
                handle
                    .non_status_calls()
                    .into_iter()
                    .filter(
                        |c| !matches!(c, RecordedCall::Get { id, .. } if id == HOMEURL_RESOURCE),
                    )
                    .collect()
            })
            .collect()
    }

    /// `folder` を最後に起こした回の偽の SHIORI の観測口（固まりの確かめ・解く手）。
    pub(crate) fn handle(&self, folder: &str) -> ScriptedShioriHandle {
        self.boots
            .borrow()
            .iter()
            .rev()
            .find(|(f, _)| f == folder)
            .map(|(_, handle)| handle.clone())
            .unwrap_or_else(|| panic!("{folder} を台本つきで起こしていない"))
    }

    /// 進みの目印を数える関数: 作り口が呼ばれた回数＋起こした全部の偽の SHIORI が受けた呼び出しの数
    /// （状態の問い合わせは除く＝SHIORI のアクターは手が空くと 500 ms ごとに問い合わせるので、数えると
    /// 止まっていても増え続ける）。数えと台帳の写しだけを掴み World を借りないので、条件の関数が足場を
    /// 可変で借りている間も読める（areka-P0-ghost-session-test-load-flake 要件 2.1・3.2）。
    pub(crate) fn progress_probe(&self) -> impl Fn() -> u64 + 'static {
        let (factory_calls, boots) = (Rc::clone(&self.factory_calls), Rc::clone(&self.boots));
        move || {
            let shiori_calls: u64 = boots.borrow().iter().map(|(_, h)| h.call_count()).sum();
            factory_calls.get() + shiori_calls
        }
    }

    /// 段を回す待ちの芯（時計 `now` と休み `pause` は檻のための継ぎ目）。`turn` が真になるまで回し、
    /// 打ち切りは進みの目印 [`SwitchRig::progress_probe`] で決める。「何を」は呼び出しの場所。
    #[track_caller]
    fn pump_with(
        &mut self,
        now: impl FnMut() -> Instant,
        pause: impl FnMut(Duration),
        mut turn: impl FnMut(&mut Self) -> bool,
    ) -> Result<(), WaitFailure> {
        let probe = self.progress_probe();
        wait_until_with(
            now,
            pause,
            &place(Location::caller()),
            Progress::Count(&probe),
            || turn(self),
        )
    }

    /// [`SwitchRig::pump_until`] の中身（時計と休みを差し替えられる・檻 9 が短い上限で呼ぶ）。
    #[track_caller]
    fn pump_until_with(
        &mut self,
        now: impl FnMut() -> Instant,
        pause: impl FnMut(Duration),
        mut done: impl FnMut(&Self) -> bool,
    ) -> Result<(), WaitFailure> {
        self.pump_with(now, pause, |rig| {
            run_ghost_quit_phase(&mut rig.world);
            drain_change_requests(&mut rig.world);
            done(rig)
        })
    }

    /// 通知の相と台本の切替要求の取り出しを、`done` が真になるまで有界に回す（届かなければ打ち切りの
    /// 文言を標準エラーへ 1 行出して `false`）。打ち切りは進みの目印で決める（進む限り時間だけでは打ち切らない）。
    #[track_caller]
    pub(crate) fn pump_until(&mut self, done: impl FnMut(&Self) -> bool) -> bool {
        reported(self.pump_until_with(Instant::now, std::thread::sleep, done))
    }

    /// 台詞を進めながら回す: 置き場のゴーストの dispatcher へ合成の Tick を注入し（台本の cue が
    /// 受け口へ届き、再生の完了が kanade へ届く）、通知の相と `Input` の段（`register_systems` が
    /// 登録した台本の切替要求の取り出しを含む）を `done` が真になるまで有界に回す（届かなければ
    /// [`SwitchRig::pump_until`] と同じく文言を出して `false`）。
    ///
    /// `done` には後戻りしない観測（呼び出しの記録が増えた・切替の予約が下りた、など）だけを渡す。
    /// 台詞の途中の状態を待たない: 台詞の時計は相手の処理を待たずに進むので（[`SwitchRig::inject_talk_tick`]）、
    /// 途中の状態は観測する前に過ぎ去りうる（areka-P0-ghost-session-test-load-flake 要件 2.6）。
    #[track_caller]
    pub(crate) fn pump_talking_until(&mut self, mut done: impl FnMut(&Self) -> bool) -> bool {
        let mut last_tick: Option<Instant> = None;
        reported(self.pump_with(Instant::now, std::thread::sleep, |rig| {
            if last_tick.is_none_or(|at| at.elapsed() >= TICK_EVERY) {
                last_tick = Some(Instant::now());
                rig.inject_talk_tick();
            }
            run_ghost_quit_phase(&mut rig.world);
            rig.world.run_schedule(Input);
            done(rig)
        }))
    }

    /// 足場の時刻の注入の唯一の口: 台詞の時計を [`TICK_STEP_MS`] 進め、置き場のゴーストの dispatcher へ
    /// `DispatcherMsg::Tick` を 1 つ送る。送り先は dispatcher の `Tick` だけで、kanade へは送らない
    /// （足場は `TickerMode::Disabled` で起こすので、足場の kanade は Tick を 1 つも受け取らない）。
    ///
    /// dispatcher は Tick を届いた順に消化し、再生中の台詞へ「その台詞が最初に見た Tick からの経過」と
    /// して渡すだけで、合成の締切も後戻りする状態も持たない。だから足場の時計が相手の処理より先へ
    /// 進んでも（相手が飢えていても）、増えるのは順番待ちの列だけで、相手が見る時刻の並びは変わらず、
    /// 合成の締切（kanade の `close_talk_deadline_ms` ＝ 30,000 ms）だけが切れる赤は作らない。これが
    /// 「追い越しうる時刻には頭打ちを置く」（`spine_wait.rs`）の例外である理由で、時計の先行の檻
    /// （`ghost_switch_talk_clock_tests.rs`）が固定する（areka-P0-ghost-session-test-load-flake 要件 2.6）。
    /// kanade へ時刻を届ける送り先を足すと、この理由が崩れる。
    fn inject_talk_tick(&mut self) {
        self.talk_clock_ms += TICK_STEP_MS;
        let now = MonotonicMs(self.talk_clock_ms);
        if let Some(dispatcher) = self
            .world
            .get_non_send::<GhostSlot>()
            .and_then(|slot| slot.0.as_ref())
            .and_then(|session| session.dispatcher())
        {
            // 降ろしている最中の dispatcher は閉じていてよい（次の反復で次のゴーストへ届く）。
            let _ = dispatcher.send(DispatcherMsg::Tick { now });
        }
    }

    /// 台詞の時計を進めずに回す: 通知の相と `Input` の段（登録済みの取り出しの系＝台本の切替要求と
    /// インストールの窓口の取り出し）を、`done` が真になるまで有界に回す（届かなければ
    /// [`SwitchRig::pump_until`] と同じく文言を出して `false`）。
    #[track_caller]
    pub(crate) fn pump_input_until(&mut self, mut done: impl FnMut(&Self) -> bool) -> bool {
        reported(self.pump_with(Instant::now, std::thread::sleep, |rig| {
            run_ghost_quit_phase(&mut rig.world);
            rig.world.run_schedule(Input);
            done(rig)
        }))
    }

    /// 段を回さずに `cond` だけを待つ（別スレッドの到着を読むだけの待ち・`spin_wait_until` を直に呼んで
    /// いた所の移し先）。打ち切りは進みの目印で決め、届かなければ文言を標準エラーへ 1 行出して `false`。
    #[track_caller]
    pub(crate) fn wait_for(&self, cond: impl FnMut() -> bool) -> bool {
        let probe = self.progress_probe();
        reported(wait_until(
            &place(Location::caller()),
            Progress::Count(&probe),
            cond,
        ))
    }

    /// 受け口から定常到達を 1 件待って読み捨てる（眠って待つ・進みの目印つき・届けば `true`）。
    /// 先に別の通知が届いたら、届いたものを標準エラーへ出して `false`。届かなければ打ち切りの文言を
    /// 出して `false`。台詞の時計を回す前に呼べば、台本の切替要求は必ず定常の kanade へ届く。
    #[track_caller]
    pub(crate) fn wait_steady(&self) -> bool {
        let probe = self.progress_probe();
        let rx = &self.world.non_send::<KanadeNoticeRx>().0;
        let what = place(Location::caller());
        match wait_recv(&what, Progress::Count(&probe), rx) {
            Ok(KanadeNotice::Steady) => true,
            Ok(other) => {
                eprintln!("定常到達を待っていた「{what}」に、先に別の通知が届いた: {other:?}");
                false
            }
            Err(failure) => reported(Err(failure)),
        }
    }

    /// 終了が指示されたか。
    pub(crate) fn exit_requested(&self) -> bool {
        self.world.non_send::<AppExit>().is_requested()
    }

    /// 置き場のゴーストを有界に降ろす（成功で `true`・置き場が空でも `true`）。
    pub(crate) fn shutdown(&mut self) -> bool {
        let Some(session) = self
            .world
            .get_non_send_mut::<GhostSlot>()
            .and_then(|mut slot| slot.0.take())
        else {
            return true;
        };
        let (tx, rx) = mpsc::channel();
        // 降ろしの OnClose・Unload が目印を進める。進んでいる限り負荷で遅いだけでは打ち切らない。
        let probe = self.progress_probe();
        run_bounded_watching(
            "置き場のゴーストを降ろす",
            Progress::Count(&probe),
            move || {
                let _ = tx.send(session.shutdown(CloseReason::User { scope: 0 }).is_ok());
            },
        );
        rx.recv().unwrap_or(false)
    }
}

/// 待ちの「何を」に使う呼び出しの場所（ファイルと行）。
fn place(at: &Location<'_>) -> String {
    format!("{}:{}", at.file(), at.line())
}

/// 待ちの結果を `bool` へ畳む。打ち切りなら文言を標準エラーへ 1 行出して `false`（黙って `false` にしない）。
fn reported(result: Result<(), WaitFailure>) -> bool {
    match result {
        Ok(()) => true,
        Err(failure) => {
            eprintln!("{failure}");
            false
        }
    }
}

/// 根の下の App スコープの置き場。
fn app_dir_of(root: &Path) -> PathBuf {
    root.join("profile")
}

/// ゴーストの根のフォルダ名。
fn folder_of(ghost_root: &Path) -> String {
    ghost_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// 検体のゴーストを `to` へ複製し、`descript.txt` の `name` をフォルダ名に、`sakura.name` を
/// 「フォルダ名のさくら」に書き換える（切替先の名前と本体側の名前を体ごとに見分ける）。
fn copy_ghost(from: &Path, to: &Path, folder: &str) {
    copy_tree(from, to);
    rewrite_lines(
        &to.join("ghost").join("master").join("descript.txt"),
        |line| {
            if line.starts_with("name,") {
                format!("name,{folder}")
            } else if line.starts_with("sakura.name,") {
                format!("sakura.name,{folder}のさくら")
            } else {
                line.to_owned()
            }
        },
    );
}

/// フォルダ `from` を `to` へ再帰で複製する。
fn copy_tree(from: &Path, to: &Path) {
    let mut stack = vec![(from.to_path_buf(), to.to_path_buf())];
    while let Some((src, dst)) = stack.pop() {
        std::fs::create_dir_all(&dst).expect("複製のフォルダを作る");
        for entry in std::fs::read_dir(&src).expect("検体を走査する") {
            let entry = entry.expect("検体の要素");
            let target = dst.join(entry.file_name());
            if entry.file_type().expect("要素の種別").is_dir() {
                stack.push((entry.path(), target));
            } else {
                std::fs::copy(entry.path(), &target).expect("検体のファイルを写す");
            }
        }
    }
}

/// UTF-8 のテキスト `path` の各行を `map` で書き換え、CRLF でつないで書き戻す。
fn rewrite_lines(path: &Path, map: impl Fn(&str) -> String) {
    let text = std::fs::read_to_string(path).expect("テキストは UTF-8");
    let rewritten: Vec<String> = text.lines().map(map).collect();
    std::fs::write(path, rewritten.join("\r\n")).expect("テキストを書き換える");
}

// 足場の待ちの檻（areka-P0-ghost-session-test-load-flake タスク 3.2・檻 9・10）。
#[path = "ghost_switch_rig_wait_tests.rs"]
mod rig_wait_tests;

// 時計の先行の檻（areka-P0-ghost-session-test-load-flake タスク 3.3・檻 8・要件 2.6）。
#[path = "ghost_switch_talk_clock_tests.rs"]
mod talk_clock_tests;
