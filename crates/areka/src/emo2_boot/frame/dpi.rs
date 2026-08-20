//! DPI 追従フェーズ（[`AuthorDpis`]・[`classify_ghost_window`]・[`run_dpi_phase`] ほか）。

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Changed, Query, With};
use bevy_ecs::system::SystemState;
use bevy_ecs::world::World;
use tracing::{debug, error, warn};

use areka_emo_present::{EmoPresenter, TargetId};
use wintf::ecs::{WindowPos, DPI};

use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
use crate::placement::dpi_sync::{self, DpiSyncHold};
use crate::placement::follow::{resize_window_keep_position, resize_window_to};
use crate::placement::resolver::SizePx;
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

use super::{Emo2Wiring, PlannedAttach, balloon_target, shell_target};

// ---------------------------------------------------------------------------
// DPI 追従フェーズ（areka-P0-emo-dpi-scaling task 4.2・design「areka / emo2_boot（DPI 追従
// フェーズ）> run_dpi_phase（frame.rs）」・Flow 2／Flow 3 手順 5・D8・Req3.1/4.1/4.2/4.3/4.4）
// ---------------------------------------------------------------------------

/// 装着時に各 target へ渡す原稿 DPI の対（shell 宣言＋balloon 宣言・D1・Req1.1）。
///
/// shell は `seriko.dpi`・balloon は `dpi` と**別宣言**であり、どちらも `u16` ゆえ取り違えても
/// コンパイルは通る——通ったまま「シェルだけバルーンの縮尺で描かれる」という静かな誤表示になる。
/// ゆえに呼び手に「どちらの `u16` か」を選ばせず、**装着対象 target の同一性**で引き当てる
/// （[`Self::for_target`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AuthorDpis {
    /// shell descript の `seriko.dpi`（無宣言・不正は上流 source.rs が 96 へ正規化済み）。
    pub(super) shell: u16,
    /// balloon descript の `dpi`（同上）。
    pub(super) balloon: u16,
}

impl AuthorDpis {
    /// 装着対象 `target` に対応する author_dpi を引く（shell target＝shell 宣言・balloon target＝
    /// balloon 宣言）。
    ///
    /// `item` の 2 target のいずれでもない値は到達＝結線バグゆえ `warn!`＋既定 96 へ縮退する
    /// （表示を失わない縮退・panic しない・log-first）。
    pub(super) fn for_target(self, item: &PlannedAttach, target: TargetId) -> u16 {
        if target == item.shell_target {
            self.shell
        } else if target == item.balloon_target {
            self.balloon
        } else {
            warn!(
                ?target,
                scope = item.scope,
                "emo2 attach: 当該 scope の shell/balloon いずれの target でもない → author_dpi 既定 96 へ縮退"
            );
            96
        }
    }
}

/// ゴースト窓の種別（窓寸 reconcile の**反映口**の選択・`spawn.rs` の marker 由来）。
///
/// キャラ窓とバルーン窓は寸法変更時の位置の決め方が異なる（D8）——キャラ窓はアンカー辺へ
/// 釘付けされた接地点を保つため [`resize_window_to`]、バルーン窓は位置がキャラ窓追従で決まる
/// 従属量ゆえ [`resize_window_keep_position`]（同フレームで二重に位置が動かない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GhostWindowKind {
    /// キャラ窓（`CharWindowMarker`）→ アンカー保存リサイズ。
    Char,
    /// バルーン窓（`BalloonWindowMarker`）→ 位置維持リサイズ。
    Balloon,
}

/// 窓 entity の marker 照合結果（[`classify_ghost_window`] の 3 分類・純判断）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GhostWindowClass {
    /// ゴースト窓（scope＋種別が確定）。
    Ghost(usize, GhostWindowKind),
    /// ゴースト窓でない（`GhostWindowMarker` 系を持たない他の窓）＝本フェーズの対象外。
    NotGhost,
    /// char/balloon の marker が同居している（`spawn.rs` の排他付与に反する結線バグ）。
    Ambiguous,
}

