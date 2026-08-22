//! 鏡像: `MirrorImage`（不変・epoch:u64）・`SharedMirror`（`RwLock<Arc<..>>` swap）・区画。
//!
//! 掲示板（マテリアライズド・ビュー）モデルの状態核。読みは共有読みハンドル
//! [`SharedMirror::load`] による同期・無待機（read lock 内 `Arc` clone のみ）。供給は
//! アクター（Task 5.x）が copy-on-write で新しい `Arc<MirrorImage>` を構築し
//! [`SharedMirror::publish`] で swap する（epoch 単調増加・design §「鏡像＋SylphyaReader」
//! State Management）。
//!
//! ## 区画（設計討議 #1 裁定）
//!
//! `MirrorImage` は集約ルート 1 つで、次の 4 区画を保持する:
//!
//! - `flat_per_asker` — フラット語彙（`%名前`）の per-asker 区画。M1 実導出フラット語彙
//!   〔username/selfname/selfname2/keroname〕は全てゴースト相対ゆえここへ着地する。
//! - `flat_global` — フラット語彙の大域区画。将来の大域フラット語彙〔screenwidth 系等〕用に確保。
//! - `dotted_global` — 点付き語彙（`system.*` 等）の大域区画。key は `PropPath` の正準文字列形。
//! - `dotted_per_asker` — 点付き語彙の per-asker 区画（`currentghost` 系・host 書込区画）。
//!   key は `PropPath` の正準文字列形。
//!
//! フラット解決順は per-asker → global（解決自体は Task 3.2 の `SylphyaReader`）。

use crate::asker::AskerId;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// epoch 付き不変鏡像（掲示板の 1 スナップショット）。
///
/// 生成後は不変として扱う（`Arc` 共有で publish される——interior mutability を持たず、
/// 公開 `&mut self` ミューテータも持たない）。後継像は [`MirrorImage::successor`] で
/// copy-on-write により構築する（区画を clone し `epoch` を +1）。フィールドは公開だが、
/// 変異は「後継像を組み立ててから `Arc` へ封じて publish する」経路に限る（design §State Management）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MirrorImage {
    /// 単調増加する世代番号。後継像ごとに +1（決定論の観測条件・R2.5）。
    pub epoch: u64,
    /// フラット語彙の per-asker 区画（`%名前`・ゴースト相対）。
    pub flat_per_asker: BTreeMap<AskerId, BTreeMap<String, String>>,
    /// フラット語彙の大域区画（将来の大域フラット語彙用に確保）。
    pub flat_global: BTreeMap<String, String>,
    /// 点付き語彙の大域区画。key は `PropPath` の正準文字列形。
    pub dotted_global: BTreeMap<String, String>,
    /// 点付き語彙の per-asker 区画。key は `PropPath` の正準文字列形。
    pub dotted_per_asker: BTreeMap<AskerId, BTreeMap<String, String>>,
}

impl MirrorImage {
    /// 初期像（epoch 0・全区画空）を構築する。
    pub fn empty() -> Self {
        MirrorImage::default()
    }

    /// 後継像を copy-on-write で構築する。
    ///
    /// 現在像の全区画を clone し、`epoch` を +1 した新しい `MirrorImage` を返す。
    /// 呼び出し側（アクター）は返された像の区画へ変更を適用してから `Arc` に封じて
    /// [`SharedMirror::publish`] する。この関数は `self` を変異しない（不変像の前提を保つ）。
    pub fn successor(&self) -> Self {
        MirrorImage {
            epoch: self.epoch + 1,
            flat_per_asker: self.flat_per_asker.clone(),
            flat_global: self.flat_global.clone(),
            dotted_global: self.dotted_global.clone(),
            dotted_per_asker: self.dotted_per_asker.clone(),
        }
    }
}

/// 共有読みハンドル: `RwLock<Arc<MirrorImage>>` を `Arc` で包み、複数読者が同一ロックを
/// 安価に共有する（`Clone` は `Arc` の clone のみ）。
///
/// single-writer（アクター）×multi-reader の並行戦略。single-writer 保証はアクター側
/// （Task 5.x）の責務で、ここでは swap プリミティブのみを提供する。
#[derive(Clone)]
pub struct SharedMirror {
    inner: Arc<RwLock<Arc<MirrorImage>>>,
}

impl SharedMirror {
    /// 初期像から共有ハンドルを構築する。
    pub fn new(initial: Arc<MirrorImage>) -> Self {
        SharedMirror {
            inner: Arc::new(RwLock::new(initial)),
        }
    }

