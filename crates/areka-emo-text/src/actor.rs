//! # actor — UI ドレインとフレーム提示ステップ（結線層）
//!
//! `spawn_emo_text`（`spawn_ui` 結線・UI ドレイン起動）・`TextLayerRuntime`
//! （UI スレッド所有の集約ルート）・`TextSlotBinding`・`present_frame`
//! （毎フレームの注入時刻駆動：リビール進行→レイアウト→描画→装着）を担う。
//!
//! **層規律**: 結線層。終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`、
//! 全 `UiSender` drop＝drain 正常終了（いずれも error ログなし）。個別メッセージの処理失敗は
//! `Err` 戻し→基盤が `error!`＋継続（log-first・ループを殺さない）。

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::rc::Rc;

use areka_actor::UiSpawnError;
use areka_emo_compose::{BoxLayout, BoxName, BoxPlacement};
use areka_emo_present::TextSlotView;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use tracing::debug;

use crate::choice::ResolvedChoiceStyle;
use crate::cursor_tag::CursorWarnGuard;
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, DWriteMetrics, ResolvedFont};
use crate::lookahead::{TalkLookahead, advance_state};
use crate::place::PlaceKey;
use crate::region::{ScaleContract, TextRegion};
use crate::sink::{EmoTextSink, TextMsg, handle_text_msg_with};
use crate::state::{SurfaceKeyOutcome, TextLayerConfig, TextLayerState};
use crate::surface::TextSurface;
use crate::viewbox_draw::{DrawStats, ViewboxExecutor};
use crate::wrap::WrapMode;
use crate::writing::WritingMode;

/// actor の装着先（結線側が emo-present `TextSlotView` から構築して routing へ登録する）。
///
/// [`crate::surface::TextSurface::attach`] の入力。emo-present は actor を知らない
/// （層純度維持・R9.5）——`ActorKey → TargetId` の対応は結線側（example/emo2-boot）が所有し、
/// `text_slot_view(target)` で得た view の値から本型を [`Self::new`] で組む。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSlotBinding {
    /// 予約スロット（`emo-text-layer-slot` Visual entity・emo-present `VisualMount` が予約）。
    pub slot: Entity,
    /// 装着先の窓 entity。
    pub window: Entity,
    /// 合成スケール k（`TextSlotView.scale` 由来・**窓 DPI 由来で 1.0 とは限らない**）。
    /// 不正値は構築時に [`ScaleContract`] の縮退規約（warn!＋1.0）で正規化済み。
    pub scale: f32,
    /// バルーン surface の物理原寸（TextSurface/swapchain の物理化に使用）。
    pub surface_size: (u32, u32),
    /// 画像座標空間の原寸（負値=反対辺解決・`TextRegion::resolve` の入力）。
    ///
    /// **作者画像空間の原寸そのもの＝k 不変**。emo-present の native 原寸
    /// （`TextSlotView::surface_size`＝k 適用**前**）を**そのまま透過**する。
    /// `TextRegion::resolve` へ物理 px を渡すのはレビューエラー
    /// （2 空間モデルの綻び目をここで構造閉塞——design.md「DPI/スケール契約」）。
    pub image_size: (u32, u32),
}

impl TextSlotBinding {
    /// `TextSlotView` の読み値（slot/window/scale/物理原寸/native 原寸）から binding を構築する。
    ///
    /// k の正規化（0 以下・非有限→warn!＋1.0 縮退）は [`ScaleContract::new`] に委譲する。
    ///
    /// # image px 原寸は**導出しない**（2026-07-30 是正・k<1 の 1px 往復欠陥）
    ///
    /// 旧実装は `image_size = round(physical_size / k)` と**逆写像で復元**していたが、これは
    /// k<1 で厳密な逆写像にならない。順写像は丸め権威 `ScaleRatio::scale_len`（round half away
    /// from zero）で誤差が ±0.5 **物理**px に収まるが、逆写像は k で割るためその誤差が
    /// ±0.5/k **画像**px へ増幅される——k>1 なら 0.5 未満で `round` が厳密に復元するのに対し、
    /// **k<1 では 0.5 を超えて 1px ずれる**（例: k=4/5・native 143 → physical 114 →
    /// `round(114/0.8)` = 142）。k<1 は本番到達可能である（`parse_author_dpi` は任意の宣言値を
    /// 素通しするため、`dpi,120` を宣言したゴーストを 100% モニタで表示すれば k=4/5 になる）。
    ///
    /// 復元しようとしていた値は presenter が**既に正確に持っている**（`TextSlotView::surface_size`
    /// ＝native）。ゆえに逆写像そのものを廃し、native を第 5 引数で受け取って透過する——
    /// 往復が構造ごと消滅し、k に依らず厳密になる。k≥1 では旧実装とバイト同一。
    pub fn new(
        slot: Entity,
        window: Entity,
        scale: f32,
        surface_size: (u32, u32),
        image_size: (u32, u32),
    ) -> Self {
        let contract = ScaleContract::new(scale, None);
        TextSlotBinding {
            slot,
            window,
            scale: contract.scale,
            surface_size,
            image_size,
        }
    }

