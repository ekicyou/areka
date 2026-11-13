//! `cargo test` がビルドしたこの crate の cdylib（[`DLL_FILE_NAME`]）の場所を、テストの実行ファイルの
//! 場所から決める唯一の口（areka-ghost の lib の試験と tests/ghost の e2e が共有する）。
//!
//! # 2 つの置き方
//! - 古い置き方（cargo 1.99 まで）: テストの実行ファイルも cdylib も `target/<profile>/deps/` に並ぶ。
//!   実行ファイルの親が `deps` なら、その隣の [`DLL_FILE_NAME`] だけを見る（従来の正準位置）。
//! - 新しい置き方（cargo 1.100 から）: 成果物ごとに `target/<profile>/build/<pkg>/<hash>/out/` へ出る。
//!   cdylib は `build/shiori4-testdll/<hash>/out/` にあり、`<hash>` はツールチェーン・機能の組・
//!   cargo の呼び方（`-p` の組）が変わるたびに増え、古いものも残る。
//!
//! # 新しい置き方での決め方（2026-11-13 開発者裁定 — design.md D-1「単一正準位置・フォールバック
//! なし」を改める）
//! 走っているテストの実行ファイルの cargo の記録（`<hash>/fingerprint/*.json` の `deps`）には、
//! リンクした `shiori4_testdll` の rlib の指紋が載る。rlib と cdylib は同じビルドの単位で同じ
//! `<hash>/out/` に出るので、`build/shiori4-testdll/*/fingerprint/lib-shiori4_testdll` がその指紋と
//! 一致する `<hash>` を探し、**ちょうど 1 つ**ならその `out/` の cdylib を使う。0 個・2 個以上は明示の
//! panic で止める。mtime で選ばず、テストがリンクしたのと同じビルドの DLL だけを使うので、古い DLL を
//! 黙って拾って壊れたビルドを緑に見せることはない（D-1 の目的は保つ）。cargo の記録の形が変わったときも
//! 一致 0 個で止まる（fail-visible）。
//!
//! 同じツールチェーンでも `cargo test --workspace` と `cargo test -p …` では機能の組が違って `<hash>` が
//! 2 つできるため、ツールチェーンの値で絞るだけでは 1 つに決まらない（実測）。そのため指紋で突き合わせる。

use std::path::{Path, PathBuf};

use crate::DLL_FILE_NAME;

/// 新しい置き方で cdylib が置かれる `build/` の下のパッケージのフォルダ名。
const PACKAGE_DIR: &str = "shiori4-testdll";
/// cargo の記録の `deps` に載る、この crate のライブラリの名前。
const LIB_NAME: &str = "shiori4_testdll";
/// ビルドの単位の指紋（16 桁の hex）を書いた cargo の記録のファイル名。
const LIB_FINGERPRINT: &str = "lib-shiori4_testdll";

/// cargo test がビルドした cdylib の場所を返す。見つからない・決められないときは理由つきで panic する。
///
/// この crate に依存するテストの実行ファイルから呼ぶこと（この crate 自身の試験からは指紋を辿れない）。
/// 単独実行で cdylib がまだ無いときは、先に `cargo test --workspace` を回すこと。
pub fn locate_built_dll() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path is available");
    let dir = exe
        .parent()
        .expect("test executable resides in a directory");
    if dir.file_name().is_some_and(|n| n == "deps") {
        let dll = dir.join(DLL_FILE_NAME);
        assert!(
            dll.exists(),
            "ビルド済みテスト DLL が正準位置に不在: {}\n単独実行時は先に `cargo build -p shiori4-testdll` を実行すること。",
            dll.display()
        );
        return dll;
    }
    // 新しい置き方: <profile>/build/<pkg>/<hash>/out/<exe>
    let unit = dir.parent().expect("out の親（<hash>）がある");
    let build = unit
        .parent()
        .and_then(Path::parent)
        .expect("<hash> の 2 つ上（build）がある");
    let linked = jsons_in(&unit.join("fingerprint"))
        .iter()
        .find_map(|json| linked_fingerprint(json))
        .unwrap_or_else(|| {
            panic!(
                "テストの実行ファイルの cargo の記録に {LIB_NAME} の指紋が無い: {}",
                unit.join("fingerprint").display()
            )
        });
    let candidates = std::fs::read_dir(build.join(PACKAGE_DIR))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .map(|hash| {
                    let fp =
                        std::fs::read_to_string(hash.join("fingerprint").join(LIB_FINGERPRINT))
                            .ok()
                            .map(|s| s.trim().to_owned());
                    (hash.join("out").join(DLL_FILE_NAME), fp)
                })
                .filter(|(dll, _)| dll.exists())
                .collect()
        })
        .unwrap_or_default();
    pick(&linked, candidates).unwrap_or_else(|why| panic!("{why}"))
}

