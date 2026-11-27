use super::*;
use crate::asker::AskerContext;
use crate::fast::FastHome;
use crate::key::PropPath;
use crate::keytable::{ByName, KeyMatch, Refusal};
use crate::pull::PullShape;
use crate::store::{FsBackend, MemBackend};
use crate::value::{DottedResolution, FlatResolution};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use temp_path_kit::TempPath;

const BOOT: &str = "areka.boot.count";
const FREE: &str = "myplugin.state";
const DEFAULTED: &str = "test.with.default";
const FAST_SHAPE: &str = "test.fast.scope(*).v";
const FAST_KEY: &str = "test.fast.scope(0).v";
const LOCKED: &str = "myplugin.locked";

static TEST_TABLE: &[KeyRow] = &[
    KeyRow {
        matches: KeyMatch::Shape(DEFAULTED),
        home: Some(Home::Memory),
        by_name: ByName::Writable,
        default: Some("既定"),
    },
    KeyRow {
        matches: KeyMatch::Shape(FAST_SHAPE),
        home: Some(Home::Fast),
        by_name: ByName::Writable,
        default: None,
    },
    KeyRow {
        matches: KeyMatch::Shape("areka.boot.count"),
        home: Some(Home::File),
        by_name: ByName::Refused(Refusal::FixedWriterOnly),
        default: None,
    },
    KeyRow {
        matches: KeyMatch::Shape(LOCKED),
        home: Some(Home::Memory),
        by_name: ByName::Refused(Refusal::NotSettable),
        default: None,
    },
    KeyRow {
        matches: KeyMatch::Free,
        home: Some(Home::Memory),
        by_name: ByName::Writable,
        default: None,
    },
];

fn a() -> AskerId {
    AskerId::new("ghost/a")
}

fn ctx(id: &str) -> AskerContext {
    AskerContext {
        asker: AskerId::new(id),
    }
}

fn roots_under(dir: &Path) -> ScopeRoots {
    ScopeRoots {
        app: Some(dir.join("app")),
        ghost: Some(dir.join("ghost")),
        shell: Some(dir.join("shell")),
        balloon: Some(dir.join("balloon")),
    }
}

fn mem_ghost() -> SylphyaInit {
    SylphyaInit::new(
        ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        },
        MemBackend::new(),
    )
}

fn stop(parts: SylphyaParts) {
    parts.publisher.close();
    parts.handle.join().expect("join");
}

#[test]
fn a_written_value_reads_back_without_a_barrier_from_any_reader() {
    let parts = spawn_sylphya(mem_ghost());
    let kvs = parts.reader.kvs(a());
    let other = parts.reader.clone().kvs(a());

    assert_eq!(
        kvs.put(PersistScope::Ghost, &[(BOOT, "3")]),
        WriteOutcome::Written
    );
    assert_eq!(other.get(BOOT).as_deref(), Some("3"));
    assert_eq!(
        kvs.set(PersistScope::Ghost, FREE, "on"),
        WriteOutcome::Written
    );
    assert_eq!(other.get(FREE).as_deref(), Some("on"));
    assert_eq!(
        other.get_in(PersistScope::Ghost, FREE).as_deref(),
        Some("on")
    );

    assert_eq!(kvs.unset(PersistScope::Ghost, FREE), WriteOutcome::Written);
    assert_eq!(other.get(FREE), None);
    assert_eq!(
        kvs.remove(PersistScope::Ghost, &[BOOT]),
        WriteOutcome::Written
    );
    assert_eq!(other.get(BOOT), None);
    assert_eq!(
        kvs.unset(PersistScope::Ghost, "myplugin.never"),
        WriteOutcome::Written,
        "無い鍵を消す頼みは書けた"
    );
    stop(parts);
}

#[test]
fn a_name_only_fixed_writers_may_write_is_refused_by_name() {
    let parts = spawn_sylphya(mem_ghost());
    let kvs = parts.reader.kvs(a());
    assert_eq!(
        kvs.set(PersistScope::Ghost, BOOT, "9"),
        WriteOutcome::Refused
    );
    assert_eq!(kvs.unset(PersistScope::Ghost, BOOT), WriteOutcome::Refused);
    assert_eq!(kvs.get(BOOT), None);
    stop(parts);
}

