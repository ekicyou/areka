//! 置換語 2 つの値の決定論テスト（design「areka / install / names」・要件 6.8・6.9・12.11）。
//!
//! 確かめること: 入れる前は置き換えない・入れた後は台詞の `%lastghostname`／`%lastobjectname` が
//! 置き換わる・バルーンだけなら `%lastghostname` は前の値のまま・複数なら最後のゴースト・
//! ゴーストを起こし直した後も載せ直される。
//!
//! 置き換わるかは本番と同じ道筋で見る: 載せる口で記憶の書き手（本物の sylphya）へ載せ、反映の柵を
//! 掛けてから、台詞の開始で凍結する表（`from_sylphya_provider`＝`talk_snapshot`）を作り、
//! `resolve_system_var` で展開する。起こし直しは偽の SHIORI の土台で本物の起動の結線を通す。

use areka_ghost::sylphya_wiring::{from_sylphya_provider, spawn_ghost_sylphya};
use areka_sakura::sysvar::{ResolvedVar, resolve_system_var};
use areka_sylphya::ScopeRoots;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::*;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

/// 値を本物の記憶の書き手へ載せ、台詞の開始の表で 2 語を展開する（`%lastghostname`, `%lastobjectname`）。
fn resolved(names: &LastInstallNames) -> (ResolvedVar, ResolvedVar) {
    let parts = spawn_ghost_sylphya(ScopeRoots::default());
    let asker = AskerId::new("ghost/install-names");
    publish(&parts.publisher, asker.clone(), names);
    parts.publisher.barrier().expect("書き手は生きている");
    let snapshot = from_sylphya_provider(parts.reader.clone(), asker)();
    parts.publisher.close();
    let _ = parts.handle.join();
    (
        resolve_system_var("lastghostname", &snapshot),
        resolve_system_var("lastobjectname", &snapshot),
    )
}

fn text(s: &str) -> ResolvedVar {
    ResolvedVar::Text(s.to_owned())
}

fn raw(name: &str) -> ResolvedVar {
    ResolvedVar::PassThrough(format!("%{name}"))
}

fn names_of(world: &World) -> Option<LastInstallNames> {
    world.get_resource::<LastInstallNames>().cloned()
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 入れる前: 値は World に無く、載せる語も無い＝台詞の 2 語はそのまま通る（要件 6.9）。
#[test]
fn before_any_install_nothing_is_replaced() {
    let world = World::new();
    assert_eq!(names_of(&world), None, "入れるまで World に無い");
    assert_eq!(
        resolved(&LastInstallNames::default()),
        (raw("lastghostname"), raw("lastobjectname"))
    );
}

/// ゴーストを入れた後: 2 語が置き換わり、`info!(install_names_updated)` が 1 件（要件 6.8）。
#[test]
fn after_a_ghost_install_both_words_are_replaced() {
    let mut world = World::new();
    let ((), events) = capture(|| {
        update(
            &mut world,
            "あたらしいゴースト".to_owned(),
            Some("ニュービー".to_owned()),
        )
    });
    let names = names_of(&world).expect("入れた後は World に在る");
    assert_eq!(
        names,
        LastInstallNames {
            ghost: Some("ニュービー".to_owned()),
            object: Some("あたらしいゴースト".to_owned()),
        }
    );
    assert_eq!(
        resolved(&names),
        (text("ニュービー"), text("あたらしいゴースト"))
    );
    let updated = events_named(&events, "install_names_updated");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].level, Level::INFO);
    assert_eq!(updated[0].field_str("ghost"), Some("ニュービー"));
    assert_eq!(updated[0].field_str("object"), Some("あたらしいゴースト"));
}

/// バルーンだけを入れた: `%lastobjectname` はバルーン、`%lastghostname` は前の値のまま（要件 6.8）。
/// まだゴーストの値が無ければ `%lastghostname` は置き換えない。
#[test]
fn a_balloon_only_install_keeps_the_previous_ghost_name() {
    let mut world = World::new();
    update(&mut world, "ふわふわ".to_owned(), None);
    let first = names_of(&world).expect("入れた後は World に在る");
    assert_eq!(first.ghost, None);
    assert_eq!(resolved(&first), (raw("lastghostname"), text("ふわふわ")));

    update(&mut world, "ゴースト甲".to_owned(), Some("甲".to_owned()));
    update(&mut world, "もこもこ".to_owned(), None);
    let names = names_of(&world).unwrap();
    assert_eq!(resolved(&names), (text("甲"), text("もこもこ")));
}

/// 続けて入れたら最後のもの。シェル・追加ファイルは宛先のゴーストの名前で置き換わる（要件 6.4・6.8）。
#[test]
fn several_installs_leave_the_last() {
    let mut world = World::new();
    update(&mut world, "ゴースト甲".to_owned(), Some("甲".to_owned()));
    update(&mut world, "ゴースト乙".to_owned(), Some("乙".to_owned()));
    assert_eq!(
        names_of(&world),
        Some(LastInstallNames {
            ghost: Some("乙".to_owned()),
            object: Some("ゴースト乙".to_owned()),
        })
    );
    update(&mut world, "乙の服".to_owned(), Some("乙".to_owned()));
    let names = names_of(&world).unwrap();
    assert_eq!(resolved(&names), (text("乙"), text("乙の服")));
}

/// 起こし直し: 入れる前に起こしたゴーストへは載せない。入れた後は、起こし直したゴーストの記憶の
/// 書き手（そのゴーストの問い合わせ元）へ値を載せ直す（`boot_wired` の 1 行・設計で決めたこと 10）。
#[test]
fn reboot_reseeds_the_values_into_the_new_ghost() {
    let mut rig = SwitchRig::new(vec![
        (
            "A",
            FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
        ),
        (
            "B",
            FakeShiori::Scripted(Box::new(|| standard_script("\\0B\\e"))),
        ),
    ]);
    let ((), before) = capture(|| rig.boot("A"));
    assert!(
        events_named(&before, "install_names_reseeded").is_empty(),
        "入れる前は載せない"
    );

    let ((), updated) = capture(|| {
        update(
            &mut rig.world,
            "ゴースト甲".to_owned(),
            Some("甲".to_owned()),
        )
    });
    let updated = events_named(&updated, "install_names_updated");
    assert_eq!(updated.len(), 1);
    assert_eq!(
        updated[0].field("published"),
        Some("true"),
        "今のゴーストへ載せた"
    );
    assert!(rig.shutdown(), "A を降ろす");

    let ((), after) = capture(|| rig.boot("B"));
    let reseeded = events_named(&after, "install_names_reseeded");
    assert_eq!(reseeded.len(), 1, "起こしたら 1 回載せ直す");
    assert_eq!(reseeded[0].field_str("ghost"), Some("甲"));
    assert_eq!(reseeded[0].field_str("object"), Some("ゴースト甲"));
    let asker = reseeded[0].field_str("asker").expect("問い合わせ元が載る");
    assert_eq!(
        std::path::Path::new(asker),
        rig.root.ghost_dir("B").join("ghost").join("master"),
        "起こし直した B の問い合わせ元（B の SHIORI のフォルダ）へ載せる"
    );
    assert!(rig.shutdown(), "B を降ろす");
}
