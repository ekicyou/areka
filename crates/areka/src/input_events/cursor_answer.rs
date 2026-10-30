//! キャラクターの窓の形の問い合わせへの答えと、その登録（shell-mouse-cursor・design.md「CursorAnswer」）。
//!
//! wintf の窓の手続きが `CursorQuery` の関数を呼ぶ。キャラクターの窓だけ受け持ち、当たり判定は
//! 見張り（`char_watch`）と同じ解決の関数で引き、形は CursorPick の純関数で選ぶ。

use std::cell::RefCell;
use std::collections::HashMap;

use bevy_ecs::prelude::*;
use tracing::{debug, error};
use windows::Win32::UI::WindowsAndMessaging::{
    HCURSOR, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO, IDC_SIZEALL, IDC_WAIT,
};
use wintf::ecs::cursor::{
    CursorAsk, CursorQuery, CursorReply, OwnedCursor, cursor_size_for_dpi, stock_cursor,
};

use super::super::hover_region::cursor_pick::{
    Origin, Shown, Spot, WindowCursor, pick_shape, plan_cursor,
};
use super::super::{MouseWiring, char_scope, resolve_hit_owned};
use crate::emo2_boot::shell_cursor_assets::ShellCursorAssets;
use crate::emo2_boot::shell_cursor_table::{CursorRows, GhostCursorKeys, Shape, SystemCursor};
use crate::placement::spawn::CharWindowMarker;

/// 同梱の手の形（要件 6.2）。番号は控えの鍵。
const BUNDLED: [(SystemCursor, &[u8]); 2] = [
    (SystemCursor::Hand, include_bytes!("../../cursor/hand.cur")),
    (SystemCursor::Grip, include_bytes!("../../cursor/grip.cur")),
];

/// OS の標準の形（要件 6.1）。
const STOCK: [SystemCursor; 8] = [
    SystemCursor::Arrow,
    SystemCursor::Cross,
    SystemCursor::Finger,
    SystemCursor::Help,
    SystemCursor::Move,
    SystemCursor::No,
    SystemCursor::Text,
    SystemCursor::Wait,
];

/// 最後に出した場面と形と出どころ（記録の比べに使う）。
type Picked = (Shown, Option<(Shape<usize>, Origin)>);

/// 窓ごとの形の状態と、標準の形・同梱の絵の控え（NonSend・UI スレッドだけ）。
pub(crate) struct CharCursors {
    windows: RefCell<HashMap<Entity, (WindowCursor, Picked)>>,
    /// 読めなかった形は `None`（今の振る舞いへ）。
    stock: Vec<(SystemCursor, Option<HCURSOR>)>,
    /// (BUNDLED の番号, 寸法) → 作ったカーソル（作れなければ `None`）。
    bundled: RefCell<HashMap<(usize, u32), Option<OwnedCursor>>>,
}

/// 答えの関数と窓ごとの状態を入れ、標準の形を 1 度だけ読む（`register_char_watch` から 1 回）。
pub(crate) fn register_char_cursor(world: &mut World) {
    let stock = STOCK
        .iter()
        .map(|&s| {
            let made = stock_cursor(stock_id(s))
                .inspect_err(|e| error!(shape = ?s, error = %e, "標準のカーソルを読めない"))
                .ok();
            (s, made)
        })
        .collect();
    world.insert_non_send(CharCursors {
        windows: RefCell::default(),
        stock,
        bundled: RefCell::default(),
    });
    world.insert_resource(CursorQuery(answer_char_cursor));
}

fn stock_id(s: SystemCursor) -> windows::core::PCWSTR {
    match s {
        SystemCursor::Arrow => IDC_ARROW,
        SystemCursor::Cross => IDC_CROSS,
        SystemCursor::Finger | SystemCursor::Hand | SystemCursor::Grip => IDC_HAND,
        SystemCursor::Help => IDC_HELP,
        SystemCursor::Move => IDC_SIZEALL,
        SystemCursor::No => IDC_NO,
        SystemCursor::Text => IDC_IBEAM,
        SystemCursor::Wait => IDC_WAIT,
    }
}

impl CharCursors {
    fn stock(&self, s: SystemCursor) -> Option<HCURSOR> {
        self.stock
            .iter()
            .find(|(k, _)| *k == s)
            .and_then(|(_, h)| *h)
    }

