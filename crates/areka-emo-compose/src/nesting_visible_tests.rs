//! 参照の表と「今の絵に出ている部品」（要件 5.11・5.13・8.3）。
//!
//! 規則（`flatten_surface` と同じ辺）: 一番上から ① element定義の子 ② 有効な着せ替えの pattern0 の先
//! （同じ animation の番号にコマが在ればコマが置き換える）③ そのサーフェスのコマ（一番上は今までの欄・
//! 部品は部品の欄）のうち描画メソッドが動くもので、着せ替えの種類なら有効なものの先、をたどる。
//! 先祖へ戻る辺は進まない。
//!
//! 後半は合成との一致の檻（task 4.4・要件 3.2・5.12）: 「部品 X のコマを足すと命令列が変わる」⇔
//! 「X が部品の列に在る」・外れた着せ替えのコマは両方を変えない・合成が切る循環の辺は報告に在る。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface, parse};

use super::{NestIssue, NestTable, SurfaceParts};
use crate::bind::BindSet;
use crate::log_capture::capture_logs;
use crate::method::ComposeMethod;
use crate::pattern::{PatternFrame, PatternState};
use crate::plan::{BlitOp, derive_ops};
use crate::world::EmoWorld;

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

/// 検体の面の表（ファイル名の慣習だけで建つ 2・11 を含む）。
fn fixture_world() -> EmoWorld {
    let images = BTreeMap::from([
        (2, "surface2.png".to_string()),
        (11, "surface11.png".to_string()),
    ]);
    EmoWorld::build_with_images(&parse(FIXTURE), &images)
}

fn table_of(text: &str) -> NestTable {
    EmoWorld::build(&parse(text)).nest_table()
}

fn frame(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

fn visible(table: &NestTable, top: u32, binds: &[u32], pattern: &PatternState) -> Vec<u32> {
    let mut out = Vec::new();
    table.visible_parts(
        top,
        &BindSet::from_ids(binds.iter().copied()),
        pattern,
        &mut out,
    );
    out
}

fn listed(world: &EmoWorld, table: &NestTable) -> BTreeSet<u32> {
    world
        .surface_ids()
        .filter(|&id| table.parts(id).is_some())
        .collect()
}

/// 入れ子も着せ替えの種類の animation も無いシェルでは表が空・部品も無い。
#[test]
fn shell_without_nesting_or_binds_gives_empty_table() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,body.png,0,0\nanimation0.interval,random,2\nanimation0.pattern0,overlay,1,50,0,0\n}\n\
         surface1\n{\nelement0,overlay,eye.png,0,0\n}\n",
    );
    assert!(table.is_empty());
    assert_eq!(table, NestTable::default());
    // 一番上のコマ（random）の先は部品に数える（③）が、表は空のまま。
    let mut pattern = PatternState::default();
    pattern.set(0, frame(1));
    assert_eq!(visible(&table, 0, &[], &pattern), vec![1]);
    assert_eq!(
        visible(&table, 0, &[], &PatternState::default()),
        Vec::<u32>::new()
    );
}

/// 検体の表: 載るのは子か着せ替えを持つ面だけ。無い番号・範囲を超える数は子にならない。
#[test]
fn fixture_table_lists_children_and_bind_targets() {
    let world = fixture_world();
    let table = world.nest_table();

    assert_eq!(
        listed(&world, &table),
        BTreeSet::from([0, 1, 2, 30, 31, 60, 61, 62, 71]),
        "50（無い番号だけ）・10・40（random だけ）は載らない"
    );
    let children = |id: u32| table.parts(id).unwrap().children.clone();
    assert_eq!(children(0), vec![10, 30]);
    // surface.append1 の element2 が 30 を足す（element定義の番号の昇順）。
    assert_eq!(children(1), vec![10, 30]);
    // ファイル名の慣習だけで建つ 11 も「在る」。
    assert_eq!(children(2), vec![10, 11]);
    assert_eq!(children(30), vec![31]);
    assert_eq!(children(31), vec![32]);
    assert_eq!(children(60), vec![60]);
    assert_eq!(children(61), vec![62]);
    assert_eq!(children(62), vec![61]);
    assert_eq!(children(71), vec![70]);
    assert_eq!(
        table.parts(0),
        Some(&SurfaceParts {
            children: vec![10, 30],
            bind_targets: vec![(100, 40)],
            bind_ids: vec![100],
            ..Default::default()
        })
    );
}

