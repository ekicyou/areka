//! `resolve` の決定論テスト（解決の判断の全場合）。
//!
//! ⑴ 解決の判断（名前・本体側名・英字の大小・前後の空白・フルパス・省略・空と空白だけ・0 体・`Reject`）
//! ⑵ 一覧の値（名前あり・`name` 無し）が同じゴーストへ戻ること
//! ⑶ World からの読み取り（GhostSlot 無し・置き場が空・実行系の無い単位は無し）
//! ⑷ `ask_kanade`（預けて後から答える・返信端の落ち・送れない・送り口が無い・`ghost_name` の有無で 2 通りの「降りた」）

use std::path::PathBuf;
use std::sync::mpsc;

use areka_kanade::{ExecutionSnapshot, ExecutionStatus};
use areka_mcp::tools::get_status::Args;
use areka_mcp::tools::{Pending, ToolCall, ToolRequest};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};

use super::*;
use crate::mcp::kanade_fake_test_support::{answer, body_and_error, ghost, rig, warn_or_worse};
use crate::mcp::{McpLater, drain};
use areka_session::ghost_session::{GhostSession, GhostSlot};

const ROOT: &str = r"C:\ssp\ghost\emily4";

fn named() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: Some("エミリ".to_string()),
        root: PathBuf::from(ROOT),
    }
}

fn unnamed() -> ActiveGhost {
    ActiveGhost {
        name: None,
        sakura_name: None,
        root: PathBuf::from(ROOT),
    }
}

/// 名前が無く本体側名だけを持つ 1 体。
fn sakura_only() -> ActiveGhost {
    ActiveGhost {
        name: None,
        sakura_name: Some("エミリ".to_string()),
        root: PathBuf::from(ROOT),
    }
}

/// 名前ありの 1 体へ `ghost_name` を、省略を起動中へ解く扱い（`UseActive`）で解いた結果。
fn solve(ghost_name: Option<&str>) -> Result<ActiveGhost, &'static str> {
    let g = named();
    resolve(Some(&g), ghost_name, Omitted::UseActive).cloned()
}

// ---- ⑴ 解決の判断 ---------------------------------------------------------

#[test]
fn name_exact_match_resolves() {
    assert_eq!(solve(Some("Emily/Phase4.5")), Ok(named()));
}

#[test]
fn name_differing_only_in_ascii_case_resolves() {
    for given in ["emily/phase4.5", "EMILY/PHASE4.5"] {
        assert_eq!(solve(Some(given)), Ok(named()), "{given}");
    }
}

#[test]
fn sakura_name_resolves() {
    assert_eq!(solve(Some("エミリ")), Ok(named()));
    // `name` が無くても本体側名で解決する。
    let g = sakura_only();
    assert_eq!(resolve(Some(&g), Some("エミリ"), Omitted::Reject), Ok(&g));
    // 本体側名も名前と同じ規則で、半角の英字の大小を畳む。
    let g = ActiveGhost {
        sakura_name: Some("Emi".to_string()),
        ..sakura_only()
    };
    assert_eq!(resolve(Some(&g), Some("eMI"), Omitted::Reject), Ok(&g));
}

#[test]
fn non_name_strings_do_not_resolve() {
    // 相方の名前・sakura.name2 に当たる文字列も、ActiveGhost が持たないのでただの外れ。
    for g in [named(), sakura_only()] {
        for given in ["エモ", "エミリー", "Emily", "Nobody"] {
            assert_eq!(
                resolve(Some(&g), Some(given), Omitted::UseActive),
                Err(CANNOT_FIND),
                "{given}"
            );
        }
    }
}

#[test]
fn kana_width_and_fullwidth_case_differences_do_not_resolve() {
    let g = ActiveGhost {
        name: Some("えもＡＢＣ".to_string()),
        sakura_name: None,
        root: PathBuf::from(ROOT),
    };
    assert_eq!(
        resolve(Some(&g), Some("えもＡＢＣ"), Omitted::UseActive),
        Ok(&g)
    );
    // かなの違い・全角の英字の大小・全角と半角。
    for given in ["エモＡＢＣ", "えもａｂｃ", "えもABC"] {
        assert_eq!(
            resolve(Some(&g), Some(given), Omitted::UseActive),
            Err(CANNOT_FIND),
            "{given}"
        );
    }
}

