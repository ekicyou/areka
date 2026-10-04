// =============================================================================
// 箱の結線の通し（areka-P0-shell-balloon task 12.3）
//
// 親（`frame_visibility_integration_tests.rs`）と同じ檻——実 emo2 fixture ＋ 実 GPU（WARP 可・MTA）で
// 本番の `emo2_frame_system` を回す——に、箱の束（サーフェス 0 にだけ箱 `talk`）を渡して、
// 可視性の相・文字の層の拡大率の相（`sync_boxes`）・提示（`shown_boxes` の写し）・`Status` の届けが
// 本番の相順でつながっていることを確かめる。
//
// 写し（`shown_boxes`）は提示（GPU）でしか埋まらないので、相の単体の檻（`balloon_visibility_phase_box_tests.rs`）
// は写しを作って渡していた。ここではそれを本物の提示で埋め、次のフレームの可視性の相が読む。
//
// 場面（要件 5.1・5.2・5.3・5.5・6.4・6.9・6.10）:
//   ⑴ 箱のあるサーフェスでは普通のバルーンの窓を出す発行が 0 件、箱の無いサーフェスへ移ると出る。
//      窓に書いていた文字は箱のあるサーフェスのあいだ保持され、箱の無いサーフェスへ戻ると窓が出直す。
//   ⑵ `\0` と `\1` を別々に判定する。
//   ⑶ 時間切れで箱を隠す印が立ち、箱の文字が消える。
//   ⑷ `Status` の組へ箱のスコープが載る。
//
// 時刻はすべて注入（`FrameTime`＋`TalkClock` の epoch）で、実時間の待機は用いない。
// =============================================================================

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_compose::{BindSet, EmoWorld, PatternState, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};

use crate::emo2_boot::balloon_visibility::configured_timeout_secs;
use crate::emo2_boot::shell_box_assets::ShellBoxAssets;
use crate::emo2_boot::target_map::shell_target;
use crate::input_events::shell_box::ShellBoxHover;

use super::*;

/// サーフェス 0 に箱 `talk`。サーフェス 10 は箱を持たない（面の表には在る）。
const SHELL: &str = "\
balloon.talk
{
size,100,40
}
surface0
{
element1,balloon,talk,10,10
}
";
/// 箱のあるサーフェスの鍵。
const BOX_SURFACE: &str = "0";
/// 箱の無いサーフェスの鍵。
const PLAIN_SURFACE: &str = "10";
/// 箱のあるサーフェスの番号（絵の差し替えで表示層へ送る形）。
const BOX_SURFACE_ID: u32 = 0;
/// 箱の無いサーフェスの番号（同上）。
const PLAIN_SURFACE_ID: u32 = 10;

/// 箱を隠す印を立てたときに文字の層が残す 1 行（`actor_box.rs` の `hide_boxes`）。
const HIDE_BOXES_MESSAGE: &str = "箱を隠す印を立てた（次の台詞の頭まで）";

/// 檻を組んで装着のフレームまで回し、箱の束を渡してシェルの窓の表示を確立させる。
///
/// emo2 のシェルは箱を持たないので、装着の相が渡した束を `shell`（シェルの文面）の束で差し替える
/// （本番ではシェルの切替の後始末が同じ `hand_to` で差し替える）。シェルの窓の文字の差し込み口は最初の
/// `\s` 相当の表示で確立するので、`scopes` の分だけ面 0 の表示を送っておく（次のフレームの drain が適用する）。
struct Cage {
    world: World,
    wiring: Emo2Wiring,
    present_tx: mpsc::Sender<PresentCommand>,
    lifecycle_tx: mpsc::Sender<TalkLifecycleSignal>,
    reports: mpsc::Receiver<areka_kanade::KanadeMsg>,
    _gw: crate::placement::spawn::GhostWindows,
}

