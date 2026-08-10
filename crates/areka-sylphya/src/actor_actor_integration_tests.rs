use super::*;
use crate::asker::{AskerContext, AskerId};
use crate::persist::{Axis, FakePersistIo, PersistKey, PersistScope, ScopeRoots};
use crate::value::{DottedResolution, FlatResolution};
use crate::vocab::DegradePolicy;
use std::path::PathBuf;

fn ctx(id: &str) -> AskerContext {
    AskerContext { asker: AskerId::new(id) }
}

fn a(id: &str) -> AskerId {
    AskerId::new(id)
}

/// 空 roots＋fake IO で起動する（本番結線を模した最小 init）。
fn init_empty() -> SylphyaInit {
    SylphyaInit {
        roots: ScopeRoots::default(),
        io: Box::new(FakePersistIo::new()),
        runtime_sink: None,
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
    parts.publisher.barrier().expect("barrier should resolve while actor is alive");

    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value("さくら".into())
    );
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "baseware.name"),
        DottedResolution::Value("areka".into())
    );
}

// === 起動時ロード: publish なしで永続復元値が読める（init 投影・reader 無ブロックも兼ねる）===

#[test]
fn initial_load_projects_persist_into_dotted_without_publish() {
    // fake IO に ghost スコープを事前確定（窓位置＋起動記録）。
    let io = FakePersistIo::new();
    let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![
            (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "100".into()),
            (PersistKey::BootCount, "7".into()),
        ],
    );
    let parts = spawn_sylphya(SylphyaInit {
        roots,
        io: Box::new(io),
        runtime_sink: None,
    });

    // publish も barrier もせず即読み: 初期像は spawn 内で同期構築済みゆえレースなく見える
    // （＝読み経路はアクター処理を一切待たない・R2.7 無ブロック）。
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "areka.window.scope(0).x"),
        DottedResolution::Value("100".into())
    );
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "areka.boot.count"),
        DottedResolution::Value("7".into())
    );
}

// === SET StoreWrite が host 点付き区画へ反映（自由 key）===

#[test]
fn set_free_key_store_writes_to_host_dotted_region() {
    let parts = spawn_sylphya(init_empty());
    parts.publisher.set(a("ghost/a"), "myplugin.customstate".into(), "on".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "myplugin.customstate"),
        DottedResolution::Value("on".into())
    );
    // 別 asker からは見えない（per-asker host 区画）。
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/b"), "myplugin.customstate"),
        DottedResolution::NotFound
    );
}

// === SET RuntimeCommand は M1 未配線（鏡像へ書込まない・呼出は非 panic）===

#[test]
fn set_runtime_command_vocab_writes_nothing_in_m1() {
    let parts = spawn_sylphya(init_empty());
    parts.publisher.set(a("ghost/a"), "surface.num".into(), "5".into());
    parts.publisher.barrier().expect("barrier");
    // RuntimeCommand は reserved seam（sink 未登録）→ どの区画にも載らない。
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "surface.num"),
        DottedResolution::NotFound
    );
}

// === PublishShiori Some/None（None は不在＝既定値縮退・鏡像へ書かない）===

#[test]
fn publish_shiori_some_sets_flat_none_records_absence() {
    let parts = spawn_sylphya(init_empty());
    parts.publisher.publish_shiori(a("ghost/a"), "username".into(), Some("Alice".into()));
    parts.publisher.publish_shiori(a("ghost/b"), "username".into(), None);
    parts.publisher.barrier().expect("barrier");

    // Some → per-asker フラットへ値が載る。
    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "username"),
        FlatResolution::Value("Alice".into())
    );
    // None → 不在記録のみ（既定値は書かない）→ 台帳の ConsumerDefault へ縮退（R4.2）。
    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/b"), "username"),
        FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
    );
}

// === barrier 順序フェンス: 連投の最後値が反映（直列 FIFO 処理の核檻）===

#[test]
fn barrier_fences_all_prior_messages_last_write_wins() {
    let parts = spawn_sylphya(init_empty());
    // 同一 key へ 0..10 を連投（FIFO 直列なら最後の 9 が最終値）。
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

// === write-through persist put が鏡像 areka.* へ投影（root None でも投影は成る・save は縮退）===

#[test]
fn persist_put_projects_to_mirror_areka_region() {
    let parts = spawn_sylphya(init_empty()); // roots 空 → save は Degraded だが鏡像投影は成る。
    parts
        .publisher
        .persist_put(PersistScope::Ghost, vec![(PersistKey::BootCount, "3".into())]);
    parts.publisher.barrier().expect("barrier");
    // 保存先 root が None でも（save_scope は Degraded・warn）鏡像 dotted 投影は独立に成立し、
    // アクターは panic せず継続する（R6.7）。
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "areka.boot.count"),
        DottedResolution::Value("3".into())
    );
}

