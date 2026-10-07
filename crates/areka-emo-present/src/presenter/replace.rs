//! 装着の置き換え（シェル・バルーンの差し替え・`areka-P0-shell-balloon-switch` 要件 4.6）。
//!
//! 置き換えの手順（`PresentCommand::ReplaceTarget` の適用・`apply_replace`）と、その中で使う
//! 「登録を消す口」[`EmoPresenter::detach_target`] を持つ。`detach_target` は表から target を外し、
//! 装着の子 2 つを消して、新しい装着へ引き継ぐ値（[`DetachedState`]）を返す。本番の呼び手は
//! 置き換えの手順だけで、登録を消したまま戻さない経路は作らない。

use super::{
    AtlasTable, BindSet, EmoPresenter, EmoWorld, Entity, PatternState, PresentError,
    PresentOutcome, ReplySender, ScaleRatio, TargetId, VisibilityOwnership, World,
};

/// 登録を消したときに、新しい装着へ引き継ぐ値。
///
/// 合成の入力（`EmoWorld`・アトラス・キャッシュ）と拡大政策は引き継がない——新しいシェル・
/// バルーンの資産と作者 DPI から作り直すものだからである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetachedState {
    /// 装着先の窓（差し替えの前後で同じ窓を使う）。
    pub window: Entity,
    /// 可視性の持ち主（バルーン窓は `External` のまま引き継ぐ）。
    pub ownership: VisibilityOwnership,
    /// 実際に表示へ適用していた k（表示が一度も成立していなければ `None`）。
    pub applied: Option<ScaleRatio>,
    /// 表示中サーフェスの素の大きさ（k 適用前）。
    pub native_size: Option<(u32, u32)>,
    /// まだ取り出されていない窓寸の要求（引き継がないと窓寸合わせが失われる）。
    pub pending_resize: Option<(u32, u32)>,
    /// 出番の世代（窓が同じなので引き継ぐ・`animated-image-playback` 要件 2.3）。
    pub stage_generation: u64,
    /// 追い付いた世代（同上）。
    pub acked_generation: u64,
}

impl EmoPresenter {
    /// target の登録を消し、装着の子 2 つ（面と文字層スロット）を窓から消して、引き継ぐ値を返す。
    ///
    /// 未登録なら `error!` を残して `Err(TargetNotAttached)` を返し、表は変えない。一度も表示して
    /// いない target（装着の子が無い）も外せる。
    pub fn detach_target(
        &mut self,
        world: &mut World,
        target: TargetId,
    ) -> Result<DetachedState, PresentError> {
        let Some(old) = self.targets.remove(&target) else {
            tracing::error!(?target, "detach_target: 未装着ターゲット");
            return Err(PresentError::TargetNotAttached(target));
        };
        let had_mount = old.mount.is_some();
        if let Some(mount) = old.mount {
            mount.despawn(world);
        }
        tracing::debug!(?target, had_mount, "detach_target: 登録と装着の子を消した");
        Ok(DetachedState {
            window: old.window,
            ownership: old.ownership,
            applied: old.applied,
            native_size: old.native_size,
            pending_resize: old.pending_resize,
            stage_generation: old.stage_generation,
            acked_generation: old.acked_generation,
        })
    }
}

impl EmoPresenter {
    /// `ReplaceTarget` の適用（design「Present」の `apply_replace` の順）。
    ///
    /// 1 回の呼び出しの中で ⑴ 未登録なら `error!`（`detach_target` が残す）と `Err`・表は不変
    /// → ⑵ 登録を消す（古い子 2 つはここで消える）→ ⑶ 引き継ぎの値で新しい装着を登録する（拡大政策
    /// だけは新しい `author_dpi` から）→ ⑷ `show` があれば今日の `ShowSurface` と同じ経路で表示する
    /// → ⑸ 返信、の順に行う。古い子と新しい子が同時に窓に在る瞬間は無い（要件 4.2）。
    ///
    /// ⑷ の失敗（新しいシェルにその面が無い等）は `apply_show` が今日と同じく `error!` を残し、その
    /// target は表示なしのまま残る。登録は済んでいるので返信は `Ok`——置き換えの失敗は窓が無い
    /// （未登録の）ときだけである（要件 5.6・design の Error Handling）。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_replace(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        emo_world: EmoWorld,
        atlas: AtlasTable,
        author_dpi: u16,
        show: Option<(u32, BindSet)>,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        let carried = match self.detach_target(world, target_id) {
            Ok(carried) => carried,
            Err(e) => {
                Self::reply(reply, Err(e));
                return;
            }
        };
        if let Err(e) = self.attach_target(
            world,
            target_id,
            carried.window,
            emo_world,
            atlas,
            author_dpi,
        ) {
            tracing::error!(?target_id, error = %e, "apply(ReplaceTarget): 登録し直せなかった");
            Self::reply(reply, Err(e));
            return;
        }
        let target = self.targets.get_mut(&target_id).expect("直上で登録済み");
        target.ownership = carried.ownership;
        // 前回の物理寸を引き継ぐので、⑷ の表示は物理寸が変わったときだけ窓寸の要求を積む。
        target.applied = carried.applied;
        target.native_size = carried.native_size;
        target.pending_resize = carried.pending_resize;
        target.stage_generation = carried.stage_generation;
        target.acked_generation = carried.acked_generation;
        tracing::info!(
            ?target_id,
            author_dpi,
            ownership = ?carried.ownership,
            show = ?show.as_ref().map(|(surface_id, _)| *surface_id),
            "apply(ReplaceTarget): 古い装着を片付けて登録し直した"
        );
        if let Some((surface_id, binds)) = show {
            // 返信は置き換え自身が返す。表示の失敗は `apply_show` の中で `error!` 済み。
            self.apply_show(
                world,
                target_id,
                surface_id,
                binds,
                PatternState::default(),
                None,
            );
        }
        Self::reply(reply, Ok(()));
    }
}