/// 窓に付いた marker（scope 付き）から scope と種別を判定する純関数（GPU/World 不要）。
///
/// `spawn_ghost_windows` は 1 窓へ `CharWindowMarker`／`BalloonWindowMarker` の**どちらか一方**
/// だけを付ける。ゆえに両方あり＝結線バグ（[`GhostWindowClass::Ambiguous`]）、どちらも無し＝
/// ゴースト窓ではない（[`GhostWindowClass::NotGhost`]）として呼び手が縮退できるよう 3 分類で返す。
pub(super) fn classify_ghost_window(
    char_scope: Option<usize>,
    balloon_scope: Option<usize>,
) -> GhostWindowClass {
    match (char_scope, balloon_scope) {
        (Some(scope), None) => GhostWindowClass::Ghost(scope, GhostWindowKind::Char),
        (None, Some(scope)) => GhostWindowClass::Ghost(scope, GhostWindowKind::Balloon),
        (None, None) => GhostWindowClass::NotGhost,
        (Some(_), Some(_)) => GhostWindowClass::Ambiguous,
    }
}

/// 報告された新物理寸を窓 client へ反映する（D8 の振り分け・二経路の共通末端）。
///
/// 種別ごとの反映口（char＝[`resize_window_to`]／balloon＝[`resize_window_keep_position`]）へ
/// 振り分ける。戻り値は**書込が起きたか**であり、`false` は失敗とは限らない——同寸のべき等
/// skip（振動しない・Req4.2）も `false` を返す（ゆえに `false` を error として鳴らさない。
/// 実失敗＝`WindowHandle` 未付与等は各反映口が `warn!` 済み）。
///
/// 物理寸は `u32`（表示バッファ外形）で報告されるが窓寸は `i32` 通貨ゆえ、ここで変換し
/// 超過・0 を弾く（log-first・panic しない・Req3.4 と同流儀の二重防波堤）。
///
/// # `route` 引数（Req 1.2・design D13・task 1.4）
///
/// **本関数は共通末端であって経路ではない**。呼出元は 2 つあり、両者は消失診断の上で
/// 別物である:
///
/// - [`dpi_phase_with`]（`Changed<DPI>` エッジ由来）→ [`PlacementRoute::DpiReproject`]
/// - [`reconcile_reported_sizes`]（drain の後段で行う報告回収・**初回表示の k₀ 補正を含み**
///   `Changed<DPI>` 非依存）→ [`PlacementRoute::ReportedSizeReconcile`]
///
/// ゆえに route を本関数の内部で決め打ちしてはならない——決め打つと DPI 変化ゼロの起動でも
/// 「DPI 由来」の偽レコードが毎回出て、要件 1.9 の受理回数突合（セッション②＝ドラッグ禁止・
/// OS 側 DPI 変更のみ）が汚染される。バルーン窓は位置据置きリサイズ
/// （[`PlacementRoute::KeepPositionResize`]）へ落ちるため route を消費しない。
pub(super) fn reconcile_window_size(
    world: &mut World,
    window: Entity,
    kind: GhostWindowKind,
    new_size: (u32, u32),
    route: PlacementRoute,
) -> bool {
    let (Ok(w), Ok(h)) = (i32::try_from(new_size.0), i32::try_from(new_size.1)) else {
        error!(
            entity = ?window,
            ?kind,
            ?new_size,
            "dpi reconcile: 報告された物理寸が i32 域を超える → 窓寸を変えない（前寸維持・log-first）"
        );
        return false;
    };
    if w == 0 || h == 0 {
        warn!(
            entity = ?window,
            ?kind,
            ?new_size,
            "dpi reconcile: 報告された物理寸に 0 軸がある → 窓寸を変えない（前寸維持）"
        );
        return false;
    }
    let new_size = SizePx { w, h };
    match kind {
        // キャラ窓: アンカー射影 T を再適用して接地点（下端中央）を保つ。
        // 経路タグは呼出元が渡した route を透過させる（本関数は共通末端であって経路ではない
        // ＝DPI 相と drain 後段の報告回収を 1 語で名乗らせない・Req 1.2／D13）。
        GhostWindowKind::Char => resize_window_to(world, window, new_size, route),
        // バルーン窓: 位置は追従で決まる従属量ゆえ据え置き、寸だけ差し替える
        // （経路語彙は関数名そのもの＝KeepPositionResize ゆえ route を消費しない）。
        GhostWindowKind::Balloon => resize_window_keep_position(world, window, new_size),
    }
}

