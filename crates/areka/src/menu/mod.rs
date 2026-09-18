//! 右クリックメニュー（areka-P0-popup-menu-minimal）。
//!
//! このファイルは登記の口を持つ: 枠 7 種とその並び（[`Frame::ORDER`]）、登記の単位
//! （[`MenuItem`]）、枠ごとに 1 つだけ持つ供給関数（[`Supplier`]）、登記・取り消し・写しの
//! 取得（[`MenuRegistry`]）。後続 spec（列挙・切替・インストール・更新）はメニュー本体を
//! 触らず、自分の枠へ供給関数を登記するだけで項目を足せる（要件 6.1）。
//! 登記の口と、表示 1 枚の旗・照会の返事待ちは [`MenuWiring`] が 1 つの資源として束ねる。
//!
//! 配下の module は役割ごとに分かれる: [`plan`]（構造の計算）・[`captions`]（項目名と
//! 表示可否の照会）・[`trigger`]（引き金と表示の段取り）・[`win32`]（OS 表示）。

// 登記の口は、メニューの結線（`wire_menu`・task 8.1）が入るまで本番から呼ばれない。
// 結線が入った時点でこの許可を外す（配下 module にも及ぶので、個別の属性は置かない）。
#![allow(dead_code)]

pub(crate) mod captions;
pub(crate) mod plan;
pub(crate) mod trigger;
pub(crate) mod win32;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use areka_kanade::KanadeMsg;
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;

use trigger::PendingQuery;

/// メニューの枠。宣言順がそのまま並び順（①〜⑦・要件 2.1）で、判別値を登記の添字に使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Frame {
    /// ① ゴースト
    Ghost,
    /// ② シェル
    Shell,
    /// ③ バルーン
    Balloon,
    /// ④ ネットワーク更新
    Update,
    /// ⑤ インストール
    Install,
    /// ⑥ 説明書
    Readme,
    /// ⑦ 終了
    Close,
}

impl Frame {
    /// 枠の並び順。並びを決める場所はここだけで、写しも計画もこの順に従う。
    pub(crate) const ORDER: [Frame; 7] = [
        Frame::Ghost,
        Frame::Shell,
        Frame::Balloon,
        Frame::Update,
        Frame::Install,
        Frame::Readme,
        Frame::Close,
    ];
}

/// メニューを出した操作の文脈。供給関数と動作の両方へ渡す。
pub(crate) struct MenuContext {
    /// 右クリックされたキャラクター窓のスコープ番号（0＝本体側・1＝相方側）。
    pub scope: u32,
}

/// 枠の項目をその場で作る供給関数。メニューを出すたびに呼ばれる（要件 6.2）。
pub(crate) type Supplier = Rc<dyn Fn(&World, &MenuContext) -> MenuItem>;

/// 項目が選ばれたときの動作。メニューを閉じた後に 1 回だけ呼ばれる。
pub(crate) type MenuAction = Rc<dyn Fn(&mut World, &MenuContext)>;

/// 登記の単位（要件 6.1）。
pub(crate) struct MenuItem {
    /// 既定名（リソースが空のとき使う）。
    pub label: String,
    /// 文言に使う SHIORI リソース名（任意・要件 3.9）。
    pub caption_resource: Option<&'static str>,
    /// `false` は選べない状態（灰色）で出す（要件 2.5）。
    pub enabled: bool,
    /// `Some(true)` でチェックを付ける（要件 2.4）。
    pub checked: Option<bool>,
    /// 選ばれたときの動作か、子項目の列のどちらか一方。
    pub body: ItemBody,
}

/// 項目の中身。動作を持つ葉か、子項目を登記順に並べたサブメニュー。
pub(crate) enum ItemBody {
    Action(MenuAction),
    Submenu(Vec<MenuItem>),
}

/// 登記の口。枠ごとに供給関数を高々 1 つ持つ。
#[derive(Default)]
pub(crate) struct MenuRegistry {
    slots: [Option<Supplier>; 7],
}

impl MenuRegistry {
    /// 枠へ供給関数を登記する。既に登記のある枠は後から来たものへ置き換え、`warn!` で記録する（要件 6.3）。
    pub(crate) fn register(&mut self, frame: Frame, supplier: Supplier) {
        if self.slots[frame as usize].replace(supplier).is_some() {
            tracing::warn!(
                event = "menu_registration_replaced",
                frame = ?frame,
                "[menu] registration replaced: the later supplier wins"
            );
        }
    }

    /// 枠の登記を取り消す（要件 6.4）。登記の無い枠に対しては何もしない。
    pub(crate) fn unregister(&mut self, frame: Frame) {
        self.slots[frame as usize] = None;
    }

    /// 登記のある枠だけを [`Frame::ORDER`] の順に、供給関数をその場で呼んで返す（要件 6.2）。
    pub(crate) fn snapshot(&self, world: &World, ctx: &MenuContext) -> Vec<(Frame, MenuItem)> {
        Frame::ORDER
            .iter()
            .filter_map(|&frame| {
                let supplier = self.slots[frame as usize].as_ref()?;
                Some((frame, supplier(world, ctx)))
            })
            .collect()
    }
}

/// 窓のクライアント座標を画面座標へ写す関数の型。写せなければ `None`。
///
/// 本番は [`win32::client_to_screen`] で、実在する窓ハンドルが要る。テストは実窓を作らずに
/// 解放ハンドラを通すため、ここへ純粋な関数を差し込む（`input_events` の
/// `RegionSource::Mock` と同じ考え方の差し替え口）。
pub(crate) type ToScreen = fn(HWND, i32, i32) -> Option<(i32, i32)>;

/// メニューに要る結線状態（UI スレッドだけが持つ NonSend 資源）。
///
/// 登記の口・運行（kanade）への送り口・表示 1 枚の旗・照会の返事待ちを 1 つに束ねる。
/// 旗と返事待ちを動かすのは [`trigger`] だけである。
pub(crate) struct MenuWiring {
    /// 登記の口。後続 spec はここへ供給関数を登記する。
    pub registry: MenuRegistry,
    /// 照会（`KanadeMsg::ResourceQuery`）の送り口。
    kanade: Sender<KanadeMsg>,
    /// 表示は 1 枚まで。右ボタンの解放で立ち、[`trigger::InFlightGuard`] が落ちると降りる。
    in_flight: Rc<Cell<bool>>,
    /// 照会の返事待ち（高々 1 件）。[`trigger::poll_menu_query`] が毎 tick 覗く。
    pending: Option<PendingQuery>,
    /// クライアント座標から画面座標への写し。
    to_screen: ToScreen,
}

impl MenuWiring {
    /// 本番の構築子。座標の写しには OS（[`win32::client_to_screen`]）を使う。
    pub(crate) fn new(kanade: Sender<KanadeMsg>) -> Self {
        Self::with_to_screen(kanade, win32::client_to_screen)
    }

    /// 座標の写しを差し替えて組み立てる。本番は [`MenuWiring::new`] 経由でだけ通る。
    fn with_to_screen(kanade: Sender<KanadeMsg>, to_screen: ToScreen) -> Self {
        MenuWiring {
            registry: MenuRegistry::default(),
            kanade,
            in_flight: Rc::new(Cell::new(false)),
            pending: None,
            to_screen,
        }
    }
}

#[cfg(test)]
#[path = "mod_registry_tests.rs"]
mod registry_tests;
