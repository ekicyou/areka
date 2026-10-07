//! スコープごとの文字の行き先（純粋層・`state.rs` の子モジュール）。
//!
//! スコープごとに「今のサーフェス番号（非表示なら無し）」と「今の行き先」を持ち、届いた指令の
//! 順に design.md「文字の行き先の決まり方」の表どおりに遷移させる。時計も窓も `World` も見ない。
//! 「箱の列」はサーフェス番号に対する element番号の昇順の箱の名前の列（[`TextLayerState::set_box_index`]
//! で受け取る）で、「既定」は列が空なら普通のバルーン、空でなければ列の先頭の箱である。
//!
//! 不変条件: 行き先が `Box(n)` のとき、`n` は必ずそのスコープの今のサーフェスの箱の列にある。
//! 箱の表が空なら、行き先は 1 度も普通のバルーンから動かない（要件 5.4）。

use std::collections::BTreeMap;

use areka_emo_compose::BoxName;
use areka_sakura::contract::ActorKey;

use super::TextLayerState;
use crate::place::TextPlace;

/// `\s` の鍵を今のシェルで解決した結果（解決そのものは結線の閉包が行う）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKeyOutcome {
    /// そのサーフェス番号を表示する。
    Show(u32),
    /// `\s[-1]`（非表示）。
    Hide,
    /// 解決できない（今のシェルにそのサーフェスが無い場合を含む）。何も変えない。
    Unresolved,
}

/// スコープ 1 つの行き先の状態。記録の無いスコープは「非表示・普通のバルーン」と同じ。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ScopeRoute {
    /// 今のサーフェス番号（`None` は非表示・まだ `\s` が届いていない）。
    surface: Option<u32>,
    /// 今の文字の行き先。
    dest: TextPlace,
    /// スコープの `\f` の指定を今持っている場所（スコープに従う場所だけ・始めは普通のバルーン）。
    /// 行き先が替わるときの写しと持ち主の移動は装飾の持ち運び（state_decoration.rs の
    /// `carry_script_decor`）が読み書きする。
    pub(super) shared: TextPlace,
}

impl Default for ScopeRoute {
    fn default() -> Self {
        ScopeRoute {
            surface: None,
            dest: TextPlace::Balloon,
            shared: TextPlace::Balloon,
        }
    }
}

impl TextLayerState {
    /// 箱の表（サーフェス番号 → element番号の昇順の箱の名前）を差し替える（シェルの切替）。
    ///
    /// 各スコープの今のサーフェス番号は保ち、行き先だけを新しい表での既定へ引き直す。
    /// 箱の場所の文字は捨てる（要件 6.8）。スコープの `\f` の指定を箱が持っていたら、捨てる前に
    /// 普通のバルーンの場所へ写して持ち主を普通のバルーンにする（design.md「装飾の持ち運び」）。
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

    /// `\s` の解決結果でスコープのサーフェス番号と行き先を決める（要件 4.1・4.2・6.1・6.2・6.4・6.6・6.9）。
    pub fn route_surface(&mut self, actor: &ActorKey, outcome: SurfaceKeyOutcome) {
        let surface = match outcome {
            SurfaceKeyOutcome::Show(id) => Some(id),
            SurfaceKeyOutcome::Hide => None,
            SurfaceKeyOutcome::Unresolved => {
                tracing::debug!(actor = %actor, "解決できない \\s——行き先もサーフェス番号も変えない");
                return;
            }
        };
        let index = &self.box_index;
        let route = self.routes.entry(actor.clone()).or_default();
        route.surface = surface;
        let keep = match &route.dest {
            TextPlace::Box(name) => boxes_of(index, surface).contains(name),
            TextPlace::Balloon => false,
        };
        if !keep {
            let dest = default_dest(index, surface);
            self.set_dest(actor, dest);
        }
        tracing::debug!(actor = %actor, ?surface, dest = ?self.destination(actor), "\\s による行き先の決定");
    }