/// 再スケール報告の供給源（本番実装は [`EmoPresenter`]）。
///
/// frame 側の結線が presenter へ求めるのは 2 つの報告だけである——(1) `Changed<DPI>` エッジで
/// 駆動する再表示の結果（[`EmoPresenter::refresh_scale`]）、(2) 表示成立点の状態照合が積んだ
/// 未消費の窓寸要求（[`EmoPresenter::take_pending_resize`]）。両者の**責任分界**（再表示が
/// 成立したら (1) が要求を自ら消費し、ゲート不成立なら一切触れず (2) が拾う）は presenter 側の
/// 契約であり、そちらの檻は emo-present in-crate が所有する。
///
/// この最小トレイトは、**frame 側の結線**（毎フレーム両経路を呼ぶ・`Some` のみ reconcile する・
/// 種別で反映口を振り分ける）を GPU 無しで決定論の檻へ入れるためのシームである（D9 の
/// 振り分け基準 (a)＝判断分岐は in-crate 純テスト、GPU readback は emo-present 別プロセス）。
pub(super) trait ScaleReportSource {
    /// 窓 DPI から k を再導出し、再表示が成立して物理寸が変わったならその新物理寸を返す。
    fn refresh_scale_report(&mut self, world: &mut World, target: TargetId) -> Option<(u32, u32)>;
    /// 表示成立点の状態照合が積んだ未消費の窓寸 reconcile 要求を取り出す（取り出しで消える）。
    fn take_scale_report(&mut self, target: TargetId) -> Option<(u32, u32)>;
}

impl ScaleReportSource for EmoPresenter {
    fn refresh_scale_report(&mut self, world: &mut World, target: TargetId) -> Option<(u32, u32)> {
        self.refresh_scale(world, target)
    }

    fn take_scale_report(&mut self, target: TargetId) -> Option<(u32, u32)> {
        self.take_pending_resize(target)
    }
}

/// `Changed<DPI>` の窓と、その窓の scope/種別を引くための marker を一度に取る query 型。
///
/// `Changed<DPI>` フィルタは `DPI` component を持つ窓のみへ効く（本番は窓生成時に必ず付与され、
/// `WM_DPICHANGED` と生成時の `GetDpiForWindow` 実値補正の双方で変化が発火する）。marker は
/// `Option` で取り、ゴースト窓でない窓（どちらも `None`）を静穏に読み飛ばす。
pub(super) type DpiChangedQuery = Query<
    'static,
    'static,
    (
        Entity,
        Option<&'static CharWindowMarker>,
        Option<&'static BalloonWindowMarker>,
    ),
    Changed<DPI>,
>;