    /// emo-present の読み取り専用増分 `TextSlotView` からの一点変換（結線の正準口・R9.1/R9.2）。
    ///
    /// # 2 つの寸を**両方**読む（R8.2）
    ///
    /// [`TextSlotView`] は **native 原寸**（`surface_size()`＝k 適用**前**）と**物理寸**
    /// （`physical_size()`＝丸め権威 `scaled_extent` を通した表示寸）を隣り合わせで公開する。
    /// 本メソッドは前者を `image_size` へ、後者を `surface_size` へ写す:
    ///
    /// - `image_size` ← `surface_size()`（native）: 作者画像空間の原寸は**k 不変**でなければ
    ///   ならない（R8.2 の供給面 `ceil(validrect 寸 × k)` が k に比例する前提）。
    /// - `surface_size` ← `physical_size()`（k 適用後）: 診断・churn 判定用の表示寸。
    ///
    /// 取り違えるとどちらの向きでも静かに壊れる——`image_size` に物理寸を入れれば画像空間が
    /// k 倍に膨らみ、`surface_size` に native を入れれば表示寸の記録が k に追随しなくなる
    /// （`presenter.rs` の `physical_size` doc が警告する「消費点での 1 トークンの取り違え」）。
    pub fn from_view(view: &TextSlotView) -> Self {
        TextSlotBinding::new(
            view.slot(),
            view.window(),
            view.scale(),
            view.physical_size(),
            view.surface_size(),
        )
    }
}

/// actor 1 人分の layout 入力——`writing_mode`／領域／フォント／折返しモードの解決済み束
/// （design.md `TextLayerRuntime.layout_input` の値型）。
///
/// 解決はすべて既存の一点解決口（[`WritingMode::resolve`]／[`TextRegion::resolve`]／
/// [`ResolvedFont::resolve`]／[`WrapMode::resolve`]）の合成であり、本型は束ねるだけで
/// 独自解釈を持たない。
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedBalloonText {
    /// 2 層マージ済み `writing_mode` の解釈結果。
    pub mode: WritingMode,
    /// 解決済みテキスト領域（validrect 絶対矩形・描画開始点・折返し閾値——全値 image px）。
    pub region: TextRegion,
    /// 解決済みフォント一式（欠落は ukadoc 既定で充足済み）。
    pub font: ResolvedFont,
    /// 折返しモードの解釈結果（`budoux_newline` 語彙解決・ON 時のみ分かち書き境界を計算）。
    pub wrap: WrapMode,
    /// hover ハイライトスタイルの解決正規形（balloon `cursor.*` モデル＋既定文字色から一点解決）。
    /// 装飾（`decorate_canvas`）が hover 行へ焼く塗り/文字色の源（design.md RuntimeContract・R4.2/4.3）。
    pub choice_style: ResolvedChoiceStyle,
}

impl ResolvedBalloonText {
    /// balloon model（2 層マージ済み）とバルーン画像原寸（**image px**・
    /// [`TextSlotBinding::image_size`] の一点導出値）から解決する。
    /// 物理 px を渡すのはレビューエラー（2 空間モデル——design.md「DPI/スケール契約」）。
    pub fn resolve(model: &BalloonModel, image_size: (u32, u32)) -> ResolvedBalloonText {
        ResolvedBalloonText::resolve_with_background(model, image_size, DEFAULT_BALLOON_BACKGROUND)
    }
}

/// choice.rs（純粋層）所有のバルーン窓物理 px 矩形を結線層から再輸出する（design.md RuntimeContract）。
/// 下流（choice-interact）は [`ChoiceHitRow::rect`] を本型で受ける——照会契約の座標系正本。
pub use crate::choice::HitRectPx;

