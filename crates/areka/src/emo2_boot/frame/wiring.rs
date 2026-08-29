//! 毎フレーム結線の NonSend 状態 [`Emo2Wiring`]（構築・アクセサ・test-support 読み口）。

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use bevy_ecs::system::SystemState;
// `World` を使うのは下の `impl Emo2Wiring` 内の `#[cfg(test)]` メソッド `apply_present` だけ
// （`areka` は lib target を持たない bin クレートゆえ非 test ビルド単位では未使用に見える）。
#[allow(unused_imports)]
use bevy_ecs::world::World;

// このグループで非 test ビルドの未使用は `TargetId` のみ（`#[cfg(test)]` メソッド
// `read_back_target` の引数型）。`EmoPresenter`・`PresentCommand` は構造体フィールドが使う。
// 属性は use 文単位で効くためグループ全体に付く。
#[allow(unused_imports)]
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};
use areka_emo_text::actor::TextLayerRuntime;
use areka_parsers::balloon::BalloonModel;

use super::super::balloon_visibility::BalloonVisibilityState;
use super::super::talk_lifecycle::TalkLifecycleSignal;
use super::super::zorder_cue::ZOrderDirective;
use super::zorder_descript::apply_descript_base;
use super::{BootAssets, DpiChangedQuery, MoveDirective, TalkClock};
use crate::placement::zorder_group_ledger::ZOrderGroupLedger;

// ---------------------------------------------------------------------------
// Emo2Wiring＋attach フェーズ（tasks.md task 4.1・design「UI 毎フレーム結線 / frame」）
// ---------------------------------------------------------------------------

