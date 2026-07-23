//! SHIORI Resource 159 項目台帳（「-」は ID 未確認注記付き）。
//!
//! ukadoc `list_shiori_resource.html` の SHIORI Resource ID 全数を key モデル上の
//! 第一級語彙として保持する（R1.4）。shiori-queried backing の名前族であり、M1 実導出は
//! `username` 1 件のみ（`%username` の供給源）・他は縮退（照会経路自体は build_request が
//! 任意 ID を通すため、実導出追加は backing 登録だけで済む）。
//!
//! ## 典拠と件数の導出（正典＝ukadoc）
//!
//! 正典 HTML の `<dt>` 要素は全 178 個だが、うち 19 個は各リソースの下位パラメータを
//! 記述する非リソース dt（`Reference0..5` / `Reference*` と `getaistate` の
//! `グラフ*群` 4 種）である。これらを除いた **リソース ID の dt は正確に 159 個**
//! （178 − 19 = 159）で、requirements 1.4・design の `SHIORI_RESOURCE_IDS` 159 件と一致する。
//! カテゴリ内訳（document 順・件数は実 dt を数えた実測値）:
//!
//! - **SHIORI 情報 5**: `version` / `craftman` / `craftmanw` / `name` / `log_path`
//! - **ゴースト情報 41**: `homeurl`〜`other_homeurl_override`（既定位置・recommendsites・
//!   popupmenu・getaistate(ex)・legacyinterface 等。requirements 1.4 の「43」は概数で、
//!   実 dt の実測は 41。language 名の特殊 dt「-」は下記のとおり別枠）
//! - **オーナードローメニュー画像 3＋文字色群 15 = 18**: `menu.*.bitmap.filename` 3 と
//!   `menu.*.color.{r,g,b}` 15
//! - **特殊エントリ「-」1**: language 名（`[変更不可]`）＝実リソース ID 未確認（下記注記）
//! - **`*button.caption` ファミリ 92**: `activaterootbutton.caption`〜`texttospeechbutton.caption`
//!   （`char*.recommendsites.caption` を含む。requirements 1.4 の「91」は概数で、実 dt の実測は 92）
//! - **tooltip 2**: `tooltip` / `balloon_tooltip`
//!
//! `*button.visible` ファミリは正典上「91 種と同数」は存在せず、visible 系の実 dt は
//! `vanishbuttonvisible` と `sakura|kero|char*.popupmenu.visible` のみ（ゴースト情報側に計上）。
//! requirements 1.4 の記述は概括であり、**正典 dt の実数 159** が語彙台帳の正本である
//! （タスク指示「task-text の内訳はヒント・正典の列挙が権威」に従う）。
//!
//! ## 特殊エントリ「-」
//!
//! 正典 HTML には ID 欄が「-」（ハイフンのみ）の dt が 1 個あり、language 名を表す
//! `[変更不可]` のリソースとして記述されるが、**実リソース ID 文字列は未確認**。
//! 語彙を落とさないため（R1.4「特殊エントリ『-』は保持」）、値 `"-"` のまま台帳に保持し、
//! 本コメントで「ID 未確認」を注記する。

/// SHIORI Resource ID 全 159 項目（件数檻: `SHIORI_RESOURCE_IDS.len() == 159`・R1.4）。
///
/// ukadoc `list_shiori_resource.html` のリソース dt を document 順に忠実転記した正本。
/// 値は先頭 `%` を持たない SHIORI リソース ID（照会時の `ID:` ヘッダ値）。テンプレート形
/// （`char*` ワイルドカードや `(入力ボックス種類)` プレースホルダ）は正典の表記のまま
/// 1 エントリとして保持する（1 dt = 1 エントリ）。
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
    // --- 特殊エントリ「-」（language 名・[変更不可]）: 実リソース ID 未確認・語彙を落とさず保持 ---
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

    /// 件数檻（R1.4・観測可能な完了条件）: SHIORI Resource は正典 dt 実測で正確に 159 項目。
    #[test]
    fn shiori_resource_ids_has_159_entries() {
        assert_eq!(SHIORI_RESOURCE_IDS.len(), 159);
    }

    /// 重複なし（件数＝集合サイズ）。誤コピペによる件数偽装を檻化。
    #[test]
    fn shiori_resource_ids_no_duplicates() {
        let set: BTreeSet<&str> = SHIORI_RESOURCE_IDS.iter().copied().collect();
        assert_eq!(
            SHIORI_RESOURCE_IDS.len(),
            set.len(),
            "SHIORI_RESOURCE_IDS に重複あり"
        );
    }

    /// 特殊エントリ「-」（language 名・ID 未確認）が語彙として保持されている（R1.4）。
    #[test]
    fn special_dash_entry_is_retained() {
        assert!(
            SHIORI_RESOURCE_IDS.contains(&"-"),
            "特殊エントリ「-」が台帳から欠落（語彙を落とさない）"
        );
    }

    /// 各カテゴリの代表 ID がスポット検証で存在する（転記漏れ・タイポの早期検出）。
    #[test]
    fn representative_ids_present_across_categories() {
        // SHIORI 情報。
        assert!(SHIORI_RESOURCE_IDS.contains(&"version"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"log_path"));
        // ゴースト情報（%username の供給源＝M1 唯一の実導出）。
        assert!(SHIORI_RESOURCE_IDS.contains(&"username"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"homeurl"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"sakura.defaultx"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"char*.defaulttop"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"getaistateex"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"legacyinterface"));
        // オーナードローメニュー画像／文字色群。
        assert!(SHIORI_RESOURCE_IDS.contains(&"menu.sidebar.bitmap.filename"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"menu.disable.font.color.b"));
        // `*button.caption` ファミリ。
        assert!(SHIORI_RESOURCE_IDS.contains(&"updatebutton.caption"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"vanishbutton.caption"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"texttospeechbutton.caption"));
        // visible 系（正典に実在する visible dt）。
        assert!(SHIORI_RESOURCE_IDS.contains(&"vanishbuttonvisible"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"sakura.popupmenu.visible"));
        // tooltip 2。
        assert!(SHIORI_RESOURCE_IDS.contains(&"tooltip"));
        assert!(SHIORI_RESOURCE_IDS.contains(&"balloon_tooltip"));
    }

    /// SHIORI 情報 5 件が document 先頭 5 エントリに正典順で並ぶ（順序も転記の一部）。
    #[test]
    fn shiori_info_five_are_leading_in_canonical_order() {
        assert_eq!(
            &SHIORI_RESOURCE_IDS[..5],
            &["version", "craftman", "craftmanw", "name", "log_path"]
        );
    }

    /// tooltip 2 件が document 末尾に並ぶ（正典順の末尾固定）。
    #[test]
    fn tooltip_pair_is_trailing() {
        assert_eq!(
            &SHIORI_RESOURCE_IDS[SHIORI_RESOURCE_IDS.len() - 2..],
            &["tooltip", "balloon_tooltip"]
        );
    }
}
