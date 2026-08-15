//! キーワード由来のバルーン基本位置の一度きりの再導出
//! （[`rederive_keyword_balloon_offset`]・areka-P0-windowposition-limit 要件 4.2/4.3/4.7）。

use bevy_ecs::prelude::*;
use tracing::{info, warn};
use wintf::ecs::WindowPos;

use super::{
    BalloonFollow, BalloonKeywordBase, CharWindowMarker, PointPx, SizePx, keyword_balloon_pos,
};

// =============================================================================
// キーワード基本位置の一度きりの再導出
// （areka-P0-windowposition-limit 要件 4.2/4.3/4.7・2026-08-14 実機サインオフ是正）
// =============================================================================

/// 再導出が成立したときの記録タグ（実機ログの grep 判定語）。
pub(super) const KEYWORD_REDERIVE_TAG: &str = "[balloon-keyword] Rederive";
/// 再導出に必要な入力を解決できなかったときの記録タグ（log-first の縮退シーム）。
pub(super) const KEYWORD_UNRESOLVED_TAG: &str = "[balloon-keyword] Unresolved";

/// キーワード由来のバルーン基本位置を**実表示寸**から導出し直し、素材を消費する
/// （[`resize_window_to`] 手順 5a 専用・要件 4.2/4.3/4.4/4.7）。
///
/// # なぜ必要なのか（実機で確認された欠陥）
///
/// キーワードの中央揃えは `(char_w − balloon_w) / 2` であり、`char_w` が**採寸した寸**の
/// ままだと、実際に表示される寸と食い違ったぶんの半分だけバルーンが横へずれる。
/// 2026-08-14 の実機サインオフでは採寸 434 に対し実表示 382 で 26px ずれ、右端で
/// 作業領域を越えて limit の関門のクランプまで誘発した（見た目には大きく偏った）。
/// 決定論檻がこれを取り逃したのは、どの檻も**ひと組の寸法**で配置を組んでいたためである
/// ——「採寸寸 ≠ 実表示寸」という状態がどこにも作られていなかった。
///
/// # 一度きりであること（4.7）と、その発火条件
///
/// 要件 4.7 はキーワードの適用を「初期既定位置の供給」にとどめると定める。ゆえに
/// **キャラ窓の寸が最初に変わった一度だけ**発火して [`BalloonKeywordBase`] を消費し、
/// 以後の寸法変化では中央へ揃え直さない——`Side` と同じ配置時確定の静的 offset として
/// 振る舞う（4.4 の現行契約）。
///
/// 発火条件を route ではなく**寸の変化**に置いているのは、どの route が最初に実表示寸を
/// 運ぶかが frame の相順に依存するからである（`DpiReproject`／`ReportedSizeReconcile`／
/// `Resnap` はいずれも presenter が報告した実表示寸を運び得る。`AnchorChange` だけは
/// 現在の表示寸をそのまま渡す）。寸が変わらない書込では配置時の offset がその寸に対して
/// 正しいままなので、素材を残して次の機会へ譲る。
///
/// # 幾何は書き写さない
///
/// P5（`resolver::resolve_placement`）とまったく同じ [`keyword_balloon_pos`] を呼ぶ。
/// `char_pos` に原点 `(0,0)` を渡すと戻り値がそのままキャラ窓左上相対の offset になる
/// （`balloon_pos − char_pos` の定義そのもの）——確定位置を足してから引き直すより
/// 飽和演算の影響を受けず、`BalloonFollow.offset` の単位とも一致する。
///
/// # 縮退（log-first・要件 6.3 の流儀）
///
/// [`BalloonFollow`] 不在・バルーン entity 破棄済み・バルーンの現寸が未確定（`None` と
/// `CW_USEDEFAULT` センチネルの双方）のいずれでも [`KEYWORD_UNRESOLVED_TAG`] の `warn!` を
/// 残し、**offset にも素材にも触れない**（無ログの縮退経路を作らない・次の機会を潰さない）。
pub(super) fn rederive_keyword_balloon_offset(
    world: &mut World,
    char_window: Entity,
    old_size: Option<SizePx>,
    new_size: SizePx,
) {
    // 素材を持たない窓（`Side`・保存値が効いた scope・消費済み）は 1 bit も触らずに戻る。
    // 縮退ではないので警告も出さない（`BalloonLimit` の発動条件と同じデータ駆動の流儀）。
    let Some(base) = world.get::<BalloonKeywordBase>(char_window).copied() else {
        return;
    };

    // **寸が変わらない書込では消費しない**（発火条件は route ではなく寸の変化）。
    //
    // `resize_window_to` は複数の上流から呼ばれ、そのうち寸を伴わない再配置
    // （[`PlacementRoute::AnchorChange`]＝現在の表示寸で射影 T を引き直すだけ）も
    // 同じ口を通る。route で絞ると、どの route が最初に**実表示寸**を運ぶかという
    // 呼出順の知識を本関数が持つことになり、frame の相順が変わるたびに静かに壊れる。
    // 寸が変わらないうちは配置時の offset がその寸に対して正しいままなので、
    // 直すべきものが無い＝素材を残して次の機会へ譲るのが正しい。
    if old_size == Some(new_size) {
        return;
    }

    let Some(follow) = world.get::<BalloonFollow>(char_window).copied() else {
        warn!(
            entity = ?char_window,
            "{KEYWORD_UNRESOLVED_TAG} キーワード再導出: キャラ窓に BalloonFollow が無く追従先バルーンを特定できない → offset を変更しない"
        );
        return;
    };

    // scope は診断の結合キー（wintf 側ログは scope を持てない層＝この行でしか結べない）。
    // spawn は [`BalloonKeywordBase`] と [`CharWindowMarker`] を必ず一緒に付けるため
    // 欠落は構造的に起きないが、発明せず縮退する（runtime 関門の `BalloonWindowMarker`
    // 欠落の腕と同じ扱い——到達不能でも黙らない）。
    let Some(scope) = world.get::<CharWindowMarker>(char_window).map(|m| m.scope) else {
        warn!(
            entity = ?char_window,
            "{KEYWORD_UNRESOLVED_TAG} キーワード再導出: キャラ窓に CharWindowMarker が無く scope を特定できない → offset を変更しない"
        );
        return;
    };

    // 判定寸＝バルーンの現寸。未確定表現は `Option::None` **だけではない**——
    // `WindowPos::default()` は `CW_USEDEFAULT`（`i32::MIN` センチネル）を寸に持つ
    // （`guard_balloon_position`／runtime 関門と同型のガード）。entity 破棄済みも
    // `world.get` が `None` を返してこの腕へ合流する。
    let balloon_size = world
        .get::<WindowPos>(follow.balloon)
        .and_then(|wp| wp.size)
        .filter(|s| s.width > 0 && s.height > 0)
        .map(|s| SizePx {
            w: s.width,
            h: s.height,
        });
    let Some(balloon_size) = balloon_size else {
        warn!(
            entity = ?char_window,
            balloon = ?follow.balloon,
            mode = ?base.mode,
            "{KEYWORD_UNRESOLVED_TAG} キーワード再導出: バルーンの寸が未確定（窓生成前／CW_USEDEFAULT センチネル／破棄済み）のため基本位置を導出できない → offset を変更しない"
        );
        return;
    };

    // P5 と同一の式（原点 (0,0) 起点＝戻り値がそのまま char 左上相対 offset）。
    // `None` は `Side` のときだけであり、その scope は素材を持たないので到達しない。
    let Some(new_offset) = keyword_balloon_pos(
        base.mode,
        PointPx { x: 0, y: 0 },
        new_size,
        balloon_size,
        base.adjust,
    ) else {
        warn!(
            entity = ?char_window,
            mode = ?base.mode,
            "{KEYWORD_UNRESOLVED_TAG} キーワード再導出: 基本位置を持たないモードの素材が付いている（構造的に起きないはず） → offset を変更しない"
        );
        return;
    };

    let old_offset = follow.offset;
    if let Some(mut f) = world.get_mut::<BalloonFollow>(char_window) {
        f.offset = new_offset;
    }
    // 一度きり（4.7）: 素材を消費して除去する。これ以降のリサイズは `Side` と同じく
    // 配置時確定の静的 offset として振る舞う。
    world.entity_mut(char_window).remove::<BalloonKeywordBase>();

    info!(
        scope,
        entity = ?char_window,
        balloon = ?follow.balloon,
        mode = ?base.mode,
        adjust = ?base.adjust,
        old_offset = ?old_offset,
        new_offset = ?new_offset,
        old_char_size = ?old_size,
        new_char_size = ?new_size,
        balloon_size = ?balloon_size,
        "{KEYWORD_REDERIVE_TAG} 実表示寸が確定したのでキーワード由来のバルーン基本位置を一度だけ導出し直した（以後は静的 offset）"
    );
}

