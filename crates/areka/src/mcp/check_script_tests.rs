//! 写し取りの値の決定論テスト（4.2）。写し取りの値が判断の事実の trait を満たし、手で組んだ事実と
//! 同じ答えを返すこと（同じ台本 → 同じ診断）を確かめる。
//!
//! 後半は解決の失敗と「何もさせない」の檻（5.4）。`ghost_name` を 10 本と同じ規則で解き、
//! 偽の SHIORI で起こした本物の単位へ台本を 2 件渡しても、SHIORI への呼出・再生・終了・
//! 切替のどれも起きないことを確かめる。

use std::collections::{BTreeMap, BTreeSet};

use areka_mcp::tools::check_script::Diagnostic;
use areka_parsers::sakura::parse_noted;
use areka_seriko::{SurfaceResolver, SurfaceTarget};

use super::*;
use crate::emo2_boot::consumer_ledger::ConsumerLedger;
use crate::mcp::check_script_judge::{ScriptFacts, diagnose};

/// 手で組んだ事実。スコープ 0 の絵は 0・3・10、スコープ 1 は 10・11、別名 `笑顔` は 3。
/// バルーンはスコープ 0 が 0・2、スコープ 1 が 1。スコープ 2 には何も無い。
struct ByHand(SurfaceResolver);

impl ScriptFacts for ByHand {
    fn resolve_surface(&self, key: &str) -> SurfaceTarget {
        self.0.resolve(key)
    }
    fn shell_has(&self, scope: u32, surface_id: u32) -> Option<bool> {
        match scope {
            0 => Some([0, 3, 10].contains(&surface_id)),
            1 => Some([10, 11].contains(&surface_id)),
            _ => None,
        }
    }
    fn balloon_has(&self, scope: u32, balloon_id: u32) -> Option<bool> {
        match scope {
            0 => Some([0, 2].contains(&balloon_id)),
            1 => Some(balloon_id == 1),
            _ => None,
        }
    }
}

fn aliases() -> BTreeMap<String, Vec<u32>> {
    BTreeMap::from([("笑顔".to_string(), vec![3])])
}

/// 上と同じ事実を、写し取りの値の形（集合の表）で組む。
fn snapshot() -> ScriptFactsSnapshot {
    let ids = |v: &[u32]| v.iter().copied().collect::<BTreeSet<u32>>();
    ScriptFactsSnapshot::new(
        aliases(),
        BTreeMap::from([(0, ids(&[0, 3, 10])), (1, ids(&[10, 11]))]),
        BTreeMap::from([(0, ids(&[0, 2])), (1, ids(&[1]))]),
    )
}

fn check(script: &str, facts: &dyn ScriptFacts) -> Vec<Diagnostic> {
    diagnose(
        script,
        &parse_noted(script),
        &ConsumerLedger::canonical(),
        Some(facts),
    )
}

#[test]
fn snapshot_answers_like_facts_built_by_hand() {
    let by_hand = ByHand(SurfaceResolver::new(aliases()));
    let snap = snapshot();
    let scripts = [
        r"\s[0]\s[3]\s[笑顔]\s[99999]\s[無い名前]\s[-1]",
        r"\b[0]\b[2]\b[5]\b[-1]\b[名前]",
        r"\1\s[10]\s[0]\b[1]\b[0]",
        r"\p[2]\s[12345]\b[9]",
        r"\0\s[11]\![change,shell,x]\s[99999]",
        r"\x\![nope]\f[sub,1]\s[99999]\e\b[7]",
    ];
    let mut fired = 0;
    for script in scripts {
        let want = check(script, &by_hand);
        assert_eq!(check(script, &snap), want, "{script}");
        fired += want.len();
    }
    // 比べた台本が何も診断しない組で素通りしていないこと（較正）。
    assert!(fired >= 8, "too few diagnostics to compare: {fired}");
}

#[test]
fn snapshot_without_a_scope_does_not_judge_it() {
    let snap = snapshot();
    assert_eq!(snap.shell_has(2, 0), None);
    assert_eq!(snap.balloon_has(2, 0), None);
    assert_eq!(snap.shell_has(0, 3), Some(true));
    assert_eq!(snap.shell_has(1, 3), Some(false));
}

