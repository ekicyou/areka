//! SHIORI Resource 159 項目台帳（「-」は ID 未確認注記付き）。
//!
//! ukadoc `list_shiori_resource.html` の SHIORI Resource ID 全数を key モデル上の
//! 第一級語彙として保持する。shiori-queried backing の名前族で、実導出は
//! `username` 1 件のみ（`%username` の供給源）・他は縮退する。
//!
//! 正典 HTML の `<dt>` は全 178 個で、うち 19 個は下位パラメータを記述する非リソース dt
//! （`Reference0..5` / `Reference*` と `getaistate` の `グラフ*群` 4 種）。
//! これらを除いたリソース ID の dt が正確に 159 個で、台帳の正本はこの実数。
//! カテゴリ内訳（document 順）:
//!
//! - **SHIORI 情報 5**: `version` / `craftman` / `craftmanw` / `name` / `log_path`
//! - **ゴースト情報 41**: `homeurl`〜`other_homeurl_override`（language 名の特殊 dt「-」は別枠）
//! - **オーナードローメニュー画像 3＋文字色群 15 = 18**: `menu.*.bitmap.filename` 3 と
//!   `menu.*.color.{r,g,b}` 15
//! - **特殊エントリ「-」1**: language 名（`[変更不可]`）
//! - **`*button.caption` ファミリ 92**: `activaterootbutton.caption`〜`texttospeechbutton.caption`
//!   （`char*.recommendsites.caption` を含む）
//! - **tooltip 2**: `tooltip` / `balloon_tooltip`
//!
//! `*button.visible` ファミリは正典に無く、visible 系の dt は `vanishbuttonvisible` と
//! `sakura|kero|char*.popupmenu.visible` のみ（ゴースト情報側に計上）。
//!
//! ## 特殊エントリ「-」
//!
//! 正典 HTML には ID 欄が「-」（ハイフンのみ）の dt が 1 個あり、language 名を表す
//! `[変更不可]` のリソースとして記述されるが、実リソース ID 文字列は未確認。
//! 語彙を落とさないため値 `"-"` のまま台帳に保持する。

