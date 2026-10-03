//! バルーンの往復の統合テスト（spec: areka-P0-shell-balloon-switch task 11.2・要件 3.1〜3.3・3.5・
//! 11.2・11.3・design「Integration Tests」・Flow 3）。
//!
//! 土台は 11.1 の [`lap_rig`]（偽の SHIORI のゴースト A・窓の一式・GPU 資源・本番の `Input`・`Update` の段）。
//! 根の `balloon/` に既定の `emo2-kakukaku` と並べて、検体 `StayseeBalloon` の複製を 2 つ目の
//! バルーンとして置く（絵の大きさが違うので、文字の層の結び先の面の大きさで新旧を見分けられる）。

use std::path::Path;

use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::SampleRoot;
use wintf::ecs::Arrangement;

use super::GhostSlot;
use super::shell_balloon_switch_session_lap_tests::{
    CREEP, LookLog, abs, calls_a, done_marks, got, idle, kinds, lap_rig, look_of, refs_of,
};
use crate::boot_config::BootContext;
use crate::boot_resolve::read_last_balloon;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::ghost_switch_test_support::{BALLOON, SwitchRig, standard_script};
use crate::emo2_boot::target_map::balloon_target;

/// 2 つ目のバルーン（検体 `StayseeBalloon` の複製・フォルダ名）。
const STAYSEE: &str = "StayseeBalloon";

/// A の `OnBoot`: 両スコープの面を出し、台詞の終わりで 2 つ目のバルーンへの切替を命じる。
const BOOT_TO_STAYSEE: &str = r"\0\s[0]\1\s[10]\0A\![change,balloon,StayseeBalloon]\e";
/// 1 度目の `OnBalloonChange` の台詞: 新しいバルーンで話し、既定のバルーンへ戻る切替を命じる。
///
/// 1 文字の台詞が切替の後の最初のフレームで現れることは
/// `emo2_boot/balloon_visibility_phase_reappear_tests.rs` が決定論で確かめる。
const CHANGE_1: &str = r"\0新しい\![change,balloon,emo2-kakukaku]\e";
/// 2 度目の `OnBalloonChange` の台詞（戻った既定のバルーンで話す）。
const CHANGE_2: &str = r"\0戻った\e";

/// 検体 `StayseeBalloon` を根の `balloon/StayseeBalloon` へ複製する。
fn add_staysee(rig: &SwitchRig) {
    let staysee = SampleRoot::acquire(STAYSEE).expect("登記済みの検体");
    super::shell_balloon_switch_session_lap_tests::copy_tree(
        staysee.folder(),
        &rig.root.balloon_dir(STAYSEE),
    );
}

/// scope 0 のバルーンの 1 フレームの見え方。
#[derive(Debug, Clone, Copy, PartialEq)]
struct BalloonFrame {
    /// 装着の文字の層のスロット（表示が一度も成立していなければ無し）。
    slot: Option<Entity>,
    /// 装着の面の素の大きさ（スロットと同じく表示の成立の後だけ）。
    size: Option<(u32, u32)>,
    visible: Option<bool>,
    /// 文字の層の今の供給面がそのスロットへ結ばれているか（供給面を装着するとスロットの配置の
    /// 大きさが供給面の大きさになる。装着の直後のスロットの配置は大きさ 0）。
    bound: bool,
}

/// 装着（スロット）ごとの見え方のまとめ。
#[derive(Debug, Clone, Default, PartialEq)]
struct BalloonMount {
    size: Option<(u32, u32)>,
    /// 装着が現れた最初のフレームの表示（最初の装着は起動の途中なので判定に使わない）。
    first_visible: Option<bool>,
    /// 装着が消える直前（最後の装着なら記録の終わり）のフレームの表示。
    last_visible: Option<bool>,
    /// 表示されたフレームで文字の層がこのスロットに結ばれていたことがあるか。
    shown_bound: bool,
}

/// scope 0 のバルーンの装着の移り変わりの記録。
#[derive(Default)]
struct BalloonLog {
    mounts: Vec<(Entity, BalloonMount)>,
}

impl BalloonLog {
    fn record(&mut self, frame: BalloonFrame) {
        let Some(slot) = frame.slot else {
            return;
        };
        if self.mounts.last().is_none_or(|(s, _)| *s != slot) {
            self.mounts.push((
                slot,
                BalloonMount {
                    size: frame.size,
                    first_visible: frame.visible,
                    ..BalloonMount::default()
                },
            ));
        }
        let (_, mount) = self.mounts.last_mut().expect("直前に積んだ");
        mount.last_visible = frame.visible;
        mount.shown_bound |= frame.visible == Some(true) && frame.bound;
    }

