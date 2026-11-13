//! # actor_writing_warn_tests — 書字方向の警告は登録口が書く（結線層・兄弟テスト）
//!
//! spec `emo-text-reveal-incremental` 件 3（要件 3.2〜3.7）。書字方向の宣言の警告と記録を、解決の
//! 内側でなく actor の登録口で数え、件数と文言が今と同じで、同じ宣言を繰り返さないことを
//! 確かめる（`actor_region_warn_tests.rs` と同じ部品と流儀）。
//!
//! [`super`]（`actor.rs`）の子モジュールとして接続してある。
//!
//! ## 固定するもの
//!
//! 1. `writing_decision_tests.rs` が解決を直に呼んで固定していた 11 の宣言の形を、装着の口
//!    （`register_actor_binding`）を通して同じ件数・文言・欄で確かめる（要件 3.2・3.5・3.6）。
//! 2. 不正な宣言で装着した後の再追従（上の段・中の段・下の段）は追加 0 件（要件 3.3）。
//! 3. 宣言の生の値が変わる再追従は、変わったキーだけ新しい値で 1 件（要件 3.3・3.5）。
//! 4. 装着のし直しはもう 1 件・箱の表の受け取りは名前ごとに 1 件・直の登録口は 0 件（要件 3.2・3.4・3.7）。
//!
//! ## 0 件の主張が恒真にならないようにする
//!
//! 捕捉窓は [`capture_checked`] で開く（窓の内側の対照の `error!` が 1 件数えられていることを
//! 件数の主張と同時に確かめる）。
//!
//! ## 決定論
//!
//! 実 DPI モニタ・実 GPU・実フォント・実窓を要さない。文字列の写像と登録口の登録だけで完結する
//! （描画資源は触らない——`present_frame` を呼ばない）。

use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_sakura::contract::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use log_capture_kit::CapturedEvent;

use super::test_support::{box_layout, capture_checked, fake_resolver, spawn_reserved_slot};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding};
use crate::draw::DEFAULT_BALLOON_BACKGROUND;
use crate::state::TextLayerConfig;

/// 不正な `vertical` の警告の文言（`writing.rs` の発行点と 1 文字も違わないこと）。
const INVALID_VERTICAL: &str =
    "vertical の値が 0／1 のいずれでもないため指定なしとして扱う（受理値: 0 / 1）";
/// 未知の `writing_mode` の警告の文言。
const UNKNOWN_WRITING_MODE: &str = "未知の writing_mode 値のため指定なしとして扱う（受理語彙: horizontal_tb / vertical_rl / vertical_lr）";
/// 矛盾の併記の `debug!` の文言。
const CONFLICT: &str = "vertical と writing_mode が異なる書字方向を指すため areka 拡張キー writing_mode を採用する（正典キー vertical は採らない）";

/// バルーンの画像の原寸（本ファイルは寸法を判定に用いない）。
const IMAGE: (u32, u32) = (288, 203);

/// 2 キーだけを持つ単層のバルーン定義（転記層を本番と同じ経路で通す）。
fn model(vertical: Option<&str>, writing_mode: Option<&str>) -> BalloonModel {
    let mut source = String::new();
    if let Some(v) = vertical {
        source.push_str(&format!("vertical,{v}\n"));
    }
    if let Some(w) = writing_mode {
        source.push_str(&format!("writing_mode,{w}\n"));
    }
    let model = parse_str(&source, None);
    assert_eq!(
        (model.vertical_raw(), model.writing_mode()),
        (vertical, writing_mode),
        "前提: 2 キーの生の値がそのまま転記される"
    );
    model
}

/// 窓の中の書字方向の記録（不正値・未知値の WARN と、矛盾の併記の DEBUG）。
#[derive(Debug, Default)]
struct Notes {
    invalid_vertical: Vec<CapturedEvent>,
    unknown_writing_mode: Vec<CapturedEvent>,
    conflicts: Vec<CapturedEvent>,
    /// 窓の中の WARN の総数（書字方向の 2 種のほかの WARN が混ざっていないことを見る）。
    warns: usize,
}

impl Notes {
    fn counts(&self) -> (usize, usize, usize) {
        (
            self.invalid_vertical.len(),
            self.unknown_writing_mode.len(),
            self.conflicts.len(),
        )
    }
}

