//! ECS 組立: キャラ窓／バルーン窓 entity spawn と公開データ構造（task 5.1）。
//!
//! 解決済み配置（[`ScopePlacement`]）から窓 entity を組み立て、識別子
//! （markers・[`GhostWindows`]）を後続（emo2-boot）へ公開する
//! （design「placement::spawn」・要件 1.1/1.2/1.5/4.1/4.5/5.1/6.1/6.2/6.3/7.2）。
//!
//! # 座標単位契約（design U1/U2）
//!
//! 位置・寸法は **`ScopePlacement` 由来の物理 px のみ**を `WindowPos` へ転記する。
//! `BoxStyle`（論理 DIP）と `DragConstraint` は一切付けない（U2・DD8。
//! 2026-07-05 の単位混在・単一モニタ誤釘付けの欠陥面そのものを消す）。
//! デモ由来の座標リテラル（`(400,200)`／`(335,0)` 等）はこのモジュールに
//! 存在しない（1.5・design「座標定数の禁止」）。
//!
//! # 窓 entity 構成（design「placement::spawn」の正本 bullet）
//!
//! キャラ窓・バルーン窓の**両方**が `DpiSuggestedRectPolicy::ExternalAuthority` を持つ
//! （areka-P0-dpi-window-vanish 要件 4.3・D3。[`external_position_authority`] に理由を記載）。
//!
//! - キャラ窓: `Name`＋`CharWindowMarker{scope}`＋`GhostWindowMarker`＋`Window{title}`
//!   ＋`WindowStyle { style: WS_POPUP|WS_VISIBLE, ex_style: WS_EX_LAYERED|WS_EX_TOOLWINDOW }`
//!   （**`WS_EX_TOPMOST` なし**・5.1／DD13）＋`WindowPos { position, size }`（物理 px）
//!   ＋`HitTest::none()`（全面ヒットで透過を殺さない）＋`Anchored(p.anchor)`（解決済み
//!   アンカーの単一真実源・**全 char 窓へ無条件付与**＝Free 窓も resize の identity 射影で
//!   読む・4.2）＋`DragConfig`（全面ドラッグ・4.1。`move_window` は非 Free アンカーの
//!   キャラ窓のみ false＝on_char_drag 単一ライター・DD15 v2／4.7、Free は true＝wndproc
//!   委譲）＋`OnDrag(on_char_drag)`＋`BalloonFollow`。**全**キャラ窓（Free 含む）は
//!   さらに `OnDragEnd(on_char_drag_end)` を持つ（最終カーソル位置への同写像適用＋
//!   確定位置の永続 write-through・非 Free は最終再固定・Free は保存専用アーム・
//!   DD15 v2 (3)・1.1・task 2.2）。
//!   なおキャラ窓へのポインタハンドラ（`OnPointerMoved`／`OnPointerPressed`）は
//!   **本モジュールでは付けない**——マウス移動／ダブルクリックを kanade へ配信する結線は
//!   `input_events::attach_char_pointer_handlers` が spawn 直後に装着する（依存方向
//!   input_events→placement。placement は `crate::` パスを持たず `super::`／外部 crate のみ
//!   参照する＝example の `#[path]` include で成立させるため。areka-P0-input-events）。
//!   Ctrl+左ダブルクリックは暫定退避（全 `GhostWindowMarker` despawn→window-close funnel→
//!   `run()` 正常復帰）で、これも input_events 側ハンドラ／main.rs の結線が担う（stand-in
//!   即終了 `on_ghost_pressed` は退役）
//! - バルーン窓: 同型（marker は `BalloonWindowMarker{scope}`・`DragConfig::default()`
//!   は付与＝バルーン単独ドラッグ可・4.5。`OnDrag(on_balloon_drag)` で単独ドラッグの
//!   相対位置記憶（4.8・DD16・task 8.3）＋`OnDragEnd(on_balloon_drag_end)` で単独ドラッグ
//!   確定 offset の永続 write-through（2.1・design C3・task 2.3。`on_balloon_drag` は連続
//!   イベントで保存トリガではなく、DragEnd 確定点でのみ 1 ドラッグ 1 書込）・`BalloonFollow`
//!   なし。さらに `BalloonLimit(p.balloon_limit)`＝`windowposition.limit` 解決値の
//!   runtime 焼込みを**バルーン窓側にだけ**持つ（areka-P0-windowposition-limit 2.7/2.8・
//!   C9/DD4。キャラ窓には付けない＝「limit の補正でキャラ窓を動かさない」の構造保証）。
//!   M1 はマウス送出なし＝ポインタハンドラを付けない・DD-IE-12。バルーン入力は
//!   M-dialogue／choice-render の領分。さらに同一スコープのキャラ窓を指す
//!   `KeepDirectlyAbove { peer }`＝「このバルーン窓はこのキャラ窓のすぐ手前に居るべき」の
//!   永続宣言を**バルーン窓側にだけ**後付けする（areka-P0-ghost-window-zorder 1.1・
//!   キャラ窓 entity を要するため後付け＝生成順は不変。同時に scope とペア両窓を結ぶ
//!   `declared` 記録を出す＝scope を知る層は areka だけ・6.1））
//!
//! # clickthrough 登録（task 5.2）
//!
//! [`register_ghost_windows_click_through`] が `Added<WindowHandle>` で
//! [`GhostWindowMarker`] 窓を αマスク clickthrough 機構
//! （wintf `ClickThroughRegistryHandle`・消費のみ）へ登録する
//! （emo-present donor `register_click_through_windows` の一般化・6.1）。
//!
//! # 重なり管理の結線（areka-P0-ghost-window-zorder task 3.2）
//!
//! [`wire_zorder_pair`] が実行時ストラテジ（既定＝案 A・補助浮上なし）を明示挿入し、
//! **挿入した当の値を起動時ログへ 1 行残し**（要件 5.6・実機ゲートの結論をバイナリ自身が
//! 名乗る）、wintf の確立系 → 維持系を clickthrough 登録と同じ確定段（`FrameFinalize`）へ
//! この順で載せる。呼び手は main.rs の起動窓シーム（同 1.1／5.6／6.1）。

