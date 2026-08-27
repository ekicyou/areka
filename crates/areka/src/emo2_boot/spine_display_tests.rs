use super::test_support::{opaque_count, variant_name};
use super::{
    BindSet, Duration, Instant, PresentCommand, SPIN_WAIT, SpineHarness, TargetId, balloon_target,
    capture_logs, count_level, run_attach_phase, settle_bounded, shell_target,
};

/// `PresentCommand`（`#[non_exhaustive]`）の表示対象 `TargetId` を取り出す（未知 variant は `None`）。
fn present_command_target(cmd: &PresentCommand) -> Option<TargetId> {
    match cmd {
        PresentCommand::ShowSurface { target, .. } => Some(*target),
        PresentCommand::Hide { target, .. } => Some(*target),
        PresentCommand::InvalidateCache { target, .. } => Some(*target),
        _ => None,
    }
}

/// 注入 Tick を増分しながら rx を有界に drain し、`want` 件の `PresentCommand` を集めて返す。
///
/// scripted OnBoot talk（`ghost`→`sakura`→`seriko`→`PresentBridge`→rx）は別スレッド群を跨いで
/// 非同期に流れるため、Tick 注入（`DispatcherMsg::Tick`）と `try_iter` drain を有界ループで交互に
/// 回し、必要件数が揃うか有界時間が尽きるまで進める（R8.3）。全 cue が `at=0.0`（`\w` なし）ゆえ最初の
/// 有効 Tick で発火し切るが、talk 起動・スレッド伝播の遅延を有界待機で吸収する。揃わなければ短い Vec を
/// 返す（呼び手が件数を assert＝hang しない）。
///
/// 有界性は壁時計 deadline（[`SPIN_WAIT`]）＋200µs poll-backoff sleep（R7.9・根拠は [`drive_shell_shown`] の doc）。
/// 観測 `received` は累積＝ラッチであり、呼出点の台本はいずれも `\w`／`\c` を含まない（全 cue `at=0.0`）
/// ため、注入時刻の前進が観測を壊すクラス（R7.8）ではない。
fn tick_and_collect(harness: &mut SpineHarness, want: usize) -> Vec<PresentCommand> {
    let mut received: Vec<PresentCommand> = Vec::new();
    let deadline = Instant::now() + SPIN_WAIT;
    let mut now = 0u64;
    loop {
        now += 1;
        harness.inject_dispatcher_tick(now);
        received.extend(harness.wiring.drain_received());
        if received.len() >= want || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    received
}

/// spine S1（boot→表示・DD-12・R1.1/1.2/1.3/1.4/8.1/8.2/8.5）: `\b` を含まない OnBoot 台本
/// （`\s[0]`）で boot→attach フェーズ→初回 `\s` 駆動を走らせ、(a) 装着サマリ `info!` が
/// planned==attached==2（DD-12 の縮退が scope 導出バグを隠さない檻＝期待 scope 数の全 target 完了）・
/// ERROR 0 件、(b) **シェルは attach 直後は非表示**（`read_back` Err＝供給面未生成・defect #5・
/// 2026-07-13 実機#5）で**バルーンは attach で面 0 を不可視のまま確立済み**（`opaque_count>0`＝面0 の
/// 実描画＋文字層スロット取得。可視性そのものは付かない）、(c) 最初のさくらスクリプト `\s[0]` cue が
/// seriko→PresentBridge→drain 経路で運ぶ
/// `ShowSurface{shell_target(0),0}` を apply するとシェルが**非表示→surface0 の実描画**へ遷移する
/// （`opaque_count>0`）ことを固定する。観測境界を実描画→readback まで延ばす（R8.2）。
///
/// # defect #5 の檻（シェルは初回 `\s` まで非表示）
///
/// 旧 DD-9 は attach 時にシェル初期面（scope0=surface0／scope>=1=surface10）を焼き込んでいたが、実機#5
/// で「起動時に規定面が一瞬ちらつく」欠陥が判明した。SSP 互換の既定は「シェル表示なし（-1）」であり、
/// 初回シェル表示は最初の `\s` cue が駆動する。本ケースは attach 直後の shell `read_back` が Err
/// （供給面未生成＝合成面なし＝透過）であること、`\s[0]` 適用でのみシェルが非表示→実描画へ遷移する
/// ことを檻に入れて回帰を防ぐ。バルーンは文字層スロット取得のため attach で面 0 の `ShowSurface` を
/// 保つが、その手前で可視性が外部所有へ移るため画面には出ない（不可視のままの確立・
/// `areka-P0-balloon-visibility` Requirement 1.1/1.3）。起動直後の不可視そのものは
/// [`spine_boot_leaves_all_balloons_invisible_with_established_slot_and_surface`] が主張する。
#[test]
fn spine_s1_boot_to_display_attaches_all_targets_with_opaque_readback() {
    let mut harness = SpineHarness::boot(r"\s[0]\e"); // \b-free（シェル面 cue \s[0] のみ）

    // attach フェーズ: DD-12 の planned==attached==2 を装着サマリで観測（縮退がバグを隠さない檻）。
    // attach は Tick 非依存（GPU 資源＋GhostWindows ゲートのみ）ゆえ boot 直後に直接駆動する。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter()
            .any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "DD-12: 計画件数＝実装着件数（planned=2 attached=2・期待 scope 数の全 target 完了）が観測できない: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach で ERROR が発火（装着失敗・log-first）: {logs:?}"
    );

    // (a-2) 文字層 k 再追従の再利用源（emo-dpi-scaling D11-3・R8.1）: attach 相は装着した各 scope の
    //       `BalloonModel` を `Emo2Wiring` へ記憶する。文字層スケール相（`run_text_scale_phase`）はこれを再利用して
    //       binding を組み直す——記憶が無ければ再追従は model 不在で skip され、文字だけが旧 k に
    //       取り残される（6.5 一次実走の欠陥）。実 attach 経路で保持されることをここで固定する。
    assert_eq!(
        harness.wiring.balloon_model_scopes(),
        vec![0u32, 1],
        "attach 済み全 scope の BalloonModel が再追従用に記憶されている（D11-3）"
    );

    // (b-1) シェルは初回 `\s` cue まで非表示（defect #5）: attach 直後の shell target は供給面未生成
    //       ＝`read_back` Err（合成面なし＝透過）。attach で surface0/surface10 を焼き付けない。
    for (label, target) in [
        ("shell scope0", shell_target(0)),
        ("shell scope1", shell_target(1)),
    ] {
        assert!(
            harness.wiring.read_back_target(target).is_err(),
            "{label} は初回 \\s cue 前は非表示であるべき（供給面未生成・read_back Err・defect #5）"
        );
    }

    // (b-2) バルーンは attach で面 0 を不可視のまま確立する（文字層スロット取得のため `ShowSurface` を
    //       保持）。供給面は生成され実描画されるので readback は非全透明（R8.1/8.2/8.5）。
    for (label, target) in [
        ("balloon scope0", balloon_target(0)),
        ("balloon scope1", balloon_target(1)),
    ] {
        let px = harness.wiring.read_back_target(target).unwrap_or_else(|e| {
            panic!("{label} の read_back 失敗（不可視でも供給面は生成済みのはず）: {e:?}")
        });
        assert!(
            opaque_count(&px) > 0,
            "{label} の readback が全透明（attach が確立した面 0 が実描画されていない・R8.1/8.2/8.5）: len={}",
            px.len()
        );
    }

    // (c) 初回 `\s[0]` cue を実 sink 経路（ghost→sakura→seriko→PresentBridge→rx）で駆動し、shell 表示
    //     対象（偶数 TargetId）へ ShowSurface{shell_target(0),0,static_binds} が届くことを確認する（R8.2）。
    let mut received = tick_and_collect(&mut harness, 1);
    let show_idx = received
        .iter()
        .position(|c| matches!(c, PresentCommand::ShowSurface { target, .. } if *target == shell_target(0)))
        .unwrap_or_else(|| {
            panic!(
                "S1: 初回 \\s[0] のシェル ShowSurface{{shell_target(0)}} が受信列に無い: variants={:?}",
                received.iter().map(variant_name).collect::<Vec<_>>()
            )
        });
    let show = received.remove(show_idx);
    match &show {
        PresentCommand::ShowSurface {
            target, surface_id, ..
        } => {
            assert_eq!(
                *target,
                shell_target(0),
                "初回 \\s[0] は shell 表示対象（偶数 TargetId・DD-3）"
            );
            assert_eq!(
                *surface_id, 0,
                "surface_id は 0（\\s[0]・seriko 数値解決の透過）"
            );
        }
        _ => unreachable!("position で ShowSurface を選別済み"),
    }

    // 形状記録後に実 presenter へ apply（実描画→readback・R8.2）。初回 `\s` 適用で shell が非表示から
    // surface0 の実描画へ遷移する（hidden→shown・defect #5 の正しい表示駆動）。
    harness.wiring.apply_present(&mut harness.world, show);
    let after_show = harness
        .wiring
        .read_back_target(shell_target(0))
        .expect("初回 \\s[0] 適用後は shell scope0 の供給面が生成され read_back 可能");
    assert!(
        opaque_count(&after_show) > 0,
        "初回 \\s[0] 適用で shell scope0 が surface0 の実描画へ遷移（非表示→非全透明・R8.1/8.2/8.5）"
    );

    harness.shutdown_bounded();
}

