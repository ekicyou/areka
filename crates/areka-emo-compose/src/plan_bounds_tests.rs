//! シェル全体の外形 [`shell_bounds`] の試験（areka-P0-offline-surface-dump 要件 4.3）。
//!
//! 数は SSP の測り（research.md §10.6・`g_anim` の 18×14・`g_size` の 12×14）と同じ形の検体で固める。

use areka_emo_atlas::AtlasTable;
use areka_parsers::shell::parse;

use super::{ShellBounds, shell_bounds};
use crate::Composer;
use crate::bind::BindSet;
use crate::log_capture::capture_logs;
use crate::pattern::PatternState;
use crate::plan_test_support::{bake_gray, bound_world};
use crate::world::EmoWorld;

/// 画像の名前と原寸（全面不透明）。
const SIZES: &[(&str, (u32, u32))] = &[
    ("base16.png", (16, 8)),
    ("base8.png", (8, 8)),
    ("red4.png", (4, 4)),
    ("green4.png", (4, 4)),
];

/// 文面を面の表にし、アトラスを当てて返す。
fn world_of(text: &str) -> (EmoWorld, AtlasTable) {
    let atlas = bake_gray(SIZES);
    (bound_world(&parse(text), &[], &atlas), atlas)
}

/// どの面の合成の絵の大きさも外形の右下を超えない（design ShellCanvasBounds の Invariants）。
fn assert_every_surface_fits(world: &EmoWorld, atlas: &AtlasTable, b: ShellBounds) {
    let mut composer = Composer::new();
    for id in world.surface_ids() {
        let out = composer
            .compose(
                world,
                atlas,
                id,
                &BindSet::default(),
                &PatternState::default(),
            )
            .unwrap_or_else(|e| panic!("surface{id} が合成できない: {e:?}"));
        assert!(
            out.width() as i64 <= b.right as i64 && out.height() as i64 <= b.bottom as i64,
            "surface{id} の絵 {}x{} が外形 {b:?} の右下を超える",
            out.width(),
            out.height()
        );
    }
}

/// `g_anim` と同じ形: 面 20〜22 の負の座標・はみ出しの element と、面 23 の runonce の pattern
/// （4,4 に 8×8）が、はみ出しの無い面 0 を含むシェル全体の外形を x −2〜16・y −2〜12（18×14）にする。
#[test]
fn g_anim_shape_is_18x14() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n}\r\n\
         surface20\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,red4.png,-2,-2\r\nelement2,overlay,green4.png,6,6\r\n}\r\n\
         surface21\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,green4.png,6,6\r\n}\r\n\
         surface22\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,red4.png,-2,-2\r\n}\r\n\
         surface23\r\n{\r\nelement0,overlay,base8.png,0,0\r\nanimation0.interval,runonce\r\nanimation0.pattern0,overlay,100,50,4,4\r\n}\r\n\
         surface100\r\n{\r\nelement0,overlay,base8.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_eq!(
        b,
        ShellBounds {
            left: -2,
            top: -2,
            right: 16,
            bottom: 12
        }
    );
    assert_eq!((b.right - b.left, b.bottom - b.top), (18, 14));
    assert_every_surface_fits(&world, &atlas, b);
}

/// `g_size` と同じ形: 面 1 の −2,−3 の 4×4 と、面 2 の `never` の pattern（6,7 に 4×4）が範囲に入り
/// x −2〜10・y −3〜11（12×14）になる。
#[test]
fn g_size_shape_with_never_is_12x14() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base8.png,0,0\r\n}\r\n\
         surface1\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,red4.png,-2,-3\r\n}\r\n\
         surface2\r\n{\r\nelement0,overlay,base8.png,0,0\r\nanimation0.interval,never\r\nanimation0.pattern0,overlay,101,0,6,7\r\n}\r\n\
         surface101\r\n{\r\nelement0,overlay,green4.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_eq!(
        b,
        ShellBounds {
            left: -2,
            top: -3,
            right: 10,
            bottom: 11
        }
    );
    assert_every_surface_fits(&world, &atlas, b);
}

/// はみ出しの無いシェルは土台の大きさのまま。
#[test]
fn shell_without_overflow_is_base_size() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n}\r\n\
         surface1\r\n{\r\nelement0,overlay,base16.png,0,0\r\nelement1,overlay,red4.png,12,4\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_eq!(
        b,
        ShellBounds {
            left: 0,
            top: 0,
            right: 16,
            bottom: 8
        }
    );
    assert_every_surface_fits(&world, &atlas, b);
}

