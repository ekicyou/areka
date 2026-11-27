//! 速い住み場所: ロックなしで読む取っ手と、同じ値を名前で読み書きする口。

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use crate::key::PropPath;
use crate::persist::PersistScope;
use crate::pull::PullShape;

const SEGMENT_LEN: usize = 16;

/// 64 ビットに収まる値（0 は「値なし」に使うので、どの値も 0 以外に写す）。
pub trait FastValue: Copy + Send + Sync + 'static {
    fn to_bits(self) -> u64;
    fn from_bits(bits: u64) -> Self;
}

struct Segment {
    keys: [AtomicU64; SEGMENT_LEN],
    values: [AtomicU64; SEGMENT_LEN],
    next: OnceLock<Box<Segment>>,
}

impl Segment {
    fn new() -> Self {
        Self {
            keys: std::array::from_fn(|_| AtomicU64::new(0)),
            values: std::array::from_fn(|_| AtomicU64::new(0)),
            next: OnceLock::new(),
        }
    }
}

/// 鍵（番号）→ 値の表。複製は同じ表を指す取っ手。
pub struct FastTable<V: FastValue> {
    head: Arc<Segment>,
    _value: PhantomData<fn() -> V>,
}

impl<V: FastValue> Clone for FastTable<V> {
    fn clone(&self) -> Self {
        Self {
            head: Arc::clone(&self.head),
            _value: PhantomData,
        }
    }
}

impl<V: FastValue> Default for FastTable<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: FastValue> FastTable<V> {
    /// 空の表を作る。
    pub fn new() -> Self {
        Self {
            head: Arc::new(Segment::new()),
            _value: PhantomData,
        }
    }

    /// 鍵の値を読む（原子の読みだけ）。
    pub fn get(&self, key: u32) -> Option<V> {
        let tag = tag_of(key);
        let mut seg = &*self.head;
        loop {
            for (k, v) in seg.keys.iter().zip(&seg.values) {
                match k.load(Ordering::Acquire) {
                    // 欄は前から順に取られるので、空の欄より後ろに鍵は無い。
                    0 => return None,
                    t if t == tag => {
                        return match v.load(Ordering::Acquire) {
                            0 => None,
                            bits => Some(V::from_bits(bits)),
                        };
                    }
                    _ => {}
                }
            }
            seg = seg.next.get()?;
        }
    }

    /// 鍵の値を書く（`None` は値なしにする）。
    pub fn set(&self, key: u32, value: Option<V>) {
        let bits = value.map_or(0, V::to_bits);
        debug_assert!(
            value.is_none() || bits != 0,
            "FastValue::to_bits returned 0"
        );
        let tag = tag_of(key);
        let mut seg = &*self.head;
        loop {
            for (k, v) in seg.keys.iter().zip(&seg.values) {
                let mut current = k.load(Ordering::Acquire);
                if current == 0 {
                    if bits == 0 {
                        return;
                    }
                    match k.compare_exchange(0, tag, Ordering::AcqRel, Ordering::Acquire) {
                        Ok(_) => current = tag,
                        Err(winner) => current = winner,
                    }
                }
                if current == tag {
                    v.store(bits, Ordering::Release);
                    return;
                }
            }
            if bits == 0 && seg.next.get().is_none() {
                return;
            }
            seg = seg.next.get_or_init(|| Box::new(Segment::new()));
        }
    }

    /// 全部の値を値なしにする（行は残す）。
    pub fn clear(&self) {
        let mut seg = Some(&*self.head);
        while let Some(s) = seg {
            for v in &s.values {
                v.store(0, Ordering::Release);
            }
            seg = s.next.get().map(|b| &**b);
        }
    }

    #[cfg(test)]
    fn segment_count(&self) -> usize {
        let mut n = 1;
        let mut seg = &*self.head;
        while let Some(next) = seg.next.get() {
            n += 1;
            seg = next;
        }
        n
    }
}

fn tag_of(key: u32) -> u64 {
    u64::from(key) + 1
}

/// 名前の形 1 つぶんの速い住み場所（住人の型を知る crate が実装して登録する）。
pub trait FastHome: Send + Sync {
    fn read_text(&self, path: &PropPath) -> Option<String>;
    fn write_text(&self, path: &PropPath, value: &str) -> Result<(), String>;
    fn clear(&self);
}

type Registered = (PersistScope, Arc<dyn FastHome>);

/// 名前の形 → （層, 速い住み場所）の表（読み口の複製どうしと番人が分け合う）。
#[derive(Clone, Default)]
pub(crate) struct FastRegistry {
    inner: Arc<RwLock<BTreeMap<PullShape, Registered>>>,
}

impl FastRegistry {
    /// 同じ形への 2 度目の登録は前の住み場所を置き換える。
    pub(crate) fn register(&self, shape: PullShape, layer: PersistScope, home: Arc<dyn FastHome>) {
        // poison でも表は常に整合な値なので中身で続ける（`PullRegistry` と同じ）。
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        guard.insert(shape, (layer, home));
    }

    /// 写してから鍵を外す（住み場所を呼ぶのは鍵の外）。
    pub(crate) fn find(&self, path: &PropPath) -> Option<Registered> {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        guard.get(&PullShape::of(path)).cloned()
    }

    /// その層に登録された住み場所の値を全部捨てる。
    pub(crate) fn clear_layer(&self, layer: PersistScope) {
        let homes: Vec<Arc<dyn FastHome>> = {
            let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
            guard
                .values()
                .filter(|(at, _)| *at == layer)
                .map(|(_, home)| home.clone())
                .collect()
        };
        for home in homes {
            home.clear();
        }
    }
}

#[cfg(test)]
#[path = "fast_tests.rs"]
mod tests;
