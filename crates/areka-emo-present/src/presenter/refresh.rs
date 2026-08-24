//! 窓 DPI 変化に伴う再スケール（`EmoPresenter::refresh_scale`）と、表示成立点が積んだ窓寸 reconcile
//! 要求の取り出し（`EmoPresenter::take_pending_resize`）。

use wintf::ecs::window::transition_diag;

use super::transition_record::{
    SURFACE_REASON_INVISIBLE, SURFACE_REASON_K_UNCHANGED, SurfaceRecord, SurfaceStage, stamp_of,
    surface_line,
};
use super::{DPI, EmoPresenter, TargetId, World, derive_scale};

impl EmoPresenter {
    /// 窓 DPI 変化に伴う再スケール（要件 4.1-4.4・design Flow 2）。
    ///
    /// 窓の現 `DPI` から k を再導出し、**前回適用 k と異なり・可視であり・再表示入力を保持している**
    /// ときだけ内部で `ShowSurface` を再実行する。表示物理寸が変われば `Some(新物理寸)` を返し、呼び手
    /// （`run_dpi_phase`）が**同一フレーム・同一 UI スレッド呼出**で窓寸 reconcile（char=`resize_window_to`
    /// ／balloon=`resize_window_keep_position`）を行う——完了後に照会値・表示寸・窓 client が揃う（要件 4.2）。
    ///
    /// # ゲート（いずれか不成立なら `None`・副作用なし）
    ///
    /// - **未登録** target。
    /// - **k 不変**: 再導出値が `applied` と等しい。`Changed<DPI>` の初回 run が全窓にマッチする仕様
    ///   （`SystemState::new`）はここで吸収される（`anchor_changed_system` と同じ流儀）。
    /// - **不可視**: `Hide`／全透明退化で消えている target を再表示で**蘇らせない**。DPI 変化は
    ///   「見えているものを描き直す」事象であって、表示を復活させる事象ではない。
    /// - **再表示入力なし**（`last_show` が `None`）: 一度も表示が成立していない。
    ///
    /// # k 導出の権威は `apply_show` 側にある
    ///
    /// ここでの [`derive_scale`] 呼出は**ゲート判定の述語**であり、実際に適用される k は `apply_show` が
    /// 自前で導出したものである（漏斗を二重化しない・design Flow 1「k 導出は show 適用ごと」）。両者は
    /// 同一の純関数へ同一入力を与える（同一 UI スレッド内・間に World 変更なし）ため必ず一致し、その一致は
    /// 再表示後の `applied` 照合で**実際に検査される**（食い違えば失敗として扱われ黙って通らない）。
    ///
    /// # [`Self::take_pending_resize`] との関係（二重 resize も取りこぼしもしない）
    ///
    /// タスク 4.2 の結線は `run_dpi_phase`（本メソッド）と drain フェーズ（`take_pending_resize`）の
    /// **両方**を毎フレーム呼ぶため、両者の責任範囲を重ねない:
    ///
    /// - **再表示して成立した**場合: その表示成立が積んだ要求を本メソッドが**取り出して**返す。ゆえに
    ///   同一フレームの drain フェーズが `take_pending_resize` を呼んでも同じ要求は二度出ない。
    /// - **ゲート不成立で再表示しなかった**場合: `pending_resize` に**一切触れない**。未消費の要求
    ///   （例: 初回表示が積んだ k₀ 補正）は drain フェーズがそのまま拾う。
    /// - **再表示が失敗した**場合: 同じく触れずに `None` を返す（前 k・前表示・未消費要求すべて維持）。
    ///
    /// # 失敗（要件 4.4）
    ///
    /// 再表示が表示成立に至らなければ `error!` を出して `None` を返し、**直前の k による表示を維持**する
    /// （`apply_show` が表示成立点より手前で early return するため前値は構造的に保たれる）。`apply_show`
    /// 自身も失敗を error! するが、それは「合成／デバイスが失敗した」ことしか語らない——どの k からどの k
    /// への再導出が落ちたのか・前表示を維持したのかは本メソッドでしか分からないため、専用のログを出す
    /// （無言の失敗経路を作らない）。全透明退化（`EmptyComposition` → Hide 縮退）は設計上許容された正常
    /// 退化ゆえ `apply_show` の `warn!` に委ね、ここでは `debug!` に留める（同一事象を二重に鳴らさない）。
    ///
    /// 進行中の talk 再生・SERIKO ループは presenter の**外**に状態を持つため、再表示はキャッシュミス 1 回の
    /// コストで済み挙動を失わない（要件 4.3）。本メソッドは target 状態を一切リセットしない。
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
        // 要件 1.4 の縮退（error! ＋ k=1.0）が「正常系のふり」で通る。`None` のまま渡して縮退させる。
        let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
        let scale = derive_scale(policy, window_dpi);

