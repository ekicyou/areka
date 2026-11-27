//! 毎フレーム結線の NonSend 状態 [`RuntimeWiring`]（構築・アクセサ・test-support 読み口）。

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, Weak};
use std::time::Duration;

use bevy_ecs::system::SystemState;
#[cfg(any(test, feature = "test-support"))]
use bevy_ecs::world::World;

#[cfg(any(test, feature = "test-support"))]
use areka_emo_present::TargetId;
use areka_emo_present::balloon_definitions::ScopeBalloonDefinitions;
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::TextLayerRuntime;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::held::HeldQueue;

use super::super::balloon_visibility::BalloonVisibilityState;
use super::super::shell_carry::ShellCarry;
use super::super::switch_assets::SwapPayload;
use super::super::talk_lifecycle::TalkLifecycleSignal;
use super::super::window_state_cue::WindowStateDirective;
use super::super::zorder_cue::ZOrderDirective;
use super::move_hold::HeldCue;
use super::status_report::BalloonStatusLedger;
use super::window_state::StayOnTopState;
use super::zorder_descript::{apply_descript_base, apply_descript_rebase};
use super::{BootAssets, DpiChangedQuery, PositionCue, TalkClock, TextLayerBusyLogged};
use crate::scope_scale::SharedScopeScaleTable;
use areka_placement::zorder_group_ledger::ZOrderGroupLedger;