/// 行ヒットジオメトリ契約（本 spec 正本・choice-interact が消費・design.md RuntimeContract）。
///
/// 提示フレーム同期スナップショットの 1 行分——1 選択肢セグメントの窓物理 px 矩形に、
/// 下流 `ChoiceSelection` 構成材料（`ordinal`/`id`/`label`/`references`）を同梱し、契約の
/// 再照会を不要にする（design.md「下流契約」）。スナップショットの population は present_actor
/// （task 8.2）が担い、本 task（8.1）では per-actor スナップショットは空のまま。
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceHitRow {
    /// スパンの配送順序数（[`crate::state::ChoiceSpan::ordinal`]・hover 注入／選択解決の主キー）。
    pub ordinal: usize,
    /// `\q` ID（不透明転写）。
    pub id: String,
    /// 表示文字列（不透明転写）。
    pub label: String,
    /// `\q` 第 3 引数以降（参照列・不透明転写）。
    pub references: Vec<String>,
    /// ヒット矩形（バルーン窓物理 px・スクロール committed 反映済み・[`HitRectPx`]）。
    pub rect: HitRectPx,
}

/// actor 1 人分の描画資源（供給面＋描画実行部＋実測 metrics——font/mode に束縛されるため
/// actor 別に持つ。行 TextLayout キャッシュの index 衝突・format 組み直しの構造回避）。
struct ActorRender {
    /// 自前 swapchain 供給面（初回のみ予約スロットへ brush 装着・以降 Present のみ）。
    surface: TextSurface,
    /// viewbox ダーティ矩形スクロールの実行部（保持ピクセルの面内 blit ＋ ダーティ矩形限定描画・
    /// 行 TextLayout キャッシュは actor の行 index に束縛）。
    executor: ViewboxExecutor,
    /// 計測専用 probe 由来の実測 metrics（actor の font/mode に束縛）。
    metrics: DWriteMetrics,
}

/// `\s` の鍵を今のシェルで解決する閉包（結線が渡す・文字の層は seriko の型を名指ししない）。
///
/// 中身は結線の純関数（`resolve_for_text`）で、今のシェルの面の表に無い番号は
/// [`SurfaceKeyOutcome::Unresolved`] を返す（design.md「箱の束の結線」）。
pub type SurfaceKeyResolver = Box<dyn Fn(&str) -> SurfaceKeyOutcome>;

