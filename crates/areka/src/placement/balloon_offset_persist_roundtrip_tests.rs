//! **保存・復元の往復**の檻（areka-P0-balloon-offset-dpi・task 7・design D15・
//! 「Testing Strategy > 保存・復元テスト」1〜3・要件 5.2／5.4／5.5／5.6／7.1／7.2）。
//!
//! # ⚠ ここへ来た保守者へ——これは開発者裁定の非回帰テストである（要件 7.2）
//!
//! 復元経路が保存値を**換算しない**のは実装の取りこぼしではない。**開発者が明示的に
//! そう裁定した**結果であり、本ファイルはその裁定が黙って反転しないために在る。
//!
//! - **裁定**: 「拡大率の変更をまたぐ保存位置の追従はしない」。
//! - **いつ・誰が**: 2026-08-14、開発者。`areka-P0-windowposition-limit` の実機検証の場で
//!   下され、`areka-P0-dpi-transition-atomicity` 要件 7 が明示的に踏襲している。当該裁定は
//!   **バルーンの相対位置**を対象とした検証の中で下されており、キャラ窓位置に限らない。
//! - **何を承知の上か**: 保存値は物理 px であり、100% で保存した位置を 200% で読むと
//!   意味が変わって初回起動時に一度崩れる（実測で再現済み）。**それでも現行仕様で問題なし**
//!   と裁定され、追跡 spec を立てないことまで決められている。
//! - **費用の前提が変わってもなお踏襲**: 2026-08-14 の裁定は費用（DPI 変化の検出・
//!   再スケールの基準・保存形式の版管理の 3 つ）を理由としており、本仕様は要件 3 の実装で
//!   **前 2 つを作ってしまった**。この変化を開発者へ提示したうえで、2026-08-27 に改めて
//!   **裁定の継承が選ばれた**——残る 3 つ目（保存形式の版管理）を導入しないこと、および
//!   先行 2 spec の記録を書き換えないことが、追従の利得より優先される。
//!
//! したがって「復元時に保存値を現在の表示 DPI へ換算すれば直る」は**直っていない**。
//! 本ファイルが赤くなったら、まず自分の変更が上の裁定を覆していないかを疑うこと。覆すなら
//! 要件 5.2 と本 doc と互換記録を先に改め、開発者の裁定を取り直すのが順序である。
//!
//! # 本ファイルが持つもの／持たないもの（重複を作らないための分担）
//!
//! | 主張 | 置き場所 |
//! |---|---|
//! | 復元経路が運ぶ**基準対**の腕分け（保存値あり／欠損／片軸） | `persist_restore_offset_base_tests.rs`（task 2.2） |
//! | 変換規則そのもの（丸め・往復・縮退・全腕の到達） | `follow_offset_space_tests.rs`（task 1.2〜） |
//! | 遷移 × アンカーの相の結合（`FrameHarness` で本番の相順を回す） | `frame_balloon_offset_follow_tests.rs`（task 6.5） |
//! | **保存 → 復元 → 係留 → 追随**を 1 本に貫いた往復 | **本ファイル** |
//!
//! 本ファイルだけが、保存 entries の構築（[`balloon_offset_entries`]）から復元 merge
//! （[`apply_restored_placements`]）を経て追従 Component（[`BalloonFollow`]）の 3 段
//! （**復元＝未係留 → 最初の観測で係留・値は不変 → 次の遷移で追随**）までを
//! **1 本の筋として**通す。上の 3 ファイルはいずれもこの筋のどこか 1 区間しか見ていない。
//!
//! # 3 段を個別に観測できる形にしてある（中段が要点）
//!
//! 両端（復元直後と追随後）だけを見る檻は、**係留が黙って値を換算していても緑になる**。
//! ゆえに中段（[`OffsetRescale::Anchored`] を返した直後）で「値が 1 bit も動いていない」ことを
//! **明示の assert として**置く。3 段はそれぞれ次の観測量で区別する。
//!
//! | 段 | 判定語 | `base().dpi` | `offset()` |
//! |---|---|---|---|
//! | ⑴ 復元直後 | （観測前） | `None`＝未係留 | 保存値と bit 同一 |
//! | ⑵ 最初の観測 | `Anchored` | `Some(その時の表示 DPI)` | **不変**（保存値と bit 同一） |
//! | ⑶ 次の遷移 | `Rescaled` | `Some(⑵ で係留した DPI)`（不変） | 係留基準からの表示 DPI 比 |
//!
//! # 空虚さの排除（「換算していない」と「換算したが同じ値になった」の弁別）
//!
//! 保存値は配置式が出す既定 offset と**別の値**にしてあり、係留に使う表示 DPI は配置式が
//! 刻む採寸 DPI（[`MEASURE_DPI`]）と**別の値**にしてある。もし復元腕が採寸 DPI を継いで
//! いたら（＝保存値を換算していたら）値は動く——その反実仮想の値を
//! [`converting_the_saved_value_at_restore_would_move_it`] が逐語で押さえているので、
//! 「換算していない」は「換算したが偶然同じ値だった」と取り違えようがない。
//!
//! # 期待値は本番の出力から採らない
//!
//! 追随後の期待値は丸めの単一権威 `ScaleRatio::scale_len`
//! （`(2·len·num + den) ÷ (2·den)`＝round half away from zero）を紙の上で適用した逐語値で
//! ある。算術は各定数の doc に残す。追随相を走らせて出た値を写していない。

