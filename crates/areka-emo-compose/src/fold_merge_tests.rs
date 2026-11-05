//! 同じ番号の `surface*`ブレスが 2 つ以上あるときの足し合わせ
//! （spec: areka-P0-emily-ghost-verification 要件 6.1・7.5・開発者の決め 2026-10-17「足し合わせる」）。
//!
//! 面ごとのブレスに絵（element定義）を書き、複数の番号を並べた共有のブレスに当たり判定と
//! 着せ替えを書くシェル（えみりの形）で、後のブレスが前の絵を消さないことを確かめる。
//! 同じ番号の行が重なったときは SSP 2.9.29 と同じ（設計の議題 7 案 A・追記と同じ 1 つの足し方）:
//! element は両方を足す・animation は前の行が勝つ（後の行は捨てる）・collision は今のまま（後の行が勝つ）。
//! ファイル名と座標は本テストのための作り物（えみりの資産は写さない）。

use areka_parsers::shell::{Interval, parse};

use crate::log_capture::capture_logs;
use crate::world::EmoWorld;

/// えみりと同じ形: 面ごとのブレス（絵）＋ 後に来る共有のブレス（当たり判定・着せ替え・point）。
const PER_FACE_THEN_SHARED: &str = "\
charset,UTF-8

surface5
{
animation50.interval,random,5
animation50.pattern0,overlay,1501,100,0,0
element0,overlay,body5.png,0,0
element1,overlay,face5.png,85,80
}

surface11
{
element0,overlay,body11.png,0,0
element1,overlay,face11.png,121,50
}

surface0,3,5,6
{
animation100.interval,bind
animation100.pattern0,bind,6000,0,90,100
collision0,10,10,50,50,Head
collision1,10,60,50,90,Bust
point.centerx,100
}

surface10,11,12
{
collision0,5,5,40,40,Head
point.centerx,80
}
";

fn layers(world: &EmoWorld, id: u32) -> Vec<(u32, String)> {
    world
        .surface(id)
        .unwrap_or_else(|| panic!("面 {id} が在る"))
        .elements
        .iter()
        .map(|e| (e.layer, e.path.as_str().to_string()))
        .collect()
}

fn collisions(world: &EmoWorld, id: u32) -> Vec<(u32, String)> {
    world
        .surface(id)
        .unwrap_or_else(|| panic!("面 {id} が在る"))
        .collisions
        .iter()
        .map(|c| (c.index, c.name.as_str().to_string()))
        .collect()
}

#[test]
fn shared_brace_adds_to_the_per_face_brace_instead_of_replacing_it() {
    let world = EmoWorld::build(&parse(PER_FACE_THEN_SHARED));

    // 面 5: 自分のブレスの絵と animation50 が残り、共有のブレスの当たり判定と animation100 が足される。
    assert_eq!(
        layers(&world, 5),
        vec![(0, "body5.png".into()), (1, "face5.png".into())],
        "面 5 の絵が共有のブレスで消えない"
    );
    assert_eq!(
        collisions(&world, 5),
        vec![(0, "Head".into()), (1, "Bust".into())]
    );
    let anims: Vec<u32> = world
        .surface(5)
        .expect("面 5")
        .animations
        .iter()
        .map(|a| a.id)
        .collect();
    assert_eq!(anims, vec![50, 100]);

    // 面 11: 絵は element定義だけから来る。共有のブレスの後でも空にならない（0×0 にならない）。
    assert_eq!(
        layers(&world, 11),
        vec![(0, "body11.png".into()), (1, "face11.png".into())],
        "面 11 の絵が共有のブレスで消えない"
    );
    assert_eq!(collisions(&world, 11), vec![(0, "Head".into())]);

    // 共有のブレスだけの番号は今までどおり新しく作られる。
    assert!(layers(&world, 6).is_empty());
    assert_eq!(collisions(&world, 6).len(), 2);
    assert!(layers(&world, 12).is_empty());
    assert_eq!(
        world.surface_ids().collect::<Vec<_>>(),
        vec![0, 3, 5, 6, 10, 11, 12]
    );
}

