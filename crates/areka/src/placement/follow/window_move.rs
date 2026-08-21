//! 窓移動・リサイズ API と反映（[`move_window_to`]・[`resize_window_to`]・[`anchor_changed_system`]・[`resize_window_keep_position`]）。

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use tracing::{debug, info, warn};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::{DPI, Point, SetWindowPosCommand, SizeI, WindowHandle, WindowPos};

use super::{
    Anchor, Anchored, BALLOON_LIMIT_CLAMP_TAG, BALLOON_LIMIT_RUNTIME_CONTEXT,
    BALLOON_LIMIT_UNRESOLVED_TAG, BalloonFollow, BalloonFollowTrigger, BalloonLimit,
    BalloonWindowMarker, CharWindowMarker, DESPAWNED_SKIP_TAG, DpiSyncHold, GhostWindows,
    MonitorSnapshot, PlacementRoute, PointPx, RectPx, SizePx, WindowKind, WindowMoveRecord,
    apply_visibility_guard, diag, follow_balloon, limit_correction, project_anchor, rect_at,
    rederive_keyword_balloon_offset, transition_diag, work_area_for_window,
};

/// R7 公開 API: UI スレッド上で呼ばれる窓移動関数（物理 px・スクリーン座標直渡し・7.1）。
///
/// - 移動は `SetWindowPosCommand`（`SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE`）経由。
///   座標は物理 px 素通し（U4・再スケールなし）
/// - 対象が [`BalloonFollow`] を持つ場合はバルーン窓も offset 維持で随伴移動する
/// - `WindowHandle` 未付与（窓生成前）は `warn!` して `false` を返す（silent no-op に
///   しない）。対象が既に破棄済み（entity 不在）なら**正常終了系**ゆえ `debug!` で
///   打ち切って `false`（task 7.3・Req 6.2）。いずれも随伴バルーンは動かさない
/// - 随伴バルーン側の `WindowHandle` 未付与は `warn!` のみ（対象自身の移動は成立
///   しているため戻り値は `true`）
///
/// 消費者: `emo2_boot::move_cue::apply_move_directive`（`\![move]` の UI スレッド適用・
/// task 7.4）が唯一の位置ライターとして本 API を呼ぶ。
///
/// # 経路タグ（Req 1.2／2.4・D13・task 1.4）
///
/// 対象窓の書込は [`PlacementRoute::MoveCue`]・随伴バルーンは
/// [`PlacementRoute::BalloonFollow`] として記録する。唯一の消費者が `\![move]` cue である
/// ため route は引数で受けない（呼出側が渡せる値が 1 つしか無く、取り違えの余地だけを
/// 増やす＝`resize_window_keep_position` と同じ判断）。**スクリプトの明示操作**ゆえ
/// 遷移ガードは適用しない（ドラッグ・`Restore` と同族・D13 帰結⑵）。
pub fn move_window_to(world: &mut World, window: Entity, x: i32, y: i32) -> bool {
    move_window_with_route(world, window, x, y, PlacementRoute::MoveCue)
}

/// [`move_window_to`] の経路引数版（対象窓の書込 route だけを呼び手が名乗る）。
///
/// 幾何の扱いは [`move_window_to`] と**完全に同一**である——変わるのは対象窓の書込へ載る
/// [`PlacementRoute`] だけで、随伴バルーンは従来どおり [`PlacementRoute::BalloonFollow`] を
/// 名乗る（随伴はキャラ窓の従属量であって、キャラ窓を動かした由来を名乗る主体ではない）。
///
/// # なぜ経路を引数で受けるか（D13 の 1 語＝1 実在トリガ）
///
/// 「寸を変えずに位置だけを書く」手続きの呼び手は 2 つある——`\![move]` cue（明示操作＝
/// [`PlacementRoute::MoveCue`]）と、DPI 遷移後の連鎖再解決（システム由来＝
/// [`PlacementRoute::ChainRealign`]・`areka-P0-dpi-transition-atomicity` C4）である。
/// 由来が違えば**挙動も違う**: 既定位置の追跡（[`track_default_char_pos`]・D9／D16）は
/// システム由来の書込にだけ効くので、遷移後の解き直しを `MoveCue` で書くと既定位置が
/// 置き去りになり、次の遷移で当該スコープが「明示的に動かされた」へ倒れる。
pub fn move_window_with_route(
    world: &mut World,
    window: Entity,
    x: i32,
    y: i32,
    route: PlacementRoute,
) -> bool {
    let follow = world.get::<BalloonFollow>(window).copied();

    if !enqueue_window_set_pos(world, window, x, y, None, Some(route)) {
        return false;
    }

    if let Some(follow) = follow {
        debug_assert!(
            x.checked_add(follow.offset.x).is_some() && y.checked_add(follow.offset.y).is_some(),
            "move target out of virtual-screen range: ({x},{y}) + {:?}",
            follow.offset
        );
        // バルーン側の失敗（WindowHandle 未付与等）は enqueue_window_set_pos が
        // warn! 済み。対象自身の移動は成立しているため true のまま返す。
        enqueue_window_set_pos(
            world,
            follow.balloon,
            x + follow.offset.x,
            y + follow.offset.y,
            None,
            Some(PlacementRoute::BalloonFollow),
        );
    }

    true
}

