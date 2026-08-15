use super::*;
use crate::placement::config::BalloonXMode;

// ------------------------------------------------------------------
// バルーン相対オフセットの保存基準（2.2/2.5・2026-07-31 実機裁定）
//
// 旧: アンカー辺基準（Bottom=(w/2,h)・Right=(w,0)）へ変換してから保存していた。
// 新: **char 窓左上基準**のまま保存する＝ランタイム BalloonFollow.offset と同一基準。
//     ランタイムの追従が全アンカーで窓相対（resize で offset を補正しない）へ
//     統一されたため、保存だけ別基準にすると保存時と復元時でサーフェス寸が違うとき
//     Δ ぶんの恒久ドリフトが出る（実機のむらさきで Δh=175px）。
//
// 基準変換の純関数（balloon_offset_to_persist/_from_persist/anchor_edge_basis）は
// 恒等になったため撤去済み。以下の檻は同じ性質を「保存 entries ⇄ 復元 merge」の
// 実経路（balloon_offset_entries → apply_restored_placements）で固定する。
// ------------------------------------------------------------------

const ALL_ANCHORS: [Anchor; 5] = [
    Anchor::Top,
    Anchor::Bottom,
    Anchor::Left,
    Anchor::Right,
    Anchor::Free,
];

/// 保存方向: [`balloon_offset_entries`] は左上基準 offset を**そのまま**書く
/// （アンカー辺基準の減算をしない）。旧基準が書いていた値を明示的に排除する。
#[test]
fn balloon_offset_entries_persist_raw_top_left_offset() {
    let offset_tl = PointPx { x: 40, y: 70 };
    let entries = balloon_offset_entries(0, offset_tl);
    assert_eq!(
        entries,
        vec![
            (
                PersistKey::BalloonOffset {
                    scope: 0,
                    axis: Axis::X
                },
                "40".to_string()
            ),
            (
                PersistKey::BalloonOffset {
                    scope: 0,
                    axis: Axis::Y
                },
                "70".to_string()
            ),
        ],
        "保存値は char 左上基準の生 offset（char_size を混ぜない）"
    );
    // 旧アンカー辺基準（char 300x500）なら Bottom は (-110,-430)・Right は (-260,70) を
    // 書いていた——それらが再び現れないことを排除する（基準反転の恒久檻）。
    for stale in ["-110", "-430", "-260"] {
        assert!(
            !entries.iter().any(|(_, v)| v == stale),
            "旧アンカー辺基準の値 {stale} が保存されている（基準変換の復活）"
        );
    }
}

/// 復元方向: 保存 BalloonOffset は**全アンカーで**基準変換なしにそのまま
/// 左上基準 offset として採用される（現 char_size を足し戻さない）。
#[test]
fn restored_balloon_offset_is_raw_saved_value_for_all_anchors() {
    let snap = snapshot_of(vec![]); // 空＝project_restore identity
    let saved = PointPx { x: 40, y: 70 };
    let entries = balloon_offset_entries(0, saved);
    for anchor in ALL_ANCHORS {
        let placements = vec![placement(
            0,
            anchor,
            PointPx { x: 100, y: 100 },
            CSZ, // 400x600（旧基準なら Bottom で +（200,600）されていた）
            PointPx { x: -50, y: 0 }, // 既定（保存値で上書きされる）
            BSZ,
        )];
        let out = apply_restored_placements(placements, &entries, &snap);
        assert_eq!(
            out[0].balloon_offset, saved,
            "anchor={anchor:?}: 保存値がそのまま左上基準 offset になっていない"
        );
        assert_eq!(
            out[0].balloon_pos,
            PointPx { x: 140, y: 170 },
            "anchor={anchor:?}: balloon_pos ≡ char_pos + 左上基準 offset"
        );
    }
}

