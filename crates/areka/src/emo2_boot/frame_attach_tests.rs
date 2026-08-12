use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use areka_emo_text::state::TextLayerConfig;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use crate::emo2_boot::assets::build_boot_assets;
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;

use super::*;
use super::test_support::{
    resnap_world,
    synth_assets,
    synth_assets_with_balloons,
    zero_clock,
};

/// DD-12 完全一致: `window_scopes == 資産 scope` → 計画件数＝窓数・missing/unused 空。
///
/// 各項目の shell/balloon target（DD-3 採番）と初期面（DD-9: scope0→0／scope1→10）・添字も検証する。
#[test]
fn plan_attachments_exact_match_plans_all_windows() {
    // 資産 scope [0,1]（DD-9: scope0→初期 surface 0／scope1→初期 surface 10）。
    let assets = synth_assets(&[(0, 0), (1, 10)]);
    let window_scopes = [0usize, 1];

    let plan = plan_attachments(&window_scopes, &assets);

    // 計画件数＝窓数（DD-12 の積極 assert・完全一致の核）。
    assert_eq!(
        plan.items.len(),
        window_scopes.len(),
        "完全一致では計画件数＝窓数"
    );
    assert!(plan.missing_assets.is_empty(), "窓あり資産なしは無い");
    assert!(plan.unused_assets.is_empty(), "資産あり窓なしは無い");

    // scope0 の項目: shell=TargetId(0)・balloon=TargetId(1)・初期面 0（DD-9）。
    assert_eq!(plan.items[0].scope, 0);
    assert_eq!(plan.items[0].shell_target, shell_target(0));
    assert_eq!(plan.items[0].balloon_target, balloon_target(0));
    assert_eq!(plan.items[0].initial_surface_id, 0, "scope0 初期面 0（DD-9）");
    assert_eq!(plan.items[0].shell_index, 0);
    assert_eq!(plan.items[0].balloon_index, Some(0));

    // scope1 の項目: shell=TargetId(2)・balloon=TargetId(3)・初期面 10（DD-9）。
    assert_eq!(plan.items[1].scope, 1);
    assert_eq!(plan.items[1].shell_target, shell_target(1));
    assert_eq!(plan.items[1].balloon_target, balloon_target(1));
    assert_eq!(plan.items[1].initial_surface_id, 10, "scope1 初期面 10（DD-9）");
    assert_eq!(plan.items[1].shell_index, 1);
    assert_eq!(plan.items[1].balloon_index, Some(1));
}

/// DD-12 窓あり資産なし: 窓 scope に対応資産が無ければ `missing_assets` へ（`items` には載らない）。
#[test]
fn plan_attachments_window_without_asset_goes_to_missing() {
    // 資産 [0,1]・窓 [0,1,2] → 窓 scope 2 は資産なし。
    let assets = synth_assets(&[(0, 0), (1, 10)]);
    let window_scopes = [0usize, 1, 2];

    let plan = plan_attachments(&window_scopes, &assets);

    assert_eq!(plan.missing_assets, vec![2usize], "窓あり資産なしは missing 検出");
    assert_eq!(plan.items.len(), 2, "資産のある 0,1 のみ装着計画に載る");
    assert!(
        plan.items.iter().all(|it| it.scope != 2),
        "scope2 は items に載らない"
    );
    assert!(plan.unused_assets.is_empty(), "全資産に窓がある");
}

/// DD-12 資産あり窓なし: 窓を持たない資産 scope は `unused_assets`（`u32`）へ（`items` には載らない）。
#[test]
fn plan_attachments_asset_without_window_goes_to_unused() {
    // 資産 [0,1]・窓 [0] のみ → 資産 scope 1 は窓なし。
    let assets = synth_assets(&[(0, 0), (1, 10)]);
    let window_scopes = [0usize];

    let plan = plan_attachments(&window_scopes, &assets);

    assert_eq!(plan.unused_assets, vec![1u32], "資産あり窓なしは unused 検出（u32）");
    assert_eq!(plan.items.len(), 1, "窓のある scope0 のみ装着計画に載る");
    assert_eq!(plan.items[0].scope, 0);
    assert!(plan.missing_assets.is_empty(), "全窓に資産がある");
}

