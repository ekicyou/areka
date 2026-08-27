//! 消費者台帳（コマンド名→担当消費者の宣言表＋一意性檻・design.md「dola — 名前写像 API の
//! 退役（＋areka 消費者台帳）」・R2.2/R2.6・D10）。
//!
//! dola（配送層）はコマンド名の語彙も名前写像 API も持たず（`command_target_of` 退役・D10）、
//! 汎用コマンド cue を完全に不透明な荷物として broadcast する。各消費者は自らのコマンド名
//! リテラル（`MoveCueSink`＝`"move"`・seriko＝`"bind"`）で cue を**名前自己選別**する
//! （`move_cue.rs` の `MoveCueSink::emit`）。
//!
//! 本台帳はその自己選別モデルの一意性不変条件を、結線層（areka app 層）で宣言・検証する単一
//! 権威表である。dola の権威表ではない（配送層は語彙フリー）。以後のコマンド追加は
//! 「消費者＋本表 1 行」のみで **dola 無改変**（R2.6 の構造的保証）。
//!
//! # 登記の粒度＝「名前＋選別子」（要件 11.2/11.3・task 3.2）
//!
//! 消費の粒度はコマンド名だけとは限らない。`\![move]` は名前だけで担当が決まる（`MoveCueSink`
//! は第 1 引数を見ずに `name != "move"` で選別する）が、`\![set,zorder,…]` は**第 1 引数まで
//! 見て**初めて担当が決まる（`ZOrderCueSink` は `("set","zorder")`／`("reset","zorder")` の組
//! だけを受理し、`\![set,他]` は担当外として読み飛ばす）。
//!
//! そこで登記のキーを **(コマンド名, 選別子)** の組とし、選別子＝**第 1 引数**を
//! `Option<String>` で持つ。2 つの形の意味は次のとおり。
//!
//! - **選別子なし**（`(name, None)`）＝「この名前の出現は、第 1 引数が何であれ全てこの担当」。
//!   `("move", None)`・`("bind", None)` がこれ。
//! - **選別子つき**（`(name, Some(sel))`）＝「この名前の出現のうち、第 1 引数がちょうど `sel`
//!   のものだけがこの担当」。`("set", Some("zorder"))`・`("reset", Some("zorder"))` がこれ。
//!   名簿に無い第 1 引数（例: `\![set,windowstate]`）は**担当なし**であり、将来 別の担当を
//!   足せる余地として空いている（要件 11.3）。
//!
//! # 一意性檻＝「1 コマンド出現に高々 1 担当」（要件 11.3）
//!
//! 不変条件は名前ではなく**実際の消費の粒度**（名前＋第 1 引数）で述べる。台帳はこれを 2 つの
//! 規則で保つ。
//!
//! 1. **同じ組の二重登録を拒む**——[`ConsumerLedger::try_register`] は既登記の組へ再登記すると
//!    [`LedgerError::Duplicate`] を返す（多重結線の検出を観測可能化する）。
//! 2. **同一名で選別子の有無を共存させない**——`("set", None)` と `("set", Some("zorder"))` が
//!    同居すると `\![set,zorder,…]` の 1 出現に 2 つの担当が作用してしまうため、
//!    [`LedgerError::SelectorConflict`] で拒む。順序はどちらでも同じく拒む。
//!
//! 正準台帳 [`ConsumerLedger::canonical`] はこの try_register を用いて 4 行（`move`・`bind`・
//! `(set,zorder)`・`(reset,zorder)`）を登記し、違反があれば構築時に panic する（正準表は一意
//! ゆえ実際には発火しない・回帰檻）。
//!
//! # 宣言する表であって、選別する機構ではない
//!
//! 本表は結線時の宣言・検証のためだけにあり、実行時に各消費者がここを引くわけではない
//! （消費者は自らの名前と選別子で自己選別する）。両者は一致していなければならないが、
//! 依存はしない——`ZOrderCueSink` が受理する組と `canonical()` の 4 行目までが同じであることは
//! 本モジュールのテストが名指しで固定する。
#![allow(dead_code)]

use std::collections::BTreeMap;

