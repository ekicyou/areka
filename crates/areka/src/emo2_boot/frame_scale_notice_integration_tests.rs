// =============================================================================
// 倍率の変わりの相の同じフレームの檻（shell-scale-linkage task 6.2・要件 4.1・4.2・5.1・5.5・5.7・7.3）
//
// 親の檻（実 emo2 fixture ＋ 実 GPU・本番の `emo2_frame_system`）で、窓の DPI の替わりから
// 表の書き込みとイベントの送出までが**同じ 1 回**の中で起きることを見る。相を後ろへ動かすと
// （次のフレームへ遅らせると）この檻が落ちる。
//
// 場面:
//   ⑴ 窓の DPI を 144 → 192 に替えて 1 回回すと、表が `200,200` になり `OnShellScaling` が 1 件届く。
//   ⑵ 装着の最初のフレーム（初めて決まった倍率）は知らせ 0 件。
//   ⑶ 作者 DPI の違うシェルの差し替えの巡は知らせ 0 件（起点を引き継がない）。
//   ⑷ 起動の途中（定常の前）は継ぎ目でなく本物の kanade へ届き、`raise_event_not_steady` で捨てられ、
//      表は書かれている。
//
// 送り先は ⑴〜⑶ が継ぎ目（`ScaleEventSeam`）、⑷ が置き場のゴーストの本物の kanade。
// =============================================================================

use std::time::Duration;

use areka_emo_compose::{BindSet, PatternState};
use areka_kanade::{KanadeConfig, KanadeMsg};

use crate::emo2_boot::target_map::{TargetWindow, shell_target};
use crate::ghost_session::ghost_session_scope_scale::scaling_text;

use super::super::scale_notice::ScaleEventSeam;
use super::*;

/// 檻: GPU の World・emo2 の結線・表示の指令の送出端。
struct Cage {
    world: World,
    wiring: Option<Emo2Wiring>,
    present_tx: mpsc::Sender<PresentCommand>,
    _lifecycle_tx: mpsc::Sender<TalkLifecycleSignal>,
    gw: crate::placement::spawn::GhostWindows,
    /// GPU の装置の許可。欄は宣言の順に落ちるので、最後に置いて装置の世界より後に返す。
    _gpu: crate::emo2_boot::spine::GpuPermit,
}

impl Cage {
    /// 全窓の DPI を `dpi` にした檻（まだ 1 フレームも回さない）。
    fn new(dpi: u16) -> Self {
        let (gpu, mut world, gw) = gpu_frame_world();
        for scope in gw.scopes().collect::<Vec<_>>() {
            for e in [
                gw.char_window(scope).expect("char 窓がある"),
                gw.balloon_window(scope).expect("balloon 窓がある"),
            ] {
                world.entity_mut(e).insert(DPI::from_dpi(dpi, dpi));
            }
        }
        let (present_tx, present_rx) = mpsc::channel::<PresentCommand>();
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
        let wiring = boot_wiring(present_rx, lifecycle_rx);
        Self {
            world,
            wiring: Some(wiring),
            present_tx,
            _lifecycle_tx: lifecycle_tx,
            gw,
            _gpu: gpu,
        }
    }

    /// 継ぎ目を入れ、その受信端を返す。
    fn seam(&mut self) -> mpsc::Receiver<KanadeMsg> {
        let (tx, rx) = mpsc::channel();
        self.world.insert_resource(ScaleEventSeam(tx));
        rx
    }

    /// 本番の相順を 1 フレーム回す。
    fn frame(&mut self) {
        let wiring = self.wiring.take().expect("結線は檻が持つ");
        self.wiring = Some(advance_frame(&mut self.world, wiring));
    }

    /// scope 0 のシェルへ面 0 の表示を送る（次のフレームの drain が適用する）。
    fn show_shell(&self) {
        self.present_tx
            .send(PresentCommand::ShowSurface {
                target: shell_target(0),
                surface_id: 0,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: None,
            })
            .expect("受信端は結線資源が保持している");
    }