/// 検体の部品: 仕込んだ子・3 段の入れ子・pattern定義の先が出る。
#[test]
fn fixture_visible_parts_follow_children_binds_and_frames() {
    let table = fixture_world().nest_table();
    let empty = PatternState::default();

    // ① 子と 3 段の入れ子。着せ替え 100 が無効なら 40 は出ない。
    assert_eq!(visible(&table, 0, &[], &empty), vec![10, 30, 31, 32]);
    // ② 有効な着せ替えの pattern0 の先。
    assert_eq!(visible(&table, 0, &[100], &empty), vec![10, 30, 31, 32, 40]);
    assert_eq!(visible(&table, 1, &[], &empty), vec![10, 30, 31, 32]);
    assert_eq!(visible(&table, 2, &[], &empty), vec![10, 11]);
    assert_eq!(visible(&table, 71, &[], &empty), vec![70]);
    // 部品の無い面。
    assert_eq!(visible(&table, 32, &[100], &empty), Vec::<u32>::new());

    // ③ 部品のコマの先（10 のまばたき→12・40 の口→41）。部品の欄から引く。
    let mut pattern = PatternState::default();
    pattern.set_part(10, 0, frame(12));
    pattern.set_part(40, 0, frame(41));
    assert_eq!(
        visible(&table, 0, &[100], &pattern),
        vec![10, 12, 30, 31, 32, 40, 41]
    );
    // 40 が出ていなければ、40 の部品のコマは数えない。
    assert_eq!(visible(&table, 0, &[], &pattern), vec![10, 12, 30, 31, 32]);
    // 一番上の欄の同じ番号のコマは部品のコマとして読まない（欄が混ざらない）。
    let mut top_only = PatternState::default();
    top_only.set(0, frame(12));
    assert_eq!(visible(&table, 1, &[], &top_only), vec![10, 12, 30, 31, 32]);
    let mut wrong_field = PatternState::default();
    wrong_field.set_part(1, 0, frame(12));
    assert_eq!(visible(&table, 1, &[], &wrong_field), vec![10, 30, 31, 32]);
}

/// ② 同じ animation の番号にコマが在れば、pattern0 の先でなくコマの先を数える。
#[test]
fn frame_replaces_bind_pattern0() {
    let table = fixture_world().nest_table();
    let mut pattern = PatternState::default();
    pattern.set(100, frame(41));
    assert_eq!(
        visible(&table, 0, &[100], &pattern),
        vec![10, 30, 31, 32, 41]
    );

    // 置き換えたコマが描かれないメソッドなら、どちらの先も数えない。
    let mut not_drawn = PatternState::default();
    not_drawn.set(
        100,
        PatternFrame {
            method: ComposeMethod::Reduce,
            ..frame(41)
        },
    );
    assert_eq!(visible(&table, 0, &[100], &not_drawn), vec![10, 30, 31, 32]);
}