/// 不変条件: 全アンカー × 複数寸法で保存 → 復元が恒等（左上基準の往復）。
/// **保存時と復元時で char_size が違っても**恒等——これがランタイム
/// （`resize_window_to` が offset を補正しない）との一致点。
#[test]
fn balloon_offset_round_trip_is_identity_across_anchors_and_size_changes() {
    let snap = snapshot_of(vec![]);
    let sizes = [
        SizePx { w: 1, h: 1 },
        SizePx { w: 128, h: 256 },
        SizePx { w: 300, h: 500 },
        SizePx { w: 1024, h: 64 },
    ];
    let offsets = [
        PointPx { x: 0, y: 0 },
        PointPx { x: -400, y: 0 },
        PointPx { x: 40, y: 70 },
        PointPx { x: -260, y: -430 },
    ];
    for anchor in ALL_ANCHORS {
        for size in sizes {
            for offset in offsets {
                let entries = balloon_offset_entries(0, offset);
                // 復元側の char_size は保存側と**別寸**でもよい（窓相対ゆえ影響しない）。
                let placements = vec![placement(
                    0,
                    anchor,
                    PointPx { x: 0, y: 0 },
                    size,
                    PointPx { x: 7, y: 7 },
                    BSZ,
                )];
                let out = apply_restored_placements(placements, &entries, &snap);
                assert_eq!(
                    out[0].balloon_offset, offset,
                    "往復恒等が破れた: anchor={anchor:?} size={size:?} offset={offset:?}"
                );
            }
        }
    }
}

/// 8.5（意味反転・2026-07-31 実機裁定）Bottom のサーフェス寸変動:
/// 保存時と**同じ寸**が表示された瞬間はバルーン位置が厳密復元され、**異なる寸**でも
/// offset は窓（左上）相対のまま不変である。
///
/// 旧檻は「下端からの距離が不変」＝下端基準を主張していたが、ランタイムの
/// `resize_window_to` が窓相対追従へ統一された以上、保存だけ下端基準にすると
/// 「小サーフェス表示中にドラッグ→保存→再起動→切替」で Δh ぶんの恒久ドリフトが出る。
/// 本檻はその下端基準の復活を明示的に排除する。
#[test]
fn balloon_offset_restores_exactly_at_same_size_and_stays_window_relative_bottom() {
    let snap = snapshot_of(vec![]); // 空＝project_restore identity（保存 char_pos 素通し）
    // 保存時: 高さ h1・ユーザーはバルーンを char 左上から下 70px／右 40px へ置いた。
    let h1 = 500;
    let size_h1 = SizePx { w: 300, h: h1 };
    let offset_tl_saved = PointPx { x: 40, y: 70 };
    // char 位置も保存する（保存 x は原点＝下端中央基準・char_pos_to_origin_x は無改変）。
    let char_pos_saved = PointPx { x: 800, y: 500 };
    let mut entries = char_pos_entries(
        0,
        char_pos_to_origin_x(Anchor::Bottom, char_pos_saved, size_h1),
    );
    entries.extend(balloon_offset_entries(0, offset_tl_saved));

    // (a) 同一寸での復元は厳密復元（char_pos も balloon_pos も保存時と一致）。
    let out_same = apply_restored_placements(
        vec![placement(
            0,
            Anchor::Bottom,
            PointPx { x: 1, y: 1 },
            size_h1,
            PointPx { x: 7, y: 7 },
            BSZ,
        )],
        &entries,
        &snap,
    );
    assert_eq!(
        out_same[0].char_pos, char_pos_saved,
        "同一寸で char 厳密復元"
    );
    assert_eq!(
        out_same[0].balloon_offset, offset_tl_saved,
        "同一寸で offset 厳密復元"
    );
    assert_eq!(
        out_same[0].balloon_pos,
        PointPx {
            x: char_pos_saved.x + offset_tl_saved.x,
            y: char_pos_saved.y + offset_tl_saved.y
        },
        "同一寸でバルーン絶対位置が厳密復元される"
    );

    // (b) 異なる高さ h2 で復元しても offset は窓（左上）相対のまま不変
    //     ——ランタイム `resize_window_to`（offset を補正しない）と同一セマンティクス。
    let h2 = 620;
    let size_h2 = SizePx { w: 300, h: h2 };
    let out_diff = apply_restored_placements(
        vec![placement(
            0,
            Anchor::Bottom,
            PointPx { x: 1, y: 1 },
            size_h2,
            PointPx { x: 7, y: 7 },
            BSZ,
        )],
        &entries,
        &snap,
    );
    assert_eq!(
        out_diff[0].balloon_offset, offset_tl_saved,
        "寸法が変わっても左上基準 offset は不変（窓相対）"
    );

    // 旧下端基準の反例排除: 下端基準なら復元 offset.y は offset_tl.y + (h2 − h1)
    // ＝ 70 + 120 = 190 になっていた（Δh ぶんの恒久ドリフト）。
    assert_ne!(
        out_diff[0].balloon_offset.y,
        offset_tl_saved.y + (h2 - h1),
        "下端基準の復活検出: 高さ差ぶん offset がドリフトしている"
    );
}

