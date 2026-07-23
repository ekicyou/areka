//! model の単体テスト。
//!
//! 本モジュールは `model` とは別モジュールであり、`super::model::{...}`
//! 経由で型へアクセスする（公開面 `pub use` の集約はタスク 5.1 の責務ゆえ
//! ここではモジュールパスで直接参照する）。これにより
//! 「不透明 NewType は read-only アクセサで中身を読める」「公開 enum の
//! `#[non_exhaustive]` シームを網羅できる」「派生（Clone/Debug/PartialEq）が
//! 全公開型で機能する」という下流共有 I/O 契約（done 基準）を固定する。

#![cfg(test)]

use super::model::{
    Animation, AppendTarget, AliasKey, Collision, CollisionName, DrawMethod, Element, ElementPath,
    Interval, Pattern, Shell, Surface, SurfaceAlias, SurfaceAppend,
};

// --- 1. 不透明 NewType の read-only 契約（要件 1.3/4.3/6.2/8.2） ---

#[test]
fn element_path_is_opaque_and_verbatim() {
    // `/` と `\` を含む生パスを無加工・無トリムで保持する（要件 4.3）。
    let p = ElementPath::new("surface/normal\\face0.png".to_string());
    assert_eq!(p.as_str(), "surface/normal\\face0.png");
}

#[test]
fn element_path_preserves_surrounding_whitespace() {
    // トリムしない（無加工保持）ことを前後空白で確認する。
    let p = ElementPath::new("  a/b.png  ".to_string());
    assert_eq!(p.as_str(), "  a/b.png  ");
}

#[test]
fn alias_key_is_opaque_and_verbatim() {
    // 数値・日本語いずれのキーも解釈せず生保持する（要件 8.2）。
    let k = AliasKey::new("笑顔".to_string());
    assert_eq!(k.as_str(), "笑顔");
}

#[test]
fn collision_name_is_opaque_and_verbatim() {
    // 領域名（Head/Bust 等）を意味解釈せず生保持する（要件 6.2）。
    let n = CollisionName::new("Head".to_string());
    assert_eq!(n.as_str(), "Head");
}

/// タスク 1.3: `DrawMethod` は妥当性を判定しない不透明 NewType。既知語（overlay）・
/// 別メソッド（replace/base）・未知名（frobnicate）・欠落相当の空文字を、いずれも
/// 意味解釈せず verbatim に保持する（要件 4.6/8.4・parser は原文を運ぶだけ）。
#[test]
fn draw_method_is_opaque_and_verbatim() {
    for m in ["overlay", "replace", "base", "frobnicate", ""] {
        assert_eq!(DrawMethod::new(m.to_string()).as_str(), m);
    }
}

// --- 2. Interval の #[non_exhaustive] variant 網羅（要件 1.4/5.1-5.3） ---

#[test]
fn interval_variants_construct_and_match() {
    // 定義クレート内からは #[non_exhaustive] enum も網羅 match 可能。
    for iv in [Interval::Bind, Interval::Random { k: 5 }, Interval::BindRandom { k: 3 }] {
        match iv {
            Interval::Bind => {}
            Interval::Random { k } => assert_eq!(k, 5),
            Interval::BindRandom { k } => assert_eq!(k, 3),
            Interval::Other(_) => {}
        }
    }
}

#[test]
fn interval_variants_compare_by_value() {
    assert_eq!(Interval::Bind, Interval::Bind);
    assert_eq!(Interval::Random { k: 5 }, Interval::Random { k: 5 });
    assert_ne!(Interval::Random { k: 5 }, Interval::Random { k: 6 });
    assert_ne!(Interval::Bind, Interval::BindRandom { k: 0 });
}

/// タスク 1.3: `Interval::Other` は未認識語彙の原文忠実転記のための **独立 variant** で
/// あり、`Bind` へ縮退しない（討議 #1 裁定・fallback-Bind 撤去の model 面の檻・要件 8.2）。
/// - `Other(kw)` は `Bind`（および他 variant）と値等価にならない。
/// - 保持語彙は原文どおりで、`Other("sometimes") != Other("periodic")`。
/// - 定義クレート内なら `#[non_exhaustive]` enum も網羅 match でき、原文を取り出せる。
#[test]
fn interval_other_is_distinct_variant_and_preserves_keyword() {
    let other = Interval::Other("sometimes".into());
    // Bind へ縮退しない（fallback-Bind 撤去の証明・要件 8.2）。
    assert_ne!(other, Interval::Bind);
    assert_ne!(other, Interval::Random { k: 0 });
    // 語彙は原文保持・別語彙とは非等価。
    assert_eq!(other, Interval::Other("sometimes".into()));
    assert_ne!(other, Interval::Other("periodic".into()));
    // 網羅 match で原文語彙を取り出せる。
    match other {
        Interval::Other(kw) => assert_eq!(&*kw, "sometimes"),
        _ => panic!("expected Interval::Other"),
    }
}

