//! 表示の記録（`display::record_display`）が失敗したときに **presenter 側の状態が 1 つも書き換わらない**
//! ことを、**4 注入点 × 3 場面**で固定する実行テスト（要件 7.1／7.2・設計 Flow 1・付録 A′ #19）。
//!
//! # 前身との違い（`presenter_upload_failure_tests.rs` からの導き直し）
//!
//! 前身は自前 swap chain 供給面の `upload` 7 点を注入し、presenter 側 5 項目の保持を見ていた。供給面が
//! 消えて失敗点が「記録」の 4 点（`CreateBitmap`／`CreateCommandList`／`EndDraw`／`Close`）になり、その
//! 位置が**容量回収（`take_recycled`）より手前**へ移った。手前になったことで保たれる範囲が広がる——
//! 失敗した適用は合成メモを 1 件も増やさず、追い出しも起こさない。ゆえに前状態の集合に**メモの件数**
//! （と表示中エントリのバイト）が加わる。
//!
//! # 何を「前の状態」と呼ぶか（design Flow 1「記録の失敗は回収より手前」）
//!
//! [`PresenterState`] の 8 項目である——`visible`／`applied`／`native_size`／`current_surface`／
//! `pending_resize`／メモの件数と表示中エントリのバイト／枠の面 entity の `GraphicsCommandList`
//! （＝画面へ渡っている描画命令）／同 entity の `Arrangement`（配置）と `AlphaMaskResource`（さわり判定）
//! ——に加えて、呼出元へ `reply` が `Err(PresentError::Device)` で返り、`error!` が**ちょうど 1 行**出ること。
//! 判定は「注入前に採った値と厳密に等しい」の 1 本である。
//!
//! # 3 場面（それぞれ別の項目に判別力がある）
//!
//! - **初回**: 表示が一度も成立していない。成功していれば装着・配置・マスク・メモが**無から生じる**。
//!   ここだけが「失敗した適用が装着を作らない」を見られる場面である。
//! - **同形再表示**: 表示確立後に、**同じ外形の別の面**を別の窓 DPI で試す。成功していれば面 id・
//!   表示記録・マスク・メモ件数・k・配置の係数・窓寸要求が同時に動く。外形が同じでも中身（α の穴）が
//!   違う面を使うので、マスクの不変は恒真ではない。
//! - **Hide 後**: 非表示にしてから試す。ここでだけ `visible`／`current_surface` に判別力がある
//!   （成功していれば可視へ戻り面 id が入る）。
//!
//! いずれの場面も**注入前の具体値を明示的に assert してから**注入し、末尾に**陽性対照**（同じ指令を
//! 注入なしで流すと当該項目が実際に動く）を置く。これが無いと「もともと何も動かない場面を見ていた」
//! 恒真の檻と区別が付かない。
//!
//! # 前提
//!
//! 既存のグラフィクステストと同一——**窓なし・実 D3D デバイス**——であり、実機 GPU 障害の再現を
//! 必要としない。注入点は `#[cfg(test)]` でのみ実体を持つ（`display.rs`）。`EndDraw`／`Close` の注入は
//! 実呼び出しの**後**に置かれているため、ここで主張するのは呼び手から見える前状態の維持だけである
//! （共有 DC 側に副作用が残らないことは `display_fault_tests.rs` が見る）。

use super::*;

use std::path::Path;
use std::time::Duration;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};

use wintf::ecs::widget::bitmap_source::AlphaMask;
use wintf::ecs::{AlphaMaskResource, Arrangement, GraphicsCommandList, Offset, Visual};

use crate::display::{DisplayFault, arm_display_fault, clear_display_fault};

use super::test_support::{
    CapturedEvent, capture, elem, make_world_with_gpu, mount_entities, set_window_dpi, shell_of,
    show_ok, spawn_window_with_dpi, surface,
};