/// 単一ライター反映口: 新しい表示サーフェス寸法に対しアンカー射影 T を再適用し、
/// 確定した position＋size を単一ライター経路で**一度だけ**書く（task 2.4・
/// Req1.1/1.3/1.7/3.1/3.4＋2.6/3.3）。
///
/// `char_window` の現在アンカー（[`Anchored`]＝単一真実源）を読み、[`project_anchor`]
/// で新 position を導出する——ドラッグ（[`policy_mapped_position`]）と**同一の T** を
/// 呼び、座標系変換を二重化しない（Req1.6）。`bottom` は `wa.bottom − h'` の再計算、
/// `snapshot` 不在は `project_anchor` が identity 縮退する。
///
/// # 縮退・失敗経路（log-first・silent failure を作らない）
///
/// - **対象 entity 不在（despawn 済み）**: 終了処理でゴースト窓が破棄された後のフレーム
///   ＝**正常終了系**ゆえ `debug!`（[`diag::DESPAWNED_SKIP_TAG`]）＋`false`（要件 6.2/6.3）。
///   直下の [`Anchored`] 欠落 `warn!` と**必ず区別する**——混ぜると終了時ログが良性ノイズで
///   埋まり、本物の結線バグ（実在窓の `Anchored` 欠落）が読めなくなる。
/// - [`Anchored`] 欠落: char 窓は spawn で必ず付与される＝異常系ゆえ `warn!`＋`false`。
/// - 非正寸（w≤0 or h≤0）: T を再適用せず現状保持＋`warn!`＋`false`（Req3.4・
///   [`BottomSnapPolicy`] の非正寸縮退と整合）。
/// - `WindowPos`／`WindowPos.position` 不在（窓生成前の異常系）: 生位置 `raw` を
///   導出できないため `warn!`＋`false`（panic しない）。
/// - べき等（Req3.1）: 導出 `(position, size)` が現 `WindowPos` と同一なら書込を行わず
///   `false`（冗長な再配置を避ける・正常系ゆえ `debug!`）。
/// - `WindowHandle` 未付与/対象不在: [`enqueue_window_set_pos`] が `warn!`＋`false`
///   （Req3.3）。このとき随伴バルーンも動かさない（[`move_window_to`] と同じ流儀）。
///
/// # 不変条件（Req1.5/1.7）
///
/// 位置・サイズは [`enqueue_window_set_pos`]（`Some(new_size)`）で 1 コマンドだけ
/// 発行する——`enqueue_window_move` を迂回する新たな bypass 書込を新設せず、単一
/// ライター規律（bypass ミラー＋Arrangement 同期）を継承する。反映段階で既に確定
/// 座標のみを書くため、切替・アンカー変更で窓が振動しない。書込成功後は
/// [`follow_balloon`] が [`BalloonFollow.offset`] を保って随伴させる（Req2.6・
/// 恒等式 `balloon_pos − char_pos ≡ offset` 維持）——ただしこの恒等式が保たれるのは
/// [`BalloonFollow.offset`] と**追従計算が出す提案位置**についてである。対象が
/// `BalloonLimit(true)` なら [`enqueue_window_set_pos`] の runtime 関門が表示位置だけを
/// 作業領域内へ補正するため、実際に書き込まれた位置と `char_pos` の差が `offset` と
/// 一致しないことがある（windowposition-limit DD6・`offset` は補正を焼き付けず生値の
/// まま）。
///
/// # `route` 引数（Req 1.2／2.4・design「PlacementRoute 配管＋guard_visibility >
/// Integration」・task 1.4）
///
/// 本関数は複数の上流（[`anchor_changed_system`]＝[`PlacementRoute::AnchorChange`]・
/// frame の毎フレーム再スナップ＝[`PlacementRoute::Resnap`]・frame の DPI 相＝
/// [`PlacementRoute::DpiReproject`]・frame の drain 後段（寸法報告回収・`Changed<DPI>` 非依存）＝
/// [`PlacementRoute::ReportedSizeReconcile`]・D13）から呼ばれる**同一の反映口**であり、どの経路が
/// 書いたかは呼出側しか知らない。ゆえに経路は引数で受け、[`enqueue_window_set_pos`] の
/// 窓移動レコードへ透過させる（D11: ラッパ関数を乱立させない）。
///
/// **task 6.1 以後、route は観測語彙であると同時に挙動の入力でもある**——可視性の遷移
/// ガード（手順 3c・[`apply_visibility_guard`]）は非ドラッグの配置系 4 経路
/// （[`AnchorChange`](PlacementRoute::AnchorChange)／[`Resnap`](PlacementRoute::Resnap)／
/// [`DpiReproject`](PlacementRoute::DpiReproject)／
/// [`ReportedSizeReconcile`](PlacementRoute::ReportedSizeReconcile)）でのみ発火する
/// （D13 帰結⑴。同じ幾何でも由来が明示操作なら引き戻さない＝Req 3.1 の「ユーザーの
/// 明示的なドラッグ以外の要因」）。
///
/// **task 6.2 以後、route は随伴バルーンの発火条件でもある**——手順 7 の
/// [`follow_balloon`] へ [`BalloonFollowTrigger::Placement`]`(route)` として渡され、
/// バルーン矩形への遷移ガード（S3′・Req 3.4）が同じ 4 経路でのみ発火する。
///
/// # バルーン追従はリサイズで補正しない（**全アンカー共通**・2026-07-31 実機裁定）
///
/// 本関数は `BalloonFollow.offset` を**一切書き換えない**。以前は Bottom に限って
/// 「原点＝下端中央からの相対を保つ」ため offset に `((w'/2−w/2), (h'−h))` を加算して
/// おり、上記の恒等式記述と矛盾していた（Bottom だけ窓相対でない＝内部分裂）。
/// 受理オラクルは参照実装 SSP の実測——SSP のバルーンは**観測時つねに現在表示中の**
/// キャラ窓に対して窓相対 (−168,−161) にある（実 DPI 120・むらさき 478×684 表示時）。
/// 一方 補正ありの areka は boot 採寸窓（543×859）を基準に置いたきり据え置き、
/// 切替後に 336px 上空へ浮かせていた。補正を撤去して
/// `areka-P0-surface-resize-resnap` Req2.6 の窓相対契約を復元し、矛盾を解消した。
/// 檻: `resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset`。
///
/// # 唯一の例外＝キーワード基本位置の一度きりの再導出（windowposition-limit 4.7）
///
/// 上の「offset を一切書き換えない」には例外が 1 つある。`windowposition.x` がキーワード
/// （`center`／`top`／`bottom`）で、かつ保存された相対位置が効いていない scope は、キャラ窓へ
/// [`BalloonKeywordBase`] を持って spawn される。この素材がある間の**最初の**リサイズだけは
/// [`rederive_keyword_balloon_offset`] が offset をキーワード式で導出し直し、素材を除去する。
/// 連続的な中央追従ではない——初期既定位置を**実際に表示される寸**から導くための一度きりの
/// 確定である（詳細は [`BalloonKeywordBase`] の doc）。素材はリサイズを待たずに消えることも
/// ある——利用者がバルーンをドラッグして相対位置を保存した瞬間に退役するからである
/// （要件 4.7・`drag_follow::retire_keyword_base_on_save`）。
// 本体では結線済み（`emo2_boot::frame::dpi`／`emo2_boot::frame::drain_resnap` が呼ぶ）。
// allow が要るのは examples が `placement/mod.rs` を `#[path]` include して本体を伴わずに
// ビルドする形があるためで、そちらでは本関数に到達する呼出が存在しない
// （[`resize_window_keep_position`] の allow と同じ事情）。
#[allow(dead_code)] // examples が #[path] include するため、本体未使用ビルドでも必要
pub fn resize_window_to(
    world: &mut World,
    char_window: Entity,
    new_size: SizePx,
    route: PlacementRoute,
) -> bool {
    // 0. 存在確認（要件 6.2/6.3・design D8 消費側）: 対象が既に despawn 済みなら
    //    **正常終了系**として debug で打ち切る。終了処理でゴースト窓が破棄された後の
    //    フレームでも寸法の再導出は走り得るため、ここを素通りさせると下の
    //    「Anchored 未付与」warn が破棄済み窓ぶんだけ鳴り、良性ノイズが本物の異常を
    //    埋める。区別すべきは水準であって、打ち切ること自体ではない——
    //    **実在する** entity の `Anchored` 欠落は下で従来どおり warn のままにする。
    if world.get_entity(char_window).is_err() {
        debug!(
            entity = ?char_window,
            ?route,
            "{DESPAWNED_SKIP_TAG} 対象 entity は既に破棄済み（despawn）→ アンカー保存リサイズを正常系として打ち切り"
        );
        return false;
    }

    // 1. Anchored（drag／resize が読む単一真実源）を読む。char 窓は spawn で必ず
    //    付与される＝欠落は異常系ゆえ log-first で no-op（silent failure にしない）。
    let Some(Anchored(anchor)) = world.get::<Anchored>(char_window).copied() else {
        warn!(
            entity = ?char_window,
            "Anchored 未付与（char 窓は spawn で必ず付与）のため resize しない"
        );
        return false;
    };

    // 2. 非正寸ガード（Req3.4）: T を再適用せず現状保持。wa.right−w／wa.bottom−h の
    //    暴走を先に弾く（BottomSnapPolicy の CW_USEDEFAULT センチネル縮退と整合）。
    if new_size.w <= 0 || new_size.h <= 0 {
        warn!(
            entity = ?char_window,
            ?new_size,
            "新しいサーフェス寸法が非正のため T 再適用せず現状保持"
        );
        return false;
    }

    // 3. 現在位置（生位置 raw）と現寸を読む。WindowPos／position 不在（窓生成前の
    //    異常系）は raw を作れないため安全に no-op（panic しない・log-first）。
    let (raw, current_size) = {
        let Some(wp) = world.get::<WindowPos>(char_window) else {
            warn!(
                entity = ?char_window,
                "WindowPos 未付与（窓生成前）のため raw を導出できず resize しない"
            );
            return false;
        };
        let Some(pos) = wp.position else {
            warn!(
                entity = ?char_window,
                "WindowPos.position 不在（窓生成前）のため raw を導出できず resize しない"
            );
            return false;
        };
        // wintf の未確定表現は `Option::None` **だけではない**（D15・task 6.3）:
        // `WindowPos::default()` は position を `CW_USEDEFAULT`（`i32::MIN` センチネル）で
        // 持ち、`on_window_add` フックがそれを実際に挿す。素通しすると
        //   ① 手順 3a の `old_rect` が `i32::MIN` 近傍の全 work area 非交差矩形になり、
        //      `guard_visibility` が「もともと画面外に留置されていた」と誤読して `Keep`
        //      ＝**6.1 が敷いた安全側 clamp の腕が黙って死ぬ**
        //   ② 手順 3b の中央付替えと射影 T の入力（raw）も同時に汚染され、位置権威の
        //      無い窓へ clamp 由来の任意 X を書く＝位置権威の僭称
        // 位置未確定は「保存すべき接地点が存在しない」状態ゆえ、上の `Option::None` と
        // 同じ腕へ合流させて打ち切る（Req 3.3「必要な入力が取得できない場合は現状維持＋
        // 警告」）。**寸センチネルとの非対称は意図的**（D15 帰結⑴）——寸未確定は接地点が
        // 実在するので resize に意味があり、手順 3a の `old_rect` 不明＝安全側 clamp で扱う。
        //
        // 判定は**センチネル一致**で行う（wintf 正典 `window_pos.rs:41`／
        // `monitor_systems.rs:408` と同型——正典が見るのは `position.x` と `size.width`
        // で、ここは位置の両軸を見る点だけが異なる）。負座標そのものは正当（実機の左隣
        // モニタは負の X を持つ）ゆえ、符号や大きさの閾値で判定してはならない。
        if pos.x == CW_USEDEFAULT || pos.y == CW_USEDEFAULT {
            warn!(
                entity = ?char_window,
                position = ?pos,
                "WindowPos.position がセンチネル（位置未確定）＝窓生成前のため raw を導出できず resize しない"
            );
            return false;
        }
        (PointPx { x: pos.x, y: pos.y }, wp.size)
    };

    // 3a. 旧矩形（書込**前**の窓矩形）＝遷移ガードが「もともと見えていたか」を判定する入力
    //     （task 6.1・S3 是正）。手順 3b の付替えより**前**の生位置で組むこと——3b 後の
    //     値は「これから書こうとしている位置」であって旧矩形ではない。
    //
    //     寸が未確定のときは `None`＝**旧矩形不明**として扱い、ガードを安全側 clamp へ
    //     倒す。ここで注意すべきは、wintf の未確定表現が `Option::None` **だけではない**
    //     ことである——`WindowPos::default()` は `Some(SizeI { CW_USEDEFAULT, .. })`
    //     （`i32::MIN` センチネル）を持つ（`wintf::ecs::window::window_pos::WindowPos`
    //     の `Default`）。素の矩形として交差判定へ入れると `saturating_add` で逆転矩形に
    //     なり、「もともと画面外に留置されていた」と誤判定して尊重側（Keep）へ倒れる
    //     ＝安全側の腕が丸ごと死ぬ（4.6 で同型の見落としが本番 panic を新設した教訓）。
    let old_rect = match current_size {
        Some(s) if s.width > 0 && s.height > 0 => Some(rect_at(
            raw,
            SizePx {
                w: s.width,
                h: s.height,
            },
        )),
        _ => None,
    };

    // 3b. 原点＝**下端中央**の保存（伺かの立ち絵は足元中央が接地点・寸法変動で原点は動かない）。
    //     旧寸の中央 x を求め、新寸でも同じ中央になる左上 x へ付け替えてから射影へ渡す。
    //     これをしないと左上 x が据え置かれ、幅が変わるたびキャラの見た目の中心が横へ動き
    //     （実機: むらさきが surface0 434 → surface1000 382 で中心が 26px ずれる）、
    //     随伴バルーンも一緒に引きずられる。旧寸不明（窓生成直後等）は付け替えない。
    //     対象は**下端吸着（Bottom）のみ**——Free は「位置を一切動かさない」契約、
    //     Top/Left/Right は各アンカー辺（上端・左端・右端）が原点であって中央ではない。
    let raw = match (anchor, current_size) {
        (Anchor::Bottom, Some(old)) if old.width > 0 && new_size.w > 0 => {
            let center_x = raw.x.saturating_add(old.width / 2);
            PointPx {
                x: center_x.saturating_sub(new_size.w / 2),
                y: raw.y,
            }
        }
        _ => raw,
    };

    // 新位置 = アンカー射影 T（bottom は wa.bottom−h' 再計算・snapshot 不在は
    // project_anchor が identity 縮退）。drag と同一 T を呼び二重化しない（Req1.6）。
    // 下端は T が再導出し、中央 x は上の付け替えで維持される＝原点（下端中央）が不動。
    let snapshot = world.get_resource::<MonitorSnapshot>();
    let new_pos = project_anchor(anchor, raw, new_size, snapshot);

    // 3c. 可視性の遷移ガード（S3 是正・task 6.1・D5/D6・Req 3.1/3.2/3.3）: 射影 T の
    //     **下流・外側**で、非ドラッグの配置系 route のときだけ「可視 → 全 work area
    //     非交差」の遷移を X の clamp で阻止する（Y は射影の所有ゆえ不変）。射影関数
    //     自体の契約は変えない——`project_anchor` は幾何しか知らず、発火条件である
    //     route（＝書込の由来）を持たないためここでしか判定できない。
    //     べき等 skip より**前**に置くのは、clamp 結果が現在値と一致した走行で冗長な
    //     書込を出さないため（design Integration の手順どおり）。
    let new_pos = apply_visibility_guard(
        char_window,
        route,
        snapshot,
        old_rect,
        raw,
        new_pos,
        new_size,
    );

    // 4. べき等 skip（Req3.1）: 導出 (position, size) が現 WindowPos と同一なら書かない
    //    （冗長な再配置を避ける・こちらは正常系ゆえ debug!）。
    if new_pos == raw && current_size == Some(SizeI::new(new_size.w, new_size.h)) {
        debug!(
            entity = ?char_window,
            ?new_pos,
            ?new_size,
            "導出 position/size が現在値と同一のため書込をスキップ（べき等）"
        );
        return false;
    }

    // 5. 一度書き（Req1.5/1.7）: 位置＋サイズを単一ライター経路で 1 コマンド発行。
    //    WindowHandle 未付与/不在は enqueue が warn!＋false（Req3.3）——false なら
    //    随伴バルーンも動かさず false を返す（move_window_to と同じ流儀）。
    if !enqueue_window_set_pos(
        world,
        char_window,
        new_pos.x,
        new_pos.y,
        Some(new_size),
        Some(route),
    ) {
        return false;
    }

    // 5b. 接地点の観測（要件 5.3・design Data Models `ground`）: **下端吸着のキャラ窓に限り**、
    //     いま書いた接地点（窓矩形の下端）と実行時のモニタ表が持つ作業領域下端の差を 1 行残す。
    //     接地点そのものは上の射影 T が決めた値を読むだけで、位置の決め方には触れない
    //     （要件 10.1 の原点規約は不変）。ガードを組立の外側に置くのは、既定運転で
    //     モニタ表の走査も `String` の確保も一切行わないためである（要件 10.4/10.6）。
    //     手順 5a より前に置くのは、5a が offset を書き換える前の**書いた通りの**接地点を
    //     残すため（5a はバルーン側の量であって接地点を動かさないが、順序の意図を明示する）。
    if matches!(anchor, Anchor::Bottom) && transition_diag::is_enabled() {
        transition_diag::log_char_ground(world, char_window, new_pos, new_size, route);
    }

    // 5a. キーワード由来のバルーン基本位置の**一度きり**の再導出
    //     （windowposition-limit 要件 4.2/4.3/4.7・2026-08-14 実機サインオフ是正）。
    //     手順 6 の追従より**前**に置く——offset を直してから追従させないと、
    //     このフレームのバルーンは古い offset で書かれ、次に何かが動くまで直らない。
    //     `BalloonKeywordBase` を持つキャラ窓（＝キーワード指定かつ保存値が効いていない
    //     scope）だけが対象で、`Side` と保存値スコープは構造的に素通しする。
    //     `current_size` は書込**前**に読んだ採寸寸（手順 5 の bypass ミラーで
    //     `WindowPos.size` は既に新寸へ変わっているため、ここで読み直してはならない）。
    rederive_keyword_balloon_offset(
        world,
        char_window,
        current_size.map(|s| SizePx {
            w: s.width,
            h: s.height,
        }),
        new_size,
    );

    // 6. 随伴バルーン維持（Req2.6）: **リサイズで `BalloonFollow.offset` を補正しない**。
    //    受理オラクルは参照実装 SSP の実測——SSP のバルーンは観測時つねに現在表示中の
    //    キャラ窓に対して窓相対にある（2026-07-31 実機裁定）。これは
    //    `areka-P0-surface-resize-resnap` Req2.6 の「追従 offset を維持」という窓相対契約
    //    そのものであり、全アンカーで恒等式 `balloon_pos − char_pos ≡ offset` が成立する
    //    ——ただしこれは**追従計算が出す提案位置**についての話である。対象が
    //    `BalloonLimit(true)` なら [`enqueue_window_set_pos`] の runtime 関門が表示位置
    //    だけを作業領域内へ補正するため、書き込まれた位置との差は offset と一致しない
    //    ことがある（windowposition-limit DD6・`offset` は補正を焼き付けず生値のまま）。
    // 確定後キャラ窓座標＋（不変の）offset で追従（提案位置は offset 恒等式維持・
    //    表示位置は上記の関門で補正され得る）。
    //    引き金はキャラ窓を動かした route そのもの——バルーン矩形への遷移ガード
    //    （task 6.2・S3′）はこれで発火可否が決まる（書込自身の `BalloonFollow` では
    //    決められない・[`BalloonFollowTrigger`] の doc 参照）。
    follow_balloon(
        world,
        char_window,
        Point {
            x: new_pos.x,
            y: new_pos.y,
        },
        BalloonFollowTrigger::Placement(route),
    );

    true
}