/// `areka-P0-balloon-visibility` Requirement 1.1 / 1.6 の spine 檻（起動直後の不可視）: 実 boot 経路
/// （ghost→窓生成→GPU 資源→attach 相）で装着した**全 scope**のバルーンが、面 0・文字の配置先・
/// 供給面の実描画まで確立していながら**一度も可視になっていない**ことを主張する。
///
/// 同じ不変条件を frame 相当で見る檻は
/// `frame_attach_tests.rs::attach_establishes_balloons_invisible_with_slot_and_surface` にあり、
/// そちらは合成資産と合成 World で attach 相だけを駆動する。本ケースは実 fixture（emo2）の boot と
/// 実窓・実 GPU を通した spine 経路で同じ主張を置く（本番の装着順・資産解決を通す）。
///
/// # 主張が空虚にならない形
///
/// `target_visible` は `attach_target` 直後から `Some(false)` を返すため、不可視だけを見る主張は
/// 「そもそも何も確立していない」場合にも真になる。ここでは同じ scope について確立の証跡——
/// 面 0 の記録（`current_surface_id`）・文字の配置先（`text_slot_view`）・供給面の実描画
/// （`opaque_count > 0`）——を先に要求してから不可視を主張するので、確立が起きていない縮退では
/// 不可視の主張へ到達する前に落ちる。
///
/// attach が可視性を外部所有へ移す手順を失うと、直後の `ShowSurface` がそのまま可視化するため
/// 最後の `target_visible` が `Some(true)` になって落ちる。
#[test]
fn spine_boot_leaves_all_balloons_invisible_with_established_slot_and_surface() {
    let mut harness = SpineHarness::boot(r"\s[0]\e"); // \b-free（可視化の契機を台本側から与えない）

    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach で ERROR が発火（装着失敗・log-first）: {logs:?}"
    );
    let scopes = harness.wiring.balloon_model_scopes();
    assert_eq!(
        scopes,
        vec![0u32, 1],
        "前提: 両 scope の balloon 装着が成立している（片方でも欠けると総当たりが空虚になる）"
    );

    for scope in scopes {
        let target = balloon_target(scope);

        // 確立の証跡（不可視の主張を空虚にしないための対）。
        assert_eq!(
            harness.wiring.presenter().current_surface_id(target),
            Some(0),
            "scope{scope}: 面 0 が確立している（撤去ではなく不可視のままの確立）"
        );
        assert!(
            harness.wiring.presenter().text_slot_view(target).is_some(),
            "scope{scope}: 文字の配置先が確立している（可視化から分離）"
        );
        let px = harness.wiring.read_back_target(target).unwrap_or_else(|e| {
            panic!("scope{scope}: 確立済みバルーンの供給面が読み戻せない: {e:?}")
        });
        assert!(
            opaque_count(&px) > 0,
            "scope{scope}: 確立した面 0 が実描画されていない（確立自体が起きていない）: len={}",
            px.len()
        );

        // 本題: そこまで確立していてもなお可視性は付いていない。
        assert_eq!(
            harness.wiring.presenter().target_visible(target),
            Some(false),
            "scope{scope}: 起動直後のバルーンは不可視のまま（Requirement 1.1・全 scope 適用の 1.6）"
        );
    }

    harness.shutdown_bounded();
}