/// 毎フレーム結線の NonSend 状態。
///
/// `EmoPresenter`（`!Send`）・`Receiver`（`!Sync`）・`Rc<RefCell<TextLayerRuntime>>`（`!Send`）を内包するため NonSend resource として `wire_runtime` が挿入し、排他 system [`frame_system`] が remove→各相→insert で駆動する。
///
/// # フィールドの可視性が 2 種類ある理由
///
/// 大半のフィールドは `frame` 配下の相だけが触るため `pub(super)`。バルーン可視性の相は兄弟モジュール `balloon_visibility` に置かれ、複数の欄を同時に可変借用する（表示層へ発行しながら状態を更新する）ので読み口メソッドでは組めない。そのため、その相が消費する `presenter`／`lifecycle_rx`／`lifecycle_held`／`balloon_visibility`／`balloon_models`／`talk_ends` と、届けの相の台帳 `balloon_status` などは `pub` にしてある。
pub struct RuntimeWiring {
    /// 表示層の指令適用ハブ（UI スレッド専有・`!Send`）。
    pub presenter: EmoPresenter,
    /// worker（seriko 経由 `PresentBridge`）からの表示指令受信端（drain の相が消費）。
    pub(super) rx: Receiver<PresentCommand>,
    /// talk スレッド（`MoveCueSink`）からの窓の位置の指令（`\![move]`・`\![execute,resetwindowpos]`）
    /// 受信端（[`run_move_drain_phase`] が消費）。
    pub(super) move_rx: Receiver<PositionCue>,
    /// 対象か基準の窓が最初の面を表示するまで預かった位置の指令（順のまま・`move_hold.rs`）。
    pub(super) move_held: RefCell<VecDeque<HeldCue>>,
    /// トークの終わりの通し番号（バルーン可視性の相が `TalkEnded` を数えて進め、位置の指令の相が読む）。
    pub talk_ends: u64,
    /// 位置の指令の相が前に指令の線を取り出したときの [`Self::talk_ends`]。その後に届いた指令の届いたときの番号。
    pub(super) move_talk_ends_seen: Cell<u64>,
    /// talk スレッド（`BalloonLifecycleSink`）からの表示ライフサイクル信号受信端（バルーン可視性相が消費）。
    ///
    /// [`super::super::wire_runtime`] が `mpsc::channel::<TalkLifecycleSignal>()` を作り、送出端を `GhostBootOptions.sinks` の 4 本目（`BalloonLifecycleSink`）へ、受信端をここへ渡す。会話の開始（[`TalkLifecycleSignal::TalkStarted`]）と、待機を含む占有区間の終端（[`TalkLifecycleSignal::DisplayEndAt`]）が、効く絶対の時刻つきで FIFO に届く。送り手はもう 1 つあり、UI スレッドの押下ハンドラが送出端の複製から利用者の中断（[`TalkLifecycleSignal::UserBreak`]）を同じ線へ流す。
    ///
    /// 取り出しは可視性相の `try_iter` による全件 drain。受信前の信号はチャネル自身が預かるので、attach 前に始まった会話の信号も落ちない。
    pub lifecycle_rx: Receiver<TalkLifecycleSignal>,
    /// 線から取り出した寿命の知らせを、届いた順のまま時刻が来るまで持つ列（可視性相が足し、取り出す）。
    pub lifecycle_held: HeldQueue<TalkLifecycleSignal>,
    /// talk スレッド（`ZOrderCueSink`）からの重なりの指令受信端（[`run_zorder_drain_phase`](super::zorder_drain::run_zorder_drain_phase) が消費）。
    ///
    /// [`super::super::wire_runtime`] が `mpsc::channel::<ZOrderDirective>()` を作り、送出端を `GhostBootOptions.sinks` の 5 本目（`ZOrderCueSink`）へ、受信端をここへ渡す。`\![set,zorder,…]`／`\![reset,zorder]` の解釈前のトークン列が到着順に届く。
    ///
    /// 窓の正本（`GhostWindows`）が World に載るまで取り出しの相は `try_iter` を呼ばず、チャネル自身が保留バッファを兼ねる（起動直後のタグも落ちない）。
    pub(super) zorder_rx: Receiver<ZOrderDirective>,
    /// 重なりのグループ台帳（正本）。取り出しの相が指令を適用し、鎖の合成の入力になる。
    ///
    /// 相関数は排他 system から呼ばれる素の関数で `Local` を取れないため、フレームを跨いで生きる器が結線状態の側に要る（`balloon_visibility` と同じ理由）。合成した鎖の写し（wintf の `ZOrderChainPlan`）は World 側に在るが、正本はこちらで二重帳簿にしない。
    ///
    /// 構築時は常に既定（グループ 0 本）。shell 設定由来の基底は起動の段が [`RuntimeWiring::seed_zorder_descript_base`] で 1 度だけ据える（取り出しの相が 1 度も走る前）。
    pub(super) zorder_ledger: ZOrderGroupLedger,
    /// ゴーストの常に手前の状態（既定＝受信端なし・オフ）。
    ///
    /// 受信端は起動の結線が [`RuntimeWiring::attach_window_state_rx`] で後から渡す。結線状態と
    /// 一緒に作り直される経路（ゴーストの切り替え・読み直し・終了）で消え、残る経路
    /// （SHIORI だけの読み直し・シェルとバルーンの切り替え）では保たれる。
    pub(super) window_state: StayOnTopState,
    /// バルーン可視性コントローラが持ち越す状態（エッジ検出・計測・抑止の記憶）。
    ///
    /// 判断中核（[`super::super::balloon_visibility::decide`]）が状態を `&mut` で受けて更新するので、フレームを跨ぐ器がここに要る（`dpi_state` と同じ理由）。UI スレッド専有で、可視性相以外は触らない。
    pub balloon_visibility: BalloonVisibilityState,
    /// 最後に kanade へ届けた「見えているバルーンの組」の台帳（フレームの終わりの届けの相が使う）。ゴーストごとに新品で、前のゴーストの組を持ち越さない。
    pub balloon_status: BalloonStatusLedger,
    /// バルーン文字層ランタイム（`register_actor_view`／`present_frame` の所有・`!Send`）。
    pub(super) runtime: Rc<RefCell<TextLayerRuntime>>,
    /// scope → attach 相で装着に使った [`BalloonModel`]（文字層 k 再追従の再利用源）。
    ///
    /// 装着（`register_actor_view`）と再追従（[`TextLayerRuntime::refresh_actor_scale`]）はどちらも `&BalloonModel` を要する。ここへ記憶して、文字層スケール相（[`run_text_scale_phase`]）が再パースせず同一モデルで binding を組み直せるようにする（再パースすれば装着時と再追従時で別モデルという静かな食い違いが生まれる）。
    ///
    /// キーは scope（再追従は「どの scope の balloon 窓か」から引く）。起動時資産は [`BalloonScopeAssets`] が scope 別の定義を保持し、attach（[`run_attach_phase`]）は当該 scope の定義を、ここと文字層結線（[`connect_balloon_text`]）の双方へ同一値で挿す。
    pub balloon_models: HashMap<u32, BalloonModel>,
    /// scope → 面ごとの定義の束（装着・バルーンの切り替えが入れる）。
    /// 今の定義（`balloon_models`）はここから書き手 [`super::balloon_face::select_balloon_face`] が引く。
    pub balloon_definitions: HashMap<u32, ScopeBalloonDefinitions>,
    /// scope → 今の定義の面の番号（書き手 [`super::balloon_face::select_balloon_face`] だけが書く）。
    pub balloon_faces: HashMap<u32, u32>,
    /// 文字層 k 追従で `text_slot_view` が `None` だった scope の警告済み集合（エッジガード）。
    ///
    /// [`run_text_scale_phase`] は毎フレーム走るので、表示未確立の縮退は scope ごとに一度だけ警告し、view が取れた時点で除去して再武装する（`areka-emo-text` の `unresolved_warned: BTreeSet<ActorKey>` と同型）。
    pub(super) text_scale_warned: BTreeSet<u32>,
    /// 装着で文字層を繋げなかった scope と、その面 0 の背景色（[`run_text_scale_phase`] が繋ぎ直す時に渡す）。
    pub(super) unconnected_text: BTreeMap<u32, (u8, u8, u8)>,
    /// 現実の出来事に時刻を付ける時計（トークの受け口・利用者の中断の口と同じ時計）。
    pub clock: TalkClock,
    /// 文字の層を借りられなかった事の、相ごとの記録の覚え（同じ縮みの間は 1 回）。
    pub(super) text_busy_logged: TextLayerBusyLogged,
    /// load-time 構築資産（attach で `take` して高々 1 回消費）。
    pub(super) assets: Option<BootAssets>,
    /// attach 完了フラグ（高々 1 回のゲート・以降 no-op）。
    pub(super) attached: bool,
    /// `Changed<DPI>` 観測の永続 [`SystemState`]（[`run_dpi_phase`]）。
    ///
    /// 本フェーズは排他 system [`frame_system`] から呼ばれる素の関数で `Local` を取れないため、run を跨いで `last_run` tick を保つ器がここに要る。毎 run で `SystemState::new` を作り直すと `last_run` が 0 のままで `Changed` が全窓へ誤マッチし続ける（毎フレーム全窓 refresh の churn）ので、必ず使い回す。
    pub(super) dpi_state: Option<SystemState<DpiChangedQuery>>,
    /// 差し替えの荷物の置き場。
    ///
    /// 起動の結線が 1 つ作り、表示の橋渡し（`PresentBridge::with_swap_slot`）に持たせ、ここには
    /// 弱い参照だけを置く。差し替えの相が引き上げて荷物を置き、橋渡しが合図の世代と突き合わせて
    /// 取り出す。置き場を強く持つのは橋渡しだけ: seriko が合図を出す前に倒れたら、荷物（置き換えの
    /// 返信の送り手）は橋渡しと一緒に消え、頼んだ段は返信の脱落を見て印を消す。ここが強く持つと
    /// 送り手が生き続け、印が残って以後の切替がずっと断られる。seriko の送り手もここに持たせない
    /// （降ろした後も World に残るので、持たせると seriko の join が戻らない＝持ち主は
    /// `GhostSession` だけ）。
    pub swap_slot: Weak<Mutex<Option<SwapPayload>>>,
    /// このフレームの drain（表示の指令の適用）に費やした時間（毎フレーム `frame_system` が
    /// 書く）。差し替えの相が、置き換えの返信のそろったフレームの `swap_ms` へ足す。
    pub(super) last_drain: Duration,
    /// ghost の読み直しの前に控えた表示中の面（装着の相が 1 度だけ使う）。
    pub shell_carry: Option<ShellCarry>,
    /// 今のシェルの静的な着せ替えの既定（起動の資産から・シェルの差し替えで置き直す）。ghost の読み直しで
    /// 控える着せ替えを「この既定との差」にするのに使う。
    pub static_binds: areka_seriko::DefaultBinds,
    /// スコープの倍率と DPI の表（ゴーストの起動ごとに新しい）。
    /// 書き手は毎フレームの相、読み手は実行系の読み口（`rewire_runtime_consumers` が登録する）。
    pub(super) scope_scale: SharedScopeScaleTable,
}

