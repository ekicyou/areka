//! 再生表のコマを描くコマと働きのコマに分ける檻
//! （spec: seriko-pattern-calls 要件 1.1・1.3・1.5・2.4・7.4・11.1・11.4）。
//!
//! 入力は `surfaces.txt` の本文そのもの（解析を経る実経路）で組む。

use areka_emo_compose::{ComposeMethod, EmoWorld};
use log_capture_kit::{LineFormat, capture_lines};

use super::FrameAct;
use crate::table::{AnimationTable, LoopTrigger};

/// 本文から表を組み、同じ走行の `tracing` 出力を 1 イベント 1 行で添えて返す。
fn table_and_logs(text: &str) -> (AnimationTable, Vec<String>) {
    let world = EmoWorld::build(&areka_parsers::shell::parse(text));
    capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world(&world)
    })
}

/// 8 語を 1 本の animation に並べた面 10（相手の animation 3・4 も同じ面に置く）。
const EIGHT_WORDS: &str = "surface10\n{\n\
    animation0.interval,random,2\n\
    animation0.pattern0,overlay,100,50,1,2\n\
    animation0.pattern1,start,3,70,5,6\n\
    animation0.pattern2,parallelstart,(3,4)\n\
    animation0.pattern3,alternativestart,[3.4]\n\
    animation0.pattern4,stop,3\n\
    animation0.pattern5,parallelstop,(3)\n\
    animation0.pattern6,AlternativeStop,(3,4)\n\
    animation0.pattern7,insert,3\n\
    animation0.pattern8,move,-1,40,0,10\n\
    animation0.pattern9,start,abc\n\
    animation3.interval,never\n\
    animation3.pattern0,overlay,103,10,0,0\n\
    animation4.interval,never\n\
    animation4.pattern0,overlay,104,10,0,0\n}\n";

/// 語ごとの働き・待ち（呼び出しと `insert` と読めない相手は 0・`move` はそのまま）・`move` の X・Y。
#[test]
fn each_word_gets_its_act_and_wait() {
    let (table, _) = table_and_logs(EIGHT_WORDS);
    let anim = table
        .animations(10)
        .iter()
        .find(|a| a.id == 0)
        .expect("animation0 は表に載る");
    let acts: Vec<&FrameAct> = anim.frames.iter().map(|f| &f.act).collect();
    let start = |targets: &[u32], pick_one| FrameAct::Start {
        targets: targets.into(),
        pick_one,
    };
    let stop = |targets: &[u32], pick_one| FrameAct::Stop {
        targets: targets.into(),
        pick_one,
    };
    assert_eq!(
        acts,
        vec![
            &FrameAct::Draw,
            &start(&[3], false),
            &start(&[3, 4], false),
            &start(&[3, 4], true),
            &stop(&[3], false),
            &stop(&[3], false),
            &stop(&[3, 4], true),
            &FrameAct::Inert,
            &FrameAct::Move,
            &FrameAct::Inert,
        ]
    );
    let waits: Vec<u32> = anim.frames.iter().map(|f| f.wait_ms).collect();
    assert_eq!(waits, vec![50, 0, 0, 0, 0, 0, 0, 0, 40, 0]);
    let mv = &anim.frames[8];
    assert_eq!((mv.x, mv.y), (0, 10), "move は X・Y をそのまま持つ");
    let draw = &anim.frames[0];
    assert_eq!(
        (draw.surface_id, draw.method.clone(), draw.x, draw.y),
        (100, ComposeMethod::Overlay, 1, 2),
        "描くコマは今と同じ"
    );
}

/// 働きのコマは描く面を持たない（絵の番号は無し・合成メソッドは記録なしの未知）。8 語以外の
/// 未知の語は今と同じに「未知の合成メソッド名」を記録する。
#[test]
fn act_frames_draw_nothing_and_log_nothing() {
    let text = format!(
        "{EIGHT_WORDS}surface20\n{{\nanimation0.interval,random,2\n\
         animation0.pattern0,someFuture,200,10,0,0\n}}\n"
    );
    let (table, logs) = table_and_logs(&text);
    for f in &table.animations(10)[0].frames[1..] {
        assert_ne!(f.act, FrameAct::Draw);
        assert_eq!(f.picture, None);
        assert!(!f.method.is_implemented(), "働きのコマは描かない: {f:?}");
        assert!(matches!(f.method, ComposeMethod::Unknown(_)));
    }
    let unknown: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("未知の合成メソッド名"))
        .collect();
    assert_eq!(unknown.len(), 1, "8 語は記録しない: {unknown:?}");
    assert!(unknown[0].contains("someFuture"), "{unknown:?}");
    assert_eq!(
        table.animations(20)[0].frames[0].method,
        ComposeMethod::Unknown("someFuture".into())
    );
}

/// `never` と純 `bind` は「呼ばれるだけ」として表に載り、未知の interval は今のまま載らない。
#[test]
fn never_and_pure_bind_are_call_only_unknown_stays_out() {
    let text = "surface10\n{\n\
        animation1.interval,never\n\
        animation1.pattern0,overlay,101,10,0,0\n\
        animation2.interval,bind\n\
        animation2.pattern0,overlay,102,0,0,0\n\
        animation5.interval,yen-e\n\
        animation5.pattern0,overlay,105,0,0,0\n}\n";
    let (table, logs) = table_and_logs(text);
    let triggers: Vec<(u32, LoopTrigger)> = table
        .animations(10)
        .iter()
        .map(|a| (a.id, a.trigger))
        .collect();
    assert_eq!(
        triggers,
        vec![(1, LoopTrigger::CallOnly), (2, LoopTrigger::CallOnly)]
    );
    assert!(
        logs.iter().any(|l| l.contains("vocab=\"yen-e\"")),
        "未知の interval の記録は今のまま: {logs:?}"
    );
    assert!(
        !table.is_continuous(),
        "呼ばれるだけの animation は繰り返しの経路を開けない"
    );
}

/// 働きのコマの欄 2 は面の番号として数えず、呼ばれるだけの animation しか無い面は動く部品に数えない。
#[test]
fn act_frames_and_call_only_parts_are_not_moving_parts() {
    // 面 10 の `move` の欄 2 は 20、面 20 には抽選の animation がある（今までは動く部品に数えた）。
    let moved = "surface10\n{\nanimation0.interval,random,2\n\
        animation0.pattern0,move,20,10,0,5\n}\n\
        surface20\n{\nanimation0.interval,random,2\n\
        animation0.pattern0,overlay,200,10,0,0\n}\n";
    let (table, _) = table_and_logs(moved);
    assert!(
        !table.has_animated_parts(),
        "move の欄 2 を面の番号に数えない"
    );

    // 面 10 の描くコマが指す面 30 には純 `bind` だけがある。
    let bind_only = "surface10\n{\nanimation0.interval,random,2\n\
        animation0.pattern0,overlay,30,10,0,0\n}\n\
        surface30\n{\nanimation0.interval,bind\n\
        animation0.pattern0,overlay,300,0,0,0\n}\n";
    let (table, _) = table_and_logs(bind_only);
    assert_eq!(table.animations(30)[0].trigger, LoopTrigger::CallOnly);
    assert!(
        !table.has_animated_parts(),
        "呼ばれるだけの animation しか無い部品は動かない"
    );
}
