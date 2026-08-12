//! 可視性の所有権と、可視化を伴わない表示確立の分離（`areka-P0-balloon-visibility` Requirement 1.1/
//! 1.3・6.1/6.2/6.6/6.8/6.9）。
//!
//! 本モジュールが持つのは**能力だけ**である——「いつ出すか・いつ消すか」の判断は UI 統合層の
//! バルーン可視性コントローラが所有し、presenter へは持ち込まない（design §Boundary Commitments
//! 「Out of Boundary: 表示の実行手段の判断化」）。

use super::{
    EmoPresenter, PresentError, PresentOutcome, TargetId, VisibilityOwnership, World, reply_channel,
};

impl EmoPresenter {
    /// target の可視性の所有者を設定する（Requirement 6.8）。
    ///
    /// 既定は [`VisibilityOwnership::CommandDriven`]（従来挙動）であり、本メソッドを呼ばない限り
    /// 挙動は 1 つも変わらない。バルーン target は装着直後・初回指令前に
    /// [`VisibilityOwnership::External`] を設定する（結線側が target の役割を知っている唯一の層）。
    ///
    /// 未装着 target は `error!` ＋ [`PresentError::TargetNotAttached`]（沈黙する失敗経路を作らない）。
    /// 設定そのものは可視状態を変えない——既に可視の target を `External` にしても消えないし、その逆も
    /// 起きない（所有権は「以後の `ShowSurface` が可視化してよいか」だけを決める）。
    pub fn set_visibility_ownership(
        &mut self,
        target: TargetId,
        ownership: VisibilityOwnership,
    ) -> Result<(), PresentError> {
        let Some(t) = self.targets.get_mut(&target) else {
            tracing::error!(
                ?target,
                ?ownership,
                "set_visibility_ownership: 未装着ターゲット"
            );
            return Err(PresentError::TargetNotAttached(target));
        };
        let previous = t.ownership;
        t.ownership = ownership;
        tracing::debug!(
            ?target,
            ?previous,
            ?ownership,
            visible = t.visible,
            "set_visibility_ownership: 可視性の所有者を設定"
        );
        Ok(())
    }

    /// [`VisibilityOwnership::External`] target の可視化（分離された可視化の唯一の入口・Requirement 6.6）。
    ///
    /// **最後に確立した表示内容**（`last_show` ＝ surface id・bind 集合・pattern）で
    /// `apply_show` の単一漏斗を再通過してから可視性を付与する。漏斗を通すのは、不可視だった期間に
    /// 窓 DPI が変わっていた場合に**現在の** DPI から k を導出し直すためである（Requirement 6.6:
    /// 「その期間ずっと表示されていた場合と同一」）——「消したときの絵をそのまま出す」実装は不可視期間中の
    /// 変化を取りこぼす。k が変わらなければ引き当てがヒットして再合成は起きない。
    ///
    /// 可視化は `VisualMount::set_visible` を通すため、枠の面と文字層スロットの双方が同時に可視・
    /// 判定復帰する（Requirement 1.7/1.8 の契約と対）。
    ///
    /// # 失敗（いずれも `error!` ＋ `Err`・可視性は変えない）
    ///
    /// - **未装着** target。
    /// - **表示が一度も確立していない**（`last_show` が `None`）: 可視化する内容が無い。中身の無い枠を
    ///   画面へ出さない（Requirement 3.4「内容が空のバルーンが可視である状態を作らない」と同じ側へ倒す）。
    /// - **漏斗の再通過が失敗した**: `apply_show` は表示成立点より手前で early return するため前状態が
    ///   保たれる。その上で可視性も付与しない（失敗した絵を出さない）。
    ///
    /// 全透明退化（`EmptyComposition` → Hide 縮退）は `apply_show` が許容する正常退化で、確立そのものが
    /// 取り消される。可視化する内容が無いため付与せず `Ok(())` を返す（`warn!` は `apply_show` 側・
    /// 同一事象を二重に鳴らさない）。
    pub fn show_target(&mut self, world: &mut World, target: TargetId) -> Result<(), PresentError> {
        let Some(t) = self.targets.get(&target) else {
            tracing::error!(?target, "show_target: 未装着ターゲット");
            return Err(PresentError::TargetNotAttached(target));
        };
        let Some((surface_id, binds, pattern)) = t.last_show.clone() else {
            tracing::error!(
                ?target,
                "show_target: 表示が一度も確立していない（可視化する内容が無い）"
            );
            return Err(PresentError::TargetNotAttached(target));
        };

        // 単一漏斗の再通過（k 再導出・キャッシュヒットなら再合成なし）。結果は `reply` で受ける——
        // `apply_show` は全経路で高々 1 回応答してから戻るため、同期呼び出しの直後に必ず値が在る。
        // 状態の後追い推測（`applied` や `current_surface_id` の照合）で成否を組み立てると、k 不変かつ
        // 失敗という組み合わせを成功と誤読するため、契約そのものを読む。
        let (tx, rx) = reply_channel::<PresentOutcome>();
        self.apply_show(world, target, surface_id, binds, pattern, Some(tx));
        match rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                // `apply_show` は失敗を error! 済みだが、それは「合成／デバイスが失敗した」ことしか
                // 語らない。可視化の要求が果たされなかったことはここでしか分からない。
                tracing::error!(
                    ?target,
                    surface_id,
                    error = %e,
                    "show_target: 漏斗の再通過が失敗（可視化しない・前状態を維持）"
                );
                return Err(e);
            }
            Err(e) => {
                tracing::error!(
                    ?target,
                    surface_id,
                    error = %e,
                    "show_target: 適用結果の応答が失われた（可視化しない）"
                );
                return Err(PresentError::TargetNotAttached(target));
            }
        }

        let Some(t) = self.targets.get_mut(&target) else {
            tracing::error!(?target, "show_target: 再通過後に target が消えた");
            return Err(PresentError::TargetNotAttached(target));
        };
        if t.current_surface_id.is_none() {
            // 全透明退化（Hide 縮退）。出す内容が無いため可視化しない（warn! は apply_show 側）。
            tracing::debug!(
                ?target,
                surface_id,
                "show_target: 全透明退化ゆえ可視化しない（Hide 縮退）"
            );
            return Ok(());
        }
        let Some(mount) = t.mount.as_ref() else {
            tracing::error!(
                ?target,
                surface_id,
                "show_target: 確立済みなのに装着が無い（可視化できない）"
            );
            return Err(PresentError::TargetNotAttached(target));
        };
        // 枠の面と文字層スロットの双方を同時に可視・判定復帰させる。
        mount.set_visible(world, true);
        t.visible = true;
        tracing::debug!(
            ?target,
            surface_id,
            ownership = ?t.ownership,
            "show_target: 可視化した"
        );
        Ok(())
    }
}
