//! 速い住み場所の単体のテスト。

use std::sync::mpsc;
use std::thread;

use super::*;
use crate::key::{Selector, parse_dotted};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Count(u32);

impl FastValue for Count {
    fn to_bits(self) -> u64 {
        u64::from(self.0) + 1
    }
    fn from_bits(bits: u64) -> Self {
        Count((bits - 1) as u32)
    }
}

#[test]
fn set_then_get_and_none_clears_one_key() {
    let table = FastTable::<Count>::new();
    assert_eq!(table.get(7), None);
    table.set(7, Some(Count(0)));
    table.set(9, Some(Count(42)));
    assert_eq!(table.get(7), Some(Count(0)));
    assert_eq!(table.get(9), Some(Count(42)));
    table.set(7, Some(Count(5)));
    assert_eq!(table.get(7), Some(Count(5)));
    table.set(7, None);
    assert_eq!(table.get(7), None);
    assert_eq!(table.get(9), Some(Count(42)));
    assert_eq!(table.get(8), None);
}

#[test]
fn removing_an_absent_key_takes_no_row() {
    let table = FastTable::<Count>::new();
    for key in 0..(SEGMENT_LEN as u32 * 3) {
        table.set(key, None);
    }
    assert_eq!(table.segment_count(), 1);
}

#[test]
fn segments_grow_with_the_number_of_keys() {
    let table = FastTable::<Count>::new();
    assert_eq!(table.segment_count(), 1);
    for key in 0..SEGMENT_LEN as u32 {
        table.set(key, Some(Count(key * 10)));
    }
    assert_eq!(table.segment_count(), 1);
    table.set(SEGMENT_LEN as u32, Some(Count(1)));
    assert_eq!(table.segment_count(), 2);
    let total = SEGMENT_LEN as u32 * 3 + 1;
    for key in SEGMENT_LEN as u32..total {
        table.set(key, Some(Count(key * 10)));
    }
    assert_eq!(table.segment_count(), 4);
    for key in 0..total {
        assert_eq!(table.get(key), Some(Count(key * 10)), "key {key}");
    }
    assert_eq!(table.get(u32::MAX), None);
}

#[test]
fn clear_empties_every_value_and_rows_are_reused() {
    let table = FastTable::<Count>::new();
    let total = SEGMENT_LEN as u32 * 2 + 3;
    for key in 0..total {
        table.set(key, Some(Count(key)));
    }
    let segments = table.segment_count();
    table.clear();
    for key in 0..total {
        assert_eq!(table.get(key), None, "key {key}");
    }
    for key in 0..total {
        table.set(key, Some(Count(key + 100)));
    }
    assert_eq!(table.segment_count(), segments);
    assert_eq!(table.get(total - 1), Some(Count(total - 1 + 100)));
}

#[test]
fn clones_point_at_the_same_table() {
    let a = FastTable::<Count>::new();
    let b = a.clone();
    a.set(3, Some(Count(30)));
    assert_eq!(b.get(3), Some(Count(30)));
    b.clear();
    assert_eq!(a.get(3), None);
}

#[test]
fn readers_on_other_threads_see_the_write_after_the_reply() {
    let table = FastTable::<Count>::new();
    let keys = SEGMENT_LEN as u32 * 2 + 1;
    thread::scope(|s| {
        let mut replies = Vec::new();
        for _ in 0..4 {
            let (tx, rx) = mpsc::channel::<u32>();
            let (back_tx, back_rx) = mpsc::channel::<Option<Count>>();
            let reader = table.clone();
            s.spawn(move || {
                for key in rx {
                    back_tx.send(reader.get(key)).unwrap();
                }
            });
            replies.push((tx, back_rx));
        }
        for key in 0..keys {
            table.set(key, Some(Count(key + 1)));
            for (tx, back_rx) in &replies {
                tx.send(key).unwrap();
                assert_eq!(back_rx.recv().unwrap(), Some(Count(key + 1)));
            }
        }
    });
}

#[test]
fn writers_on_many_threads_keep_every_key() {
    let table = FastTable::<Count>::new();
    let per = SEGMENT_LEN as u32 * 2;
    thread::scope(|s| {
        for t in 0..4u32 {
            let writer = table.clone();
            s.spawn(move || {
                for i in 0..per {
                    let key = i * 4 + t;
                    writer.set(key, Some(Count(key)));
                }
            });
        }
    });
    for key in 0..per * 4 {
        assert_eq!(table.get(key), Some(Count(key)), "key {key}");
    }
    assert_eq!(table.segment_count(), (per * 4) as usize / SEGMENT_LEN);
}

/// 試験の中の「名前で書ける速い行」: `test.fast(<番号>)` を数の文字列で読み書きする。
struct NamedCounts(FastTable<Count>);

fn key_of(path: &PropPath) -> Option<u32> {
    match path.segs.as_slice() {
        [root, leaf] if root.name == "test" && root.selector.is_none() && leaf.name == "fast" => {
            match leaf.selector {
                Some(Selector::ByIndex(n)) => Some(n),
                _ => None,
            }
        }
        _ => None,
    }
}

impl FastHome for NamedCounts {
    fn read_text(&self, path: &PropPath) -> Option<String> {
        self.0.get(key_of(path)?).map(|c| c.0.to_string())
    }
    fn write_text(&self, path: &PropPath, value: &str) -> Result<(), String> {
        let key = key_of(path).ok_or_else(|| format!("not a fast row: {path}"))?;
        let n = value.parse::<u32>().map_err(|e| e.to_string())?;
        self.0.set(key, Some(Count(n)));
        Ok(())
    }
    fn clear(&self) {
        self.0.clear();
    }
}

#[test]
fn write_by_name_is_read_by_handle_and_back() {
    let handle = FastTable::<Count>::new();
    let home: Arc<dyn FastHome> = Arc::new(NamedCounts(handle.clone()));
    let path = parse_dotted("test.fast(2)").unwrap();

    assert_eq!(home.read_text(&path), None);
    home.write_text(&path, "120").unwrap();
    assert_eq!(handle.get(2), Some(Count(120)));

    handle.set(2, Some(Count(96)));
    assert_eq!(home.read_text(&path).as_deref(), Some("96"));

    assert!(home.write_text(&path, "x").is_err());
    assert_eq!(handle.get(2), Some(Count(96)));

    home.clear();
    assert_eq!(handle.get(2), None);
    assert_eq!(home.read_text(&path), None);
}
