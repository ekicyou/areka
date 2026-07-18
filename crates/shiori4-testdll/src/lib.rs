//! x64 SHIORI4 決定論テスト DLL（`shiori4-testdll`）の crate ルート。
//!
//! 本 crate は areka の「実 SHIORI 境界を踏む決定論テストゴースト」の**脳**を担う cdylib であり、
//! 正典イベント集合へ実 emo2 pasta 採取のゴールデンスナップショットを決定論 replay する
//! （design.md "ReplayBrain・ReplayFactory・shiori_factory export"）。本ファイルは task 1.1 の
//! **crate 雛形（scaffold）**であり、以下を確定する:
//!
//! - 契約定数 [`DLL_FILE_NAME`]（出力 DLL ファイル名の単一権威・fixture descript の `shiori,` 行と
//!   e2e の locate/コピーが共有する・design.md §SnapshotTable Service Interface）。
//! - 生成入口 [`shiori_factory`]（production SHIORI4 と同形の `extern "system"` エクスポート ABI 形状・
//!   要件 2.1）。本 task では **scaffold stub**（本実装の `ReplayFactory` は task 1.4 で導入）。
//! - `cargo test --workspace` 実行後、ビルド成果物 cdylib が現れる**単一の正準位置**を実測固定する
//!   in-crate spike テスト（要件 1.2・5.4／design.md D-1「単一正準位置・フォールバックなし」）。
//!   **実測結果（task 1.1 spike）**: 正準位置は `target/<profile>/deps/shiori4_testdll.dll`
//!   （= `current_exe()` と同一ディレクトリ）。詳細な実測根拠は spike テスト本体の doc を参照。
//!
//! ## 依存境界（design.md "Allowed Dependencies"）
//! 本 crate は [`shiori-abi`](shiori_abi) ＋ [`windows-core`](windows_core) のみに依存する。
//! `windows`(Win32)・`tracing`・areka 系 crate へは依存しない（自給・D-2）。task 1.4 で
//! `ReplayBrain`/`ReplayFactory` を `shiori_abi::interface` の COM 面へ実装する際に
//! shiori-abi を実消費する（scaffold stub の本 task では shiori-abi の COM 面は未消費）。

use core::ffi::c_void;

use windows_core::HRESULT;

/// SHIORI/3.0 リクエスト解析・応答分類の純粋ロジック（task 1.2）。
///
/// `parse_request`（request line の別＋`ID:` 抽出・design.md D-4）と `select_response`
/// （収載→凍結応答／未知→204／構造不整合→400・要件 2.2/2.3/2.4）を提供する。項目は
/// crate 内消費（`pub(crate)`）で、task 1.4 の `ReplayBrain::Get` から呼ぶ（本 task では
/// `shiori_factory` へは未結線）。
mod request;

/// 正典 GET イベントごとの凍結ゴールデンスナップショットの静的表（task 1.3）。
///
/// `snapshots/<EventID>.txt`（SHIORI/3.0 応答全文）を `include_str!` でコンパイル時に埋め込み
/// （実行時 I/O ゼロ・要件 1.5）、取り込み時に CRLF 正規化する（git EOL 変換への免疫・要件 2.5／
/// research.md §7.3）。`snapshot_for(id)` は task 1.4 の `ReplayBrain::Get` が `select_response` の
/// `lookup` クロージャへ無改修で差し込む（本 task では `shiori_factory` へ未結線）。
mod snapshot;

/// 出力 DLL ファイル名（design.md §SnapshotTable Service Interface の契約定数・単一権威）。
///
/// この値は次の 2 箇所が共有する契約値である:
/// - テストゴースト fixture の `ghost/master/descript.txt` の `shiori,` 行（D-8）。
/// - e2e の `locate_built_test_dll()`／fixture への DLL コピー（要件 1.2・5.4）。
///
/// D-8 で確定した命名（crate `shiori4-testdll`・`[lib] name = "shiori4_testdll"`）の帰結として、
/// x64 ネイティブビルドの cdylib 出力ファイル名は `shiori4_testdll.dll` になる。
pub const DLL_FILE_NAME: &str = "shiori4_testdll.dll";

