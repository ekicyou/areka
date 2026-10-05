//! 焼く一覧と記録の檻（spec: areka-P0-surface-element-nesting・要件 1.4・1.5・3.1・3.2・3.5・
//! 6.1・8.1）。
//!
//! 見ているのは 3 つ——⑴ 数字だけの element定義の欄は、その名前の絵が在っても焼く一覧に
//! 入らず、読み込みの失敗も出ない ⑵ 入れ子の検体（`areka-emo-compose/tests/fixtures/
//! surface-nesting/`）を読み込むと、仕込んだ誤り 1 件につき `warn!` が 1 行ずつ出て、それ以外の
//! `warn!`／`error!` は 0 行 ⑶ 描画メソッド `balloon` の element定義は欄が数字だけでも箱の名前と
//! して読まれる（入れ子の報告に載らない）。
//!
//! 復号器はメモリ上のもの（パスで引くだけで、ファイルの実在を見ない）。「フォルダに在る」は
//! 復号器にその名前で絵を入れることで作る——読みに行けば必ず焼けるので、索引表に載らないことが
//! 「読みに行かなかった」の証になる。

use super::*;

use std::path::PathBuf;

use areka_emo_atlas::MemoryDecoder;
use temp_path_kit::TempPath;

use super::test_support::{CapturedEvent, capture_events};

/// 本モジュールが出す記録の宛先（既定の target＝モジュールパス）。
const SHELL_TARGET: &str = "areka_emo_present::shell_target";

/// 検体のフォルダに在る画像（`surfaces.txt` 以外の全部）。
const FIXTURE_IMAGES: [&str; 8] = [
    "body.png",
    "eye.png",
    "eye_closed.png",
    "mouth.png",
    "mouth_open.png",
    "part.png",
    "surface11.png",
    "surface2.png",
];

/// 検体の element定義に書かれた数字だけの欄（`surface.append1` の 30 を含む・重複なし）。
const DIGIT_FIELDS: [&str; 11] = [
    "10",
    "11",
    "30",
    "31",
    "32",
    "9999",
    "4294967296",
    "60",
    "61",
    "62",
    "70",
];

/// 入れ子の検体のフォルダ（`areka-emo-compose` の検体を書き換えずに読む・タスク 1.2）。
fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("areka-emo-compose")
        .join("tests")
        .join("fixtures")
        .join("surface-nesting")
}

/// 不透明 1×1 PBGRA の絵を `names` の名前で入れた復号器。
fn decoder_with(dir: &Path, names: &[&str]) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    for name in names {
        dec.insert(dir.join(name), 1, 1, 4, vec![10u8, 20, 30, 255], true);
    }
    dec
}

/// `warn!` 1 行を「本文|欄=値…」の 1 本の文字列にする（`message` 以外の欄は名前の昇順）。
fn describe(e: &CapturedEvent) -> String {
    let mut line = e.message().to_string();
    for (name, value) in e.fields_map() {
        if name != "message" {
            line.push_str(&format!("|{name}={value}"));
        }
    }
    line
}

/// 水準が `warn` 以上（`warn!`／`error!`）の記録。宛先は問わない。
fn warn_or_worse(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .collect()
}

/// ⑴ 数字だけの欄は、その名前の絵が在っても焼かない（要件 1.5・3.5）。
#[test]
fn digit_only_fields_are_not_baked_even_when_such_images_exist() {
    let dir = fixture_dir();
    let names: Vec<&str> = FIXTURE_IMAGES
        .iter()
        .chain(&DIGIT_FIELDS)
        .copied()
        .collect();
    let dec = decoder_with(&dir, &names);

    let target = load_shell_target(&dir, &dec).expect("検体のシェルは読める");

    // 較正: 画像の element定義と、ファイル名の慣習だけで建つ子 11 の絵は焼かれている。
    for name in ["body.png", "part.png", "surface11.png"] {
        assert!(
            target.atlas().resolve(SetId(0), name).is_some(),
            "画像 {name} は焼かれているはず"
        );
    }
    for name in DIGIT_FIELDS {
        assert!(
            target.atlas().resolve(SetId(0), name).is_none(),
            "数字だけの欄 {name} を画像として焼いた"
        );
    }
    assert!(
        target.bake_errors().is_empty(),
        "焼く段で落ちた絵: {:?}",
        target.bake_errors()
    );
}

