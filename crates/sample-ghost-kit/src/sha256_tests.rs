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

// 下の 2 つの答えは `python -c "import hashlib; print(hashlib.sha256(b'a' * N).hexdigest())"`
// （N は 55 と 64）で作った。55 は最後のブロックに詰めの印と長さが 1 ブロックで収まる最大、
// 64 は最後のブロックが空になり詰めだけの 1 ブロックが足される境目。

#[test]
fn digest_of_55_bytes_matches_the_known_answer() {
    assert_eq!(
        sha256_hex(&[b'a'; 55]),
        "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
    );
}

#[test]
fn digest_of_64_bytes_matches_the_known_answer() {
    assert_eq!(
        sha256_hex(&[b'a'; 64]),
        "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
    );
}
