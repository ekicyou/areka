//! 永続値 ⇄ 配置の変換を所有する純関数群（areka-P0-position-persist・design C1）。
//!
//! 統一プロパティシステム（sylphya）が確定した永続 key 族（窓位置・バルーン相対
//! オフセット・起動記録・vanish 回数）を**消費**し、IO・World 変異と分離した
//! 決定論的な純関数として「値を差す／値を書く」変換を提供する。永続ストア実体
//! （形式・原子性・寛容読取・スコープ分離）は sylphya の領分であり、本モジュールは
//! その契約（[`areka_sylphya::PersistKey`]／[`areka_sylphya::Axis`]）を消費するのみ。
//!
//! 記憶への送り口は書く [`persist_entries`] と消す [`forget_entries`] の 2 つで、書く呼び手は次の 3 か所:
//! - キャラ窓のドラッグの確定（`follow::drag_follow` の `on_char_drag_end`・窓の位置と、組の相手の
//!   窓の位置を 1 つの束で 1 回。相手の束は `follow::sticky_follow` の `finish_char_drag` が返す）
//! - バルーンのドラッグの確定（`follow::balloon_drag` の `on_balloon_drag_end`・バルーンのずらし）
//! - 起動の最後に並べ終えた時点と、既定の位置への並べ直し（`reset_pos` の `reset_window_positions`）
//!   の後（どちらも [`persist_unremembered_char_positions`]・記憶に位置が無いキャラの位置と、
//!   印の無い古い記憶の隣り合いの印）
//!
//! 記憶の位置を寄せて表示した起動（[`RestoredPositionsClamped`]）では、送り口（[`persist_entries`] と、
//! 並べ終えた時点の保存が呼ぶその本体）が 3 か所のどの束からも、印の無い古い記憶へ隣り合いの印を
//! 書き足さない（window-work-area-clamp 4.6）。
//!
//! 消す呼び手はバルーン位置の初期化（`reset_balloon_pos` の `reset_balloon_positions`・ずらしの鍵）
//! だけ。
//!
//! 戻す側（[`apply_restored_placements`]）は純関数のままで、送り口に届かない。
//! 並べ終えた時点の時機は、完了 spec `areka-P0-position-persist` の要件 1.9（ドラッグの確定でだけ
//! 書く）を `areka-P0-char-position-save-on-exit` で改めたもの。一度もドラッグしていない
//! キャラが起動のたびに並べ直されないようにするため（記録は `doc/COMPAT_ARCHITECTURE.md` 8 章）。
//! 決定論的な変換——寛容 parse（[`parse_px`]）と保存 entries 構築
//! （[`char_pos_entries`]／[`balloon_offset_entries`]）——のほか、復元 merge・
//! 再射影も本モジュールにある。送り口と保存の側（`PersistWiring`・[`persist_entries`] ほか）は子の
//! モジュール `save`（`persist_save.rs`）にあり、ここから公開する。
//!
//! バルーン相対オフセットの基準変換（アンカー辺基準）は 2026-07-31 の実機裁定で撤去した
//! ——保存基準はランタイム（[`super::follow::BalloonFollow`]`.offset`）と同じ **char 窓左上**
//! であり、変換は不要（[`balloon_offset_entries`] 参照）。キャラ窓位置の原点符号化
//! （[`char_pos_to_origin_x`]＝下端中央）は別の関心事ゆえ従来どおり維持する。

use std::collections::BTreeMap;
use std::path::Path;

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{read_ghost_names, shiori_dir};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{Axis, PersistKey, PersistScope, ScopeRoots, load_scope};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::warn;

use super::chain_finalize::{AdjacencyMark, ChainRect};
use super::chain_rule::{ChainRule, ChainTerms};
use super::follow::{
    AnchorSource, Anchored, MonitorSnapshot, OffsetBase, RestoredWindow, keep_restored_groups,
    project_anchor, rect_at, work_area_for_window,
};
use super::resolver::{Anchor, PointPx, ScopePlacement, SizePx, clamp_axis};

/// 永続値の寛容 parse（design C1・6.1）。
///
/// 数値文字列 → `Some(i32)`。非数値・空文字列 → `None`（＝「値なし」・復元の
/// `merge_scope` が鍵ごとに warn を出して既定へ戻る）。前後空白は許容する（寛容）。
/// 決定論（同一入力→同一出力）。
fn parse_px(value: &str) -> Option<i32> {
    // 寛容縮退（6.1）: 非数値・空・小数は「値なし」＝None（panic なし）。
    value.trim().parse::<i32>().ok()
}

