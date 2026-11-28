//! スコープごとの文字の行き先（純粋層・`state.rs` の子モジュール）。
//!
//! 行き先の答えは [`areka_emo_compose::TextRoute`] が決める。ここは答えが替わった瞬間の
//! `\f` の指定の持ち運びと記録だけを足す。時計も窓も `World` も見ない。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_compose::{BoxName, BoxSelect, TextRoute};
use areka_sakura::contract::ActorKey;

pub use areka_emo_compose::SurfaceKeyOutcome;

use super::TextLayerState;
use crate::place::TextPlace;

/// シェルの切替の戻り先。seriko の戻り先と同じ材料。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SurfaceFallback {
    /// 新しいシェルの面の番号の集合（「在る」の判定）。
    pub surface_ids: BTreeSet<u32>,
    /// スコープごとの戻り先の候補の並び（descript の既定の面 → 組み込みの既定の面）。
    pub candidates: BTreeMap<ActorKey, Vec<u32>>,
}

/// スコープ 1 つの行き先の状態。記録の無いスコープは「非表示・普通のバルーン」と同じ。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ScopeRoute {
    route: TextRoute,
    /// スコープの `\f` の指定を今持っている場所（スコープに従う場所だけ・始めは普通のバルーン）。
    /// 行き先が替わるときの写しと持ち主の移動は装飾の持ち運び（state_decoration.rs の
    /// `carry_script_decor`）が読み書きする。
    pub(super) shared: TextPlace,
}

impl Default for ScopeRoute {
    fn default() -> Self {
        ScopeRoute {
            route: TextRoute::default(),
            shared: TextPlace::Balloon,
        }
    }
}

impl TextLayerState {
    /// 箱の表（サーフェス番号 → element番号の昇順の箱の名前）を差し替える（シェルの切替）。
    ///
    /// 各スコープの今のサーフェス番号は保ち、行き先だけを新しい表での既定へ引き直す。
    /// 箱の場所の文字は捨てる。スコープの `\f` の指定を箱が持っていたら、捨てる前に
    /// 普通のバルーンの場所へ写して持ち主を普通のバルーンにする。
    pub fn set_box_index(&mut self, index: BTreeMap<u32, Vec<BoxName>>) {
        self.fold_shared_into_balloon();
        self.box_index = index;
        self.actors.retain(|key, _| key.place == TextPlace::Balloon);
        self.clears.retain(|key, _| key.place == TextPlace::Balloon);
        self.reset_routes();
        tracing::debug!(
            surfaces = self.box_index.len(),
            "箱の表を差し替え（番号は保ち、行き先を既定へ引き直す）"
        );
    }

    /// `\s` の解決結果でスコープのサーフェス番号と行き先を決める。
    pub fn route_surface(&mut self, actor: &ActorKey, outcome: SurfaceKeyOutcome) {
        if outcome == SurfaceKeyOutcome::Unresolved {
            tracing::debug!(actor = %actor, "解決できない \\s——行き先もサーフェス番号も変えない");
            return;
        }
        let route = &mut self.routes.entry(actor.clone()).or_default().route;
        let surface = match outcome {
            SurfaceKeyOutcome::Show(id) => Some(id),
            _ => None,
        };
        if route.follow_surface(&self.box_index, outcome) {
            let dest = route.dest().clone();
            self.carry_script_decor(actor, &dest);
        }
        tracing::debug!(actor = %actor, ?surface, dest = ?self.destination(actor), "\\s による行き先の決定");
    }

    /// シェルの切替: 今の面が新しいシェルに無いスコープだけを、候補の最初に在る面（無ければ非表示）へ
    /// 替える（行き先の引き直しは [`Self::route_surface`] に任せる）。非表示のスコープは替えない。
    /// 替えたスコープがあれば `true`。
    pub fn fall_back_surfaces(&mut self, fallback: &SurfaceFallback) -> bool {
        let moves: Vec<(ActorKey, u32, SurfaceKeyOutcome)> = self
            .routes
            .iter()
            .filter_map(|(actor, scope)| {
                let candidates = fallback
                    .candidates
                    .get(actor)
                    .map_or(&[][..], Vec::as_slice);
                let outcome = scope
                    .route
                    .fallback_outcome(&fallback.surface_ids, candidates)?;
                Some((actor.clone(), scope.route.surface()?, outcome))
            })
            .collect();
        for (actor, missing, outcome) in &moves {
            let to = match outcome {
                SurfaceKeyOutcome::Show(id) => Some(*id),
                _ => None,
            };
            tracing::debug!(actor = %actor, missing, ?to, "シェルの切替で今の面が無い——戻り先へ引き直す");
            self.route_surface(actor, *outcome);
        }
        !moves.is_empty()
    }

