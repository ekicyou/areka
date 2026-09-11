//! # 行送りの後も収まる行は全部見える（症状 F・再生の相の追試・R17.1〜17.4）
//!
//! [`crate::viewbox_draw`] の兄弟檻（`viewbox_draw_scroll_retain_tests.rs`）が純粋レイアウト
//! （`FixedMetrics`）で組んだ入力を扱うのに対し、本檻は**本番の提示経路**
//! （[`present_frame`] → `present_actor` → 実フォントの `DWriteMetrics` → `ViewboxExecutor`）を
//! 実物の相方側バルーン幾何（`emo2-kakukaku` の `balloonk0s.txt` 由来・288×203 の面に
//! 文字描画範囲 (24,40)-(240,133)＝幅 216・高さ 93・折返し基準 254＝描画範囲の外・
//! `budoux_newline,1`＝分かち書き境界での折返し）で駆動する。
//!
//! **オラクル**は「同じ cue 列・同じ注入時刻を、まっさらな runtime に**その 1 フレームだけ**
//! 提示させた面」である。まっさらな runtime の初回フレームは行指紋が空＝全域ダーティ＝全域
//! 再描画と同値ゆえ、1 字ずつ積み上げた面（差分描画）とバイト等価でなければ、積み上げの
//! どこかで画素を失っている（症状 F）。
//!
//! **冒頭の改行を含まない台本（`boot_cues`）の 3 本は HEAD で緑である**——描画対象の非回帰の
//! 檻であり、症状 F は描画側では再現しない。較正は viewbox 側と同じで、描画対象から先頭可視行を
//! 1 つ落とす欠陥を注入すると本檻も赤になる。
//!
//! **症状 F を再現するのは `leading_gap_cues` の 2 本**である（2026-09-10 の根因の確定・R17.3）。
//! 実機の相方側 scope は、pasta が話者切替の前に前の scope へ出す `\n[150]` を**空のまま**
//! 受け取るため、台本の先頭に `NewLine { ratio: 1.5 }` が立つ。この空き 45 は、送り量の原点が
//! 最初の行の開始側だと永久に送られず、最新の 1 行だけが描画範囲の上から 1.5 行ぶん下に残る。
//!
//! **拡大率**は 1.0 と 2.0 の両方で回す。症状 F を観測した実機走行は 192 dpi（k=2.0・生ログの
//! `apply(ShowSurface)` 行が `k_ratio=ScaleRatio { num: 2, den: 1 }`）で、k=1.0 だけの檻では
//! 実機の量子化・blit 量を通らない。拡大率が整数なので真位置と確定位置が一致し、k=2.0 でも
//! byte 等価を受け入れ基準に保てる。

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::prelude::World;

use super::test_support::{com_world, cue, opaque_count, spawn_reserved_slot};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::state::TextLayerConfig;

/// 相方側バルーンの面寸（`balloonk0.png` 288×203）。
const IMAGE: (u32, u32) = (288, 203);
/// 文字描画範囲の丈（image px・(24,40)-(240,133)）——供給面の寸はこれ × 拡大率。
const REGION_SIZE: (u32, u32) = (216, 93);
/// 実機と同じ字の丈（`font.height,28` → 行送り 30＝28 ＋ 行間 2）。
const FONT_HEIGHT: u32 = 28;
/// reveal 間隔（秒/グリフ・`test_support::cue` が焼く duration と対）。
const REVEAL_INTERVAL: f64 = 0.05;

/// バルーン面の物理寸（`ceil(image × k)`）。
fn physical(k: f32) -> (u32, u32) {
    (
        (IMAGE.0 as f32 * k).ceil() as u32,
        (IMAGE.1 as f32 * k).ceil() as u32,
    )
}

/// 実物の相方側バルーン記述（`descript.txt` ＋ `balloonk0s.txt` の 2 層マージ相当）。
///
/// `validrect` は `top,40`／`bottom,-70`／`left,24`／`right,-48`（負値＝反対辺基準）、
/// `wordwrappoint.x` は基層の `-34` を継いで 254——描画範囲の右端 240 の**外**（粗い定義）。
fn kero_model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(Some(-34), None),
        ValidRect::new(Some(40), Some(-70), Some(24), Some(-48)),
        Font::new(
            Some("Yu Gothic UI".to_string()),
            Some(FONT_HEIGHT),
            FontColor::new(None, None, None),
        ),
        None,
        // `budoux_newline,1`（実 descript.txt）＝分かち書き境界での折返し（塊先決）。
        Some("1".to_string()),
    )
}

