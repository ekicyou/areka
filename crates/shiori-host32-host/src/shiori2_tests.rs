//! `shiori2` の決定論テスト（タスク 2.1・要件 2.1/2.5/7.3）。
//!
//! 窓も I/O も持たない純関数の入出力だけで成立する（x64 のみ・32bit 成果物不要）。
//! 参照値のバイト列は符号化器から導かず、研究記録 10.1 節から手で写した定数として持つ。

use super::{ShioriVersion, VersionVerdict, decide_version, version_probe_request};
use crate::charset::Charset;

/// 研究記録 10.1 節の確かめの要求（SSP が華和梨へ送ったもの）の全文。
const PROBE_UTF8_SSP: &[u8] = b"GET Version SHIORI/2.6\r\nCharset: UTF-8\r\nSender: SSP\r\n\r\n";

/// 研究記録 10.1 節の華和梨 7.4 の答えの全文。
const KAWARI_REPLY: &[u8] = b"SHIORI/2.6 200 OK\r\nCharset: Shift_JIS\r\nCraftman: KawariDevelopmentTeam\r\nID: KAWARI.kdt\r\nVersion: 7.4.0\r\n\r\n";

fn decided(version: ShioriVersion, evidence: &str) -> VersionVerdict {
    VersionVerdict::Decided {
        version,
        evidence: evidence.to_string(),
    }
}

/// 記録に書く綴りは "SHIORI/3.0"／"SHIORI/2.x"。
#[test]
fn version_spelling_for_logs() {
    assert_eq!(ShioriVersion::Shiori3.as_str(), "SHIORI/3.0");
    assert_eq!(ShioriVersion::Shiori2.as_str(), "SHIORI/2.x");
}

/// 確かめの要求の全文のバイトが研究記録 10.1 節と一致する（要件 2.1）。
#[test]
fn probe_request_bytes_match_ssp_record() {
    assert_eq!(version_probe_request(Charset::UTF_8, "SSP"), PROBE_UTF8_SSP);
}

/// `Charset` の行は渡された文字コードの正規名で書く。
#[test]
fn probe_request_uses_canonical_charset_name() {
    let sjis = Charset::for_label("sjis").expect("sjis は解決できる");
    assert_eq!(
        version_probe_request(sjis, "areka"),
        b"GET Version SHIORI/2.6\r\nCharset: Shift_JIS\r\nSender: areka\r\n\r\n"
    );
}

/// 3.0 の答えは状態の番号によらず 3.0（要件 7.3）。
#[test]
fn shiori3_replies_decide_shiori3() {
    assert_eq!(
        decide_version(b"SHIORI/3.0 204 No Content\r\nCharset: UTF-8\r\nSender: pasta\r\n\r\n"),
        decided(ShioriVersion::Shiori3, "SHIORI/3.0 204 No Content")
    );
    assert_eq!(
        decide_version(b"SHIORI/3.0 400 Bad Request\r\n\r\n"),
        decided(ShioriVersion::Shiori3, "SHIORI/3.0 400 Bad Request")
    );
}

/// 華和梨の答えは 2.x・根拠は先頭の行そのまま（要件 7.3）。
#[test]
fn kawari_reply_decides_shiori2() {
    assert_eq!(
        decide_version(KAWARI_REPLY),
        decided(ShioriVersion::Shiori2, "SHIORI/2.6 200 OK")
    );
}

/// 空・版の字の無い先頭の行・2.x でも 3.x でもない版は決められない（要件 2.5・7.3）。
#[test]
fn unknown_replies_are_undecided() {
    for reply in [&b""[..], b"garbage", b"SHIORI/1.1 200 OK\r\n\r\n"] {
        assert!(
            matches!(decide_version(reply), VersionVerdict::Undecided { .. }),
            "{:?} は決められないはず",
            String::from_utf8_lossy(reply)
        );
    }
}
