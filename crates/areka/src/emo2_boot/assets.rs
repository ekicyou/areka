//! 構築入力（BootAssets）の組立と shell descript からの static bindset 抽出。
//!
//! `build_boot_assets`（shell: `surfaces.txt` 読取→`areka_parsers::shell::parse`→bake→scope ごとに
//! `EmoWorld::build`＋`bind_atlas`／balloon: `build_balloon_target`＋`BalloonModel`／
//! `SurfaceResolver`＝`alias_snapshot()`／static bindset＝`default_bind_ids`→`build_static_bindset`）と
//! `default_bind_ids`（`sakura.bindgroup{N}.default==1` の N 抽出・DD-8・ukadoc 正典）を所有する。
//! 戻り値だけで以後ファイル I/O 不要にする（parse／bake は 1 回・`AtlasTable` は Clone 共有）。
//! 失敗は `BootWiringError`（`#[from]` 変換群）で観測可能化する。
//!
//! `default_bind_ids` は tasks.md task 2.3 で実装済み。`build_boot_assets` の骨格は残り、
//! 実装は tasks.md task 2.6 が担う。

use std::collections::BTreeMap;

/// shell descript KV から `default==1` の bindgroup id を抽出する純関数（DD-8・ukadoc 正典）。
///
/// `sakura.bindgroup{N}.default` 形（`N` は `u32`）のキーで、値を trim した結果が `"1"` の
/// エントリの `N` を集める。抽出は sakura scope 限定であり、`kero.*` scope の bindgroup default
/// は本タスクの対象外（M-dual 増分・design.md Non-Goals「kero scope の bindgroup default 分離」）。
///
/// 除外条件:
/// - 値（trim 後）が `"1"` でないもの（`"0"`／`"2"`／空／`"10"` 等）。
/// - `kero.*` 等 `sakura.bindgroup` 以外の prefix を持つキー。
/// - `.name`／`.group` 等 `.default` 以外の suffix を持つキーや、その他の無関係キー。
/// - 中間 `N` が `u32` として parse できないキー（`XYZ`／空／負値／数字混在）。
///
/// prefix/suffix は厳密一致で判定する（`strip_prefix`/`strip_suffix`）。パターンを部分文字列
/// として含むだけのキー（例 `xsakura.bindgroup1.default`／`sakura.bindgroup1.defaultx`）は
/// match せず、中間を `u32` として厳密 parse することで数値部の false-positive も防ぐ。
///
/// 純粋関数（状態・I/O なし）。戻り値は決定論のため数値昇順にソートし重複を除去する
/// （`BTreeMap` のキー反復は lexicographic 順のため numeric 昇順とは一致しない）。
pub fn default_bind_ids(shell_kv: &BTreeMap<String, String>) -> Vec<u32> {
    /// sakura scope の bindgroup キー prefix（厳密一致）。
    const PREFIX: &str = "sakura.bindgroup";
    /// bindgroup default キー suffix（厳密一致）。
    const SUFFIX: &str = ".default";

    let mut ids: Vec<u32> = shell_kv
        .iter()
        .filter_map(|(key, value)| {
            // 値（trim 後）が "1" のエントリだけを対象にする。
            if value.trim() != "1" {
                return None;
            }
            // `sakura.bindgroup` <N> `.default` を厳密に剥がし、中間を u32 として parse する。
            key.strip_prefix(PREFIX)
                .and_then(|rest| rest.strip_suffix(SUFFIX))
                .and_then(|mid| mid.parse::<u32>().ok())
        })
        .collect();

    // 決定論: numeric 昇順ソート＋重複除去（キー反復順に依存しない安定した戻り値）。
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(key, value)` のスライスから shell descript KV 相当の `BTreeMap` を組む。
    fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    /// 観測可能な完了条件: emo2 fixture 相当 KV から `[1100,1207,1302,1500,1800]` を抽出する。
    ///
    /// noise（`.name` エントリ・`kero.*`・`sakura.menu`・`sakura.bindoption*`・`charset`/`type`/
    /// `seriko.*`）を混在させても `.default==1` の 5 件だけが昇順で抽出されることを固定する
    /// （実 fixture `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt` 実測）。
    #[test]
    fn default_bind_ids_extracts_emo2_defaults() {
        let map = kv(&[
            // --- メタ／既定（noise・非 bindgroup） ---
            ("charset", "UTF-8"),
            ("type", "shell"),
            ("seriko.use_self_alpha", "1"), // value=="1" だが bindgroup キーでない → 非抽出
            ("sakura.defaultx", "0"),
            ("kero.defaultx", "0"),
            ("sakura.balloon.alignment", "left"),
            ("kero.balloon.alignment", "right"),
            // --- 腕 ---
            ("sakura.bindgroup1100.name", "腕,伸び"),
            ("sakura.bindgroup1100.default", "1"),
            ("sakura.bindgroup1101.name", "腕,組み"),
            // --- 口 ---
            ("sakura.bindgroup1206.name", "口,‥‥"),
            ("sakura.bindgroup1207.name", "口,にこっ"),
            ("sakura.bindgroup1207.default", "1"),
            ("sakura.bindgroup1208.name", "口,小口"),
            // --- 目 ---
            ("sakura.bindgroup1301.name", "目,ジトー"),
            ("sakura.bindgroup1302.name", "目,通常"),
            ("sakura.bindgroup1302.default", "1"),
            ("sakura.bindgroup1303.name", "目,笑顔"),
            // --- まばたき（default なし） ---
            ("sakura.bindgroup1400.name", "まばたき,通常"),
            ("sakura.bindgroup1403.name", "まばたき,----"),
            // --- 眉 ---
            ("sakura.bindgroup1500.name", "眉,通常"),
            ("sakura.bindgroup1500.default", "1"),
            ("sakura.bindgroup1501.name", "眉,オコ"),
            // --- 紅／キラリ（default なし） ---
            ("sakura.bindgroup1600.name", "紅,差し"),
            ("sakura.bindgroup1700.name", "キラリ,キラリ1"),
            // --- 髪飾り ---
            ("sakura.bindgroup1800.name", "髪飾り,リボン"),
            ("sakura.bindgroup1800.default", "1"),
            ("sakura.bindgroup1801.name", "髪飾り,ボンボン"),
            // --- 着せ替えオプション／メニュー（noise） ---
            ("sakura.bindoption0.group", "腕,mustselect"),
            ("sakura.menu", "auto"),
        ]);
        assert_eq!(
            default_bind_ids(&map),
            vec![1100, 1207, 1302, 1500, 1800],
            "emo2 fixture 相当 KV からは default==1 の 5 件のみを昇順抽出する"
        );
    }

    /// `default` が `1` 以外（`0`／`2`／空／非数）の値は抽出しない。
    #[test]
    fn default_bind_ids_excludes_non_one_values() {
        let map = kv(&[
            ("sakura.bindgroup9990.default", "0"),
            ("sakura.bindgroup9991.default", "2"),
            ("sakura.bindgroup9992.default", ""),
            ("sakura.bindgroup9993.default", "true"),
            ("sakura.bindgroup9994.default", "10"), // "10" は "1" と等しくない
        ]);
        assert_eq!(
            default_bind_ids(&map),
            Vec::<u32>::new(),
            "default==1 以外は全て非抽出"
        );
    }

    /// `kero.*` scope の bindgroup default は本タスク対象外（M-dual 増分）＝非抽出。
    #[test]
    fn default_bind_ids_excludes_kero_scope() {
        let map = kv(&[
            ("kero.bindgroup50.default", "1"),
            ("kero.bindgroup1100.default", "1"),
            // sakura 側は抽出される対照。
            ("sakura.bindgroup1100.default", "1"),
        ]);
        assert_eq!(
            default_bind_ids(&map),
            vec![1100],
            "kero scope は非抽出（M-dual 増分）・sakura scope のみ抽出"
        );
    }

    /// 無関係キー（`.name`・任意 noise）は `default==1` を持たない限り無視する。
    #[test]
    fn default_bind_ids_ignores_unrelated_keys() {
        let map = kv(&[
            ("sakura.bindgroup1100.name", "腕,伸び"), // .name は非抽出
            ("charset", "UTF-8"),
            ("some.random.key", "1"),
            ("sakura.menu", "auto"),
        ]);
        assert_eq!(
            default_bind_ids(&map),
            Vec::<u32>::new(),
            "default==1 の bindgroup キーが無ければ何も抽出しない"
        );
    }

    /// 中間の N が u32 として parse できないキーは無視する。
    #[test]
    fn default_bind_ids_ignores_malformed_middle() {
        let map = kv(&[
            ("sakura.bindgroupXYZ.default", "1"), // 非数値
            ("sakura.bindgroup.default", "1"),    // 空（middle なし）
            ("sakura.bindgroup-1.default", "1"),  // 負値は u32 parse 不可
            ("sakura.bindgroup12ab.default", "1"), // 数字混在
        ]);
        assert_eq!(
            default_bind_ids(&map),
            Vec::<u32>::new(),
            "middle が u32 でないキーは非抽出"
        );
    }

    /// prefix/suffix は厳密一致。パターンを部分文字列として含むだけのキーは match しない。
    #[test]
    fn default_bind_ids_requires_strict_prefix_and_suffix() {
        let map = kv(&[
            ("xsakura.bindgroup1.default", "1"),   // prefix 前に余分
            ("sakura.bindgroup2.defaultx", "1"),   // suffix 後に余分
            ("sakura.bindgroup3.default.extra", "1"), // suffix の後に別セグメント
            ("prefix.sakura.bindgroup4.default", "1"), // 別 prefix 配下
            // 対照: 厳密一致は抽出される。
            ("sakura.bindgroup5.default", "1"),
        ]);
        assert_eq!(
            default_bind_ids(&map),
            vec![5],
            "prefix/suffix 厳密一致のキーのみ抽出（部分一致は除外）"
        );
    }

    /// 決定論: 結果は数値昇順（lexicographic ではなく numeric）で返る。
    ///
    /// BTreeMap のキー反復は lexicographic 順（"1000" < "100" < "200" < "90"）であり、
    /// numeric 昇順（90 < 100 < 200 < 1000）と一致しない。結果が numeric 昇順であることを
    /// 檻に入れ、キー反復順への依存を排除する。
    #[test]
    fn default_bind_ids_returns_sorted_numeric_ascending() {
        let map = kv(&[
            ("sakura.bindgroup90.default", "1"),
            ("sakura.bindgroup100.default", "1"),
            ("sakura.bindgroup1000.default", "1"),
            ("sakura.bindgroup200.default", "1"),
        ]);
        let ids = default_bind_ids(&map);
        assert_eq!(
            ids,
            vec![90, 100, 200, 1000],
            "numeric 昇順で返る（lexicographic キー順に依存しない）"
        );
        // 重複なし（決定論の担保）。
        let mut sorted_unique = ids.clone();
        sorted_unique.dedup();
        assert_eq!(ids, sorted_unique, "結果に重複が無い");
    }

    /// 値は trim して比較する（前後空白付き `" 1 "` は抽出・`" 0 "` は非抽出）。
    #[test]
    fn default_bind_ids_trims_value_whitespace() {
        let map = kv(&[
            ("sakura.bindgroup10.default", " 1 "),
            ("sakura.bindgroup20.default", "\t1\n"),
            ("sakura.bindgroup30.default", " 0 "),
        ]);
        assert_eq!(
            default_bind_ids(&map),
            vec![10, 20],
            "trim 後に \"1\" と一致する値のみ抽出"
        );
    }
}
