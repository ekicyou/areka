//! 窓 DPI 変化に伴う再スケール（`EmoPresenter::refresh_scale`）と、表示成立点が積んだ窓寸 reconcile
//! 要求の取り出し（`EmoPresenter::take_pending_resize`）。

use bevy_ecs::change_detection::Tick;
use bevy_ecs::resource::Resource;
use wintf::ecs::GraphicsCore;
use wintf::ecs::window::transition_diag;

use super::transition_record::{
    SURFACE_REASON_INVISIBLE, SURFACE_REASON_K_UNCHANGED, SurfaceRecord, SurfaceStage, stamp_of,
    surface_line,
};
use super::{DPI, EmoPresenter, PresentCommand, TargetId, World, derive_scale};

/// presenter が最後に見た `GraphicsCore` の変化の印（[`EmoPresenter::refresh_after_device_change`] の控え）。
///
/// World ごとに 1 つなので、1 つの World に presenter は 1 つの前提（本番は `crates/areka-runtime/src/lib.rs` の 1 つだけ）。
#[derive(Resource)]
struct SeenGraphicsCore(Tick);

impl EmoPresenter {
    /// 窓 DPI 変化に伴う再スケール。
    ///
    /// 窓の現 `DPI` から k を再導出し、前回適用 k と異なり・可視であり・再表示入力を保持しているときだけ
    /// 内部で `ShowSurface` を再実行する。表示物理寸が変われば `Some(新物理寸)` を返し、呼び手
    /// （`run_dpi_phase`）が同一フレーム・同一 UI スレッド呼出で窓寸 reconcile（char=`resize_window_to`
    /// ／balloon=`resize_window_keep_position`）を行う。
    ///
    /// # ゲート（いずれか不成立なら `None`・副作用なし）
    ///
    /// - 未登録 target。
    /// - k 不変: 再導出値が `applied` と等しい。`Changed<DPI>` の初回 run が全窓にマッチする仕様
    ///   （`SystemState::new`）はここで吸収される。
    /// - 不可視: `Hide`／全透明退化で消えている target を再表示で蘇らせない。DPI 変化は
    ///   「見えているものを描き直す」事象であって、表示を復活させる事象ではない。
    /// - 再表示入力なし（`last_show` が `None`）: 一度も表示が成立していない。
    ///
    /// # k 導出の権威は `apply_show` 側にある
    ///
    /// ここでの [`derive_scale`] 呼出はゲート判定の述語で、実際に適用される k は `apply_show` が自前で
    /// 導出したもの。両者は同一の純関数へ同一入力を与える（同一 UI スレッド内・間に World 変更なし）ので
    /// 必ず一致する。再表示の成否は `apply_show` の戻り値で読み、状態（`applied`）の後追いでは推し量らない。
    ///
    /// # [`Self::take_pending_resize`] との関係（二重 resize も取りこぼしもしない）
    ///
    /// 結線は `run_dpi_phase`（本メソッド）と drain フェーズ（`take_pending_resize`）の両方を毎フレーム
    /// 呼ぶので、責任範囲を重ねない:
    ///
    /// - 再表示して成立した場合: その表示成立が積んだ要求を本メソッドが取り出して返す。同一フレームの
    ///   drain フェーズが `take_pending_resize` を呼んでも同じ要求は二度出ない。
    /// - ゲート不成立で再表示しなかった場合: `pending_resize` に一切触れない。未消費の要求（例: 初回表示が
    ///   積んだ k₀ 補正）は drain フェーズがそのまま拾う。
    /// - 再表示が失敗した場合: 同じく触れずに `None` を返す（前 k・前表示・未消費要求すべて維持）。
    ///
    /// # 失敗
    ///
    /// 再表示が表示成立に至らなければ `error!` を出して `None` を返し、直前の k による表示を維持する
    /// （`apply_show` が表示成立点より手前で early return するため前値は保たれる）。`apply_show` 自身も
    /// 失敗を `error!` するが、どの k からどの k への再導出が落ちたのか・前表示を維持したのかは本メソッドでしか
    /// 分からないので専用のログを出す。全透明退化（`EmptyComposition` → Hide 縮退・外形の上限越えの
    /// `ExtentTooLarge` も同じ）は許容された正常退化なので `apply_show` の `warn!` に委ね、ここでは
    /// `debug!` に留める（同一事象を二重に鳴らさない）。
    ///
    /// 進行中の talk 再生・SERIKO ループは presenter の外に状態を持つので、再表示はキャッシュミス 1 回の
    /// コストで済み挙動を失わない。本メソッドは target 状態を一切リセットしない。
    pub fn refresh_scale(&mut self, world: &mut World, target_id: TargetId) -> Option<(u32, u32)> {
        // ゲート判定に要る値を先に取り出して借用を閉じる（以降 `apply_show` が `&mut self` を要する）。
        let (window, policy, previous, visible, last_show) = {
            let t = self.targets.get(&target_id)?;
            (
                t.window,
                t.policy,
                t.applied,
                t.visible,
                t.last_show.clone(),
            )
        };

        // 窓 DPI は `apply_show` と同一経路で読む。**component 不在を 96 で捏造しない**——捏造すると
        // k=1.0 への縮退（error! ＋ k=1.0）が「正常系のふり」で通る。`None` のまま渡して縮退させる。
        let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
        let scale = derive_scale(policy, window_dpi);

        if previous == Some(scale) {
            tracing::trace!(?target_id, k_ratio = ?scale, "refresh_scale: k 不変（再表示しない）");
            // 見送りを理由つきで記録する。判定側はこの行を持つ窓を「再導出結果が得られない窓＝現状維持が
            // 正しい窓」として合否から除外する。記録が無いと「書込が来ていない」と「対象外だった」の区別が
            // 付かず、遷移の有界性判定が対象外の窓で不合格になる。
            if transition_diag::is_enabled() {
                transition_diag::emit_line(&surface_line(&SurfaceRecord {
                    stamp: stamp_of(world),
                    stage: SurfaceStage::Skipped,
                    target_id,
                    size: None,
                    resized: None,
                    reason: Some(SURFACE_REASON_K_UNCHANGED),
                }));
            }
            return None;
        }
        if !visible {
            tracing::debug!(
                ?target_id,
                "refresh_scale: 不可視ゆえ再表示しない（Hide/全透明退化を蘇らせない）"
            );
            if transition_diag::is_enabled() {
                transition_diag::emit_line(&surface_line(&SurfaceRecord {
                    stamp: stamp_of(world),
                    stage: SurfaceStage::Skipped,
                    target_id,
                    size: None,
                    resized: None,
                    reason: Some(SURFACE_REASON_INVISIBLE),
                }));
            }
            return None;
        }
        let Some((surface_id, binds, pattern)) = last_show else {
            tracing::debug!(
                ?target_id,
                "refresh_scale: 再表示入力なし（表示が一度も成立していない）"
            );
            return None;
        };

        // 表示更新は `apply_show` の単一漏斗を通し、成否は戻り値で読む。
        if let Err(e) = self.apply_show(world, target_id, surface_id, binds, pattern, false) {
            tracing::error!(
                ?target_id,
                k_ratio_from = ?previous,
                k_ratio_to = ?scale,
                window_dpi = ?window_dpi,
                error = %e,
                "refresh_scale: 再表示が成立せず（直前の k による表示を維持）"
            );
            return None;
        }
        if self.targets.get(&target_id)?.current_surface_id.is_none() {
            // 全透明退化。`apply_show` が warn! 済みなので重ねない。
            tracing::debug!(
                ?target_id,
                "refresh_scale: 再表示が全透明退化（Hide 縮退・warn は apply_show 側）"
            );
            return None;
        }

        // 成立: 状態照合が積んだ要求をここで消費して返す（drain フェーズと二重に出さない）。物理寸が
        // 変わらなければ `None`（k だけ変わって丸め後の寸が同じ場合が実在する）。
        self.take_pending_resize(target_id)
    }

