//! 本番の道筋の兄弟テスト（design「Testing Strategy / 背景スレッドの口」の本番の道筋・「要求 1 件の
//! 一周」・要件 2.3・5.1・5.2・5.3・5.5・9.9・10.1）。
//!
//! 切替の土台（[`SwitchRig`]）の上で、本物の窓口（`Input` の段の取り出しの系）→ 背景スレッド
//! `update` → 本物の口 → kanade → 偽の SHIORI を通す。取得口だけを偽物（[`FakeFetch`]）に差し替え、
//! 更新先には 1 ファイルの木（`updates2.dau` と本文）を置く。要求は台本の受け口と同じ生の要求の
//! 送出端から入れる。確かめること: `changed` の一周でイベントが正典の順に SHIORI へ届き、総括の
//! 返事の台詞が終わってから同じゴーストが `OnGhostChanging` 無しに起き直る（`OnGhostChanged` は
//! 自分→自分）／`none` の一周では切替の要求が 0 件。
//!
//! 実時間に依らない: 進みは偽の SHIORI の記録・窓口の段・kanade の返信端の受け取りで揃える。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_update::Fetch;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::desk::{Stage, UpdateDesk, raw_sender};
use super::{RawUpdateRequest, TargetKind};
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::{HOMEURL_RESOURCE, RecordedCall};
use crate::ghost_session::GhostSlot;
use crate::install::fetch_url_test_support::FakeFetch;
use crate::menu::captions::send_query;

/// 偽の配布サイト（ネットワークには出ない）。
const HOMEURL: &str = "https://example.invalid/areka-update/";
/// 配る 1 ファイルとその本文・MD5（`hello` の MD5）。
const FILE: &str = "update-probe.txt";
const BODY: &[u8] = b"hello";
const BODY_MD5: &str = "5d41402abc4b2a76b9719d911017c592";
/// 総括への返事の台詞（置き換わった今のトーク＝読み直しはこの台詞の完了を待つ）。
const SUMMARY_TALK: &str = r"\0更新しました\e";

/// A の偽の SHIORI（起こすたびに新品）: 標準の台本に、更新のイベントの返事（総括だけ台詞）と
/// `useorigin1` 204、起こし直しの `OnGhostChanged` の台詞を足す（204 だと起動の根が `OnBoot` へ続く）。
fn ghost_a() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        let mut script = standard_script(r"\0A\e")
            .get("useorigin1", Ok(None))
            .get("OnGhostChanged", Ok(Some(r"\0A\e".to_owned())))
            .get("OnUpdateResult", Ok(Some(SUMMARY_TALK.to_owned())));
        for id in [
            "OnUpdateBegin",
            "OnUpdateReady",
            "OnUpdate.OnDownloadBegin",
            "OnUpdate.OnMD5CompareBegin",
            "OnUpdate.OnMD5CompareComplete",
            "OnUpdateComplete",
        ] {
            script = script.get(id, Ok(None));
        }
        script
    }))
}

/// A の `descript.txt` に更新先を書き、`local` があれば手元のファイルを置いてから A を起こし、定常に
/// 着いた土台。窓口の取得口の作り方は、`FILE` を 1 件だけ持つ定義ファイルを配る偽物へ差し替える。
fn running_a(local: Option<&[u8]>) -> SwitchRig {
    let mut rig = SwitchRig::new(vec![("A", ghost_a()), ("B", ghost_a())]);
    // 起こし直すときの窓の準備が閉包を投函する先（`Input` の段では走らない）。
    rig.world.insert_resource(WintfTaskPool::new());
    let a_dir = rig.root.ghost_dir("A");
    let descript = a_dir.join("ghost").join("master").join("descript.txt");
    let text = std::fs::read_to_string(&descript).expect("descript.txt は UTF-8");
    let mut lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("homeurl,"))
        .collect();
    let homeurl = format!("homeurl,{HOMEURL}");
    lines.push(&homeurl);
    std::fs::write(&descript, lines.join("\r\n")).expect("descript.txt に更新先を書く");
    if let Some(bytes) = local {
        std::fs::write(a_dir.join(FILE), bytes).expect("手元のファイルを置く");
    }
    let manifest = format!("{FILE}\x01{BODY_MD5}\r\n");
    let site = FakeFetch::new()
        .serve(&format!("{HOMEURL}updates2.dau"), manifest.as_bytes())
        .serve(&format!("{HOMEURL}{FILE}"), BODY);
    rig.world.non_send_mut::<UpdateDesk>().new_fetch =
        Arc::new(move || Ok(Box::new(site.clone()) as Box<dyn Fetch>));
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// 台本の受け口と同じ道（生の要求の送出端）で、今のゴーストの更新を頼む（理由は `script`）。
fn request_ghost_update(rig: &SwitchRig) {
    raw_sender(&rig.world)
        .send(RawUpdateRequest::Current(vec![TargetKind::Ghost]))
        .expect("窓口の生の要求の受信端は生きている");
}

