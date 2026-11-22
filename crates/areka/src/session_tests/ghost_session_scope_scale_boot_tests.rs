//! 本物の起動（切替の試験台・`boot_ghost` → `rewire_runtime_consumers` → `wire`）を通す結合テスト。
//!
//! 起動の後に毎フレームの配線（`RuntimeWiring`）の表へ手で書き、MCP の `get_property`（受け口から汲む）と
//! ゴーストの道（本番の `\![get,property]` の受け口 `PropGetCueSink` が kanade へ頼む返し）の答えを見る。実の窓・実の画面は使わない。

use std::sync::mpsc::{self, Sender};

use areka_ghost::prop_get_sink::PropGetCueSink;
use areka_ghost::sylphya_wiring::ghost_asker_id;
use areka_kanade::KanadeMsg;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::{ToolRequest, outcome};
use areka_runtime::consumer_ledger::test_support::carrier_cue;
use areka_sakura::contract::CueSink;

use super::ghost_session_switch_tests::{A_TO_B, ask, ghost_a, ghost_b, lap_rig};
use areka_runtime::frame::RuntimeWiring;
use areka_runtime::target_map::TargetWindow;
use areka_session::ghost_session::ghost_session_scope_scale::test_support::applied;
use areka_session::ghost_session::{GhostSlot, RuntimeRebuild, rebuild_runtime_in_place};
use areka_session::ghost_switch::SwitchInFlight;
use areka_session::ghost_switch_test_support::{FakeShiori, SwitchRig};

const NAMES: [&str; 5] = [
    "currentghost.scope(0).scaling",
    "currentghost.balloon.scope(0).scaling",
    "currentghost.scope(0).currentmonitor.dpi",
    "currentghost.balloon.scope(0).currentmonitor.dpi",
    "currentghost.scope(1).scaling",
];

/// 作者の基準 96 の絵を DPI 144 で出したスコープ 0 の答え（スコープ 1 は窓が無い＝値なし）。
const AT_150: [Option<&str>; 5] = [
    Some("150,150"),
    Some("150,150"),
    Some("144"),
    Some("144"),
    None,
];

/// A・B を据えた試験台と MCP の受け口（A は起動記録あり）を作り、A を起こして定常に着かせる。
fn booted_rig(a: FakeShiori) -> (SwitchRig, Sender<ToolRequest>) {
    let mut rig = lap_rig(a, ghost_b());
    let (tx, rx) = mpsc::channel();
    crate::mcp::install(&mut rig.world, rx);
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    (rig, tx)
}

/// 今の毎フレームの配線の表のスコープ 0 へ（キャラクター・バルーンとも）倍率と DPI を書く。
fn write_scope0(rig: &SwitchRig, num: u32, dpi: u16) {
    let table = rig
        .world
        .get_non_send::<RuntimeWiring>()
        .expect("毎フレームの配線がある")
        .scope_scale_table();
    let mut t = table.write().unwrap();
    t.set(0, TargetWindow::Shell, applied(num, 96, Some(dpi)));
    t.set(0, TargetWindow::Balloon, applied(num, 96, Some(dpi)));
}

fn ask_all(rig: &mut SwitchRig, tx: &Sender<ToolRequest>) -> Vec<Option<(String, ToolOutcome)>> {
    NAMES.iter().map(|n| ask(rig, tx, n)).collect()
}

/// ゴーストが自分で書く自由な名前（問い手ごとの区画にだけ在る）。
const OWN_KEY: &str = "probe.owner";

/// 今の実行系の読み口と問い手で組んだ本番の受け口へ `NAMES` と [`OWN_KEY`] を問う合図を 1 つ流し、
/// kanade へ頼んだ返しを取る。倍率の名前は問い手に依らないので、問い手の取り違えは [`OWN_KEY`] で見る。
fn ghost_path(rig: &SwitchRig) -> Vec<(String, Vec<String>)> {
    let runtime = rig
        .world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime())
        .expect("置き場に実行系がある");
    let asker = ghost_asker_id(&runtime.mount().shiori.dir);
    let publisher = runtime.sylphya_publisher();
    publisher.set(asker.clone(), OWN_KEY.to_owned(), "A".to_owned());
    publisher.barrier().expect("sylphya が生きている");
    let (tx, rx) = mpsc::channel();
    let mut sink = PropGetCueSink::new(runtime.sylphya_reader().clone(), asker, tx);
    let tokens: Vec<&str> = ["property", "OnScaleProbe"]
        .into_iter()
        .chain(NAMES)
        .chain([OWN_KEY])
        .collect();
    sink.emit(carrier_cue("get", &tokens));
    rx.try_iter()
        .map(|m| match m {
            KanadeMsg::PropertyEvent { id, references } => (id, references),
            _ => panic!("PropertyEvent 以外が届いた"),
        })
        .collect()
}