/// 同じ element番号は両方を足し（前の行が先）、同じ animation番号の後の行（interval も pattern も）
/// は捨てる。collision は今のまま＝同じ collision番号の前の行を除いて後の行を末尾へ。
#[test]
fn later_brace_adds_same_element_number_and_drops_same_animation_number() {
    let text = "\
surface7
{
element0,overlay,a0.png,0,0
element1,overlay,a1.png,1,1
collision0,0,0,10,10,Old
collision1,0,0,20,20,Keep
animation9.interval,random,3
animation9.pattern0,overlay,100,50,0,0
animation9.pattern1,overlay,101,50,0,0
}

surface7
{
element1,overlay,b1.png,2,2
element2,overlay,b2.png,3,3
collision0,0,0,30,30,New
animation9.interval,bind
animation9.pattern1,overlay,201,60,0,0
}
";
    let world = EmoWorld::build(&parse(text));
    let face = world.surface(7).expect("面 7");

    let got: Vec<(u32, &str, (i64, i64))> = face
        .elements
        .iter()
        .map(|e| (e.layer, e.path.as_str(), e.transform.offset()))
        .collect();
    assert_eq!(
        got,
        vec![
            (0, "a0.png", (0, 0)),
            (1, "a1.png", (1, 1)),
            (1, "b1.png", (2, 2)),
            (2, "b2.png", (3, 3))
        ],
        "element1 は両方（前の行が先）・element2 は足される"
    );

    // collision は SSP で測れなかった＝今のまま: 書き直した collision0 は後の行として末尾へ。
    assert_eq!(
        collisions(&world, 7),
        vec![(1, "Keep".into()), (0, "New".into())]
    );
    assert_eq!(face.collisions[1].right, 30, "collision0 は後の行の矩形");

    // animation9: 前の行が勝つ（interval も pattern も前のまま・後の pattern1 は捨てる）。
    assert_eq!(face.animations.len(), 1);
    let anim = &face.animations[0];
    assert_eq!(anim.id, 9);
    assert_eq!(anim.interval, Interval::Random { k: 3 });
    assert_eq!(pattern_surfaces(&world, 7, 9), vec![100, 101]);
}

/// interval 行を書かない後のブレスも、すでにある animation番号なら pattern の行ごと捨てる。
#[test]
fn later_brace_without_interval_line_is_dropped_for_an_existing_animation() {
    let text = "\
surface8
{
animation1.interval,always
animation1.pattern0,overlay,100,50,0,0
animation1.pattern1,overlay,101,50,0,0
}

surface8
{
animation1.pattern1,overlay,201,60,0,0
animation1.pattern2,overlay,202,70,0,0
}
";
    let world = EmoWorld::build(&parse(text));
    let face = world.surface(8).expect("面 8");

    assert_eq!(face.animations.len(), 1);
    assert_eq!(
        face.animations[0].interval,
        Interval::Other("always".into())
    );
    assert_eq!(pattern_surfaces(&world, 8, 1), vec![100, 101]);
}

fn pattern_surfaces(world: &EmoWorld, id: u32, anim: u32) -> Vec<i64> {
    world
        .surface(id)
        .unwrap_or_else(|| panic!("面 {id} が在る"))
        .animations
        .iter()
        .find(|a| a.id == anim)
        .unwrap_or_else(|| panic!("面 {id} に animation{anim} が在る"))
        .patterns
        .iter()
        .map(|p| p.surface_id)
        .collect()
}