        if previous == Some(scale) {
            tracing::trace!(?target_id, k_ratio = ?scale, "refresh_scale: k 不変（再表示しない）");
            // 見送りを理由つきで記録する（Requirement 4.6・design C3）。判定側はこの行を持つ窓を
            // 「再導出結果が得られない窓＝現状維持が正しい窓」として合否から除外する——記録が
            // 無いと「書込が来ていない」と「そもそも対象外だった」の区別が付かず、遷移の
            // 有界性判定が対象外の窓で不合格になる。判定そのものは 1 つも変えない。
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

        // 表示更新は既存の単一漏斗をそのまま通す（`reply` なし＝内部再実行・design Flow 2）。成立点の記録・
        // 失敗時の early return・D10 ログ・状態照合報告はすべて `apply_show` 側の不変条件がそのまま効く。
        self.apply_show(world, target_id, surface_id, binds, pattern, None);

        let t = self.targets.get(&target_id)?;
        if t.applied != Some(scale) {
            // 表示成立に至らなかった。前 k・前表示は `apply_show` の early return が保っている（要件 4.4）。
            if t.visible {
                tracing::error!(
                    ?target_id,
                    k_ratio_from = ?previous,
                    k_ratio_to = ?scale,
                    window_dpi = ?window_dpi,
                    "refresh_scale: 再表示が成立せず（直前の k による表示を維持）"
                );
            } else {
                // 全透明退化（`EmptyComposition` → Hide 縮退）。`apply_show` が warn! 済みゆえ重ねない。
                tracing::debug!(
                    ?target_id,
                    "refresh_scale: 再表示が全透明退化（Hide 縮退・warn は apply_show 側）"
                );
            }
            return None;
        }

        // 成立: 状態照合が積んだ要求をここで消費して返す（drain フェーズと二重に出さない）。物理寸が
        // 変わらなければ `None`＝窓寸 reconcile 不要（k だけ変わって丸め後の寸が同じ場合が実在する）。
        self.take_pending_resize(target_id)
    }

    /// 表示成立点の状態照合が積んだ**窓寸 reconcile 要求**を取り出す（取り出しで消える・drain 契約）。
    ///
    /// `Some(新物理寸)` は「直近の表示成立で物理寸が前回適用寸から変わった（初回表示を含む）」ことを
    /// 表す。呼び手（emo2_boot の frame drain フェーズ）は**同一フレーム内**で char 窓なら
    /// `resize_window_to`（アンカー保存）・balloon 窓なら `resize_window_keep_position` を呼び、窓
    /// client を新物理寸へ合わせる（design Flow 2／Flow 3 手順 5・議題 #2 裁定）。未登録 target・
    /// 要求なしは `None`。
    ///
    /// # なぜ `reply`（[`PresentOutcome`]）ではなくここに置くのか
    ///
    /// 本番の drain 経路（`run_drain_phase`）は指令へ `reply` を**同梱しない**（撃ちっぱなし）ため、
    /// [`PresentOutcome`] を太らせても報告は呼び手へ届かない。加えて報告は「表示が成立したという
    /// **状態**」であり、エッジ（`Changed<DPI>`）の消費順序に依存してはならない（議題 #2 裁定）。
    /// ゆえに target ごとの**取り出し可能な状態**として置く。
    ///
    /// # 取りこぼさない（未消費なら保持）
    ///
    /// 要求は取り出されるまで消えない。呼び手が或るフレームで取り出さなくても次に取り出した者が最新の
    /// 物理寸を受け取るため、報告が黙って失われる経路が無い。逆に取り出した後は同寸表示を何度繰り返しても
    /// `None` のままで、窓へ無用な書込（churn）を誘発しない。
    pub fn take_pending_resize(&mut self, target: TargetId) -> Option<(u32, u32)> {
        self.targets.get_mut(&target)?.pending_resize.take()
    }
}
