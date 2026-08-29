//! **拡大率遷移でのバルーン追従オフセットの追随**——相の結合の行列
//! （areka-P0-balloon-offset-dpi・task 6.5・design「Testing Strategy > Integration Tests」
//! 1／2 の後半／4／5／6・要件 3.2／3.5／4.2／7.1／7.3／9.8）。
//!
//! # 本ファイルが持つもの／持たないもの
//!
//! 相の結合（`FrameHarness` で本番と同じ順に相を回す）で追随を見る檻は 3 ファイルに分かれる。
//! 重複を作らないための分担は次のとおりで、本ファイルは**残りの 5 つ**を持つ。
//!
//! | design の項目 | 置き場所 |
//! |---|---|
//! | 1. 遷移 × アンカーの行列 | **本ファイル**（[`the_transition_matrix_follows_the_display_dpi_ratio`]） |
//! | 2. キーワードとの排他（素材が残る腕・受容残余の腕） | `frame_balloon_offset_keyword_gate_tests.rs`（task 6.3） |
//! | 2. 同（**素材消費後**の腕と揃えの残差） | **本ファイル**（[`the_transition_after_the_material_is_consumed_keeps_the_centering_within_the_allowance`]） |
//! | 3. べき等 skip の収束 | `frame_balloon_offset_converge_tests.rs`（task 6.2） |
//! | 4. 待ち札との共存 | **本ファイル**（[`a_window_held_for_sync_regains_the_follow_after_the_hold_is_released`]） |
//! | 5. 拡大率が変わらない寸法変化では触らない | **本ファイル**（面の切替／作業領域の再スナップの 2 本） |
//! | 6. ドラッグ由来にも同一規則 | **本ファイル**（確立の事後条件と追随の 2 本） |
//!
//! # 期待値は本番の出力から採らない（丸めの独立導出）
//!
//! 期待オフセットは [`EXPECTED_OFFSETS`] に**手計算の逐語値**として置く。導出は丸めの単一権威
//! `ScaleRatio::scale_len`（`(2·len·num + den) ÷ (2·den)`＝round half away from zero）を
//! 紙の上で適用したものであり、追随相を走らせて出た値を写していない——写すと「実装が何を
//! 出しても緑になる」空虚な行列になる。各値の算術は同定数の doc に逐条で残す。
//!
//! # 空振り防止の証人（design Integration 1 が名指しする 3 つ）
//!
//! 行列の各組は次の 3 点を**主張の前に**固定する。1 つでも欠けると「追随が無くても緑になる
//! 行列」になり、要件 7.1 の網羅が意味を失う。
//!
//! 1. **比が 1 でない**——遷移の前後で表示 DPI が実際に違う。
//! 2. **オフセットが非ゼロ**——0 は何倍しても 0 なので、追随の有無を区別できない。
//! 3. **バルーンが実際に動いた**——値だけでなく窓書込と `WindowPos` の双方で確かめる。
//!
//! 加えて「旧値と期待値が違う」ことも毎組で主張する。追随が丸ごと無くても値が一致するなら、
//! その組は探針として退化している。
//!
//! # 追随を無効化すると落ちる（是正前は失敗する側・要件 7.4）
//!
//! 追随の適用（`rescale_balloon_follow_offset` の中の `apply_rescaled` 呼出）を潰すと、
//! 本ファイルの 7 本のうち**追随を主張する 4 本**が赤になる（実測 2026-08-28）——行列・
//! 素材消費後の残差・待ち札の解除後・ドラッグ由来の追随。据置きを主張する 3 本（面の切替・
//! 作業領域の再スナップ・ドラッグ確立の事後条件）は緑のまま残る。これらは追随が**走らない
//! こと**を主張しているので、「全部が赤くなる」ことは正しさの条件ではない。
//!
//! ⚠ 相そのもの（関数の先頭で `Unchanged` を返す）を潰すと**7 本すべて**が赤になるが、
//! 落ちるのは [`settle_at`] の自己検査（基準の係留）であって主張の側ではない——係留も
//! 同じ関数が担うためである。追随の是正対を確かめたいときは `apply_rescaled` の側を潰す
//! こと。相ごと潰した結果を「行列が効いている」証拠に使うと帰属を取り違える。

use wintf::ecs::drag::DragEvent;
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::SetWindowPosCommand;
use wintf::ecs::{DPI, Point, WindowPos};

use crate::placement::config::BalloonXMode;
use crate::placement::dpi_sync::DpiSyncHold;
use crate::placement::follow::{Anchored, BalloonFollow, on_balloon_drag};
use crate::placement::resolver::{Anchor, PointPx, SizePx, keyword_balloon_pos};
use crate::placement::spawn::BalloonKeywordBase;
use crate::placement::transition_diag::{OFFSET_VERDICT_KEYWORD_PENDING, OFFSET_VERDICT_RESCALED};

use super::test_support::{
    FakeReports, FrameHarness, capture_logs, s2_monitors_with_work_area, s2_neighbor_work_area,
    s2_taskbar_hidden_work_area, s2_work_area_for_dpi,
};
use super::*;

// ---------------------------------------------------------------------------
// 行列の定義（3 遷移 × 5 アンカー × 2 スコープ＝30 組）
// ---------------------------------------------------------------------------

/// 表示 DPI 遷移の 3 組（design Integration 1 が名指しする組）。
///
/// `96→120` は比 5/4（96 の倍数でない DPI・要件 7.6）、`96→192` は比 2（丸めが 1 度も
/// 起きない対照）、`120→192` は比 8/5（分母・分子とも 96 の倍数でない）。
const TRANSITIONS: [(u16, u16); 3] = [(96, 120), (96, 192), (120, 192)];

