//! animationN 集約（interval 3 種・疎 pattern・負センチネル）。

use super::{Animation, DrawMethod, Interval, Pattern, decode, lex};

// runonce / periodic,数値 / talk,数値 の読みは兄弟の decode_interval_trigger_tests.rs。

/// `animationN.interval,bind` ＋ 単一 pattern → interval Bind・pattern 正規化。
#[test]
fn animation_bind_interval_with_single_pattern() {
    let input =
        "surface1\n{\nanimation100.interval,bind\nanimation100.pattern0,overlay,1100,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(
        shell.surfaces[0].animations,
        vec![Animation {
            id: 100,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 1100,
                target: "1100".into(),
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
}

/// `animationN.interval,random,K` → Interval::Random{k}。
#[test]
fn animation_random_interval_parses_k() {
    let input =
        "surface1\n{\nanimation0.interval,random,4\nanimation0.pattern0,overlay,10,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 0);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::Random { k: 4 }
    );
}

/// `animationN.interval,bind+random,K` → Interval::BindRandom{k}。
#[test]
fn animation_bind_random_interval_parses_k() {
    let input = "surface1\n{\nanimation1400.interval,bind+random,4\nanimation1400.pattern1,overlay,1410,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 1400);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::BindRandom { k: 4 }
    );
}

/// 疎 pattern（pattern0/pattern1/pattern3・pattern2 欠番）→ index [0,1,3] を合成せず保持。
#[test]
fn sparse_pattern_indices_are_preserved_not_synthesized() {
    let input = "\
surface1
{
animation0.interval,random,4
animation0.pattern0,overlay,10,50,0,0
animation0.pattern1,overlay,11,60,0,0
animation0.pattern3,overlay,13,70,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    let indices: Vec<u32> = shell.surfaces[0].animations[0]
        .patterns
        .iter()
        .map(|p| p.index)
        .collect();
    assert_eq!(indices, vec![0, 1, 3]);
}

/// 負のサーフェス参照 ID（`overlay,-1`）→ surface_id を i64 の -1 として失わず保持。
#[test]
fn negative_surface_id_is_preserved_as_sentinel() {
    let input =
        "surface1\n{\nanimation0.interval,random,4\nanimation0.pattern3,overlay,-1,80,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(
        shell.surfaces[0].animations[0].patterns,
        vec![Pattern {
            index: 3,
            method: DrawMethod::new("overlay".to_string()),
            surface_id: -1,
            target: "-1".into(),
            wait: 80,
            x: 0,
            y: 0,
        }]
    );
}

/// 複数 animation は id 初出順で集約される。
#[test]
fn multiple_animations_aggregate_in_first_appearance_order() {
    let input = "\
surface1000
{
animation1100.interval,bind
animation1100.pattern0,overlay,1100,0,0,0
animation1200.interval,bind
animation1200.pattern0,overlay,1200,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 2);
    assert_eq!(shell.surfaces[0].animations[0].id, 1100);
    assert_eq!(shell.surfaces[0].animations[1].id, 1200);
}

/// 同一 animation id の複数 pattern 行は同一 animation の patterns へ束ねる（別 animation にしない）。
#[test]
fn multiple_pattern_lines_bind_into_same_animation() {
    let input = "\
surface1000
{
animation1400.interval,bind+random,4
animation1400.pattern1,overlay,1410,0,0,0
animation1400.pattern2,overlay,1420,0,0,0
animation1400.pattern3,overlay,1430,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 1400);
    let indices: Vec<u32> = shell.surfaces[0].animations[0]
        .patterns
        .iter()
        .map(|p| p.index)
        .collect();
    assert_eq!(indices, vec![1, 2, 3]);
}

/// interval 行が pattern 行より後でも同一 id へ正しく束ねる（順序寛容・堅牢性）。
#[test]
fn interval_after_patterns_still_binds() {
    let input = "\
surface1
{
animation5.pattern0,overlay,50,0,0,0
animation5.pattern1,overlay,51,0,0,0
animation5.interval,random,2
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 5);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::Random { k: 2 }
    );
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 2);
}

/// interval 行が無い（pattern のみ）animation → 既定 Interval::Bind で pattern を失わない（堅牢性）。
#[test]
fn animation_without_interval_defaults_to_bind() {
    let input = "surface1\n{\nanimation7.pattern0,overlay,70,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(shell.surfaces[0].animations[0].id, 7);
    assert_eq!(shell.surfaces[0].animations[0].interval, Interval::Bind);
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 1);
}

/// ウェイトの `最小-最大` は小さい方を待ちにして `WaitRange` を積む（ランダムは未対応）。
/// 両側が数でないものは今どおり 0（崩れは積まない）。
#[test]
fn pattern_wait_range_takes_minimum_and_is_reported() {
    let input = "\
surface0
{
animation1.interval,bind
animation1.pattern0,overlay,10, 50 - 20 ,0,0
animation1.pattern1,overlay,11,abc-5,0,0
animation1.pattern2,overlay,12,30,0,0
}
";
    let text = crate::shell::parse_all(input);
    let waits: Vec<u32> = text.shell.surfaces[0].animations[0]
        .patterns
        .iter()
        .map(|p| p.wait)
        .collect();
    assert_eq!(waits, vec![20, 0, 30]);
    assert_eq!(
        text.issues,
        vec![crate::shell::ShellIssue::WaitRange {
            heading: "surface0".into(),
            text: "50 - 20".into(),
        }]
    );
    assert_eq!(text.issues[0].severity(), tracing::Level::WARN);
}
