use super::*;
use crate::asker::{AskerContext, AskerId};
use crate::guardian::wait_tests::{Hooked, ask};
use crate::keytable::Home;
use crate::persist::{Axis, PersistKey, PersistScope, ScopeRoots};
use crate::store::{FileWrite, MemBackend, Store, StoreBackend};
use crate::value::{DottedResolution, FlatResolution};
use crate::vocab::DegradePolicy;
use std::path::PathBuf;
use std::sync::Arc;

fn ctx(id: &str) -> AskerContext {
    AskerContext {
        asker: AskerId::new(id),
    }
}

fn a(id: &str) -> AskerId {
    AskerId::new(id)
}

/// 空 roots＋偽物の置き場の口で起動する（本番結線を模した最小 init）。
fn ghost_roots() -> ScopeRoots {
    ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    }
}

/// 番人とは別の核で、ゴーストの層のファイルを読む。
fn read_ghost_file(backend: Arc<dyn StoreBackend>, key: &str) -> Option<String> {
    Store::open(ghost_roots(), backend).unwrap().read(
        &a("ghost/a"),
        key,
        Some(PersistScope::Ghost),
        Home::File,
    )
}

fn init_empty() -> SylphyaInit {
    SylphyaInit::new(ScopeRoots::default(), MemBackend::new())
}

// === 配線: spawn → publish → barrier → read（barrier がフェンス）===

#[test]
fn spawn_publish_static_barrier_then_read_sees_value() {
    let parts = spawn_sylphya(init_empty());
    parts.publisher.publish_static(
        a("ghost/a"),
        vec![("selfname".into(), "さくら".into())],
        vec![("baseware.name".into(), "areka".into())],
    );
    // barrier 復帰 = 上記 publish が片づいている（mpsc FIFO＋直列処理）。
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value("さくら".into())
    );
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "baseware.name"),
        DottedResolution::Value("areka".into())
    );
}

// === 起動の前にファイルに在る値が、番人への読みで読める ===

#[test]
fn values_already_in_the_file_are_read_through_the_guardian() {
    let mem = MemBackend::new();
    let roots = ghost_roots();
    let mut seed = Store::open(roots.clone(), mem.clone()).unwrap();
    let _ = seed.try_write(&FileWrite {
        layer: PersistScope::Ghost,
        root: roots.ghost.clone(),
        puts: vec![
            ("areka.window.scope(0).x".into(), "100".into()),
            ("areka.boot.count".into(), "7".into()),
        ],
        removes: Vec::new(),
        typed: Vec::new(),
    });
    let parts = spawn_sylphya(SylphyaInit::new(roots, mem));

    for (key, value) in [
        ("areka.window.scope(0).x", "100"),
        ("areka.boot.count", "7"),
    ] {
        assert_eq!(
            ask(&parts.publisher, a("ghost/a"), None, key, Home::File).as_deref(),
            Some(value)
        );
    }
    parts.publisher.close();
    parts.handle.join().expect("join");
}

// === SET StoreWrite が host 点付き区画へ反映（自由 key）===

#[test]
fn set_free_key_store_writes_to_host_dotted_region() {
    let parts = spawn_sylphya(init_empty());
    parts
        .publisher
        .set(a("ghost/a"), "myplugin.customstate".into(), "on".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "myplugin.customstate"),
        DottedResolution::Value("on".into())
    );
    // 別 asker からは見えない（per-asker host 区画）。
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/b"), "myplugin.customstate"),
        DottedResolution::NotFound
    );
}

// === SET RuntimeCommand は M1 未配線（書込まない・呼出は非 panic）===

#[test]
fn set_runtime_command_vocab_writes_nothing_in_m1() {
    let parts = spawn_sylphya(init_empty());
    parts
        .publisher
        .set(a("ghost/a"), "surface.num".into(), "5".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "surface.num"),
        DottedResolution::NotFound
    );
}

// === PublishShiori Some/None（None は不在＝既定値縮退・書かない）===

#[test]
fn publish_shiori_some_sets_flat_none_records_absence() {
    let parts = spawn_sylphya(init_empty());
    parts
        .publisher
        .publish_shiori(a("ghost/a"), "username".into(), Some("Alice".into()));
    parts
        .publisher
        .publish_shiori(a("ghost/b"), "username".into(), None);
    parts.publisher.barrier().expect("barrier");

    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "username"),
        FlatResolution::Value("Alice".into())
    );
    // None → 不在記録のみ（既定値は書かない）→ 台帳の ConsumerDefault へ縮退。
    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/b"), "username"),
        FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
    );
}

// === barrier 順序フェンス: 連投の最後値が反映（直列 FIFO 処理の核檻）===

#[test]
fn barrier_fences_all_prior_messages_last_write_wins() {
    let parts = spawn_sylphya(init_empty());
    for i in 0..10u32 {
        parts.publisher.publish_static(
            a("ghost/a"),
            vec![("counter".into(), i.to_string())],
            vec![],
        );
    }
    parts.publisher.barrier().expect("barrier");
    // barrier 復帰時、連投全件が反映済み＝最後の値が見える（直列 FIFO の証明）。
    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "counter"),
        FlatResolution::Value("9".into())
    );
}

// === write-through persist put が実 IO へ確定・独立ロードで復元（往復・root あり）===

#[test]
fn persist_put_write_through_commits_to_real_io() {
    let mem = MemBackend::new();
    let parts = spawn_sylphya(SylphyaInit::new(ghost_roots(), mem.clone()));
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![
            (PersistKey::BootCount, "5".into()),
            (PersistKey::VanishCount, "2".into()),
        ],
    );
    // barrier 復帰 = put の書きが確定済み。
    parts.publisher.barrier().expect("barrier");

    // 番人とは別の核で観測（ファイルへ確定した証明）。
    assert_eq!(
        read_ghost_file(mem.clone(), "areka.boot.count").as_deref(),
        Some("5")
    );
    assert_eq!(
        read_ghost_file(mem, "areka.vanish.count").as_deref(),
        Some("2")
    );
}

