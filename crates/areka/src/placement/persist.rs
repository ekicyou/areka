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
//! 再射影・`PersistWiring` は後続タスクで本モジュールへ追加する。
//!
//! バルーン相対オフセットの基準変換（アンカー辺基準）は 2026-07-31 の実機裁定で撤去した
//! ——保存基準はランタイム（[`super::follow::BalloonFollow`]`.offset`）と同じ **char 窓左上**
//! であり、変換は不要（[`balloon_offset_entries`] 参照）。キャラ窓位置の原点符号化
//! （[`char_pos_to_origin_x`]＝下端中央）は別の関心事ゆえ従来どおり維持する。

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
/// （windowposition-limit DD6）。`main.rs` の `restore_merged_placements` は本関数の直後に起動時関門
/// [`super::balloon_limit::apply_balloon_limit`] を通し、`balloon_limit` が真の scope の
/// `balloon_pos` だけを作業領域内へ補正する（`balloon_offset` は生値のまま＝補正を
/// 焼き付けない）。ゆえに関門通過後の `balloon_pos` は表示位置、`balloon_offset` は論理
/// 相対位置であり、両者の差が恒等式を満たすとは限らない。本関数自身はクランプしない。
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
    // キーワード再導出の素材は**保存値が効いた scope では落とす**（windowposition-limit
    // 要件 4.7「永続値を優先し、キーワード指定の適用は初期既定位置の供給にとどめる」）。
    // 落とさないと、実表示寸確定の再導出がユーザーの保存した相対位置をキーワード既定へ
    // 上書きしてしまう——保存値優先の順位が静かに反転する。offset 欠損側（resolver 既定を
    // 保持する腕）は素材をそのまま運ぶ。
    let (balloon_offset, balloon_keyword_base) = match (saved_bx, saved_by) {
        // 保存 offset は**char 左上基準**（ランタイム BalloonFollow.offset と同一基準）ゆえ
        // 基準変換なしで採用する（2.3・[`balloon_offset_entries`] の基準記述）。
        (Some(x), Some(y)) => (PointPx { x, y }, None),
        // offset 欠損 → resolver 既定 offset（左上基準）を保持（2.4）。
        _ => (placement.balloon_offset, placement.balloon_keyword_base),
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
        // 基準対は本タスク（areka-P0-balloon-offset-dpi task 2.1）では**素通し**する
        // ——保存値採用腕を未係留（`dpi: None`）にするのは後続タスクの責務であり、
        // ここで先取りすると採用腕の意味論が 2 か所に分かれる。
        balloon_offset_base: placement.balloon_offset_base,
        // limit の解決値は merge の対象外（永続化しない・毎起動 descript から解決する）
        // ゆえ、入力の値をそのまま転記する（merge 規則は無改変）。
        balloon_limit: placement.balloon_limit,
        anchor: placement.anchor,
        // 保存値が効いた scope では `None`（要件 4.7）。上の match が決めている。
        balloon_keyword_base,
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
    let Some(wiring) = world.get_non_send::<PersistWiring>() else {
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
#[path = "persist_entries_tests.rs"]
mod entries_tests;
#[cfg(test)]
#[path = "persist_io_wiring_tests.rs"]
mod io_wiring_tests;
#[cfg(test)]
#[path = "persist_restore_tests.rs"]
mod restore_tests;