/// 台帳のキー＝（コマンド名, 選別子＝第 1 引数）。選別子なしは「名前まるごと」を意味する。
///
/// 決定論のため `BTreeMap` のキーとして使う。`Option` の順序（`None` < `Some(_)`）により、
/// 同一名の登記は「選別子なし → 選別子つき（辞書順）」の順で並ぶ。
type LedgerKey = (String, Option<String>);

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
/// - [`ZOrderSink`](CommandConsumer::ZOrderSink): 重なり指定・重なり解除を消費する
///   [`ZOrderCueSink`](super::zorder_cue::ZOrderCueSink)。正準台帳が `(set, zorder)`・
///   `(reset, zorder)` の 2 組を登記する（要件 11.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandConsumer {
    /// `\![move]` の担当消費者（[`MoveCueSink`](super::move_cue::MoveCueSink)）。
    MoveSink,
    /// `\![bind]`（着せ替え）の担当消費者（表示系 seriko）。正準台帳が登記する（task 7.2）。
    Seriko,
    /// 重なり指定・重なり解除の担当消費者
    /// （[`ZOrderCueSink`](super::zorder_cue::ZOrderCueSink)）。名前だけでは決まらず、
    /// 第 1 引数が `zorder` の出現だけを担当する（要件 11.2）。
    ZOrderSink,
}

/// 選別子を記録本文へ書くときの見え方（「無い」側も読める形にする——片側だけの本文では
/// 衝突が読めないため・要件 8.3 の規律）。
fn selector_label(selector: &Option<String>) -> String {
    match selector {
        Some(selector) => format!("'{selector}'"),
        None => "なし".to_string(),
    }
}

/// 消費者台帳の構築時失敗（一意性違反の観測可能化・log-first ではなく Result で呼び手へ返す）。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LedgerError {
    /// 同じ（コマンド名, 選別子）の組が二重登録された（多重結線＝要件 11.3 の禁止事項）。
    #[error(
        "コマンド名 '{name}'（選別子: {}）が消費者台帳へ二重登録されました（1 出現=高々 1 担当・要件 11.3）",
        selector_label(.selector)
    )]
    Duplicate {
        /// 二重登録が検出されたコマンド名。
        name: String,
        /// 二重登録が検出された選別子（第 1 引数。名前まるごとの登記なら `None`）。
        selector: Option<String>,
    },
    /// 同一コマンド名について、選別子を伴う登記と伴わない登記を同居させようとした。
    ///
    /// 同居すると、選別子つきの組に当たる 1 出現へ 2 つの担当が作用してしまい
    /// 「1 コマンド出現に高々 1 担当」（要件 11.3）が破れる。順序はどちらでも拒む。
    #[error(
        "コマンド名 '{name}' は選別子 {} で登記済みのため、選別子 {} を追加できません\
         （同一名で選別子の有無は共存できない・1 出現=高々 1 担当・要件 11.3）",
        selector_label(.existing),
        selector_label(.incoming)
    )]
    SelectorConflict {
        /// 衝突が起きたコマンド名。
        name: String,
        /// 既に登記されていた側の選別子。
        existing: Option<String>,
        /// 追加しようとした側の選別子。
        incoming: Option<String>,
    },
}

/// （コマンド名, 選別子）→ 担当消費者の宣言台帳（結線層の単一権威表・要件 11.2/11.3）。
///
/// 登記は [`try_register`](Self::try_register)（重複と選別子の排他違反を拒否）で行い、
/// 照会は [`consumer_of`](Self::consumer_of)（未登記＝`None`＝自己選別モデルの良性未処理）で行う。
#[derive(Debug, Default, Clone)]
pub struct ConsumerLedger {
    /// （コマンド名, 選別子）→担当消費者（決定論のため `BTreeMap`）。
    table: BTreeMap<LedgerKey, CommandConsumer>,
}

impl ConsumerLedger {
    /// 空の台帳を作る（登記は [`try_register`](Self::try_register) で追加する）。
    pub fn new() -> Self {
        Self::default()
    }

