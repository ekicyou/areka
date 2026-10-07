//! 出番の世代と、合図が追い付くまでの欄の外し（spec: areka-P0-animated-image-playback task 4.3・
//! 要件 2.3）。
//!
//! 面の表・コマ・指令の送り方は 4.2 の `presenter_film_tests.rs` の補助をそのまま使う（面 0 ＝
//! 回数つき `fin.png` と終わりなし `end.png` を並べたバルーン）。

use super::*;

use super::film_tests::{
    T, ack, applies, attach, build_balloon, established, frames, golden, hide, send, shown,
};
use super::test_support::{capture, make_world_with_gpu};

const DROP_MESSAGE: &str = "apply(StageAck): 捨てた";

/// 見えていない外から所有される対象を出すたびに出番の世代が 1 つ進み、見えている対象への
/// `show_target` では進まない。未登録の対象は `None`。
#[test]
fn each_show_of_a_hidden_target_advances_the_stage_generation_by_one() {
    let mut world = make_world_with_gpu();
    let mut presenter = established(&mut world);
    assert_eq!(presenter.stage_generation(T), Some(0), "0 から始まる");

    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(presenter.stage_generation(T), Some(1));
    presenter
        .show_target(&mut world, T)
        .expect("show_target（重ねがけ）");
    assert_eq!(
        presenter.stage_generation(T),
        Some(1),
        "見えている対象への show_target で進んだ"
    );

    for expected in [2, 3] {
        hide(&mut presenter, &mut world);
        presenter.show_target(&mut world, T).expect("show_target");
        assert_eq!(presenter.stage_generation(T), Some(expected));
    }
    assert_eq!(presenter.stage_generation(TargetId(99)), None);
}

/// 大きすぎる合図（出番の世代より大きい）・未装着の対象への合図は `debug!` で捨てる。追い付いた
/// 世代は変わらないので、指令の回数つきの欄は外されたまま。古い合図で戻ることも無い。
#[test]
fn too_large_and_unattached_acks_are_ignored() {
    let mut world = make_world_with_gpu();
    let (w, _) = build_balloon();
    let mut presenter = established(&mut world);
    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(presenter.stage_generation(T), Some(1));

    let ((), events) = capture(|| {
        ack(&mut presenter, &mut world, T, 2);
        ack(&mut presenter, &mut world, TargetId(99), 1);
    });
    let dropped = events
        .iter()
        .filter(|e| e.message() == DROP_MESSAGE)
        .count();
    assert_eq!(dropped, 2, "捨てた合図の debug! が 1 件ずつ出ていない");

    let running = frames(&w, Some(1), Some(1));
    send(&mut presenter, &mut world, 0, BindSet::default(), running);
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &frames(&w, None, Some(1))),
        "大きすぎる合図で追い付いたことになった"
    );

    // 追い付いた後に古い合図が来ても戻らない（大きい方を取る）。
    ack(&mut presenter, &mut world, T, 1);
    ack(&mut presenter, &mut world, T, 0);
    let running = frames(&w, Some(2), Some(0));
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        running.clone(),
    );
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &running),
        "古い合図で追い付いた世代が戻った"
    );
}

