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

// ---------------------------------------------------------------------------
// 本 spec の枠の檻（design.md「枠の檻」の 1〜5）
//
// 毎フレームの終わりに、絵の番号・箱の四角・窓の可視（[`Look`]）と、届いた組
// （`balloon_reports`）を判定する。箱だけの番号の檻（5）はログも判定する。
// どれも直した後の正しい振る舞いを主張する（直す前のコードでは、ずれの場面で落ちる）。
// ---------------------------------------------------------------------------

/// 面 0 の箱 `talk` の四角の左上（`SHELL`・`SHELL_TWO_PLACES` とも `10,10`）。DPI 96 の檻では
/// 拡大率が 1 なので、element定義の X,Y がそのままシェルの窓の物理 px になる。
const AT_FACE0: (f32, f32) = (10.0, 10.0);
/// 面 10 の箱 `talk` の四角の左上（`SHELL_TWO_PLACES` の `60,30`）。
const AT_FACE10: (f32, f32) = (60.0, 30.0);

/// 組が 1 件も届いていない（見えている組が変わらない）。
const NO_REPORT: Vec<Vec<(u32, u32)>> = Vec::new();

/// 1 フレームの終わりに見る、1 スコープの姿。
#[derive(Debug, PartialEq)]
struct Look {
    /// シェルの窓に表示している絵の番号（非表示は `None`）。
    picture: Option<u32>,
    /// 文字の出ている箱の四角の左上（手前から）。
    boxes: Vec<(f32, f32)>,
    /// 普通のバルーンの窓の可視。
    window: Option<bool>,
}

impl Cage {
    fn look(&self, scope: u32) -> Look {
        Look {
            picture: self.picture_id(scope),
            boxes: self
                .wiring
                .runtime
                .borrow()
                .shown_boxes(&ActorKey::from(scope.to_string()))
                .iter()
                .map(|b| (b.rect.left, b.rect.top))
                .collect(),
            window: self.window_visible(scope),
        }
    }

    /// 届いた組を取り出す（取り出した分は次の判定に残らない）。
    fn reports(&self) -> Vec<Vec<(u32, u32)>> {
        balloon_reports(&self.reports)
    }
}

/// 絵 `picture` の上で、箱が `at` に出て窓は隠れている。
fn boxed(picture: u32, at: (f32, f32)) -> Look {
    Look {
        picture: Some(picture),
        boxes: vec![at],
        window: Some(false),
    }
}

/// 絵 `picture` の上で、窓が出て箱は出ていない。
fn windowed(picture: u32) -> Look {
    Look {
        picture: Some(picture),
        boxes: Vec::new(),
        window: Some(true),
    }
}

/// 絵 `picture` の上で、窓も箱も出ていない。
fn blank(picture: u32) -> Look {
    Look {
        picture: Some(picture),
        boxes: Vec::new(),
        window: Some(false),
    }
}

/// 面 0（絵も `\s` も面 0）の箱にだけ文字を出す。提示が写しを埋めるのはフレームの終わりなので
/// 2 フレーム回す。
fn speak_in_box(cage: &mut Cage) {
    cage.surface(0, BOX_SURFACE);
    cage.text(0, "あい");
    cage.frame();
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "前提: 箱に文字"
    );
    assert_eq!(
        cage.reports(),
        vec![vec![(0, 0)]],
        "前提: 箱のスコープが載った"
    );
}

/// 箱と窓の両方に文字を持たせ、面 0 の箱を出した姿にする（`SHELL`・窓の文字は保持）。
fn speak_in_both(cage: &mut Cage) {
    cage.picture(0, PLAIN_SURFACE_ID);
    cage.surface(0, PLAIN_SURFACE);
    cage.text(0, "う");
    cage.frame();
    assert_eq!(cage.look(0), windowed(PLAIN_SURFACE_ID), "前提: 窓に文字");
    cage.picture(0, BOX_SURFACE_ID);
    cage.surface(0, BOX_SURFACE);
    cage.text(0, "あい");
    cage.frame();
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "前提: 箱に文字（窓の文字は保持して隠す）"
    );
    assert_eq!(
        cage.reports(),
        vec![vec![(0, 0)]],
        "前提: 窓が見えた組が 1 度だけ届き、窓 → 箱の入れ替わりで組は欠けない"
    );
}

/// 1: 置き場所が違う面へ・絵が先（要件 1.1・2.1・4.1・4.2）。絵だけ届いたフレームで箱は
/// 新しい置き場所に出て、組は欠けない。
#[test]
fn box_moves_with_the_picture_when_the_picture_arrives_first() {
    let mut cage = Cage::boot(SHELL_TWO_PLACES, &[0]);
    speak_in_box(&mut cage);

    cage.picture(0, PLAIN_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(PLAIN_SURFACE_ID, AT_FACE10),
        "絵だけ届いたフレーム: 新しい絵の上で箱は新しい置き場所（要件 1.1）"
    );
    assert_eq!(
        cage.reports(),
        NO_REPORT,
        "箱は出たままなので組は欠けない（要件 2.1）"
    );

    cage.surface(0, PLAIN_SURFACE);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(PLAIN_SURFACE_ID, AT_FACE10),
        "`\\s` が後から届いても表示は替わらない"
    );
    assert_eq!(
        cage.reports(),
        NO_REPORT,
        "`\\s` が後から届いても組は欠けない"
    );
}