/// 窓位置（左上基準）を**原点＝下端中央の x** へ移す（保存方向・Bottom のみ）。
///
/// 伺かの立ち絵は足元中央が接地点＝原点。左上 x をそのまま保存すると、サーフェス寸が
/// 変わったときに「同じ左上」が別の中央を指してしまい、復元でキャラとバルーンが横へずれる
/// （実機: むらさき surface1000 382 で保存 → surface0 434 で復元し原点が 26px ずれ、
/// バルーンが 104px ずれた）。x のみ中央基準へ移し、y は下端が work area 由来で毎起動
/// 再導出されるため左上のまま（復元側 `project_anchor` が下端を再固定する）。
/// Bottom 以外は原点が中央ではないため恒等（従来どおり左上基準）。
pub fn char_pos_to_origin_x(anchor: Anchor, pos: PointPx, char_size: SizePx) -> PointPx {
    match anchor {
        Anchor::Bottom => PointPx {
            x: pos.x.saturating_add(char_size.w / 2),
            y: pos.y,
        },
        _ => pos,
    }
}

/// [`char_pos_to_origin_x`] の逆（復元方向）: 原点 x から現寸の左上 x を導出する。
pub fn char_pos_from_origin_x(anchor: Anchor, pos: PointPx, char_size: SizePx) -> PointPx {
    match anchor {
        Anchor::Bottom => PointPx {
            x: pos.x.saturating_sub(char_size.w / 2),
            y: pos.y,
        },
        _ => pos,
    }
}

/// キャラ窓の位置を記憶へ書くときの形を決める向き（areka-P0-window-link-directives 要件 4.8・
/// 設計の決め 6）。
///
/// 次の起動はこの窓を descript の向きで読み戻すので、今の向き（台本で変えたかもしれない）では
/// なく [`AnchorSource`]`.descript` で書く。射影（ドラッグの縛り）には今の向きを使い、形にだけ
/// これを使う。`AnchorSource` が無い窓（テストの組み立てなど）は [`Anchored`] を使い、どちらも
/// 無ければ `None`。
pub fn saved_form_anchor(world: &World, window: Entity) -> Option<Anchor> {
    match world.get::<AnchorSource>(window) {
        Some(source) => Some(source.descript),
        None => world.get::<Anchored>(window).map(|a| a.0),
    }
}

/// キャラ窓位置の保存 entries を構築する（design C1・純関数・1.6）。
///
/// スコープ別の [`PersistKey::WindowPos`]（X/Y）へ `pos` を i32 の `Display` で
/// 文字列化して載せる。値ドメインは物理 px・仮想スクリーン絶対 i32（負値可）。
pub fn char_pos_entries(scope: u32, pos: PointPx) -> Vec<(PersistKey, String)> {
    vec![
        (
            PersistKey::WindowPos {
                scope,
                axis: Axis::X,
            },
            pos.x.to_string(),
        ),
        (
            PersistKey::WindowPos {
                scope,
                axis: Axis::Y,
            },
            pos.y.to_string(),
        ),
    ]
}

/// バルーン相対オフセットの保存 entries を構築する（design C1・純関数・2.5）。
///
/// スコープ別の [`PersistKey::BalloonOffset`]（X/Y）へ**キャラ窓左上基準**の相対
/// オフセットを i32 の `Display` で文字列化して載せる。
///
/// # 基準は char 窓左上（アンカー辺基準変換をしない・2026-07-31 実機裁定）
///
/// 保存表現はセッション内の [`super::follow::BalloonFollow`]`.offset` と**同一の基準**
/// （char 窓左上相対・物理 px）である。以前は Bottom を下端中央・Right を右端とする
/// 「アンカー辺基準」へ移してから保存していたが、ランタイム側の追従が全アンカーで
/// 窓相対（`balloon_pos − char_pos ≡ offset` 不変）へ統一されたため、保存だけ別基準に
/// すると保存時と復元時でサーフェス寸が違うときに Δ ぶんの恒久ドリフトが出る
/// （実機のむらさきで Δh=175px）。左上基準で保存すれば、保存時と同じサーフェス寸が
/// 表示された瞬間に厳密復元され、異なる寸でも窓相対のまま——ランタイムと一致する。
pub fn balloon_offset_entries(scope: u32, offset_persist: PointPx) -> Vec<(PersistKey, String)> {
    vec![
        (
            PersistKey::BalloonOffset {
                scope,
                axis: Axis::X,
            },
            offset_persist.x.to_string(),
        ),
        (
            PersistKey::BalloonOffset {
                scope,
                axis: Axis::Y,
            },
            offset_persist.y.to_string(),
        ),
    ]
}