/// DD-12 `usize`→`u32` 変換境界: 小 usize scope は u32 資産 scope と一致・`u32::MAX` 超過は missing。
#[test]
fn plan_attachments_usize_to_u32_conversion_boundary() {
    let assets = synth_assets(&[(0, 0), (1, 10)]);

    // 小さい usize scope (0,1) は u32 資産 scope と正しく一致する。
    let plan = plan_attachments(&[0usize, 1], &assets);
    assert_eq!(plan.items.len(), 2, "小 usize scope は u32 資産と一致");
    assert_eq!(plan.items[0].scope, 0);
    assert_eq!(plan.items[1].scope, 1);

    // usize > u32::MAX は如何なる u32 資産 scope とも一致し得ず missing 分類（64bit 環境でのみ表現可能）。
    #[cfg(target_pointer_width = "64")]
    {
        let overflow = (u32::MAX as usize) + 1; // u32 に収まらない境界超過値
        let plan = plan_attachments(&[0usize, overflow], &assets);
        assert_eq!(plan.items.len(), 1, "overflow scope は装着計画に載らない");
        assert_eq!(plan.items[0].scope, 0, "収まる 0 のみ計画に載る");
        assert_eq!(
            plan.missing_assets,
            vec![overflow],
            "u32 超過 usize は missing 分類（変換不能）"
        );
    }
}

/// `balloon_index` の `None` 経路: shell 資産はあるが同 scope の balloon 資産が無い場合。
#[test]
fn plan_attachments_marks_missing_balloon_index_none() {
    // shell scope [0,1]・balloon scope [0] のみ → scope1 は shell だけで balloon なし。
    let assets = synth_assets_with_balloons(&[(0, 0), (1, 10)], &[0]);
    let plan = plan_attachments(&[0usize, 1], &assets);

    assert_eq!(plan.items.len(), 2, "shell 資産の揃う 0,1 が計画に載る");
    assert_eq!(plan.items[0].balloon_index, Some(0), "scope0 は balloon あり");
    assert_eq!(plan.items[1].balloon_index, None, "scope1 は balloon なし → None");
}

/// R4.2 headless: `text_slot_view` が `None`（初回 ShowSurface 未合流）の経路では文字層を
/// 接続せず `false` を返し、次フレーム再試行へ委ねる（panic しない・register も呼ばない）。
///
/// 実 `EmoPresenter::new()` は何も装着していないため `text_slot_view(any) == None`。この実源の
/// `None` を `connect_balloon_text` に与え、接続が起きない（`false`）ことを GPU 不要で檻に入れる。
#[test]
fn connect_balloon_text_skips_when_view_none() {
    // 初回 ShowSurface 前の presenter は text_slot_view が None（R4.2 の遅延化を実源で再現）。
    let presenter = EmoPresenter::new();
    let view = presenter.text_slot_view(TargetId(1));
    assert!(view.is_none(), "初回 ShowSurface 前の text_slot_view は None（前提）");

    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    let model = areka_parsers::balloon::parse_str("", None);

    // None 経路: 文字層を接続せず false（次フレーム再試行へ委ねる）。panic しない。
    let registered = connect_balloon_text(&runtime, view, ActorKey::from("0"), &model);
    assert!(
        !registered,
        "text_slot_view None では文字層を接続せず false を返す（R4.2）"
    );

    // register_actor_view 未呼出の担保: runtime は無改変で再借用可能（lingering borrow / poison なし）。
    assert!(
        runtime.try_borrow_mut().is_ok(),
        "None 経路は runtime に触れない（借用/poison を残さない）"
    );
}

/// ゲート（GPU 資源）不成立の World では装着しない・panic しない・資産を消費しない（R1.3/4.1）。
///
/// `GraphicsCore`／`WucGraphicsResource`／`GhostWindows` を一切持たない `World` に対し
/// `run_attach_phase` を複数回呼んでも、`attached` は false のまま・`assets` は `Some` のまま
/// （高々 1 回消費の `take` はゲート成立後にのみ行うため未消費）で、panic しないことを headless に固定する。
#[test]
fn run_attach_phase_without_gpu_does_not_attach_or_consume_assets() {
    let mut wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<MoveDirective>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
        TalkClock::new(Arc::new(|| 0.0)),
        synth_assets(&[(0, 0), (1, 10)]),
    );
    // GraphicsCore/WucGraphicsResource/GhostWindows を一切持たない素の World（ゲート不成立）。
    let mut world = World::new();

    run_attach_phase(&mut wiring, &mut world);
    assert!(!wiring.attached, "GPU 資源なしでは装着しない（attached=false）");
    assert!(
        wiring.assets.is_some(),
        "ゲート不成立では assets を take しない（次フレーム再試行のため保持）"
    );
    assert!(
        wiring.balloon_model_scopes().is_empty(),
        "ゲート不成立では per-scope BalloonModel も記憶しない（attach 相の副作用ゼロ・D11-3）"
    );

    // 冪等: 再試行してもゲート不成立なら未装着のまま・panic しない・資産も保持する。
    run_attach_phase(&mut wiring, &mut world);
    assert!(!wiring.attached, "再試行でもゲート不成立なら未装着");
    assert!(wiring.assets.is_some(), "再試行でも assets を保持");
}