// ------------------------------------------------------------------
// project_restore（復元時再射影・5.1/5.2/5.3・design C1・Testing Strategy §2）
//   project_anchor（アンカー辺再導出）＋補軸 clamp（可視性保証）。
//   単一モニタ wa=(0,0,1920,1040)・size=(400,600) → bottom 揃え y=1040−600=440。
// ------------------------------------------------------------------

fn wa_rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
    RectPx {
        left,
        top,
        right,
        bottom,
    }
}

fn snapshot_of(rects: Vec<RectPx>) -> MonitorSnapshot {
    MonitorSnapshot { work_areas: rects }
}

/// 復元テスト共通の char 寸（物理 px）。bottom 揃え y = wa.bottom − 600。
const SZ: SizePx = SizePx { w: 400, h: 600 };

/// 5.3: 域内 Free は恒等（identity 射影＋両軸 clamp が域内で no-op）。
#[test]
fn project_restore_free_inside_work_area_is_identity() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    // 窓 (500..900, 300..900) は wa 内へ完全に収まる
    let pos = PointPx { x: 500, y: 300 };
    assert_eq!(
        project_restore(Anchor::Free, pos, SZ, &snap),
        pos,
        "域内 Free は不要な再射影をしない＝恒等（5.3）"
    );
}

/// 5.3: 既に下端一致＋x 域内の Bottom はべき等＝恒等（不要な再射影をしない）。
#[test]
fn project_restore_bottom_already_anchored_inside_is_identity() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    // bottom 揃え y = 1040 − 600 = 440・x は wa 内
    let pos = PointPx { x: 500, y: 440 };
    assert_eq!(
        project_restore(Anchor::Bottom, pos, SZ, &snap),
        pos,
        "既に下端一致＋x 域内なら恒等（5.3・べき等）"
    );
}

/// 5.1/5.2: 保存 y が現 work area 外（背の高いモニタからの復元）の Bottom は、
/// 下端吸着で域内へ戻り（下端 = wa.bottom − h）水平位置は保持する。
#[test]
fn project_restore_bottom_y_outside_snaps_bottom_and_preserves_x() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    // 保存 y=2000 は現 work area 下端 1040 の外
    let pos = PointPx { x: 500, y: 2000 };
    let out = project_restore(Anchor::Bottom, pos, SZ, &snap);
    assert_eq!(out.y, 1040 - 600, "下端吸着維持: y = wa.bottom − h（5.2）");
    assert_eq!(out.y + SZ.h, 1040, "下端が wa.bottom に一致＝域内（5.1）");
    assert_eq!(out.x, 500, "水平位置（X 意図）は保持（5.2）");
}

/// 5.1: 保存 x が現 work area 右外の Bottom は、x を [wa.left, wa.right−w] へ
/// clamp して域内へ戻す（モニタ喪失シナリオ＝最近傍 wa）。
#[test]
fn project_restore_bottom_x_outside_clamps_into_work_area() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    let pos = PointPx { x: 3000, y: 440 }; // x は wa 右外
    let out = project_restore(Anchor::Bottom, pos, SZ, &snap);
    assert_eq!(
        out.x,
        1920 - 400,
        "x を [wa.left, wa.right−w] 内へ clamp（5.1）"
    );
    assert_eq!(out.x + SZ.w, 1920, "右端が wa.right に一致＝域内");
    assert_eq!(out.y, 1040 - 600, "下端吸着は維持（5.2）");
}

