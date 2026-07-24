//! 点付き語彙台帳: ルート枝 10・汎用名 17・SET 有効群・ext 亜枝イベント名予約。
//!
//! ukadoc `list_propertysystem.html` の点付きプロパティ木を key モデル上の第一級
//! 語彙として保持する（R1.2）。M1 は `baseware.*` のみ実導出し、他ルート枝配下は
//! NOT_FOUND へ縮退する（R5.2・backing 登録だけで実導出化できる差替シーム付き）。
//!
//! SET 有効群は書込 API の型シームのみを予約し、M1 では実書込を行わない（R3.4）。
//! ext 亜枝（`activeghostlist(...).ext.*`・`pluginlist(...).ext.*`）の対応イベント名は
//! 予約のみで、M1 ではイベント発火を行わない（R3.5）。

use crate::vocab::SetSemantics;

/// 点付き語彙のルート枝 10 本（件数檻: `DOTTED_ROOTS.len() == 10`・R1.2）。
///
/// ukadoc 一次 HTML 全文検証でルート枝はちょうど 10 本（`calendarlist` 等は存在しない）。
/// M1 実導出は `baseware` のみ・他は NOT_FOUND 縮退（key モデル上は第一級で保持）。
pub const DOTTED_ROOTS: &[&str] = &[
    "system",
    "baseware",
    "ghostlist",
    "activeghostlist",
    "currentghost",
    "balloonlist",
    "headlinelist",
    "pluginlist",
    "history",
    "rateofuselist",
];

/// 汎用プロパティ名 17 種（件数檻: `GENERIC_PROP_NAMES.len() == 17`・R1.2）。
///
/// リスト系ルート枝（ghostlist / currentghost 等）配下で共通に現れる名前族。
/// `shiori.変数名`・`char*.bind.menu` はテンプレート形（変数名/キャラクタ番号を含む名の型）
/// で、語彙台帳としては型どおりに第一級保持する（brief.md Scope 節「汎用プロパティ名 17 種」
/// を逐語転記）。`[SET]` 印の 4 名（menu / sakura.bind.menu / kero.bind.menu / char*.bind.menu）
/// は SET 有効群にも登録される（`SET_EFFECTIVE` 参照）。
pub const GENERIC_PROP_NAMES: &[&str] = &[
    "name",
    "sakuraname",
    "keroname",
    "craftmanw",
    "craftmanurl",
    "path",
    "thumbnail",
    "update_result",
    "update_time",
    "homeurl",
    "username",
    "shiori.変数名",
    "index",
    "menu",
    "sakura.bind.menu",
    "kero.bind.menu",
    "char*.bind.menu",
];