use std::collections::BTreeMap;

use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::DeferredWorld;
use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::drag::{DragConfig, OnDrag, OnDragEnd};
use wintf::ecs::layout::HitTest;
use wintf::ecs::{
    DpiSuggestedRectPolicy, FrameFinalize, KeepDirectlyAbove, Point, SizeI, Window, WindowHandle,
    WindowPos, WindowStyle, ZOrderPairStrategy, apply_zorder_pair_maintenance,
    establish_owner_links,
};

use super::diag::{log_zorder_pair_declared, log_zorder_pair_strategy};
use super::follow::{
    on_balloon_drag, on_balloon_drag_end, on_char_drag, on_char_drag_end, Anchored, BalloonFollow,
};
use super::config::BalloonXMode;
use super::resolver::{PointPx, ScopePlacement};
use super::source::GhostTitles;

// ---------------------------------------------------------------------------
// 識別 markers（6.2）
// ---------------------------------------------------------------------------

/// スコープ別キャラ窓の識別 marker（6.2・補助的な逆引き。正本は [`GhostWindows`]）。
#[allow(dead_code)] // 結線（main.rs シーム）は task 6.2
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharWindowMarker {
    /// スコープ番号（0=本体・1=相方・…）。
    pub scope: usize,
}

/// スコープ別バルーン窓の識別 marker（6.2・補助的な逆引き。正本は [`GhostWindows`]）。
#[allow(dead_code)] // 結線（main.rs シーム）は task 6.2
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalloonWindowMarker {
    /// スコープ番号（対応するキャラ窓と同じ番号）。
    pub scope: usize,
}

/// placement 生成窓の共通標識（smoke close・一括 despawn・clickthrough 登録の標的）。
///
/// # despawn 掃除（areka-P0-dpi-window-vanish 要件 6.1・D8）
///
/// `on_remove` component hook（wintf `Visual::on_add`／`VisualGraphics::on_remove` の先例）で
/// [`GhostWindows`] から当該 scope エントリを落とす。**hook にしているのが要点**で、
/// 「終了処理から掃除関数を呼ぶ」形にすると呼出点結合になり、別経路の despawn
/// （Ctrl+左ダブルクリック退避・将来の個別 close 等）を取りこぼす。marker が消える所＝
/// 窓が消える所であり、そこが唯一の掃除トリガである。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[component(on_remove = on_ghost_window_marker_remove)]
pub struct GhostWindowMarker;

/// [`GhostWindowMarker`] 除去 hook: [`GhostWindows`] から scope エントリを落とす（6.1）。
///
/// # 触ってよいもの／いけないもの（要件 6.4 の構造的保証）
///
/// **Resource（[`GhostWindows`]）だけ**を触る。生存している entity の component は
/// 読みも書きもしない——それゆえ「掃除の前後で生存窓の位置・寸法・追従関係が変化しない」
/// は檻の主張ではなく**構造の帰結**である。`DeferredWorld` は `get::<C>()` で他 entity の
/// component を覗けてしまうが、ここでそれをやってはならない。
///
/// 除去成立も no-op も `debug!` 止まり（正常終了系＝良性ノイズを作らない・要件 6.2 の前提）。
/// Resource 未挿入（ダミー窓フォールバック経路・素の `World` の檻）は静かに no-op。
fn on_ghost_window_marker_remove(mut world: DeferredWorld, hook: HookContext) {
    let entity = hook.entity;
    // Resource 未挿入は no-op（`resource_mut` だと panic するので `get_resource_mut`）。
    let Some(mut registry) = world.get_resource_mut::<GhostWindows>() else {
        return;
    };
    match registry.remove_entry_of(entity) {
        Some((scope, windows)) => debug!(
            scope,
            ?entity,
            char_window = ?windows.char_window,
            balloon_window = ?windows.balloon_window,
            "placement: ゴースト窓レジストリから scope エントリを除去"
        ),
        // 対の後追い despawn（最初の片割れが既に scope ごと落としている）＝正常系。
        None => debug!(
            ?entity,
            "placement: ゴースト窓 despawn だがレジストリに該当 scope なし（除去済み・良性）"
        ),
    }
}

// ---------------------------------------------------------------------------
// BalloonLimit（windowposition-limit C9・DD4）
// ---------------------------------------------------------------------------

