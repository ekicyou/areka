// task 7.3 の檻: 装飾入りの配置（`LayoutEngine::layout_styled`）と装飾入りの描画
// （`ViewboxExecutor::render_styled`）が**毎フレームの本番経路**（`present_frame`）へ
// 実際に繋がっていること（要件 14.1／14.2／14.3）と、候補列の解決台帳が計測器と描画器で
// **1 つ**であること（要件 9.8）。
//
// ここは COM／GPU を使う——「配線が実在する」という主張は、純粋層の単体では作れない。
// 述語は 3 つとも「装飾を含む台本」と「含まない台本」の差で述べ、従来の入口（`layout`／
// `render`）へ戻すと赤になる形にしてある（各テストの「較正」を参照）。

use areka_sakura::contract::{ActorKey, CueCommand, FONT_TAG_CARRIER, TalkCue};
use bevy_ecs::prelude::World;
use log_capture_kit::capture;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use super::test_support::{cue, geo_model, opaque_count, spawn_reserved_slot};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::state::TextLayerConfig;

/// バルーン面の代表寸（既定 12px の文字が横に十分並び、行が数本入る大きさ）。
const IMAGE: (u32, u32) = (240, 160);

/// 全グリフが出そろう十分に後の注入時刻（reveal を待たない）。
const SETTLED: f64 = 100.0;

/// `\f[トークン…]` を運ぶ汎用キャリアの cue（再生時間 0・`state_decoration_tests` と同流儀）。
fn font(tokens: &[&str]) -> TalkCue {
    cue(
        "0",
        0.0,
        CueCommand::command_carrier(
            FONT_TAG_CARRIER,
            tokens.iter().map(|t| (*t).to_owned()).collect(),
        ),
    )
}

fn text(body: &str) -> TalkCue {
    cue("0", 0.0, CueCommand::Text(body.to_owned()))
}

fn newline() -> TalkCue {
    cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 })
}

/// 台本 1 本を本番の 1 フレーム経路（`present_frame`）へ通し、供給面を読み戻す。
///
/// 返り値は `(BGRA 密配列, 面の寸法)`。装着と描画は本番と同じ `present_frame` が行う——
/// ここで別経路を組むと「配線が実在する」の主張が空振りする。
fn run_script(script: &[TalkCue]) -> (Vec<u8>, (u32, u32)) {
    // 本番 UI スレッド（MTA）を再現（`actor_runtime_frame_tests` と同一方針）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    world.insert_resource(core);
    world.insert_resource(wuc);

    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    for cue in script {
        rt.apply_cue(cue);
    }
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE),
        ResolvedBalloonText::resolve(&geo_model(), IMAGE),
    );
    present_frame(&mut rt, &mut world, SETTLED).expect("装着＋描画フレーム");

    let surface = rt.surface(&actor).expect("装着済み actor の供給面");
    let size = surface.size();
    (surface.read_back().expect("read_back"), size)
}

/// インクのある最下行（ブロック軸の到達点）。インクが無ければ `None`。
fn last_inked_row(bytes: &[u8], size: (u32, u32)) -> Option<u32> {
    let (w, h) = size;
    (0..h).rev().find(|y| {
        let row = &bytes[(*y * w * 4) as usize..((*y + 1) * w * 4) as usize];
        row.chunks_exact(4).any(|px| px[3] != 0)
    })
}

/// R14.2（描画側の配線）: `\f[height,…]` で大きくした文字は**実際に大きく描かれる**。
///
/// 較正: `present_actor` を従来の `ViewboxExecutor::render` へ戻す（装飾の表を渡さない）と
/// 文字の大きさが既定のままになり、非透明画素数が装飾なしと並んで本述語が赤になる。
#[test]
fn a_bigger_height_actually_paints_bigger_glyphs_through_the_frame_path() {
    let (plain, _) = run_script(&[text("あい")]);
    let (styled, _) = run_script(&[font(&["height", "40"]), text("あい")]);

    let plain_ink = opaque_count(&plain);
    let styled_ink = opaque_count(&styled);
    assert!(plain_ink > 0, "装飾なしの台本にもインクがある（前提）");
    assert!(
        styled_ink > plain_ink * 2,
        "既定 12px → 40px で非透明画素が大きく増える（装飾入りの描画の入口が繋がっている）: \
         {plain_ink} -> {styled_ink}"
    );
}

/// R14.1／R14.2（配置側の配線）: 行の丈が**行内の最大の大きさ**で決まり、次の行が下がる。
///
/// インクを持たない全角空白に大きさを与えるので、描かれる字形は 2 本の台本で同一である
/// （どちらも既定の大きさの「い」1 文字だけ）。違うのは行送りだけ——描画側の配線では
/// 説明できない差を見ている。
///
/// 較正: `present_actor` を従来の `LayoutEngine::layout_with_cursor_warn` へ戻す
/// （番号列を渡さない）と行送りが既定のままになり、最下行が一致して本述語が赤になる。
#[test]
fn the_line_pitch_follows_the_biggest_size_on_the_line_through_the_frame_path() {
    let (plain, size) = run_script(&[text("　"), newline(), text("い")]);
    let (styled, styled_size) = run_script(&[
        font(&["height", "60"]),
        text("　"),
        font(&["default"]),
        newline(),
        text("い"),
    ]);
    assert_eq!(size, styled_size, "面の寸法は台本によらず同じ（前提）");

    let plain_bottom = last_inked_row(&plain, size).expect("装飾なしの「い」にインクがある");
    let styled_bottom = last_inked_row(&styled, size).expect("装飾ありの「い」にインクがある");
    assert!(
        styled_bottom > plain_bottom,
        "60px の全角空白が 1 行目の丈を決め、2 行目の「い」が下がる（装飾入りの配置の入口が\
         繋がっている）: {plain_bottom} -> {styled_bottom}"
    );
    assert_eq!(
        opaque_count(&plain),
        opaque_count(&styled),
        "描かれる字形は同じ（差は行送りだけ——描画側の配線では説明できない）"
    );
}

/// R9.8: 候補列の解決台帳は計測器と描画器で**1 つ**——同じ候補列の全滅の記録が 1 件。
///
/// 較正: `DWriteMetrics::new`／`ViewboxExecutor::new`（それぞれ自前の台帳を作る）へ戻すと
/// 同じ記録が 2 件になり赤。装飾入りの入口をどちらも呼ばなければ 0 件になり赤。
#[test]
fn a_dead_candidate_list_is_recorded_once_by_the_shared_catalog() {
    const MISSING: &str = "areka-no-such-font-xyz";

    let (_, events) = capture(|| {
        run_script(&[font(&["name", MISSING]), text("あい")]);
    });

    let records: Vec<&str> = events
        .iter()
        .filter(|ev| ev.message().contains("フォントの候補が 1 つも見つからない"))
        .filter_map(|ev| ev.field("candidates"))
        .filter(|candidates| candidates.contains(MISSING))
        .collect();

    assert_eq!(
        records.len(),
        1,
        "同じ候補列の全滅の記録は 1 件（台帳が 1 つ・要件 9.8）: {records:?}"
    );
}