/// 入れ子（element定義の子・pattern の先の面がさらに pattern を持つ）は位置を足して数え、負の番号と
/// 描かないメソッドの pattern は数えず、循環（element定義も pattern も）は黙って打ち切る（記録を出さない）。
/// pattern の循環は `sometimes` に置く: `walk` はどの interval も辿るが、外形の表（束縛のときに
/// `compute_extent` が作る）は bind・`always` だけを辿る（bind の循環でも記録を出さないことは
/// `bind_cycle_emits_no_warn`）。
#[test]
fn nesting_adds_offsets_and_skips_non_drawing() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base8.png,0,0\r\n\
         element1,overlay,30,-5,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlay,40,0,0,10\r\n\
         animation1.interval,sometimes\r\nanimation1.pattern0,overlay,-1,0,500,500\r\nanimation1.pattern1,move,101,0,500,500\r\nanimation1.pattern2,start,101,0,500,500\r\n}\r\n\
         surface30\r\n{\r\nelement0,overlay,red4.png,0,0\r\n}\r\n\
         surface40\r\n{\r\nanimation0.interval,always\r\nanimation0.pattern0,overlay,41,0,3,0\r\n}\r\n\
         surface41\r\n{\r\nelement0,overlay,green4.png,0,0\r\nelement1,overlay,41,0,0\r\n}\r\n\
         surface101\r\n{\r\nelement0,overlay,green4.png,0,0\r\nanimation0.interval,sometimes\r\nanimation0.pattern0,overlay,101,0,0,0\r\n}\r\n",
    );
    let mut b = None;
    let logs = capture_logs(|| b = Some(shell_bounds(&world, &atlas)));
    // x: 30 を −5 に置く → −5。y: 0→40 を (0,10)・40→41 を (3,0) で 41 の 4×4 は (3,10)〜(7,14)。
    assert_eq!(
        b,
        Some(ShellBounds {
            left: -5,
            top: 0,
            right: 8,
            bottom: 14
        })
    );
    assert!(logs.trim().is_empty(), "外形は記録を出さない: {logs}");
}

/// 純 `bind` の pattern は描画メソッドを問わず面の絵の大きさ（外形の表）に数えられる。`overlayfast` で
/// 外へはみ出しても、外形はその絵を収める（Invariants）。
#[test]
fn pure_bind_pattern_non_overlay_still_fits() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlayfast,10,0,30,0\r\n}\r\n\
         surface10\r\n{\r\nelement0,overlay,red4.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    // 30,0 に置いた 4×4 が右端を 34 にする（数えなければ土台の 16 のまま）。
    assert_eq!(
        b,
        ShellBounds {
            left: 0,
            top: 0,
            right: 34,
            bottom: 8
        }
    );
    assert_every_surface_fits(&world, &atlas, b);
}

/// `always` の pattern も描画メソッドを問わず面の絵の大きさに数えられる。`interpolate` で外へ
/// はみ出しても、外形はその絵を収める（Invariants）。
#[test]
fn always_pattern_non_overlay_still_fits() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n\
         animation0.interval,always\r\nanimation0.pattern0,interpolate,10,100,0,20\r\n}\r\n\
         surface10\r\n{\r\nelement0,overlay,red4.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    // 0,20 に置いた 4×4 が下端を 24 にする（数えなければ土台の 8 のまま）。
    assert_eq!(
        b,
        ShellBounds {
            left: 0,
            top: 0,
            right: 16,
            bottom: 24
        }
    );
    assert_every_surface_fits(&world, &atlas, b);
}

/// 着せ替えの pattern が循環するシェルでも、外形は記録を出さない（`shell_bounds` は面ごとの外形を表から
/// 引くだけ・表を作るときの循環の知らせは束縛のときの debug）。
#[test]
fn bind_cycle_emits_no_warn() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base8.png,0,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlay,10,0,0,0\r\n}\r\n\
         surface10\r\n{\r\nelement0,overlay,red4.png,0,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlay,0,0,0,0\r\n}\r\n",
    );
    let mut b = None;
    let logs = capture_logs(|| b = Some(shell_bounds(&world, &atlas)));
    assert!(logs.trim().is_empty(), "外形は記録を出さない: {logs}");
    assert_every_surface_fits(&world, &atlas, b.expect("外形"));
}

/// 広がりの予算（指摘 3）: 子の面は入る 1 ＋行 1 つで 2 を数える（指摘 130）。予算 4 のシェルの外形は、負の位置に
/// 置いた element定義の子 3 つのうち 2 つ目まで入って 3 つ目を数えず、`error!` を 1 行だけ出す。予算 6 なら 3 つとも数える。
#[test]
fn spent_budget_leaves_children_not_yet_entered_out_of_the_bounds() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,10,-10,0\r\nelement1,overlay,10,-20,0\r\nelement2,overlay,10,-30,0\r\n}\r\n\
         surface10\r\n{\r\nelement0,overlay,red4.png,0,0\r\n}\r\n",
    );
    let run = |limit| {
        let mut b = None;
        let logs = capture_logs(|| b = Some(super::shell_bounds_limited(&world, &atlas, limit)));
        (b.expect("外形"), logs)
    };
    let (b, logs) = run(4);
    assert_eq!(b.left, -20, "{b:?}");
    assert_eq!(logs.matches("level=ERROR").count(), 1, "{logs}");
    assert!(
        logs.contains("walk=\"bounds\"") && logs.contains("root=0"),
        "{logs}"
    );
    let (b, logs) = run(6);
    assert_eq!(b.left, -30, "{b:?}");
    assert!(!logs.contains("level=ERROR"), "{logs}");
}