/// アンカー変化トリガ（Req1.4・consumer 契約のみ・producer=seriko は非所有）。
///
/// `Changed<Anchored>` の char 窓それぞれについて、**現在の表示寸法**
/// （`WindowPos.size`＝新寸ではなく今まさに表示している寸法）を読み、
/// [`resize_window_to`] を呼んで**新しいアンカー**（変化後の [`Anchored`]）に対応する
/// 射影 T を再適用する——新しいアンカー辺を work area の対応辺へ合わせる。
/// アンカー値の解決（`seriko.alignmenttodesktop` 優先度チェーン）と
/// `\![set,alignmenttodesktop]` の cue routing は上流（parsers／seriko）の領分＝
/// 本 spec は非所有。本 system は `Anchored` の変化に反応する **consumer** に徹する
/// （design「System Flows > アンカー変化トリガ」・Req4.2）。schedule への登録（結線）は
/// main.rs／runtime 側の領分ゆえ、本 task は system の**定義のみ**を持つ
/// （`create_windows` が window_system.rs で定義のみ・登録は runtime、という repo 慣行）。
///
/// # 変更検知の永続性（毎フレーム全マッチしない・Req1.4 の要）
///
/// [`SystemState`] を [`Local`] に保持して `last_run` tick を run を跨いで引き継ぐ。
/// 毎 run で新規 `QueryState` を作ると `last_run` が過去 0 のまま `Changed` が全窓へ
/// 誤マッチするため不可。[`SystemState::get`] は fetch 後に `last_run` を進めるので、
/// 次 run 以降は「前回 run 以後に `Anchored` が変わった窓」だけがマッチする。
/// 初回 run は `SystemState::new` が `last_run` を過去へ置く仕様で全 char 窓が
/// マッチし得るが、[`resize_window_to`] が同寸・同位置でべき等 skip して吸収する
/// （design Implementation Notes）。
///
/// # 縮退（panic しない・log-first）
///
/// `WindowPos.size` 不在／未生成（窓生成前）の窓は skip する（現寸を導出できない）。
/// アンカー欠落・非正寸・`WindowHandle` 未付与など残りの縮退は [`resize_window_to`]
/// 側が warn＋no-op で吸収する（Req3.3/3.4・二重に弾かない）。
///
/// # Concurrency
///
/// UI スレッド・World 排他（`&mut World`）。他 actor は触れない（design State Management）。
#[allow(dead_code)] // schedule 登録（結線）は main.rs／runtime 側の領分（本 task は定義のみ）
pub fn anchor_changed_system(
    world: &mut World,
    mut state: Local<Option<SystemState<Query<'static, 'static, Entity, Changed<Anchored>>>>>,
) {
    // Changed<Anchored> 検知の永続シーム: SystemState を跨 run で使い回し last_run を保つ。
    let state = state.get_or_insert_with(|| SystemState::new(world));
    // 変更窓を collect して borrow を即解放してから &mut World ループへ（先例
    // create_windows の collect→release→&mut World ループを Changed 対応にしたもの）。
    let changed: Vec<Entity> = state.get(world).iter().collect();
    for entity in changed {
        // 現在の表示寸法（新寸ではない）を読む。size 不在／未生成は skip（panic しない）。
        let Some(size) = world.get::<WindowPos>(entity).and_then(|wp| wp.size) else {
            continue;
        };
        // 現寸で resize_window_to → 現在アンカー（変化後）で project_anchor を再適用。
        // 経路タグは AnchorChange（Req 1.2 の「変化を引き起こした経路」・task 1.4）。
        resize_window_to(
            world,
            entity,
            SizePx {
                w: size.width,
                h: size.height,
            },
            PlacementRoute::AnchorChange,
        );
    }
}

