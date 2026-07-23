//! 消費者台帳（コマンド名→担当消費者の宣言表＋一意性檻・design.md「dola — 名前写像 API の
//! 退役（＋areka 消費者台帳）」・R2.2/R2.6・D10）。
//!
//! dola（配送層）はコマンド名の語彙も名前写像 API も持たず（`command_target_of` 退役・D10）、
//! 汎用コマンド cue を完全に不透明な荷物として broadcast する。各消費者は自らのコマンド名
//! リテラル（`MoveCueSink`＝`"move"`・seriko＝`"bind"`）で cue を**名前自己選別**する
//! （`move_cue.rs` の `MoveCueSink::emit`）。
//!
//! 本台帳はその自己選別モデルの一意性不変条件——**「1 コマンド名の担当消費者は高々 1」**——を
//! 結線層（areka app 層）で宣言・検証する単一権威表である。dola の権威表ではない（配送層は
//! 語彙フリー）。以後のコマンド追加は「消費者＋本表 1 行」のみで **dola 無改変**（R2.6 の
//! 構造的保証）。
//!
//! # 一意性檻（R2.2）
//!
//! [`ConsumerLedger::try_register`] は同一コマンド名を二重登録しようとすると
//! [`LedgerError::Duplicate`] を返す（多重結線の検出を観測可能化する）。正準台帳
//! [`ConsumerLedger::canonical`] はこの try_register を用いて `move`・`bind` を
//! 登記し、重複があれば構築時に panic する（正準表は一意ゆえ実際には発火しない・回帰檻）。

use std::collections::BTreeMap;

/// コマンド名を消費する担当消費者の識別子（軽量・design File Structure「move→MoveCueSink・
/// bind→seriko」）。
///
/// dola はコマンド名語彙を持たないため、本 enum は**結線層が誰にコマンド名を割り当てたか**を
/// 宣言するためだけの識別子である（実際の消費は各消費者が自らの名前で自己選別する）。
///
/// - [`MoveSink`](CommandConsumer::MoveSink): `\![move]` を消費する
///   [`MoveCueSink`](super::move_cue::MoveCueSink)（talk スレッド側名前選別 sink）。
/// - [`Seriko`](CommandConsumer::Seriko): `\![bind]`（着せ替え）を消費する表示系 seriko。
///   正準台帳（[`ConsumerLedger::canonical`]）が `bind` → `Seriko` を登記する（task 7.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandConsumer {
    /// `\![move]` の担当消費者（[`MoveCueSink`](super::move_cue::MoveCueSink)）。
    MoveSink,
    /// `\![bind]`（着せ替え）の担当消費者（表示系 seriko）。正準台帳が登記する（task 7.2）。
    Seriko,
}

/// 消費者台帳の構築時失敗（一意性違反の観測可能化・log-first ではなく Result で呼び手へ返す）。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LedgerError {
    /// 同一コマンド名が二重登録された（多重結線＝R2.2 の禁止事項）。
    #[error("コマンド名 '{name}' が消費者台帳へ二重登録されました（1 名前=高々 1 消費者・R2.2）")]
    Duplicate {
        /// 二重登録が検出されたコマンド名。
        name: String,
    },
}

/// コマンド名 → 担当消費者の宣言台帳（結線層の単一権威表・R2.2/R2.6）。
///
/// 登記は [`try_register`](Self::try_register)（重複を [`LedgerError::Duplicate`] で拒否）で行い、
/// 照会は [`consumer_of`](Self::consumer_of)（未登記＝`None`＝自己選別モデルの良性未処理）で行う。
#[derive(Debug, Default, Clone)]
pub struct ConsumerLedger {
    /// コマンド名→担当消費者（決定論のため `BTreeMap`）。
    table: BTreeMap<String, CommandConsumer>,
}

impl ConsumerLedger {
    /// 空の台帳を作る（登記は [`try_register`](Self::try_register) で追加する）。
    pub fn new() -> Self {
        Self::default()
    }

    /// コマンド名を担当消費者へ登記する。同一名が既に登記済みなら
    /// [`LedgerError::Duplicate`] を返す（多重結線の検出＝R2.2・観測可能）。
    pub fn try_register(
        &mut self,
        name: &str,
        consumer: CommandConsumer,
    ) -> Result<(), LedgerError> {
        if self.table.contains_key(name) {
            return Err(LedgerError::Duplicate {
                name: name.to_string(),
            });
        }
        self.table.insert(name.to_string(), consumer);
        Ok(())
    }

    /// コマンド名の担当消費者を引く。未登記なら `None`（自己選別モデルでは未知名は良性未処理・R2.5）。
    pub fn consumer_of(&self, name: &str) -> Option<CommandConsumer> {
        self.table.get(name).copied()
    }

