//! `log_history` の決定論テスト（置き場・番号の口・並行・毒された排他）。
//!
//! プロセスに 1 つの置き場は並走する他のテストも積むので、件数や番号の絶対値では確かめない。
//! 並行と毒は手元の `Mutex<History>` を置き場と同じ取り出し方（[`lock`]）で扱って確かめる。

use std::sync::Mutex;

use super::*;

fn stamp() -> Stamp {
    Stamp {
        year: 2026,
        month: 1,
        day: 5,
        hour: 3,
        minute: 7,
    }
}

fn draft_of(kind: Kind, body: &str) -> Draft {
    Draft {
        kind,
        label: kind.default_label().to_string(),
        name: kind.default_name().to_string(),
        body: body.to_string(),
    }
}

// ---------------------------------------------------------------------------
// 置き場と番号の口（要件 6.2・6.4）
// ---------------------------------------------------------------------------

#[test]
fn a_record_after_reading_last_id_is_in_the_snapshot_after_that_id() {
    let body = "convention-test: 番号を読んでから積んだ 1 件";
    let before = last_id();
    let at = local_now();
    let id = record(at, draft_of(Kind::Network, body));
    assert!(
        id > before,
        "振った番号 {id} が読んだ番号 {before} より大きい"
    );
    assert!(last_id() >= id, "最後に振った番号は積んだ番号以上");

    // 並走する他のテストが積んでも、「読んだ番号より大きい」で絞った中に積んだ 1 件がある。
    let mine: Vec<Record> = snapshot(Kind::Network)
        .into_iter()
        .filter(|r| r.id > before && r.body == body)
        .collect();
    assert_eq!(mine.len(), 1, "積んだ 1 件がちょうど 1 回入る");
    let r = &mine[0];
    assert_eq!(r.id, id);
    assert_eq!(r.at, at);
    assert_eq!(r.kind, Kind::Network);
    assert_eq!(r.label, "Info");
    assert_eq!(r.name, "[SYSTEM]");

    // 他の種別の写しには入らない。
    assert!(snapshot(Kind::Status).iter().all(|r| r.id != id));
}

#[test]
fn local_now_is_a_plausible_local_time() {
    let s = local_now();
    assert!(s.year >= 2024, "年 {}", s.year);
    assert!((1..=12).contains(&s.month), "月 {}", s.month);
    assert!((1..=31).contains(&s.day), "日 {}", s.day);
    assert!(s.hour <= 23, "時 {}", s.hour);
    assert!(s.minute <= 59, "分 {}", s.minute);
}

// ---------------------------------------------------------------------------
// 並行（要件 1.9）
// ---------------------------------------------------------------------------

#[test]
fn eight_threads_of_500_get_ids_1_to_4000_exactly_once() {
    const THREADS: usize = 8;
    const EACH: usize = 500;
    let store = Mutex::new(History::new());
    std::thread::scope(|s| {
        for t in 0..THREADS {
            let store = &store;
            s.spawn(move || {
                for i in 0..EACH {
                    // 種別を 5 つに散らす（4,000 / 5 = 800 件で上限に掛からない）。
                    let kind = Kind::ALL[(t * EACH + i) % Kind::ALL.len()];
                    lock(store).push(stamp(), draft_of(kind, &format!("{t}-{i}")));
                }
            });
        }
    });

    let h = lock(&store);
    assert_eq!(h.last_id(), (THREADS * EACH) as u64);
    let mut ids: Vec<u64> = Kind::ALL
        .into_iter()
        .flat_map(|k| h.rows(k).map(|r| r.id))
        .collect();
    assert_eq!(ids.len(), THREADS * EACH, "取りこぼし 0 件");
    ids.sort_unstable();
    let expected: Vec<u64> = (1..=(THREADS * EACH) as u64).collect();
    assert_eq!(ids, expected, "番号 1〜4,000 がちょうど 1 回ずつ");
    // どの列も番号の昇順。
    for k in Kind::ALL {
        let row: Vec<u64> = h.rows(k).map(|r| r.id).collect();
        assert!(row.windows(2).all(|w| w[0] < w[1]), "{k:?} の列が昇順");
    }
}

// ---------------------------------------------------------------------------
// 毒された排他（要件 1.10）
// ---------------------------------------------------------------------------

#[test]
fn a_poisoned_store_still_accepts_records() {
    let store = Mutex::new(History::new());
    lock(&store).push(stamp(), draft_of(Kind::Error, "毒の前"));

    let panicked = std::thread::scope(|s| {
        s.spawn(|| {
            let _held = store.lock().unwrap();
            panic!("排他を握ったままパニックして毒す");
        })
        .join()
    });
    assert!(panicked.is_err());
    assert!(store.is_poisoned(), "排他が毒されている");

    // 同じ取り出し方なら毒されていても積めて、番号は続きから。
    let id = lock(&store).push(stamp(), draft_of(Kind::Error, "毒の後"));
    assert_eq!(id, 2);
    let h = lock(&store);
    let bodies: Vec<&str> = h.rows(Kind::Error).map(|r| r.body.as_str()).collect();
    assert_eq!(bodies, ["毒の前", "毒の後"]);
}

// ---------------------------------------------------------------------------
// 規則のフィルタの答え（要件 1.7・1.9・6.3）
// ---------------------------------------------------------------------------

const LEVELS: [Level; 5] = [
    Level::ERROR,
    Level::WARN,
    Level::INFO,
    Level::DEBUG,
    Level::TRACE,
];