impl RuntimeWiring {
    /// 結線資源を構築する（`wire_runtime` が呼ぶ）。`assets` は `Some` で保持し、attach フェーズ（[`run_attach_phase`]）が `take` で高々 1 回消費する。`move_rx`／`lifecycle_rx`／`zorder_rx` は各 sink（talk スレッド）と対の受信端。可視性の持ち越し状態と重なりの台帳は外から与えず初期値から始める。
    pub fn new(
        presenter: EmoPresenter,
        rx: Receiver<PresentCommand>,
        move_rx: Receiver<PositionCue>,
        lifecycle_rx: Receiver<TalkLifecycleSignal>,
        zorder_rx: Receiver<ZOrderDirective>,
        runtime: Rc<RefCell<TextLayerRuntime>>,
        clock: TalkClock,
        assets: BootAssets,
    ) -> Self {
        let static_binds = assets.static_binds.clone();
        Self {
            presenter,
            rx,
            move_rx,
            move_held: RefCell::default(),
            talk_ends: 0,
            move_talk_ends_seen: Cell::new(0),
            lifecycle_rx,
            lifecycle_held: HeldQueue::new(),
            zorder_rx,
            zorder_ledger: ZOrderGroupLedger::default(),
            window_state: StayOnTopState::default(),
            balloon_visibility: BalloonVisibilityState::default(),
            balloon_status: BalloonStatusLedger::default(),
            runtime,
            balloon_models: HashMap::new(),
            balloon_definitions: HashMap::new(),
            balloon_faces: HashMap::new(),
            text_scale_warned: BTreeSet::new(),
            unconnected_text: BTreeMap::new(),
            clock,
            text_busy_logged: TextLayerBusyLogged::default(),
            assets: Some(assets),
            attached: false,
            // 初回 [`run_dpi_phase`] で遅延生成する（`SystemState::new` は `&mut World` を要する）。
            dpi_state: None,
            // 置き場なし（引き上げは常に失敗）から始める。起動の結線は表示の橋渡しの置き場の
            // 弱い参照へ差し替える。
            swap_slot: Weak::new(),
            last_drain: Duration::ZERO,
            // 控えは読み直しの起こし直しだけが渡す（ふつうの起動・切替は無し）。
            shell_carry: None,
            static_binds,
            scope_scale: SharedScopeScaleTable::default(),
        }
    }

