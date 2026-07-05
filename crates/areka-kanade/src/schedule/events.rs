//! events — ukadoc Reference 表の実装正本（イベント発火順序の単一正本）。
//!
//! 本モジュールは boot 系列・定常運転・close の各イベントについて、NOTIFY／GET の別と
//! Reference 構成を設計書「ukadoc Reference 表（運行表の単一正本）」どおりに組み立てる
//! **純粋関数群**を提供する（副作用なし・時刻や config を引数で受け取り [`ShioriCall`] を
//! 返すだけ）。mock fixture の期待応答・状態機械の期待列・観測ハーネス（4.1）の assert は
//! すべて本表から導出され、fixture・検証・実装が単一の正本を共有する（Req 7.1・DD-9）。
//!
//! # 公開面（DD-9 の例外）
//! `schedule/` は `pub(crate)` に閉じるが、本モジュールの Reference 表構成関数のみ例外的に
//! `pub` とする。`tests/` 配下の統合テストクレートは `pub(crate)` を参照できないため、
//! ハーネスが同一の値を fixture の期待値として再利用できるよう、これらの関数だけを
//! クレートの公開面（[`crate::events`] 経由）に露出する。運行状態機械の内部実装
//! （Phase／State／Action／Input／step 本体・boot/steady/close の遷移ロジック）は
//! `pub(crate)` のまま非公開である。
//!
//! # ukadoc Reference 表（本モジュールが唯一の実装点）
//! | イベント | Method | M1 送出値 |
//! |---|---|---|
//! | `OnInitialize` | NOTIFY | References なし（空 Vec・M1 にリロード概念なし） |
//! | `OnFirstBoot` | GET | Ref0=`"0"`（固定・Req 1.6） |
//! | `OnBoot` | GET | Ref0=`config.shell_name`（Ref6/7 省略） |
//! | `basewareversion` | NOTIFY | Ref0=`config.baseware_version`・Ref1=`config.baseware_name`（Ref2 省略） |
//! | `OnSecondChange` | GET（talk 再生可能時）／NOTIFY（talk 再生不能時） | Ref0=`now_ms / 3_600_000` の 10 進文字列・Ref1=`"0"`・Ref2=`"0"`・Ref3=`"1"`(GET)/`"0"`(NOTIFY) |
//! | `OnClose` | GET | Ref0=`reason.as_ref_str()`（"user"/"system"・Ref1/2 省略） |

use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriCall};

/// `OnSecondChange` Ref0 の除数（ミリ秒→時。正典: OS 連続起動時間 hour）。
const MS_PER_HOUR: u64 = 3_600_000;

/// `OnInitialize`（NOTIFY・References なし）。
///
/// M1 にリロード概念がないため Ref0（正典: リロード時 `reload`）は送出せず空 Vec とする。
pub fn on_initialize() -> ShioriCall {
    ShioriCall::Notify {
        id: "OnInitialize",
        references: Vec::new(),
    }
}

/// `OnFirstBoot`（GET・Ref0=`"0"` 固定）。
///
/// M1 は vanish count 等の永続値を持たない（永続化は position-persist の領分）ため、
/// 起動種別イベントは毎回同一の運行として Ref0 を固定値 `"0"` で構成する（Req 1.6）。
/// この応答が 204 であれば呼び手はフォールスルーして [`on_boot`] へ進む。
pub fn on_first_boot() -> ShioriCall {
    ShioriCall::Get {
        id: "OnFirstBoot",
        references: vec!["0".to_string()],
    }
}

/// `OnBoot`（GET・Ref0=`config.shell_name`）。
///
/// Ref6/7（前回 crash 情報・MATERIA/SSP）は crash 情報を持たない M1 では省略する。
pub fn on_boot(config: &KanadeConfig) -> ShioriCall {
    ShioriCall::Get {
        id: "OnBoot",
        references: vec![config.shell_name.clone()],
    }
}

/// `basewareversion`（NOTIFY・Ref0=バージョン／Ref1=本体識別）。
///
/// Ref2（詳細数値・SSP のみ）は省略する。
pub fn baseware_version(config: &KanadeConfig) -> ShioriCall {
    ShioriCall::Notify {
        id: "basewareversion",
        references: vec![
            config.baseware_version.clone(),
            config.baseware_name.clone(),
        ],
    }
}

