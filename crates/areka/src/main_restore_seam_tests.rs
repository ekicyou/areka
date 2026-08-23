use super::*;
use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_parsers::charset::DefaultEncoding;
use placement::balloon_limit::BALLOON_LIMIT_CLAMP_TAG;
use placement::follow::MonitorSnapshot;
use placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use placement::test_support::{ExpectField, capture_logs, expect_one};
use std::path::Path;

/// 復元テスト共通寸法（persist.rs の merge テストと同流儀）。
const CSZ: SizePx = SizePx { w: 400, h: 600 };
const BSZ: SizePx = SizePx { w: 200, h: 300 };

/// このテスト専用の一意な一時ディレクトリ（persist.rs の load テストと同規約・
/// 外部 tempfile 非依存）。
fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_main_restore_seam_tests_{tag}"));
    dir
}

/// `resolve` が成功する最小ゴーストパッケージ（persist.rs の `plant_minimal_ghost` と同型）。
fn plant_minimal_ghost(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
    )
    .expect("write ghost descript");
    std::fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");
}

/// scope 0 の合成 placement（resolver 出力を模す。`balloon_pos ≡ char_pos + balloon_offset`
/// の事後条件を満たす）。既定 char_pos は saved と別位置に置く（優先の証明のため）。
fn synthetic_placement(default_char_pos: PointPx) -> ScopePlacement {
    let balloon_offset = PointPx { x: -50, y: 10 };
    ScopePlacement {
        scope: 0,
        char_pos: default_char_pos,
        char_size: CSZ,
        balloon_pos: PointPx {
            x: default_char_pos.x + balloon_offset.x,
            y: default_char_pos.y + balloon_offset.y,
        },
        balloon_size: BSZ,
        balloon_offset,
        // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
        balloon_limit: true,
        anchor: Anchor::Free,
        balloon_keyword_base: None,
    }
}

/// 1.4: 植えた保存位置が既定位置に優先して merge 済み placement の char_pos へ載る
/// （load→apply シームが実際に結線されている証明＝spawn される窓の初期位置が保存位置）。
/// saved 窓を完全に覆う work area ゆえ `project_restore` は恒等＝保存値素通し。
#[test]
fn restore_seam_prefers_saved_position_over_default() {
    let root = unique_temp_dir("prefers_saved");
    plant_minimal_ghost(&root);
    // profile root = <ghost/master>/profile/areka（boot 結線と同一構築）。
    let profile = profile_areka_root(&root.join("ghost").join("master"));
    std::fs::create_dir_all(&profile).expect("create profile/areka");
    std::fs::write(
        profile.join("sylphya.toml"),
        "format-version = 1\n[window.0]\nx = \"800\"\ny = \"300\"\n".as_bytes(),
    )
    .expect("plant sylphya.toml");

    // 既定は saved とは別位置。saved 窓 (800,300)-(1200,900) を覆う広い work area
    // ゆえ再射影は恒等（保存値素通し）。
    let default_char_pos = PointPx { x: 100, y: 100 };
    let placements = vec![synthetic_placement(default_char_pos)];
    let snapshot = MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2160,
        }],
    };

    let (out, restored) =
        restore_merged_placements(&root, placements, &snapshot, DefaultEncoding::Ansi);

    assert_eq!(out.len(), 1);
    // 保存位置が採用された scope は「既定配置ではない」として報告される（scg 7.3）。
    // これを受けて起動シームが台帳の既定位置を落とし、連鎖の再解決から除外する。
    assert_eq!(
        restored.iter().copied().collect::<Vec<_>>(),
        vec![0usize],
        "保存位置を採用した scope が復元済みとして報告される（scg 7.3）"
    );
    assert_eq!(
        out[0].char_pos,
        PointPx { x: 800, y: 300 },
        "植えた保存位置が既定(100,100)に優先して spawn 前 placements へ載る（1.4）"
    );
    // 事後条件（design C1）: 寸法・anchor は不変。
    assert_eq!(out[0].char_size, CSZ);
    assert_eq!(out[0].balloon_size, BSZ);
    assert_eq!(out[0].anchor, Anchor::Free);

    let _ = std::fs::remove_dir_all(&root);
}

