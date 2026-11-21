//! 窓ごとの当たり判定の出入りの状態と、出入りを決める純関数。
//!
//! 状態の置き場所は `MouseWiring.hover`（鍵＝窓の entity）。後から足すマウスカーソル・ツールチップ・
//! ホバーアニメーションは [`HoverState`] を読む口を足して使う。結線は `char_watch`。

use areka_kanade::HoverEdge;

/// 窓 1 つの見張りの状態。表に在る＝カーソルがその窓の上に居る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HoverState {
    pub scope: u32,
    /// 前に見た位置（窓 client 物理 px・動いたかの比べに使う）。
    pub client: (i64, i64),
    /// 前に見た位置（縮約後サーフェス px・Reference に載せる）。
    pub surface: (i64, i64),
    pub region: Option<String>,
}

/// 今の見え方: 窓の上（その位置の当たり判定つき）か、窓の外か。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum HoverSighting {
    Over {
        scope: u32,
        client: (i64, i64),
        surface: (i64, i64),
        region: Option<String>,
    },
    Away,
}

/// 送る出入り 1 つ（Reference0／1＝`surface`・Reference4＝`region`〔無ければ空〕）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HoverSend {
    pub edge: HoverEdge,
    pub surface: (i64, i64),
    pub region: Option<String>,
}

/// 前の状態と今の見え方から、次の状態と送る出入り（送る順）を返す（7 行の表）。
pub(super) fn plan_hover(
    prev: Option<&HoverState>,
    now: HoverSighting,
) -> (Option<HoverState>, Vec<HoverSend>) {
    match now {
        HoverSighting::Away => {
            let Some(p) = prev else {
                return (None, Vec::new());
            };
            // 座標は窓の中で最後に見た位置。
            let mut sends = Vec::new();
            if p.region.is_some() {
                sends.push(HoverSend {
                    edge: HoverEdge::Leave,
                    surface: p.surface,
                    region: p.region.clone(),
                });
            }
            sends.push(HoverSend {
                edge: HoverEdge::LeaveAll,
                surface: p.surface,
                region: p.region.clone(),
            });
            (None, sends)
        }
        HoverSighting::Over {
            scope,
            client,
            surface,
            region,
        } => {
            let mut sends = Vec::new();
            match prev {
                None => {
                    sends.push(HoverSend {
                        edge: HoverEdge::EnterAll,
                        surface,
                        region: region.clone(),
                    });
                    if region.is_some() {
                        sends.push(HoverSend {
                            edge: HoverEdge::Enter,
                            surface,
                            region: region.clone(),
                        });
                    }
                }
                Some(p) if p.region != region => {
                    if p.region.is_some() {
                        sends.push(HoverSend {
                            edge: HoverEdge::Leave,
                            surface,
                            region: p.region.clone(),
                        });
                    }
                    if region.is_some() {
                        sends.push(HoverSend {
                            edge: HoverEdge::Enter,
                            surface,
                            region: region.clone(),
                        });
                    }
                }
                Some(_) => {}
            }
            let next = HoverState {
                scope,
                client,
                surface,
                region,
            };
            (Some(next), sends)
        }
    }
}

#[path = "cursor_pick.rs"]
pub(super) mod cursor_pick;

#[cfg(test)]
#[path = "hover_region_tests.rs"]
mod tests;