/// 検査対象の target（全テストで 1 つ）。
const TARGET: TargetId = TargetId(0);
/// 表示を確立する面（＝前の状態を作る面）。
const ESTABLISHED_ID: u32 = 1000;
/// 失敗させる `ShowSurface` が指す面（確立済みの面とは別 id・**同じ外形で別のα**）。
const ATTEMPTED_ID: u32 = 3000;
/// 2 面に共通の native 外形（同形＝外形の変化に頼らずに前状態維持を見る）。
const FACE_SIZE: (u32, u32) = (4, 3);
/// 表示確立時の窓 DPI（author_dpi と同値＝k は恒等）。
const AUTHOR_DPI: u16 = 96;
/// 注入直前に切り替える窓 DPI（k=2/1＝成功していれば `applied`・配置の係数・窓寸要求が動く）。
const MOVED_DPI: u16 = 192;
/// 注入前に枠の面 entity の `Arrangement.offset` へ仕込む番兵。
///
/// `mount.set_layout` は現値と異なるとき `logical_arrangement`（**offset は常に 0**）を丸ごと書く。
/// 本番の正常経路がこの値を作ることは無いので、注入後に残っていれば `set_layout` は 1 度も呼ばれて
/// いない——k が動かない変異（配置だけを書き直す形）でも呼出そのものを検出できる。
const SENTINEL_OFFSET: (f32, f32) = (99.0, 77.0);

/// `record_display` が失敗し得る 4 点（`DisplayFault` の全 variant）。
///
/// variant が増えたらここに足さなければ被覆が黙って減る。列挙の網羅は [`fault_marker`] の `match` が
/// コンパイルで、本表の重複と件数は [`the_matrix_covers_four_fault_points`] が実行時に見張る。
const ALL_DISPLAY_FAULTS: [DisplayFault; 4] = [
    DisplayFault::CreateBitmap,
    DisplayFault::CreateCommandList,
    DisplayFault::EndDraw,
    DisplayFault::Close,
];

/// 注入が載せる HRESULT（`display.rs` の `fault_point` と同値＝E_FAIL）。
const INJECTED_HRESULT: i32 = 0x8000_4005u32 as i32;

/// 注入点が文脈文字列へ残す字面（`display.rs` の `injected_context` は `<injected:{at:?}>`）。
///
/// 4 つの定数を写し取らず variant 名だけを持つのは、`match` の網羅で「variant が増えたら
/// コンパイルが落ちる」を得つつ、文脈文字列の綴りの正本を `display.rs` 側 1 箇所に保つためである。
fn fault_marker(at: DisplayFault) -> &'static str {
    match at {
        DisplayFault::CreateBitmap => "CreateBitmap",
        DisplayFault::CreateCommandList => "CreateCommandList",
        DisplayFault::EndDraw => "EndDraw",
        DisplayFault::Close => "Close",
    }
}

// ── フィクスチャ（同形・別α の 2 面）──────────────────────────────────

/// 面 1000（全不透明）と面 3000（**内側に α=0 の穴**）の `(EmoWorld, AtlasTable)` を返す。
///
/// 穴は内側の 2 画素だけで、外周は全て不透明である。α=0 除外トリムは外周を残すので**合成外形は
/// 両面とも `FACE_SIZE`**——外形を変えずに、表示バイトとさわり判定マスクの双方を弁別可能にできる。
/// 共有ヘルパ `build_two_face_assets` の 2 面は**どちらも全不透明**でマスクが同一になるため、
/// 「マスクが前値のまま」の主張が恒真になってしまう。ここで専用の fixture が要る唯一の理由がそれで、
/// 作り方（atlas bake → `EmoWorld` → `bind_atlas`）は共有ヘルパと同一である。
fn build_two_face_assets_with_hole() -> (EmoWorld, AtlasTable) {
    let (w, h) = FACE_SIZE;
    let base = Path::new("shell/master");
    let surfaces = vec![
        surface(ESTABLISHED_ID, vec![elem("p.png", 0, 0)]),
        surface(ATTEMPTED_ID, vec![elem("q.png", 0, 0)]),
    ];

    let gradient = |salt: u8, hole: bool| -> Vec<u8> {
        let mut img: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                // 内側（外周でない画素）だけを透明にする＝トリム後の外形は変わらない。
                let inner = x > 0 && y > 0 && x + 1 < w && y + 1 < h;
                if hole && inner {
                    img.extend_from_slice(&[0, 0, 0, 0]);
                    continue;
                }
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.extend_from_slice(&[b, g, r, 0xFF]);
            }
        }
        img
    };

    let mut dec = MemoryDecoder::new();
    dec.insert(base.join("p.png"), w, h, w * 4, gradient(0x11, false), true);
    dec.insert(base.join("q.png"), w, h, w * 4, gradient(0x77, true), true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない: {:?}",
        baked.errors
    );

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