/// SET 有効群（正準語彙のうち SET が有効な名前族・件数 21・R3.4）。
///
/// 書込 API の**型シームのみ**を予約し、M1 では実書込を行わない。全項目を
/// [`SetSemantics::RuntimeCommand`]（運行コマンド書込・ランタイムへの命令）へ写す——
/// 設計判断（design.md「sylphya アクター」Responsibilities）で「SET 有効群 → RuntimeCommand」
/// と確定しており、[`SetSemantics::StoreWrite`] は**正準語彙外の自由 dotted key**（asker 別
/// host 区画への反映）に予約され、SET 有効群の正準語彙には現れない。SET 無効な正準語彙への
/// 書込は `NotSettable`（受理＋警告＋非反映）として扱う（`doc/COMPAT_ARCHITECTURE.md` 対応表・
/// 正典沈黙の areka 裁量）。
///
/// 群構成（brief.md Scope 節「SET 有効群」・ukadoc `list_propertysystem.html`）:
/// - 基本 3 項: `surface.num`（SET=`\s[]` 等価）・`animation.num`（SET=`\i[]` 連続等価）・`seriko.defaultsurface`
/// - mousecursor 群 10 項: 本体側 6・balloon 側 4
/// - seriko.cursor / tooltip 群 4 項: cursor の path/name・tooltip の text/name
/// - menu / bind.menu 群 4 項: `GENERIC_PROP_NAMES` の `[SET]` 4 名と同一
pub const SET_EFFECTIVE: &[(&str, SetSemantics)] = &[
    // 基本 3 項（surface/animation/defaultsurface）。
    ("surface.num", SetSemantics::RuntimeCommand),
    ("animation.num", SetSemantics::RuntimeCommand),
    ("seriko.defaultsurface", SetSemantics::RuntimeCommand),
    // mousecursor 群 10 項（本体側 6）。
    ("mousecursor", SetSemantics::RuntimeCommand),
    ("mousecursor.text", SetSemantics::RuntimeCommand),
    ("mousecursor.wait", SetSemantics::RuntimeCommand),
    ("mousecursor.hand", SetSemantics::RuntimeCommand),
    ("mousecursor.grip", SetSemantics::RuntimeCommand),
    ("mousecursor.arrow", SetSemantics::RuntimeCommand),
    // mousecursor 群（balloon 側 4）。
    ("balloon.mousecursor", SetSemantics::RuntimeCommand),
    ("balloon.mousecursor.text", SetSemantics::RuntimeCommand),
    ("balloon.mousecursor.wait", SetSemantics::RuntimeCommand),
    ("balloon.mousecursor.arrow", SetSemantics::RuntimeCommand),
    // seriko.cursor / tooltip 群 4 項。
    ("seriko.cursor.path", SetSemantics::RuntimeCommand),
    ("seriko.cursor.name", SetSemantics::RuntimeCommand),
    ("seriko.tooltip.text", SetSemantics::RuntimeCommand),
    ("seriko.tooltip.name", SetSemantics::RuntimeCommand),
    // menu / bind.menu 群 4 項（GENERIC_PROP_NAMES の [SET] 4 名）。
    ("menu", SetSemantics::RuntimeCommand),
    ("sakura.bind.menu", SetSemantics::RuntimeCommand),
    ("kero.bind.menu", SetSemantics::RuntimeCommand),
    ("char*.bind.menu", SetSemantics::RuntimeCommand),
];

/// ext 亜枝の GET イベント名（予約のみ・M1 では発火しない・R3.5）。
///
/// `activeghostlist(...).ext.*`・`pluginlist(...).ext.*` は所有者（ゴースト/プラグイン）側へ
/// この名の SHIORI/PLUGIN イベントを発生させて値を取得する語彙。M1 は名前の予約に留め、
/// イベント発火（ext 亜枝の実働）は M2 の領分。
pub const EXT_EVENT_GET: &str = "property.get";

