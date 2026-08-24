//! task 2.1 の 4 態様檻（design.md Testing Strategy・要件 3.1/3.5）:
//! 1. 正常ロード（happy path）——1.1 spike の deps-dir 解決で built cdylib を実ロード。
//! 2. DLL 欠落——不在パスで `LoadLibraryW` 失敗。
//! 3. 不正イメージ——非 PE テキストファイルを `.dll` 名でロードし失敗。
//! 4. シンボル未解決——`shiori_factory` を持たない実 x64 DLL（kernel32.dll）で `GetProcAddress` 失敗。
//!
//! いずれの失敗態様も `Err` を返し（silent success を偽装しない・要件 3.5）、取得済み HMODULE は
//! Drop で解放される。COM は初期化しない（D-6）。

use super::*;

/// 態様2（**DLL 欠落**・`DLL欠落`）: 実在しないパスは `LoadLibraryW` が失敗し `Err`
/// （要件 3.5）。アーティファクト不要の決定論檻。
#[test]
fn missing_dll_returns_err() {
    let result = InProcLibrary::load(Path::new(r"Z:\does\not\exist\nope.dll"));
    let err = result.err().expect("欠落 DLL は Err を返すこと");
    assert!(
        err.contains("LoadLibraryW failed"),
        "欠落 DLL はロード失敗として顕在化すること: {err}"
    );
}

/// 態様3（**不正イメージ**・`不正イメージ`）: 非 PE テキストを `.dll` 名で置き `LoadLibraryW` に
/// 拒否させる。一意な一時ファイルを使い、檻の後始末で削除する（決定論・要件 3.5）。
#[test]
fn invalid_image_returns_err() {
    // 一意な一時 .dll パス（プロセス id＋nanos で衝突回避）。
    let unique = format!(
        "areka_inproc_invalid_{}_{}.dll",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let path = std::env::temp_dir().join(&unique);
    std::fs::write(&path, b"not a real dll").expect("一時不正イメージを書き出す");

    let result = InProcLibrary::load(&path);

    // 後始末（best-effort）: assert より先に一時ファイルを掃除する。
    let _ = std::fs::remove_file(&path);

    let err = result
        .err()
        .expect("不正イメージ（非 PE）は Err を返すこと");
    assert!(
        err.contains("LoadLibraryW failed"),
        "不正イメージはロード失敗として顕在化すること: {err}"
    );
}

/// 態様4（**シンボル未解決**・`シンボル未解決`）: `kernel32.dll` は必ず存在し `LoadLibraryW` は
/// 成功するが `shiori_factory` を持たない。ゆえに解決段で `GetProcAddress` 失敗＝`Err`（要件 3.5）。
/// エラー文言でロード失敗ではなく**シンボル解決失敗**であることを確認する（決定論）。
#[test]
fn unresolved_symbol_returns_err() {
    // 名前ロード（system DLL はプロセス常駐・検索パスで解決）。
    let result = InProcLibrary::load(Path::new("kernel32.dll"));
    let err = result
        .err()
        .expect("shiori_factory を持たない DLL は Err を返すこと");
    // ロード失敗ではなくシンボル解決失敗であることを区別する。
    assert!(
        err.contains("shiori_factory") && err.contains("unresolved"),
        "kernel32 は LoadLibraryW 成功後 shiori_factory の GetProcAddress で失敗すること: {err}"
    );
    assert!(
        !err.contains("LoadLibraryW failed"),
        "kernel32 のロード自体は成功する（失敗はシンボル段）: {err}"
    );
}

/// 態様1（**正常ロード**・`正常ロード`）: 1.1 spike の deps-dir 解決で built cdylib を実ロードし、
/// `shiori_factory` を解決して `IShioriFactory` を得る（要件 3.1）。
///
/// cdylib は `cargo test --workspace` が同一 deps ディレクトリへ uplift する
/// （`current_exe().parent().join(DLL_FILE_NAME)`・1.1 spike 実測／design.md D-1・tasks.md
/// Implementation Notes）。deps を pop しない。**不在時は silent skip せず明示 panic**（要件 3.1／
/// design.md D-1・workspace-artifact 先例）。
#[test]
fn happy_path_loads_and_resolves_factory() {
    let test_exe = std::env::current_exe().expect("test executable path is available");
    let deps_dir = test_exe
        .parent()
        .expect("test executable resides in a deps directory");
    let dll_path = deps_dir.join(shiori4_testdll::DLL_FILE_NAME);

    assert!(
        dll_path.exists(),
        "built test DLL が正準位置に不在: {}\n\
         この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置（deps）へ出力する。\
         単独実行時は先に `cargo test --workspace`（または `cargo build -p shiori4-testdll`）を\
         実行すること（フォールバックは設けない・design.md D-1・1.1 spike）。",
        dll_path.display()
    );

    let (library, factory) = InProcLibrary::load(&dll_path)
        .expect("built cdylib は正常ロードされ factory を解決すること");

    // 最小の生存確認: 生成した factory を IUnknown へ cast できる（＝有効な COM 参照）。
    let _unknown: windows::core::IUnknown = factory
        .cast()
        .expect("IShioriFactory は IUnknown へ cast 可能な有効 COM 参照であること");

    // FreeLibrary 順序不変条件（モジュール doc）に従い、COM 参照（factory）を先に、
    // ロード済みライブラリを後に解放する。
    drop(_unknown);
    drop(factory);
    drop(library);
}