use super::*;
use crate::placement::follow::{BalloonFollow, OffsetBase, OffsetRescale, rescale_follow_offset};
use bevy_ecs::entity::Entity;
use wintf::ecs::DPI;

// ---------------------------------------------------------------------------
// 行列の定義
// ---------------------------------------------------------------------------

const CSZ: SizePx = SizePx { w: 400, h: 600 };
const BSZ: SizePx = SizePx { w: 200, h: 300 };

/// 配置式が刻む**採寸 DPI**（係留済みの基準対が持つ値）。
///
/// 96 の倍数でない値を採る（要件 7.6）。保存値採用腕がこれを継いでいないことが
/// 本ファイルの弁別対象ゆえ、下の [`RESTORE_DPIS`] のどの水準とも**別の意味**を持つ。
const MEASURE_DPI: DPI = DPI {
    dpi_x: 120,
    dpi_y: 120,
};

/// 保存されていたオフセット（キャラ窓左上相対・物理 px）。
///
/// 両軸とも**非ゼロ**（0 は何倍しても 0 なので換算の有無を区別できない）かつ
/// [`FORMULA_DEFAULT`] と**別の値**（採否の優先順位が空虚にならない）。
const SAVED: PointPx = PointPx { x: -512, y: -48 };

/// 配置式が出す既定オフセット（保存値が無いときに残る値）。[`SAVED`] とは別の値。
const FORMULA_DEFAULT: PointPx = PointPx { x: -50, y: 10 };

/// 復元先の表示 DPI の候補。**保存側の拡大率とは無関係**であることが本仕様の要点ゆえ、
/// 採寸 DPI と一致する水準（120）も、しない水準（96／144／192）も並べる。
///
/// 144 は 96 の倍数でない（要件 7.6）。
const RESTORE_DPIS: [u16; 4] = [96, 120, 144, 192];

/// 全アンカー（`Anchor` の 5 腕）。保存・復元の往復はアンカーに依らないことを主張する。
const ANCHORS: [Anchor; 5] = [
    Anchor::Top,
    Anchor::Bottom,
    Anchor::Left,
    Anchor::Right,
    Anchor::Free,
];

/// 3 段の筋で使う復元先の表示 DPI（⑵ で係留される水準）。採寸 DPI とも保存側とも違う。
const STAGE_ANCHOR_DPI: u16 = 144;

/// 3 段の筋で使う次の遷移先の表示 DPI（⑶ で追随する水準）。
const STAGE_NEXT_DPI: u16 = 192;

/// ⑶ 追随後の**手計算の期待オフセット**（係留基準 `SAVED @144` → 表示 DPI 192）。
///
/// 比は表示 DPI の整数 2 つから直に組む＝`192 ÷ 144`。`scale_len` は非負の大きさへ
/// `(2·len·num + den) ÷ (2·den)`（切り捨て除算）を返し、`scale_signed` が符号を戻す。
///
/// - x: `|−512|` → `(2·512·192 + 144) ÷ (2·144) = 196752 ÷ 288 = 683.16… → 683` → **−683**
/// - y: `|−48|` → `(2·48·192 + 144) ÷ (2·144) = 18576 ÷ 288 = 64.5 → 64` → **−64**
const STAGE_RESCALED: PointPx = PointPx { x: -683, y: -64 };