// ── task 3.3: attach が scope 別バルーン定義を文字層へ供給する（R2.6/4.1/4.2） ──────
//
// 実 emo2 fixture（`emo2-kakukaku`）では scope0 が `balloons0.png`／scope1 が `balloonk0.png`
// を採用し、面別上書き（`balloons0s.txt`／`balloonk0s.txt`）の `validrect`／`windowposition`
// が互いに異なる。ゆえに「先頭 scope の定義を全 scope へ配る」実装はこの檻で必ず落ちる。
// attach は GPU 資源ゲート越しにしか走らないため、檻も headless GPU（WARP 可・MTA）で
// 本番 [`run_attach_phase`] をそのまま駆動する（シームを噛ませない）。

/// emo2 fixture ルート（`CARGO_MANIFEST_DIR`＝`crates/areka` 相対・assets.rs テストと同一規約）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（assets.rs テストと同一規約）。
fn emo2_balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

/// [`resnap_world`]（2 スコープ窓＋偽 `WindowHandle`）へ実 GPU 資源を載せた attach 檻の World。
///
/// attach ゲートは `GhostWindows`＋`GraphicsCore`＋`WucGraphicsResource::is_valid()` の連言ゆえ、
/// 本番 attach を駆動するにはこの 3 点が要る。本番 UI スレッドと同じ MTA で COM を初期化する
/// （記憶: areka WUC は MTA スレッドで動く）。WARP 可（`GraphicsCore::new()`）。
fn gpu_attach_world() -> (World, GhostWindows) {
    // SAFETY: WIC デコード／D3D に要る COM の MTA 初期化
    // （既初期化の S_FALSE／RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let (mut world, gw) = resnap_world();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    world.insert_resource(core);
    world.insert_resource(wuc);
    (world, gw)
}

/// 観測可能な完了条件（tasks.md task 3.3・R4.1/4.2）: **相方側 scope の文字描画領域が相方側
/// 定義から解決される**。実 emo2 fixture＋実 GPU で本番 attach を駆動し、per-scope の
/// [`BalloonModel`] が
/// (a) 再追従の記憶写像 `balloon_models`、(b) 文字層結線 [`connect_balloon_text`]
/// の**双方へ同一値**で届くことを固定する。
///
/// (b) の観測は `refresh_actor_scale` の判定契約を利用する——装着直後の同一 view に対し、
/// **相方側定義**で再追従を求めると「binding・文字描画領域とも同値」で `false`（no-op）に
/// なり、**本体側定義**なら `validrect` 差により `true`（再構築）になる。すなわち
/// `false`/`true` の対が「装着時にどちらの定義で領域を解決したか」を弁別する。
///
/// 先頭 scope の定義を全 scope へ配る実装では (a) が相方側で本体側の値を返し、(b) は
/// 相方側定義に対して `true`（＝装着が別定義だった証跡）を返して落ちる。
#[test]
fn attach_supplies_each_scope_its_own_balloon_model_to_map_and_text_layer() {
    let (mut world, _gw) = gpu_attach_world();
    let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");
    assert_eq!(
        assets.balloons.iter().map(|b| b.scope).collect::<Vec<_>>(),
        vec![0u32, 1],
        "前提: balloon 資産は scope 昇順 [0,1]（添字＝scope の対応）"
    );
    // 期待値は資産そのものから取る（fixture 実値との突き合わせは assets.rs の檻が所有するが、
    // 非空虚性のため両 scope の実値が実際に異なることをここでも 1 点だけ固定する）。
    let sakura = assets.balloons[0].model.clone();
    let kero = assets.balloons[1].model.clone();
    assert_eq!(
        (sakura.validrect().top(), kero.validrect().top()),
        (Some(46), Some(40)),
        "前提: 本体側／相方側の validrect が実際に異なる（同値なら本檻は何も弁別しない）"
    );

    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    let mut wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<MoveDirective>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        Rc::clone(&runtime),
        zero_clock(),
        assets,
    );

    run_attach_phase(&mut wiring, &mut world);
    assert!(
        wiring.attached,
        "前提: GhostWindows＋GPU 資源のゲート成立で attach が走る"
    );
    assert_eq!(
        wiring.balloon_model_scopes(),
        vec![0u32, 1],
        "前提: 両 scope の balloon 装着が成立している"
    );

    // (a) 再追従の記憶写像（D11-3）: scope ごとに **その scope の** 定義が入る（R4.1）。
    assert_eq!(
        wiring.balloon_models[&0].validrect().top(),
        sakura.validrect().top(),
        "本体側 scope には本体側定義が入る"
    );
    assert_eq!(
        wiring.balloon_models[&1].validrect().top(),
        kero.validrect().top(),
        "相方側 scope には相方側定義が入る（先頭 scope の配り回しではない・R4.1）"
    );
    assert_eq!(
        wiring.balloon_models[&1].windowposition().x(),
        Some(-190),
        "相方側 scope の windowposition も相方側定義由来（balloonk0s.txt 実値）"
    );

    // (b) 文字層結線へ届いた値: 装着と同一の scope→アクタ写像（R4.2）で登録済みの binding は、
    //     相方側定義で解決済み＝相方側定義での再追従は no-op（false）になる。
    let view = wiring
        .presenter
        .text_slot_view(balloon_target(1))
        .expect("前提: attach 初回 ShowSurface で相方側バルーンの文字層スロットが成立する");
    assert!(
        !runtime
            .borrow_mut()
            .refresh_actor_scale(&ActorKey::from("1"), &view, &kero),
        "相方側 scope の文字描画領域は相方側定義から解決済み（同値ゆえ再構築しない・R4.1）"
    );
    // 非空虚性: 本体側定義なら validrect 差で必ず再構築が要る＝上の false は「何を渡しても
    // false」ではない（弁別が生きている）。
    assert!(
        runtime
            .borrow_mut()
            .refresh_actor_scale(&ActorKey::from("1"), &view, &sakura),
        "本体側定義は相方側 scope の解決済み領域と異なる（この差が上の no-op を意味あるものにする）"
    );
}

