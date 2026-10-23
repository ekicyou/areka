//! `bind_spec.rs` の試験（research.md §10.4 の 13 通りとスコープごとの名前の表）。

use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::resolve_with_shell;
use areka_seriko::{BindResolver, DefaultBinds, ScopeStates};
use log_capture_kit::{CapturedEvent, capture};

use super::{apply_bind_specs, read_bind_specs};
use crate::offline_dump::offline_dump_test_support::{ShellSpec, build_ghost};
use crate::scope_bind_fixture_test_support::{
    BLUE, CHAR2_CATEGORY, GREEN, KERO_CATEGORY, RED, SAKURA_CATEGORY, scope_bind_ghost,
};

const MISSING: &str = "offline_dump_bind_missing";
const KEPT: &str = "offline_dump_bind_mustselect_kept";
const UNREADABLE: &str = "offline_dump_bind_unreadable";

const UNIFORM: u32 = 1;
const CASUAL: u32 = 2;
const SWIM: u32 = 3;
const HAT: u32 = 4;
const RIBBON: u32 = 5;
const A: u32 = 6;
const B: u32 = 7;

/// research.md §10.4 の `g_bind` のシェル（cloth＝排他・head＝mustselect・acc＝multiple）。
const G_BIND_DESCRIPT: &[&str] = &[
    "sakura.bindgroup1.name,cloth,uniform",
    "sakura.bindgroup1.default,1",
    "sakura.bindgroup2.name,cloth,casual",
    "sakura.bindgroup2.default,0",
    "sakura.bindgroup3.name,cloth,swim",
    "sakura.bindgroup3.default,0",
    "sakura.bindgroup4.name,head,hat",
    "sakura.bindgroup4.default,1",
    "sakura.bindgroup5.name,head,ribbon",
    "sakura.bindgroup5.default,0",
    "sakura.bindgroup6.name,acc,a",
    "sakura.bindgroup6.default,1",
    "sakura.bindgroup7.name,acc,b",
    "sakura.bindgroup7.default,1",
    "sakura.bindoption0.group,head,mustselect",
    "sakura.bindoption1.group,acc,multiple",
];

/// `g_bind` の検体を `target\` の下に組む。
fn g_bind(label: &str) -> temp_path_kit::TempPath {
    build_ghost(
        label,
        Some(&["name,g_bind"]),
        &[ShellSpec {
            folder: "master",
            descript: G_BIND_DESCRIPT,
            surfaces: &[],
            images: &[],
        }],
    )
}

/// 検体のシェルを読み、既定の組に `raw` をスコープ `scope` へ当てて、スコープ 0〜3 の組と記録を返す。
fn run(ghost_root: &std::path::Path, raw: &str, scope: u32) -> ([Vec<u32>; 4], Vec<CapturedEvent>) {
    let model = resolve_with_shell(ghost_root, DefaultEncoding::Ansi, None).expect("検体は読める");
    let resolver = BindResolver::from_groups(&model.bindgroups);
    let mut states = ScopeStates::new(DefaultBinds::from_groups(&model.bindgroups));
    let ((), events) = capture(|| {
        let specs = read_bind_specs(raw);
        apply_bind_specs(&specs, scope, &resolver, &mut states);
    });
    let ids = |s: &str| {
        states
            .current_binds(&areka_sakura::ActorKey::from(s))
            .ids()
            .to_vec()
    };
    ([ids("0"), ids("1"), ids("2"), ids("3")], events)
}

/// 記録のうち `level` の `event` の (category, part) の列。
fn logged(events: &[CapturedEvent], level: tracing::Level, event: &str) -> Vec<(String, String)> {
    events
        .iter()
        .filter(|e| e.level == level && e.field_str("event") == Some(event))
        .map(|e| {
            let f = |k| e.field_str(k).unwrap_or_default().to_owned();
            (f("category"), f("part"))
        })
        .collect()
}

fn warn_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .count()
}

fn pair(c: &str, p: &str) -> (String, String) {
    (c.to_owned(), p.to_owned())
}