// ---- 解決と振り分け（5.4・要件 2.2・2.3） ----

use areka_mcp::tools::{ToolCall, ToolRequest, outcome};

use crate::mcp::dump_surface::WaitAnswer;
use crate::mcp::resolve::{CANNOT_FIND, NOT_ACTIVE};

/// 窓の無いゴーストの答え（surface とバルーンを診ないと添える）。
fn no_window_answer(count: usize) -> areka_mcp::ToolOutcome {
    outcome::ok(&format!("{count} diagnostics ({NO_WINDOW_NOTE})"))
}

fn call(script: &str, ghost_name: Option<&str>) -> ToolCall {
    ToolCall::CheckScript(Args {
        script: script.to_owned(),
        ghost_name: ghost_name.map(str::to_owned),
    })
}

fn emily() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: None,
        root: std::path::PathBuf::from(r"C:\ssp\ghost\emily4"),
    }
}

/// 空の World で振り分ける。（その場の答え, 上限つきで待った答え）。答えは別のスレッドからも届く。
fn dispatch_and_wait(
    active: Option<&ActiveGhost>,
    ghost_name: Option<&str>,
) -> (
    Option<areka_mcp::ToolOutcome>,
    Option<(String, areka_mcp::ToolOutcome)>,
) {
    let (request, pending) = ToolRequest::new(call(r"\0\s[0]こんにちは\e", ghost_name));
    crate::mcp::dispatch(&mut World::new(), active, request);
    let at_once = pending.try_answer().ok().flatten().map(|a| a.outcome);
    let waited = match &at_once {
        Some(_) => None,
        None => pending.wait_answer().map(|a| (a.ghost, a.outcome)),
    };
    (at_once, waited)
}

/// ゴーストが居ないとき、名前を省くと `NG:Specified ghost is not active` をその場で答える
/// （名前を渡すと 10 本と同じく `CANNOT_FIND`＝解決の規則のまま）。
#[test]
fn no_active_ghost_is_not_active() {
    assert_eq!(
        dispatch_and_wait(None, None),
        (Some(outcome::ng(NOT_ACTIVE)), None)
    );
    assert_eq!(
        dispatch_and_wait(None, Some("Emily/Phase4.5")),
        (Some(outcome::ng(CANNOT_FIND)), None)
    );
}

/// 違う名前は `NG:Cannot find active ghost from specified name` をその場で答える（判断へ進まない）。
#[test]
fn a_wrong_name_cannot_find() {
    let g = emily();
    assert_eq!(
        dispatch_and_wait(Some(&g), Some("Someone else")),
        (Some(outcome::ng(CANNOT_FIND)), None)
    );
}

/// 省略と当たる名前は起動中の 1 体へ解決し、別のスレッドからそのゴーストの名前を添えて答える
/// （空の World＝窓が無いので surface とバルーンは診ない）。
///
/// # 非空虚性
/// 省略を断ると、その場の答えが `NG:` で赤。答えが届かなければ待った答えが `None` で赤。
#[test]
fn omitted_or_matching_name_answers_with_the_running_ghost() {
    let g = emily();
    for name in [None, Some("Emily/Phase4.5")] {
        assert_eq!(
            dispatch_and_wait(Some(&g), name),
            (
                None,
                Some(("Emily/Phase4.5".to_owned(), no_window_answer(0)))
            ),
            "{name:?}"
        );
    }
}

// ---- 何もさせない（5.4・要件 2.4・5.2・偽の SHIORI で起こした本物の単位） ----

use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::ghost_session::GhostSlot;

/// kanade に今の実行の状態を問い、台詞の再生中かを答える。問いは受信の順に処理されるので、先に送られた
/// 依頼が済むまで待つ口にもなる（状態の問い合わせは SHIORI を呼ばない・呼出の記録からも除かれる）。
fn talking(world: &World) -> bool {
    let kanade = world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|session| session.kanade())
        .expect("起動したゴーストには kanade の送り口がある")
        .clone();
    let (reply, rx) = areka_actor::reply_channel();
    kanade
        .send(areka_kanade::KanadeMsg::StatusQuery { reply })
        .expect("kanade に届く");
    rx.recv_timeout(std::time::Duration::from_secs(20))
        .expect("状態の返事が届く")
        .render()
        .is_some_and(|states| states.split(',').any(|s| s == "talking"))
}

