//! 供給面アップロードが失敗したときに **presenter 側の状態が 1 つも書き換わらない**ことを、
//! 表示を一度確立してから失敗を注入する形で固定する実行テスト（要件 5.2・5.4・5.6）。
//!
//! # 何を「前の状態」と呼ぶか（design.md `#### C4` → Responsibilities & Constraints ⒝）
//!
//! presenter 側の前状態は 5 項目である——`visible`／`applied`／`native_size`／`current_surface`／
//! `mount.set_bounds` 未呼出（＝枠の面 entity の `Arrangement` が不変）——に加えて、呼出元へ
//! `reply` が `Err` で返ること。本ファイルの [`PresenterState`] がその 5 項目をそのまま持ち、
//! 判定は「注入前に採った値と厳密に等しい」の 1 本である。
//!
//! 5 項目めの「未呼出」は**値の一致だけでは主張できない**。早期 return が失われた形では
//! `set_bounds` は呼ばれるが、`chain.upload` の commit が失敗し得る操作より後ろにあるため
//! `chain.size()` は旧寸のまま——同じ寸が書き直されるだけで `Arrangement.size` は動かない。
//! そこで注入前に `Arrangement.offset` へ番兵 [`SENTINEL_OFFSET`] を仕込む。`set_bounds` は
//! size 引数に関わらず offset を `(0.0, 0.0)` へ**無条件で**書く（`mount.rs` の
//! `arr.offset = Offset { x: 0.0, y: 0.0 }`）ので、番兵が残っていることが「1 度も呼ばれて
//! いない」ことの直接の証拠になる。本番側に口を足していない（要件 5.6 に無関係）。
//!
//! # 表示を先に確立することが検査の前提である
//!
//! 何も表示していない状態で失敗を注入しても、5 項目はもともと未確定（`None`／`false`）なので
//! 「保たれた」は恒真になる。ゆえに各テストはまず有効な `ShowSurface` で表示を確立し、**注入前の
//! 具体値を明示的に assert してから**注入する。さらに「成功していれば動いていたはずの値」を
//! 別の面・別の窓 DPI で用意する——同じ面を同じ DPI で流すと、保持と上書きが区別できない。
//!
//! # 観測点は動かさない（要件 5.6）
//!
//! 保持を成立させているのは `presenter/show.rs` の `chain.upload` 直後の早期 return 1 箇所である。
//! 本ファイルは `show.rs` に一切触れず、その分岐を見張る逐語検査
//! （`transition_record_tests` の `the_previous_size_is_read_immediately_before_the_upload_and_the_error_branch_is_unmoved`）
//! の被覆も縮めない。ここで足すのは**実行テストによる裏取り**だけである。
//!
//! # 前提（要件 5.4）
//!
//! 既存のグラフィクステストと同一——**窓なし・実 D3D デバイス**——であり、実機 GPU 障害の再現を
//! 必要としない。注入点は `#[cfg(test)]` でのみ実体を持つ（要件 5.5）。
//!
//! # 残余（5.3 → 8.3 の申し送り）
//!
//! `set_bounds` のもう一方の効果＝`mount.rs` の `SpriteVisual::SetSize` は本ファイルでは観測して
//! いない（`mount.rs` が `Arrangement` を権威と明記し `SetSize` は失敗をログのみで許す最善努力
//! ゆえ許容）。台帳へ登記する。

use super::*;

use std::path::Path;
use std::time::Duration;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};

use wintf::ecs::{Arrangement, Offset, Visual};

use crate::chain::{UploadFault, arm_upload_fault, clear_upload_fault};

use super::test_support::{
    build_two_face_assets, elem, make_world_with_gpu, mount_entities, set_window_dpi, shell_of,
    show_ok, spawn_window_with_dpi, surface,
};

