//! ゴースト窓ペアの重なり関係——宣言・再断行要求・確立記録・実行時ストラテジ
//!
//! 「同一スコープにおいてバルーン窓は自分のキャラ窓のすぐ手前に居る」という不変条件
//! （要件 1.1）を、**entity 参照だけの宣言**として持つ層である。宣言する側（areka）は
//! scope を知る唯一の層であり、HWND も Win32 も知らない。反映する側（wintf）は HWND と
//! Win32 を知る唯一の層である——この責務分界ゆえ、宣言は wintf 側に住み、`Entity` を
//! 取る（wintf → areka の import は禁止ゆえ scope 型は受け取れない）。
//!
//! - `KeepDirectlyAbove`: 永続宣言（バルーン窓へ付与・peer はキャラ窓）
//! - `ReassertZOrder`: 重なりを断行し直す一回限りの要求（未適用／適用済み・検証待ちの 2 段階）
//! - `ExpectedOrder`: 適用済み要求が次巡で照合する期待隣接
//! - `OwnerLink`: 案 A の owner を張った事実の記録（切離しに使う）
//! - `ZOrderPairStrategy`: 案 A／案 B を実行時に切り替える設定
//!
//! - `decide_pair_fix`: 是正判断の純関数（実機・World 不要）
//! - 診断記録の語彙: 行を組む純関数（`*_line`）と、それを出すだけの `log_*`／`record_*`
//!
//! 確立系は兄弟モジュール [`super::zorder_pair_establish`]、維持系は
//! [`super::zorder_pair_maintain`]、記録の**行を組む純関数**は
//! [`super::zorder_pair_diag`] にある（本ファイルが 1,000 行の上限に迫るため分割した）。
//! **記録を出すマクロは本ファイル内に置くこと**——`tracing` の出力先は呼び出し元の
//! module path が既定であり、他ファイルへ移すとサインオフの grep 対象が分裂する。

use bevy_ecs::prelude::*;
use tracing::{debug, error, info, warn};
use windows::Win32::Foundation::HWND;

use super::zorder_pair_diag::{
    fix_line, hwnd_field, owner_detach_failed_line, owner_detached_line,
    owner_establish_failed_line, owner_established_line, sink_observed_line, skip_line,
    verify_failed_line,
};
use crate::api::{get_window_above, get_window_below, is_window_visible};

// ============================================================================
// KeepDirectlyAbove - ペア関係の永続宣言
// ============================================================================

/// ペア宣言（バルーン窓へ付与・peer はキャラ窓 entity）。
/// 「この窓は peer 窓のすぐ手前に居るべき」の永続宣言。
///
/// 付与するのは scope を知る層（areka の spawn）。宣言は片側（手前に居るべき窓＝
/// バルーン窓）にのみ付き、対になる窓は `peer` で指す。スコープ間には宣言を張らない
/// ——これが「スコープ間の上下関係を固定の規則で決めない」（要件 3.1）と「是正時に
/// 当該スコープの 2 窓しか動かさない」（要件 3.4）の構造的な根拠である。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepDirectlyAbove {
    pub peer: Entity,
}

// ============================================================================
// ExpectedOrder / ReassertZOrder - 再断行の一時要求と検証段階
// ============================================================================

/// 適用済みの再断行要求が**次巡で照合する期待隣接**。
///
/// 「`above` が `below` のすぐ手前に居る」——是正の 2 つの腕
/// （バルーンを手前へ／キャラを直後へ）はどちらも最終的にこの同一の隣接へ収束するため、
/// 期待値はこの 1 形で足りる。照合は `measure_window_below(above) == Some(below)`
/// （`GW_HWNDNEXT` を辿り、不可視の窓を読み飛ばした**最も近い可視の背後**）で行い、
/// 不一致なら `verify-failed` を `error!` で記録する（要件 6.2）。同レコードの `expected` が
/// この値、`measured` が実測値である。
///
/// 座標・寸法のフィールドは**持たない**——是正は表示順のみを動かす（要件 1.6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedOrder {
    /// 手前側に居るべき窓（通常はバルーン窓）
    pub above: HWND,
    /// 背後側に居るべき窓（通常はキャラ窓）
    pub below: HWND,
}

// SAFETY: `ExpectedOrder` は 2 つの `HWND`（windows-rs では `*mut c_void` の newtype）を
// 保持するため、自動では Send/Sync が導出されない（windows-rs 0.62.2 は HWND に
// Send/Sync を実装しない）。よってこの手動 impl は冗長ではなく必須である
// （これが無いと [`ReassertZOrder`] が `Component` の `Send + Sync` 要求を満たせない）。
// 健全性: ここでの HWND は**期待隣接を記録した値**であり、窓の所有権も解放責務も伴わない
// 不透明なウィンドウ識別子にすぎない（読み書きは値のコピーのみ）。この値を実際に Win32 へ
// 渡すのは維持系の system であり、Win32 を呼ぶ system は NonSend パラメータで UI スレッド
// 固定されている——他スレッドが HWND を用いて窓を操作する経路は存在しない。
// `ZOrder::InsertAfter(HWND)` と同根の crate 標準の HWND 取り扱い方針。
unsafe impl Send for ExpectedOrder {}
unsafe impl Sync for ExpectedOrder {}

