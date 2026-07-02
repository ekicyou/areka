//! プリスキャン単体テスト（検出/未検出/寛容/上限/非ASCII打ち切り/BOM）。
//!
//! `prescan_charset` の観測可能振る舞い（タスク 2.2・D1/D2・BOM note）を固定する:
//! - `charset,<name>` 検出（emo2 descript.txt L1 由来）。
//! - 大小差・前後空白・CRLF 寛容。
//! - UTF-8 BOM 前置でも走査開始前に BOM を読み飛ばして検出。
//! - 宣言なし（balloons0s.txt 由来）は `None`。
//! - 非 ASCII バイト先行で走査打ち切り → `None`。
//! - 4096 バイト上限超えの宣言は打ち切り → `None`。
//! - `charset:`（コロン・カンマなし異体）は不一致 → `None`。

use super::prescan::prescan_charset;

/// `charset,UTF-8` を検出して名前を返す（emo2-kakukaku/descript.txt L1 由来）。
#[test]
fn detects_charset_utf8() {
    let bytes = b"charset,UTF-8\ntype,balloon\nname,kakukaku for emo-gs\n";
    assert_eq!(prescan_charset(bytes), Some("UTF-8".to_string()));
}

/// 大小差・キー／値前後空白・CRLF 行末を寛容に扱う。
#[test]
fn tolerates_case_space_and_crlf() {
    let bytes = b"  CHARSET , utf-8  \r\ntype,balloon\r\n";
    assert_eq!(prescan_charset(bytes), Some("utf-8".to_string()));
}

/// UTF-8 BOM（EF BB BF）を走査開始前に読み飛ばして `charset,UTF-8` を検出する。
#[test]
fn skips_utf8_bom_before_scan() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"charset,UTF-8\ntype,balloon\n");
    assert_eq!(prescan_charset(&bytes), Some("UTF-8".to_string()));
}

/// charset 宣言が無い入力は `None`（emo2-kakukaku/balloons0s.txt L1/L2/L4 由来）。
#[test]
fn no_declaration_returns_none() {
    let bytes = b"windowposition.x,266\nwindowposition.y,-129\nwordwrappoint.x,-49\n";
    assert_eq!(prescan_charset(bytes), None);
}

/// charset 行より前に非 ASCII バイトが現れると走査を打ち切り `None`。
#[test]
fn non_ascii_before_declaration_cuts_off() {
    // 先頭行に非 ASCII バイト（0x82 = SJIS リード）を含む → 走査打ち切り。
    let mut bytes = vec![b'a', b',', 0x82, 0xA0, b'\n'];
    bytes.extend_from_slice(b"charset,UTF-8\n");
    assert_eq!(prescan_charset(&bytes), None);
}

/// 4096 バイトの ASCII パディング超えに置いた宣言は上限打ち切りで `None`。
#[test]
fn declaration_beyond_4096_cap_is_ignored() {
    // 4096 バイトを ASCII コメント行で埋めてから charset 宣言を置く。
    let mut bytes = Vec::new();
    // '#' と '\n' で満たす（改行様式は寛容だが上限内には宣言が無い）。
    while bytes.len() < 4096 {
        bytes.push(b'#');
    }
    bytes.push(b'\n');
    bytes.extend_from_slice(b"charset,UTF-8\n");
    assert_eq!(prescan_charset(&bytes), None);
}

/// 上限直前に置いた宣言は検出される（上限境界の正常側）。
#[test]
fn declaration_within_4096_cap_is_detected() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"charset,UTF-8\n");
    while bytes.len() < 4000 {
        bytes.push(b'#');
    }
    assert_eq!(prescan_charset(&bytes), Some("UTF-8".to_string()));
}

/// `charset:UTF-8`（コロン・カンマなし）は異体ゆえ不一致 → `None`（D2）。
#[test]
fn colon_variant_is_rejected() {
    let bytes = b"charset:UTF-8\ntype,balloon\n";
    assert_eq!(prescan_charset(bytes), None);
}

/// 空入力は panic せず `None`。
#[test]
fn empty_input_returns_none() {
    assert_eq!(prescan_charset(b""), None);
}
