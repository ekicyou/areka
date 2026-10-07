//! `dump_balloon` の実際の描画を通るテスト（spec: areka-P0-mcp-dump-images・要件 3.1・3.3・3.4・
//! 3.7・3.9・4.1・6.2・7.4 ⑶⑷⑸、spec: areka-P0-mcp-dump-images-residue・要件 1.3・5.1・7.1 ⑴・
//! x64 だけ接続）。土台は `dump_surface` の側の
//! [`crate::mcp::dump_surface::gpu_test_support`]。
//!
//! 台本の待ち（`\_w`）と指令は注入する Tick の時刻で届き、字は実時間の台詞の時刻
//! （巡の時刻−話の起点）で現れる（別の時計）。話の起点は届いた指令ごとの「届いた時刻−指令の時刻」の
//! 最大で、遅れて届く指令が後へ押す。台詞の字が現れ切ったかは、スコープ 0 の現れる時刻の列が台詞の
//! 字数だけ揃った（字は届いた分しか列に載らない）時刻 `T` から、列の末尾 `L` だけ実時間が過ぎた
//! ことで知る: `T` までに届いた指令は起点を `T` より後へ押せず、後から届く指令は自分の時刻（`L` 以上）
//! までの字を消さないので、台詞の時刻は `L` を越える。巡の途中で届いた指令の分の 1 巡のずれを
//! 見込み、2 巡続けて越えたら現れ切ったとする。

use std::time::Duration;

use areka_kanade::RaiseOutcome;
use areka_mcp::tools::dump_balloon::Args;
use areka_mcp::{ToolContent, ToolOutcome};
use areka_sakura::ActorKey;
use bevy_ecs::world::World;
use log_capture_kit::count_levels;
use wintf::ecs::FrameTime;

use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::spine::RecordedCall;
use crate::emo2_boot::target_map::balloon_target;
use crate::mcp::dump_surface::dump_surface_tests::is_picture;
use crate::mcp::dump_surface::gpu_test_support::{FLUSH_EVENT, GpuRig, decode_png, unpremultiply};
use crate::mcp::dump_surface::{Job, Step, finish};

/// スコープ 0 が話す字（[`LINE`]・[`LINE_THEN_HIDE`] の本文・[`LINE_THEN_ANOTHER`] の最初の本文）。
const WORDS: &str = "表示の確かめ";
/// スコープ 0 だけが話す台詞（surface は検体の `element0` 1 枚の 1101・`dump_surface` の側と同じ）。
const LINE: &str = r"\0\s[1101]表示の確かめ\e";
/// [`LINE`] の後、Tick の時刻で 1.5 秒待ってバルーンを隠す台詞（Tick を実時間ほどで刻めば、
/// 字が実時間で現れ切った後に隠れる。先に隠すと、後から現れた字で表示が戻る）。
const LINE_THEN_HIDE: &str = r"\0\s[1101]表示の確かめ\_w[1500]\b[-1]\e";
/// [`LINE`] の後、Tick の時刻で 1.5 秒待って文字を消し、別の字を話す台詞（呼び出しの後に台詞を
/// 進めて文字の面を変えるため。待ちの理由は [`LINE_THEN_HIDE`] と同じ）。
const LINE_THEN_ANOTHER: &str = r"\0\s[1101]表示の確かめ\_w[1500]\c書き替えた\e";

/// 成功の本文（要件 3.7）。
fn ok_text(scope: u32) -> String {
    format!(
        "OK:balloon of scope {scope} as last drawn (before scaling and transparency; kept even if \
         the balloon is hidden now)"
    )
}

/// `scope` の普通のバルーンの文字の面の（大きさ, 読み戻した乗算済み BGRA）。面が無ければ `None`。
fn text_surface(world: &World, scope: u32) -> Option<((u32, u32), Vec<u8>)> {
    let runtime = world.get_non_send::<Emo2Wiring>()?.runtime().borrow();
    let surface = runtime.surface(&ActorKey::from(scope.to_string()))?;
    Some((surface.size(), surface.read_back().ok()?))
}

/// 透明でない画素を持つ。
fn has_ink(bytes: &[u8]) -> bool {
    bytes.as_chunks::<4>().0.iter().any(|p| p[3] != 0)
}