/// 偽の SHIORI で A を起こし（切替先の B も据える）、受け口を置いて、起動の台詞を言い終えるまで回す
/// （再生中の kanade は切替や終了の求めを後回しにするので、言い終えてからでないと副作用が見えない）。
/// `\![change,ghost,B]`・`\-`・文字を含む台本を 2 件続けて送り、`Input` の段を 1 回回して答えを上限
/// つきで待つと、2 件とも窓の無いゴーストの答えで、A の SHIORI の呼出の記録は増えず、再生も始まらず、
/// B は起きず、終了も切替も指示されていない。最後に A を降ろせる＝起きたまま
/// （get_expression_table の `real_unit_answers_from_the_current_shell_each_call_without_side_effects`
/// と同じ土台）。
///
/// # 非空虚性
/// 処理が kanade へ終了（`CloseRequest`）や切替（`ChangeGhost`）を送ると、言い終えた kanade は
/// `OnClose`／`OnGhostChanging` を SHIORI へ問う（偽の SHIORI はその問いの答えを持たず倒れ、状態の
/// 返事が届かない・届いても呼出の記録が増える）ので赤（2 つとも処理へ送らせて赤を確かめた）。
/// 台本を再生へ回すと再生中になって赤。答えずに捨てると `None` で赤。
#[test]
fn real_unit_answers_two_scripts_without_running_them() {
    use std::sync::mpsc;

    let mut rig = SwitchRig::new(vec![
        (
            "A",
            FakeShiori::Scripted(Box::new(|| standard_script(r"\0A\e"))),
        ),
        (
            "B",
            FakeShiori::Scripted(Box::new(|| standard_script(r"\0B\e"))),
        ),
    ]);
    rig.boot("A");
    // 窓を作らない土台では装着が起きず、預けた答えが届かない。表示の結線を外して窓の無いゴーストとして
    // 答えさせる（`shell_balloon_switch_tests.rs` と同じ外し方・判断の事実は 4.2 のテストが固定する）。
    let unwired = rig.world.remove_non_send::<Emo2Wiring>().is_some();
    let (tx, rx) = mpsc::channel();
    crate::mcp::install(&mut rig.world, rx);
    let steady = rig.wait_steady();
    let quiet = rig.pump_talking_until(|rig| !talking(&rig.world));
    let calls_before = rig.calls("A");

    let ask = |script: &str| {
        let (request, pending) = ToolRequest::new(call(script, Some("A")));
        assert!(tx.send(request).is_ok(), "受け口は生きている");
        pending
    };
    let first = ask(r"\0こんにちは\![change,ghost,B]\-");
    let second = ask(r"\1もう一度\-\![change,ghost,B]まだ読む\e");
    rig.world.run_schedule(wintf::ecs::Input);
    let both = [&first, &second].map(|p| p.wait_answer().map(|a| (a.ghost, a.outcome)));
    // 答えの後にもう 1 回段を回してから、kanade に先の依頼を片付けさせて見る。
    rig.world.run_schedule(wintf::ecs::Input);
    let still = (
        talking(&rig.world),
        crate::mcp::resolve::active(&rig.world).and_then(|g| g.name),
        rig.world.get_non_send::<SwitchInFlight>().is_none(),
        rig.exit_requested(),
        rig.calls("B").len(),
    );
    let calls_after = rig.calls("A");
    let down = rig.shutdown();

    let answer = Some(("A".to_owned(), no_window_answer(0)));
    assert_eq!(
        (unwired, steady, quiet, both, down),
        (true, true, true, [answer.clone(), answer], true),
        "（結線を外した・定常に着いた・言い終えた・2 件の答え・降ろせた）"
    );
    assert_eq!(calls_after, calls_before, "呼出の記録は増えない");
    assert_eq!(
        still,
        (false, Some("A".to_owned()), true, false, 0),
        "（再生中, 今のゴースト, 切替の途中でない, 終了の指示, B を起こした回数）"
    );
}
