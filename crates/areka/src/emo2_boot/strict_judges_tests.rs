//! `ArekaStrictJudges` の答えが `check_script` と同じ物差しの写しであること（spec: mcp-strict-errors 要件 2.7）。

use areka_sakura::strict::{DetectionPoint, StrictJudges};

use super::{ArekaStrictJudges, production};

/// `\!` は正準の台帳の答え（登記の在る組だけ拾われる）をそのまま返す。
#[test]
fn command_handled_follows_the_canonical_ledger() {
    let judges = ArekaStrictJudges::canonical();
    let rows: [(&str, Option<&str>, bool); 5] = [
        ("move", None, true),
        ("set", Some("zorder"), true),
        ("open", Some("readme"), true),
        ("open", Some("nosuchthing"), false),
        ("nosuchcommand", None, false),
    ];
    for (name, first, want) in rows {
        assert_eq!(
            judges.command_handled(name, first),
            want,
            "({name}, {first:?}) は拾われるか"
        );
    }
}

/// `\f` は文字の層の分類を検出点へ写し、診ないものは None のまま。
#[test]
fn font_maps_the_text_layer_class_to_detection_points() {
    let judges = ArekaStrictJudges::canonical();
    let rows: [(&[&str], Option<DetectionPoint>); 4] = [
        (&["sub", "1"], Some(DetectionPoint::FontIgnored)),
        (&["colour", "red"], Some(DetectionPoint::FontUnknownKey)),
        (&[], Some(DetectionPoint::FontUnknownKey)),
        (&["bold", "1"], None),
    ];
    for (args, want) in rows {
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(judges.font(&args), want, "{args:?} の検出点");
    }
}

/// 本番の構築へ入れる判定は在る（無いと strict の記録が無言で 0 件になる）。
#[test]
fn production_judges_are_present() {
    let judges = production().expect("本番の判定");
    assert!(!judges.command_handled("nosuchcommand", None));
}

// ---- 文書を読む通しの檻（spec: mcp-strict-errors task 9.2・要件 2.2・2.4・2.7・4.5・7.1・7.2・7.6） ----
//
// 実の走査（`parse_noted` → `compile_strict`）・覆い（`StrictTap`）・事実を入れた表情の受け口
// （`SerikoSink`）・areka の判定・文字の層の受け手（`EmoTextSink`）を、`talk_handler` でテストの
// スレッドの上に回す。文字の層の知らせは、名前つきの箱を持つ試しのサーフェスを載せた状態へ、
// 受け手が送った知らせをこのスレッドの待ち行列で捌かせる（UI のスレッドも窓も使わない）。
// 記録は捕まえた出来事をログの履歴の下書き（`log_history::draft`）に通して読む。

mod doc_tables {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::mpsc;

    use areka_emo_compose::{EmoWorld, fold_boxes};
    use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
    use areka_emo_text::state::TextLayerConfig;
    use areka_parsers::sakura::parse_noted;
    use areka_parsers::shell::{parse, parse_boxes};
    use areka_sakura::strict::{DetectionPoint, RECORD_TARGET, StrictHooks};
    use areka_sakura::{
        ChoiceWaiting, SakuraMsg, StartTalk, SystemVarSnapshot, TalkDone, TalkId, talk_handler,
    };
    use areka_seriko::{
        BindResolver, MockSurfaceOutput, SerikoLoopConfig, SkinFacts, SurfaceResolver, spawn_seriko,
    };
    use bevy_ecs::world::World;
    use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
    use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
    use wintf::executor::{FilterResult, MessageLoop};

    use crate::emo2_boot::consumer_ledger::ConsumerLedger;
    use crate::emo2_boot::shell_box_assets::ShellBoxAssets;
    use crate::emo2_boot::strict_judges::ArekaStrictJudges;
    use crate::log_history::{FieldText, Kind, TARGET_ERROR, draft};
    use crate::mcp::check_script_judge::diagnose;

    const DOC: &str = include_str!("../../../../doc/ssp-mcp/areka-tools.md");
    const GHOST: &str = "窓際のぱすたさん";