/// 復元専用射影＝アンカー辺再導出（[`project_anchor`]・5.2）＋補軸 clamp（5.1）
/// （design C1・純関数・決定論・永続不書込）。
///
/// 保存位置 `pos` を**毎起動**現在の work area 幾何へ再射影する（アンカーは保存せず
/// 毎起動再解決・Req1.8）。復元値が現 work area の外（別 DPI・別モニタ構成からの
/// 復元）でも、アンカー辺を対応辺へ再固定し補助軸を域内へ縮退することで、必ず可視
/// 域内へ収める（Req5.1）。域内かつアンカー辺一致の入力に対してはべき等＝恒等
/// （Req5.3）。
///
/// # 射影規則
///
/// `wa` は「`pos` に置いた窓矩形の中心が属するモニタの work area」
/// （[`work_area_for_window`]・最近傍規則込み——どのモニタにも属さない復元位置は
/// 最近傍 wa を採る＝モニタ喪失シナリオ）。既存の吸着規則は [`project_anchor`] へ
/// 委譲し二重定義しない（Req5.2・Req1.6/1.8）。補軸の clamp は、再固定した窓が wa と少しも
/// 交わらない（まったく見えない）ときだけ掛ける（一部でも見えれば位置のまま・Req5.3）:
/// - `Bottom`/`Top`: アンカー辺（y）を再固定後、`x` を `[wa.left, wa.right − w]` へ clamp。
/// - `Left`/`Right`: アンカー辺（x）を再固定後、`y` を `[wa.top, wa.bottom − h]` へ clamp。
/// - `Free`: identity 射影（位置保持）＋両軸を wa 内へ clamp（可視性保証のみ・Req2.5）。
///
/// # graceful degradation
///
/// `snapshot` が空で [`work_area_for_window`] が `None` を返す場合は `pos` をそのまま
/// 返す（identity＝架空の既定矩形を発明しない・既存縮退流儀・panic しない・Req5.1）。
/// この場合、再射影結果を永続へ書き戻さないのは merge 側（`apply_restored_placements`）
/// が純関数で書込 API を持たない構造遮断が担う（Req5.4）。
///
/// 本番の呼び手（[`merge_scope`]）は吸着の当て直しだけの位置も要るので [`project_restore_parts`] を
/// 直に呼ぶ。本関数はその結果だけを返す檻向けの口。
#[cfg(any(test, feature = "test-support"))]
pub fn project_restore(
    anchor: Anchor,
    pos: PointPx,
    size: SizePx,
    snapshot: &MonitorSnapshot,
) -> PointPx {
    project_restore_parts(anchor, pos, size, snapshot).1
}

/// `project_restore` の本体。吸着の当て直しだけをした左上（`project_anchor` の結果・作業領域が
/// 取れなければ `pos`）と、窓ごとの判定の結果の 2 つを返す（組の寄せが計算し直さずに使う）。
fn project_restore_parts(
    anchor: Anchor,
    pos: PointPx,
    size: SizePx,
    snapshot: &MonitorSnapshot,
) -> (PointPx, PointPx) {
    // wa＝`pos` に置いた窓矩形の中心が属するモニタの work area（最近傍規則込み）。
    // 空 snapshot は identity（架空矩形を発明しない・既存縮退流儀・Req5.1）。
    let window = rect_at(pos, size);
    let Some(wa) = work_area_for_window(snapshot, window) else {
        tracing::info!(
            target: "areka::persist::project",
            ?anchor, input_x = pos.x, input_y = pos.y,
            "project_restore: work area なし→identity"
        );
        return (pos, pos);
    };

    // アンカー辺の再導出（5.2）＝ project_anchor へ委譲（Bottom は下端吸着の `bottom_snap`・
    // Top/Left/Right は wa 対応辺固定・Free は identity）。二重定義しない（Req1.6/1.8）。
    // project_anchor 内部の wa も同一 window 矩形から引くため補軸 clamp の wa と整合する。
    let projected = project_anchor(anchor, pos, size, Some(snapshot));

    // 補軸 clamp は「アンカー射影後の char 矩形が work area と**全く交差しない**
    // （＝完全に不可視）」ときのみ適用する（Req5.1: モニタ構成変化で保存位置が域外へ
    // 落ちたときの可視化）。一部でも交差する（＝可視）なら保存位置（アンカー辺再導出のみ）を
    // そのまま用いる（Req5.3: 収まる＝一部でも可視のとき不要な再射影をしない）。
    //
    // これで通常構成では復元＝保存（idempotent）を保証し、保存側（`project_anchor`／
    // `bottom_snap` は補軸 x を clamp しない）との非対称による座標ずれを解消する
    // ——実機サインオフ検出: 端付近（右端を数十 px はみ出す）へ置いた Bottom char が、
    // 復元でだけ `wa.right − w` へ内側に寄せられて立ち位置がずれ、追従 balloon もずれた
    // （保存 x=3493・wa.right=3840・w=434 → 復元だけが 3406 へ clamp していた）。
    let projected_rect = rect_at(projected, size);
    let visible = projected_rect.left < wa.right
        && projected_rect.right > wa.left
        && projected_rect.top < wa.bottom
        && projected_rect.bottom > wa.top;
    let result = if visible {
        // Req5.3: 一部でも可視 → 保存位置（アンカー辺再導出のみ）をそのまま用いる。
        projected
    } else {
        // Req5.1: 完全に不可視（モニタ構成変化等）→ 補軸を wa 内へ寄せて可視化する。
        match anchor {
            Anchor::Bottom | Anchor::Top => PointPx {
                x: clamp_axis(projected.x, wa.left, wa.right.saturating_sub(size.w)),
                y: projected.y,
            },
            Anchor::Left | Anchor::Right => PointPx {
                x: projected.x,
                y: clamp_axis(projected.y, wa.top, wa.bottom.saturating_sub(size.h)),
            },
            Anchor::Free => PointPx {
                x: clamp_axis(projected.x, wa.left, wa.right.saturating_sub(size.w)),
                y: clamp_axis(projected.y, wa.top, wa.bottom.saturating_sub(size.h)),
            },
        }
    };
    // 復元射影の計測ログ（実機診断・保存↔復元の座標突合）: input＝保存値、projected＝
    // アンカー辺再導出後、result＝可視性判定後。visible=false かつ input≠result なら
    // モニタ構成変化での可視化 clamp が働いた証跡。
    tracing::info!(
        target: "areka::persist::project",
        ?anchor,
        input_x = pos.x, input_y = pos.y,
        size_w = size.w, size_h = size.h,
        wa_left = wa.left, wa_top = wa.top, wa_right = wa.right, wa_bottom = wa.bottom,
        projected_x = projected.x, projected_y = projected.y,
        visible,
        result_x = result.x, result_y = result.y,
        clamped = (result.x != projected.x || result.y != projected.y),
        "project_restore"
    );
    (projected, result)
}

