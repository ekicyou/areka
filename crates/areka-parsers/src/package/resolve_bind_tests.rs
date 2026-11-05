//! resolve_bind_tests — `resolve_bind::read_bindgroup_defaults` の兄弟テスト。
//!
//! もと `resolve.rs`（今は無い）の中の試験 2 つ（`.name` の転記・bindoption の分解）を中身を変えずに移した。

use super::model::{BindGroupDefaults, ScopeBinds};
use super::resolve_bind::read_bindgroup_defaults;

/// 書かれていないスコープへ返す空の欄。
static NO_BINDS: ScopeBinds = ScopeBinds {
    default_on: Vec::new(),
    names: Vec::new(),
    mustselect: Vec::new(),
    multiple: Vec::new(),
};

/// 表からスコープ番号の欄を引く（書かれていなければ空の欄）。
fn scope(d: &BindGroupDefaults, n: u32) -> &ScopeBinds {
    d.get(&n).unwrap_or(&NO_BINDS)
}

/// スコープ `n` で (カテゴリ, パーツ) を宣言した bindgroup 番号を転記順に（後勝ちの引きは seriko）。
fn ids_of(d: &BindGroupDefaults, n: u32, category: &str, part: &str) -> Vec<u32> {
    scope(d, n)
        .names
        .iter()
        .filter(|b| b.category == category && b.part == part)
        .map(|b| b.id)
        .collect()
}

#[cfg(test)]
mod bindgroup_name_transcription_tests {
    //! `read_bindgroup_defaults` の `.name` 転記（task 1.2）の in-source 檻。
    //!
    //! shell descript.txt を合成 tempdir に書き出し、`read_bindgroup_defaults` を
    //! 直接呼んで `.name` サフィックスの忠実転記（2/3 フィールド・trim・sakura/kero
    //! 区別・重複後勝ち・パーツ欠落 skip）と、既存 `.default` 経路の無改変（R1.6）を
    //! 決定論で固定する。一時パスは共通窓口 `temp-path-kit` 経由で組む（プロセス間でも一意）。

    use std::fs;

    use temp_path_kit::TempPath;

    use super::{ids_of, scope};
    use crate::charset::DefaultEncoding;

    use super::read_bindgroup_defaults;