/// UI スレッド所有の集約ルート（NonSend・design.md「TextLayerRuntime」正本）。
///
/// 純粋状態（[`TextLayerState`]）・binding（結線側が登録）・COM 資源（actor 別の
/// 供給面/描画実行部——World 資源から遅延構築）を束ねる。`Rc<RefCell<_>>` で
/// [`spawn_emo_text`] の UI ドレインとフレーム提示ステップ（[`present_frame`]）が
/// 同一 UI スレッド上で共有する（`!Send`——スレッドを跨がない）。
///
/// - cue 適用（[`apply_cue`](Self::apply_cue)）は純粋状態の更新のみで World に触れない。
/// - World への装着・描画はフレーム提示ステップ（[`present_frame`]）が担う。
/// - 時刻は常に注入（`talk_time`）——内部で `Instant::now()` を読まない（R3.3）。
pub struct TextLayerRuntime {
    /// 純粋状態機械（actor 別・cue 列→行/グリフ状態）。
    state: TextLayerState,
    /// 場所 → 装着先（予約スロット）。結線側が [`register_actor`](Self::register_actor) で登録
    /// （スコープで引く口は普通のバルーンの場所 [`PlaceKey::balloon`] を引く）。
    routing: HashMap<PlaceKey, TextSlotBinding>,
    /// 場所 → layout 入力（writing_mode/region/font の解決済み束）。routing と対で登録。
    layout_input: HashMap<PlaceKey, ResolvedBalloonText>,
    /// actor 別の描画資源（遅延生成——`ActorRender` 不在の解決フレームで World 資源から構築・装着。
    /// k 再追従（[`TextLayerRuntime::refresh_actor_scale`]）は本 map の当該エントリだけを破棄し、
    /// 次フレームの再生成へ委ねる——純粋状態 `state` には触れない・R8.2/R8.3）。
    surfaces: HashMap<PlaceKey, ActorRender>,
    /// 調整値（行送りの行間 `line_gap`）。reveal ペースは配送 duration 由来ゆえ char_wait は
    /// 持たない（本 config は `DWriteMetrics::new` の行送り算出にのみ使う）。
    /// [`TextLayerRuntime::new`] で正規化済み——以降はこの値をそのまま配る。
    config: TextLayerConfig,
    /// 未解決 actor の warn を actor ごと初回のみに抑える記録（以降は debug!——
    /// design Error Handling「未 binding actor の cue」）。
    unresolved_warned: BTreeSet<PlaceKey>,
    /// actor → hover 注入状態（`None`＝ハイライト無し・[`inject_choice_hover`](Self::inject_choice_hover)
    /// で更新・present_actor の装飾（task 8.2）が読む・UI スレッド専用）。
    choice_hover: HashMap<PlaceKey, Option<usize>>,
    /// actor → 提示フレーム同期ヒット行スナップショット（[`choice_hit_rows`](Self::choice_hit_rows)
    /// の照会源）。population は present_actor（task 8.2）が present 成功時に行う——本 task では空のまま。
    choice_snapshot: HashMap<PlaceKey, Vec<ChoiceHitRow>>,
    /// `\_l` の座標解決の縮退（`CursorDegrade`＝`Unparsable`／`CenterAxisMismatch`・5.1〜5.3）の actor ごと warn-once 持続状態。present_actor が
    /// [`LayoutEngine::layout_styled`] へ `&mut` で渡す持続 guard——per-frame layout 呼出での
    /// 重複警告を走査を跨いで抑止する（`unresolved_warned` と同型・行出力へは影響しない）。
    cursor_warn: CursorWarnGuard,
    /// actor → バルーンの背景色（面 0 の原点画素・sRGB 非 premultiplied）。無効表示の見た目の
    /// 色を導くためだけに使う（要件 4.6）。結線側が装着の**前**に
    /// [`set_balloon_background`](Self::set_balloon_background) で入れる——未設定は
    /// [`DEFAULT_BALLOON_BACKGROUND`]（白）。読み口は
    /// [`background_of`](Self::background_of)（`actor_decoration.rs`）。
    balloon_background: HashMap<ActorKey, (u8, u8, u8)>,
    /// actor → 普通のバルーンの名前（結線が入れるバルーンのフォルダ名）。折り返しの基準と無視した
    /// 書き出し位置の 2 つの警告の名前の欄に出す（未設定は `スコープ{番号}のバルーン`・要件 3.12）。
    /// 入れる口は [`set_balloon_label`](Self::set_balloon_label)（`actor_decoration.rs`）。
    balloon_label: HashMap<ActorKey, String>,
    /// `\s` の鍵の解決の閉包（始めは無し）。無いあいだは `Emote` を読まない＝行き先は常に
    /// 普通のバルーン（要件 5.4）。本番の差し込みは箱の束の受け取り（`actor_box.rs`）が行う。
    surface_resolver: Option<SurfaceKeyResolver>,
    /// 箱の `font.name` のフォントファイルを探す場所の順（シェル → ゴースト・要件 3.10）。
    /// 箱の束の受け取りが入れ、読み込む側（`balloon-font-file`）が読む。
    box_font_dirs: Vec<PathBuf>,
    /// 「箱を隠す印」の立っているスコープ（`actor_box.rs` の `hide_boxes`）。台詞の頭（`ClearAll`）で下ろす。
    hidden_boxes: BTreeSet<ActorKey>,
    /// 今のシェルの箱の束の置き場所の表（箱の束の受け取りが入れ、毎フレームの箱の同期が引く）。
    box_layout: BoxLayout,
    /// 箱の場所 → 登録済みの置き場所（`routing`・`layout_input` と対・`actor_box.rs` の `sync_boxes`）。
    box_sites: HashMap<PlaceKey, BoxPlacement>,
    /// はみ出しを警告済みの（サーフェス番号, 箱の名前）。箱の束を差し替えると空に戻す。
    box_overflow_warned: BTreeSet<(u32, BoxName)>,
    /// 定義の 2 つの警告（折り返しの基準・無視した書き出し位置）を出した箱の名前。箱の登録には
    /// 前の配置の入力が毎回無いので、名前ごとに 1 度だけにする。箱の束を差し替えると空に戻す。
    box_definition_warned: BTreeSet<BoxName>,
    /// スコープ → 最後に提示したフレームで文字が 1 字以上見えていた箱の四角（手前から・
    /// `actor_box.rs` の `shown_boxes`）。提示のたびに作り直し、出なくなった箱（`\c`・台詞の頭・
    /// 箱を隠す印・箱の同期が登録を外した箱）はその場で外す。台本の `\s` の受け取りでは外さない
    /// （置き場所は絵の番号に従い、絵が替わった同期で外れる）。
    shown_boxes: HashMap<ActorKey, Vec<ShownBox>>,
    /// 先渡しの空回しで求めた区間の全文の持ち主（[`preview_talk`](Self::preview_talk) が入れ、
    /// 合図の適用が消去と届いた数を数える）。正本ではない（届いた字の正本は `state`）。
    lookahead: TalkLookahead,
}