/// 全アンカー（`Anchor` の 5 腕を漏れなく列挙する）。
///
/// 追随は基準対と表示 DPI だけから値を決めるため、アンカーは**結果に影響しないことが
/// 期待される軸**である。行列がそれを実測で固定する（[`EXPECTED_OFFSETS`] はアンカーに
/// 依らない 1 組だけを持つ）。アンカーが違えばキャラ窓の射影先は変わるので、
/// 「同じ期待値が 5 腕すべてで成立する」は空虚な主張ではない。
///
/// 配列は腕が増えてもコンパイルエラーにならないので、[`anchor_enumeration_is_exhaustive`]
/// が `match` で網羅を構造的に保つ（「全アンカー」が黙って一部アンカーへ縮むのを防ぐ）。
const ANCHORS: [Anchor; 5] = [
    Anchor::Top,
    Anchor::Bottom,
    Anchor::Left,
    Anchor::Right,
    Anchor::Free,
];

/// 行列が到達すべき組の総数（3 遷移 × 5 アンカー × 2 スコープ）。
const MATRIX_COMBINATIONS: usize = TRANSITIONS.len() * ANCHORS.len() * 2;

/// 行列 30 組のうち、**バルーンの X の恒等式を免除する**組の件数。
///
/// `Anchor::Left` はキャラ窓を作業領域左端（31）へ固定するので、scope 0 の追従オフセット
/// （−412 系）が置く行き先は主モニタ（31..2574）とも隣接モニタ（2574..3874）とも交差しない
/// ——可視性ガードが X を引き戻すのが正しい。3 遷移 × 1 アンカー × 1 スコープ＝3 組。
/// 追随そのもの（offset の値）はこの 3 組でも通常どおり主張する。
const MATRIX_CLAMPED_COMBINATIONS: usize = TRANSITIONS.len();

/// 基準オフセット（`frame_test_support::resnap_placements` の fixture 値・scope 昇順）。
///
/// 追随の基準対は起動直後の整地で**値を変えずに**係留されるので（要件 5.2）、遷移の
/// 入力はつねにこの 2 値である。両軸とも非ゼロ＝空振り防止の証人 2 を満たす。
const BASE_OFFSETS: [PointPx; 2] = [PointPx { x: -412, y: -25 }, PointPx { x: 285, y: -19 }];

/// **手計算の期待オフセット**（[`TRANSITIONS`] の順 × scope 昇順）。
///
/// 丸めの単一権威 `ScaleRatio::scale_len` は非負の大きさに対し
/// `(2·len·num + den) ÷ (2·den)`（切り捨て除算）を返し、`scale_signed` が符号を戻す。
/// 各値の算術は次のとおりで、いずれも追随相の出力を写したものではない。
///
/// | 遷移 | 比 | scope 0 x=412 | scope 0 y=25 | scope 1 x=285 | scope 1 y=19 |
/// |---|---|---|---|---|---|
/// | 96→120 | 5/4 | (4120+4)/8=515.5→**515** | (250+4)/8=31.75→**31** | (2850+4)/8=356.75→**356** | (190+4)/8=24.25→**24** |
/// | 96→192 | 2/1 | 412·2=**824** | 25·2=**50** | 285·2=**570** | 19·2=**38** |
/// | 120→192 | 8/5 | (6592+5)/10=659.7→**659** | (400+5)/10=40.5→**40** | (4560+5)/10=456.5→**456** | (304+5)/10=30.9→**30** |
///
/// 符号は [`BASE_OFFSETS`] のものをそのまま引き継ぐ（`scale_signed` は大きさだけを権威へ
/// 委ね、符号を保存する）。
const EXPECTED_OFFSETS: [[PointPx; 2]; 3] = [
    // 96→120
    [PointPx { x: -515, y: -31 }, PointPx { x: 356, y: -24 }],
    // 96→192
    [PointPx { x: -824, y: -50 }, PointPx { x: 570, y: -38 }],
    // 120→192
    [PointPx { x: -659, y: -40 }, PointPx { x: 456, y: -30 }],
];

/// 単発の腕で使う遷移前の水準（作者基準と等倍）。
const LOW_DPI: u16 = 96;

/// 単発の腕で使う遷移後の水準（比 2＝丸めが起きない）。
const HIGH_DPI: u16 = 192;

// ---------------------------------------------------------------------------
// 駆動口（兄弟ファイルの `settle` と同型・基準の係留まで済ませる）
// ---------------------------------------------------------------------------

/// 当該スコープの追従 Component。
fn follow_of(harness: &FrameHarness, scope: usize) -> BalloonFollow {
    harness
        .world
        .get::<BalloonFollow>(harness.char_window(scope))
        .copied()
        .expect("キャラ窓に BalloonFollow がある")
}

/// 当該スコープの追従オフセット（現在値）。
fn offset_of(harness: &FrameHarness, scope: usize) -> PointPx {
    follow_of(harness, scope).offset()
}

/// 窓の位置（`WindowPos.position`）。
fn pos_of(harness: &FrameHarness, entity: Entity) -> Point {
    harness
        .world
        .get::<WindowPos>(entity)
        .and_then(|wp| wp.position)
        .expect("窓に位置がある")
}

/// 窓の現寸（`WindowPos.size`）。
fn size_of(harness: &FrameHarness, entity: Entity) -> SizePx {
    let size = harness
        .world
        .get::<WindowPos>(entity)
        .and_then(|wp| wp.size)
        .expect("窓に寸がある");
    SizePx {
        w: size.width,
        h: size.height,
    }
}