#[test]
fn name_with_surrounding_whitespace_resolves() {
    for name in ["Emily/Phase4.5", "エミリ"] {
        for given in [
            format!(" {name} "),
            format!("\t{name}\t"),
            format!("\u{3000}{name}\u{3000}"),
            format!(" \t\u{3000}{name}"),
            format!("{name}\u{3000}\t "),
        ] {
            assert_eq!(solve(Some(&given)), Ok(named()), "{given:?}");
        }
    }
}

#[test]
fn inner_whitespace_difference_does_not_resolve() {
    let g = ActiveGhost {
        name: Some("Emily Phase".to_string()),
        sakura_name: None,
        root: PathBuf::from(ROOT),
    };
    assert_eq!(
        resolve(Some(&g), Some("Emily Phase"), Omitted::UseActive),
        Ok(&g)
    );
    // 途中の空白は除かない。
    for given in ["Emily  Phase", "Emily\u{3000}Phase", "EmilyPhase"] {
        assert_eq!(
            resolve(Some(&g), Some(given), Omitted::UseActive),
            Err(CANNOT_FIND),
            "{given:?}"
        );
    }
}

#[test]
fn blank_or_empty_cannot_find() {
    // 空文字・空白だけは省略ではなく、何にも当たらない名前（0 体も）。
    let (n, u, s) = (named(), unnamed(), sakura_only());
    for active in [Some(&n), Some(&u), Some(&s), None] {
        for omitted in [Omitted::UseActive, Omitted::Reject] {
            for given in ["", "   ", "\t", "\u{3000}"] {
                assert_eq!(
                    resolve(active, Some(given), omitted),
                    Err(CANNOT_FIND),
                    "{given:?} {omitted:?} {}",
                    active.is_some()
                );
            }
        }
    }
}

#[test]
fn full_path_with_surrounding_whitespace_does_not_resolve() {
    // フルパスとの照合では前後の空白を除かない。
    for g in [named(), unnamed()] {
        for given in [
            r" C:\ssp\ghost\emily4",
            r"C:\ssp\ghost\emily4 ",
            " C:\\ssp\\ghost\\emily4\\ ",
            "\tC:/ssp/ghost/emily4/\t",
            "\u{3000}C:/ssp/ghost/emily4",
        ] {
            assert_eq!(
                resolve(Some(&g), Some(given), Omitted::UseActive),
                Err(CANNOT_FIND),
                "{given:?}"
            );
        }
    }
}

#[test]
fn every_spelling_resolves_to_the_same_ghost() {
    // 綴りは返り値に残らない。どの綴りでも起動中のゴーストの値そのもの。
    let g = named();
    for given in [
        "Emily/Phase4.5",
        "emily/PHASE4.5",
        " Emily/Phase4.5\u{3000}",
        "エミリ",
        "\tエミリ ",
        ROOT,
        "c:/ssp/ghost/emily4/",
    ] {
        let got = resolve(Some(&g), Some(given), Omitted::Reject);
        assert_eq!(got, Ok(&g), "{given:?}");
        let got = got.unwrap();
        assert!(std::ptr::eq(got, &g), "{given:?}");
        assert_eq!(listed_value(got), "Emily/Phase4.5", "{given:?}");
    }
}

#[test]
fn full_path_resolves() {
    assert_eq!(solve(Some(ROOT)), Ok(named()));
}

#[test]
fn full_path_with_case_separator_and_trailing_differences_resolves() {
    for given in [
        r"c:\SSP\Ghost\EMILY4",
        "C:/ssp/ghost/emily4",
        r"C:\ssp\ghost\emily4\",
        "c:/SSP/ghost/Emily4/",
        r"C:\ssp/ghost\emily4\\",
    ] {
        assert_eq!(solve(Some(given)), Ok(named()), "{given}");
    }
}

#[test]
fn folder_name_only_does_not_resolve() {
    assert_eq!(solve(Some("emily4")), Err(CANNOT_FIND));
}