/// **反実仮想**——保存値採用腕が採寸 DPI（120）を継いでいた場合に、最初の観測で
/// 出てしまう値（表示 DPI ÷ 120 で換算される）。逐語の手計算。
///
/// | 復元先 | 比 | x = `|−512|` | y = `|−48|` |
/// |---|---|---|---|
/// | 96 | 96/120 | `(2·512·96 + 120) ÷ 240 = 98424 ÷ 240 = 410.1 → 410` | `(2·48·96 + 120) ÷ 240 = 9336 ÷ 240 = 38.9 → 38` |
/// | 144 | 144/120 | `(2·512·144 + 120) ÷ 240 = 147576 ÷ 240 = 614.9 → 614` | `(2·48·144 + 120) ÷ 240 = 13944 ÷ 240 = 58.1 → 58` |
/// | 192 | 192/120 | `(2·512·192 + 120) ÷ 240 = 196728 ÷ 240 = 819.7 → 819` | `(2·48·192 + 120) ÷ 240 = 18552 ÷ 240 = 77.3 → 77` |
///
/// 符号は [`SAVED`] から引き継ぐ。120 は恒等ゆえ表に無い（換算しても値が動かない水準＝
/// 弁別に使えない）。
const COUNTERFACTUAL_IF_CONVERTED: [(u16, PointPx); 3] = [
    (96, PointPx { x: -410, y: -38 }),
    (144, PointPx { x: -614, y: -58 }),
    (192, PointPx { x: -819, y: -77 }),
];

// ---------------------------------------------------------------------------
// 補助
// ---------------------------------------------------------------------------

fn dpi(v: u16) -> DPI {
    DPI { dpi_x: v, dpi_y: v }
}

fn snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        }],
    }
}

/// 配置式が出した（＝基準対が**係留済み**の）1 スコープ分の出力を模す。
fn placement_from_formula(anchor: Anchor) -> ScopePlacement {
    let char_pos = PointPx { x: 500, y: 300 };
    ScopePlacement {
        scope: 0,
        char_pos,
        char_size: CSZ,
        balloon_pos: PointPx {
            x: char_pos.x + FORMULA_DEFAULT.x,
            y: char_pos.y + FORMULA_DEFAULT.y,
        },
        balloon_size: BSZ,
        balloon_offset: FORMULA_DEFAULT,
        balloon_offset_base: OffsetBase {
            offset: FORMULA_DEFAULT,
            dpi: Some(MEASURE_DPI),
        },
        // 画面内維持の関門を有効にしたまま往復させる——復元が「補正後の表示位置」を
        // オフセットへ焼き付けないこと（要件 5.6）は、関門が効き得る形でしか主張できない。
        balloon_limit: true,
        anchor,
        balloon_keyword_base: None,
    }
}

/// 保存 → 復元の 1 往復。保存側は**本番の保存 entries 構築**を通す。
fn roundtrip(anchor: Anchor, saved: Option<PointPx>) -> ScopePlacement {
    let entries = match saved {
        Some(v) => balloon_offset_entries(0, v),
        None => vec![],
    };
    let out =
        apply_restored_placements(vec![placement_from_formula(anchor)], &entries, &snapshot());
    out.into_iter().next().expect("1 スコープ分が返る")
}

/// 追随相（`emo2_boot::frame::balloon_offset_follow`）の**腕の割り当てを写した**観測 1 回。
///
/// 値を決めるのは本番の純関数 [`rescale_follow_offset`]、書くのは本番の書込口
/// （[`BalloonFollow::anchor_base_dpi`]／[`BalloonFollow::apply_rescaled`]）であり、
/// ここが持つのは「どの腕でどちらを呼ぶか」の対応だけである。その対応が本番と一致して
/// いることは相の結合の檻（`frame_balloon_offset_follow_tests.rs`・
/// `frame_balloon_offset_keyword_gate_tests.rs`・`frame_balloon_offset_roundtrip_tests.rs`）が World 上で
/// 固定している——ただし**恒等比の腕だけは 2026-08-28 までどの World 檻も固定していなかった**
/// （戻りの遷移が 1 本も無かった）。その穴を塗り直したのが上記 3 ファイル目である。本ファイルは
/// **腕そのものを毎回 assert する**ことで、写し取った対応が黙ってずれないようにする。
fn observe(follow: &mut BalloonFollow, current: DPI) -> OffsetRescale {
    let verdict = rescale_follow_offset(follow.base(), current);
    match verdict {
        OffsetRescale::Anchored { base_dpi } => follow.anchor_base_dpi(base_dpi),
        OffsetRescale::Rescaled { offset, .. } => follow.apply_rescaled(offset),
        // 恒等比の腕も**基準から引き直す**（2026-08-28 実装時是正・本番と同じ写し）。
        OffsetRescale::Unchanged => {
            let base_offset = follow.base().offset;
            follow.apply_rescaled(base_offset);
        }
        OffsetRescale::Unresolved { .. } => {}
    }
    verdict
}