    /// 名前の形の `\b[名前]` で行き先を切り替える（要件 4.3・4.9）。
    ///
    /// 今のサーフェスの箱の列にその名前が無ければ、名前・サーフェス番号・スコープを含む
    /// `warn!` を 1 行残して行き先を変えない（要件 4.4・10.1）。空回しの写し
    /// （[`TextLayerState::rehearsal_copy`]）では `warn!` を残さない（本番の適用が残す）。
    pub fn route_select(&mut self, actor: &ActorKey, name: &str) {
        let surface = self.current_surface(actor);
        let found = boxes_of(&self.box_index, surface)
            .iter()
            .find(|n| n.as_str() == name)
            .cloned();
        match found {
            Some(name) => {
                tracing::debug!(actor = %actor, name = name.as_str(), "\\b[名前] で行き先を箱へ切り替え");
                self.set_dest(actor, TextPlace::Box(name));
            }
            None if self.rehearsal => {}
            None => {
                let surface = surface.map_or_else(|| "非表示".to_owned(), |id| id.to_string());
                tracing::warn!(
                    name,
                    surface = %surface,
                    actor = %actor,
                    "\\b[名前] の箱が今のサーフェスに無い——行き先を切り替えない"
                );
            }
        }
    }

    /// スコープの今の文字の行き先（記録の無いスコープは普通のバルーン）。
    pub fn destination(&self, actor: &ActorKey) -> TextPlace {
        self.routes
            .get(actor)
            .map_or(TextPlace::Balloon, |route| route.dest.clone())
    }

    /// スコープの今のサーフェス番号（非表示・まだ `\s` が届いていなければ `None`）。
    pub fn current_surface(&self, actor: &ActorKey) -> Option<u32> {
        self.routes.get(actor).and_then(|route| route.surface)
    }

    /// 台詞の頭・シェルの切替: 全スコープの行き先を既定へ戻す（要件 4.10。サーフェス番号は変えない）。
    ///
    /// 持ち主を普通のバルーンへ戻してから、普通のバルーンを出発点に既定の行き先へ移す
    /// （既定がスコープに従う箱なら、そこへ指定を写して持ち主にする）。台詞の頭では
    /// どの場所の指定も空なので何も写らない（要件 3.19）。
    pub(super) fn reset_routes(&mut self) {
        self.fold_shared_into_balloon();
        let actors: Vec<ActorKey> = self.routes.keys().cloned().collect();
        for actor in actors {
            let Some(route) = self.routes.get_mut(&actor) else {
                continue;
            };
            route.dest = TextPlace::Balloon;
            let dest = default_dest(&self.box_index, route.surface);
            self.set_dest(&actor, dest);
        }
    }

    /// スコープの指定を箱が持っていたら普通のバルーンの場所へ写し、持ち主を普通のバルーンにする。
    fn fold_shared_into_balloon(&mut self) {
        let actors: Vec<ActorKey> = self.routes.keys().cloned().collect();
        for actor in actors {
            let Some(route) = self.routes.get_mut(&actor) else {
                continue;
            };
            let shared = std::mem::replace(&mut route.shared, TextPlace::Balloon);
            if shared != TextPlace::Balloon {
                self.copy_script(&actor, &shared, &TextPlace::Balloon);
            }
        }
    }

    /// 行き先を `dest` へ替える。替わる瞬間に `\f` の指定を持ち運ぶ（`carry_script_decor`）。
    fn set_dest(&mut self, actor: &ActorKey, dest: TextPlace) {
        if self.destination(actor) == dest {
            return;
        }
        self.carry_script_decor(actor, &dest);
        self.routes.entry(actor.clone()).or_default().dest = dest;
    }
}

/// サーフェスの箱の列（非表示・箱の無い番号は空）。
fn boxes_of(index: &BTreeMap<u32, Vec<BoxName>>, surface: Option<u32>) -> &[BoxName] {
    surface
        .and_then(|id| index.get(&id))
        .map_or(&[], Vec::as_slice)
}

/// 既定の行き先: 箱の列が空なら普通のバルーン、空でなければ先頭の箱（要件 4.2）。
fn default_dest(index: &BTreeMap<u32, Vec<BoxName>>, surface: Option<u32>) -> TextPlace {
    boxes_of(index, surface)
        .first()
        .map_or(TextPlace::Balloon, |name| TextPlace::Box(name.clone()))
}
