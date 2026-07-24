//! parse の単体テスト。
//!
//! 公開 facade `parse`（lexer→decode 結線）の契約を検証する:
//! - 空入力で空 `Shell`（要件 2.2）。
//! - 同一入力 → 同一出力の純粋・決定的（要件 2.4）。
//! - facade 結線（surface/element/collision/animation・append・alias が
//!   `parse()` 経由で期待モデルへ写像される・要件 2.1）。
//! - subset 外/不正断片でもパニックせず部分認識を返す（要件 9.3）。

use super::model::{
    Animation, AppendTarget, Collision, CollisionName, DefRef, DrawMethod, Element, ElementPath,
    Interval, Pattern, Shell, Surface, SurfaceAlias, SurfaceAppend,
};
use super::parse;

/// 空入力は空 `Shell`（surfaces/appends/aliases すべて空）を返す（要件 2.2）。
#[test]
fn empty_input_yields_empty_shell() {
    let shell = parse("");
    assert_eq!(
        shell,
        Shell {
            surfaces: vec![],
            appends: vec![],
            aliases: vec![],
            animation_sort: None,
            collision_sort: None,
            definitions: vec![],
        }
    );
}

/// 空白・ヘッダのみの入力も空 `Shell` を返す（寛容スキップ・要件 2.2/9.3）。
#[test]
fn header_only_input_yields_empty_shell() {
    let shell = parse("charset,UTF-8\ndescript\n{\n}\n");
    assert_eq!(shell.surfaces, vec![]);
    assert_eq!(shell.appends, vec![]);
    assert_eq!(shell.aliases, vec![]);
}

/// 同一の非自明入力を2回パースすると等価な `Shell` を返す（純粋・決定的・要件 2.4）。
#[test]
fn parse_is_deterministic() {
    let input = "\
surface0
{
element0,overlay,body.png,10,20
collision0,1,2,3,4,Head
animation0.interval,bind
animation0.pattern0,overlay,100,50,0,0
}

surface.append0
{
collision0,5,6,7,8,Bust
}

kero.surface.alias
{
smile,[0,1,2]
}
";
    let a = parse(input);
    let b = parse(input);
    assert_eq!(a, b);
}

/// facade 結線: surface（element/collision/animation）・append・alias が
/// `parse()` 経由で期待モデルへ写像される（lex→decode 結線の証明・要件 2.1）。
#[test]
fn facade_wires_lex_and_decode() {
    let input = "\
surface0
{
element0,overlay,body.png,10,20
collision0,1,2,3,4,Head
animation0.interval,bind
animation0.pattern0,overlay,100,50,0,0
}

surface.append0
{
collision0,5,6,7,8,Bust
}

kero.surface.alias
{
smile,[0,1,2]
}
";
    let shell = parse(input);

    let expected = Shell {
        surfaces: vec![Surface {
            id: 0,
            targets: vec![AppendTarget::Single(0)],
            elements: vec![Element {
                layer: 0,
                path: ElementPath::new("body.png".to_string()),
                x: 10,
                y: 20,
            }],
            collisions: vec![Collision {
                index: 0,
                left: 1,
                top: 2,
                right: 3,
                bottom: 4,
                name: CollisionName::new("Head".to_string()),
            }],
            animations: vec![Animation {
                id: 0,
                interval: Interval::Bind,
                patterns: vec![Pattern {
                    index: 0,
                    // task 1.2: overlay 行の method を忠実転記（field[1]＝"overlay"）。
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: 100,
                    wait: 50,
                    x: 0,
                    y: 0,
                }],
            }],
        }],
        appends: vec![SurfaceAppend {
            targets: vec![AppendTarget::Single(0)],
            elements: vec![],
            collisions: vec![Collision {
                index: 0,
                left: 5,
                top: 6,
                right: 7,
                bottom: 8,
                name: CollisionName::new("Bust".to_string()),
            }],
            animations: vec![],
        }],
        aliases: vec![SurfaceAlias {
            key: super::model::AliasKey::new("smile".to_string()),
            ids: vec![0, 1, 2],
        }],
        animation_sort: None,
        collision_sort: None,
        // 登場順ストリーム: surface0 → surface.append0 → kero.surface.alias（要件 12.5(d)）。
        definitions: vec![DefRef::Surface(0), DefRef::Append(0), DefRef::Alias(0)],
    };

    assert_eq!(shell, expected);
}

/// subset 外/不正断片を含む入力でもパニックせず、認識可能部分は保持する
/// （局所吸収・全域継続・部分認識・要件 9.3）。
#[test]
fn malformed_fragment_does_not_panic_and_keeps_partial() {
    // 壊れた行を含むが、正常な surface と element は認識される。
    let input = "\
surface0
{
element0,overlay,body.png,10,20
this_is_a_garbage_line_without_meaning
}
";
    let shell = parse(input);
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].id, 0);
    assert_eq!(shell.surfaces[0].elements.len(), 1);
    assert_eq!(shell.surfaces[0].elements[0].path.as_str(), "body.png");
}

// --- タスク 1.3: 公開 `parse()` 経由の method 忠実転記マトリクス＋Interval::Other 転記 ---
//
// タスク 1.2 の decode 分岐（overlay フィルタ撤去・fallback-Bind 撤去）が公開 facade
// `parse()` を通しても保たれることを end-to-end で確定させる（要件 2.1/4.6/8.2/8.4）。
// decode_tests は decode 単体を、本テストは lex→decode 結線後の観測面をピンする。

/// 公開 `parse()` 経由でも method 忠実転記マトリクス（overlay/replace/未知名/欠落）が
/// 全行保存される（要件 2.1/4.6/8.4）。overlay フィルタが復活すると 4 行が overlay 1 行に
/// 激減し len==4 が FAIL する（facade 側の行数保存の檻）。
#[test]
fn parse_transcribes_full_method_matrix() {
    let input = "\
surface0
{
animation0.interval,bind
animation0.pattern0,overlay,100,0,0,0
animation0.pattern1,replace,101,0,0,0
animation0.pattern2,frobnicate,102,0,0,0
animation0.pattern3
}
";
    let shell = parse(input);
    assert_eq!(shell.surfaces.len(), 1);
    let patterns = &shell.surfaces[0].animations[0].patterns;
    assert_eq!(patterns.len(), 4);
    let methods: Vec<&str> = patterns.iter().map(|p| p.method.as_str()).collect();
    // overlay/replace/未知名は verbatim・欠落は空文字（下流 Unknown 吸収）。
    assert_eq!(methods, vec!["overlay", "replace", "frobnicate", ""]);
}

/// 公開 `parse()` 経由でも未認識 interval キーワード（`sometimes`）は `Interval::Bind` へ
/// 倒れず `Interval::Other("sometimes")` へ忠実転記され、隣接する正当な animation
/// （`random,4`）は影響を受けず decode される（要件 2.1/8.2）。fallback-Bind が復活すると
/// interval==Bind となり Other アサートが FAIL する。
#[test]
fn parse_transcribes_unrecognized_interval_as_other_with_surviving_neighbor() {
    let input = "\
surface0
{
animation0.interval,sometimes,5
animation0.pattern0,overlay,10,0,0,0
animation1.interval,random,4
animation1.pattern0,overlay,20,0,0,0
}
";
    let shell = parse(input);
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].animations.len(), 2);
    // sometimes は Interval::Other へ忠実転記（Bind へ倒さない）・pattern は残る。
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::Other("sometimes".into())
    );
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 1);
    // 隣接する正当な animation は random,4 として decode される。
    assert_eq!(
        shell.surfaces[0].animations[1].interval,
        Interval::Random { k: 4 }
    );
}