/// 端付近に置いた Bottom char（右端を数十 px はみ出す＝**一部可視**）は復元で
/// クランプされず保存 x をそのまま用いる（Req5.3・実機サインオフ検出の恒久回帰）。
/// 保存側 `project_anchor`／BottomSnapPolicy は補軸 x を clamp しないため、一部可視の
/// 位置は復元でも同一でなければ立ち位置がずれ、追従 balloon もずれる（保存↔復元の
/// クランプ非対称）。実機値: 保存 x=3493・wa.right=3840・w=434（右端 3927 で 87px はみ出し）
/// → 修正前は 3406 へ clamp していた欠陥を固定する。
#[test]
fn project_restore_bottom_partially_visible_keeps_saved_x() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 3840, 2100)]);
    let size = SizePx { w: 434, h: 687 };
    // 一部可視（rect [3493,3927] が wa [0,3840] と交差）→ 保存 x を維持。
    let out = project_restore(Anchor::Bottom, PointPx { x: 3493, y: 1553 }, size, &snap);
    assert_eq!(
        out.x, 3493,
        "一部でも可視なら保存 x を維持（clamp しない・Req5.3・保存↔復元 idempotent）"
    );
    assert_eq!(out.y, 2100 - 687, "Bottom は下端吸着で y を再導出（5.2）");
    // 対比: 完全に不可視（rect [4000,4434] が wa と交差なし）はクランプで可視化（Req5.1）。
    let off = project_restore(Anchor::Bottom, PointPx { x: 4000, y: 1553 }, size, &snap);
    assert_eq!(
        off.x,
        3840 - 434,
        "完全不可視（モニタ構成変化相当）はクランプで画面内へ寄せる（Req5.1）"
    );
}

/// 5.1/5.2: Left アンカーは x を wa.left へ固定し、補軸 y を域内へ clamp する。
#[test]
fn project_restore_left_pins_left_edge_and_clamps_y() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    let pos = PointPx { x: 3000, y: 2000 }; // x/y とも外
    let out = project_restore(Anchor::Left, pos, SZ, &snap);
    assert_eq!(out.x, 0, "左端固定: x = wa.left（5.2）");
    assert_eq!(
        out.y,
        1040 - 600,
        "補軸 y を [wa.top, wa.bottom−h] へ clamp（5.1）"
    );
}

/// Free（域外）: identity 射影（アンカー辺固定なし）＋両軸のみ可視性 clamp。
#[test]
fn project_restore_free_clamps_both_axes_with_identity_projection() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    let pos = PointPx { x: 3000, y: 2000 }; // 両軸とも外
    let out = project_restore(Anchor::Free, pos, SZ, &snap);
    assert_eq!(
        out,
        PointPx {
            x: 1920 - 400,
            y: 1040 - 600
        },
        "Free＝アンカー辺再固定なし・両軸 clamp のみ（identity 射影＋可視性保証）"
    );
}

/// 空 snapshot は全アンカーで恒等（架空矩形を発明しない・既存縮退流儀・5.1 note）。
#[test]
fn project_restore_empty_snapshot_is_identity() {
    let snap = snapshot_of(vec![]);
    let pos = PointPx { x: 3000, y: 2000 };
    for anchor in ALL_ANCHORS {
        assert_eq!(
            project_restore(anchor, pos, SZ, &snap),
            pos,
            "空 snapshot は identity 縮退: {anchor:?}"
        );
    }
}

/// 5.1: どのモニタにも属さない復元位置は最近傍 wa を採り、その中へ吸着＋clamp
/// する（2 面構成・モニタ喪失シナリオ＝design §2）。
#[test]
fn project_restore_off_all_monitors_uses_nearest_work_area() {
    // A=左・B=右の 2 面。保存位置は B のさらに右外＝どのモニタにも属さない
    let a = wa_rect(0, 0, 1920, 1040);
    let b = wa_rect(1920, 0, 3840, 1040);
    let snap = snapshot_of(vec![a, b]);
    // 窓中心 (5000+200, 500+300)=(5200,800) は B が最近傍
    let pos = PointPx { x: 5000, y: 500 };
    let out = project_restore(Anchor::Bottom, pos, SZ, &snap);
    assert_eq!(out.y, 1040 - 600, "最近傍 B の下端へ吸着");
    assert_eq!(
        out.x,
        3840 - 400,
        "x を最近傍 B の [left, right−w] 内へ clamp（5.1）"
    );
    assert!(out.x >= b.left, "x は B 内");
}

// ------------------------------------------------------------------
// apply_restored_placements（復元 merge・1.4/1.5/1.6/2.3/2.4/2.5/6.1・design C1）
//   純関数・決定論・永続不書込。saved pos 優先→project_restore→balloon 導出。
// ------------------------------------------------------------------

