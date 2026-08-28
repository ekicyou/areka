// =============================================================================
// タグ入口の結線（task 6.2・要件 11.2）と、shell 設定由来の基底の起動時結線
// （task 6.3・要件 5.1／5.2）の決定論テスト
//
// この task が足すのは判断ではなく**配線**である。受け口も台帳も相の本体も既に在って
// 兄弟のテストが判断を全部押さえているので、ここで測るのは 3 点だけになる。
//
// 1. タグを含む台本が受け口へ届くと、指令が**台帳適用の相まで**通ること（到達性）。
// 2. その相が**維持系より前**に走ること（相順）。
// 3. 結線の字面そのもの（受け渡し口・入口の登録・受け渡し構造・相の呼出）。
//
// 3 を字面で押さえるのは、配線の欠けが挙動に現れにくいからである。呼出を丸ごと削っても
// 判断のテストは 1 本も赤くならず（task 4.2／5.1／5.2 で三度実証済み）、相順に至っては
// 「是正が 1 心拍ぶん遅れる」だけなので、実窓を持たない檻には**原理的に**映らない。
//
// 相順は 2 で挙動としても測る——ただし測れるのは「確定段へ `.before` で載せた仕事が
// 同じ巡のうちに維持系から見えるか」という**機構**であって、本番の `emo2_frame_system`
// がその形で載っていることは 3 の字面が受け持つ（本番の相は `Emo2Wiring` が挿さっていない
// World では丸ごと無操作なので、headless の檻からは観測できない）。
//
// 字面の走査は必ず**説明文を落とした本文**へ当てる。素の全文には本ファイルの説明の語も
// 相手ファイルの doc の語も入るので、当てる先を間違えると検査が恒真になる
// （task 3.1 の教訓。各檻の末尾に両方向の対照を置いてある）。
//
// task 6.3 が足す 2 本（⑷）も同じ性質を持つ——設定の値を起動の段まで運ぶ経路は、
// 途中の 1 行を落としても「設定を書いていないゴースト」と全く同じ挙動になるので、
// 挙動を見る檻には**原理的に**映らない。運ぶ側（`main.rs`）と据える側（`mod.rs`）の
// 字面をそれぞれ名指しで押さえる。据えた値が実際に効くことは兄弟の
// `frame/zorder_descript_tests.rs` が挙動で受け持つ。
// =============================================================================

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use std::sync::mpsc::channel;
use wintf::ecs::FrameFinalize;
use wintf::ecs::window::{ZOrderGroups, apply_zorder_group_maintenance};

use super::frame::run_zorder_drain_phase;
use super::zorder_cue::{ZOrderCueSink, ZOrderDirective};
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{spawn_ghost_windows, wire_zorder_pair};
use crate::placement::zorder_group_ledger::ZOrderGroupLedger;

// ---------------------------------------------------------------- 道具立て

/// `\![name,tokens...]` の汎用キャリア cue を組む（正準形＝`Custom` の String 配列）。
fn carrier_cue(name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

/// 文字の cue（台本の大半はこれで、受け口には全部届く）。
fn text_cue(text: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text(text.into()),
        duration: 0.0,
    }
}

