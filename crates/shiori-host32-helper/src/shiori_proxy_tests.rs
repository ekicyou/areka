//! Task 5.2: プロキシの i686 単体テスト（design §387 Validation・Testing Strategy Unit #6・
//! 確定設計判断 (d)）。4 節立て:
//! 1. 純関数部（コードページ符号化）の決定的検証。
//! 2. `kernel32.dll` への load で `EntryNotFound` を決定的に証明（エクスポート欠落態様・R6.2）。
//! 3. testdll を直接 `load`→`drop` し、courtesy unload の実呼出（観測マーカー）と無 panic を確認
//!    （R2.2・E2E では Drop 経路が実行されない補完）。あわせて、別のフォルダから読んだ写しの
//!    unload がその印を書かないことを確認する。
//! 4. testdll 越しの `request` の往復（GET・NOTIFY）。
//!
//! 偽の DLL を load するテストを足すときの決まりは、錠 `TESTDLL_SERIAL` の説明にある。

use super::*;
use crate::proxy_test_support::{TestRoot, copy_testdll_into_unique_target_dir};
use std::path::Path;
use std::sync::Mutex;

/// 印の環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` を差すテスト同士の直列化。
///
/// 環境変数はプロセス全体で 1 つなので、差すテスト同士はこの錠で直列にする。偽の DLL の unload は、
/// 環境変数に値があり、印の親フォルダが自分の置き場と同じときだけ印を書く。汚れた錠は無視して続ける。
///
/// 偽の DLL `shiori.dll` を load するテストを足すときの決まり:
/// 1. 偽の DLL `shiori.dll` を load するテストは、自分だけのフォルダへ写してから絶対パスで読む。
/// 2. 印の環境変数を差さないテストは、錠を取らなくてよい。そのテストの unload は、他のテストの印を
///    書かない（偽の DLL が、自分の置き場と同じフォルダの印だけを書くため）。
/// 3. 印の環境変数を差すテストは、この錠を取る。印のパスは、読む DLL と同じフォルダに置く。
/// 4. 今、偽の DLL を load するテストは、`testdll_drop_invokes_courtesy_unload`・
///    `testdll_unload_from_another_folder_leaves_marker_untouched`・
///    `testdll_request_roundtrip_get_and_notify`・`a_relative_dll_path_is_refused`・
///    `close_reports_unload_returning_false`・`main_loopback_tests.rs` の loopback のテスト。
///    うち環境変数を差すのは `testdll_drop_invokes_courtesy_unload` と
///    `testdll_unload_from_another_folder_leaves_marker_untouched`。
static TESTDLL_SERIAL: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------
// 1. 純関数部の検証: コードページ符号化
// ---------------------------------------------------------------------

/// ASCII パスは CP_ACP でも同一バイト列に符号化される（NUL 終端は付けない＝正味長）。
/// 環境依存の少ない ASCII で決定的に検証する。
#[test]
fn ansi_encode_ascii_is_identity_no_nul() {
    let p = Path::new(r"C:\ghost\master");
    let bytes = ansi_encode(p).expect("ASCII path must encode");
    // ASCII は CP_ACP でそのままの 1 バイト列。NUL 終端規約: ansi_encode は終端を付けない。
    assert_eq!(bytes, br"C:\ghost\master".to_vec());
    // 末尾に NUL が付いていないこと（正味長）。
    assert_ne!(bytes.last(), Some(&0u8));
    assert_eq!(bytes.len(), r"C:\ghost\master".len());
}

/// 空パスは空バイト列（防御的許容・確保 0 バイトも有効）。
#[test]
fn ansi_encode_empty_is_empty() {
    let bytes = ansi_encode(Path::new("")).expect("empty path encodes to empty");
    assert!(bytes.is_empty());
}

/// 日本語混じりは Shift_JIS（コードページ 932）の複数バイトへ符号化され、ASCII 部と混在する。
/// 機械の既定コードページ（CP_ACP）に依らないよう、932 を明示して渡す。
#[test]
fn mixed_japanese_encodes_to_shift_jis_multibyte() {
    let p = Path::new(r"C:\ゴースト");
    let encoded = encode_with_codepage(932, p).expect("mixed path must encode without panic");
    assert_eq!(
        encoded.bytes,
        b"C:\\\x83\x53\x81\x5B\x83\x58\x83\x67".to_vec()
    );
    assert!(!encoded.lossy);
}