/// 起動時とシェルの切り替え（本体の資産づくり）の永続値先読み（本モジュール**唯一の IO 点**・design C1・A1 シーム）。
///
/// ゴーストの descript を確かめ（[`read_ghost_names`]）、Ghost 永続スコープ root
/// （[`profile_areka_root`]`(&`[`shiori_dir`]`(ghost_root))`＝boot 結線と同じ SHIORI の置き場の
/// 綴り・単一権威）を導出し、[`load_scope`]`(Ghost, ..)` で永続 entries を読む。シェルは
/// 解決しない（既定のシェルが無くても保存した状態を戻す）。
///
/// 全縮退は warn/debug ＋ 空 `Vec`（起動を止めない・panic しない・Req6.1/6.3）:
/// - ゴーストの descript の不在・読めない → `warn!` ＋ 空（本関数で処理）。
/// - ファイル不在・破損・read 障害 → [`load_scope`] が寛容に空へ縮退する（sylphya 契約）。
///
/// IO は本番実装 [`FsPersistIo`]（sylphya 内で FS に触れる唯一の型）。返り値は決定論的順序
/// （[`load_scope`] の契約）。
pub fn load_restored_state(ghost_root: &Path) -> Vec<(PersistKey, String)> {
    // ゴーストの descript が在って読めるときだけ戻す（不在の根を永続の根にしない）。
    // charset の無い descript は SSP の既定（Ansi）で読む。
    if let Err(err) = read_ghost_names(ghost_root, DefaultEncoding::Ansi) {
        warn!(
            ghost_root = %ghost_root.display(),
            error = ?err,
            "ゴーストのマウント解決に失敗（永続復元をスキップ＝既定位置解決へ縮退）"
        );
        return Vec::new();
    }

    // Ghost スコープ root＝profile_areka_root(shiori_dir)（boot 結線と同一構築・R6.5）。
    let roots = ScopeRoots {
        ghost: Some(profile_areka_root(&shiori_dir(ghost_root))),
        ..ScopeRoots::default()
    };

    // load_scope は全 IO 縮退（不在・破損・read 障害）を寛容に空へ落とす（sylphya 契約・Req6.1）。
    load_scope(PersistScope::Ghost, &roots, &FsPersistIo)
}

/// entries から指定 key の値を線形探索する（純関数・決定論）。
fn entry_value<'a>(entries: &'a [(PersistKey, String)], target: PersistKey) -> Option<&'a str> {
    entries
        .iter()
        .find(|(k, _)| *k == target)
        .map(|(_, v)| v.as_str())
}

