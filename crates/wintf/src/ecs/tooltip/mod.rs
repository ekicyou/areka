//! ツールチップ（マウスを置いた所に出る短い説明）。
//!
//! 窓の中に「範囲」を登録しておくと、マウスがその範囲に入ってしばらく止まったときに、
//! OS の標準のツールチップで説明の文字を出す。出すまでの待ち時間・消すきっかけ・置き場所は
//! wintf が決める（WinUI 3 のツールチップに倣った決まり。下の「決まり」）。
//!
//! 関数は `wintf::ecs::tooltip::register(...)` のようにこのモジュールの道で呼ぶ。型は
//! `wintf::ecs` からも使える。どの関数も UI スレッドから呼ぶ。
//!
//! # 使い方
//!
//! ## 文字を預けておく（静的）
//!
//! 説明が決まっているなら、登録のときに文字を預ける。出す番が来ると wintf がそのまま出す。
//!
//! ```no_run
//! use bevy_ecs::prelude::*;
//! use wintf::ecs::{Rect, TooltipArea, TooltipRange, TooltipRangeId, TooltipRegisterError};
//!
//! /// `window` は `Window` を持つエンティティ（窓のハンドルがまだ無くてもよい）。
//! fn add_help(world: &mut World, window: Entity) -> Result<TooltipRangeId, TooltipRegisterError> {
//!     // 窓の中の矩形（論理の単位＝96 DPI の 1 ピクセル。左と上の端を含み、右と下の端を含まない）。
//!     let button = TooltipRange {
//!         area: TooltipArea::Rect(Rect { left: 10.0, top: 10.0, right: 110.0, bottom: 40.0 }),
//!         text: Some("保存する\n（Ctrl+S）".to_string()),
//!     };
//!     wintf::ecs::tooltip::register(world, window, button)
//! }
//!
//! /// 窓の全体を 1 つの範囲にする。重なった所で数え始めるのは後から登録した範囲。ただし窓の全体の
//! /// 説明が出ている間は、中のボタンへ移っても切り替わらない（下の「範囲の粒度」）。
//! fn add_whole_window_help(world: &mut World, window: Entity) {
//!     let whole = TooltipRange {
//!         area: TooltipArea::WholeWindow,
//!         text: Some("ドラッグで動かせます".to_string()),
//!     };
//!     let id = wintf::ecs::tooltip::register(world, window, whole).expect("窓がある");
//!
//!     // 中身の差し替え（重なりの順は変わらない。次の出す番から効く）と取り消し。
//!     let changed = TooltipRange { area: TooltipArea::WholeWindow, text: Some("右クリックでメニュー".into()) };
//!     wintf::ecs::tooltip::update(world, id, changed);
//!     wintf::ecs::tooltip::unregister(world, id);
//! }
//! ```
//!
//! ## 出す番が来たときに文字を作る（動的）
//!
//! 文字を預けない範囲（`text` が `None` か空）は、出す番が来ると窓に付けた [`OnTooltip`] の関数に
//! 知らせるだけで、何も出さない。知らせの中で [`supply_text`] を呼べば、その場で出る。
//!
//! ```no_run
//! use bevy_ecs::prelude::*;
//! use wintf::ecs::{OnTooltip, TooltipArea, TooltipNotice, TooltipRange};
//!
//! fn on_tooltip(world: &mut World, notice: &TooltipNotice) {
//!     if let TooltipNotice::TurnStarted(turn) = notice {
//!         // turn.position は知らせを出した時のマウスの位置（窓の中・論理の単位）。
//!         let text = format!("x = {:.0}, y = {:.0}", turn.position.x, turn.position.y);
//!         wintf::ecs::tooltip::supply_text(world, turn.token, &text);
//!     }
//! }
//!
//! fn setup(world: &mut World, window: Entity) {
//!     world.entity_mut(window).insert(OnTooltip(on_tooltip));
//!     let range = TooltipRange { area: TooltipArea::WholeWindow, text: None };
//!     wintf::ecs::tooltip::register(world, window, range).expect("窓がある");
//! }
//! ```
//!
//! ## 文字が後から届く
//!
//! 知らせの中で文字を用意できないときは、知らせに載った印 [`TooltipTurnToken`] を覚えておき、
//! 文字が出来たときに印と一緒に渡す。渡す前にマウスが離れていれば、出す番は終わっていて
//! [`TooltipSupply::StaleTurn`] が返り、何も出ない。
//!
//! ```no_run
//! use bevy_ecs::prelude::*;
//! use std::time::{Duration, Instant};
//! use wintf::ecs::world::tick_wake;
//! use wintf::ecs::{TooltipNotice, TooltipSupply, TooltipTurnToken, Update};
//!
//! /// 文字を待っている出す番の印と、文字が出来る時刻。
//! #[derive(Resource, Default)]
//! struct Pending(Option<(TooltipTurnToken, Instant)>);
//!
//! /// 窓に `OnTooltip(on_tooltip)` を付けておく。
//! fn on_tooltip(world: &mut World, notice: &TooltipNotice) {
//!     match notice {
//!         TooltipNotice::TurnStarted(turn) => {
//!             let ready_at = Instant::now() + Duration::from_secs(1);
//!             world.insert_resource(Pending(Some((turn.token, ready_at))));
//!             // マウスが止まっていても、その時刻に画面更新が回るよう期限を預ける。出す番の間は
//!             // 画面更新がおよそ 100 ミリ秒ごとに回るので、渡すのはその時刻から最大 100 ミリ秒ほど
//!             // 遅れうる。
//!             tick_wake::arm_deadline(ready_at);
//!         }
//!         TooltipNotice::TurnEnded { token, .. } => {
//!             // 文字が出来る前に終わった。待つのをやめる（やめずに渡しても StaleTurn が返るだけ）。
//!             let mut pending = world.resource_mut::<Pending>();
//!             if pending.0.is_some_and(|(t, _)| t == *token) {
//!                 pending.0 = None;
//!             }
//!         }
//!         _ => {}
//!     }
//! }
//!
//! /// 画面更新ごとに回す。文字が出来ていれば渡す。
//! fn deliver(world: &mut World) {
//!     let Some((token, ready_at)) = world.resource::<Pending>().0 else { return };
//!     if Instant::now() < ready_at {
//!         return;
//!     }
//!     world.resource_mut::<Pending>().0 = None;
//!     match wintf::ecs::tooltip::supply_text(world, token, "1 秒後に届いた説明") {
//!         TooltipSupply::Shown => {}
//!         TooltipSupply::StaleTurn => {} // 既に終わっていた。何も出ていない
//!         other => eprintln!("出せなかった: {other:?}"),
//!     }
//! }
//!
//! fn setup(world: &mut World) {
//!     world.init_resource::<Pending>();
//!     world.resource_mut::<Schedules>().add_systems(Update, deliver);
//! }
//! ```
//!
//! # 決まり
//!
//! - **待ち時間**: 範囲に入ってから、OS の「マウスを止めたとみなす時間」の設定（既定は
//!   400 ミリ秒）の **2 倍**（既定なら約 0.8 秒）で出す番が来る。設定は数え始めるたびに読む。
//!   読めなければ 400 ミリ秒として続ける。範囲の中で動かしても数え直さない。待ち時間を変える口は無い。
//! - **出し直し**: 前のツールチップが消えてから **0.2 秒以内**に範囲に入ったとき（同じ範囲でも）は、
//!   待ち時間は設定の **1 倍**（既定なら 0.4 秒）。
//! - **置き場所**: その出す番で初めて出した時のマウスの位置の真上に、左右の中央を合わせ、
//!   少し離して出す。上に収まらなければ下に出し、画面の作業領域からはみ出さないよう寄せる。
//!   幅は 320（論理の単位）と作業領域の幅の小さい方まで。越える行は折り返す（切れ目の無い長い
//!   URL も幅で割る）。改行は `\n`・`\r\n`・`\r` のどれでもよい。出ている間は位置を動かさない。
//! - **安全地帯**: マウスが「範囲」「ツールチップ」「その 2 つをつなぐ通り道」のどこかにある間は
//!   消えない。範囲からツールチップの上へマウスを移しても消えない。出たまま置いておく限り、時間で
//!   消えることは無い。
//! - **消えるきっかけ**（出す番の終わり。[`TooltipEndReason`]）:
//!   - マウスが安全地帯から出た（`LeftSafeZone`）。
//!   - マウスのボタン（左・右・中・拡張の 2 つ）のどれかを押した（`ButtonPressed`）。押した後は、
//!     範囲から出て入り直すまで出ない。
//!   - 窓が隠れた、または窓の絵や当たり判定が消えてマウスの下で窓が受けなくなった
//!     （`WindowHidden`）。
//!   - 窓のエンティティが無くなった（`WindowDestroyed`）。
//!   - 範囲の登録を取り消した（`RangeUnregistered`）。[`unregister`] は戻る前に消して終わりを知らせる。
//!
//!   どれも出す番そのものが終わり、[`TooltipNotice::TurnEnded`] が届く。一方、[`dismiss`] と空の文字の
//!   [`supply_text`] は**ツールチップを消すだけで出す番は続く**（続いて文字を渡せばまた出る）。
//! - **マウスを受けている所だけ**: 範囲の矩形の中でも、窓がそこでマウスを受けていない所（透過の窓の
//!   絵の無い所など）では出す番は来ない。範囲が重なった所で数え始めるのは、後から登録した範囲
//!   （出す番が来た後の切り替わりは下の「範囲の粒度」）。
//! - **出す番と印**: 出す番は同時に 1 つ（全部の窓を通して）。出す番ごとに新しい印
//!   [`TooltipTurnToken`] が付き、使い回されない。同じ印について、来た（[`TooltipNotice::TurnStarted`]）
//!   と終わった（[`TooltipNotice::TurnEnded`]）はそれぞれ 1 回だけ届く。待っている途中で範囲から
//!   出ただけなら、何も届かない。知らせの関数は World を借りたまま同期で呼ばれるので、その中から
//!   [`supply_text`] などを呼べる。
//! - **預けた文字と渡した文字**: 文字を預けた範囲では、知らせが届く前に既に出ている
//!   （[`TooltipTurn::has_text`] が真）。その間に [`supply_text`] を呼ぶと、渡した文字に置き換わる。
//!   預けていない範囲では、渡されるまで何も出ない。出す番が続いている限り、渡すのに時間の上限は無い。
//! - **終わった印へ渡したとき**: [`TooltipSupply::StaleTurn`] が返り、何も出さない（[`dismiss`] は偽）。
//!   前の出す番の印を、後の出す番の間に渡しても同じ。誤りではないので、気にせず渡してよい。
//! - **範囲の粒度**: 1 つの範囲の中では、マウスを動かしても出し直さない。説明を出し分けたい単位
//!   （ボタンごと・項目ごと）で、別々の範囲として登録する。範囲が切り替わるかは、出す番が来る前か
//!   後かで違う:
//!   - 来る前（待っている間）に別の範囲へ移ると、その範囲に切り替えて最初から数え直す（重なった所
//!     では後から登録した範囲）。
//!   - 来た後は、マウスが安全地帯（その範囲・ツールチップ・通り道）の中にある限り、別の範囲の上へ
//!     移っても同じ出す番が続き、出ている説明も変わらない。安全地帯から出た回のうちに前の出す番が
//!     終わり、その時マウスの下にある範囲で数え始める。
//!
//!   このため、範囲の中に別の範囲を入れる（窓の全体とその中のボタンなど）と、外側の説明が出ている
//!   間は、内側へ移っても内側の説明に切り替わらない。行き来しながら説明を切り替えたい範囲は、
//!   重ねずに並べて登録する。
//! - [`update`] で差し替えた中身は、次の出す番から効く。
//!
//! # 今風の見た目にする（exe のマニフェスト）
//!
//! ツールチップは OS の標準の部品（comctl32）で出す。今の Windows の見た目は部品の**版 6** でしか
//! 出ない。版 6 を使うかは exe に埋めたマニフェストで決まり、ライブラリの
//! wintf からは選べないので、**wintf を使う exe の側で申告する**。申告しないと古い版（版 5）で動き、
//! 出す・消す・折り返しは同じように働くが、見た目と大きさが古いものになる。
//!
//! マニフェスト（例えば `app.manifest` として、パッケージの `Cargo.toml` の隣に置く）:
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
//! <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
//!   <dependency>
//!     <dependentAssembly>
//!       <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
//!         processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
//!     </dependentAssembly>
//!   </dependency>
//! </assembly>
//! ```
//!
//! それをリンカへ渡す `build.rs`（`Cargo.toml` を変える必要は無い。MSVC のリンカのときだけ渡す）:
//!
//! ```no_run
//! // build.rs の main の中身
//! println!("cargo:rerun-if-changed=app.manifest");
//! println!("cargo:rerun-if-changed=build.rs");
//! if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
//!     return;
//! }
//! let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
//!     .join("app.manifest");
//! // bin の exe にだけ渡す（サンプルなら rustc-link-arg-examples）。
//! println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
//! println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());
//! ```
//!
//! 既にマニフェストを埋めている exe なら、その中の `<dependency>` に上の 1 項目を足せばよい。
//!
//! # 既知の限界
//!
//! - wintf の窓ではない所（別のアプリの窓の上や、ツールチップの下が別のアプリのとき）で、ボタンを
//!   すぐ離す短い押下は拾えないことがある（ツールチップが出ている間、100 ミリ秒ごとに見回る瞬間に
//!   押されていれば拾える）。その場合ツールチップは、マウスが安全地帯を出るまで残る。
//! - 画面の更新の最中に届いて捨てられたボタンの押下は、判定の時点でまだ押されていれば拾えるが、
//!   既に離されていれば拾えない。
//! - マウスを動かさないまま別のアプリの窓が上に重なった場合、OS がマウスの離脱を知らせるまで
//!   「範囲に入っている」のままになる。
//! - wintf の窓の外で安全地帯から出たことは、最大で 100 ミリ秒遅れて拾う。
//! - 知らせの外で [`supply_text`] を呼んで初めて出すとき、位置は最後の判定の時のマウスの位置で、
//!   出す番の間の判定は 100 ミリ秒ごとなので、その分だけ古いことがある。
//! - 後から最前面へ出し直した別のいつも手前の窓は、ツールチップを一時的に覆いうる。出ている間は
//!   100 ミリ秒ごとにツールチップを最前面へ戻すので、覆われるのは長くてもその間だけ。
//!
//! # 記録
//!
//! `tracing` で記録する（記録の対象はこのモジュールの道 `wintf::ecs::tooltip` の下）。渡した文字の
//! 本文は載せず、文字数だけを載せる。
//!
//! - `debug`: `[tooltip_shown]`（出した）・`[tooltip_hidden]`（消した・理由付き）・
//!   `[tooltip_supply_stale]`（終わった印で渡された）。
//! - `trace`: `[tooltip_turn]`（入った・数え始め・使った待ち時間・来た・来なかった理由・終わりの理由）。
//! - `warn`: `[tooltip_show_failed]`（出せなかった）・`[tooltip_hover_time_unreadable]`（待ち時間の
//!   設定を読めなかった。1 回だけ）・`[tooltip_monitor_unreadable]`（画面の情報を読めなかった）。
//!
//! 判定の分岐まで見るには `RUST_LOG=wintf::ecs::tooltip=trace` のように開ける。