/// ③ 有効でない着せ替えの種類のコマは数えない。着せ替えでないコマは無条件に数える。
#[test]
fn inactive_bind_frames_are_not_counted() {
    let text = "\
surface0
{
element0,overlay,body.png,0,0
element1,overlay,5,0,0
animation9.interval,bind
animation9.pattern0,overlay,6,0,0,0
}
surface5
{
element0,overlay,part.png,0,0
animation7.interval,bind+random,2
animation7.pattern1,overlay,6,50,0,0
animation8.interval,random,2
animation8.pattern1,overlay,7,50,0,0
}
surface6
{
element0,overlay,eye.png,0,0
}
surface7
{
element0,overlay,eye.png,0,0
}
";
    let table = table_of(text);
    // pattern0 を持たない bind+random だけの 5 も載る。
    assert_eq!(
        table.parts(5),
        Some(&SurfaceParts {
            children: vec![],
            bind_targets: vec![],
            bind_ids: vec![7],
            ..Default::default()
        })
    );

    // 一番上の欄: 無効な着せ替え 9 のコマは数えない。
    let mut top = PatternState::default();
    top.set(9, frame(7));
    assert_eq!(visible(&table, 0, &[], &top), vec![5]);
    assert_eq!(visible(&table, 0, &[9], &top), vec![5, 7]);

    // 部品の欄: 無効な着せ替え 7 のコマは数えない・random 8 は数える。
    let mut part = PatternState::default();
    part.set_part(5, 7, frame(6));
    part.set_part(5, 8, frame(7));
    assert_eq!(visible(&table, 0, &[], &part), vec![5, 7]);
    assert_eq!(visible(&table, 0, &[7], &part), vec![5, 6, 7]);

    // 描画メソッドが動かないコマは数えない。
    let mut not_drawn = PatternState::default();
    not_drawn.set_part(
        5,
        8,
        PatternFrame {
            method: ComposeMethod::Replace,
            ..frame(7)
        },
    );
    assert_eq!(visible(&table, 0, &[], &not_drawn), vec![5]);
}

/// 着せ替えの pattern0 の先は index 0・0 以上・描画メソッドが動くものだけ（欄 2 が animation の番号の語も除く）。
#[test]
fn bind_targets_keep_only_drawable_pattern0() {
    let text = "\
surface0
{
element0,overlay,body.png,0,0
animation1.interval,bind
animation1.pattern0,overlay,-1,0,0,0
animation2.interval,bind
animation2.pattern0,reduce,3,0,0,0
animation3.interval,bind
animation3.pattern0,insert,4,0,0,0
animation4.interval,bind
animation4.pattern1,overlay,3,0,0,0
animation5.interval,bind
animation5.pattern0,add,3,0,0,0
animation6.interval,random,2
animation6.pattern0,overlay,4,0,0,0
}
surface3
{
element0,overlay,eye.png,0,0
}
surface4
{
element0,overlay,eye.png,0,0
}
";
    let table = table_of(text);
    assert_eq!(
        table.parts(0),
        Some(&SurfaceParts {
            children: vec![],
            bind_targets: vec![(5, 3)],
            bind_ids: vec![1, 2, 3, 4, 5],
            ..Default::default()
        })
    );
    assert_eq!(
        visible(&table, 0, &[1, 2, 3, 4, 5, 6], &PatternState::default()),
        vec![3]
    );
}

/// 先祖へ戻る辺は進まない（自分自身・相互・コマが一番上へ戻る）。有限で終わる。
#[test]
fn back_edges_to_ancestors_are_not_followed() {
    let table = fixture_world().nest_table();
    let empty = PatternState::default();
    assert_eq!(visible(&table, 60, &[], &empty), Vec::<u32>::new());
    assert_eq!(visible(&table, 61, &[], &empty), vec![62]);
    assert_eq!(visible(&table, 62, &[], &empty), vec![61]);

    // 部品のコマが一番上・途中の先祖を指しても、そこへは戻らない。
    let mut pattern = PatternState::default();
    pattern.set_part(32, 0, frame(30));
    pattern.set_part(31, 0, frame(0));
    pattern.set_part(10, 0, frame(12));
    assert_eq!(visible(&table, 0, &[], &pattern), vec![10, 12, 30, 31, 32]);
}

/// 渡された列は先に空にしてから入れる（呼び手の列を使い回す）。
#[test]
fn out_is_cleared_and_reused() {
    let table = fixture_world().nest_table();
    let mut out = vec![999, 1, 2];
    table.visible_parts(2, &BindSet::default(), &PatternState::default(), &mut out);
    assert_eq!(out, vec![10, 11]);
    table.visible_parts(32, &BindSet::default(), &PatternState::default(), &mut out);
    assert!(out.is_empty());
}