/// 捕捉窓の中で `f` を走らせ、書字方向の記録を種類ごとに選り分けて返す。
fn noting<R>(f: impl FnOnce() -> R) -> (R, Notes) {
    let (value, events) = capture_checked(f);
    let mut notes = Notes::default();
    for e in events {
        if e.level == tracing::Level::WARN {
            notes.warns += 1;
        }
        match (e.level, e.message()) {
            (tracing::Level::WARN, INVALID_VERTICAL) => notes.invalid_vertical.push(e),
            (tracing::Level::WARN, UNKNOWN_WRITING_MODE) => notes.unknown_writing_mode.push(e),
            (tracing::Level::DEBUG, CONFLICT) => notes.conflicts.push(e),
            _ => {}
        }
    }
    (value, notes)
}

struct Fixture {
    world: World,
    rt: TextLayerRuntime,
    window: Entity,
    slot: Entity,
    actor: ActorKey,
}

fn fixture() -> Fixture {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    Fixture {
        world,
        rt: TextLayerRuntime::new(TextLayerConfig::default()),
        window,
        slot,
        actor: ActorKey::from("1"),
    }
}

impl Fixture {
    /// 拡大率 `scale` の結線（物理寸は原寸の `scale` 倍）。
    fn binding(&self, scale: f32) -> TextSlotBinding {
        let surface = (
            (IMAGE.0 as f32 * scale) as u32,
            (IMAGE.1 as f32 * scale) as u32,
        );
        TextSlotBinding::new(self.slot, self.window, scale, surface, IMAGE)
    }

    fn attach(&mut self, model: &BalloonModel) {
        let binding = self.binding(1.0);
        self.rt
            .register_actor_binding(self.actor.clone(), binding, model);
    }

    fn refresh(&mut self, scale: f32, model: &BalloonModel) -> bool {
        let binding = self.binding(scale);
        self.rt.refresh_actor_binding(&self.actor, binding, model)
    }
}

/// 装着 1 回の記録（窓の WARN は書字方向の 2 種だけ）。
fn attach_noting(vertical: Option<&str>, writing_mode: Option<&str>) -> Notes {
    let mut f = fixture();
    let m = model(vertical, writing_mode);
    let ((), notes) = noting(|| f.attach(&m));
    assert_eq!(
        notes.warns,
        notes.invalid_vertical.len() + notes.unknown_writing_mode.len(),
        "書字方向のほかの WARN が混ざっていない: {notes:?}"
    );
    notes
}

// ── 1. 装着の件数と文言（要件 3.2・3.5・3.6・11 の宣言の形） ──

/// 不正な `vertical`（`2` と空文字列）の単独宣言は、装着で WARN 1 件・`value` 欄に生の値。
#[test]
fn attaching_invalid_vertical_alone_warns_once_with_the_raw_value() {
    for value in ["2", ""] {
        let notes = attach_noting(Some(value), None);
        assert_eq!(notes.counts(), (1, 0, 0), "vertical {value:?}: {notes:?}");
        assert_eq!(notes.invalid_vertical[0].field_str("value"), Some(value));
    }
}

/// 宣言なし・`vertical` 単独 2 形・`writing_mode` 単独 3 形・一致の併記 2 形は記録 0 件。
#[test]
fn attaching_valid_or_absent_declarations_records_nothing() {
    for (vertical, writing_mode) in [
        (None, None),
        (Some("0"), None),
        (Some("1"), None),
        (None, Some("horizontal_tb")),
        (None, Some("vertical_rl")),
        (None, Some("vertical_lr")),
        (Some("0"), Some("horizontal_tb")),
        (Some("1"), Some("vertical_rl")),
    ] {
        let notes = attach_noting(vertical, writing_mode);
        assert_eq!(
            (notes.counts(), notes.warns),
            ((0, 0, 0), 0),
            "vertical {vertical:?} ＋ writing_mode {writing_mode:?}: {notes:?}"
        );
    }
}

/// 矛盾の併記 3 形は DEBUG ちょうど 1 件・WARN 0 件。欄は両キーの生の値。
#[test]
fn attaching_conflicting_declarations_records_exactly_one_debug_with_both_raw_values() {
    for (vertical, writing_mode) in [
        ("0", "vertical_rl"),
        ("1", "horizontal_tb"),
        ("1", "vertical_lr"),
    ] {
        let notes = attach_noting(Some(vertical), Some(writing_mode));
        assert_eq!(
            (notes.counts(), notes.warns),
            ((0, 0, 1), 0),
            "vertical,{vertical} ＋ writing_mode,{writing_mode}: {notes:?}"
        );
        let debug = &notes.conflicts[0];
        assert_eq!(debug.field_str("vertical"), Some(vertical));
        assert_eq!(debug.field_str("writing_mode"), Some(writing_mode));
    }
}

