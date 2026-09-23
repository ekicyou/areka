//! RFC 1321 A.5 のベクトルで CNG の MD5 を較正する（要件 10.3）。
use super::md5_hex;

#[test]
fn rfc1321_vectors() {
    for (input, want) in [
        ("", "d41d8cd98f00b204e9800998ecf8427e"),
        ("a", "0cc175b9c0f1b6a831c399e269772661"),
        ("abc", "900150983cd24fb0d6963f7d28e17f72"),
        ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
        (
            "abcdefghijklmnopqrstuvwxyz",
            "c3fcd3d76192e4007dfb496cca67e13b",
        ),
    ] {
        assert_eq!(md5_hex(input.as_bytes()), want, "input {input:?}");
    }
}
