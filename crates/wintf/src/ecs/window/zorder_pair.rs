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
//! 確立系・維持系は本フィーチャーの後続タスクが同ファイルへ足す。

use bevy_ecs::prelude::*;
use tracing::{debug, error, info};
use windows::Win32::Foundation::HWND;

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
/// 期待値はこの 1 形で足りる。照合は `get_window_below(above) == Some(below)`
/// （`GetWindow(GW_HWNDNEXT)` 実測）で行い、不一致なら `verify-failed` を `error!` で
/// 記録する（要件 6.2）。同レコードの `expected` がこの値、`measured` が実測値である。
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
#[allow(dead_code)] // 各種別を組み立てる結線は後続タスク（確立系・維持系・z 変化検知）で入る
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
    /// 手前側の 1 つ背後にある窓（`GetWindow(above, GW_HWNDNEXT)` 実測）
    pub measured_below_of_above: Option<HWND>,
    /// 背後側の 1 つ手前にある窓（`GetWindow(below, GW_HWNDPREV)` 実測）
    pub measured_above_of_below: Option<HWND>,
}

/// 是正の挿入位置。
///
/// `SetWindowPos` の `hWndInsertAfter` は**指定窓の背後**へ対象を置くため、
/// 「キャラのすぐ手前」を作る挿入位置はキャラ自身ではなく**キャラの 1 つ手前の窓**である。
/// その窓が居ない（キャラが最上位）縁だけが [`InsertSpec::TopEdge`] になる。
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
// 配線は本フィーチャーの後続タスク（維持系 `apply_zorder_pair_maintenance`・
// 案 B の `compute_pair_z_intent`）で入る。
#[allow(dead_code)]
pub(crate) fn decide_pair_fix(obs: &PairObservation) -> PairFixDecision {
    if !obs.above_alive || !obs.below_alive {
        return PairFixDecision::Skip(SkipReason::PeerMissing);
    }
    let (Some(above), Some(below)) = (obs.above_hwnd, obs.below_hwnd) else {
        return PairFixDecision::Skip(SkipReason::HandleMissing);
    };

    // 同値ガード。正準の判定は「手前側の 1 つ背後が背後側か」（design.md Invariants）だが、
    // 逆向きの実測が隣接を示す場合も併せて止める——そちらだけが隣接を示すのは 2 つの実測が
    // 食い違った異常であり、そのまま是正へ流すと挿入位置が動かす窓自身になってしまう。
    if obs.measured_below_of_above == Some(below) || obs.measured_above_of_below == Some(above) {
        return PairFixDecision::Skip(SkipReason::AlreadyAdjacent);
    }

    // 「バルーンをキャラのすぐ手前へ」の挿入位置は、キャラの 1 つ手前の窓。
    // その窓が居なければキャラが最上位であり、最上位へ置くほか無い。
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
//
// # 出力先と水準
//
// target は module path 既定（`wintf::ecs::window::zorder_pair`）——サインオフの
// `RUST_LOG` はこの名前で点灯させる。水準はレコード表どおりで、失敗と検証不一致だけが
// error、確立成功が info、残りは診断専用の debug（既定運転では無音）である。

/// 値が取得できなかったフィールドの番兵。
///
/// フィールドごと落とさないのは、落とすと「記録が出ていない」のと「その経路には
/// その値が無い」の区別が事後に付かなくなるためである。
const UNKNOWN: &str = "-";

/// owner 確立の記録タグ（grep 判定語）。
const OWNER_ESTABLISHED_TAG: &str = "[zorder-pair] owner-established";
/// 是正の記録タグ（指令と実測を同一行に載せる・要件 6.1）。
const FIX_TAG: &str = "[zorder-pair] fix";
/// 見送りの記録タグ（理由必須・要件 6.3）。
const SKIP_TAG: &str = "[zorder-pair] skip";
/// 検証不一致の記録タグ（error 水準・要件 6.2）。
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";
/// owner 確立失敗の記録タグ（error 水準・要件 6.2）。
const OWNER_ESTABLISH_FAILED_TAG: &str = "[zorder-pair] owner-establish-failed";
/// 沈降観測の記録タグ（要件 4.4／7.5）。
const SINK_OBSERVED_TAG: &str = "[zorder-pair] sink-observed";

/// HWND をログ用の 16 進表現へ（不在は番兵）。
///
/// `Debug` 表現をそのまま使わないのは、値の中に空白や記号が混じると
/// `field=value` の 1 行から機械的に切り出せなくなるためである。
fn hwnd_field(hwnd: Option<HWND>) -> String {
    match hwnd {
        Some(h) => format!("0x{:X}", h.0 as usize),
        None => UNKNOWN.to_string(),
    }
}

/// 真偽値をログ用表現へ（判定そのものが取れなかった場合は番兵）。
///
/// 「取れなかった」を `false` へ潰さない——潰すと「沈まなかった」という**異常**と
/// 「測れなかった」という**観測の欠落**が同じ字面になる。
fn tristate_field(value: Option<bool>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => UNKNOWN.to_string(),
    }
}