/// 重なりの再断行の一時要求（one-shot）。挿入元:
///   ① `establish_owner_links`（初期隣接の確定）
///   ② balloon-visibility（要件 2.6: show 後に挿入）★相互登記の契約点
///   ③ `WM_WINDOWPOSCHANGED` z 変化検知（案 B の B2）
/// 維持系が消費し、適用→検証の完了で remove する。
///
/// 段階は `pending_verify` が表す——`None` は「挿入済み・未適用」、`Some(_)` は
/// 「適用済みで実測検証待ち」。検証を次巡へ遅らせるのは、適用（`SetWindowPosCommand`）の
/// flush が tick 後であり、同巡の実測では指令が反映されていないためである。
/// この 2 段階が「指令は出したが効かなかった」を切り分ける（要件 6.2）。
///
/// 他クレート（表示制御を所有する仕様）から挿入されるため公開到達性を持つ。
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReassertZOrder {
    /// 適用済みで実測検証待ち（expected は検証時の期待隣接）
    pub pending_verify: Option<ExpectedOrder>,
}

// NOTE: `ReassertZOrder` 自身に手動 Send/Sync は不要——`Option<ExpectedOrder>` は
// [`ExpectedOrder`] の Send/Sync を通じて自動導出される。冗長な `unsafe` は置かない。

// ============================================================================
// OwnerLink - 案 A の owner 確立記録
// ============================================================================

/// 案 A の owner 確立済み記録（バルーン窓へ付与・切離しに使う）。
///
/// `set_window_owner` が成功した事実そのものであり、再確立の抑止（冪等性）と、
/// 破棄経路での `clear_window_owner` 呼出に使う（要件 5.9）。owner を張れなかった
/// ペアにはこの記録が付かない——「張った」と「張っていない」がここで区別される。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerLink {
    pub owner_hwnd: HWND,
}

// SAFETY: `OwnerLink` は `HWND`（`*mut c_void` の newtype）を保持するため、自動では
// Send/Sync が導出されず、この手動 impl は `Component`（ECS は Send+Sync を要求）に
// するために必須である。健全性: 保持するのは owner 窓の不透明な識別子の**値の写し**で
// あり、所有権も破棄責務も持たない（窓の破棄は `WindowRegistry`／`Window` drop が担う）。
// この HWND を Win32 へ渡すのは owner 確立系・切離し系であり、いずれも NonSend パラメータで
// UI スレッド固定された system である。`ZOrder::InsertAfter(HWND)` と同根。
unsafe impl Send for OwnerLink {}
unsafe impl Sync for OwnerLink {}

// ============================================================================
// ZOrderPairStrategy - 実行時ストラテジ
// ============================================================================

/// 実行時ストラテジ（areka main が挿入。既定は `OwnerLink { raise_assist: false }`）。
///
/// 案 A（Win32 owner の OS 保証）を本線とし、実機ゲートで要件 5.1〜5.5 の毀損が
/// 判明した場合のみ案 B へ切り替える（要件 5.6）。宣言の付与コードは両案で同一であり、
/// 切り替わるのは wintf 側の反映手段だけである。
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZOrderPairStrategy {
    /// 案 A: owner 保証。`raise_assist` はゲート G7 FAIL 時のみ true（要件 1.3 の明示実装）
    OwnerLink { raise_assist: bool },
    /// 案 B: B2 検知＋B3 同乗の明示維持
    ExplicitMaintenance,
}

impl Default for ZOrderPairStrategy {
    /// 既定は案 A・raise assist 無効（design.md「State（コンポーネント契約）」）。
    ///
    /// バリアントがフィールドを持つため `#[derive(Default)]` の `#[default]` 属性は
    /// 使えず、手書きの impl とする。
    fn default() -> Self {
        Self::OwnerLink {
            raise_assist: false,
        }
    }
}

// ============================================================================
// decide_pair_fix - 是正判断の純関数
// ============================================================================

/// 判断を起こしたトリガの種別。
///
/// z 変化の 2 種は**どちらの窓の重なりが動いたか**で分ける（design.md
/// 「WM_WINDOWPOSCHANGED z 変化検知」の正準定義）。動いた窓が手前側（バルーン）なら
/// [`PairTrigger::RaisedAbove`]、背後側（キャラ）なら [`PairTrigger::RaisedBelow`] である
/// ——同じ「z が動いた」でも要件 1.3 と要件 1.2 のどちらの腕かが変わるため、
/// ここを 1 種へ潰してはならない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// 組み立てているのは維持系の `Reassert` だけである。残る 3 種の供給者は後続タスク
// （z 変化検知＝案 B／補助浮上）が入れるまで存在しない。
#[allow(dead_code)]
pub(crate) enum PairTrigger {
    /// owner 確立直後の初期隣接の確定
    Establish,
    /// 一回限りの再断行要求（[`ReassertZOrder`]・要件 2.6 のシームを含む）
    Reassert,
    /// 手前側（バルーン窓）の重なりが動いた
    RaisedAbove,
    /// 背後側（キャラ窓）の重なりが動いた
    RaisedBelow,
}