/// 毎フレーム三相結線の NonSend 状態（design「Emo2Wiring＋emo2_frame_system」・State Management）。
///
/// `EmoPresenter`（`!Send`）・`Receiver`（`!Sync`）・`Rc<RefCell<TextLayerRuntime>>`（`!Send`）を
/// 内包するため NonSend resource として `wire_emo2_boot`（task 5.1）が挿入する。本 task（4.1）は
/// 構築（[`Emo2Wiring::new`]）と attach フェーズ（[`run_attach_phase`]）に加え、drain フェーズ
/// （[`run_drain_phase`]・`rx` を消費）・text フェーズ（[`run_text_phase`]・`clock` を消費）・排他
/// system [`emo2_frame_system`]（3 フェーズを remove→insert で駆動）を所有する（task 4.1／4.2）。
///
/// # フィールドの可視性が 2 種類ある理由
///
/// 大半のフィールドは `frame` 配下の相（attach／dpi／drain／text）だけが触るため `pub(super)`
/// ＝`emo2_boot::frame` 内である。ただしバルーン可視性の相は design の File Structure Plan により
/// **兄弟モジュール** `emo2_boot::balloon_visibility` に置かれる（判断中核と同居させるため）。
/// その相が消費する 5 つ（`presenter`／`lifecycle_rx`／`balloon_visibility`／`balloon_models`／
/// `clock`）だけを `pub(in crate::emo2_boot)` へ広げてある。相は 5 つを**同時に**可変借用するため
/// （表示層へ発行しながら状態を更新する）、読み口メソッドを並べる形では組めない。広げた先は
/// emo2_boot の内側どまりで、公開面は 1 つも増えない。
pub struct Emo2Wiring {
    /// 表示層の指令適用ハブ（UI スレッド専有・`!Send`）。
    pub(in crate::emo2_boot) presenter: EmoPresenter,
    /// worker（seriko 経由 `PresentBridge`）からの表示指令受信端（task 4.2 の drain で消費）。
    pub(super) rx: Receiver<PresentCommand>,
    /// talk スレッド（`MoveCueSink`）からの `\![move]` 指令受信端（frame 相 drain＝[`run_move_drain_phase`]
    /// が消費）。
    ///
    /// `PresentBridge` の `rx` と同型の配線: `wire_emo2_boot`（task 9.1）が
    /// `mpsc::channel::<MoveDirective>()` の受信端を受け渡し、frame 相の [`emo2_frame_system`] が
    /// [`run_move_drain_phase`] 経由で `try_iter` し `apply_move_directive` へ適用する（task 9.2）。
    pub(super) move_rx: Receiver<MoveDirective>,
    /// talk スレッド（`BalloonLifecycleSink`）からの表示ライフサイクル信号受信端
    /// （バルーン可視性相が消費・Requirements 4.1／4.5）。
    ///
    /// `move_rx` と同型の配線: [`super::super::wire_emo2_boot`] が
    /// `mpsc::channel::<TalkLifecycleSignal>()` を作り、送出端を `GhostBootOptions.sinks` の
    /// 4 本目（`BalloonLifecycleSink`）へ、受信端をここへ渡す（design 決定 D4＝α）。会話の開始
    /// （[`TalkLifecycleSignal::TalkStarted`]）と、待機を含む占有区間の終端
    /// （[`TalkLifecycleSignal::DisplayEndAt`]）が talk 相対秒で FIFO に届く。
    ///
    /// 取り出しは可視性相の `try_iter` による**全件 drain**。受信前の信号はチャネル
    /// 自身が保留バッファとして預かるため、attach 前に始まった会話の信号も落ちない。
    pub(in crate::emo2_boot) lifecycle_rx: Receiver<TalkLifecycleSignal>,
    /// talk スレッド（`ZOrderCueSink`）からの重なりの指令受信端（frame 相の取り出し
    /// ＝[`run_zorder_drain_phase`](super::zorder_drain::run_zorder_drain_phase) が消費）。
    ///
    /// `move_rx` と同型の配線: [`super::super::wire_emo2_boot`] が
    /// `mpsc::channel::<ZOrderDirective>()` を作り、送出端を `GhostBootOptions.sinks` の
    /// 5 本目（`ZOrderCueSink`）へ、受信端をここへ渡す（areka-P0-scope-zorder-pinning task 6.2）。
    /// `\![set,zorder,…]`／`\![reset,zorder]` の**解釈前のトークン列**が到着順に届く。
    ///
    /// 窓の正本（`GhostWindows`）が World に載るまで取り出しの相は `try_iter` を呼ばず、
    /// チャネル自身が保留バッファを兼ねる——起動直後のタグも落ちない。
    pub(super) zorder_rx: Receiver<ZOrderDirective>,
    /// 重なりのグループ台帳（正本）。取り出しの相が指令を適用し、鎖の合成の入力になる。
    ///
    /// `balloon_visibility` と同じ理由でここに置く——相関数は排他 system から呼ばれる素の
    /// 関数で `Local` を取れないため、フレームを跨いで生きる器が結線状態の側に要る。
    /// 合成した鎖の写し（wintf の `ZOrderChainPlan`）は World 側に在るが、**正本はこちら**で
    /// あり二重帳簿にはしない（design「Data Models」）。
    ///
    /// 構築の時点では常に既定（グループ 0 本）から始まる。shell 設定由来の基底は、
    /// 起動の段が [`Emo2Wiring::seed_zorder_descript_base`] で 1 度だけ据える
    /// （取り出しの相が 1 度も走る前＝タグの実行より前・要件 5.1）。
    pub(super) zorder_ledger: ZOrderGroupLedger,
    /// バルーン可視性コントローラが持ち越す状態（エッジ検出・計測・抑止の記憶）。
    ///
    /// 判断中核（[`super::super::balloon_visibility::decide`]）は状態を `&mut` で受けて更新する
    /// ため、フレームを跨いで生きる器がここに要る（`dpi_state` と同じ理由——相関数は排他 system
    /// から呼ばれる素の関数で `Local` を取れない）。UI スレッド専有で、可視性相以外は触らない。
    pub(in crate::emo2_boot) balloon_visibility: BalloonVisibilityState,
    /// バルーン文字層ランタイム（`register_actor_view`／`present_frame` の所有・`!Send`）。
    pub(super) runtime: Rc<RefCell<TextLayerRuntime>>,
    /// scope → attach 相で装着に使った [`BalloonModel`]（文字層 k 再追従の再利用源・D11-3・R8.1）。
    ///
    /// `register_actor_view`（装着）と [`TextLayerRuntime::refresh_actor_scale`]（再追従）はいずれも
    /// `&BalloonModel` を要する。装着で使ったモデルをここへ記憶しておき、文字層スケール相
    /// （[`run_text_scale_phase`]）の再追従が
    /// **再パースせず同一モデル**で binding を組み直せるようにする（再パースすれば「装着時と再追従時で
    /// 別モデル」という静かな食い違いの余地が生まれる）。
    ///
    /// 起動時資産は [`BalloonScopeAssets`] が scope 別の定義を保持する（旧 `BootAssets.balloon_model`
    /// の共有 1 本は撤去済み）。ゆえにキーは **scope**——再追従は「どの scope の balloon 窓か」から
    /// 引くのが自然である。attach（[`run_attach_phase`]）は当該 scope の資産から取り出した定義を
    /// **ここと文字層結線（[`connect_balloon_text`]）の双方へ同一値で**挿す（Req 4.1/4.2）。
    pub(in crate::emo2_boot) balloon_models: HashMap<u32, BalloonModel>,
    /// 文字層 k 追従で `text_slot_view` が `None` だった scope の警告済み集合（R8.6 のエッジガード）。
    ///
    /// [`run_text_scale_phase`] は毎フレーム走る。表示未確立の縮退を素朴に `warn!` すると毎フレーム
    /// 鳴って log を溺れさせるため、**scope ごとに一度だけ**鳴らし、view が取れた時点で除去して
    /// 再武装する（`areka-emo-text` の `unresolved_warned: BTreeSet<ActorKey>` と同型の先例）。
    /// R8.6 が求めるのは縮退が**観測できる**ことであって毎回鳴ることではない。
    pub(super) text_scale_warned: BTreeSet<u32>,
    /// talk 起点相対秒の時刻源（text フェーズと可視性相が同一の `talk_time` を引く）。
    pub(in crate::emo2_boot) clock: TalkClock,
    /// load-time 構築資産（attach で `take` して高々 1 回消費）。
    pub(super) assets: Option<BootAssets>,
    /// attach 完了フラグ（高々 1 回のゲート・以降 no-op）。
    pub(super) attached: bool,
    /// `Changed<DPI>` 観測の**永続** [`SystemState`]（[`run_dpi_phase`]・emo-dpi-scaling task 4.2）。
    ///
    /// `anchor_changed_system` の `Local<Option<SystemState<..>>>` と同じ役割を担う。あちらは bevy の
    /// system 引数として `Local` を受けられるが、本フェーズは排他 system [`emo2_frame_system`] から
    /// 呼ばれる**素の関数**（design の署名 `run_dpi_phase(&mut Emo2Wiring, &mut World)`）ゆえ `Local`
    /// を取れない——run を跨いで `last_run` tick を保つ器がここに要る。毎 run で `SystemState::new`
    /// を作り直すと `last_run` が 0 のままとなり `Changed` が全窓へ誤マッチし続ける（＝毎フレーム
    /// 全窓 refresh の churn）ため、必ず使い回す。
    pub(super) dpi_state: Option<SystemState<DpiChangedQuery>>,
}