/// spine S3（`\b` 配送・R5.4/DD-5・R8.2）: `\b[-1]`→`\b[0]` を含む scripted OnBoot 台本を実 sink 経路
/// （`ghost`→`sakura`→`seriko`→`PresentBridge`→rx）で流し、受信 `PresentCommand` 列に
/// `Hide{balloon_target(0)}`→`ShowSurface{balloon_target(0), surface_id:0, binds:default}` が
/// **順序どおり**現れることをアサートする（受信列順序＝本ケースの観測完了条件）。続けて記録済み指令を
/// 実 presenter へ apply し、balloon target の `read_back` が非全透明（surface0 の実描画・R8.2）で
/// attach 初期面と同一バイトを再駆動することを確認する。
///
/// # readback 遷移の観測境界（実装事実の申し送り・CONCERNS 相当）
///
/// `EmoPresenter::apply_hide` は WUC visual の可視フラグを落とすのみで swap chain の供給面
/// （`source_tex`）は破棄しない。`read_back` はその供給面を直読みするため **Hide は readback の
/// バイトを変えない**（emo-present の `empty_composition_degrades_to_hidden_and_replies_ok` が
/// 同事実を固定＝Hide 縮退後も `read_back` は旧供給面長のまま成立）。加えて本ケースが見る scope0 の
/// バルーン系列（emo2-kakukaku の `balloons0.png`）は面 0 の 1 枚のみで、attach が確立する面も 0 ゆえ、
/// `\b[-1]`→`\b[0]` の前後で readback バイトは不変（両方 surface0）。よって「Hide→全透明」型の
/// ピクセル遷移は本経路では観測不能である。本テストは
/// (1) 受信列順序（R5.4 の本質・観測完了条件）と (2) apply 後の balloon readback が非全透明かつ attach
/// 初期面と同一（surface_id/binds が正しく貫通し実描画された証跡・R8.2）で `\b` 配送の貫通を檻に入れる。
#[test]
fn spine_s3_balloon_face_cue_delivers_hide_then_show_in_order() {
    let mut harness = SpineHarness::boot(r"\b[-1]\b[0]\e");

    // 先に attach（balloon target を生成・面 surface0 を不可視のまま確立）。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("attached=2")),
        "S3 前提: attach 完了（balloon target 生成）が観測できない: {logs:?}"
    );

    // attach 初期面（surface0）の balloon readback を基準として捕捉（非全透明）。
    let baseline = harness
        .wiring
        .read_back_target(balloon_target(0))
        .expect("attach 後の balloon read_back（初期面 surface0）");
    assert!(
        opaque_count(&baseline) > 0,
        "前提: attach 初期面（balloon surface0）は非全透明"
    );

    // scripted OnBoot talk を Tick 注入で駆動し、受信 PresentCommand を 2 件（Hide→ShowSurface）集める。
    let received = tick_and_collect(&mut harness, 2);
    assert_eq!(
        received.len(),
        2,
        "\\b[-1]\\b[0] は Hide→ShowSurface のちょうど 2 件を配送するはず（受信 {} 件・variants={:?}）",
        received.len(),
        received.iter().map(variant_name).collect::<Vec<_>>()
    );

    // 受信列順序（R5.4/DD-5）: 1 件目 Hide{balloon(0)}・2 件目 ShowSurface{balloon(0),0,default}。
    match &received[0] {
        PresentCommand::Hide { target, reply } => {
            assert_eq!(
                *target,
                balloon_target(0),
                "1 件目は balloon 表示対象の Hide（\\b[-1]）"
            );
            assert!(reply.is_none(), "reply は None（撃ちっぱなし）");
        }
        other => panic!("1 件目は Hide{{balloon}} のはず: {}", variant_name(other)),
    }
    match &received[1] {
        PresentCommand::ShowSurface {
            target,
            surface_id,
            binds,
            pattern,
            reply,
        } => {
            assert_eq!(
                *target,
                balloon_target(0),
                "2 件目は balloon 表示対象の ShowSurface（\\b[0]）"
            );
            assert_eq!(
                *surface_id, 0,
                "surface_id は 0（\\b[0]・seriko 解決済み数値の透過）"
            );
            assert_eq!(
                *binds,
                BindSet::default(),
                "binds は既定（空集合＝バルーン着せ替えなし・DD-5/R5.1）"
            );
            // 非退行（task 9.4・R5.4）: loop 不活性（Inert）経路の cue 由来 ShowSurface は pattern 寄与なし＝空
            // （PatternState 拡張前と観測等価）。ループを活性化する boot_live 経路でのみ pattern が載る。
            assert!(
                pattern.is_empty(),
                "loop 不活性経路の cue 由来 ShowSurface は pattern 空（従来と観測等価・R5.4）"
            );
            assert!(reply.is_none(), "reply は None（撃ちっぱなし）");
        }
        other => panic!(
            "2 件目は ShowSurface{{balloon,0,default}} のはず: {}",
            variant_name(other)
        ),
    }

    // 実 presenter へ apply（実描画→readback まで観測境界を延ばす・R8.2）。形状記録後に move で流す。
    let mut cmds = received.into_iter();
    let hide = cmds.next().expect("Hide");
    harness.wiring.apply_present(&mut harness.world, hide);
    // apply_hide は供給面を破棄しない → read_back は基準（surface0）のまま（可視フラグは readback に映らない）。
    let after_hide = harness
        .wiring
        .read_back_target(balloon_target(0))
        .expect("Hide 後も balloon 供給面は保持され read_back 可能");
    assert_eq!(
        after_hide, baseline,
        "apply_hide は swap chain 供給面を破棄しない（read_back は供給面直読み・可視フラグ非反映）"
    );

    let show = cmds.next().expect("ShowSurface");
    harness.wiring.apply_present(&mut harness.world, show);
    let after_show = harness
        .wiring
        .read_back_target(balloon_target(0))
        .expect("ShowSurface{0} 後の balloon read_back");
    assert!(
        opaque_count(&after_show) > 0,
        "ShowSurface{{0}} 適用後の balloon は非全透明（surface0 の実描画・R8.2）"
    );
    assert_eq!(
        after_show, baseline,
        "\\b[0]→ShowSurface{{balloon,0,default}} は attach 初期面（surface0）と同一バイトを再駆動する（surface_id/binds の貫通証跡）"
    );

    harness.shutdown_bounded();
}