    /// scope 0 のシェルの窓の DPI を替える。
    fn set_shell_dpi(&mut self, dpi: u16) {
        let window = self.gw.char_window(0).expect("char 窓がある");
        self.world
            .entity_mut(window)
            .insert(DPI::from_dpi(dpi, dpi));
    }

    /// 表の scope 0 のシェルの `scaling` の文字（行が無ければ `None`）。
    fn shell_scaling(&self) -> Option<String> {
        let table = self.wiring.as_ref().expect("結線").scope_scale_table();
        let table = table.read().expect("表の錠");
        table
            .get(0, TargetWindow::Shell)
            .map(|v| scaling_text(v.ratio))
    }

    /// 装着し、scope 0 のシェルを表示した（倍率が初めて決まった）ところまで回す。
    fn boot_and_show_shell(&mut self) {
        self.frame();
        self.show_shell();
        self.frame();
    }
}

/// 受信端に溜まったイベントを（名前・Reference）の列で取り出す（GET の返信なし以外は失敗）。
fn notices(rx: &mpsc::Receiver<KanadeMsg>) -> Vec<(String, Vec<String>)> {
    rx.try_iter()
        .map(|msg| match msg {
            KanadeMsg::RaiseEvent {
                id,
                references,
                method: areka_kanade::ShioriMethod::Get,
                reply: None,
            } => (id, references),
            _ => panic!("GET の返信なしの RaiseEvent 以外が届いた"),
        })
        .collect()
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// `event` が `name` の記録の数。
fn count_event(events: &[LogEvent], name: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .count()
}

/// ⑴ 窓の DPI を 144 → 192 に替えて 1 回回すと、その 1 回の中で表が `200,200` になり、
/// `OnShellScaling` が 1 件届く（要件 4.1・5.1・5.5・7.3）。
#[test]
fn dpi_change_writes_the_table_and_raises_one_notice_within_the_same_frame() {
    let mut cage = Cage::new(144);
    let rx = cage.seam();
    cage.boot_and_show_shell();
    assert_eq!(
        cage.shell_scaling().as_deref(),
        Some("150,150"),
        "前提: 144 の画面で倍率が初めて決まっている"
    );
    assert_eq!(notices(&rx), vec![], "前提: 初めて決まった倍率は知らせない");

    cage.set_shell_dpi(192);
    cage.frame();
    assert_eq!(
        cage.shell_scaling().as_deref(),
        Some("200,200"),
        "DPI を替えた 1 回の中で表が新しい値になる"
    );
    assert_eq!(
        notices(&rx),
        vec![(
            "OnShellScaling".to_string(),
            strings(&["200", "150", "200", "150", "0"])
        )],
        "同じ 1 回の中で知らせがちょうど 1 件届く"
    );
}

/// ⑵ 装着の最初のフレームは、倍率が初めて決まるだけなので知らせ 0 件（要件 5.7）。
#[test]
fn attach_frame_raises_no_notice() {
    let mut cage = Cage::new(144);
    let rx = cage.seam();
    cage.frame();
    assert_eq!(notices(&rx), vec![], "装着の最初のフレームは知らせない");
    cage.show_shell();
    cage.frame();
    assert_eq!(
        cage.shell_scaling().as_deref(),
        Some("150,150"),
        "シェルの最初の表示で表は書かれる"
    );
    assert_eq!(notices(&rx), vec![], "シェルの最初の表示も知らせない");
}

/// ⑶ 作者 DPI の違うシェルへの差し替えの巡は、起点を引き継がないので知らせ 0 件（要件 5.7）。
#[test]
fn shell_replacement_with_another_author_dpi_raises_no_notice() {
    let mut cage = Cage::new(144);
    let rx = cage.seam();
    cage.boot_and_show_shell();
    assert_eq!(notices(&rx), vec![], "前提: ここまで知らせは無い");

    let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");
    let shell = assets
        .shells
        .into_iter()
        .find(|s| s.scope == 0)
        .expect("scope 0 のシェル資産");
    cage.present_tx
        .send(PresentCommand::ReplaceTarget {
            target: shell_target(0),
            emo_world: Box::new(shell.emo_world),
            atlas: shell.atlas,
            author_dpi: 192,
            show: Some((0, BindSet::default())),
            reply: None,
        })
        .expect("受信端は結線資源が保持している");
    cage.frame();
    assert_eq!(
        cage.shell_scaling().as_deref(),
        Some("75,75"),
        "差し替えの巡で表は新しい作者 DPI の倍率になる（144 ÷ 192）"
    );
    assert_eq!(notices(&rx), vec![], "差し替えの巡は知らせない");
}

/// ⑷ 起動の途中（kanade が定常の前）の倍率の変わりは、継ぎ目が無ければ置き場のゴーストの本物の
/// kanade へ届き、`raise_event_not_steady` で捨てられる。表は書かれている（設計 Flow 3）。
#[test]
fn notice_before_steady_reaches_the_real_kanade_and_is_dropped_but_the_table_is_written() {
    let mut cage = Cage::new(144);
    let ((), events) = capture_logs(|| {
        // 捕捉の窓の中で起こすと、kanade のスレッドの記録も同じ窓に入る。
        let (kanade, _handle) = areka_kanade::spawn_kanade(
            KanadeConfig::new("", ""),
            mpsc::channel().0,
            mpsc::channel().0,
            Box::new(|_, _| {}),
        );
        cage.world
            .insert_non_send(crate::ghost_session::GhostSlot(Some(
                crate::ghost_session::GhostSession::for_test(
                    Some(kanade.clone()),
                    std::path::PathBuf::from("ghost/test"),
                ),
            )));
        cage.boot_and_show_shell();
        cage.set_shell_dpi(192);
        cage.frame();
        // 受け箱は順に処理するので、この問い合わせの返事が来たら上のイベントは処理済み。
        let (reply, answered) = areka_actor::reply_channel();
        kanade
            .send(KanadeMsg::ResourceQuery { ids: vec![], reply })
            .expect("kanade は生きている");
        answered
            .recv_timeout(Duration::from_secs(30))
            .expect("kanade が返事をする");
    });
    assert_eq!(
        cage.shell_scaling().as_deref(),
        Some("200,200"),
        "送ったイベントが捨てられても表は書かれている"
    );
    assert_eq!(
        count_event(&events, "scale_event_raised"),
        1,
        "相は本物の kanade へ 1 件送った: {events:?}"
    );
    assert_eq!(
        count_event(&events, "raise_event_not_steady"),
        1,
        "kanade は定常の前なので捨てた: {events:?}"
    );
    assert_eq!(count_event(&events, "scale_event_not_sent"), 0);
}

/// ⑸ 継ぎ目も置き場のゴーストも無いときは送れず、表は書いたままイベントごとに警告を 1 件残す
/// （設計 Error Handling・`scale_event_not_sent`）。
#[test]
fn notice_without_kanade_warns_once_and_keeps_the_table_written() {
    let mut cage = Cage::new(144);
    cage.world
        .insert_non_send(crate::ghost_session::GhostSlot(None));
    cage.boot_and_show_shell();
    cage.set_shell_dpi(192);
    let ((), events) = capture_logs(|| cage.frame());
    assert_eq!(
        cage.shell_scaling().as_deref(),
        Some("200,200"),
        "送れなくても表は書かれている"
    );
    assert_eq!(
        count_event(&events, "scale_event_not_sent"),
        1,
        "1 件のイベントに警告が 1 件: {events:?}"
    );
    assert!(
        events
            .iter()
            .filter(|e| e.field_str("event") == Some("scale_event_not_sent"))
            .all(|e| e.level == tracing::Level::WARN),
        "送れない記録は警告の段"
    );
    assert_eq!(count_event(&events, "scale_event_raised"), 0);
}