/// 検査対象の target（全テストで 1 つ）。
const TARGET: TargetId = TargetId(0);
/// 表示を確立する面（＝前の状態を作る面）。
const ESTABLISHED_ID: u32 = 1000;
/// 失敗させる `ShowSurface` が指す面（確立済みの面とは別 id）。
const ATTEMPTED_ID: u32 = 3000;
/// 確立する面の native 外形。
const ESTABLISHED_SIZE: (u32, u32) = (3, 2);
/// 失敗させる面の native 外形（確立済みと**別寸**＝成功していれば `native_size` が動く）。
const ATTEMPTED_SIZE: (u32, u32) = (5, 4);
/// 表示確立時の窓 DPI（author_dpi と同値＝k は恒等）。
const AUTHOR_DPI: u16 = 96;
/// 注入直前に切り替える窓 DPI（k=2/1＝成功していれば `applied` と bounds が動く）。
const MOVED_DPI: u16 = 192;
/// 注入前に枠の面 entity の `Arrangement.offset` へ仕込む番兵。
///
/// `mount.set_bounds` は size 引数に関わらず offset を `(0.0, 0.0)` へ**無条件で**書く。本番の
/// 正常経路がこの値を作ることは無い（`mount.rs` は生成時も更新時も原点 0,0 で書く）ので、
/// 注入後にこの値が残っていれば `set_bounds` は 1 度も呼ばれていない。
const SENTINEL_OFFSET: (f32, f32) = (99.0, 77.0);

/// 7 つの失敗点を踏む順序。**`Present` を最後に置くことが必須**である。
///
/// `Present` は供給面の内部状態を commit した**後**にあるため、そこで失敗すると供給面の寸だけが
/// 新寸へ進む（要件 5.9 の既知の残余 ⒜）。順序を崩して `Present` を先に踏むと、以降の反復で
/// 供給面の寸が試行寸と一致してしまい「外形変更経路」が成立しなくなる——寸法変更 3 点
/// （`CreateSourceTex`／`CreateStaging`／`ResizeBuffers`）が踏まれず、注入が届かないまま
/// `upload` が成功して檻が壊れる。
const FAULTS_SHAPE_CHANGE: [UploadFault; 7] = [
    UploadFault::CreateSourceTex,
    UploadFault::CreateStaging,
    UploadFault::ResizeBuffers,
    UploadFault::SourceTexCast,
    UploadFault::GetBuffer,
    UploadFault::BackbufferCast,
    UploadFault::Present,
];

/// 外形不変経路で踏み得る失敗点（寸法変更 3 点はそもそも通らない）。同じ理由で `Present` は最後。
const FAULTS_SHAPE_UNCHANGED: [UploadFault; 4] = [
    UploadFault::SourceTexCast,
    UploadFault::GetBuffer,
    UploadFault::BackbufferCast,
    UploadFault::Present,
];

// ── 観測（design C4 ⒝ の 5 項目）────────────────────────────────────────

/// presenter 側の「前の状態」5 項目のスナップショット。
///
/// 4 項目は公開照会（`target_visible`／`applied_ratio`／`current_surface_id`）と私有
/// `PresentTarget.native_size` から読む。`native_size` に公開の直読み口は無く、
/// `text_slot_view().surface_size()` は「表示が一度成立していること」を前提に同じ値を返す派生口
/// なので、単一真実源そのものを見る（in-source テストの特権）。
///
/// 5 項目めの bounds は `mount.set_bounds` の**効果**——枠の面 entity の `Arrangement`——で観測する。
///
/// ここで size だけを見ても足りない。早期 return が失われた形でも `chain.upload` の commit は
/// 失敗し得る全操作より後ろにあり `chain.size()` は旧寸のままなので、`set_bounds` は**旧寸で
/// 呼ばれる**——`Arrangement.size` は同じ値が書き直されるだけで、値の一致からは「未呼出」と
/// 「旧寸での呼出」を区別できない。区別を与えるのは offset で、`set_bounds` はこれを size 引数と
/// 無関係に `(0.0, 0.0)` へ書く。ゆえに各テストは注入前に [`SENTINEL_OFFSET`] を仕込み、
/// スナップショットは offset と size の両方を持つ。
#[derive(Debug, Clone, Copy, PartialEq)]
struct PresenterState {
    /// `PresentTarget.visible`（可視性の単一真実源）。
    visible: Option<bool>,
    /// `PresentTarget.applied`（実適用 k の単一真実源）。
    applied: Option<ScaleRatio>,
    /// `PresentTarget.native_size`（k 適用前の合成外形）。
    native_size: Option<(u32, u32)>,
    /// `PresentTarget.current_surface_id`（最後に確立した面 id）。
    current_surface: Option<u32>,
    /// 枠の面 entity の `Arrangement`（offset x/y・size w/h）＝`mount.set_bounds` の効果。
    bounds: Option<(f32, f32, f32, f32)>,
}