/// バルーン窓の limit 有効値（scope 別解決済み・spawn 焼込み・runtime 単一真実源）。
///
/// `windowposition.limit` の解決値を runtime へ運ぶ唯一の表現である
/// （areka-P0-windowposition-limit 要件 2.7/2.8・design C9/DD4）。
/// `PlacementConfig`／`ScopeConfig` は spawn 後に破棄され Resource 化もされないため、
/// limit 値が runtime まで生き残る道は窓 entity 上の Component だけ——
/// `ScopeConfig.balloon_limit` → `ScopePlacement.balloon_limit` → 本 Component の
/// **一方向転写**で、spawn 時に焼き込んだあとは不変（永続化もしない。limit は
/// 毎起動 `descript.txt` から解決し直す）。
///
/// # 付与対象はバルーン窓のみ（要件 2.8 の構造保証）
///
/// [`Anchored`] と同型の spawn 焼込みだが**付く側が逆**である。`Anchored` はキャラ窓
/// にのみ、本 Component はバルーン窓にのみ付く。runtime 関門はこの Component を
/// 持つ entity への書き込みだけを補正するため、「キャラ窓に付いていない」ことが
/// そのまま「limit の補正でキャラ窓の位置を変更しない」（2.8）の証明になる——
/// 檻の主張ではなく構造の帰結である。
// 消費者（runtime 関門・`follow::enqueue_window_set_pos`）が入るのは task 3.3 だが、
// `#[allow(dead_code)]` は付けない——spawn が構築し檻が読むため lint は鳴らず、
// 不要な allow は本当に未結線の項目を隠すだけである（隣接 markers の allow は
// 「構築側が未実装」だった過渡状態の名残で、本 Component は事情が異なる）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalloonLimit(pub bool);

// ---------------------------------------------------------------------------
// BalloonKeywordBase（windowposition-limit 4.7・実機サインオフ是正）
// ---------------------------------------------------------------------------

/// 実表示寸が確定した瞬間に**一度だけ**キーワード由来のバルーン基本位置を導出し直す
/// ための素材（消費したら除去する使い切りの Component）。
///
/// # 付与対象は**キャラ窓**（`BalloonLimit` とは逆）
///
/// 導出し直す対象が `BalloonFollow.offset` であり、それがキャラ窓側に居るからである。
/// 付くのは `ScopePlacement.balloon_keyword_base` が `Some` の scope だけ——すなわち
/// `windowposition.x` がキーワード指定で、かつ保存された相対位置が効いていない scope に
/// 限る。`Side`（数値指定・未指定）のキャラ窓には**構造的に付かない**ので、
/// 「数値指定の分岐は 1 ビットも変わらない」（要件 4.5/5.2）は檻の主張ではなく
/// 構造の帰結である。
///
/// # 保存された相対位置が効いている間は本 Component が居ない（要件 4.7・不変条件）
///
/// 要件 4.7 は While 節（状態）であり、保存値は**起動時に読み込まれる**だけでなく
/// **セッション中にも生まれる**。ゆえに退役点は 2 つあり、どちらも「保存された相対位置が
/// 生まれた事象」に結び付いている:
///
/// - 起動時: `persist::merge_scope` が保存 offset 両軸を読めた scope へ `None` を置く
///   （そもそも付与されない）。
/// - セッション中: バルーン単独ドラッグの DragEnd 書込と同じ観測点で
///   `follow::drag_follow::retire_keyword_base_on_save` が除去する。
///
/// 起動時だけに置いていた頃は、実表示寸が採寸寸と一致する**ふつうの**ゴースト
/// （＝再解決が同寸 skip で素材を消費しない）で、ドラッグ後の寸法変化がキーワード既定を
/// 利用者の相対位置へ上書きしていた（2026-08-14 の feature 検証で確認・task 4.6 で是正）。
///
/// # 一度きりである理由と、その限界
///
/// 要件 4.7 はキーワードの適用を「初期既定位置の供給にとどめる」と定める——連続的な
/// 中央追従ではない。ただし採寸した寸と実際に表示される寸は食い違うことがあり
/// （実機 2026-08-14: 採寸 434 に対し実表示 382）、そのとき初期既定位置は**表示される
/// 寸から導かれていなければ意味を成さない**。ゆえに消費の契機は**キャラ窓の寸が実際に
/// 変わった最初の書込**であり、そこで本 Component を消費する。
/// **その後にバルーン寸やキャラ寸が変わっても中央へ揃え直さない**
/// ——`Side` と同じく配置時確定の静的 offset として振る舞う（4.4 の現行契約どおり）。
///
/// 特定の route（`PlacementRoute::ReportedSizeReconcile` など）を「実表示寸が判る唯一の
/// 瞬間」として条件に置く案は**却下した**。実表示寸を最初に運ぶ route は
/// `DpiReproject`／`ReportedSizeReconcile`／`Resnap` のいずれにもなり得て、どれが先に
/// 来るかは frame の相順に依存するため、route を条件にすると相順が変わるたびに静かに
/// 壊れる（判定の詳細は `follow::keyword_base::rederive_keyword_balloon_offset` の doc）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalloonKeywordBase {
    /// キーワードが定める基本位置の種別（中央上／中央下）。
    pub mode: BalloonXMode,
    /// 作者指定の調整量（`windowposition.y` の数値＋`balloon.offsetx/offsety`・4.4）。
    pub adjust: PointPx,
}

// ---------------------------------------------------------------------------
// GhostWindows（後続 emo2-boot への引き渡し正本・6.1/6.2）
// ---------------------------------------------------------------------------

