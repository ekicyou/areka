//! `areka-P0-shiori-loadu` のテスト（design「テスト → `shiori_proxy_loadu_tests.rs`」）。
//! 既存 `mod tests`（`shiori_proxy.rs` 末尾）には触らず、本仕様のテストはここへ置く。
//!
//! 群 A（x64 常時）: 入口の選択の判断表 4 行（要件 1.1〜1.4・1.8・6.3）。DLL を読まない純関数の
//! テストゆえ i686 に限らない（要件 9.5）。
//! 群 C（x64 常時）: 表せない字の検出の決定論（要件 3.3・3.4・6.5）。
//! 群 B（x64 常時）: `loadu` の枝の UTF-8 固定バイト列と入口名の語（要件 2.1・2.2・2.4・4.3・6.4）。
//! 群 D（i686 限定）: 2 本目の偽 DLL `shiori_loadu.dll` を実際に読む 2 本（要件 6.6・6.7・6.9・9.5）。

use super::*;
use std::path::PathBuf;
use std::sync::Mutex;

// ---------------------------------------------------------------------
// 群 A: 入口の選択の判断表 4 行（x64 常時）
// ---------------------------------------------------------------------

/// `loadu` 役のダミー。呼ばれない（fn ポインタの同一性だけを見る）。
/// 本体を `dummy_load` と変えて、同一内容の関数の統合でアドレスが一致する事故を避ける。
unsafe extern "C" fn dummy_loadu(_hdir: HGLOBAL, _len: usize) -> u8 {
    1
}

/// `load` 役のダミー。呼ばれない。
unsafe extern "C" fn dummy_load(_hdir: HGLOBAL, _len: usize) -> u8 {
    2
}

fn loadu_fn() -> LoadFn {
    dummy_loadu
}

fn load_fn() -> LoadFn {
    dummy_load
}

/// 両方在る → `loadu` だけ（要件 1.1）。優先順を逆にするとここが赤になる。
#[test]
fn both_present_chooses_loadu() {
    match choose_init_entry(Some(loadu_fn()), Some(load_fn())) {
        Ok(InitEntry::Loadu(f)) => assert_eq!(f as usize, loadu_fn() as usize),
        Ok(InitEntry::Load(_)) => panic!("両方在るのに load を選んだ（loadu 優先の違反）"),
        Err(e) => panic!("両方在るのに失敗した: {e:?}"),
    }
}

/// `loadu` のみ → `loadu`（要件 1.2）。
#[test]
fn loadu_only_chooses_loadu() {
    match choose_init_entry(Some(loadu_fn()), None) {
        Ok(InitEntry::Loadu(f)) => assert_eq!(f as usize, loadu_fn() as usize),
        Ok(InitEntry::Load(_)) => panic!("loadu のみなのに load を選んだ"),
        Err(e) => panic!("loadu のみの DLL を受け入れなかった: {e:?}"),
    }
}

/// `load` のみ → `load`（要件 1.3・今日どおり）。
#[test]
fn load_only_chooses_load() {
    match choose_init_entry(None, Some(load_fn())) {
        Ok(InitEntry::Load(f)) => assert_eq!(f as usize, load_fn() as usize),
        Ok(InitEntry::Loadu(_)) => panic!("load のみなのに loadu を選んだ"),
        Err(e) => panic!("load のみの DLL を受け入れなかった: {e:?}"),
    }
}

/// 両方無い → 入口が無い失敗。名札は既存テスト `kernel32_yields_entry_not_found` が固定する
/// `"load"`（要件 1.4）。
#[test]
fn neither_present_is_entry_not_found_load() {
    match choose_init_entry(None, None) {
        Err(ProxyError::EntryNotFound(sym)) => assert_eq!(sym, "load"),
        Err(e) => panic!("expected EntryNotFound(\"load\"), got {e:?}"),
        Ok(_) => panic!("入口が両方無いのに選択が成功した"),
    }
}

/// `func()` は選ばれた変種が持つ fn ポインタそのものを返す（呼出側はこれを 1 回だけ呼ぶ・要件 1.5）。
#[test]
fn func_returns_the_held_pointer() {
    assert_eq!(
        InitEntry::Loadu(loadu_fn()).func() as usize,
        loadu_fn() as usize
    );
    assert_eq!(
        InitEntry::Load(load_fn()).func() as usize,
        load_fn() as usize
    );
}

// ---------------------------------------------------------------------
// 群 C: 表せない字の検出の決定論（x64 常時・要件 3.3・3.4・6.5）
// ---------------------------------------------------------------------
// コードページを引数で固定し、機械の既定コードページ（CP_ACP）は読まない。20127 が無効な機械では
// 往路が 0 以下＝`Err` になって赤で気付く（黙って緑にならない・代替は 1252）。