mod geometry;
mod os;
mod ranges;
mod system;
mod turn;

pub use os::TooltipOsError;
pub use ranges::{TooltipArea, TooltipRange, TooltipRangeId};
pub use turn::{TooltipEndReason, TooltipTurnToken};

pub(crate) use system::note_button_press;

use crate::ecs::{PointF, Window};
use bevy_ecs::prelude::*;
use std::time::Instant;

/// 出す番が来た知らせの中身。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipTurn {
    pub window: Entity,
    pub range: TooltipRangeId,
    /// 知らせを出した時のマウスの位置（窓の中・論理の単位）。
    pub position: PointF,
    pub token: TooltipTurnToken,
    /// その範囲に文字が預けてあるか（真なら既に出ている）。
    pub has_text: bool,
}

/// 知らせの種類（来た・終わった）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipNotice {
    TurnStarted(TooltipTurn),
    TurnEnded {
        window: Entity,
        range: TooltipRangeId,
        token: TooltipTurnToken,
        reason: TooltipEndReason,
    },
}

/// 窓に付ける、知らせを受ける関数。World を借りたまま同期で呼ばれる。
#[derive(Component, Clone, Copy)]
pub struct OnTooltip(pub fn(world: &mut World, notice: &TooltipNotice));

/// 文字を渡した・消した結果。
#[non_exhaustive]
#[derive(Debug)]
pub enum TooltipSupply {
    /// 出した（または出ていた表示を置き換えた）。
    Shown,
    /// 文字が空だったので出さなかった（出ていれば消した）。
    Cleared,
    /// 印の出す番は既に終わっていた。何も出していない。
    StaleTurn,
    /// OS の側の失敗で出せなかった。出す番は続いている。
    Failed(TooltipOsError),
}

