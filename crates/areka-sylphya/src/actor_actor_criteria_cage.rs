use super::*;
use crate::asker::AskerId;
use crate::mirror::{MirrorImage, SharedMirror};
use crate::persist::{FakePersistIo, ScopeRoots};

fn a(id: &str) -> AskerId {
    AskerId::new(id)
}

/// 空 roots＋fake IO でアクターを起動しつつ、鏡像ハンドル（[`SharedMirror`]）の clone を保持
/// して返す。`spawn_sylphya` の内部配線（初期像構築→`SharedMirror::new`→`spawn_actor` で
/// [`run_actor`] を回す）を忠実に再現するが、epoch を直接観測できるよう `shared` を手元に残す
/// 点のみが異なる（reader は epoch を露出しないため・設計フェンス予約シーム）。
///
/// 返す `(shared, publisher, handle)` の `shared` は初期像 epoch 0。以降のアクター publish で
/// epoch は単調増加する。
fn spawn_with_mirror_observer() -> (SharedMirror, SylphyaPublisher, areka_actor::ActorHandle) {
    // 空 roots → 初期像は投影なしの epoch 0（build_initial_image 相当）。
    let shared = SharedMirror::new(Arc::new(MirrorImage::empty()));
    let actor_shared = shared.clone();
    let io: Box<dyn PersistIo> = Box::new(FakePersistIo::new());
    let (tx, handle) = areka_actor::spawn_actor::<SylphyaMsg, _>("sylphya-epoch-cage", move |rx| {
        run_actor(rx, actor_shared, ScopeRoots::default(), io, None);
    });
    (shared, SylphyaPublisher { tx }, handle)
}

// === (A) epoch 単調増加: アクターの publish swap 経路で各変異メッセージ→後継 epoch+1 ===

/// 鏡像 epoch が **アクターの publish 経路** で単調増加する（R3.3/2.5）。
///
/// 観測条件: `SharedMirror::load().epoch`（保持した鏡像 clone・アクターが `apply` の効果列を
/// 実行して `SharedMirror::publish` で swap する経路を突く）。核檻:
/// - 各 **変異** メッセージ（PublishStatic／SET StoreWrite）は後継像を 1 つ publish し epoch を
///   厳密に +1 する（run_actor の `if mutated { publish }`）。
/// - 各 barrier 復帰時点で epoch は反映済み（フェンス）。
/// - **非変異** メッセージ（SET RuntimeCommand＝reserved seam・sink 未登録／PublishShiori{None}＝
///   不在記録）は publish を起こさず epoch を据え置く（epoch が「実変化」に対応する不変）。
#[test]
fn epoch_increments_monotonically_through_actor_publishes() {
    let (shared, publisher, handle) = spawn_with_mirror_observer();

    // 初期像 epoch 0。
    let e0 = shared.load().epoch;
    assert_eq!(e0, 0, "initial mirror epoch must be 0");

    // 変異①: PublishStatic（フラット per-asker 書込）→ 後継 epoch = e0 + 1。
    publisher.publish_static(a("ghost/a"), vec![("selfname".into(), "x".into())], vec![]);
    publisher.barrier().expect("barrier while alive");
    let e1 = shared.load().epoch;
    assert_eq!(e1, e0 + 1, "one mutating publish must advance epoch by exactly 1");

    // 変異②: SET StoreWrite（自由 key → host 点付き区画書込）→ 後継 epoch = e1 + 1。
    publisher.set(a("ghost/a"), "myplugin.k".into(), "v".into());
    publisher.barrier().expect("barrier while alive");
    let e2 = shared.load().epoch;
    assert_eq!(e2, e1 + 1, "second mutating publish must advance epoch by exactly 1");
    assert!(e2 > e1 && e1 > e0, "epoch must be strictly monotonically increasing");

    // 非変異①: SET RuntimeCommand（sink 未登録 → 予約 seam・書込なし）→ epoch 据え置き。
    publisher.set(a("ghost/a"), "surface.num".into(), "5".into());
    publisher.barrier().expect("barrier while alive");
    assert_eq!(
        shared.load().epoch,
        e2,
        "reserved runtime-command SET must not publish (no state change → epoch unchanged)"
    );

    // 非変異②: PublishShiori{None}（204/失敗 → 不在記録のみ・鏡像へ書かない）→ epoch 据え置き。
    publisher.publish_shiori(a("ghost/a"), "username".into(), None);
    publisher.barrier().expect("barrier while alive");
    assert_eq!(
        shared.load().epoch,
        e2,
        "absent-record (204) must not publish (no state change → epoch unchanged)"
    );

    // 変異③: 再度の変異で単調増加が続く（据え置き後も +1 する）。
    publisher.publish_static(a("ghost/b"), vec![("selfname".into(), "y".into())], vec![]);
    publisher.barrier().expect("barrier while alive");
    assert_eq!(shared.load().epoch, e2 + 1, "mutation after no-ops must resume +1");

    // 正典終了で確実に畳む（join で panic 不在も確認）。
    publisher.close();
    handle.join().expect("clean close joins without panic");
}