/// 1.5: 永続不在（sylphya.toml を植えない）→ merge は既定 placement に完全恒等
/// （保存位置が無ければ従来の既定位置解決のまま）。
#[test]
fn restore_seam_without_persist_is_identity_default() {
    let root = unique_temp_dir("no_persist_default");
    plant_minimal_ghost(&root); // resolve は成功するが sylphya.toml は植えない。

    let default_char_pos = PointPx { x: 100, y: 100 };
    let placements = vec![synthetic_placement(default_char_pos)];
    let expected = placements.clone();
    let snapshot = MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2160,
        }],
    };

    let (out, restored) =
        restore_merged_placements(&root, placements, &snapshot, DefaultEncoding::Ansi);

    assert_eq!(
        out, expected,
        "永続不在は既定 placement に恒等＝既定位置解決のまま（1.5）"
    );
    assert!(
        restored.is_empty(),
        "永続不在なら復元済み scope は無い＝全 scope が既定配置のまま（scg 7.3）"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// areka-P0-windowposition-limit 2.2/4.7/6.1・design C6/DD6:
/// **起動時関門が復元合流シームに実際に結線されている**ことの証明。
///
/// 植えた保存位置が優先されて（＝合流順序は不変・要件 4.7）そのキャラ位置から導いた
/// バルーン表示位置が作業領域の外へ出るとき、シームの出力では
/// - `balloon_pos`（表示位置）が作業領域内へ補正され、
/// - `balloon_offset`（論理相対位置＝保存値・作者指定の系譜）は**生値のまま**残り、
/// - `[balloon-limit] Clamp` が当該 scope で記録される（要件 6.1）。
///
/// 補正が `balloon_offset` へ焼き付いていたら（DD6 違反）3 番目の assert が赤になる。
#[test]
fn restore_seam_clamps_balloon_display_position_but_keeps_offset_raw() {
    let root = unique_temp_dir("balloon_limit_gate");
    plant_minimal_ghost(&root);
    let profile = profile_areka_root(&root.join("ghost").join("master"));
    std::fs::create_dir_all(&profile).expect("create profile/areka");
    // 保存位置は作業領域の左端近く。balloon_offset(-50) を足すと左辺を 20px はみ出す。
    std::fs::write(
        profile.join("sylphya.toml"),
        "format-version = 1\n[window.0]\nx = \"30\"\ny = \"300\"\n".as_bytes(),
    )
    .expect("plant sylphya.toml");

    // 既定は保存値と別位置（保存値優先＝合流順序の証明のため）。
    let default_char_pos = PointPx { x: 900, y: 200 };
    let placements = vec![synthetic_placement(default_char_pos)];
    let snapshot = MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        }],
    };

    let ((out, restored), events) = capture_logs(|| {
        restore_merged_placements(&root, placements, &snapshot, DefaultEncoding::Ansi)
    });

    assert_eq!(out.len(), 1);
    assert_eq!(
        restored.iter().copied().collect::<Vec<_>>(),
        vec![0usize],
        "保存位置を採用した scope として報告される（合流規則は関門の設置で変わらない）"
    );
    assert_eq!(
        out[0].char_pos,
        PointPx { x: 30, y: 300 },
        "保存値が既定(900,200)に優先する＝合流順序は不変（要件 4.7）"
    );
    assert_eq!(
        out[0].balloon_pos,
        PointPx { x: 0, y: 310 },
        "バルーン表示位置は作業領域の左辺へ補正される（要件 2.2・起動時関門の結線）"
    );
    assert_eq!(
        out[0].balloon_offset,
        PointPx { x: -50, y: 10 },
        "論理相対位置は生値のまま＝補正を焼き付けない（DD6・要件 3.1(d)）"
    );
    // キャラ窓は limit 補正で動かない（要件 2.8）。
    assert_eq!(out[0].char_size, CSZ);
    assert_eq!(out[0].balloon_size, BSZ);
    assert_eq!(out[0].anchor, Anchor::Free);

    let hit = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(
        hit.expect_field("scope"),
        "0",
        "補正した scope を記録する（要件 6.1）"
    );

    let _ = std::fs::remove_dir_all(&root);
}
