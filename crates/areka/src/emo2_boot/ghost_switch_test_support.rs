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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{BasewareRoot, ShioriWiring, TickerMode};
use areka_kanade::{BootOrigin, CloseReason, KanadeNotice, MonotonicMs};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use sample_ghost_kit::SampleRoot;
use shiori_host32_host::ExitKind;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::AppExit;
use wintf::ecs::Input;

use super::Emo2BootInputs;
use super::frame::{KanadeNoticeRx, run_ghost_quit_phase};
use super::ghost_switch::drain_change_requests;
use super::sample_test_support::acquire_emo2;
use super::spine::{
    RecordedCall, ScriptedShioriBackend, ScriptedShioriBackendBuilder, ScriptedShioriHandle,
    run_bounded, spin_wait_until,
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
    /// 起動の結線が同期で成立しない（結線の入力の根が実在しない＝`boot_ghost_strict` は `Err`）。
    /// 窓の準備は構成入力の本物の根で通る。
    WiringFail,
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
    /// 起こす実行系に App スコープの置き場（[`SwitchRig::app_dir`]）を渡すか（既定は渡さない＝
    /// 最後に使ったゴーストと印を実行系の記憶の書き手が書かない）。[`SwitchRig::wire_app_memory`] で立てる。
    app_memory: Rc<Cell<bool>>,
    /// 注入した Tick の合成の時刻（単調増加・ゴーストをまたいでも戻さない）。
    talk_clock_ms: u64,
    /// 根の木の寿命（捨てると木が消えるので最後に落ちる欄に置く）。
    sample: SampleRoot,
}

impl SwitchRig {
    /// 根を組み、World を組み立てる（系の登録 1 回・作り口・終了の指示の受け口）。
    /// `scripts` はフォルダ名ごとの偽の SHIORI（無いフォルダを起こすと panic）。
    pub(crate) fn new(scripts: Vec<(&str, FakeShiori)>) -> Self {
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
        let app_memory = Rc::new(Cell::new(false));
        let app_dir = app_dir_of(sample.root());
        let wired_app = Rc::clone(&app_memory);
        world.insert_non_send(GhostBootInputsSource(Box::new(
            move |cfg: &ConfigInputs, origin: BootOrigin| {
                let folder = folder_of(&cfg.ghost_root);
                let mut ghost_root = cfg.ghost_root.clone();
                let shiori = match scripts.get(&folder) {
                    Some(FakeShiori::Scripted(script)) => {
                        let (backend, handle) = script().build();
                        ledger.borrow_mut().push((folder, handle));
                        ShioriWiring::Custom(Box::new(move || {
                            Ok(Box::new(backend) as Box<dyn areka_kanade::ShioriBackend>)
                        }))
                    }
                    Some(FakeShiori::ConnectFail) => {
                        ShioriWiring::Custom(Box::new(|| Err(CONNECT_ERR.to_owned())))
                    }
                    Some(FakeShiori::WiringFail) => {
                        ghost_root = PathBuf::from("ghost_switch_test_support/無い/ghost");
                        ShioriWiring::Custom(Box::new(|| Err(CONNECT_ERR.to_owned())))
                    }
                    None => panic!("偽の SHIORI の台本が無いゴースト: {folder}"),
                };
                GhostBootInputs {
                    wiring: Emo2BootInputs {
                        ghost_root,
                        balloon_root: cfg.balloon_root.clone(),
                        shiori,
                        ticker: TickerMode::Disabled,
                        app_profile_dir: wired_app.get().then(|| app_dir.clone()),
                        boot_origin: origin,
                    },
                    // 結線ありの腕だけを通す（fallback に落ちると記録が残らず判定が赤になる）。
                    helper_exe: PathBuf::from("ghost_switch_test_support/使わない/helper.exe"),
                    kanade_stop: notice_tx.clone(),
                }
            },
        )));

        Self {
            world,
            root,
            boots,
            app_memory,
            talk_clock_ms: 0,
            sample,
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

    /// `folder` を起こした回ごとの呼出列（状態の問い合わせを除く・起こした順）。
    pub(crate) fn calls(&self, folder: &str) -> Vec<Vec<RecordedCall>> {
        self.boots
            .borrow()
            .iter()
            .filter(|(f, _)| f == folder)
            .map(|(_, handle)| handle.non_status_calls())
            .collect()
    }

    /// 通知の相と台本の切替要求の取り出しを、`done` が真になるまで有界に回す（期限切れは `false`）。
    pub(crate) fn pump_until(&mut self, mut done: impl FnMut(&Self) -> bool) -> bool {
        spin_wait_until(|| {
            run_ghost_quit_phase(&mut self.world);
            drain_change_requests(&mut self.world);
            done(self)
        })
    }

    /// 台詞を進めながら回す: 置き場のゴーストの dispatcher へ合成の Tick を注入し（台本の cue が
    /// 受け口へ届き、再生の完了が kanade へ届く）、通知の相と `Input` の段（`register_systems` が
    /// 登録した台本の切替要求の取り出しを含む）を `done` が真になるまで有界に回す（期限切れは `false`）。
    pub(crate) fn pump_talking_until(&mut self, mut done: impl FnMut(&Self) -> bool) -> bool {
        let mut last_tick: Option<Instant> = None;
        spin_wait_until(|| {
            if last_tick.is_none_or(|at| at.elapsed() >= TICK_EVERY) {
                last_tick = Some(Instant::now());
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
            run_ghost_quit_phase(&mut self.world);
            self.world.run_schedule(Input);
            done(self)
        })
    }

    /// 受け口から定常到達を 1 件待って読み捨てる（有界・届けば `true`・先に別の通知が届いたら
    /// `false`）。台詞の時計を回す前に呼べば、台本の切替要求は必ず定常の kanade へ届く。
    pub(crate) fn wait_steady(&self) -> bool {
        let rx = &self.world.non_send::<KanadeNoticeRx>().0;
        matches!(
            rx.recv_timeout(Duration::from_secs(20)),
            Ok(KanadeNotice::Steady)
        )
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
        run_bounded(
            "置き場のゴーストを降ろす",
            Duration::from_secs(20),
            move || {
                let _ = tx.send(session.shutdown(CloseReason::User { scope: 0 }).is_ok());
            },
        );
        rx.recv().unwrap_or(false)
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
    let descript = to.join("ghost").join("master").join("descript.txt");
    let text = std::fs::read_to_string(&descript).expect("descript.txt は UTF-8");
    let rewritten: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with("name,") {
                format!("name,{folder}")
            } else if line.starts_with("sakura.name,") {
                format!("sakura.name,{folder}のさくら")
            } else {
                line.to_owned()
            }
        })
        .collect();
    std::fs::write(&descript, rewritten.join("\r\n")).expect("descript.txt を書き換える");
}
