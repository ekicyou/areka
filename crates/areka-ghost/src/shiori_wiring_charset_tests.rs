//! 初期の文字コード決定（`initial_charset`／`default_charset`）の決定論テスト
//! （タスク 3.2・要件 1.3／2.1／2.4／2.6／7.1／9.4／10.3）。
//!
//! 窓も 32bit 成果物も実 SHIORI も要さない。descript の固定物は本番と同じ経路
//! （`package::resolve`）で作り（`ShioriMount` は `#[non_exhaustive]`）、決定の結果は
//! `CharsetNegotiator::current`／`policy` で、記録は `capture_events` の観測で固定する。
//!
//! # 「0 件」の主張が恒真にならない理由
//! 捕捉窓 [`crate::test_log_capture::capture_events`] は共有機構
//! [`log_capture_kit::capture`] へ委譲しており、窓の内側で番兵イベントを 1 件発火して
//! 捕捉できなければ panic する（`crates/log-capture-kit/src/capture.rs` の crate doc）。
//! さらに本ファイルの各テストは同じ窓の内側で必ず 1 件の `charset_initial`（info）を
//! 観測するので、「警告 0 件」の主張には毎回その陽性対照が同居している。

use std::path::PathBuf;

use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{self, ShioriMount};
use log_capture_kit::CapturedEvent;
use shiori_host32_host::{Charset, CharsetPolicy};
use temp_path_kit::TempPath;
use tracing::Level;

use super::{default_charset, initial_charset, real_connect};
use crate::test_log_capture::capture_events;

/// 起動ログの宛先（`shiori_wiring` の `LOG_TARGET` と同じ綴り・design §Monitoring）。
const BOOT_TARGET: &str = "ghost-boot";

/// descript の 2 キーを持つ固定物を本番と同じ経路（`package::resolve`）で作る。
///
/// `ShioriMount` は `#[non_exhaustive]` ゆえ `areka-parsers` の外から struct リテラルでは
/// 組めない（本ファイル以外の既存テストと同じ理由・同じ手口）。返り値の [`TempPath`] を
/// テストの間だけ生かしておくこと（破棄で中身ごと消える）。
fn build_shiori_mount(
    encoding: Option<&str>,
    force_encoding: Option<&str>,
) -> (TempPath, ShioriMount) {
    let temp = TempPath::new("ghost-shiori-wiring-charset");
    let root = temp.path().to_path_buf();
    let master_dir = root.join("ghost").join("master");
    std::fs::create_dir_all(&master_dir).expect("fixture master dir 作成に失敗");
    std::fs::create_dir_all(root.join("shell").join("master"))
        .expect("fixture shell dir 作成に失敗");

    let mut descript = String::from("shiori,whatever.dll\n");
    if let Some(label) = encoding {
        descript.push_str(&format!("shiori.encoding,{label}\n"));
    }
    if let Some(label) = force_encoding {
        descript.push_str(&format!("shiori.forceencoding,{label}\n"));
    }
    std::fs::write(master_dir.join("descript.txt"), descript)
        .expect("fixture descript.txt 書き込みに失敗");

    let mount = package::resolve(&root, DefaultEncoding::Ansi)
        .expect("fixture ghost_root の resolve に失敗");
    (temp, mount.shiori)
}

/// 実在しない helper 実行ファイルのパス（`shiori_wiring.rs` の既存テストと同じ手口）。
///
/// 本ファイルの接続テストはクロージャを呼ばないので実際には使われないが、万一呼ばれても
/// 実プロセスが起動しないことを綴りで示しておく。
fn unreachable_helper_exe() -> PathBuf {
    PathBuf::from(r"Z:\definitely\does\not\exist\helper.exe")
}

/// 捕捉列から「語彙フィールド `event` が `name`・レベルが `level`・宛先が `ghost-boot`」の
/// イベントだけを取り出す。
///
/// 件数の主張は**必ずこの絞り込みの長さ**で行う（捕捉列全体の長さで数えると、無関係な
/// イベントが 1 件増えただけで主張が壊れ、逆に語彙の改名には気づけない）。
fn boot_events<'a>(
    events: &'a [CapturedEvent],
    name: &str,
    level: Level,
) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| {
            e.target == BOOT_TARGET && e.level == level && e.field_str("event") == Some(name)
        })
        .collect()
}

/// `charset_initial`（info）がちょうど 1 件あることを確かめ、その `charset`／`source` を返す。
fn expect_single_initial(events: &[CapturedEvent]) -> (String, String) {
    let hits = boot_events(events, "charset_initial", Level::INFO);
    assert_eq!(
        hits.len(),
        1,
        "決定の情報ログはちょうど 1 行のはず（要件 2.6）: 捕捉={:?}",
        events.iter().map(|e| e.fields_map()).collect::<Vec<_>>()
    );
    let charset = hits[0]
        .field_str("charset")
        .expect("charset フィールドが生値で載っているはず")
        .to_string();
    let source = hits[0]
        .field_str("source")
        .expect("source フィールドが生値で載っているはず")
        .to_string();
    (charset, source)
}