// ── 観測（前状態 8 項目）────────────────────────────────────────────────

/// presenter 側の「前の状態」のスナップショット。
///
/// 私有フィールド（`native_size`／`pending_resize`／`cache`）を直に読むのは in-source テストの特権で
/// ある。`take_pending_resize` は**取り出して消す**ので観測には使えない（観測が状態を壊す）。
#[derive(Debug, Clone, PartialEq)]
struct PresenterState {
    /// `PresentTarget.visible`（可視性の単一真実源）。
    visible: Option<bool>,
    /// `PresentTarget.applied`（実適用 k の単一真実源）。
    applied: Option<ScaleRatio>,
    /// `PresentTarget.native_size`（表示中エントリの原寸）。
    native_size: Option<(u32, u32)>,
    /// `PresentTarget.current_surface_id`（最後に確立した面 id）。
    current_surface: Option<u32>,
    /// `PresentTarget.pending_resize`（未消費の窓寸 reconcile 要求）。
    pending_resize: Option<(u32, u32)>,
    /// 合成メモの件数（失敗点が回収より手前へ移ったことで加わった項目）。
    entries: usize,
    /// 表示中エントリの原寸バイト（`read_back`・未表示なら `None`）。
    memo_bytes: Option<Vec<u8>>,
    /// 枠の面 entity の表示記録＝画面へ渡っている描画命令（未装着なら `None`）。
    display: Option<GraphicsCommandList>,
    /// 枠の面 entity の配置（offset＋論理寸＋係数 k・未装着なら `None`）。
    arrangement: Option<Arrangement>,
    /// 枠の面 entity へ供給済みのαマスク（未装着・未供給なら `None`）。
    mask: Option<AlphaMask>,
}

/// 装着済みなら枠の面 entity（未装着なら `None`）。
///
/// 共有補助 `mount_entities` は「表示成立後」を前提に `expect` するので、**装着が無い場面**（初回の
/// 失敗）を観測できない。ここは装着の有無そのものが観測対象ゆえ `Option` で読む。
fn surface_entity_of(presenter: &EmoPresenter, target: TargetId) -> Option<Entity> {
    Some(
        presenter
            .targets
            .get(&target)?
            .mount
            .as_ref()?
            .surface_entity(),
    )
}

/// 合成メモの件数。
///
/// `ComposeCache::len` は `cache.rs` 私有の `#[cfg(test)]` 観測口で、ここからは届かない。製品側へ
/// 数える口を足さない代わりに、**本ファイルが使う 2 つのキー**を `get`（最近使用順を動かさない）で
/// 引いて数える。この 2 つ以外のキーは本ファイルでは一度も挿入されないので、これは近似ではなく
/// 実数である。
fn entry_count(presenter: &EmoPresenter, target: TargetId) -> usize {
    let Some(t) = presenter.targets.get(&target) else {
        return 0;
    };
    [ESTABLISHED_ID, ATTEMPTED_ID]
        .into_iter()
        .filter(|id| {
            t.cache
                .get(*id, &BindSet::default(), &PatternState::default())
                .is_some()
        })
        .count()
}

/// 前状態を 1 度に採る（判定は「注入前と厳密に等しい」の 1 本にする）。
fn snapshot(presenter: &EmoPresenter, world: &World, target: TargetId) -> PresenterState {
    let entity = surface_entity_of(presenter, target);
    let t = presenter.targets.get(&target);
    PresenterState {
        visible: presenter.target_visible(target),
        applied: presenter.applied_ratio(target),
        native_size: t.and_then(|t| t.native_size),
        current_surface: presenter.current_surface_id(target),
        pending_resize: t.and_then(|t| t.pending_resize),
        entries: entry_count(presenter, target),
        memo_bytes: presenter.read_back(target).ok(),
        display: entity.and_then(|e| world.get::<GraphicsCommandList>(e).cloned()),
        arrangement: entity.and_then(|e| world.get::<Arrangement>(e).copied()),
        mask: entity
            .and_then(|e| world.get::<AlphaMaskResource>(e))
            .and_then(|r| r.mask().cloned()),
    }
}