/// 指定スコープ・指定種別の窓書込だけを取り出す（兄弟 2 ファイルと同型）。
fn writes_for(writes: &[SetWindowPosCommand], scope: u32, kind: &str) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind)
        .cloned()
        .collect()
}

/// `kind=offset` の観測行だけを取り出す。
fn offset_lines(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("kind=offset")).collect()
}

/// 追随後の相対位置が置くバルーン矩形が、**どれかの作業領域と交差するか**。
///
/// 交差しなければ可視性ガード（`guard_visibility`・別仕様が所有し別の檻が固定している）が
/// X を作業領域内へ引き戻すため、「バルーンは キャラ窓位置 ＋ 新しい offset に居る」という
/// 窓相対の恒等式は**表示位置の上では成立しない**（`resnap_placements` の doc が言うとおり、
/// この恒等式は追従計算の出力時点の事後条件であって表示位置の恒久不変量ではない）。
///
/// ガードは **Y を一切変更しない**（同関数の事後条件）ので、Y の恒等式は交差の有無に依らず
/// 常に主張できる。本判定は X の恒等式を主張してよい組を選り分けるためだけに使う。
fn follow_target_stays_visible(at: Point, size: SizePx, dpi: u16) -> bool {
    [s2_work_area_for_dpi(dpi), s2_neighbor_work_area()]
        .into_iter()
        .any(|wa| {
            at.x < wa.right && at.x + size.w > wa.left && at.y < wa.bottom && at.y + size.h > wa.top
        })
}

/// 全スコープのキャラ窓のアンカーを差し替える（spawn が焼き込む [`Anchored`] の上書き）。
///
/// **整地より前**に呼ぶこと——整地でキャラ窓をそのアンカーの固定点へ落ち着かせてから
/// 遷移を起こさないと、遷移フレームの窓書込に「初回のアンカー移動」が混ざる。
fn set_anchor(harness: &mut FrameHarness, anchor: Anchor) {
    for scope in harness.scopes().to_vec() {
        let char_window = harness.char_window(scope);
        harness
            .world
            .entity_mut(char_window)
            .insert(Anchored(anchor));
    }
}

/// 起動直後の整地——3 つの源と窓の拡大率を `dpi` へ揃え、拡大率の相の初回全窓マッチ
/// （永続 `SystemState` の仕様）を空回しして消費する。
///
/// この 1 巡で未係留の基準（`OffsetBase::unpinned`）が `dpi` へ**係留**される（要件 5.2）。
/// 係留の完了は自己検査する——済んでいないと次の遷移は係留の腕へ落ち、追随が起きないまま
/// 「値が動かない」だけの空虚な緑になる。
fn settle_at(harness: &mut FrameHarness, source: &mut FakeReports, dpi: u16) {
    harness.set_monitor_sources_for_dpi(dpi);
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            follow_of(harness, scope).base(),
            crate::placement::follow::OffsetBase {
                offset: BASE_OFFSETS[scope],
                dpi: Some(DPI::from_dpi(dpi, dpi)),
            },
            "scope={scope}: 整地で基準対が「fixture 値 × 係留済み {dpi}」になっていない（探針が退化している）"
        );
    }
}

/// [`LOW_DPI`] で整地する（単発の腕の共通前段）。
fn settle(harness: &mut FrameHarness, source: &mut FakeReports) {
    settle_at(harness, source, LOW_DPI);
}