    /// 装着の並び（スロットは互いに違うので数だけ見る・最初の装着の `first_visible` は落とす）。
    fn summary(&self) -> Vec<BalloonMount> {
        self.mounts
            .iter()
            .enumerate()
            .map(|(i, (_, m))| BalloonMount {
                first_visible: if i == 0 { None } else { m.first_visible },
                ..m.clone()
            })
            .collect()
    }

    /// 最後の装着は表示されて文字の層が結ばれたか。
    fn last_shown_bound(&self) -> bool {
        self.mounts.last().is_some_and(|(_, m)| m.shown_bound)
    }
}

/// scope 0 のバルーンの今の見え方。
fn balloon_of(rig: &SwitchRig) -> BalloonFrame {
    let world = &rig.world;
    let Some(wiring) = world.get_non_send::<Emo2Wiring>() else {
        return BalloonFrame {
            slot: None,
            size: None,
            visible: None,
            bound: false,
        };
    };
    let presenter = wiring.presenter();
    let view = presenter.text_slot_view(balloon_target(0));
    BalloonFrame {
        slot: view.map(|v| v.slot()),
        size: view.map(|v| v.surface_size()),
        visible: presenter.target_visible(balloon_target(0)),
        bound: view.is_some_and(|v| {
            let supply = wiring
                .runtime()
                .borrow()
                .surface(&ActorKey::from("0"))
                .map(|s| s.size());
            world.get::<Arrangement>(v.slot()).is_some_and(|a| {
                a.size.width > 0.0
                    && supply
                        .is_some_and(|(w, h)| (w as f32, h as f32) == (a.size.width, a.size.height))
            })
        }),
    }
}

/// `last_balloon_recorded` の記録のバルーンの並び（投函の順）。
fn recorded_balloons(events: &[CapturedEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("last_balloon_recorded"))
        .map(|e| e.field_str("balloon").map(str::to_owned))
        .collect()
}

/// 可視性の制御が「本制御が発行していない可視状態の変化」を記録した件数。
fn external_transitions(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.message().contains("本制御が発行していない可視状態の変化"))
        .count()
}

/// 置き場のゴーストの記憶の書き手へ柵を掛けてから、A の `LastBalloon` を実 fs から読む。
fn last_balloon(rig: &SwitchRig) -> Option<String> {
    let fenced = rig
        .world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime())
        .is_some_and(|runtime| runtime.sylphya_publisher().barrier().is_ok());
    assert!(fenced, "記憶の書き手へ柵を掛けられない");
    read_last_balloon(&rig.root.ghost_dir("A"))
}

/// 根の `balloon/<folder>` の `descript.txt` の `name`。
fn balloon_name(rig: &SwitchRig, folder: &str) -> String {
    areka_ghost::catalog::list_balloons(&rig.root)
        .into_iter()
        .find(|e| e.identity.folder == folder)
        .and_then(|e| e.identity.name)
        .unwrap_or_else(|| panic!("バルーン {folder} に name が在る"))
}

/// scope 0 の文字の層に今載っている文字（グリフだけを連ねる）。
pub(super) fn scope0_text(rig: &SwitchRig) -> String {
    rig.world
        .get_non_send::<Emo2Wiring>()
        .and_then(|w| {
            w.runtime()
                .borrow()
                .state()
                .actor_state(&ActorKey::from("0"))
                .map(|s| {
                    s.items()
                        .iter()
                        .filter_map(|i| match i {
                            areka_emo_text::state::TextItem::Glyph { text } => {
                                Some(text.to_string())
                            }
                            _ => None,
                        })
                        .collect()
                })
        })
        .unwrap_or_default()
}