    /// 正準台帳を構築する（現行登記＝`move` → [`CommandConsumer::MoveSink`]・`bind` →
    /// [`CommandConsumer::Seriko`]）。
    ///
    /// 以後のコマンド追加は「消費者＋本表 1 行（`try_register`）」のみで **dola 無改変**（R2.6）。
    /// 正準表は一意ゆえ [`try_register`](Self::try_register) は Ok を返す——万一将来の編集で重複を
    /// 作れば `expect` が構築時に panic して回帰を止める（一意性の内部整合檻）。
    pub fn canonical() -> Self {
        let mut ledger = Self::new();
        ledger
            .try_register("move", CommandConsumer::MoveSink)
            .expect("正準台帳: 'move' は一意（重複は編集ミス）");
        ledger
            .try_register("bind", CommandConsumer::Seriko)
            .expect("正準台帳: 'bind' は一意（重複は編集ミス）");
        ledger
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 正準台帳に `move`・`bind` が共に登記されている → 照会で `move`→[`CommandConsumer::MoveSink`]・
    /// `bind`→[`CommandConsumer::Seriko`] が引ける（両登録の共存・Observable の中核・task 7.2）。
    #[test]
    fn canonical_has_move_and_bind_registered() {
        let ledger = ConsumerLedger::canonical();
        assert_eq!(
            ledger.consumer_of("move"),
            Some(CommandConsumer::MoveSink),
            "正準台帳は move → MoveSink を登記している（R2.2）"
        );
        assert_eq!(
            ledger.consumer_of("bind"),
            Some(CommandConsumer::Seriko),
            "正準台帳は bind → Seriko を登記している（task 7.2・R2.2）"
        );
    }

    /// 未登記のコマンド名は `None`（自己選別モデルでは未知名は良性未処理・R2.5）。
    #[test]
    fn unregistered_name_is_none() {
        let ledger = ConsumerLedger::canonical();
        assert_eq!(
            ledger.consumer_of("noexist"),
            None,
            "登記のないコマンド名は None（自己選別モデルの良性未処理・R2.5）"
        );
    }

    /// 一意性檻（R2.2）: 同一コマンド名を二重登録しようとすると
    /// [`LedgerError::Duplicate`] で検出できる（多重結線なしを決定論的に観測）。
    #[test]
    fn duplicate_registration_is_detected() {
        let mut ledger = ConsumerLedger::new();
        ledger
            .try_register("move", CommandConsumer::MoveSink)
            .expect("初回登記は成功する");
        let err = ledger
            .try_register("move", CommandConsumer::Seriko)
            .expect_err("同一名の二重登録は Err で検出される（1 名前=高々 1 消費者・R2.2）");
        assert_eq!(
            err,
            LedgerError::Duplicate {
                name: "move".to_string(),
            },
            "重複は Duplicate{{name}} として観測可能"
        );
        // 二重登録は最初の登記を上書きしない（担当は据え置き）。
        assert_eq!(ledger.consumer_of("move"), Some(CommandConsumer::MoveSink));
    }

    /// 正準台帳の構築は重複なしで成功する（内部整合＝一意性檻が緑）。二エントリ（move・bind）が
    /// 共存しても一意性檻は保たれ、既登記名（`bind`）の再登記は [`LedgerError::Duplicate`] で
    /// 検出され、別名の追加は独立に成功する（task 7.2・R2.2）。
    #[test]
    fn canonical_builds_without_duplicate() {
        // canonical() は内部 try_register（move・bind）が Ok（重複なら expect が panic する）。
        let ledger = ConsumerLedger::canonical();
        assert_eq!(ledger.consumer_of("move"), Some(CommandConsumer::MoveSink));
        assert_eq!(ledger.consumer_of("bind"), Some(CommandConsumer::Seriko));

        // 二エントリ共存下でも一意性檻は保たれる: 既登記名 bind の再登記は Duplicate で検出される。
        let mut ext = ledger.clone();
        let err = ext
            .try_register("bind", CommandConsumer::MoveSink)
            .expect_err("既登記名 bind の再登記は Err で検出される（1 名前=高々 1 消費者・R2.2）");
        assert_eq!(
            err,
            LedgerError::Duplicate {
                name: "bind".to_string(),
            },
            "二エントリ共存下でも重複は Duplicate{{name}} として観測可能"
        );
        // 既登記の担当は据え置き（上書きしない）。
        assert_eq!(ext.consumer_of("bind"), Some(CommandConsumer::Seriko));

        // 相異なる名前は独立に登記できる（別名は重複でない）。
        ext.try_register("resize", CommandConsumer::Seriko)
            .expect("別名 resize の追加は重複でない");
        assert_eq!(ext.consumer_of("resize"), Some(CommandConsumer::Seriko));
    }
}