/// 1 スコープぶんの合成配置（値は散らしただけで意味を持たない。この相は窓を動かさない）。
fn placement(scope: usize) -> ScopePlacement {
    let base = 100 * (scope as i32 + 1);
    ScopePlacement {
        scope,
        char_pos: PointPx { x: base, y: base },
        char_size: SizePx { w: 200, h: 300 },
        balloon_pos: PointPx {
            x: base + 220,
            y: base,
        },
        balloon_size: SizePx { w: 180, h: 120 },
        balloon_offset: PointPx { x: 220, y: 0 },
        balloon_limit: false,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 指定したスコープの窓だけを持つ World を組む（`GhostWindows` は Resource として載る）。
fn world_with_scopes(scopes: &[usize]) -> World {
    let mut world = World::new();
    let placements: Vec<ScopePlacement> = scopes.iter().map(|s| placement(*s)).collect();
    let titles = GhostTitles::from_scope_titles(
        scopes
            .iter()
            .map(|s| (*s, format!("scope-{s}")))
            .collect::<Vec<_>>(),
    );
    spawn_ghost_windows(&mut world, &placements, &titles);
    world
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 空白の連なりを 1 つに詰める（改行や字下げの入り方で檻が壊れないようにする）。
fn squeeze(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 字面の現れる位置（無ければその場で落として、檻の前提が崩れたことを名指しで告げる）。
fn index_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("檻の錨が本文に見当たらない（前提が崩れている）: {needle}"))
}

// ---------------------------------------------------------------------------
// ⑴ 到達性——タグを含む台本が台帳適用の相まで届く（完了状態）
// ---------------------------------------------------------------------------

/// 台本に混ざった `\![set,zorder,1,0]` が、受け口→受け渡し口→取り出しの相を通って
/// **台帳に載る**（task 6.2 の完了状態）。
///
/// 台本には他人宛の演出も文字も流れるので、同じ受け口へまとめて浴びせる。届いてよいのは
/// 1 本だけであり、他の cue は台帳にも受け口にも痕跡を残さない（要件 11.2）。
#[test]
fn t_zwi01_a_script_tag_reaches_the_ledger_through_the_wired_channel() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);

    // 台本の並び——文字・他人宛のタグ・担当外の選別子・そして自分宛のタグ 1 本。
    sink.emit(text_cue("あひる"));
    sink.emit(carrier_cue("move", &["10", "20"]));
    sink.emit(carrier_cue("set", &["windowstate", "minimize"]));
    sink.emit(carrier_cue("set", &["zorder", "1", "0"]));

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        ledger.groups().len(),
        1,
        "台帳へ載ったグループがちょうど 1 本ではない（届いていないか、担当外まで拾っている）"
    );
    let members: Vec<u32> = ledger.groups()[0]
        .members
        .iter()
        .map(|element| element.scope)
        .collect();
    assert_eq!(
        members,
        vec![1, 1, 0, 0],
        "台帳の要素列が書かれたとおり（手前が scope 1）になっていない"
    );

    // 射影まで通って受け口が出来ていること——ここまでが「相まで届く」の終端である。
    let groups = world
        .get_resource::<ZOrderGroups>()
        .expect("受け口（ZOrderGroups）が出来ていない＝射影まで届いていない");
    assert_eq!(
        groups.groups.len(),
        1,
        "受け口へ置かれたグループがちょうど 1 本ではない: {:?}",
        groups.groups
    );
    assert_eq!(
        groups.groups[0].members.len(),
        4,
        "実在する窓 4 枚が射影に載っていない"
    );
    assert!(
        groups.pending,
        "射影が動いた巡に印が立っていない（維持系が次の巡に動かない）"
    );
}

/// 逆側——重なりのタグを 1 本も含まない台本では、受け口の Resource すら作られない
/// （要件 11.2「担当が存在しないコマンドは従来どおり良性に読み飛ばされる」）。
///
/// 上の檻だけでは「何を浴びせても 1 本載る」形と区別が付かない。
#[test]
fn t_zwi02_a_script_without_the_tag_leaves_the_mechanism_absent() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);

    sink.emit(text_cue("あひる"));
    sink.emit(carrier_cue("move", &["10", "20"]));
    sink.emit(carrier_cue("set", &["windowstate", "minimize"]));
    sink.emit(carrier_cue("reset", &["windowstate"]));

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert!(
        ledger.groups().is_empty(),
        "担当外のコマンドで台帳が動いた（自己選別が漏れている）"
    );
    assert!(
        world.get_resource::<ZOrderGroups>().is_none(),
        "グループが 1 本も無い走行で受け口が作られた（既定＝非強制が構造で成り立っていない）"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 相順——確定段へ `.before` で載せた仕事は同じ巡のうちに維持系から見える
// ---------------------------------------------------------------------------

/// 確定段の stand-in——受け口の印を立てるだけの最小の仕事（本番では取り出しの相が立てる）。
fn raise_the_pending_mark(mut groups: ResMut<ZOrderGroups>) {
    groups.pending = true;
}

/// 本番と同じ順序で確定段を組んだ World（`wire_zorder_pair` が 3 本を先に載せる）。
///
/// 本番も `open_startup_window`（`wire_zorder_pair`）→ `wire_emo2_boot`（相の登録）の順で
/// あり、**登録の順は維持系のほうが先**である。順序指定を落とすと相は後ろへ回る。
fn wired_finalize_world() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_resource(ZOrderGroups::default());
    wire_zorder_pair(&mut world);
    world
}