    /// 装置の作り直し・失いへの追従。毎フレーム呼ぶ。
    ///
    /// `Res<GraphicsCore>` の変化の印は作り直しと本当の失いのときだけ立つ。印を見たら全部の
    /// target の控え（古い装置の頁の絵と閉じた命令の列）を捨て、装置が有効なら見えている target の最後の
    /// 表示を出し直す（`refresh_scale` の出し直しの道を倍率の門なしで通す）。無効な間は捨てるだけで、
    /// 作り直しの回にもう一度印が立って出し直す。初めて呼んだ回は印を控えるだけ。
    pub fn refresh_after_device_change(&mut self, world: &mut World) {
        let Some(changed) = world
            .get_resource_change_ticks::<GraphicsCore>()
            .map(|t| t.changed)
        else {
            return;
        };
        let now = world.read_change_tick();
        let seen = world.get_resource::<SeenGraphicsCore>().map(|s| s.0);
        if seen.is_some_and(|seen| !changed.is_newer_than(seen, now)) {
            return;
        }
        // 時刻を進めて控える: 同じ時刻のうちに後から立った印も、次の回に新しいと分かる。
        let mark = world.increment_change_tick();
        world.insert_resource(SeenGraphicsCore(mark));
        if seen.is_none() {
            return;
        }

        let valid = world.resource::<GraphicsCore>().is_valid();
        tracing::debug!(
            valid,
            "refresh_after_device_change: 装置の変化を見た（控えを捨てる）"
        );
        let ids: Vec<TargetId> = self.targets.keys().copied().collect();
        for target in ids {
            if let Some(t) = self.targets.get_mut(&target) {
                t.discard_hit_gpu();
                *t.gpu.get_mut() = None;
            }
            self.apply(
                world,
                PresentCommand::InvalidateCache {
                    target,
                    reply: None,
                },
            );
            if !valid {
                continue;
            }
            let Some((surface_id, binds, pattern)) = self
                .targets
                .get(&target)
                .filter(|t| t.visible)
                .and_then(|t| t.last_show.clone())
            else {
                continue;
            };
            // 失敗は `apply_show` が記録する（前の表示は保たれる）。
            let _ = self.apply_show(world, target, surface_id, binds, pattern, false);
        }
    }