impl TextLayerRuntime {
    /// 空のランタイムを構築する（COM 資源は初回解決フレームで World 資源から遅延構築）。
    ///
    /// 調整値はここで 1 度だけ正規化する（[`TextLayerConfig::normalized`]）——非有限・負の
    /// 行間の縮退警告は構築時の 1 件だけで、以降は正規化済みの値を配る（R1.6）。
    pub fn new(config: TextLayerConfig) -> TextLayerRuntime {
        TextLayerRuntime {
            state: TextLayerState::default(),
            routing: HashMap::new(),
            layout_input: HashMap::new(),
            surfaces: HashMap::new(),
            config: config.normalized(),
            unresolved_warned: BTreeSet::new(),
            choice_hover: HashMap::new(),
            choice_snapshot: HashMap::new(),
            cursor_warn: CursorWarnGuard::default(),
            balloon_background: HashMap::new(),
            balloon_label: HashMap::new(),
            surface_resolver: None,
            box_font_dirs: Vec::new(),
            hidden_boxes: BTreeSet::new(),
            box_layout: BoxLayout::default(),
            box_sites: HashMap::new(),
            box_overflow_warned: BTreeSet::new(),
            box_definition_warned: BTreeSet::new(),
            shown_boxes: HashMap::new(),
            lookahead: TalkLookahead::default(),
        }
    }

    /// cue を actor 別の純粋状態機械へ適用する（UI ドレインの適用点・World に触れない）。
    ///
    /// `Clear`／`ClearAll` は描画実行部の全域リセット要求点でもある（planner 初期化＋確定行
    /// TextLayout キャッシュの全破棄——design「Clear で全破棄」・破棄はこの口だけ・次フレームは
    /// `FramePlan::FullClear`＝全域透明・R4.3）。純粋状態（[`TextLayerState::apply_cue`]）を
    /// 空にするだけでは既描画サーフェスに古いピクセルが残留するため（#6 欠陥）、提示層の
    /// 描画実行部にも同じ消去を伝える必要がある:
    ///
    /// - `Clear`＝**対象スコープのみ**（`cue.actor` の描画実行部だけをクリア・R6.4/R7.4）。
    /// - `ClearAll`＝**全スコープ**（装着済み全 actor の描画実行部をクリア・#6 の冒頭全消し・
    ///   R6.4/R7.4）。上流は残存スコープを列挙できないため、全消しは本ランタイムが自己完結して
    ///   行う（`state.rs::apply_cue` の全 `actor_states` 消去と対）。
    pub fn apply_cue(&mut self, cue: &TalkCue) {
        // 状態を進める前の行き先（`\c` が消すのは今の行き先の場所だけ＝純粋状態の `apply_cue` と
        // 同じ宛先）。先渡しの区間の番号は空回しと同じく、この行き先で消去を数える（要件 4.1）。
        let place = PlaceKey {
            actor: cue.actor.clone(),
            place: self.state.destination(&cue.actor),
        };
        self.lookahead.note_cue(cue, &place);
        // catch-all を置かず variant を明示し、将来 dola が clear 系 variant を追加した際に
        // コンパイラへ描画実行部側の再検討を強制する（no-catch-all 規律）。
        match &cue.command {
            CueCommand::Clear => {
                if let Some(render) = self.surfaces.get_mut(&place) {
                    render.executor.request_clear();
                }
                // 選択肢ライフサイクルの原子的無効化（R5.1/5.2/5.4）: 当該 actor の hover を None へ
                // リセットし、ヒット行スナップショットを純粋状態の選択肢消去と**同時**に無効化する
                // （表示と hit の片方だけが古い状態に残らない——present を待たず `choice_hit_rows` が空・
                // `choice_active` が false へ揃う）。スパン初期化は下段 `state.apply_cue(Clear)` が
                // items と同一ライフサイクルで担う。snapshot を明示除去するのは、`choice_active` が
                // span 由来で即時に false へ倒れる一方、snapshot は present まで stale 行を保持しうる
                // 隙間を塞ぎ、5.2 の原子性を照会時点で成立させるため（次 present の空再導出と冪等）。
                self.choice_hover.remove(&place);
                self.choice_snapshot.remove(&place);
            }
            CueCommand::ClearAll => {
                for render in self.surfaces.values_mut() {
                    render.executor.request_clear();
                }
                // 全スコープの原子的無効化（R5.1/5.2/5.4・#6 冒頭全消し）: 保持する**全** actor の
                // hover／ヒット行スナップショットを一括初期化する（上流は残存スコープを列挙できない——
                // `state.rs::apply_cue(ClearAll)` の全 actor_states 消去と対）。cue が名指ししない
                // actor の stale hover／snapshot も同時に消え、片方だけ古い状態が残らない（5.2）。
                self.choice_hover.clear();
                self.choice_snapshot.clear();
                // 箱を隠す印は次の台詞の頭まで（design.md「箱を数に入れた表示の判断」）。
                self.hidden_boxes.clear();
            }
            // 他コマンドは描画実行部への全域クリアを要さない（グリフ更新は present_frame が
            // リビール進行として描き、非担当コマンドは reveal を汚さない）。`Cursor` の
            // warn-once 良性スキップ・記録は純粋層 `state.apply_cue` が担う（本口は clear 要否のみ）。
            // `\s` の行き先への受け渡しは下段の `advance_state` が担う（空回しと同じ規則・要件 2.2）。
            CueCommand::Emote { .. }
            | CueCommand::Text(_)
            | CueCommand::Choice { .. }
            | CueCommand::EntityRef(_)
            | CueCommand::Custom { .. }
            | CueCommand::NewLine { .. }
            | CueCommand::Cursor { .. }
            | CueCommand::BalloonSurface { .. }
            | CueCommand::Wait => {}
        }
        advance_state(&mut self.state, self.surface_resolver.as_deref(), cue);
        // `\c`・台詞の頭で文字が無くなった箱を、提示を待たずに写しから外す。`\s` の受け取りは
        // 行き先だけを替え、箱の置き場所は絵の番号に従う（次の箱の同期が決める）ので外さない。
        self.prune_shown_boxes();
    }

