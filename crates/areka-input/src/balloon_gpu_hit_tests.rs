//! 実 GPU で提示した当たりの行へ、本番の移動と押下のハンドラを通す。

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::{SpanKind, TextLayerConfig};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::test_support::{
    bubble_move, capture_logs, gpu_balloon_world, headless_runtime_wiring, insert_wiring,
    left_press,
};
use super::{
    BalloonWiring, ChoiceHitRow, hit_choice_row, on_balloon_pointer_moved,
    on_balloon_pointer_pressed,
};

fn init_com() {
    // SAFETY: 描画資源に要る COM の MTA 初期化（既初期化の S_FALSE は無視・テストスレッドごと）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// 普通のバルーン（scope 0）に選択肢を順に載せた文字の層。
fn runtime_with_choices(choices: &[(&str, &str)]) -> Rc<RefCell<TextLayerRuntime>> {
    let rt = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    for (id, text) in choices {
        rt.borrow_mut().apply_cue(&TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Choice {
                id: (*id).into(),
                text: (*text).into(),
                references: Vec::new(),
            },
            duration: 0.0,
        });
    }
    rt
}

fn center(row: &ChoiceHitRow) -> (i32, i32) {
    (
        ((row.rect.left + row.rect.right) / 2.0) as i32,
        ((row.rect.top + row.rect.bottom) / 2.0) as i32,
    )
}

/// 拡大率 2.0 の行の中心で、移動は強調をその行の ordinal に当て、押下は選択を送って INFO を 1 行残す。
#[test]
fn handlers_hit_rows_presented_at_scale_two_without_dividing_the_point() {
    init_com();
    let _gpu = areka_runtime::spine::GpuPermit::take();
    let rt = runtime_with_choices(&[("OnYes", "はい")]);
    let (mut world, window) = gpu_balloon_world(&rt, 2.0, (120, 60), (20, 20));

    let rows = rt.borrow().choice_hit_rows(&ActorKey::from("0")).to_vec();
    let row = rows
        .iter()
        .find(|r| r.kind == SpanKind::Choice)
        .cloned()
        .expect("前提: 選択肢の当たり行がある");
    let (x, y) = center(&row);
    assert!(
        hit_choice_row(&rows, x as f32, y as f32).is_some(),
        "前提: 行の中心は行の中: {row:?}"
    );
    assert_eq!(
        hit_choice_row(&rows, x as f32 / 2.0, y as f32 / 2.0),
        None,
        "前提: 行の中心を 2 で割った点はどの行にも当たらない: {rows:?}"
    );

    world.insert_non_send(headless_runtime_wiring(Rc::clone(&rt)));
    let rx = insert_wiring(&mut world);

    assert!(!on_balloon_pointer_moved(
        &mut world,
        window,
        window,
        &bubble_move(x, y)
    ));
    assert_eq!(
        world.non_send::<BalloonWiring>().hover(0),
        Some(row.ordinal),
        "移動は行の ordinal を強調する"
    );

    let logs = capture_logs(|| {
        assert!(on_balloon_pointer_pressed(
            &mut world,
            window,
            window,
            &left_press(x, y)
        ));
    });
    let sent = rx.try_recv().expect("押下は選択を送る");
    assert_eq!((sent.kind, sent.id.as_str()), (SpanKind::Choice, "OnYes"));
    let selected: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("level=INFO") && l.contains("choice_selected"))
        .collect();
    assert_eq!(selected.len(), 1, "INFO choice_selected が 1 行: {logs:?}");
}

/// 折り返した選択肢の後の行では index と ordinal が食い違い、移動は ordinal を強調する。
#[test]
fn hover_on_a_row_after_a_wrapped_choice_highlights_its_ordinal() {
    init_com();
    let _gpu = areka_runtime::spine::GpuPermit::take();
    let rt = runtime_with_choices(&[
        ("OnLong", "あいうえおかきくけこさしすせそたちつてと"),
        ("OnShort", "な"),
    ]);
    let (mut world, window) = gpu_balloon_world(&rt, 1.0, (60, 200), (0, 0));

    let rows = rt.borrow().choice_hit_rows(&ActorKey::from("0")).to_vec();
    let (index, row) = rows
        .iter()
        .enumerate()
        .find(|(i, r)| *i != r.ordinal)
        .map(|(i, r)| (i, r.clone()))
        .unwrap_or_else(|| panic!("前提: index と ordinal が食い違う行がある: {rows:?}"));
    let (x, y) = center(&row);
    assert_eq!(
        hit_choice_row(&rows, x as f32, y as f32),
        Some(index),
        "前提: 行の中心はその行に当たる"
    );

    world.insert_non_send(headless_runtime_wiring(Rc::clone(&rt)));
    let _rx = insert_wiring(&mut world);

    on_balloon_pointer_moved(&mut world, window, window, &bubble_move(x, y));
    assert_eq!(
        world.non_send::<BalloonWiring>().hover(0),
        Some(row.ordinal),
        "強調は行の index（{index}）でなく ordinal"
    );
}