/// 是正を見送った理由（要件 6.3——理由の無い見送りを型として作らせない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipReason {
    /// 既にバルーンがキャラのすぐ手前に居る（収束の同値ガード）
    AlreadyAdjacent,
    /// ペアの一方が既に存在しない（要件 1.5——残る窓には一切書き込まない）
    PeerMissing,
    /// 両窓とも生きているが、まだ OS のハンドルが付いていない
    HandleMissing,
    /// 自分が出した指令の反響、ないし当該ストラテジでは是正の要らない変化
    EchoOrIrrelevant,
    /// 現在のストラテジがこのトリガを扱わない構成になっている
    StrategyDisabled,
}

/// 是正判断の入力。実測値は呼出側（維持系）が同一巡で採取したものを渡す。
///
/// 本構造体は Win32 を呼ばない——「いま何が起きていて、実際の重なりはどうだったか」の
/// 写しだけを持つ。これが判断を実機不要の決定論的テストで固定できる根拠である（要件 7.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PairObservation {
    /// 判断を起こしたトリガ種別
    pub trigger: PairTrigger,
    /// 実行時ストラテジ（案 A／案 B・raise assist の有無）
    pub strategy: ZOrderPairStrategy,
    /// 手前側（バルーン窓）の entity と [`WindowHandle`](super::WindowHandle) の生存
    pub above_alive: bool,
    /// 背後側（キャラ窓）の生存
    pub below_alive: bool,
    /// 手前側の HWND（未付与は `None`）
    pub above_hwnd: Option<HWND>,
    /// 背後側の HWND（未付与は `None`）
    pub below_hwnd: Option<HWND>,
    /// 手前側の**最も近い可視の背後**にある窓（`GW_HWNDNEXT` を辿り不可視窓を読み飛ばした実測）。
    ///
    /// 「1 つ背後」ではない——OS が owner の直上に置く不可視の補助窓（スレッド既定の IME 窓）が
    /// 挟まると、配置が正しくても隣接が成立しなくなるためである（実測層 [`measure_window_below`]
    /// の doc に根拠）。判断側の扱いはこの値の意味が変わっても一切変わらない。
    pub measured_below_of_above: Option<HWND>,
    /// 背後側の**最も近い可視の手前**にある窓（`GW_HWNDPREV` を辿り不可視窓を読み飛ばした実測）。
    ///
    /// 意味の変化の理由は [`Self::measured_below_of_above`] と同じ。この値はそのまま
    /// 是正の挿入位置にもなる（[`measure_window_above`] の doc を参照）。
    pub measured_above_of_below: Option<HWND>,
}

/// 是正の挿入位置。
///
/// `SetWindowPos` の `hWndInsertAfter` は**指定窓の背後**へ対象を置くため、
/// 「キャラのすぐ手前」を作る挿入位置はキャラ自身ではなく**キャラの最も近い可視の手前の窓**である。
/// その窓が居ない（キャラより手前に可視の窓が無い）縁だけが [`InsertSpec::TopEdge`] になる。
///
/// バリアントは 2 つで尽きており、**常時最前面（`ZOrder::TopMost`）を表す値は無い**
/// ——要件 4.3／8.1 はこの型の形そのもので満たされる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InsertSpec {
    /// 指定窓のすぐ背後へ（`ZOrder::InsertAfter` へ写像）
    After(HWND),
    /// 最上位へ（`ZOrder::Top` へ写像。常時最前面ではない）
    TopEdge,
}

/// 是正判断の結果。
///
/// 座標・寸法のフィールドを**一切持たない**——是正は表示順のみを動かすという要件 1.6 を、
/// 実行時の判定ではなく型の形で保証する。動かす窓も宣言ペアの 2 枚のいずれか 1 枚に
/// 限られる（腕の種別がそのまま対象を決め、第三の窓を指す余地が無い＝要件 3.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairFixDecision {
    /// 何もしない（理由必須・要件 6.3）
    Skip(SkipReason),
    /// バルーンをキャラのすぐ手前へ（動かすのは `above_hwnd`・要件 1.2）
    PlaceAboveOverBelow { insert_after: InsertSpec },
    /// キャラをバルーンのすぐ背後へ（動かすのは `below_hwnd`・要件 1.3）
    PlaceBelowUnderAbove { insert_after: HWND },
}

