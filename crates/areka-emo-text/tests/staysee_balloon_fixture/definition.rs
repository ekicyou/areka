//! 定義ファイル（`descript.txt`・`install.txt`）の復号と読み取りを固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.3**／**7.1**・設計 **C2** の B 行）。

use std::collections::BTreeMap;

use areka_emo_text::writing::WritingMode;
use areka_parsers::kv::parse_kv;

use super::test_support::{read_decoded, staysee_model};

// ── 定義の読み取りの期待値（要件 3.3・7.1）──────────────────────────────────────

/// 既定バルーンの id。`descript.txt` の `id` と `install.txt` の `directory` の両方（要件 7.1）。
const EXPECTED_ID: &str = "StayseeBalloon";
/// `descript.txt`／`install.txt` の `name`。
const EXPECTED_NAME: &str = "Balloon for Staysee Syncfield";
/// `descript.txt` が宣言する文字コード（復号の規約が効いていることの確認点）。
const EXPECTED_CHARSET: &str = "Shift_JIS";
/// `descript.txt`／`install.txt` の `type`。
const EXPECTED_TYPE: &str = "balloon";

/// `validrect.left`／`top`／`right`／`bottom` の**宣言された生値**（負値は反対辺基準）。
const EXPECTED_VALIDRECT: (i32, i32, i32, i32) = (22, 20, -26, -47);
/// `font.height`。`font.name` が無いので書体は ukadoc 既定へ縮退する（要件 1.3）。
const EXPECTED_FONT_HEIGHT: u32 = 12;
/// `font.color.r`／`g`／`b`。
const EXPECTED_FONT_COLOR: (u8, u8, u8) = (0, 40, 100);

/// KV から鍵を引く。無ければどのファイルのどの鍵が無いかを名指しで落とす。
fn kv_get<'a>(map: &'a BTreeMap<String, String>, file: &str, key: &str) -> &'a str {
    map.get(key).map(String::as_str).unwrap_or_else(|| {
        panic!(
            "{file} に鍵 `{key}` が無い（実在する鍵は {} 個: {:?}）",
            map.len(),
            map.keys().collect::<Vec<_>>()
        )
    })
}

/// `descript.txt` を宣言された文字コードで復号し、素性の鍵を宣言どおりに読み取る。
///
/// `charset,Shift_JIS` を宣言しているので復号の規約（既定 Ansi・宣言優先）が効いていなければ
/// `craftmanw` の日本語が化け、`name` 等の突合も意味を失う。
#[test]
fn descript_identity_keys_are_read_as_declared() {
    let map = parse_kv(&read_decoded("descript.txt"));
    assert!(
        map.len() >= 5,
        "descript.txt から読めた鍵が {} 個しかない（復号か KV 解析が壊れている合図）",
        map.len()
    );
    for (key, expected) in [
        ("charset", EXPECTED_CHARSET),
        ("type", EXPECTED_TYPE),
        ("id", EXPECTED_ID),
        ("name", EXPECTED_NAME),
    ] {
        assert_eq!(
            kv_get(&map, "descript.txt", key),
            expected,
            "descript.txt の `{key}` が宣言どおりに読めない（出所: 上流 fe1b02f3 の descript.txt）"
        );
    }
}

/// `install.txt` の `directory` が `descript.txt` の `id` と同綴りである（要件 7.1）。
///
/// 下流（`baseware-root-layout` の既定 id 定数・`nar-install` の畳み込み先・
/// `alpha-release-signoff` の zip の中の綴り）はこの事実を写す。片方だけ変わったら赤になる。
#[test]
fn install_directory_matches_descript_id() {
    let descript = parse_kv(&read_decoded("descript.txt"));
    let install = parse_kv(&read_decoded("install.txt"));

    let id = kv_get(&descript, "descript.txt", "id");
    let directory = kv_get(&install, "install.txt", "directory");
    assert_eq!(
        directory, id,
        "install.txt の `directory` と descript.txt の `id` が食い違う（既定バルーン id は 1 綴りであること・要件 7.1）"
    );
    assert_eq!(
        id, EXPECTED_ID,
        "既定バルーン id が期待 `{EXPECTED_ID}` と違う（下流 3 spec の申し送り先が写す綴り）"
    );
    for (key, expected) in [
        ("charset", EXPECTED_CHARSET),
        ("type", EXPECTED_TYPE),
        ("name", EXPECTED_NAME),
    ] {
        assert_eq!(
            kv_get(&install, "install.txt", key),
            expected,
            "install.txt の `{key}` が宣言どおりに読めない"
        );
    }
}