/// スコープ 0 の、届いた字の数と最後の字が現れる台詞の時刻（秒）。字が 1 つも無ければ `None`。
fn reveals(world: &World) -> Option<(usize, f64)> {
    let runtime = world.get_non_send::<Emo2Wiring>()?.runtime().borrow();
    let times = runtime
        .state()
        .actor_state(&ActorKey::from("0".to_owned()))?
        .reveal()
        .times();
    Some((times.len(), *times.last()?))
}

/// スコープ 0 の台詞の字が現れ切るまで、巡ごとに Tick の時刻を `step_ms` 進めて回す
/// （期限切れは `false`・判定はファイルの頭の説明のとおり）。
fn speak(gpu: &mut GpuRig, step_ms: u64) -> bool {
    let words = WORDS.chars().count();
    // 字の列が揃ったのを見た時刻（`T`）と、前の巡で台詞の時刻が末尾を越えていたか。
    let mut all_by: Option<f64> = None;
    let mut past_before = false;
    gpu.frames_until(step_ms, |world| {
        let Some((_, last)) = reveals(world).filter(|&(n, _)| n == words) else {
            return false;
        };
        let all_by = *all_by.get_or_insert_with(dola::runtime::clock::now);
        let past = world.resource::<FrameTime>().0 - all_by >= last;
        std::mem::replace(&mut past_before, past) && past
    })
}

/// スコープ 0 のバルーンの可視。
fn balloon_visible(world: &World) -> Option<bool> {
    world
        .non_send::<Emo2Wiring>()
        .presenter()
        .target_visible(balloon_target(0))
}

/// 本文＋画像 1 枚の答えの（isError, 本文, MIME, 復号した（幅, 高さ, 乗算していない RGBA））。
/// 形が違えば panic。
fn read_picture(outcome: &ToolOutcome) -> (bool, String, String, (u32, u32, Vec<u8>)) {
    let [
        ToolContent::Text(text),
        ToolContent::Image { data, mime_type },
    ] = outcome.content.as_slice()
    else {
        panic!("本文＋画像 1 枚の形ではない: {:?}", outcome.content);
    };
    (
        outcome.is_error,
        text.clone(),
        mime_type.clone(),
        decode_png(data),
    )
}

/// `a` と `b` の（大きさが同じなら）食い違った画素の数。大きさが違えば `None`。
fn differing(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>)) -> Option<usize> {
    ((a.0, a.1) == (b.0, b.1)).then(|| {
        a.2.as_chunks::<4>()
            .0
            .iter()
            .zip(b.2.as_chunks::<4>().0)
            .filter(|(p, q)| p != q)
            .count()
    })
}

/// 背景（乗算済み BGRA）の上へ、文字の面（乗算済み BGRA）を `origin` へそのまま重ねる
/// （縮めない・乗算済みの「上に重ねる」・画素ごとに整数で丸める）。検査の側で別に組む期待の絵。
fn place_text(
    background: &(u32, u32, Vec<u8>),
    text: &((u32, u32), Vec<u8>),
    origin: (u32, u32),
) -> Vec<u8> {
    let (w, h, bg) = background;
    let ((tw, th), ink) = text;
    let mut out = bg.clone();
    for ty in 0..*th {
        for tx in 0..*tw {
            let (x, y) = (origin.0 + tx, origin.1 + ty);
            if x >= *w || y >= *h {
                continue;
            }
            let s = &ink[((ty * tw + tx) * 4) as usize..][..4];
            let d = &mut out[((y * w + x) * 4) as usize..][..4];
            let keep = 255 - u32::from(s[3]);
            for (dst, &src) in d.iter_mut().zip(s) {
                *dst = (u32::from(src) + (u32::from(*dst) * keep + 127) / 255).min(255) as u8;
            }
        }
    }
    out
}

/// 背景と違う画素の外接矩形（左, 上, 右, 下・両端を含む）。違う画素が無ければ `None`。
fn bbox_against(
    picture: &(u32, u32, Vec<u8>),
    background: &(u32, u32, Vec<u8>),
) -> Option<(u32, u32, u32, u32)> {
    let w = picture.0;
    picture
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .zip(background.2.as_chunks::<4>().0)
        .enumerate()
        .filter(|(_, (p, q))| p != q)
        .map(|(i, _)| (i as u32 % w, i as u32 / w))
        .fold(None, |acc, (x, y)| {
            let (l, t, r, b) = acc.unwrap_or((x, y, x, y));
            Some((l.min(x), t.min(y), r.max(x), b.max(y)))
        })
}