/// ⑵ 検体を読み込むと、仕込んだ誤り 1 件につき `warn!` が 1 行ずつ出て、他の `warn!`／`error!`
/// （画像の読み込みの失敗を含む）は 0 行（要件 3.1・3.2・3.5・6.1・8.1）。
///
/// 数字だけの欄の名前の絵は復号器に**入れない**——焼く一覧に入っていれば、ここで脱落の
/// `warn!` が出て数が合わなくなる。
#[test]
fn fixture_load_warns_once_per_planted_issue_and_nothing_else() {
    let dir = fixture_dir();
    let dec = decoder_with(&dir, &FIXTURE_IMAGES);

    let (target, events) =
        capture_events(|| load_shell_target(&dir, &dec).expect("検体のシェルは読める"));

    let mut got: Vec<String> = warn_or_worse(&events)
        .into_iter()
        .inspect(|e| assert_eq!(e.target, SHELL_TARGET, "宛先が権威でない: {}", describe(e)))
        .map(describe)
        .collect();
    got.sort();

    let missing = "shell: element定義が指したサーフェスが無いので置かない";
    let cycle = "shell: element定義の参照が循環するので、先祖へ戻る参照を置かない";
    let in_child = "shell: 子として置かれたサーフェスの箱は親の中に置かない";
    let mut want = vec![
        format!("{missing}|element=1|surface=50|target=\"9999\""),
        format!("{missing}|element=2|surface=50|target=\"4294967296\""),
        format!("{cycle}|element=1|surface=60|target=60"),
        format!("{cycle}|element=1|surface=61|target=62"),
        format!("{cycle}|element=1|surface=62|target=61"),
        format!("{in_child}|child=70|name=\"fuda\"|parent=71"),
    ];
    want.sort();
    assert_eq!(got, want);

    // 報告そのものも件数どおり（記録と報告の 1 対 1）。
    assert_eq!(target.nest_report.issues.len(), 5);
    assert!(target.bake_errors().is_empty());
}

/// ⑶ 描画メソッド `balloon` の element定義は、欄が数字だけでも箱の名前（要件 1.4）。
///
/// 番号として読まれていれば「無い番号」の報告に載る。箱の名前として読まれるので、名前の
/// `balloon.*`ブレスが無い箱の報告（`ElementUnknownBrace`）に載り、入れ子の報告は 0 件である。
#[test]
fn balloon_element_with_digit_only_field_is_a_box_name() {
    let dir = TempPath::new("shell-target-nesting-balloon-digits");
    std::fs::write(
        dir.child("surfaces.txt"),
        concat!(
            "charset,UTF-8\n",
            "surface0\n{\n",
            "element0,overlay,base.png,0,0\n",
            "element1,balloon,8888,0,0\n",
            "}\n",
        ),
    )
    .expect("記述ファイル作成");
    let dec = decoder_with(dir.path(), &["base.png", "8888"]);

    let (target, events) =
        capture_events(|| load_shell_target(dir.path(), &dec).expect("シェルは読める"));

    assert!(
        target.nest_report.issues.is_empty(),
        "{:?}",
        target.nest_report
    );
    assert!(target.atlas().resolve(SetId(0), "8888").is_none());
    let got: Vec<String> = warn_or_worse(&events).into_iter().map(describe).collect();
    assert_eq!(
        got,
        vec![
            "shell: 名前の balloon.*ブレスが無い箱の element定義を読み捨てた|element=1|name=\"8888\"|surface=0"
                .to_string()
        ]
    );
}