#[test]
fn relative_path_does_not_resolve() {
    for given in [r"ghost\emily4", "ghost/emily4", r".\ghost\emily4"] {
        assert_eq!(solve(Some(given)), Err(CANNOT_FIND), "{given}");
    }
}

#[test]
fn omitted_with_use_active_resolves_to_the_active_one() {
    assert_eq!(solve(None), Ok(named()));
}

#[test]
fn omitted_with_reject_is_not_active() {
    let g = named();
    assert_eq!(resolve(Some(&g), None, Omitted::Reject), Err(NOT_ACTIVE));
}

#[test]
fn no_active_ghost_omitted_is_not_active() {
    for omitted in [Omitted::UseActive, Omitted::Reject] {
        assert_eq!(resolve(None, None, omitted), Err(NOT_ACTIVE));
    }
}

#[test]
fn no_active_ghost_with_name_cannot_find() {
    for omitted in [Omitted::UseActive, Omitted::Reject] {
        assert_eq!(
            resolve(None, Some("Emily/Phase4.5"), omitted),
            Err(CANNOT_FIND)
        );
        assert_eq!(resolve(None, Some(ROOT), omitted), Err(CANNOT_FIND));
    }
}

#[test]
fn reject_with_name_still_resolves() {
    let g = named();
    assert_eq!(
        resolve(Some(&g), Some("Emily/Phase4.5"), Omitted::Reject),
        Ok(&g)
    );
}

// ---- ⑵ 一覧の値 -----------------------------------------------------------

#[test]
fn listed_value_is_name_when_present() {
    assert_eq!(listed_value(&named()), "Emily/Phase4.5");
}

#[test]
fn listed_value_is_full_path_without_trailing_separator_when_name_absent() {
    assert_eq!(listed_value(&unnamed()), ROOT);
    let trailing = ActiveGhost {
        name: None,
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4\"),
    };
    assert_eq!(listed_value(&trailing), ROOT);
}

#[test]
fn listed_value_resolves_back_to_the_same_ghost() {
    for g in [named(), unnamed(), sakura_only()] {
        for omitted in [Omitted::UseActive, Omitted::Reject] {
            let value = listed_value(&g);
            assert_eq!(resolve(Some(&g), Some(&value), omitted), Ok(&g), "{value}");
        }
    }
}

// ---- ⑶ World からの読み取り ------------------------------------------------

#[test]
fn active_is_none_without_ghost_slot() {
    assert_eq!(active(&World::new()), None);
}

#[test]
fn active_is_none_when_slot_is_empty() {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(None));
    assert_eq!(active(&world), None);
}

#[test]
fn active_is_none_when_session_has_no_runtime() {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        None,
        PathBuf::from(ROOT),
    ))));
    assert_eq!(active(&world), None);
}

// ---- ⑷ kanade へ 1 通送って後から答える配線 ---------------------------------

const ASK: KanadeAsk = KanadeAsk {
    tool: "probe",
    what: "問い合わせ",
    unavailable: "Probe is not available",
    unavailable_event: "mcp_probe_unavailable",
    gone_event: "mcp_probe_kanade_gone",
    dropped_event: "mcp_probe_reply_dropped",
};

/// 問い合わせ 1 通を `ask_kanade` で送り、待つ側を返す。返事が届いたら `answer` が受けた「降りた」の文言を本文にする。
fn ask(world: &mut World, ghost_name: Option<&str>) -> (Pending, Vec<CapturedEvent>) {
    let (req, pending) = ToolRequest::new(ToolCall::GetStatus(Args { ghost_name: None }));
    let ((), events) = capture(|| {
        ask_kanade(
            world,
            &ghost(),
            ghost_name,
            ASK,
            req.reply,
            |reply| KanadeMsg::StatusQuery { reply },
            |_: ExecutionStatus, gone| outcome::value(gone),
        )
    });
    (pending, events)
}

fn take_query(rx: &mpsc::Receiver<KanadeMsg>) -> ReplySender<ExecutionStatus> {
    match rx.try_recv() {
        Ok(KanadeMsg::StatusQuery { reply }) => reply,
        other => panic!(
            "問い合わせ 1 通が届いているはず: {:?}",
            other.map(|_| "別の知らせ")
        ),
    }
}