/// ペアの重なりをどう是正するかの純判断（Win32 も World も触らない）。
///
/// 判定は次の順で行う——先に置いた歯止めほど強い。
///
/// 1. **生存**: 一方でも欠けていれば [`SkipReason::PeerMissing`]。残る窓には触れない（要件 1.5）。
/// 2. **ハンドル**: 生きていても HWND 未付与なら [`SkipReason::HandleMissing`]。
/// 3. **隣接**: 既に隣接していれば必ず [`SkipReason::AlreadyAdjacent`]。適用が誘発する
///    z 変化で再是正が走る往復を断つ同値ガードであり、トリガ種別より優先する。
/// 4. **トリガ種別×ストラテジ**: 下表のとおり。
///
/// | トリガ | 案 A（raise assist 無効） | 案 A（有効） | 案 B |
/// |---|---|---|---|
/// | `Establish` / `Reassert` | 手前へ | 手前へ | 手前へ |
/// | `RaisedAbove` | `StrategyDisabled` | 背後へ（要件 1.3） | 背後へ |
/// | `RaisedBelow` | `StrategyDisabled` | `EchoOrIrrelevant` | 手前へ（要件 1.2） |
///
/// 案 A で `RaisedBelow` を見送るのは、キャラ（owner）の浮上には OS が被 owner の
/// バルーンを連れて上がるためである（ゲート G6 の保証）——是正は要らない。
/// 案 A・raise assist 無効ではそもそも z 変化検知を結線しないが、万一トリガが届いても
/// 判断側で止まる形にしておく（供給者の有無に判断の正しさを依存させない）。
//
// 呼び出しは維持系（[`apply_zorder_pair_maintenance`](super::apply_zorder_pair_maintenance)）。
// 案 B の `compute_pair_z_intent` も同じ関数を呼ぶ（判断の一元点は 1 つに保つ）。
pub(crate) fn decide_pair_fix(obs: &PairObservation) -> PairFixDecision {
    if !obs.above_alive || !obs.below_alive {
        return PairFixDecision::Skip(SkipReason::PeerMissing);
    }
    let (Some(above), Some(below)) = (obs.above_hwnd, obs.below_hwnd) else {
        return PairFixDecision::Skip(SkipReason::HandleMissing);
    };

    // 同値ガード。正準の判定は「手前側の最も近い可視の背後が背後側か」（design.md Invariants）だが、
    // 逆向きの実測が隣接を示す場合も併せて止める——そちらだけが隣接を示すのは 2 つの実測が
    // 食い違った異常であり、そのまま是正へ流すと挿入位置が動かす窓自身になってしまう。
    if obs.measured_below_of_above == Some(below) || obs.measured_above_of_below == Some(above) {
        return PairFixDecision::Skip(SkipReason::AlreadyAdjacent);
    }

    // 「バルーンをキャラのすぐ手前へ」の挿入位置は、キャラの最も近い可視の手前の窓。
    // その窓が居なければ（可視の窓としては）キャラが最上位であり、最上位へ置くほか無い。
    //
    // 読み飛ばした結果ここに入るのは可視の窓だが、それで正しい——`SetWindowPos` の
    // `hWndInsertAfter` はその窓の**すぐ背後**へ対象を置くので、間に挟まっていた不可視窓
    // （OS が owner の直上に置く補助窓）より**手前**に入る。
    let place_above = PairFixDecision::PlaceAboveOverBelow {
        insert_after: match obs.measured_above_of_below {
            Some(front_of_below) => InsertSpec::After(front_of_below),
            None => InsertSpec::TopEdge,
        },
    };

    // 上表そのままの組で尽くす（トリガ・ストラテジのどちらかにバリアントが増えれば
    // ここがコンパイルエラーになり、決め忘れが黙って既定へ落ちない）。
    match (obs.trigger, obs.strategy) {
        (PairTrigger::Establish | PairTrigger::Reassert, _) => place_above,
        (
            PairTrigger::RaisedAbove,
            ZOrderPairStrategy::OwnerLink { raise_assist: true }
            | ZOrderPairStrategy::ExplicitMaintenance,
        ) => PairFixDecision::PlaceBelowUnderAbove {
            insert_after: above,
        },
        (PairTrigger::RaisedBelow, ZOrderPairStrategy::ExplicitMaintenance) => place_above,
        (PairTrigger::RaisedBelow, ZOrderPairStrategy::OwnerLink { raise_assist: true }) => {
            PairFixDecision::Skip(SkipReason::EchoOrIrrelevant)
        }
        (
            PairTrigger::RaisedAbove | PairTrigger::RaisedBelow,
            ZOrderPairStrategy::OwnerLink {
                raise_assist: false,
            },
        ) => PairFixDecision::Skip(SkipReason::StrategyDisabled),
    }
}

// ============================================================================
// 診断記録の語彙（要件 6）
// ============================================================================
//
// design.md「診断ログ語彙（要件 6）」のレコード表が正本である。本節はその表を
// そのまま写した 6 種の記録を出す手段を提供する（呼び出す結線は確立系・維持系＝
// 後続タスクの担当）。
//
// # レコード表のうち `declared` が本節に無い理由
//
// `declared` の必須フィールドは scope・キャラ窓・バルーン窓であり、scope を知る層は
// areka だけである（wintf → areka の import は禁止ゆえ scope 型を受け取る口も作れない）。
// ゆえに `declared` の出力点は areka の窓生成であり、本節が持つのは wintf が自力で
// 全フィールドを埋められる 6 種に限る。scope は `declared` レコードとの entity 結合で
// 2 段 grep する（`log_window_move` の先例）。
//
// # 行は純関数が組み、ログはそれを出すだけ
//
// サインオフの grep 判定語と出力書式は 1:1 で対応する。書式が意図せず変われば手順が
// 静かに嘘になるため、組立を純関数（`*_line`）へ切り出してテストで語彙を固定し、
// `log_*`／`record_*` はその戻り値をそのまま本文にする（組立を二重に持たない）。
// その純関数群は兄弟の [`zorder_pair_diag`](super::zorder_pair_diag) にある——
// マクロを 1 つも含まない文字列組立だけをあちらへ置き、**マクロを呼ぶ側は本ファイルに
// 残す**（`tracing` の出力先は呼び出し元の module path が既定であり、移すとサインオフの
// grep 対象が分裂する）。
//
// # 出力先と水準
//
// target は module path 既定（`wintf::ecs::window::zorder_pair`）——サインオフの
// `RUST_LOG` はこの名前で点灯させる。水準はレコード表どおりで、失敗と検証不一致だけが
// error、確立成功が info、残りは診断専用の debug（既定運転では無音）である。