    /// スコープの倍率と DPI の表（同じ表を共有する）。
    pub fn scope_scale_table(&self) -> SharedScopeScaleTable {
        self.scope_scale.clone()
    }

    /// shell 設定（`seriko.zorder`）由来の基底を台帳へ据える（起動の段の入口）。
    ///
    /// 台帳は `frame` の内側に閉じているので、起動の結線（`wire_runtime`）はこの口から触る。呼ぶのは結線状態 1 つにつき `insert_non_send` より前（その結線状態で走る最初の `Update` より前）の 1 回だけで（起こし直しでは `boot_ghost` が新しい結線状態で呼ぶ）、基底はタグの実行を待たずに最初の維持の巡から効く。
    ///
    /// 解釈・拒否・記録はすべて [`apply_descript_base`] が持ち、ここは台帳への到達だけを引き受ける。
    ///
    /// [`apply_descript_base`]: super::zorder_descript::apply_descript_base
    pub(crate) fn seed_zorder_descript_base(&mut self, zorder_raw: Option<&str>) {
        apply_descript_base(&mut self.zorder_ledger, zorder_raw);
    }

    /// 常に手前の指令の受信端を渡す（起動の結線の口）。
    pub fn attach_window_state_rx(&mut self, rx: Receiver<WindowStateDirective>) {
        self.window_state.rx = Some(rx);
    }