/// ⑴ 両方が解決できるなら `shiori.forceencoding` が勝ち、強制が効く（要件 2.1・2.2）。
#[test]
fn force_encoding_wins_over_encoding_and_is_forced() {
    let (_temp, shiori) = build_shiori_mount(Some("UTF-8"), Some("EUC-JP"));

    let mut negotiator = None;
    let events = capture_events(|| {
        negotiator = Some(initial_charset(&shiori, DefaultEncoding::Ansi));
    });
    let negotiator = negotiator.expect("initial_charset は値を返すはず");

    assert_eq!(
        negotiator.policy(),
        CharsetPolicy::Force(Charset::for_label("EUC-JP").expect("EUC-JP は解決できるラベル")),
        "forceencoding が効いていれば方針は Force（要件 2.2）"
    );
    let (charset, source) = expect_single_initial(&events);
    assert_eq!(charset, "EUC-JP", "正規名で記録されるはず（要件 2.6）");
    assert_eq!(source, "forceencoding", "決定根拠は forceencoding");
    assert!(
        boot_events(&events, "charset_label_unresolved", Level::WARN).is_empty(),
        "解決できる宣言では警告は出ない（同じ窓に上の charset_initial が居るので窓は生きている）"
    );
}

/// ⑵ `shiori.encoding` だけなら、その値が初期値になり強制はされない（要件 2.3）。
#[test]
fn encoding_only_is_adopted_and_not_forced() {
    let (_temp, shiori) = build_shiori_mount(Some("EUC-JP"), None);

    let mut negotiator = None;
    let events = capture_events(|| {
        negotiator = Some(initial_charset(&shiori, DefaultEncoding::Ansi));
    });
    let negotiator = negotiator.expect("initial_charset は値を返すはず");

    assert_eq!(
        negotiator.policy(),
        CharsetPolicy::Negotiate(Charset::for_label("EUC-JP").expect("EUC-JP は解決できるラベル")),
        "encoding のみなら以後は交渉に従う＝Negotiate（要件 2.3）"
    );
    let (charset, source) = expect_single_initial(&events);
    assert_eq!(charset, "EUC-JP");
    assert_eq!(source, "encoding", "決定根拠は encoding");
    assert!(
        boot_events(&events, "charset_label_unresolved", Level::WARN).is_empty(),
        "解決できる宣言では警告は出ない"
    );
}

/// ⑶ 宣言が無ければ既定の固定写像で決まる（要件 1.3・`Ansi`→Shift_JIS／`Utf8`→UTF-8）。
#[test]
fn no_declaration_falls_back_to_the_fixed_default_mapping() {
    for (default, expected) in [
        (DefaultEncoding::Ansi, "Shift_JIS"),
        (DefaultEncoding::Utf8, "UTF-8"),
    ] {
        let (_temp, shiori) = build_shiori_mount(None, None);

        let mut negotiator = None;
        let events = capture_events(|| {
            negotiator = Some(initial_charset(&shiori, default));
        });
        let negotiator = negotiator.expect("initial_charset は値を返すはず");

        assert_eq!(
            negotiator.current().name(),
            expected,
            "既定 {default:?} は {expected} へ固定写像される（要件 1.3）"
        );
        assert_eq!(
            negotiator.policy(),
            CharsetPolicy::Negotiate(negotiator.current()),
            "既定からの出発は強制ではない"
        );
        let (charset, source) = expect_single_initial(&events);
        assert_eq!(charset, expected);
        assert_eq!(source, "default", "決定根拠は default");
        assert!(
            boot_events(&events, "charset_label_unresolved", Level::WARN).is_empty(),
            "宣言が無いのは後退ではない＝警告なし"
        );
    }
}

/// `default_charset` の 2 腕の固定写像そのもの（要件 1.3・OS ロケールを読まない）。
#[test]
fn default_charset_is_a_fixed_two_arm_mapping() {
    assert_eq!(default_charset(DefaultEncoding::Ansi), Charset::SHIFT_JIS);
    assert_eq!(default_charset(DefaultEncoding::Utf8), Charset::UTF_8);
}

