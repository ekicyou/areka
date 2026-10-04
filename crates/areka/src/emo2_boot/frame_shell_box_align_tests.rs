// =============================================================================
// 絵と箱の置き場所の揃え（areka-P0-shell-balloon-frame-align・要件 4.1〜4.4）
//
// 親（`frame_shell_box_integration_tests.rs`）の `Cage`——実 emo2 fixture ＋ 実 GPU で本番の
// `emo2_frame_system` を回す檻——に、シェルの文面と絵の差し替え（`present_tx` への
// `ShowSurface`／`Hide`）を渡して、台本の `\s` と絵が届く順を決定論で組む。
//
// 檻で使う面の番号（面 0・面 10）は emo2 の検体が合成できる面から選ぶ。ここではまず、
// その 2 つの面が検体で実際に合成できることと、普通のバルーンの窓を出して隠す手順が
// 檻の道具で踏めることを確かめる（面が合成できなければ、この檻の番号だけを選び直す）。
//
// 時刻はすべて注入（`FrameTime`＋`TalkClock` の epoch）で、実時間の待機は用いない。
// =============================================================================

use super::*;

/// 面 0 と面 10 の両方に箱 `talk` を、別の置き場所で持つ文面。
const SHELL_TWO_PLACES: &str = "\
balloon.talk
{
size,100,40
}
surface0
{
element1,balloon,talk,10,10
}
surface10
{
element1,balloon,talk,60,30
}
";

/// 檻で使う 2 つの面が emo2 の検体で合成でき、絵の差し替えが表示層の番号に届く。
/// 非表示を送れば番号が外れ、もう一度送れば戻る。あわせて、箱の無い面で普通のバルーンの
/// 窓を出して隠す手順（文字を書いてから消す）が檻の道具で踏めることを確かめる。
#[test]
fn both_cage_surfaces_compose_on_the_emo2_sample() {
    for shell in [SHELL_TWO_PLACES, SHELL] {
        let mut cage = Cage::boot(shell, &[0]);
        assert_eq!(
            cage.picture_id(0),
            Some(0),
            "前提: 装着で面 0 を表示している"
        );

        cage.picture(0, PLAIN_SURFACE_ID);
        cage.frame();
        assert_eq!(
            cage.picture_id(0),
            Some(PLAIN_SURFACE_ID),
            "面 {PLAIN_SURFACE_ID} が検体で合成でき、表示層の番号が替わる"
        );

        cage.hide_picture(0);
        cage.frame();
        assert_eq!(cage.picture_id(0), None, "非表示で番号が外れる");

        cage.picture(0, BOX_SURFACE_ID);
        cage.frame();
        assert_eq!(
            cage.picture_id(0),
            Some(BOX_SURFACE_ID),
            "非表示の後でも面 {BOX_SURFACE_ID} が合成できる"
        );
    }

    // 普通のバルーンの窓を出して隠す（親の文面＝面 0 に箱あり・面 10 に箱なし）。
    let mut cage = Cage::boot(SHELL, &[0]);
    cage.picture(0, PLAIN_SURFACE_ID);
    cage.surface(0, PLAIN_SURFACE);
    cage.text(0, "あ");
    cage.frame();
    assert_eq!(cage.window_visible(0), Some(true), "文字を書くと窓が出る");
    cage.cue(0, CueCommand::Clear);
    cage.frame();
    assert_eq!(
        cage.window_visible(0),
        Some(false),
        "文字を消すと窓が隠れる"
    );
    assert!(
        cage.shown_boxes(0).is_empty(),
        "箱の無い面なので箱は最後まで出ない"
    );
}
