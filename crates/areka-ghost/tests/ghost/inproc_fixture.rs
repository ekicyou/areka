//! inproc_fixture.rs — InProc 実 DLL 駆動のテストゴースト組立と成果物 DLL locate（テスト支援）。
//!
//! 本ファイルの LOCATE 部（task 4.1）は、`cargo test --workspace` がビルドした x64 cdylib
//! `shiori4_testdll.dll` の**単一の正準位置**を決定論的に特定する [`locate_built_test_dll`] を提供する。
//! 組立部（`assemble_test_ghost`・task 4.2）は同ファイルへ後続タスクが追加し、consume は tasks 5.x。
//!
//! # 単一正準位置・フォールバックなし（要件 1.2・5.4／design.md D-1）
//! 正準位置は `target/<profile>/deps/shiori4_testdll.dll`（= テストバイナリ `current_exe()` と同一
//! ディレクトリ）**ただ 1 箇所**である。これは task 1.1 の uplift spike が実測で確定した事実で、
//! `cargo test`/`cargo test --workspace` は cdylib を deps へリンクするが `target/<profile>/`（deps の
//! 親）への uplift は起こさない（design.md D-1 の散文が推定した「deps を pop」は実測で誤り＝spike が
//! 上書きする）。glob／mtime／多段フォールバックは**採らない**——将来 cargo 挙動変化時に古い deps/ の
//! DLL を拾って壊れたビルドを隠蔽する silent green を防ぐため（fail-visible・設計討議#1）。挙動変化は
//! 不在時の明示 panic で即座に顕在化させる。
//!
//! 本モジュールの一部の pub 項目は、consume する後続タスク（4.2 組立・5.x e2e）が結線されるまで
//! この test binary 内で未使用になり得るため、dead-code 警告を抑止する。
#![allow(dead_code)]

use std::path::PathBuf;

/// `cargo test --workspace` がビルドした x64 cdylib `shiori4_testdll.dll` の**単一の正準位置**を
/// 決定論的に特定して返す（要件 1.2・5.4／design.md D-1）。
///
/// 正準位置は `target/<profile>/deps/shiori4_testdll.dll`（= テストバイナリ `current_exe()` と同一
/// ディレクトリ）**ただ 1 箇所**である。導出はテストバイナリの `current_exe()` の親ディレクトリ（deps）へ
/// 契約定数 [`shiori4_testdll::DLL_FILE_NAME`] を join するだけで、`deps` を pop しない
/// （task 1.1 uplift spike の実測に忠実・design.md D-1 散文の「deps を pop」推定を spike が上書き）。
///
/// # 単一正準位置・フォールバックなし（design.md D-1・設計討議#1）
/// glob／mtime／`target/<profile>/`（deps の親）への多段フォールバックは**採らない**。将来 cargo の
/// レイアウトが変わって古い deps/ の DLL が残った場合、フォールバック glob はその陳腐 DLL を拾って
/// 壊れたビルドを silent green で隠蔽し得る——単一位置固定＋不在時の明示 panic なら、レイアウト変化を
/// fail-visible に即座に顕在化できる（`cargo test --workspace` の workspace-artifact 前提と同律）。
///
/// # Panics
/// 正準位置に DLL が不在の場合、次の一手を示す明示的なメッセージで panic する（silent skip はしない・
/// 非在パスも返さない）。この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置へ出力
/// するため、単独実行時は先に `cargo build -p shiori4-testdll` を実行すること。
pub fn locate_built_test_dll() -> PathBuf {
    // current_exe = target/<profile>/deps/<name>-<hash>.exe（テストバイナリ）。
    let test_exe = std::env::current_exe().expect("test executable path is available");

    // 親ディレクトリ = target/<profile>/deps（cdylib もここへ出力される・唯一の正準ディレクトリ）。
    // deps を pop しない（task 1.1 spike 実測）。
    let deps_dir = test_exe
        .parent()
        .expect("test executable resides in a deps directory");

    // 正準位置 = target/<profile>/deps/shiori4_testdll.dll（契約定数を testdll crate から参照）。
    let canonical_dll = deps_dir.join(shiori4_testdll::DLL_FILE_NAME);

    assert!(
        canonical_dll.exists(),
        "ビルド済みテスト DLL が正準位置に不在: {}\n\
         この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置へ出力する。\
         単独実行時は先に `cargo build -p shiori4-testdll` を実行すること（フォールバックは設けない・\
         design.md D-1）。",
        canonical_dll.display()
    );

    canonical_dll
}

/// このテストゴーストの LOCATE 檻。
mod tests {
    /// `locate_built_test_dll()` は (a) 実在し (b) ファイル名が `shiori4_testdll.dll`
    /// （== [`shiori4_testdll::DLL_FILE_NAME`]）で (c) 親ディレクトリ名が `deps` のパスを返すこと
    /// （要件 1.2・5.4／design.md D-1「単一正準位置・フォールバックなし」）。
    #[test]
    fn locate_returns_existing_canonical_deps_dir_dll() {
        let dll = super::locate_built_test_dll();

        // (a) 実在（不在なら locate 自身が明示 panic するので、成功戻り値は必ず実在）。
        assert!(dll.exists(), "locate は実在するパスを返すこと: {}", dll.display());

        // (b) ファイル名 == 契約定数 DLL_FILE_NAME。
        assert_eq!(
            dll.file_name().and_then(|s| s.to_str()),
            Some(shiori4_testdll::DLL_FILE_NAME),
            "locate 戻り値のファイル名は契約定数と一致すること: {}",
            dll.display()
        );

        // (c) 親ディレクトリ名 == "deps"（単一正準位置＝deps・design.md D-1）。
        assert_eq!(
            dll.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()),
            Some("deps"),
            "locate 戻り値の親ディレクトリは deps であること: {}",
            dll.display()
        );
    }

    /// locate 戻り値は絶対パスで `target` ツリー内を指すこと（正準位置の健全性・design.md D-1）。
    #[test]
    fn locate_returns_absolute_path_inside_target_tree() {
        let dll = super::locate_built_test_dll();
        assert!(dll.is_absolute(), "locate 戻り値は絶対パスであること: {}", dll.display());
        assert!(
            dll.components().any(|c| c.as_os_str() == "target"),
            "locate 戻り値は target ツリー内を指すこと: {}",
            dll.display()
        );
    }
}