// ── 合成との一致の檻（task 4.4・要件 3.2・5.11・5.12・5.13・8.3） ──
//
// 規則（`visible_parts`）と合成の再帰（`derive_ops` → `flatten_surface`）は別々に書いてあるので、
// 片方だけを変えるとずれる。ここでは両者を同じ面の表・同じ着せ替えの集合・同じコマの状態で動かし、
// 結論が一致することを全ての（一番上, 部品）の組で確かめる。

/// 目印のサーフェス（どこからも指されない・目印の画像を 1 枚だけ持つ）。
const MARKER: u32 = 900;
/// 目印のコマを置く animation の番号（どの文面にも無い番号）。
const MARK_ANIM: u32 = 9999;

/// 検体と小さな文面で使う画像の名前（どれも 1×1 の不透明）。
const RELS: &[&str] = &[
    "a.png",
    "b.png",
    "c.png",
    "body.png",
    "eye.png",
    "eye_closed.png",
    "part.png",
    "mouth.png",
    "mouth_open.png",
    "surface2.png",
    "surface11.png",
    "marker.png",
];

/// 1 つの面の表と、それを回す着せ替えの集合・コマの状態。
struct Case {
    name: &'static str,
    world: EmoWorld,
    atlas: AtlasTable,
    table: NestTable,
    binds: Vec<BindSet>,
    patterns: Vec<PatternState>,
    /// 先頭から何個のコマの状態が「文面の pattern定義どおりのコマだけ」か。循環の檻はこれだけを使う
    /// （定義に無い先を指すコマは、報告が知らない循環を作れる）。
    from_defs: usize,
}

impl Case {
    fn new(
        name: &'static str,
        text: &str,
        images: &[(u32, &str)],
        binds: &[&[u32]],
        patterns: Vec<PatternState>,
        from_defs: usize,
    ) -> Case {
        let text = format!("{text}\nsurface{MARKER}\n{{\nelement0,overlay,marker.png,0,0\n}}\n");
        let images: BTreeMap<u32, String> =
            images.iter().map(|(i, p)| (*i, p.to_string())).collect();
        let mut world = EmoWorld::build_with_images(&parse(&text), &images);
        let atlas = bake_atlas(Path::new("shell/master"), RELS);
        world.bind_atlas(&atlas, SetId(0));
        let table = world.nest_table();
        let binds = binds
            .iter()
            .map(|b| BindSet::from_ids(b.iter().copied()))
            .collect();
        Case {
            name,
            world,
            atlas,
            table,
            binds,
            patterns,
            from_defs,
        }
    }

    fn ops(&self, top: u32, binds: &BindSet, pattern: &PatternState) -> Vec<BlitOp> {
        let (mut ops, mut visited) = (Vec::new(), Vec::new());
        derive_ops(
            &mut ops,
            &mut visited,
            &self.world,
            &self.atlas,
            top,
            binds,
            pattern,
        );
        ops
    }

    fn parts(&self, top: u32, binds: &BindSet, pattern: &PatternState) -> Vec<u32> {
        let mut out = Vec::new();
        self.table.visible_parts(top, binds, pattern, &mut out);
        out
    }

    /// 面の表に在る番号（目印は除く・目印を一番上にすると目印のコマが先祖へ戻る）。
    fn ids(&self) -> Vec<u32> {
        self.world
            .surface_ids()
            .filter(|&id| id != MARKER)
            .collect()
    }

    /// 3.2 の報告の循環（親, element, 指した番号）。
    fn reported_cycles(&self) -> BTreeSet<(u32, u32, u32)> {
        self.world
            .nest_report()
            .issues
            .into_iter()
            .filter_map(|issue| match issue {
                NestIssue::Cycle {
                    surface,
                    element,
                    target,
                } => Some((surface, element, target)),
                NestIssue::MissingTarget { .. } => None,
            })
            .collect()
    }
}