/// 呼び出しの列（`GET 名`／`NOTIFY 名`／`UNLOAD`）。
fn kinds(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|call| match call {
            RecordedCall::Get { id, .. } => format!("GET {id}"),
            RecordedCall::Notify { id, .. } => format!("NOTIFY {id}"),
            RecordedCall::Unload => "UNLOAD".to_owned(),
            RecordedCall::Status => "STATUS".to_owned(),
        })
        .collect()
}

/// `id` の GET の Reference（最初の 1 件）。
fn refs_of(calls: &[RecordedCall], id: &str) -> Option<Vec<String>> {
    calls.iter().find_map(|call| match call {
        RecordedCall::Get {
            id: got,
            references,
        } if got == id => Some(references.clone()),
        _ => None,
    })
}

fn named(events: &[CapturedEvent], name: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .count()
}

/// A の最初の起動の偽の SHIORI が受けた `GET homeurl` の数（定常到達の照会と本テストの確かめ）。
fn homeurl_gets(rig: &SwitchRig) -> usize {
    rig.handle("A")
        .non_status_calls()
        .iter()
        .filter(|c| matches!(c, RecordedCall::Get { id, .. } if id == HOMEURL_RESOURCE))
        .count()
}

/// ゴーストの `.update-work/` の下の印つきの走行フォルダ（成功した走行の残り）。
fn marked_leftovers(ghost_dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(ghost_dir.join(".update-work"))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.join("committed").is_file())
                .collect()
        })
        .unwrap_or_default()
}

/// 起動系列（起動記録あり＝`OnFirstBoot` なし）に続く更新のイベントの正典の順（1 ファイルの `changed`）。
const CHANGED_ROUND: [&str; 12] = [
    "NOTIFY OnInitialize",
    "GET username",
    "GET OnBoot",
    "NOTIFY basewareversion",
    "GET useorigin1",
    "GET OnUpdateBegin",
    "GET OnUpdateReady",
    "GET OnUpdate.OnDownloadBegin",
    "GET OnUpdate.OnMD5CompareBegin",
    "GET OnUpdate.OnMD5CompareComplete",
    "GET OnUpdateComplete",
    "GET OnUpdateResult",
];

