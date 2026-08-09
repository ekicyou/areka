use super::test_support::{bump_char_window_dpi, opaque_count, variant_name};
use super::{
    capture_logs, run_attach_phase, run_dpi_phase, shell_target, spin_wait_until, Duration,
    Instant, LoopRng, PresentCommand, SpineHarness, SPIN_WAIT,
};

// ===========================================================================
// task 9.4 — まばたきスモーク（direct send_tick 注入 → SERIKO ループ → pattern 搬送指令）
//
// 本 task の主眼はハーネスの ripple 修正（spawn_seriko arity／loop_tables／ShowSurface.pattern）＋
// 直接 tick 注入配線（`SerikoSink::send_tick`）＋既存テスト非退行（loop 不活性経路）だが、direct
// send_tick が実 emo2 表のループへ届き pattern を載せた表示指令を生む配線を最小スモークで裏付ける。
// 実 kero/sakura まばたき 1 周の PresentCommand 列 golden（2106→2110→-1→ベース復帰）と R3.4 default-OFF
// 対照は task 10.2 が本スモークの上に構築する（本 task では作らない）。
// ===========================================================================

/// 常時発火 rng（1/N 抽選で必ず 0 を返す＝毎境界で抽選通過・actor.rs／looper.rs `always_fire` と同旨）。
///
/// 「固定注入乱数列」の最小形（発見的 entropy 非依存・R7.1/7.2）。まばたきスモークは「発火が起きること」
/// のみを檻に入れるため定数 0 で足りる（発火順序・回数を厳密に固定する full golden は task 10.2）。
fn always_fire_rng() -> LoopRng {
    Box::new(|_bound: u32| 0)
}