/// research.md §10.4 の 13 通り（指定 → スコープ 0 の着ているもの・無い名前の warn）。
#[test]
fn research_10_4_thirteen_cases() {
    let ghost = g_bind("offline-bind-spec");
    let none = Vec::new;
    type Case = (&'static str, Vec<u32>, Vec<(String, String)>);
    let cases: Vec<Case> = vec![
        ("cloth,__ALL__,1", vec![SWIM, HAT, A, B], none()),
        ("cloth,,1", vec![SWIM, HAT, A, B], none()),
        ("cloth,casual", vec![CASUAL, HAT, A, B], none()),
        ("cloth,uniform,0", vec![HAT, A, B], none()),
        ("head,hat,0", vec![UNIFORM, HAT, A, B], none()),
        ("head,__ALL__,0", vec![UNIFORM, HAT, A, B], none()),
        ("head,ribbon,1", vec![UNIFORM, RIBBON, A, B], none()),
        ("acc,__ALL__,0", vec![UNIFORM, HAT], none()),
        ("acc,a,false;cloth,swim,true", vec![SWIM, HAT, B], none()),
        (
            "cloth, casual,1",
            vec![UNIFORM, HAT, A, B],
            vec![pair("cloth", " casual")],
        ),
        (
            "cloth,casual ,1",
            vec![UNIFORM, HAT, A, B],
            vec![pair("cloth", "casual ")],
        ),
        (
            "nosuch,x,1",
            vec![UNIFORM, HAT, A, B],
            vec![pair("nosuch", "x")],
        ),
        (
            "cloth,nosuchpart,1",
            vec![UNIFORM, HAT, A, B],
            vec![pair("cloth", "nosuchpart")],
        ),
    ];
    for (raw, mut wearing, missing) in cases {
        let ([s0, ..], events) = run(ghost.path(), raw, 0);
        wearing.sort_unstable();
        assert_eq!(s0, wearing, "{raw}: 着ているもの");
        assert_eq!(
            logged(&events, tracing::Level::WARN, MISSING),
            missing,
            "{raw}: {events:?}"
        );
        assert_eq!(
            warn_count(&events),
            missing.len(),
            "{raw}: warn はほかに無い: {events:?}"
        );
    }
}

/// mustselect の OFF は外さず info を 1 パーツに 1 件（warn ではない＝書き出しの記録に入らない・5.4）。
#[test]
fn mustselect_off_is_kept_with_info_per_part() {
    let ghost = g_bind("offline-bind-spec-kept");
    let (_, events) = run(ghost.path(), "head,hat,0", 0);
    assert_eq!(
        logged(&events, tracing::Level::INFO, KEPT),
        vec![pair("head", "hat")]
    );
    let (_, events) = run(ghost.path(), "head,__ALL__,0", 0);
    assert_eq!(
        logged(&events, tracing::Level::INFO, KEPT),
        vec![pair("head", "hat"), pair("head", "ribbon")]
    );
}

/// 中身の空の区切りは黙って飛ばし、欄の数か ON/OFF が読めない指定はその指定だけ warn で飛ばす（5.7）。
#[test]
fn unreadable_specs_warn_and_empty_separators_are_skipped() {
    let ghost = g_bind("offline-bind-spec-unreadable");
    let raw = ";cloth,casual,2;cloth;acc,a,1,x;cloth,swim,TRUE;;acc,b,0;";
    let ([s0, ..], events) = run(ghost.path(), raw, 0);
    assert_eq!(s0, vec![UNIFORM, HAT, A], "読めた acc,b,0 だけが効く");
    let unreadable: Vec<String> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.field_str("event") == Some(UNREADABLE))
        .map(|e| e.field_str("spec").unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        unreadable,
        ["cloth,casual,2", "cloth", "acc,a,1,x", "cloth,swim,TRUE"],
        "{events:?}"
    );
    assert_eq!(warn_count(&events), 4, "{events:?}");
}

/// スコープ 0・1・2 の名前の表: 指定のスコープだけに、そのスコープの名前で当たる（5.8・5.9）。
#[test]
fn applies_only_to_the_given_scope_with_its_own_names() {
    let ghost = scope_bind_ghost();
    let defaults = [vec![RED], vec![GREEN], vec![BLUE], vec![]];
    let categories = [SAKURA_CATEGORY, KERO_CATEGORY, CHAR2_CATEGORY];
    for (scope, category) in (0u32..).zip(categories) {
        let want = |id: u32| {
            let mut want = defaults.clone();
            want[scope as usize] = vec![id];
            want
        };
        // 排他のカテゴリ全部の ON は書かれた順の最後（青）だけが残る。
        let (got, events) = run(ghost.path(), &format!("{category},__ALL__,1"), scope);
        assert_eq!(got, want(BLUE), "スコープ {scope}: {events:?}");
        assert_eq!(warn_count(&events), 0, "{events:?}");

        let (got, events) = run(ghost.path(), &format!("{category},緑,1"), scope);
        assert_eq!(got, want(GREEN), "スコープ {scope}: {events:?}");
        assert_eq!(warn_count(&events), 0, "{events:?}");

        // ほかのスコープの名前では当たらない（無い名前の warn）。
        for other in categories.into_iter().filter(|c| *c != category) {
            let (got, events) = run(ghost.path(), &format!("{other},緑,1"), scope);
            assert_eq!(got, defaults, "スコープ {scope} に {other}");
            assert_eq!(
                logged(&events, tracing::Level::WARN, MISSING),
                vec![pair(other, "緑")]
            );
        }
    }
    // 名前の無いスコープ 3 へのカテゴリ全部は、無いカテゴリの warn を 1 件だけ残す。
    let (got, events) = run(ghost.path(), &format!("{SAKURA_CATEGORY},,1"), 3);
    assert_eq!(got, defaults);
    assert_eq!(
        logged(&events, tracing::Level::WARN, MISSING),
        vec![pair(SAKURA_CATEGORY, "__ALL__")]
    );
    assert_eq!(warn_count(&events), 1, "{events:?}");
}