/// 範囲の登録の誤り。
#[derive(Debug, thiserror::Error)]
pub enum TooltipRegisterError {
    #[error("窓のエンティティが無い")]
    NoSuchWindow,
}

/// 範囲を登録する（後から登録したものが、重なった所で勝つ）。
///
/// `window` は [`Window`] を持つエンティティ（まだ窓のハンドルが無くてもよい）。
/// 無ければ [`TooltipRegisterError::NoSuchWindow`]。
pub fn register(
    world: &mut World,
    window: Entity,
    range: TooltipRange,
) -> Result<TooltipRangeId, TooltipRegisterError> {
    let mut e = world
        .get_entity_mut(window)
        .ok()
        .filter(|e| e.contains::<Window>())
        .ok_or(TooltipRegisterError::NoSuchWindow)?;
    let id = match e.get_mut::<ranges::TooltipRanges>() {
        Some(mut table) => table.add(window, range),
        None => {
            let mut table = ranges::TooltipRanges::default();
            let id = table.add(window, range);
            e.insert(table);
            id
        }
    };
    wake_next_frame();
    Ok(id)
}

/// 登録の中身を差し替える（重なりの順は変えない・次の出す番から効く）。無ければ偽。
pub fn update(world: &mut World, id: TooltipRangeId, range: TooltipRange) -> bool {
    let replaced = world
        .get_mut::<ranges::TooltipRanges>(id.window())
        .is_some_and(|mut table| table.replace(id, range));
    if replaced {
        wake_next_frame();
    }
    replaced
}

