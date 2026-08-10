//! surface.append ターゲット指定の捕捉（元 decode_tests.rs タスク 4.4 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

use super::{
    Animation, AppendTarget, Collision, CollisionName, DrawMethod, Interval, Pattern, decode, lex,
};

// --- タスク 4.4: surface.append ターゲット指定の捕捉（展開しない転記） ---
//
// 検証範囲（要件 7.1/7.2/7.3）:
// - `surface.appendNNN,tgt,a-b,...` のターゲット指定を、ヘッダ数値 NNN を第1要素とし、
//   後続の単一 ID・範囲 a-b を同格の順序付き記述子リスト（Single/Range）として保持する。
//   範囲は展開しない・ヘッダのカテゴリ番号的特別扱いはしない（要件 7.2）。
// - 追記ブロック内の collision/animation を通常 surface と同一のモデル表現で保持する。
//   animation は decode_animations（タスク 4.3）の集約を再利用する（要件 7.3）。
// - 範囲展開と実 surface ツリーへの転記は下流に委ね、パーサは忠実な転記のみ行う（要件 7.2）。

/// 混在ターゲット（ヘッダ＋範囲＋範囲）＋ collision 捕捉（要件 7.1/7.2）。
/// `surface.append10,2100-2110,2200-2210` → [Single(10), Range{2100,2110}, Range{2200,2210}]。
/// ヘッダ数値 10 は第1要素として一様に Single で扱い、範囲は展開しない。
#[test]
fn append_mixed_targets_with_collision_are_captured() {
    let input = "\
surface.append10,2100-2110,2200-2210
{
collision0,52,38,156,80,Head
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(
        shell.appends[0].targets,
        vec![
            AppendTarget::Single(10),
            AppendTarget::Range {
                start: 2100,
                end: 2110,
            },
            AppendTarget::Range {
                start: 2200,
                end: 2210,
            },
        ]
    );
    // collision は通常 surface と同一表現（要件 7.3）。
    assert_eq!(
        shell.appends[0].collisions,
        vec![Collision {
            index: 0,
            left: 52,
            top: 38,
            right: 156,
            bottom: 80,
            name: CollisionName::new("Head".to_string()),
        }]
    );
    // surface には偽の append が混入しない。
    assert!(shell.surfaces.is_empty());
}

/// 列挙なしヘッダ（単一ヘッダ数値のみ）→ targets == [Single(2200)]・animation 集約（要件 7.2/7.3）。
#[test]
fn append_header_only_captures_single_target_and_animation() {
    let input = "\
surface.append2200
{
animation0.interval,random,4
animation0.pattern0,overlay,2206,0,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(shell.appends[0].targets, vec![AppendTarget::Single(2200)]);
    // animation は decode_animations の集約を再利用する（要件 7.3）。
    assert_eq!(
        shell.appends[0].animations,
        vec![Animation {
            id: 0,
            interval: Interval::Random { k: 4 },
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 2206,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }]
    );
    // collision は無い。
    assert!(shell.appends[0].collisions.is_empty());
}

/// 単一 ID 列挙（範囲でない）→ [Single(10), Single(2100)]（要件 7.2）。
#[test]
fn append_single_enumeration_targets() {
    let input = "surface.append10,2100\n{\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(
        shell.appends[0].targets,
        vec![AppendTarget::Single(10), AppendTarget::Single(2100)]
    );
}

/// 範囲は展開しない: [Single(10), Range{2100,2110}] は記述子 2 個（個別 ID 12 個ではない・要件 7.2）。
#[test]
fn append_range_is_not_expanded() {
    let input = "surface.append10,2100-2110\n{\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    // targets の長さは記述子数（2）であり、範囲展開後の個別 ID 数（1 + 11 = 12）ではない。
    assert_eq!(shell.appends[0].targets.len(), 2);
    assert_eq!(
        shell.appends[0].targets,
        vec![
            AppendTarget::Single(10),
            AppendTarget::Range {
                start: 2100,
                end: 2110,
            },
        ]
    );
}

/// append の animation はタスク 4.3 の集約（疎 index ＋負センチネル）を再利用する（要件 7.3）。
/// pattern0（surface_id 2106）＋ pattern3（surface_id -1）→ 1 個の Animation・index [0,3]・-1 保持。
#[test]
fn append_animation_reuses_sparse_and_negative_sentinel_aggregation() {
    let input = "\
surface.append2200
{
animation0.interval,random,4
animation0.pattern0,overlay,2106,0,0,0
animation0.pattern3,overlay,-1,80,0,0
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(shell.appends[0].animations.len(), 1);
    let anim = &shell.appends[0].animations[0];
    assert_eq!(anim.id, 0);
    let indices: Vec<u32> = anim.patterns.iter().map(|p| p.index).collect();
    assert_eq!(indices, vec![0, 3]);
    // 負センチネル -1 を i64 で失わず保持する。
    assert_eq!(anim.patterns[1].surface_id, -1);
}

/// 複数 append ブロックは出現順で保持する（要件 7.1）。
#[test]
fn multiple_append_blocks_keep_appearance_order() {
    let input = "\
surface.append10,2100
{
}
surface.append2200
{
}
surface.append2110
{
}
";
    let shell = decode(lex(input));
    assert_eq!(shell.appends.len(), 3);
    assert_eq!(
        shell.appends[0].targets,
        vec![AppendTarget::Single(10), AppendTarget::Single(2100)]
    );
    assert_eq!(shell.appends[1].targets, vec![AppendTarget::Single(2200)]);
    assert_eq!(shell.appends[2].targets, vec![AppendTarget::Single(2110)]);
}

/// append の collision は通常 surface の collision と完全に同一構造（同一 Collision 型・要件 7.3）。
#[test]
fn append_collision_uses_same_representation_as_surface() {
    let surface_input = "surface1000\n{\ncollision0,93,62,271,130,Head\n}\n";
    let append_input = "surface.append1000\n{\ncollision0,93,62,271,130,Head\n}\n";
    let surface_shell = decode(lex(surface_input));
    let append_shell = decode(lex(append_input));
    // 同一 collision 行は surface でも append でも同じ Collision 値になる。
    assert_eq!(
        append_shell.appends[0].collisions,
        surface_shell.surfaces[0].collisions
    );
}