/// task 2.5（Req 1.3）: UI 配線層（`Emo2Wiring`）の presenter 読み口から collision-geometry の
/// `resolve_hit_region` を呼べることを固定する。`input-events` の第一消費者（task 2.6/2.7 の
/// `RegionSource::Presenter`）が `Emo2Wiring::presenter()` を借りて resolver を叩く経路の縮図。
///
/// 未装着 `EmoPresenter`（現サーフェス無し）では collision-geometry の documented degrade により
/// `region: None` へ正常縮退する（hit_region.rs 4.4/5.3）。GPU/表示なしで決定論的に成立する。
#[test]
fn presenter_accessor_feeds_resolve_hit_region() {
    use crate::emo2_boot::hit_region::{resolve_hit_region, HitRegion};

    let wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<MoveDirective>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
        TalkClock::new(Arc::new(|| 0.0)),
        synth_assets(&[(0, 0), (1, 10)]),
    );

    // UI 配線層の読み口から resolver を呼べる（Req 1.3・型が resolve_hit_region の
    // 第 1 引数 &EmoPresenter に一致することをコンパイル時にも固定）。未装着ゆえ region None。
    let got = resolve_hit_region(wiring.presenter(), 0, 100, 100);
    assert_eq!(
        got,
        HitRegion {
            scope: 0,
            region: None,
            surface_point: (100, 100),
        },
        "未装着 presenter は region None・座標は等倍縮退（collision-geometry の正常縮退・Req1.3）"
    );
}

/// task 9.1 存在檻: `MoveCueSink`（送出端）→ `Emo2Wiring`（受信端 `move_rx`）の channel 配線が
/// 到達可能であること（`wire_emo2_boot` が `mpsc::channel::<MoveDirective>()` を生成し送出端を
/// sinks 第 3 要素の `MoveCueSink` へ、受信端を `Emo2Wiring` へ渡す配線の縮図）。
///
/// 送出端を持つ `MoveCueSink` に `\![move]` キャリア cue を `emit` すると、`Emo2Wiring` が保持する
/// 受信端から同一 `MoveDirective` が drain できる（frame 相 drain＝task 9.2 の適用は本檻の範囲外）。
#[test]
fn move_cue_sink_reaches_emo2_wiring_receiver() {
    use super::super::move_cue::MoveCueSink;
    use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};

    // wire_emo2_boot 手順4 と同型: 単一 channel の送出端を sink へ、受信端を Emo2Wiring へ。
    let (move_tx, move_rx) = mpsc::channel::<MoveDirective>();
    let mut move_sink = MoveCueSink::new(move_tx);
    let wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        move_rx,
        mpsc::channel::<TalkLifecycleSignal>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
        TalkClock::new(Arc::new(|| 0.0)),
        synth_assets(&[(0, 0)]),
    );

    // sink（talk スレッド相当）へ `\![move]` キャリアを emit → 受信端（Emo2Wiring）へ届く。
    move_sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("1"),
        command: CueCommand::command_carrier(
            "move",
            ["-353", "", "", "0", "base", "base"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        duration: 0.0,
    });

    let drained = wiring.drain_move_directives();
    assert_eq!(drained.len(), 1, "sink→Emo2Wiring の受信端へちょうど 1 件届く");
    assert_eq!(drained[0].scope, 1, "scope は cue.actor（\\1）由来");
    assert_eq!(
        drained[0].base,
        crate::emo2_boot::move_cue::MoveBase::Scope(0),
        "base=scope0（fixture 形）"
    );
}