    /// 表示成立点の状態照合が積んだ**窓寸 reconcile 要求**を取り出す（取り出しで消える・drain 契約）。
    ///
    /// `Some(新物理寸)` は「直近の表示成立で物理寸が前回適用寸から変わった（初回表示を含む）」ことを
    /// 表す。呼び手（`areka` の `runtime::frame` の drain フェーズ）は同一フレーム内で char 窓なら
    /// `resize_window_to`（アンカー保存）・balloon 窓なら `resize_window_keep_position` を呼び、窓 client を
    /// 新物理寸へ合わせる。未登録 target・要求なしは `None`。
    ///
    /// # なぜ `reply`（`PresentOutcome`）ではなくここに置くのか
    ///
    /// 本番の drain 経路（`run_drain_phase`）は指令へ `reply` を同梱しない（撃ちっぱなし）ので、
    /// `PresentOutcome` を太らせても報告は呼び手へ届かない。加えて報告は「表示が成立したという状態」で
    /// あり、エッジ（`Changed<DPI>`）の消費順序に依存してはならない。ゆえに target ごとの取り出し可能な
    /// 状態として置く。
    ///
    /// # 取りこぼさない（未消費なら保持）
    ///
    /// 要求は取り出されるまで消えない。呼び手が或るフレームで取り出さなくても次に取り出した者が最新の
    /// 物理寸を受け取る。逆に取り出した後は同寸表示を何度繰り返しても `None` のままで、窓へ無用な書込
    /// （churn）を誘発しない。
    pub fn take_pending_resize(&mut self, target: TargetId) -> Option<(u32, u32)> {
        self.targets.get_mut(&target)?.pending_resize.take()
    }
}
