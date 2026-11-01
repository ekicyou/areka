//! 本物の起動（切替の試験台・`boot_ghost` → `rewire_runtime_consumers` → `wire`）を通す結合テスト
//! （spec shell-scale-linkage 要件 3.10・4.5・7.2）。
//!
//! 起動の後に毎フレームの配線（`Emo2Wiring`）の表へ手で書き、MCP の `get_property`（受け口から汲む）と
//! ゴーストの道（`ShioriHostSink` の `GetProperty`）の答えを見る。実の窓・実の画面は使わない。

use std::sync::mpsc::{self, Sender};

use areka_ghost::sylphya_wiring::ghost_asker_id;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::get_property::Args;
use areka_mcp::tools::{ToolCall, ToolRequest, outcome};
use shiori_abi::error::ShioriError;
use shiori_abi::interface::IShioriHost;
use windows::core::HSTRING;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::super::*;
use super::applied;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::ghost_session::{GhostSlot, RuntimeRebuild, rebuild_runtime_in_place};
use crate::shiori_host::ShioriHostSink;

/// A の `OnBoot`: 台詞の途中で B への切替を命じる。
const A_TO_B: &str = "\\0A\\![change,ghost,B,--option=raise-event]\\e";

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

fn ghost_a(on_boot: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        standard_script(on_boot)
            .get(
                "OnGhostChanging",
                Ok(Some("\\0Bへ交代します\\e".to_owned())),
            )
            .get("OnGhostChanged", Ok(None))
    }))
}

fn ghost_b() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        standard_script("\\0B\\e").get("OnGhostChanged", Ok(None))
    }))
}

/// A・B を据えた試験台と MCP の受け口（A は起動記録あり）を作り、A を起こして定常に着かせる。
fn booted_rig(a: FakeShiori) -> (SwitchRig, Sender<ToolRequest>) {
    let mut rig = SwitchRig::new(vec![("A", a), ("B", ghost_b())]);
    // 切替先の窓の閉包の投函先（取り出しの系は無いので走らない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
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
        .get_non_send::<Emo2Wiring>()
        .expect("毎フレームの配線がある")
        .scope_scale_table();
    let mut t = table.write().unwrap();
    t.set(0, TargetWindow::Shell, applied(num, 96, Some(dpi)));
    t.set(0, TargetWindow::Balloon, applied(num, 96, Some(dpi)));
}

/// MCP の `get_property` を受け口へ送って汲み、その場の答え（ゴースト名と結果）を取る。
fn ask(rig: &mut SwitchRig, tx: &Sender<ToolRequest>, name: &str) -> Option<(String, ToolOutcome)> {
    let (req, pending) = ToolRequest::new(ToolCall::GetProperty(Args {
        property_name: name.to_owned(),
        ghost_name: None,
    }));
    tx.send(req).ok()?;
    crate::mcp::drain(&mut rig.world);
    pending
        .try_answer()
        .ok()
        .flatten()
        .map(|a| (a.ghost, a.outcome))
}

fn ask_all(rig: &mut SwitchRig, tx: &Sender<ToolRequest>) -> Vec<Option<(String, ToolOutcome)>> {
    NAMES.iter().map(|n| ask(rig, tx, n)).collect()
}

/// 今の実行系の読み口から作ったゴーストの道で `NAMES` を読む。
fn ghost_path(rig: &SwitchRig) -> Vec<Result<String, String>> {
    let sink = {
        let runtime = rig
            .world
            .get_non_send::<GhostSlot>()
            .and_then(|slot| slot.0.as_ref())
            .and_then(|session| session.runtime())
            .expect("置き場に実行系がある");
        ShioriHostSink::with_sylphya(
            runtime.sylphya_reader().clone(),
            runtime.sylphya_publisher().clone(),
            ghost_asker_id(&runtime.mount().shiori.dir),
        )
    };
    let host: IShioriHost = sink.into();
    NAMES
        .iter()
        .map(|n| match host.get_property(&HSTRING::from(*n)) {
            Ok(v) => Ok(v.to_string()),
            Err(ShioriError::PropertyNotFound) => Err("PropertyNotFound".to_string()),
            Err(e) => Err(format!("{e:?}")),
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

/// 同じ時点に MCP とゴーストの道が同じ名前に同じ文字・同じ値なしを返す（要件 3.10・7.2）。
///
/// # 非空虚性
/// `rewire_runtime_consumers` の `wire` を外すと両方の道が全部「無い名前」で赤。
/// `wire` へ配線の表でなく空の表を渡すと、書いた値が読めずに赤。
#[test]
fn mcp_and_ghost_paths_answer_the_same_scale() {
    let (mut rig, tx) = booted_rig(ghost_a("\\0A\\e"));
    write_scope0(&rig, 144, 144);
    let mcp = ask_all(&mut rig, &tx);
    let shiori = ghost_path(&rig);
    let down = rig.shutdown();

    let expected_shiori: Vec<Result<String, String>> = AT_150
        .iter()
        .map(|v| v.map(str::to_owned).ok_or("PropertyNotFound".to_owned()))
        .collect();
    assert_eq!(
        (mcp, shiori, down),
        (answers("A", AT_150), expected_shiori, true),
        "（{NAMES:?} への MCP の答え・ゴーストの道の答え・降ろせた）"
    );
}

/// SHIORI だけの読み直し（配線し直しの 2 回目）の後も、新しい読み口が同じ表を読む（要件 4.5）。
/// 読み直しの後に表を書き換えると、新しい読み口がその値を答える＝写しでなく同じ表。
///
/// # 非空虚性
/// 読み直しで `wire` へ新しい空の表を渡すと、読み直しの後の答えが値なしで赤。
#[test]
fn shiori_reload_reads_the_same_table() {
    let (mut rig, tx) = booted_rig(ghost_a("\\0A\\e"));
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

/// B へ替えた（新しい毎フレームの配線）後は、A の表に書いた値を答えない（要件 4.5）。
///
/// # 非空虚性
/// 表をプロセスで 1 つ持つか、切替で前の配線の表を使い回すと、替えた後も `150,150` を答えて赤。
#[test]
fn switching_to_another_ghost_drops_the_previous_scale() {
    let (mut rig, tx) = booted_rig(ghost_a(A_TO_B));
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