impl Cage {
    fn boot(shell: &str, shell_scopes: &[u32]) -> Self {
        let (mut world, gw) = gpu_frame_world();
        // 本番は `wire_balloon_choice` が入れる。無いと箱に文字が出たフレームで観測の欠落が 1 行鳴る。
        world.insert_non_send(ShellBoxHover::default());
        let reports = seat_ghost(&mut world);
        let (present_tx, present_rx) = mpsc::channel::<PresentCommand>();
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
        let mut wiring = boot_wiring(present_rx, lifecycle_rx);
        wiring.clock.observe_cue(0.0);
        world.insert_resource(FrameTime(5.0));

        wiring = advance_frame(&mut world, wiring);
        assert_eq!(
            wiring.balloon_model_scopes(),
            vec![0u32, 1],
            "前提: 両 scope の balloon 装着が成立している"
        );

        let text_world = EmoWorld::build(&parse(shell));
        let (layout, report) = fold_boxes(&parse_boxes(shell), &BTreeMap::new(), &text_world);
        assert_eq!(report.issues, vec![], "文面は誤りを持たない");
        ShellBoxAssets {
            layout,
            aliases: text_world.alias_snapshot(),
            surface_ids: BTreeSet::from([0, 10]),
            font_dirs: Vec::new(),
        }
        .hand_to(&mut wiring.runtime.borrow_mut(), &mut world);

        let mut cage = Self {
            world,
            wiring,
            present_tx,
            lifecycle_tx,
            reports,
            _gw: gw,
        };
        for &scope in shell_scopes {
            cage.picture(scope, 0);
        }
        cage.frame();
        for &scope in shell_scopes {
            assert!(
                cage.wiring
                    .presenter
                    .text_slot_view(shell_target(scope))
                    .is_some(),
                "前提: scope{scope} のシェルの窓の文字の差し込み口が確立している"
            );
        }
        assert!(
            balloon_reports(&cage.reports).is_empty(),
            "前提: まだ何も見えていない"
        );
        cage
    }

    /// 本番の相順を 1 フレーム回し、その間のログを返す。
    fn frame(&mut self) -> Vec<LogEvent> {
        let wiring = std::mem::replace(&mut self.wiring, placeholder_wiring());
        let (wiring, events) = capture_logs(|| advance_frame(&mut self.world, wiring));
        self.wiring = wiring;
        events
    }

    /// 本番の adapter が流すのと同じ形で、scope の文字の層へ cue を当てる（時刻 0 起点）。
    fn cue(&self, scope: u32, command: CueCommand) {
        self.wiring.runtime.borrow_mut().apply_cue(&TalkCue {
            at: 0.0,
            actor: ActorKey::from(scope.to_string()),
            command,
            duration: 0.0,
        });
    }

    fn surface(&self, scope: u32, key: &str) {
        self.cue(scope, CueCommand::Emote { key: key.into() });
    }

    fn text(&self, scope: u32, text: &str) {
        self.cue(scope, CueCommand::Text(text.into()));
    }

    /// scope のシェルの窓へ絵の差し替えを送る（本番の adapter が `\s` から作る `ShowSurface` と同じ形・
    /// 次のフレームの drain が適用する）。台本の `\s`（[`Self::surface`]）とは別の口で、届く順を檻が決める。
    fn picture(&self, scope: u32, surface_id: u32) {
        self.present_tx
            .send(PresentCommand::ShowSurface {
                target: shell_target(scope),
                surface_id,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: None,
            })
            .expect("受信端は結線資源が保持している");
    }

    /// scope のシェルの窓へ絵の非表示（`\s[-1]` 相当）を送る（次のフレームの drain が適用する）。
    fn hide_picture(&self, scope: u32) {
        self.present_tx
            .send(PresentCommand::Hide {
                target: shell_target(scope),
                reply: None,
            })
            .expect("受信端は結線資源が保持している");
    }

    /// scope のシェルの窓に表示層が今表示している絵の番号（非表示なら `None`）。
    fn picture_id(&self, scope: u32) -> Option<u32> {
        self.wiring
            .presenter
            .current_surface_id(shell_target(scope))
    }

    fn window_visible(&self, scope: u32) -> Option<bool> {
        self.wiring.presenter.target_visible(balloon_target(scope))
    }

    fn shown_boxes(&self, scope: u32) -> Vec<String> {
        self.wiring
            .runtime
            .borrow()
            .shown_boxes(&ActorKey::from(scope.to_string()))
            .iter()
            .map(|b| b.name.as_str().to_owned())
            .collect()
    }
}