/// 5 項目を 1 度に採る（判定は「注入前と厳密に等しい」の 1 本にする）。
fn snapshot(presenter: &EmoPresenter, world: &World, target: TargetId) -> PresenterState {
    let (surface_entity, _slot) = mount_entities(presenter, target);
    PresenterState {
        visible: presenter.target_visible(target),
        applied: presenter.applied_ratio(target),
        native_size: presenter.targets.get(&target).and_then(|t| t.native_size),
        current_surface: presenter.current_surface_id(target),
        bounds: world
            .get::<Arrangement>(surface_entity)
            .map(|a| (a.offset.x, a.offset.y, a.size.width, a.size.height)),
    }
}

/// 物理外形 `(w, h)` に対応する bounds 期待値（原点 0,0・等倍は `mount.rs` の契約）。
/// ＝`set_bounds` が**呼ばれた後**の形。陽性対照だけがこれを使う。
fn bounds_of(size: (u32, u32)) -> Option<(f32, f32, f32, f32)> {
    Some((0.0, 0.0, size.0 as f32, size.1 as f32))
}

/// 番兵を仕込んだ後・`set_bounds` が**一度も呼ばれていない**ときの bounds 期待値。
fn sentinel_bounds_of(size: (u32, u32)) -> Option<(f32, f32, f32, f32)> {
    Some((
        SENTINEL_OFFSET.0,
        SENTINEL_OFFSET.1,
        size.0 as f32,
        size.1 as f32,
    ))
}

/// 枠の面 entity の `Arrangement.offset` へ番兵を仕込む（注入前に 1 度だけ）。
///
/// 本番コードには触れない——`Arrangement` は world の component であり、テストから直接書ける。
fn plant_sentinel_offset(presenter: &EmoPresenter, world: &mut World, target: TargetId) {
    let (surface_entity, _slot) = mount_entities(presenter, target);
    let mut arr = world
        .get_mut::<Arrangement>(surface_entity)
        .expect("表示確立後は枠の面 entity に Arrangement がある");
    arr.offset = Offset {
        x: SENTINEL_OFFSET.0,
        y: SENTINEL_OFFSET.1,
    };
}

// ── 注入（武装と解除を 1 つの不可分な操作に閉じる）──────────────────────

/// 注入 → `ShowSurface` → 武装解除 を **1 つの不可分な操作**として閉じる唯一の入口。
///
/// `chain` の `fault_point` は**一致した点でしか武装を解かない**ので、届かなかった注入は旗が
/// 立ったまま残り、同一スレッドの後続 `upload` で発火する。解除を `Drop` に持たせ
/// `arm_upload_fault` の呼出を本関数の内側 1 箇所だけに限ることで、「解除の書き忘れ」も
/// 「assert の panic で解除を飛び越すこと」も**構造的に起こり得ない**
/// （`chain_fault_tests.rs` の `upload_with_armed_fault` と同じ規律）。
fn show_with_armed_fault(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
    at: UploadFault,
) -> PresentError {
    /// 生存期間の終わり（正常終了・panic による巻き戻しの双方）で必ず武装を降ろす番人。
    struct Disarm;
    impl Drop for Disarm {
        fn drop(&mut self) {
            clear_upload_fault();
        }
    }

    let _disarm = Disarm;
    arm_upload_fault(at);

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
    match rx.recv_timeout(Duration::from_secs(10)) {
        // 要件 5.2/5.3: 呼出元へ失敗が返る（`chain.rs` が `device_err` 経由で error! 済み）。
        Ok(Err(e)) => e,
        Ok(Ok(())) => panic!(
            "注入した失敗点 {at:?} で ShowSurface が Ok を返した（注入が届いていない＝前状態保持を判定できない）"
        ),
        Err(e) => panic!("reply（ShowSurface）を受信できない: {e}"),
    }
}