/// 実機の台本（`boot.pasta:78-90`）と同じ形の cue 列——2 行に折り返す台詞 → `\n[150]`
/// （＝`NewLine { ratio: 1.5 }`）→ 次の台詞。返り値は (cue 列, 最終時刻)。
fn boot_cues(actor: &str) -> (Vec<TalkCue>, f64) {
    let first = "僕はエモ。クール系の可愛い娘。";
    let second = "イイジャン！‥‥ええと、";
    let t1 = first.chars().count() as f64 * REVEAL_INTERVAL;
    let t_end = t1 + second.chars().count() as f64 * REVEAL_INTERVAL;
    let cues = vec![
        cue(actor, 0.0, CueCommand::Text(first.into())),
        cue(actor, t1, CueCommand::NewLine { ratio: 1.5 }),
        cue(actor, t1, CueCommand::Text(second.into())),
    ];
    (cues, t_end)
}

/// 実機の相方側 scope と同じ形の cue 列——**先頭が `NewLine { ratio: 1.5 }`**（トーク開始時の
/// 空の scope へ落ちる `\n[150]`）で、以降は [`boot_cues`] と同じ台本。返り値は (cue 列, 最終時刻)。
fn leading_gap_cues(actor: &str) -> (Vec<TalkCue>, f64) {
    let (mut cues, t_end) = boot_cues(actor);
    cues.insert(0, cue(actor, 0.0, CueCommand::NewLine { ratio: 1.5 }));
    (cues, t_end)
}

/// 行送り軸の帯 `[y0, y1)` の非透明ピクセル数（BGRA 密配列・物理 px）。
fn ink_in_band(bytes: &[u8], w: u32, y0: u32, y1: u32) -> usize {
    let mut count = 0;
    for y in y0..y1 {
        for x in 0..w {
            if bytes[((y * w + x) * 4 + 3) as usize] != 0 {
                count += 1;
            }
        }
    }
    count
}

/// まっさらな runtime に同じ cue 列を積み、**その 1 フレームだけ**提示した面を返す（オラクル）。
fn oracle_frame(world: &mut World, cues: &[TalkCue], actor: &ActorKey, t: f64, k: f32) -> Vec<u8> {
    let (window, slot) = spawn_reserved_slot(world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    for c in cues {
        rt.apply_cue(c);
    }
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, k, physical(k), IMAGE),
        ResolvedBalloonText::resolve(&kero_model(), IMAGE),
    );
    present_frame(&mut rt, world, t).expect("オラクル提示フレーム");
    rt.surface(actor)
        .expect("オラクル供給面")
        .read_back()
        .expect("オラクル read_back")
}

/// 台本を 1 字ずつ通しで駆動し、各フレームを積み上げ側とオラクル側で突き合わせる。
fn drive_scroll_lap(k: f32, frame_period: f64) {
    let (cues, t_end) = boot_cues("1");
    drive_lap(k, frame_period, &cues, t_end);
}

/// 与えた cue 列で 1 周を駆動し、最終フレームの供給面を返す（帯の追加判定は呼び手が持つ）。
fn drive_lap(k: f32, frame_period: f64, cues: &[TalkCue], t_end: f64) -> Vec<u8> {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("1");

    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    for c in cues {
        rt.apply_cue(c);
    }
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, k, physical(k), IMAGE),
        ResolvedBalloonText::resolve(&kero_model(), IMAGE),
    );

    // 供給面の寸＝ceil(文字描画範囲 × k)。先頭可視行の帯は上端 0 から字の丈 × k ぶん
    // （送りの式より先頭可視行は常に描画面の上端 0 に来る）。
    let w = (REGION_SIZE.0 as f32 * k).ceil() as u32;
    let h = (REGION_SIZE.1 as f32 * k).ceil() as u32;
    let head_band = (FONT_HEIGHT as f32 * k).round() as u32;
    let steps = ((t_end / frame_period).ceil() as usize) + 2;
    let mut checked = 0usize;
    for step in 1..=steps {
        let t = step as f64 * frame_period;
        present_frame(&mut rt, &mut world, t).expect("提示フレーム");
        let live = rt
            .surface(&actor)
            .expect("供給面")
            .read_back()
            .expect("read_back");
        assert_eq!(
            live.len(),
            (w * h * 4) as usize,
            "k={k}: 供給面は文字描画範囲 × k の丈（{w}×{h}）"
        );

        let oracle = oracle_frame(&mut world, cues, &actor, t, k);
        assert!(
            opaque_count(&oracle) > 0,
            "k={k} t={t}: オラクル面にインクが在る（空面同士の一致を排除）"
        );
        let head_oracle = ink_in_band(&oracle, w, 0, head_band);
        let head_live = ink_in_band(&live, w, 0, head_band);
        assert_eq!(
            head_oracle > 0,
            head_live > 0,
            "k={k} t={t}: 先頭可視行の帯 [0,{head_band}) のインクの有無が一致する\
             （オラクル {head_oracle} 画素 vs 積み上げ {head_live} 画素）\
             ——症状 F は積み上げ側だけが 0 になる形"
        );
        assert_eq!(
            oracle, live,
            "k={k} t={t}: 1 字ずつ積み上げた面と、1 フレームだけ提示した面（全域再描画と同値）が\
             byte 等価"
        );
        checked += 1;
    }
    assert!(
        checked >= 10,
        "k={k}: 台本を通しで駆動した（実際 {checked} フレーム）"
    );

    let last = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert!(
        ink_in_band(&last, w, 0, head_band) > 0,
        "k={k}: 最終フレームの先頭可視行の帯 [0,{head_band}) にインクが在る\
         （症状 F: 最新の 1 行しか見えない）"
    );
    last
}

