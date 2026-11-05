//! 面の切り替えと着せ替えの変化の発行列（spec: areka-P0-surface-element-nesting 要件 5.6・5.7・
//! 5.9・5.12・8.3・tasks.md 7.5・design.md「一番上のサーフェスの切り替え」）。
//!
//! 表は `surfaces.txt` の本文から実経路で組み、cue と刻みを同じ `handle_message` へ順に流して、
//! 発行先の記録の並びで確かめる（刻みは注入の絶対時刻・乱数は常に発火）。

use super::test_support::*;
use crate::looper::tests::{always_fire, cfg};
use crate::output::DisplayCommand;
use crate::test_support::table_of;
use areka_emo_compose::BindSet;
use areka_sakura::CueCommand;

/// 一番上 0・2 は子 100 を置き、自分の animation9（200 → 201 → `-1`）を持つ。1 は何も置かない。
/// 100 の animation0 は 101（開き目・すぐ）→ 102（閉じ目・500ms 後）→ `-1`（さらに 500ms 後）。
const BLINK: &str = "surface0\n{\nelement0,overlay,100,0,0\n\
    animation9.interval,random,2\n\
    animation9.pattern0,overlay,200,0,0,0\n\
    animation9.pattern1,overlay,201,500,0,0\n\
    animation9.pattern2,overlay,-1,500,0,0\n}\n\
    surface1\n{\n}\n\
    surface2\n{\nelement0,overlay,100,0,0\n\
    animation9.interval,random,2\n\
    animation9.pattern0,overlay,200,0,0,0\n\
    animation9.pattern1,overlay,201,500,0,0\n\
    animation9.pattern2,overlay,-1,500,0,0\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,500,0,0\n\
    animation0.pattern2,overlay,-1,500,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface200\n{\n}\nsurface201\n{\n}\n";

/// 一番上 0 が子 100 を置く。100 の animation5 は着せ替え（`bind+random`）で 105 → 106 → `-1`。
const BIND_CHILD: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation5.interval,bind+random,2\n\
    animation5.pattern0,overlay,105,0,0,0\n\
    animation5.pattern1,overlay,106,500,0,0\n\
    animation5.pattern2,overlay,-1,500,0,0\n}\n\
    surface105\n{\n}\nsurface106\n{\n}\n";

/// 一番上 0 が子 100 を置き、1 は何も置かない。100 の animation0 は 101 → `-2`（`-1` 以外の負）。
const NEGATIVE_CHILD: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface1\n{\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-2,500,0,0\n}\n\
    surface101\n{\n}\n";

/// この檻の足場（本体は兄弟の足場 `ClockRig`）。
type Rig = ClockRig;

impl Rig {
    fn from_text(text: &str, static_binds: BindSet) -> Self {
        let table = table_of(text);
        assert!(table.has_animated_parts(), "前提: 動く部品の在る表");
        Self::configured(cfg(table, always_fire()), eye_names(), static_binds)
    }

    /// 時計が `ms` のときに `\0\s[key]`。
    fn surface(&mut self, ms: u64, key: &str) -> Vec<DisplayCommand> {
        self.hear(ms, emote_cue(0.0, "0", key))
    }

    /// 時計が `ms` のときに `\0\![bind,目,閉,on]`。
    fn bind_eye(&mut self, ms: u64, on: &str) -> Vec<DisplayCommand> {
        let command = CueCommand::command_carrier(
            "bind",
            vec!["目".to_string(), "閉".to_string(), on.to_string()],
        );
        self.hear(ms, cue(0.0, "0", command, 0.0))
    }
}