/// スコープ 1 体ぶんの窓 entity 対（キャラ窓＋バルーン窓）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeWindows {
    /// キャラ窓 entity。
    pub char_window: Entity,
    /// バルーン窓 entity。
    pub balloon_window: Entity,
    /// **システム由来の再アンカーで追随する既定**キャラ窓位置（物理 px・scg 7.3・
    /// atom D9／D16）。
    ///
    /// 「まだ誰にも動かされていない」ことの判定基準。現在位置がこの値と一致する間は
    /// 既定配置のままであり、ゴースト台本の移動指令や利用者のドラッグで動いた
    /// スコープは一致しなくなる——移動側へフックを足さずに除外できる。
    ///
    /// # 初期値は spawn 時の resolver 出力だが、そこに固定はされない（atom task 5.5）
    ///
    /// 拡大率が変わればキャラ窓は新しい寸で下端中央を保ったまま置き直され、**誰も触って
    /// いないスコープの位置も動く**。この基準を spawn 時の値に固定したままだと、遷移後は
    /// 全スコープが「明示的に動かされた」へ倒れて連鎖の解き直しが丸ごと空振りする
    /// （`areka-P0-dpi-transition-atomicity` 要件 6.2 が名指しで禁じる状態）。
    ///
    /// ゆえに単一の窓書込口（`follow::window_move::enqueue_window_set_pos`）が、
    /// **システム由来の再アンカー**（`PlacementRoute::is_system_reanchor`）で、かつ
    /// **書込前の現在位置がこの値と一致していた**ときに限り、この値を書込先へ運ぶ。
    /// 明示操作（`\![move]`・復元・ドラッグ・limit 解放時補正）では運ばない——一致しない
    /// という事実そのものが除外の根拠だからである。
    ///
    /// **`None` は「そもそも既定配置ではない」**を表す。前回セッションで保存された
    /// 位置が復元されたスコープがこれにあたる（`main.rs` の復元マージが採用した位置は
    /// 利用者の意思による配置であって resolver 既定ではない）。`None` のスコープは
    /// 連鎖の再解決から**常に除外**され、既定位置へ引き戻されない（scg 7.3）。
    /// システム由来の再アンカーでも `None` は `None` のままである（D9）。
    pub default_char_pos: Option<PointPx>,
}

/// 後続（emo2-boot）への引き渡し正本（6.1/6.2）。
///
/// 「スコープ×種別 → Entity」の唯一の正本（markers は補助的な逆引き）。
/// spawn 完了時に Resource 挿入＋戻り値の両方で公開される。窓 despawn 後の
/// Entity 無効化は M1 では追跡しない（emo2-boot は起動直後に読む前提・design
/// Revalidation Trigger）。
#[derive(Resource, Clone, Debug)]
pub struct GhostWindows {
    /// スコープ番号 → 窓 entity 対（非公開・アクセサ経由）。
    windows: BTreeMap<usize, ScopeWindows>,
}

#[allow(dead_code)] // 消費側（emo2-boot／main.rs シーム task 6.2）は後続
impl GhostWindows {
    /// スコープのキャラ窓 entity を返す（未知スコープは `None`・panic しない）。
    pub fn char_window(&self, scope: usize) -> Option<Entity> {
        self.windows.get(&scope).map(|w| w.char_window)
    }

    /// スコープのバルーン窓 entity を返す（未知スコープは `None`・panic しない）。
    pub fn balloon_window(&self, scope: usize) -> Option<Entity> {
        self.windows.get(&scope).map(|w| w.balloon_window)
    }