/// テスト用 `ScopePlacement` 構築（`balloon_pos ≡ char_pos + balloon_offset` の
/// 事後条件を満たす resolver 出力を模す）。
fn placement(
    scope: usize,
    anchor: Anchor,
    char_pos: PointPx,
    char_size: SizePx,
    balloon_offset: PointPx,
    balloon_size: SizePx,
) -> ScopePlacement {
    ScopePlacement {
        scope,
        char_pos,
        char_size,
        balloon_pos: PointPx {
            x: char_pos.x + balloon_offset.x,
            y: char_pos.y + balloon_offset.y,
        },
        balloon_size,
        balloon_offset,
        // windowposition-limit: 正典既定（有効）。復元 merge は limit を変換しない。
        balloon_limit: true,
        anchor,
        balloon_keyword_base: None,
    }
}

fn wp(scope: u32, axis: Axis, v: &str) -> (PersistKey, String) {
    (PersistKey::WindowPos { scope, axis }, v.to_string())
}
fn bo(scope: u32, axis: Axis, v: &str) -> (PersistKey, String) {
    (PersistKey::BalloonOffset { scope, axis }, v.to_string())
}

/// 復元テスト共通寸法。
const CSZ: SizePx = SizePx { w: 400, h: 600 };
const BSZ: SizePx = SizePx { w: 200, h: 300 };

/// 1.4: 保存 WindowPos が両軸とも parse できるとき char_pos を保存値へ差し替える
/// （既定位置解決に優先）。空 snapshot ゆえ project_restore は identity で保存値素通し。
#[test]
fn apply_saved_window_pos_takes_priority_over_default() {
    let snap = snapshot_of(vec![]); // 空＝project_restore identity（保存値素通し）
    let placements = vec![placement(
        0,
        Anchor::Free,
        PointPx { x: 100, y: 100 }, // 既定
        CSZ,
        PointPx { x: -50, y: 10 },
        BSZ,
    )];
    let entries = vec![wp(0, Axis::X, "800"), wp(0, Axis::Y, "500")];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0].char_pos,
        PointPx { x: 800, y: 500 },
        "保存位置が既定(100,100)に優先する（1.4）"
    );
    // 事後条件: 寸法・anchor は不変
    assert_eq!(out[0].char_size, CSZ);
    assert_eq!(out[0].balloon_size, BSZ);
    assert_eq!(out[0].anchor, Anchor::Free);
}

/// 1.5/2.4: 空 entries → 出力は入力 placements に完全恒等（identity）。
#[test]
fn apply_empty_entries_is_identity() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    let placements = vec![
        placement(
            0,
            Anchor::Bottom,
            PointPx { x: 500, y: 440 },
            CSZ,
            PointPx { x: -200, y: 0 },
            BSZ,
        ),
        placement(
            1,
            Anchor::Free,
            PointPx { x: 100, y: 200 },
            CSZ,
            PointPx { x: 400, y: 12 },
            BSZ,
        ),
    ];

    let out = apply_restored_placements(placements.clone(), &[], &snap);

    assert_eq!(out, placements, "空 entries は入力恒等（1.5/2.4）");
}

/// windowposition-limit: 復元 merge は `balloon_limit` を**両方向とも**転記する。
///
/// 既存の復元檻はすべて `balloon_limit: true` の placement を投入するため、
/// merge が「無効化しない」ことしか固定できていなかった。`merge_scope` の転記を
/// `true` 固定へ変異させても既存檻は 1 本も落ちない（実測）。それでは
/// 「`windowposition.limit,0` の scope に保存位置があると limit が黙って復活する」
/// という退行を検出できない——保存位置を持つ scope はユーザが自分でドラッグした
/// scope であり、画面外に居る可能性が最も高い当のケースである。
///
/// 本檻は `false` を投入して、保存値が効く経路（scope0）と効かない経路（scope1）の
/// **両方**で `false` のまま出てくることを固定する。
#[test]
fn apply_restored_placements_carries_balloon_limit_false_both_ways() {
    let snap = snapshot_of(vec![]); // identity 射影（保存値素通し）
    // `Free` は射影を持たないので保存値がそのまま出る（既存の恒等檻と同じ流儀）。
    let disabled = |scope: usize, char_pos: PointPx| ScopePlacement {
        balloon_limit: false,
        ..placement(
            scope,
            Anchor::Free,
            char_pos,
            CSZ,
            PointPx { x: -200, y: 0 },
            BSZ,
        )
    };
    let placements = vec![
        disabled(0, PointPx { x: 500, y: 440 }),
        disabled(1, PointPx { x: 100, y: 200 }),
    ];
    // scope0 にだけ保存位置を与える（merge が働く経路と働かない経路を 1 回で covering）。
    let entries = vec![wp(0, Axis::X, "777"), wp(0, Axis::Y, "333")];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(
        out[0].char_pos,
        PointPx { x: 777, y: 333 },
        "前提: scope0 は保存値が効いて merge_scope を通っている"
    );
    assert!(
        !out[0].balloon_limit,
        "保存位置のある scope でも limit=0 は復活しない（merge_scope の転記）"
    );
    assert!(
        !out[1].balloon_limit,
        "保存位置のない scope でも limit=0 のまま（恒等経路）"
    );
}