/// 指定の rel_path 群を焼いた単一セットのアトラス（COM/WIC 非依存）。
fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
    let elements: Vec<Element> = rels
        .iter()
        .map(|r| Element {
            layer: 0,
            path: ElementPath::new(r.to_string()),
            x: 0,
            y: 0,
        })
        .collect();
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let mut dec = MemoryDecoder::new();
    for r in rels {
        dec.insert(base.join(r), 1, 1, 4, vec![1, 2, 3, 255], true);
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let result = bake(&[set], &dec, PackConfig::default());
    assert!(result.errors.is_empty(), "焼くのは失敗しない");
    result.table
}

fn not_drawn(surface_id: u32) -> PatternFrame {
    PatternFrame {
        method: ComposeMethod::Reduce,
        ..frame(surface_id)
    }
}

/// 検体: 子・3 段の入れ子・着せ替え 100 の pattern0 の先 40・40 の口 41・循環 60〜62。
fn fixture_case() -> Case {
    // 先頭 2 つは pattern定義どおりのコマだけ（10 のまばたき→12・40 の口→41）。
    let mut parts = PatternState::default();
    parts.set_part(10, 0, frame(12));
    parts.set_part(40, 0, frame(41));
    // 一番上のコマが着せ替え 100 の pattern0 を置き換える。
    let mut replaced = parts.clone();
    replaced.set(100, frame(41));
    // 一番上の欄の着せ替えでないコマ。
    let mut top_plain = PatternState::default();
    top_plain.set(0, frame(12));
    // 多段の内側・pattern定義の先のさらに先の部品のコマ（先祖へ戻るものを含む）。
    let mut deep = parts.clone();
    deep.set_part(32, 0, frame(12));
    deep.set_part(31, 0, frame(0));
    deep.set_part(41, 0, frame(30));
    // 描画メソッドが動かないコマ。
    let mut reduce = PatternState::default();
    reduce.set(100, not_drawn(41));
    reduce.set_part(10, 0, not_drawn(12));
    Case::new(
        "検体",
        FIXTURE,
        &[(2, "surface2.png"), (11, "surface11.png")],
        &[&[], &[100]],
        vec![
            PatternState::default(),
            parts,
            replaced,
            top_plain,
            deep,
            reduce,
        ],
        2,
    )
}

/// 子の中の着せ替え・bind+random・random・pattern定義の先の中の子が混ざる文面（循環なし）。
fn layered_case() -> Case {
    let text = "\
surface0
{
element0,overlay,body.png,0,0
element1,overlay,5,10,10
animation9.interval,bind
animation9.pattern0,overlay,6,0,0,0
animation3.interval,random,2
animation3.pattern0,overlay,8,50,0,0
}
surface5
{
element0,overlay,part.png,0,0
element1,overlay,7,1,1
animation7.interval,bind+random,2
animation7.pattern1,overlay,6,50,0,0
animation8.interval,random,2
animation8.pattern1,overlay,8,50,0,0
}
surface6
{
element0,overlay,eye.png,0,0
animation4.interval,bind
animation4.pattern0,overlay,9,0,2,2
}
surface7
{
element0,overlay,mouth.png,0,0
animation1.interval,random,2
animation1.pattern0,overlay,9,50,0,0
}
surface8
{
element0,overlay,c.png,0,0
}
surface9
{
element0,overlay,a.png,0,0
element1,overlay,8,3,3
}
";
    let mut top_random = PatternState::default();
    top_random.set(3, frame(8));
    let mut parts = PatternState::default();
    parts.set_part(5, 8, frame(8));
    parts.set_part(5, 7, frame(6));
    parts.set_part(7, 1, frame(9));
    let mut replaced = parts.clone();
    replaced.set(9, frame(8));
    replaced.set_part(6, 4, frame(8));
    let mut reduce = PatternState::default();
    reduce.set_part(5, 7, not_drawn(6));
    reduce.set_part(7, 1, not_drawn(9));
    Case::new(
        "多段と着せ替え",
        text,
        &[],
        &[&[], &[9], &[7], &[4], &[9, 7, 4]],
        // replaced だけが定義に無い先を指す（9 の pattern0 は 6・4 の pattern0 は 9）。
        vec![PatternState::default(), top_random, parts, reduce, replaced],
        4,
    )
}