// === アクター死亡（Close→join）: 送信は SendError（非 panic）・barrier は Err・読みは縮退 ===

#[test]
fn actor_death_via_close_makes_sends_observable_and_reads_degrade() {
    let SylphyaParts {
        reader,
        publisher,
        handle,
    } = spawn_sylphya(init_empty());
    // 死後に生前の値が読めないことを確かめるため、先に値を入れておく。
    publisher.publish_static(
        a("ghost/a"),
        vec![("selfname".into(), "生前値".into())],
        vec![],
    );
    publisher.barrier().expect("barrier while alive");

    // 正典終了経路: Close → join で停止完了を待つ（join 復帰 = body return = rx drop 済み）。
    publisher.close();
    handle
        .join()
        .expect("clean Close should join without panic");

    // 死亡後の fire-and-forget 投函は panic しない（SendError → warn＋縮退）。
    publisher.publish_static(
        a("ghost/a"),
        vec![("selfname".into(), "死後値".into())],
        vec![],
    );
    publisher.set(a("ghost/a"), "myplugin.x".into(), "y".into());
    publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "9".into())],
    );

    // 死亡後の barrier は Err（送信端で死亡を観測可能・ハングしない）。
    assert!(
        matches!(publisher.barrier(), Err(areka_actor::ReplyError::Dropped)),
        "dead actor barrier must surface ReplyError, not hang"
    );

    // 止まった番人への読みは panic せず、値なし（縮退の決まり）を返す。
    assert_eq!(
        reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
    );
}

// === アクター死亡（panic→join 検出）: join が Panicked・読みは縮退 ===

#[test]
fn actor_panic_is_detected_by_join_and_reads_degrade() {
    let SylphyaParts {
        reader,
        publisher,
        handle,
    } = spawn_sylphya(SylphyaInit::new(
        ghost_roots(),
        Arc::new(Hooked {
            inner: MemBackend::new(),
            volatile_uri: None,
            panic_on_write: true,
        }),
    ));
    // 死後に生前の値が読めないことを確かめるため、先に値を入れておく。
    publisher.publish_static(
        a("ghost/a"),
        vec![("selfname".into(), "生前値".into())],
        vec![],
    );
    publisher.barrier().expect("barrier before panic");

    // 永続 put → 確定で panic 誘発。
    publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "1".into())],
    );

    // join がアクタースレッドの panic を握り潰さず観測（バグ観測・areka-actor 規約 4）。
    let err = handle
        .join()
        .expect_err("panicking actor must surface at join");
    assert!(
        matches!(err, areka_actor::ActorError::Panicked { .. }),
        "expected Panicked, got {err:?}"
    );

    // 落ちた番人への読みは panic せず、値なし（縮退の決まり）を返す（表示系を殺さない）。
    assert_eq!(
        reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
    );
    // 死亡後の barrier は Err（ハングしない）。
    assert!(
        publisher.barrier().is_err(),
        "dead actor barrier must be Err"
    );
}

// === barrier なしの読み: 値の無い名前は縮退の決まりで返り、複製した reader も同じ ===

#[test]
fn reads_without_a_barrier_return_by_the_degrade_rule() {
    let parts = spawn_sylphya(init_empty());
    // barrier なしで読んでも、値の無い名前は縮退の決まりで返る。
    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "username"),
        FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
    );
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "system.none"),
        DottedResolution::NotFound
    );
    // reader は clone 可（複数消費エンジンが同じ番人へ聞く）。
    let reader2 = parts.reader.clone();
    assert_eq!(
        reader2.resolve_flat(&ctx("ghost/a"), "username"),
        FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
    );
}

// === アクター統合の決定論反復（スレッド配線が flake しないこと）===

#[test]
fn spawn_publish_barrier_read_is_deterministic_over_iterations() {
    for i in 0..20u32 {
        let parts = spawn_sylphya(init_empty());
        parts.publisher.publish_static(
            a("ghost/a"),
            vec![("selfname".into(), format!("v{i}"))],
            vec![],
        );
        parts.publisher.barrier().expect("barrier");
        assert_eq!(
            parts.reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Value(format!("v{i}"))
        );
        parts.publisher.close();
        parts.handle.join().expect("join");
    }
}

// === 鍵を消す投函: 記憶から消え、ほかの鍵は残る ===

#[test]
fn persist_remove_drops_the_key_from_file_and_keeps_others() {
    let mem = MemBackend::new();
    let parts = spawn_sylphya(SylphyaInit::new(ghost_roots(), mem.clone()));
    let x = PersistKey::BalloonOffset {
        scope: 0,
        axis: Axis::X,
    };
    let y = PersistKey::BalloonOffset {
        scope: 0,
        axis: Axis::Y,
    };
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![
            (x, "5".into()),
            (y, "6".into()),
            (PersistKey::BootCount, "3".into()),
        ],
    );
    parts
        .publisher
        .persist_remove(PersistScope::Ghost, vec![x, y]);
    parts.publisher.barrier().expect("barrier");

    for key in [
        "areka.balloon.offset.scope(0).x",
        "areka.balloon.offset.scope(0).y",
    ] {
        assert_eq!(read_ghost_file(mem.clone(), key), None, "記憶から消える");
    }
    assert_eq!(
        read_ghost_file(mem, "areka.boot.count").as_deref(),
        Some("3"),
        "ほかの鍵は残る"
    );

    parts.publisher.close();
    parts.handle.join().expect("join");
}