/// 拡大率 1（要件 3.1・3.7・7.4 ⑶）: 台詞の後の `dump_balloon`（省略）の画素が、背景の合成の上へ
/// 文字の面の読み戻しを、バルーンの定義の文字の領域の原点へそのまま重ねたものと一致する。
///
/// # 非空虚性
/// 文字の面に透明でない画素が在ることを先に確かめる（無ければ背景だけの比較になり何も固定しない）。
/// 原点の渡し忘れ（文字が左上へ寄る）・縦横の取り違えは画素が食い違って赤。文字を重ねないと
/// 背景だけになって赤。原点が整数であることも確かめる（期待の絵の組み方の前提）。
#[test]
fn balloon_text_lies_on_the_background_at_the_text_area_origin() {
    let mut gpu = GpuRig::new(LINE, 96);
    let spoke = speak(&mut gpu, 100);
    let text = text_surface(&gpu.rig.world, 0);
    let pending = gpu.dump_balloon(None);
    let answer = gpu.answer_of(&pending);
    let background = gpu.balloon_composed(0);
    let origin = gpu.text_area_origin(0, (background.0, background.1));
    let down = gpu.shutdown();

    let text = text.expect("台詞の後は文字の面が在る");
    assert!(has_ink(&text.1), "台詞の後の文字の面に透明でない画素が在る");
    assert_eq!(origin, (origin.0.trunc(), origin.1.trunc()), "原点は整数");
    let expected = (
        background.0,
        background.1,
        unpremultiply(&place_text(
            &background,
            &text,
            (origin.0 as u32, origin.1 as u32),
        )),
    );
    let answer = answer.expect("装着の後は上限のうちに答えが届く");
    let (is_error, body, mime, picture) = read_picture(&answer.outcome);
    assert_eq!(
        (
            spoke,
            down,
            is_error,
            body,
            mime,
            differing(&picture, &expected)
        ),
        (
            true,
            true,
            false,
            ok_text(0),
            "image/png".to_owned(),
            Some(0)
        ),
        "（字が現れ切った, 降ろせた, isError, 本文, MIME, 期待の絵と食い違った画素の数）"
    );
}

/// 拡大率が 1 でない（要件 3.3・7.4 ⑶⑸）: 窓の `DPI` を 144 にすると、大きさは原寸（背景の合成と
/// 同じ）のままで、背景と違う画素の外接矩形が拡大率 1 のときと上下左右 1 画素以内で一致する。
///
/// # 非空虚性
/// 144 で文字の面が 96 のときより大きい（拡大した面を縮める経路を本当に通る）ことと、外接矩形が
/// 在ることを確かめる。縮める比の逆・位置のずれ（拡大した原点をそのまま使う）は外接矩形がずれて赤。
/// 拡大した大きさで返すと大きさで赤。
#[test]
fn balloon_at_dpi_144_keeps_native_size_and_text_position() {
    let run = |dpi: u16| {
        let mut gpu = GpuRig::new(LINE, dpi);
        let spoke = speak(&mut gpu, 100);
        let text_size = text_surface(&gpu.rig.world, 0).map(|(size, _)| size);
        let pending = gpu.dump_balloon(None);
        let answer = gpu.answer_of(&pending);
        let background = gpu.balloon_composed(0);
        let down = gpu.shutdown();
        let answer = answer.expect("装着の後は上限のうちに答えが届く");
        let (is_error, body, _, picture) = read_picture(&answer.outcome);
        let background = (background.0, background.1, unpremultiply(&background.2));
        (
            (spoke, down, is_error, body),
            text_size,
            (picture.0, picture.1),
            (background.0, background.1),
            bbox_against(&picture, &background),
        )
    };
    let (facts1, text1, size1, native1, bbox1) = run(96);
    let (facts15, text15, size15, native15, bbox15) = run(144);

    let ok = (true, true, false, ok_text(0));
    assert_eq!(
        (facts1, facts15, size1, size15),
        (ok.clone(), ok, native1, native15),
        "（96 の事実, 144 の事実, 96 の大きさ, 144 の大きさ）"
    );
    let (t1, t15) = (
        text1.expect("96 の文字の面"),
        text15.expect("144 の文字の面"),
    );
    assert!(
        t15.0 > t1.0 && t15.1 > t1.1,
        "144 の文字の面は 96 より大きい: {t1:?} → {t15:?}"
    );
    let (b1, b15) = (
        bbox1.expect("96 の外接矩形"),
        bbox15.expect("144 の外接矩形"),
    );
    let near = |a: u32, b: u32| a.abs_diff(b) <= 1;
    assert!(
        near(b1.0, b15.0) && near(b1.1, b15.1) && near(b1.2, b15.2) && near(b1.3, b15.3),
        "外接矩形（左, 上, 右, 下）が 1 画素以内で一致: 96 {b1:?} / 144 {b15:?}"
    );
}