    /// 同梱の絵を寸法ごとに 1 度だけ作る。作れなければその寸法で 1 度 error を残し、指差しを出す。
    fn bundled(&self, index: usize, bytes: &[u8], size: u32) -> Option<HCURSOR> {
        self.bundled
            .borrow_mut()
            .entry((index, size))
            .or_insert_with(|| {
                OwnedCursor::from_cur_bytes(bytes, size)
                    .inspect_err(|e| {
                        error!(shape = ?BUNDLED[index].0, size, error = %e, "同梱のカーソルを作れないので指差しを出す")
                    })
                    .ok()
            })
            .as_ref()
            .map(OwnedCursor::handle)
            .or_else(|| self.stock(SystemCursor::Finger))
    }

    fn handle(
        &self,
        shape: &Shape<usize>,
        assets: &ShellCursorAssets,
        size: u32,
    ) -> Option<HCURSOR> {
        match shape {
            Shape::File(n) => assets.cursor_for(*n, size),
            Shape::System(s) => match BUNDLED.iter().position(|(k, _)| k == s) {
                Some(i) => self.bundled(i, BUNDLED[i].1, size),
                None => self.stock(*s),
            },
        }
    }
}

/// 問い合わせへの答え（`CursorQuery` の関数）。キャラクターの窓でなければ、または結線や資源が
/// 無ければ `None`（今の振る舞い）。透けた所は `Keep`。
pub(crate) fn answer_char_cursor(
    world: &World,
    window: Entity,
    ask: &CursorAsk,
) -> Option<CursorReply> {
    let scope = char_scope(world, window)?;
    world.get_non_send::<MouseWiring>()?;
    let assets = world.get_non_send::<ShellCursorAssets>()?;
    let cursors = world.get_non_send::<CharCursors>()?;
    // キャラクターの窓は高々数枚なので、答えるたびに消えた窓を刈る。
    cursors
        .windows
        .borrow_mut()
        .retain(|e, _| world.get::<CharWindowMarker>(*e).is_some());
    let prev = cursors.windows.borrow().get(&window).cloned();

    let resolve = || {
        if ask.see_through {
            return Spot::SeeThrough;
        }
        let (x, y) = (i64::from(ask.client.0), i64::from(ask.client.1));
        match resolve_hit_owned(world, scope, x, y).region {
            Some(r) => Spot::Region(r),
            None => Spot::Plain,
        }
    };
    let (next, shown) = plan_cursor(
        prev.as_ref().map(|(c, _)| c),
        ask.trigger,
        ask.client,
        ask.buttons_down,
        resolve,
    );

    // 作れなかったファイルはその場面の既定へ落とす（ブレス → descript → 既定・要件 6.5）。
    let size = cursor_size_for_dpi(ask.dpi);
    let (no_rows, no_ghost) = (CursorRows::default(), GhostCursorKeys::default());
    let tiers = [
        (&assets.rows, &assets.ghost),
        (&no_rows, &assets.ghost),
        (&no_rows, &no_ghost),
    ];
    let (picked, cursor) = tiers
        .iter()
        .find_map(|(rows, ghost)| {
            let picked = pick_shape(scope, &shown, rows, ghost)?;
            let cursor = cursors.handle(&picked.0, assets, size)?;
            Some((Some(picked), Some(cursor)))
        })
        .unwrap_or_else(|| (pick_shape(scope, &shown, &no_rows, &no_ghost), None));

    let now: Picked = (shown, picked);
    if prev.as_ref().map(|(_, p)| p) != Some(&now) {
        let (shown, picked) = &now;
        let (scene, region) = match shown {
            Shown::Region { scene, region } => (format!("{scene:?}"), Some(region.as_str())),
            other => (format!("{other:?}"), None),
        };
        debug!(
            scope,
            region,
            scene,
            shape = ?picked.as_ref().map(|p| &p.0),
            origin = ?picked.as_ref().map(|p| p.1),
            "キャラクターの窓のカーソルの形を選び直した"
        );
    }
    let reply = match (&now.0, cursor) {
        (Shown::Pass, _) => Some(CursorReply::Keep),
        (_, Some(h)) => Some(CursorReply::Show(h)),
        (_, None) => None,
    };
    cursors.windows.borrow_mut().insert(window, (next, now));
    reply
}

#[cfg(test)]
#[path = "cursor_answer_tests.rs"]
mod tests;