/// 1.6/2.5: scope 別 entries は交差しない。scope0 の WindowPos は scope1 に波及せず、
/// scope1 は既定を保つ。
#[test]
fn apply_scopes_do_not_cross() {
    let snap = snapshot_of(vec![]); // identity 射影
    let placements = vec![
        placement(
            0,
            Anchor::Free,
            PointPx { x: 100, y: 100 },
            CSZ,
            PointPx { x: -50, y: 0 },
            BSZ,
        ),
        placement(
            1,
            Anchor::Free,
            PointPx { x: 700, y: 700 },
            CSZ,
            PointPx { x: 30, y: 5 },
            BSZ,
        ),
    ];
    // scope0 のみ保存（位置＋バルーン）。scope1 は entries 無し＝既定保持。
    let entries = vec![
        wp(0, Axis::X, "800"),
        wp(0, Axis::Y, "500"),
        bo(0, Axis::X, "-30"),
        bo(0, Axis::Y, "20"),
    ];

    let out = apply_restored_placements(placements.clone(), &entries, &snap);

    // scope0 は保存値へ
    assert_eq!(
        out[0].char_pos,
        PointPx { x: 800, y: 500 },
        "scope0 保存位置"
    );
    assert_eq!(
        out[0].balloon_offset,
        PointPx { x: -30, y: 20 },
        "scope0 保存 offset"
    );
    // scope1 は完全恒等（scope0 の entries に汚染されない）
    assert_eq!(
        out[1], placements[1],
        "scope1 は既定恒等（scope 分離・1.6/2.5）"
    );
}

/// 2.4: BalloonOffset 欠損時は resolver 既定 offset を保持し、最終 char_pos へ追従する。
#[test]
fn apply_balloon_offset_absent_keeps_default_following_char() {
    let snap = snapshot_of(vec![]); // identity
    let default_offset = PointPx { x: -50, y: 10 };
    let placements = vec![placement(
        0,
        Anchor::Free,
        PointPx { x: 100, y: 100 },
        CSZ,
        default_offset,
        BSZ,
    )];
    // WindowPos のみ（BalloonOffset 無し）。
    let entries = vec![wp(0, Axis::X, "800"), wp(0, Axis::Y, "500")];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(out[0].char_pos, PointPx { x: 800, y: 500 });
    assert_eq!(
        out[0].balloon_offset, default_offset,
        "offset 欠損は既定 offset 保持（2.4）"
    );
    assert_eq!(
        out[0].balloon_pos,
        PointPx {
            x: 800 + default_offset.x,
            y: 500 + default_offset.y
        },
        "既定 offset が最終 char_pos に追従する（2.4）"
    );
}

/// 2.3: 保存 BalloonOffset 両軸あり → そのまま左上基準 offset として採用し
/// balloon_pos へ反映する（Free・保存基準＝ランタイム基準）。
#[test]
fn apply_saved_balloon_offset_is_derived_free() {
    let snap = snapshot_of(vec![]); // identity
    let placements = vec![placement(
        0,
        Anchor::Free,
        PointPx { x: 100, y: 100 },
        CSZ,
        PointPx { x: -50, y: 0 }, // 既定（保存値で上書きされる）
        BSZ,
    )];
    let entries = vec![
        wp(0, Axis::X, "800"),
        wp(0, Axis::Y, "500"),
        bo(0, Axis::X, "-30"),
        bo(0, Axis::Y, "20"),
    ];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(out[0].char_pos, PointPx { x: 800, y: 500 });
    assert_eq!(
        out[0].balloon_offset,
        PointPx { x: -30, y: 20 },
        "保存値＝左上基準 offset（基準変換なし・2.3）"
    );
    assert_eq!(
        out[0].balloon_pos,
        PointPx { x: 770, y: 520 },
        "balloon_pos ≡ 最終 char_pos + 導出 offset"
    );
}