/// 追い付くまでの指令は回数つきの子の欄だけを外して通す（捨てない・終わりなしのコマと面の番号の
/// 切り替えは効く）。追い付いた後の指令はそのまま通る。
#[test]
fn commands_are_stripped_until_the_ack_catches_up_then_pass_unchanged() {
    let mut world = make_world_with_gpu();
    let (w, _) = build_balloon();
    let mut presenter = established(&mut world);
    presenter.show_target(&mut world, T).expect("show_target");

    let stale = frames(&w, Some(2), Some(1));
    send(&mut presenter, &mut world, 0, BindSet::default(), stale);
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &frames(&w, None, Some(1))),
        "追い付く前の指令の回数つきの欄が外れていない（終わりなしは残る）"
    );
    send(
        &mut presenter,
        &mut world,
        1,
        BindSet::default(),
        PatternState::default(),
    );
    assert_eq!(
        presenter.current_surface_id(T),
        Some(1),
        "追い付く前の面の切り替えが捨てられた"
    );

    ack(&mut presenter, &mut world, T, 1);
    let fresh = frames(&w, Some(1), Some(0));
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        fresh.clone(),
    );
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &fresh),
        "追い付いた後の指令が手を入れられた"
    );

    // 外すのは預かるかの判定の前: 隠れていて追い付いていない対象へ、回数つきの欄だけが
    // `last_show` と違う指令が着くと、外した結果が `last_show` と同じなので預からずに通る
    // （引き当てが当たり合成 0 回）。外さずに判定すると預かってしまう。
    hide(&mut presenter, &mut world);
    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(presenter.stage_generation(T), Some(2));
    hide(&mut presenter, &mut world);
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::default(),
            frames(&w, Some(2), Some(0)),
        )
    });
    assert_eq!(
        applies(&events),
        (1, 0),
        "預かるかの判定の前に回数つきの欄が外れていない"
    );
}

/// 命令で見える対象（シェル）は世代を持たない: 出し直しても 0 のまま、指令は手を入れられない。
#[test]
fn shell_targets_have_no_stage_generation() {
    let mut world = make_world_with_gpu();
    let (w, _) = build_balloon();
    let mut presenter = EmoPresenter::new();
    attach(
        &mut presenter,
        &mut world,
        VisibilityOwnership::CommandDriven,
    );
    for _ in 0..2 {
        let running = frames(&w, Some(1), Some(1));
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::default(),
            running.clone(),
        );
        assert_eq!(presenter.target_visible(T), Some(true));
        assert_eq!(
            presenter.read_back(T).expect("read_back"),
            golden(0, &BindSet::default(), &running),
            "シェルへの指令に手が入った"
        );
        hide(&mut presenter, &mut world);
        presenter.show_target(&mut world, T).expect("show_target");
        assert_eq!(presenter.stage_generation(T), Some(0));
    }
}

/// 対象の差し替えは出番の世代と追い付いた世代を引き継ぐ（窓が同じ）。
#[test]
fn replace_target_carries_both_generations() {
    let mut world = make_world_with_gpu();
    let (w, _) = build_balloon();
    let replace = |presenter: &mut EmoPresenter, world: &mut World| {
        let (emo_world, atlas) = build_balloon();
        presenter.apply(
            world,
            PresentCommand::ReplaceTarget {
                target: T,
                emo_world: Box::new(emo_world),
                atlas,
                author_dpi: 96,
                show: Some((0, BindSet::default())),
                reply: None,
            },
        );
    };

    // 差し替えた対象は見えないまま面 0（経過 0）で確立する。そこへ回数つきの欄だけが違う指令を
    // 送ると、追い付いていなければ外されて `last_show` と同じ＝通る（合成 0 回）、追い付いて
    // いれば外されずに預かる（通らない）。
    let running = frames(&w, Some(1), None);

    // 追い付いていない（世代 1・合図 0）まま差し替える: 外し続ける。
    let mut presenter = established(&mut world);
    presenter.show_target(&mut world, T).expect("show_target");
    replace(&mut presenter, &mut world);
    assert_eq!(presenter.stage_generation(T), Some(1));
    assert_eq!(presenter.target_visible(T), Some(false));
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::default(),
            running.clone(),
        )
    });
    assert_eq!(
        applies(&events),
        (1, 0),
        "差し替えで出番の世代が 0 に戻った"
    );

    // 追い付いた（世代 1・合図 1）まま差し替える: そのまま預かる。
    let mut presenter = shown(&mut world);
    replace(&mut presenter, &mut world);
    assert_eq!(presenter.stage_generation(T), Some(1));
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::default(),
            running.clone(),
        )
    });
    assert_eq!(
        applies(&events),
        (0, 0),
        "差し替えで追い付いた世代が 0 に戻った"
    );
}