/// 枠の面 entity の `Arrangement.offset` へ番兵を仕込む（注入前に 1 度だけ）。
///
/// 本番コードには触れない——`Arrangement` は world の component であり、テストから直接書ける。
fn plant_sentinel_offset(presenter: &EmoPresenter, world: &mut World, target: TargetId) {
    let entity = surface_entity_of(presenter, target).expect("表示確立後は装着済み");
    let mut arr = world
        .get_mut::<Arrangement>(entity)
        .expect("表示確立後は枠の面 entity に Arrangement がある");
    arr.offset = Offset {
        x: SENTINEL_OFFSET.0,
        y: SENTINEL_OFFSET.1,
    };
}

// ── 注入（武装と解除を 1 つの不可分な操作に閉じる）──────────────────────

/// 注入 → `ShowSurface` → 武装解除 を **1 つの不可分な操作**として閉じる唯一の入口。
///
/// `display.rs` の `fault_point` は**一致した点でしか武装を解かない**ので、届かなかった注入は旗が
/// 立ったまま残り、同一スレッドの後続の記録で発火する。解除を `Drop` に持たせ `arm_display_fault` の
/// 呼出を本関数の内側 1 箇所だけに限ることで、「解除の書き忘れ」も「assert の panic で解除を飛び越す
/// こと」も**構造的に起こり得ない**（`display_fault_tests.rs` の `record_with_armed_fault` と同じ規律）。
fn show_with_armed_fault(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
    at: DisplayFault,
) -> (PresentError, Vec<CapturedEvent>) {
    /// 生存期間の終わり（正常終了・panic による巻き戻しの双方）で必ず武装を降ろす番人。
    struct Disarm;
    impl Drop for Disarm {
        fn drop(&mut self) {
            clear_display_fault();
        }
    }

    let _disarm = Disarm;
    arm_display_fault(at);

    let (outcome, events) = capture(|| {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        rx.recv_timeout(Duration::from_secs(10))
    });

    let err = match outcome {
        // 要件 7.1／7.2: 呼出元へ失敗が返る（`display.rs` が `device_err` 経由で error! 済み）。
        Ok(Err(e)) => e,
        Ok(Ok(())) => panic!(
            "注入した失敗点 {at:?} で ShowSurface が Ok を返した（注入が届いていない＝前状態維持を判定できない）"
        ),
        Err(e) => panic!("reply（ShowSurface）を受信できない: {e}"),
    };
    (err, events)
}

/// 注入の失敗が本番と同じ形（`PresentError::Device` ＋ 注入点の文脈 ＋ E_FAIL）で返り、
/// `error!` が**ちょうど 1 行**出ていること（要件 7.1／7.2: ログ無しで縮退しない・二重に鳴らさない）。
fn assert_injected_failure(err: &PresentError, events: &[CapturedEvent], at: DisplayFault) {
    match err {
        PresentError::Device { hresult, context } => {
            assert!(
                context.contains(fault_marker(at)),
                "文脈文字列が注入点を指していない（{at:?}・実測 {context}）"
            );
            assert_eq!(
                *hresult, INJECTED_HRESULT,
                "注入は E_FAIL を載せる（{at:?}）"
            );
        }
        other => {
            panic!("注入の失敗は PresentError::Device のはずだが {other:?} が返った（{at:?}）")
        }
    }
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "注入 1 回につき error! はちょうど 1 行（{at:?}・実測 {} 行）: {errors:?}",
        errors.len()
    );
}

// ── 場面の組み立て ─────────────────────────────────────────────────────

/// 同形 2 面の world を載せた target を 1 つ装着する（**まだ表示は成立していない**）。
fn attached_presenter(world: &mut World, dpi: u16) -> (EmoPresenter, Entity) {
    let window = spawn_window_with_dpi(world, dpi);
    let (emo_world, atlas) = build_two_face_assets_with_hole();

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(world, TARGET, window, emo_world, atlas, AUTHOR_DPI)
        .expect("attach_target 失敗");
    (presenter, window)
}