/// `OnSecondChange`（talk 再生可能なら GET・不能なら NOTIFY）。
///
/// - Ref0: `now_ms / 3_600_000` の 10 進文字列（正典: OS 連続起動時間 hour）。
/// - Ref1: `"0"`（見切れ・emo 領分ゆえ M1 固定）。
/// - Ref2: `"0"`（重なり・同上）。
/// - Ref3: talk 再生可否——`talk_playable==true` なら `"1"`（GET）、`false` なら `"0"`（NOTIFY）。
///
/// DD-6: talk 再生中（再生不能時）は NOTIFY（Ref3=0）で発行し、返却スクリプトは構造的に
/// 破棄される（active talk 中に Value が届く経路を発生源から塞ぐ）。
pub fn on_second_change(now: MonotonicMs, talk_playable: bool) -> ShioriCall {
    let hours = (now.0 / MS_PER_HOUR).to_string();
    let ref3 = if talk_playable { "1" } else { "0" };
    let references = vec![hours, "0".to_string(), "0".to_string(), ref3.to_string()];
    if talk_playable {
        ShioriCall::Get {
            id: "OnSecondChange",
            references,
        }
    } else {
        ShioriCall::Notify {
            id: "OnSecondChange",
            references,
        }
    }
}

/// `OnClose`（GET・Ref0=`reason.as_ref_str()`）。
///
/// Ref1/2（スコープ番号・SSP）は単一スコープの M1 では省略する。
pub fn on_close(reason: CloseReason) -> ShioriCall {
    ShioriCall::Get {
        id: "OnClose",
        references: vec![reason.as_ref_str().to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// GET variant を分解して (id, references) を取り出す（Notify なら panic）。
    fn expect_get(call: ShioriCall) -> (&'static str, Vec<String>) {
        match call {
            ShioriCall::Get { id, references } => (id, references),
            ShioriCall::Notify { .. } => panic!("expected GET, got NOTIFY"),
        }
    }

    /// NOTIFY variant を分解して (id, references) を取り出す（Get なら panic）。
    fn expect_notify(call: ShioriCall) -> (&'static str, Vec<String>) {
        match call {
            ShioriCall::Notify { id, references } => (id, references),
            ShioriCall::Get { .. } => panic!("expected NOTIFY, got GET"),
        }
    }

    #[test]
    fn on_initialize_is_notify_with_empty_references() {
        let (id, references) = expect_notify(on_initialize());
        assert_eq!(id, "OnInitialize");
        assert!(references.is_empty());
    }

    #[test]
    fn on_first_boot_is_get_with_fixed_zero_ref0() {
        let (id, references) = expect_get(on_first_boot());
        assert_eq!(id, "OnFirstBoot");
        assert_eq!(references, vec!["0".to_string()]);
    }

    #[test]
    fn on_boot_is_get_with_shell_name_ref0() {
        let (id, references) = expect_get(on_boot(&config()));
        assert_eq!(id, "OnBoot");
        assert_eq!(references, vec!["master".to_string()]);
    }

    #[test]
    fn baseware_version_is_notify_with_version_and_name() {
        let (id, references) = expect_notify(baseware_version(&config()));
        assert_eq!(id, "basewareversion");
        assert_eq!(references, vec!["1.0.0".to_string(), "areka".to_string()]);
    }

    #[test]
    fn on_second_change_playable_is_get_ref3_one() {
        // 7_200_000 ms = 2 hours。
        let (id, references) = expect_get(on_second_change(MonotonicMs(7_200_000), true));
        assert_eq!(id, "OnSecondChange");
        assert_eq!(
            references,
            vec![
                "2".to_string(),
                "0".to_string(),
                "0".to_string(),
                "1".to_string(),
            ]
        );
    }

    #[test]
    fn on_second_change_not_playable_is_notify_ref3_zero() {
        // 3_600_000 ms = 1 hour。
        let (id, references) = expect_notify(on_second_change(MonotonicMs(3_600_000), false));
        assert_eq!(id, "OnSecondChange");
        assert_eq!(
            references,
            vec![
                "1".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
            ]
        );
    }

    #[test]
    fn on_second_change_ref0_truncates_toward_zero() {
        // 端数（1 時間未満）は切り捨てて "0"（3_599_999 ms < 1 hour）。
        let (_, references) = expect_get(on_second_change(MonotonicMs(3_599_999), true));
        assert_eq!(references[0], "0");
    }

    #[test]
    fn on_close_user_maps_to_user() {
        let (id, references) = expect_get(on_close(CloseReason::User));
        assert_eq!(id, "OnClose");
        assert_eq!(references, vec!["user".to_string()]);
    }

    #[test]
    fn on_close_system_maps_to_system() {
        let (id, references) = expect_get(on_close(CloseReason::System));
        assert_eq!(id, "OnClose");
        assert_eq!(references, vec!["system".to_string()]);
    }
}
