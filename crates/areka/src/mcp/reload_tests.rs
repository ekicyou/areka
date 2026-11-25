//! `reload` の決定論テスト。
//!
//! 返事はどれもその場で届く（`try_answer` が待たずに答えを持つ）。終了の途中だけは返事を落とす。
//! 受理の文の `since_id` は [`answer`] へ偽の窓口と偽の番号を渡して固定する（実行系の無い World では
//! 窓口は受理しない）。

use std::cell::Cell;

use areka_actor::ReplyError;
use areka_mcp::ToolContent;
use areka_mcp::tools::{Pending, ToolCall, ToolRequest};
use bevy_ecs::schedule::Schedules;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::*;
use crate::mcp::kanade_fake_test_support::ghost;
use crate::session_tests::events_named;
use areka_session::log_history::TARGET_ERROR;

fn call(world: &mut World, target: &str) -> (Pending, Vec<CapturedEvent>) {
    let args = Args {
        target: target.to_string(),
        ghost_name: None,
    };
    let (req, pending) = ToolRequest::new(ToolCall::Reload(args.clone()));
    let ((), events) = capture(|| handle(world, &ghost(), args, req.reply));
    (pending, events)
}

/// その場で届いた答えの (本文, isError)。
fn answered(pending: &Pending) -> (String, bool) {
    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    let [ToolContent::Text(text)] = answer.outcome.content.as_slice() else {
        panic!("本文 1 つ: {:?}", answer.outcome.content);
    };
    (text.clone(), answer.outcome.is_error)
}

/// 6 語以外（空・前後の空白・別の語）は `NG:Unknown target (…)` をその場で返し、窓口へ渡さない
/// （`reload_refused` も `reload_started` も残らない）。
#[test]
fn unknown_targets_answer_unknown_target_without_asking_the_desk() {
    for target in ["", " shell", "shell ", "aigraph", "reloadsurface"] {
        let (pending, events) = call(&mut World::new(), target);
        assert_eq!(
            (
                answered(&pending),
                events_named(&events, "reload_refused").len(),
                events_named(&events, "reload_started").len(),
            ),
            (
                (
                    "NG:Unknown target (ghost / shiori / shell / balloon / makoto / descript)"
                        .to_string(),
                    true
                ),
                0,
                0,
            ),
            "target={target:?}"
        );
    }
}

/// makoto（大小を区別しない）は `NG:Reloading makoto is not supported yet` をその場で返し、
/// `TARGET_ERROR` の `warn!(reload_makoto_unsupported)` を 1 件残す。窓口へは渡さない。
#[test]
fn makoto_answers_not_supported_yet_with_one_warn() {
    for target in ["makoto", "MaKoTo"] {
        let (pending, events) = call(&mut World::new(), target);
        let warns = events_named(&events, "reload_makoto_unsupported")
            .into_iter()
            .map(|e| {
                (
                    e.level,
                    e.target.clone(),
                    e.field("ghost").map(str::to_owned),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            (
                answered(&pending),
                warns,
                events_named(&events, "reload_refused").len()
            ),
            (
                ("NG:Reloading makoto is not supported yet".to_string(), true),
                vec![(
                    Level::WARN,
                    TARGET_ERROR.to_owned(),
                    Some("Emily/Phase4.5".to_owned())
                )],
                0,
            ),
            "target={target:?}"
        );
    }
}

/// 5 語（大小違いも）は窓口へ「MCP」の出どころで渡る。実行系の無い World では窓口が断り
/// （`reload_refused`・reason=no_context）、`NG:Cannot reload now (…)` をその場で返す。
#[test]
fn desk_refusal_answers_cannot_reload_now() {
    for target in ["ghost", "Shiori", "SHELL", "balloon", "descript"] {
        let (pending, events) = call(&mut World::new(), target);
        let refused = events_named(&events, "reload_refused")
            .into_iter()
            .map(|e| {
                (
                    e.field_str("reason").map(str::to_owned),
                    e.field("origin").map(str::to_owned),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            (answered(&pending), refused),
            (
                (
                    "NG:Cannot reload now (another switch, reload, update or install is in progress)"
                        .to_string(),
                    true
                ),
                vec![(Some("no_context".to_owned()), Some("Mcp".to_owned()))],
            ),
            "target={target:?}"
        );
    }
}

/// 終了の途中は返事を送らずに落とす（橋が既存の "shutting down" で答える）。
#[test]
fn closing_drops_the_reply() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    areka_session::reload::register(&mut world);
    let _ = areka_session::exit_wait::begin_close(&mut world);

    let (pending, events) = call(&mut world, "shell");

    assert!(matches!(pending.try_answer(), Err(ReplyError::Dropped)));
    assert_eq!(events_named(&events, "reload_started").len(), 0);
}

/// 受理は窓口の判定の直後に読んだ番号を `since_id` に載せた SSP の文（isError: false）で答え、断りは
/// `NG:Cannot reload now (…)`、終了の途中は答えを持たない。番号は窓口を
/// 呼ぶ前でなく後に読む（窓口が積んだ `reload_started` より後ろだけを `get_log` で読ませる）。
#[test]
fn accepted_answers_ssp_text_with_last_id_read_after_the_desk() {
    let id = Cell::new(41_u64);
    let asked = Cell::new(None);
    let accepted = answer(
        "Descript",
        &ghost(),
        |target| {
            asked.set(Some(target));
            id.set(id.get() + 1);
            ReloadVerdict::Accepted
        },
        || id.get(),
    );
    assert_eq!(
        (accepted, asked.take()),
        (
            Some(outcome::ok(
                "reload requested. It runs asynchronously; errors while loading (dictionary errors etc.) will be recorded to the error log. Wait until get_status answers again, then check get_log tool (log_type=error, since_id=42)"
            )),
            Some(ReloadTarget::Descript(
                areka_runtime::reload_args::DescriptParts::ALL
            )),
        )
    );

    let refused = answer(
        "ghost",
        &ghost(),
        |_| ReloadVerdict::Refused(areka_session::reload::RefusedReason::GhostSwitching),
        || panic!("断りでは番号を読まない"),
    );
    let closing = answer(
        "shiori",
        &ghost(),
        |_| ReloadVerdict::Closing,
        || panic!("終了の途中では番号を読まない"),
    );
    assert_eq!(
        (refused, closing),
        (
            Some(outcome::ng(
                "Cannot reload now (another switch, reload, update or install is in progress)"
            )),
            None
        )
    );
}
