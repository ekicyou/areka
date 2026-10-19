//! [`sha256_hex`](super::sha256_hex) の自己テスト（spec: `areka-P0-sample-url-fetch` 要件 3.4）。
//!
//! FIPS 180-4 の試験値 4 つ（空・`abc`・448 ビットの 2 ブロックの文・`a` の 100 万回）を
//! 判定する。詰め物が 1 ブロックに収まる・2 ブロックへ溢れる・多数のブロックを回す、の
//! 3 通りの道を全て通る。

use super::sha256_hex;

#[test]
fn empty_message() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn abc() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn two_block_448_bit_message() {
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn one_million_a() {
    assert_eq!(
        sha256_hex(&vec![b'a'; 1_000_000]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}
