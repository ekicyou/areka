//! 読み込みの記録を出すかどうかの口（spec: placement-measure-bake-once 要件 3.4・3.6）。
//!
//! 読み込みの中身は 1 本のまま、入口がこの値を渡して記録の行を `if records.emits() { … }` で
//! 切り替える。読み込みを `Err` で終える失敗の記録は、この値に関わらず出す。

/// 読み込みの記録を出すかどうか（読み込みの中身は同じ 1 本）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoadRecords {
    /// 今までどおり出す（既存の入口・着替えの道・起動の資産の組み立て）。
    Emit,
    /// 読み込みの記録を出さない（採寸・windowposition の読み）。失敗の記録は出す。
    Quiet,
}

impl LoadRecords {
    /// 読み込みの記録を出すとき `true`。
    pub(crate) fn emits(self) -> bool {
        self == LoadRecords::Emit
    }
}