/// `advance_frame` へ渡している間だけ `Cage::wiring` に置く空の結線（相は回さない）。
fn placeholder_wiring() -> Emo2Wiring {
    boot_wiring(mpsc::channel().1, mpsc::channel().1)
}

/// scope の普通のバルーンの窓を「出す」遷移の行の数（可視性の相が出す 1 行）。
fn show_transitions(events: &[LogEvent], scope: u32) -> usize {
    visibility_lines(events)
        .into_iter()
        .filter(|e| {
            e.message().contains("可視状態が遷移した")
                && e.field("scope") == Some(scope.to_string().as_str())
                && e.field("visible") == Some("true")
        })
        .count()
}

/// ⑴＋⑷: 箱のあるサーフェスでは窓を出す発行が 0 件で、箱の文字のスコープが `Status` に載る。
/// 箱の無いサーフェスへ移ると窓が出る（要件 5.1・5.2・5.5・6.4）。窓に書いた文字は箱のある
/// サーフェスのあいだ保持され、箱の無いサーフェスへ戻ると窓が出直す（要件 6.9）。
#[test]
fn box_surface_withholds_the_balloon_window_and_a_plain_surface_shows_it() {
    let mut cage = Cage::boot(SHELL, &[0]);

    // 箱のあるサーフェスで話す。提示が写しを埋めるのはこのフレームの終わりなので 2 フレーム回す。
    cage.surface(0, BOX_SURFACE);
    cage.text(0, "あい");
    let mut events = cage.frame();
    events.extend(cage.frame());
    assert_eq!(
        cage.shown_boxes(0),
        vec!["talk".to_owned()],
        "前提: 拡大率の相が箱を同期し、提示が箱 talk を面にした"
    );
    assert_eq!(
        show_transitions(&events, 0),
        0,
        "箱のあるサーフェスでは普通のバルーンの窓を出す発行が 0 件（要件 5.1）: {events:?}"
    );
    assert_eq!(cage.window_visible(0), Some(false));
    assert_eq!(
        balloon_reports(&cage.reports),
        vec![vec![(0, 0)]],
        "箱に文字が出ているスコープは窓が見えているときと同じ形で載る（要件 5.5）"
    );

    // 箱の無いサーフェスへ移って話す: 以後の文字は窓へ書かれ、窓が出る（要件 5.2・6.4）。
    cage.surface(0, PLAIN_SURFACE);
    cage.text(0, "う");
    let events = cage.frame();
    assert_eq!(
        show_transitions(&events, 0),
        1,
        "箱の無いサーフェスへ移ると窓が出る: {events:?}"
    );
    assert_eq!(cage.window_visible(0), Some(true));
    assert!(
        cage.shown_boxes(0).is_empty(),
        "箱の文字は保持したまま表示をやめる（要件 6.4）"
    );
    cage.frame();
    assert!(
        cage.shown_boxes(0).is_empty(),
        "次の提示でも箱は面にならない"
    );
    assert!(
        balloon_reports(&cage.reports).is_empty(),
        "箱から窓へ移っても見えている組は同じ（送り直さない）"
    );

    // 窓に文字が在るまま箱のあるサーフェスへ: 窓は消え、箱の文字が出直す（要件 6.9・6.3）。
    cage.surface(0, BOX_SURFACE);
    let mut events = cage.frame();
    events.extend(cage.frame());
    assert_eq!(cage.window_visible(0), Some(false), "窓の表示をやめる");
    assert_eq!(show_transitions(&events, 0), 0);
    assert_eq!(cage.shown_boxes(0), vec!["talk".to_owned()]);
    assert!(
        events
            .iter()
            .all(|e| !e.message().contains(HIDE_BOXES_MESSAGE)),
        "文字が窓から外れたことによる非表示は箱を隠さない: {events:?}"
    );

    // 同じ台詞のまま箱の無いサーフェスへ戻ると、保持していた文字で窓が出直す（要件 6.9 の逆向き）。
    cage.surface(0, PLAIN_SURFACE);
    let events = cage.frame();
    assert_eq!(
        show_transitions(&events, 0),
        1,
        "保持していた文字で窓が出直す: {events:?}"
    );
    assert_eq!(cage.window_visible(0), Some(true));
    assert_eq!(
        cage.wiring.runtime.borrow().balloon_shown_glyphs(
            &ActorKey::from("0"),
            cage.wiring.presenter().current_surface_id(shell_target(0)),
            5.0,
        ),
        1,
        "窓に出ているのは箱の無いサーフェスで書いた文字だけ（箱の文字は窓へ混ざらない）"
    );
}