    /// 生成済みスコープ番号を昇順で列挙する（`BTreeMap` キー順）。
    pub fn scopes(&self) -> impl Iterator<Item = usize> + '_ {
        self.windows.keys().copied()
    }

    /// スコープの既定キャラ位置を返す（初期値は spawn 時の resolver 出力・以後は
    /// **システム由来の再アンカーで追随する**・atom D9／D16）。
    ///
    /// 現在位置がこの値と一致するかで「既定配置のまま＝まだ誰にも動かされていない」ことを
    /// 判定する（scg 7.3）。拡大率の遷移で位置が動いても、動かしたのがシステム側であれば
    /// この値が一緒に動くため一致は保たれる——読み手は「起動時の値」ではなく
    /// **「最後にシステムが置いた既定位置」**として読むこと。
    ///
    /// **`None` は 2 通り**——未知スコープ、または当該スコープが
    /// そもそも既定配置ではない（保存位置の復元）。連鎖の再解決はどちらの `None` も
    /// 「対象外」として同じに扱えばよい。
    pub fn default_char_pos(&self, scope: usize) -> Option<PointPx> {
        self.windows.get(&scope).and_then(|w| w.default_char_pos)
    }

    /// スコープの既定キャラ位置を更新する（実表示寸での連鎖再解決が確定させた値・scg 7.1／
    /// システム由来の再アンカーの書込先・atom D9／D16）。
    ///
    /// 未知スコープは **no-op**（panic せず `false` を返す）。台帳を再解決後の真値へ揃え、
    /// 以後の「既定配置のまま」判定が確定後の位置を基準に働くようにする。
    ///
    /// # 呼び手は 2 つある（互いに重ならない）
    ///
    /// - 連鎖確定（`emo2_boot::frame::drain_resnap::finalize_chain_once_with`）——反映は
    ///   `move_window_to`＝`PlacementRoute::MoveCue` で行うため、下の追随規則には掛からない。
    ///   ゆえに確定後の X をここで明示的に揃える必要がある（scg 7.1）。
    /// - 単一の窓書込口（`follow::window_move::enqueue_window_set_pos`）——システム由来の
    ///   再アンカーで、書込前の位置が既定位置と一致していたときだけ書込先へ運ぶ（D9／D16）。
    pub fn set_default_char_pos(&mut self, scope: usize, pos: PointPx) -> bool {
        match self.windows.get_mut(&scope) {
            Some(w) => {
                w.default_char_pos = Some(pos);
                true
            }
            None => false,
        }
    }

    /// スコープを「既定配置ではない」と標す（保存位置が復元されたスコープ・scg 7.3）。
    ///
    /// 復元位置は**利用者の意思による配置**であって resolver 既定ではない。標されたスコープは
    /// 連鎖の再解決から常に除外され、既定位置へ引き戻されない。未知スコープは no-op
    /// （panic せず `false`）。
    ///
    /// 呼び手は復元マージを行う `main.rs` の起動シームのみ（保存位置が入り込む唯一の経路）。
    pub fn clear_default_char_pos(&mut self, scope: usize) -> bool {
        match self.windows.get_mut(&scope) {
            Some(w) => {
                w.default_char_pos = None;
                true
            }
            None => false,
        }
    }

    /// `entity` が char/balloon いずれかに一致する scope エントリを**丸ごと**除去し、
    /// 除去した `(scope, ScopeWindows)` を返す（areka-P0-dpi-window-vanish 6.1・D8）。
    ///
    /// 対（char＋balloon）は spawn/despawn とも原子的な生存単位ゆえ、**片割れの entity
    /// 1 個で scope エントリごと**落とす。不一致（既に除去済み・非ゴースト entity・
    /// 空レジストリ）は `None` を返すだけの **no-op**——panic せず、`Err` も出さない。
    /// これが「対の後追い despawn が良性である」ことの構造的な根拠である。
    pub fn remove_entry_of(&mut self, entity: Entity) -> Option<(usize, ScopeWindows)> {
        let scope = self
            .windows
            .iter()
            .find(|(_, w)| w.char_window == entity || w.balloon_window == entity)
            .map(|(scope, _)| *scope)?;
        self.windows.remove(&scope).map(|w| (scope, w))
    }
}

// ---------------------------------------------------------------------------
// spawn_ghost_windows（bare World で動く組立・headless テスト可）
// ---------------------------------------------------------------------------

