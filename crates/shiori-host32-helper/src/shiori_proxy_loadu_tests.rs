//! `areka-P0-shiori-loadu` のテスト（design「テスト → `shiori_proxy_loadu_tests.rs`」）。
//! 既存 `mod tests`（`shiori_proxy.rs` 末尾）には触らず、本仕様のテストはここへ置く。
//!
//! 群 A（x64 常時）: 入口の選択の判断表 4 行（要件 1.1〜1.4・1.8・6.3）。DLL を読まない純関数の
//! テストゆえ i686 に限らない（要件 9.5）。

use super::*;

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