/// MCP の答えの期待（値なしは「無い名前」）。
fn answers(ghost: &str, values: [Option<&str>; 5]) -> Vec<Option<(String, ToolOutcome)>> {
    values
        .into_iter()
        .map(|v| {
            let o = match v {
                Some(v) => outcome::value(v),
                None => outcome::ng("Cannot find such property name."),
            };
            Some((ghost.to_owned(), o))
        })
        .collect()
}

/// 同じ時点に MCP とゴーストの道が同じ名前に同じ文字を返し、MCP の値なしはゴーストの道では空になる。
/// ゴーストの道は自分で書いた [`OWN_KEY`] も読める。
///
/// # 非空虚性
/// `rewire_runtime_consumers` の `wire` を外すと両方の道が全部値なしで赤。
/// `wire` へ配線の表でなく空の表を渡すと、書いた値が読めずに赤。
/// 受け口の問い手を `ghost_asker_id` から別の問い手に替えると、[`OWN_KEY`] が空で赤。
#[test]
fn mcp_and_ghost_paths_answer_the_same_scale() {
    let (mut rig, tx) = booted_rig(ghost_a("\\0A\\e", None));
    write_scope0(&rig, 144, 144);
    let mcp = ask_all(&mut rig, &tx);
    let ghost = ghost_path(&rig);
    let down = rig.shutdown();

    let expected_ghost = vec![(
        "OnScaleProbe".to_owned(),
        AT_150
            .iter()
            .map(|v| v.unwrap_or(""))
            .chain(["A"])
            .map(str::to_owned)
            .collect(),
    )];
    assert_eq!(
        (mcp, ghost, down),
        (answers("A", AT_150), expected_ghost, true),
        "（{NAMES:?} への MCP の答え・ゴーストの道の返し・降ろせた）"
    );
}

/// SHIORI だけの読み直し（配線し直しの 2 回目）の後も、新しい読み口が同じ表を読む。
/// 読み直しの後に表を書き換えると、新しい読み口がその値を答える＝写しでなく同じ表。
///
/// # 非空虚性
/// 読み直しで `wire` へ新しい空の表を渡すと、読み直しの後の答えが値なしで赤。
#[test]
fn shiori_reload_reads_the_same_table() {
    let (mut rig, tx) = booted_rig(ghost_a("\\0A\\e", None));
    write_scope0(&rig, 144, 144);
    let before = ask_all(&mut rig, &tx);
    let rebuilt = rebuild_runtime_in_place(&mut rig.world, "A");
    let after = ask_all(&mut rig, &tx);
    write_scope0(&rig, 192, 192);
    let rewritten = ask_all(&mut rig, &tx);
    let down = rig.shutdown();

    assert!(matches!(rebuilt, RuntimeRebuild::Rebuilt), "{rebuilt:?}");
    let at_200 = [
        Some("200,200"),
        Some("200,200"),
        Some("192"),
        Some("192"),
        None,
    ];
    assert_eq!(
        (before, after, rewritten, down),
        (
            answers("A", AT_150),
            answers("A", AT_150),
            answers("A", at_200),
            true
        ),
        "（読み直す前・読み直した後・書き換えた後の答え・降ろせた）"
    );
}

/// B へ替えた（新しい毎フレームの配線）後は、A の表に書いた値を答えない。
///
/// # 非空虚性
/// 表をプロセスで 1 つ持つか、切替で前の配線の表を使い回すと、替えた後も `150,150` を答えて赤。
#[test]
fn switching_to_another_ghost_drops_the_previous_scale() {
    let (mut rig, tx) = booted_rig(ghost_a(A_TO_B, None));
    write_scope0(&rig, 144, 144);
    let booted = ask_all(&mut rig, &tx);
    let welcomed = rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || (!rig.calls("B").is_empty() && rig.world.get_non_send::<SwitchInFlight>().is_none())
    });
    let switched = ask_all(&mut rig, &tx);
    let down = rig.shutdown();

    assert_eq!(
        (booted, welcomed, switched, down),
        (answers("A", AT_150), true, answers("B", [None; 5]), true),
        "（A の答え・B の定常・B へ替えた後の答え・降ろせた）"
    );
}