/// 2: 置き場所が違う面へ・`\s` が先（要件 1.1・2.1・4.1・4.2）。`\s` だけ届いたフレームでは
/// 絵も箱も前の置き場所のまま、空の組は届かない。絵が届いたフレームで両方が替わる。
#[test]
fn box_stays_with_the_picture_when_the_script_surface_arrives_first() {
    let mut cage = Cage::boot(SHELL_TWO_PLACES, &[0]);
    speak_in_box(&mut cage);

    cage.surface(0, PLAIN_SURFACE);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "`\\s` だけ届いたフレーム: 絵が前のままなので箱も前の置き場所（要件 1.1）"
    );
    assert_eq!(
        cage.reports(),
        NO_REPORT,
        "写しは空にならず、空の組は届かない（要件 2.1）"
    );

    cage.picture(0, PLAIN_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(PLAIN_SURFACE_ID, AT_FACE10),
        "絵が届いたフレームで箱も新しい置き場所へ（要件 1.1）"
    );
    assert_eq!(
        cage.reports(),
        NO_REPORT,
        "置き場所が替わるあいだも組は欠けない"
    );
}

/// 3（絵が先）: 窓 ↔ 箱の入れ替わり（要件 1.2・1.4・1.8・2.2・4.1・4.2）。絵が替わったフレームで
/// ちょうど片方だけが出て入れ替わり、組からスコープが外れない。
#[test]
fn window_and_box_swap_with_the_picture_when_the_picture_arrives_first() {
    let mut cage = Cage::boot(SHELL, &[0]);
    speak_in_both(&mut cage);

    // 箱 → 窓
    cage.picture(0, PLAIN_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        windowed(PLAIN_SURFACE_ID),
        "絵が替わったフレームで箱が消え、窓が出る（要件 1.2・1.8）"
    );
    assert_eq!(
        cage.reports(),
        NO_REPORT,
        "片方が出続けるので組は欠けない（要件 2.2）"
    );
    cage.surface(0, PLAIN_SURFACE);
    cage.frame();
    assert_eq!(
        cage.look(0),
        windowed(PLAIN_SURFACE_ID),
        "`\\s` が後から届いても同じ"
    );
    assert_eq!(cage.reports(), NO_REPORT);

    // 窓 → 箱
    cage.picture(0, BOX_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "絵が替わったフレームで窓が隠れ、保持していた箱の文字が出直す（要件 1.4・1.8）"
    );
    assert_eq!(
        cage.reports(),
        NO_REPORT,
        "片方が出続けるので組は欠けない（要件 2.2）"
    );
    cage.surface(0, BOX_SURFACE);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "`\\s` が後から届いても同じ"
    );
    assert_eq!(cage.reports(), NO_REPORT);
}

/// 3（`\s` が先）: 窓 ↔ 箱の入れ替わり（要件 1.2・1.4・1.8・2.2・4.1・4.2）。`\s` だけ届いた
/// フレームでは前の送り先を出し続け、絵が替わったフレームで入れ替わる。
#[test]
fn window_and_box_swap_with_the_picture_when_the_script_surface_arrives_first() {
    let mut cage = Cage::boot(SHELL, &[0]);
    speak_in_both(&mut cage);

    // 箱 → 窓
    cage.surface(0, PLAIN_SURFACE);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "`\\s` だけ届いたフレーム: 絵が前のままなので箱を出し続け、窓は出さない（要件 1.8）"
    );
    assert_eq!(cage.reports(), NO_REPORT, "組は欠けない（要件 2.2）");
    cage.picture(0, PLAIN_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        windowed(PLAIN_SURFACE_ID),
        "絵が替わったフレームで箱が消え、窓が出る（要件 1.2・1.8）"
    );
    assert_eq!(cage.reports(), NO_REPORT);

    // 窓 → 箱
    cage.surface(0, BOX_SURFACE);
    cage.frame();
    assert_eq!(
        cage.look(0),
        windowed(PLAIN_SURFACE_ID),
        "`\\s` だけ届いたフレーム: 絵が前のままなので窓を出し続け、箱は出さない（要件 1.8）"
    );
    assert_eq!(cage.reports(), NO_REPORT, "組は欠けない（要件 2.2）");
    cage.picture(0, BOX_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "絵が替わったフレームで窓が隠れ、保持していた箱の文字が出直す（要件 1.4・1.8）"
    );
    assert_eq!(cage.reports(), NO_REPORT);
}

