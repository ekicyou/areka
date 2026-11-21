//! emo2 fixture を用いた bake パイプラインの統合テスト（shell・balloon 横断）。
//!
//! 実 fixture emo2 の shell surface 一式・balloon 画像を bake パイプライン
//! （マニフェスト導出 → デコード → 正規化 → トリム → packing → 焼付）に通し、
//! WIC 腕越しに実 PNG を復号して成功/失敗を判定する。
//!
//! - shell E2E: `surfaces.txt` を `areka_parsers::shell::parse` で読み、SurfaceSet として
//!   bake。全 element が索引表に載り頁が生成される。
//! - balloon E2E: balloon 画像を surface 表現に**手組み**（balloon→surface 適応は本層外）し、
//!   shell と同一の bake 経路で処理されることを実証。

use crate::test_support::{BIND_DEBUG, elem, emo2_shell, emo2_shell_dir, set, surface, with_arm};
use crate::{ManifestDeriver, PackConfig, SetId, bake};

/// emo2 shell の実測フィクスチャ事実（本統合テストが依拠する）。
///
/// `surfaces.txt` が参照する 55 の distinct element 画像は全て実在する。そのうち
/// ちょうど 1 枚 `purple/a/null.png`（382×547）は**番号ごとの透明度を持つパレットの絵**
/// （番号 1 が透明度 0）ゆえ α の道を通る。全画素が番号 1 なので**全画素が透明**になり、
/// トリム後 0 寸の空エントリ（`placement == None`）として索引表に載る。失敗ではない。
/// 残る 54 枚は α をそのまま使う。すなわち本 fixture の bake は**失敗 0 件**で、
/// 55 の manifest key が全て索引表に載る。
const SHELL_NULL_PNG_KEY: &str = "purple/a/null.png";

/// emo2 shell — 全 element が索引表に載り頁が生成される（実 fixture で行使）。
///
/// 実 `surfaces.txt` を parse → SurfaceSet として bake し、**失敗は 0 件**・全 manifest key が
/// resolve→Some・頁生成・surface0.png が配置されることを見る。
#[test]
fn emo2_shell_all_elements_baked() {
    let (surfaces, base) = emo2_shell();

    let manifest = {
        let set = set(&surfaces, &base);
        ManifestDeriver.derive(std::slice::from_ref(&set))
    };
    assert!(
        manifest
            .keys
            .iter()
            .any(|k| k.set == SetId(0) && k.rel_path == "surface0.png"),
        "manifest contains known key surface0.png"
    );
    // fixture 事実の前提: 全透明のパレットの絵の key が manifest に含まれる（含まれなければ本テストの前提が崩れる）。
    assert!(
        manifest
            .keys
            .iter()
            .any(|k| k.set == SetId(0) && k.rel_path == SHELL_NULL_PNG_KEY),
        "manifest contains the documented null.png key {SHELL_NULL_PNG_KEY}"
    );

    with_arm(|dec| {
        let set = set(&surfaces, &base);
        // 本物の焼きはこのファイルの shell だけの焼きで 1 回。記録を捕まえ、null.png の記録も同じ焼きで見る。
        let (result, out) = crate::log_capture::capture_logs(|| {
            bake(std::slice::from_ref(&set), dec, PackConfig::default())
        });

        // `purple/a/null.png`（着せ替えの「無し」の絵）は `warn` 0 件・見分けた `debug` 1 件。
        let null_lines: Vec<&str> = out
            .lines()
            .filter(|l| l.contains(&format!("rel_path=\"{SHELL_NULL_PNG_KEY}\"")))
            .collect();
        assert!(
            !null_lines.iter().any(|l| l.contains("level=WARN")),
            "{SHELL_NULL_PNG_KEY} の warn は 0 件: {out}"
        );
        let debug = null_lines
            .iter()
            .filter(|l| l.contains("level=DEBUG") && l.contains(BIND_DEBUG))
            .count();
        assert_eq!(debug, 1, "見分けた debug は 1 件: {out}");

        assert!(
            result.errors.is_empty(),
            "emo2 shell bakes with zero failures, got {:?}",
            result.errors
        );

        assert_eq!(
            result.table.len(),
            manifest.keys.len(),
            "every manifest key is present in the table (no drop-outs)"
        );

        for key in &manifest.keys {
            assert!(
                result.table.resolve(key.set, &key.rel_path).is_some(),
                "manifest key {key:?} resolves in the table"
            );
        }

        // パレットの絵の null.png は α の道を通り「全画素が透明な絵」として載る＝索引表に在り、原寸は
        // 382×547 のまま、トリム後 0 寸ゆえ placement は None。
        let null_id = result
            .table
            .resolve(SetId(0), SHELL_NULL_PNG_KEY)
            .unwrap_or_else(|| panic!("{SHELL_NULL_PNG_KEY} resolves in the table"));
        let null_entry = result.table.entry(null_id);
        assert_eq!(
            (null_entry.original.w, null_entry.original.h),
            (382, 547),
            "{SHELL_NULL_PNG_KEY} keeps its original extent"
        );
        assert!(
            null_entry.placement.is_none(),
            "{SHELL_NULL_PNG_KEY} is fully transparent via palette alpha → empty placement"
        );

        assert!(
            !result.table.pages().is_empty(),
            "at least one atlas page generated"
        );
        assert!(result.table.page(0).is_some(), "page(0) present");

        let id = result
            .table
            .resolve(SetId(0), "surface0.png")
            .expect("surface0.png resolves");
        assert!(
            result.table.entry(id).placement.is_some(),
            "surface0.png is a real non-transparent image → placed"
        );
    });
}

/// emo2 balloon — shell と同一の bake 経路で処理される。
///
/// balloon 画像を surface 表現に手組みし、shell set と **1 度の bake** で
/// 併せて処理。両 set の代表 key が別 SetId で resolve→placement を持つことで、balloon が
/// shell と同一機構で処理されることを実証。shell の側は `surface0.png` の 1 枚だけにする
/// （55 枚全部の焼きは shell のテストが見る）。
#[test]
fn emo2_balloon_same_bake_path_as_shell() {
    let shell_base = emo2_shell_dir();
    let surfaces = vec![surface(0, vec![elem(0, "surface0.png")])];
    let balloon_base = crate::sample_test_support::emo2_balloon_root();

    // 画像 1 枚を 1 element として持つ（layer0・x/y=0・collision/animation 空）。
    let balloon_surfaces = vec![
        surface(0, vec![elem(0, "balloons0.png")]),
        surface(1, vec![elem(0, "balloonk0.png")]),
    ];

    with_arm(|dec| {
        let shell_set = set(&surfaces, &shell_base);
        let balloon_set = set(&balloon_surfaces, &balloon_base);

        let result = bake(&[shell_set, balloon_set], dec, PackConfig::default());

        assert!(
            result.errors.is_empty(),
            "combined shell+balloon bake has zero failures, got {:?}",
            result.errors
        );

        let shell_id = result
            .table
            .resolve(SetId(0), "surface0.png")
            .expect("shell surface0.png resolves in combined bake");
        assert!(
            result.table.entry(shell_id).placement.is_some(),
            "shell surface0.png placed"
        );

        for rel in ["balloons0.png", "balloonk0.png"] {
            let id = result
                .table
                .resolve(SetId(1), rel)
                .unwrap_or_else(|| panic!("balloon {rel} resolves in set 1"));
            assert!(
                result.table.entry(id).placement.is_some(),
                "balloon {rel} loaded via the SAME bake path as shell → placed"
            );
        }

        assert!(
            !result.table.pages().is_empty(),
            "pages materialized for combined bake"
        );
    });
}