/// 解決済み配置からキャラ窓・バルーン窓 entity を組み立てる（design「placement::spawn」）。
///
/// bare `World` だけで動く（`spawn_dummy_window` と同型・headless テスト可）。
/// 位置・寸法は **`placements`（[`ScopePlacement`]・物理 px）由来のみ**を
/// `WindowPos` へ転記し、座標リテラルを一切持たない（1.5・U1）。
/// スコープごとにバルーン窓を先に spawn し（`BalloonFollow.balloon` が entity を
/// 要するため）、次にそのバルーンを参照するキャラ窓を spawn する。
///
/// 完了時に [`GhostWindows`] を Resource として挿入し、同じ内容を戻り値でも
/// 返す（6.1・Resource 挿入＋戻り値の両方で公開）。
pub fn spawn_ghost_windows(
    world: &mut World,
    placements: &[ScopePlacement],
    titles: &GhostTitles,
) -> GhostWindows {
    let mut windows = BTreeMap::new();

    for p in placements {
        let title = titles.title(p.scope);

        // バルーン窓（design「窓 entity 構成（バルーン窓）」: キャラ窓と同型・
        // marker は BalloonWindowMarker・DragConfig::default() 付与＝単独ドラッグ可
        // （4.5）・OnDrag(on_balloon_drag) で単独ドラッグの相対位置記憶
        // （4.8・DD16・task 8.3）・BalloonFollow なし。
        // M1 はバルーンにマウス送出なし＝ポインタハンドラを付けない（DD-IE-12・
        // task 3.2：stand-in `on_ghost_pressed` 登録を撤去。バルーン入力は
        // M-dialogue／choice-render の領分でリゾルバは shell 窓専用）。
        let balloon_window = world
            .spawn((
                Name::new(format!("Ghost-Balloon-Window-{}", p.scope)),
                BalloonWindowMarker { scope: p.scope },
                GhostWindowMarker,
                Window {
                    title: title.to_string(),
                    ..Default::default()
                },
                window_style(),
                window_pos(p.balloon_pos.x, p.balloon_pos.y, p.balloon_size.w, p.balloon_size.h),
                HitTest::none(),
                // 位置権威の外部宣言（areka-P0-dpi-window-vanish 4.3・D3）。バルーン窓も
                // OS 直書きから外す——キャラ窓だけ外すと、DPI 跨ぎでバルーンだけが OS 提案
                // 位置へ飛び、`balloon_pos − char_pos ≡ offset` の恒等式が構造的に崩れる。
                external_position_authority(),
                DragConfig::default(),
                // limit 解決値の runtime 焼込み（windowposition-limit 2.7・C9/DD4）。
                // `ScopePlacement.balloon_limit` の転写であり、キャラ窓には付けない
                // （＝2.8 の構造保証。下の char 窓 spawn に対応する行は無い）。
                BalloonLimit(p.balloon_limit),
                OnDrag(on_balloon_drag),
                // バルーン単独ドラッグ確定 offset の永続 write-through（2.1・8.1・
                // design C3・task 2.3）。on_balloon_drag は連続イベント（in-session offset
                // 更新）で保存トリガではない——DragEnd 確定点でのみ 1 ドラッグ 1 書込する。
                OnDragEnd(on_balloon_drag_end),
            ))
            .id();

        // キャラ窓（design「窓 entity 構成（キャラ窓）」: OnDrag(on_char_drag) で
        // バルーン追従（4.2）・BalloonFollow は ScopePlacement.balloon_offset の転写
        // （配置時 1 回だけ確定・4.4））
        let char_window = world
            .spawn((
                Name::new(format!("Ghost-Char-Window-{}", p.scope)),
                CharWindowMarker { scope: p.scope },
                GhostWindowMarker,
                Window {
                    title: title.to_string(),
                    ..Default::default()
                },
                window_style(),
                window_pos(p.char_pos.x, p.char_pos.y, p.char_size.w, p.char_size.h),
                HitTest::none(),
                // 位置権威の外部宣言（areka-P0-dpi-window-vanish 4.3・D3・S1 是正の源断ち）。
                external_position_authority(),
                // 解決済みアンカーの単一真実源を全 char 窓へ**無条件付与**する（4.2/1.6）。
                // Free 窓も付ける——resize の identity 射影（project_anchor の Free 腕）が
                // Anchored を読むため。二値吸着フラグ（旧 BottomSnap marker）は廃し、
                // ドラッグ／リサイズはこの単一値を読んで射影 T を分岐する（Req1.6・DD15 v2）。
                Anchored(p.anchor),
                // 非 Free アンカー（Bottom/Top/Left/Right）のキャラ窓は
                // move_window=false＝wndproc は窓を動かさず on_char_drag が単一ライター
                // （DD15 v2・4.7・task 8.2R）。Free は従来どおり wndproc 直接移動（4.1）。
                // 二値吸着フラグは持たず anchor 単一値から導出する（Req1.6）。
                // threshold 等は既定を保つ。
                DragConfig {
                    move_window: p.anchor.is_free(),
                    ..Default::default()
                },
                OnDrag(on_char_drag),
                BalloonFollow {
                    balloon: balloon_window,
                    offset: p.balloon_offset,
                },
                // マウス入力配線（areka-P0-input-events）: キャラ窓のポインタ移動／押下を
                // kanade へ配信する `OnPointerMoved`／`OnPointerPressed` は**ここでは付けない**。
                // 依存方向は input_events→placement（placement は `crate::` パスを持てない＝
                // example の `#[path]` include で成立させるため）ゆえ、ハンドラ装着は spawn
                // 直後に `input_events::attach_char_pointer_handlers` が行う（stand-in 即終了
                // `on_ghost_pressed` は退役。Ctrl+左ダブルクリック暫定退避もそのハンドラ側の
                // 責務・DD-IE-7）。
            ))
            .id();

        // DragEnd 最終適用＋位置保存の結線（1.1/1.9/4.7/1.6・design C2/C3・task 2.2）:
        // Free 含む**全**キャラ窓へ OnDragEnd を無条件結線する（吸着はドラッグ中の制約で
        // あって保存条件ではない・1.1）。非 Free は on_char_drag_end が Anchored を読んで
        // project_anchor でアンカー辺へ最終再固定し（最終 DragEvent 欠落の穴埋め・DD15 v2 (3)）、
        // Free は射影が identity ゆえ wndproc 確定位置を素通しする保存専用アームとして働く。
        // いずれも on_char_drag_end 末尾で CharWindowMarker.scope を逆引きして位置を
        // Ghost 永続スコープへ write-through する。
        world
            .entity_mut(char_window)
            .insert(OnDragEnd(on_char_drag_end));

        // キーワード再導出の素材（windowposition-limit 4.7・実機サインオフ是正）:
        // `Some` の scope のキャラ窓にだけ後付けする。`Side` と保存値が効いた scope には
        // 構造的に付かない（`BalloonKeywordBase` の doc 参照）。後付けなのは上の
        // `OnDragEnd` と同じ形——条件付きの Component を spawn タプルへ混ぜられないため。
        if let Some((mode, adjust)) = p.balloon_keyword_base {
            world
                .entity_mut(char_window)
                .insert(BalloonKeywordBase { mode, adjust });
        }

        // ゴースト窓ペアの重なり宣言（areka-P0-ghost-window-zorder 要件 1.1／6.1・
        // design「areka / placement > spawn ペア宣言」）:
        // 「このバルーン窓はこのキャラ窓のすぐ手前に居るべき」を永続宣言として
        // **バルーン窓側にだけ**付ける（対になるキャラ窓は `peer` が指す）。
        //
        // 後付けなのは生成順の帰結である——`BalloonFollow.balloon` がバルーン entity を
        // 要するためバルーンを先に spawn しており、その時点でキャラ窓 entity はまだ無い。
        // 順序を入れ替えて「宣言のために」生成順を変えることはしない（上の OnDragEnd
        // 後付けと同じ形）。
        //
        // 宣言はスコープ内ペアにのみ張り、スコープ間には一切張らない——これが
        // 「スコープ間の上下関係を固定規則で決めない」（要件 3.1）と「是正時に当該
        // スコープの 2 窓しか動かさない」（要件 3.4）の構造的な根拠である。
        // 宣言を消費する確立系・維持系（wintf 側）の結線は main.rs が行う。
        world
            .entity_mut(balloon_window)
            .insert(KeepDirectlyAbove { peer: char_window });

        // 宣言の記録（要件 6.1）。wintf 側の各レコードは scope を持てない（wintf は
        // scope を知る層ではない）ため、**scope とペア両窓を結び付ける記録はここでしか
        // 出せない**。実機ログはこの行の entity を結合キーに wintf 側レコードへ辿る。
        log_zorder_pair_declared(p.scope, char_window, balloon_window);

        windows.insert(
            p.scope,
            ScopeWindows {
                char_window,
                balloon_window,
                // spawn へ渡る placements が resolver 既定である前提で `Some` を置く。
                // 保存位置が復元された場合は起動シーム（`main.rs`）が直後に
                // `clear_default_char_pos` で `None` へ落とす（scg 7.3）。
                default_char_pos: Some(p.char_pos),
            },
        );
    }

    let ghost_windows = GhostWindows { windows };
    world.insert_resource(ghost_windows.clone());
    ghost_windows
}

