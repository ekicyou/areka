use super::*;
use crate::asker::AskerId;
use crate::persist::ScopeRoots;
use crate::store::MemBackend;

fn a(id: &str) -> AskerId {
    AskerId::new(id)
}

// === 死亡後 send／barrier の WARN 記録（無音失敗禁止）===

/// アクター死亡後の投函縮退が **無音でない**（WARN を出す）ことを檻化する。
///
/// 死亡後の fire-and-forget send（[`SylphyaPublisher::send`] 経由）と barrier
/// （[`SylphyaPublisher::barrier`]）はいずれも `SendError` を warn 記録して縮退する
/// （panic せず・無音でもない）。[`crate::test_log_capture::capture`]（共有機構 `log-capture-kit`
/// への委譲で並列負荷下の Interest::never 焼き付きを根絶・決定論）で両縮退 WARN を捕捉して照合する。
/// 既存 [`actor_integration_tests::actor_death_via_close_makes_sends_observable_and_reads_degrade`]
/// は「panic しない・barrier Err」を突くが、WARN の存在（無音失敗禁止）は本檻が初出で突く。
#[test]
fn send_after_death_logs_warn_not_silent() {
    use crate::test_log_capture::{assert_logged, capture};

    let parts = spawn_sylphya(SylphyaInit::new(ScopeRoots::default(), MemBackend::new()));
    // 正典終了経路で確実に畳む（join 復帰 = body return = rx drop 済み = 以降 send は SendError）。
    parts.publisher.close();
    parts
        .handle
        .join()
        .expect("clean close joins without panic");

    // 死亡後の fire-and-forget 投函: SendError → WARN 記録して縮退（無音でない）。
    let send_events = capture(|| {
        parts.publisher.publish_static(
            a("ghost/a"),
            vec![("selfname".into(), "死後値".into())],
            vec![],
        );
        parts
            .publisher
            .set(a("ghost/a"), "myplugin.x".into(), "y".into());
    });
    assert_logged(
        &send_events,
        tracing::Level::WARN,
        LOG_TARGET,
        "actor stopped; message dropped",
    );

    // 死亡後の barrier: 投函不能 → WARN 記録＋`ReplyError::Dropped`（ハングしない・無音でない）。
    let barrier_events = capture(|| {
        let r = parts.publisher.barrier();
        assert!(
            matches!(r, Err(areka_actor::ReplyError::Dropped)),
            "dead-actor barrier must surface ReplyError::Dropped"
        );
    });
    assert_logged(
        &barrier_events,
        tracing::Level::WARN,
        LOG_TARGET,
        "barrier could not be posted",
    );
}