/// 子が閉じ目のコマの途中で `\s` を、同じ子を置く面へ切り替える: `Show` は 1 件だけで閉じ目を
/// 載せている（要件 5.6）。一番上の animation は載らず、次の境界で最初のコマから始まる（要件 5.9）。
/// 次の刻みで同じ `Show` は二重に出ない。
#[test]
fn switch_mid_blink_emits_one_show_carrying_the_closed_eye_frame() {
    let mut rig = Rig::from_text(BLINK, BindSet::from_ids([]));
    rig.surface(0, "0");
    rig.tick(0);
    let (_, _, p) = single_show(rig.tick(1000));
    assert_eq!(top_frames(&p), vec![(9, 200)]);
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    let (_, _, p) = single_show(rig.tick(1600));
    assert_eq!(top_frames(&p), vec![(9, 201)], "前提: 一番上は 2 コマ目");
    assert_eq!(part_frames(&p, 100), vec![(0, 102)], "前提: 子は閉じ目");

    let cmds = rig.surface(1600, "2");
    let (sid, _, p) = single_show(cmds);
    assert_eq!(sid, 2);
    assert_eq!(
        part_frames(&p, 100),
        vec![(0, 102)],
        "切り替えの 1 枚目から閉じ目"
    );
    assert!(top_frames(&p).is_empty(), "一番上の再生は捨てた");

    assert!(rig.tick(1700).is_empty(), "同じ Show を二重に出さない");

    // 次の境界: 一番上は面 2 で最初のコマ（200）から。子は経過 1000 で `-1` に着いて止まる。
    let (sid, _, p) = single_show(rig.tick(2000));
    assert_eq!(sid, 2);
    assert_eq!(top_frames(&p), vec![(9, 200)], "一番上は最初から");
    assert!(part_frames(&p, 100).is_empty());
}

/// 子を置いていない面へ行って戻っても、子は見えなかった間も進んでいた続きから出る（要件 5.7）。
#[test]
fn leaving_to_a_face_without_the_child_and_back_resumes_without_rewind() {
    let mut rig = Rig::from_text(BLINK, BindSet::from_ids([]));
    rig.surface(0, "0");
    rig.tick(0);
    let (_, _, p) = single_show(rig.tick(1000));
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

    let (sid, _, p) = single_show(rig.surface(1000, "1"));
    assert_eq!(sid, 1);
    assert!(p.is_empty(), "子を置いていない面に部品のコマは無い");
    assert!(rig.tick(1600).is_empty(), "見えない子のために描き直さない");

    let (sid, _, p) = single_show(rig.surface(1600, "0"));
    assert_eq!(sid, 0);
    assert_eq!(
        part_frames(&p, 100),
        vec![(0, 102)],
        "戻った 1 枚目が経過 600 の閉じ目（巻き戻らない）"
    );
    assert!(rig.tick(1700).is_empty(), "同じ Show を二重に出さない");
}

/// 着せ替えの変化でも `Show` は 1 件だけ。外した瞬間の `Show` に外れた側の部品のコマは載らず、
/// 付け直した瞬間の `Show` は時計の続きのコマを載せる（要件 5.12・5.6）。
#[test]
fn bind_change_emits_one_show_without_the_removed_side_frames() {
    let mut rig = Rig::from_text(BIND_CHILD, BindSet::from_ids([5]));
    rig.surface(0, "0");
    rig.tick(0);
    let (_, _, p) = single_show(rig.tick(1000));
    assert_eq!(part_frames(&p, 100), vec![(5, 105)]);
    let (_, _, p) = single_show(rig.tick(1600));
    assert_eq!(
        part_frames(&p, 100),
        vec![(5, 106)],
        "前提: 着せ替えの子が 2 コマ目"
    );

    let (sid, binds, p) = single_show(rig.bind_eye(1600, "0"));
    assert_eq!(sid, 0);
    assert_eq!(binds, BindSet::from_ids([]));
    assert!(p.is_empty(), "外れた側の部品のコマを載せない: {p:?}");

    // 刻みを挟まずに付け直す: 時計は次の刻みまで残っているので続きのコマが載る。
    let (_, binds, p) = single_show(rig.bind_eye(1600, "1"));
    assert_eq!(binds, BindSet::from_ids([5]));
    assert_eq!(part_frames(&p, 100), vec![(5, 106)]);

    assert!(rig.tick(1700).is_empty(), "同じ Show を二重に出さない");
}

/// 時計を書き換えない口（出来事の直後の `refresh`）は `-1` 以外の負の番号のコマを書かない（tasks.md 7.3 の申し送り）。
/// 見えない間に負のコマへ着いた子の面へ戻ると、部品のコマの無い `Show` が 1 件だけ出る。
#[test]
fn switch_back_onto_a_negative_frame_carries_no_part_frame() {
    let mut rig = Rig::from_text(NEGATIVE_CHILD, BindSet::from_ids([]));
    rig.surface(0, "0");
    rig.tick(0);
    let (_, _, p) = single_show(rig.tick(1000));
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    single_show(rig.surface(1000, "1"));
    assert!(rig.tick(1600).is_empty());

    let (sid, _, p) = single_show(rig.surface(1600, "0"));
    assert_eq!(sid, 0);
    assert!(p.is_empty(), "負の番号のコマは載らない: {p:?}");
}
