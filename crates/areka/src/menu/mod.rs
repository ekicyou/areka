//! 右クリックメニュー（areka-P0-popup-menu-minimal）。
//!
//! このファイルは登記の口を持つ: 枠 7 種とその並び（[`Frame::ORDER`]）、登記の単位
//! （[`MenuItem`]）、枠ごとに 1 つだけ持つ供給関数（[`Supplier`]）、登記・取り消し・写しの
//! 取得（[`MenuRegistry`]）。後続 spec（列挙・切替・インストール・更新）はメニュー本体を
//! 触らず、自分の枠へ供給関数を登記するだけで項目を足せる（要件 6.1）。
//! 登記の口と、表示 1 枚の旗・照会の返事待ちは [`MenuWiring`] が 1 つの資源として束ねる。
//! 起動時の結線（[`wire_menu`]）と、本体が自分で登記する 2 項目（説明書・終了）もここに置く。
//!
//! 配下の module は役割ごとに分かれる: [`plan`]（構造の計算）・[`captions`]（項目名と
//! 表示可否の照会）・[`trigger`]（引き金と表示の段取り）・[`win32`]（OS 表示）。

pub(crate) mod captions;
pub(crate) mod plan;
pub(crate) mod trigger;
pub(crate) mod win32;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use areka_kanade::{CloseReason, KanadeMsg};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::Schedules;
use windows::Win32::Foundation::HWND;
use wintf::ecs::Input;
use wintf::ecs::pointer::{OnPointerReleased, dispatch_pointer_events};

use crate::input_events::MouseWiring;
use crate::placement::spawn::CharWindowMarker;
use crate::readme;
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
    // 本仕様の組込 2 項目はどちらも葉で、本番でこれを作るのは後続 spec
    // （ghost-shell-balloon-switch ほか・一覧をサブメニューで出す）である。
    #[allow(dead_code)]
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
    // 後続 spec（ghost-shell-balloon-switch ほか）が自分の枠を下ろす口。本仕様の中に呼び手は無い。
    #[allow(dead_code)]
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

/// メニューを起動に結ぶ。boot 成功後に `main.rs` から 1 回だけ呼ぶ（入力の結線の直後）。
///
/// 行うのは結線状態の挿入・組込 2 項目の登記・返事の取り出しの登録までで、**窓には触れない**。
/// 呼ばれるのは `app.run()` の前で、キャラクター窓はまだ 1 枚も無いからである（窓を作る
/// クロージャは `app.run()` の最初の数 tick で動く）。解放ハンドラはそのクロージャが
/// [`attach_release_handlers`] で付ける。
///
/// `Schedules` 資源は在る前提（`wire_choice_drain` と同じ）。運行（kanade）への送り口は
/// 照会（`KanadeMsg::ResourceQuery`）に使う。
pub(crate) fn wire_menu(world: &mut World, kanade: Sender<KanadeMsg>) {
    wire_menu_with(world, MenuWiring::new(kanade));
}

/// 結線の中身。結線状態を受け取るのは、テストが画面座標への写しを差し替えた状態
/// （[`MenuWiring::with_to_screen`]）で同じ手順を通せるようにするためである。
///
/// 手順は ⑴ 組込の 2 項目を登記して結線状態を World へ入れる、⑵ 照会の返事の取り出しを
/// 毎 tick の入力の段へ登録する。⑵ の並びは `dispatch_pointer_events` の後——解放ハンドラが
/// 同じ tick に預けた返事待ちを、その tick のうちに 1 度覗けるようにする。
fn wire_menu_with(world: &mut World, mut wiring: MenuWiring) {
    wiring
        .registry
        .register(Frame::Readme, Rc::new(readme_item));
    wiring.registry.register(Frame::Close, Rc::new(close_item));
    world.insert_non_send(wiring);
    world.resource_mut::<Schedules>().add_systems(
        Input,
        trigger::poll_menu_query.after(dispatch_pointer_events),
    );
}