#[test]
fn after_removing_the_names_the_three_names_have_no_value() {
    let parts = spawn_sylphya(mem_ghost());
    let p = &parts.publisher;
    let names = ["selfname", "selfname2", "keroname"];
    p.publish_static(
        a(),
        names
            .iter()
            .map(|n| (n.to_string(), "値".to_string()))
            .collect(),
        vec![],
    );
    for name in names {
        assert_eq!(
            parts.reader.resolve_flat(&ctx("ghost/a"), name),
            FlatResolution::Value("値".into())
        );
    }
    p.remove_flat(a(), names.iter().map(|n| n.to_string()).collect());
    for name in names {
        assert!(
            !matches!(
                parts.reader.resolve_flat(&ctx("ghost/a"), name),
                FlatResolution::Value(_)
            ),
            "{name}"
        );
        assert!(
            !parts
                .reader
                .talk_snapshot(&ctx("ghost/a"))
                .contains_key(name)
        );
    }
    stop(parts);
}

#[test]
fn an_unlayered_write_is_the_askers_own_and_touches_no_file() {
    let dir = TempPath::under_target("sylphya-guardian-unlayered");
    let parts = spawn_sylphya(SylphyaInit::new(
        roots_under(dir.path()),
        FsBackend::with_manual_clock(),
    ));
    parts.publisher.set(a(), FREE.into(), "on".into());

    let mine = parts.reader.kvs(a());
    assert_eq!(mine.get(FREE).as_deref(), Some("on"));
    assert_eq!(
        mine.get_in(PersistScope::Ghost, FREE).as_deref(),
        Some("on")
    );
    assert_eq!(mine.get_in(PersistScope::App, FREE), None);
    assert_eq!(parts.reader.kvs(AskerId::new("ghost/b")).get(FREE), None);
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/b"), FREE),
        DottedResolution::NotFound
    );
    stop(parts);
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "フォルダもファイルも作らない"
    );
}

#[test]
fn app_volatile_outlives_the_guardian_and_the_other_layers_do_not() {
    let dir = TempPath::under_target("sylphya-guardian-volatile-life");
    let backend = FsBackend::with_manual_clock();
    let layers = [
        (PersistScope::App, "A"),
        (PersistScope::Ghost, "G"),
        (PersistScope::Shell, "S"),
        (PersistScope::Balloon, "B"),
    ];

    let first = spawn_sylphya(SylphyaInit::new(roots_under(dir.path()), backend.clone()));
    let kvs = first.reader.kvs(a());
    for (layer, value) in layers {
        assert_eq!(kvs.set(layer, FREE, value), WriteOutcome::Written);
    }
    stop(first);

    let second = spawn_sylphya(SylphyaInit::new(roots_under(dir.path()), backend));
    let other_ghost = second.reader.kvs(AskerId::new("ghost/b"));
    assert_eq!(
        other_ghost.get_in(PersistScope::App, FREE).as_deref(),
        Some("A")
    );
    let same_ghost = second.reader.kvs(a());
    for (layer, _) in &layers[1..] {
        assert_eq!(same_ghost.get_in(*layer, FREE), None, "{layer:?}");
    }
    assert_eq!(same_ghost.get(FREE).as_deref(), Some("A"));
    stop(second);
}