// ---------------------------------------------------------------------
// 2. kernel32.dll で EntryNotFound（R6.2・設計判断 (d)）
// ---------------------------------------------------------------------

/// `kernel32.dll` は必ず存在し LoadLibraryExW は成功するが、`load`/`unload`/`request` の
/// SHIORI エクスポートを持たない。ゆえに `ShioriByteProxy::load` は解決段で
/// `ProxyError::EntryNotFound` を**決定的**に返す（エクスポート欠落態様の証明・R6.2）。
/// 半構築を残さないため内部で FreeLibrary 済み（Err = 状態残さず）。
#[test]
fn kernel32_yields_entry_not_found() {
    let dll = Path::new(r"C:\Windows\System32\kernel32.dll");
    let root = TestRoot::new("k32"); // 実在 dir（符号化・確保段には到達しない）。
    let result = ShioriByteProxy::load(dll, root.path());
    match result {
        Err(ProxyError::EntryNotFound(sym)) => {
            // 初期化の入口 `loadu`・`load` が両方無いときの名札は "load"。
            assert_eq!(sym, "load", "kernel32 は SHIORI load を持たない");
        }
        // Ok は kernel32 が SHIORI エクスポートを持たない以上あり得ない。他 ProxyError も不可。
        // ShioriByteProxy は Debug 非実装ゆえ Result 全体は整形せず、Err 変種のみ整形する。
        Err(e) => panic!("expected EntryNotFound(\"load\"), got other error {e:?}"),
        Ok(_) => panic!("expected EntryNotFound(\"load\"), but load unexpectedly succeeded"),
    }
}

/// 今の作業フォルダから絶対パス `path` への相対パス（同じドライブであること）。
/// 作業フォルダはプロセス全体で 1 つなので、変えずに読むだけにする（並ぶテストを乱さない）。
#[cfg(target_arch = "x86")]
fn relative_from_cwd(path: &Path) -> std::path::PathBuf {
    let cwd = std::env::current_dir().expect("current dir is available");
    let from: Vec<_> = cwd.components().collect();
    let to: Vec<_> = path.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    assert!(
        common > 0,
        "作業フォルダと {} は同じドライブのはず",
        path.display()
    );
    let mut relative: std::path::PathBuf = from[common..].iter().map(|_| "..").collect();
    relative.extend(&to[common..]);
    relative
}

/// 件 1（指摘 1・要件 7.1・7.4）: 依存の探し先を読む DLL のフォルダとシステムの既定に絞る旗は
/// 絶対パスを要るので、作業フォルダからの相対パスは `LoadLibraryFailed` になり、同じ DLL の絶対パスは
/// 読める。素の `LoadLibraryW` に戻すと相対パスで読めて赤になる。
/// 偽の DLL を自分だけのフォルダへ写して読み、印の環境変数は差さないので錠は取らない。
#[test]
#[cfg(target_arch = "x86")]
fn a_relative_dll_path_is_refused() {
    let (root, dll) = copy_testdll_into_unique_target_dir("relative");
    let relative = relative_from_cwd(&dll);
    assert!(relative.is_relative(), "{}", relative.display());
    assert!(
        relative.exists(),
        "相対パスは作業フォルダからは在る: {}",
        relative.display()
    );

    match ShioriByteProxy::load(&relative, root.path()) {
        Err(ProxyError::LoadLibraryFailed(_)) => {}
        Err(e) => panic!("相対パスは LoadLibraryFailed のはず: {e:?}"),
        Ok(_) => panic!("相対の DLL パスは読まないこと: {}", relative.display()),
    }

    let proxy = ShioriByteProxy::load(&dll, root.path())
        .unwrap_or_else(|e| panic!("同じ DLL の絶対パスは読めること: {e:?}"));
    drop(proxy);
}

// ---------------------------------------------------------------------
// 3. testdll 直接 load→drop で courtesy unload 実証（R2.2・E2E 補完）
// ---------------------------------------------------------------------