    /// （コマンド名, 選別子）の組を担当消費者へ登記する。
    ///
    /// `selector` は消費の粒度を決める**第 1 引数**である。`None` は「この名前の出現は第 1 引数に
    /// よらず全てこの担当」を、`Some(sel)` は「第 1 引数がちょうど `sel` の出現だけがこの担当」を
    /// 宣言する（モジュール doc「登記の粒度」）。
    ///
    /// 次の 2 つを拒む（いずれも観測可能な `Err`。黙って上書きも黙って無視もしない）。
    ///
    /// - 同じ組の再登記 → [`LedgerError::Duplicate`]
    /// - 同一名で選別子の有無が食い違う登記 → [`LedgerError::SelectorConflict`]
    ///
    /// 拒否したときは表を一切変更しない（既存の担当は据え置き）。
    pub fn try_register(
        &mut self,
        name: &str,
        selector: Option<&str>,
        consumer: CommandConsumer,
    ) -> Result<(), LedgerError> {
        let key: LedgerKey = (name.to_string(), selector.map(str::to_string));
        if self.table.contains_key(&key) {
            return Err(LedgerError::Duplicate {
                name: key.0,
                selector: key.1,
            });
        }
        // 同一名の既存登記だけを走る（`(name, None)` は同名キーの最小値なので、そこから
        // 名前が変わるまでが同一名の範囲）。選別子の有無が食い違うものが 1 つでもあれば排他違反。
        let same_name_from: LedgerKey = (name.to_string(), None);
        let conflict = self
            .table
            .range(same_name_from..)
            .take_while(|((existing_name, _), _)| existing_name.as_str() == name)
            .find(|((_, existing_selector), _)| existing_selector.is_none() != key.1.is_none());
        if let Some(((_, existing_selector), _)) = conflict {
            return Err(LedgerError::SelectorConflict {
                name: key.0.clone(),
                existing: existing_selector.clone(),
                incoming: key.1,
            });
        }
        self.table.insert(key, consumer);
        Ok(())
    }

    /// 1 つのコマンド出現（コマンド名＋第 1 引数）の担当消費者を引く。
    ///
    /// 引き方はモジュール doc「登記の粒度」の 2 つの形をそのまま写したもの。
    ///
    /// 1. 名前まるごとの登記 `(name, None)` があれば、`selector` が何であれそれが担当である
    ///    （`\![move]` も `\![move,--x=10]` も同じ `MoveSink`）。
    /// 2. 無ければ、`selector` とちょうど同じ選別子つきの登記を引く。名簿に無い第 1 引数と、
    ///    第 1 引数の無い出現はどちらも `None`＝担当なしになる。
    ///
    /// 相互排他（[`LedgerError::SelectorConflict`]）により ⑴ と ⑵ は同時に成り立たないので、
    /// 1 出現が引く担当は高々 1 つである（要件 11.3）。未登記名は `None`——自己選別モデルでは
    /// 担当のいないコマンドは良性の未処理であって、異常ではない（要件 11.2）。
    pub fn consumer_of(&self, name: &str, selector: Option<&str>) -> Option<CommandConsumer> {
        if let Some(consumer) = self.table.get(&(name.to_string(), None)) {
            return Some(*consumer);
        }
        let selector = selector?;
        self.table
            .get(&(name.to_string(), Some(selector.to_string())))
            .copied()
    }