/// 注入の失敗が本番と同じ形（`PresentError::Device`）で返っていること。
fn assert_device_error(err: &PresentError, at: UploadFault) {
    assert!(
        matches!(err, PresentError::Device { .. }),
        "注入の失敗は PresentError::Device のはずだが {err:?} が返った（{at:?}）"
    );
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

// ── フィクスチャ（別寸 2 面）───────────────────────────────────────────

/// 面 1000＝`small`・面 3000＝`large` の**別寸・別バイト**を持つ単一 world。
///
/// `presenter_test_support` の `build_two_face_assets` は同寸 2 面（外形不変経路の材料）なので、
/// 「成功していれば `native_size` が動いていたはず」を作れない。別寸の対がここで要る唯一の理由が
/// それであり、作り方（atlas bake → `EmoWorld` → `bind_atlas`）は共有ヘルパと同一である。
fn build_two_size_assets(small: (u32, u32), large: (u32, u32)) -> (EmoWorld, AtlasTable) {
    assert_ne!(
        small, large,
        "フィクスチャ前提: 2 面が同寸では native_size の上書きを判定できない"
    );
    let base = Path::new("shell/master");
    let surfaces = vec![
        surface(ESTABLISHED_ID, vec![elem("p.png", 0, 0)]),
        surface(ATTEMPTED_ID, vec![elem("q.png", 0, 0)]),
    ];

    // 全不透明（α=255）ゆえ α=0 除外トリムは全域を残し、合成外形は正確に `w×h` になる。
    let gradient = |w: u32, h: u32, salt: u8| -> Vec<u8> {
        let mut img: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.extend_from_slice(&[b, g, r, 0xFF]);
            }
        }
        img
    };

    let mut dec = MemoryDecoder::new();
    dec.insert(
        base.join("p.png"),
        small.0,
        small.1,
        small.0 * 4,
        gradient(small.0, small.1, 0x11),
        true,
    );
    dec.insert(
        base.join("q.png"),
        large.0,
        large.1,
        large.0 * 4,
        gradient(large.0, large.1, 0x77),
        true,
    );

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

/// 別寸 2 面の world を載せた target を 1 つ装着し、面 1000 で表示を確立する。
fn presenter_with_established_display(world: &mut World) -> (EmoPresenter, Entity) {
    let window = spawn_window_with_dpi(world, AUTHOR_DPI);
    let (emo_world, atlas) = build_two_size_assets(ESTABLISHED_SIZE, ATTEMPTED_SIZE);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(world, TARGET, window, emo_world, atlas, AUTHOR_DPI)
        .expect("attach_target 失敗");
    // ここが「前の状態」——表示を一度確立しなければ保持の主張は恒真になる。
    show_ok(&mut presenter, world, TARGET, ESTABLISHED_ID);
    (presenter, window)
}

/// 注入前の 5 項目が**確立済みの具体値**であることを固定する（恒真の檻を作らないための前提検査）。
fn assert_established_baseline(before: &PresenterState, visible: bool, current: Option<u32>) {
    assert_eq!(before.visible, Some(visible), "前提: visible が想定と違う");
    assert!(
        before
            .applied
            .expect("前提: 表示確立後は applied が確定している")
            .is_identity(),
        "前提: 確立時の k は恒等（窓 DPI ＝ author_dpi）"
    );
    assert_eq!(
        before.native_size,
        Some(ESTABLISHED_SIZE),
        "前提: native_size が確立した面の原寸でない"
    );
    assert_eq!(
        before.current_surface, current,
        "前提: 現サーフェスが想定と違う"
    );
    assert_eq!(
        before.bounds,
        sentinel_bounds_of(ESTABLISHED_SIZE),
        "前提: bounds が「確立した面の物理寸 ＋ 仕込んだ番兵 offset」でない"
    );
}

// ── 外形変更経路（7 失敗点すべて）──────────────────────────────────────