/// SSP 2.9.29 の測りの形（research.md 12 節の 0・1・2・3・5・6・8・9）を写したシェル。
/// 絵の名前は測りと同じ（灰・赤・緑・青）。pattern の面は 100＝赤・101＝緑・102＝青。
/// 形 3 は、追記の interval が捨てられたと分かるよう、測りの `bind` でなく `runonce` にした。
const SSP_SHAPES: &str = "\
surface0
{
element0,overlay,gray.png,0,0
element1,overlay,red.png,0,0
}
surface.append0
{
element1,overlay,blue.png,20,20
}
surface1
{
animation1.interval,bind
animation1.pattern0,overlay,100,0,0,0
animation1.pattern1,overlay,101,0,0,0
}
surface.append1
{
animation1.pattern2,overlay,102,0,0,0
}
surface2
{
element0,overlay,gray.png,0,0
element1,overlay,red.png,0,0
}
surface2
{
element1,overlay,blue.png,20,20
}
surface3
{
animation1.interval,bind
animation1.pattern0,overlay,100,0,0,0
}
surface.append3
{
animation1.interval,runonce
animation1.pattern1,overlay,101,0,0,0
}
surface5
{
animation1.interval,bind
animation1.pattern0,overlay,100,0,0,0
}
surface.append5
{
animation1.pattern0,overlay,102,0,0,0
}
surface6
{
animation1.interval,bind
animation1.pattern0,overlay,100,0,0,0
animation1.pattern1,overlay,101,0,0,0
}
surface6
{
animation1.pattern2,overlay,102,0,0,0
}
surface8
{
element0,overlay,gray.png,0,0
}
surface8
{
animation1.interval,bind
animation1.pattern0,overlay,102,0,0,0
}
surface9
{
element0,overlay,gray.png,0,0
}
surface.append9
{
animation1.interval,bind
animation1.pattern0,overlay,102,0,0,0
}
";

const ELEMENT_TWICE: &str = "element番号の二重定義";
const ANIMATION_TWICE: &str = "animation番号の二重定義";

/// 追記と 2 つ目の波括弧は同じ 1 つの足し方で、SSP 2.9.29 の画素の読みと同じ面になる。
/// 重なったときだけ `warn!`（element は形 0・2、animation は形 1・3・5・6）、新しい番号（形 8・9）は黙る。
#[test]
fn append_and_second_brace_match_the_ssp_2_9_29_measurements() {
    let mut built = None;
    let logs = capture_logs(|| built = Some(EmoWorld::build(&parse(SSP_SHAPES))));
    let world = built.expect("面の表");

    // 形 0（追記）・形 2（波括弧）: 赤と青の両方を描く。
    for id in [0, 2] {
        assert_eq!(
            layers(&world, id),
            vec![
                (0, "gray.png".into()),
                (1, "red.png".into()),
                (1, "blue.png".into())
            ],
            "形 {id}: 同じ element番号は両方を足す"
        );
    }
    // 形 1・5（追記）・形 6（波括弧）: すでにある animation1 の後の行を捨てる。
    assert_eq!(pattern_surfaces(&world, 1, 1), vec![100, 101], "形 1");
    assert_eq!(pattern_surfaces(&world, 5, 1), vec![100], "形 5");
    assert_eq!(pattern_surfaces(&world, 6, 1), vec![100, 101], "形 6");
    // 形 3: 追記の animation1 は interval ごと丸ごと捨てる。
    let face3 = world.surface(3).expect("面 3");
    assert_eq!(face3.animations.len(), 1);
    assert_eq!(face3.animations[0].interval, Interval::Bind, "形 3");
    assert_eq!(pattern_surfaces(&world, 3, 1), vec![100], "形 3");
    // 形 8（波括弧）・形 9（追記）: 無い番号の animation は足す。
    for id in [8, 9] {
        assert_eq!(pattern_surfaces(&world, id, 1), vec![102], "形 {id}");
        assert_eq!(layers(&world, id), vec![(0, "gray.png".into())]);
    }

    let warns = |text: &str| -> Vec<String> {
        logs.lines()
            .filter(|l| l.contains("level=WARN") && l.contains(text))
            .map(str::to_string)
            .collect()
    };
    let element = warns(ELEMENT_TWICE);
    assert_eq!(element.len(), 2, "element の二重定義は形 0・2 だけ: {logs}");
    assert!(element[0].contains("surface_id=0") && element[0].contains("element=1"));
    assert!(element[1].contains("surface_id=2"));
    let animation = warns(ANIMATION_TWICE);
    assert_eq!(
        animation.len(),
        4,
        "animation の二重定義は形 1・3・5・6: {logs}"
    );
    for (line, id) in animation.iter().zip([1, 3, 5, 6]) {
        assert!(
            line.contains(&format!("surface_id={id}")) && line.contains("animation_id=1"),
            "{line}"
        );
    }
    assert!(
        !logs.contains("append が既存 animation id を再定義"),
        "追記の丸ごとの置き換えの記録は無くなった: {logs}"
    );
}