/// US-ASCII（20127）
const CP_US_ASCII: u32 = 20127;
/// UTF-8（65001）
const CP_UTF8: u32 = 65001;

/// ASCII だけ → 表せない字なし・バイト列は恒等（NUL 無し）。
#[test]
fn us_ascii_ascii_only_is_not_lossy_and_identity() {
    let p = r"C:\ghost\master";
    let e = encode_with_codepage(CP_US_ASCII, Path::new(p)).expect("ASCII は 20127 で符号化できる");
    assert!(!e.lossy, "ASCII だけなのに表せない字ありと判定した");
    assert_eq!(e.bytes, p.as_bytes());
}

/// 非 ASCII を含む → 表せない字あり（置き換えられても往復で元に戻らない）。
#[test]
fn us_ascii_non_ascii_is_lossy() {
    let e = encode_with_codepage(CP_US_ASCII, Path::new(r"C:\ゴースト\master"))
        .expect("置き換えはあっても変換自体は成功する");
    assert!(e.lossy, "非 ASCII を含むのに表せない字なしと判定した");
}

/// UTF-8 では絵文字も表せる → 表せない字なし・バイト列は UTF-8 と一致。
#[test]
fn utf8_emoji_is_not_lossy_and_matches_utf8() {
    let p = r"C:\ゴースト😀\master";
    let e = encode_with_codepage(CP_UTF8, Path::new(p)).expect("UTF-8 は何でも表せる");
    assert!(!e.lossy, "UTF-8 なのに表せない字ありと判定した");
    assert_eq!(e.bytes, p.as_bytes());
}

/// 空パス → 空・表せない字なし（今日と同じ）。
#[test]
fn empty_path_is_empty_and_not_lossy() {
    let e = encode_with_codepage(CP_US_ASCII, Path::new("")).expect("空は空へ");
    assert!(e.bytes.is_empty());
    assert!(!e.lossy);
}

// ---------------------------------------------------------------------
// 群 B: `loadu` の枝の UTF-8 固定バイト列（x64 常時・要件 2.1・2.2・2.4・6.4）
// ---------------------------------------------------------------------

/// CP932 に在る字（ゴースト）と無い字（😀）を含むパス → `loadu` の前段が返すバイト列は文字列の
/// UTF-8 そのもの（置き換え無し・NUL 無し）。fn ポインタは呼ばれないのでダミーでよい。
#[test]
fn loadu_init_bytes_are_utf8_verbatim_no_nul() {
    let p = r"C:\ゴースト😀\master";
    let bytes = init_bytes(&InitEntry::Loadu(loadu_fn()), Path::new(p)).expect("UTF-8 にできる");
    assert_eq!(bytes, p.as_bytes());
    assert!(!bytes.contains(&0u8), "NUL が混ざった");
}

/// 入口名の行の語は `loadu`／`load`（要件 4.3・fixture の記録の語と同じ）。
#[test]
fn init_entry_name_is_fixed_word() {
    assert_eq!(InitEntry::Loadu(loadu_fn()).name(), "loadu");
    assert_eq!(InitEntry::Load(load_fn()).name(), "load");
}

// ---------------------------------------------------------------------
// 群 D: 2 本目の偽 DLL を i686 で実際に読む（要件 6.6・6.7・6.9・9.5）
// ---------------------------------------------------------------------
// env `HOST32_TESTDLL_LOADU_*` はプロセス global。既存 `mod tests` の fixture の env とは重ならないので
// 既存 `TESTDLL_SERIAL` とは共用せず、群 D の 2 本だけを自前の mutex で直列化する。

/// 群 D の直列化（汚染ロックは無視して継続）。
static LOADU_SERIAL: Mutex<()> = Mutex::new(());

/// 記録ファイルのパスを fixture へ渡す env（テスト専用）。
const ENV_LOADU_RECORD: &str = "HOST32_TESTDLL_LOADU_RECORD";
/// `loadu` の偽返却を注入する env（テスト専用）。
const ENV_LOADU_FAIL: &str = "HOST32_TESTDLL_LOADU_FAIL";

/// 2 本目の偽 DLL `shiori_loadu.dll` の所在。env `HOST32_TESTDLL_LOADU_DLL` → i686 の debug／release
/// 成果物 → 無ければ先ビルドの命令を書いて panic（黙って飛ばさない・要件 6.9）。
fn resolve_loadu_testdll() -> PathBuf {
    if let Ok(p) = std::env::var("HOST32_TESTDLL_LOADU_DLL") {
        let path = PathBuf::from(p);
        assert!(
            path.is_file(),
            "HOST32_TESTDLL_LOADU_DLL={} が実ファイルでない",
            path.display()
        );
        return path;
    }
    let target_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/i686-pc-windows-msvc");
    for profile in ["debug", "release"] {
        let candidate = target_root.join(profile).join("shiori_loadu.dll");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "shiori_loadu.dll が見つからない。まず PowerShell で \
         `cargo build -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` を実行するか、\
         env HOST32_TESTDLL_LOADU_DLL に絶対パスを設定すること。探索した base: {}",
        target_root.display()
    );
}