impl PairFixDecision {
    /// 是正の腕が指す挿入位置（見送りには挿入位置が無い＝`None`）。
    ///
    /// 2 つの腕は動かす窓こそ違うが、`SetWindowPos` へ渡す挿入位置はどちらも
    /// 「ある窓の背後」か「最上位」に収まるため 1 つの型で表せる。
    ///
    /// # 何のための射影か
    ///
    /// 是正の記録（[`record_verification`] の一致側）は「出した指令」として挿入位置を
    /// 載せるが、その記録が出るのは適用の**次巡**である。よって維持系（タスク 2.2）は
    /// 適用時に判断からこの値を取り出して持ち越し、次巡の検証で
    /// [`record_verification`] の `insert_after` 引数へ渡す——本メソッドはその
    /// 「判断 → 記録に載る指令」の射影であり、記録側が判断の型を知らずに済む境界でもある。
    /// 持ち越しは維持系が
    /// [`IssuedPairFix`](super::IssuedPairFix) として entity へ預ける。
    pub(crate) fn insert_spec(&self) -> Option<InsertSpec> {
        match self {
            PairFixDecision::Skip(_) => None,
            PairFixDecision::PlaceAboveOverBelow { insert_after } => Some(*insert_after),
            PairFixDecision::PlaceBelowUnderAbove { insert_after } => {
                Some(InsertSpec::After(*insert_after))
            }
        }
    }
}

/// ペアの相手が失われた形（切離しの記録に載る）。
///
/// 実体ごと消えたのか、実体は残ってハンドルだけが外れたのかを分けるのは、破棄経路の
/// どの段階で切離しが走ったのかが事後に読めるようにするためである（design.md「破棄経路」）。
/// 1 語へ潰すと、この 2 つが記録の上で見分けられなくなる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeerLoss {
    /// 相手の実体が消えた（despawn 済み）
    Despawned,
    /// 実体は残っているが OS のハンドルが外れた
    HandleRemoved,
}

// ============================================================================
// 実測層——隣接は「最も近い可視の隣」で測る
// ============================================================================
//
// # なぜ 1 歩ではなく可視の隣なのか（実機で確定した根拠）
//
// スレッド既定の IME 窓（クラス `IME`／タイトル `Default IME`／**不可視**／矩形 0x0）は
// キャラ窓を owner に持ち、Windows は被 owner 窓を owner の手前に保つ。バルーン窓も同じ
// キャラ窓を owner に持つ被 owner 窓であるため、キャラ窓の直上には被 owner 窓が 2 枚並び、
// その内部順序は我々の制御外である。ゆえに `GetWindow` を 1 歩だけ辿る測り方では、
// **配置が正しくても隣接判定が落ちる**（実機で毎回 `verify-failed` が出ていた形）。
//
// 要件 1.2 の趣旨は Introduction の「会話が読める」＝**可視の遮蔽**であり、可視の窓だけを
// 見ればバルーンはキャラのすぐ手前に居る。よって走査は不可視の窓を読み飛ばし、
// **最も近い可視の隣**を返す（測り方の裁定は requirements.md 受入基準 1.2 の但し書き）。

/// 兄弟列を辿る向き（実測層の内部語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NeighborSide {
    /// 手前側（前面へ向かう＝`GW_HWNDPREV`）
    Above,
    /// 背後側（背面へ向かう＝`GW_HWNDNEXT`）
    Below,
}

impl NeighborSide {
    /// 隣へ 1 歩進む（可視・不可視は問わない生の隣）。
    fn step(self, hwnd: HWND) -> windows::core::Result<Option<HWND>> {
        match self {
            NeighborSide::Above => get_window_above(hwnd),
            NeighborSide::Below => get_window_below(hwnd),
        }
    }

    /// 失敗・打切りの記録に載る向きの語。
    fn label(self) -> &'static str {
        match self {
            NeighborSide::Above => "手前",
            NeighborSide::Below => "背後",
        }
    }
}

/// 兄弟列を何枚まで辿るかの上限（両向き共通）。
///
/// 兄弟列が壊れて環になっている場合や、不可視の窓が延々と続く場合に無限に辿らないための
/// 歯止めである。デスクトップのトップレベル窓がこの枚数を超えることは実運用では起きないが、
/// 超えた場合は打切りとして記録に残し、値は**不在（番兵）へ倒す**——測れなかったことを
/// 偽の失敗にすり替えない（沈降観測の走査もこの上限を共有する）。
const SIBLING_SCAN_LIMIT: usize = 512;