/// 1 窓ぶんの位置（と任意で寸法）を enqueue する共通経路（物理 px 素通し・
/// 単一ライター・task 2.3 で move 専用から size 対応へ一般化）。
///
/// `WindowHandle` を直接引いて `SetWindowPosCommand` を enqueue し、ECS 側の
/// `WindowPos.position` を `bypass_change_detection()` で先行反映する。
///
/// # size 引数（`None`＝移動専用の後方互換／`Some`＝位置＋寸を一度に反映）
///
/// - `None`: 移動専用。flags は `SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE`・
///   width/height=0 で、`WindowPos.position` のみ bypass ミラーし `WindowPos.size`
///   は**触らない**（`move_window_to`／drag 経路の従来挙動と完全に同一）。
/// - `Some(s)`: flags から `SWP_NOSIZE` を外し（＝`SWP_NOZORDER | SWP_NOACTIVATE`）、
///   `width=s.w`／`height=s.h` を渡す。`WindowPos.position` に加え `WindowPos.size`
///   を `SizeI::new(s.w, s.h)` で bypass ミラーする（`resize_window_to` の反映口）。
///
/// `size.is_some()` で明示分岐して flags/寸法を合成し、`SWP_NOSIZE` の付け外しミスを
/// 防ぐ（Req1.5：本経路を迂回する第二の bypass 書込経路を新設しない）。
///
/// bypass の理由: 実アプリでは flush 後の `SetWindowPos` が同期発火させる
/// `WM_WINDOWPOSCHANGED` echo が同値を（同じく bypass で）再書込するため、
/// ここで `Changed<WindowPos>` を発火させると `apply_window_pos_changes` が
/// 別フラグの `SetWindowPos` を二重発行してしまう。bypass なら発行は本関数の
/// 1 コマンドに閉じ、headless World（echo が来ない）でも `WindowPos` が
/// 期待座標・寸法を示す決定論シームになる。
///
/// # Arrangement.offset の直接同期（task 8.3-fix・4.8 実機ブロッカ）
///
/// bypass 書込は `Changed<WindowPos>` を発火させないため、wintf の
/// `sync_window_arrangement_from_window_pos` は本経路の移動を拾えない。放置すると
/// `GlobalArrangement`（αマスクヒットテストの境界）が spawn 位置に取り残され、
/// 移動後のバルーンがクリック死する（実機で確認された 4.8 ブロッカ）。wintf 自身が
/// ドラッグ対象窓の DragEnd で行う直接同期（drag/dispatch.rs
/// 「[DragEnd] Direct Arrangement.offset sync」＝`WindowPos.position` を `as f32`
/// 転写・同値ガード付き）と同じパターンを、本経路で動かした窓にも適用する。
/// `Changed<Arrangement>` は発火する（GA 再計算に必要）が、ゴースト窓の
/// `GlobalArrangement.bounds` は零寸のため `window_pos_sync_system` の
/// `width <= 0` ガードが skip し、`SetWindowPos` echo ループにはならない
/// （donor の DragEnd 同期と同じ性質）。
///
/// # `route` 引数（Req 1.2／2.4・design D11「enum 引数配管」・task 1.4）
///
/// 位置を書いた**経路**（＝要件 2.4 の「最終位置を書き込んだ主体」の名指し語彙）を
/// 呼出側から受け取り、**書込成功時に**窓移動レコード 1 行を専用 target
/// （[`diag::DIAG_TARGET`]）へ出す。ラッパ関数を増やさず引数で配管するのは D11 の裁定で、
/// route は後続タスクで遷移ガードの発火条件・warn 水準分岐の第一級入力にもなる。
///
/// `None` は「本 target が観測を**所有しない**書込」であり、該当するのは
/// **ドラッグ経路のキャラ窓書込のみ**である（[`on_char_drag`]／[`on_char_drag_end`]）——
/// design「placement::diag > Risks」の裁定どおり、ドラッグ中の観測は wintf の `[drag]`
/// target が所有し本 target を通らない（Req 2.4 の結論語彙も「[`PlacementRoute`] 名
/// ＋ wintf `[drag]`／提案位置書込の 2 語」と規定されている）。ドラッグに随伴する
/// バルーン側の書込は [`PlacementRoute::BalloonFollow`] を持つ（Req 2.5 の判別材料）。
///
/// `\![move]` cue（[`move_window_to`] の対象窓）は **`None` ではない**——D13 で
/// [`PlacementRoute::MoveCue`] を新設し、スクリプト明示移動を名指しできるようにした
/// （無記録のままだと Q3「ドラッグ以外の経路での消失」の観測に穴が残る）。
///
/// `None` でも**挙動は完全に同一**であり、変わるのはレコードを出すか否かだけである。
///
/// # runtime 関門（areka-P0-windowposition-limit C7・要件 2.1/2.2/2.6〜2.9/5.5）
///
/// **契約追記**: 対象 entity が [`BalloonLimit`]`(true)` を持つ場合、書込位置は
/// キャラ窓帰属モニタの作業領域内へ補正される（[`apply_balloon_limit_gate`]）。
/// それ以外の対象（キャラ窓・[`BalloonLimit`]`(false)`・Component 非保持窓）の挙動は
/// 従来と **bit 同一**である——分岐は route ではなく**対象窓の Component**＝
/// データ駆動で、route 語彙は観測のままに保たれる（DD1）。
///
/// 本関数が ECS 経路の位置書込の**単一ライター**であることが、要件 2.1「経路に
/// よらず常時」を経路個別の規律ではなく**構造**で成立させる要点である（経路③④⑤⑦
/// および将来の書込口が自動被覆される）。補正は `x`／`y` を再束縛して以降の全処理
/// （`SetWindowPosCommand`・`WindowPos` ミラー・`Arrangement.offset` 同期・
/// [`diag::log_window_move`] レコード）へ透過する＝**位置ログには補正後の最終位置が
/// 残る**（DD7）。route は元書込のものを保つ——補正は同一書込の一部であって別書込では
/// ないからである。
pub(super) fn enqueue_window_set_pos(
    world: &mut World,
    window: Entity,
    x: i32,
    y: i32,
    size: Option<SizePx>,
    route: Option<PlacementRoute>,
) -> bool {
    // 「entity 不在＝破棄済み」と「実在するが `WindowHandle` 未付与＝窓生成前」を混ぜない
    // （task 7.3・Req 6.2/6.3・task 3.2 が消費側 4 入口へ敷いたのと同じ区別）。前者は終了
    // 処理の**正常終了系**——終了処理でゴースト窓が破棄された後も随伴書込（`follow_balloon`）
    // は走り得るため、`warn!` のままにすると終了時ログが良性ノイズで埋まって本物の異常が
    // 読めなくなる（6.2 → 7.3 の申し送り）。後者は結線の異常ゆえ `warn!` を保つ。
    if world.get_entity(window).is_err() {
        debug!(
            entity = ?window,
            x, y,
            "{DESPAWNED_SKIP_TAG} 移動対象窓は既に破棄済み（despawn）→ 窓移動を正常系として打ち切り"
        );
        return false;
    }
    // 待ち札の適用範囲の不変条件（atom 設計 C5・要件 5.8）: 「[`DpiSyncHold`] を持つ窓への
    // 窓書込は 0」。見送りは 4 つの点（拡大率の相・報告寸の突合・**実表示寸の**再スナップ・
    // **作業領域変化を契機とする**再スナップ）で行うが、**書込口は本関数 1 つ**なので、
    // すり抜け経路が増えたかどうかはここでしか構造的に見張れない。4 点目は task 6.5 で
    // 加えた——「再スナップ」という日本語が別々の 2 関数を指しており、当初の列挙は
    // `resnap_shell_targets` の側しか覆っていなかった（実際に本監視が鳴った穴である）。
    // 実機では `warn!` として見え、テストビルドでは `debug_assert!` がその場で落とす。
    //
    // **随伴バルーンの追従（[`PlacementRoute::BalloonFollow`]）だけは対象外**である。ここで
    // 外しているのは**この監視**であって見送りではない——随伴の追従はそもそも 4 つの見送り点を
    // 通らないので、対象外にしなくても止まりはしない。外す理由は、止まらない書込をこの監視が
    // 「漏れ」として鳴らしてしまうからである（偽の警報を防ぐ）。
    //
    // 2 窓の中心が別々のモニタに乗る配置（キャラ窓は表と揃い、バルーンだけまだ食い違う）では、
    // 待ち札の付いたバルーンへ随伴が正当に届く。バルーンの位置は独立した量ではなくキャラ窓の
    // 従属量であり、キャラ窓が動いた同一書込の一部として動くからである。しかも
    // [`dpi_sync_decision`] は拡大率と表の値だけを見て作業領域を見ないので、表が古くても
    // 随伴の位置は壊れない。バルーン**自身**の書込（拡大率の相・報告寸の突合・2 つの再スナップ）は
    // 4 点の見送りが引き続き覆う。将来 4 点以外の書込口が増えれば、それはここで鳴る
    // （本関数で握り潰さない）。
    if route != Some(PlacementRoute::BalloonFollow)
        && let Some(hold) = world.get::<DpiSyncHold>(window)
    {
        let since_frame = hold.since_frame;
        warn!(
            entity = ?window,
            ?route,
            since_frame,
            x, y,
            "整合待ちの札がある窓へ窓書込が到達した（待ち札の適用範囲に漏れがある）"
        );
        debug_assert!(
            false,
            "DpiSyncHold のある窓へ窓書込が到達した: entity={window:?} route={route:?} since_frame={since_frame}"
        );
    }

    let Some(handle) = world.get::<WindowHandle>(window).copied() else {
        warn!(
            entity = ?window,
            x, y,
            "移動対象窓の WindowHandle 未付与（窓生成前）のため移動しない"
        );
        return false;
    };

    // runtime 関門（windowposition-limit C7・DD1/DD5/DD7）: `BalloonLimit(true)` の窓に
    // 限り、書き込もうとする矩形をキャラ窓帰属モニタの作業領域内へ補正する。ここで
    // `x`／`y` を再束縛するため、以降の書込・ミラー・レコードはすべて補正後の最終位置を
    // 見る。可視性ガード（`follow_balloon` 内の `guard_balloon_position`）は本関数を
    // **呼ぶ前**に走り終えており、limit の補正が最後の語になる（DD5）。
    let corrected = apply_balloon_limit_gate(world, window, PointPx { x, y }, size, route);
    let (x, y) = (corrected.x, corrected.y);

    // size 有無で flags と width/height を明示分岐（SWP_NOSIZE の付け外しミスを防ぐ）。
    // None＝移動専用（SWP_NOSIZE 付・後方互換）／Some＝位置＋寸（SWP_NOSIZE 外し）。
    let (flags, w, h) = match size {
        Some(s) => (SWP_NOZORDER | SWP_NOACTIVATE, s.w, s.h),
        None => (SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE, 0, 0),
    };

    // 要求語彙タグ（要件 2.1・design D3）: 「どの経路が・どの窓へ」を書込指令へ載せる。
    // 単一ライターがここ 1 箇所で付けるので、経路が増えても札の付け忘れが構造的に起きない
    // （生成の 7 引数は不変＝`with_tag` はビルダで足すだけ）。
    SetWindowPosCommand::enqueue(
        SetWindowPosCommand::new(handle.hwnd, x, y, w, h, flags, None)
            .with_tag(transition_diag::write_tag(world, window, route)),
    );

    // 既定位置の追跡（design D9／D16・要件 6.2）: **ミラーを書き換える前**に呼ぶ——本関数が
    // 見るのは「書込**前**の現在位置」であり、下の bypass ミラーが `WindowPos.position` を
    // 新位置へ書いた後では一致判定が常に成立してしまう（明示操作で動かした窓まで追随する）。
    track_default_char_pos(world, window, route, PointPx { x, y });

    match world.get_mut::<WindowPos>(window) {
        Some(mut wp) => {
            let wp = wp.bypass_change_detection();
            wp.position = Some(Point { x, y });
            // Some のときのみ寸法もミラー（None は size を触らない＝移動専用の後方互換）
            if let Some(s) = size {
                wp.size = Some(SizeI::new(s.w, s.h));
            }
        }
        None => {
            debug!(
                entity = ?window,
                "WindowPos 未付与のため ECS 側ミラー更新はスキップ（コマンドは enqueue 済み）"
            );
        }
    }

    // wintf DragEnd donor と同型の直接同期（doc コメント参照）。同値なら
    // 書かない（Deref 読みのみ＝Changed<Arrangement> を発火させない）。
    let new_offset = Offset {
        x: x as f32,
        y: y as f32,
    };
    match world.get_mut::<Arrangement>(window) {
        Some(mut arr) => {
            if arr.offset != new_offset {
                arr.offset = new_offset;
            }
        }
        None => {
            debug!(
                entity = ?window,
                "Arrangement 未付与のため GA ヒットテスト境界の同期はスキップ"
            );
        }
    }

    // 窓移動レコード（Req 1.2）: **書込成功時のみ** 1 レコードを専用 target へ出す。
    // 経路語彙を持たない書込（ドラッグ）は route=None ＝無記録（doc 参照）。
    // `\![move]` は D13 で `MoveCue` を得たので記録される（:753 で `Some` を渡す）。
    if let Some(route) = route {
        log_window_move(world, window, route, x, y, size);
    }

    true
}