/// ext 亜枝の SET イベント名（予約のみ・M1 では発火しない・R3.5）。
pub const EXT_EVENT_SET: &str = "property.set";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// ルート枝 件数檻（R1.2・観測可能な完了条件）。
    #[test]
    fn dotted_roots_has_10_entries() {
        assert_eq!(DOTTED_ROOTS.len(), 10);
    }

    /// ルート枝 10 本の集合が過不足なく一致（タイポ・重複・欠落を檻化）。
    #[test]
    fn dotted_roots_match_exactly() {
        let expected: [&str; 10] = [
            "system",
            "baseware",
            "ghostlist",
            "activeghostlist",
            "currentghost",
            "balloonlist",
            "headlinelist",
            "pluginlist",
            "history",
            "rateofuselist",
        ];
        let got: BTreeSet<&str> = DOTTED_ROOTS.iter().copied().collect();
        let exp: BTreeSet<&str> = expected.iter().copied().collect();
        assert_eq!(got, exp);
        // 重複なし（集合サイズ＝配列長）。
        assert_eq!(DOTTED_ROOTS.len(), got.len(), "DOTTED_ROOTS に重複あり");
    }

    /// 汎用名 件数檻（R1.2）。
    #[test]
    fn generic_prop_names_has_17_entries() {
        assert_eq!(GENERIC_PROP_NAMES.len(), 17);
    }

    /// 汎用名に重複がない（件数＝集合サイズ）。
    #[test]
    fn generic_prop_names_no_duplicates() {
        let set: BTreeSet<&str> = GENERIC_PROP_NAMES.iter().copied().collect();
        assert_eq!(GENERIC_PROP_NAMES.len(), set.len(), "GENERIC_PROP_NAMES に重複あり");
    }

    /// 汎用名 17 種が過不足なく一致（brief.md Scope 節を逐語転記した集合）。
    #[test]
    fn generic_prop_names_match_exactly() {
        let expected: [&str; 17] = [
            "name",
            "sakuraname",
            "keroname",
            "craftmanw",
            "craftmanurl",
            "path",
            "thumbnail",
            "update_result",
            "update_time",
            "homeurl",
            "username",
            "shiori.変数名",
            "index",
            "menu",
            "sakura.bind.menu",
            "kero.bind.menu",
            "char*.bind.menu",
        ];
        let got: BTreeSet<&str> = GENERIC_PROP_NAMES.iter().copied().collect();
        let exp: BTreeSet<&str> = expected.iter().copied().collect();
        assert_eq!(got, exp);
    }

    /// SET 有効群 件数檻（基本 3＋mousecursor 10＋seriko.cursor/tooltip 4＋menu 4 = 21・R3.4）。
    #[test]
    fn set_effective_has_21_entries() {
        assert_eq!(SET_EFFECTIVE.len(), 21);
    }

    /// SET 有効群に key 重複がない。
    #[test]
    fn set_effective_no_duplicate_keys() {
        let set: BTreeSet<&str> = SET_EFFECTIVE.iter().map(|(k, _)| *k).collect();
        assert_eq!(SET_EFFECTIVE.len(), set.len(), "SET_EFFECTIVE に key 重複あり");
    }

    /// SET 有効群「全項目の網羅」檻: task 2.3 が名指しする全群メンバーが登録済み（R3.4）。
    #[test]
    fn set_effective_covers_all_named_group_members() {
        let keys: BTreeSet<&str> = SET_EFFECTIVE.iter().map(|(k, _)| *k).collect();
        let required: [&str; 21] = [
            // 基本 3 項。
            "surface.num",
            "animation.num",
            "seriko.defaultsurface",
            // mousecursor 群 10 項。
            "mousecursor",
            "mousecursor.text",
            "mousecursor.wait",
            "mousecursor.hand",
            "mousecursor.grip",
            "mousecursor.arrow",
            "balloon.mousecursor",
            "balloon.mousecursor.text",
            "balloon.mousecursor.wait",
            "balloon.mousecursor.arrow",
            // seriko.cursor / tooltip 群 4 項。
            "seriko.cursor.path",
            "seriko.cursor.name",
            "seriko.tooltip.text",
            "seriko.tooltip.name",
            // menu / bind.menu 群 4 項。
            "menu",
            "sakura.bind.menu",
            "kero.bind.menu",
            "char*.bind.menu",
        ];
        for member in required {
            assert!(keys.contains(member), "SET 有効群に {member} が欠落");
        }
        // 網羅＝過不足なし（余剰も許さない）。
        let exp: BTreeSet<&str> = required.iter().copied().collect();
        assert_eq!(keys, exp);
    }

    /// SET 有効群の意味論は全て RuntimeCommand（design.md 確定・StoreWrite は正準語彙外の自由 key 用）。
    #[test]
    fn set_effective_all_runtime_command() {
        for (key, sem) in SET_EFFECTIVE {
            assert_eq!(
                *sem,
                SetSemantics::RuntimeCommand,
                "{key} は RuntimeCommand であるべき"
            );
        }
        // 代表的な運行コマンド書込の意味論写像を確認（surface.num SET＝\s[] 等価）。
        let surface = SET_EFFECTIVE.iter().find(|(k, _)| *k == "surface.num").unwrap();
        assert_eq!(surface.1, SetSemantics::RuntimeCommand);
    }

    /// menu / bind.menu 群は GENERIC_PROP_NAMES の [SET] 4 名と一致（二重登録の整合）。
    #[test]
    fn menu_group_consistent_with_generic_names() {
        let set_keys: BTreeSet<&str> = SET_EFFECTIVE.iter().map(|(k, _)| *k).collect();
        for menu_name in ["menu", "sakura.bind.menu", "kero.bind.menu", "char*.bind.menu"] {
            assert!(
                GENERIC_PROP_NAMES.contains(&menu_name),
                "{menu_name} が GENERIC_PROP_NAMES に不在"
            );
            assert!(set_keys.contains(menu_name), "{menu_name} が SET_EFFECTIVE に不在");
        }
    }

    /// ext 亜枝イベント名は予約定数として厳密一致（R3.5・発火しない）。
    #[test]
    fn ext_event_names_reserved_exact() {
        assert_eq!(EXT_EVENT_GET, "property.get");
        assert_eq!(EXT_EVENT_SET, "property.set");
    }
}