/// element定義と pattern定義が混ざった循環（0⇄5・6⇄7・20→21→22→20）。
fn mixed_cycle_case() -> Case {
    let text = "\
surface0
{
element0,overlay,a.png,0,0
element1,overlay,5,0,0
}
surface5
{
element0,overlay,b.png,0,0
animation1.interval,bind
animation1.pattern0,overlay,0,0,0,0
}
surface6
{
element0,overlay,c.png,0,0
element1,overlay,7,0,0
}
surface7
{
element0,overlay,eye.png,0,0
animation2.interval,random,2
animation2.pattern0,overlay,6,50,0,0
}
surface20
{
element0,overlay,a.png,0,0
element1,overlay,21,0,0
}
surface21
{
element0,overlay,b.png,0,0
element1,overlay,22,0,0
}
surface22
{
element0,overlay,c.png,0,0
animation5.interval,bind
animation5.pattern0,overlay,20,0,0,0
}
";
    let mut top = PatternState::default();
    top.set(2, frame(6));
    let mut part = PatternState::default();
    part.set_part(7, 2, frame(6));
    Case::new(
        "混ざった循環",
        text,
        &[],
        &[&[], &[1], &[5], &[1, 5]],
        vec![PatternState::default(), top, part],
        3,
    )
}

fn cases() -> [Case; 3] {
    [fixture_case(), layered_case(), mixed_cycle_case()]
}

/// 「部品 X のコマを足すと命令列が変わる」⇔「X が今の絵に出ている部品に在る」（要件 5.11・5.13）。
///
/// 足すコマは、X の部品の欄の、どの文面にも無い animation の番号に、目印のサーフェスを指す Overlay。
/// X の段が合成されれば必ず目印の命令が 1 つ増え、合成されなければ何も変わらない。一番上 T と
/// 面の表に在る X の全ての組・全ての着せ替えの集合・全てのコマの状態で確かめる。
#[test]
fn adding_a_part_frame_changes_ops_iff_part_is_visible() {
    for case in cases() {
        let ids = case.ids();
        let (mut seen_visible, mut seen_hidden) = (0, 0);
        for binds in &case.binds {
            for (k, pattern) in case.patterns.iter().enumerate() {
                for &top in &ids {
                    let base = case.ops(top, binds, pattern);
                    let visible = case.parts(top, binds, pattern);
                    for &x in &ids {
                        let mut marked = pattern.clone();
                        marked.set_part(x, MARK_ANIM, frame(MARKER));
                        let changed = case.ops(top, binds, &marked) != base;
                        let listed = visible.binary_search(&x).is_ok();
                        assert_eq!(
                            changed, listed,
                            "{}: 一番上 {top}・部品 {x}・着せ替え {binds:?}・コマの状態 {k}・部品の列 {visible:?}",
                            case.name
                        );
                        if listed {
                            seen_visible += 1;
                        } else {
                            seen_hidden += 1;
                        }
                    }
                }
            }
        }
        assert!(
            seen_visible > 0 && seen_hidden > 0,
            "{}: 両側の行が在る",
            case.name
        );
    }
}