    /// サーフェス 0 は箱 `a` を持つ。
    const SHELL: &str = "\
balloon.a
{
size,100,50
}
surface0
{
element1,balloon,a,0,0
}
";
    /// 今のサーフェスに在る箱（名前の形の `\b` の「出ない台本」）。
    const PRESENT_BOX: &str = r"\b[a]";
    /// 今のシェルの別名の表に在る名前（7 行目の「出ない台本」）。
    const PRESENT_ALIAS: &str = r"\s[笑顔]";

    /// スコープ 0 の絵は 0・3（別名 `笑顔` は 3）、バルーンは 0。
    fn aliases() -> BTreeMap<String, Vec<u32>> {
        BTreeMap::from([("笑顔".to_string(), vec![3])])
    }
    fn surface_ids() -> BTreeSet<u32> {
        BTreeSet::from([0, 3])
    }
    fn facts() -> SkinFacts {
        SkinFacts::new(
            aliases(),
            BTreeMap::from([(0, surface_ids())]),
            BTreeMap::from([(0, BTreeSet::from([0]))]),
        )
    }

    /// 印の対の間の表の行（見出しと区切りの 2 行を除く）を欄に割る。
    fn table(marker: &str) -> Vec<Vec<String>> {
        let begin = format!("<!-- {marker}:begin -->");
        let end = format!("<!-- {marker}:end -->");
        let (_, rest) = DOC.split_once(&begin).expect("始めの印");
        let (body, _) = rest.split_once(&end).expect("終わりの印");
        body.lines()
            .map(str::trim)
            .filter(|l| l.starts_with('|'))
            .skip(2)
            .map(|l| {
                l.trim_matches('|')
                    .split('|')
                    .map(|c| c.trim().to_string())
                    .collect()
            })
            .collect()
    }

    /// 欄の中の `…` の綴りを全部。
    fn spans(cell: &str) -> Vec<String> {
        cell.split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect()
    }

    fn unquote(cell: &str) -> String {
        cell.trim_matches('`').to_string()
    }

    /// 記録 1 件（下書きの本文を割った形）。
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Rec {
        kind: String,
        start: usize,
        end: usize,
        message: String,
        text: String,
        point: String,
    }

    /// 本文 `<kind> <start>-<end> <message>: <text> point="<点>"` を割る。
    fn split_body(body: &str) -> Rec {
        let bad = || -> ! { panic!("記録の本文の形でない: {body}") };
        let (kind, rest) = body.split_once(' ').unwrap_or_else(|| bad());
        let (range, rest) = rest.split_once(' ').unwrap_or_else(|| bad());
        let (start, end) = range.split_once('-').unwrap_or_else(|| bad());
        let (message, rest) = rest.split_once(": ").unwrap_or_else(|| bad());
        let (text, point) = rest.rsplit_once(" point=\"").unwrap_or_else(|| bad());
        Rec {
            kind: kind.into(),
            start: start.parse().unwrap_or_else(|_| bad()),
            end: end.parse().unwrap_or_else(|_| bad()),
            message: message.into(),
            text: text.into(),
            point: point.strip_suffix('"').unwrap_or_else(|| bad()).into(),
        }
    }

    /// 終わりと選択待ちの知らせの受け皿（中身は見ない）。
    struct Notice;
    impl From<TalkDone> for Notice {
        fn from(_: TalkDone) -> Self {
            Notice
        }
    }
    impl From<ChoiceWaiting> for Notice {
        fn from(_: ChoiceWaiting) -> Self {
            Notice
        }
    }

    /// このスレッドの待ち行列に積まれた伝言を全部汲んでから抜ける（`spine.rs` の `pump_until_idle` と同じ定石）。
    fn pump_until_idle() {
        // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ。
        unsafe { PostQuitMessage(0) };
        MessageLoop::run(|_, _| FilterResult::Forward);
    }

