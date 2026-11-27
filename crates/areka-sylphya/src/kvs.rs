//! KVS の口: 鍵の名前と層の名前だけで読む・書く・消す口と、書き・消しの結果。

use crate::persist::PersistFailure;

/// 書き・消しの結果（4 つの形）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteOutcome {
    /// 書けた。
    Written,
    /// 名前では書けない鍵だったので断った。
    Refused,
    /// 書けなかった。
    Failed(WriteFailed),
    /// 読み取りだけの走りなので控えた。
    Withheld,
}

/// 書けなかった理由。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteFailed {
    /// 覚える値を書けなかった（告知の道へ渡る中身と同じ物）。
    Persist(PersistFailure),
    /// 揮発の値・速い値を書けなかった（告知の道へは渡らない）。
    Volatile { key: String, reason: String },
    /// 番人が止まっていて、返事が来なかった（告知の道へは渡らない）。
    Stopped,
}
