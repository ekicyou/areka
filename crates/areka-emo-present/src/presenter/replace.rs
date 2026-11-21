//! 装着の置き換え（シェル・バルーンの差し替え）。
//!
//! 置き換えの手順（`PresentCommand::ReplaceTarget` の適用・`apply_replace`）と、その中で使う
//! 「登録を消す口」[`EmoPresenter::detach_target`] を持つ。`detach_target` は表から target を外し、
//! 装着の子 2 つを消して、新しい装着へ引き継ぐ値（[`DetachedState`]）を返す。本番の呼び手は
//! 置き換えの手順だけで、登録を消したまま戻さない経路は作らない。

use super::{
    AtlasTable, BindSet, EmoPresenter, EmoWorld, Entity, NeverShown, PatternState, PresentError,
    PresentOutcome, ReplySender, ScaleRatio, TargetId, VisibilityOwnership, World,
};

/// 登録を消したときに、新しい装着へ引き継ぐ値。
///
/// 合成の入力（`EmoWorld`・アトラス・キャッシュ）と拡大政策は引き継がない——新しいシェル・
/// バルーンの資産と作者 DPI から作り直すものだからである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DetachedState {
    /// 装着先の窓（差し替えの前後で同じ窓を使う）。
    pub window: Entity,
    /// 可視性の持ち主（バルーン窓は `External` のまま引き継ぐ）。
    pub ownership: VisibilityOwnership,
    /// 実際に表示へ適用していた k（表示が一度も成立していなければ `None`）。
    pub applied: Option<ScaleRatio>,
    /// 最初の面を表示しないまま決まった理由。
    pub never_shown: Option<NeverShown>,
    /// 面の指令を一度でも受けたか。
    pub surface_requested: bool,
    /// 表示中サーフェスの素の大きさ（k 適用前）。
    pub native_size: Option<(u32, u32)>,
    /// まだ取り出されていない窓寸の要求（引き継がないと窓寸合わせが失われる）。
    pub pending_resize: Option<(u32, u32)>,
    /// 出番の世代（窓が同じなので引き継ぐ）。
    pub stage_generation: u64,
    /// 追い付いた世代（同上）。
    pub acked_generation: u64,
}

impl EmoPresenter {
    /// target の登録を消し、装着の子 2 つ（面と文字層スロット）を窓から消して、引き継ぐ値を返す。
    ///
    /// 未登録なら `error!` を残して `Err(TargetNotAttached)` を返し、表は変えない。一度も表示して
    /// いない target（装着の子が無い）も外せる。
    pub(super) fn detach_target(
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
            never_shown: old.never_shown,
            surface_requested: old.surface_requested,
            native_size: old.native_size,
            pending_resize: old.pending_resize,
            stage_generation: old.stage_generation,
            acked_generation: old.acked_generation,
        })
    }
}

impl EmoPresenter {
    /// `ReplaceTarget` の適用。
    ///
    /// 未登録なら `error!`（`detach_target` が残す）と `Err` を返し表は不変。そうでなければ登録を消し
    /// （古い子 2 つはここで消える）、引き継ぎの値で新しい装着を登録し（拡大政策だけは新しい
    /// `author_dpi` から）、`show` があれば `ShowSurface` と同じ経路で表示して返信する。古い子と新しい
    /// 子が同時に窓に在る瞬間は無い。
    ///
    /// 表示の失敗（新しいシェルにその面が無い等）は `apply_show` が `error!` を残し、その target は表示
    /// なしのまま残る。登録は済んでいるので返信は `Ok`。置き換えの失敗は未登録のときだけ。
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
        self.attach_target(target_id, carried.window, emo_world, atlas, author_dpi);
        let target = self.targets.get_mut(&target_id).expect("直上で登録済み");
        target.ownership = carried.ownership;
        // 前回の物理寸を引き継ぐので、続く表示は物理寸が変わったときだけ窓寸の要求を積む。
        target.applied = carried.applied;
        target.never_shown = carried.never_shown;
        target.surface_requested = carried.surface_requested;
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
            let outcome = self.apply_show(
                world,
                target_id,
                surface_id,
                binds,
                PatternState::default(),
                false,
            );
            self.record_show_outcome(target_id, &outcome);
        }
        Self::reply(reply, Ok(()));
    }
}