/// DPI フェーズの本体（報告源を抽象化した中核・[`run_dpi_phase`] が本番の presenter を渡す）。
///
/// 永続 `state` で `Changed<DPI>` を観測し（`anchor_changed_system` 先例と同じ流儀）、変化した
/// 各ゴースト窓について対応 target の [`ScaleReportSource::refresh_scale_report`] を呼ぶ。
///
/// # 位置の権威と寸の権威の分離（S2 是正・design D7・Req 4.1／4.2／4.5／4.6）
///
/// **窓寸**を合わせるのは報告が `Some(新物理寸)` のときだけ（[`reconcile_window_size`]）だが、
/// **位置**はその成否に条件付けない——`None` のキャラ窓は
/// [`reproject_char_window_at_current_size`] で**現寸のまま**射影 T を一度通す。位置の再射影が
/// [`resize_window_to`] の内部にしかない以上、`Some` ゲートの下流に置くと「再導出結果が得られ
/// ない経路で位置の再射影ごと欠落する」（診断レポート §1.2 の S2）。正常系は同寸・同 work area
/// ゆえべき等 skip で書込ゼロ＝ Req 4.5 はそのまま成立する。バルーン窓の `None` は位置据置き
/// （位置は従属量ゆえキャラ窓確定後の追従が随伴させる）。
///
/// # べき等・churn なし（Flow 2 キー決定 (b)）
///
/// 初回 run は `SystemState::new` の仕様で全窓へマッチするが、k 差分が無ければ報告は `None`
/// （presenter のゲート）であり、仮に報告があっても同寸なら反映口がべき等 skip する。ゆえに
/// 初回全マッチは無害に吸収される。`Changed<DPI>` が無いフレームは query が空＝実質 no-op。
///
/// # 失敗（Req4.4）
///
/// 再導出・再表示の失敗は presenter が `error!`＋`None` で前 k・前表示を維持する。本関数は
/// `None` で**窓寸を一切触らない**（前寸維持）——位置だけは上記のとおり現寸で射影を通す
/// （寸が古いまま位置だけ正す瞬間は同一フレームの drain 後段の reconcile が閉じる・D7）。
/// panic しない。
pub(super) fn dpi_phase_with<S: ScaleReportSource>(
    source: &mut S,
    state: &mut Option<SystemState<DpiChangedQuery>>,
    world: &mut World,
) {
    // 永続シーム: run を跨いで同一 SystemState を使い回し `last_run` を保つ（毎 run 新規生成は
    // `last_run` が 0 のままとなり全窓へ誤マッチし続ける＝毎フレーム再表示の churn）。
    let state = state.get_or_insert_with(|| SystemState::new(world));
    // 変化窓を collect して World の不変借用を即解放してから `&mut World` のループへ入る
    // （`anchor_changed_system` と同じ collect→release→&mut ループ）。
    let mut targets: Vec<Entity> = state.get(world).iter().map(|(entity, ..)| entity).collect();
    // 整合待ちの札を持つ窓は、`Changed<DPI>` が立たなくても対象へ入れる（設計 C5）——前フレーム
    // までに見送った窓は変化を既に消費済みであり、和集合にしないと札が永遠に外れない。
    let held: Vec<Entity> = world
        .query_filtered::<Entity, With<DpiSyncHold>>()
        .iter(world)
        .collect();
    for window in held {
        if !targets.contains(&window) {
            targets.push(window);
        }
    }

    // 第 1 巡（ゲート）: 全対象の札の付け外しを**処理より前に**済ませる。処理と混ぜて 1 巡に
    // すると、先に解除・処理されたキャラ窓の随伴書込が、まだ札の付いたバルーン窓へ届く
    // （＝待ち札の適用範囲の不変条件を自分で破る）。順序に依らせないための 2 巡である。
    let now = dpi_sync::current_frame(world);
    let mut proceed: Vec<(Entity, usize, GhostWindowKind)> = Vec::new();
    for window in targets {
        let char_scope = world.get::<CharWindowMarker>(window).map(|m| m.scope);
        let balloon_scope = world.get::<BalloonWindowMarker>(window).map(|m| m.scope);
        let (scope, kind) = match classify_ghost_window(char_scope, balloon_scope) {
            GhostWindowClass::Ghost(scope, kind) => (scope, kind),
            // ゴースト窓でない窓の DPI 変化は本フェーズの対象外（正常・静穏に読み飛ばす）。
            // ゲートにも掛けない——待ち札はゴースト窓の持ち物である（設計 C5）。
            GhostWindowClass::NotGhost => continue,
            GhostWindowClass::Ambiguous => {
                error!(
                    entity = ?window,
                    "dpi: char/balloon marker が同居する窓（spawn の排他付与に反する）→ 再スケールを skip"
                );
                continue;
            }
        };
        // 整合ゲート（設計 C5・要件 5.8）: 窓の拡大率と帰属モニタの表が揃うまで、当該窓の
        // 再導出も窓書込も行わない。**描画は止めない**（drain 相は素通り）。
        if !dpi_sync::apply_dpi_phase_gate(world, window, now) {
            continue;
        }
        proceed.push((window, scope, kind));
    }

    // 第 2 巡（処理）: 通過した窓だけを従来どおり再導出・反映する。
    for (window, scope, kind) in proceed {
        // target 採番（DD-3: shell=2*scope／balloon=2*scope+1）は u32 域。収まらない scope は
        // 如何なる target とも対応しない（plan_attachments の usize→u32 境界と同じ扱い）。
        let Ok(scope) = u32::try_from(scope) else {
            error!(
                entity = ?window,
                scope,
                "dpi: scope が u32 に収まらず target を採番できない → 再スケールを skip"
            );
            continue;
        };
        let target = match kind {
            GhostWindowKind::Char => shell_target(scope),
            GhostWindowKind::Balloon => balloon_target(scope),
        };
        // 再導出→（差分があれば）再表示。`None` は「**窓寸**を触らない」が正しい（前表示・前寸の
        // 維持・Req4.4）。なお `None` は k 不変とは同義でない——不可視・未表示・失敗に加え、**k は
        // 変わったが丸め後の物理寸が同じ**場合も `None` である（`refresh_scale` の doc が明記）。
        // ゆえに文字層 k 追従の判断材料にはこの戻り値を使わない（[`run_text_scale_phase`] を参照）。
        //
        // **位置**は寸の再導出結果に条件付けない（S2 是正・D7・下の `None` 腕）。
        match source.refresh_scale_report(world, target) {
            // 経路タグ: 本フェーズは `Changed<DPI>` エッジ駆動＝真に DPI 由来（Req 1.2・D13）。
            Some(new_size) => {
                reconcile_window_size(world, window, kind, new_size, PlacementRoute::DpiReproject);
            }
            // 再導出結果なし: 寸は触らないが、**位置は現寸のまま射影 T を一度通す**。
            // バルーン窓は位置据置きのまま（位置は従属量ゆえ、キャラ窓確定後の
            // [`follow_balloon`]＝`resize_window_to` 手順 6/7 が随伴させる）。
            None => match kind {
                GhostWindowKind::Char => {
                    // 経路タグ: 本フェーズは `Changed<DPI>` エッジ駆動＝真に DPI 由来（D13）。
                    reproject_char_window_at_current_size(
                        world,
                        window,
                        PlacementRoute::DpiReproject,
                    );
                }
                GhostWindowKind::Balloon => {}
            },
        }
    }
}