/// 面 1000 で表示を確立し、窓 DPI を動かして番兵を仕込んだ状態（同形再表示・Hide 後 の共通前段）。
fn established_then_dpi_moved(world: &mut World) -> EmoPresenter {
    let (mut presenter, window) = attached_presenter(world, AUTHOR_DPI);
    // ここが「前の状態」——表示を一度確立しなければ保持の主張は恒真になる。
    show_ok(&mut presenter, world, TARGET, ESTABLISHED_ID);
    set_window_dpi(world, window, MOVED_DPI);
    plant_sentinel_offset(&presenter, world, TARGET);
    presenter
}

/// 確立済み場面の前提（成功していれば動く項目が、注入前に**具体値**で在ること）。
fn assert_established_baseline(before: &PresenterState, visible: bool, current: Option<u32>) {
    assert_eq!(before.visible, Some(visible), "前提: visible が想定と違う");
    assert_eq!(
        before.current_surface, current,
        "前提: 現サーフェスが想定と違う"
    );
    assert!(
        before
            .applied
            .expect("前提: 表示確立後は applied が確定している")
            .is_identity(),
        "前提: 確立時の k は恒等（窓 DPI ＝ author_dpi）"
    );
    assert_eq!(
        before.native_size,
        Some(FACE_SIZE),
        "前提: native_size が確立した面の原寸でない"
    );
    assert_eq!(before.entries, 1, "前提: メモは確立した 1 件だけ");
    assert!(
        before
            .display
            .as_ref()
            .is_some_and(|d| *d != GraphicsCommandList::empty()),
        "前提: 確立で表示記録が枠の面 entity へ載っている"
    );
    let arr = before.arrangement.expect("前提: 装着済みなら配置がある");
    assert_eq!(
        (arr.offset.x, arr.offset.y),
        SENTINEL_OFFSET,
        "前提: 番兵 offset が仕込まれている（set_layout の呼出を検出できる形）"
    );
    assert_eq!(
        (arr.size.width as u32, arr.size.height as u32),
        FACE_SIZE,
        "前提: 配置の論理寸は原寸"
    );
    assert_eq!(arr.scale.x, 1.0, "前提: 確立時の係数は等倍");
    assert!(before.mask.is_some(), "前提: 確立でαマスクが供給済み");
}

/// `Hide` を適用し、reply が `Ok(())` であることを確認する。
fn hide_ok(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::Hide {
            target,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "Hide が Ok でない"
    );
}

// ── 場面 ⑴ 初回（装着・メモが無から生じる場面）──────────────────────────

/// 要件 7.1／7.2: **初回の適用**が 4 注入点のどこで失敗しても、装着も配置もマスクもメモも生じない。
///
/// この場面でだけ「失敗した適用が装着を作らない」が見られる（他の 2 場面は装着済みから始まる）。
/// 失敗が回収（`take_recycled`）より手前にあることの直接の帰結として、メモの件数は 0 のままである。
#[test]
fn display_failure_on_the_first_apply_creates_no_mount_no_memo_and_no_state() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _window) = attached_presenter(&mut world, MOVED_DPI);

    let before = snapshot(&presenter, &world, TARGET);
    assert_eq!(
        before,
        PresenterState {
            visible: Some(false),
            applied: None,
            native_size: None,
            current_surface: None,
            pending_resize: None,
            entries: 0,
            memo_bytes: None,
            display: None,
            arrangement: None,
            mask: None,
        },
        "前提: attach しただけの target は何も確定していない"
    );

    for at in ALL_DISPLAY_FAULTS {
        let (err, events) =
            show_with_armed_fault(&mut presenter, &mut world, TARGET, ESTABLISHED_ID, at);
        assert_injected_failure(&err, &events, at);
        assert_eq!(
            snapshot(&presenter, &world, TARGET),
            before,
            "{at:?} の失敗で初回適用が状態を作った（装着・メモ・照会値のいずれかが動いた）"
        );
    }

    // ── 陽性対照: 注入なしなら同じ指令が全項目を実際に立ち上げる ──────────
    show_ok(&mut presenter, &mut world, TARGET, ESTABLISHED_ID);
    let after = snapshot(&presenter, &world, TARGET);
    assert_eq!(
        after.current_surface,
        Some(ESTABLISHED_ID),
        "陽性対照: 成功した初回 ShowSurface は現サーフェスを立てる"
    );
    assert_eq!(after.native_size, Some(FACE_SIZE));
    assert_eq!(
        after.applied,
        Some(ScaleRatio::new(2, 1).expect("非ゼロ比")),
        "陽性対照: 窓 192／author 96 の初回成立は k=2/1"
    );
    assert_eq!(
        after.pending_resize,
        Some((FACE_SIZE.0 * 2, FACE_SIZE.1 * 2)),
        "陽性対照: 初回成立は窓寸 reconcile 要求を積む"
    );
    assert_eq!(after.entries, 1, "陽性対照: メモが 1 件生じる");
    assert!(
        after.display.is_some() && after.arrangement.is_some() && after.mask.is_some(),
        "陽性対照: 成功した初回 ShowSurface は装着・表示記録・配置・マスクを作る"
    );
}