    /// 台本を strict で最後まで（選択肢で終わるなら選択待ちまで）再生し、残った strict の記録を順に返す。
    fn play(script: &str) -> Vec<Rec> {
        let ((), events) = log_capture_kit::capture(|| {
            // 文字の層: 名前つきの箱を持つサーフェス 0 を今のサーフェスにした状態。
            let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
                TextLayerConfig::default(),
            )));
            let world = EmoWorld::build(&parse(SHELL));
            let (layout, _) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
            ShellBoxAssets {
                layout,
                aliases: aliases(),
                surface_ids: surface_ids(),
                ..ShellBoxAssets::default()
            }
            .hand_to(&mut runtime.borrow_mut(), &mut World::new());
            runtime.borrow_mut().apply_cue(&TalkCue {
                at: 0.0,
                actor: ActorKey::from("0"),
                command: CueCommand::Emote { key: "0".into() },
                duration: 0.0,
            });
            let (text, _drain) =
                spawn_emo_text(Rc::clone(&runtime)).expect("このスレッドで文字の線を起こせる");
            // 表情の受け口: 今のシェルとバルーンの事実を入れる。
            let (surface, _seriko) = spawn_seriko(
                SurfaceResolver::new(aliases()),
                areka_seriko::DefaultBinds::uniform(areka_emo_compose::BindSet::default()),
                BindResolver::empty(),
                SerikoLoopConfig::disabled(),
                MockSurfaceOutput::new(),
            );
            surface.set_facts(facts());

