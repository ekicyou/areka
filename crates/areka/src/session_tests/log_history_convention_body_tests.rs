//! 決まりの表の出所が本体の `src` に在ることを確かめる構造の試験。

use areka_session::log_history::RULES;

// ---------------------------------------------------------------------------
// 名指ししたモジュールの実在
// ---------------------------------------------------------------------------

/// target の先頭のクレート名から（クレートの `src`・根のファイル）を引く。知らないクレートは None。
/// 本体は相対で、下の crate は読み口で引く。
fn crate_of(name: &str) -> Option<(std::path::PathBuf, &'static str)> {
    match name {
        "areka" => Some((
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            "main.rs",
        )),
        "areka_update" => Some((
            areka_session::crate_source_test_support::crate_path("areka-update", "src"),
            "lib.rs",
        )),
        _ => None,
    }
}

/// target の在りかを判定し、無ければ理由を返す。導けない target も理由付きで返す（黙って通さない）。
fn locate(target: &str) -> Result<(), String> {
    let mut segments = target.split("::");
    let head = segments.next().unwrap_or_default();
    let Some((src, root_file)) = crate_of(head) else {
        if target.contains("::") {
            return Err(format!("{target}: クレート `{head}` のフォルダを導けない"));
        }
        // `::` を含まない明示の target はゴーストの実行時のファイルに字面がある。
        let path =
            areka_session::crate_source_test_support::crate_path("areka-ghost", "src/runtime.rs");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("{target}: {} を読めない: {e}", path.display()))?;
        let literal = format!("target: \"{target}\"");
        return if text.contains(&literal) {
            Ok(())
        } else {
            Err(format!(
                "{target}: {} に `{literal}` が無い",
                path.display()
            ))
        };
    };
    let rest: Vec<&str> = segments.collect();
    let candidates = if rest.is_empty() {
        vec![src.join(root_file)]
    } else {
        let module = rest.iter().fold(src, |p, s| p.join(s));
        vec![module.with_extension("rs"), module.join("mod.rs")]
    };
    if candidates.iter().any(|p| p.is_file()) {
        Ok(())
    } else {
        Err(format!("{target}: どれも無い {candidates:?}"))
    }
}

#[test]
fn every_rule_target_names_a_module_or_literal_that_exists() {
    let missing: Vec<String> = RULES
        .iter()
        .filter_map(|r| locate(r.target).err())
        .collect();
    assert!(
        missing.is_empty(),
        "規則の表の target がソースに無い:\n{}",
        missing.join("\n")
    );
}

#[test]
fn locate_turns_red_for_missing_and_underivable_targets() {
    // 判定が何でも通す形になっていないことの較正（当たりは同じ表の実物）。
    assert_eq!(locate("areka"), Ok(()));
    assert_eq!(locate("areka::update"), Ok(()));
    assert_eq!(locate("areka_update::fetch"), Ok(()));
    assert_eq!(locate("ghost-boot"), Ok(()));
    for bogus in [
        "areka::no_such_module",
        "areka_update::no_such_module",
        "no-such-literal-target",
        "no_such_crate::x",
    ] {
        assert!(locate(bogus).is_err(), "{bogus} は赤");
    }
}