fn follow_of(placement: &ScopePlacement) -> BalloonFollow {
    BalloonFollow::new(
        Entity::from_raw_u32(1).expect("テスト用 entity index は有効"),
        placement.balloon_offset_base,
    )
}

// ---------------------------------------------------------------------------
// 1. 保存値は換算されない（要件 5.2／5.6／7.2・design Testing Strategy 保存・復元 1）
// ---------------------------------------------------------------------------

/// 5.2／5.6／7.2: 保存 entries の構築から復元 merge までを通した往復で、採用された
/// オフセットは保存値と **bit 同一**であり、基準対は**未係留**のまま運ばれる。
///
/// アンカー 5 腕すべてで成立する（保存・復元の往復はアンカーに依らない）。関門
/// （`balloon_limit: true`）を有効にしたまま通しているので、「補正後の表示位置を
/// オフセットへ焼き付けない」（要件 5.6）も同時に押さえている。
#[test]
fn a_saved_offset_survives_the_save_restore_roundtrip_bit_for_bit() {
    for anchor in ANCHORS {
        let out = roundtrip(anchor, Some(SAVED));
        assert_eq!(
            out.balloon_offset, SAVED,
            "{anchor:?}: 保存 → 復元の往復で採用値が変わった（換算も補正も入ってはならない）"
        );
        assert_eq!(
            out.balloon_offset_base,
            OffsetBase::unpinned(SAVED),
            "{anchor:?}: 保存値採用腕の基準対は未係留（dpi: None）かつ保存値と bit 同一"
        );
        assert_ne!(
            SAVED, FORMULA_DEFAULT,
            "探針が退化している——保存値と配置式既定が同値では採否も換算も見分けられない"
        );
    }
}

/// 5.2／7.2: 保存 entries が載せる文字列は**生値そのもの**である（補正後の表示位置でも
/// 換算後の値でもない・要件 5.6 の保存側）。
#[test]
fn the_persisted_entries_carry_the_raw_offset_verbatim() {
    let entries = balloon_offset_entries(0, SAVED);
    let value_of = |axis: Axis| {
        entries
            .iter()
            .find(|(k, _)| *k == PersistKey::BalloonOffset { scope: 0, axis })
            .map(|(_, v)| v.as_str())
            .expect("両軸が載る")
    };
    assert_eq!(value_of(Axis::X), "-512", "保存されるのは生値の逐語表現");
    assert_eq!(value_of(Axis::Y), "-48", "保存されるのは生値の逐語表現");
}

/// 5.2／7.2: **復元先の表示 DPI が何であれ**、最初の観測は値を 1 bit も動かさずに
/// 係留するだけである（腕は必ず [`OffsetRescale::Anchored`]）。
///
/// 「両端しか見ない檻は係留が黙って換算していても緑になる」への対処が本テストであり、
/// 3 段の**中段**を単独で観測している。
#[test]
fn the_first_observation_anchors_without_moving_the_value_at_any_restore_dpi() {
    for anchor in ANCHORS {
        for d in RESTORE_DPIS {
            let out = roundtrip(anchor, Some(SAVED));
            let mut follow = follow_of(&out);
            assert_eq!(follow.offset(), SAVED, "{anchor:?}/{d}: 復元直後（⑴）");
            assert_eq!(follow.base().dpi, None, "{anchor:?}/{d}: ⑴ は未係留");

            let verdict = observe(&mut follow, dpi(d));

            assert_eq!(
                verdict,
                OffsetRescale::Anchored { base_dpi: dpi(d) },
                "{anchor:?}/{d}: 未係留の基準は観測した表示 DPI を逐語で採る"
            );
            assert_eq!(
                follow.offset(),
                SAVED,
                "{anchor:?}/{d}: ⑵ 係留で値が動いた——換算してはならない（開発者裁定）"
            );
            assert_eq!(
                follow.base(),
                OffsetBase {
                    offset: SAVED,
                    dpi: Some(dpi(d)),
                },
                "{anchor:?}/{d}: ⑵ の基準は値そのままで表示 DPI だけが刻まれる"
            );
        }
    }
}