/// 指定窓から `side` の向きへ辿り、**最も近い可視の窓**を実測する。
///
/// 記録に載る実測値の型は `Option<HWND>` であり、「その向きに可視の窓が居ない」と
/// 「走査に失敗した／上限に達した」はどちらも不在（記録では番兵 `-`）になる。値として
/// この 2 つを分けないのは、載せる先のフィールドが 1 つだからである——**代わりに失敗と
/// 打切りそのものを必ず記録へ残す**ことで、番兵が出た理由が事後に読めるようにする
/// （[areka-log-first-no-silent-failure]）。
///
/// 失敗の記録に `[zorder-pair] ...` のタグを付けないのは、サインオフの grep 判定語が
/// design.md「診断ログ語彙（要件 6）」のレコード表で閉じているためである（判定語を
/// 勝手に増やさない）。出力先は本モジュール既定のままで、水準は warn——
/// 走査が失敗しても是正そのものは続行でき、利用者の操作は妨げられない（要件 6.4）。
fn measure_nearest_visible(hwnd: HWND, side: NeighborSide) -> Option<HWND> {
    let mut cursor = hwnd;
    for _ in 0..SIBLING_SCAN_LIMIT {
        match side.step(cursor) {
            // 可視の窓に当たった時点で終わり——これが「最も近い可視の隣」である。
            Ok(Some(found)) if is_window_visible(found) => return Some(found),
            // 不可視の窓は会話を遮らないので隣とは数えず、その先を見る。
            Ok(Some(found)) => cursor = found,
            // その向きにもう窓が無い（正常な不在）。
            Ok(None) => return None,
            Err(err) => {
                warn!(
                    "窓の最も近い可視の{side}の走査に失敗しました（記録には不在として載ります） hwnd={hwnd} error={code:?} message={message}",
                    side = side.label(),
                    hwnd = hwnd_field(Some(cursor)),
                    code = err.code(),
                    message = err.message(),
                );
                return None;
            }
        }
    }
    warn!(
        "窓の最も近い可視の{side}の走査が上限に達しました（記録には不在として載ります） hwnd={hwnd} limit={SIBLING_SCAN_LIMIT}",
        side = side.label(),
        hwnd = hwnd_field(Some(hwnd)),
    );
    None
}

/// 指定窓の**最も近い可視の手前**にある窓を実測する。
///
/// 不可視の窓を読み飛ばす理由と、失敗・打切りを不在へ倒す約束は
/// [`measure_nearest_visible`] の doc を参照。
///
/// この値は「バルーンをキャラのすぐ手前へ」の**挿入位置**にもなる
/// （[`decide_pair_fix`] の `place_above`）。読み飛ばした結果として
/// `SetWindowPos` の `hWndInsertAfter` へ渡るのは可視の窓だが、それで正しい——
/// 対象はその可視窓の**すぐ背後**、すなわち間に挟まっていた不可視窓より**手前**へ入る。
pub(crate) fn measure_window_above(hwnd: HWND) -> Option<HWND> {
    measure_nearest_visible(hwnd, NeighborSide::Above)
}

/// 指定窓の**最も近い可視の背後**にある窓を実測する。
///
/// 向きが逆であることを除けば [`measure_window_above`] と同じ約束である。
///
/// こちらは**適用後の検証**が使う向きである——「手前側の最も近い可視の背後が背後側の窓か」が
/// 隣接の正準判定であり（design.md Invariants）、[`record_verification`] の
/// `measured_below_of_above` に入るのがこの値である。
pub(crate) fn measure_window_below(hwnd: HWND) -> Option<HWND> {
    measure_nearest_visible(hwnd, NeighborSide::Below)
}

/// 兄弟列を前面側へ辿った結果（沈降観測が使う）。
///
/// 「前面窓より背面に居るか」は 1 回の問い合わせでは分からない——Win32 に 2 つの窓の
/// 前後を直接比べる API は無く、隣を 1 枚ずつ辿るほかない。よって走査の**結果そのもの**を
/// 値にして、判定は純関数へ渡す（実機不要の決定論的テストで判定を固定するため）。
pub(crate) struct FrontScan {
    /// 対象より手前に居た**可視の**窓（手前へ向かう順）。
    ///
    /// 不可視の窓を入れないのは実測層の他の関数と同じ理由である（可視の窓だけを見る）。
    /// 判定に使う前面窓は必ず可視なので、除いても「前面窓が手前に居るか」は変わらない。
    pub windows: Vec<HWND>,
    /// 最前面まで辿り着いたか。
    ///
    /// `false` は走査の失敗ないし上限による打切りであり、**列が不完全**であることを意味する。
    /// この区別が無いと「探したが見つからなかった」と「最後まで探せなかった」が同じ字面に
    /// なり、測れなかっただけの巡に偽の判定が出る。
    pub reached_top: bool,
}

