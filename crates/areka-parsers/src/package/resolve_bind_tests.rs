//! resolve_bind_tests — `resolve_bind::read_bindgroup_defaults` の兄弟テスト。
//!
//! `resolve.rs` の中の試験 2 つ（`.name` の転記・bindoption の分解）を中身を変えずに移した。

use super::resolve_bind::read_bindgroup_defaults;

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

    use crate::charset::DefaultEncoding;
    use crate::package::model::BindScope;

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

        assert_eq!(defaults.sakura_names.len(), 1);
        let n = &defaults.sakura_names[0];
        assert_eq!(n.id, 1100);
        assert_eq!(n.category, "腕");
        assert_eq!(n.part, "伸び");
        assert_eq!(n.thumbnail, Some("thumb".to_string()));
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
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

        assert_eq!(defaults.sakura_names.len(), 1);
        assert_eq!(defaults.sakura_names[0].thumbnail, None);
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "口", "笑い"),
            Some(1200)
        );
    }

    /// kero 側は本体側と区別して `kero_names` へ入る（R1.2）。
    #[test]
    fn kero_name_distinguished_from_sakura() {
        let temp = shell_with_descript(
            "kero_scope",
            "charset,UTF-8\nkero.bindgroup2100.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(defaults.sakura_names.is_empty(), "sakura 側は空であるべき");
        assert_eq!(defaults.kero_names.len(), 1);
        assert_eq!(
            defaults.resolve_name(BindScope::Kero, "腕", "伸び"),
            Some(2100)
        );
        // sakura スコープでは解決できない（別集合）。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), None);
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
            defaults.sakura_names.is_empty(),
            "パーツ欠落の宣言は転記されない: {:?}",
            defaults.sakura_names
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

        assert!(defaults.sakura_names.is_empty());
    }

    /// 重複 (カテゴリ, パーツ) はキー昇順走査の後勝ち（D2）＝最後の ID を採る。
    #[test]
    fn duplicate_category_part_last_wins() {
        // 1100 と 1400 が同じ (腕, 伸び) を宣言。キー昇順で 1100→1400 の順に転記され、
        // `resolve_name` は後勝ちで 1400 を返す。
        let temp = shell_with_descript(
            "duplicate",
            "charset,UTF-8\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             sakura.bindgroup1400.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1400)
        );
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

        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(defaults.kero_default_on, vec![2100]);
        // 名前も併せて転記されている（両経路が同一走査で共存）。
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
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
        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(defaults.kero_default_on, vec![2100]);

        // 名前宣言が皆無 → 名前表は空（default,1 が名前を捏造しない・R1.5/R1.6）。
        assert!(
            defaults.sakura_names.is_empty(),
            "名前宣言皆無なら sakura_names は空"
        );
        assert!(
            defaults.kero_names.is_empty(),
            "名前宣言皆無なら kero_names は空"
        );

        // 名前解決は全スコープで None（着せ替え ID を捏造しない）。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), None);
        assert_eq!(defaults.resolve_name(BindScope::Kero, "腕", "伸び"), None);
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
            defaults.sakura_names.len() >= 30,
            "emo2 の sakura .name 宣言が十分に転記されている（実測 30 本超）: {}",
            defaults.sakura_names.len()
        );
        // 代表的な宣言が (カテゴリ, パーツ) → ID へ引ける。
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "口", "にこっ"),
            Some(1207)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "目", "通常"),
            Some(1302)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "髪飾り", "ボンボン"),
            Some(1801)
        );
        // 宣言済みの全組が漏れなく引ける（サムネ無し 2 フィールド形でも解決）。
        for n in &defaults.sakura_names {
            assert_eq!(
                defaults.resolve_name(BindScope::Sakura, &n.category, &n.part),
                Some(n.id),
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

    use crate::charset::DefaultEncoding;
    use crate::package::model::BindScope;

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

        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "目"));
        // mustselect 単独宣言は multiple 集合へ入らない（2 集合は独立）。
        assert!(defaults.sakura_multiple.is_empty());
        assert!(!defaults.is_multiple(BindScope::Sakura, "腕"));
        // 非宣言カテゴリはどちらの集合にも入らない＝既定（捏造しない）。
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
        assert!(!defaults.is_multiple(BindScope::Sakura, "紅"));
        // 別スコープ（kero）には漏れない。
        assert!(!defaults.is_mustselect(BindScope::Kero, "腕"));
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

        assert_eq!(defaults.sakura_multiple, vec!["紅".to_string()]);
        assert!(defaults.is_multiple(BindScope::Sakura, "紅"));
        // multiple 宣言は mustselect ではない（2 集合は独立）。
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
        assert!(defaults.sakura_mustselect.is_empty());
        // 別スコープ（kero）には漏れない。
        assert!(!defaults.is_multiple(BindScope::Kero, "紅"));
        assert!(defaults.kero_multiple.is_empty());
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

        assert_eq!(defaults.kero_multiple, vec!["尻尾飾り".to_string()]);
        assert!(defaults.is_multiple(BindScope::Kero, "尻尾飾り"));
        assert!(!defaults.is_multiple(BindScope::Sakura, "尻尾飾り"));
        assert!(defaults.sakura_multiple.is_empty());
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

        assert!(defaults.is_mustselect(BindScope::Kero, "腕"));
        assert!(!defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.sakura_mustselect.is_empty());
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

        assert_eq!(defaults.sakura_mustselect, vec!["腕".to_string()]);
        assert_eq!(defaults.sakura_multiple, vec!["腕".to_string()]);
        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_multiple(BindScope::Sakura, "腕"));
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

        assert_eq!(defaults.sakura_mustselect, vec!["腕".to_string()]);
        assert_eq!(defaults.sakura_multiple, vec!["腕".to_string()]);
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
        assert_eq!(defaults.sakura_multiple, vec!["紅".to_string()]);
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
        // mustselect + unknown → mustselect のみ収録（各語 trim）。
        assert_eq!(defaults.sakura_mustselect, vec!["腕".to_string()]);
        assert!(!defaults.is_multiple(BindScope::Sakura, "腕"));
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
            defaults.sakura_mustselect.is_empty(),
            "不完全値・未知語は mustselect へ収録されない: {:?}",
            defaults.sakura_mustselect
        );
        assert!(
            defaults.sakura_multiple.is_empty(),
            "不完全値・空カテゴリは multiple へ収録されない: {:?}",
            defaults.sakura_multiple
        );
        for category in ["腕", "口", "眉", "頬", ""] {
            assert!(!defaults.is_mustselect(BindScope::Sakura, category));
            assert!(!defaults.is_multiple(BindScope::Sakura, category));
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
        assert!(defaults.sakura_mustselect.is_empty());
        assert!(defaults.kero_mustselect.is_empty());
        assert!(defaults.sakura_multiple.is_empty());
        assert!(defaults.kero_multiple.is_empty());
        // 併存する `.default`／`.name` 経路は通常どおり成立する（読み取り失敗にしない）。
        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Kero, "腕", "伸び"),
            Some(2100)
        );
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
            first.sakura_mustselect,
            vec!["腕".to_string(), "口".to_string()]
        );
        assert_eq!(
            first.sakura_multiple,
            vec!["口".to_string(), "紅".to_string()]
        );
        assert_eq!(first.kero_multiple, vec!["尻尾飾り".to_string()]);
        assert!(first.kero_mustselect.is_empty());
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

        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "口"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "眉"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "目"));
        // emo2 は multiple を 1 件も宣言していない。
        assert!(
            defaults.sakura_multiple.is_empty(),
            "emo2 に multiple 宣言は無い: {:?}",
            defaults.sakura_multiple
        );
        assert!(defaults.kero_multiple.is_empty());
        // 宣言のないカテゴリ（紅・髪飾りは .name 宣言はあるが bindoption 宣言なし）＝既定。
        for category in ["紅", "髪飾り", "まばたき"] {
            assert!(!defaults.is_mustselect(BindScope::Sakura, category));
            assert!(!defaults.is_multiple(BindScope::Sakura, category));
        }
        // kero 側は宣言なし＝全偽（スコープ隔離）。
        assert!(!defaults.is_mustselect(BindScope::Kero, "腕"));
        assert!(defaults.kero_mustselect.is_empty());
    }
}