/// ⑷ 強制の宣言が「通信では使えない既知の限界」なら、警告 1 行を出して `shiori.encoding`
/// へ後退し、**強制の効力も失う**（要件 2.4・10.3）。
#[test]
fn unusable_force_encoding_warns_once_and_loses_the_forcing_effect() {
    let (_temp, shiori) = build_shiori_mount(Some("UTF-8"), Some("UTF-16"));

    let mut negotiator = None;
    let events = capture_events(|| {
        negotiator = Some(initial_charset(&shiori, DefaultEncoding::Ansi));
    });
    let negotiator = negotiator.expect("initial_charset は値を返すはず");

    assert_eq!(
        negotiator.policy(),
        CharsetPolicy::Negotiate(Charset::UTF_8),
        "後退先は shiori.encoding の UTF-8 で、強制の効力は失われる（要件 2.4）"
    );

    let warns = boot_events(&events, "charset_label_unresolved", Level::WARN);
    assert_eq!(warns.len(), 1, "警告はちょうど 1 行（要件 2.4）");
    assert_eq!(
        warns[0].field_str("key"),
        Some("shiori.forceencoding"),
        "どのキーの宣言が退けられたかを記録する"
    );
    assert_eq!(warns[0].field_str("label"), Some("UTF-16"));
    assert_eq!(
        warns[0].field_str("reason"),
        Some("not_encodable"),
        "UTF-16 は「通信では使えない既知の限界」として未知ラベルと区別される（要件 10.3）"
    );
    assert_eq!(
        warns[0].field_str("fallback"),
        Some("UTF-8"),
        "後退先は最終的に採用した正規名（design「後退先を先に決めてから記録する」）"
    );

    let (charset, source) = expect_single_initial(&events);
    assert_eq!(charset, "UTF-8");
    assert_eq!(source, "encoding");
}

/// ⑸ 2 キーとも未知のラベルなら警告 2 行を出して既定へ後退する（要件 9.4・10.3）。
#[test]
fn both_keys_unknown_warn_twice_and_fall_back_to_the_default() {
    let (_temp, shiori) =
        build_shiori_mount(Some("nonsuch-encoding-b"), Some("nonsuch-encoding-a"));

    let mut negotiator = None;
    let events = capture_events(|| {
        negotiator = Some(initial_charset(&shiori, DefaultEncoding::Ansi));
    });
    let negotiator = negotiator.expect("initial_charset は値を返すはず");

    assert_eq!(
        negotiator.policy(),
        CharsetPolicy::Negotiate(Charset::SHIFT_JIS),
        "2 キーとも退けられたら既定（Ansi→Shift_JIS）で非強制"
    );

    let warns = boot_events(&events, "charset_label_unresolved", Level::WARN);
    assert_eq!(warns.len(), 2, "退けた宣言の数だけ警告が出る（要件 7.1）");
    assert_eq!(warns[0].field_str("key"), Some("shiori.forceencoding"));
    assert_eq!(warns[0].field_str("label"), Some("nonsuch-encoding-a"));
    assert_eq!(warns[1].field_str("key"), Some("shiori.encoding"));
    assert_eq!(warns[1].field_str("label"), Some("nonsuch-encoding-b"));
    for warn in &warns {
        assert_eq!(
            warn.field_str("reason"),
            Some("unknown"),
            "Encoding Standard に無いラベルの理由は unknown（要件 10.3）"
        );
        assert_eq!(
            warn.field_str("fallback"),
            Some("Shift_JIS"),
            "2 行とも最終的に採用した既定を後退先として記録する"
        );
    }

    let (charset, source) = expect_single_initial(&events);
    assert_eq!(charset, "Shift_JIS");
    assert_eq!(source, "default");
}

/// ⑹ 情報ログの `charset` は派生 `Debug`（`Charset(Encoding { .. })`）ではなく正規名であり、
/// `source` は 3 語彙のいずれかである（要件 2.6・design §Monitoring）。
#[test]
fn charset_initial_records_the_canonical_name_not_the_debug_form() {
    let (_temp, shiori) = build_shiori_mount(None, Some("shift_jis"));

    let events = capture_events(|| {
        let _ = initial_charset(&shiori, DefaultEncoding::Utf8);
    });

    let (charset, source) = expect_single_initial(&events);
    assert_eq!(
        charset, "Shift_JIS",
        "別名の綴りではなく正規名で記録する（`?charset` では Charset(Encoding {{ .. }}) になる）"
    );
    assert!(
        !charset.contains("Encoding"),
        "派生 Debug 表現が漏れていないこと: {charset:?}"
    );
    assert_eq!(source, "forceencoding");
}