/// 表示 DPI を `dpi` へ動かして 1 フレーム回す（モニタ表と窓の拡大率を同時に更新＝
/// 待ち札の付かない通常の遷移）。
fn transition_to(harness: &mut FrameHarness, source: &mut FakeReports, dpi: u16) {
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

// ---------------------------------------------------------------------------
// 1. 遷移 × アンカー × スコープの行列（要件 3.1／7.1・design Integration 1）
// ---------------------------------------------------------------------------

/// **完了条件の本体**: 表示 DPI 遷移 3 組 × 全アンカー 5 腕 × 全スコープ 2 つの
/// **30 組すべて**で、書込**前**に読んだオフセットが表示 DPI 比で追随する。
///
/// 到達数は数えて [`MATRIX_COMBINATIONS`] と突合する——「ループが途中で回っていなかった」
/// を件数で塞ぐ（行列は緑になったが 1 組も回っていない、が最も静かな失敗である）。
#[test]
fn the_transition_matrix_follows_the_display_dpi_ratio() {
    let mut reached = 0usize;
    let mut clamped = 0usize;

    for (t, (from, to)) in TRANSITIONS.into_iter().enumerate() {
        // 空振り防止の証人 1: 比が 1 でない。
        assert_ne!(
            from, to,
            "遷移 {t} の前後で表示 DPI が同じ（比 1 では追随の有無を区別できない）"
        );

        for anchor in ANCHORS {
            let mut harness = FrameHarness::new();
            let mut source = FakeReports::default();
            set_anchor(&mut harness, anchor);
            settle_at(&mut harness, &mut source, from);

            // 書込**前**に読む（design Integration 1 の「書込前に読んだ値と突合」）。
            let before: Vec<(usize, PointPx, Point, Point)> = harness
                .scopes()
                .to_vec()
                .into_iter()
                .map(|scope| {
                    (
                        scope,
                        offset_of(&harness, scope),
                        pos_of(&harness, harness.char_window(scope)),
                        pos_of(&harness, harness.balloon_window(scope)),
                    )
                })
                .collect();

            transition_to(&mut harness, &mut source, to);
            let writes = harness.drain_writes();

            for (scope, old_offset, _old_char, old_balloon) in before {
                let scope_u32 = u32::try_from(scope).expect("scope は u32 域");
                let expected = EXPECTED_OFFSETS[t][scope];

                // 空振り防止の証人 2: オフセットが非ゼロ（0 は何倍しても 0）。
                assert_eq!(
                    old_offset, BASE_OFFSETS[scope],
                    "{anchor:?} {from}→{to} scope={scope}: 遷移前のオフセットが fixture 値でない（前段が既に動かしている）"
                );
                assert!(
                    old_offset.x != 0 && old_offset.y != 0,
                    "{anchor:?} {from}→{to} scope={scope}: 遷移前のオフセットに 0 の軸がある（追随の有無を区別できない）"
                );
                // 探針の非退化: 期待値が旧値と一致するなら、追随が無くても緑になる。
                assert_ne!(
                    expected, old_offset,
                    "{anchor:?} {from}→{to} scope={scope}: 期待値が旧値と同じ（この組は追随を観測できない）"
                );

                // 主張そのもの（要件 3.1）——手計算の逐語値と bit 一致。
                assert_eq!(
                    offset_of(&harness, scope),
                    expected,
                    "{anchor:?} {from}→{to} scope={scope}: 追随後のオフセットが表示 DPI 比の手計算値でない"
                );

                // 基準対は追随で動かない（出力を入力へ戻さない＝往復無誤差の前提・要件 3.3）。
                let base = follow_of(&harness, scope).base();
                assert_eq!(
                    (base.offset, base.dpi),
                    (BASE_OFFSETS[scope], Some(DPI::from_dpi(from, from))),
                    "{anchor:?} {from}→{to} scope={scope}: 追随が基準対を書き換えた（次の遷移が二重に拡大する）"
                );

                // 空振り防止の証人 3: バルーンが実際に動いた。窓書込（指令）と ECS ミラーの
                // 双方で確かめる——片方だけだと「指令は出たが位置は据置き」「位置は動いたが
                // 単一ライターを通っていない」のどちらかを取りこぼす。
                let balloon_writes = writes_for(&writes, scope_u32, "balloon");
                assert!(
                    !balloon_writes.is_empty(),
                    "{anchor:?} {from}→{to} scope={scope}: バルーンへの窓書込が 1 件も無い（追随が窓へ届いていない）: {writes:?}"
                );
                let new_balloon = pos_of(&harness, harness.balloon_window(scope));
                assert_ne!(
                    new_balloon, old_balloon,
                    "{anchor:?} {from}→{to} scope={scope}: バルーン窓の位置が 1px も動いていない"
                );
                // 落ち着き先は「確定済みキャラ窓位置 ＋ **新しい** offset」（窓相対の恒等式）。
                let char_pos = pos_of(&harness, harness.char_window(scope));
                let target = Point {
                    x: char_pos.x + expected.x,
                    y: char_pos.y + expected.y,
                };
                // Y は可視性ガードが触らない（同関数の事後条件）ので、交差の有無に依らず主張する。
                assert_eq!(
                    new_balloon.y, target.y,
                    "{anchor:?} {from}→{to} scope={scope}: バルーンの Y が新しい offset の位置に無い（旧 offset の中間位置か）"
                );
                let balloon_size = size_of(&harness, harness.balloon_window(scope));
                if follow_target_stays_visible(target, balloon_size, to) {
                    assert_eq!(
                        new_balloon.x, target.x,
                        "{anchor:?} {from}→{to} scope={scope}: バルーンの X が新しい offset の位置に無い"
                    );
                } else {
                    // 追随の行き先がどの作業領域とも交差しない組。可視性ガードが X を
                    // 引き戻すのが正しい（本仕様の対象外・別の檻が所有）。
                    clamped += 1;
                    let wa = s2_work_area_for_dpi(to);
                    assert!(
                        new_balloon.x >= wa.left && new_balloon.x <= wa.right - balloon_size.w,
                        "{anchor:?} {from}→{to} scope={scope}: 完全不可視の行き先なのに可視性ガードが X を引き戻していない: {new_balloon:?}"
                    );
                }

                reached += 1;
            }
        }
    }

    assert_eq!(
        reached, MATRIX_COMBINATIONS,
        "行列が全組へ到達していない（到達 {reached} 組・期待 {MATRIX_COMBINATIONS} 組）"
    );
    // X の恒等式を免除した組の件数を固定する——免除が黙って広がると、行列は緑のまま
    // 「バルーンがどこに居ても通る」檻へ退化する。免除は `Anchor::Left` × scope 0 の
    // 3 遷移ぶんだけ（キャラ窓が作業領域左端 31 に固定され、offset −412 系のバルーンが
    // 主・隣接いずれの作業領域とも交差しなくなる組）である。
    assert_eq!(
        clamped, MATRIX_CLAMPED_COMBINATIONS,
        "可視性ガードで X を免除した組の件数が変わった（免除 {clamped} 組・期待 {MATRIX_CLAMPED_COMBINATIONS} 組）"
    );
}

// ---------------------------------------------------------------------------
// 2. キーワード素材の消費**後**の腕と揃えの残差（要件 4.2／7.3・design D8）
// ---------------------------------------------------------------------------
//
// 素材が残る腕・受容残余の腕・自己回復は `frame_balloon_offset_keyword_gate_tests.rs`
// （task 6.3）が持つ。ここが持つのは残る 1 腕——**素材が消費された後の遷移**では見送りが
// 解けて追随だけが効き、その結果の中央揃えが D8 の許容量（1 軸 ≤3px）以内に収まること。

/// キーワード素材の揃え種別（中央上・兄弟ファイルと同一）。
const KEYWORD_MODE: BalloonXMode = BalloonXMode::CenterTop;

/// キーワード素材の調整量（作者指定 0＝揃えの式だけを見る）。
const KEYWORD_ADJUST: PointPx = PointPx { x: 0, y: 0 };

/// D8 が定める揃えの残差の許容量（1 軸あたり・物理 px）。
const ALIGNMENT_ALLOWANCE_PX: i32 = 3;

/// 素材を消費させるための**拡大率不変**の寸法変化（scope 昇順・面の切替と同じ経路）。
///
/// 元寸は scope 0 が 434×687・scope 1 が 278×357。
const CONSUME_CHAR_SIZES: [(u32, u32); 2] = [(440, 700), (280, 360)];

/// 消費後の遷移（96→192）で報告するキャラ窓の実表示寸（[`CONSUME_CHAR_SIZES`] の 2 倍）。
const HIGH_CHAR_SIZES: [(u32, u32); 2] = [(880, 1400), (560, 720)];

/// 同じ遷移で報告するバルーン窓の実表示寸（fixture の 223×158 の 2 倍）。
const HIGH_BALLOON_SIZE: (u32, u32) = (446, 316);

/// 全スコープのキャラ窓へキーワード素材を付ける（本番の spawn と同じ Component）。
fn attach_material(harness: &mut FrameHarness) {
    for scope in harness.scopes().to_vec() {
        let char_window = harness.char_window(scope);
        harness
            .world
            .entity_mut(char_window)
            .insert(BalloonKeywordBase {
                mode: KEYWORD_MODE,
                adjust: KEYWORD_ADJUST,
            });
    }
}

/// 現在の実寸に対する「理想の中央揃え」オフセット（P5 と同じ式を同じ口から呼ぶ）。
fn ideal_alignment(harness: &FrameHarness, scope: usize) -> PointPx {
    keyword_balloon_pos(
        KEYWORD_MODE,
        PointPx { x: 0, y: 0 },
        size_of(harness, harness.char_window(scope)),
        size_of(harness, harness.balloon_window(scope)),
        KEYWORD_ADJUST,
    )
    .expect("中央揃えのモードは基本位置を持つ")
}

/// **素材消費後の腕**（要件 4.2／7.3・design D8）: 素材が消えた後の遷移では追随だけが効き、
/// 中央揃えの残差が許容量以内に収まる。
///
/// 段取りは 3 つ。⑴ [`LOW_DPI`] で整地して素材を付ける。⑵ **拡大率を変えずに**キャラ窓の
/// 実表示寸だけを動かし（報告寸の突合＝面の切替と同じ経路）、再導出に素材を消費させる
/// ——この段は表示 DPI が動かないので追随は 1 度も走らず、基準対は「再導出値 × [`LOW_DPI`]」
/// に焼き直される。⑶ 96→192 の遷移を、キャラ窓とバルーン窓の**両方**が倍寸を報告する形で
/// 起こす。素材はもう無いので見送りは解け、追随が基準から引き直す。
///
/// 残差の出所は D8 の 3 つ（両寸の個別丸め・中点の整数除算・追随の丸め）だけであり、
/// 上限を主張するだけでなく**実値も逐語で固定**する（上限内での悪化を見逃さないため）。
#[test]
fn the_transition_after_the_material_is_consumed_keeps_the_centering_within_the_allowance() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    attach_material(&mut harness);

    // ⑵ 拡大率不変の寸法変化で素材を消費させる。
    for scope in harness.scopes().to_vec() {
        let scope32 = u32::try_from(scope).expect("scope は u32 域");
        source
            .pending
            .insert(shell_target(scope32).0, CONSUME_CHAR_SIZES[scope]);
    }
    harness.run_reconcile(&mut source);
    let _consumed = harness.drain_writes();

    for scope in harness.scopes().to_vec() {
        assert!(
            harness
                .world
                .get::<BalloonKeywordBase>(harness.char_window(scope))
                .is_none(),
            "scope={scope}: 寸法変化で素材が消費されていない（探針の退化——以降は見送りの腕を見てしまう）"
        );
        // 再導出は絶対値で書くので、この時点の offset は「その寸に対する理想の中央揃え」。
        // 基準 DPI は再導出時点の表示 DPI（[`LOW_DPI`]）へ係留されている。
        let base = follow_of(&harness, scope).base();
        assert_eq!(
            (base.offset, base.dpi),
            (
                ideal_alignment(&harness, scope),
                Some(DPI::from_dpi(LOW_DPI, LOW_DPI))
            ),
            "scope={scope}: 再導出が基準対を「理想の中央揃え × 現在の表示 DPI」へ焼き直していない"
        );
    }
    let consumed_offsets: Vec<PointPx> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| offset_of(&harness, scope))
        .collect();
    // 手計算（`CenterTop`・調整 0・バルーン 223×158）:
    //   scope 0: x=(440−223)/2=108・y=−158 ／ scope 1: x=(280−223)/2=28・y=−158
    assert_eq!(
        consumed_offsets,
        vec![PointPx { x: 108, y: -158 }, PointPx { x: 28, y: -158 }],
        "再導出値が中央揃え式の手計算と一致しない（以降の期待値の土台が崩れている）"
    );

    // ⑶ 消費後の遷移（96→192・両窓が倍寸を報告する）。
    for scope in harness.scopes().to_vec() {
        let scope32 = u32::try_from(scope).expect("scope は u32 域");
        source
            .refresh
            .insert(shell_target(scope32).0, HIGH_CHAR_SIZES[scope]);
        source
            .refresh
            .insert(balloon_target(scope32).0, HIGH_BALLOON_SIZE);
    }
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    harness.advance_frame();
    let logs = capture_logs(|| {
        harness.run_placement_phases(&mut source);
    });

    // 見送りは解けている——判定語が `rescaled` であって `keyword-pending` ではない。
    let lines = offset_lines(&logs);
    assert_eq!(
        lines.len(),
        harness.scopes().len(),
        "追随の観測行がスコープ数と一致しない: {logs:?}"
    );
    for line in &lines {
        assert!(
            line.contains(&format!("verdict={OFFSET_VERDICT_RESCALED}")),
            "素材が消費済みなのに追随が効いていない（判定語が rescaled でない）: {line}"
        );
        assert!(
            !line.contains(&format!("verdict={OFFSET_VERDICT_KEYWORD_PENDING}")),
            "素材が無いのに見送りの判定語が出ている: {line}"
        );
    }

    for scope in harness.scopes().to_vec() {
        // 報告寸が実際に着地している（残差の比較対象が本物であることの前提）。
        assert_eq!(
            (
                size_of(&harness, harness.char_window(scope)),
                size_of(&harness, harness.balloon_window(scope))
            ),
            (
                SizePx {
                    w: HIGH_CHAR_SIZES[scope].0 as i32,
                    h: HIGH_CHAR_SIZES[scope].1 as i32,
                },
                SizePx {
                    w: HIGH_BALLOON_SIZE.0 as i32,
                    h: HIGH_BALLOON_SIZE.1 as i32,
                }
            ),
            "scope={scope}: 遷移後の実表示寸が報告どおりでない（残差の比較対象が崩れる）"
        );

        // 追随の結果（手計算・比 2 ゆえ丸めなし）: scope 0 (108,−158)×2=(216,−316)、
        // scope 1 (28,−158)×2=(56,−316)。
        let expected = [PointPx { x: 216, y: -316 }, PointPx { x: 56, y: -316 }][scope];
        assert_eq!(
            offset_of(&harness, scope),
            expected,
            "scope={scope}: 消費後の遷移で追随が表示 DPI 比の手計算値を出していない"
        );

        // 理想の中央揃え（手計算）: scope 0 x=(880−446)/2=217・scope 1 x=(560−446)/2=57、
        // y はいずれも −316。残差は x で 1px・y で 0px＝許容量 3px 以内。
        let ideal = [PointPx { x: 217, y: -316 }, PointPx { x: 57, y: -316 }][scope];
        assert_eq!(
            ideal_alignment(&harness, scope),
            ideal,
            "scope={scope}: 理想の中央揃えが手計算と一致しない（許容量の判定基準が崩れる）"
        );
        let residual = ((expected.x - ideal.x).abs(), (expected.y - ideal.y).abs());
        assert_eq!(
            residual,
            (1, 0),
            "scope={scope}: 揃えの残差の実値が変わった（上限内でも悪化を見逃さない）"
        );
        assert!(
            residual.0 <= ALIGNMENT_ALLOWANCE_PX && residual.1 <= ALIGNMENT_ALLOWANCE_PX,
            "scope={scope}: 揃えの残差が D8 の許容量 {ALIGNMENT_ALLOWANCE_PX}px を超えた: {residual:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. 待ち札との共存（design Integration 4・先行仕様 atom 設計 C5）
// ---------------------------------------------------------------------------

/// **見送られた窓は追随を失わない**: 拡大率通知が表更新より先に届いて整合待ちの札が付いた
/// フレームでは追随が 1 bit も動かず、札が解除されたフレームで**基準からの全比**で追い付く。
///
/// 待ちのあいだに部分的に追随してしまうと、解除フレームで残りの比を掛けることになり
/// （出力を入力へ戻す形）、往復で誤差が積む。ここが固定しているのは「見送りは分割ではなく
/// 全部の先送りである」という性質そのものである。
#[test]
fn a_window_held_for_sync_regains_the_follow_after_the_hold_is_released() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let before: Vec<(usize, PointPx)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| (scope, offset_of(&harness, scope)))
        .collect();

    // 待ちフレーム: 窓の拡大率だけが新しく、モニタ表はまだ旧水準のまま。
    harness.set_window_dpi(HIGH_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    // 零件を主張する**前に**、駆動が生きていること（札が実際に付いた）を固定する。
    for scope in harness.scopes().to_vec() {
        assert!(
            harness
                .world
                .get::<DpiSyncHold>(harness.char_window(scope))
                .is_some(),
            "scope={scope}: 拡大率と表が食い違うのに待ち札が付いていない（ゲートが走っていない）"
        );
    }
    for (scope, old_offset) in &before {
        assert_eq!(
            offset_of(&harness, *scope),
            *old_offset,
            "scope={scope}: 見送り中の窓でオフセットが動いた（待ち札を追い越している）"
        );
    }
    let _waiting = harness.drain_writes();

    // 解除フレーム: 表が追いつく。`Changed<DPI>` はもう立たないが、札を持つ窓は対象集合へ
    // 和集合で入る（設計 C5）——ここで追随が取り戻される。
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    for (scope, old_offset) in before {
        assert!(
            harness
                .world
                .get::<DpiSyncHold>(harness.char_window(scope))
                .is_none(),
            "scope={scope}: 表が追いついたのに待ち札が外れていない"
        );
        // 96→192 の全比（[`EXPECTED_OFFSETS`] の 2 行目と同じ手計算値）。
        let expected = EXPECTED_OFFSETS[1][scope];
        assert_ne!(
            expected, old_offset,
            "scope={scope}: 期待値が旧値と同じ（この腕は追随を観測できない）"
        );
        assert_eq!(
            offset_of(&harness, scope),
            expected,
            "scope={scope}: 札の解除後に追随が取り戻されていない（見送りで失われた／部分的にしか掛かっていない）"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. 拡大率が変わらない寸法変化では 1 bit も動かさない（要件 3.2／9.8）
// ---------------------------------------------------------------------------

/// **面の切替**（要件 9.8）: 拡大率を変えずにキャラ窓の実表示寸だけが変わっても、
/// 追従オフセットは 1 bit も動かない。
///
/// 発火条件が `Changed<DPI>` に閉じているので構造的に成立するはずの性質だが、寸法差を
/// 発火条件へ足す実装（あり得る誤り）はここで赤になる。零件の主張ゆえ、駆動が生きている
/// こと（寸が実際に変わり、窓書込が実際に出たこと）を先に固定する。
#[test]
fn a_surface_swap_at_a_constant_scale_does_not_move_the_offset() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let before: Vec<(usize, BalloonFollow, SizePx)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| {
            (
                scope,
                follow_of(&harness, scope),
                size_of(&harness, harness.char_window(scope)),
            )
        })
        .collect();

    for scope in harness.scopes().to_vec() {
        let scope32 = u32::try_from(scope).expect("scope は u32 域");
        source
            .pending
            .insert(shell_target(scope32).0, CONSUME_CHAR_SIZES[scope]);
    }
    harness.run_reconcile(&mut source);
    let writes = harness.drain_writes();

    for (scope, old, old_size) in before {
        let scope_u32 = u32::try_from(scope).expect("scope は u32 域");
        // 駆動が生きている: 寸が本当に変わり、キャラ窓が実際に書かれた。
        let new_size = size_of(&harness, harness.char_window(scope));
        assert_ne!(
            new_size, old_size,
            "scope={scope}: 面の切替で寸が変わっていない（据置きの主張が空虚になる）"
        );
        assert_eq!(
            writes_for(&writes, scope_u32, "char").len(),
            1,
            "scope={scope}: 面の切替でキャラ窓が書かれていない（駆動が死んでいる）: {writes:?}"
        );
        // 主張そのもの——現在値も基準対も bit 同一。
        assert_eq!(
            follow_of(&harness, scope),
            old,
            "scope={scope}: 拡大率が変わらない寸法変化でオフセット（または基準対）が動いた（要件 3.2／9.8）"
        );
    }
}

/// **作業領域の再スナップ**（要件 3.2／9.7）: 拡大率を変えずに作業領域だけが動いても、
/// 追従オフセットは 1 bit も動かない。
///
/// 「随伴バルーンの追従オフセットは変わらない」という下流の期待のうち、**本仕様が上書き
/// しない側**がこれである（拡大率遷移の側だけが要件 3.1 で反転した・要件 9.7）。
#[test]
fn a_work_area_resnap_at_a_constant_scale_does_not_move_the_offset() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let before: Vec<(usize, BalloonFollow, (i32, i32))> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| {
            (
                scope,
                follow_of(&harness, scope),
                harness.ground_point(scope),
            )
        })
        .collect();

    // タスクバーを隠した形＝**拡大率は一切変えずに**作業領域だけが動く構成変更。
    let hidden = s2_taskbar_hidden_work_area(LOW_DPI);
    assert_ne!(
        hidden.bottom,
        s2_work_area_for_dpi(LOW_DPI).bottom,
        "探針が退化している: 作業領域下端が動かない（再スナップを観測できない）"
    );
    harness.set_monitor_table(s2_monitors_with_work_area(LOW_DPI, hidden));
    harness.advance_frame();
    let change = harness.run_placement_phases(&mut source);
    assert!(
        change.is_some(),
        "作業領域源が差し替わっていない（再スナップの段が走らない＝据置きの主張が空虚になる）"
    );

    for (scope, old, old_ground) in before {
        // 駆動が生きている: 再スナップでキャラ窓の接地点が新しい下端へ移った。
        assert_eq!(
            harness.ground_point(scope).1,
            hidden.bottom,
            "scope={scope}: 再スナップで接地点が新しい作業領域下端へ移っていない（駆動が死んでいる）"
        );
        assert_ne!(
            harness.ground_point(scope),
            old_ground,
            "scope={scope}: 接地点が 1px も動いていない"
        );
        // 主張そのもの。
        assert_eq!(
            follow_of(&harness, scope),
            old,
            "scope={scope}: 作業領域の再スナップでオフセット（または基準対）が動いた（要件 3.2／9.7）"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. ドラッグ由来にも同一規則（要件 3.5・design Integration 6）
// ---------------------------------------------------------------------------

/// バルーン単独ドラッグで決める相対位置（キャラ窓左上相対・物理 px）。
///
/// 両軸とも奇数かつ非ゼロにしてある——比 5/4 で丸めが実際に働く値でないと、
/// 「作者指定と同じ丸め規則が効いている」の主張が丸め無しの腕で空振りする。
const DRAG_OFFSET: PointPx = PointPx { x: -333, y: -77 };

/// バルーン窓をキャラ窓相対 `offset` の位置へ置き、単独ドラッグのハンドラを本番と同じ
/// 引数で呼ぶ（`wndproc` が実窓位置を書いた後の状態を模す）。
fn drag_balloon_to(harness: &mut FrameHarness, scope: usize, offset: PointPx) {
    let char_pos = pos_of(harness, harness.char_window(scope));
    let balloon = harness.balloon_window(scope);
    let mut window_pos = harness
        .world
        .get_mut::<WindowPos>(balloon)
        .expect("バルーン窓に WindowPos がある");
    window_pos.position = Some(Point {
        x: char_pos.x + offset.x,
        y: char_pos.y + offset.y,
    });
    let ev = Phase::Bubble(DragEvent {
        target: balloon,
        start_position: Point::new(0, 0),
        position: Point::new(0, 0),
        is_primary: true,
        timestamp: std::time::Instant::now(),
    });
    assert!(
        !on_balloon_drag(&mut harness.world, balloon, balloon, &ev),
        "scope={scope}: バルーンドラッグのハンドラがイベントを消費した（伝播続行が規約）"
    );
}

/// **確立の事後条件**（要件 3.5・design D14）: バルーン単独ドラッグは相対位置を新しい
/// **基準**として焼き直し、基準 DPI は**その時点の表示 DPI**になる。
///
/// 基準 DPI を刻まない（未係留のまま残す）実装だと次の遷移が係留の腕へ落ち、値が動かない
/// ——本テストが基準対を逐語で見るのはそのためである。
#[test]
fn a_balloon_only_drag_establishes_the_base_at_the_current_display_dpi() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    for scope in harness.scopes().to_vec() {
        assert_ne!(
            offset_of(&harness, scope),
            DRAG_OFFSET,
            "scope={scope}: ドラッグ先が偶然いまの offset と一致している（探針の退化）"
        );
        drag_balloon_to(&mut harness, scope, DRAG_OFFSET);

        assert_eq!(
            follow_of(&harness, scope).base(),
            crate::placement::follow::OffsetBase {
                offset: DRAG_OFFSET,
                dpi: Some(DPI::from_dpi(LOW_DPI, LOW_DPI)),
            },
            "scope={scope}: ドラッグの確立が「決めた相対位置 × その時点の表示 DPI」を基準にしていない"
        );
        assert_eq!(
            offset_of(&harness, scope),
            DRAG_OFFSET,
            "scope={scope}: ドラッグで決めた相対位置が現在値になっていない"
        );
    }
}

/// **同一の追随規則**（要件 3.5）: ドラッグで決めた相対位置にも、作者指定由来と同じ
/// 表示 DPI 比・同じ丸めが適用される。
///
/// 期待値は手計算（比 5/4・`(2·len·5 + 4) ÷ 8`）:
/// x は `(2·333·5+4)/8 = 3334/8 = 416.75 → 416`、y は `(2·77·5+4)/8 = 774/8 = 96.75 → 96`。
/// 符号は基準から引き継ぐので `(−416, −96)` である。作者指定由来の値
/// （[`EXPECTED_OFFSETS`] の 1 行目）と**同じ算術**で出ていることが要件 3.5 の主張である。
#[test]
fn a_drag_established_offset_follows_by_the_same_rule_as_an_author_specified_one() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    for scope in harness.scopes().to_vec() {
        drag_balloon_to(&mut harness, scope, DRAG_OFFSET);
    }

    transition_to(&mut harness, &mut source, 120);
    let writes = harness.drain_writes();

    let expected = PointPx { x: -416, y: -96 };
    assert_ne!(
        expected, DRAG_OFFSET,
        "期待値がドラッグ値と同じ（この腕は追随を観測できない）"
    );
    for scope in harness.scopes().to_vec() {
        let scope_u32 = u32::try_from(scope).expect("scope は u32 域");
        assert_eq!(
            offset_of(&harness, scope),
            expected,
            "scope={scope}: ドラッグ由来の相対位置に作者指定と同一の追随規則が適用されていない"
        );
        // 基準対はドラッグで焼き直されたまま動かない（追随は基準を書かない）。
        assert_eq!(
            follow_of(&harness, scope).base(),
            crate::placement::follow::OffsetBase {
                offset: DRAG_OFFSET,
                dpi: Some(DPI::from_dpi(LOW_DPI, LOW_DPI)),
            },
            "scope={scope}: 追随がドラッグ由来の基準対を書き換えた"
        );
        // バルーンが実際に新しい相対位置へ動いた。
        assert!(
            !writes_for(&writes, scope_u32, "balloon").is_empty(),
            "scope={scope}: バルーンへの窓書込が 1 件も無い: {writes:?}"
        );
        let char_pos = pos_of(&harness, harness.char_window(scope));
        assert_eq!(
            pos_of(&harness, harness.balloon_window(scope)),
            Point {
                x: char_pos.x + expected.x,
                y: char_pos.y + expected.y,
            },
            "scope={scope}: バルーンが追随後の相対位置に居ない"
        );
    }
}
/// [`ANCHORS`] が `Anchor` の腕を 1 つも取りこぼしていないことを**構造で**保つ。
///
/// 配列は腕が増えても黙って通るため、行列の「全アンカー」が縮んでも誰も気づかない。
/// `match` を非網羅にすればコンパイルが落ちるので、腕を足した者が必ずここへ来る。
#[test]
fn anchor_enumeration_is_exhaustive() {
    for anchor in ANCHORS {
        // 腕を足すとこの `match` が非網羅になってコンパイルエラーになる。
        // そのときは `ANCHORS` にも新しい腕を足すこと。
        match anchor {
            Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Free => {}
        }
    }
    assert_eq!(
        ANCHORS.len(),
        5,
        "ANCHORS の腕数が変わった——行列の期待値と MATRIX_COMBINATIONS を見直すこと"
    );
}
