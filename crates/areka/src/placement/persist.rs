//! 永続値 ⇄ 配置の変換を所有する純関数群（areka-P0-position-persist・design C1）。
//!
//! 統一プロパティシステム（sylphya）が確定した永続 key 族（窓位置・バルーン相対
//! オフセット・起動記録・vanish 回数）を**消費**し、IO・World 変異と分離した
//! 決定論的な純関数として「値を差す／値を書く」変換を提供する。永続ストア実体
//! （形式・原子性・寛容読取・スコープ分離）は sylphya の領分であり、本モジュールは
//! その契約（[`areka_sylphya::PersistKey`]／[`areka_sylphya::Axis`]）を消費するのみ。
//!
//! 本モジュールは永続への書込 API を持たない（保存の投函口は上位の結線層が持つ）。
//! task 1.1（Foundation）で用意するのは決定論的な変換のみ——寛容 parse（[`parse_px`]）
//! と保存 entries 構築（[`char_pos_entries`]／[`balloon_offset_entries`]）。復元 merge・
//! 再射影・バルーン基準変換・`PersistWiring` は後続タスクで本モジュールへ追加する。

use std::path::Path;

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::resolve;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{Axis, PersistKey, PersistScope, ScopeRoots, SylphyaPublisher, load_scope};
use bevy_ecs::world::World;
use tracing::{debug, warn};

use super::follow::{MonitorSnapshot, project_anchor, work_area_for_window};
use super::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};

/// 永続値の寛容 parse（design C1・6.1）。
///
/// 数値文字列 → `Some(i32)`。非数値・空文字列 → `None`（＝「値なし」・呼び手が warn＋
/// 既定へ縮退する）。前後空白は許容する（寛容）。決定論（同一入力→同一出力）。
pub fn parse_px(value: &str) -> Option<i32> {
    // 寛容縮退（6.1）: 非数値・空・小数は「値なし」＝None（panic なし）。
    value.trim().parse::<i32>().ok()
}

/// キャラ窓位置の保存 entries を構築する（design C1・純関数・1.6）。
///
/// スコープ別の [`PersistKey::WindowPos`]（X/Y）へ `pos` を i32 の `Display` で
/// 文字列化して載せる。値ドメインは物理 px・仮想スクリーン絶対 i32（負値可）。
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
/// スコープ別の [`PersistKey::BalloonOffset`]（X/Y）へアンカー辺基準 offset を i32 の
/// `Display` で文字列化して載せる。`offset_persist` は基準変換済み（アンカー辺基準）の
/// 物理 px オフセットであること（変換自体は後続タスクの純関数が担う）。
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

/// バルーン相対オフセットのアンカー辺基準点（**char 左上相対**・物理 px・design C1）。
///
/// サーフェス寸法変動に対して不変な基準辺を、char 窓左上を原点とした相対点で返す:
/// - `Bottom`（下端吸着）: `(0, h)` — 下端。左上ではなく下端を基準にすることで、
///   サーフェス高さが変わっても下端からの距離が保たれる（2.2）。
/// - `Top`・`Left`: `(0, 0)` — 左上。
/// - `Right`: `(w, 0)` — 右上（右端）。
/// - `Free`: `(0, 0)` — 左上（縮退・アンカー辺なし。往復恒等のため 0 で固定・檻固定）。
fn anchor_edge_basis(anchor: Anchor, char_size: SizePx) -> PointPx {
    match anchor {
        // Bottom＝**下端中央**（伺かの立ち絵は足元中央が接地点＝原点）。x も中央基準に
        // 取ることで、サーフェス寸が変わっても（幅・高さとも）バルーンの相対位置が不変になる
        // （Req2.2 の「寸法変動に不変な基準点」を x 軸まで徹底）。
        Anchor::Bottom => PointPx {
            x: char_size.w / 2,
            y: char_size.h,
        },
        Anchor::Right => PointPx {
            x: char_size.w,
            y: 0,
        },
        // Top・Left・Free は左上基準（Free は縮退・6.x/2.2）。
        Anchor::Top | Anchor::Left | Anchor::Free => PointPx { x: 0, y: 0 },
    }
}

/// バルーン相対オフセットを**保存方向**へ基準変換する（design C1・純関数・2.2/2.5）。
///
/// `offset_tl` は char 窓左上を基準に採ったセッション内表現。保存はサーフェス寸に
/// 不変なアンカー辺基準へ移す: `persist = offset_tl − 基準点(char 左上相対)`。
/// これにより下端吸着キャラでもサーフェス高さ変動でオフセットがずれない（2.2）。
pub fn balloon_offset_to_persist(anchor: Anchor, offset_tl: PointPx, char_size: SizePx) -> PointPx {
    let basis = anchor_edge_basis(anchor, char_size);
    PointPx {
        x: offset_tl.x - basis.x,
        y: offset_tl.y - basis.y,
    }
}