    /// 先渡し（再生の前に渡された、これから届く合図の全部）を受け取る: 今の状態の写しで空回しし、
    /// 区間の全文を入れ替える（要件 2.1）。受け取ったことを `debug!` 1 行（合図の数・求めた区間の数）で
    /// 残す——実機の確かめは、最初の字の適用より前にこの行があることを読む（design.md「Monitoring」）。
    fn preview_talk(&mut self, upcoming: &[TalkCue]) {
        self.lookahead
            .install(&self.state, self.surface_resolver.as_deref(), upcoming);
        debug!(
            cues = upcoming.len(),
            sections = self.lookahead.section_count(),
            "先渡しを受け取った——空回しで区間の全文を求めた"
        );
    }

    /// 純粋状態機械（可視グリフ数・actor 状態の読み取り口）。
    pub fn state(&self) -> &TextLayerState {
        &self.state
    }

    /// 調整値（行送りの行間 `line_gap`・構築時に正規化済み）。
    pub fn config(&self) -> &TextLayerConfig {
        &self.config
    }

    /// actor の供給面が予約スロットへ装着済みか。
    pub fn is_attached(&self, actor: &ActorKey) -> bool {
        self.surfaces.contains_key(&PlaceKey::balloon(actor))
    }

    /// 装着済み actor の供給面（readback 等の観測口・未装着は `None`）。
    pub fn surface(&self, actor: &ActorKey) -> Option<&TextSurface> {
        self.surface_at(&PlaceKey::balloon(actor))
    }

    /// 場所（普通のバルーンか箱）の供給面（readback 等の観測口・面が無ければ `None`）。
    pub fn surface_at(&self, place: &PlaceKey) -> Option<&TextSurface> {
        self.surfaces.get(place).map(|render| &render.surface)
    }

    /// 装着済み actor の決定論観測統計（[`ViewboxExecutor::stats`]・未装着は `None`）。
    ///
    /// [`surface`](Self::surface) と同型の additive アクセサ（R9.2 非抵触——emo2-boot 消費経路の
    /// 再定義ではない）。example／統合テストがこの口から actor 別の [`DrawStats`]
    /// （blit・`DrawTextLayout` 実行回数・行 TextLayout 生成回数・FullClear 回数）を読み、
    /// 「可視窓のみ移動フレームで確定 content の再描画が起きない」等を決定論的に観測する
    /// （R3.5/R10.3・目視非依存）。`ViewboxExecutor::stats()` は runtime 内部の `ActorRender` に
    /// 抱えられており、この読み口がないと example から R10.3 checkpoint が成立しない。
    pub fn draw_stats(&self, actor: &ActorKey) -> Option<DrawStats> {
        self.surfaces
            .get(&PlaceKey::balloon(actor))
            .map(|render| render.executor.stats())
    }