// ── 場面 ⑵ 同形再表示（外形が変わらない別の面）────────────────────────────

/// 要件 7.1／7.2: 表示確立後に**同形の別の面**を新しい k で試して失敗しても、前状態が 1 つも動かない。
///
/// 成功していれば面 id・表示記録・マスク・メモ件数・k・配置の係数・窓寸要求が同時に動く場面である。
/// 2 面は同じ外形で**α の穴だけが違う**ので、表示バイトとマスクの不変は恒真ではない。
#[test]
fn display_failure_on_a_same_shape_reshow_keeps_every_previous_value() {
    let mut world = make_world_with_gpu();
    let mut presenter = established_then_dpi_moved(&mut world);

    let before = snapshot(&presenter, &world, TARGET);
    assert_established_baseline(&before, true, Some(ESTABLISHED_ID));

    for at in ALL_DISPLAY_FAULTS {
        let (err, events) =
            show_with_armed_fault(&mut presenter, &mut world, TARGET, ATTEMPTED_ID, at);
        assert_injected_failure(&err, &events, at);
        assert_eq!(
            snapshot(&presenter, &world, TARGET),
            before,
            "{at:?} の失敗で presenter 側の前状態が書き換わった"
        );
    }

    // ── 陽性対照 ───────────────────────────────────────────────────────
    show_ok(&mut presenter, &mut world, TARGET, ATTEMPTED_ID);
    let after = snapshot(&presenter, &world, TARGET);
    assert_eq!(
        after.current_surface,
        Some(ATTEMPTED_ID),
        "陽性対照: 成功した再表示は現サーフェスを動かす"
    );
    assert_eq!(
        after.entries, 2,
        "陽性対照: 成功した再表示はメモを 1 件足す"
    );
    assert_ne!(
        after.memo_bytes, before.memo_bytes,
        "陽性対照: 成功した再表示は表示中エントリのバイトを入れ替える"
    );
    assert_ne!(
        after.display, before.display,
        "陽性対照: 成功した再表示は表示記録を差し替える"
    );
    assert_ne!(
        after.mask, before.mask,
        "陽性対照: 成功した再表示はαマスクを差し替える（同形でもα が違う）"
    );
    assert_eq!(
        after.applied,
        Some(ScaleRatio::new(2, 1).expect("非ゼロ比")),
        "陽性対照: 窓 DPI を動かした後の成功は k を恒等から動かす"
    );
    let arr = after.arrangement.expect("陽性対照: 装着済み");
    assert_eq!(
        (arr.offset.x, arr.offset.y),
        (0.0, 0.0),
        "陽性対照: 成功した再表示は set_layout を通る（番兵が消える）"
    );
    assert_eq!(
        (arr.size.width as u32, arr.size.height as u32),
        FACE_SIZE,
        "陽性対照: 配置の論理寸は原寸のまま（k は寸へ焼かない）"
    );
    assert_eq!(arr.scale.x, 2.0, "陽性対照: 配置の係数が新しい k へ動く");
    assert_eq!(
        after.pending_resize,
        Some((FACE_SIZE.0 * 2, FACE_SIZE.1 * 2)),
        "陽性対照: 物理寸の変化が窓寸 reconcile 要求として積まれる"
    );
}

