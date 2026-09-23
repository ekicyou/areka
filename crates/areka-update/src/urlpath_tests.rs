use super::*;

#[test]
fn is_encoded_accepts_ascii_with_well_formed_percent() {
    assert!(is_encoded("ghost/master/shell.txt"));
    assert!(is_encoded("a%E3%81%82/b%2fc"));
    assert!(is_encoded(""));
}

#[test]
fn is_encoded_rejects_percent_without_two_hex_digits() {
    assert!(!is_encoded("100%.txt"));
    assert!(!is_encoded("a%2"));
    assert!(!is_encoded("a%G0"));
    assert!(!is_encoded("a%%41"));
}

#[test]
fn is_encoded_rejects_non_ascii() {
    assert!(!is_encoded("ゴースト/a.txt"));
}

#[test]
fn decode_yields_raw_bytes() {
    assert_eq!(decode("a%E3%81%82/b%20c"), "aあ/b c".as_bytes());
    assert_eq!(decode("x%2fy%2Fz"), b"x/y/z");
    // 非 UTF-8 のバイト列もそのまま返す（Shift_JIS の「あ」= 82 A0）
    assert_eq!(decode("%82%A0.txt"), b"\x82\xA0.txt");
}

#[test]
fn encode_keeps_unreserved_and_slash_and_uppercases() {
    assert_eq!(encode("ghost/master/A-z_0.9~"), "ghost/master/A-z_0.9~");
    assert_eq!(encode("ゴ/a b.txt"), "%E3%82%B4/a%20b.txt");
    assert_eq!(encode("100%!(x)=@,"), "100%25%21%28x%29%3D%40%2C");
}

#[test]
fn encode_then_decode_roundtrips() {
    let s = "シェル/surface 0%.png";
    let e = encode(s);
    assert!(is_encoded(&e));
    assert_eq!(decode(&e), s.as_bytes());
}