/// 復元 merge（純関数・決定論・**永続不書込**＝Req5.4 の構造遮断——本関数から書込 API へ
/// 到達できない・返すのは新しい `Vec<ScopePlacement>` のみ）（design C1）。
///
/// 各 scope について:
/// - `WindowPos` x/y が**両軸とも** [`parse_px`] できたときのみ char_pos を保存値へ差し替え、
///   [`project_restore_parts`]（アンカー再解決＋見えないときだけ域内 clamp・毎起動 live 再射影）を適用する。片軸でも
///   欠損/非数値なら resolver 既定 char_pos を保持する（Req1.5/6.1）。
/// - `BalloonOffset` x/y が両軸とも parse できれば、その値を**そのまま**左上基準オフセット
///   として採用する（保存表現＝ランタイム表現＝char 左上基準・アンカー辺基準変換なし）。
///   無ければ resolver 既定 offset を保持する
///   （Req2.4）。いずれの場合も最終 char_pos に追従させて `balloon_pos` を再導出する。
///
/// # 事後条件（design C1 Postconditions）
///
/// 出力は入力と同じ scope 集合・同じ寸法（char/balloon）・同じ anchor。変わるのは char_pos と
/// balloon 導出（`balloon_pos`/`balloon_offset`）のみ。永続状態には触れない。
///
/// `balloon_offset ≡ balloon_pos − char_pos` が成立するのは**本関数の出力時点**までである
/// （windowposition-limit DD6）。`main.rs` の `restore_merged_placements` は本関数と同じ配置を返す
/// [`restore_and_judge`] の直後に起動時関門
/// [`super::balloon_limit::apply_balloon_limit`] を通し、`balloon_limit` が真の scope の
/// `balloon_pos` だけを作業領域内へ補正する（`balloon_offset` は生値のまま＝補正を
/// 焼き付けない）。ゆえに関門通過後の `balloon_pos` は表示位置、`balloon_offset` は論理
/// 相対位置であり、両者の差が恒等式を満たすとは限らない。
///
/// 寄せは 2 段: 窓ごとの寄せは [`project_restore_parts`]（`merge_scope` の中）、組の寄せは
/// [`super::follow::keep_restored_groups`]（その後・鎖の前の窓との組と記憶の矩形の組をつないだ束ごと・
/// window-work-area-clamp 要件 4.1〜4.4・audit-placement-follow-residue 要件 1）。
/// 組の寄せは窓とバルーンの位置を同じ量だけ動かし、`balloon_offset` は変えない（要件 7.7）。
/// 寄せた位置は記憶へ書き戻さない（要件 4.5・本関数は純関数のまま）。
///
/// `ScopePlacement.scope` は `usize`・[`PersistKey`] の scope は `u32` ゆえ、entries 突合は
/// `scope as u32` で一貫キャストする。
pub fn apply_restored_placements(
    placements: Vec<ScopePlacement>,
    entries: &[(PersistKey, String)],
    snapshot: &MonitorSnapshot,
) -> Vec<ScopePlacement> {
    restore_and_judge(placements, entries, snapshot).0
}

/// 窓の一式ごとに据える「この起動で記憶の位置を寄せて表示した」（window-work-area-clamp 要件 4.6）。
/// 真なら記憶への送り口（[`persist_entries`]）は、どの呼び手の束からも古い記憶へ隣り合いの印を
/// 書き足さない。保存はしない。
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RestoredPositionsClamped(pub bool);

/// [`apply_restored_placements`] と同じ配置に、記憶から戻したスコープのどれかを吸着の当て直し
/// だけの位置と違う位置に表示したか（[`RestoredPositionsClamped`] の値）を添えて返す。
/// 判定は組の寄せの後の配置で、写像が計算した値を使う（記憶の位置を計算し直さない）。
pub fn restore_and_judge(
    placements: Vec<ScopePlacement>,
    entries: &[(PersistKey, String)],
    snapshot: &MonitorSnapshot,
) -> (Vec<ScopePlacement>, bool) {
    // 鎖の見方は記憶の無いスコープも含む全スコープの descript の揃え方（起動の前で台本の吸着は無い）。
    let chain = ChainRule::from_terms(
        placements
            .iter()
            .map(|p| {
                let terms = ChainTerms {
                    descript: p.anchor,
                    default_x: None,
                    live: p.anchor,
                };
                (p.scope, terms)
            })
            .collect(),
    );
    let (mut merged, windows) = restored_windows(placements, entries, snapshot);
    for shift in keep_restored_groups(&windows, &chain, snapshot) {
        let Some(p) = merged.iter_mut().find(|p| p.scope == shift.scope) else {
            continue; // 組の窓は merged から組んだので必ずある。
        };
        p.balloon_pos = PointPx {
            x: p.balloon_pos
                .x
                .saturating_add(shift.to.x.saturating_sub(p.char_pos.x)),
            y: p.balloon_pos
                .y
                .saturating_add(shift.to.y.saturating_sub(p.char_pos.y)),
        };
        p.char_pos = shift.to;
    }
    let clamped = windows.iter().any(|w| {
        merged
            .iter()
            .any(|p| p.scope == w.scope && p.char_pos != w.projected)
    });
    (merged, clamped)
}