/// spine まばたきスモーク（R7.1/7.2/7.3・DD・task 9.4）: 実 emo2 shell 表＋固定注入乱数（常時発火）で
/// `boot_live` し、kero まばたきの `interval,random,4` アニメ（pattern0=2106／pattern1=2110／pattern3=-1）を
/// 持つ surface 2100 を `\s[2100]` で表示させたのち、`SerikoSink::send_tick` を**直接注入**（loop ticker
/// 不起動・sleep 不使用）して 1000ms 絶対グリッド境界を跨がせ、seriko ループが **pattern を載せた**
/// ShowSurface{shell_target(0),2100} を PresentBridge→rx へ発行することを固定する。direct send_tick →
/// LoopRuntime → emit_display → adapter → rx の end-to-end 配線が spine で生きていることの最小自動檻。
///
/// # 表示中ゲート（loop は Show 済み slot のみ評価・R6.1/2.1）
///
/// ループは表示中の slot に対してのみアニメ評価する。まず `\s[2100]` cue を dispatcher tick で駆動し
/// rx に ShowSurface{2100} が現れる（＝seriko が ScopeStates に scope0 shell=surface2100 を記録済み）まで
/// 待ってから send_tick を注入する。dispatcher tick は talk/cue clock を進めるのみで seriko ループには
/// 届かない（ghost 側にループ結線なし）ため、ループ発火はこの直接注入 send_tick だけが供給する。
///
/// # 決定論（注入時刻＋注入乱数のみ・R7.2/7.3）
///
/// 起動 tick（now=0・境界初期化・非跨ぎ・無発行）→ 40ms 刻みで進め、境界跨ぎ（1000/2000/…）で常時発火 rng が
/// 抽選通過→pattern 進行。boot→talk→sink やスレッド伝播の非同期遅延は有界待機（deadline＋poll-backoff
/// sleep・R7.9）で吸収する（時刻源は注入のみ＝壁時計は有界性にしか使わない）。
#[test]
fn spine_blink_smoke_send_tick_drives_loop_pattern_command() {
    // 実表＋常時発火固定 rng でループ活性化（既存テストは Inert＝非退行・本テストのみ Live）。
    let mut harness = SpineHarness::boot_live(r"\s[2100]\e", always_fire_rng());

    // surface2100（kero まばたき random,4）を表示させる: \s[2100] cue を dispatcher tick で駆動し、
    // rx に shell ShowSurface{2100} が現れる（＝seriko が表示中 slot を記録済み）まで有界待機。
    // 有界性は壁時計 deadline（[`SPIN_WAIT`]）＋200µs poll-backoff sleep（R7.9・根拠は `drive_shell_shown` の doc）。
    // 観測 `shown` はラッチ・台本 `\s[2100]\e` は全 cue `at=0.0`＝観測を壊す後続 cue なし（R7.8 非該当）。
    let mut shown = false;
    let deadline = Instant::now() + SPIN_WAIT;
    let mut now = 0u64;
    loop {
        now += 1;
        harness.inject_dispatcher_tick(now);
        for cmd in harness.wiring.drain_received() {
            if matches!(&cmd, PresentCommand::ShowSurface { target, surface_id, .. }
                if *target == shell_target(0) && *surface_id == 2100)
            {
                shown = true;
            }
        }
        if shown || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    assert!(
        shown,
        "\\s[2100]（kero まばたき surface）の初回シェル表示が有界内に rx へ現れない（表示中ゲート前提不成立）"
    );

    // send_tick を直接注入して 1000ms 絶対グリッド境界を跨がせる（loop ticker 不起動・**時刻前進は
    // 注入 Tick のみ**＝この意味で sleep 不使用。待機の poll-backoff sleep は R7.9 の明示例外）。
    // pattern を載せた ShowSurface{shell_target(0),2100} が現れるまで有界待機（sub 秒進行＋境界跨ぎの双方を送る）。
    // 有界性は壁時計 deadline（[`SPIN_WAIT`]）＋200µs poll-backoff sleep（R7.9・根拠は `drive_shell_shown` の doc）。
    // 観測 `pattern_carrying` はラッチ（seriko tick の時刻前進は発火機会を増やすのみ＝R7.8 非該当）。
    let mut pattern_carrying = false;
    let mut now = 0u64;
    harness.inject_seriko_tick(now); // 起動 tick（境界初期化・非跨ぎ・無発行）
    let deadline = Instant::now() + SPIN_WAIT;
    loop {
        now += 40; // 小刻みに進め境界跨ぎ（1000ms グリッド）と pattern 進行（sub 秒）の双方を供給
        harness.inject_seriko_tick(now);
        for cmd in harness.wiring.drain_received() {
            if let PresentCommand::ShowSurface {
                target,
                surface_id,
                pattern,
                ..
            } = &cmd
            {
                if *target == shell_target(0) && *surface_id == 2100 && !pattern.is_empty() {
                    pattern_carrying = true;
                }
            }
        }
        if pattern_carrying || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    assert!(
        pattern_carrying,
        "direct send_tick が実 emo2 ループを駆動して pattern 搬送 ShowSurface{{shell_target(0),2100}} を発行しない（send_tick→LoopRuntime→emit→adapter→rx 配線が死んでいる？）"
    );

    harness.shutdown_bounded();
}

// ===========================================================================
// task 10.2 — 実 emo2 まばたき 1 周の PresentCommand 列 golden（kero/sakura）＋ R3.4 既定 OFF 対照
//
// 9.4 の `boot_live`/`inject_seriko_tick`/`always_fire_rng` の上に、実 emo2 fixture の実測アニメ
// （kero surface2100 `interval,random,4` 2106/2110/-1・sakura surface1000 `interval,bind+random,4`
// 1400 系 1412/1411/1410）を固定注入乱数＋scripted `send_tick` で 1 周歩かせ、発行される
// `PresentCommand` 列の **厳密 golden**（pattern 搬送 surface id の完全一致）を檻に入れる。
// あわせて R3.4「fixture 既定 OFF」の対照——`\![bind]` を通さない既定では always_fire でも bind
// ゲートが抽選を塞ぎ **一切発行しない**——を負の檻として固定する（design Testing Strategy E2E-1/2）。
//
// 全て決定論（実 emo2 asset fixture＋固定注入 rng＋scripted send_tick・sleep 不使用・Close→join）。
// 実 SHIORI/pasta 非依存（surface は注入 cue で表示・実機/実 DPI サインオフは task 10.3）。
// ===========================================================================

/// `cmd` が scope0 shell 宛の `ShowSurface{shown_surface}` であることを検証し、その pattern が
/// `anim_id` に載せる現在コマ surface id を返す（コマ不在＝ベース復帰は `None`）。
///
/// golden の各 tick が運ぶのは常に「表示中 surface（`shown_surface`）の ShowSurface に、まばたき
/// アニメの現在コマを `pattern` へ載せたもの」。表示対象（偶数 TargetId＝shell）と表示中 surface の
/// 透過を都度 assert し、可変部（pattern のコマ surface id）を返して呼び手が golden 照合する。
fn shell_pattern_frame(cmd: &PresentCommand, shown_surface: u32, anim_id: u32) -> Option<u32> {
    match cmd {
        PresentCommand::ShowSurface {
            target,
            surface_id,
            pattern,
            ..
        } => {
            assert_eq!(*target, shell_target(0), "shell 表示対象（scope0・偶数 TargetId・DD-3）");
            assert_eq!(
                *surface_id, shown_surface,
                "表示中 surface（seriko 数値解決の透過・pattern はこの面のアニメに従属）"
            );
            pattern.get(anim_id).map(|f| f.surface_id)
        }
        other => panic!("ShowSurface を期待（golden の各 tick は面表示指令）: {}", variant_name(other)),
    }
}

/// dispatcher tick で OnBoot talk を駆動し、scope0 shell が `surface_id` を表示する（必要なら binds に
/// `require_bind` を含む）まで有界スピンする。観測できた指令は drain 済みゆえ、復帰後の rx は talk 由来
/// 指令について空——以降 `inject_seriko_tick` が発行する loop 指令だけを純粋に観測できる。
///
/// `require_bind` は `\![bind,...]` 貫通の証跡: bind 適用は表示中 scope で **binds 更新済みの Show 再発行**
/// （`apply_bind`→`BindApplyOutcome::Changed`）を生むため、「shell surface が該当 bind id を含んで表示された」
/// = current_binds へ当該 id が書き込まれた（＝bind ゲートが ON になった）ことの end-to-end 証跡になる。
///
/// # 有界性は壁時計 deadline ＋ poll-backoff sleep（R7.9）
///
/// 本関数は**実 async（SHIORI/pasta アクタ→seriko→[`PresentBridge`]）の到着をラッチで待つ純粋な待機
/// ループ**である。1 反復は「tick 注入＋drain」だけで極めて安価ゆえ、反復回数の上限は CPU 競合下
/// （大規模再ビルド直後の再スキャン圧など）で**数 ms で尽き**、製品コードが正常でも早合点で赤になる
/// （偽陽性）。ゆえに有界性は [`std::time::Instant`] deadline で与える（長さは [`SPIN_WAIT`] に統一
/// ——PR #96 が純粋ポーリング用に導入した猶予と同じ物差しを、本ハイブリッド群でも用いる）。
///
/// **R7.8（注入模擬時刻が観測窓を追い越して観測条件を壊すクラス）とは別クラス**であり、時刻の頭打ちでは
/// 直らない。判別根拠は 2 点——(a) 観測 `satisfied` は**ラッチ**で、一度成立したら後続 cue に壊されない。
/// (b) 本関数の呼出点の台本はいずれも `\s[...]\e` 系で `\w`／`\c` を含まず**全 cue が `at=0.0`** ＝観測を
/// 壊す後続 cue が構造的に存在しない。ゆえに注入時刻の前進は観測に有利にしか働かず、頭打ちは不要。
///
/// 譲歩は `std::thread::yield_now()` ではなく**短い `sleep` の poll-backoff** とする。Windows の
/// `yield_now` は `SwitchToThread`＝**同一プロセッサ**の ready スレッドにしか譲らず、飽和下では別コアの
/// worker へ譲れずに busy-spin が worker を CPU 飢餓させる。値 200µs は `areka-kanade` の
/// `drive_ticks_until_disconnect` で開発者裁定により根治として採用されているものと同じ。決定論檻の
/// 「no sleep」原則に対する**明示例外**（競合飢餓の根治には必須）。
fn drive_shell_shown(harness: &mut SpineHarness, surface_id: u32, require_bind: Option<u32>) {
    let deadline = Instant::now() + SPIN_WAIT;
    let mut satisfied = false;
    let mut now = 0u64;
    loop {
        now += 1;
        harness.inject_dispatcher_tick(now);
        for cmd in harness.wiring.drain_received() {
            if let PresentCommand::ShowSurface {
                target,
                surface_id: sid,
                binds,
                ..
            } = &cmd
            {
                if *target == shell_target(0)
                    && *sid == surface_id
                    && require_bind.is_none_or(|id| binds.contains(id))
                {
                    satisfied = true;
                }
            }
        }
        if satisfied || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    assert!(
        satisfied,
        "OnBoot talk が scope0 shell surface {surface_id}（require_bind={require_bind:?}）を有界内に表示しない（boot→talk→sink 経路不通 or bind 未貫通）"
    );
}

/// `now_ms` の seriko tick を 1 発直接注入し、ちょうど 1 件の `PresentCommand` が rx へ届くまで有界
/// スピンして返す（`spin_wait_until` の密 yield → backoff sleep）。golden の各コマ遷移 tick は変化
/// 1 件を発行する（6.1/6.2）ため、この直列注入→1 件回収で発行列の順序と内容を決定論的に照合できる。
/// 届かなければ件数 assert が落ちる（hang しない）。打ち切りは反復回数でなく [`spin_wait_until`] の
/// 時刻期限（注入は 1 発のみで各反復は系を進めない純粋なポーリング＝反復は経過時間の代理にならない）。
/// 観測 `buf` は累積＝ラッチゆえ R7.8 の追い越しは構造的に無い。
fn seriko_tick_expect_one(harness: &mut SpineHarness, now_ms: u64) -> PresentCommand {
    harness.inject_seriko_tick(now_ms);
    let mut buf: Vec<PresentCommand> = Vec::new();
    spin_wait_until(|| {
        buf.extend(harness.wiring.drain_received());
        !buf.is_empty()
    });
    assert_eq!(
        buf.len(),
        1,
        "golden の各 tick はちょうど 1 指令を発行する（now_ms={now_ms}・実受信 {} 件・variants={:?}）",
        buf.len(),
        buf.iter().map(variant_name).collect::<Vec<_>>()
    );
    buf.into_iter().next().expect("len==1 を確認済み")
}

/// spine E2E-1（kero まばたき 1 周 golden・R7.2/7.3・design Testing Strategy E2E-1）: 実 emo2 shell 表
/// ＋固定注入乱数（常時発火）で `boot_live` し、`\s[2100]`（kero まばたき surface・`interval,random,4`・
/// bind 非依存）を注入 cue で表示させたのち、`SerikoSink::send_tick` を直接注入して 1000ms 絶対グリッド
/// 境界を跨がせ 1 周を歩かせる。発行 `PresentCommand` 列の **厳密 golden**——pattern0=2106 → pattern1=2110
/// → `-1`（ベース復帰＝空 pattern）——を実 fixture 実測値（surfaces.txt `surface.append10,2100` の
/// `animation0` 2106(w0)/2110(w40)/-1(w80)・t=[0,40,120]）で固定する。
///
/// send_tick→LoopRuntime→emit_display→adapter→rx の end-to-end 配線が、実 emo2 表の実測列を
/// 忠実に運ぶことの決定論自動檻（実 SHIORI/pasta・GUI 非依存・実機/実 DPI サインオフは task 10.3）。
#[test]
fn spine_e2e_kero_blink_one_cycle_golden() {
    // 実表＋常時発火固定 rng でループ活性化（既存 spine テストは Inert＝非退行・本テストのみ Live）。
    let mut harness = SpineHarness::boot_live(r"\s[2100]\e", always_fire_rng());
    // 表示中ゲート前提: surface2100 を scope0 shell へ表示（seriko ScopeStates に記録）。bind 不要（Random）。
    drive_shell_shown(&mut harness, 2100, None);

    // 起動 tick（境界初期化・非跨ぎ・無発行）。以降は境界 1000 のみを跨いで 1 周を歩く（2000 未到達＝再抽選なし）。
    harness.inject_seriko_tick(0);

    // ── 1 周 golden（kero・-1 終端）: 2106 → 2110 → ベース復帰（空 pattern）。実 fixture t=[0,40,120]。 ──
    let c1 = seriko_tick_expect_one(&mut harness, 1000); // 発火＋elapsed0 → pattern0
    assert_eq!(
        shell_pattern_frame(&c1, 2100, 0),
        Some(2106),
        "kero 1 周 pattern0=2106（実 fixture surface.append10,2100 animation0）"
    );
    let c2 = seriko_tick_expect_one(&mut harness, 1040); // elapsed40 → pattern1
    assert_eq!(
        shell_pattern_frame(&c2, 2100, 0),
        Some(2110),
        "kero pattern1=2110（wait40・実 fixture）"
    );
    let c3 = seriko_tick_expect_one(&mut harness, 1120); // elapsed120 → -1 Stopped → ベース復帰
    assert_eq!(
        shell_pattern_frame(&c3, 2100, 0),
        None,
        "kero pattern3=-1 到達でコマ除去＝ベース復帰（要件 4.3）"
    );
    // ベース復帰は「空 pattern の ShowSurface」——sakura の末尾残留（1410 が残る）との決定的な対照点。
    match &c3 {
        PresentCommand::ShowSurface { pattern, .. } => assert!(
            pattern.is_empty(),
            "kero の -1 ベース復帰は空 pattern（要件 4.3・sakura 残留と対照）"
        ),
        other => panic!("ShowSurface を期待: {}", variant_name(other)),
    }

    harness.shutdown_bounded();
}

/// spine E2E-2a（sakura まばたき 1 周 golden・bind ON 貫通・R7.2/9.2・design Testing Strategy E2E-2）: 実 emo2
/// shell 表＋固定注入乱数（常時発火）で `boot_live` し、`\s[1000]`（sakura 着せ替え surface・`bind+random,4`
/// のまばたき animation1400 を持つ）＋`\![bind,まばたき,通常,1]`（実 sakura bindgroup 貫通）を OnBoot talk で
/// 流して 1400 bindgroup を **ON** にしたのち、`send_tick` 直接注入で 1 周を歩かせる。発行 `PresentCommand` 列の
/// **厳密 golden**——pattern1=1412 → pattern2=1411 → pattern3=1410（**残留**・`-1` なし）——を実 fixture 実測値
/// （surfaces.txt surface1000 `animation1400` 1412(w0)/1411(w150)/1410(w22)・t=[0,150,172]）で固定する。
///
/// `\![bind,まばたき,通常,1]` は OnBoot talk 内で sakura compile → Custom キャリア cue → broadcast →
/// seriko `apply_bind`（`bind_resolver.resolve(Sakura,"まばたき","通常")==1400`）で current_binds へ 1400 を
/// 書き込む実経路を通る（mayuna 成果物・read-only 参照）。`drive_shell_shown(.., Some(1400))` が binds に 1400 を
/// 含む Show 再発行の観測で貫通を担保する。kero（`-1`→空 pattern）と対照的に末尾コマが残留する（要件 4.4）。
#[test]
fn spine_e2e_sakura_blink_after_bind_one_cycle_golden() {
    // \s[1000]（sakura 着せ替え surface）＋ \![bind,まばたき,通常,1]（1400 bindgroup ON）。常時発火 rng で
    // ループ活性化（bind ゲート ON の 1400 のみ発火・半目 1401/ジトー 1402 は OFF ゆえ非発火）。
    let mut harness = SpineHarness::boot_live(r"\s[1000]\![bind,まばたき,通常,1]\e", always_fire_rng());
    // 表示中＋bind ON 前提: scope0 shell surface1000 が binds に 1400 を含んで表示される
    // （\![bind] 貫通で current_binds へ 1400 が書き込まれた end-to-end 証跡＝bind 再発行 Show）。
    drive_shell_shown(&mut harness, 1000, Some(1400));

    // 起動 tick（境界初期化）。以降は境界 1000 のみを跨ぎ 1 周を歩く（2000 未到達＝再抽選なし）。
    harness.inject_seriko_tick(0);

    // ── 1 周 golden（sakura・末尾残留）: 1412 → 1411 → 1410（残留・-1 なし）。実 fixture t=[0,150,172]。 ──
    let c1 = seriko_tick_expect_one(&mut harness, 1000); // 発火＋elapsed0 → pattern1
    assert_eq!(
        shell_pattern_frame(&c1, 1000, 1400),
        Some(1412),
        "sakura 1 周 pattern1=1412（実 fixture surface1000 animation1400・先頭 wait0）"
    );
    let c2 = seriko_tick_expect_one(&mut harness, 1150); // elapsed150 → pattern2
    assert_eq!(
        shell_pattern_frame(&c2, 1000, 1400),
        Some(1411),
        "sakura pattern2=1411（wait150・実 fixture）"
    );
    let c3 = seriko_tick_expect_one(&mut harness, 1172); // elapsed172 → pattern3 末尾非負 → 残留
    assert_eq!(
        shell_pattern_frame(&c3, 1000, 1400),
        Some(1410),
        "sakura pattern3=1410 残留（-1 なし末尾＝FinishedResidual・要件 4.4）"
    );
    // 末尾は残留ゆえ空でない——kero の -1 ベース復帰（空 pattern）との決定的な対照点。
    match &c3 {
        PresentCommand::ShowSurface { pattern, .. } => assert!(
            !pattern.is_empty(),
            "sakura の末尾は最終コマ残留（空でない・要件 4.4・kero の -1 と対照）"
        ),
        other => panic!("ShowSurface を期待: {}", variant_name(other)),
    }

    harness.shutdown_bounded();
}

/// spine E2E-2b（R3.4「fixture 既定 OFF」の対照檻・design Testing Strategy E2E-2）: fixture 既定
/// （`\![bind]` を **通さない**＝まばたき bindgroup 1400/1401/1402 は全 OFF）で surface1000 を表示し、
/// 常時発火 rng で境界を複数跨いでも **一切発行しない**ことを固定する。bind ゲート（`BindRandom` は
/// bindgroup ON のときだけ `should_fire` を呼ぶ・要件 3.1）が抽選そのものを塞ぐため、抽選 → 再生 → pattern
/// 搬送が起きない。
///
/// # always_fire で「ゲートが塞ぐ」を積極証明する（R3.4 の核心）
///
/// 乱数を常時発火（1/N 抽選が呼ばれれば必ず通過）にしておくことで、もし bind ゲートが leak すれば
/// 1400/1401/1402 は **必ず**発火し pattern 搬送指令が現れる。それが現れない＝発行ゼロは「抽選が呼ばれて
/// いない（ゲートが塞いだ）」ことの証跡になる（sakura bind ON 版〔上〕が同じ rng+tick で発火するのと対照）。
/// surface1000 のまばたきは全て `bind+random`（無条件 `random` は無い）ため、既定 OFF では何も動かない。
#[test]
fn spine_e2e_sakura_blink_default_off_emits_nothing() {
    // fixture 既定（bind OFF・\![bind] なし）で surface1000 を表示。always_fire でもゲートが塞ぐ＝発行ゼロ。
    let mut harness = SpineHarness::boot_live(r"\s[1000]\e", always_fire_rng());
    drive_shell_shown(&mut harness, 1000, None); // bind なし＝1400/1401/1402 は全 OFF（R3.4 既定）

    // 境界を複数跨ぐ seriko tick を注入し、発行が一切現れないことを固定する（起動 tick→1000/2000/…の境界跨ぎ）。
    harness.inject_seriko_tick(0); // 起動 tick（境界初期化）
    let mut emitted: Vec<PresentCommand> = Vec::new();
    for now in [1000u64, 2000, 3000, 4000, 5000] {
        harness.inject_seriko_tick(now); // 各々 1000ms 絶対グリッド境界を跨ぐ
        // 有界 settle drain（spine_s4 の負検証と同流儀・sleep 不使用・yield_now のみ）。
        for _ in 0..5_000 {
            emitted.extend(harness.wiring.drain_received());
            std::thread::yield_now();
        }
    }
    // R3.4 の檻: bind ゲート OFF は always_fire でも抽選を塞ぐ＝発行ゼロ（ゲート leak なら pattern 搬送指令が漏れる）。
    assert!(
        emitted.is_empty(),
        "R3.4 既定 OFF: bind ゲート OFF は always_fire でも一切発行しない（ゲート leak 検出・実受信 {} 件・variants={:?}）",
        emitted.len(),
        emitted.iter().map(variant_name).collect::<Vec<_>>()
    );

    harness.shutdown_bounded();
}

// ===========================================================================
// 要件 4.3（進行中挙動が DPI 変化を跨いで失われない）の実経路檻
//
// 既存の DPI 変化 spine 2 本（`spine_dpi_change_refreshes_balloon_text_scale_on_real_attach`／
// `spine_dpi_change_while_balloon_hidden_lands_on_next_show`）はいずれも `SpineHarness::boot`
// ＝`LoopDriver::Inert` で起動しており、**活性ループと DPI 変化が一度も同居していない**。
// ゆえに要件 4.3 の 3 つの主張のうち (a) クラッシュ・表示消失なし／(b) 文字の状態保存は檻に
// 入っていたが、(c)「SERIKO ループが DPI 変化を跨いで進行し続ける」は
// 「ループ状態は presenter の外（seriko worker）にある」という**構造からの帰結**にとどまり、
// 一度も観測されていなかった。以下がその空白を閉じる。
// ===========================================================================

/// シェル target の適用 k（`applied_scale`）を読む短縮（`None`＝未表示は前提違反ゆえ panic）。
fn shell_applied_scale(harness: &SpineHarness, scope: u32) -> f32 {
    harness
        .wiring
        .presenter()
        .applied_scale(shell_target(scope))
        .expect("表示済みの shell target は適用 k を持つ")
}

/// dispatcher tick で OnBoot talk を駆動しつつ、届いた `PresentCommand` を**実 presenter へ適用**し、
/// scope0 shell が `surface_id` を表示する（＝実描画が成立する）まで有界スピンする。
///
/// [`drive_shell_shown`] は観測のみで指令を捨てるため presenter 側に表示が成立しない。ループ継続の
/// 檻は「表示が生きていること」を readback で見るため、適用まで行うこの版が要る。
///
/// 有界性は [`drive_shell_shown`] と同じく**壁時計 deadline（[`SPIN_WAIT`]）＋ 200µs poll-backoff sleep**（R7.9）。
/// 反復回数上限が CPU 競合下で数 ms で尽きる偽陽性、R7.8 との判別根拠（観測 `shown` はラッチ・呼出点の
/// 台本は全 cue `at=0.0` ＝観測を壊す後続 cue が無い）、`yield_now`（Windows では `SwitchToThread` ＝同一
/// プロセッサにしか譲らない）を避ける理由、および「no sleep」原則への明示例外である旨は
/// [`drive_shell_shown`] の doc を参照。
fn drive_shell_shown_and_presented(harness: &mut SpineHarness, surface_id: u32) {
    let deadline = Instant::now() + SPIN_WAIT;
    let mut shown = false;
    let mut now = 0u64;
    loop {
        now += 1;
        harness.inject_dispatcher_tick(now);
        for cmd in harness.wiring.drain_received() {
            if matches!(&cmd, PresentCommand::ShowSurface { target, surface_id: sid, .. }
                if *target == shell_target(0) && *sid == surface_id)
            {
                shown = true;
            }
            harness.wiring.apply_present(&mut harness.world, cmd);
        }
        if shown || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    assert!(
        shown,
        "OnBoot talk が scope0 shell surface {surface_id} を有界内に表示しない（boot→talk→sink 経路不通）"
    );
}

/// seriko tick を 1 発注入して 1 件の指令を回収し、**実 presenter へ適用**したうえで、その指令が
/// 運んだ pattern の現在コマ surface id（コマ不在＝ベース復帰は `None`）と適用後の readback を返す。
fn seriko_tick_apply_one(
    harness: &mut SpineHarness,
    now_ms: u64,
    shown_surface: u32,
    anim_id: u32,
) -> (Option<u32>, Vec<u8>) {
    let cmd = seriko_tick_expect_one(harness, now_ms);
    let frame = shell_pattern_frame(&cmd, shown_surface, anim_id);
    harness.wiring.apply_present(&mut harness.world, cmd);
    let px = harness
        .wiring
        .read_back_target(shell_target(0))
        .unwrap_or_else(|e| {
            panic!("ループ指令適用後の shell scope0 read_back 失敗（表示が消えた・要件 4.3）: {e:?}")
        });
    assert!(
        opaque_count(&px) > 0,
        "ループ指令適用後の shell readback が全透明（表示消失・要件 4.3）: len={}",
        px.len()
    );
    (frame, px)
}

/// **活性 SERIKO ループを跨ぐ DPI 変化**（要件 4.3 の残る主張 (c)・R7.2/8.1/8.2）: 実 emo2 表＋常時発火
/// 固定 rng で `boot_live` し、kero まばたき（surface2100 `animation0`＝2106/2110/`-1`）を 1 周の**途中まで**
/// 歩かせた状態で**シェル窓の DPI を変える**。その後——
///
/// 1. DPI 変化直後の再表示が**進行中のコマ（2106）を載せたまま**新 k で成立する
///    （＝`refresh_scale` が `last_show` の pattern を捨てない＝進行中挙動を失わない）。
/// 2. ループは**リセットも停止もせず同じ 1 周の続き**（2110 → `-1` ベース復帰）を、実 fixture 実測の
///    golden どおりに発行し続ける（先頭 2106 へ戻らない・無発行にならない）。
/// 3. その全区間で shell の `read_back` が成立し全透明にならない（クラッシュ・表示消失なし）。
/// 4. DPI 変化以降の表示は一貫して**新 k の物理寸**（旧 k の絵が残らない・k 変化が実描画へ届いている）。
///
/// # 既存 DPI 檻との差（なぜ本ケースが要るか）
///
/// 既存 2 本は `LoopDriver::Inert`（ループ完全不活性）で、変化させるのも**バルーン**窓の DPI である。
/// SERIKO ループが載るのはシェル表示スロットゆえ、活性ループ×シェル窓 DPI 変化という本番の組み合わせは
/// どこにも存在しなかった。本ケースはその 1 点だけを足す（(a)(b) の重複観測はしない）。
///
/// # 決定論（sleep 不使用・注入時刻＋注入乱数のみ）
///
/// tick は `send_tick` 直接注入（loop ticker 不起動）、乱数は常時発火の固定注入列。DPI は
/// [`bump_char_window_dpi`] が現在値と必ず異なる実機水準を選ぶ（「たまたま同値」で空虚化しない）。
///
/// # 実測の変異キル（2026-07-26・本ワークツリー）
///
/// - `EmoPresenter::refresh_scale` が再表示時に `last_show` の pattern を捨てて
///   `PatternState::default()` で `apply_show` する変異（＝DPI 変化で進行中の SERIKO コマを失う）:
///   `-p areka` 522 本中**本テストのみ**が落ち（他 521 本生存）、落ちる assert も狙いどおり
///   「DPI 変化直後の再表示が進行中コマを失いベース面へ戻っている」である。同変異は
///   `-p areka-emo-present`（91 本）では **1 本も落ちない**——`refresh_scale` の pattern 保存は
///   本テスト追加前まで repo 内のどの檻も観測していなかった（exclusive）。
/// - `apply_show` が k≠1 のとき `binds`／`pattern` を既定へ落とす変異では、本テストと
///   `areka-emo-present` の `show_surface_scales_layered_bind_and_pattern_content_with_single_k`
///   （要件 2.3 の同時追加檻）が**共倒れ**（shared）。既存 521 本は全生存。
#[test]
fn spine_dpi_change_during_live_seriko_loop_keeps_loop_progressing() {
    // 実表＋常時発火固定 rng でループ活性化（既存 DPI 檻は Inert＝この組み合わせは本ケースが初）。
    let mut harness = SpineHarness::boot_live(r"\s[2100]\e", always_fire_rng());

    // 実 attach（供給面・視覚を本番経路で生成）→ 表示中ゲート成立まで talk を駆動して実適用する。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("attached=2")),
        "前提: attach 完了が観測できない: {logs:?}"
    );
    drive_shell_shown_and_presented(&mut harness, 2100);

    let base_k0 = harness
        .wiring
        .read_back_target(shell_target(0))
        .expect("前提: 初回 \\s[2100] 適用で shell 供給面が生成される");
    assert!(
        opaque_count(&base_k0) > 0,
        "前提: DPI 変化前の shell が実描画されている"
    );
    let k_before = shell_applied_scale(&harness, 0);

    // ── 1 周の途中まで歩かせる（起動 tick → 境界 1000 で発火＋elapsed0 → pattern0=2106） ──
    harness.inject_seriko_tick(0); // 起動 tick（境界初期化・非跨ぎ・無発行）
    let (frame1, frame1_k0) = seriko_tick_apply_one(&mut harness, 1000, 2100, 0);
    assert_eq!(
        frame1,
        Some(2106),
        "前提: DPI 変化前に kero 1 周の pattern0=2106 まで進んでいる（実 fixture animation0）"
    );
    assert_ne!(
        frame1_k0, base_k0,
        "前提: 進行中コマが実際に表示画素を変えている（変わらなければ以降の継続観測が空虚）"
    );

    // ── ここでシェル窓の DPI が変わる（モニタ跨ぎ・表示スケール変更の決定論的代替） ──
    let new_dpi = bump_char_window_dpi(&mut harness, 0);
    run_dpi_phase(&mut harness.wiring, &mut harness.world);
    let k_after = shell_applied_scale(&harness, 0);
    assert_ne!(
        k_after, k_before,
        "前提: DPI={new_dpi} で shell target の適用 k が実際に変わる（変わらなければ本ケースは空虚）"
    );

    // (3)(4) 表示は消えず、新 k の物理寸で載り直している。
    let frame1_k1 = harness
        .wiring
        .read_back_target(shell_target(0))
        .unwrap_or_else(|e| panic!("DPI 変化直後の shell read_back 失敗（表示消失・要件 4.3）: {e:?}"));
    assert!(
        opaque_count(&frame1_k1) > 0,
        "DPI 変化直後の shell readback が全透明（表示消失・要件 4.3）: len={}",
        frame1_k1.len()
    );
    assert_ne!(
        frame1_k1.len(),
        frame1_k0.len(),
        "DPI 変化後も旧 k の物理寸のまま（k が実描画へ届いていない）"
    );

    // ── (2) ループは同じ 1 周の続きを歩き続ける（先頭へ戻らない・止まらない） ──
    let (frame2, frame2_k1) = seriko_tick_apply_one(&mut harness, 1040, 2100, 0);
    assert_eq!(
        frame2,
        Some(2110),
        "DPI 変化を跨いでもループは同じ 1 周の続き（pattern1=2110）を発行する（先頭 2106 へ戻さない＝リセットなし・要件 4.3）"
    );
    assert_eq!(
        frame2_k1.len(),
        frame1_k1.len(),
        "DPI 変化後のループ指令が旧 k の寸へ戻っている（k の一貫性が崩れた）"
    );
    assert_ne!(
        frame2_k1, frame1_k1,
        "コマ遷移が表示画素へ届いていない（ループは動いたが絵が更新されない）"
    );

    let (frame3, base_k1) = seriko_tick_apply_one(&mut harness, 1120, 2100, 0);
    assert_eq!(
        frame3, None,
        "DPI 変化を跨いだ 1 周が実 fixture どおり `-1` 終端（ベース復帰）まで到達する（要件 4.3/4.3 終端）"
    );
    assert_eq!(
        base_k1.len(),
        frame1_k1.len(),
        "ベース復帰も新 k の物理寸で成立する"
    );

    // (1) DPI 変化直後の再表示が**進行中のコマを載せたまま**だったことの決定的な対照:
    //     同じ k のベース復帰の絵（base_k1）と異なる＝2106 のコマが再表示に生きていた。
    //     `refresh_scale` が pattern を捨てて素の面を出す実装なら、ここで両者が一致して落ちる。
    assert_ne!(
        frame1_k1, base_k1,
        "DPI 変化直後の再表示が進行中コマを失いベース面へ戻っている（進行中挙動の喪失・要件 4.3）"
    );

    harness.shutdown_bounded();
}