    /// シェルの差し替えで、基底を新しいシェルの `seriko.zorder` へ置き直す。
    ///
    /// 解釈・拒否・記録は [`apply_descript_rebase`] が持つ（`None`・解釈できない値は基底なし）。
    ///
    /// [`apply_descript_rebase`]: super::zorder_descript::apply_descript_rebase
    pub(super) fn reseed_zorder_descript_base(&mut self, zorder_raw: Option<&str>) {
        apply_descript_rebase(&mut self.zorder_ledger, zorder_raw);
    }

    /// 当たり判定 resolver への読み口。
    ///
    /// 内包する [`EmoPresenter`] を読み取り専用で貸し出す。本体の入力の層の `resolve_hit_owned` が [`crate::hit_region::resolve_hit_region`] の第 2 引数（`&EmoPresenter`）へそのまま渡す。所有・可変アクセスは frame 相に閉じたまま。呼び手は `.map(RuntimeWiring::presenter)` の関数パス形で借りるので、`.presenter()` の grep では見つからない。
    pub fn presenter(&self) -> &EmoPresenter {
        &self.presenter
    }

    /// 最後に描いた時点でバルーンに出ていた台詞のトークの番号（読み口だけ・利用者の中断が
    /// 押されたバルーンのトークを kanade へ運ぶ）。
    pub fn shown_talk_id(&self) -> Option<areka_sakura::TalkId> {
        self.balloon_visibility.talk_id()
    }

    /// 装着の相（`run_attach_phase`）が済んだか。済む前は表示の層に target が 1 つも無い
    /// （MCP の `dump_surface`／`dump_balloon` が、済むまで答えを後回しにするのに読む）。
    pub fn attached(&self) -> bool {
        self.attached
    }

    /// 文字層 runtime への共有ハンドル読み口。
    ///
    /// 本体の入力の層（バルーンの入力）の `on_balloon_pointer_moved` などが、`Rc::clone` して
    /// world 側の借用を即座に解く形で選択肢のハイライト・確定を橋渡しする。`runtime` の所有・
    /// 可変アクセスは frame 相の text フェーズに閉じたまま、配線層へは read 口のみを開ける。
    pub fn runtime(&self) -> &Rc<RefCell<TextLayerRuntime>> {
        &self.runtime
    }

    /// `\![move]` 指令受信端への test 専用アクセサ。
    ///
    /// channel 配線の到達性（`MoveCueSink`→`RuntimeWiring` の受信端が届く）を決定論に固定するための
    /// 最小 read 口で、本番の移動の取り出しの相は `move_rx` を閉じたまま使う。
    #[cfg(test)]
    pub(super) fn drain_move_directives(&self) -> Vec<PositionCue> {
        self.move_rx.try_iter().collect()
    }

    /// 表示ライフサイクル信号受信端への test-support アクセサ。
    ///
    /// channel 配線の到達性（会話の再生が `BalloonLifecycleSink`→`RuntimeWiring` の受信端へ届く）を
    /// 決定論に固定するための最小 read 口。本番の可視性相は同じ全件取り出しを自前で行う。
    #[cfg(any(test, feature = "test-support"))]
    pub fn drain_lifecycle_signals(&self) -> Vec<TalkLifecycleSignal> {
        self.lifecycle_rx.try_iter().collect()
    }

    // ── spine 観測用 test-support アクセサ ──────────
    //
    // 決定論 spine（`crate::spine`）が private フィールドを読む・通すための最小口（5 つ・getter/passthrough のみ・`cfg(any(test, feature = "test-support"))` に閉じる）。

    /// target の表示中画素（BGRA・`stride=width*4`）を読み戻す（`EmoPresenter::read_back` passthrough）。
    #[cfg(any(test, feature = "test-support"))]
    pub fn read_back_target(
        &self,
        target: TargetId,
    ) -> Result<Vec<u8>, areka_emo_present::PresentError> {
        self.presenter.read_back(target)
    }