/// SHIORI Resource ID 全 159 項目。
///
/// ukadoc `list_shiori_resource.html` のリソース dt を document 順に忠実転記した正本。
/// 値は先頭 `%` を持たない SHIORI リソース ID（照会時の `ID:` ヘッダ値）。テンプレート形
/// （`char*` ワイルドカードや `(入力ボックス種類)` プレースホルダ）は正典の表記のまま
/// 1 エントリとして保持する（1 dt = 1 エントリ）。
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html
pub const SHIORI_RESOURCE_IDS: &[&str] = &[
    // --- SHIORI 情報 5 ---
    "version",
    "craftman",
    "craftmanw",
    "name",
    "log_path",
    // --- ゴースト情報 41（homeurl〜other_homeurl_override）---
    "homeurl",
    "useorigin1",
    "username",
    "sakura.defaultx",
    "kero.defaultx",
    "char*.defaultx",
    "sakura.defaulty",
    "kero.defaulty",
    "char*.defaulty",
    "sakura.defaultleft",
    "kero.defaultleft",
    "char*.defaultleft",
    "sakura.defaulttop",
    "kero.defaulttop",
    "char*.defaulttop",
    // 入力ボックス種類の既定位置は正典上 1 dt に 2 語（left/top）を併記＝1 エントリで保持。
    "(入力ボックス種類).defaultleft (入力ボックス種類).defaulttop",
    "sakura.recommendsites",
    "sakura.portalsites",
    "kero.recommendsites",
    "char*.recommendsites",
    "sakura.recommendbuttoncaption",
    "sakura.portalbuttoncaption",
    "kero.recommendbuttoncaption",
    "char*.recommendbuttoncaption",
    "updatebuttoncaption",
    "vanishbuttoncaption",
    "readmebuttoncaption",
    "vanishbuttonvisible",
    "sakura.popupmenu.visible",
    "kero.popupmenu.visible",
    "char*.popupmenu.visible",
    "sakura.popupmenu.type",
    "kero.popupmenu.type",
    "char*.popupmenu.type",
    "sakura.popupmenu.applybindtoself",
    "kero.popupmenu.applybindtoself",
    "char*.popupmenu.applybindtoself",
    "getaistate",
    "getaistateex",
    "legacyinterface",
    "other_homeurl_override",
    // --- オーナードローメニュー画像 3＋文字色群 15 = 18 ---
    "menu.sidebar.bitmap.filename",
    "menu.background.bitmap.filename",
    "menu.foreground.bitmap.filename",
    "menu.background.font.color.r",
    "menu.background.font.color.g",
    "menu.background.font.color.b",
    "menu.foreground.font.color.r",
    "menu.foreground.font.color.g",
    "menu.foreground.font.color.b",
    "menu.separator.color.r",
    "menu.separator.color.g",
    "menu.separator.color.b",
    "menu.frame.color.r",
    "menu.frame.color.g",
    "menu.frame.color.b",
    "menu.disable.font.color.r",
    "menu.disable.font.color.g",
    "menu.disable.font.color.b",
    // --- 特殊エントリ「-」（language 名・[変更不可]）: 実リソース ID 未確認 ---
    "-",
    // --- `*button.caption` ファミリ 92（activaterootbutton〜texttospeechbutton）---
    "activaterootbutton.caption",
    "addressbarbutton.caption",
    "alignrootbutton.caption",
    "alwaysstayontopbutton.caption",
    "alwaystrayiconvisiblebutton.caption",
    "balloonhistorybutton.caption",
    "balloonrootbutton.caption",
    "biffallbutton.caption",
    "biffbutton.caption",
    "calendarbutton.caption",
    "callghosthistorybutton.caption",
    "callghostrootbutton.caption",
    "callsstpsendboxbutton.caption",
    "char*.recommendsites.caption",
    "charsetbutton.caption",
    "closeballoonbutton.caption",
    "closebutton.caption",
    "collisionvisiblebutton.caption",
    "configurationbutton.caption",
    "configurationrootbutton.caption",
    "debugballoonbutton.caption",
    "definedsurfaceonlybutton.caption",
    "dressuprootbutton.caption",
    "duibutton.caption",
    "enableballoonmovebutton.caption",
    "firststaffbutton.caption",
    "ghostexplorerbutton.caption",
    "ghosthistorybutton.caption",
    "ghostinstallbutton.caption",
    "ghostrootbutton.caption",
    "headlinesensehistorybutton.caption",
    "headlinesenserootbutton.caption",
    "helpbutton.caption",
    "hidebutton.caption",
    "historyrootbutton.caption",
    "inforootbutton.caption",
    "leavepassivebutton.caption",
    "messengerbutton.caption",
    "pluginhistorybutton.caption",
    "pluginrootbutton.caption",
    "portalrootbutton.caption",
    "purgeghostcachebutton.caption",
    "quitbutton.caption",
    "rateofuseballoonbutton.caption",
    "rateofusebutton.caption",
    "rateofuserootbutton.caption",
    "rateofusetotalbutton.caption",
    "readmebutton.caption",
    "termsbutton.caption",
    "recommendrootbutton.caption",
    "regionenabledbutton.caption",
    "reloadinfobutton.caption",
    "resetballoonpositionbutton.caption",
    "resettodefaultbutton.caption",
    "scriptlogbutton.caption",
    "shellrootbutton.caption",
    "shellscaleotherbutton.caption",
    "shellscalerootbutton.caption",
    "sntpbutton.caption",
    "switchactivatewhentalkbutton.caption",
    "switchactivatewhentalkexceptupdatebutton.caption",
    "switchautobiffbutton.caption",
    "switchautoheadlinesensebutton.caption",
    "switchblacklistingbutton.caption",
    "switchcompatiblemodebutton.caption",
    "switchconsolealwaysvisiblebutton.caption",
    "switchconsolevisiblebutton.caption",
    "switchdeactivatebutton.caption",
    "switchdontactivatebutton.caption",
    "switchdontforcealignbutton.caption",
    "switchduivisiblebutton.caption",
    "switchforcealignfreebutton.caption",
    "switchforcealignlimitbutton.caption",
    "switchignoreserikomovebutton.caption",
    "switchlocalsstpbutton.caption",
    "switchmovetodefaultpositionbutton.caption",
    "switchproxybutton.caption",
    "switchquietbutton.caption",
    "switchreloadbutton.caption",
    "switchreloadtempghostbutton.caption",
    "switchremotesstpbutton.caption",
    "switchrootbutton.caption",
    "switchtalkghostbutton.caption",
    "systeminfobutton.caption",
    "updatebutton.caption",
    "updatefmobutton.caption",
    "updateplatformbutton.caption",
    "utilityrootbutton.caption",
    "vanishbutton.caption",
    "aistatebutton.caption",
    "dictationbutton.caption",
    "texttospeechbutton.caption",
    // --- tooltip 2 ---
    "tooltip",
    "balloon_tooltip",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// 正典 dt の実数どおり 159 項目。
    #[test]
    fn shiori_resource_ids_has_159_entries() {
        assert_eq!(SHIORI_RESOURCE_IDS.len(), 159);
    }

    /// 重複なし（件数＝集合サイズ）。
    #[test]
    fn shiori_resource_ids_no_duplicates() {
        let set: BTreeSet<&str> = SHIORI_RESOURCE_IDS.iter().copied().collect();
        assert_eq!(
            SHIORI_RESOURCE_IDS.len(),
            set.len(),
            "SHIORI_RESOURCE_IDS に重複あり"
        );
    }

    /// 特殊エントリ「-」（language 名・ID 未確認）が語彙として保持されている。
    #[test]
    fn special_dash_entry_is_retained() {
        assert!(
            SHIORI_RESOURCE_IDS.contains(&"-"),
            "特殊エントリ「-」が台帳から欠落（語彙を落とさない）"
        );
    }

    /// 各カテゴリの代表 ID が存在する（転記漏れ・タイポの検出）。
    #[test]
    fn representative_ids_present_across_categories() {
        assert!(SHIORI_RESOURCE_IDS.contains(&"version"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"log_path"));
        // `username` は `%username` の供給源。
        assert!(SHIORI_RESOURCE_IDS.contains(&"username"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"homeurl"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"sakura.defaultx"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"char*.defaulttop"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"getaistateex"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"legacyinterface"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"menu.sidebar.bitmap.filename"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"menu.disable.font.color.b"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"updatebutton.caption"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"vanishbutton.caption"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"texttospeechbutton.caption"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"vanishbuttonvisible"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"sakura.popupmenu.visible"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"tooltip"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"balloon_tooltip"));
    }

    /// SHIORI 情報 5 件が document 先頭 5 エントリに正典順で並ぶ。
    #[test]
    fn shiori_info_five_are_leading_in_canonical_order() {
        assert_eq!(
            &SHIORI_RESOURCE_IDS[..5],
            &["version", "craftman", "craftmanw", "name", "log_path"]
        );
    }

    /// tooltip 2 件が document 末尾に並ぶ。
    #[test]
    fn tooltip_pair_is_trailing() {
        assert_eq!(
            &SHIORI_RESOURCE_IDS[SHIORI_RESOURCE_IDS.len() - 2..],
            &["tooltip", "balloon_tooltip"]
        );
    }
}