/// ⑺ 初期値の決定は [`real_connect`] を**呼んだ時点**で同期に済んでいる（タスク 3.4・要件 2.7）。
///
/// このテストは返ってきたクロージャを**一度も呼ばない**。それでも決定の情報ログが 1 行
/// 出ていることを見るので、決定がクロージャの内側（＝shiori アクタースレッド上・接続成功時）
/// へ遅れていないことが分かる。決定を `move ||` の内側へ落とす退化を入れると、捕捉窓が
/// 別スレッド・別時点の発火を拾えず `charset_initial` が 0 件になって赤になる。
///
/// 遅らせてはならない理由は 2 つ（design §shiori_wiring）: 起動ログが呼出スレッドの同期発火
/// しか捕捉できないこと、そして接続に失敗しても決定と根拠が記録に残ること（要件 2.6）。
#[test]
fn real_connect_decides_the_initial_charset_before_returning_the_closure() {
    let (_temp, shiori) = build_shiori_mount(None, Some("EUC-JP"));

    let events = capture_events(|| {
        // クロージャを束縛せずに捨てる＝接続手続きは 1 段も走らない（窓生成も spawn も無し）。
        let _ = real_connect(unreachable_helper_exe(), shiori, DefaultEncoding::Ansi);
    });

    let (charset, source) = expect_single_initial(&events);
    assert_eq!(
        charset, "EUC-JP",
        "descript の宣言が本番の接続入口まで通っているはず（暫定の UTF-8 なら赤）"
    );
    assert_eq!(source, "forceencoding", "決定根拠も記録される");
}

/// ⑻ 接続のたびに初期値を決め直し、前回の結果を引き継がない（要件 5.2）。
///
/// [`real_connect`] を 2 回呼ぶと `charset_initial` は 2 行出て、各行の文字コードは
/// **その呼び出しに渡した既定**を反映する。交渉状態を 1 つだけ作って共有したり、
/// 最初の決定を憶えて使い回したりすると（＝前回セッションの採用結果を引き継ぐ形）、
/// 件数か値のどちらかが崩れて赤になる。
#[test]
fn every_connect_derives_the_initial_value_again_and_inherits_nothing() {
    let (_temp, shiori) = build_shiori_mount(None, None);

    let events = capture_events(|| {
        let _ = real_connect(
            unreachable_helper_exe(),
            shiori.clone(),
            DefaultEncoding::Ansi,
        );
        let _ = real_connect(unreachable_helper_exe(), shiori, DefaultEncoding::Utf8);
    });

    let hits = boot_events(&events, "charset_initial", Level::INFO);
    assert_eq!(
        hits.len(),
        2,
        "接続 2 回なら決定も 2 回（使い回すと 1 件になる）: 捕捉={:?}",
        events.iter().map(|e| e.fields_map()).collect::<Vec<_>>()
    );
    assert_eq!(
        hits[0].field_str("charset"),
        Some("Shift_JIS"),
        "1 回目は宣言なし＋既定 Ansi＝Shift_JIS（要件 2.7 の emo2 の形）"
    );
    assert_eq!(
        hits[1].field_str("charset"),
        Some("UTF-8"),
        "2 回目は 1 回目の結果ではなくこの呼び出しの引数で決まる（要件 5.2）"
    );
    for hit in &hits {
        assert_eq!(
            hit.field_str("source"),
            Some("default"),
            "どちらの決定も宣言なし＝既定から出発する"
        );
    }
}

/// ⑼ 接続の値は「起動時に決めた交渉状態」をそのまま受け取る（要件 2.7・5.2）。
///
/// この最後の受け渡しだけは挙動で確かめられない——接続の値が組まれるのは i686 helper との
/// 接続が成立した後だけで、決定論テストからは到達できないため。そこで本番ファイルの本文
/// そのものを判定材料にし、交渉状態を**組み立てている箇所が 1 つだけ**であることを数える。
///
/// 唯一の組立点は初期値を決める関数の末尾であり、接続の値へは決まった値を渡すだけである。
/// 接続のところで暫定の値を作り直す形（本タスク以前の継ぎ目）へ戻すと組立点が 2 つになって
/// 赤になる。関数の本文はコメントも含めて検査するので、説明の側では組立の綴りを書かない
/// （書くと赤になる——検査を緩めるのではなく説明の方を言い換えること）。
#[test]
fn the_connection_receives_the_already_decided_negotiator() {
    const SOURCE: &str = include_str!("shiori_wiring.rs");

    assert_eq!(
        SOURCE.matches("CharsetNegotiator::new").count(),
        1,
        "交渉状態の組立点は初期値を決める関数の 1 か所だけのはず\
         （接続のところで作り直すと 2 になる・要件 2.7／5.2）"
    );
    // 「接続の値がフィールド短縮記法で受け取ること」を別に主張していたが、外した。
    // 意味の変わらない書き換え（`negotiator: negotiator,`）で赤になる一方、決定を
    // クロージャの内側へ移す退化は上の出現数と
    // `real_connect_decides_the_initial_charset_before_returning_the_closure` が既に
    // 捕まえる。欠陥を捕まえず書き方だけを縛る検査は置かない。
}