    /// hover 状態注入（契約正本・R4.1）。`None`＝ハイライト無し。UI スレッド専用（runtime は `!Send`）。
    ///
    /// 注入値は actor ごとに保持し、次の提示フレームの装飾（task 8.2）が読む。`ordinal` が現存
    /// 選択肢スパンに無い場合も **panic せず**そのまま保持し、描画時に「ハイライト無し」として
    /// 縮退する（stale ordinal——`decorate_canvas` が hover 印を付けない・design.md RuntimeContract）。
    /// 縮退検出時は `debug!` を一件出す（log-first・ループを殺さない）。
    pub fn inject_choice_hover(&mut self, actor: &ActorKey, hover: Option<usize>) {
        self.inject_choice_hover_at(&PlaceKey::balloon(actor), hover);
    }

    /// 場所を指す hover 状態注入（箱の選択肢の強調・要件 8.2）。意味は
    /// [`inject_choice_hover`](Self::inject_choice_hover) と同じで、宛先が場所の鍵になる。
    pub fn inject_choice_hover_at(&mut self, place: &PlaceKey, hover: Option<usize>) {
        // 現存スパンに無い ordinal は縮退（ハイライト無し）——検出を debug ログに残す（panic しない）。
        if let Some(ordinal) = hover {
            let exists = self
                .state
                .place_state(place)
                .is_some_and(|s| s.choices().iter().any(|span| span.ordinal == ordinal));
            if !exists {
                debug!(
                    actor = %place.actor,
                    ordinal,
                    "inject_choice_hover: 現存選択肢スパンに無い ordinal——ハイライト無しとして縮退（保持のみ・panic なし）"
                );
            }
        }
        self.choice_hover.insert(place.clone(), hover);
    }

    /// 行ヒットジオメトリ照会（契約正本・R3.2）。
    ///
    /// **鮮度契約**: 最後に提示（present）したフレームの導出値＝表示と同一 layout からの単一導出
    /// （R3.3/5.2・population は present_actor＝task 8.2）。未装着・選択肢なし・スナップショット未
    /// population は空 slice。
    pub fn choice_hit_rows(&self, actor: &ActorKey) -> &[ChoiceHitRow] {
        self.choice_hit_rows_at(&PlaceKey::balloon(actor))
    }

    /// 場所を指す行ヒットジオメトリ照会（要件 8.2）。箱の場所の矩形は**シェルの窓**の物理 px
    /// （箱の位置を足し済み）。鮮度契約は [`choice_hit_rows`](Self::choice_hit_rows) と同じ。
    pub fn choice_hit_rows_at(&self, place: &PlaceKey) -> &[ChoiceHitRow] {
        self.choice_snapshot.get(place).map_or(&[], Vec::as_slice)
    }

    /// 「選択肢表示中」照会（R1.3・照会のみ＝バリア解決はしない）。
    ///
    /// **表示層自身**の選択肢スパン集合（[`ActorTextState::choices`](crate::state::ActorTextState::choices)）が
    /// 非空であることを表す（DD-6——供給側 `CuePlayerState::WaitingForChoice` バリアの真実源とは別）。
    /// スコープの**どの場所か**（普通のバルーン・箱）に選択肢があれば真（要件 8.4——時間切れの
    /// 待ちを止める条件と kanade の選択待ちの単位がスコープだから）。未知 actor・スパン空は `false`。
    pub fn choice_active(&self, actor: &ActorKey) -> bool {
        self.state
            .places()
            .any(|(key, s)| key.actor == *actor && !s.choices().is_empty())
    }

    /// `\s` の解決の閉包だけを差し込む（テスト専用の口・本番は箱の束の受け取り
    /// `set_box_layout` が表と一緒にこの欄へ入れる）。
    #[cfg(test)]
    pub(super) fn set_surface_resolver(&mut self, resolve: SurfaceKeyResolver) {
        self.surface_resolver = Some(resolve);
    }

    /// 検査用の読み口: その場所の配置の入力を引いて [`present::arrange_lines`] を呼ぶだけ
    /// （`present_actor` が冒頭でしている引き当てと同じ）。配置の入力か状態が無ければ `None`。
    /// 字幅は呼び手が渡す（決まった字幅で GPU の資源なしに本番と同じ手順の行の列を取る）。
    #[cfg(test)]
    pub(super) fn arrange_for_test(
        &mut self,
        place: &PlaceKey,
        metrics: &dyn crate::layout::GlyphMetrics,
        talk_time: f64,
    ) -> Option<Vec<crate::layout::PositionedLine>> {
        let resolved = self.layout_input.get(place)?;
        present::arrange_lines(
            &self.state,
            &mut self.cursor_warn,
            place,
            resolved,
            metrics,
            talk_time,
        )
    }
}