/// spine S4（`\b` なし完走・R5.5・R1 系）: `\b` を含まない OnBoot 台本が S1 経路（boot→表示）を完走し、
/// かつ受信 `PresentCommand` 列に **balloon 表示対象（奇数 TargetId）宛の指令が一切現れない**
/// （＝`\b` 由来の面切替が無い）ことを固定する。emo2 のバルーン fixture（`emo2-kakukaku`）では scope
/// ごとに別系列が解決され（scope0＝`balloons0.png`／scope1＝`balloonk0.png`）、いずれの系列も面 0 の
/// 1 枚だけを持つ。加えて本ケースの OnBoot 台本自体が `\b` を含まないため、どの scope でもバルーン
/// 面切替なしで完走する（R5.5）。`\s[0]` はシェル面指令（偶数 TargetId）を 1 件生むため、
/// 「talk が実際に流れたが balloon 面切替は無い」を受信列で決定論的に区別できる。
#[test]
fn spine_s4_balloon_free_onboot_completes_without_balloon_face_switch() {
    let mut harness = SpineHarness::boot(r"\s[0]\e"); // \b-free（シェル面 cue のみ）

    // S1 経路: attach 完走（planned==attached==2）＋ shell/balloon readback 非全透明。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter()
            .any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "S4: boot→表示（attach 完走・planned=2 attached=2）が観測できない: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach で ERROR なし: {logs:?}"
    );
    // シェルは初回 `\s` cue まで非表示（defect #5・2026-07-13 実機#5）: attach 直後は供給面未生成
    // ＝`read_back` Err（合成面なし＝透過）。attach で surface0 を焼き付けない。
    assert!(
        harness.wiring.read_back_target(shell_target(0)).is_err(),
        "shell scope0 は初回 \\s cue 前は非表示であるべき（供給面未生成・read_back Err・defect #5）"
    );
    // バルーンは attach で面 0 を不可視のまま確立する（文字層スロット取得のため `ShowSurface` を
    // 保持）＝供給面は実描画されており非全透明。
    let balloon_px = harness
        .wiring
        .read_back_target(balloon_target(0))
        .unwrap_or_else(|e| panic!("balloon scope0 の read_back 失敗: {e:?}"));
    assert!(
        opaque_count(&balloon_px) > 0,
        "balloon scope0 の readback が全透明（attach が確立した面 0 が実描画されていない）"
    );

    // OnBoot talk（\s[0]）を駆動し、少なくとも 1 件（シェル面指令）を受信＝talk が実際に流れたことを担保。
    let mut received = tick_and_collect(&mut harness, 1);
    assert!(
        !received.is_empty(),
        "\\b なし OnBoot 台本の talk が有界内に発火しない（boot→talk 経路が通っていない）"
    );
    // settle 前段（Tick 注入だけ・打ち切り条件を持たない）: 旧ループと同じ注入列
    // `1_000_000..1_000_000 + 5_000` を**毎回すべて**注入し、各注入の直後に回収する。範囲は
    // リテラルの固定列ゆえ環境の速さに依らず常に同一で、時刻が観測を追い越さない（要件 4.3）。
    for now in 1_000_000u64..1_000_000 + 5_000 {
        harness.inject_dispatcher_tick(now);
        received.extend(harness.wiring.drain_received());
    }
    // settle 後段（回収だけ・時刻は進めない）: 残余（万一の balloon 指令）を、反復回数ではなく
    // 壁時計の最小持続と連続空観測で有界に回収する（要件 4.2・4.5）。負荷下でも回収機会が
    // 縮まないので、この不在主張が空虚な緑になりにくい。
    settle_bounded(|| {
        let got = harness.wiring.drain_received();
        let n = got.len();
        received.extend(got);
        n
    });

    // R5.5: 受信列に balloon 表示対象（奇数 TargetId）宛は一切現れない（`\b` 由来面切替なしで完走）。
    for cmd in &received {
        if let Some(t) = present_command_target(cmd) {
            assert_eq!(
                t.0 % 2,
                0,
                "\\b なし台本で balloon 表示対象（奇数 TargetId）宛の指令が現れた（面切替 leak・R5.5 違反）: {:?} / variant={}",
                t,
                variant_name(cmd)
            );
        }
    }

    harness.shutdown_bounded();
}