/// 「まだ誰にも動かされていない」の基準（`ScopeWindows.default_char_pos`）を、**システム由来の
/// 再アンカー**が動かした先へ追随させる（design D9／D16・要件 6.2）。
///
/// # なぜ要るか
///
/// 連鎖の再解決は「明示的に再配置されたスコープを対象から外す」ために、現在位置が既定位置と
/// 一致するかを見る（`scope-chain-gap` 7.3・`drain_resnap::collect_chain_states`）。ところが
/// 拡大率が変われば**全スコープ**の位置がシステム側の都合で動く——既定位置を据え置いたままだと
/// 全件が「明示的に動かされた」へ倒れ、遷移後の解き直しが丸ごと空振りする（要件 6.2 が
/// 名指しで禁じる状態）。ゆえに、システム由来の書込に限って基準そのものを一緒に運ぶ。
///
/// # 追随させる条件（3 つすべて）
///
/// 1. **route がシステム由来**（[`PlacementRoute::is_system_reanchor`]）。明示操作
///    （`MoveCue`・`Restore`・`BalloonLimitRelease`）と route を持たない書込（ドラッグ＝
///    `None`）では追随させない——これらは「利用者が置いた位置」であって、既定位置との
///    食い違いこそが除外の根拠だからである。
/// 2. **対象がキャラ窓**（[`CharWindowMarker`] を持つ）。既定位置はスコープのキャラ窓に対して
///    のみ定義される量で、バルーン窓は従属量ゆえ持たない。
/// 3. **書込前の現在位置が既定位置と一致**。一致しない窓は既にドラッグ・台本で動かされて
///    おり、そこへ追随させると「動かされた」という事実そのものを消してしまう。
///
/// 既定位置が `None`（保存位置が復元されたスコープ）は `None` のままにする——`None` は
/// 「そもそも既定配置ではない」を表す標であって、位置の欠落ではない（`scope-chain-gap` 7.3）。
///
/// # 連鎖確定の書込とは重ならない
///
/// 起動時の連鎖確定（`drain_resnap::finalize_chain_once_with`）は反映を
/// [`move_window_to`]＝[`PlacementRoute::MoveCue`]＝**明示操作**の経路で行い、確定値を自分で
/// [`GhostWindows::set_default_char_pos`] へ書く。本関数はその経路に効かないので、2 つの
/// 書き手が同じ量を別々の値で書くことはない。
///
/// # 追随させないときは何もしない（無音）
///
/// 上の 3 条件は**常時**評価される（窓書込のたびに通る）ので、外れた場合は正常な素通りである。
/// 一方、キャラ窓かつシステム由来なのに台帳・位置が引けない形は結線の異常ゆえ `debug!` を残す
/// ——書込そのものは成立しており失敗ではないが、無音にすると「追随したのか対象外だったのか」が
/// 事後に判らなくなる。
fn track_default_char_pos(
    world: &mut World,
    window: Entity,
    route: Option<PlacementRoute>,
    written: PointPx,
) {
    // 条件 1: システム由来の再アンカーだけが基準を運ぶ。
    if !route.is_some_and(PlacementRoute::is_system_reanchor) {
        return;
    }
    // 条件 2: 既定位置を持つのはスコープのキャラ窓だけ。
    let Some(scope) = world.get::<CharWindowMarker>(window).map(|m| m.scope) else {
        return;
    };
    // 条件 3 の左辺: 書込**前**の現在位置。
    let Some(current) = world.get::<WindowPos>(window).and_then(|wp| wp.position) else {
        debug!(
            entity = ?window,
            scope,
            ?route,
            "WindowPos／position 未付与のため既定位置の追跡を見送る（書込は成立）"
        );
        return;
    };
    let Some(mut ghost_windows) = world.get_resource_mut::<GhostWindows>() else {
        debug!(
            entity = ?window,
            scope,
            ?route,
            "GhostWindows 未挿入のため既定位置の追跡を見送る（書込は成立）"
        );
        return;
    };
    // `None`（保存位置の復元）は `None` のまま——「既定配置ではない」の標を位置で塗り潰さない。
    let Some(default_pos) = ghost_windows.default_char_pos(scope) else {
        return;
    };
    // 条件 3: 一致していた窓だけが追随する。
    if default_pos.x != current.x || default_pos.y != current.y {
        return;
    }
    if default_pos == written {
        return;
    }
    ghost_windows.set_default_char_pos(scope, written);
    debug!(
        entity = ?window,
        scope,
        ?route,
        from_x = default_pos.x,
        from_y = default_pos.y,
        to_x = written.x,
        to_y = written.y,
        "既定位置をシステム由来の再アンカー先へ追随させた（D9／D16・要件 6.2）"
    );
}