// --- 3. AppendTarget のシーム（要件 7.2・記述子で保持・展開しない） ---

#[test]
fn append_target_single_and_range() {
    let single = AppendTarget::Single(10);
    let range = AppendTarget::Range { start: 2100, end: 2110 };
    match single {
        AppendTarget::Single(id) => assert_eq!(id, 10),
        _ => panic!("expected Single"),
    }
    match range {
        AppendTarget::Range { start, end } => {
            assert_eq!(start, 2100);
            assert_eq!(end, 2110);
        }
        _ => panic!("expected Range"),
    }
    // 記述子どうしは値等価。
    assert_eq!(AppendTarget::Single(10), AppendTarget::Single(10));
    assert_ne!(
        AppendTarget::Single(10),
        AppendTarget::Range { start: 10, end: 10 }
    );
}

// --- 4. 派生（Clone / Debug / PartialEq）の確認（要件 1.5） ---

#[test]
fn derives_clone_debug_partial_eq() {
    let s = Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new("a/b.png".to_string()),
            x: 1,
            y: 2,
        }],
        collisions: vec![],
        animations: vec![],
    };
    // Clone + PartialEq。
    let cloned = s.clone();
    assert_eq!(s, cloned);
    // Debug（フィールドマーカを含む非空出力）。
    let dbg = format!("{s:?}");
    assert!(!dbg.is_empty());
    assert!(dbg.contains("Surface"));
    assert!(dbg.contains("elements"));
}

// --- 5. 組み立てた Shell 値の構造アサーション（要件 8.4 の重複キー保持含む） ---

#[test]
fn assembled_shell_preserves_structure_and_duplicate_alias_keys() {
    let shell = Shell {
        surfaces: vec![Surface {
            id: 0,
            targets: vec![AppendTarget::Single(0)],
            elements: vec![Element {
                layer: 0,
                path: ElementPath::new("surface0.png".to_string()),
                x: 0,
                y: 0,
            }],
            collisions: vec![Collision {
                index: 0,
                left: 10,
                top: 20,
                right: 100,
                bottom: 200,
                name: CollisionName::new("Head".to_string()),
            }],
            animations: vec![Animation {
                id: 4,
                interval: Interval::Random { k: 4 },
                patterns: vec![Pattern {
                    index: 0,
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: -1, // 負値センチネル保持（要件 5.5）
                    wait: 100,
                    x: 0,
                    y: 0,
                }],
            }],
        }],
        appends: vec![SurfaceAppend {
            targets: vec![
                AppendTarget::Single(10),
                AppendTarget::Range { start: 2100, end: 2110 },
            ],
            elements: vec![],
            collisions: vec![],
            animations: vec![],
        }],
        // 同一キーを 2 エントリ持ち、Vec が潰さず保持することを示す（要件 8.4）。
        aliases: vec![
            SurfaceAlias {
                key: AliasKey::new("通常".to_string()),
                ids: vec![0, 1, 2],
            },
            SurfaceAlias {
                key: AliasKey::new("通常".to_string()),
                ids: vec![10, 11],
            },
        ],
        animation_sort: None,
        collision_sort: None,
        definitions: vec![],
    };

    // surface 構造。
    assert_eq!(shell.surfaces.len(), 1);
    let sf = &shell.surfaces[0];
    assert_eq!(sf.id, 0);
    assert_eq!(sf.elements[0].path.as_str(), "surface0.png");
    assert_eq!(sf.elements[0].x, 0);
    assert_eq!(sf.collisions[0].name.as_str(), "Head");
    assert_eq!(sf.collisions[0].right, 100);
    assert_eq!(sf.animations[0].id, 4);
    assert_eq!(sf.animations[0].interval, Interval::Random { k: 4 });
    // 負値 surface_id が i64 として保持される（要件 5.5）。
    assert_eq!(sf.animations[0].patterns[0].surface_id, -1);

    // append 記述子（展開せず記述子のまま）。
    assert_eq!(shell.appends.len(), 1);
    assert_eq!(shell.appends[0].targets.len(), 2);
    assert_eq!(shell.appends[0].targets[0], AppendTarget::Single(10));
    assert_eq!(
        shell.appends[0].targets[1],
        AppendTarget::Range { start: 2100, end: 2110 }
    );

    // 重複キーの alias が両方保持される（潰されない・要件 8.4）。
    assert_eq!(shell.aliases.len(), 2);
    assert_eq!(shell.aliases[0].key.as_str(), "通常");
    assert_eq!(shell.aliases[1].key.as_str(), "通常");
    assert_eq!(shell.aliases[0].ids, vec![0, 1, 2]);
    assert_eq!(shell.aliases[1].ids, vec![10, 11]);
}