/// 拡大率 1.0（96 dpi）——量子化の無い基準の走行（1 コマ 1 字）。
#[test]
fn present_actor_scroll_keeps_first_visible_line_ink_k1() {
    drive_scroll_lap(1.0, REVEAL_INTERVAL);
}

/// 拡大率 2.0（192 dpi）——症状 F を観測した実機走行と同じ拡大率（1 コマ 1 字）。
#[test]
fn present_actor_scroll_keeps_first_visible_line_ink_k2() {
    drive_scroll_lap(2.0, REVEAL_INTERVAL);
}

/// 描画が重いときの粗い刻み（1 コマ 78 ms＝症状 E の実測中央値・1 コマで複数字進む
/// catch-up）。行が 2 つ以上まとめて増えるフレームや、送りを跨ぐ刻みが reveal 間隔の
/// 整数倍に乗らない位相を通す。
#[test]
fn present_actor_scroll_keeps_first_visible_line_ink_coarse_cadence() {
    drive_scroll_lap(2.0, 0.078);
    drive_scroll_lap(2.0, 0.19);
    drive_scroll_lap(1.0, 0.078);
}

/// 冒頭に `\n[150]` が立つ台本を拡大率 1.0 で駆動する（症状 F の再現・R17.3）。
///
/// 送りが起きた後の最終フレームで ⑴ 先頭可視行の帯（面の上端 0 から字の丈 × k）にインクが
/// 在ること ⑵ その下にもインクが在ること＝見えている行が 2 つ以上あること、を主張する。
/// 最新行は縮退規則（最新行への飽和）で常に可視ゆえ、⑵ が成り立つのは「最新行より前の行も
/// 見えている」ときだけである。直す前の HEAD では最新の 1 行だけが面の上から 1.5 行ぶん下に
/// 残るため ⑴ が 0 画素で赤になる。全域再描画のオラクルとのバイト等価は [`drive_lap`] が持つ。
#[test]
fn present_actor_leading_gap_keeps_earlier_lines_visible_k1() {
    assert_leading_gap_lap(1.0);
}

/// 同じ台本を拡大率 2.0（192 dpi＝症状 F を観測した実機走行と同じ）で駆動する。
#[test]
fn present_actor_leading_gap_keeps_earlier_lines_visible_k2() {
    assert_leading_gap_lap(2.0);
}

/// 冒頭に空きが立つ台本の 1 周と、最終フレームの帯の判定（拡大率で共有する本体）。
fn assert_leading_gap_lap(k: f32) {
    let (cues, t_end) = leading_gap_cues("1");
    let last = drive_lap(k, REVEAL_INTERVAL, &cues, t_end);
    let w = (REGION_SIZE.0 as f32 * k).ceil() as u32;
    let h = (REGION_SIZE.1 as f32 * k).ceil() as u32;
    let head_band = (FONT_HEIGHT as f32 * k).round() as u32;
    let head = ink_in_band(&last, w, 0, head_band);
    let below = ink_in_band(&last, w, head_band, h);
    assert!(
        head > 0,
        "k={k}: 冒頭に空きが在る台本でも、送りの後の先頭可視行の帯 [0,{head_band}) に\
         インクが在る（症状 F: 冒頭の空きが送られず最新の 1 行だけが残る）"
    );
    assert!(
        below > 0,
        "k={k}: 先頭可視行より下の帯 [{head_band},{h}) にもインクが在る＝収まる行が\
         2 つ以上見えている（最新行は飽和規則で常に可視ゆえ、これは前の行も見えている証跡）"
    );
}