/// 隠れたバルーン（要件 3.4・7.4 ⑷）: 字が現れ切ってからバルーンを隠した後の `dump_balloon` が、
/// 隠す前と同じ画素を成功で返す。
///
/// # 非空虚性
/// 隠す前は可視・後は不可視であることと、隠す前の絵に文字が載っている（背景と違う）ことを確かめる。
/// 隠している間に断る・背景だけ・空の絵を返すと赤。
#[test]
fn hidden_balloon_returns_the_same_pixels() {
    // Tick を実時間ほど（1 ms ごとに 1 ms）で刻み、字が現れ切った後に隠れるようにする。
    let mut gpu = GpuRig::new(LINE_THEN_HIDE, 96);
    let spoke = speak(&mut gpu, 1);
    let visible_before = balloon_visible(&gpu.rig.world);
    let pending = gpu.dump_balloon(None);
    let before = gpu.answer_of(&pending);
    // 字は現れ切った（`speak`）ので、残りの `\_w[1500]` は Tick を大きく刻んで進める（1 ms ずつでは
    // 1,500 巡以上が要り、巡の遅い机で待ちが届かない・[`read_after_talk`] と同じ）。
    let hidden = gpu.frames_until(100, |world| balloon_visible(world) == Some(false));
    let pending = gpu.dump_balloon(None);
    let after = gpu.answer_of(&pending);
    let background = gpu.balloon_composed(0);
    let down = gpu.shutdown();

    let (b_err, b_body, _, before) = read_picture(&before.expect("隠す前").outcome);
    let (a_err, a_body, _, after) = read_picture(&after.expect("隠した後").outcome);
    let background = (background.0, background.1, unpremultiply(&background.2));
    assert!(
        bbox_against(&before, &background).is_some(),
        "隠す前の絵に文字が載っている"
    );
    assert_eq!(
        (
            spoke,
            visible_before,
            hidden,
            down,
            (b_err, b_body),
            (a_err, a_body),
            differing(&after, &before)
        ),
        (
            true,
            Some(true),
            true,
            true,
            (false, ok_text(0)),
            (false, ok_text(0)),
            Some(0)
        ),
        "（字が現れ切った, 隠す前の可視, 隠れた, 降ろせた, 前の答え, 後の答え, 前後で食い違った画素の数）"
    );
}

/// 一度も話していないスコープ（要件 3.9・7.4 ⑸）: スコープ 0 だけが話した後のスコープ 1 の
/// `dump_balloon` が、スコープ 1 のバルーンの背景の合成と一致する絵を成功で返す。
///
/// # 非空虚性
/// スコープ 0 の文字が載ったこと（台詞が届いた）を確かめる。文字の無いバルーンを断る・スコープ 0 の
/// 絵（文字入り・別の面）を返すと赤。
#[test]
fn never_spoken_scope_returns_only_the_background() {
    let mut gpu = GpuRig::new(LINE, 96);
    let spoke = speak(&mut gpu, 100);
    let pending = gpu.dump_balloon(Some(1));
    let answer = gpu.answer_of(&pending);
    let background = gpu.balloon_composed(1);
    let down = gpu.shutdown();

    let background = (background.0, background.1, unpremultiply(&background.2));
    let (is_error, body, mime, picture) =
        read_picture(&answer.expect("装着の後は上限のうちに答えが届く").outcome);
    assert_eq!(
        (
            spoke,
            down,
            is_error,
            body,
            mime,
            differing(&picture, &background)
        ),
        (
            true,
            true,
            false,
            ok_text(1),
            "image/png".to_owned(),
            Some(0)
        ),
        "（字が現れ切った, 降ろせた, isError, 本文, MIME, 背景の合成と食い違った画素の数）"
    );
}