#[cfg(test)]
mod char_scope_and_line_tests {
    //! `char{N}.*`（N は 2 以上）の読みと、名前を決めた行の番号（offline-surface-dump 1.2）。

    use std::fs;

    use temp_path_kit::TempPath;

    use crate::charset::DefaultEncoding;
    use crate::package::model::BindScope;

    use super::read_bindgroup_defaults;
    use crate::package::resolve_bind::parse_kv_with_line;

    fn shell_with_descript(tag: &str, descript_body: &str) -> TempPath {
        let dir = TempPath::new(&format!("parsers-bind-char-{}", tag.replace('_', "-")));
        fs::write(dir.path().join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        dir
    }

    /// 値とキーの前後の空白を取り、1 始まりの行の番号を添える。
    #[test]
    fn line_read_trims_key_and_value() {
        let map = parse_kv_with_line("charset,UTF-8\n  a  ,  x y  \n");
        assert_eq!(map.get("a"), Some(&("x y".to_string(), 2)));
    }

    /// 同じキーが 2 行あれば後の行の値と行の番号になる。カンマの無い行・キーが空の行も数える。
    #[test]
    fn line_read_same_key_takes_later_line() {
        let map = parse_kv_with_line("a,1\nno comma\n,empty key\na,3\n");
        assert_eq!(map.get("a"), Some(&("3".to_string(), 4)));
        assert_eq!(map.len(), 1);
    }

    /// CRLF でも行の番号がずれず、値に CR が残らない。
    #[test]
    fn line_read_crlf_keeps_line_numbers() {
        let map = parse_kv_with_line("a,1\r\n\r\nb,2\r\n");
        assert_eq!(map.get("a"), Some(&("1".to_string(), 1)));
        assert_eq!(map.get("b"), Some(&("2".to_string(), 3)));
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
        assert_eq!(defaults.sakura_names.len(), 1);
        assert_eq!(defaults.sakura_names[0].part, "曲げ");
        assert_eq!(defaults.sakura_names[0].line, 4);
        assert_eq!(defaults.kero_names[0].line, 3);
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
        let c2 = BindScope::Char(2);

        assert_eq!(d.default_on(BindScope::Sakura), &[1]);
        assert_eq!(d.default_on(BindScope::Kero), &[2]);
        assert_eq!(d.default_on(c2), &[30]);
        assert_eq!(d.resolve_name(c2, "帽子", "赤"), Some(30));
        assert_eq!(d.resolve_name(c2, "帽子", "青"), Some(31));
        assert_eq!(d.category_ids(c2, "帽子"), vec![30, 31]);
        assert!(d.is_mustselect(c2, "帽子"));
        assert!(d.is_multiple(c2, "帽子"));
        let lines: Vec<usize> = d.char_names[&2].iter().map(|n| n.line).collect();
        assert_eq!(lines, vec![6, 7]);
        // ほかのスコープへ漏れない。
        assert_eq!(d.resolve_name(BindScope::Sakura, "帽子", "赤"), None);
        assert!(!d.is_mustselect(BindScope::Kero, "帽子"));
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
        let c3 = BindScope::Char(3);

        assert!(d.default_on(c3).is_empty());
        assert_eq!(d.resolve_name(c3, "帽子", "赤"), None);
        assert!(d.category_ids(c3, "帽子").is_empty());
        assert!(!d.is_mustselect(c3, "帽子"));
        assert!(!d.is_multiple(c3, "帽子"));
        assert!(d.sakura_default_on.is_empty() && d.kero_default_on.is_empty());
        assert!(d.sakura_names.is_empty() && d.kero_names.is_empty());
        assert_eq!(
            d.char_default_on.keys().copied().collect::<Vec<_>>(),
            vec![2]
        );
    }
}