/// 窓移動レコード（Req 1.2）を World から転写して組み、専用 target へ出す
/// （[`enqueue_window_set_pos`] の書込成功時専用・design「placement::diag > Invariants」）。
///
/// `diag` は placement の最下流で `World`・wintf 型に依存しない契約ゆえ、
/// **窓種別（[`CharWindowMarker`]／[`BalloonWindowMarker`]）・scope・DPI（`DPI` component）の
/// 読み出しは呼出側である本モジュールの仕事**である。`entity` は wintf 側ログ
/// （`entity = ?e`・scope を持たない）との**結合キー**として必ず入れる——Req 1.9 の
/// scope 別 DPI 受理計数は、この結合による 2 段 grep で機械化される。
///
/// 種別 marker が無い窓（placement が生成したゴースト窓ではない）は、種別・scope を
/// 発明せずレコードを出さない。ただし「出さなかった事実」自体は同 target へ残す
/// （silent skip を作らない・log-first）。
fn log_window_move(
    world: &World,
    window: Entity,
    route: PlacementRoute,
    x: i32,
    y: i32,
    size: Option<SizePx>,
) {
    let identity = world
        .get::<CharWindowMarker>(window)
        .map(|m| (WindowKind::Char, m.scope))
        .or_else(|| {
            world
                .get::<BalloonWindowMarker>(window)
                .map(|m| (WindowKind::Balloon, m.scope))
        });
    let Some((kind, scope)) = identity else {
        debug!(
            target: diag::DIAG_TARGET,
            entity = ?window,
            route = route.as_str(),
            "窓種別 marker 不在（placement 生成のゴースト窓ではない）ゆえ窓移動レコードを出さない"
        );
        return;
    };

    diag::log_window_move(&WindowMoveRecord {
        route,
        entity: window,
        kind,
        scope,
        x,
        y,
        // 寸を伴う経路（`Some`）は実寸を必ず詰める。`None` は移動専用（`SWP_NOSIZE`）の
        // 書込に限り、レコード側は番兵 `-` でフィールドを落とさない（grep 語の不変）。
        size: size.map(|s| (s.w, s.h)),
        dpi: world.get::<DPI>(window).map(|d| d.dpi_x as u32),
    });
}