/// スコープ 2 と邪魔をしない（要件 4.1・6.2）: 成功と判断の失敗を混ぜた一連の `dump_balloon` の前後で、
/// 偽の SHIORI の呼び出しの列が後の関所の印 1 件のほかに増えず、ERROR の記録が 0 件。スコープ 2 と
/// 負のスコープは `NG:No such scope in this ghost`。
///
/// # 非空虚性
/// 前後とも関所（[`GpuRig::flush_to_shiori`]）を通してから列を読む。ゴーストへイベントを送ると列が
/// 伸びて赤。判断の失敗の記録を `error!` にすると件数で赤。成功の 3 回が画像つきで答えたことと、
/// 前の列に `OnBoot` が載っている（観測口が生きている）ことも確かめる。
#[test]
fn scope_two_is_no_such_scope_and_calls_do_not_disturb_the_ghost() {
    let mut gpu = GpuRig::new(LINE, 96);
    let spoke = speak(&mut gpu, 100);
    let first = gpu.flush_to_shiori();
    let before = gpu.calls();
    let (answers, levels) = count_levels(|| {
        [None, Some(0), Some(1), Some(2), Some(-1)].map(|scope| {
            let pending = gpu.dump_balloon(scope);
            gpu.answer_of(&pending).map(|a| a.outcome)
        })
    });
    let second = gpu.flush_to_shiori();
    let after = gpu.calls();
    let down = gpu.shutdown();

    let booted = before
        .iter()
        .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnBoot"));
    let mut expected = before.clone();
    expected.push(RecordedCall::Notify {
        id: FLUSH_EVENT.to_owned(),
        references: Vec::new(),
    });
    let shapes = answers.each_ref().map(|a| {
        a.as_ref().map(|o| {
            (
                o.is_error,
                o.content.len(),
                matches!(o.content.last(), Some(ToolContent::Image { .. })),
            )
        })
    });
    let no_scope = ToolOutcome {
        content: vec![ToolContent::Text(
            "NG:No such scope in this ghost".to_owned(),
        )],
        is_error: true,
    };
    let picture = Some((false, 2, true));
    let sent = Some(RaiseOutcome::NoReply);
    assert_eq!(
        (
            spoke,
            first,
            second,
            down,
            booted,
            shapes[..3].to_vec(),
            answers[3..].to_vec(),
            levels.error,
            after
        ),
        (
            true,
            sent,
            sent,
            true,
            true,
            vec![picture; 3],
            vec![Some(no_scope.clone()), Some(no_scope)],
            0,
            expected
        ),
        "（字が現れ切った, 前の関所, 後の関所, 降ろせた, 前の列に OnBoot, 成功の 3 回の形, \
         スコープ 2・-1 の答え, ERROR の件数, 後の呼び出しの列）"
    );
}

/// 呼び出しを受けた時点の期待の絵と、そのとき受けた読み出しの口を台詞を進めてから読んだ結果。
struct ReadAfterTalk {
    /// 最初の台詞の字が現れ切った。
    spoke: bool,
    /// 呼び出しの時点の文字の面に透明でない画素が在った。
    inked: bool,
    /// 読み出しを待つ間に、文字の面の読み戻しが呼び出しの時点と違うようになった。
    changed: bool,
    /// 置き場のゴーストを降ろせた。
    down: bool,
    /// 呼び出しの時点の背景と文字から組んだ期待の絵（乗算していない RGBA）。
    expected: (u32, u32, Vec<u8>),
    /// 読み出しの口の最後の結果（読めたら仕事・失敗なら答え・期限切れは `Ok(None)`）。
    read: Result<Option<Job>, ToolOutcome>,
}

