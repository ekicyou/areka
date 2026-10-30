//! 差し替えの荷物から事実を組む [`swapped`] の決定論テスト（spec: mcp-strict-errors 要件 4.1）。
//!
//! 表示の層（GPU なしの `EmoPresenter`）に今のシェルとバルーンを登録し、荷物から組んだ事実を
//! `SkinLookup` で問う。写し先を取り違えると答えが変わる番号を使う。

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::EmoPresenter;
use areka_seriko::{BindResolver, SkinLookup, SurfaceResolver, SurfaceTarget};
use bevy_ecs::world::World;

use super::*;
use crate::emo2_boot::assets::{BalloonAssets, BalloonScopeAssets, ScopeAssets, ShellAssets};
use crate::emo2_boot::target_map::{balloon_target, shell_target};
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::source::{DescriptSource, GhostTitles};

/// 今のシェル（面 1・2、別名 `今` は 2）と今のバルーン（面 0・1）。
const NOW_SHELL: &str = "surface1\n{\n}\nsurface2\n{\n}\nkero.surface.alias\n{\n今,[2]\n}\n";
const NOW_BALLOON: &str = "surface0\n{\n}\nsurface1\n{\n}\n";
/// 新しいシェル（面 10・11、別名 `新` は 11）と新しいバルーン（面 5）。
const NEW_SHELL: &str = "surface10\n{\n}\nsurface11\n{\n}\nkero.surface.alias\n{\n新,[11]\n}\n";
const NEW_BALLOON: &str = "surface5\n{\n}\n";

fn world_of(text: &str) -> EmoWorld {
    EmoWorld::build(&areka_parsers::shell::parse(text))
}

fn atlas() -> AtlasTable {
    AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
}

/// scope 0・1 に今のシェルとバルーンを登録した表示の層。
fn presenter() -> EmoPresenter {
    let mut ecs = World::new();
    let mut presenter = EmoPresenter::new();
    for scope in [0, 1] {
        for (target, text) in [
            (shell_target(scope), NOW_SHELL),
            (balloon_target(scope), NOW_BALLOON),
        ] {
            let window = ecs.spawn_empty().id();
            presenter
                .attach_target(&mut ecs, target, window, world_of(text), atlas(), 96)
                .expect("登録できる");
        }
    }
    presenter
}

/// scope 0・1・2 の新しいシェルの荷物（scope 2 は窓を持たない）。
fn shell_built() -> SwapBuilt {
    SwapBuilt::Shell {
        assets: ShellAssets {
            shells: [0, 1, 2]
                .map(|scope| ScopeAssets {
                    scope,
                    emo_world: world_of(NEW_SHELL),
                    atlas: atlas(),
                    initial_surface_id: 10,
                })
                .into(),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: areka_seriko::DefaultBinds::uniform(BindSet::default()),
            bind_resolver: BindResolver::empty(),
            loop_table: areka_seriko::AnimationTable::empty(),
            author_dpi: 96,
            bake_failures: Vec::new(),
            boxes: crate::emo2_boot::shell_box_assets::ShellBoxAssets::default(),
            cursors: crate::emo2_boot::shell_cursor_assets::ShellCursorAssets::default(),
        },
        source: DescriptSource {
            ghost_kv: BTreeMap::new(),
            shell_kv: BTreeMap::new(),
            shell_dir: PathBuf::from("shell/B"),
            titles: GhostTitles::from_scope_titles([(0, "s".to_string())]),
        },
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: BTreeMap::new(),
        },
        restored: Vec::new(),
    }
}

/// scope 0・1・2 の新しいバルーンの荷物（scope 2 は窓を持たない）。
fn balloon_built() -> SwapBuilt {
    SwapBuilt::Balloon {
        assets: BalloonAssets {
            balloons: [0, 1, 2]
                .map(|scope| BalloonScopeAssets {
                    scope,
                    emo_world: world_of(NEW_BALLOON),
                    atlas: atlas(),
                    model: areka_parsers::balloon::parse_str("", None),
                    background_color: (255, 255, 255),
                    name: String::new(),
                })
                .into(),
            loop_tables: [0, 1, 2]
                .map(|s| (s, areka_seriko::AnimationTable::empty()))
                .into(),
            author_dpi: 96,
        },
    }
}

/// シェルの差し替え: 窓を持つスコープだけに荷物の面と先頭の World の別名表が入り、バルーンは今のまま。
#[test]
fn shell_swap_takes_payload_shells_and_keeps_balloons() {
    let facts = swapped(&presenter(), &[0, 1], &shell_built());
    for scope in [0, 1] {
        assert_eq!(facts.shell_has(scope, 10), Some(true), "scope {scope}");
        assert_eq!(facts.shell_has(scope, 1), Some(false), "古い面は残らない");
        // バルーンへ荷物の面を書かない（今のバルーンのまま）。
        assert_eq!(facts.balloon_has(scope, 1), Some(true));
        assert_eq!(facts.balloon_has(scope, 10), Some(false));
    }
    assert_eq!(facts.shell_has(2, 10), None, "窓の無いスコープは入れない");
    assert_eq!(facts.resolve_surface("新"), SurfaceTarget::Show(11));
    assert_eq!(facts.resolve_surface("今"), SurfaceTarget::Unresolved);
}

/// バルーンの差し替え: 窓を持つスコープだけに荷物のバルーンが入り、シェルと別名表は今のまま。
#[test]
fn balloon_swap_takes_payload_balloons_and_keeps_shells() {
    let facts = swapped(&presenter(), &[0, 1], &balloon_built());
    for scope in [0, 1] {
        assert_eq!(facts.balloon_has(scope, 5), Some(true), "scope {scope}");
        assert_eq!(facts.balloon_has(scope, 1), Some(false), "古い面は残らない");
        // シェルへ荷物の面を書かない（今のシェルのまま）。
        assert_eq!(facts.shell_has(scope, 2), Some(true));
        assert_eq!(facts.shell_has(scope, 5), Some(false));
    }
    assert_eq!(facts.balloon_has(2, 5), None, "窓の無いスコープは入れない");
    assert_eq!(facts.resolve_surface("今"), SurfaceTarget::Show(2));
}