/// 全ゴースト窓共通の `WindowStyle`（DD13: `WS_EX_TOPMOST` を含めない＝既定
/// z-order 非 topmost・5.1。`WS_EX_LAYERED` は clickthrough トグルの同伴フラグ）。
fn window_style() -> WindowStyle {
    WindowStyle {
        style: WS_POPUP | WS_VISIBLE,
        ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW,
    }
}

/// 全ゴースト窓共通の位置権威宣言（areka-P0-dpi-window-vanish 要件 4.3・D3・task 5.1）。
///
/// # なぜ全ゴースト窓に要るのか
///
/// `WM_DPICHANGED` の OS 提案矩形は「モニタ間の DPI 比で現在位置を素直に拡縮した」
/// 位置であって、接地点規約（下端中央）とは無関係である。wintf 側はこの component が
/// **未付与の窓へは従来どおり提案位置を書き込む**（Per-Monitor v2 の標準応答＝
/// 非ゴースト窓の後方互換）。ゴースト窓の位置を決める権威は areka の配置系
/// （`project_anchor`／`resize_window_to`／DPI 相の再射影）ただ 1 つであり、
/// 二重ライターを許すと OS 由来座標が `WindowPos.position` へ landing して、
/// 以後の射影が「直前に areka が確定した接地点」ではなく OS 提示値を生位置として読む
/// （診断レポート §1.1 の連鎖①〜④＝S1。実機セッション①で `applied=true` が 84/84）。
///
/// 付与の責務が窓の所有者側にあること（wintf は読むだけ）は D3 の裁定である。
fn external_position_authority() -> DpiSuggestedRectPolicy {
    DpiSuggestedRectPolicy::ExternalAuthority
}

/// `ScopePlacement` 由来の位置・寸法（物理 px）だけを転記した `WindowPos`（U1）。
fn window_pos(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
    WindowPos {
        position: Some(Point { x, y }),
        size: Some(SizeI::new(w, h)),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// 重なり管理の結線（areka-P0-ghost-window-zorder task 3.2）
// ---------------------------------------------------------------------------

/// ゴースト窓ペアの重なり管理を World へ結線する
/// （areka-P0-ghost-window-zorder task 3.2・要件 1.1／5.6／6.1）。
///
/// 結線するのは 2 つである。
///
/// - **実行時ストラテジ** [`ZOrderPairStrategy`]。既定は案 A（Win32 の owner 関係を張り、
///   以後の重なりを OS に保証させる）・補助浮上なし。`Default` まかせにせず**値を明示して
///   挿入する**——実機ゲート（design.md「Plan A 実機可否ゲートとフォールバック分岐」）の
///   結果を反映するのはこの 1 箇所であり、どの案で動いているかが読む人の目に入る所に
///   無いと、切替が「既定値をどこかで変える」形に散る。挿入した当の値は
///   [`log_zorder_pair_strategy`] で起動時ログへも 1 行残す——判定表は spec 配下の文書で
///   あって、目の前のバイナリが本当にその結論どおりに動いているかは、バイナリ自身が
///   名乗る以外に確かめようがない（要件 5.6）。
/// - **確立系 → 維持系**を `FrameFinalize` へこの順で載せる（[`IntoScheduleConfigs::chain`]）。
///
/// # なぜ順序を付けるのか
///
/// 確立系（[`establish_owner_links`]）は owner を張った巡に再断行の要求
/// （[`ReassertZOrder`](wintf::ecs::ReassertZOrder)）を挿し、それを維持系
/// （[`apply_zorder_pair_maintenance`]）が消費して初期の隣接を確定させる。順序を付けないと
/// 消費が 1 巡遅れる。必要なのは順序だけではなく `chain` が置く**同期点**でもある——要求は
/// `Commands` 経由で挿さるため、同期点が無ければ同じ巡ではまだ実体に付いていない。
///
/// # なぜ確定段なのか
///
/// クリック透過の登録（[`register_ghost_windows_click_through`]）と同じ確定段だからである
/// （design.md「Implementation Notes」Integration——wintf 自身は schedule へ自動登録せず、
/// 載せる場所を決めるのは areka 側という既存の流儀）。どちらの system も Win32 を呼ぶため
/// UI スレッド固定であり、その担保は system 側の `NonSendMarker` が持つ。
///
/// 呼び手は main.rs の起動窓シーム（`open_startup_window`）で、`Schedules` 資源が既在の
/// World（`EcsWorld` 内 World）に対し、schedule 実行外で 1 回だけ同期に呼ぶ
/// （クリック透過登録と同じ作法）。
pub fn wire_zorder_pair(world: &mut World) {
    // 挿入と記録は**同じ束縛**から行う（要件 5.6・task 5.1 の観測条項）。値を 2 度書くと
    // 片方だけ変えたときに記録が静かに嘘をつく——起動時ログは「どの方式で動いているか」を
    // 名乗る唯一の手段ゆえ、嘘をつける形にしてはならない。
    let strategy = ZOrderPairStrategy::OwnerLink {
        raise_assist: false,
    };
    world.insert_resource(strategy);
    log_zorder_pair_strategy(strategy);
    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        (establish_owner_links, apply_zorder_pair_maintenance).chain(),
    );
}

// ---------------------------------------------------------------------------
// clickthrough 登録 system（task 5.2・6.1）
// ---------------------------------------------------------------------------

/// clickthrough 登録面の偽装境界（fake boundary）シーム。
///
/// 実体は wintf の [`ClickThroughRegistryHandle`]（NonSend・`WinApp::run` の
/// 結線で挿入）だが、その constructor は wintf 内部（pub(crate)）で headless
/// テストから構築できない。登録呼び出しの決定論的観測のため、登録面をこの
/// trait で抽象し、テストは偽 registrar（呼び出し記録）を NonSend として
/// 挿し込む（本 repo の偽装境界パターン）。
trait ClickThroughRegistrar: 'static {
    /// 監視対象窓（window Entity ＋ HWND）を登録する。
    fn register_window(&self, window: Entity, hwnd: HWND);
}