/// バルーン相対オフセットを**復元方向**へ基準変換する（design C1・純関数・2.2/8.5）。
///
/// [`balloon_offset_to_persist`] の厳密な逆で、**現在の** `char_size` で基準点を
/// 足し戻す: `offset_tl = persisted + 基準点(char 左上相対)`。保存時と復元時で
/// サーフェス高さが異なっても、下端基準の相対関係が保たれる（8.5）。
///
/// 不変条件: `balloon_offset_from_persist(a, balloon_offset_to_persist(a, o, s), s) == o`。
pub fn balloon_offset_from_persist(
    anchor: Anchor,
    persisted: PointPx,
    char_size: SizePx,
) -> PointPx {
    let basis = anchor_edge_basis(anchor, char_size);
    PointPx {
        x: persisted.x + basis.x,
        y: persisted.y + basis.y,
    }
}

/// 1 軸クランプ（`lo ≤ v ≤ hi`・逆転区間は `lo` 優先・非 panic）。
///
/// resolver `clamp_axis`／follow の縮退流儀と同じ min/max 書き（`i32::clamp` は
/// 逆転区間で panic するため使わない＝「パニックしない」契約の防波堤）。
fn clamp_axis(v: i32, lo: i32, hi: i32) -> i32 {
    v.min(hi).max(lo)
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
/// 委譲し二重定義しない（Req5.2・Req1.6/1.8）:
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
#[allow(dead_code)] // 結線（apply_restored_placements・task 1.4）は後続タスクの領分
pub fn project_restore(
    anchor: Anchor,
    pos: PointPx,
    size: SizePx,
    snapshot: &MonitorSnapshot,
) -> PointPx {
    // wa＝`pos` に置いた窓矩形の中心が属するモニタの work area（最近傍規則込み）。
    // 空 snapshot は identity（架空矩形を発明しない・既存縮退流儀・Req5.1）。
    let window = RectPx {
        left: pos.x,
        top: pos.y,
        right: pos.x.saturating_add(size.w),
        bottom: pos.y.saturating_add(size.h),
    };
    let Some(wa) = work_area_for_window(snapshot, window) else {
        tracing::info!(
            target: "areka::persist::project",
            ?anchor, input_x = pos.x, input_y = pos.y,
            "project_restore: work area なし→identity"
        );
        return pos;
    };

    // アンカー辺の再導出（5.2）＝ project_anchor へ委譲（Bottom は BottomSnapPolicy・
    // Top/Left/Right は wa 対応辺固定・Free は identity）。二重定義しない（Req1.6/1.8）。
    // project_anchor 内部の wa も同一 window 矩形から引くため補軸 clamp の wa と整合する。
    let projected = project_anchor(anchor, pos, size, Some(snapshot));

    // 補軸 clamp は「アンカー射影後の char 矩形が work area と**全く交差しない**
    // （＝完全に不可視）」ときのみ適用する（Req5.1: モニタ構成変化で保存位置が域外へ
    // 落ちたときの可視化）。一部でも交差する（＝可視）なら保存位置（アンカー辺再導出のみ）を
    // そのまま用いる（Req5.3: 収まる＝一部でも可視のとき不要な再射影をしない）。
    //
    // これで通常構成では復元＝保存（idempotent）を保証し、保存側（`project_anchor`／
    // BottomSnapPolicy は補軸 x を clamp しない）との非対称による座標ずれを解消する
    // ——実機サインオフ検出: 端付近（右端を数十 px はみ出す）へ置いた Bottom char が、
    // 復元でだけ `wa.right − w` へ内側に寄せられて立ち位置がずれ、追従 balloon もずれた
    // （保存 x=3493・wa.right=3840・w=434 → 復元だけが 3406 へ clamp していた）。
    let projected_rect = RectPx {
        left: projected.x,
        top: projected.y,
        right: projected.x.saturating_add(size.w),
        bottom: projected.y.saturating_add(size.h),
    };
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
    result
}

/// 起動時の永続値先読み（本モジュール**唯一の IO 点**・design C1・A1 シーム）。
///
/// mount 解決（[`areka_parsers::package::resolve`]＝mount 規則の単一権威・二重実装しない）で
/// ghost mount を得て、Ghost 永続スコープ root（[`profile_areka_root`]`(&model.shiori.dir)`＝
/// boot 結線と同一構築・単一権威）を導出し、[`load_scope`]`(Ghost, ..)` で永続 entries を読む。
///
/// 全縮退は warn/debug ＋ 空 `Vec`（起動を止めない・panic しない・Req6.1/6.3）:
/// - mount 解決失敗 → `warn!` ＋ 空（本関数で処理）。
/// - ファイル不在・破損・read 障害 → [`load_scope`] が寛容に空へ縮退する（sylphya 契約）。
///
/// IO は本番実装 [`FsPersistIo`]（sylphya 内で FS に触れる唯一の型）。返り値は決定論的順序
/// （[`load_scope`] の契約）。
#[allow(dead_code)] // 結線（main.rs シーム・task 6.1）は後続タスクの領分
pub fn load_restored_state(
    ghost_root: &Path,
    default_encoding: DefaultEncoding,
) -> Vec<(PersistKey, String)> {
    // mount 解決は単一権威（source.rs と同一 resolve 呼び口・規則を二重化しない）。
    let model = match resolve(ghost_root, default_encoding) {
        Ok(model) => model,
        Err(err) => {
            warn!(
                ghost_root = %ghost_root.display(),
                error = ?err,
                "ゴーストのマウント解決に失敗（永続復元をスキップ＝既定位置解決へ縮退）"
            );
            return Vec::new();
        }
    };

    // Ghost スコープ root＝profile_areka_root(shiori.dir)（boot 結線と同一構築・R6.5）。
    let roots = ScopeRoots {
        ghost: Some(profile_areka_root(&model.shiori.dir)),
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
///   [`project_restore`]（アンカー再解決＋域内 clamp・毎起動 live 再射影）を適用する。片軸でも
///   欠損/非数値なら resolver 既定 char_pos を保持する（Req1.5/6.1）。
/// - `BalloonOffset` x/y が両軸とも parse できれば [`balloon_offset_from_persist`]（**現** char_size・
///   anchor 基準）で左上基準オフセットを導出する。無ければ resolver 既定 offset を保持する
///   （Req2.4）。いずれの場合も最終 char_pos に追従させて `balloon_pos` を再導出する。
///
/// # 事後条件（design C1 Postconditions）
///
/// 出力は入力と同じ scope 集合・同じ寸法（char/balloon）・同じ anchor。変わるのは char_pos と
/// balloon 導出（`balloon_pos`/`balloon_offset`）のみ。永続状態には触れない。
///
/// `ScopePlacement.scope` は `usize`・[`PersistKey`] の scope は `u32` ゆえ、entries 突合は
/// `scope as u32` で一貫キャストする。
#[allow(dead_code)] // 結線（main.rs シーム・task 6.1）は後続タスクの領分
pub fn apply_restored_placements(
    placements: Vec<ScopePlacement>,
    entries: &[(PersistKey, String)],
    snapshot: &MonitorSnapshot,
) -> Vec<ScopePlacement> {
    placements
        .into_iter()
        .map(|p| merge_scope(p, entries, snapshot))
        .collect()
}

/// 1 scope 分の復元 merge（[`apply_restored_placements`] の要素写像・純関数）。
fn merge_scope(
    placement: ScopePlacement,
    entries: &[(PersistKey, String)],
    snapshot: &MonitorSnapshot,
) -> ScopePlacement {
    let scope = placement.scope as u32;

    // --- char_pos: 保存 WindowPos が両軸とも parse できたときのみ差替え＋再射影（1.4/1.5/6.1）---
    let saved_x = entry_value(
        entries,
        PersistKey::WindowPos {
            scope,
            axis: Axis::X,
        },
    )
    .and_then(parse_px);
    let saved_y = entry_value(
        entries,
        PersistKey::WindowPos {
            scope,
            axis: Axis::Y,
        },
    )
    .and_then(parse_px);
    let char_pos = match (saved_x, saved_y) {
        // 両軸そろったときのみ保存値を採用し、毎起動 live 再射影（アンカー再解決＋域内 clamp）。
        // 保存 x は**原点＝下端中央**基準（Bottom）ゆえ、現寸の左上へ戻してから射影へ渡す
        // （寸法が保存時と異なっても原点が一致する＝キャラもバルーンも横へずれない）。
        (Some(x), Some(y)) => project_restore(
            placement.anchor,
            char_pos_from_origin_x(placement.anchor, PointPx { x, y }, placement.char_size),
            placement.char_size,
            snapshot,
        ),
        // 片軸でも欠損/非数値 → resolver 既定 char_pos を保持（1.5/6.1）。
        _ => placement.char_pos,
    };

    // --- balloon: 保存 offset があれば基準逆変換で導出、無ければ既定 offset 保持（2.3/2.4）---
    let saved_bx = entry_value(
        entries,
        PersistKey::BalloonOffset {
            scope,
            axis: Axis::X,
        },
    )
    .and_then(parse_px);
    let saved_by = entry_value(
        entries,
        PersistKey::BalloonOffset {
            scope,
            axis: Axis::Y,
        },
    )
    .and_then(parse_px);
    let balloon_offset = match (saved_bx, saved_by) {
        // 保存 offset（アンカー辺基準）を現 char_size で左上基準へ足し戻す（2.2/2.3）。
        (Some(x), Some(y)) => {
            balloon_offset_from_persist(placement.anchor, PointPx { x, y }, placement.char_size)
        }
        // offset 欠損 → resolver 既定 offset（左上基準）を保持（2.4）。
        _ => placement.balloon_offset,
    };
    // どちらの場合も最終 char_pos へ追従させて balloon_pos を再導出する。
    let balloon_pos = PointPx {
        x: char_pos.x.saturating_add(balloon_offset.x),
        y: char_pos.y.saturating_add(balloon_offset.y),
    };

    // 復元マージの計測ログ（実機診断・保存↔復元の座標突合）: saved_window＝保存された
    // 窓位置（無ければ default 保持）、default_char＝resolver 既定、char_pos＝復元後の窓位置、
    // balloon_offset（左上基準・現 char_size で逆変換済み）、balloon_pos＝最終バルーン位置。
    tracing::info!(
        target: "areka::persist::restore",
        scope = placement.scope,
        anchor = ?placement.anchor,
        saved_win_x = ?saved_x, saved_win_y = ?saved_y,
        default_char_x = placement.char_pos.x, default_char_y = placement.char_pos.y,
        char_x = char_pos.x, char_y = char_pos.y,
        char_w = placement.char_size.w, char_h = placement.char_size.h,
        saved_off_x = ?saved_bx, saved_off_y = ?saved_by,
        balloon_off_x = balloon_offset.x, balloon_off_y = balloon_offset.y,
        balloon_x = balloon_pos.x, balloon_y = balloon_pos.y,
        "merge_scope restore"
    );

    ScopePlacement {
        scope: placement.scope,
        char_pos,
        char_size: placement.char_size,
        balloon_pos,
        balloon_size: placement.balloon_size,
        balloon_offset,
        anchor: placement.anchor,
    }
}

// ---------------------------------------------------------------------------
// State Management（PersistWiring）＋保存投函ヘルパ（design C1・task 2.1・
// Req1.1 ドラッグ確定即時書込／1.9 発火規律／6.2 保存失敗縮退／7.1 Ghost スコープ固定）
// ---------------------------------------------------------------------------

/// UI スレッド常駐の保存投函口（**NonSend** リソース・`MouseWiring`／`Emo2Wiring` 先例）。
///
/// [`SylphyaPublisher`] は `Clone + Send`（内部 `std::sync::mpsc::Sender`）だが `Sync` を仮定せず、
/// UI スレッド専有の規律とも一致するため **NonSend** リソースとして World に持たせる（design C1
/// State Management・軸B）。DragEnd 観測点（[`super::follow`] の task 2.2/2.3 フック）が
/// [`persist_entries`] 経由でこの publisher の clone 送信端から保存 entries を投函する。
#[allow(dead_code)] // 挿入（main.rs シーム＝C4・task 2.4/6.x）は後続タスクの領分
pub struct PersistWiring {
    /// sylphya アクターへの変異投函の送信端（`persist_put` の fire-and-forget 投函に用いる）。
    pub publisher: SylphyaPublisher,
}

/// 保存 entries を Ghost 永続スコープへ write-through 投函するフック用ヘルパ（design C1・
/// Req1.1/7.1/6.2）。
///
/// World から [`PersistWiring`] NonSend リソースを引き、存在すれば
/// [`SylphyaPublisher::persist_put`]`(PersistScope::Ghost, entries)` を呼ぶ（**Ghost スコープ固定**＝
/// 当該ゴーストスコープのみ・Req7.1）。`persist_put` は fire-and-forget（reply なし）で**非ブロッキング**
/// ——UI スレッドに同期 IO を持ち込まない（write-through の実 IO 確定は sylphya アクタースレッドの領分・
/// commit 失敗は sylphya が `error!`＋`Degraded` に縮退し旧状態を破壊しない・design「persist_put 先の
/// commit 失敗」）。
///
/// [`PersistWiring`] 不在（例: fallback boot 経路が挿入しなかった場合）は `debug!` ＋ **no-op**で、
/// **panic しない**（6.2 系縮退・無音失敗なし）。`world` は共有参照で足りる（NonSend の読取のみ）——
/// DragEnd フックは `&mut World` を保持するが `&World` へ暗黙 reborrow して渡せる。
#[allow(dead_code)] // 結線（follow.rs DragEnd フック＝C2/C3・task 2.2/2.3）は後続タスクの領分
pub fn persist_entries(world: &World, entries: Vec<(PersistKey, String)>) {
    let Some(wiring) = world.get_non_send_resource::<PersistWiring>() else {
        // fallback 未挿入等で PersistWiring が無い → debug!＋no-op（起動を止めない・6.2）。
        debug!(
            entry_count = entries.len(),
            "PersistWiring 不在: 保存投函を no-op（fallback boot 経路の縮退・6.2）"
        );
        return;
    };
    // Ghost スコープ固定（7.1）。fire-and-forget＝UI スレッド非ブロッキング（同期 IO を持ち込まない）。
    wiring.publisher.persist_put(PersistScope::Ghost, entries);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // parse_px（寛容 parse・6.1）: 数値→Some(i32)・非数値/空→None・決定論
    // ------------------------------------------------------------------

    #[test]
    fn parse_px_numeric_string_yields_some() {
        assert_eq!(parse_px("123"), Some(123));
        assert_eq!(parse_px("0"), Some(0));
        assert_eq!(parse_px("-5"), Some(-5));
        // 仮想スクリーン絶対座標は大きな値・負値もある（1.7）。
        assert_eq!(parse_px("1920"), Some(1920));
        assert_eq!(parse_px("-1080"), Some(-1080));
    }

    #[test]
    fn parse_px_non_numeric_or_empty_yields_none() {
        assert_eq!(parse_px(""), None, "空文字列は値なし");
        assert_eq!(parse_px("abc"), None, "非数値は値なし");
        assert_eq!(parse_px("12.5"), None, "小数は i32 でないため値なし");
        assert_eq!(parse_px("9999999999999999999"), None, "i32 溢れは値なし");
        assert_eq!(parse_px("   "), None, "空白のみは値なし");
    }

    #[test]
    fn parse_px_tolerates_surrounding_whitespace() {
        assert_eq!(parse_px(" 42 "), Some(42));
    }

    #[test]
    fn parse_px_is_deterministic() {
        for s in ["777", "-3", "x", ""] {
            assert_eq!(parse_px(s), parse_px(s), "同一入力→同一出力: {s:?}");
        }
    }

    // ------------------------------------------------------------------
    // char_pos_entries（保存 entries・1.6）: WindowPos X/Y の key/値等価
    // ------------------------------------------------------------------

    #[test]
    fn char_pos_entries_builds_windowpos_x_y_pairs() {
        let entries = char_pos_entries(0, PointPx { x: 1486, y: 353 });
        assert_eq!(
            entries,
            vec![
                (
                    PersistKey::WindowPos {
                        scope: 0,
                        axis: Axis::X
                    },
                    "1486".to_string()
                ),
                (
                    PersistKey::WindowPos {
                        scope: 0,
                        axis: Axis::Y
                    },
                    "353".to_string()
                ),
            ]
        );
    }

    #[test]
    fn char_pos_entries_carries_scope_and_negative_values() {
        // スコープ別（1.6）＋負値（仮想デスクトップ左／上のモニタ・1.7）。
        let entries = char_pos_entries(1, PointPx { x: -400, y: -20 });
        assert_eq!(
            entries,
            vec![
                (
                    PersistKey::WindowPos {
                        scope: 1,
                        axis: Axis::X
                    },
                    "-400".to_string()
                ),
                (
                    PersistKey::WindowPos {
                        scope: 1,
                        axis: Axis::Y
                    },
                    "-20".to_string()
                ),
            ]
        );
    }

    // ------------------------------------------------------------------
    // balloon_offset_entries（保存 entries・2.5）: BalloonOffset X/Y の key/値等価
    // ------------------------------------------------------------------

    #[test]
    fn balloon_offset_entries_builds_balloonoffset_x_y_pairs() {
        let entries = balloon_offset_entries(0, PointPx { x: -400, y: 0 });
        assert_eq!(
            entries,
            vec![
                (
                    PersistKey::BalloonOffset {
                        scope: 0,
                        axis: Axis::X
                    },
                    "-400".to_string()
                ),
                (
                    PersistKey::BalloonOffset {
                        scope: 0,
                        axis: Axis::Y
                    },
                    "0".to_string()
                ),
            ]
        );
    }

    #[test]
    fn balloon_offset_entries_carries_scope() {
        let entries = balloon_offset_entries(2, PointPx { x: 336, y: 12 });
        assert_eq!(
            entries,
            vec![
                (
                    PersistKey::BalloonOffset {
                        scope: 2,
                        axis: Axis::X
                    },
                    "336".to_string()
                ),
                (
                    PersistKey::BalloonOffset {
                        scope: 2,
                        axis: Axis::Y
                    },
                    "12".to_string()
                ),
            ]
        );
    }

    /// 保存した値が sylphya の寛容 parse で読み戻せること（key/値等価の往復・8.1 の前哨）。
    /// entries の String は parse_px でそのまま i32 へ戻る（Display ⇄ parse 対称）。
    #[test]
    fn entries_values_round_trip_through_parse_px() {
        let pos = PointPx { x: -7, y: 1040 };
        for (_, v) in char_pos_entries(0, pos) {
            assert!(parse_px(&v).is_some(), "保存値 {v:?} は parse_px で読める");
        }
        assert_eq!(parse_px(&char_pos_entries(0, pos)[0].1), Some(pos.x));
        assert_eq!(parse_px(&char_pos_entries(0, pos)[1].1), Some(pos.y));
    }

    // ------------------------------------------------------------------
    // balloon_offset_to_persist / _from_persist（アンカー辺基準変換・2.2/2.5・design C1）
    //   基準点（char 左上相対）: Bottom=(0,h)・Top/Left=(0,0)・Right=(w,0)・Free=(0,0)
    //   persist = offset_tl − 基準点(相対) ／ from_persist = persisted + 基準点(相対)
    // ------------------------------------------------------------------

    const ALL_ANCHORS: [Anchor; 5] = [
        Anchor::Top,
        Anchor::Bottom,
        Anchor::Left,
        Anchor::Right,
        Anchor::Free,
    ];

    /// 各アンカーで基準点（char 左上相対）が design C1 の規定どおりになる。
    #[test]
    fn to_persist_subtracts_the_correct_anchor_edge_basis() {
        let size = SizePx { w: 300, h: 500 };
        let offset_tl = PointPx { x: 40, y: 70 };

        // Bottom: 基準＝**下端中央** (w/2, h)=(150,500) → persist = (40−150, 70−500) = (-110, -430)
        // （原点は足元中央＝寸法変動で動かない点。x も中央基準に取り Req2.2 を x 軸まで徹底）
        assert_eq!(
            balloon_offset_to_persist(Anchor::Bottom, offset_tl, size),
            PointPx { x: -110, y: -430 }
        );
        // Top: 基準 (0, 0) → persist = offset_tl
        assert_eq!(
            balloon_offset_to_persist(Anchor::Top, offset_tl, size),
            PointPx { x: 40, y: 70 }
        );
        // Left: 基準 (0, 0) → persist = offset_tl
        assert_eq!(
            balloon_offset_to_persist(Anchor::Left, offset_tl, size),
            PointPx { x: 40, y: 70 }
        );
        // Right: 基準 (w, 0) → persist = (40−300, 70−0) = (-260, 70)
        assert_eq!(
            balloon_offset_to_persist(Anchor::Right, offset_tl, size),
            PointPx { x: -260, y: 70 }
        );
        // Free: 基準 (0, 0)（縮退・檻固定）→ persist = offset_tl
        assert_eq!(
            balloon_offset_to_persist(Anchor::Free, offset_tl, size),
            PointPx { x: 40, y: 70 }
        );
    }

    /// from_persist は現在の char_size で基準点を足し戻す（to_persist の逆・現寸使用）。
    #[test]
    fn from_persist_adds_back_the_anchor_edge_basis_with_current_size() {
        let size = SizePx { w: 300, h: 500 };
        // Bottom: persisted (-110, -430) + 基準＝下端中央 (150, 500) = (40, 70)
        assert_eq!(
            balloon_offset_from_persist(Anchor::Bottom, PointPx { x: -110, y: -430 }, size),
            PointPx { x: 40, y: 70 }
        );
        // Right: persisted (-260, 70) + 基準 (300, 0) = (40, 70)
        assert_eq!(
            balloon_offset_from_persist(Anchor::Right, PointPx { x: -260, y: 70 }, size),
            PointPx { x: 40, y: 70 }
        );
    }

    /// design C1 の不変条件（Invariant）: 全アンカー × 複数寸法で往復恒等。
    /// `from_persist(a, to_persist(a, o, s), s) == o`。
    #[test]
    fn round_trip_identity_holds_for_all_anchors_and_sizes() {
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
                    let persisted = balloon_offset_to_persist(anchor, offset, size);
                    let restored = balloon_offset_from_persist(anchor, persisted, size);
                    assert_eq!(
                        restored, offset,
                        "往復恒等が破れた: anchor={anchor:?} size={size:?} offset={offset:?}"
                    );
                }
            }
        }
    }

    /// 8.5 サーフェス寸不変性: 高さ h1 で保存したオフセットを、異なる高さ h2 で復元しても
    /// バルーンとアンカー辺（下端）の相対関係が保たれる（＝下端からの距離が不変）。
    ///
    /// Bottom 吸着では、下端からの縦距離 = (char 下端) − (balloon 左上 y)
    ///   = (char.y + h) − (char.y + offset_tl.y) = h − offset_tl.y。
    /// persist 値の y はまさに `offset_tl.y − h`（= −(下端からの距離)）であり寸法非依存。
    /// よって別の高さ h2 で復元した offset_tl'.y は、その h2 に対して同じ「下端からの距離」を与える。
    #[test]
    fn balloon_offset_is_invariant_under_char_surface_height_change_bottom() {
        // 保存時: 高さ h1。ユーザーはバルーンを char 左上から下 70px の位置へ置いた。
        let h1 = 500;
        let size_h1 = SizePx { w: 300, h: h1 };
        let offset_tl_saved = PointPx { x: 40, y: 70 };
        // 保存時の「下端からの距離」（上向き正）: h1 − offset_tl.y。
        let distance_from_bottom_saved = h1 - offset_tl_saved.y; // 430

        let persisted = balloon_offset_to_persist(Anchor::Bottom, offset_tl_saved, size_h1);

        // 復元時: 異なる高さ h2（サーフェスが縮んだ／伸びた）。
        let h2 = 620;
        let size_h2 = SizePx { w: 300, h: h2 };
        let offset_tl_restored = balloon_offset_from_persist(Anchor::Bottom, persisted, size_h2);

        // 復元後の「下端からの距離」は保存時と一致する（＝下端基準の関係が保たれた）。
        let distance_from_bottom_restored = h2 - offset_tl_restored.y;
        assert_eq!(
            distance_from_bottom_restored, distance_from_bottom_saved,
            "下端からの距離がサーフェス高さ変動で保たれていない"
        );

        // 反例の檻: もし左上基準（persist=offset_tl そのまま）だったら、
        // 高さが変わっても左上距離を固定してしまい下端距離はずれる——それを排除する。
        let naive_topleft_restored = persisted.y; // 左上基準なら復元 y は persist.y のまま
        assert_ne!(
            h2 - naive_topleft_restored,
            distance_from_bottom_saved,
            "この寸法差では左上基準は下端距離を保てないはず（テスト前提の健全性）"
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
            anchor,
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

    /// 2.3: 保存 BalloonOffset 両軸あり → balloon_offset_from_persist で導出し balloon_pos へ反映。
    /// Free アンカーは基準 (0,0) ゆえ from_persist は identity（persisted＝左上基準 offset）。
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
            "Free 基準(0,0)ゆえ persisted＝左上基準 offset（2.3）"
        );
        assert_eq!(
            out[0].balloon_pos,
            PointPx { x: 770, y: 520 },
            "balloon_pos ≡ 最終 char_pos + 導出 offset"
        );
    }

    /// 2.2/2.3: Bottom アンカーの保存 BalloonOffset はアンカー辺（下端）基準で足し戻す
    /// （現 char_size で from_persist）。空 snapshot ゆえ char_pos は保存値素通し。
    #[test]
    fn apply_saved_balloon_offset_is_derived_bottom_basis() {
        let snap = snapshot_of(vec![]); // identity（char_pos は保存値のまま）
        let placements = vec![placement(
            0,
            Anchor::Bottom,
            PointPx { x: 100, y: 100 },
            CSZ, // h=600
            PointPx { x: -50, y: 0 },
            BSZ,
        )];
        // persisted (0, -430)（下端**中央**基準）→ from_persist(Bottom, (0,-430), 400x600)
        // = (0+200, -430+600) = (200, 170)（基準点＝(w/2, h)＝足元中央）。
        let entries = vec![
            wp(0, Axis::X, "800"),
            wp(0, Axis::Y, "500"),
            bo(0, Axis::X, "0"),
            bo(0, Axis::Y, "-430"),
        ];

        let out = apply_restored_placements(placements, &entries, &snap);

        // 保存 x=800 は**原点（下端中央）基準**ゆえ、現寸 w=400 の左上は 800−200=600。
        assert_eq!(out[0].char_pos, PointPx { x: 600, y: 500 });
        assert_eq!(
            out[0].balloon_offset,
            PointPx { x: 200, y: 170 },
            "Bottom 基準＝下端中央(w/2,h)で足し戻す: x=0+200・y=-430+600=170（2.2/2.3）"
        );
        assert_eq!(out[0].balloon_pos, PointPx { x: 800, y: 670 });
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
    // load_restored_state（唯一の IO 点・8.1 前哨・6.1/6.3 寛容縮退）
    // ------------------------------------------------------------------

    /// このテスト専用の一意な一時ディレクトリ（source.rs と同規約・外部 tempfile 非依存）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_placement_persist_tests_{tag}"));
        dir
    }

    /// 最小ゴーストパッケージ（ghost/master/descript.txt ＋ shell/master dir）を組む。
    /// resolve が成功する最小構成（source.rs の失敗経路テストと同型）。
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

    /// 起動時先読み: profile_areka_root(shiori.dir) に植えた sylphya.toml が
    /// `(PersistKey, String)` エントリとして読み戻る（load_scope 契約経由・8.1 前哨）。
    #[test]
    fn load_restored_state_reads_planted_sylphya_toml() {
        let root = unique_temp_dir("load_reads_planted");
        plant_minimal_ghost(&root);
        // profile root = <ghost/master>/profile/areka（boot 結線と同一構築）。
        let profile = profile_areka_root(&root.join("ghost").join("master"));
        std::fs::create_dir_all(&profile).expect("create profile/areka");
        std::fs::write(
            profile.join("sylphya.toml"),
            "format-version = 1\n[window.0]\nx = \"1486\"\ny = \"353\"\n[boot]\ncount = \"1\"\n"
                .as_bytes(),
        )
        .expect("plant sylphya.toml");

        let entries = load_restored_state(&root, DefaultEncoding::Ansi);

        assert!(
            entries.contains(&(
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X
                },
                "1486".to_string()
            )),
            "植えた WindowPos.x が読める: {entries:?}"
        );
        assert!(
            entries.contains(&(
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::Y
                },
                "353".to_string()
            )),
            "植えた WindowPos.y が読める: {entries:?}"
        );
        assert!(
            entries.contains(&(PersistKey::BootCount, "1".to_string())),
            "植えた BootCount が読める: {entries:?}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 起動時先読み: 永続ファイル不在（profile 未作成）→ 空（load_scope の不在縮退・6.1）。
    #[test]
    fn load_restored_state_absent_file_is_empty() {
        let root = unique_temp_dir("load_absent_file");
        plant_minimal_ghost(&root); // resolve は成功するが sylphya.toml は植えない
        let entries = load_restored_state(&root, DefaultEncoding::Ansi);
        assert!(entries.is_empty(), "永続ファイル不在は空縮退: {entries:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 起動時先読み: mount 解決失敗（ghost_root 不在）→ 空 Vec（寛容縮退・panic なし・6.1/6.3）。
    #[test]
    fn load_restored_state_mount_resolve_failure_is_empty() {
        let root = unique_temp_dir("load_mount_fail").join("no_such_ghost");
        let entries = load_restored_state(&root, DefaultEncoding::Ansi);
        assert!(
            entries.is_empty(),
            "mount 解決失敗は空縮退（起動を止めない）"
        );
    }

    // ------------------------------------------------------------------
    // PersistWiring / persist_entries（保存投函ヘルパ・design C1 State Management・
    //   task 2.1・Req1.1/1.9/6.2/7.1・Testing Strategy §1/§6）
    //   NonSend リソース存在 → persist_put（Ghost 固定・fire-and-forget）／不在 → debug!＋no-op。
    // ------------------------------------------------------------------

    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{SylphyaInit, spawn_sylphya};
    use std::sync::Arc;

    /// 共有 fake IO（アクターへ `Box<dyn PersistIo>` として移送しつつ、同一ストアを別ハンドルの
    /// `load_scope` で観測するための newtype ラッパ。actor.rs の write-through 檻と同流儀）。
    struct SharedFakeIo(Arc<FakePersistIo>);
    impl PersistIo for SharedFakeIo {
        fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

    /// 存在檻（7.1/1.1）: headless World に `PersistWiring` を挿入して `persist_entries` を呼ぶと、
    /// Ghost スコープへ write-through され、barrier 後に別ハンドルの `load_scope` で読み戻せる
    /// （＝`persist_put` が実際に投函され実 IO へ確定した証明）。
    #[test]
    fn persist_entries_with_wiring_write_through_to_ghost_scope() {
        // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す）。
        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(std::path::PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });

        // World へ NonSend リソースとして挿入（UI スレッド常駐・MouseWiring/Emo2Wiring 先例）。
        let mut world = World::new();
        world.insert_non_send_resource(PersistWiring {
            publisher: parts.publisher.clone(),
        });

        // char_pos_entries を persist_entries 経由で投函（Ghost 固定はヘルパ内で強制・7.1）。
        let entries = char_pos_entries(0, PointPx { x: 1486, y: 353 });
        persist_entries(&world, entries);

        // barrier 復帰 = 上記 put の write-through 保存（save_scope）まで完了（同一送信端 FIFO）。
        parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");

        // アクターと同一ストアを別ハンドルの load_scope で観測（実 IO 通過＝persist_put 投函の証明）。
        let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X
                },
                "1486".to_string()
            )),
            "persist_entries が Ghost へ write-through していない（WindowPos.x）: {loaded:?}"
        );
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::Y
                },
                "353".to_string()
            )),
            "persist_entries が Ghost へ write-through していない（WindowPos.y）: {loaded:?}"
        );

        // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
        parts.publisher.close();
        let _ = parts.handle.join();
    }

    /// 不在縮退檻（6.2/design「PersistWiring 不在は debug!＋no-op」）: `PersistWiring` 未挿入の
    /// World で `persist_entries` を呼んでも panic せず no-op で戻る（fallback boot 経路の縮退）。
    #[test]
    fn persist_entries_without_wiring_is_noop_and_never_panics() {
        let world = World::new(); // PersistWiring 未挿入（fallback 未挿入経路を模す）。
        // panic しない・no-op（debug ログのみ）。ここへ到達＝縮退成功。
        persist_entries(&world, char_pos_entries(0, PointPx { x: 1, y: 2 }));
        persist_entries(&world, balloon_offset_entries(0, PointPx { x: -400, y: 0 }));
    }
}