impl Emo2Wiring {
    /// 結線資源を構築する（`wire_emo2_boot`＝task 5.1／9.1／4.1 が呼ぶ）。`assets` は `Some` で
    /// 保持し、attach フェーズ（[`run_attach_phase`]）が `take` で高々 1 回消費する。`move_rx` は
    /// `MoveCueSink`（talk スレッド）と対の受信端で、frame 相 drain（task 9.2）が消費する。
    /// `lifecycle_rx` は `BalloonLifecycleSink`（同じく talk スレッド）と対の受信端で、バルーン
    /// 可視性相が消費する。`zorder_rx` は `ZOrderCueSink`（同じく talk スレッド）と対の
    /// 受信端で、取り出しの相（task 6.2）が消費する。可視性の持ち越し状態と重なりの台帳は
    /// 外から与えず初期値から始める。
    pub fn new(
        presenter: EmoPresenter,
        rx: Receiver<PresentCommand>,
        move_rx: Receiver<MoveDirective>,
        lifecycle_rx: Receiver<TalkLifecycleSignal>,
        zorder_rx: Receiver<ZOrderDirective>,
        runtime: Rc<RefCell<TextLayerRuntime>>,
        clock: TalkClock,
        assets: BootAssets,
    ) -> Self {
        Self {
            presenter,
            rx,
            move_rx,
            lifecycle_rx,
            zorder_rx,
            // 台帳は既定（グループ 0 本）から始める。shell 設定由来の基底を据えるのは
            // 起動の段の担当であり、結線はその器を用意するところまでを負う。
            zorder_ledger: ZOrderGroupLedger::default(),
            // 可視性の持ち越し状態は初期値（未観測・計測なし）から始める。
            balloon_visibility: BalloonVisibilityState::default(),
            runtime,
            // attach 相が装着した scope ごとに埋める（D11-3）。
            balloon_models: HashMap::new(),
            // 縮退警告のエッジガード（初期は未警告＝最初の縮退で 1 回鳴る）。
            text_scale_warned: BTreeSet::new(),
            clock,
            assets: Some(assets),
            attached: false,
            // 初回 [`run_dpi_phase`] で遅延生成する（`SystemState::new` は `&mut World` を要する）。
            dpi_state: None,
        }
    }

