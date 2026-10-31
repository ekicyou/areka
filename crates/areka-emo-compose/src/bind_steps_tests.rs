//! 着せ替えの描く手順の資源（task 2.1・要件 5.5・6.5・6.7・6.8）。

use areka_parsers::shell::parse;

use super::BindStep::{Draw, Insert};
use crate::log_capture::capture_logs;
use crate::world::EmoWorld;

const SHELL: &str = "\
charset,UTF-8

surface0
{
element0,overlay,body.png,0,0

animation1.interval,bind
animation1.pattern2,overlay,12,0,5,6
animation1.pattern1,insert,2
animation1.pattern0,overlay,10,0,1,2

animation2.interval,bind
animation2.pattern0,overlay,20,0,3,4

animation3.interval,bind+random,4
animation3.pattern0,overlay,30,0,0,0
animation3.pattern1,overlay,31,50,0,0
animation3.pattern2,insert,2

animation4.interval,bind
animation4.pattern0,insert,9
animation4.pattern1,insert,5
animation4.pattern2,insert,abc
animation4.pattern3,overlay,-1,0,0,0
animation4.pattern4,start,2
animation4.pattern5,move,0,0,10,10
animation4.pattern6,unknownmethod,40,0,0,0
animation4.pattern7,overlay,41,0,0,0

animation5.interval,random,3
animation5.pattern0,overlay,50,0,0,0
}
";

fn world() -> EmoWorld {
    EmoWorld::build(&parse(SHELL))
}

/// 3 コマの A は 後ろ・`insert,B`・前 を番号の昇順に並べ、各コマの座標を持つ（6.7・6.8）。
#[test]
fn pure_bind_walks_all_patterns_in_index_order() {
    let w = world();
    assert_eq!(
        w.bind_steps(0, 1),
        [
            Draw {
                surface: 10,
                x: 1,
                y: 2
            },
            Insert { target: 2 },
            Draw {
                surface: 12,
                x: 5,
                y: 6
            },
        ]
    );
    assert_eq!(
        w.bind_steps(0, 2),
        [Draw {
            surface: 20,
            x: 3,
            y: 4
        }]
    );
}

/// `bind+random` は pattern0 だけ（insert も 2 コマ目以降も入らない）。
#[test]
fn bind_random_uses_pattern0_only() {
    assert_eq!(
        world().bind_steps(0, 3),
        [Draw {
            surface: 30,
            x: 0,
            y: 0
        }]
    );
}

/// 相手が無い・`bind` でない・読めない `insert`、負の番号、8 語、描かない語は手順に入らない（6.5）。
#[test]
fn unusable_patterns_are_left_out() {
    assert_eq!(
        world().bind_steps(0, 4),
        [Draw {
            surface: 41,
            x: 0,
            y: 0
        }]
    );
}

/// `bind` 種でない animation と無い番号は空。
#[test]
fn non_bind_and_missing_animations_are_empty() {
    let w = world();
    assert!(w.bind_steps(0, 5).is_empty());
    assert!(w.bind_steps(0, 99).is_empty());
    assert!(w.bind_steps(7, 1).is_empty());
}

/// 手順を作るときに記録は 1 本も出さない（5.5）。
#[test]
fn building_steps_logs_nothing() {
    let w = world();
    let logs = capture_logs(|| {
        crate::bind_steps::BindSteps::build(&w);
    });
    assert!(logs.is_empty(), "手順を作るときに記録が出た: {logs}");
}