/// 指定窓より手前に居る**可視の**窓を、手前へ向かって辿って集める。
///
/// [`measure_window_above`] を繰り返すのではなく `get_window_above` を直に使うのは、
/// あちらが**走査の失敗と「手前に可視の窓が居ない」を同じ `None` へ潰す**ためである。
/// 沈降観測では両者を混同できない——失敗を「最前面まで辿った」と扱うと、前面窓を
/// 見つけられなかっただけの巡に「背面に居ない」という偽の失敗が記録される。
/// 失敗は記録に残したうえで [`FrontScan::reached_top`] を `false` にする。
///
/// 上限は実測層と共有する（[`SIBLING_SCAN_LIMIT`]）。
pub(crate) fn measure_windows_in_front(hwnd: HWND) -> FrontScan {
    let mut windows = Vec::new();
    let mut cursor = hwnd;
    for _ in 0..SIBLING_SCAN_LIMIT {
        match get_window_above(cursor) {
            Ok(Some(found)) => {
                // 不可視の窓は列に入れない（可視の窓だけを見る＝実測層と同じ規則）。
                if is_window_visible(found) {
                    windows.push(found);
                }
                cursor = found;
            }
            Ok(None) => {
                return FrontScan {
                    windows,
                    reached_top: true,
                };
            }
            Err(err) => {
                warn!(
                    "手前側の走査に失敗しました（前面窓との相対は判定しません） hwnd={hwnd} error={code:?} message={message}",
                    hwnd = hwnd_field(Some(cursor)),
                    code = err.code(),
                    message = err.message(),
                );
                return FrontScan {
                    windows,
                    reached_top: false,
                };
            }
        }
    }
    warn!(
        "手前側の走査が上限に達しました（前面窓との相対は判定しません） hwnd={hwnd} limit={SIBLING_SCAN_LIMIT}",
        hwnd = hwnd_field(Some(hwnd)),
    );
    FrontScan {
        windows,
        reached_top: false,
    }
}

/// owner 確立の記録を出す（info・要件 6.1）。
pub(crate) fn log_owner_established(
    entity: Entity,
    peer: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    measured_prev: Option<HWND>,
) {
    info!(
        "{}",
        owner_established_line(entity, peer, owned_hwnd, owner_hwnd, measured_prev)
    );
}

/// owner 確立失敗の記録を出す（error・要件 6.2）。
///
/// 記録するだけで再試行はしない——同じハンドルでの再試行は同じ失敗を繰り返し、
/// 記録を二重化するだけである（design.md「Error Strategy」）。再試行を抑える印は
/// 呼び出し側（確立系）が付ける（[`OwnerEstablishFailed`](super::OwnerEstablishFailed)）。
pub(crate) fn log_owner_establish_failed(entity: Entity, error: &windows::core::Error) {
    error!("{}", owner_establish_failed_line(entity, error));
}

/// owner 切離しの記録を出す（debug・破棄経路）。
///
/// `[zorder-pair] ...` のタグを付けないのは、サインオフの grep 判定語が design.md
/// 「診断ログ語彙（要件 6）」のレコード表で閉じているためである（判定語を勝手に増やさない）。
/// 切離しは表示順の調整ではなく破棄の後始末であり、レコード表のどの行にも当たらない。
/// 水準が debug なのは、ゴーストの終了ごとに必ず起きる正常な出来事だからである
/// （既定運転では無音・サインオフの `RUST_LOG` では見える）。
pub(crate) fn log_owner_detached(
    entity: Entity,
    peer: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    loss: PeerLoss,
) {
    debug!(
        "{}",
        owner_detached_line(entity, peer, owned_hwnd, owner_hwnd, loss)
    );
}

/// owner 切離し失敗の記録を出す（error）。
///
/// 切離せなければ OS の破棄カスケードが残り、要件 5.9 を構造的に満たせなくなる
/// ——沈黙させてはならない失敗である（[areka-log-first-no-silent-failure]）。
/// それでも `Err` を伝播させず記録に留めるのは、破棄の途中で利用者の操作を止めないため
/// である（要件 6.4）。タグを付けない理由は [`log_owner_detached`] と同じ。
pub(crate) fn log_owner_detach_failed(
    entity: Entity,
    peer: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    error: &windows::core::Error,
) {
    error!(
        "{}",
        owner_detach_failed_line(entity, peer, owned_hwnd, owner_hwnd, error)
    );
}

/// 沈降観測の記録を出す（debug・要件 4.4／7.5）。
///
/// 出すのは非活性化の**次の巡**であり、出す主体は維持系
/// （[`observe_marked_pair_sinks`](super::zorder_pair_sink::observe_marked_pair_sinks)）である。
/// 非活性化の瞬間に出さない理由は当該モジュールの doc を参照（活性化の途中で測ると
/// 偽の失敗を記録するため）。
pub(crate) fn log_sink_observed(
    entity: Entity,
    adjacency_ok: bool,
    foreground: Option<HWND>,
    behind_foreground: Option<bool>,
) {
    debug!(
        "{}",
        sink_observed_line(entity, adjacency_ok, foreground, behind_foreground)
    );
}