#[test]
fn re_rooting_a_layer_drops_its_volatile_values_and_keeps_its_file() {
    let dir = TempPath::under_target("sylphya-guardian-kvs-re-root");
    let parts = spawn_sylphya(SylphyaInit::new(
        roots_under(dir.path()),
        FsBackend::with_manual_clock(),
    ));
    let kvs = parts.reader.kvs(a());
    for layer in [PersistScope::Shell, PersistScope::Balloon] {
        assert_eq!(kvs.set(layer, FREE, "v"), WriteOutcome::Written);
        assert_eq!(kvs.put(layer, &[(BOOT, "1")]), WriteOutcome::Written);

        let away = dir.child(&format!("{layer:?}-other"));
        parts.publisher.set_layer_root(layer, Some(away));
        assert_eq!(kvs.get_in(layer, FREE), None, "{layer:?}");
        assert_eq!(kvs.get_in(layer, BOOT), None, "{layer:?}");

        let back = roots_under(dir.path()).root_of(layer).cloned();
        parts.publisher.set_layer_root(layer, back);
        assert_eq!(kvs.get_in(layer, BOOT).as_deref(), Some("1"), "{layer:?}");
        assert_eq!(kvs.get_in(layer, FREE), None, "{layer:?}");
    }
    stop(parts);
}

#[test]
fn the_place_of_a_running_guardian_can_be_renamed() {
    let dir = TempPath::under_target("sylphya-guardian-rename");
    let roots = roots_under(dir.path());
    let ghost = roots.ghost.clone().unwrap();
    let parts = spawn_sylphya(SylphyaInit::new(roots, FsBackend::with_manual_clock()));
    let kvs = parts.reader.kvs(a());
    assert_eq!(
        kvs.put(PersistScope::Ghost, &[(BOOT, "1")]),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(BOOT).as_deref(), Some("1"));

    let moved = dir.child("ghost-moved");
    std::fs::rename(&ghost, &moved).expect("走っている番人の置き場の名前を替えられる");
    assert!(moved.join("sylphya.sqlite").is_file());
    stop(parts);
}

#[test]
fn a_stopped_guardian_answers_stopped_and_no_value() {
    let parts = spawn_sylphya(mem_ghost());
    parts
        .publisher
        .publish_static(a(), vec![("selfname".into(), "生前値".into())], vec![]);
    let kvs = parts.reader.kvs(a());
    let reader = parts.reader.clone();
    stop(parts);

    assert_eq!(
        kvs.set(PersistScope::Ghost, FREE, "1"),
        WriteOutcome::Failed(WriteFailed::Stopped)
    );
    assert_eq!(
        kvs.put(PersistScope::Ghost, &[(BOOT, "1")]),
        WriteOutcome::Failed(WriteFailed::Stopped)
    );
    assert_eq!(kvs.get(FREE), None);
    assert!(!matches!(
        reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value(_)
    ));
    assert!(reader.talk_snapshot(&ctx("ghost/a")).is_empty());
}

#[test]
fn the_table_default_answers_only_when_no_layer_has_a_value() {
    let parts = spawn_sylphya(mem_ghost().with_table(TEST_TABLE));
    let kvs = parts.reader.kvs(a());
    assert_eq!(kvs.get(DEFAULTED).as_deref(), Some("既定"));
    assert_eq!(
        parts.reader.resolve_dotted_str(&ctx("ghost/a"), DEFAULTED),
        DottedResolution::Value("既定".into())
    );
    assert_eq!(kvs.get_in(PersistScope::Ghost, DEFAULTED), None);

    assert_eq!(
        kvs.set(PersistScope::Ghost, DEFAULTED, "x"),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(DEFAULTED).as_deref(), Some("x"));
    assert_eq!(
        kvs.unset(PersistScope::Ghost, DEFAULTED),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(DEFAULTED).as_deref(), Some("既定"));
    stop(parts);
}

#[test]
fn refusal_by_name_follows_the_guardians_table() {
    let global = spawn_sylphya(mem_ghost());
    assert_eq!(
        global.reader.kvs(a()).set(PersistScope::Ghost, LOCKED, "1"),
        WriteOutcome::Written
    );
    stop(global);

    let parts = spawn_sylphya(mem_ghost().with_table(TEST_TABLE));
    let kvs = parts.reader.kvs(a());
    assert_eq!(
        kvs.set(PersistScope::Ghost, LOCKED, "1"),
        WriteOutcome::Refused
    );
    assert_eq!(
        kvs.unset(PersistScope::Ghost, LOCKED),
        WriteOutcome::Refused
    );
    assert_eq!(kvs.get(LOCKED), None);
    assert_eq!(
        kvs.put(PersistScope::Ghost, &[(LOCKED, "2")]),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(LOCKED).as_deref(), Some("2"));
    stop(parts);
}