/// 結線 API（UI スレッド＝pump スレッドから呼ぶ・design.md「TextLayerActor」正本）:
/// `spawn_ui` で UI ドレインを起動し、受信口 [`EmoTextSink`] と drain の join ハンドルを返す。
///
/// handler は `runtime` の `Rc` clone を捕捉し（`!Send` handler・基盤許容）、
/// [`handle_text_msg_with`]（終了規律の正準写像）へ委譲して cue を純粋状態へ適用する。
/// 先渡し（`TextMsg::Upcoming`）は実行時の `preview_talk` へ渡し、空回しで区間の全文を求める。
/// 終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`・全 `UiSender`
/// （＝全 [`EmoTextSink`] クローン）drop＝drain 正常終了（R1.4・error ログなし）。
/// 個別 cue の適用失敗（runtime 借用競合など）は `Err` 戻し→基盤が `error!`＋継続する
/// （R1.5——失敗は終了経路ではない・panic しない）。
///
/// # 前提
///
/// UI（pump）スレッドから呼ぶこと。誤用は呼出時検出不能（基盤既知リスク——`spawn_ui` の
/// Risks 参照・spawn 時 debug! 診断で緩和）。
pub fn spawn_emo_text(
    runtime: Rc<RefCell<TextLayerRuntime>>,
) -> Result<(EmoTextSink, wintf_winmsg_executor::JoinHandle<()>), UiSpawnError> {
    let (tx, handle) = areka_actor::spawn_ui("emo-text", move |msg: TextMsg| {
        handle_text_msg_with(
            msg,
            |cue| match runtime.try_borrow_mut() {
                Ok(mut rt) => {
                    rt.apply_cue(&cue);
                    Ok(())
                }
                // 借用競合（UI スレッド上の別処理が runtime を保持中）——panic せず Err 戻しで
                // 基盤の error!＋継続に乗せる（当該 cue は失われるが後続の受理は破壊しない・R1.5）。
                Err(_) => Err(format!(
                    "TextLayerRuntime が借用中のため cue を適用できない（actor={}, at={}）——当該 cue は失われる",
                    cue.actor, cue.at
                )),
            },
            |upcoming| match runtime.try_borrow_mut() {
                Ok(mut rt) => {
                    rt.preview_talk(&upcoming);
                    Ok(())
                }
                // 借用競合——cue と同じく Err 戻しで基盤の error!＋継続に乗せる（そのトークは
                // 先渡し無し＝修正前の動きになり、文節の折り返しでは warn が出る）。
                Err(_) => Err(format!(
                    "TextLayerRuntime が借用中のため先渡しを受け取れない（cues={}）——そのトークは先渡し無し",
                    upcoming.len()
                )),
            },
        )
    })?;
    Ok((EmoTextSink::new(tx), handle))
}

#[path = "actor_attach.rs"]
mod attach;

#[path = "actor_box.rs"]
mod boxes;

#[path = "actor_present.rs"]
mod present;

pub use boxes::ShownBox;
pub use present::present_frame;

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "actor_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "actor_runtime_frame_tests.rs"]
mod runtime_frame_tests;

#[cfg(test)]
#[path = "actor_choice_contract_tests.rs"]
mod choice_contract_tests;

#[cfg(test)]
#[path = "actor_clear_atomicity_tests.rs"]
mod clear_atomicity_tests;

#[cfg(test)]
#[path = "actor_scale_refresh_tests.rs"]
mod scale_refresh_tests;

#[cfg(test)]
#[path = "actor_region_warn_tests.rs"]
mod region_warn_tests;

#[cfg(test)]
#[path = "actor_scroll_retain_tests.rs"]
mod scroll_retain_tests;

#[cfg(test)]
#[path = "actor_route_tests.rs"]
mod route_tests;

#[cfg(test)]
#[path = "actor_lookahead_tests.rs"]
mod lookahead_tests;

#[cfg(test)]
#[path = "actor_box_tests.rs"]
mod box_tests;

#[cfg(test)]
#[path = "actor_box_sync_tests.rs"]
mod box_sync_tests;

#[cfg(test)]
#[path = "actor_box_present_tests.rs"]
mod box_present_tests;

/// task 7.2: バルーン背景色の受け口（要件 4.6）。
#[path = "actor_decoration.rs"]
mod decoration;

#[cfg(test)]
#[path = "actor_decoration_tests.rs"]
mod decoration_tests;

#[cfg(test)]
#[path = "actor_decoration_frame_tests.rs"]
mod decoration_frame_tests;