/// 要件 5.2/5.3: **7 つの失敗点すべて**で、presenter 側 5 項目が 1 つも書き換わらず `reply` は `Err`。
///
/// 注入する `ShowSurface` は「別 id・別 native 寸・別 k」——成功していれば `current_surface`・
/// `native_size`・`applied`・bounds の 4 項目が同時に動く状況である。`visible` は確立時点で既に
/// true ゆえこの場面では判別力を持たない（それは
/// [`upload_failure_after_hide_leaves_the_target_hidden_and_the_previous_values_intact`] が持つ）。
///
/// 1 つの実デバイスで 7 点を続けて踏めるのは、失敗のたびに presenter も供給面も前状態のまま
/// 残るからである（`Present` だけは供給面の寸が進むので最後に置く・[`FAULTS_SHAPE_CHANGE`] の doc）。
#[test]
fn upload_failure_preserves_every_presenter_side_value_at_all_seven_fault_points() {
    let mut world = make_world_with_gpu();
    let (mut presenter, window) = presenter_with_established_display(&mut world);

    // 窓 DPI を動かす: この後の ShowSurface が成功していれば k が 2/1 へ、物理寸も別値へ動く。
    set_window_dpi(&mut world, window, MOVED_DPI);

    // 失敗経路の `set_bounds` は旧寸で呼ばれ得るので、寸だけでは呼出を検知できない（[`snapshot`] の doc）。
    plant_sentinel_offset(&presenter, &mut world, TARGET);

    let before = snapshot(&presenter, &world, TARGET);
    assert_established_baseline(&before, true, Some(ESTABLISHED_ID));

    for at in FAULTS_SHAPE_CHANGE {
        let err = show_with_armed_fault(&mut presenter, &mut world, TARGET, ATTEMPTED_ID, at);
        assert_device_error(&err, at);
        assert_eq!(
            snapshot(&presenter, &world, TARGET),
            before,
            "{at:?} の失敗で presenter 側の前状態が書き換わった"
        );
    }

    // ── 陽性対照 ────────────────────────────────────────────────────────
    // 同じ `ShowSurface` を注入なしで流すと 4 項目が実際に動く。これが無いと上の不変は
    // 「もともと何も動かない場面を見ていた」可能性を排除できない（恒真の檻との区別が付かない）。
    show_ok(&mut presenter, &mut world, TARGET, ATTEMPTED_ID);
    let after = snapshot(&presenter, &world, TARGET);
    assert_eq!(
        after.current_surface,
        Some(ATTEMPTED_ID),
        "陽性対照: 成功した ShowSurface は現サーフェスを動かす"
    );
    assert_eq!(
        after.native_size,
        Some(ATTEMPTED_SIZE),
        "陽性対照: 成功した ShowSurface は native 原寸を動かす"
    );
    assert!(
        !after
            .applied
            .expect("陽性対照: 成功後は applied が確定している")
            .is_identity(),
        "陽性対照: 窓 DPI を動かした後の成功は k を恒等から動かす"
    );
    assert_eq!(
        after.bounds,
        bounds_of((ATTEMPTED_SIZE.0 * 2, ATTEMPTED_SIZE.1 * 2)),
        "陽性対照: 成功した ShowSurface は bounds を k 適用後の物理寸へ動かす"
    );
}

// ── 外形不変経路（表示画素まで見る）────────────────────────────────────

/// 要件 5.2: **外形が変わらない**面切替で失敗しても、5 項目に加えて**表示画素そのもの**が前の内容。
///
/// 同寸 2 面（`build_two_face_assets`）を使うので、成功していれば動くのは `current_surface` と
/// 表示バイトだけである——寸も k も動かない場面で「絵だけが半端に入れ替わる」ことがないことを
/// 見る。`Present` だけは要件 5.9 の既知の残余 ⒜（`source_tex` に未提示の試行内容が残る）ゆえ
/// 最後に置き、そこで初めて読み戻しが試行内容へ変わることを期待値として記録する。
#[test]
fn upload_failure_on_a_same_shape_surface_keeps_the_surface_id_and_the_displayed_pixels() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, AUTHOR_DPI);
    let (emo_world, atlas, golden_established, golden_attempted) =
        build_two_face_assets(ESTABLISHED_SIZE.0, ESTABLISHED_SIZE.1);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TARGET, window, emo_world, atlas, AUTHOR_DPI)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TARGET, ESTABLISHED_ID);

    // 外形不変経路では旧寸＝新寸ゆえ、番兵 offset だけが `set_bounds` の呼出を映す。
    plant_sentinel_offset(&presenter, &mut world, TARGET);

    let before = snapshot(&presenter, &world, TARGET);
    assert_established_baseline(&before, true, Some(ESTABLISHED_ID));
    assert_eq!(
        presenter.read_back(TARGET).expect("前提の read_back 失敗"),
        golden_established,
        "前提: 確立した表示画素が golden と一致しない"
    );

    for at in FAULTS_SHAPE_UNCHANGED {
        let err = show_with_armed_fault(&mut presenter, &mut world, TARGET, ATTEMPTED_ID, at);
        assert_device_error(&err, at);
        assert_eq!(
            snapshot(&presenter, &world, TARGET),
            before,
            "{at:?} の失敗で presenter 側の前状態が書き換わった"
        );

        let expected_pixels = if at == UploadFault::Present {
            // 既知の残余 ⒜（要件 5.9・2026-08-22 設計ディスカッション 議題 2 の裁定）: `Present`
            // 失敗では backbuffer は前フレームのままだが `source_tex` は試行内容を持つため、
            // CPU 読み戻しは**未提示の試行内容**を返す。presenter 側 5 項目はそれでも不変である。
            &golden_attempted
        } else {
            &golden_established
        };
        assert_eq!(
            &presenter
                .read_back(TARGET)
                .expect("失敗後の read_back 失敗"),
            expected_pixels,
            "{at:?} の失敗後の表示画素が期待値と違う"
        );
    }

    // 陽性対照: 注入なしなら同じ指令が現サーフェスと表示画素を実際に動かす。
    show_ok(&mut presenter, &mut world, TARGET, ATTEMPTED_ID);
    assert_eq!(
        presenter.current_surface_id(TARGET),
        Some(ATTEMPTED_ID),
        "陽性対照: 成功した ShowSurface は現サーフェスを動かす"
    );
    assert_eq!(
        presenter
            .read_back(TARGET)
            .expect("陽性対照の read_back 失敗"),
        golden_attempted,
        "陽性対照: 成功した ShowSurface は表示画素を動かす"
    );
}