#[test]
fn a_write_mixing_homes_is_split_per_home() {
    let parts = spawn_sylphya(SylphyaInit::new(ScopeRoots::default(), MemBackend::new()));
    let kvs = parts.reader.kvs(a());
    let outcome = kvs.put(PersistScope::Ghost, &[(BOOT, "1"), (FREE, "2")]);
    assert!(
        matches!(outcome, WriteOutcome::Failed(WriteFailed::Persist(_))),
        "置き場の無い層のファイルの鍵は書けない: {outcome:?}"
    );
    assert_eq!(
        kvs.get_in(PersistScope::Ghost, FREE).as_deref(),
        Some("2"),
        "メモリーの鍵は別の書きとして入る"
    );
    stop(parts);
}

#[derive(Default)]
struct TextHome {
    value: Mutex<Option<String>>,
}

impl FastHome for TextHome {
    fn read_text(&self, _path: &PropPath) -> Option<String> {
        self.value.lock().unwrap().clone()
    }

    fn write_text(&self, _path: &PropPath, value: &str) -> Result<(), String> {
        *self.value.lock().unwrap() = Some(value.to_owned());
        Ok(())
    }

    fn clear(&self) {
        *self.value.lock().unwrap() = None;
    }
}

#[test]
fn a_fast_name_lives_only_in_its_registered_layer_and_reads_without_the_guardian() {
    let parts = spawn_sylphya(mem_ghost().with_table(TEST_TABLE));
    let home = Arc::new(TextHome::default());
    parts.reader.register_fast(
        PullShape::parse(FAST_SHAPE).unwrap(),
        PersistScope::Ghost,
        home.clone(),
    );
    let kvs = parts.reader.kvs(a());

    assert_eq!(
        kvs.set(PersistScope::Ghost, FAST_KEY, "7"),
        WriteOutcome::Written
    );
    assert_eq!(home.value.lock().unwrap().as_deref(), Some("7"));
    assert_eq!(kvs.get(FAST_KEY).as_deref(), Some("7"));
    assert_eq!(
        kvs.get_in(PersistScope::Ghost, FAST_KEY).as_deref(),
        Some("7")
    );
    assert_eq!(kvs.get_in(PersistScope::Shell, FAST_KEY), None);
    assert!(matches!(
        kvs.set(PersistScope::Shell, FAST_KEY, "8"),
        WriteOutcome::Failed(WriteFailed::Volatile { .. })
    ));

    stop(parts);
    assert_eq!(
        kvs.get(FAST_KEY).as_deref(),
        Some("7"),
        "番人を通らずに読む"
    );
}

#[test]
fn re_rooting_a_layer_clears_the_fast_homes_registered_there() {
    let dir = TempPath::under_target("sylphya-guardian-fast-re-root");
    let parts = spawn_sylphya(
        SylphyaInit::new(roots_under(dir.path()), FsBackend::with_manual_clock())
            .with_table(TEST_TABLE),
    );
    let shell = Arc::new(TextHome::default());
    parts.reader.register_fast(
        PullShape::parse(FAST_SHAPE).unwrap(),
        PersistScope::Shell,
        shell.clone(),
    );
    let kvs = parts.reader.kvs(a());
    assert_eq!(
        kvs.set(PersistScope::Shell, FAST_KEY, "1"),
        WriteOutcome::Written
    );

    parts
        .publisher
        .set_layer_root(PersistScope::Shell, Some(dir.child("shell-2")));
    parts.publisher.barrier().expect("barrier");
    assert_eq!(*shell.value.lock().unwrap(), None);
    assert_eq!(kvs.get(FAST_KEY), None);
    stop(parts);
}

#[test]
fn wait_limit_names_the_sqlite_file() {
    assert!(crate::guardian::WAIT_LIMIT.contains("sylphya.sqlite"));
}