/// epoch 単調増加の決定論反復（スレッド配線が flake しないこと・R9.1）。
///
/// 連投 N 件（全変異）→ barrier → epoch == N を反復検証する（毎周新規アクター）。
#[test]
fn epoch_advance_is_deterministic_over_iterations() {
    for _ in 0..20u32 {
        let (shared, publisher, handle) = spawn_with_mirror_observer();
        for i in 0..8u32 {
            // 同一 key の連投でも各件が状態変化（値が変わる）→ 各件 +1。
            publisher.publish_static(
                a("ghost/a"),
                vec![("counter".into(), i.to_string())],
                vec![],
            );
        }
        publisher.barrier().expect("barrier");
        assert_eq!(
            shared.load().epoch,
            8,
            "8 mutating publishes must yield epoch 8 (deterministic, no flake)"
        );
        publisher.close();
        handle.join().expect("join");
    }
}

// === (B) 死亡後 send／barrier の WARN 記録（無音失敗禁止・R6.7/8.1）===

/// アクター死亡後の投函縮退が **無音でない**（WARN を出す）ことを檻化する（R8.1）。
///
/// 死亡後の fire-and-forget send（[`SylphyaPublisher::send`] 経由）と barrier
/// （[`SylphyaPublisher::barrier`]）はいずれも `SendError` を warn 記録して縮退する
/// （panic せず・無音でもない）。[`crate::test_log_capture::capture`]（interest-keeper で
/// 並列負荷下の Interest::never 焼き付きを根絶・決定論）で両縮退 WARN を捕捉して照合する。
/// 既存 [`actor_integration_tests::actor_death_via_close_makes_sends_observable_and_reader_continues`]
/// は「panic しない・barrier Err」を突くが、WARN の存在（無音失敗禁止）は本檻が初出で突く。
#[test]
fn send_after_death_logs_warn_not_silent() {
    use crate::test_log_capture::{assert_logged, capture};

    let parts = spawn_sylphya(SylphyaInit {
        roots: ScopeRoots::default(),
        io: Box::new(FakePersistIo::new()),
        runtime_sink: None,
    });
    // 正典終了経路で確実に畳む（join 復帰 = body return = rx drop 済み = 以降 send は SendError）。
    parts.publisher.close();
    parts.handle.join().expect("clean close joins without panic");

    // 死亡後の fire-and-forget 投函: SendError → WARN 記録して縮退（無音でない）。
    let send_events = capture(|| {
        parts
            .publisher
            .publish_static(a("ghost/a"), vec![("selfname".into(), "死後値".into())], vec![]);
        parts.publisher.set(a("ghost/a"), "myplugin.x".into(), "y".into());
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