/// 窓ごとの写像（[`merge_scope`]）の結果と、記憶から位置を戻したスコープの組の寄せの入力
/// （スコープ昇順）。記憶の矩形と吸着の当て直しだけの位置は写像が計算した値をそのまま使う
/// （計算し直さない）。印は警告を出さない読み（知らない語の警告は [`saved_adjacency`] の 1 回に任せる）。
fn restored_windows(
    placements: Vec<ScopePlacement>,
    entries: &[(PersistKey, String)],
    snapshot: &MonitorSnapshot,
) -> (Vec<ScopePlacement>, Vec<RestoredWindow>) {
    let mut windows = Vec::new();
    let merged = placements
        .into_iter()
        .map(|p| {
            let (merged, parts) = merge_scope(p, entries, snapshot);
            if let Some((remembered, projected)) = parts {
                windows.push(RestoredWindow {
                    scope: merged.scope,
                    remembered: ChainRect {
                        x: remembered.x,
                        y: remembered.y,
                        w: merged.char_size.w,
                        h: merged.char_size.h,
                    },
                    projected,
                    shown: merged.char_pos,
                    mark: entry_value(
                        entries,
                        PersistKey::WindowAdjacent {
                            scope: merged.scope as u32,
                        },
                    )
                    .and_then(AdjacencyMark::parse),
                });
            }
            merged
        })
        .collect();
    windows.sort_by_key(|w| w.scope);
    (merged, windows)
}

/// 保存 WindowPos の x・y が**両方とも** [`parse_px`] できたときだけ、その値（x は原点基準の
/// まま）を返す。片方だけ・数字でない値・鍵なしは `None`（[`merge_scope`] と
/// [`has_saved_char_pos`] が共有する唯一の読み取り＝判定を 2 か所に書かない）。
fn saved_char_pos(entries: &[(PersistKey, String)], scope: u32) -> Option<PointPx> {
    let key_of = |axis| PersistKey::WindowPos { scope, axis };
    Some(PointPx {
        x: saved_axis(entries, key_of, Axis::X)?,
        y: saved_axis(entries, key_of, Axis::Y)?,
    })
}

/// 保存値（`key_of` が組む鍵＝WindowPos か BalloonOffset）の 1 軸を [`parse_px`] で読む
/// （採否は決めない＝窓位置の判定は [`saved_char_pos`] だけが持つ。復元ログが軸ごとの読みを
/// 出すためにも使う）。
fn saved_axis(
    entries: &[(PersistKey, String)],
    key_of: impl Fn(Axis) -> PersistKey,
    axis: Axis,
) -> Option<i32> {
    entry_value(entries, key_of(axis)).and_then(parse_px)
}

/// スコープのキャラクター窓の位置（x・y の両方）が記憶にあるか
/// （char-position-save-on-exit 2.4/2.6・design RestoredScopes）。
///
/// [`merge_scope`] が記憶の値を採るスコープで、かつそのときだけ真。記憶の値が既定と
/// 同じでも真（値の違いでなく記憶の有無で決める・2.6）。
fn has_saved_char_pos(entries: &[(PersistKey, String)], scope: usize) -> bool {
    saved_char_pos(entries, scope as u32).is_some()
}

/// 隣り合いの印 1 つ分の記憶の項目（dpi-realign-remembered-chain 要件 6.2・design AdjacencyPersist）。
pub fn adjacency_entry(scope: u32, mark: AdjacencyMark) -> (PersistKey, String) {
    (
        PersistKey::WindowAdjacent { scope },
        mark.as_str().to_string(),
    )
}

/// 記憶の項目からスコープの隣り合いの印を読む。鍵が無ければ `None`。知らない語は `warn!` を
/// 出して `None`（印の無い古い記憶と同じ扱い＝位置のまま立つ）。
fn saved_adjacency(entries: &[(PersistKey, String)], scope: usize) -> Option<AdjacencyMark> {
    let value = entry_value(
        entries,
        PersistKey::WindowAdjacent {
            scope: scope as u32,
        },
    )?;
    let mark = AdjacencyMark::parse(value);
    if mark.is_none() {
        warn!(
            target: "areka::persist::restore",
            scope,
            value,
            "隣り合いの印の語が分からないので印は無いものとして扱う"
        );
    }
    mark
}