/// 全 [`CharWindowMarker`] 窓へ解放ハンドラ（[`trigger::on_char_pointer_released`]）を付ける。
///
/// # タイミング契約
///
/// キャラクター窓を作った**直後**に、同じ `&mut World` クロージャの中で呼ぶこと（`main.rs` の
/// `open_startup_window` が `input_events::attach_char_pointer_handlers` の隣で呼ぶ）。付くのは
/// 呼ばれた時点に在る窓だけなので、窓を作り直す spec は作り直した窓へもう一度呼ぶ。既に付いて
/// いる窓へ呼んでも、同じハンドラで置き換わるだけで害は無い。
///
/// [`wire_menu`] との前後は問わない。ハンドラは結線（`MenuWiring`／`MouseWiring`）が無ければ
/// 解放を無視するだけなので、boot に失敗して結線の無い起動でも害は無い。
///
/// 付けた枚数は `debug!` で記録する。0 枚は意図した呼び方ではありえない（窓より先に呼んだ）
/// ので `warn!` にする——黙って 0 枚に付けると、メニューが出ない理由がログに残らない。
pub(crate) fn attach_release_handlers(world: &mut World) {
    // クエリが `&mut World` を借りるので、先に対象を集めてから 1 件ずつ付ける。
    let char_windows: Vec<Entity> = world
        .query_filtered::<Entity, With<CharWindowMarker>>()
        .iter(world)
        .collect();
    let count = char_windows.len();
    for window in char_windows {
        world
            .entity_mut(window)
            .insert(OnPointerReleased(trigger::on_char_pointer_released));
    }
    if count == 0 {
        tracing::warn!(
            event = "menu_release_handlers_attached",
            count,
            "[menu] no character window to attach the release handler to: the menu cannot open"
        );
    } else {
        tracing::debug!(
            event = "menu_release_handlers_attached",
            count,
            "[menu] release handlers attached"
        );
    }
}

/// 枠へ供給関数を登記する（World 越しの入口）。結線の前に呼ばれたら `warn!` で記録して
/// 何もしない。
// 後続 spec（ghost-shell-balloon-switch ほか）の登記の口。本仕様の中に呼び手は無い。
#[allow(dead_code)]
pub(crate) fn register(world: &mut World, frame: Frame, supplier: Supplier) {
    let Some(mut wiring) = world.get_non_send_mut::<MenuWiring>() else {
        tracing::warn!(
            event = "menu_register_no_wiring",
            frame = ?frame,
            "[menu] MenuWiring absent: the registration is dropped"
        );
        return;
    };
    wiring.registry.register(frame, supplier);
}

/// ⑥「説明書」の供給関数。有効／無効は写しを取るたびにファイルの在否で決まる
/// （要件 4.3・6.2・11.4）。項目名とリソース名は [`captions::FRAME_CAPTIONS`] から引く。
fn readme_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    MenuItem {
        label: captions::default_label(Frame::Readme).to_string(),
        caption_resource: Some(captions::resource_for(Frame::Readme)),
        enabled: readme::is_available(world),
        checked: None,
        body: ItemBody::Action(Rc::new(open_readme)),
    }
}

/// 「説明書」の動作。台本の `\![open,readme]` と同じ関数で開く（要件 4.2）。
fn open_readme(world: &mut World, _ctx: &MenuContext) {
    readme::open_from_world(world);
}

/// ⑦「終了」の供給関数。常に選べる（要件 2.2）。
fn close_item(_world: &World, _ctx: &MenuContext) -> MenuItem {
    MenuItem {
        label: captions::default_label(Frame::Close).to_string(),
        caption_resource: Some(captions::resource_for(Frame::Close)),
        enabled: true,
        checked: None,
        body: ItemBody::Action(Rc::new(request_close)),
    }
}

/// 「終了」の動作。既存の終了指示の経路（`MouseWiring::send_close_request`）へ、メニューを
/// 出した窓のスコープを載せた終了指示を 1 件送る（要件 5.1・6.6）。メニューだけの終了の道は
/// 作らない——窓を閉じるのは、終了の握手が終わったことを受けた側である。
fn request_close(world: &mut World, ctx: &MenuContext) {
    let Some(mut wiring) = world.get_non_send_mut::<MouseWiring>() else {
        tracing::warn!(
            event = "menu_close_no_mouse_wiring",
            scope = ctx.scope,
            "[menu] MouseWiring absent: the close request is not sent"
        );
        return;
    };
    wiring.send_close_request(CloseReason::User { scope: ctx.scope });
}

#[cfg(test)]
#[path = "mod_registry_tests.rs"]
mod registry_tests;

#[cfg(test)]
#[path = "mod_wiring_tests.rs"]
mod wiring_tests;