impl SkipReason {
    /// 記録に載る理由語（サインオフの grep 判定語）。
    ///
    /// 5 種が互いに異なる語であることがそのまま要件 6.3 の「理由を伴う見送り」の
    /// 実質になる——1 語へ潰れると記録があっても理由が読めない。
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            SkipReason::AlreadyAdjacent => "AlreadyAdjacent",
            SkipReason::PeerMissing => "PeerMissing",
            SkipReason::HandleMissing => "HandleMissing",
            SkipReason::EchoOrIrrelevant => "EchoOrIrrelevant",
            SkipReason::StrategyDisabled => "StrategyDisabled",
        }
    }
}

impl InsertSpec {
    /// 挿入位置のログ用表現（縁は専用の語で、ハンドル表現と混ざらない）。
    fn as_field(&self) -> String {
        match self {
            InsertSpec::After(hwnd) => hwnd_field(Some(*hwnd)),
            InsertSpec::TopEdge => "top-edge".to_string(),
        }
    }
}

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
    /// 現時点の消費者はテストのみで、実際の持ち越しは 2.2 が結線する。
    // 呼び出しの結線は後続タスク（維持系が判断を適用し、次巡の検証へ指令を持ち越す）で入る。
    #[allow(dead_code)]
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

/// owner 確立の記録行（純関数）。
///
/// `measured_prev` は確立直後に採った「owner（キャラ窓）の 1 つ手前」の実測であり、
/// ここに被 owner（バルーン窓）が現れることが案 A の中核保証（ゲート G6）の証跡になる。
fn owner_established_line(
    entity: Entity,
    peer: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    measured_prev: Option<HWND>,
) -> String {
    format!(
        "{OWNER_ESTABLISHED_TAG} entity={entity:?} peer={peer:?} owned_hwnd={owned} \
         owner_hwnd={owner} measured_prev={measured}",
        owned = hwnd_field(Some(owned_hwnd)),
        owner = hwnd_field(Some(owner_hwnd)),
        measured = hwnd_field(measured_prev),
    )
}

/// 是正の記録行（純関数）——**出した指令と、その後の実測を同じ 1 行に載せる**。
///
/// 行を「指令」と「実測」に分けないのは design.md「Implementation Notes > Validation」の
/// 裁定である。分けると「指令は出したが効かなかった」の判定が 2 行の突合になり、
/// 過去に同型の誤診を生んでいる。
fn fix_line(
    entity: Entity,
    peer: Entity,
    insert_after: InsertSpec,
    measured_next_after_fix: Option<HWND>,
) -> String {
    format!(
        "{FIX_TAG} entity={entity:?} peer={peer:?} insert_after={insert_after} \
         measured_next_after_fix={measured}",
        insert_after = insert_after.as_field(),
        measured = hwnd_field(measured_next_after_fix),
    )
}