/// ⑵＋⑷: `\0` が箱のあるサーフェス・`\1` が箱の無いサーフェスなら、`\1` の窓だけが出て、
/// `\0` は箱の文字で、両方が `Status` の組に載る（要件 5.3・5.5）。
#[test]
fn scopes_are_judged_separately() {
    let mut cage = Cage::boot(SHELL, &[0, 1]);

    cage.surface(0, BOX_SURFACE);
    cage.text(0, "あい");
    cage.surface(1, PLAIN_SURFACE);
    cage.text(1, "う");
    let mut events = cage.frame();
    events.extend(cage.frame());

    assert_eq!(show_transitions(&events, 0), 0, "{events:?}");
    assert_eq!(show_transitions(&events, 1), 1, "{events:?}");
    assert_eq!(cage.window_visible(0), Some(false));
    assert_eq!(cage.window_visible(1), Some(true));
    assert_eq!(cage.shown_boxes(0), vec!["talk".to_owned()]);
    assert!(cage.shown_boxes(1).is_empty());
    assert_eq!(
        balloon_reports(&cage.reports).last(),
        Some(&vec![(0, 0), (1, 0)]),
        "箱のスコープと窓のスコープが並んで載る"
    );
}

/// ⑶＋⑷: 台詞が終わって待ち時間が過ぎると、箱だけに文字が出ているスコープにも箱を隠す印が
/// 立ち、箱の文字が消え、`Status` の組から外れる（要件 6.10・5.6）。
#[test]
fn timeout_raises_the_hide_boxes_flag_and_the_box_text_disappears() {
    // 待ち時間の確定はプロセスで一度きりの記録なので、捕捉窓の外で先に確定させる。
    let timeout_secs = configured_timeout_secs();
    let mut cage = Cage::boot(SHELL, &[0]);
    let display_end = 5.0;
    cage.lifecycle_tx
        .send(TalkLifecycleSignal::DisplayEndAt(display_end))
        .expect("受信端は結線資源が保持している");

    cage.surface(0, BOX_SURFACE);
    cage.text(0, "あい");
    let mut events = cage.frame();
    events.extend(cage.frame());
    assert_eq!(
        cage.shown_boxes(0),
        vec!["talk".to_owned()],
        "前提: 箱に文字が出ている"
    );
    assert_eq!(cage.window_visible(0), Some(false), "前提: 窓は出ていない");
    assert_eq!(
        events
            .iter()
            .filter(|e| e.message().contains("タイムアウト計測を開始"))
            .count(),
        1,
        "箱だけに文字が出ているスコープでも計測が始まる: {events:?}"
    );
    assert_eq!(balloon_reports(&cage.reports), vec![vec![(0, 0)]]);

    cage.world
        .insert_resource(FrameTime(display_end + timeout_secs + 1.0));
    let events = cage.frame();
    let marked: Vec<&str> = events
        .iter()
        .filter(|e| e.message().contains(HIDE_BOXES_MESSAGE))
        .map(|e| e.field("actor").unwrap_or_default())
        .collect();
    assert_eq!(marked, vec!["0"], "時間切れで箱を隠す印が立つ: {events:?}");
    assert!(cage.shown_boxes(0).is_empty(), "箱の文字が消える");
    assert_eq!(
        balloon_reports(&cage.reports),
        vec![Vec::new()],
        "消えたフレームで空の組が届く"
    );

    cage.frame();
    assert!(
        cage.shown_boxes(0).is_empty(),
        "印は次の台詞の頭まで立ったまま（次の提示でも面にならない）"
    );
}

// 絵と箱の置き場所の揃え（areka-P0-shell-balloon-frame-align）。本ファイルの `Cage` をそのまま
// 使うため子に置く。
#[path = "frame_shell_box_align_tests.rs"]
mod align_tests;