fn idle() -> ExecutionStatus {
    ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE)
}

/// 「降りた」の文言は `ghost_name` の有無で 2 通り（返事を受けた `answer` へも同じ文言を渡す）。
const GONE: [(Option<&str>, &str); 2] = [(Some("Emily/Phase4.5"), CANNOT_FIND), (None, NOT_ACTIVE)];

/// 返事を待たずに預け、1 通だけ送る。返事の前に回しても預けたまま、届いたら答えて置き場から外れる。
#[test]
fn ask_kanade_defers_and_answers_when_the_reply_arrives() {
    for (ghost_name, gone) in GONE {
        let (mut world, rx) = rig();
        let (pending, called) = ask(&mut world, ghost_name);
        assert!(matches!(pending.try_answer(), Ok(None)), "呼んだ直後は未着");
        let reply = take_query(&rx);
        assert!(rx.try_recv().is_err(), "問い合わせは 1 通だけ");
        let ((), before) = capture(|| drain(&mut world));
        assert!(matches!(pending.try_answer(), Ok(None)), "返事の前は未着");
        assert_eq!(world.non_send::<McpLater>().0.len(), 1, "預けたまま");

        reply.send(idle()).expect("返信端は生きている");
        let ((), after) = capture(|| drain(&mut world));
        assert_eq!(
            body_and_error(&answer(&pending)),
            (gone.to_string(), false),
            "{ghost_name:?}"
        );
        assert!(world.non_send::<McpLater>().0.is_empty());
        assert_eq!(
            warn_or_worse(&called) + warn_or_worse(&before) + warn_or_worse(&after),
            0
        );
    }
}

/// 呼んだ直後（最初の覗きの前）に返事が届いていても、最初の覗きで答える。
#[test]
fn ask_kanade_answers_a_reply_ready_at_once() {
    let (mut world, rx) = rig();
    let (pending, _) = ask(&mut world, None);
    take_query(&rx).send(idle()).expect("返信端は生きている");
    drain(&mut world);
    assert_eq!(
        body_and_error(&answer(&pending)),
        (NOT_ACTIVE.to_string(), false)
    );
}

/// 返事の前に返信端が落ちた → 「降りた」の `NG:`・warn 以上 0 件。
#[test]
fn ask_kanade_reads_a_dropped_reply_as_gone() {
    for (ghost_name, gone) in GONE {
        let (mut world, rx) = rig();
        let (pending, called) = ask(&mut world, ghost_name);
        drop(take_query(&rx));
        let ((), polled) = capture(|| drain(&mut world));
        assert_eq!(
            body_and_error(&answer(&pending)),
            (format!("NG:{gone}"), true),
            "{ghost_name:?}"
        );
        assert_eq!(
            warn_or_worse(&called) + warn_or_worse(&polled),
            0,
            "{ghost_name:?}: {called:?} {polled:?}"
        );
    }
}

/// kanade が止まっていて送れない → その場で「降りた」の `NG:`・預けない・warn 以上 0 件。
#[test]
fn ask_kanade_reads_an_unsendable_kanade_as_gone() {
    for (ghost_name, gone) in GONE {
        let (mut world, rx) = rig();
        drop(rx);
        let (pending, called) = ask(&mut world, ghost_name);
        assert_eq!(
            body_and_error(&answer(&pending)),
            (format!("NG:{gone}"), true),
            "{ghost_name:?}"
        );
        assert_eq!(warn_or_worse(&called), 0, "{ghost_name:?}: {called:?}");
        assert!(world.non_send::<McpLater>().0.is_empty(), "預けない");
    }
}

/// 置き場に送り口が無い（空の World）→ `unavailable` の `NG:`・warn ちょうど 1 件・error 0 件。
#[test]
fn ask_kanade_answers_unavailable_without_a_kanade_in_the_slot() {
    let (pending, called) = ask(&mut World::new(), Some("Emily/Phase4.5"));
    assert_eq!(
        body_and_error(&answer(&pending)),
        ("NG:Probe is not available".to_string(), true)
    );
    let levels: Vec<_> = called.iter().map(|ev| ev.level).collect();
    assert_eq!(levels, [tracing::Level::WARN], "{called:?}");
}