/// task 4.1 完了状態の檻: **会話を再生すると**フレーム側（`Emo2Wiring`）が受信端
/// `lifecycle_rx` から表示ライフサイクル信号を取り出せること（Requirements 4.1 / 4.5）。
///
/// `wire_emo2_boot` の配線（`mpsc::channel::<TalkLifecycleSignal>()` を生成し、送出端を
/// `GhostBootOptions.sinks` 4 本目の `BalloonLifecycleSink` へ、受信端を `Emo2Wiring` へ渡す）の
/// 縮図を、実台本の再生経路（parse → compile → `CueSheet` → `CuePlayer` の broadcast）の上で
/// 通す。受け取った信号の消費（計測の破棄・占有終端の更新）は task 4.4 の配線が担うため本檻の
/// 範囲外で、ここで固定するのは「会話の再生が受信端へ届く」ことだけである。
///
/// 再生は**注入時刻のみ**で駆動する（実時間の待機なし・Requirement 4.9 / 9.2）。tick 時刻は
/// 台本自身の発火時刻と占有終端から採るため、注入時刻が観測すべき時点を追い越さない
/// （Requirement 9.3——反復回数による有界化もしない）。
#[test]
fn talk_playback_reaches_emo2_wiring_lifecycle_receiver() {
    use super::super::talk_lifecycle::BalloonLifecycleSink;
    use areka_sakura::sysvar::SystemVarSnapshot;
    use dola::cue::CuePlayer;

    // wire_emo2_boot 手順4/5 と同型: 単一 channel の送出端を sink へ、受信端を Emo2Wiring へ。
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<MoveDirective>().1,
        lifecycle_rx,
        Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
        TalkClock::new(Arc::new(|| 0.0)),
        synth_assets(&[(0, 0)]),
    );

    // 待機（`\w9`）を含む実台本（Requirement 4.1 の「占有区間の終端」が最後の文字より後になる形）。
    let instructions = areka_parsers::sakura::parse(r"\0あい\w9\1うえ\e");
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
    let horizon = compiled.sheet.absolute_end_time();
    assert!(
        horizon > 0.0,
        "前提: 待機を含む台本の占有終端は正（台本が空なら本檻は空虚になる）"
    );

    let mut player = CuePlayer::from_sheet(&compiled.sheet);
    player.register_sink(Box::new(BalloonLifecycleSink::new(lifecycle_tx)));

    // 台本の発火時刻を昇順に辿り、最後に占有終端へ達する（sleep もスピンも使わない）。
    let mut tick_points: Vec<f64> = compiled
        .sheet
        .cues()
        .iter()
        .map(|cue| compiled.sheet.absolute_fire_time(cue))
        .collect();
    tick_points.push(horizon);
    for at in tick_points {
        player.tick(at);
    }

    // フレーム側の受信端から取り出す（本番は task 4.4 の可視性相が同じ口を drain する）。
    let signals = wiring.drain_lifecycle_signals();
    assert_eq!(
        signals.first(),
        Some(&TalkLifecycleSignal::TalkStarted),
        "会話の開始通知が先行する（Ordering 契約・Requirement 4.5 の計測破棄契機）"
    );

    let ends: Vec<f64> = signals
        .iter()
        .filter_map(|signal| match signal {
            TalkLifecycleSignal::DisplayEndAt(end) => Some(*end),
            TalkLifecycleSignal::TalkStarted => None,
        })
        .collect();
    assert!(
        ends.windows(2).all(|pair| pair[0] < pair[1]),
        "占有終端は狭義単調増加で届く（実際の受信列: {ends:?}）"
    );
    assert_eq!(
        ends.last().copied(),
        Some(horizon),
        "最後に届く占有終端は台本の占有区間の終端と一致する（待機込み・Requirement 4.1）"
    );
}