/// 維持系より**前**に載せた仕事は、同じ巡のうちに維持系が読む。
///
/// 読めた証拠は印が降りていることである——維持対象のグループが 1 本も無い巡は、②の門を
/// 通った維持系が⑤で印を降ろす。維持系が先に走っていれば門は閉じたままで、後から立った
/// 印はそのまま残る（下の対の檻）。
#[test]
fn t_zwi03_work_placed_before_the_group_maintenance_is_seen_in_the_same_pass() {
    let mut world = wired_finalize_world();
    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        raise_the_pending_mark.before(apply_zorder_group_maintenance),
    );

    world.run_schedule(FrameFinalize);

    assert!(
        !world.resource::<ZOrderGroups>().pending,
        "維持系より前に立てた印が同じ巡で消費されていない（相順の機構が効いていない）"
    );
}

/// 逆側——維持系より**後ろ**に載せると、その巡は誰も読まない（是正が 1 心拍ぶん遅れる）。
///
/// 片側だけでは空虚である。印が常に降りる World でも、常に残る World でも、片方の主張は
/// 緑になる。この対があって初めて「印の値が相順を読んでいる」と言える。
#[test]
fn t_zwi04_work_placed_after_the_group_maintenance_is_one_heartbeat_late() {
    let mut world = wired_finalize_world();
    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        raise_the_pending_mark.after(apply_zorder_group_maintenance),
    );

    world.run_schedule(FrameFinalize);

    assert!(
        world.resource::<ZOrderGroups>().pending,
        "維持系より後ろに立てた印がその巡で消えている（対照が成立していない＝上の檻が空虚）"
    );
}

// ---------------------------------------------------------------------------
// ⑶ 結線の字面——削っても挙動に現れない 4 点を名指しで塞ぐ
// ---------------------------------------------------------------------------