    /// 名前の形の `\b[名前]` で行き先を切り替える。
    ///
    /// 今のサーフェスの箱の列にその名前が無ければ、名前・サーフェス番号・スコープを含む
    /// `warn!` を 1 行残して行き先を変えない。空回しの写し
    /// （[`TextLayerState::rehearsal_copy`]）では `warn!` を残さない（本番の適用が残す）。
    ///
    /// 箱が見つからず空回しの写しでもない（＝`warn!` を残した）ときだけ真を返す（strict の知らせ）。
    pub fn route_select(&mut self, actor: &ActorKey, name: &str) -> bool {
        let select = match self.routes.get_mut(actor) {
            Some(scope) => scope.route.select_box(&self.box_index, name),
            None => BoxSelect::Missing,
        };
        match select {
            BoxSelect::Changed | BoxSelect::Same => {
                tracing::debug!(actor = %actor, name, "\\b[名前] で行き先を箱へ切り替え");
                if select == BoxSelect::Changed {
                    let dest = self.destination(actor);
                    self.carry_script_decor(actor, &dest);
                }
                false
            }
            BoxSelect::Missing if self.rehearsal => false,
            BoxSelect::Missing => {
                let surface = self
                    .current_surface(actor)
                    .map_or_else(|| "非表示".to_owned(), |id| id.to_string());
                tracing::warn!(
                    name,
                    surface = %surface,
                    actor = %actor,
                    "\\b[名前] の箱が今のサーフェスに無い——行き先を切り替えない"
                );
                true
            }
        }
    }

    /// スコープの今の文字の行き先（記録の無いスコープは普通のバルーン）。
    pub fn destination(&self, actor: &ActorKey) -> TextPlace {
        self.routes
            .get(actor)
            .map_or(TextPlace::Balloon, |scope| scope.route.dest().clone())
    }

    /// スコープの今のサーフェス番号（非表示・まだ `\s` が届いていなければ `None`）。
    pub fn current_surface(&self, actor: &ActorKey) -> Option<u32> {
        self.routes
            .get(actor)
            .and_then(|scope| scope.route.surface())
    }

    /// 台詞の頭・シェルの切替: 全スコープの行き先を既定へ戻す（サーフェス番号は変えない）。
    ///
    /// 持ち主を普通のバルーンへ戻してから、普通のバルーンを出発点に既定の行き先へ移す
    /// （既定がスコープに従う箱なら、そこへ指定を写して持ち主にする）。台詞の頭では
    /// どの場所の指定も空なので何も写らない。
    pub(super) fn reset_routes(&mut self) {
        self.fold_shared_into_balloon();
        let actors: Vec<ActorKey> = self.routes.keys().cloned().collect();
        for actor in actors {
            let Some(scope) = self.routes.get_mut(&actor) else {
                continue;
            };
            scope.route.reset(&self.box_index);
            let dest = scope.route.dest().clone();
            if dest != TextPlace::Balloon {
                self.carry_script_decor(&actor, &dest);
            }
        }
    }

    /// スコープの指定を箱が持っていたら普通のバルーンの場所へ写し、持ち主を普通のバルーンにする。
    fn fold_shared_into_balloon(&mut self) {
        let actors: Vec<ActorKey> = self.routes.keys().cloned().collect();
        for actor in actors {
            let Some(scope) = self.routes.get_mut(&actor) else {
                continue;
            };
            let shared = std::mem::replace(&mut scope.shared, TextPlace::Balloon);
            if shared != TextPlace::Balloon {
                self.copy_script(&actor, &shared, &TextPlace::Balloon);
            }
        }
    }
}