// =============================================================================
// runtime 関門（areka-P0-windowposition-limit C7・task 3.3）
// =============================================================================

/// 書込直前の位置を `windowposition.limit=1` のバルーン窓に限って作業領域内へ補正する
/// （design「C7: runtime 関門」・要件 2.1／2.2／2.6／2.7／2.8／2.9／5.5／6.1／6.3）。
///
/// # 発動条件はデータ駆動であって route ではない（DD1）
///
/// 対象 entity が [`BalloonLimit`]`(true)` を持つときだけ補正する。持たない窓
/// （キャラ窓は spawn で**構造的に**付与されない＝2.8 の証明）と `false` の窓は
/// 最初の `match` で即座に `proposed` を返す——読み出しも警告も一切増えないので、
/// 従来経路と bit 同一である（2.7/2.8）。route を見ないので、`PlacementRoute` は
/// 観測語彙のままであり、将来 ECS 経路に書込口が増えても自動で被覆される（2.1）。
///
/// # 判定矩形（2.2 経路⑦の被覆）
///
/// 判定するのは「書き込もうとする位置 × 書き込もうとする寸」である。`size` が
/// `Some` ならその寸（位置据え置きのリサイズ＝[`resize_window_keep_position`] は
/// これで被覆される）、`None` なら対象の現 `WindowPos.size`。**位置が動かず寸だけが
/// 変わる書込**も、これで内包判定に掛かる。
///
/// # 基準領域はキャラ窓の帰属モニタ（5.5）
///
/// [`BalloonWindowMarker::scope`] → [`GhostWindows`]（scope→窓 entity の正本）→
/// キャラ窓 entity → その `WindowPos` 矩形 → [`work_area_for_window`] の順で引く。
/// **帰属規則そのもの（窓中心 half-open ＋最近傍フォールバック）は 1 bit も変えない**
/// ——既存関数へキャラ窓矩形を渡すだけである。バルーン窓自身の帰属で引かないのは、
/// はみ出したバルーンが隣モニタへ帰属して基準が振動するのを避けるためで、正典
/// 「シェルが居る画面の中に維持する」の素直な読みでもある。
///
/// # 可視性状態を**入力に持たない**（要件 2.6）
///
/// 本関門が読むのは上記の 6 つ（`BalloonLimit`／`BalloonWindowMarker`／
/// `GhostWindows`／キャラ窓 `WindowPos`／`MonitorSnapshot`／対象 `WindowPos.size`）
/// だけで、可視性を表す状態は**ひとつも含まない**（そもそもバルーンの可視性は
/// `emo2_boot::balloon_visibility` が表示層で所有し、placement の World には無い）。
/// ゆえに非表示中の書込も可視中とまったく同一に補正され、「次に可視となった時点で
/// 2.1 が成立している」が全書込の被覆から導かれる。檻
/// `runtime_gate_result_is_identical_for_hidden_and_visible_balloons` が、World 上で
/// 可視性に最も近い表現（`WindowStyle` の `WS_VISIBLE`）を落としても結果が
/// bit 同一であることで固定する。
///
/// # 位置だけを触る（2.9）
///
/// 戻り値は位置のみ。`size`・可視性・Z 順（`KeepDirectlyAbove`）へは一切触れない
/// ——呼出側が受け取った `size` はそのまま書かれる。
///
/// # 縮退（log-first・design Error Handling「runtime 関門で基準解決不能」）
///
/// 上記の解決がどこかで途切れたら [`BALLOON_LIMIT_UNRESOLVED_TAG`] の `warn!` を
/// 残して `proposed` を素通しする。**書込自体は阻害しない**——架空の作業領域を
/// 発明しないこと（`work_area_for_window` と同方針）と、補正できないことを理由に
/// 窓の位置更新を落とさないことの両立である。無ログの縮退経路は作らない（6.3）。
///
/// 補正が発火したときだけ [`BALLOON_LIMIT_CLAMP_TAG`] の `info!` を出す。
/// [`limit_correction`] の `None` は「クランプしても位置が動かない」の意であって
/// 「内包されている」ではない（逆転区間で既に左上に居るときははみ出したままでも
/// `None`）——毎フレーム偽の補正ログを出さないための設計であり、`None` では
/// ログも位置の書換も起こさない（6.1 は「実際に動かしたとき」の記録を求める）。
fn apply_balloon_limit_gate(
    world: &World,
    window: Entity,
    proposed: PointPx,
    size: Option<SizePx>,
    route: Option<PlacementRoute>,
) -> PointPx {
    // 発動条件（DD1）: limit 無効・Component 非保持は 1 bit も触らず戻る。
    if !matches!(world.get::<BalloonLimit>(window), Some(BalloonLimit(true))) {
        return proposed;
    }

    // 判定寸: 明示寸（`Some`）が優先。`None` は対象の現寸。未確定表現は
    // `Option::None` **だけではない**——`WindowPos::default()` は
    // `CW_USEDEFAULT`（`i32::MIN` センチネル）を寸に持つため、正値フィルタで
    // 一緒に落とす（`guard_balloon_position` と同型のガード）。
    let target_size = size
        .or_else(|| {
            world
                .get::<WindowPos>(window)
                .and_then(|wp| wp.size)
                .map(|s| SizePx {
                    w: s.width,
                    h: s.height,
                })
        })
        .filter(|s| s.w > 0 && s.h > 0);
    let Some(target_size) = target_size else {
        warn!(
            entity = ?window,
            context = BALLOON_LIMIT_RUNTIME_CONTEXT,
            route = ?route,
            ?proposed,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} 対象バルーンの寸が未確定（窓生成直後／CW_USEDEFAULT センチネル）のため内包判定できない → 補正せず書込は続行"
        );
        return proposed;
    };

    let Some(scope) = world.get::<BalloonWindowMarker>(window).map(|m| m.scope) else {
        warn!(
            entity = ?window,
            context = BALLOON_LIMIT_RUNTIME_CONTEXT,
            route = ?route,
            ?proposed,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} 対象窓に BalloonWindowMarker が無く scope を特定できない → 補正せず書込は続行"
        );
        return proposed;
    };

    let char_window = world
        .get_resource::<GhostWindows>()
        .and_then(|gw| gw.char_window(scope));
    let Some(char_window) = char_window else {
        warn!(
            entity = ?window,
            scope,
            context = BALLOON_LIMIT_RUNTIME_CONTEXT,
            route = ?route,
            ?proposed,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} GhostWindows 未挿入または当該 scope のキャラ窓が無く基準モニタを決められない → 補正せず書込は続行"
        );
        return proposed;
    };

    let Some(char_rect) = window_rect_of(world, char_window) else {
        warn!(
            entity = ?window,
            scope,
            char_window = ?char_window,
            context = BALLOON_LIMIT_RUNTIME_CONTEXT,
            route = ?route,
            ?proposed,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} キャラ窓の位置または寸が未確定で矩形を組めない → 補正せず書込は続行"
        );
        return proposed;
    };

    // 5.5: 帰属規則は既存関数のまま（キャラ窓矩形を渡すだけ）。
    let area = world
        .get_resource::<MonitorSnapshot>()
        .and_then(|snapshot| work_area_for_window(snapshot, char_rect));
    let Some(area) = area else {
        warn!(
            entity = ?window,
            scope,
            context = BALLOON_LIMIT_RUNTIME_CONTEXT,
            route = ?route,
            ?proposed,
            ?char_rect,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} MonitorSnapshot 未挿入またはモニタ 0 台のためキャラ窓の作業領域を決められない → 補正せず書込は続行"
        );
        return proposed;
    };

    let Some(corrected) = limit_correction(proposed, target_size, area) else {
        return proposed;
    };

    info!(
        scope,
        context = BALLOON_LIMIT_RUNTIME_CONTEXT,
        entity = ?window,
        route = ?route,
        from = ?proposed,
        to = ?corrected,
        balloon_size = ?target_size,
        work_area = ?area,
        "{BALLOON_LIMIT_CLAMP_TAG} runtime 関門: バルーン窓の書込位置を作業領域内へ補正した（寸・可視性・Z 順には触れない）"
    );
    corrected
}

