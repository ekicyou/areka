//! シェルの切替が新しいシェルの箱の束を文字の層へ渡す決定論テスト（areka-P0-shell-balloon 要件 6.8・
//! descript-window-keys 要件 3.2・3.5）。
//!
//! 土台は親（`switch_finish_tests.rs`）の頼んだ段の組み立てをそのまま使う。親が 1,000 行の上限を
//! 超えたので兄弟のファイルへ出した（spec: runtime-crate-split 設計 D9）。

use super::*;

// ---------------------------------------------------------------- 箱の束（areka-P0-shell-balloon）

/// 前のシェル: 面 10 に箱 `old`。
const OLD_SHELL: &str = "\
balloon.old
{
size,100,50
}
surface10
{
element1,balloon,old,0,0
}
";

/// 新しいシェル: 同じ面 10 に箱 `new`（名前も位置も違う）。
const NEW_SHELL: &str = "\
balloon.new
{
size,80,40
}
surface10
{
element1,balloon,new,5,5
}
";

/// テストの中の surfaces.txt の文面から箱の束を組む（本番と同じく別名の写しと面の表の番号を採る）。
fn box_bundle(text: &str) -> ShellBoxAssets {
    let world = areka_emo_compose::EmoWorld::build(&areka_parsers::shell::parse(text));
    let (layout, report) = areka_emo_compose::fold_boxes(
        &areka_parsers::shell::parse_boxes(text),
        &BTreeMap::new(),
        &world,
    );
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    ShellBoxAssets {
        layout,
        aliases: world.alias_snapshot(),
        surface_ids: world.surface_ids().collect(),
        font_dirs: Vec::new(),
        fallback_candidates: std::collections::BTreeMap::new(),
        markers: Default::default(),
    }
}

fn cue(actor: &ActorKey, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: actor.clone(),
        command,
        duration: 0.0,
    }
}

/// スコープの文字を持つ場所（場所の名前・文字の数）。普通のバルーンは `balloon`。
fn texts(wiring: &RuntimeWiring, actor: &ActorKey) -> Vec<(String, usize)> {
    let runtime = wiring.runtime().borrow();
    runtime
        .state()
        .places()
        .filter(|(key, state)| key.actor == *actor && !state.items().is_empty())
        .map(|(key, state)| {
            let name = match &key.place {
                TextPlace::Balloon => "balloon".to_owned(),
                TextPlace::Box(name) => name.as_str().to_owned(),
            };
            (name, state.items().len())
        })
        .collect()
}

/// シェルの切替は新しいシェルの箱の束を文字の層へ渡す（要件 6.8）: 前のシェルの箱の文字は
/// 残らず、`\s` を書かない次の台詞は新しいシェルの同じ番号のサーフェス（面 10）の箱へ入る。
#[test]
fn shell_switch_hands_the_new_box_bundle_to_the_text_layer() {
    let SwapFinish::Shell {
        source,
        balloon,
        restored,
        ..
    } = shell_finish()
    else {
        unreachable!("shell_finish はシェルの残り")
    };
    let finish = SwapFinish::Shell {
        source,
        balloon,
        restored,
        boxes: box_bundle(NEW_SHELL),
        cursors: ShellCursorAssets::default(),
    };
    let mut rig = rig_on(World::new(), SkinKind::Shell, false, finish);
    // 起動の装着の相が渡した前のシェルの束と、面 10 の箱 `old` に出ている文字。
    box_bundle(OLD_SHELL).hand_to(&mut rig.wiring.runtime().borrow_mut(), &mut rig.world);
    let actor = ActorKey::from("0");
    {
        let mut runtime = rig.wiring.runtime().borrow_mut();
        runtime.apply_cue(&cue(&actor, CueCommand::Emote { key: "10".into() }));
        runtime.apply_cue(&cue(&actor, CueCommand::Text("古い".into())));
    }
    let before = texts(&rig.wiring, &actor);

    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    frame(&mut rig);
    let after_switch = texts(&rig.wiring, &actor);
    rig.wiring
        .runtime()
        .borrow_mut()
        .apply_cue(&cue(&actor, CueCommand::Text("新しい".into())));
    let next_talk = texts(&rig.wiring, &actor);

    assert_eq!(
        (before, after_switch, next_talk, stage(&rig.world)),
        (
            vec![("old".to_owned(), 2)],
            Vec::new(),
            vec![("new".to_owned(), 3)],
            None,
        ),
        r"(切替の前, 切替の後, `\s` を書かない次の台詞, 印)"
    );
}

/// 前のシェル: 面 10 と面 20（どちらも箱なし）。
const OLD_SHELL_WITH_20: &str = "surface10
{
}
surface20
{
}
";

/// シェルの切替で今の面（20）が新しいシェルに無いスコープは、文字の層も箱の束の候補の並びの
/// 最初に在る面（5 は無いので 10）へ替わり、`\s` を書かない次の台詞はその面の箱 `new` へ入る
/// （spec: descript-window-keys 要件 3.2・3.5）。seriko と同じ材料（面の番号の集合と候補の並び）で決める。
///
/// # 非空虚性
/// `hand_to` が戻り先を呼ばないと、今の面が 20 のままで次の台詞は普通のバルーンへ入って赤。
#[test]
fn shell_switch_falls_the_text_layer_back_to_the_bundle_candidates() {
    let SwapFinish::Shell {
        source,
        balloon,
        restored,
        ..
    } = shell_finish()
    else {
        unreachable!("shell_finish はシェルの残り")
    };
    let actor = ActorKey::from("0");
    let mut boxes = box_bundle(NEW_SHELL);
    boxes.fallback_candidates = BTreeMap::from([(actor.clone(), vec![5, 10])]);
    let finish = SwapFinish::Shell {
        source,
        balloon,
        restored,
        boxes,
        cursors: ShellCursorAssets::default(),
    };
    let mut rig = rig_on(World::new(), SkinKind::Shell, false, finish);
    box_bundle(OLD_SHELL_WITH_20).hand_to(&mut rig.wiring.runtime().borrow_mut(), &mut rig.world);
    rig.wiring
        .runtime()
        .borrow_mut()
        .apply_cue(&cue(&actor, CueCommand::Emote { key: "20".into() }));
    let before = rig
        .wiring
        .runtime()
        .borrow()
        .state()
        .current_surface(&actor);

    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    frame(&mut rig);
    let after = rig
        .wiring
        .runtime()
        .borrow()
        .state()
        .current_surface(&actor);
    rig.wiring
        .runtime()
        .borrow_mut()
        .apply_cue(&cue(&actor, CueCommand::Text("新しい".into())));

    assert_eq!(
        (before, after, texts(&rig.wiring, &actor), stage(&rig.world)),
        (Some(20), Some(10), vec![("new".to_owned(), 3)], None),
        r"(切替の前の面, 切替の後の面, `\s` を書かない次の台詞, 印)"
    );
}