// ── 不可視からの失敗（`visible` に判別力を持たせる場面）──────────────────

/// 要件 5.2: `Hide` 後に失敗した `ShowSurface` は target を**可視へ戻さない**（5 項目すべて不変）。
///
/// この場面でだけ `visible` が判別力を持つ——成功していれば `visible` は true へ、
/// `current_surface` は `Some(3000)` へ動く。entity 側（枠の面・文字層スロットの `Visual`）も
/// 併せて見るのは、「照会は false を返すが entity は可視」という食い違いを見逃さないためである。
#[test]
fn upload_failure_after_hide_leaves_the_target_hidden_and_the_previous_values_intact() {
    let mut world = make_world_with_gpu();
    let (mut presenter, window) = presenter_with_established_display(&mut world);

    hide_ok(&mut presenter, &mut world, TARGET);
    set_window_dpi(&mut world, window, MOVED_DPI);
    plant_sentinel_offset(&presenter, &mut world, TARGET);

    let before = snapshot(&presenter, &world, TARGET);
    // `Hide` は可視性と現サーフェスだけを落とす（k・原寸・bounds は保持される）。
    assert_established_baseline(&before, false, None);

    let err = show_with_armed_fault(
        &mut presenter,
        &mut world,
        TARGET,
        ATTEMPTED_ID,
        UploadFault::SourceTexCast,
    );
    assert_device_error(&err, UploadFault::SourceTexCast);
    assert_eq!(
        snapshot(&presenter, &world, TARGET),
        before,
        "失敗した ShowSurface が不可視 target の前状態を書き換えた"
    );

    let (surface_entity, slot) = mount_entities(&presenter, TARGET);
    assert!(
        !world
            .get::<Visual>(surface_entity)
            .expect("surface に Visual")
            .is_visible,
        "失敗した ShowSurface が枠の面 entity を可視へ戻した"
    );
    assert!(
        !world
            .get::<Visual>(slot)
            .expect("slot に Visual")
            .is_visible,
        "失敗した ShowSurface が文字層スロット entity を可視へ戻した"
    );

    // 陽性対照: 注入なしなら同じ指令が可視へ戻す（＝上の false 維持は判別力を持つ）。
    show_ok(&mut presenter, &mut world, TARGET, ATTEMPTED_ID);
    assert_eq!(
        presenter.target_visible(TARGET),
        Some(true),
        "陽性対照: 成功した ShowSurface は指令駆動 target を可視へ戻す"
    );
    assert!(
        world
            .get::<Visual>(surface_entity)
            .expect("surface に Visual")
            .is_visible,
        "陽性対照: 成功した ShowSurface は枠の面 entity を可視へ戻す"
    );
}