/// 窓寸の再導出結果が得られなかったキャラ窓を、**現在の寸のまま**射影 T へ通す（S2 是正・
/// design D7・Req 4.1／4.2／4.5／4.6・診断レポート §1.2）。
///
/// # なぜ必要か（位置の権威と寸の権威の分離）
///
/// 位置の再射影は [`resize_window_to`] の**内部**にしかないため、`refresh_scale_report` の
/// `Some` ゲートの下流に置くと「再導出結果が得られない経路では位置の再射影ごと欠落する」。
/// `None` を返す経路には**不可視**・**未表示**（いずれも Req 4.6 が名指しで扱う状況）と
/// 「k は変わったが丸め後の物理寸が同じ」（正常系で日常的に起こる）が含まれ、いずれも
/// **窓の DPI は変わっている＝接地すべき work area が変わっている**。ゆえに寸の成否に
/// 関わらず射影を一度通す。
///
/// # Req 4.5（現状維持）との関係——正常系は書込ゼロで抜ける
///
/// 現寸をそのまま渡すので手順 3b（下端中央の付け替え）は恒等、[`project_anchor`] が Y を
/// **変化後の** work area 下端から再導出する。同寸・同 work area なら導出値が現在値と一致し
/// [`resize_window_to`] のべき等 skip が書込ゼロで抜ける。書込が起きるのは**現位置が接地点
/// 規約に違反しているとき**だけであり、それは Req 4.1／4.2 が要求する保全そのものである
/// （design「dpi_phase 位置/寸分離 > Risks / Req 4.5 との整合」＝矛盾ではなく優先順位）。
///
/// # 縮退（log-first・要件 6.2/6.3 の区別を踏襲）
///
/// 窓寸が引けない場合は現状維持のまま打ち切る。水準は 2 分される（[`reconcile_reported_sizes`]
/// と同じ区別・混ぜると終了時ログの良性ノイズが本物の異常を埋める）:
///
/// - **entity 破棄済み**: 終了処理の**正常終了系**ゆえ `debug!`（[`DESPAWNED_SKIP_TAG`]）。
/// - **実在するが `WindowPos.size` 不在**（窓生成前）: 真の異常ゆえ `warn!`。
///
/// # `route` は呼び手が名乗る（task 5.2 で引数化・D13 の 1 語＝1 実在トリガ）
///
/// 本関数は「現寸のまま射影 T を一度通す」という**手続き**であって、トリガではない。
/// 呼び手は 2 つあり、実在するトリガが違う——拡大率の相は `Changed<DPI>` エッジ
/// （[`DpiReproject`](PlacementRoute::DpiReproject)）、作業領域変化を契機とする再スナップは
/// 作業領域源の差し替え（[`WorkAreaResnap`](PlacementRoute::WorkAreaResnap)）である。
/// ここで語を固定すると、ログ上で「拡大率が動いたから移した」のか「作業領域が動いたから
/// 移した」のかが切り分けられなくなる。
///
/// 戻り値は**窓へ書込が起きたか**（`false` はべき等 skip・縮退の双方を含み、失敗とは限らない
/// ＝[`reconcile_window_size`] と同じ流儀）。panic しない。
pub(super) fn reproject_char_window_at_current_size(
    world: &mut World,
    window: Entity,
    route: PlacementRoute,
) -> bool {
    let Some(current) = world.get::<WindowPos>(window).and_then(|wp| wp.size) else {
        // 経路語を載せる（task 5.2 で呼び手が 2 つになった）——載せないと、拡大率の相と
        // 作業領域再スナップのどちらが打ち切ったのかがログから判らない。
        if world.get_entity(window).is_err() {
            debug!(
                entity = ?window,
                ?route,
                "{DESPAWNED_SKIP_TAG} reproject: 窓 entity が破棄済み（despawn）→ 位置再射影を正常系として打ち切り"
            );
        } else {
            warn!(
                entity = ?window,
                ?route,
                "reproject: WindowPos.size 未確定（窓生成前）のため現寸を読めず、位置を再射影せず現状維持"
            );
        }
        return false;
    };
    resize_window_to(
        world,
        window,
        SizePx {
            w: current.width,
            h: current.height,
        },
        route,
    )
}