/// 診断の target・規則の target・取り決めの target・当たらない target・橋渡しの target。
const TARGETS: [&str; 9] = [
    "areka::perf",
    "wintf::tick_diag",
    "areka::boot_config",
    "areka::update::desk",
    "areka::log::script",
    "areka::log::error",
    "areka::alert",
    "log",
    "",
];

#[test]
fn a_query_that_is_not_an_event_always_passes_and_is_never_wanted() {
    // `tracing::enabled!` とスパンは出来事でない。断ると「断った」印がスレッドに残り、
    // 次の出来事を履歴の層だけが飛ばす（design-validation.md 指摘 1）。
    for level in LEVELS {
        for target in TARGETS {
            assert!(
                passes(false, level, target),
                "出来事でない問い合わせ {level:?} {target:?} は通す"
            );
            assert!(
                !wants(false, level, target),
                "出来事でない問い合わせ {level:?} {target:?} に関心は無い"
            );
        }
    }
    // 前置ガードの実物の形（診断の target の debug）。
    assert!(passes(false, Level::DEBUG, "areka::perf"));
}

#[test]
fn an_event_is_wanted_and_passes_exactly_when_classify_hits() {
    let cases = [
        (Level::WARN, "wintf::tick_diag", true),
        (Level::ERROR, "areka::alert", true),
        (Level::INFO, "areka::boot_config", true),
        (Level::INFO, "areka::log::script", true),
        (Level::INFO, "areka::update::desk", true),
        (Level::DEBUG, "areka::log::script", false),
        (Level::DEBUG, "areka::perf", false),
        (Level::TRACE, "areka::boot_config", false),
        (Level::INFO, "areka::alert", false),
        (Level::INFO, "log", false),
    ];
    for (level, target, hit) in cases {
        assert_eq!(
            wants(true, level, target),
            hit,
            "wants {level:?} {target:?}"
        );
        assert_eq!(
            passes(true, level, target),
            hit,
            "passes {level:?} {target:?}"
        );
    }
    // 全組み合わせで振り分けの当たり外れと一致する。
    for level in LEVELS {
        for target in TARGETS {
            let hit = classify(level, target).is_some();
            assert_eq!(
                wants(true, level, target),
                hit,
                "wants {level:?} {target:?}"
            );
            assert_eq!(
                passes(true, level, target),
                hit,
                "passes {level:?} {target:?}"
            );
        }
    }
}

#[test]
fn the_filter_hints_info_as_the_most_verbose_level() {
    use tracing_subscriber::layer::Filter;
    let hint =
        <HistoryFilter as Filter<tracing_subscriber::Registry>>::max_level_hint(&HistoryFilter);
    assert_eq!(hint, Some(tracing_subscriber::filter::LevelFilter::INFO));
}

// ---------------------------------------------------------------------------
// 取り決めの文書との一致（要件 7.3）
// ---------------------------------------------------------------------------

/// 取り決めの文書（`CARGO_MANIFEST_DIR` からの相対）。
const CONVENTION_DOC: &str = "../../doc/ssp-mcp/log-convention.md";
const TABLE_BEGIN: &str = "<!-- log-rules:begin -->";
const TABLE_END: &str = "<!-- log-rules:end -->";
/// 照合の列の語（完全一致は true）。
const MATCH_EXACT: &str = "完全一致";
const MATCH_SUBTREE: &str = "下も含む";

/// 文書の目印の間の表を（種別の語・target・完全一致か）の列で読む（見出しと区切りの行は飛ばす）。
fn doc_rules(doc: &str) -> Vec<(String, String, bool)> {
    let begin = doc.find(TABLE_BEGIN).expect("文書に表の始まりの目印がある");
    let end = doc.find(TABLE_END).expect("文書に表の終わりの目印がある");
    assert!(begin < end, "始まりの目印が終わりの目印より前");
    doc[begin + TABLE_BEGIN.len()..end]
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|'))
        .skip(2)
        .map(|l| {
            let cells: Vec<&str> = l
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().trim_matches('`'))
                .collect();
            assert_eq!(cells.len(), 3, "表の行は 3 列（種別・target・照合）: {l}");
            let exact = match cells[2] {
                MATCH_EXACT => true,
                MATCH_SUBTREE => false,
                other => panic!(
                    "照合の列の語が `{MATCH_EXACT}` でも `{MATCH_SUBTREE}` でもない: {other}"
                ),
            };
            (cells[0].to_string(), cells[1].to_string(), exact)
        })
        .collect()
}

#[test]
fn the_convention_doc_table_equals_the_rules_and_the_two_convention_targets() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CONVENTION_DOC);
    let doc = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("取り決めの文書 {} を読めない: {e}", path.display()));
    let rows = doc_rules(&doc);
    assert!(!rows.is_empty(), "文書の表から 1 行も読めていない");

    let documented: std::collections::HashSet<_> = rows.iter().cloned().collect();
    assert_eq!(documented.len(), rows.len(), "文書の表に同じ行が 2 度ある");

    let expected: std::collections::HashSet<(String, String, bool)> = RULES
        .iter()
        .map(|r| (r.kind.word().to_string(), r.target.to_string(), r.exact))
        .chain([
            (
                Kind::Script.word().to_string(),
                TARGET_SCRIPT.to_string(),
                true,
            ),
            (
                Kind::Error.word().to_string(),
                TARGET_ERROR.to_string(),
                true,
            ),
        ])
        .collect();
    assert_eq!(expected.len(), RULES.len() + 2);

    let missing: Vec<_> = expected.difference(&documented).collect();
    let extra: Vec<_> = documented.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "文書の表と規則の表が食い違う\n文書に無い: {missing:?}\n規則に無い: {extra:?}"
    );
}