/// 2.2/2.3（意味反転・2026-07-31 実機裁定）: Bottom でも保存 BalloonOffset は
/// **char 左上基準のまま**採用する（現 char_size を足し戻さない）。
/// 一方 char 窓位置（`char_pos_to_origin_x` 系）は下端中央原点のまま＝無改変で、
/// バルーン基準と char 基準が独立であることを同一檻で弁別する。
/// 空 snapshot ゆえ char_pos は保存値素通し。
#[test]
fn apply_saved_balloon_offset_is_char_top_left_basis_bottom() {
    let snap = snapshot_of(vec![]); // identity（char_pos は保存値のまま）
    let placements = vec![placement(
        0,
        Anchor::Bottom,
        PointPx { x: 100, y: 100 },
        CSZ, // 400x600
        PointPx { x: -50, y: 0 },
        BSZ,
    )];
    // persisted (0, -430) は左上基準ゆえ、そのまま offset になる（旧下端中央基準なら
    // (0+200, -430+600) = (200,170) へ膨らんでいた）。
    let entries = vec![
        wp(0, Axis::X, "800"),
        wp(0, Axis::Y, "500"),
        bo(0, Axis::X, "0"),
        bo(0, Axis::Y, "-430"),
    ];

    let out = apply_restored_placements(placements, &entries, &snap);

    // 保存 x=800 は**原点（下端中央）基準**ゆえ、現寸 w=400 の左上は 800−200=600
    // （char 窓の原点符号化は本変更でも無改変）。
    assert_eq!(out[0].char_pos, PointPx { x: 600, y: 500 });
    assert_eq!(
        out[0].balloon_offset,
        PointPx { x: 0, y: -430 },
        "Bottom でも保存値は左上基準そのまま（char_size を足し戻さない・2.2/2.3）"
    );
    assert_ne!(
        out[0].balloon_offset,
        PointPx { x: 200, y: 170 },
        "下端中央基準の足し戻しが復活していない"
    );
    assert_eq!(out[0].balloon_pos, PointPx { x: 600, y: 70 });
}

/// 6.1: 片軸破損（非数値）→ 当該 scope の char_pos は既定を保持（両軸揃わないと差替えない）。
#[test]
fn apply_one_axis_corrupt_keeps_default() {
    let snap = snapshot_of(vec![]); // identity
    let default = PointPx { x: 100, y: 100 };
    let placements = vec![placement(
        0,
        Anchor::Free,
        default,
        CSZ,
        PointPx { x: -50, y: 0 },
        BSZ,
    )];
    // Y が非数値 → 両軸揃わず＝既定保持。
    let entries = vec![wp(0, Axis::X, "800"), wp(0, Axis::Y, "abc")];

    let out = apply_restored_placements(placements.clone(), &entries, &snap);

    assert_eq!(
        out[0].char_pos, default,
        "片軸破損は既定 char_pos 保持（6.1）"
    );
    assert_eq!(out[0], placements[0], "破損時は当該 scope 恒等");
}

/// 5.1/5.2: 保存位置が現 work area 外でも project_restore で域内へ（Bottom 下端吸着）。
/// merge が project_restore を実際に通していることの結合檻。
#[test]
fn apply_saved_pos_is_reprojected_into_work_area() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    let placements = vec![placement(
        0,
        Anchor::Bottom,
        PointPx { x: 500, y: 440 },
        CSZ,
        PointPx { x: -200, y: 0 },
        BSZ,
    )];
    // 保存 y=2000 は域外 → 下端吸着 y=1040−600=440。
    // 保存 x=500 は**原点（下端中央）基準**ゆえ左上は 500−200=300（域内で保持）。
    let entries = vec![wp(0, Axis::X, "500"), wp(0, Axis::Y, "2000")];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(
        out[0].char_pos,
        PointPx { x: 300, y: 440 },
        "域外保存 y は Bottom 吸着で域内へ（5.1/5.2）・x は原点基準から左上へ戻す"
    );
}

/// scope の usize と PersistKey の u32 の一致取り（大きめ scope でも取り違えない）。
#[test]
fn apply_matches_scope_usize_to_persist_u32() {
    let snap = snapshot_of(vec![]);
    let placements = vec![placement(
        3,
        Anchor::Free,
        PointPx { x: 10, y: 10 },
        CSZ,
        PointPx { x: 0, y: 0 },
        BSZ,
    )];
    let entries = vec![wp(3, Axis::X, "77"), wp(3, Axis::Y, "88")];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(
        out[0].char_pos,
        PointPx { x: 77, y: 88 },
        "scope 3 の u32 一致"
    );
}