/// `E_POINTER`（NULL/無効ポインタ）の HRESULT。
///
/// Win32 の `E_POINTER = 0x8000_4003`。本 crate は `windows`(Win32) へ依存しない
/// （design.md "Allowed Dependencies"）ため、[`windows_core::HRESULT`] のローカル定数として保持する。
const E_POINTER: HRESULT = HRESULT(0x8000_4003u32 as i32);

/// `E_NOTIMPL`（未実装）の HRESULT。
///
/// Win32 の `E_NOTIMPL = 0x8000_4001`。scaffold stub の [`shiori_factory`] は本実装（task 1.4 の
/// `ReplayFactory`）未導入を fail-visible に示すため、非 NULL `out` に対して本コードを返す
/// （`out` へは何も書き込まない＝半構築非露出）。
const E_NOTIMPL: HRESULT = HRESULT(0x8000_4001u32 as i32);

/// `IShioriFactory` 生成入口の scaffold stub（本実装は task 1.4 の `ReplayFactory` が置き換える）。
///
/// production SHIORI4 生成入口（`crates/areka/src/reference_brain.rs::shiori_factory`）と同形の
/// エクスポート ABI 形状を確定する（要件 2.1）: `#[unsafe(no_mangle)]`＋`extern "system"`（Windows COM
/// 標準の呼出規約・x64/ARM64 では `extern "C"` と同一 ABI だが COM 整合で正準表記は `system`）。
/// 将来 host-32／InProc 経路が `GetProcAddress("shiori_factory")` で引ける形を満たす。
///
/// 本 task（1.1）は crate 雛形につき、生成入口は非 panic の scaffold stub とする:
/// - `out` が NULL の場合: 防御的に [`E_POINTER`] を返し、何も書き込まない。
/// - それ以外: 本実装未導入を示す [`E_NOTIMPL`] を返し、`out` へは何も書き込まない（半構築非露出）。
///
/// task 1.4 で `ReplayFactory::into()`→`into_raw()` を `*out` へ move-out し `S_OK` を返す本実装へ
/// 差し替える（reference_brain の `shiori_factory` と同律）。
///
/// # Safety
/// `out` は非 NULL の有効な書込先ポインタであること（呼び出し側が保証）。`out` が NULL の場合は
/// 防御的に [`E_POINTER`] を返し、何も書き込まない。本 stub は成功コードを返さないため、
/// 現時点で `out` に所有権が move-out されることはない。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn shiori_factory(out: *mut *mut c_void) -> HRESULT {
    // 前提の防御: out が NULL なら何も書き込まず判別可能な失敗を返す（reference_brain と同律）。
    if out.is_null() {
        return E_POINTER;
    }
    // scaffold stub: 本実装（task 1.4 の ReplayFactory）未導入を fail-visible に示す。
    // out へは書き込まない（半構築非露出）。
    E_NOTIMPL
}

#[cfg(test)]
mod tests {
    //! crate 雛形の in-crate 檻。
    //!
    //! 中核は **uplift spike**（`built_cdylib_appears_at_single_canonical_deps_dir`）で、
    //! `cargo test` がビルドする cdylib が現れる単一の正準位置を実測固定する
    //! （要件 1.2・5.4／design.md D-1）。併せて scaffold stub の生成入口 ABI 形状を存在チェックする。

    use super::*;
    use std::path::PathBuf;

