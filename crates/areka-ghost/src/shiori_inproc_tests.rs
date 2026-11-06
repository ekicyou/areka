//! task 2.1 の 4 態様檻（design.md Testing Strategy・要件 3.1/3.5）:
//! 1. 正常ロード（happy path）——1.1 spike の deps-dir 解決で built cdylib を実ロード。
//! 2. DLL 欠落——不在パスで `LoadLibraryExW` 失敗。
//! 3. 不正イメージ——非 PE テキストファイルを `.dll` 名でロードし失敗。
//! 4. シンボル未解決——`shiori_factory` を持たない実 x64 DLL（kernel32.dll）で `GetProcAddress` 失敗。
//!
//! いずれの失敗態様も `Err` を返し（silent success を偽装しない・要件 3.5）、取得済み HMODULE は
//! Drop で解放される。COM は初期化しない（D-6）。
//!
//! 相対の DLL パスを読まないこと（#28）と、相対のフォルダを整えて接続できること（#29）もここで縛る。
//! テストの DLL の探し（[`built_test_dll`]）と、それを写したゴースト（[`dll_ghost`]）は
//! `shiori_inproc_adapter_tests`・`runtime_tests` もここから引く。

use std::path::PathBuf;

use temp_path_kit::TempPath;

use super::*;

/// テストの DLL（`shiori4-testdll` の cdylib）＝テストの実行ファイルと同じ deps に置かれたもの。
///
/// cdylib は `cargo test --workspace` が同じ deps ディレクトリへ置く（design.md D-1・1.1 spike）。
/// **不在は黙って飛ばさず panic**（フォールバックは設けない）。
pub(crate) fn built_test_dll() -> PathBuf {
    let test_exe = std::env::current_exe().expect("test executable path is available");
    let dll = test_exe
        .parent()
        .expect("test executable resides in a deps directory")
        .join(shiori4_testdll::DLL_FILE_NAME);
    assert!(
        dll.exists(),
        "built test DLL が正準位置に不在: {}\n\
         この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置（deps）へ出力する。\
         単独実行時は先に `cargo test --workspace`（または `cargo build -p shiori4-testdll`）を\
         実行すること（フォールバックは設けない・design.md D-1・1.1 spike）。",
        dll.display()
    );
    dll
}

/// `TempPath` の下に、テストの DLL を `ghost/master` へ写した解決できるゴーストを作る
/// （`shiori,` 行＝テストの DLL 名・`shell/master` の最小の descript）。返り値を生かしておくこと。
pub(crate) fn dll_ghost(label: &str) -> TempPath {
    let temp = TempPath::new(label);
    let ghost_master = temp.path().join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        format!(
            "charset,UTF-8\nname,Shiori4TestGhost\nshiori,{}\nseriko.defaultsurfacedirectoryname,master\n",
            shiori4_testdll::DLL_FILE_NAME
        ),
    )
    .expect("write ghost descript.txt");
    let shell_dir = temp.path().join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,Shiori4TestShell\n",
    )
    .expect("write shell descript.txt");
    std::fs::copy(
        built_test_dll(),
        ghost_master.join(shiori4_testdll::DLL_FILE_NAME),
    )
    .expect("copy built test DLL into ghost/master");
    temp
}

/// 今の作業フォルダから絶対パス `path` への相対パス（同じドライブであること）。
fn relative_from_cwd(path: &Path) -> PathBuf {
    let cwd = std::env::current_dir().expect("current dir is available");
    let from: Vec<_> = cwd.components().collect();
    let to: Vec<_> = path.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    assert!(
        common > 0,
        "作業フォルダと {} は同じドライブのはず",
        path.display()
    );
    let mut relative: PathBuf = from[common..].iter().map(|_| "..").collect();
    relative.extend(&to[common..]);
    relative
}

/// 態様2（**DLL 欠落**・`DLL欠落`）: 実在しないパスは `LoadLibraryExW` が失敗し `Err`
/// （要件 3.5）。アーティファクト不要の決定論檻。
#[test]
fn missing_dll_returns_err() {
    let result = InProcLibrary::load(Path::new(r"Z:\does\not\exist\nope.dll"));
    let err = result.err().expect("欠落 DLL は Err を返すこと");
    assert!(
        err.contains("LoadLibraryExW failed"),
        "欠落 DLL はロード失敗として顕在化すること: {err}"
    );
}