/// [`LINE_THEN_ANOTHER`] の最初の台詞を出し、そのときの背景の合成と文字の面から期待の絵を組んで、
/// 本物の `answer` から読み出しを待つ種を受け取る。次に Tick を進めて文字の面が呼び出しの時点と
/// 違うようになるまで台詞を進め、それから読み出しの口を読めるまで（有界に）呼ぶ。
fn read_after_talk() -> ReadAfterTalk {
    let mut gpu = GpuRig::new(LINE_THEN_ANOTHER, 96);
    // Tick を実時間ほどで刻み、最初の字が現れ切ってから文字を消す（[`LINE_THEN_HIDE`] と同じ）。
    let spoke = speak(&mut gpu, 1);
    let text = text_surface(&gpu.rig.world, 0).expect("台詞の後は文字の面が在る");
    let background = gpu.balloon_composed(0);
    let origin = gpu.text_area_origin(0, (background.0, background.1));
    assert_eq!(origin, (origin.0.trunc(), origin.1.trunc()), "原点は整数");
    let args = Args {
        scope: None,
        ghost_name: None,
    };
    let Some(Step::Read(_, mut reader)) = super::answer(&gpu.rig.world, &args) else {
        panic!("文字の面が在れば、答えの種は読み出しを待つ種");
    };
    let changed = gpu.frames_until(100, |world| {
        text_surface(world, 0).is_some_and(|now| now != text)
    });
    let mut read = Ok(None);
    gpu.frames_until(0, |_| {
        read = reader();
        !matches!(read, Ok(None))
    });
    let down = gpu.shutdown();

    let expected = (
        background.0,
        background.1,
        unpremultiply(&place_text(
            &background,
            &text,
            (origin.0 as u32, origin.1 as u32),
        )),
    );
    ReadAfterTalk {
        spoke,
        inked: has_ink(&text.1),
        changed,
        down,
        expected,
        read,
    }
}

/// 呼び出しを受けた時点の絵（要件 1.3・7.1 ⑴）: 読み出しを待つ種を受け取った後に台詞を進めて
/// 文字の面を変えても、読み出しの口から得た仕事を走らせた絵は、呼び出しの時点の背景と文字から
/// 組んだ期待の絵と一致する。
///
/// # 非空虚性
/// 呼び出しの時点の文字の面に字が在ることと、読む前に文字の面の読み戻しが呼び出しの時点と
/// 違うようになったこと（`changed`）を確かめる。文字の面の写しを読み出しのフレームで取り直すと、
/// 後の内容（消した後の別の字）が載って画素が食い違い赤。
#[test]
fn balloon_is_the_picture_at_the_call_even_after_the_talk_moves_on() {
    let read = read_after_talk();

    let job = read
        .read
        .expect("読み出しは失敗しない")
        .expect("上限のうちに読める");
    let answered = finish(super::TOOL, 0, job, Duration::ZERO);
    let (is_error, body, mime, picture) = read_picture(&answered);
    assert_eq!(
        (
            read.spoke,
            read.inked,
            read.changed,
            read.down,
            is_error,
            body,
            mime,
            differing(&picture, &read.expected)
        ),
        (
            true,
            true,
            true,
            true,
            false,
            ok_text(0),
            "image/png".to_owned(),
            Some(0)
        ),
        "（字が現れ切った, 呼び出しの時点の文字の面に字が在った, 読む前に文字の面が変わった, \
         降ろせた, isError, 本文, MIME, 期待の絵と食い違った画素の数）"
    );
}

/// 符号化のスレッドの記録（要件 5.1）: 上と同じ形で得た `dump_balloon` の本物の仕事を、記録を
/// 数える中で符号化のスレッドの体（`finish`）に通すと、ERROR が 0 件で画像つきの成功。
///
/// # 非空虚性
/// 本物の重ね合わせ・PNG・base64 を通る（読めた仕事が在ることを確かめる）。判定の関数
/// `is_picture` と `finish` の ERROR の数え方は `dump_surface_tests.rs` で較正済み。仕事の中で
/// `fail` を呼ぶと件数と判定の両方で赤。
#[test]
fn balloon_job_runs_on_the_encoding_body_without_error() {
    let read = read_after_talk();

    let job = read
        .read
        .expect("読み出しは失敗しない")
        .expect("上限のうちに読める");
    let (answered, levels) = count_levels(|| finish(super::TOOL, 0, job, Duration::ZERO));
    assert_eq!(
        (
            read.spoke,
            read.changed,
            read.down,
            is_picture(&answered),
            levels.error
        ),
        (true, true, true, true, 0),
        "（字が現れ切った, 読む前に文字の面が変わった, 降ろせた, 画像つきの成功, ERROR の件数）"
    );
}