/// 記憶に位置（x・y の両方）があるスコープと、その覚えた印の一覧（`restore_merged_placements` が返す）。
/// 位置の無いスコープは鍵に入れない（印だけあっても入れない）。
pub fn restored_scope_marks(
    entries: &[(PersistKey, String)],
    merged: &[ScopePlacement],
) -> BTreeMap<usize, Option<AdjacencyMark>> {
    merged
        .iter()
        .filter(|p| has_saved_char_pos(entries, p.scope))
        .map(|p| (p.scope, saved_adjacency(entries, p.scope)))
        .collect()
}

/// 1 scope 分の復元 merge（[`apply_restored_placements`] の要素写像・純関数）。
/// 記憶の位置が採られたときは、記憶の左上（今の大きさで置いた・吸着の当て直しの前）と
/// 吸着の当て直しだけをした左上も返す（組の寄せの入力・[`restored_windows`]）。
fn merge_scope(
    placement: ScopePlacement,
    entries: &[(PersistKey, String)],
    snapshot: &MonitorSnapshot,
) -> (ScopePlacement, Option<(PointPx, PointPx)>) {
    let scope = placement.scope as u32;
    let window_key = |axis| PersistKey::WindowPos { scope, axis };
    let offset_key = |axis| PersistKey::BalloonOffset { scope, axis };
    warn_unusable_pair(entries, scope, window_key);
    warn_unusable_pair(entries, scope, offset_key);

    // --- char_pos: 保存 WindowPos が両軸とも parse できたときのみ差替え＋再射影（1.4/1.5/6.1）---
    let saved_window = saved_char_pos(entries, scope);
    let (char_pos, restored_parts) = match saved_window {
        // 両軸そろったときのみ保存値を採用し、毎起動 live 再射影（アンカー再解決＋域内 clamp）。
        // 保存 x は**原点＝下端中央**基準（Bottom）ゆえ、現寸の左上へ戻してから射影へ渡す
        // （寸法が保存時と異なっても原点が一致する＝キャラもバルーンも横へずれない）。
        Some(saved) => {
            let remembered = char_pos_from_origin_x(placement.anchor, saved, placement.char_size);
            let (projected, shown) =
                project_restore_parts(placement.anchor, remembered, placement.char_size, snapshot);
            (shown, Some((remembered, projected)))
        }
        // 片軸でも欠損/非数値 → resolver 既定 char_pos を保持（1.5/6.1）。
        None => (placement.char_pos, None),
    };

    // --- balloon: 保存 offset があればそのまま採用、無ければ既定 offset 保持（2.3/2.4）---
    let saved_bx = saved_axis(entries, offset_key, Axis::X);
    let saved_by = saved_axis(entries, offset_key, Axis::Y);
    // キーワード再導出の素材は**保存値が効いた scope では落とす**（windowposition-limit
    // 要件 4.7「永続値を優先し、キーワード指定の適用は初期既定位置の供給にとどめる」）。
    // 落とさないと、実表示寸確定の再導出がユーザーの保存した相対位置をキーワード既定へ
    // 上書きしてしまう——保存値優先の順位が静かに反転する。offset 欠損側（resolver 既定を
    // 保持する腕）は素材をそのまま運ぶ。
    // 基準対（areka-P0-balloon-offset-dpi 要件 5.1／5.2・design D15）も同じ腕で決まる。
    // 保存値は**保存時の表示 DPI を記録していない**（要件 5.1）ゆえ、採用した保存値の基準
    // DPI は発明できない——`OffsetBase::unpinned`＝未係留（`dpi: None`）として運び、最初の
    // 観測で値を変えずに係留させる（要件 5.2「換算せずそのまま採用」の実装上の姿）。
    // 採寸 DPI を継ぐと、主モニタと違う拡大率へ復元されたとき保存値を二重に拡大する。
    let (balloon_offset, balloon_offset_base, balloon_keyword_base) = match (saved_bx, saved_by) {
        // 保存 offset は**char 左上基準**（ランタイム BalloonFollow.offset と同一基準）ゆえ
        // 基準変換なしで採用する（2.3・[`balloon_offset_entries`] の基準記述）。
        // 未係留の基準は「記憶から戻した」の印の源でもある（窓を組むときに
        // `face_windowposition::BalloonUserMoved` を付ける）。
        (Some(x), Some(y)) => {
            let saved = PointPx { x, y };
            (saved, OffsetBase::unpinned(saved), None)
        }
        // offset 欠損 → resolver 既定 offset（左上基準）を保持（2.4）。基準対も配置式が
        // 出したもの（`Some(採寸 DPI)`）をそのまま運ぶ。
        _ => (
            placement.balloon_offset,
            placement.balloon_offset_base,
            placement.balloon_keyword_base,
        ),
    };
    // どちらの場合も最終 char_pos へ追従させて balloon_pos を再導出する。
    let balloon_pos = PointPx {
        x: char_pos.x.saturating_add(balloon_offset.x),
        y: char_pos.y.saturating_add(balloon_offset.y),
    };

    // 復元マージの計測ログ（実機診断・保存↔復元の座標突合）: saved_window＝保存された
    // 窓位置（無ければ default 保持）、default_char＝resolver 既定、char_pos＝復元後の窓位置、
    // balloon_offset（左上基準・保存値ならそのまま）、balloon_pos＝最終バルーン位置。
    tracing::info!(
        target: "areka::persist::restore",
        scope = placement.scope,
        anchor = ?placement.anchor,
        saved_win_x = ?saved_axis(entries, window_key, Axis::X),
        saved_win_y = ?saved_axis(entries, window_key, Axis::Y),
        default_char_x = placement.char_pos.x, default_char_y = placement.char_pos.y,
        char_x = char_pos.x, char_y = char_pos.y,
        char_w = placement.char_size.w, char_h = placement.char_size.h,
        saved_off_x = ?saved_bx, saved_off_y = ?saved_by,
        balloon_off_x = balloon_offset.x, balloon_off_y = balloon_offset.y,
        balloon_x = balloon_pos.x, balloon_y = balloon_pos.y,
        "merge_scope restore"
    );

    let merged = ScopePlacement {
        scope: placement.scope,
        char_pos,
        char_size: placement.char_size,
        balloon_pos,
        balloon_size: placement.balloon_size,
        balloon_offset,
        // 保存値採用腕では未係留・欠損腕では配置式の基準対。上の match が決めている。
        balloon_offset_base,
        // limit の解決値は merge の対象外（永続化しない・毎起動 descript から解決する）
        // ゆえ、入力の値をそのまま転記する（merge 規則は無改変）。
        balloon_limit: placement.balloon_limit,
        anchor: placement.anchor,
        // 保存値が効いた scope では `None`（要件 4.7）。上の match が決めている。
        balloon_keyword_base,
    };
    (merged, restored_parts)
}

