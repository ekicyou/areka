//! codec を 3 系統以上の文字コードで固定する決定論テスト
//! （タスク 2.3・要件 9.1/9.2/9.3/9.6/9.7/9.8/12.1）。
//!
//! 窓も I/O も持たない純関数の入出力だけで成立する——32bit 成果物・実 SHIORI・実機を
//! 一切要さない（要件 9.8。`tests/` ではなく `src/` の兄弟ファイルに置く理由がこれ）。
//!
//! ## 期待値の出どころ
//! 参照値のバイト列も `Charset` ヘッダの綴りも、**符号化器から導かず定数として**持つ
//! （要件 9.1）。`Charset::encode`／`Charset::name` の出力を期待値に使うと、符号化が
//! どう退化しても両辺が同じだけずれて緑のままになる（恒真）。定数の出所は
//! design.md §Supporting References。

use super::*;
use crate::charset::CharsetNegotiator;

// ---- 定数（符号化器から導かない）-------------------------------------------

/// 「あ」（U+3042）の UTF-8 バイト列。
const A_UTF_8: &[u8] = &[0xE3, 0x81, 0x82];
/// 「あ」の Shift_JIS バイト列。
const A_SHIFT_JIS: &[u8] = &[0x82, 0xA0];
/// 「あ」の EUC-JP バイト列（Shift_JIS だけを特別扱いする実装では通らない系統・要件 9.7）。
const A_EUC_JP: &[u8] = &[0xA4, 0xA2];
/// 「あ」の ISO-2022-JP バイト列（漢字集合へのエスケープと ASCII への復帰を含む）。
const A_ISO_2022_JP: &[u8] = &[0x1B, 0x24, 0x42, 0x24, 0x22, 0x1B, 0x28, 0x42];

/// 4 系統の（ラベル, `Charset` ヘッダに出るべき正規名, 「あ」のバイト列）。
///
/// 正規名も期待バイト列も逐語の定数であり、`Charset::name()`／`Charset::encode()` から
/// 導かない（要件 9.1）。ラベル解決だけは本番の `Charset::for_label` を通す——ここが
/// 2 値表へ退化すれば EUC-JP と ISO-2022-JP の行が解決できずに赤になる（要件 9.7）。
const FAMILIES: &[(&str, &str, &[u8])] = &[
    ("utf-8", "UTF-8", A_UTF_8),
    ("shift_jis", "Shift_JIS", A_SHIFT_JIS),
    ("euc-jp", "EUC-JP", A_EUC_JP),
    ("iso-2022-jp", "ISO-2022-JP", A_ISO_2022_JP),
];

/// 本仕様適用前の `build_request` が emo2 相当の GET に対して産んだバイト列の逐語写し
/// （要件 9.6・12.1）。
///
/// 出所: `git show 4db516fe:crates/shiori-host32-host/src/shiori3.rs` の `build_request`
/// ——組み立てた `String` をそのまま `out.into_bytes()` して返す版（`Charset` ヘッダ値は
/// `header_value()` ＝ `"UTF-8"` 固定）。その組立順に従って書き下したものであり、
/// **今日の符号化器を 1 度も通していない**（非 ASCII は UTF-8 のバイトを `\x` 表記で直書き）。
///
/// 要求の形は emo2 の OnBoot GET と同形にした——`sender` は `Shiori3Client` の既定
/// `DEFAULT_SENDER`（`client.rs` の既定 sender の定義行）、`Reference0` は
/// `areka/src/emo2_boot/spine_conformance_script.rs` の `SHELL_NAME`（emo2 のシェル名・
/// `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt` の
/// `name` 行の逐語）で、非 ASCII を含むため
/// 「UTF-8 経路のバイト列が 1 バイトも変わらない」を実際に検査できる。
const PRE_SPEC_EMO2_ONBOOT_GET: &[u8] = b"GET SHIORI/3.0\r\n\
Charset: UTF-8\r\n\
Sender: areka\r\n\
ID: OnBoot\r\n\
Reference0: \xE3\x80\x8C\xE3\x82\xB3\xE3\x83\xB3\xE3\x83\x95\xE3\x82\xA3\xE3\x82\xBA\xE3\x83\xAA\xE3\x83\xBC\xE3\x80\x8D\xEF\xBC\x86\xE3\x80\x8CCity-Pop'n\xE3\x80\x8D\r\n\
SecurityLevel: local\r\n\
\r\n";

// ---- 補助 -------------------------------------------------------------------