    /// shell 設定（`seriko.zorder`）由来の基底を台帳へ据える（起動の段の入口・要件 5.1）。
    ///
    /// 台帳は `frame` の内側に閉じているので、起動の結線（`wire_emo2_boot`）はこの口から
    /// 触る。呼ぶのは **`insert_non_send` より前・最初の `FrameFinalize` より前**の 1 回だけで
    /// あり、そのとき取り出しの相はまだ 1 度も走っていない——ゆえに基底はタグの実行を待たずに
    /// 最初の維持の巡から効く（design「descript 起動時適用」）。
    ///
    /// 解釈・拒否・記録はすべて [`apply_descript_base`] が持つ。ここは台帳への到達だけを
    /// 引き受け、判断を 1 つも持たない。
    ///
    /// [`apply_descript_base`]: super::zorder_descript::apply_descript_base
    pub(in crate::emo2_boot) fn seed_zorder_descript_base(&mut self, zorder_raw: Option<&str>) {
        apply_descript_base(&mut self.zorder_ledger, zorder_raw);
    }

    /// 当たり判定 resolver への読み口（design DD-IE-9/DD-IE-10・「Modified Files」mod.rs 行）。
    ///
    /// 内包する [`EmoPresenter`] を読み取り専用で貸し出す。`input-events` の region 解決
    /// （`RegionSource::Presenter`＝task 2.6/2.7）が、この借用を
    /// [`super::hit_region::resolve_hit_region`]`(presenter, scope, x, y)` の第 1 引数
    /// （`&EmoPresenter`）へそのまま渡して当たり判定を解決する（Req 1.3・collision-geometry の
    /// 契約を消費のみ）。所有・可変アクセスは presenter を専有する frame 相（attach/drain/text）に
    /// 閉じたまま、UI 配線層へは read 口のみを開ける（本番表面を最小に保つ）。
    ///
    /// 本番の呼び手は結線済み（実測）——`input_events/mod.rs:316` が
    /// `.map(Emo2Wiring::presenter)` の関数パス形で借り、`resolve_hit_owned`（`:309`）が当たり判定へ
    /// 渡す。到達の起点は `main.rs` の `attach_char_pointer_handlers` 呼出（キャラ窓ポインタ
    /// ハンドラ装着）と `main.rs` の `wire_mouse_input` 呼出。呼び出しがメソッド構文でないため
    /// `.presenter()` の grep では見つからないが、dead_code 判定上は live である。
    pub(crate) fn presenter(&self) -> &EmoPresenter {
        &self.presenter
    }

    /// 文字層 runtime への共有ハンドル読み口（design「アクセサ（emo2_boot/frame.rs）」・
    /// `Emo2Wiring::runtime()`・Req 4.1）。
    ///
    /// choice-interact のバルーン選択肢対話配線（`super::super::input_events::balloon`）が、この借用を
    /// 経由して `TextLayerRuntime` を読み取り選択肢ハイライト／確定を橋渡しする。既存 [`presenter()`]
    /// アクセサと同型の additive な読み口であり、挙動は一切変えない（`runtime` の所有・可変アクセスは
    /// frame 相の text フェーズに閉じたまま、配線層へは read 口のみを開ける）。上流クレート
    /// （`areka-emo-text`）には一切手を入れない（R8.5）。
    ///
    /// 本番の呼び手は結線済みで 3 箇所ある（実測・いずれも `Rc::clone` して world 側の借用を即座に
    /// 解く形）——`input_events/balloon.rs:394`（ポインタ移動での選択肢 hover 追従）・`:564`
    /// （クリックによる選択確定）・`:748`（バルーン離脱での hover 解除）。到達の起点は
    /// `main.rs` の `wire_balloon_choice` と `attach_balloon_pointer_handlers` 呼出。
    ///
    /// [`presenter()`]: Self::presenter
    pub(crate) fn runtime(&self) -> &Rc<RefCell<TextLayerRuntime>> {
        &self.runtime
    }

    /// `\![move]` 指令受信端への test-support アクセサ（task 9.1 の存在檻・9.3 の e2e で消費）。
    ///
    /// 本番の frame 相 drain（task 9.2）は `move_rx` を private に閉じて `apply_move_directive` へ
    /// 適用する。9.1 段階では channel 配線の到達性（`MoveCueSink`→`Emo2Wiring` の受信端が届く）を
    /// 決定論に固定するための最小 read 口として `#[cfg(test)]` で開ける（本番表面は増やさない）。
    #[cfg(test)]
    pub(crate) fn drain_move_directives(&self) -> Vec<MoveDirective> {
        self.move_rx.try_iter().collect()
    }