/// 一致の檻が空振りしない: 部品の列に pattern定義の先・その先の部品のコマの先・多段の内側が並ぶ。
#[test]
fn equivalence_cage_reaches_pattern_targets_and_inner_part_frames() {
    let case = fixture_case();
    let binds = BindSet::from_ids([100]);
    // 40（pattern定義の先）・41（40 の部品のコマの先）・12（10 の部品のコマの先）。
    assert_eq!(
        case.parts(0, &binds, &case.patterns[1]),
        vec![10, 12, 30, 31, 32, 40, 41]
    );
    // 一番上 1 は着せ替え 100 を持たないので、同じ集合でも 40 は出ない。
    assert_eq!(
        case.parts(1, &binds, &case.patterns[1]),
        vec![10, 12, 30, 31, 32]
    );
}

/// 着せ替えの集合から外れた着せ替えの種類のコマ（一番上の欄・部品の欄）を足しても、命令列も
/// 部品の列も変わらない（要件 5.12）。
#[test]
fn inactive_bind_frames_change_neither_ops_nor_parts() {
    for case in cases() {
        let ids = case.ids();
        let mut rows = 0;
        for binds in &case.binds {
            for (k, pattern) in case.patterns.iter().enumerate() {
                for &top in &ids {
                    let base_ops = case.ops(top, binds, pattern);
                    let base_parts = case.parts(top, binds, pattern);
                    for &s in &ids {
                        let Some(parts) = case.table.parts(s) else {
                            continue;
                        };
                        for &b in parts.bind_ids.iter().filter(|&&b| !binds.contains(b)) {
                            let mut added = pattern.clone();
                            added.set_part(s, b, frame(MARKER));
                            if s == top {
                                added.set(b, frame(MARKER));
                            }
                            let at = format!(
                                "{}: 一番上 {top}・面 {s}・着せ替えの種類 {b}・着せ替え {binds:?}・コマの状態 {k}",
                                case.name
                            );
                            assert_eq!(case.ops(top, binds, &added), base_ops, "{at}");
                            assert_eq!(case.parts(top, binds, &added), base_parts, "{at}");
                            rows += 1;
                        }
                    }
                }
            }
        }
        assert!(rows > 0, "{}: 外れた着せ替えの行が在る", case.name);
    }
}

/// 合成が先祖へ戻るとして切った element定義の辺（`debug!` の行から）を (親, element, 子) で返す。
fn cut_element_edges(logs: &str) -> Vec<(u32, u32, u32)> {
    logs.lines()
        .filter(|l| l.contains("level=DEBUG") && l.contains("子のサーフェスを置かない"))
        .filter(|l| l.contains("先祖へ戻る参照"))
        .map(|l| {
            let field = |name: &str| -> u32 {
                let start = l.find(&format!(" {name}=")).expect(name) + name.len() + 2;
                l[start..]
                    .split(' ')
                    .next()
                    .unwrap()
                    .trim_matches('"')
                    .parse()
                    .unwrap()
            };
            (field("surface_id"), field("element"), field("child"))
        })
        .collect()
}

/// 合成が実際に切る循環の element定義の辺は、どの一番上から見ても 3.2 の報告に在る（要件 3.2・8.3）。
/// pattern定義の辺で切る循環（`warn!`）は報告の対象でない。検体と文面では、全ての一番上で切った辺を
/// 合わせると報告の循環にちょうどなる。
#[test]
fn cut_cycle_edges_from_every_top_are_reported() {
    for case in cases() {
        let report = case.reported_cycles();
        let mut cut_all = BTreeSet::new();
        for binds in &case.binds {
            for (k, pattern) in case.patterns[..case.from_defs].iter().enumerate() {
                for top in case.ids() {
                    let logs = capture_logs(|| {
                        case.ops(top, binds, pattern);
                    });
                    for edge in cut_element_edges(&logs) {
                        assert!(
                            report.contains(&edge),
                            "{}: 一番上 {top}・着せ替え {binds:?}・コマの状態 {k} で切った辺 {edge:?} が報告 {report:?} に在る",
                            case.name
                        );
                        cut_all.insert(edge);
                    }
                }
            }
        }
        assert_eq!(cut_all, report, "{}: 切った辺の和＝報告の循環", case.name);
    }
}
