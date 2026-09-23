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