/// `changed` の一周: `OnUpdateBegin` → `OnUpdateReady` → 1 ファイル分（`OnDownloadBegin`・MD5 の照合）→
/// `OnUpdateComplete`（`changed`・置いたファイル）→ `OnUpdateResult`（`ghost\x01OK\x011`）が偽の SHIORI
/// に届き、ファイルが手元に置かれる。総括の返事の台詞が再生中の間は切替が kanade に保留され（照会の
/// 往復で確かめる）、台詞を進めて終わると A が `OnGhostChanging`・`OnClose`・`OnBoot` 無しに起き直り
/// `OnGhostChanged`（Ref0＝A のさくら・Ref2＝A）を受ける。読み直しの切替が終わった後、`.update-work` に
/// 印つきの残りは無い（エンジンの後に置いた残りも、切替を終える時点で消える・要件 5.9）。
///
/// # 非空虚性
/// 手続きの読み直しの頼み（`request_reload`）を外すと切替の予約が立たず期限切れで赤、写しの順を
/// 入れ替えると呼び出しの列の判定が赤。
#[test]
fn a_changed_round_reaches_shiori_in_order_and_reboots_the_same_ghost_after_the_summary_talk() {
    let mut rig = running_a(None);
    let a_dir = rig.root.ghost_dir("A");

    let (reserved, events) = capture(|| {
        request_ghost_update(&rig);
        rig.pump_input_until(|rig| rig.world.get_non_send::<SwitchInFlight>().is_some())
    });
    assert!(reserved, "読み直しの切替が予約される: {events:?}");
    assert_eq!(named(&events, "update_reload_requested"), 1, "{events:?}");
    // 偽の取得の一周は掴まれた古い写しを作れないので、エンジンが走り終えた後（読み直しの予約の後）に
    // 成功した走行の残りを置く。
    let leftover = a_dir.join(".update-work").join("held-run");
    std::fs::create_dir_all(leftover.join("old")).expect("走行フォルダを組む");
    std::fs::write(leftover.join("committed"), b"").expect("印を置く");

    // 予約の切替は kanade へ送ってある。同じ送出端で照会を 1 往復させ、返事を受けた時点で kanade は
    // 切替の要求を捌き終えている。まだ定常（再生中）なら照会は SHIORI まで届く＝切替は台詞の完了待ち。
    let before = homeurl_gets(&rig);
    let kanade = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.kanade().cloned())
        .expect("A の kanade の送出端");
    send_query(&kanade, vec![HOMEURL_RESOURCE])
        .expect("kanade へ照会を送れる")
        .recv()
        .expect("kanade が照会に答える");
    let held_while_talking = (homeurl_gets(&rig) - before, rig.calls("A").len());

    let rebooted = rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || (rig.calls("A").len() == 2 && rig.world.get_non_send::<SwitchInFlight>().is_none())
    });
    let (a, b) = (rig.calls("A"), rig.calls("B"));
    let exited = rig.exit_requested();
    let placed = std::fs::read(a_dir.join(FILE)).ok();
    let leftovers = marked_leftovers(&a_dir);
    assert!(rig.shutdown());

    assert!(rebooted && !exited, "A が起き直る（終了しない）: {a:?}");
    assert_eq!(leftovers, Vec::<PathBuf>::new(), "印つきの残りは無い");
    assert_eq!(held_while_talking, (1, 1), "台詞の間は起き直らない");
    assert_eq!((a.len(), b.len()), (2, 0), "A を 1 回起こし直すだけ: {a:?}");
    let mut expected: Vec<String> = CHANGED_ROUND.iter().map(|s| (*s).to_owned()).collect();
    expected.push("UNLOAD".to_owned());
    assert_eq!(kinds(&a[0]), expected, "送出の順");

    let begin = refs_of(&a[0], "OnUpdateBegin").expect("OnUpdateBegin");
    assert_eq!(
        (begin[0].as_str(), begin[3].as_str(), begin[4].as_str()),
        ("A", "ghost", "script")
    );
    let complete = refs_of(&a[0], "OnUpdateComplete").expect("OnUpdateComplete");
    assert_eq!(&complete[..2], ["changed", FILE]);
    let result = refs_of(&a[0], "OnUpdateResult").expect("OnUpdateResult");
    assert_eq!(result, ["ghost\x01OK\x011"]);
    assert_eq!(placed.as_deref(), Some(BODY), "ファイルが手元に置かれる");

    let second = kinds(&a[1]);
    for name in ["GET OnGhostChanging", "GET OnBoot"] {
        assert!(
            !second.contains(&name.to_owned()),
            "{name} は 0 件: {second:?}"
        );
    }
    assert!(
        a.iter().flatten().all(|c| !matches!(c,
            RecordedCall::Get { id, .. } | RecordedCall::Notify { id, .. } if id == "OnClose")),
        "OnClose は 0 件: {a:?}"
    );
    let changed = refs_of(&a[1], "OnGhostChanged").expect("OnGhostChanged が届く");
    assert_eq!(
        (changed[0].as_str(), changed[2].as_str()),
        ("Aのさくら", "A"),
        "自分→自分"
    );
}

/// `none` の一周（手元が配布と同じ）: `OnUpdateBegin` → `OnUpdateComplete`（`none`）→ `OnUpdateResult`
/// （`ghost\x01OK\x010`）が届き、依頼が終わっても（窓口の段が戻っても）切替の要求は 0 件。
#[test]
fn a_none_round_requests_no_switch() {
    let mut rig = running_a(Some(BODY));

    let (done, events) = capture(|| {
        request_ghost_update(&rig);
        rig.pump_input_until(|rig| {
            rig.calls("A")[0]
                .iter()
                .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnUpdateResult"))
                && rig.world.non_send::<UpdateDesk>().stage == Stage::Idle
        })
    });
    let a = rig.calls("A");
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    assert!(rig.shutdown());

    assert!(done, "依頼が終わる: {events:?}");
    assert_eq!(
        kinds(&a[0])[4..],
        [
            "GET useorigin1",
            "GET OnUpdateBegin",
            "GET OnUpdateComplete",
            "GET OnUpdateResult"
        ]
    );
    assert_eq!(
        refs_of(&a[0], "OnUpdateComplete").map(|r| r[0].clone()),
        Some("none".to_owned())
    );
    assert_eq!(
        refs_of(&a[0], "OnUpdateResult"),
        Some(vec!["ghost\x01OK\x010".to_owned()])
    );
    assert_eq!((a.len(), reserved), (1, false), "起こし直さない");
    for name in [
        "update_reload_requested",
        "update_reload_skipped",
        "ghost_switch_requested",
    ] {
        assert_eq!(named(&events, name), 0, "{name}: {events:?}");
    }
}