    /// テスト専用の一意な shell ディレクトリを作り、descript.txt を書き込んで返す。
    ///
    /// 一時ディレクトリは共通窓口 `temp-path-kit` 経由で組むので、名前にプロセス識別子と
    /// 連番が入り**プロセス間でも一意**になる。返り値が生きている間だけ実体が存在し、
    /// 破棄で中身ごと消える（後始末を呼び忘れる余地が無い）。
    fn shell_with_descript(tag: &str, descript_body: &str) -> TempPath {
        // 札は `-` と英数字だけ（窓口の約束）。呼出側の tag は関数名由来で `_` を含む。
        let dir = TempPath::new(&format!("parsers-bindgroup-name-{}", tag.replace('_', "-")));
        fs::write(dir.path().join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        dir
    }

    /// 3 フィールド形（カテゴリ, パーツ, サムネ）を忠実転記し、名前解決できる。
    #[test]
    fn name_three_fields_transcribed_and_resolvable() {
        let temp = shell_with_descript(
            "three_fields",
            "charset,UTF-8\nsakura.bindgroup1100.name,腕,伸び,thumb\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(scope(&defaults, 0).names.len(), 1);
        let n = &scope(&defaults, 0).names[0];
        assert_eq!(n.id, 1100);
        assert_eq!(n.category, "腕");
        assert_eq!(n.part, "伸び");
        assert_eq!(n.thumbnail, Some("thumb".to_string()));
        assert_eq!(ids_of(&defaults, 0, "腕", "伸び"), vec![1100]);
    }

    /// 2 フィールド形（サムネ欠落）は `thumbnail == None` で転記する。
    #[test]
    fn name_two_fields_thumbnail_none() {
        let temp = shell_with_descript(
            "two_fields",
            "charset,UTF-8\nsakura.bindgroup1200.name,口,笑い\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(scope(&defaults, 0).names.len(), 1);
        assert_eq!(scope(&defaults, 0).names[0].thumbnail, None);
        assert_eq!(ids_of(&defaults, 0, "口", "笑い"), vec![1200]);
    }

    /// kero 側は本体側と区別して表の 1（kero）の `names` へ入る（R1.2）。
    #[test]
    fn kero_name_distinguished_from_sakura() {
        let temp = shell_with_descript(
            "kero_scope",
            "charset,UTF-8\nkero.bindgroup2100.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(
            scope(&defaults, 0).names.is_empty(),
            "sakura 側は空であるべき"
        );
        assert_eq!(scope(&defaults, 1).names.len(), 1);
        assert_eq!(ids_of(&defaults, 1, "腕", "伸び"), vec![2100]);
        // sakura スコープでは解決できない（別集合）。
        assert_eq!(ids_of(&defaults, 0, "腕", "伸び"), Vec::<u32>::new());
    }

    /// パーツ欠落（第 2 フィールド無し）の宣言は転記対象外（捏造しない・R1.5）。
    #[test]
    fn name_missing_part_not_transcribed() {
        let temp = shell_with_descript(
            "missing_part",
            "charset,UTF-8\nsakura.bindgroup1300.name,カテゴリのみ\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(
            scope(&defaults, 0).names.is_empty(),
            "パーツ欠落の宣言は転記されない: {:?}",
            scope(&defaults, 0).names
        );
    }

    /// 空パーツ（`カテゴリ,`）も転記対象外。
    #[test]
    fn name_empty_part_not_transcribed() {
        let temp = shell_with_descript(
            "empty_part",
            "charset,UTF-8\nsakura.bindgroup1301.name,カテゴリ,\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(scope(&defaults, 0).names.is_empty());
    }

    /// 重複 (カテゴリ, パーツ) はキー昇順走査の後勝ち（D2）＝最後の ID を採る。
    #[test]
    fn duplicate_category_part_last_wins() {
        // 1100 と 1400 が同じ (腕, 伸び) を宣言。キー昇順で 1100→1400 の順に転記され、
        // seriko の引き（`BindResolver::from_groups`）は後勝ちで 1400 を採る。
        let temp = shell_with_descript(
            "duplicate",
            "charset,UTF-8\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             sakura.bindgroup1400.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(ids_of(&defaults, 0, "腕", "伸び"), vec![1100, 1400]);
    }

    /// R1.6: `.default` 経路は `.name` 増設後も無改変（value=="1" のみ・番号集合）。
    #[test]
    fn default_path_unchanged_alongside_names() {
        let temp = shell_with_descript(
            "default_coexist",
            "charset,UTF-8\n\
             sakura.bindgroup1100.default,1\n\
             sakura.bindgroup1101.default,0\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             kero.bindgroup2100.default,1\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(scope(&defaults, 0).default_on, vec![1100]);
        assert_eq!(scope(&defaults, 1).default_on, vec![2100]);
        // 名前も併せて転記されている（両経路が同一走査で共存）。
        assert_eq!(ids_of(&defaults, 0, "腕", "伸び"), vec![1100]);
    }

    /// R1.6（純粋非退行）: `.default` on/off **のみ**（`.name` 皆無）の shell descript は、
    /// 従来どおり default 集合を転記しつつ名前表を一切生成しない（名前解決の追加が
    /// 既定 on/off マウント結果を変えない・捏造しない）。`default_path_unchanged_alongside_names`
    /// が `.name` 併存を扱うのに対し、本テストは名前宣言が皆無の既定 on/off 専用ケースを固定する。
    #[test]
    fn default_only_no_names_leaves_names_empty() {
        let temp = shell_with_descript(
            "default_only_no_names",
            "charset,UTF-8\n\
             sakura.bindgroup1100.default,1\n\
             sakura.bindgroup1101.default,0\n\
             kero.bindgroup2100.default,1\n\
             kero.bindgroup2101.default,0\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // 既定 on/off は従来どおり（value=="1" のみ収集・off は非収集）。
        assert_eq!(scope(&defaults, 0).default_on, vec![1100]);
        assert_eq!(scope(&defaults, 1).default_on, vec![2100]);

        // 名前宣言が皆無 → 名前表は空（default,1 が名前を捏造しない・R1.5/R1.6）。
        assert!(
            scope(&defaults, 0).names.is_empty(),
            "名前宣言皆無なら sakura の names は空"
        );
        assert!(
            scope(&defaults, 1).names.is_empty(),
            "名前宣言皆無なら kero の names は空"
        );

        // 名前解決は全スコープで None（着せ替え ID を捏造しない）。
        assert_eq!(ids_of(&defaults, 0, "腕", "伸び"), Vec::<u32>::new());
        assert_eq!(ids_of(&defaults, 1, "腕", "伸び"), Vec::<u32>::new());
    }

    /// Observable（emo2 実 fixture）: 宣言済みの全 (カテゴリ, パーツ) が ID へ解決する。
    #[test]
    fn emo2_declared_names_all_resolve() {
        let shell_dir = crate::sample_test_support::emo2_root()
            .join("shell")
            .join("master");
        let defaults = read_bindgroup_defaults(&shell_dir, DefaultEncoding::Utf8);

        // 非空虚（emo2 は 30 本超の sakura .name を持つ）。
        assert!(
            scope(&defaults, 0).names.len() >= 30,
            "emo2 の sakura .name 宣言が十分に転記されている（実測 30 本超）: {}",
            scope(&defaults, 0).names.len()
        );
        // 代表的な宣言が (カテゴリ, パーツ) → ID へ引ける。
        assert_eq!(ids_of(&defaults, 0, "腕", "伸び"), vec![1100]);
        assert_eq!(ids_of(&defaults, 0, "口", "にこっ"), vec![1207]);
        assert_eq!(ids_of(&defaults, 0, "目", "通常"), vec![1302]);
        assert_eq!(ids_of(&defaults, 0, "髪飾り", "ボンボン"), vec![1801]);
        // 宣言済みの全組が漏れなく引ける（サムネ無し 2 フィールド形でも解決）。
        for n in &scope(&defaults, 0).names {
            assert!(
                ids_of(&defaults, 0, &n.category, &n.part).contains(&n.id),
                "宣言済み ({}, {}) が ID {} へ解決しない",
                n.category,
                n.part,
                n.id
            );
        }
    }
}

#[cfg(test)]
mod bindoption_options_tests {
    //! `read_bindgroup_defaults` の `bindoption*.group` 取り込みの in-source 檻（bindopt 4.2）。
    //!
    //! shell descript.txt を合成 tempdir に書き出し、`read_bindgroup_defaults` を直接呼んで
    //! `sakura/kero.bindoption*.group,カテゴリ,オプション[+オプション...]` の取り込みを
    //! 決定論で固定する。網羅マトリクス: ukadoc 正典 3 値（`mustselect`／非宣言＝既定／
    //! `multiple`）・`+` 区切り併記・未知語・不完全値・宣言ゼロ・sakura/kero 隔離・再読の
    //! 同一結果。一時パスは共通窓口 `temp-path-kit` 経由で組む（プロセス間でも一意）。
    //!
    //! 語彙: 「排他置換（exclusive）」「既定（Default＝高々 1 個・解除可）」「複数可
    //! （Multiple）」。ここで検証するのは**宣言の所属の転記**だけで、「排他か」の解釈は
    //! 下流 seriko の責務（転写層原則）。

    use std::fs;

    use temp_path_kit::TempPath;

    use super::{ids_of, scope};
    use crate::charset::DefaultEncoding;

    use super::read_bindgroup_defaults;

    /// テスト専用の一意な shell ディレクトリを作り、descript.txt を書き込んで返す。
    ///
    /// 一時ディレクトリは共通窓口 `temp-path-kit` 経由で組むので、名前にプロセス識別子と
    /// 連番が入り**プロセス間でも一意**になる。返り値が生きている間だけ実体が存在し、
    /// 破棄で中身ごと消える（後始末を呼び忘れる余地が無い）。
    fn shell_with_descript(tag: &str, descript_body: &str) -> TempPath {
        // 札は `-` と英数字だけ（窓口の約束）。呼出側の tag は関数名由来で `_` を含む。
        let dir = TempPath::new(&format!(
            "parsers-bindoption-options-{}",
            tag.replace('_', "-")
        ));
        fs::write(dir.path().join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        dir
    }

    /// `mustselect` 単独宣言はスコープ別に収録され、multiple 側へは漏れない（bindopt 1.2）。
    ///
    /// 既存挙動不変の錨。非宣言カテゴリ・別スコープはどちらの集合にも入らない＝既定。
    #[test]
    fn mustselect_declared_categories_ingested() {
        let temp = shell_with_descript(
            "declared",
            "charset,UTF-8\n\
             sakura.bindoption0.group,腕,mustselect\n\
             sakura.bindoption3.group,目,mustselect\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(scope(&defaults, 0).mustselect.iter().any(|c| c == "腕"));
        assert!(scope(&defaults, 0).mustselect.iter().any(|c| c == "目"));
        // mustselect 単独宣言は multiple 集合へ入らない（2 集合は独立）。
        assert!(scope(&defaults, 0).multiple.is_empty());
        assert!(!scope(&defaults, 0).multiple.iter().any(|c| c == "腕"));
        // 非宣言カテゴリはどちらの集合にも入らない＝既定（捏造しない）。
        assert!(!scope(&defaults, 0).mustselect.iter().any(|c| c == "紅"));
        assert!(!scope(&defaults, 0).multiple.iter().any(|c| c == "紅"));
        // 別スコープ（kero）には漏れない。
        assert!(!scope(&defaults, 1).mustselect.iter().any(|c| c == "腕"));
    }

    /// `multiple` 単独宣言は multiple としてスコープ別に**収録**される（bindopt 1.1）。
    ///
    /// 旧実装は `multiple` を破棄しており、下流に「複数可の明示宣言」と「非宣言（既定＝
    /// 高々 1 個・解除可）」を区別する情報が存在しなかった——情報欠落の根。本檻は収録
    /// されること（＝区別が成立すること）を固定する。
    #[test]
    fn multiple_option_ingested() {
        let temp = shell_with_descript(
            "multiple",
            "charset,UTF-8\nsakura.bindoption0.group,紅,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(scope(&defaults, 0).multiple, vec!["紅".to_string()]);
        assert!(scope(&defaults, 0).multiple.iter().any(|c| c == "紅"));
        // multiple 宣言は mustselect ではない（2 集合は独立）。
        assert!(!scope(&defaults, 0).mustselect.iter().any(|c| c == "紅"));
        assert!(scope(&defaults, 0).mustselect.is_empty());
        // 別スコープ（kero）には漏れない。
        assert!(!scope(&defaults, 1).multiple.iter().any(|c| c == "紅"));
        assert!(scope(&defaults, 1).multiple.is_empty());
    }

    /// kero 側の `multiple` は本体側と区別して収録する（bindopt 1.1・スコープ隔離）。
    #[test]
    fn kero_multiple_distinguished_from_sakura() {
        let temp = shell_with_descript(
            "kero_multiple",
            "charset,UTF-8\nkero.bindoption0.group,尻尾飾り,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(scope(&defaults, 1).multiple, vec!["尻尾飾り".to_string()]);
        assert!(scope(&defaults, 1).multiple.iter().any(|c| c == "尻尾飾り"));
        assert!(!scope(&defaults, 0).multiple.iter().any(|c| c == "尻尾飾り"));
        assert!(scope(&defaults, 0).multiple.is_empty());
    }

    /// kero 側は本体側と区別して取り込む（bindopt 1.2・スコープ隔離）。
    #[test]
    fn kero_mustselect_distinguished_from_sakura() {
        let temp = shell_with_descript(
            "kero_scope",
            "charset,UTF-8\nkero.bindoption0.group,腕,mustselect\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(scope(&defaults, 1).mustselect.iter().any(|c| c == "腕"));
        assert!(!scope(&defaults, 0).mustselect.iter().any(|c| c == "腕"));
        assert!(scope(&defaults, 0).mustselect.is_empty());
    }

    /// `+` 区切り併記（`mustselect+multiple`）は両方の集合へ収録し情報を落とさない
    /// （ukadoc 正典「オプションは `+` 区切りで複数指定可」・bindopt 1.3）。
    ///
    /// 旧実装は値全体と `mustselect` の完全一致で判定していたため、`+` 結合値は
    /// mustselect 側も含めて全部落ちていた。どちらを優先するか（bindopt D4）は
    /// 下流 seriko の解釈であり、転写層は両方を写す。
    #[test]
    fn plus_separated_both_options_ingested_into_both_sets() {
        let temp = shell_with_descript(
            "plus_both",
            "charset,UTF-8\nsakura.bindoption0.group,腕,mustselect+multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(scope(&defaults, 0).mustselect, vec!["腕".to_string()]);
        assert_eq!(scope(&defaults, 0).multiple, vec!["腕".to_string()]);
        assert!(scope(&defaults, 0).mustselect.iter().any(|c| c == "腕"));
        assert!(scope(&defaults, 0).multiple.iter().any(|c| c == "腕"));
    }

    /// `+` 区切りの語順は結果に影響しない（`multiple+mustselect`・bindopt 1.3）。
    #[test]
    fn plus_separated_option_order_is_insensitive() {
        let temp = shell_with_descript(
            "plus_order",
            "charset,UTF-8\nsakura.bindoption0.group,腕,multiple+mustselect\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(scope(&defaults, 0).mustselect, vec!["腕".to_string()]);
        assert_eq!(scope(&defaults, 0).multiple, vec!["腕".to_string()]);
    }

    /// `+` 区切りに未知語が混ざっても、認識できる語のみ収録して未知語は読み流す
    /// （寛容パース・bindopt 1.3/1.4）。空白入りの語も trim して認識する。
    #[test]
    fn plus_separated_unknown_word_is_skipped() {
        let temp = shell_with_descript(
            "plus_unknown",
            "charset,UTF-8\n\
             sakura.bindoption0.group,紅,unknown+multiple\n\
             sakura.bindoption1.group,腕,mustselect + unknown\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // unknown+multiple → multiple のみ収録。
        assert_eq!(scope(&defaults, 0).multiple, vec!["紅".to_string()]);
        assert!(!scope(&defaults, 0).mustselect.iter().any(|c| c == "紅"));
        // mustselect + unknown → mustselect のみ収録（各語 trim）。
        assert_eq!(scope(&defaults, 0).mustselect, vec!["腕".to_string()]);
        assert!(!scope(&defaults, 0).multiple.iter().any(|c| c == "腕"));
    }

    /// 未知語のみ・オプション欄空・カテゴリ空・`,` なしはいずれも収録対象外
    /// （捏造しない・寛容パース維持・bindopt 1.4/1.5）。
    ///
    /// 収録対象外＝どちらの集合にも入らない＝当該カテゴリは既定（高々 1 個・解除可）。
    #[test]
    fn missing_or_empty_fields_not_ingested() {
        let temp = shell_with_descript(
            "missing",
            "charset,UTF-8\n\
             sakura.bindoption0.group,腕\n\
             sakura.bindoption1.group,,mustselect\n\
             sakura.bindoption2.group,口,\n\
             sakura.bindoption4.group,眉,unknown\n\
             sakura.bindoption5.group,頬,Mustselect\n\
             sakura.bindoption6.group, ,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // オプション欄欠落（腕・`,` なし）・カテゴリ空（,mustselect）・オプション欄空（口,）
        // ・未知語のみ（眉）・大小異なる語（頬・完全一致のみ認識）・空白のみカテゴリ（multiple）
        // はいずれも非収録。
        assert!(
            scope(&defaults, 0).mustselect.is_empty(),
            "不完全値・未知語は mustselect へ収録されない: {:?}",
            scope(&defaults, 0).mustselect
        );
        assert!(
            scope(&defaults, 0).multiple.is_empty(),
            "不完全値・空カテゴリは multiple へ収録されない: {:?}",
            scope(&defaults, 0).multiple
        );
        for category in ["腕", "口", "眉", "頬", ""] {
            assert!(!scope(&defaults, 0).mustselect.iter().any(|c| c == category));
            assert!(!scope(&defaults, 0).multiple.iter().any(|c| c == category));
        }
    }

    /// bindoption 宣言が 1 件も無い shell も読み取りは成立し、全カテゴリ既定になる
    /// （読み取り失敗にしない・bindopt 1.6）。
    #[test]
    fn no_bindoption_declarations_yields_all_default() {
        let temp = shell_with_descript(
            "no_bindoption",
            "charset,UTF-8\n\
             sakura.bindgroup1100.default,1\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             kero.bindgroup2100.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // 4 集合すべて空＝全カテゴリ既定（高々 1 個・解除可）。
        assert!(scope(&defaults, 0).mustselect.is_empty());
        assert!(scope(&defaults, 1).mustselect.is_empty());
        assert!(scope(&defaults, 0).multiple.is_empty());
        assert!(scope(&defaults, 1).multiple.is_empty());
        // 併存する `.default`／`.name` 経路は通常どおり成立する（読み取り失敗にしない）。
        assert_eq!(scope(&defaults, 0).default_on, vec![1100]);
        assert_eq!(ids_of(&defaults, 0, "腕", "伸び"), vec![1100]);
        assert_eq!(ids_of(&defaults, 1, "腕", "伸び"), vec![2100]);
    }

    /// 同一 descript を再読すると同一の収録結果を返す（決定論・走査順維持・bindopt 1.7）。
    #[test]
    fn same_descript_reread_yields_same_result() {
        let temp = shell_with_descript(
            "determinism",
            "charset,UTF-8\n\
             sakura.bindoption0.group,腕,mustselect\n\
             sakura.bindoption1.group,口,mustselect+multiple\n\
             sakura.bindoption2.group,紅,multiple\n\
             sakura.bindoption3.group,眉,unknown\n\
             kero.bindoption0.group,尻尾飾り,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let first = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);
        let second = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(first, second, "同一入力に同一結果（決定論）");
        // 収録順もキー昇順で固定（走査順維持）。
        assert_eq!(
            scope(&first, 0).mustselect,
            vec!["腕".to_string(), "口".to_string()]
        );
        assert_eq!(
            scope(&first, 0).multiple,
            vec!["口".to_string(), "紅".to_string()]
        );
        assert_eq!(scope(&first, 1).multiple, vec!["尻尾飾り".to_string()]);
        assert!(scope(&first, 1).mustselect.is_empty());
    }

    /// Observable（emo2 実 fixture）: 腕・口・眉・目 が `mustselect` 宣言と判別でき、
    /// 宣言のないカテゴリ（紅・髪飾り・まばたき）はどちらの集合にも属さない＝既定
    /// （高々 1 個・解除可）と判別できる（bindopt 1.1/1.2）。
    ///
    /// emo2 は `multiple` を 1 件も宣言していない——本件の表情固着はまさに「非宣言の
    /// まばたきカテゴリ（1400-1403）」で起きており、非宣言が `multiple` と同一視されて
    /// いたことが根であった。
    #[test]
    fn emo2_bindoption_categories_discriminated() {
        let shell_dir = crate::sample_test_support::emo2_root()
            .join("shell")
            .join("master");
        let defaults = read_bindgroup_defaults(&shell_dir, DefaultEncoding::Utf8);

        assert!(scope(&defaults, 0).mustselect.iter().any(|c| c == "腕"));
        assert!(scope(&defaults, 0).mustselect.iter().any(|c| c == "口"));
        assert!(scope(&defaults, 0).mustselect.iter().any(|c| c == "眉"));
        assert!(scope(&defaults, 0).mustselect.iter().any(|c| c == "目"));
        // emo2 は multiple を 1 件も宣言していない。
        assert!(
            scope(&defaults, 0).multiple.is_empty(),
            "emo2 に multiple 宣言は無い: {:?}",
            scope(&defaults, 0).multiple
        );
        assert!(scope(&defaults, 1).multiple.is_empty());
        // 宣言のないカテゴリ（紅・髪飾りは .name 宣言はあるが bindoption 宣言なし）＝既定。
        for category in ["紅", "髪飾り", "まばたき"] {
            assert!(!scope(&defaults, 0).mustselect.iter().any(|c| c == category));
            assert!(!scope(&defaults, 0).multiple.iter().any(|c| c == category));
        }
        // kero 側は宣言なし＝全偽（スコープ隔離）。
        assert!(!scope(&defaults, 1).mustselect.iter().any(|c| c == "腕"));
        assert!(scope(&defaults, 1).mustselect.is_empty());
    }
}

#[cfg(test)]
mod char_scope_and_line_tests {
    //! `char{N}.*`（N は 2 以上）の読みと、名前を決めた行の番号（offline-surface-dump 1.2）。

    use std::fs;

    use temp_path_kit::TempPath;

    use super::{ids_of, scope};
    use crate::charset::DefaultEncoding;

    use super::read_bindgroup_defaults;

    fn shell_with_descript(tag: &str, descript_body: &str) -> TempPath {
        let dir = TempPath::new(&format!("parsers-bind-char-{}", tag.replace('_', "-")));
        fs::write(dir.path().join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        dir
    }

    /// 名前には決めた行の番号（同じキーは後の行・CRLF でもずれない）が付く。
    #[test]
    fn names_carry_deciding_line() {
        let temp = shell_with_descript(
            "name_line",
            "charset,UTF-8\r\n\
             sakura.bindgroup10.name,腕,伸び\r\n\
             kero.bindgroup20.name,尾,長い\r\n\
             sakura.bindgroup10.name,腕,曲げ\r\n",
        );
        let defaults = read_bindgroup_defaults(temp.path(), DefaultEncoding::Utf8);
        assert_eq!(scope(&defaults, 0).names.len(), 1);
        assert_eq!(scope(&defaults, 0).names[0].part, "曲げ");
        assert_eq!(scope(&defaults, 0).names[0].line, 4);
        assert_eq!(scope(&defaults, 1).names[0].line, 3);
    }

    /// `char2.*` の既定・名前・bindoption が引け、sakura・kero と混ざらない。
    #[test]
    fn char2_defaults_names_and_options_resolve() {
        let temp = shell_with_descript(
            "char2",
            "charset,UTF-8\n\
             sakura.bindgroup1.default,1\n\
             kero.bindgroup2.default,1\n\
             char2.bindgroup30.default,1\n\
             char2.bindgroup31.default,0\n\
             char2.bindgroup30.name,帽子,赤,thumb\n\
             char2.bindgroup31.name,帽子,青\n\
             char2.bindoption0.group,帽子,mustselect+multiple\n",
        );
        let d = read_bindgroup_defaults(temp.path(), DefaultEncoding::Utf8);

        assert_eq!(scope(&d, 0).default_on, &[1]);
        assert_eq!(scope(&d, 1).default_on, &[2]);
        assert_eq!(scope(&d, 2).default_on, &[30]);
        assert_eq!(ids_of(&d, 2, "帽子", "赤"), vec![30]);
        assert_eq!(ids_of(&d, 2, "帽子", "青"), vec![31]);
        assert!(scope(&d, 2).mustselect.iter().any(|c| c == "帽子"));
        assert!(scope(&d, 2).multiple.iter().any(|c| c == "帽子"));
        let lines: Vec<usize> = d[&2].names.iter().map(|n| n.line).collect();
        assert_eq!(lines, vec![6, 7]);
        // ほかのスコープへ漏れない。
        assert_eq!(ids_of(&d, 0, "帽子", "赤"), Vec::<u32>::new());
        assert!(!scope(&d, 1).mustselect.iter().any(|c| c == "帽子"));
    }

    /// `char3` の定義が無ければ、そのスコープの照会はすべて空。`char0`・`char1` は読まない。
    #[test]
    fn char3_without_definitions_is_empty() {
        let temp = shell_with_descript(
            "char3",
            "charset,UTF-8\n\
             char2.bindgroup30.default,1\n\
             char2.bindgroup30.name,帽子,赤\n\
             char1.bindgroup40.default,1\n\
             char0.bindgroup41.name,帽子,赤\n",
        );
        let d = read_bindgroup_defaults(temp.path(), DefaultEncoding::Utf8);

        assert!(scope(&d, 3).default_on.is_empty());
        assert_eq!(ids_of(&d, 3, "帽子", "赤"), Vec::<u32>::new());
        assert!(!scope(&d, 3).mustselect.iter().any(|c| c == "帽子"));
        assert!(!scope(&d, 3).multiple.iter().any(|c| c == "帽子"));
        assert!(scope(&d, 0).default_on.is_empty() && scope(&d, 1).default_on.is_empty());
        assert!(scope(&d, 0).names.is_empty() && scope(&d, 1).names.is_empty());
        assert_eq!(d.keys().copied().collect::<Vec<_>>(), vec![2]);
    }
}

#[cfg(test)]
mod bindgroup_warning_tests {
    //! 読みの崩れの記録（指摘 9・14・49）と数の形・空のカテゴリ（指摘 47・48）。
    //!
    //! 記録は `log_capture_kit::capture` で確かめ、一時の根は `temp-path-kit`。

    use std::fs;

    use log_capture_kit::CapturedEvent;
    use temp_path_kit::TempPath;

    use crate::charset::DefaultEncoding;
    use crate::package::model::BindGroupDefaults;

    use super::scope;

    use super::read_bindgroup_defaults;

    fn shell_with_descript(tag: &str, descript_body: &str) -> TempPath {
        let dir = TempPath::new(&format!("parsers-bind-warn-{}", tag.replace('_', "-")));
        fs::write(dir.path().join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        dir
    }

    /// 読みを記録つきで回し、表と `warn!` の件を返す。
    fn read_capturing(shell: &TempPath) -> (BindGroupDefaults, Vec<CapturedEvent>) {
        let (d, events) = log_capture_kit::capture(|| {
            read_bindgroup_defaults(shell.path(), DefaultEncoding::Utf8)
        });
        let warns = events
            .into_iter()
            .filter(|e| e.level == tracing::Level::WARN)
            .collect();
        (d, warns)
    }

    /// NotFound 以外で読めない descript はパスと誤りを `warn!` して空。無いだけなら黙って空。
    #[test]
    fn unreadable_shell_descript_warns_and_reads_no_binds() {
        let shell = TempPath::new("parsers-bind-warn-unreadable");
        let descript = shell.path().join("descript.txt");
        fs::create_dir_all(&descript).expect("descript.txt as a folder");

        let (d, warns) = read_capturing(&shell);

        assert_eq!(d, BindGroupDefaults::default());
        assert_eq!(warns.len(), 1, "{warns:?}");
        assert_eq!(
            warns[0].field("path"),
            Some(descript.display().to_string().as_str()),
            "{warns:?}"
        );
        assert!(warns[0].field("error").is_some(), "{warns:?}");

        let missing = TempPath::new("parsers-bind-warn-missing");
        fs::create_dir_all(missing.path()).expect("shell dir");
        let (d, warns) = read_capturing(&missing);
        assert_eq!(d, BindGroupDefaults::default());
        assert!(warns.is_empty(), "{warns:?}");
    }

    /// 綴り違いの bindoption は鍵・行・値つきで `warn!`（結果は寛容のまま＝入らない）。
    #[test]
    fn misspelled_bindoption_warns() {
        let temp = shell_with_descript(
            "misspelled_option",
            "charset,UTF-8\nsakura.bindoption0.group,腕,mustselct\n",
        );
        let (d, warns) = read_capturing(&temp);

        assert!(!scope(&d, 0).mustselect.iter().any(|c| c == "腕"));
        assert_eq!(warns.len(), 1, "{warns:?}");
        assert_eq!(warns[0].field("key"), Some("sakura.bindoption0.group"));
        assert_eq!(warns[0].field("line"), Some("2"));
        assert_eq!(warns[0].field("value"), Some("腕,mustselct"));
    }

    /// 0/1 でない `.default` は鍵・行・値つきで `warn!`（off のまま）。
    #[test]
    fn non_binary_default_warns() {
        let temp = shell_with_descript(
            "non_binary_default",
            "charset,UTF-8\nsakura.bindgroup10.default,yes\n",
        );
        let (d, warns) = read_capturing(&temp);

        assert!(scope(&d, 0).default_on.is_empty());
        assert_eq!(warns.len(), 1, "{warns:?}");
        assert_eq!(warns[0].field("key"), Some("sakura.bindgroup10.default"));
        assert_eq!(warns[0].field("line"), Some("2"));
        assert_eq!(warns[0].field("value"), Some("yes"));
    }

    /// `.default,0` は正しい宣言なので記録しない。
    #[test]
    fn default_zero_does_not_warn() {
        let temp = shell_with_descript(
            "default_zero",
            "charset,UTF-8\nsakura.bindgroup10.default,0\nsakura.bindgroup11.default,1\n",
        );
        let (d, warns) = read_capturing(&temp);

        assert_eq!(scope(&d, 0).default_on, &[11]);
        assert!(warns.is_empty(), "{warns:?}");
    }

    /// 番号は数字だけ・先頭の 0 は `0` のときだけ。外れた鍵は読まずに `warn!`。
    #[test]
    fn bindgroup_number_rejects_plus_and_leading_zero() {
        let temp = shell_with_descript(
            "number_shape",
            "charset,UTF-8\n\
             sakura.bindgroup0010.default,1\n\
             sakura.bindgroup+11.default,1\n\
             sakura.bindgroup0.default,1\n\
             sakura.bindgroup012.name,腕,伸び\n\
             sakura.bindoption+1.group,腕,mustselect\n\
             char02.bindgroup13.default,1\n",
        );
        let (d, warns) = read_capturing(&temp);

        assert_eq!(scope(&d, 0).default_on, &[0]);
        assert!(scope(&d, 0).names.is_empty());
        assert!(!scope(&d, 0).mustselect.iter().any(|c| c == "腕"));
        assert_eq!(d.keys().copied().collect::<Vec<_>>(), vec![0]);
        let keys: Vec<&str> = warns.iter().filter_map(|w| w.field("key")).collect();
        assert_eq!(
            keys,
            vec![
                "sakura.bindgroup+11.default",
                "sakura.bindgroup0010.default",
                "sakura.bindgroup012.name",
                "sakura.bindoption+1.group",
            ],
            "{warns:?}"
        );
    }

    /// 空のカテゴリの `.name` は転記しない（bindoption と同じく断る）。
    #[test]
    fn name_with_empty_category_is_rejected() {
        let temp = shell_with_descript(
            "empty_category",
            "charset,UTF-8\nsakura.bindgroup10.name,,伸び\n",
        );
        let (d, warns) = read_capturing(&temp);

        assert!(scope(&d, 0).names.is_empty());
        assert_eq!(warns.len(), 1, "{warns:?}");
        assert_eq!(warns[0].field("key"), Some("sakura.bindgroup10.name"));
    }

    /// パーツ名の無い `.name` の記録は鍵と行でスコープを見分けられる。
    #[test]
    fn nameless_part_warnings_carry_key_and_line() {
        let temp = shell_with_descript(
            "nameless_part",
            "charset,UTF-8\nsakura.bindgroup10.name,x\nkero.bindgroup10.name,y\n",
        );
        let (_, warns) = read_capturing(&temp);

        assert_eq!(warns.len(), 2, "{warns:?}");
        // 走査は鍵の昇順（kero が先）。
        assert_eq!(warns[0].field("key"), Some("kero.bindgroup10.name"));
        assert_eq!(warns[0].field("line"), Some("3"));
        assert_eq!(warns[1].field("key"), Some("sakura.bindgroup10.name"));
        assert_eq!(warns[1].field("line"), Some("2"));
    }
}