/// x・y の組の鍵のうち、在るが読めない鍵と、相方の鍵が無い鍵に `warn!` を 1 行ずつ出す
/// （spec: audit-placement-core 要件 9.2）。どちらも [`merge_scope`] は採らずに既定へ戻る。相方が在って
/// 読めないときは、相方の行だけを出す。
fn warn_unusable_pair(
    entries: &[(PersistKey, String)],
    scope: u32,
    key_of: impl Fn(Axis) -> PersistKey,
) {
    for (axis, other) in [(Axis::X, Axis::Y), (Axis::Y, Axis::X)] {
        let key = key_of(axis);
        let Some(value) = entry_value(entries, key) else {
            continue;
        };
        let reason = if parse_px(value).is_none() {
            "数として読めない"
        } else if entry_value(entries, key_of(other)).is_none() {
            "片方の軸だけ"
        } else {
            continue;
        };
        warn!(
            target: "areka::persist::restore",
            event = "persist_value_unusable",
            scope,
            key = %key.to_canonical_key(),
            value,
            reason,
            "記憶の値が使えないので既定の位置へ戻す"
        );
    }
}

#[path = "persist_save.rs"]
mod save;
#[cfg(test)]
use save::with_adjacency_marks;
pub use save::{
    PersistWiring, forget_entries, persist_entries, persist_unremembered_char_positions,
};

#[cfg(test)]
#[path = "persist_adjacency_tests.rs"]
mod adjacency_tests;
#[cfg(test)]
#[path = "balloon_offset_persist_roundtrip_tests.rs"]
mod balloon_offset_persist_roundtrip_tests;
#[cfg(test)]
#[path = "persist_entries_tests.rs"]
mod entries_tests;
#[cfg(test)]
#[path = "persist_finalize_save_tests.rs"]
pub(crate) mod finalize_save_tests;
#[cfg(test)]
#[path = "persist_io_wiring_tests.rs"]
mod io_wiring_tests;
#[cfg(test)]
#[path = "persist_restore_group_tests.rs"]
mod restore_group_tests;
#[cfg(test)]
#[path = "persist_restore_offset_base_tests.rs"]
mod restore_offset_base_tests;
#[cfg(test)]
#[path = "persist_restore_tests.rs"]
mod restore_tests;
#[cfg(test)]
#[path = "persist_saved_form_tests.rs"]
mod saved_form_tests;