/// testdll を直接 `load`→`drop` し、Drop の courtesy `unload` が実呼出されること
/// （unload 観測マーカーのファイル作成）と、Drop 全体が無 panic で完了することを確認する
/// （R2.2 の受入証拠・E2E は helper をプロセスごと落とすため Drop が走らない、その補完）。
///
/// 印の環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` を差すので錠 `TESTDLL_SERIAL` を取る。他のテストの
/// unload は、偽の DLL の置き場の判定（印の親フォルダが自分の置き場と同じときだけ書く）によって、
/// この印を書かない。
#[test]
#[cfg_attr(
    not(target_arch = "x86"),
    ignore = "i686 専用: 32bit testdll(shiori.dll) を load するため x64 では BAD_EXE_FORMAT。`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で実行"
)]
fn testdll_drop_invokes_courtesy_unload() {
    // 印の環境変数を差すテスト同士を直列化。汚染ロックは無視して継続。
    let _serial = TESTDLL_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // テストの根を load_dir とし、そこへ shiori.dll を写す（絶対 dll_path＝load_dir\shiori.dll・
    // cwd=load_dir 慣習の縮図）。根は proxy より先に作るので、proxy の後に消える。
    let (root, dll_path) = copy_testdll_into_unique_target_dir("drop");
    let load_dir = root.path();

    // unload 観測マーカーの一時パス（load 前に未作成であること）。
    let marker = load_dir.join("unload.marker");
    assert!(!marker.exists(), "marker は load 前に未作成");

    // SAFETY: edition 2024 では set_var は unsafe（プロセス global 変更）。環境変数を差すテストは
    // 錠 `TESTDLL_SERIAL` で直列になっている。ほかのテストは、この環境変数を偽の DLL の unload の
    // 中で読むだけである。set は load 前に行う。
    unsafe {
        std::env::set_var("HOST32_TESTDLL_UNLOAD_MARKER", &marker);
    }

    // load 成功（testdll の load は入力 HGLOBAL を GlobalFree し true 返し）。
    let proxy = ShioriByteProxy::load(&dll_path, load_dir).expect("testdll load must succeed");

    // まだ Drop していないのでマーカーは未作成。
    assert!(!marker.exists(), "unload はまだ呼ばれていない");

    // Drop → courtesy unload 実呼出 → testdll が marker ファイルを作成。無 panic で完了。
    drop(proxy);

    assert!(
        marker.exists(),
        "Drop の courtesy unload が呼ばれ marker が作成されるはず（R2.2）"
    );
    let content = std::fs::read(&marker).expect("read marker");
    assert_eq!(content, b"unloaded", "testdll unload が書くマーカー内容");

    // 後始末: env を外す（根は Drop で消える）。
    // SAFETY: set_var と同じく unsafe。環境変数を差すテストは錠で直列になっており（錠の中で外す）、
    // ほかのテストは、この環境変数を偽の DLL の unload の中で読むだけである。
    unsafe {
        std::env::remove_var("HOST32_TESTDLL_UNLOAD_MARKER");
    }
}

/// 同じプロセスに読まれた別の写し（フォルダ Y）の unload が、フォルダ X 宛ての印を書かないこと。
/// 印のテストと往復のテストの間の順番に頼らず、競合の芯「別の写しの unload が印を書く」を
/// 1 本の中で切り出す（要件 3.2）。時間待ちは無く、1 つのスレッドで順に進む。
/// 印の環境変数を差すので錠 `TESTDLL_SERIAL` を取る。印のパスは X の DLL と同じフォルダに置く。
#[test]
#[cfg_attr(
    not(target_arch = "x86"),
    ignore = "i686 専用: 32bit testdll(shiori.dll) を load するため x64 では BAD_EXE_FORMAT。`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で実行"
)]
fn testdll_unload_from_another_folder_leaves_marker_untouched() {
    let _serial = TESTDLL_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (dir_x, dll_x) = copy_testdll_into_unique_target_dir("x");
    let (dir_y, dll_y) = copy_testdll_into_unique_target_dir("y");
    let marker_x = dir_x.path().join("unload.marker");
    let marker_y = dir_y.path().join("unload.marker");

    // SAFETY: edition 2024 では set_var は unsafe（プロセス global 変更）。環境変数を差すテストは
    // 錠 `TESTDLL_SERIAL` で直列になっている。ほかのテストは、この環境変数を偽の DLL の unload の
    // 中で読むだけである。
    unsafe {
        std::env::set_var("HOST32_TESTDLL_UNLOAD_MARKER", &marker_x);
    }

    let proxy_x = ShioriByteProxy::load(&dll_x, dir_x.path()).expect("testdll X load must succeed");
    let proxy_y = ShioriByteProxy::load(&dll_y, dir_y.path()).expect("testdll Y load must succeed");
    drop(proxy_y);

    assert!(
        !marker_x.exists(),
        "X has no marker: 別のフォルダ Y の写しの unload が X 宛ての印を書いた"
    );
    assert!(!marker_y.exists(), "Y has no marker");

    drop(proxy_x);
    let content = std::fs::read(&marker_x).expect("X の unload が印を書くはず");
    assert_eq!(content, b"unloaded", "X の unload が書く印の中身");

    // SAFETY: set_var と同じ（錠の中で外す）。
    unsafe {
        std::env::remove_var("HOST32_TESTDLL_UNLOAD_MARKER");
    }
}