/// 判断結果を記録し、**適用すべき是正だけ**を返す（要件 6.3）。
///
/// 見送りはこの関数の中で必ず理由つきの記録になり、返り値としては何も返らない。
/// 返るのは常に是正の腕であり、見送りは決して返らない。
///
/// # これは規約であって構造保証ではない
///
/// [`decide_pair_fix`] は `pub(crate)` で [`PairFixDecision::Skip`] をそのまま返すため、
/// 本関数を通さずに判断結果を握り潰す書き方は**言語上は可能**である。`pub(crate)` は
/// クレート内のどのモジュールからも呼べる可視性であり、維持系（タスク 2.2）を別ファイルへ
/// 置いても塞げない——狭めるには戻り値の型を変えるしかない（下記）。したがってここにあるのは
/// **「判断結果を適用へ渡すときは必ず本関数を経由する」という規約**であり、
/// 記録の無い見送りが起きないことは 2.2 がこの規約を守ることに依存している。
///
/// # 型で塞ぐ案を採らない理由
///
/// [`decide_pair_fix`] の戻り値を「本関数でしか開けない型」（下位モジュールに置いた
/// フィールド非公開の包み）で返せば構造保証になるが、採らない。design.md
/// 「Service Interface（純関数）」は当該関数の署名を
/// `fn decide_pair_fix(obs: &PairObservation) -> PairFixDecision` と明記しており、
/// この形はタスク 1.3 でレビュー承認済みである。診断語彙の都合で承認済みの判断関数の
/// 署名を変えるのは、変更の理由と場所が食い違う。規約側で守り、逸脱はテスト
/// （見送り 5 種が理由つきで記録されること）とレビューで検出する。
///
/// 確立系（[`establish_owner_links`](super::establish_owner_links)）も見送りをここへ通す
/// ——確立は [`decide_pair_fix`] を経由しない判断だが、**見送りの記録語彙は 1 つ**であり、
/// 理由も既存の 5 種で尽きるため、入口を分けない。
pub(crate) fn record_decision(
    entity: Entity,
    peer: Entity,
    decision: PairFixDecision,
) -> Option<PairFixDecision> {
    match decision {
        PairFixDecision::Skip(reason) => {
            debug!("{}", skip_line(entity, peer, reason));
            None
        }
        fix => Some(fix),
    }
}

/// ペア宣言を持たない窓に付いていた再断行の要求を落としたことを記録する（warn）。
///
/// [`ReassertZOrder`] は他クレートから挿入される公開の契約点であり、宣言の無い窓へ
/// 付けることも言語上は可能である。そのまま読み飛ばすと、要求が entity に残り続けて
/// 毎巡黙って無視される——**記録の無い見送り**になる（要件 6.3 が禁じる形）。
/// よって記録して落とす。
///
/// `[zorder-pair] ...` のタグを付けないのは、サインオフの grep 判定語が design.md
/// 「診断ログ語彙（要件 6）」のレコード表で閉じているためである（判定語を勝手に増やさない）。
/// 水準が warn なのは、挿入元の誤りであって表示順そのものは壊れていないからである。
pub(crate) fn log_orphan_request_dropped(entity: Entity) {
    warn!(
        "ペア宣言の無い窓に再断行の要求が付いていたため落としました（是正は行いません） entity={entity:?}"
    );
}

/// ペア宣言を持たない窓に残っていた沈降観測の目印を落としたことを記録する（warn）。
///
/// 目印は宣言を持つ窓にしか付かないが、付いてから次の巡までに宣言が外れる余地はある。
/// そのまま読み飛ばすと目印が entity に残り続けて毎巡黙って無視される——**記録の無い
/// 見送り**になる（要件 6.3 が禁じる形）。よって記録して落とす。タグを付けない理由は
/// [`log_orphan_request_dropped`] と同じ（サインオフの grep 判定語を増やさない）。
pub(crate) fn log_orphan_sink_mark_dropped(entity: Entity) {
    warn!(
        "沈降観測の目印がペア宣言の無い窓に付いていたため落としました（観測は行いません） entity={entity:?}"
    );
}

/// 出した指令の持ち越しが無い検証待ちの要求を落としたことを記録する（warn）。
///
/// 維持系は適用と同時に [`IssuedPairFix`](super::IssuedPairFix) を預けるため、自分で
/// 適用した要求がこの形になることはない。外から検証段階の値だけを作られた場合に、
/// **出してもいない指令を是正として記録したり、偽の検証不一致を error で残したり
/// しない**ための枝である。タグを付けない理由は [`log_orphan_request_dropped`] と同じ。
pub(crate) fn log_unbacked_verification_dropped(entity: Entity, expected: ExpectedOrder) {
    warn!(
        "検証待ちの要求に、出した指令の持ち越しがありません（検証を行わず落とします） \
         entity={entity:?} expected_above={above} expected_below={below}",
        above = hwnd_field(Some(expected.above)),
        below = hwnd_field(Some(expected.below)),
    );
}

/// 適用後の実測照合を記録し、期待どおりだったかを返す（要件 6.1／6.2）。
///
/// - 一致: 是正の記録（debug）——出した指令 `insert_after` と実測を同じ行に載せる。
/// - 不一致: 検証不一致の記録（error）。実測が取れなかった場合も一致とは扱わない。
///
/// 不一致でも再試行の輪は作らない（外部の窓操作と競合すると恒常再試行になる）。
/// 次のトリガが来た時点で自然に再是正される。
pub(crate) fn record_verification(
    entity: Entity,
    peer: Entity,
    insert_after: InsertSpec,
    expected: ExpectedOrder,
    measured_below_of_above: Option<HWND>,
) -> bool {
    if measured_below_of_above == Some(expected.below) {
        debug!(
            "{}",
            fix_line(entity, peer, insert_after, measured_below_of_above)
        );
        true
    } else {
        error!(
            "{}",
            verify_failed_line(entity, peer, expected, measured_below_of_above)
        );
        false
    }
}

#[cfg(test)]
#[path = "zorder_pair_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "zorder_pair_measure_tests.rs"]
mod measure_tests;

#[cfg(test)]
#[path = "zorder_pair_diag_tests.rs"]
mod diag_tests;
