//! フラット 26 トークン台帳＋構文記録（`%*` は解決対象外・`\%` は語彙外）。
//!
//! 全 26 トークンを key モデル上の第一級エントリとして保持する（R1.1）。
//! lexer が届かない `%m?` も第一級（token に `?` を残す・lexer 非依存）。
//! backing 層は brief.md Scope 節の typology に従って割り当て、実導出は
//! M1 で源に着地する 4 件（username/selfname/selfname2/keroname）のみ。

use crate::vocab::{BackingLayer, DegradePolicy, M1Status};

/// フラット語彙 1 件（`% 抜きの名`・backing 層・M1 実導出状態・縮退政策）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlatEntry {
    /// 例 "username"（先頭 `%` を除いた名。`%m?` は "m?" と `?` を残す）。
    pub token: &'static str,
    /// backing 実体層（R3.6・5 層タクソノミー）。
    pub layer: BackingLayer,
    /// M1 実導出状態（源着地済みか、縮退中か）。
    pub m1: M1Status,
    /// 縮退政策（R3.1）。username のみ ConsumerDefault・他は PassThroughRaw。
    pub degrade: DegradePolicy,
}

/// フラット 26 トークン全数（件数檻: `FLAT_VOCAB.len() == 26`・R1.1）。
///
/// 層割当（brief.md Scope typology）:
/// - `username` = ShioriQuery（SHIORI Resource 照会）。
/// - `selfname`/`selfname2`/`keroname` = StaticConfig（descript 由来）。
/// - 暦時計・時刻ネタ・画面・OS 起動時間（month..second/et/wronghour/
///   screenwidth/screenheight/exh）= SystemEnv（OS 由来・注入シーム必須）。
/// - 単語ランダム系・インストール文脈（ms..dms/lastghostname/lastobjectname）
///   = RuntimeState（他エンジン所有の運行状態・出所は正典未規定＝縮退のまま予約）。
pub const FLAT_VOCAB: &[FlatEntry] = &[
    // 暦時計（SystemEnv・縮退・素通し）。
    FlatEntry {
        token: "month",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "day",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "hour",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "minute",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "second",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    // M1 実導出（源着地済み）。username のみ ConsumerDefault。
    FlatEntry {
        token: "username",
        layer: BackingLayer::ShioriQuery,
        m1: M1Status::Derived,
        degrade: DegradePolicy::ConsumerDefault,
    },
    FlatEntry {
        token: "selfname",
        layer: BackingLayer::StaticConfig,
        m1: M1Status::Derived,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "selfname2",
        layer: BackingLayer::StaticConfig,
        m1: M1Status::Derived,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "keroname",
        layer: BackingLayer::StaticConfig,
        m1: M1Status::Derived,
        degrade: DegradePolicy::PassThroughRaw,
    },
    // 画面・OS 起動時間・時刻ネタ（SystemEnv・縮退・素通し）。
    FlatEntry {
        token: "screenwidth",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "screenheight",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "exh",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "et",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "wronghour",
        layer: BackingLayer::SystemEnv,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    // 単語ランダム系 10（RuntimeState・縮退・素通し・出所は正典未規定）。
    // `m?` は lexer で `?` がトークン化不能だが key モデル上は第一級（token に `?` を残す）。
    FlatEntry {
        token: "ms",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "mz",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "ml",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "mc",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "mh",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "mt",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "me",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "mp",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "m?",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "dms",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    // インストール文脈（RuntimeState・縮退・素通し・M1 外）。
    FlatEntry {
        token: "lastghostname",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
    FlatEntry {
        token: "lastobjectname",
        layer: BackingLayer::RuntimeState,
        m1: M1Status::Degraded,
        degrade: DegradePolicy::PassThroughRaw,
    },
];

/// 構文記録（`%*`・`%property[...]`）——解決対象外の別記法として語彙記録のみ行う（R1.6）。
///
/// `\%`（エスケープ記法）はここにも `FLAT_VOCAB` にも含めない。
pub const SYNTAX_RECORDS: &[&str] = &["*", "property[...]"];

#[cfg(test)]
mod tests {
    use super::*;

    /// 件数檻（R1.1・観測可能な完了条件）。
    #[test]
    fn flat_vocab_has_26_entries() {
        assert_eq!(FLAT_VOCAB.len(), 26);
    }

    /// 26 トークン全数が過不足なく一致（タイポ・重複を檻化）。
    #[test]
    fn flat_vocab_tokens_match_exactly() {
        let expected: [&str; 26] = [
            "month",
            "day",
            "hour",
            "minute",
            "second",
            "username",
            "selfname",
            "selfname2",
            "keroname",
            "screenwidth",
            "screenheight",
            "exh",
            "et",
            "wronghour",
            "ms",
            "mz",
            "ml",
            "mc",
            "mh",
            "mt",
            "me",
            "mp",
            "m?",
            "dms",
            "lastghostname",
            "lastobjectname",
        ];
        let mut got: Vec<&str> = FLAT_VOCAB.iter().map(|e| e.token).collect();
        got.sort_unstable();
        let mut exp: Vec<&str> = expected.to_vec();
        exp.sort_unstable();
        assert_eq!(got, exp);
    }

    /// `username` のみ ConsumerDefault・他は全て PassThroughRaw（R3.1）。
    #[test]
    fn only_username_is_consumer_default() {
        for e in FLAT_VOCAB {
            if e.token == "username" {
                assert_eq!(e.degrade, DegradePolicy::ConsumerDefault, "username");
            } else {
                assert_eq!(
                    e.degrade,
                    DegradePolicy::PassThroughRaw,
                    "{} must be PassThroughRaw",
                    e.token
                );
            }
        }
        // NotFound はフラット語彙には出現しない。
        assert!(
            FLAT_VOCAB
                .iter()
                .all(|e| e.degrade != DegradePolicy::NotFound)
        );
    }

    /// M1 実導出は username/selfname/selfname2/keroname の 4 件のみ Derived。
    #[test]
    fn only_four_tokens_are_m1_derived() {
        let derived: Vec<&str> = FLAT_VOCAB
            .iter()
            .filter(|e| e.m1 == M1Status::Derived)
            .map(|e| e.token)
            .collect();
        let mut got = derived.clone();
        got.sort_unstable();
        assert_eq!(got, vec!["keroname", "selfname", "selfname2", "username"]);
        // 代表的な縮退トークンは Degraded。
        for t in ["month", "screenwidth", "ms", "lastghostname"] {
            let e = FLAT_VOCAB.iter().find(|e| e.token == t).unwrap();
            assert_eq!(e.m1, M1Status::Degraded, "{t}");
        }
    }

    /// `%*`・`%property[...]` は構文記録のみ・`\%` はどちらの表にも出ない（R1.6）。
    #[test]
    fn syntax_records_are_star_and_property() {
        assert_eq!(SYNTAX_RECORDS, &["*", "property[...]"]);
        // `%*` は解決対象語彙に漏れない。
        assert!(FLAT_VOCAB.iter().all(|e| e.token != "*"));
        // `\%`（エスケープ）はいずれの表にも含まれない。
        assert!(
            FLAT_VOCAB
                .iter()
                .all(|e| e.token != "\\%" && e.token != "%")
        );
        assert!(!SYNTAX_RECORDS.contains(&"\\%"));
        assert!(!SYNTAX_RECORDS.contains(&"%"));
    }
}
