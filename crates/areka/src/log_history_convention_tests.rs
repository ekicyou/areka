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
