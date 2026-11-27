use super::*;
use crate::asker::{AskerContext, AskerId};
use crate::persist::{Axis, FakePersistIo, PersistKey, PersistScope, ScopeRoots};
use crate::value::{DottedResolution, FlatResolution};
use crate::vocab::DegradePolicy;
use std::path::PathBuf;

fn ctx(id: &str) -> AskerContext {
    AskerContext {
        asker: AskerId::new(id),
    }
}

fn a(id: &str) -> AskerId {
    AskerId::new(id)
}

/// 空 roots＋fake IO で起動する（本番結線を模した最小 init）。
fn init_empty() -> SylphyaInit {
    SylphyaInit {
        roots: ScopeRoots::default(),
        io: Box::new(FakePersistIo::new()),
    }
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
    // barrier 復帰 = 上記 publish が鏡像へ反映済み（mpsc FIFO＋直列処理）。
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

// === 起動時ロード: publish なしで永続復元値が読める（init 投影・reader 無ブロックも兼ねる）===

#[test]
fn initial_load_projects_persist_into_dotted_without_publish() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X,
                },
                "100".into(),
            ),
            (PersistKey::BootCount, "7".into()),
        ],
    );
    let parts = spawn_sylphya(SylphyaInit {
        roots,
        io: Box::new(io),
    });

    // publish も barrier もせず即読み: 初期像は spawn 内で同期構築済みゆえレースなく見える
    // （＝読み経路はアクター処理を一切待たない）。
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "areka.window.scope(0).x"),
        DottedResolution::Value("100".into())
    );
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "areka.boot.count"),
        DottedResolution::Value("7".into())
    );
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

// === SET RuntimeCommand は M1 未配線（鏡像へ書込まない・呼出は非 panic）===

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

// === PublishShiori Some/None（None は不在＝既定値縮退・鏡像へ書かない）===

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
    let shared = std::sync::Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(shared.clone()),
    });
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![
            (PersistKey::BootCount, "5".into()),
            (PersistKey::VanishCount, "2".into()),
        ],
    );
    // barrier 復帰 = put の write-through 保存（save_scope）まで完了。
    parts.publisher.barrier().expect("barrier");

    // アクターと同じ store を別ハンドルの load_scope で観測（実 IO 通過の証明）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &shared);
    assert!(
        loaded.contains(&(PersistKey::BootCount, "5".into())),
        "write-through が実 IO へ確定していない: {loaded:?}"
    );
    assert!(
        loaded.contains(&(PersistKey::VanishCount, "2".into())),
        "write-through が実 IO へ確定していない: {loaded:?}"
    );
}

// === アクター死亡（Close→join）: 送信は SendError（非 panic）・barrier は Err・reader 継続 ===

#[test]
fn actor_death_via_close_makes_sends_observable_and_reader_continues() {
    let SylphyaParts {
        reader,
        publisher,
        handle,
    } = spawn_sylphya(init_empty());
    // 既知値を確立（後で最終鏡像として読めることを確認するため）。
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

    // reader は最後に publish された鏡像を保持し続ける（死後投函は反映されない）。
    assert_eq!(
        reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value("生前値".into())
    );
}

/// 確定で必ず panic する IO（アクター death by panic を決定論的に誘発する）。
struct PanicIo;
impl crate::persist::PersistIo for PanicIo {
    fn read(&self, _path: &std::path::Path) -> std::io::Result<Option<String>> {
        Ok(None)
    }
    fn commit(&self, _path: &std::path::Path, _content: &str) -> std::io::Result<()> {
        panic!("injected persist commit panic");
    }
}

// === アクター死亡（panic→join 検出）: join が Panicked・reader は最終鏡像で継続 ===

#[test]
fn actor_panic_is_detected_by_join_and_reader_continues() {
    let SylphyaParts {
        reader,
        publisher,
        handle,
    } = spawn_sylphya(SylphyaInit {
        roots: ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        },
        io: Box::new(PanicIo),
    });
    // 先に既知値を確立（panic 前の最終鏡像）。
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

    // reader は panic 前の最終鏡像を読み続けられる（表示系を殺さない）。
    assert_eq!(
        reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value("生前値".into())
    );
    // 死亡後の barrier は Err（ハングしない）。
    assert!(
        publisher.barrier().is_err(),
        "dead actor barrier must be Err"
    );
}

// === reader 無ブロック（構造＋挙動）: アクター処理前でも読みは即確定・大域ロック不在 ===

#[test]
fn reader_does_not_block_on_actor_supply() {
    let parts = spawn_sylphya(init_empty());
    // 一切の barrier なしで即読み: 読み経路は鏡像 read lock 内 Arc clone のみで、アクターの
    // メッセージ処理・チャネル送受信を一切待たない（大域ロック不在）。
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
    // reader は clone 可（複数消費エンジンが同一鏡像を共有・供給と独立）。
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

// === 鍵を消す投函: 記憶から消え、ほかの鍵は残る・像は出さない ===

#[test]
fn persist_remove_drops_the_key_from_file_and_keeps_others() {
    let shared = std::sync::Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let (parts, mirror) = spawn_parts(
        SylphyaInit {
            roots: roots.clone(),
            io: Box::new(shared.clone()),
        },
        None,
    );
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

    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &shared),
        vec![(PersistKey::BootCount, "3".into())],
        "記憶からも消え、ほかの鍵は残る"
    );
    assert_eq!(mirror.load().epoch, 0, "覚える値の書き・消しは像を出さない");

    parts.publisher.close();
    parts.handle.join().expect("join");
}