/// 決定論: 同一入力→同一出力。
#[test]
fn apply_is_deterministic() {
    let snap = snapshot_of(vec![wa_rect(0, 0, 1920, 1040)]);
    let placements = vec![placement(
        0,
        Anchor::Bottom,
        PointPx { x: 500, y: 440 },
        CSZ,
        PointPx { x: -200, y: 0 },
        BSZ,
    )];
    let entries = vec![wp(0, Axis::X, "600"), wp(0, Axis::Y, "440")];
    let a = apply_restored_placements(placements.clone(), &entries, &snap);
    let b = apply_restored_placements(placements, &entries, &snap);
    assert_eq!(a, b, "同一入力→同一出力");
}

// ------------------------------------------------------------------
// キーワード再導出の素材と保存値の優先順位（要件 4.7・2026-08-14 実機是正）
//
// 要件 4.7 は「永続値を優先し、キーワード指定の適用は初期既定位置の供給にとどめる」。
// 実表示寸確定時の再導出はキーワードの初期既定位置を引き直す仕掛けなので、保存された
// 相対位置が効いている scope では**素材ごと落として**発火させない——落とさないと、
// 再導出がユーザーの保存値をキーワード既定へ静かに上書きしてしまう。
// ------------------------------------------------------------------

/// 保存 offset が両軸そろって効いた scope は再導出の素材を失う（4.7）。
#[test]
fn merge_drops_the_keyword_base_when_a_saved_balloon_offset_wins() {
    let snap = snapshot_of(vec![]); // identity 射影（保存値素通し）
    let keyworded = |scope: usize| ScopePlacement {
        balloon_keyword_base: Some((BalloonXMode::CenterTop, PointPx { x: 0, y: -12 })),
        ..placement(
            scope,
            Anchor::Free,
            PointPx { x: 500, y: 440 },
            CSZ,
            PointPx { x: 17, y: -224 },
            BSZ,
        )
    };
    let placements = vec![keyworded(0), keyworded(1)];
    // scope0 にだけ保存 offset を与える（勝つ経路と勝たない経路を 1 回で covering）。
    let entries = vec![bo(0, Axis::X, "-40"), bo(0, Axis::Y, "-300")];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(
        out[0].balloon_offset,
        PointPx { x: -40, y: -300 },
        "前提＝scope0 は保存 offset が勝っている"
    );
    assert_eq!(
        out[0].balloon_keyword_base, None,
        "保存 offset が勝った scope に再導出の素材が残っている（4.7 の優先順位が反転する）"
    );
    assert_eq!(
        out[1].balloon_keyword_base,
        Some((BalloonXMode::CenterTop, PointPx { x: 0, y: -12 })),
        "保存 offset の無い scope で素材が落ちている（キーワードの初期既定位置が失われる）"
    );
}

/// 片軸だけの保存 offset は「値なし」＝resolver 既定が残る腕であり、素材も残る
/// （既存の片軸縮退規則と同じ側へ倒れることを固定する）。
#[test]
fn merge_keeps_the_keyword_base_when_the_saved_offset_is_half_missing() {
    let snap = snapshot_of(vec![]);
    let base = Some((BalloonXMode::CenterBottom, PointPx { x: 5, y: 6 }));
    let placements = vec![ScopePlacement {
        balloon_keyword_base: base,
        ..placement(
            0,
            Anchor::Free,
            PointPx { x: 500, y: 440 },
            CSZ,
            PointPx { x: 17, y: 687 },
            BSZ,
        )
    }];
    // y 軸だけ保存されている（片軸欠損＝採用しない既存規則）。
    let entries = vec![bo(0, Axis::Y, "-300")];

    let out = apply_restored_placements(placements, &entries, &snap);

    assert_eq!(
        out[0].balloon_offset,
        PointPx { x: 17, y: 687 },
        "前提＝片軸欠損では resolver 既定 offset が保持される"
    );
    assert_eq!(
        out[0].balloon_keyword_base, base,
        "既定 offset を保持した腕で素材が落ちている（offset と素材の腕がずれている）"
    );
}
