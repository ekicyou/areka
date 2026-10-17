//! 同じ番号の `surface*`ブレスが 2 つ以上あるときの足し合わせ
//! （spec: areka-P0-emily-ghost-verification 要件 6.1・7.5・開発者の決め 2026-10-17「足し合わせる」）。
//!
//! 面ごとのブレスに絵（element定義）を書き、複数の番号を並べた共有のブレスに当たり判定と
//! 着せ替えを書くシェル（えみりの形）で、後のブレスが前の絵を消さないことを確かめる。
//! 同じ名前の行（element番号・collision番号・animation番号＋pattern番号）が重なったら後の行が勝つ。
//! ファイル名と座標は本テストのための作り物（えみりの資産は写さない）。

use areka_parsers::shell::{Interval, parse};

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

/// 同じ名前の行が重なったら後の行が勝ち、重ならない行は足される。
#[test]
fn same_named_line_in_a_later_brace_wins() {
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
            (1, "b1.png", (2, 2)),
            (2, "b2.png", (3, 3))
        ],
        "element1 は後の行・element0 は残り・element2 は足される"
    );

    // 書き直した collision0 は後に書かれた行として末尾へ（画家のアルゴリズム＝後の定義が手前）。
    assert_eq!(
        collisions(&world, 7),
        vec![(1, "Keep".into()), (0, "New".into())]
    );
    assert_eq!(face.collisions[1].right, 30, "collision0 は後の行の矩形");

    // animation9: interval は後の行・pattern1 は後の行・pattern0 は残る。1 本のまま。
    assert_eq!(face.animations.len(), 1);
    let anim = &face.animations[0];
    assert_eq!(anim.id, 9);
    assert_eq!(anim.interval, Interval::Bind);
    let mut patterns: Vec<(u32, i64, u32)> = anim
        .patterns
        .iter()
        .map(|p| (p.index, p.surface_id, p.wait))
        .collect();
    patterns.sort();
    assert_eq!(patterns, vec![(0, 100, 50), (1, 201, 60)]);
}

/// interval 行を書かない後のブレスは前の interval を変えない（書いた行だけが勝つ）。
/// pattern は pattern番号ごとに後の行が勝ち、重ならない番号は足される。
#[test]
fn later_brace_without_interval_line_keeps_the_earlier_interval() {
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
    let anim = &face.animations[0];
    assert_eq!(
        anim.interval,
        Interval::Other("always".into()),
        "interval 行の無い後のブレスは前の interval を Bind へ戻さない"
    );
    let mut patterns: Vec<(u32, i64, u32)> = anim
        .patterns
        .iter()
        .map(|p| (p.index, p.surface_id, p.wait))
        .collect();
    patterns.sort();
    assert_eq!(patterns, vec![(0, 100, 50), (1, 201, 60), (2, 202, 70)]);
}
