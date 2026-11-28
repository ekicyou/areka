use crate::asker::{AskerContext, AskerId};
use crate::key::parse_dotted;
use crate::persist::ScopeRoots;
use crate::store::MemBackend;
use crate::value::{DottedResolution, FlatResolution};
use crate::vocab::DegradePolicy;
use crate::{SylphyaInit, SylphyaParts, spawn_sylphya};

fn ctx(id: &str) -> AskerContext {
    AskerContext {
        asker: AskerId::new(id),
    }
}

fn spawn() -> SylphyaParts {
    spawn_sylphya(SylphyaInit::new(ScopeRoots::default(), MemBackend::new()))
}

fn flat(parts: &SylphyaParts, asker: &str, name: &str, value: &str) {
    parts.publisher.publish_static(
        AskerId::new(asker),
        vec![(name.into(), value.into())],
        vec![],
    );
}

fn per_asker(parts: &SylphyaParts, asker: &str, key: &str, value: &str) {
    parts
        .publisher
        .set(AskerId::new(asker), key.into(), value.into());
}

fn global(parts: &SylphyaParts, key: &str, value: &str) {
    parts.publisher.publish_static(
        AskerId::new("ghost/a"),
        vec![],
        vec![(key.into(), value.into())],
    );
}

#[test]
fn flat_value_present_per_asker() {
    let parts = spawn();
    flat(&parts, "ghost/a", "selfname", "Alice");
    assert_eq!(
        parts.reader.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value("Alice".into())
    );
}

#[test]
fn flat_absent_names_degrade_by_the_ledger() {
    let parts = spawn();
    for (name, policy) in [
        ("username", DegradePolicy::ConsumerDefault),
        ("selfname", DegradePolicy::PassThroughRaw),
        ("not_a_real_token", DegradePolicy::PassThroughRaw),
    ] {
        assert_eq!(
            parts.reader.resolve_flat(&ctx("ghost/a"), name),
            FlatResolution::Degraded(policy),
            "{name}"
        );
    }
}

#[test]
fn dotted_per_asker_precedes_global() {
    let parts = spawn();
    global(&parts, "x.y", "GLOBAL");
    per_asker(&parts, "ghost/a", "x.y", "PERASKER");
    let path = parse_dotted("x.y").unwrap();
    assert_eq!(
        parts.reader.resolve_dotted(&ctx("ghost/a"), &path),
        DottedResolution::Value("PERASKER".into())
    );
    assert_eq!(
        parts.reader.resolve_dotted(&ctx("ghost/b"), &path),
        DottedResolution::Value("GLOBAL".into())
    );
}

#[test]
fn dotted_absent_and_malformed_are_not_found() {
    let parts = spawn();
    assert_eq!(
        parts
            .reader
            .resolve_dotted(&ctx("ghost/a"), &parse_dotted("system.no_such").unwrap()),
        DottedResolution::NotFound
    );
    for key in ["a..b", ""] {
        assert_eq!(
            parts.reader.resolve_dotted_str(&ctx("ghost/a"), key),
            DottedResolution::NotFound,
            "{key:?}"
        );
    }
}

#[test]
fn dotted_str_roundtrips_with_canonical_stored_key() {
    let parts = spawn();
    global(&parts, "baseware.version", "0.1");
    per_asker(&parts, "ghost/a", "MyPlugin.Scope(2).Y", "512");
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "baseware.version"),
        DottedResolution::Value("0.1".into())
    );
    assert_eq!(
        parts
            .reader
            .resolve_dotted_str(&ctx("ghost/a"), "myplugin.scope(2).y"),
        DottedResolution::Value("512".into())
    );
}

#[test]
fn talk_snapshot_contains_only_real_values_and_stays_frozen() {
    let parts = spawn();
    flat(&parts, "ghost/a", "selfname", "Alice");
    let snap = parts.reader.talk_snapshot(&ctx("ghost/a"));
    assert_eq!(snap.get("selfname").map(String::as_str), Some("Alice"));
    assert!(!snap.contains_key("username"));
    assert!(!snap.contains_key("month"));

    flat(&parts, "ghost/a", "selfname", "CHANGED");
    assert_eq!(snap.get("selfname").map(String::as_str), Some("Alice"));
    let again = parts.reader.talk_snapshot(&ctx("ghost/a"));
    assert_eq!(again.get("selfname").map(String::as_str), Some("CHANGED"));
}

/// 問い手ごとの値が、平らな名前でも点付きの名前でも両方向に混ざらない。
#[test]
fn per_asker_non_mixing_flat_and_dotted() {
    let parts = spawn();
    flat(&parts, "ghost/a", "selfname", "Alice");
    flat(&parts, "ghost/a", "keroname", "KeroA");
    flat(&parts, "ghost/b", "selfname", "Bob");
    per_asker(&parts, "ghost/a", "myplugin.scope(0).x", "10");
    per_asker(&parts, "ghost/a", "myplugin.only_a", "A1");
    per_asker(&parts, "ghost/b", "myplugin.scope(0).x", "99");
    per_asker(&parts, "ghost/b", "myplugin.only_b", "B2");
    let r = &parts.reader;

    assert_eq!(
        r.resolve_flat(&ctx("ghost/a"), "selfname"),
        FlatResolution::Value("Alice".into())
    );
    assert_eq!(
        r.resolve_flat(&ctx("ghost/b"), "selfname"),
        FlatResolution::Value("Bob".into())
    );
    assert_eq!(
        r.resolve_flat(&ctx("ghost/b"), "keroname"),
        FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
    );
    assert_eq!(
        r.resolve_dotted_str(&ctx("ghost/a"), "myplugin.scope(0).x"),
        DottedResolution::Value("10".into())
    );
    assert_eq!(
        r.resolve_dotted_str(&ctx("ghost/b"), "myplugin.scope(0).x"),
        DottedResolution::Value("99".into())
    );
    assert_eq!(
        r.resolve_dotted_str(&ctx("ghost/b"), "myplugin.only_a"),
        DottedResolution::NotFound
    );
    assert_eq!(
        r.resolve_dotted_str(&ctx("ghost/a"), "myplugin.only_b"),
        DottedResolution::NotFound
    );
}