/// 同一 [`FakePersistIo`] を `Arc` 共有する委譲 IO（テスト専用・安全）。
///
/// `FakePersistIo` は内部 `Mutex` で Clone 不可ゆえ、write-through が実 IO シームを通ったことは
/// 「アクターへ渡した IO と同じ store」を別ハンドルの [`load_scope`] で観測して確認する。
/// `Arc` 共有で unsafe を用いずに実現する。
#[derive(Clone)]
struct SharedFakeIo(std::sync::Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &std::path::Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &std::path::Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

// === write-through persist put が実 IO へ確定・独立ロードで復元（往復・root あり）===

#[test]
fn persist_put_write_through_commits_to_real_io() {
    let shared = SharedFakeIo(std::sync::Arc::new(FakePersistIo::new()));
    let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(shared.clone()),
        runtime_sink: None,
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
    // 鏡像 areka.* 投影も成る（write-through の両面）。
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "areka.boot.count"),
        DottedResolution::Value("5".into())
    );
}

// === アクター死亡（Close→join）: 送信は SendError（非 panic）・barrier は Err・reader 継続 ===

#[test]
fn actor_death_via_close_makes_sends_observable_and_reader_continues() {
    let SylphyaParts { reader, publisher, handle } = spawn_sylphya(init_empty());
    // 既知値を確立（後で最終鏡像として読めることを確認するため）。
    publisher.publish_static(a("ghost/a"), vec![("selfname".into(), "生前値".into())], vec![]);
    publisher.barrier().expect("barrier while alive");

    // 正典終了経路: Close → join で停止完了を待つ（join 復帰 = body return = rx drop 済み）。
    publisher.close();
    handle.join().expect("clean Close should join without panic");

    // 死亡後の fire-and-forget 投函は panic しない（SendError → warn＋縮退）。
    publisher.publish_static(a("ghost/a"), vec![("selfname".into(), "死後値".into())], vec![]);
    publisher.set(a("ghost/a"), "myplugin.x".into(), "y".into());
    publisher.persist_put(PersistScope::Ghost, vec![(PersistKey::BootCount, "9".into())]);

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

/// dispatch で必ず panic する運行 sink（アクター death by panic を決定論的に誘発する）。
struct PanicSink;
impl RuntimeCommandSink for PanicSink {
    fn dispatch(&self, _asker: &AskerId, _key: &str, _value: &str) {
        panic!("injected runtime-command dispatch panic");
    }
}

// === アクター死亡（panic→join 検出）: join が Panicked・reader は最終鏡像で継続 ===

#[test]
fn actor_panic_is_detected_by_join_and_reader_continues() {
    let SylphyaParts { reader, publisher, handle } = spawn_sylphya(SylphyaInit {
        roots: ScopeRoots::default(),
        io: Box::new(FakePersistIo::new()),
        runtime_sink: Some(Box::new(PanicSink)),
    });
    // 先に既知値を確立（panic 前の最終鏡像）。
    publisher.publish_static(a("ghost/a"), vec![("selfname".into(), "生前値".into())], vec![]);
    publisher.barrier().expect("barrier before panic");

    // RuntimeCommand SET → apply が RuntimeCommandReserved → sink.dispatch で panic 誘発。
    publisher.set(a("ghost/a"), "surface.num".into(), "5".into());

    // join がアクタースレッドの panic を握り潰さず観測（バグ観測・areka-actor 規約 4）。
    let err = handle.join().expect_err("panicking actor must surface at join");
    assert!(
        matches!(err, areka_actor::ActorError::Panicked { .. }),
        "expected Panicked, got {err:?}"
    );

    // reader は panic 前の最終鏡像を読み続けられる（表示系を殺さない・R6.7）。
    assert_eq!(
        reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value("生前値".into())
    );
    // 死亡後の barrier は Err（ハングしない）。
    assert!(publisher.barrier().is_err(), "dead actor barrier must be Err");
}

// === reader 無ブロック（構造＋挙動）: アクター処理前でも読みは即確定・大域ロック不在 ===

#[test]
fn reader_does_not_block_on_actor_supply() {
    let parts = spawn_sylphya(init_empty());
    // 一切の barrier なしで即読み: 読み経路は鏡像 read lock 内 Arc clone のみで、アクターの
    // メッセージ処理・チャネル送受信を一切待たない（R2.7 無ブロック・大域ロック不在）。
    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "username"),
        FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
    );
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), "system.none"),
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
        // 正典終了で確実に畳む。
        parts.publisher.close();
        parts.handle.join().expect("join");
    }
}