/// `WindowPos` から窓矩形を組む（位置・寸のどちらかが未確定なら `None`）。
///
/// 未確定表現が `Option::None` だけでないのは wintf の既知の性質である——
/// `WindowPos::default()` は位置に `CW_USEDEFAULT`（`i32::MIN` センチネル）を持つ。
/// 素通しすると `saturating_add` で逆転矩形になり、モニタ帰属が意味を失う
/// （`resize_window_to` 手順 3／`guard_balloon_position` と同一の判定式）。
/// 負座標そのものは正当（左隣モニタは負の X を持つ）ゆえ、符号や閾値では判定しない。
fn window_rect_of(world: &World, window: Entity) -> Option<RectPx> {
    let wp = world.get::<WindowPos>(window)?;
    let pos = wp
        .position
        .filter(|p| p.x != CW_USEDEFAULT && p.y != CW_USEDEFAULT)?;
    let size = wp.size.filter(|s| s.width > 0 && s.height > 0)?;
    Some(rect_at(
        PointPx { x: pos.x, y: pos.y },
        SizePx {
            w: size.width,
            h: size.height,
        },
    ))
}

// =============================================================================
// resize_window_keep_position（areka-P0-emo-dpi-scaling task 2.2・R3.1/R4.2）
// =============================================================================

/// 現在位置を維持して窓寸のみ更新する（balloon 窓の DPI 追従用・R3.1/R4.2）。
///
/// 私有単一ライター経路 [`enqueue_window_set_pos`]`(.., Some(new_size))` の**薄い公開
/// ラッパ**——DPI 変化フェーズ（design D8・Flow 2）が balloon 窓の寸を k 追従させる
/// ための唯一の正規手段であり、単一ライター規律（`SetWindowPosCommand` 発行＋
/// `WindowPos` bypass ミラー＋`Arrangement.offset` 同期）を迂回する第二の書込経路を
/// 新設しない（Req1.5 の継承）。
///
/// [`resize_window_to`] との違いは**位置の決め方**だけである。あちらは
/// [`Anchored`] を読んで [`project_anchor`] で位置を再導出する（キャラ窓＝接地点が
/// アンカー辺に釘付けされる）が、こちらは `WindowPos.position` の**現在値をそのまま
/// 据え置き**、寸法だけを差し替える。balloon 窓の位置は [`follow_balloon`] が
/// キャラ窓＋`BalloonFollow.offset` から決める従属量であり、寸法変更の場面で
/// 独自にアンカー射影をかけると同フレーム内で二重に位置が動くため。
///
/// # 縮退・失敗経路（log-first・silent failure を作らない）
///
/// - **対象 entity 不在（despawn 済み）**: 正常終了系ゆえ `debug!`
///   （[`diag::DESPAWNED_SKIP_TAG`]）＋`false`（要件 6.2/6.3・[`resize_window_to`] と同一流儀）。
/// - 非正寸（w≤0 or h≤0）: 何も書かず `warn!`＋`false`（[`resize_window_to`] の
///   非正寸縮退と同一流儀）。
/// - `WindowPos` 不在（窓生成前の異常系）: 現在位置を読めないため `warn!`＋`false`。
/// - `WindowPos.position` 不在（窓生成前）: 同上 `warn!`＋`false`（panic しない）。
/// - べき等 skip（R4.2）: 現 `WindowPos.size` が新寸と同一なら**書込を一切行わず**
///   `false`（k 不変・同寸で窓が振動しないための檻・正常系ゆえ `debug!`）。
/// - `WindowHandle` 未付与/対象不在: [`enqueue_window_set_pos`] が `warn!`＋`false`
///   （判定を二重化せず委譲する）。
///
/// 消費者: `emo2_boot` の DPI 追従フェーズ（`run_dpi_phase`／`reconcile_reported_sizes`・結線済み）。
///
/// # 経路タグ（Req 1.2・task 1.4）
///
/// 本関数の書込は定義上つねに [`PlacementRoute::KeepPositionResize`]（経路語彙が関数名
/// そのもの）ゆえ、[`resize_window_to`] と違い route を引数で受けない——受けても
/// 呼出側が渡せる値は 1 つしか無く、取り違えの余地だけを増やすため。
#[allow(dead_code)] // examples が #[path] include するため、本体未使用ビルドでも必要
pub fn resize_window_keep_position(world: &mut World, window: Entity, new_size: SizePx) -> bool {
    // 0. 存在確認（要件 6.2/6.3・design D8 消費側）: 破棄済みバルーン窓は正常終了系として
    //    debug で打ち切る（下の `WindowPos` 未付与 warn は**実在する**窓の異常に取っておく）。
    if world.get_entity(window).is_err() {
        debug!(
            entity = ?window,
            "{DESPAWNED_SKIP_TAG} 対象 entity は既に破棄済み（despawn）→ 位置据置きリサイズを正常系として打ち切り"
        );
        return false;
    }

    // 1. 非正寸ガード（R3.1）: 窓寸として成立しない値は書かない。
    if new_size.w <= 0 || new_size.h <= 0 {
        warn!(
            entity = ?window,
            ?new_size,
            "新しい窓寸法が非正のためリサイズせず現状保持"
        );
        return false;
    }

    // 2. 現在位置（維持する値）と現寸（べき等判定の材料）を読む。
    //    WindowPos／position 不在は窓生成前の異常系＝log-first で no-op（panic しない）。
    let (pos, current_size) = {
        let Some(wp) = world.get::<WindowPos>(window) else {
            warn!(
                entity = ?window,
                "WindowPos 未付与（窓生成前）のため現在位置を読めずリサイズしない"
            );
            return false;
        };
        let Some(pos) = wp.position else {
            warn!(
                entity = ?window,
                "WindowPos.position 不在（窓生成前）のため現在位置を読めずリサイズしない"
            );
            return false;
        };
        (pos, wp.size)
    };

    // 3. べき等 skip（R4.2・D8）: 同寸なら書込ゼロ。位置は据え置きゆえ寸だけを見れば
    //    十分（resize_window_to の (position, size) 一致判定の位置維持版）。
    //    現寸不明（None＝窓生成直後）は判定が成立しないので書込へ進む。
    if current_size == Some(SizeI::new(new_size.w, new_size.h)) {
        debug!(
            entity = ?window,
            ?new_size,
            "窓寸が現在値と同一のため書込をスキップ（べき等・振動防止）"
        );
        return false;
    }

    // 4. 一度書き: 現在位置＋新寸を単一ライター経路で 1 コマンド発行する。
    //    WindowHandle 未付与/対象不在は enqueue が warn!＋false（二重に弾かない）。
    enqueue_window_set_pos(
        world,
        window,
        pos.x,
        pos.y,
        Some(new_size),
        Some(PlacementRoute::KeepPositionResize),
    )
}