    /// **実測固定（uplift spike・task 1.1）**: `cargo test`（`--workspace` 含む）のビルド成果物 cdylib が
    /// 現れる**単一の正準位置**は `target/<profile>/deps/shiori4_testdll.dll`
    /// （= `current_exe()` と同一ディレクトリ）である（要件 1.2・5.4／design.md D-1）。
    ///
    /// **実測根拠（本 spike が確定させた事実）**:
    /// cargo は cdylib を必ず `target/<profile>/deps/` にリンク出力する。`target/<profile>/`（deps の親）
    /// への「uplift（トップレベル複製）」は `cargo build`（primary ビルド）でのみ起き、
    /// `cargo test` / `cargo test --workspace` では**起きない**（`--no-run` 実測で確認）。したがって
    /// 常設ゲート `cargo test --workspace` が確実に生成する単一の正準位置は deps ディレクトリ自身であり、
    /// design.md D-1 の散文が推定した `target/<profile>/`（deps を pop）ではない——D-1 が命じた
    /// 「実装先頭タスクでの uplift 実証 spike で確定した単一の正準位置のみを locate する」に従い、
    /// spike の実測が prose の推定を上書きする（正準位置＝deps・単一・フォールバックなし）。
    ///
    /// 導出は `current_exe()` から決定論的に行う: テストバイナリは
    /// `target/<profile>/deps/<name>-<hash>.exe` に置かれるため、その**親ディレクトリ（deps）**へ
    /// [`DLL_FILE_NAME`] を join した位置が正準 cdylib である（cdylib 出力はハッシュ接尾なしの素名
    /// `shiori4_testdll.dll` 固定）。areka-ghost tests の e2e もこの同一 deps を共有するため、
    /// `locate_built_test_dll()`（task 3 系）はこの導出を逐語再利用できる。
    ///
    /// glob／mtime／`target/<profile>/` へのフォールバックは採らない（design.md D-1・設計討議#1
    /// 「単一正準位置・フォールバックなし」——将来 cargo 挙動変化時に古い DLL を拾って壊れたビルドを
    /// 隠蔽する silent green を防ぐため。挙動変化は下記 assert の明示 panic で即座に顕在化させる）。
    #[test]
    fn built_cdylib_appears_at_single_canonical_deps_dir() {
        // current_exe = target/<profile>/deps/<name>-<hash>.exe
        let test_exe = std::env::current_exe().expect("test executable path is available");

        // 親ディレクトリ = target/<profile>/deps（cdylib もここへ出力される・実測固定の正準ディレクトリ）。
        let deps_dir: PathBuf = test_exe
            .parent()
            .expect("test executable resides in a deps directory")
            .to_path_buf();
        assert_eq!(
            deps_dir.file_name().and_then(|s| s.to_str()),
            Some("deps"),
            "cargo のテストバイナリ配置（target/<profile>/deps/...）を前提とする。\
             layout が変化した場合は本 spike と locate 導出を再実証すること（design.md D-1）。\
             observed test_exe = {}",
            test_exe.display()
        );

        // 正準位置 = target/<profile>/deps/shiori4_testdll.dll（契約定数を rlib 面から参照）。
        let canonical_dll = deps_dir.join(DLL_FILE_NAME);

        assert!(
            canonical_dll.exists(),
            "ビルド成果物 cdylib が正準位置に不在: {}\n\
             この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置へ出力する。\
             単独実行時は先に `cargo build -p shiori4-testdll` を実行すること（フォールバックは設けない・\
             design.md D-1）。",
            canonical_dll.display()
        );
    }

    /// scaffold stub の生成入口 ABI 形状の存在チェック（要件 2.1）。
    ///
    /// `shiori_factory` が `extern "system" fn(*mut *mut c_void) -> HRESULT` 形状でリンク可能であり、
    /// NULL `out` に対して [`E_POINTER`] を返す防御が働くことを確認する（reference_brain と同律）。
    /// 本実装（成功時 move-out・`S_OK`）は task 1.4 で導入するため、本 task では NULL 防御のみ実証する。
    #[test]
    fn scaffold_shiori_factory_export_has_expected_abi_shape_and_defends_null_out() {
        // NULL out → E_POINTER（何も書き込まない）。
        let hr = unsafe { shiori_factory(core::ptr::null_mut()) };
        assert_eq!(
            hr, E_POINTER,
            "NULL out に対し scaffold stub は E_POINTER を返すこと, got 0x{:08X}",
            hr.0
        );

        // 非 NULL out → scaffold stub は本実装未導入を示す E_NOTIMPL を返し、out を書き換えない。
        let mut out: *mut c_void = core::ptr::null_mut();
        let hr = unsafe { shiori_factory(&mut out) };
        assert_eq!(
            hr, E_NOTIMPL,
            "非 NULL out に対し scaffold stub は E_NOTIMPL を返すこと（本実装は task 1.4）, got 0x{:08X}",
            hr.0
        );
        assert!(
            out.is_null(),
            "scaffold stub は out へ何も書き込まないこと（半構築非露出）"
        );
    }

    /// 契約定数 [`DLL_FILE_NAME`] が D-8 の命名（`shiori4_testdll.dll`）に固定されていること（回帰防止）。
    #[test]
    fn dll_file_name_is_fixed_to_d8_naming() {
        assert_eq!(
            DLL_FILE_NAME, "shiori4_testdll.dll",
            "出力 DLL ファイル名は D-8 命名に固定されていること"
        );
    }
}