/// `BalloonModel` が宣言された描画範囲・文字の高さと色を写し取る。
///
/// `validrect` は**宣言された生値**で固定する（負値の反対辺解決は画像原寸に依るので
/// 後続の領域解決テストの担当）。ここで見るのは「読み手が宣言を落としていないか」である。
#[test]
fn balloon_model_reads_declared_geometry_and_font() {
    let model = staysee_model();

    let validrect = model.validrect();
    assert_eq!(
        (
            validrect.left(),
            validrect.top(),
            validrect.right(),
            validrect.bottom()
        ),
        (
            Some(EXPECTED_VALIDRECT.0),
            Some(EXPECTED_VALIDRECT.1),
            Some(EXPECTED_VALIDRECT.2),
            Some(EXPECTED_VALIDRECT.3)
        ),
        "validrect の宣言値（left/top/right/bottom）が期待 {EXPECTED_VALIDRECT:?} と違う（出所: 上流 descript.txt の `validrect.*` 4 行）"
    );

    let font = model.font();
    assert_eq!(
        font.height(),
        Some(EXPECTED_FONT_HEIGHT),
        "font.height が期待 {EXPECTED_FONT_HEIGHT} と違う（出所: descript.txt `font.height,12`）"
    );
    let color = font.color();
    assert_eq!(
        (color.r(), color.g(), color.b()),
        (
            Some(EXPECTED_FONT_COLOR.0),
            Some(EXPECTED_FONT_COLOR.1),
            Some(EXPECTED_FONT_COLOR.2)
        ),
        "font.color が期待 {EXPECTED_FONT_COLOR:?} と違う（出所: descript.txt `font.color.r/g/b`）"
    );
}

/// 宣言の無い鍵が「未指定」として扱われ、書字方向が横書きへ解決される。
///
/// StayseeBalloon は `vertical`・`writing_mode`・`wordwrappoint.*`・`font.name` を宣言しない。
/// これらが `Some` で返るようになったら、読み手が既定値を宣言と取り違えた合図である
/// （縮退の行き先＝横書き・`validrect` の遠辺・ukadoc 既定書体は下流の解決層が決める）。
#[test]
fn undeclared_keys_stay_unspecified_and_resolve_to_horizontal() {
    let model = staysee_model();

    assert_eq!(
        model.vertical_raw(),
        None,
        "`vertical` は宣言が無いので未指定であること（出所: 上流 descript.txt に当該行なし）"
    );
    assert_eq!(
        model.writing_mode(),
        None,
        "`writing_mode` は宣言が無いので未指定であること"
    );
    let wrap = model.wordwrappoint();
    assert_eq!(
        (wrap.x(), wrap.y()),
        (None, None),
        "`wordwrappoint.x/y` は宣言が無いので未指定であること（折返し基準は validrect の遠辺へ縮退する）"
    );
    assert_eq!(
        model.font().name(),
        None,
        "`font.name` は宣言が無いので未指定であること（ukadoc 既定書体へ縮退する＝要件 1.3 のとおり areka 側の既定は変えない）"
    );
    assert_eq!(
        WritingMode::resolve(&model),
        WritingMode::HorizontalTb,
        "縦書きの宣言が無いので横書きへ解決されること"
    );
}