/// 既定コードページ（CP932 等）に無い字 `😀` を含む一時フォルダを `load_dir` として作り、記録と偽返却の
/// env を設定して `ShioriByteProxy::load` を 1 回呼ぶ。戻りは (load の結果の Ok/Err, 記録の中身, load_dir)。
/// proxy は結果を返す前に drop する（unload→FreeLibrary）。一時フォルダと env は後始末する。
fn load_loadu_testdll(fail: bool) -> (Result<(), ProxyError>, String, PathBuf) {
    let _serial = LOADU_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dll = resolve_loadu_testdll();
    let load_dir = std::env::temp_dir().join(format!(
        "host32_loadu_test_{}_{}_ゴースト😀",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&load_dir).expect("create temp load_dir");
    // 毎回新しいフォルダの中に置くので、記録は必ずこの呼出の分だけになる（追記の持ち越し無し）。
    let record = load_dir.join("record.txt");

    // SAFETY: edition 2024 では set_var／remove_var は unsafe（プロセス global）。これらの env を
    // 読み書きするのは群 D だけで、LOADU_SERIAL で直列化している。
    unsafe {
        std::env::set_var(ENV_LOADU_RECORD, &record);
        if fail {
            std::env::set_var(ENV_LOADU_FAIL, "1");
        } else {
            std::env::remove_var(ENV_LOADU_FAIL);
        }
    }
    let result = ShioriByteProxy::load(&dll, &load_dir).map(drop);
    // SAFETY: 同上（LOADU_SERIAL 保持中）。
    unsafe {
        std::env::remove_var(ENV_LOADU_RECORD);
        std::env::remove_var(ENV_LOADU_FAIL);
    }
    let text = std::fs::read_to_string(&record).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&load_dir);
    (result, text, load_dir)
}

/// `load_dir` の UTF-8 の小文字 16 進（fixture の記録の書式と同じ）。
fn utf8_hex(p: &Path) -> String {
    p.to_str()
        .expect("一時フォルダは UTF-8 で表せる")
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// D-1: 両方持つ DLL で `loadu` だけが実際に呼ばれ、UTF-8 のパスを受け取る（要件 6.6）。
/// 選択の優先順を逆にすると記録が `load\t…` になって赤。
#[test]
#[cfg_attr(
    not(target_arch = "x86"),
    ignore = "i686 専用: 32bit の shiori_loadu.dll を読むため x64 では BAD_EXE_FORMAT。`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で実行"
)]
fn testdll_loadu_is_called_with_utf8_and_load_is_not() {
    let (result, text, load_dir) = load_loadu_testdll(false);
    if let Err(e) = result {
        panic!("shiori_loadu.dll の確立が失敗した: {e:?}（記録: {text:?}）");
    }
    assert_eq!(
        text,
        format!("loadu\t{}\n", utf8_hex(&load_dir)),
        "記録は loadu の 1 行だけで、受け取ったのは load_dir の UTF-8 そのもの"
    );
    assert_eq!(
        text.lines().filter(|l| l.starts_with("load\t")).count(),
        0,
        "load が呼ばれた"
    );
}

/// D-2: `loadu` が偽を返すと「初期化が偽を返した」失敗になり、`load` へは落ちない（要件 6.7）。
#[test]
#[cfg_attr(
    not(target_arch = "x86"),
    ignore = "i686 専用: 32bit の shiori_loadu.dll を読むため x64 では BAD_EXE_FORMAT。`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で実行"
)]
fn testdll_loadu_false_is_load_returned_false_without_falling_back() {
    let (result, text, _) = load_loadu_testdll(true);
    match result {
        Err(ProxyError::LoadReturnedFalse) => {}
        Err(e) => panic!("expected LoadReturnedFalse, got {e:?}（記録: {text:?}）"),
        Ok(()) => panic!("loadu の偽返却を注入したのに確立が成功した（記録: {text:?}）"),
    }
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "記録は 1 行のはず: {text:?}");
    assert!(
        lines[0].starts_with("loadu\t"),
        "記録が loadu でない: {text:?}"
    );
    assert_eq!(
        lines.iter().filter(|l| l.starts_with("load\t")).count(),
        0,
        "loadu の偽返却のあと load へ落ちた"
    );
}
