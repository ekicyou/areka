//! 読み込みで組んだ面の表を 1 度だけ渡す口（[`ShellTarget::take_world`]）と、形の誤りを読み込みの
//! ときに 1 件 1 行の `warn!` で出すことの檻（areka-P0-offline-surface-dump 要件 7.5・design
//! ShellWorldOnce・ShapeIssuesOnce）。

use super::*;

use areka_emo_atlas::MemoryDecoder;
use temp_path_kit::TempPath;

use super::test_support::{CapturedEvent, capture_events, write_descript};

/// 知らない `animation-sort`・描かないメソッドの pattern（面 0 の animation0）・循環（面 10）・
/// 両方に当たる pattern（面 0 の animation1 が自分を `replace` で指す）を 1 つずつ持つ。
const TEXT: &str = concat!(
    "charset,UTF-8\r\n",
    "animation-sort,sideways\r\n",
    "surface0\r\n{\r\n",
    "element0,overlay,base.png,0,0\r\n",
    "animation0.interval,bind\r\nanimation0.pattern0,replace,20,0,0,0\r\n",
    "animation1.interval,bind\r\nanimation1.pattern0,replace,0,0,0,0\r\n",
    "}\r\n",
    "surface10\r\n{\r\n",
    "element0,overlay,part.png,0,0\r\n",
    "animation0.interval,bind\r\nanimation0.pattern0,overlay,10,0,0,0\r\n",
    "}\r\n",
    "surface20\r\n{\r\nelement0,overlay,part.png,0,0\r\n}\r\n",
);

/// 検体を一時フォルダに置き、絵を入れた復号器と一緒に返す。
fn fixture() -> (TempPath, MemoryDecoder) {
    let dir = TempPath::new("shell-target-shape-issues");
    std::fs::write(dir.child("surfaces.txt"), TEXT).expect("記述ファイル作成");
    write_descript(dir.path());
    let mut dec = MemoryDecoder::new();
    for name in ["base.png", "part.png"] {
        dec.insert(dir.child(name), 1, 1, 4, vec![10u8, 20, 30, 255], true);
    }
    (dir, dec)
}

/// `warn!` 1 行を「本文|欄=値…」にする（`message` 以外の欄は名前の昇順）。
fn describe(e: &CapturedEvent) -> String {
    let mut line = e.message().to_string();
    for (name, value) in e.fields_map() {
        if name != "message" {
            line.push_str(&format!("|{name}={value}"));
        }
    }
    line
}

/// 1 度目（読み込みで組んだ面の表）と 2 度目（組み直し）は同じ面の集まりを返す。
#[test]
fn take_world_first_and_second_return_same_surfaces() {
    let (dir, dec) = fixture();
    let mut target = load_shell_target(dir.path(), &dec).expect("シェルは読める");
    assert!(format!("{target:?}").contains("world: Some"));

    let first = target.take_world();
    assert!(format!("{target:?}").contains("world: None"));
    let second = target.take_world();

    let ids: Vec<u32> = first.surface_ids().collect();
    assert_eq!(ids, vec![0, 10, 20]);
    assert_eq!(ids, second.surface_ids().collect::<Vec<u32>>());
}

/// 読み込みで形の誤りが 1 件 1 行の `warn!` で出て、他の `warn!`／`error!` は 0 行。
/// 循環と描かないメソッドの両方に当たる pattern は描かないメソッドの 1 行だけ。
#[test]
fn load_warns_each_shape_issue_once() {
    let (dir, dec) = fixture();
    let (_target, events) =
        capture_events(|| load_shell_target(dir.path(), &dec).expect("シェルは読める"));

    let mut got: Vec<String> = events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .map(describe)
        .collect();
    got.sort();

    let sort = "shell: animation-sort の値が ascend／descend でないので descend として組む";
    let undrawn = "shell: areka が描けない描画メソッドの pattern を描かない";
    let cycle = "shell: pattern の参照が循環するので、先祖へ戻る参照を置かない";
    let mut want = vec![
        format!("{sort}|value=\"sideways\""),
        format!("{undrawn}|animation_id=0|method=\"replace\"|pattern=0|surface_id=0"),
        format!("{undrawn}|animation_id=1|method=\"replace\"|pattern=0|surface_id=0"),
        format!("{cycle}|animation_id=0|pattern=0|surface_id=10|target=10"),
    ];
    want.sort();
    assert_eq!(got, want);
}