/// バルーンの往復（要件 11.2・3.1〜3.3・3.5・11.3）: A（`emo2-kakukaku`）→ 台本の
/// `\![change,balloon,StayseeBalloon]` → 台詞の終わりで差し替え → `OnBalloonChange`（Ref0＝名前・
/// Ref1＝絶対パス）→ `LastBalloon`＝`StayseeBalloon` → その台詞の `emo2-kakukaku` への切替で逆向きに
/// 1 周。集めて 1 回で判定: ⑴ 呼出列（「切り替え前」のイベントは 0 件・SHIORI を降ろさない）と
/// Reference ⑵ 記憶と今のバルーン ⑶ scope 0 のバルーンの装着ごとに: 古い装着は差し替えの直前まで
/// 見えていた（文字を出し終えて時間切れを待つ）・新しい装着は差し替えの直後は隠れている・次の台詞で
/// 見え、文字の層がその新しいスロットに結ばれる・面の大きさが新しいバルーンのもの・可視性の制御は
/// 古い装着の消滅を「外から隠された」と読まない ⑷ キャラ窓の見え方（装着の子・窓寸・表示）は
/// 1 通りのまま（シェルに触れない・要件 3.5）⑸ 終了の指示なし。
///
/// # 非空虚性
/// 差し替えの後始末で可視の記憶を忘れないと（`forget_scope` を外す）、可視性の制御が古い装着の
/// 消滅を外からの変化として記録して ⑶ の外因の記録が 0 件でなくなり赤。文字の層が新しいスロットへ
/// 結び直されないと（文字の層の追従の相の再追従を外す）、⑶ の `shown_bound` が偽のまま期限切れで赤。
#[test]
fn script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot() {
    let mut lap = lap_rig(|| {
        standard_script(BOOT_TO_STAYSEE)
            .get("OnBalloonChange", Ok(Some(CHANGE_1.to_owned())))
            .get("OnBalloonChange", Ok(Some(CHANGE_2.to_owned())))
    });
    add_staysee(&lap.rig);
    let staysee_name = balloon_name(&lap.rig, STAYSEE);
    let kakukaku_name = balloon_name(&lap.rig, BALLOON);
    let steady = lap.rig.wait_steady();

    let mut looks = LookLog::default();
    let mut balloons = BalloonLog::default();
    let windows = lap.windows.clone();
    // 台詞の時計は 1 ms ずつ進める。バルーンの時間切れ（既定 30 秒）は UI の時刻（`FrameTime` と
    // `TalkClock`＝dola の実時間）で数えるので、古いバルーンが差し替えの直前まで見えているかの余裕は
    // 実時間で 30 秒（この往復は通常 数秒で終わる）。
    let ((round_trip, last), events) = capture(|| {
        let round_trip = lap.frames_until(CREEP, |rig| {
            looks.record(look_of(rig, &windows));
            balloons.record(balloon_of(rig));
            got(rig, "OnBalloonChange", 2) && idle(rig) && balloons.mounts.len() >= 3 && {
                balloons.last_shown_bound() && scope0_text(rig).contains("戻った")
            }
        });
        (round_trip, last_balloon(&lap.rig))
    });

    let calls = calls_a(&lap.rig);
    let boots = lap.rig.calls("A").len();
    let current = lap
        .rig
        .world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.balloon.folder.clone());
    let exit_requested = lap.rig.exit_requested();
    let shutdown_ok = lap.rig.shutdown();
    let path = |folder: &str| abs(&lap.rig.root.balloon_dir(folder));
    let (staysee_size, kakukaku_size) = (
        image_size(&lap.rig.root.balloon_dir(STAYSEE)),
        image_size(&lap.rig.root.balloon_dir(BALLOON)),
    );
    let mount = |size, first_visible| BalloonMount {
        size: Some(size),
        first_visible,
        last_visible: Some(true),
        shown_bound: true,
    };

    assert_eq!(
        (
            (steady, round_trip, boots, kinds(&calls)),
            (
                refs_of(&calls, "OnBalloonChange"),
                refs_of(&calls, "OnShellChanging")
            ),
            (recorded_balloons(&events), last, current),
            done_marks(&events),
            (balloons.summary(), external_transitions(&events)),
            (looks.distinct.len(), looks.blank_after_full),
            (exit_requested, shutdown_ok),
        ),
        (
            (
                true,
                true,
                1,
                vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnBoot".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                    "GET OnBalloonChange".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "GET OnBalloonChange".to_owned(),
                    "GET OnTranslate".to_owned(),
                ]
            ),
            (
                vec![
                    vec![staysee_name, path(STAYSEE)],
                    vec![kakukaku_name, path(BALLOON)],
                ],
                Vec::<Vec<String>>::new(),
            ),
            (
                vec![Some(STAYSEE.to_owned()), Some(BALLOON.to_owned())],
                Some(BALLOON.to_owned()),
                Some(BALLOON.to_owned()),
            ),
            vec![Some("None".to_owned()); 2],
            (
                vec![
                    mount(kakukaku_size, None),
                    mount(staysee_size, Some(false)),
                    mount(kakukaku_size, Some(false)),
                ],
                0,
            ),
            (1, false),
            (false, true),
        ),
        "((定常, 往復が終わった, 起動の回数, 呼出列), (OnBalloonChange, OnShellChanging の Reference), \
         (LastBalloon の投函の並び, 実 fs の LastBalloon, 今のバルーン), 完了の記録の印の台詞の終わり方, \
         (scope 0 のバルーンの装着ごとの (面の大きさ, 現れたときの表示, 消える直前の表示, 文字の層が\
         結ばれて見えた), 可視性の制御の外因の記録の数), (キャラ窓の見え方の数, 子の欠けたフレーム), \
         (終了の指示, 降ろせた))"
    );
}

/// バルーンの面 0（`balloons0.png`）の素の大きさ（PNG の頭の IHDR から読む）。
fn image_size(dir: &Path) -> (u32, u32) {
    let bytes = std::fs::read(dir.join("balloons0.png")).expect("balloons0.png を読む");
    let be = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().expect("4 バイト"));
    (be(16), be(20))
}