    /// 正準台帳を構築する（現行登記＝`move` → [`CommandConsumer::MoveSink`]・`bind` →
    /// [`CommandConsumer::Seriko`]・`(set, zorder)` と `(reset, zorder)` →
    /// [`CommandConsumer::ZOrderSink`]）。
    ///
    /// 後ろの 2 行は `ZOrderCueSink` が自己選別する組とちょうど同じである（表は宣言し、受け口は
    /// 自ら選別する——実行時に受け口が本表を引くわけではない）。
    ///
    /// 以後のコマンド追加は「消費者＋本表 1 行（`try_register`）」のみで **dola 無改変**（R2.6）。
    /// 正準表は一意ゆえ [`try_register`](Self::try_register) は Ok を返す——万一将来の編集で重複や
    /// 選別子の排他違反を作れば `expect` が構築時に panic して回帰を止める（一意性の内部整合檻）。
    pub fn canonical() -> Self {
        let mut ledger = Self::new();
        ledger
            .try_register("move", None, CommandConsumer::MoveSink)
            .expect("正準台帳: 'move'（選別子なし）は一意（重複・排他違反は編集ミス）");
        ledger
            .try_register("bind", None, CommandConsumer::Seriko)
            .expect("正準台帳: 'bind'（選別子なし）は一意（重複・排他違反は編集ミス）");
        ledger
            .try_register("set", Some("zorder"), CommandConsumer::ZOrderSink)
            .expect("正準台帳: ('set','zorder') は一意（重複・排他違反は編集ミス）");
        ledger
            .try_register("reset", Some("zorder"), CommandConsumer::ZOrderSink)
            .expect("正準台帳: ('reset','zorder') は一意（重複・排他違反は編集ミス）");
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
            ledger.consumer_of("move", None),
            Some(CommandConsumer::MoveSink),
            "正準台帳は move（選別子なし）→ MoveSink を登記している（R2.2）"
        );
        assert_eq!(
            ledger.consumer_of("bind", None),
            Some(CommandConsumer::Seriko),
            "正準台帳は bind（選別子なし）→ Seriko を登記している（task 7.2・R2.2）"
        );
    }

    /// 正準台帳は重なり指定・重なり解除の 2 組を [`CommandConsumer::ZOrderSink`] として登記する
    /// （名前＋選別子の粒度＝要件 11.2/11.3・task 3.2）。受け口 `ZOrderCueSink` が自己選別する
    /// 組（`("set","zorder")`・`("reset","zorder")`）とちょうど一致していること。
    #[test]
    fn canonical_registers_both_zorder_pairs() {
        let ledger = ConsumerLedger::canonical();
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::ZOrderSink),
            "正準台帳は (set, zorder) → ZOrderSink を登記している（要件 11.2）"
        );
        assert_eq!(
            ledger.consumer_of("reset", Some("zorder")),
            Some(CommandConsumer::ZOrderSink),
            "正準台帳は (reset, zorder) → ZOrderSink を登記している（要件 11.2）"
        );
    }

    /// 未登記のコマンド名は選別子の有無によらず `None`（自己選別モデルでは未知名は良性未処理・
    /// 要件 11.2）。
    #[test]
    fn unregistered_name_is_none() {
        let ledger = ConsumerLedger::canonical();
        assert_eq!(
            ledger.consumer_of("noexist", None),
            None,
            "登記のないコマンド名は None（自己選別モデルの良性未処理・要件 11.2）"
        );
        assert_eq!(
            ledger.consumer_of("noexist", Some("zorder")),
            None,
            "選別子を伴っても、名前ごと未登記なら None（要件 11.2）"
        );
    }

    /// 名前まるごとの登記（選別子なし）は、第 1 引数が何であってもその名前の全出現を担当する。
    /// `MoveCueSink` が名前だけを見て自己選別している実体（`move_cue.rs` の `name != "move"`）と
    /// 揃えるための規則。**両側から挟む**: 第 1 引数の無い出現も、有る出現も同じ担当を引く。
    #[test]
    fn selectorless_registration_owns_every_first_parameter() {
        let ledger = ConsumerLedger::canonical();
        assert_eq!(
            ledger.consumer_of("move", None),
            Some(CommandConsumer::MoveSink),
            "第 1 引数の無い move は MoveSink の担当"
        );
        assert_eq!(
            ledger.consumer_of("move", Some("--x=10")),
            Some(CommandConsumer::MoveSink),
            "第 1 引数を伴う move も同じ MoveSink の担当（名前まるごとの登記は第 1 引数を問わない）"
        );
    }

    /// 選別子つきの登記は、その選別子ちょうどの出現だけを担当する。名簿に無い第 1 引数と、
    /// 第 1 引数の無い出現は、どちらも担当なし＝良性未処理（要件 11.2）。
    /// **両側から挟む**: 名簿にある `zorder` は引けて、名簿に無い `windowstate` と裸の `set` は
    /// 引けない。
    #[test]
    fn selector_bearing_registration_owns_only_listed_selectors() {
        let ledger = ConsumerLedger::canonical();
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::ZOrderSink),
            "名簿にある選別子は引ける"
        );
        assert_eq!(
            ledger.consumer_of("set", None),
            None,
            "第 1 引数の無い裸の set は担当なし（名前まるごとの登記ではない・要件 11.2）"
        );
        assert_eq!(
            ledger.consumer_of("set", Some("windowstate")),
            None,
            "名簿に無い第 1 引数は担当なし＝将来の担当のための余地（要件 11.3）"
        );
        assert_eq!(
            ledger.consumer_of("reset", None),
            None,
            "裸の reset も同様に担当なし（要件 11.2）"
        );
    }

    /// 一意性檻: 同じ名前と同じ選別子の組を二重登録しようとすると [`LedgerError::Duplicate`]
    /// で検出できる（1 出現＝高々 1 担当・要件 11.3）。選別子なしの組でも同じ。
    #[test]
    fn duplicate_registration_is_detected() {
        let mut ledger = ConsumerLedger::new();
        ledger
            .try_register("move", None, CommandConsumer::MoveSink)
            .expect("初回登記は成功する");
        let err = ledger
            .try_register("move", None, CommandConsumer::Seriko)
            .expect_err("同一の組の二重登録は Err で検出される（1 出現=高々 1 担当・要件 11.3）");
        assert_eq!(
            err,
            LedgerError::Duplicate {
                name: "move".to_string(),
                selector: None,
            },
            "重複は Duplicate{{name, selector}} として観測可能"
        );
        // 二重登録は最初の登記を上書きしない（担当は据え置き）。
        assert_eq!(
            ledger.consumer_of("move", None),
            Some(CommandConsumer::MoveSink)
        );
    }

    /// **完了条件の中核（要件 11.3）**: 同じコマンド名の**別の第 1 引数**は、後から**別の担当**として
    /// 登記できる。重なり指定を本機能が持ったまま、将来 `set` の別のサブコマンドに別の担当が
    /// 付けられる余地が型で残っていること。
    #[test]
    fn different_selector_under_same_name_registers_as_another_consumer() {
        let mut ledger = ConsumerLedger::canonical();
        ledger
            .try_register("set", Some("windowstate"), CommandConsumer::Seriko)
            .expect("同名・別選別子は重複でない（将来の担当の余地・要件 11.3）");

        // 2 つの選別子が別々の担当を引く（相互に混ざらない）。
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::ZOrderSink),
            "先の登記は後から足した選別子に押しのけられない"
        );
        assert_eq!(
            ledger.consumer_of("set", Some("windowstate")),
            Some(CommandConsumer::Seriko),
            "後から足した選別子は自分の担当を引く（要件 11.3 の余地）"
        );
    }

    /// 上のテストと**対で**置く: 同じコマンド名の**同じ第 1 引数**は後から別担当にできない。
    /// 「別の選別子なら足せる」が「何でも足せる」に化けていないことを、隣り合わせで示す。
    #[test]
    fn same_selector_under_same_name_is_rejected() {
        let mut ledger = ConsumerLedger::canonical();
        let err = ledger
            .try_register("set", Some("zorder"), CommandConsumer::Seriko)
            .expect_err("同名・同選別子の再登記は Err（1 出現=高々 1 担当・要件 11.3）");
        assert_eq!(
            err,
            LedgerError::Duplicate {
                name: "set".to_string(),
                selector: Some("zorder".to_string()),
            },
            "同じ組の再登記は Duplicate として観測可能"
        );
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::ZOrderSink),
            "既登記の担当は据え置き（上書きしない）"
        );
    }

    /// 相互排他（選別子つき → 選別子なしの順）: `("set","zorder")` が居るところへ
    /// 名前まるごとの `("set", なし)` は足せない。足せてしまうと重なり指定の 1 出現に
    /// 2 つの担当が作用し「1 出現＝高々 1 担当」（要件 11.3）が破れる。
    #[test]
    fn selectorless_after_selector_bearing_conflicts() {
        let mut ledger = ConsumerLedger::new();
        ledger
            .try_register("set", Some("zorder"), CommandConsumer::ZOrderSink)
            .expect("初回登記は成功する");
        let err = ledger
            .try_register("set", None, CommandConsumer::Seriko)
            .expect_err("選別子つきが居るところへ名前まるごとの登記は入れない（要件 11.3）");
        assert_eq!(
            err,
            LedgerError::SelectorConflict {
                name: "set".to_string(),
                existing: Some("zorder".to_string()),
                incoming: None,
            },
            "排他違反は SelectorConflict として、名前と両側の選別子を名指しで観測できる"
        );
        // 拒否された登記は表に載らない（既存の担当だけが残る）。
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::ZOrderSink)
        );
        assert_eq!(ledger.consumer_of("set", None), None);
    }

    /// 相互排他（選別子なし → 選別子つきの順）: 上と**逆の順**でも同じく拒否される。
    /// 片方の順序しか押さえない檻は、順序に依存した実装（後勝ちの上書き等）を素通りさせる。
    #[test]
    fn selector_bearing_after_selectorless_conflicts() {
        let mut ledger = ConsumerLedger::new();
        ledger
            .try_register("set", None, CommandConsumer::Seriko)
            .expect("初回登記は成功する");
        let err = ledger
            .try_register("set", Some("zorder"), CommandConsumer::ZOrderSink)
            .expect_err("名前まるごとが居るところへ選別子つきの登記は入れない（要件 11.3）");
        assert_eq!(
            err,
            LedgerError::SelectorConflict {
                name: "set".to_string(),
                existing: None,
                incoming: Some("zorder".to_string()),
            },
            "逆順でも排他違反は SelectorConflict として観測できる"
        );
        // 拒否された登記は表に載らない。名前まるごとの担当だけが残り、その担当は
        // 第 1 引数によらず引ける。
        assert_eq!(
            ledger.consumer_of("set", None),
            Some(CommandConsumer::Seriko)
        );
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::Seriko),
            "名前まるごとの登記が残っている以上、zorder も同じ担当が引かれる"
        );
    }

    /// 排他違反の記録は、どの名前のどの選別子どうしが衝突したかを本文で名指しする
    /// （黙って諦めない・要件 8.3 の規律）。
    #[test]
    fn selector_conflict_message_names_the_offending_pair() {
        let err = LedgerError::SelectorConflict {
            name: "set".to_string(),
            existing: Some("zorder".to_string()),
            incoming: None,
        };
        let text = err.to_string();
        assert!(text.contains("set"), "本文にコマンド名が載る: {text}");
        assert!(text.contains("zorder"), "本文に既存の選別子が載る: {text}");
        assert!(
            text.contains("なし"),
            "本文に「選別子なし」の側も載る（片側だけでは衝突が読めない）: {text}"
        );
    }

    /// 別のコマンド名どうしは互いに干渉しない（排他は同一名の中だけの規則）。
    /// 排他の実装が名前をまたいで効いてしまうと、正準台帳の 4 行がそもそも組めなくなる。
    #[test]
    fn exclusion_applies_only_within_the_same_name() {
        let mut ledger = ConsumerLedger::new();
        ledger
            .try_register("set", Some("zorder"), CommandConsumer::ZOrderSink)
            .expect("(set, zorder) の登記");
        ledger
            .try_register("move", None, CommandConsumer::MoveSink)
            .expect("別名 move の名前まるごと登記は (set,zorder) と衝突しない");
        ledger
            .try_register("reset", Some("zorder"), CommandConsumer::ZOrderSink)
            .expect("別名 reset の選別子つき登記も衝突しない");
        assert_eq!(
            ledger.consumer_of("move", None),
            Some(CommandConsumer::MoveSink)
        );
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::ZOrderSink)
        );
        assert_eq!(
            ledger.consumer_of("reset", Some("zorder")),
            Some(CommandConsumer::ZOrderSink)
        );
    }

    /// 正準台帳の構築は重複・排他違反なしで成功する（内部整合＝一意性檻が緑）。4 エントリが
    /// 共存しても檻は保たれ、既登記の組の再登記は [`LedgerError::Duplicate`] で検出され、
    /// 別名の追加は独立に成功する（task 7.2・要件 11.3）。
    #[test]
    fn canonical_builds_without_duplicate() {
        // canonical() は内部 try_register（4 行）が Ok（重複なら expect が panic する）。
        let ledger = ConsumerLedger::canonical();
        assert_eq!(
            ledger.consumer_of("move", None),
            Some(CommandConsumer::MoveSink)
        );
        assert_eq!(
            ledger.consumer_of("bind", None),
            Some(CommandConsumer::Seriko)
        );
        assert_eq!(
            ledger.consumer_of("set", Some("zorder")),
            Some(CommandConsumer::ZOrderSink)
        );
        assert_eq!(
            ledger.consumer_of("reset", Some("zorder")),
            Some(CommandConsumer::ZOrderSink)
        );

        // 4 エントリ共存下でも一意性檻は保たれる: 既登記の組 bind の再登記は Duplicate で
        // 検出される。
        let mut ext = ledger.clone();
        let err = ext
            .try_register("bind", None, CommandConsumer::MoveSink)
            .expect_err("既登記の組 bind の再登記は Err で検出される（要件 11.3）");
        assert_eq!(
            err,
            LedgerError::Duplicate {
                name: "bind".to_string(),
                selector: None,
            },
            "4 エントリ共存下でも重複は Duplicate{{name, selector}} として観測可能"
        );
        // 既登記の担当は据え置き（上書きしない）。
        assert_eq!(ext.consumer_of("bind", None), Some(CommandConsumer::Seriko));

        // 相異なる名前は独立に登記できる（別名は重複でない）。
        ext.try_register("resize", None, CommandConsumer::Seriko)
            .expect("別名 resize の追加は重複でない");
        assert_eq!(
            ext.consumer_of("resize", None),
            Some(CommandConsumer::Seriko)
        );
    }
}