    /// 表示ライフサイクル信号受信端への test-support アクセサ（task 4.1 の配線檻で消費）。
    ///
    /// 本番の可視性相は `lifecycle_rx` を private に閉じたまま同じ全件取り出しを行い、
    /// 判断中核の観測スナップショットへ載せる。4.1 段階では channel 配線の到達性
    /// （会話の再生が `BalloonLifecycleSink`→`Emo2Wiring` の受信端へ届く）を決定論に固定する
    /// ための最小 read 口として `#[cfg(test)]` で開ける（`drain_move_directives` と同型・本番
    /// 表面は増やさない）。
    #[cfg(test)]
    pub(crate) fn drain_lifecycle_signals(&self) -> Vec<TalkLifecycleSignal> {
        self.lifecycle_rx.try_iter().collect()
    }

    // ── spine 観測用 test-support アクセサ（tasks.md task 6.2・spine S1/S3/S4） ──────────
    //
    // 本番結線（`wire_emo2_boot`＝task 5.1／`emo2_frame_system`）は `presenter`/`rx` を private に
    // 閉じ、drain/apply/readback は `run_attach_phase`／`run_drain_phase` 内で完結する。決定論 spine
    // （兄弟モジュール `super::spine`・`#[cfg(test)]`）は「受信 `PresentCommand` 列の形状記録」
    // （apply 前に値取り出し）と「apply 後の `read_back` 観測」（R8.2・観測境界をアダプタ記録に
    // 留めない）を行うため、private フィールドへ最小の read/passthrough を要する。以下 3 つは
    // getter/passthrough のみ（本番ロジックは一切変えない）で `#[cfg(test)]` ゲートし本番表面を増やさない。

    /// target の表示中画素（BGRA・`stride=width*4`）を読み戻す（`EmoPresenter::read_back` passthrough・S1/S3/S4）。
    #[cfg(test)]
    pub(crate) fn read_back_target(
        &self,
        target: TargetId,
    ) -> Result<Vec<u8>, areka_emo_present::PresentError> {
        self.presenter.read_back(target)
    }

    /// rx にキュー済みの `PresentCommand` を非ブロックで FIFO 全件取り出す（S3/S4 の受信列記録用）。
    ///
    /// `run_drain_phase` と同じ `Receiver::try_iter` だが、spine は形状記録のため apply 前に**値**として
    /// 取り出す（`PresentCommand` は `reply: Option<ReplySender>` ゆえ非 Clone・move で受ける）。
    #[cfg(test)]
    pub(crate) fn drain_received(&mut self) -> Vec<PresentCommand> {
        self.rx.try_iter().collect()
    }

    /// 1 件の `PresentCommand` を presenter へ適用する（`EmoPresenter::apply` passthrough・S3）。
    ///
    /// `drain_received` で取り出した指令を、形状記録後に実 presenter へ流して実描画→readback まで
    /// 通す（R8.2）ための最小口。本番は同じ `apply` を `run_drain_phase` が呼ぶ。
    #[cfg(test)]
    pub(crate) fn apply_present(&mut self, world: &mut World, cmd: PresentCommand) {
        self.presenter.apply(world, cmd);
    }

    /// 外部所有（`VisibilityOwnership::External`）な target を可視化する
    /// （`EmoPresenter::show_target` passthrough・`areka-P0-balloon-visibility` task 4.2）。
    ///
    /// attach 相はバルーンを**不可視のまま確立**するため、spine が「可視バルーン」を前提とする
    /// ケースはその前提を自分で組む必要がある。本番で可視化を発行するのはバルーン可視性制御
    /// の相であり、本口はそれを spine から代行するだけの passthrough である。
    #[cfg(test)]
    pub(crate) fn show_balloon_target(
        &mut self,
        world: &mut World,
        target: TargetId,
    ) -> Result<(), areka_emo_present::PresentError> {
        self.presenter.show_target(world, target)
    }

    /// 再追従用に記憶している [`BalloonModel`] の scope 集合（昇順・emo-dpi-scaling D11-3 の観測口）。
    ///
    /// 「attach 相が per-scope の model を実際に保持したか」は本番 attach（GPU 資源＋実資産）を
    /// 通さないと観測できないため、spine（in-crate GPU ハーネス）から見えるだけの read を開ける。
    #[cfg(test)]
    pub(crate) fn balloon_model_scopes(&self) -> Vec<u32> {
        let mut scopes: Vec<u32> = self.balloon_models.keys().copied().collect();
        scopes.sort_unstable();
        scopes
    }
}