/// 7.2（空虚さの排除）: もし復元腕が採寸 DPI を継いでいたら、最初の観測で値は**動く**。
///
/// 逐語の反実仮想を置くことで、上の 2 本の「動かない」が「換算したが偶然同じ値だった」
/// では**あり得ない**ことを示す。ここが赤くなったのは丸めの権威が変わったときであり、
/// 裁定の話ではない。
#[test]
fn converting_the_saved_value_at_restore_would_move_it() {
    for (d, expected) in COUNTERFACTUAL_IF_CONVERTED {
        let converted_base = OffsetBase {
            offset: SAVED,
            dpi: Some(MEASURE_DPI),
        };
        assert_eq!(
            rescale_follow_offset(converted_base, dpi(d)),
            OffsetRescale::Rescaled {
                offset: expected,
                saturated: false,
            },
            "採寸 DPI を継いだ基準は表示 DPI {d} で換算される（＝弁別可能）"
        );
        assert_ne!(
            expected, SAVED,
            "反実仮想が保存値と同値では弁別に使えない（{d}）"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. 復元後の遷移には追随が効く（要件 5.4／7.1・保存・復元 2）
// ---------------------------------------------------------------------------

/// 5.2／5.4／7.1: **復元（未係留）→ 最初の観測で係留（値は不変）→ 次の遷移で追随**の
/// 3 段が、この順に、それぞれ別の観測量として成立する。
///
/// 空振り防止の証人を主張の前に置く: ⑵ と ⑶ で表示 DPI が実際に違う（比 ≠ 1）・
/// オフセットが両軸とも非ゼロ・⑶ の値が ⑵ と違う。
#[test]
fn restore_then_anchor_then_follow_runs_as_three_distinct_stages() {
    assert_ne!(
        STAGE_ANCHOR_DPI, STAGE_NEXT_DPI,
        "証人 1: ⑵→⑶ の比が 1 では追随の有無を見分けられない"
    );
    assert!(
        SAVED.x != 0 && SAVED.y != 0,
        "証人 2: 0 は何倍しても 0 ゆえ換算の有無を見分けられない"
    );

    // ⑴ 復元——未係留のまま、保存値と bit 同一。
    let out = roundtrip(Anchor::Free, Some(SAVED));
    let mut follow = follow_of(&out);
    assert_eq!(follow.offset(), SAVED, "⑴ 復元直後の値は保存値と bit 同一");
    assert_eq!(follow.base(), OffsetBase::unpinned(SAVED), "⑴ は未係留");

    // ⑵ 最初の観測——係留するが、値は 1 bit も動かない。
    let anchored = observe(&mut follow, dpi(STAGE_ANCHOR_DPI));
    assert_eq!(
        anchored,
        OffsetRescale::Anchored {
            base_dpi: dpi(STAGE_ANCHOR_DPI)
        },
        "⑵ の腕は係留である（追随ではない）"
    );
    assert_eq!(
        follow.offset(),
        SAVED,
        "⑵ 係留では値が変わらない——ここが両端だけ見る檻の盲点"
    );
    assert_eq!(
        follow.base(),
        OffsetBase {
            offset: SAVED,
            dpi: Some(dpi(STAGE_ANCHOR_DPI)),
        },
        "⑵ 係留後の基準は「保存値 @ その時の表示 DPI」"
    );

    // ⑶ 次の遷移——係留した基準からの表示 DPI 比で追随する。
    let rescaled = observe(&mut follow, dpi(STAGE_NEXT_DPI));
    assert_eq!(
        rescaled,
        OffsetRescale::Rescaled {
            offset: STAGE_RESCALED,
            saturated: false,
        },
        "⑶ の腕は追随である（係留でも無遷移でもない）"
    );
    assert_eq!(
        follow.offset(),
        STAGE_RESCALED,
        "⑶ 追随後の値は係留基準からの表示 DPI 比（逐語）"
    );
    assert_ne!(
        STAGE_RESCALED, SAVED,
        "証人 3: ⑶ が ⑵ と同値では追随が走ったか判らない"
    );
    assert_eq!(
        follow.base(),
        OffsetBase {
            offset: SAVED,
            dpi: Some(dpi(STAGE_ANCHOR_DPI)),
        },
        "⑶ は基準対を動かさない（出力を入力へ戻さない＝往復無誤差の前提）"
    );
}

/// 5.4: 係留が済んでいない限り追随は起きない——⑵ を飛ばして 2 度続けて観測しても、
/// 1 度目は必ず係留であり、追随はその後にしか来ない（段の**順序**の固定）。
#[test]
fn the_follow_never_precedes_the_anchoring() {
    let out = roundtrip(Anchor::Free, Some(SAVED));
    let mut follow = follow_of(&out);

    // 復元直後にいきなり「別の」表示 DPI を観測しても、腕は係留である。
    let first = observe(&mut follow, dpi(STAGE_NEXT_DPI));
    assert_eq!(
        first,
        OffsetRescale::Anchored {
            base_dpi: dpi(STAGE_NEXT_DPI)
        },
        "未係留の基準は、表示 DPI が保存側と違っても、まず係留される"
    );
    assert_eq!(
        follow.offset(),
        SAVED,
        "初回観測で追随が走ってはならない（保存値の二重拡大）"
    );

    // 同じ水準をもう一度観測しても無遷移（値も基準も動かない）。
    assert_eq!(
        observe(&mut follow, dpi(STAGE_NEXT_DPI)),
        OffsetRescale::Unchanged,
        "係留済みで同一 DPI なら無遷移"
    );
    assert_eq!(follow.offset(), SAVED);
}

// ---------------------------------------------------------------------------
// 3. 採否の優先順位と生値保存は不変（要件 5.5／5.6・保存・復元 3）
// ---------------------------------------------------------------------------

/// 5.5: 採否の優先順位は**本仕様の前後で変わらない**——保存値があれば保存値、無ければ
/// 配置式の既定。片軸だけの保存値は採用されない（既存の受理境界も不変）。
///
/// `persist_restore_offset_base_tests.rs` は同じ順位を**基準対の腕**の側から押さえて
/// いる。本テストは往復（保存 entries を本番の構築子で作る）の側から押さえる。
#[test]
fn the_adoption_priority_is_unchanged() {
    for anchor in ANCHORS {
        // 保存値あり → 保存値が勝つ。
        let adopted = roundtrip(anchor, Some(SAVED));
        assert_eq!(
            adopted.balloon_offset, SAVED,
            "{anchor:?}: 保存値があれば保存値が勝つ"
        );

        // 保存値なし → 配置式の既定が残り、基準対も配置式のもの（係留済み）のまま。
        let defaulted = roundtrip(anchor, None);
        assert_eq!(
            defaulted.balloon_offset, FORMULA_DEFAULT,
            "{anchor:?}: 保存値が無ければ配置式の既定"
        );
        assert_eq!(
            defaulted.balloon_offset_base,
            OffsetBase {
                offset: FORMULA_DEFAULT,
                dpi: Some(MEASURE_DPI),
            },
            "{anchor:?}: 欠損腕は配置式の基準対を素通しする（未係留へ落とさない）"
        );

        // 片軸だけ → 採用しない（既定のまま）。
        let half = apply_restored_placements(
            vec![placement_from_formula(anchor)],
            &[(
                PersistKey::BalloonOffset {
                    scope: 0,
                    axis: Axis::X,
                },
                SAVED.x.to_string(),
            )],
            &snapshot(),
        );
        assert_eq!(
            half[0].balloon_offset, FORMULA_DEFAULT,
            "{anchor:?}: 片軸のみの保存値は採用腕へ入らない"
        );
    }
}

/// 5.6: 復元は**補正後の表示位置をオフセットへ焼き付けない**。
///
/// 画面外へ大きく外れる保存値（関門が効き得る値）を通しても、`balloon_offset` は保存値の
/// まま——復元が動かすのは `balloon_pos` の導出であって、保存された相対量ではない。
#[test]
fn the_restore_never_bakes_a_correction_into_the_saved_offset() {
    // 作業領域（0..1920 × 0..1040）から確実に外れる相対量。
    let far_off = PointPx {
        x: -9_000,
        y: -7_000,
    };
    for anchor in ANCHORS {
        let out = roundtrip(anchor, Some(far_off));
        assert_eq!(
            out.balloon_offset, far_off,
            "{anchor:?}: 画面外でもオフセットは生値のまま（補正を焼き付けない）"
        );
        assert_eq!(
            out.balloon_offset_base,
            OffsetBase::unpinned(far_off),
            "{anchor:?}: 基準対も生値のまま未係留"
        );
        assert!(
            out.balloon_limit,
            "{anchor:?}: 関門そのものは復元の対象外（毎起動 descript から解決する）"
        );
    }
}