/// 見送りの記録行（純関数・理由必須）。
fn skip_line(entity: Entity, peer: Entity, reason: SkipReason) -> String {
    format!(
        "{SKIP_TAG} entity={entity:?} peer={peer:?} reason={reason}",
        reason = reason.as_str(),
    )
}

/// 検証不一致の記録行（純関数）——期待した隣接と実測を同じ行へ。
fn verify_failed_line(
    entity: Entity,
    peer: Entity,
    expected: ExpectedOrder,
    measured: Option<HWND>,
) -> String {
    format!(
        "{VERIFY_FAILED_TAG} entity={entity:?} peer={peer:?} expected_above={above} \
         expected_below={below} measured={measured}",
        above = hwnd_field(Some(expected.above)),
        below = hwnd_field(Some(expected.below)),
        measured = hwnd_field(measured),
    )
}

/// owner 確立失敗の記録行（純関数）。
fn owner_establish_failed_line(entity: Entity, error: &windows::core::Error) -> String {
    format!(
        "{OWNER_ESTABLISH_FAILED_TAG} entity={entity:?} error={code:?} message={message}",
        code = error.code(),
        message = error.message(),
    )
}

/// 沈降観測の記録行（純関数）。
///
/// `behind_foreground` は「当該窓が前面窓より背面に居るか」の判定で、前面窓が取れない
/// ／比較できない場合は番兵になる（偽と混同させない）。
fn sink_observed_line(
    entity: Entity,
    adjacency_ok: bool,
    foreground: Option<HWND>,
    behind_foreground: Option<bool>,
) -> String {
    format!(
        "{SINK_OBSERVED_TAG} entity={entity:?} adjacency_ok={adjacency_ok} \
         foreground={foreground} behind_foreground={behind}",
        foreground = hwnd_field(foreground),
        behind = tristate_field(behind_foreground),
    )
}

/// owner 確立の記録を出す（info・要件 6.1）。
// 呼び出しの結線は後続タスク（確立系 `establish_owner_links`）で入る。
#[allow(dead_code)]
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
/// 記録を二重化するだけである（design.md「Error Strategy」）。
// 呼び出しの結線は後続タスク（確立系 `establish_owner_links`）で入る。
#[allow(dead_code)]
pub(crate) fn log_owner_establish_failed(entity: Entity, error: &windows::core::Error) {
    error!("{}", owner_establish_failed_line(entity, error));
}

/// 沈降観測の記録を出す（debug・要件 4.4／7.5）。
// 呼び出しの結線は後続タスク（維持系の遅延観測）で入る。
#[allow(dead_code)]
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
/// 本関数を通さずに判断結果を握り潰す書き方は**言語上は可能**である。加えて design.md
/// 「File Structure Plan」は確立系・維持系を本ファイルへ置くと定めており、その実装
/// （タスク 2.2）は同一モジュールに来る——モジュールが Rust の可視性境界である以上、
/// 可視性で塞ぐ余地は無い。したがってここにあるのは
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
// 呼び出しの結線は後続タスク（維持系 `apply_zorder_pair_maintenance`）で入る。
#[allow(dead_code)]
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

/// 適用後の実測照合を記録し、期待どおりだったかを返す（要件 6.1／6.2）。
///
/// - 一致: 是正の記録（debug）——出した指令 `insert_after` と実測を同じ行に載せる。
/// - 不一致: 検証不一致の記録（error）。実測が取れなかった場合も一致とは扱わない。
///
/// 不一致でも再試行の輪は作らない（外部の窓操作と競合すると恒常再試行になる）。
/// 次のトリガが来た時点で自然に再是正される。
// 呼び出しの結線は後続タスク（維持系 `apply_zorder_pair_maintenance`）で入る。
#[allow(dead_code)]
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
#[path = "zorder_pair_diag_tests.rs"]
mod diag_tests;