/// DPI 追従フェーズ（design「run_dpi_phase（frame.rs）」・D8・Req3.1/4.1/4.2/4.3）。
///
/// `Changed<DPI>` の窓を永続 [`SystemState`]（`anchor_changed_system` 先例）で観測し、当該窓に
/// 対応する target の `refresh_scale` を呼ぶ。`Some(新物理寸)` なら char 窓は
/// [`resize_window_to`]（アンカー保存）・balloon 窓は [`resize_window_keep_position`] で窓 client を
/// 新物理寸へ reconcile する——**同一フレーム・同一 UI スレッド呼出**で完結するため、フェーズ
/// 終了時点で照会値（`applied_scale`）・表示寸・窓 client が揃う（Req4.2）。
///
/// 進行中の talk 再生・SERIKO ループは presenter の**外**に状態を持つため、再表示はキャッシュ
/// ミス 1 回のコストで済み挙動を失わない（Req4.3）。本フェーズは target 状態を一切リセットしない。
///
/// # 窓寸 reconcile の第 2 経路
///
/// エッジ（`Changed<DPI>`）観測は「再表示のトリガ」に徹する。窓寸の整合そのものは**表示が成立
/// したという状態**に紐づき、[`emo2_frame_system`](super::emo2_frame_system) が drain の後段で
/// 直接呼ぶ [`reconcile_reported_sizes`] が同一フレーム内で拾う（初回表示の k₀ 補正＝Flow 3
/// 手順 5 はこちらの経路で landing する。呼び出し位置は `areka-P0-balloon-visibility` design
/// 決定 D5・task 4.3 で `run_drain_phase` 末尾から相順の所有者へ移した）。両者は
/// presenter の消費規約により二重にも取りこぼしにもならない。
///
/// # 文字層の k 追従は本フェーズが担わない（task 7.2・D11-4・Req8）
///
/// バルーンの**文字**も新 k へ追従させる必要があるが、その反映点は本フェーズではなく
/// [`run_text_scale_phase`] である——適用 k の更新点は 1 フレームに 2 つ（本フェーズの
/// `refresh_scale` と drain 相の `apply_show`）あり、本フェーズの戻り値だけを見ると後者を取り
/// こぼすためである（詳細は [`run_text_scale_phase`] の doc）。
pub fn run_dpi_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    // presenter（報告源）と dpi_state（永続観測器）は互いに素なフィールドゆえ同時に借りられる。
    let Emo2Wiring {
        presenter,
        dpi_state,
        ..
    } = wiring;
    dpi_phase_with(presenter, dpi_state, world);
}