/// 受け渡し口・入口の登録・受け渡し構造への引き渡しが、本番の結線に在る（task 6.2 の 3 点）。
#[test]
fn t_zwi05_the_boot_wires_the_channel_the_sink_and_the_handoff() {
    let raw = include_str!("mod.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains(
            "let (zorder_tx, zorder_rx) = std::sync::mpsc::channel::<ZOrderDirective>();"
        ),
        "指令の受け渡し口（チャネル 1 組）が本文に無い: {squeezed}"
    );
    assert!(
        squeezed.contains("let zorder_sink = ZOrderCueSink::new(zorder_tx);"),
        "受け口を送出端から組む行が本文に無い"
    );
    assert!(
        squeezed.contains(
            "sinks: vec![ Box::new(surface_sink), Box::new(clocked_text_sink), Box::new(move_sink), Box::new(lifecycle_sink), Box::new(zorder_sink), ],"
        ),
        "入口の登録（配送の sinks）が既存 4 本＋重なりの受け口の形になっていない"
    );
    assert!(
        squeezed.contains("lifecycle_rx, zorder_rx,"),
        "受信端が受け渡し構造（Emo2Wiring::new）へ渡されていない＝取り出しの相が読む先が無い"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("pub fn wire_emo2_boot("),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("完成済み 5 トラックのエンジン"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("完成済み 5 トラックのエンジン"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

/// 毎フレームの相は**維持系より前**へ載る（task 3.3 → 6.2 の必須事項）。
///
/// 後ろに載ると、窓が現れた巡の是正が最大 1 心拍ぶん遅れる。遅れるだけで結果は同じなので、
/// 挙動を見る檻には映らない——順序指定の字面そのものを名指しで押さえる。
#[test]
fn t_zwi06_the_frame_system_is_ordered_before_the_group_maintenance() {
    let raw = include_str!("mod.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains("emo2_frame_system.before(apply_zorder_group_maintenance)"),
        "相の登録に維持系より前という指定が無い（登録は維持系のほうが先なので、指定を落とすと相は後ろへ回る）"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("app.world().borrow_mut().world_mut().insert_non_send(wiring);"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("依存方向（レイヤ規律・design.md"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("依存方向（レイヤ規律・design.md"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

/// 相の呼出は既存の指令適用の相（`\![move]` の取り出し）の**直後**に 1 つだけ在る。
///
/// 呼出を丸ごと削っても判断の檻は 1 本も赤くならない（task 4.2／5.1／5.2 で三度実証）。
/// ここが唯一その削除を捕まえる。
#[test]
fn t_zwi07_the_frame_calls_the_zorder_drain_right_after_the_move_drain() {
    let raw = include_str!("frame.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains(
            "run_zorder_drain_phase(&wiring.zorder_rx, &mut wiring.zorder_ledger, world);"
        ),
        "取り出しの相の呼出が本文に無い（相順の所有者から呼ばれていない）"
    );
    assert_eq!(
        squeezed.matches("run_zorder_drain_phase(").count(),
        1,
        "取り出しの相の呼出がちょうど 1 つではない（二重に回すと指令が二度適用される）"
    );

    let move_at = index_of(&squeezed, "run_move_drain_phase(&wiring, world);");
    let zorder_at = index_of(&squeezed, "run_zorder_drain_phase(");
    assert!(
        move_at < zorder_at,
        "取り出しの相が `\\![move]` の相より前に置かれている（設計の相順と食い違う・move={move_at}・zorder={zorder_at}）"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("pub fn emo2_frame_system(world: &mut World) {"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("donor パターン: remove→各フェーズ→insert"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("donor パターン: remove→各フェーズ→insert"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

// ---------------------------------------------------------------------------
// ⑷ 起動の段の結線——shell 設定の値が台帳まで運ばれる（task 6.3・要件 5.1／5.2）
// ---------------------------------------------------------------------------

/// 起動窓の準備が読んだ `seriko.zorder` の生の値が、`main` の 1 本道で結線へ渡る。
///
/// 落としても「設定を書いていないゴースト」と挙動が 1 ミリも変わらないので、挙動の檻には
/// 映らない。運ぶ側の字面をここが唯一押さえる。
#[test]
fn t_zwi08_the_entry_point_carries_the_shell_setting_into_the_wiring() {
    let raw = include_str!("../main.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains("let zorder_raw = prepared.zorder_raw.clone();"),
        "準備の結果から重なりの生の値を取り出す行が本文に無い"
    );
    assert!(
        squeezed
            .contains("let zorder_raw = startup.as_ref().and_then(|prep| prep.zorder_raw.clone());"),
        "起動窓の戻り値から重なりの生の値を受け取る行が本文に無い"
    );
    assert!(
        squeezed.contains("author_dpi, zorder_raw.as_deref(), );"),
        "重なりの生の値が結線（wire_emo2_boot）へ渡されていない＝設定が台帳へ届かない: {squeezed}"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("fn open_startup_window(app: &WinApp, cfg: &ConfigInputs)"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("起動窓の準備が descript から**1 度だけ**読み取った値"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("起動窓の準備が descript から**1 度だけ**読み取った値"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

/// 結線は台帳へ基底を据えてから結線状態を World へ載せる（要件 5.1 の適用時点）。
///
/// 順序が逆になると結線状態は既に move 済みで種を蒔けない（コンパイルが通らない）ため、
/// ここが押さえるのは**呼出が 1 つだけ在ること**と**載せるより手前に在ること**である。
/// 呼出を丸ごと削っても、設定を書いていないゴーストと同じ挙動になるだけで誰も気づかない。
#[test]
fn t_zwi09_the_boot_seats_the_descript_base_before_inserting_the_wiring() {
    let raw = include_str!("mod.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert_eq!(
        squeezed
            .matches("wiring.seed_zorder_descript_base(zorder_descript);")
            .count(),
        1,
        "shell 設定由来の基底を据える呼出がちょうど 1 つではない（0 なら設定が効かず、2 なら二度据える）"
    );

    let seed_at = index_of(&squeezed, "wiring.seed_zorder_descript_base(zorder_descript);");
    let insert_at = index_of(
        &squeezed,
        "app.world().borrow_mut().world_mut().insert_non_send(wiring);",
    );
    assert!(
        seed_at < insert_at,
        "基底を据える段が結線状態を World へ載せるより後ろに在る（seed={seed_at}・insert={insert_at}）"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("zorder_descript: Option<&str>,"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("解釈できない値は理由とともに記録され"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("解釈できない値は理由とともに記録され"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}