// ── 場面 ⑶ Hide 後（`visible` に判別力がある場面）───────────────────────

/// 要件 7.1／7.2: `Hide` 後に失敗した `ShowSurface` は target を**可視へ戻さない**（前状態すべて不変）。
///
/// この場面でだけ `visible`／`current_surface` が判別力を持つ——成功していれば `visible` は true へ、
/// `current_surface` は `Some(3000)` へ動く。entity 側（枠の面・文字層スロットの `Visual`）も併せて
/// 見るのは、「照会は false を返すが entity は可視」という食い違いを見逃さないためである。
#[test]
fn display_failure_after_hide_leaves_the_target_hidden_and_every_previous_value_intact() {
    let mut world = make_world_with_gpu();
    let (mut presenter, window) = attached_presenter(&mut world, AUTHOR_DPI);
    show_ok(&mut presenter, &mut world, TARGET, ESTABLISHED_ID);
    hide_ok(&mut presenter, &mut world, TARGET);
    set_window_dpi(&mut world, window, MOVED_DPI);
    plant_sentinel_offset(&presenter, &mut world, TARGET);

    let before = snapshot(&presenter, &world, TARGET);
    // `Hide` は可視性と現サーフェスだけを落とす（k・原寸・配置・マスク・メモは保持される）。
    assert_established_baseline(&before, false, None);

    let (surface_entity, slot) = mount_entities(&presenter, TARGET);
    for at in ALL_DISPLAY_FAULTS {
        let (err, events) =
            show_with_armed_fault(&mut presenter, &mut world, TARGET, ATTEMPTED_ID, at);
        assert_injected_failure(&err, &events, at);
        assert_eq!(
            snapshot(&presenter, &world, TARGET),
            before,
            "{at:?} の失敗が不可視 target の前状態を書き換えた"
        );
        assert!(
            !world
                .get::<Visual>(surface_entity)
                .expect("surface に Visual")
                .is_visible,
            "{at:?} の失敗が枠の面 entity を可視へ戻した"
        );
        assert!(
            !world
                .get::<Visual>(slot)
                .expect("slot に Visual")
                .is_visible,
            "{at:?} の失敗が文字層スロット entity を可視へ戻した"
        );
    }

    // 陽性対照: 注入なしなら同じ指令が可視へ戻す（＝上の false 維持は判別力を持つ）。
    show_ok(&mut presenter, &mut world, TARGET, ATTEMPTED_ID);
    assert_eq!(
        presenter.target_visible(TARGET),
        Some(true),
        "陽性対照: 成功した ShowSurface は指令駆動 target を可視へ戻す"
    );
    assert_eq!(
        presenter.current_surface_id(TARGET),
        Some(ATTEMPTED_ID),
        "陽性対照: 成功した ShowSurface は現サーフェスを立て直す"
    );
    assert!(
        world
            .get::<Visual>(surface_entity)
            .expect("surface に Visual")
            .is_visible,
        "陽性対照: 成功した ShowSurface は枠の面 entity を可視へ戻す"
    );
}

// ── 被覆の番人 ─────────────────────────────────────────────────────────

/// 4 注入点 × 3 場面の「4」を実行時に固定する（「3」は上の `#[test]` 3 本そのものである）。
///
/// variant の追加は [`fault_marker`] の `match` がコンパイルで捕まえるが、[`ALL_DISPLAY_FAULTS`] から
/// 1 つ落とす／重複させる改変はコンパイルを通ってしまい、被覆だけが静かに減る。
#[test]
fn the_matrix_covers_four_fault_points() {
    assert_eq!(ALL_DISPLAY_FAULTS.len(), 4, "記録が失敗し得るのは 4 点");
    for (i, at) in ALL_DISPLAY_FAULTS.iter().enumerate() {
        assert!(
            !ALL_DISPLAY_FAULTS[..i].contains(at),
            "被覆表に重複がある: {at:?}（4 点は互いに異なる）"
        );
    }
}