/// フォルダの中の `*.json` の中身（読めないものは飛ばす）。
fn jsons_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .filter_map(|e| std::fs::read_to_string(e.path()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// 記録の `deps` の `[…,"shiori4_testdll",<bool>,<u64>]` の `<u64>` を、指紋のファイルと同じ形
/// （8 バイトを小さい桁から並べた 16 桁の hex）にして返す。
fn linked_fingerprint(json: &str) -> Option<String> {
    let key = format!("\"{LIB_NAME}\",");
    let rest = &json[json.find(&key)? + key.len()..];
    let rest = &rest[rest.find(',')? + 1..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let n: u64 = digits.parse().ok()?;
    Some(n.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect())
}

/// 指紋が一致する候補がちょうど 1 つならそれを返す。そうでなければ理由の文。
fn pick(linked: &str, candidates: Vec<(PathBuf, Option<String>)>) -> Result<PathBuf, String> {
    let all: Vec<String> = candidates
        .iter()
        .map(|(p, f)| format!("{} ({})", p.display(), f.as_deref().unwrap_or("?")))
        .collect();
    let mut same: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|(_, f)| f.as_deref() == Some(linked))
        .map(|(p, _)| p)
        .collect();
    match same.len() {
        1 => Ok(same.remove(0)),
        0 => Err(format!(
            "テストがリンクした {LIB_NAME} (指紋 {linked}) と同じビルドのテスト DLL が無い。候補: {all:?}\n\
             単独実行時は先に `cargo test --workspace` を実行すること。"
        )),
        n => Err(format!(
            "テストがリンクした {LIB_NAME} (指紋 {linked}) と同じビルドのテスト DLL が {n} 個あり、1 つに決められない: {all:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(path: &str, fp: Option<&str>) -> (PathBuf, Option<String>) {
        (PathBuf::from(path), fp.map(str::to_owned))
    }

    /// cargo 1.100 の実物の記録の形（数は 11-13 の実測）：deps の u64 は指紋のファイルの hex を小さい桁から読んだもの。
    #[test]
    fn linked_fingerprint_matches_the_fingerprint_file_form() {
        let json = r#"{"rustc":1,"deps":[[3713217444090848635,"shiori_abi",false,1],[10308109417370294510,"shiori4_testdll",false,1663381338026695859]]}"#;
        assert_eq!(
            linked_fingerprint(json).as_deref(),
            Some("b35c189383841517")
        );
        assert_eq!(linked_fingerprint(r#"{"deps":[]}"#), None);
    }

    #[test]
    fn pick_takes_the_one_with_the_linked_fingerprint() {
        let got = pick(
            "aa",
            vec![
                c("old/out/a.dll", Some("bb")),
                c("new/out/a.dll", Some("aa")),
                c("bad/out/a.dll", None),
            ],
        );
        assert_eq!(got, Ok(PathBuf::from("new/out/a.dll")));
    }

    #[test]
    fn pick_refuses_none_or_several() {
        assert!(pick("aa", vec![c("old/out/a.dll", Some("bb"))]).is_err());
        assert!(pick("aa", vec![]).is_err());
        assert!(
            pick(
                "aa",
                vec![c("x/out/a.dll", Some("aa")), c("y/out/a.dll", Some("aa"))]
            )
            .is_err()
        );
    }
}