            let sinks: Vec<Box<dyn CueSink + Send>> = vec![Box::new(surface), Box::new(text)];
            let (tx, _rx) = mpsc::channel::<Notice>();
            let hooks = StrictHooks {
                ghost: GHOST.into(),
                judges: Arc::new(ArekaStrictJudges::canonical()),
            };
            let mut handle = talk_handler(sinks, SystemVarSnapshot::default(), tx, Some(hooks));
            let start = StartTalk {
                strict: true,
                ..StartTalk::new(TalkId(1), script)
            };
            let mut flow = handle(SakuraMsg::Start(start));
            let mut t = 0.0;
            while flow.is_continue() && t < 1e7 {
                flow = handle(SakuraMsg::Tick(t));
                t += 1e6;
            }
            // 受け手を落としてトークの終わりまで積み、文字の層の知らせをこのスレッドで捌く。
            drop(handle);
            pump_until_idle();
        });
        events
            .iter()
            .filter(|ev| ev.target == RECORD_TARGET)
            .map(|ev| {
                let d = draft(
                    ev.level,
                    &ev.target,
                    ev.fields.iter().map(|(name, v)| FieldText {
                        name,
                        debug: &v.debug,
                        raw: v.str_raw.as_deref(),
                    }),
                )
                .expect("記録はエラーログの種別に当たる");
                // 記録の名は束ねに渡したゴースト名（`listed_ghost_name` で作る・areka-ghost の config が固定）。
                assert_eq!((d.kind, d.name.as_str()), (Kind::Error, GHOST), "{script}");
                split_body(&d.body)
            })
            .collect()
    }

    /// 記録の target はエラーログの target（`get_log` の `log_type=error` で読める・2.2）。
    #[test]
    fn record_target_is_the_error_log_target() {
        assert_eq!(RECORD_TARGET, TARGET_ERROR);
    }

    /// ⑶ の 17 行: 「出る台本」で文書の種類と文面の記録が 1 件残り、「出ない台本」で残らない（7.1・2.4）。
    #[test]
    fn diagnostic_rows_replay_as_the_document_says() {
        // （種類, 場合, 文面）。
        let kinds: Vec<(String, String, String)> = table("diagnostic-kinds")
            .into_iter()
            .map(|c| (unquote(&c[0]), c[1].clone(), unquote(&c[2])))
            .collect();
        assert_eq!(kinds.len(), 10, "種類と文面の表の行数");
        // 文面が 2 つ以上ある種類の行が引く「場合」（行の番号 → 種類の表の「場合」の欄）。
        let case_of = |row: usize| match row {
            9 => "閉じていない",
            10 | 11 => "既定の値へ落とす",
            15 => "閉じが無い",
            16 => "開いている間の新しい開き",
            17 => "開いていない所の閉じ",
            _ => "—",
        };
        let rows = table("diagnostic-rows");
        let mut checked = 0;
        for c in &rows {
            let row: usize = c[0].parse().expect("行の番号");
            let kind = unquote(&c[1]);
            let candidates: Vec<&(String, String, String)> =
                kinds.iter().filter(|(k, _, _)| *k == kind).collect();
            let message = match candidates.as_slice() {
                [only] => &only.2,
                many => {
                    &many
                        .iter()
                        .find(|(_, case, _)| case == case_of(row))
                        .unwrap_or_else(|| panic!("{row} 行目の場合が種類の表に無い"))
                        .2
                }
            };
            let red = spans(&c[3]);
            let mut green: Vec<String> = spans(&c[4])
                .into_iter()
                // 名前の形の `\b` は、再生では今のサーフェスに在る箱の名前で確かめる（⑺）。
                .map(|s| {
                    if s == r"\b[名前]" {
                        PRESENT_BOX.into()
                    } else {
                        s
                    }
                })
                .collect();
            if green.is_empty() {
                assert_eq!(c[4], "表に在る名前", "{row} 行目の「出ない台本」");
                green.push(PRESENT_ALIAS.into());
            }
            assert!(!red.is_empty(), "{row} 行目に「出る台本」が無い");
            for script in &red {
                let got = play(script);
                assert_eq!(got.len(), 1, "{row} 行目 {script}: {got:?}");
                assert_eq!(
                    (got[0].kind.as_str(), got[0].message.as_str()),
                    (kind.as_str(), message.as_str()),
                    "{row} 行目 {script}"
                );
            }
            for script in &green {
                assert_eq!(play(script), [], "{row} 行目 {script}");
            }
            checked += 1;
        }
        assert_eq!((checked, rows.len()), (17, 17), "確かめた行の数と ⑶ の行数");
    }

    /// 誤りを複数含む台本で、記録の種類・位置・綴り・文面・順が `check_script` の診断と同じ（7.2・2.7）。
    #[test]
    fn records_match_check_script_diagnostics() {
        let script = r"\s[99999]あ\x\![nosuch]\b[99]\f[sub,1]\_w[abc]い\s[-2]\_a[x]う\_a[y]え\_a\![set,choicetimeout,abc]\f[colour,red]\_a";
        let want: Vec<(String, usize, usize, String, String)> = diagnose(
            script,
            &parse_noted(script),
            &ConsumerLedger::canonical(),
            Some(&facts()),
        )
        .into_iter()
        .map(|d| {
            (
                d.kind.as_str().into(),
                d.start,
                d.end,
                d.text,
                d.message.into(),
            )
        })
        .collect();
        assert!(want.len() >= 8, "誤りを複数含む: {want:?}");
        let got: Vec<(String, usize, usize, String, String)> = play(script)
            .into_iter()
            .map(|r| (r.kind, r.start, r.end, r.text, r.message))
            .collect();
        assert_eq!(got, want);
    }

    /// 検出点の表: 列挙の名前の集まりと「点」の列が同じで数も同じ。各行の赤の台本でその点の記録が
    /// ちょうど 1 件、緑の台本で 0 件（4.5・7.6）。
    #[test]
    fn detection_point_table_matches_the_enumeration() {
        let rows = table("strict-points");
        let doc: BTreeSet<String> = rows.iter().map(|c| c[0].clone()).collect();
        let code: BTreeSet<String> = DetectionPoint::ALL
            .iter()
            .map(|p| p.name().to_string())
            .collect();
        assert_eq!(doc, code, "点の名前の集まり");
        assert_eq!(rows.len(), DetectionPoint::ALL.len(), "行の数");
        for c in &rows {
            let point = &c[0];
            let red = match spans(&c[4]).as_slice() {
                [red] => red.clone(),
                other => panic!("{point} の赤の台本は 1 つ: {other:?}"),
            };
            let green = match spans(&c[5]).as_slice() {
                [green] => green.clone(),
                [] => {
                    assert_eq!(c[5], "今のサーフェスに在る箱の名前", "{point} の緑の台本");
                    PRESENT_BOX.into()
                }
                other => panic!("{point} の緑の台本は 1 つ: {other:?}"),
            };
            let got = play(&red);
            let hits = got.iter().filter(|r| &r.point == point).count();
            assert_eq!(hits, 1, "{point} の赤の台本 {red}: {got:?}");
            assert_eq!(play(&green), [], "{point} の緑の台本 {green}");
        }
    }
}