/// 与えた ASCII 接頭辞で始まる最初の行の、接頭辞より後ろのバイト列を返す。
///
/// 復号せずバイト列のまま切り出すので、期待値の定数とバイト単位で比べられる
/// （ISO-2022-JP のエスケープ列も 1 バイトも落ちない）。
fn header_value_bytes<'a>(bytes: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    bytes
        .split(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .find_map(|line| line.strip_prefix(prefix))
        .unwrap_or_else(|| {
            panic!(
                "接頭辞 {:?} で始まる行が無い: {bytes:?}",
                String::from_utf8_lossy(prefix)
            )
        })
}

/// 「あ」1 文字を `Reference0` に載せた GET を、与えた文字コードで組む。
fn build_a_get(charset: Charset) -> EncodedRequest {
    let references = vec!["あ".to_owned()];
    build_request(&ShioriRequest {
        method: Method::Get,
        id: "OnTest",
        references: &references,
        sender: "areka",
        status: None,
        charset,
    })
}

/// `Value` に任意のバイト列を載せた 200 応答を組む（`Charset` ヘッダは任意）。
fn response_with(charset_header: Option<&str>, value: &[u8]) -> Vec<u8> {
    let mut out = b"SHIORI/3.0 200 OK\r\n".to_vec();
    if let Some(label) = charset_header {
        out.extend_from_slice(b"Charset: ");
        out.extend_from_slice(label.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"Value: ");
    out.extend_from_slice(value);
    out.extend_from_slice(b"\r\n\r\n");
    out
}

/// ラベルを解決する（解決できないなら、それは本番側の退化なのでテストを落とす）。
fn charset(label: &str) -> Charset {
    Charset::for_label(label).unwrap_or_else(|error| {
        panic!("ラベル {label:?} が解決できない（対応集合の退化）: {error:?}")
    })
}

// ---- ⑴ 要求の符号化（要件 9.1・9.7）----------------------------------------

/// ⑴ 4 系統で組んだ要求の `Charset` ヘッダの綴りと `Reference0` のバイト列が、
/// 符号化器から導かない定数と一致する（要件 9.1・3.1〜3.3・9.7）。
#[test]
fn request_bytes_and_charset_header_match_the_constants_in_four_families() {
    for &(label, canonical_name, expected_a) in FAMILIES {
        let encoded = build_a_get(charset(label));

        assert_eq!(
            header_value_bytes(&encoded.bytes, b"Charset: "),
            canonical_name.as_bytes(),
            "{label:?}: Charset ヘッダの綴りが定数と違う"
        );
        assert_eq!(
            header_value_bytes(&encoded.bytes, b"Reference0: "),
            expected_a,
            "{label:?}: 「あ」のバイト列が定数と違う（宣言した文字コードで符号化していない）"
        );
        assert_eq!(encoded.replaced, 0, "{label:?}: 「あ」は 4 系統とも表せる");
    }
}

/// ⑼ `Charset` は request line の直後の最初のヘッダ行である（要件 3.3）。
///
/// 非 ASCII を含み得る行より前に置く、という正典の要請をバイト列の位置で固定する。
#[test]
fn charset_is_the_first_header_line_in_every_family() {
    for &(label, canonical_name, _) in FAMILIES {
        let encoded = build_a_get(charset(label));
        let mut lines = encoded.bytes.split(|&b| b == b'\n');

        assert_eq!(
            lines.next(),
            Some(&b"GET SHIORI/3.0\r"[..]),
            "{label:?}: 先頭行が request line でない"
        );
        let second = lines.next().expect("2 行目が無い");
        assert_eq!(
            second,
            [b"Charset: ", canonical_name.as_bytes(), b"\r"].concat(),
            "{label:?}: Charset が最初のヘッダ行でない"
        );
    }
}

/// ⑹ 現在の文字コードで表せない文字は 10 進の数値文字参照になり、置換数が返る
/// （要件 3.5・10.1）。Shift_JIS だけの性質ではないことを EUC-JP でも固定する。
#[test]
fn unmappable_char_becomes_a_numeric_character_reference_in_the_request() {
    for label in ["shift_jis", "euc-jp"] {
        let references = vec!["\u{1F600}".to_owned()];
        let encoded = build_request(&ShioriRequest {
            method: Method::Get,
            id: "OnTest",
            references: &references,
            sender: "areka",
            status: None,
            charset: charset(label),
        });

        assert_eq!(
            header_value_bytes(&encoded.bytes, b"Reference0: "),
            b"&#128512;",
            "{label:?}: 表せない文字が数値文字参照になっていない"
        );
        assert_eq!(encoded.replaced, 1, "{label:?}: 置換数が 1 でない");
    }
}

/// ⑻ UTF-8 で組んだ要求が、本仕様適用前のバイト列と 1 バイトも違わない
/// （要件 9.6・12.1・3.4）。
#[test]
fn utf8_request_equals_the_pre_spec_byte_stream() {
    let references = vec!["「コンフィズリー」＆「City-Pop'n」".to_owned()];
    let encoded = build_request(&ShioriRequest {
        method: Method::Get,
        id: "OnBoot",
        references: &references,
        sender: "areka",
        status: None,
        charset: Charset::UTF_8,
    });

    assert_eq!(
        encoded.bytes, PRE_SPEC_EMO2_ONBOOT_GET,
        "UTF-8 経路のバイト列が適用前と変わった（左＝現在・右＝適用前の逐語定数）"
    );
    assert_eq!(encoded.replaced, 0);
}

// ---- ⑵〜⑸⑺ 応答の復号（要件 9.2・9.3・4.x）--------------------------------

/// ⑵ 4 系統の応答が、宣言した文字コードで復号されて `Value` が正しく読める（要件 9.2・4.2）。
///
/// 方針の中の文字コードは UTF-8 に固定しているので、ヘッダの宣言を見ずに復号する実装
/// （あるいは Shift_JIS だけを見る実装）では EUC-JP・ISO-2022-JP・Shift_JIS の行が赤になる。
#[test]
fn responses_in_four_families_decode_by_their_declared_charset() {
    for &(_, canonical_name, a_bytes) in FAMILIES {
        let response = response_with(Some(canonical_name), a_bytes);
        let parsed = parse_response(&response, CharsetPolicy::Negotiate(Charset::UTF_8))
            .expect("宣言つきの 200 応答は解析できる");

        assert_eq!(
            parsed.charset_header.as_deref(),
            Some(canonical_name),
            "{canonical_name}: 生の綴りが返っていない"
        );
        assert_eq!(
            parsed.value.as_deref(),
            Some("あ"),
            "{canonical_name}: 宣言どおりに復号できていない"
        );
        assert!(
            !parsed.decode_had_errors,
            "{canonical_name}: 妥当な並びで置換が起きた"
        );
    }
}

/// ⑶ `Charset` ヘッダ省略時は方針の中の文字コード（＝要求に用いた文字コード）を継承する
/// （要件 4.3・9.3）。既定の 2 つ以外でも同じ規則であることを EUC-JP で固定する。
#[test]
fn omitted_charset_header_inherits_the_policy_charset() {
    for (inherited, a_bytes) in [
        (Charset::SHIFT_JIS, A_SHIFT_JIS),
        (charset("euc-jp"), A_EUC_JP),
    ] {
        let response = response_with(None, a_bytes);
        let parsed = parse_response(&response, CharsetPolicy::Negotiate(inherited))
            .expect("ヘッダ省略でも解析できる");

        assert_eq!(parsed.charset_header, None);
        assert_eq!(
            parsed.value.as_deref(),
            Some("あ"),
            "{:?}: 継承した文字コードで復号していない",
            inherited.name()
        );
    }
}

/// ⑷ 解決できないラベルでは方針の中の文字コードで復号を続け、生の綴りをそのまま返す
/// （要件 4.4・10.3・9.3）。
///
/// 「Encoding Standard に無いラベル」と「通信では使えない既知の限界（UTF-16 系）」の
/// 双方を同じ経路で通す。採用しない判断は `CharsetNegotiator` の側にあり、codec は
/// 事実を返すだけ。
#[test]
fn unresolvable_labels_fall_back_to_the_policy_charset() {
    for label in ["no-such-charset", "UTF-16", "replacement"] {
        for (inherited, a_bytes) in [
            (Charset::SHIFT_JIS, A_SHIFT_JIS),
            (charset("euc-jp"), A_EUC_JP),
        ] {
            let response = response_with(Some(label), a_bytes);
            let parsed = parse_response(&response, CharsetPolicy::Negotiate(inherited))
                .expect("解決できないラベルで失敗してはならない");

            assert_eq!(parsed.charset_header.as_deref(), Some(label));
            assert_eq!(
                parsed.value.as_deref(),
                Some("あ"),
                "{label:?}／{}: 方針の中の文字コードで復号していない",
                inherited.name()
            );
        }
    }
}

/// ⑸ 強制方針では応答の `Charset` ヘッダを復号に用いない（要件 4.5・5.4）。
///
/// 本体は強制した文字コードのバイト列で、ヘッダは別の文字コードを名乗る。ヘッダを
/// 見て復号する実装なら「あ」が読めずに赤になる。
#[test]
fn forced_policy_ignores_the_response_header() {
    for (forced, a_bytes) in [
        (Charset::UTF_8, A_UTF_8),
        (charset("euc-jp"), A_EUC_JP),
        (Charset::SHIFT_JIS, A_SHIFT_JIS),
    ] {
        // ヘッダは ISO-2022-JP を名乗るが、本体は強制した文字コードで書かれている。
        let response = response_with(Some("ISO-2022-JP"), a_bytes);
        let parsed = parse_response(&response, CharsetPolicy::Force(forced))
            .expect("強制方針でも解析できる");

        // 生の値は事実として返るが、復号には使われていない。
        assert_eq!(parsed.charset_header.as_deref(), Some("ISO-2022-JP"));
        assert_eq!(
            parsed.value.as_deref(),
            Some("あ"),
            "{}: 強制した文字コードで復号していない",
            forced.name()
        );
    }
}

/// ⑺ 宣言された文字コードとして不正な並びは代替文字で吸収し、置換ありを報告する
/// （要件 4.7・10.2）。解析は失敗しない。
#[test]
fn invalid_byte_sequence_is_absorbed_and_reported() {
    // 各文字コードの 2 バイト文字の先行バイトだけを置いて並びを壊す。
    for (label, broken) in [("Shift_JIS", 0x82u8), ("EUC-JP", 0xA4), ("UTF-8", 0xE3)] {
        let response = response_with(Some(label), &[broken]);
        let parsed = parse_response(&response, CharsetPolicy::Negotiate(Charset::UTF_8))
            .expect("不正な並びで解析を失敗させてはならない");

        assert_eq!(parsed.status, 200);
        let value = parsed.value.expect("Value 行は残る");
        assert!(
            value.contains('\u{FFFD}'),
            "{label}: 代替文字へ吸収されていない: {value:?}"
        );
        assert!(parsed.decode_had_errors, "{label}: 置換ありが報告されない");
    }
}

/// 文字コードヘッダ名の一致は大文字小文字を問わず、名前の前後の空白も許容する
/// （要件 4.1。タスク 2.2 の申し送り——`scan_charset_header` のこの 2 つの寛容さに
/// テストが無かった）。
///
/// 綴りを変えても実際に復号へ効くことまで見る（拾えなければ UTF-8 のまま読むので
/// 「あ」が読めずに赤になる）。
#[test]
fn charset_header_name_matches_case_insensitively_and_tolerates_spacing() {
    for name in [
        "Charset",
        "charset",
        "CHARSET",
        "ChArSeT",
        " Charset ",
        "\tcharset",
    ] {
        let mut response = b"SHIORI/3.0 200 OK\r\n".to_vec();
        response.extend_from_slice(name.as_bytes());
        response.extend_from_slice(b": EUC-JP\r\nValue: ");
        response.extend_from_slice(A_EUC_JP);
        response.extend_from_slice(b"\r\n\r\n");

        let parsed = parse_response(&response, CharsetPolicy::Negotiate(Charset::UTF_8))
            .expect("解析できる");

        assert_eq!(
            parsed.charset_header.as_deref(),
            Some("EUC-JP"),
            "{name:?}: ヘッダ名を拾えていない"
        );
        assert_eq!(
            parsed.value.as_deref(),
            Some("あ"),
            "{name:?}: 拾った宣言が復号に効いていない"
        );
    }
}

// ---- ⑽ 採用が次の要求に現れる連鎖（要件 9.3・9.7 の要）---------------------

/// ⑽ 応答で採用した文字コードが、**次の要求**の `Charset` ヘッダと参照値のバイト列に現れる
/// （要件 9.3・4.2・4.6）。
///
/// 交渉状態と符号化の連続呼出だけで成り立つ（窓も I/O も無い）。`current()` が変わること
/// （`charset_tests.rs` が固定）と「次の要求のバイト列が変わること」は単位の違う主張なので、
/// ここで要求バイト列そのものを定数と比べる。
#[test]
fn adopted_charset_appears_in_the_next_request() {
    let mut negotiator = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    // 初期は Shift_JIS——最初の要求は定数の Shift_JIS バイト列で出る。
    let first = build_a_get(negotiator.current());
    assert_eq!(header_value_bytes(&first.bytes, b"Charset: "), b"Shift_JIS");
    assert_eq!(
        header_value_bytes(&first.bytes, b"Reference0: "),
        A_SHIFT_JIS
    );

    // EUC-JP を名乗る応答を 1 通受ける（採用の規則は交渉状態が所有する）。
    negotiator.note_response(Some("EUC-JP"), false);

    // 次の要求はヘッダの綴りも参照値のバイト列も EUC-JP になる。
    let second = build_a_get(negotiator.current());
    assert_eq!(
        header_value_bytes(&second.bytes, b"Charset: "),
        b"EUC-JP",
        "採用が次の要求のヘッダに現れていない"
    );
    assert_eq!(
        header_value_bytes(&second.bytes, b"Reference0: "),
        A_EUC_JP,
        "採用が次の要求の本文バイト列に現れていない"
    );
    assert_ne!(
        first.bytes, second.bytes,
        "採用の前後で要求バイト列が同じ（採用が効いていない）"
    );
}
