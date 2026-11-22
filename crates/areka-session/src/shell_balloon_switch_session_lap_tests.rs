//! シェルの往復の統合テスト。
//!
//! 偽の SHIORI の土台（[`SwitchRig`]・emo2 の複製のゴースト A）に、`R_POST_and_KOMAINU` のシェルを
//! [`SampleRoot::add_shell_copy`] で `second` と名付けて写したものを A の 2 つ目のシェルとして足す。
//! 2 つのシェルは絵の大きさが違うので、差し替えで窓寸が変わる。同じ World に GPU 資源（実 GPU）と、
//! 本物の配置の準備（作業領域だけ合成）で組んだ窓の一式（偽 HWND）を据え、本番の `Input`・`Update` の
//! 段（`frame_system`）をそのまま回す。台詞は置き場のゴーストの dispatcher へ合成の Tick を
//! 注入して進める（止めれば台詞の時計は進まない）。判定は集めてから 1 回・降ろすのは有界に行う。

use std::cell::Cell;
use std::collections::BTreeSet;
use std::path::Path;

use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::Children;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::SampleRoot;
use wintf::ecs::WindowPos;

use super::GhostSlot;
pub(crate) use super::shell_balloon_switch_session_lap_test_support::*;
use crate::boot_resolve::read_last_shell;
use crate::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::shell_balloon_switch::{
    SkinKind, SkinOrigin, SkinRequest, SkinSpec, SkinVerdict, request_skin_switch,
};
use areka_input::user_break::UserBreakWiring;
use areka_placement::spawn::GhostWindows;
use areka_runtime::frame::RuntimeWiring;
use areka_runtime::skin_switch::{SkinSwitchInFlight, SkinSwitchStage};
use areka_runtime::spine::ScriptedShioriBackendBuilder;
use areka_runtime::target_map::shell_target;

/// 起動のシェル（emo2）の面 0・面 10 の素の大きさ。
const MASTER_SIZES: [(i32, i32); 2] = [(434, 687), (336, 400)];
/// 2 つ目のシェル（`R_POST_and_KOMAINU`）の面 0・面 10 の素の大きさ。
const SECOND_SIZES: [(i32, i32); 2] = [(236, 462), (140, 160)];

// ---------------------------------------------------------------- 土台

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
    // 検体の窓口は `test-support` に入れられないので、写す元のシェルは呼び手が作る。
    let rpost = SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済みの検体");
    for shell in shells {
        rpost
            .add_shell_copy("master", shell, shell)
            .expect("2 つ目のシェルを写す");
    }
    lap_rig_on(
        SwitchRig::new(ghosts),
        &rpost.folder().join("shell"),
        shells,
    )
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

/// 置き場のゴーストの記憶の書き手へ柵を掛ける（書き終えた記憶を実 fs から読めるようにする）。
pub(super) fn fence(rig: &SwitchRig) {
    let fenced = rig
        .world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime())
        .is_some_and(|runtime| runtime.sylphya_publisher().barrier().is_ok());
    assert!(fenced, "記憶の書き手へ柵を掛けられない");
}

/// 柵を掛けてから、A の `LastShell` を実 fs から読む。
pub(super) fn last_shell(rig: &SwitchRig) -> Option<String> {
    fence(rig);
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

/// 往復: A（`master`）→ 台本の `\![change,shell,second,--option=raise-event]` →
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
    // フレームの境目に来るとは限らない。途中の記憶は投函の記録の並びで見る。
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
    let presenter = world.get_non_send::<RuntimeWiring>().map(|w| w.presenter());
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

/// `raise-event` 無しの台本の切替: `OnShellChanging` は 0 件。命令の後も台本が
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

    // Tick が溜まっても命令の後の待ちを食い切らないよう、受理までは ACCEPT の上限までしか進めない。
    let started = lap.frames_until(CREEP, |rig| {
        log.record(look_of(rig, &windows));
        !log.distinct.is_empty()
    });
    let requested = lap.frames_until(ACCEPT, |rig| {
        log.record(look_of(rig, &windows));
        !idle(rig)
    });
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

/// メニュー相当の要求（出どころ＝メニュー・フォルダ名で指す）で今のシェル自身を選ぶ: `OnShellChanging` が 1 件（Ref0・Ref1 とも今のシェルの名前・`204`＝台詞なし）届き、無視せず
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

// ---------------------------------------------------------------- 進みの目印の檻

/// UI の側の要約の畳み: 文字の層の要約が前の巡と違えば、
/// または切り替えの予約が下りれば（取りやめ・定常の知らせ）1 増える。前の巡と同じ・予約が立った
/// だけ・在り続ける・無いままでは増えない。
///
/// # 非空虚性
/// 畳みが要約を比べずに毎巡足すと、止まっている巡でも増えて赤。予約が下りたことを数えないと、
/// 取りやめや定常の知らせだけが進んだ巡で増えず赤。
#[test]
fn ui_seen_counts_only_ui_side_progress() {
    let moves = Cell::new(0);
    let mut seen = UiSeen::default();
    let mut step = |text: Vec<(u64, usize)>, held: [bool; 2]| {
        seen.note(text, held, &moves);
        moves.get()
    };
    let got = [
        step(Vec::new(), [false, false]),
        step(vec![(1, 0)], [false, false]),
        step(vec![(1, 3)], [false, false]),
        step(vec![(1, 3)], [false, false]),
        step(vec![(1, 3)], [true, false]),
        step(vec![(1, 3)], [true, false]),
        step(vec![(1, 3)], [false, false]),
        step(vec![(1, 3)], [false, true]),
        step(vec![(1, 3)], [false, false]),
        step(vec![(2, 0), (1, 0)], [false, false]),
    ];
    assert_eq!(
        got,
        [0, 1, 2, 2, 2, 2, 3, 3, 4, 5],
        "(初め, 台詞の起動, 台詞の表示, 止まった, 予約が立った, 在り続ける, 取りやめで下りた, \
         ゴースト切替の予約が立った, 定常の知らせで下りた, 次の台詞の起動)"
    );
}