impl ClickThroughRegistrar for ClickThroughRegistryHandle {
    fn register_window(&self, window: Entity, hwnd: HWND) {
        self.register(window, hwnd);
    }
}

/// `Added<WindowHandle>` で [`GhostWindowMarker`] 窓を αマスク clickthrough
/// 機構へ登録する system（design「placement::spawn」正本 signature・6.1。
/// emo-present donor `register_click_through_windows` の一般化）。
///
/// WUC 化により ULW の自動 α ヒットテストが失われるため、機構が α を評価
/// できるよう placement 生成窓（キャラ窓・バルーン窓）を明示登録する。
/// `WindowHandle` は wintf の窓生成が HWND 生成後に付与するため
/// `Added<WindowHandle>` で「HWND が付いた瞬間」を捉え、各窓を厳密に 1 回
/// 登録する（`register` は同一 Entity 再登録を dedupe するため冪等でもある）。
/// `ClickThroughRegistryHandle` は `WinApp::run` の結線で NonSend リソース
/// として挿入される。ごく初期の tick で未挿入の可能性へ `Option` で防御する
/// （headless でも no-op で安全）。schedule への結線は main.rs シーム
/// `open_startup_window`（task 6.2）が `FrameFinalize` へ行う。
pub fn register_ghost_windows_click_through(
    new_windows: Query<(Entity, &WindowHandle), (With<GhostWindowMarker>, Added<WindowHandle>)>,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    register_ghost_windows_via(new_windows, handle);
}

/// [`register_ghost_windows_click_through`] の汎用実装（偽装境界）。
///
/// query filter（`GhostWindowMarker` × `Added<WindowHandle>`）ごとこの system
/// が production 経路の正体であり、公開 system は実 registrar 型
/// （[`ClickThroughRegistryHandle`]）を束縛した thin wrapper。型が一致しない
/// と wrapper が compile できないため、filter の乖離は型システムが防ぐ。
fn register_ghost_windows_via<R: ClickThroughRegistrar>(
    new_windows: Query<(Entity, &WindowHandle), (With<GhostWindowMarker>, Added<WindowHandle>)>,
    handle: Option<NonSend<R>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (entity, wh) in new_windows.iter() {
        handle.register_window(entity, wh.hwnd);
        debug!(?entity, "placement: クリック透過機構へゴースト窓を登録");
    }
}

#[cfg(test)]
#[path = "spawn_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "spawn_assembly_tests.rs"]
mod assembly_tests;
#[cfg(test)]
#[path = "spawn_cleanup_tests.rs"]
mod cleanup_tests;
#[cfg(test)]
#[path = "spawn_clickthrough_tests.rs"]
mod clickthrough_tests;
#[cfg(test)]
#[path = "spawn_follow_pipeline_tests.rs"]
mod follow_pipeline_tests;
#[cfg(test)]
#[path = "spawn_zorder_pair_export_tests.rs"]
mod zorder_pair_export_tests;
#[cfg(test)]
#[path = "spawn_zorder_pair_deferred_tests.rs"]
mod zorder_pair_deferred_tests;
#[cfg(test)]
#[path = "spawn_zorder_pair_wiring_tests.rs"]
mod zorder_pair_wiring_tests;