/// 登録を取り消す（続いている出す番があれば、戻る前にツールチップを消して終わりを知らせる）。
/// 無ければ偽。
pub fn unregister(world: &mut World, id: TooltipRangeId) -> bool {
    system::with_os_tip(|tip| system::unregister_with(world, id, Instant::now(), tip))
}

/// 続いている出す番に文字を渡す。その出す番で初めて出した位置に出す（まだ出していなければ、
/// 最後の判定の時のマウスの位置。出す番の間の判定は 100 ミリ秒ごとなので、知らせの外で渡すと
/// その分だけ古いことがある）。印の出す番が終わっていれば [`TooltipSupply::StaleTurn`]。
pub fn supply_text(world: &mut World, token: TooltipTurnToken, text: &str) -> TooltipSupply {
    system::with_os_tip(|tip| system::supply_text_with(world, token, text, Instant::now(), tip))
}

/// 続いている出す番のツールチップを消す（出す番は続く）。印が続いていれば真。
pub fn dismiss(world: &mut World, token: TooltipTurnToken) -> bool {
    system::with_os_tip(|tip| system::dismiss_with(world, token, Instant::now(), tip))
}

/// 判定の資源を置き、画面更新の末尾（`FrameFinalize`）に判定を足す。`EcsWorld` の作成が 1 回呼ぶ。
pub(crate) fn install(world: &mut World) {
    world.insert_non_send(system::TooltipSession::default());
    world
        .resource_mut::<Schedules>()
        .add_systems(crate::ecs::world::FrameFinalize, system::tooltip_frame);
}

/// マウスが動かないままでも、次の画面更新で判定が回るよう期限を預ける。
fn wake_next_frame() {
    crate::ecs::world::tick_wake::arm_deadline(Instant::now());
}

#[cfg(test)]
#[path = "system_apply_tests.rs"]
mod system_apply_tests;
#[cfg(test)]
#[path = "system_tests.rs"]
mod system_tests;