/// 未知の `writing_mode` ＋ 有効な `vertical` は WARN 1 件（矛盾の併記ではない）。文言は
/// 実挙動どおり「指定なしとして扱う」を言い、「フォールバック」を言わない。
#[test]
fn attaching_unknown_writing_mode_warns_once_saying_treated_as_undeclared() {
    let notes = attach_noting(Some("1"), Some("diagonal_bt"));
    assert_eq!(notes.counts(), (0, 1, 0), "{notes:?}");
    let warn = &notes.unknown_writing_mode[0];
    assert_eq!(warn.field_str("value"), Some("diagonal_bt"));
    assert!(warn.message().contains("指定なしとして扱う"));
    assert!(!warn.message().contains("フォールバック"));
}

/// 正典の綴りに近いだけの `writing_mode`（区切り・大小・空）も未知として WARN 1 件ずつ（後ろの空白は
/// 設定ファイルの読み取りが落とすので、登録口では正典の値として届く）。
#[test]
fn attaching_near_miss_writing_mode_warns_once_each() {
    for value in ["vertical-rl", "VERTICAL_RL", ""] {
        let notes = attach_noting(Some("1"), Some(value));
        assert_eq!(notes.counts(), (0, 1, 0), "{value:?}: {notes:?}");
        assert_eq!(
            notes.unknown_writing_mode[0].field_str("value"),
            Some(value)
        );
    }
}

/// 不正な `vertical` ＋ 有効な `writing_mode` は WARN 1 件（矛盾の併記ではない）。
#[test]
fn attaching_invalid_vertical_with_valid_writing_mode_warns_once() {
    let notes = attach_noting(Some("2"), Some("vertical_rl"));
    assert_eq!(notes.counts(), (1, 0, 0), "{notes:?}");
}

/// 両キーとも壊れていると WARN 2 件（各キー 1 件ずつ）。
#[test]
fn attaching_with_both_declarations_broken_warns_twice() {
    let notes = attach_noting(Some("2"), Some("diagonal_bt"));
    assert_eq!(notes.counts(), (1, 1, 0), "{notes:?}");
    assert_eq!(notes.invalid_vertical[0].field_str("value"), Some("2"));
    assert_eq!(
        notes.unknown_writing_mode[0].field_str("value"),
        Some("diagonal_bt")
    );
}

// ── 2. 同じ宣言の再追従で繰り返さない（要件 3.3・3.6） ──

/// 不正な宣言で装着した後、上の段（3 回）・中の段（文字の層が読まないキーだけ違う）・下の段（拡大率が違う）の
/// どの再追従でも追加 0 件。
#[test]
fn refresh_with_the_same_broken_declaration_adds_nothing_on_any_tier() {
    let mut f = fixture();
    let broken = model(Some("2"), Some("diagonal_bt"));
    f.attach(&broken);

    let (top, notes) = noting(|| (0..3).map(|_| f.refresh(1.0, &broken)).collect::<Vec<_>>());
    assert_eq!(top, vec![false; 3], "前提: 上の段");
    assert_eq!(
        (notes.counts(), notes.warns),
        ((0, 0, 0), 0),
        "上の段: {notes:?}"
    );

    // 文字の層が読まないキー（`windowposition.x`）だけを足す＝入力は違うが解いた値は同じ。
    let moved = parse_str(
        concat!(
            "vertical,2\n",
            "writing_mode,diagonal_bt\n",
            "windowposition.x,10\n"
        ),
        None,
    );
    assert_ne!(moved, broken, "前提: 入力が違う");
    let (middle, notes) = noting(|| f.refresh(1.0, &moved));
    assert!(!middle, "前提: 中の段");
    assert_eq!(
        (notes.counts(), notes.warns),
        ((0, 0, 0), 0),
        "中の段: {notes:?}"
    );

    let (lower, notes) = noting(|| f.refresh(2.0, &broken));
    assert!(lower, "前提: 下の段（作り直し）");
    assert_eq!(
        (notes.counts(), notes.warns),
        ((0, 0, 0), 0),
        "下の段: {notes:?}"
    );
}

// ── 3. 値の変わる再追従（要件 3.3・3.5） ──