/// 3（移る先が空）: 箱にだけ文字を持たせて箱の無い面へ切り替える（要件 1.2・1.8・2.3）。
/// 絵が替わったフレームで箱が消えて組からスコープが外れ、窓は出ない。1 字目のフレームで
/// 窓が出て組に戻る。
#[test]
fn moving_into_an_empty_window_drops_the_scope_until_the_first_glyph() {
    let mut cage = Cage::boot(SHELL, &[0]);
    speak_in_box(&mut cage);

    cage.picture(0, PLAIN_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        blank(PLAIN_SURFACE_ID),
        "絵が替わったフレームで箱が消え、移る先の窓は空なので出ない（要件 1.2・1.8）"
    );
    assert_eq!(
        cage.reports(),
        vec![Vec::new()],
        "箱も窓も出ていないフレームでスコープを外す（要件 2.3）"
    );

    cage.surface(0, PLAIN_SURFACE);
    cage.frame();
    assert_eq!(
        cage.look(0),
        blank(PLAIN_SURFACE_ID),
        "`\\s` だけでは何も出ない"
    );
    assert_eq!(cage.reports(), NO_REPORT);

    cage.text(0, "う");
    cage.frame();
    assert_eq!(
        cage.look(0),
        windowed(PLAIN_SURFACE_ID),
        "1 字目が届いたフレームで窓が出る"
    );
    assert_eq!(cage.reports(), vec![vec![(0, 0)]], "そのフレームで組に戻る");
}

/// 4: `\s` の後・絵の前に書いた文字（要件 1.9）。絵が替わる前のフレームは前の表示のまま、
/// 絵が替わったフレームで行き先（箱 → 窓なら窓・窓 → 箱なら箱）に出る。
#[test]
fn text_written_before_the_picture_arrives_appears_with_the_picture() {
    let mut cage = Cage::boot(SHELL, &[0]);
    speak_in_box(&mut cage);

    // 箱 → 窓: 窓には `\s` の後に書いた文字しか無い。
    cage.surface(0, PLAIN_SURFACE);
    cage.text(0, "う");
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "絵が替わる前のフレームは前の表示のまま（`\\s` の後の文字はまだ窓に出ない・要件 1.9）"
    );
    assert_eq!(cage.reports(), NO_REPORT);
    cage.picture(0, PLAIN_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        windowed(PLAIN_SURFACE_ID),
        "絵が替わったフレームで、あいだに書いた文字が窓に出る（要件 1.9）"
    );
    assert_eq!(cage.reports(), NO_REPORT);

    // 窓 → 箱
    cage.surface(0, BOX_SURFACE);
    cage.text(0, "え");
    cage.frame();
    assert_eq!(
        cage.look(0),
        windowed(PLAIN_SURFACE_ID),
        "絵が替わる前のフレームは前の表示のまま（`\\s` の後の文字はまだ箱に出ない・要件 1.9）"
    );
    assert_eq!(cage.reports(), NO_REPORT);
    cage.picture(0, BOX_SURFACE_ID);
    cage.frame();
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "絵が替わったフレームで箱に出る（要件 1.9）"
    );
    assert_eq!(cage.reports(), NO_REPORT);
}

/// 5: 出して隠した後に箱だけ（要件 3.1・3.2・4.3）。普通のバルーンの窓を一度出して隠した後に
/// 箱にだけ文字が出ても、警告の段の行は出ず、組は `(スコープ, 0)` で 1 度だけ届く。
#[test]
fn box_only_after_hiding_the_window_reports_without_a_warning() {
    let mut cage = Cage::boot(SHELL, &[0]);
    cage.picture(0, PLAIN_SURFACE_ID);
    cage.surface(0, PLAIN_SURFACE);
    cage.text(0, "あ");
    cage.frame();
    assert_eq!(cage.look(0), windowed(PLAIN_SURFACE_ID), "前提: 窓を出した");
    cage.cue(0, CueCommand::Clear);
    cage.frame();
    assert_eq!(cage.look(0), blank(PLAIN_SURFACE_ID), "前提: 窓を隠した");
    assert_eq!(
        cage.reports(),
        vec![vec![(0, 0)], Vec::new()],
        "前提: 窓が見えた組と消えた組が届いた"
    );

    cage.picture(0, BOX_SURFACE_ID);
    cage.surface(0, BOX_SURFACE);
    cage.text(0, "い");
    let mut events = cage.frame();
    events.extend(cage.frame());
    assert_eq!(
        cage.look(0),
        boxed(BOX_SURFACE_ID, AT_FACE0),
        "箱にだけ文字"
    );
    let warned: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .filter(|e| {
            e.field("event")
                .is_some_and(|v| v.contains("balloon_status_surface_unknown"))
        })
        .collect();
    assert_eq!(
        warned.len(),
        0,
        "箱だけの普通の使い方で警告の段の行を出さない（要件 3.2）: {warned:?}"
    );
    assert_eq!(
        cage.reports(),
        vec![vec![(0, 0)]],
        "要件 5.5 の番号（切り替えていない普通のバルーン＝0）で 1 度だけ載る（要件 3.1）"
    );

    cage.frame();
    assert_eq!(cage.reports(), NO_REPORT, "同じ組は送り直さない");
}