    /// rx にキュー済みの `PresentCommand` を非ブロックで FIFO 全件取り出す（S3/S4 の受信列記録用）。
    ///
    /// `run_drain_phase`（上限まで先に集め、同じ target へ続く reply の無い ShowSurface を最後の 1 件に
    /// 畳む）と違い、本補助は上限も畳みも持たず届いた分を全部そのまま返す。spine は形状記録のため apply 前に
    /// **値**として取り出す（`PresentCommand` は `reply: Option<ReplySender>` ゆえ非 Clone・move で受ける）。
    #[cfg(any(test, feature = "test-support"))]
    pub fn drain_received(&mut self) -> Vec<PresentCommand> {
        self.rx.try_iter().collect()
    }

    /// 1 件の `PresentCommand` を presenter へ適用する（`EmoPresenter::apply` passthrough）。
    ///
    /// `drain_received` で取り出した指令を、形状記録後に実 presenter へ流して実描画→readback まで
    /// 通すための最小口。本番は同じ `apply` を `run_drain_phase` が呼ぶ。
    #[cfg(any(test, feature = "test-support"))]
    pub fn apply_present(&mut self, world: &mut World, cmd: PresentCommand) {
        self.presenter.apply(world, cmd);
    }

    /// 外部所有（`VisibilityOwnership::External`）な target を可視化する
    /// （`EmoPresenter::show_target` passthrough）。
    ///
    /// attach 相はバルーンを**不可視のまま確立**するため、spine が「可視バルーン」を前提とする
    /// ケースはその前提を自分で組む必要がある。本番で可視化を発行するのはバルーン可視性制御
    /// の相であり、本口はそれを spine から代行するだけの passthrough である。
    #[cfg(any(test, feature = "test-support"))]
    pub fn show_balloon_target(
        &mut self,
        world: &mut World,
        target: TargetId,
    ) -> Result<(), areka_emo_present::PresentError> {
        self.presenter.show_target(world, target)
    }

    /// 再追従用に記憶している [`BalloonModel`] の scope 集合（昇順）。
    ///
    /// 「attach 相が per-scope の model を実際に保持したか」は本番 attach（GPU 資源＋実資産）を
    /// 通さないと観測できないため、spine（in-crate GPU ハーネス）から見えるだけの read を開ける。
    #[cfg(any(test, feature = "test-support"))]
    pub fn balloon_model_scopes(&self) -> Vec<u32> {
        let mut scopes: Vec<u32> = self.balloon_models.keys().copied().collect();
        scopes.sort_unstable();
        scopes
    }
}

// 本体のテスト（`crates/areka/src/runtime_tests/`）が使う私有の欄へそのまま渡す口（`test-support` のときだけ・
// 本番の可視性は変えない）。
#[cfg(feature = "test-support")]
pub mod test_support {
    use std::time::Duration;

    use areka_placement::zorder_group_ledger::ZOrderGroupLedger;

    use super::super::window_state::StayOnTopState;
    use super::RuntimeWiring;

    /// [`RuntimeWiring`] の重なりの組の帳簿（`zorder_ledger`）を読む。
    pub fn zorder_ledger(wiring: &RuntimeWiring) -> &ZOrderGroupLedger {
        &wiring.zorder_ledger
    }

    /// [`RuntimeWiring`] の重なりの組の帳簿（`zorder_ledger`）を書く。
    pub fn zorder_ledger_mut(wiring: &mut RuntimeWiring) -> &mut ZOrderGroupLedger {
        &mut wiring.zorder_ledger
    }

    /// [`RuntimeWiring`] の常に手前の状態（`window_state`）を読む。
    pub fn window_state(wiring: &RuntimeWiring) -> &StayOnTopState {
        &wiring.window_state
    }

    /// [`RuntimeWiring`] の常に手前の状態（`window_state`）を書く。
    pub fn window_state_mut(wiring: &mut RuntimeWiring) -> &mut StayOnTopState {
        &mut wiring.window_state
    }

    /// [`RuntimeWiring`] の直近の drain の所要（`last_drain`）を書く。
    pub fn last_drain_mut(wiring: &mut RuntimeWiring) -> &mut Duration {
        &mut wiring.last_drain
    }
}