/// `vertical` が `0` → `2`（どちらも横書き＝中の段）で 1 件、`2` → `3` で新しい値で 1 件。
/// `writing_mode` だけが変わったときは `vertical` の警告が出ない（変わったキーの分だけ）。
#[test]
fn refresh_records_only_the_key_whose_raw_value_changed() {
    let mut f = fixture();
    f.attach(&model(Some("0"), None));

    let (changed, notes) = noting(|| f.refresh(1.0, &model(Some("2"), None)));
    assert!(!changed, "前提: 書字方向は同じ＝中の段");
    assert_eq!(notes.counts(), (1, 0, 0), "0 → 2: {notes:?}");
    assert_eq!(notes.invalid_vertical[0].field_str("value"), Some("2"));

    let (_, notes) = noting(|| f.refresh(1.0, &model(Some("3"), None)));
    assert_eq!(notes.counts(), (1, 0, 0), "2 → 3: {notes:?}");
    assert_eq!(notes.invalid_vertical[0].field_str("value"), Some("3"));

    let (_, notes) = noting(|| f.refresh(1.0, &model(Some("3"), Some("diagonal_bt"))));
    assert_eq!(
        notes.counts(),
        (0, 1, 0),
        "writing_mode だけが変わった: {notes:?}"
    );
}

/// 矛盾の併記の生の値が変わる再追従は DEBUG 1 件（新しい値で）、同じ併記の再追従は 0 件。
#[test]
fn refresh_records_one_debug_when_the_conflicting_raw_values_change() {
    let mut f = fixture();
    let conflict = model(Some("0"), Some("vertical_rl"));
    f.attach(&conflict);

    let (_, notes) = noting(|| f.refresh(2.0, &conflict));
    assert_eq!(notes.counts(), (0, 0, 0), "同じ併記: {notes:?}");

    let (_, notes) = noting(|| f.refresh(2.0, &model(Some("1"), Some("horizontal_tb"))));
    assert_eq!((notes.counts(), notes.warns), ((0, 0, 1), 0), "{notes:?}");
    assert_eq!(notes.conflicts[0].field_str("vertical"), Some("1"));
    assert_eq!(
        notes.conflicts[0].field_str("writing_mode"),
        Some("horizontal_tb")
    );
}

// ── 4. 装着のし直し・箱の表・直の登録口（要件 3.2・3.4・3.7） ──

/// 同じ不正な宣言のバルーンで装着をし直す（切り替え）と、前に依らずもう 1 件。
#[test]
fn re_attaching_the_same_broken_balloon_warns_again() {
    let mut f = fixture();
    let broken = model(Some("2"), None);
    f.attach(&broken);
    let ((), notes) = noting(|| f.attach(&broken));
    assert_eq!(notes.counts(), (1, 0, 0), "{notes:?}");
}

/// 不正な `vertical` の箱の名前が 2 つのサーフェスに在っても、受け取り 1 回で 1 件。
/// もう 1 度受け取るとまた 1 件。
#[test]
fn receiving_the_box_table_warns_once_per_box_name() {
    let layout = box_layout(concat!(
        "balloon.a\n{\nsize,100,50\nvertical,2\n}\n",
        "balloon.b\n{\nsize,60,40\n}\n",
        "surface0\n{\nelement1,balloon,a,0,0\nelement2,balloon,b,0,100\n}\n",
        "surface1\n{\nelement1,balloon,a,30,40\n}\n",
    ));
    let mut f = fixture();
    for round in ["1 回目", "2 回目"] {
        let ((), notes) = noting(|| {
            f.rt.set_box_layout(&mut f.world, layout.clone(), fake_resolver(), vec![]);
        });
        assert_eq!(notes.counts(), (1, 0, 0), "{round}: {notes:?}");
        assert_eq!(notes.invalid_vertical[0].field_str("value"), Some("2"));
    }
}

/// 直の登録口 `register_actor`（解決済みの値を受け取る試験の口）は書字方向を記録しない。
#[test]
fn direct_registration_records_no_direction_notes() {
    let mut f = fixture();
    let broken = model(Some("2"), Some("diagonal_bt"));
    let resolved =
        ResolvedBalloonText::resolve_with_background(&broken, IMAGE, DEFAULT_BALLOON_BACKGROUND);
    let binding = f.binding(1.0);
    let ((), notes) = noting(|| f.rt.register_actor(f.actor.clone(), binding, resolved));
    assert_eq!((notes.counts(), notes.warns), ((0, 0, 0), 0), "{notes:?}");
}