/// 態様3（**不正イメージ**・`不正イメージ`）: 非 PE テキストを `.dll` 名で置き `LoadLibraryExW` に
/// 拒否させる。一時フォルダは `TempPath`（ワークツリーの `target\` の下・破棄で消える）。
#[test]
fn invalid_image_returns_err() {
    let temp = TempPath::new("inproc-invalid-image");
    let path = temp.child("invalid.dll");
    std::fs::write(&path, b"not a real dll").expect("一時不正イメージを書き出す");

    // 窓を止めるためにスレッドのエラーモードを一時だけ変える（`load_library_quiet`）。終えたら元へ戻ること。
    // SAFETY: 引数なしの読み出しだけ。
    let mode_before = unsafe { windows::Win32::System::Diagnostics::Debug::GetThreadErrorMode() };
    let result = InProcLibrary::load(&path);
    // SAFETY: 同上。
    let mode_after = unsafe { windows::Win32::System::Diagnostics::Debug::GetThreadErrorMode() };

    assert_eq!(
        mode_before, mode_after,
        "読み込みの後、スレッドのエラーモードは元へ戻ること"
    );

    let err = result
        .err()
        .expect("不正イメージ（非 PE）は Err を返すこと");
    assert!(
        err.contains("LoadLibraryExW failed"),
        "不正イメージはロード失敗として顕在化すること: {err}"
    );
}

/// 態様4（**シンボル未解決**・`シンボル未解決`）: `kernel32.dll` は必ず存在しロードは成功するが
/// `shiori_factory` を持たない。ゆえに解決段で `GetProcAddress` 失敗＝`Err`（要件 3.5）。
/// エラー文言でロード失敗ではなく**シンボル解決失敗**であることを確認する（決定論）。
/// 相対パスは読まない（#28）ので、システムフォルダの絶対パスで渡す。
#[test]
fn unresolved_symbol_returns_err() {
    let system_root = std::env::var_os("SystemRoot").expect("SystemRoot is set on Windows");
    let kernel32 = Path::new(&system_root)
        .join("System32")
        .join("kernel32.dll");
    let result = InProcLibrary::load(&kernel32);
    let err = result
        .err()
        .expect("shiori_factory を持たない DLL は Err を返すこと");
    // ロード失敗ではなくシンボル解決失敗であることを区別する。
    assert!(
        err.contains("shiori_factory") && err.contains("unresolved"),
        "kernel32 はロード成功後 shiori_factory の GetProcAddress で失敗すること: {err}"
    );
    assert!(
        !err.contains("LoadLibraryExW failed"),
        "kernel32 のロード自体は成功する（失敗はシンボル段）: {err}"
    );
}

/// 態様1（**正常ロード**・`正常ロード`）: deps の built cdylib を実ロードし、
/// `shiori_factory` を解決して `IShioriFactory` を得る（要件 3.1）。
#[test]
fn happy_path_loads_and_resolves_factory() {
    let dll_path = built_test_dll();

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

/// #28: 今の作業フォルダから組み立てた相対パスでは DLL を読まず、同じ DLL の絶対パスでは読む
/// （`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR` は絶対パスを要る。`LoadLibraryW` へ戻すと相対で読めて赤）。
#[test]
fn a_relative_dll_path_is_refused() {
    let ghost = dll_ghost("inproc-relative-dll");
    let dll = ghost
        .path()
        .join("ghost")
        .join("master")
        .join(shiori4_testdll::DLL_FILE_NAME);
    let relative = relative_from_cwd(&dll);
    assert!(relative.is_relative(), "{}", relative.display());
    assert!(
        relative.exists(),
        "相対パスは作業フォルダからは在る: {}",
        relative.display()
    );

    let err = match InProcLibrary::load(&relative) {
        Ok((library, factory)) => {
            // 順序不変条件どおり COM 参照を先に放してから落とす。
            drop(factory);
            drop(library);
            panic!("相対の DLL パスは読まないこと: {}", relative.display());
        }
        Err(err) => err,
    };
    assert!(
        err.contains("LoadLibraryExW failed"),
        "相対パスはロード失敗になること: {err}"
    );

    let (library, factory) = InProcLibrary::load(&dll).expect("同じ DLL の絶対パスは読めること");
    drop(factory);
    drop(library);
}

/// #29: `TempPath` の下に DLL を写したゴーストを今の作業フォルダからの相対パスで `resolve` しても、
/// `inproc_connect` がフォルダを整えて（絶対・`\`・末尾 1 つ）接続できる。
/// #28 で相対の DLL パスは読めないので、整えを外すと赤。
#[test]
fn inproc_connect_accepts_a_relative_mount_folder() {
    use areka_parsers::charset::DefaultEncoding;
    use areka_parsers::package;

    let ghost = dll_ghost("inproc-relative-mount");
    let relative_root = relative_from_cwd(ghost.path());
    let mount = package::resolve_with_shell(&relative_root, DefaultEncoding::Utf8, None)
        .expect("fixture の resolve");
    assert!(
        mount.shiori.dir.is_relative(),
        "fixture は相対のフォルダのはず: {}",
        mount.shiori.dir.display()
    );

    let mut backend = inproc_connect(mount.shiori)().expect("相対のフォルダでも接続できること");
    assert!(
        matches!(backend.unload(), Ok(ExitKind::Clean)),
        "unload は Ok(ExitKind::Clean)"
    );
}