// ---------------------------------------------------------------------
// 4. testdll 越し request loopback（R3.2/3.3/3.4/3.6・Task 4.1）
// ---------------------------------------------------------------------

/// testdll を直接 `load` した proxy に対し `request` を実呼出し、固定 SHIORI/3.0 応答が
/// 往復することを確認する（design §435 の HGLOBAL 非対称契約）。
///
/// - GET(`OnTestValue`) → 200 OK＋固定 `Value`。
/// - NOTIFY(`OnTestNotify`) → 204 No Content。
///
/// helper クレートには codec が無いため request バイト列は手書きする。無 panic 完了が
/// 「入力 callee-free／応答 caller-free」（二重解放なし）の観測証拠。印の環境変数
/// `HOST32_TESTDLL_UNLOAD_MARKER` を差さないので錠 `TESTDLL_SERIAL` を取らない。
#[test]
#[cfg_attr(
    not(target_arch = "x86"),
    ignore = "i686 専用: 32bit testdll(shiori.dll) を load するため x64 では BAD_EXE_FORMAT。`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で実行"
)]
fn testdll_request_roundtrip_get_and_notify() {
    // 一意なテストの根へ shiori.dll を写す（絶対 dll_path＝load_dir\shiori.dll）。
    let (root, dll_path) = copy_testdll_into_unique_target_dir("req");

    // load 成功（testdll の load は入力 HGLOBAL を callee-free し true 返し）。
    let proxy = ShioriByteProxy::load(&dll_path, root.path()).expect("testdll load must succeed");

    // GET(OnTestValue) → 200 OK＋固定 Value。
    let get_req = b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestValue\r\n\r\n";
    let get_resp = proxy
        .request(get_req)
        .expect("GET request must succeed via testdll");
    let get_text = String::from_utf8(get_resp).expect("GET response must be valid UTF-8");
    assert!(
        get_text.contains("SHIORI/3.0 200 OK"),
        "expected 200 OK, got: {get_text:?}"
    );
    assert!(
        get_text.contains("Value: \\0\\s[0]host32 request roundtrip ok\\e"),
        "expected fixed Value line, got: {get_text:?}"
    );

    // NOTIFY(OnTestNotify) → 204 No Content。
    let notify_req =
        b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestNotify\r\n\r\n";
    let notify_resp = proxy
        .request(notify_req)
        .expect("NOTIFY request must succeed via testdll");
    let notify_text = String::from_utf8(notify_resp).expect("NOTIFY response must be valid UTF-8");
    assert!(
        notify_text.contains("SHIORI/3.0 204 No Content"),
        "expected 204 No Content, got: {notify_text:?}"
    );

    // Drop で courtesy unload→FreeLibrary。往復も Drop も無 panic＝二重解放なし（R3.3/3.4）。
    drop(proxy);
}

// ---------------------------------------------------------------------
// 5. 後始末の失敗を種類で返す（指摘 9・要件 10.11）
// ---------------------------------------------------------------------

/// `OnTestUnloadFalse` の後の `unload` は偽を返すので、`close` は `UnloadReturnedFalse` を返す。
/// 偽の DLL を自分だけのフォルダへ写して読み、印の環境変数は差さないので錠は取らない。
#[test]
#[cfg(target_arch = "x86")]
fn close_reports_unload_returning_false() {
    let (root, dll) = copy_testdll_into_unique_target_dir("close");
    let proxy = ShioriByteProxy::load(&dll, root.path()).expect("testdll load must succeed");
    let resp = proxy
        .request(b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\nID: OnTestUnloadFalse\r\n\r\n")
        .expect("OnTestUnloadFalse request must succeed");
    assert!(
        resp.starts_with(b"SHIORI/3.0 204"),
        "{}",
        String::from_utf8_lossy(&resp)
    );
    match proxy.close() {
        Err(ProxyError::UnloadReturnedFalse) => {}
        other => panic!("unload の偽は UnloadReturnedFalse のはず: {other:?}"),
    }
}