    /// 現在の鏡像スナップショットを取得する（同期・無待機）。
    ///
    /// # 無ブロッキング照会不変（R2.7 / R8.3）
    ///
    /// この関数が行うのは **read lock 取得 → `Arc` clone → lock 解放** のみである。
    /// チャネルの送受信（send/recv）・ファイル IO・OS 呼出・他アクター/他スレッドへの
    /// ブロッキング照会は一切行わない。ゆえにプロパティ解決の読み経路が
    /// システム全体の直列化点（大域ロック）にならない。
    pub fn load(&self) -> Arc<MirrorImage> {
        // lock が poison していても保持する Arc<MirrorImage> は常に整合な不変値なので
        // 復帰して読み経路を非 panic に保つ（読み経路は決して失敗させない・R2.7）。
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard)
    }

    /// 新しい鏡像を publish する（write lock 内で `Arc` を差し替え）。
    ///
    /// copy-on-write: 呼び出し側が [`MirrorImage::successor`] で epoch を +1 した後継像を
    /// 構築し、その `Arc` を渡す。single-writer 前提のためここでは swap のみを行う。
    pub fn publish(&self, next: Arc<MirrorImage>) {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        *guard = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn epoch_monotonic_on_successors() {
        let e0 = MirrorImage::empty();
        assert_eq!(e0.epoch, 0);
        let e1 = e0.successor();
        assert_eq!(e1.epoch, 1);
        let e2 = e1.successor();
        assert_eq!(e2.epoch, 2);
        assert!(e0.epoch < e1.epoch && e1.epoch < e2.epoch);
    }

    #[test]
    fn same_epoch_same_content_via_arc_clone() {
        let shared = SharedMirror::new(Arc::new(MirrorImage::empty()));
        let a = shared.load();
        let b = shared.load();
        assert_eq!(a.epoch, b.epoch);
        assert_eq!(*a, *b);
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn predecessor_image_unchanged_after_publish() {
        let e0 = Arc::new(MirrorImage::empty());
        let shared = SharedMirror::new(Arc::clone(&e0));
        let held_old = shared.load();
        let mut next = e0.successor();
        next.flat_global.insert("k".into(), "v".into());
        shared.publish(Arc::new(next));
        assert_eq!(held_old.epoch, 0);
        assert!(held_old.flat_global.is_empty());
        assert_eq!(shared.load().epoch, 1);
        assert_eq!(
            shared.load().flat_global.get("k").map(String::as_str),
            Some("v")
        );
    }

    #[test]
    fn per_asker_partitions_are_independent() {
        let a1 = AskerId::new("ghost/a");
        let a2 = AskerId::new("ghost/b");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a1.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        img.flat_per_asker
            .entry(a2.clone())
            .or_default()
            .insert("selfname".into(), "Bob".into());
        assert_eq!(
            img.flat_per_asker[&a1].get("selfname").map(String::as_str),
            Some("Alice")
        );
        assert_eq!(
            img.flat_per_asker[&a2].get("selfname").map(String::as_str),
            Some("Bob")
        );
        // a1 に無い名前は a2 の値へ漏れない
        assert!(!img.flat_per_asker[&a1].contains_key("nope"));
    }

    #[test]
    fn dotted_partitions_hold_canonical_string_keys() {
        let asker = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_global
            .insert("system.baseware".into(), "areka".into());
        img.dotted_per_asker
            .entry(asker.clone())
            .or_default()
            .insert("areka.window.scope(0).x".into(), "10".into());
        assert_eq!(
            img.dotted_global.get("system.baseware").map(String::as_str),
            Some("areka")
        );
        assert_eq!(
            img.dotted_per_asker[&asker]
                .get("areka.window.scope(0).x")
                .map(String::as_str),
            Some("10")
        );
    }

    #[test]
    fn shared_mirror_clone_shares_same_lock() {
        let shared = SharedMirror::new(Arc::new(MirrorImage::empty()));
        let clone = shared.clone();
        let mut next = shared.load().successor();
        next.flat_global.insert("x".into(), "1".into());
        shared.publish(Arc::new(next));
        // クローンは同一ロックを共有するので publish が見える
        assert_eq!(clone.load().epoch, 1);
        assert_eq!(
            clone.load().flat_global.get("x").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn multi_reader_threads_share_arc_deterministically() {
        let shared = SharedMirror::new(Arc::new({
            let mut img = MirrorImage::empty();
            img.flat_global.insert("k".into(), "v".into());
            img
        }));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let s = shared.clone();
                std::thread::spawn(move || {
                    let img = s.load();
                    (img.epoch, img.flat_global.get("k").cloned())
                })
            })
            .collect();
        for h in handles {
            let (epoch, v) = h.join().unwrap();
            assert_eq!(epoch, 0);
            assert_eq!(v.as_deref(), Some("v"));
        }
    }

    #[test]
    fn empty_image_has_all_partitions_empty() {
        let img = MirrorImage::empty();
        assert_eq!(img.epoch, 0);
        assert!(img.flat_per_asker.is_empty());
        assert!(img.flat_global.is_empty());
        assert!(img.dotted_global.is_empty());
        assert!(img.dotted_per_asker.is_empty());
    }
}
