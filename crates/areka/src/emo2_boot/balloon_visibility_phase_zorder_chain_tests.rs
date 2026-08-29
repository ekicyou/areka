//! バルーンの再表示は**鎖の計画へ 1 ビットも作用しない**（要件 7.3・design DD-9）。
//!
//! 「再表示」とはここでも**合成層の shown エッジ**を指す（裁定済みの定義）。Windows 上の
//! 窓は出しっぱなしのまま、中身の絵だけを消して描き直す——だから窓の表示状態は動かず、
//! HWND にも owner にも作用しない。
//!
//! # 何が変わったのか（初版の引き金の退役）
//!
//! 初版は「隠れていた間に外から重なりが崩されているかもしれない」と考え、描き直した直後に
//! 是正を促す引き金を置いていた。所有の鎖の下ではその心配そのものが消える——重なりは
//! Windows 側の owner 関係が構造として保っており、絵の消去・再描画はその関係に触れない。
//! ゆえに引き金は退役し、要件 7.3 は**鎖が崩れる経路が存在しない**という構造で満たす。
//!
//! # 何を固定するか
//!
//! 1. **不作用**——再表示を模した入力（表示の発行と中身の絵の消去を交互に繰り返す）を
//!    本番の発行経路へ通しても、公開済みの鎖の計画が 1 ビットも変わらないこと。
//! 2. **観測点が生きていること**——同じ檻の中で、窓の在庫が動いたときには計画が
//!    **確かに変わる**ことを対照として置く。これが無いと、計画を読む口が壊れていても
//!    「変わらなかった」が緑で通る。
//! 3. **撤去の完了**——退役した引き金の名前も、旧受け口の名前も、Z 順の起床の旗も、
//!    配線層の本文から消えていること（字面）。
//!
//! # 記録の捕捉をしない理由
//!
//! ここが主張するのは「受け口の値が動かない」ことであって「記録が出ない」ことではない。
//! 値を直に読むので捕捉窓は要らず、既定の実行器がスレッドローカルの差し替えを拾えない
//! という罠（`zorder_pair_establish_tests.rs:142-152`）にも触れない。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::EmoWorld;
use areka_emo_present::{EmoPresenter, PresentCommand};
use bevy_ecs::prelude::Entity;
use bevy_ecs::world::World;
use std::sync::mpsc::{Receiver, Sender, channel};
use wintf::ecs::window::{ChainPlan, ZOrderChainPlan};

use super::issue_show;
use crate::emo2_boot::frame::run_zorder_drain_phase;
use crate::emo2_boot::target_map::balloon_target;
use crate::emo2_boot::zorder_cue::ZOrderDirective;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;
use crate::placement::zorder_group_ledger::ZOrderGroupLedger;

// ---------------------------------------------------------------------------
// 檻の組み立て
// ---------------------------------------------------------------------------

/// 1 スコープぶんの合成配置（値は散らしただけで判定に関与しない——この相も鎖の相も
/// 窓を 1 mm も動かさない）。
fn placement(scope: usize) -> ScopePlacement {
    let base = 100 * (scope as i32 + 1);
    ScopePlacement {
        scope,
        char_pos: PointPx { x: base, y: base },
        char_size: SizePx { w: 200, h: 300 },
        balloon_pos: PointPx {
            x: base + 220,
            y: base,
        },
        balloon_size: SizePx { w: 180, h: 120 },
        balloon_offset: PointPx { x: 220, y: 0 },
        balloon_limit: false,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 既存の World へスコープ一式を（正本ごと作り直して）載せる。
fn spawn_scopes(world: &mut World, scopes: &[usize]) {
    let placements: Vec<ScopePlacement> = scopes.iter().map(|s| placement(*s)).collect();
    let titles = GhostTitles::from_scope_titles(
        scopes
            .iter()
            .map(|s| (*s, format!("scope-{s}")))
            .collect::<Vec<_>>(),
    );
    spawn_ghost_windows(world, &placements, &titles);
}

/// `\![set,zorder,tokens...]` 相当の指令。
fn set_directive(tokens: &[&str]) -> ZOrderDirective {
    ZOrderDirective::Set {
        tokens: tokens.iter().map(|t| (*t).to_string()).collect(),
    }
}

/// 公開されている鎖の計画（受け口が無ければ `None`）。
fn chain(world: &World) -> Option<ChainPlan> {
    world
        .get_resource::<ZOrderChainPlan>()
        .and_then(|plan| plan.chain.clone())
}

/// 受け口の差分の印。Resource が無いときは `None`。
fn dirty(world: &World) -> Option<bool> {
    world.get_resource::<ZOrderChainPlan>().map(|p| p.dirty)
}

/// 印を倒す（本番では適用系が読んだ時点で倒す欄——ここでは「次に立ったか」を測るための
/// 下ごしらえである）。
fn clear_dirty(world: &mut World) {
    world
        .get_resource_mut::<ZOrderChainPlan>()
        .expect("受け口が無い World で印を倒そうとした")
        .dirty = false;
}

/// 計画の**逐語の写し**（1 ビットの差も文字列の差として出る）。
fn snapshot(world: &World) -> String {
    format!("{:?}", chain(world))
}

/// 鎖の計画を 1 本公開した World と、その台帳・受信端一式を組む。
///
/// 指令の経路をそのまま通す——射影だけを別経路で作ると、本番が通らない道を檻に入れる
/// ことになる（`zorder_drain_projection_tests.rs` と同じ流儀）。
fn world_with_published_chain() -> (
    World,
    ZOrderGroupLedger,
    Sender<ZOrderDirective>,
    Receiver<ZOrderDirective>,
) {
    let (tx, rx) = channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = World::new();
    spawn_scopes(&mut world, &[0, 1]);
    tx.send(set_directive(&["1", "0"]))
        .expect("受信端は同じ関数が保持している");
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    (world, ledger, tx, rx)
}

/// 表示層へ scope の balloon target を headless 装着する（可視状態は `Some(false)` から始まる）。
fn attach_balloon(presenter: &mut EmoPresenter, world: &mut World, scope: u32) -> Entity {
    let window = world.spawn_empty().id();
    presenter
        .attach_target(
            world,
            balloon_target(scope),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");
    window
}

// ---------------------------------------------------------------------------
// ⑴ 不作用——再表示を模した入力で鎖の計画が 1 ビットも変わらない
// ---------------------------------------------------------------------------

/// 中身の絵を消して描き直しても、公開済みの鎖の計画も差分の印も動かない（要件 7.3）。
///
/// 通すのは**本番の発行経路そのもの**（`issue_show` と、非表示の既存漏斗
/// `PresentCommand::Hide`）である。判断を迂回した手書きの入力ではないので、
/// 発行の側に重なりへ作用する経路が生えれば、その瞬間にここが赤くなる。
///
/// # 対照を同じ檻に置く理由
///
/// 「変わらない」だけを主張する檻は、計画を読む口が壊れていても緑で通る。だから最後に
/// **窓の在庫を動かして計画が確かに変わる**ことを見る。この 1 本が、上の 3 巡の不変を
/// 「観測できるはずのものが観測されなかった」に格上げする。
#[test]
fn a_simulated_redisplay_does_not_move_the_chain_plan_by_one_bit() {
    let (mut world, mut ledger, _tx, rx) = world_with_published_chain();

    // 檻の前提——鎖が実際に公開されている（空の計画で「変わらない」を言っても意味が無い）。
    let published = chain(&world).expect("鎖の計画が公開されていない（檻の前提が崩れている）");
    assert_eq!(
        published.members.len(),
        4,
        "2 スコープ 4 窓の鎖が立っていない: {published:?}"
    );
    assert_eq!(
        dirty(&world),
        Some(true),
        "公開の直後は印が立っている（適用系がまだ読んでいない）"
    );

    // 適用系が読んだ後の定常状態から測る。
    clear_dirty(&mut world);
    let before = snapshot(&world);

    let mut presenter = EmoPresenter::new();
    attach_balloon(&mut presenter, &mut world, 0);

    // 再表示を模した入力——「描き直す」と「中身の絵を消す」を交互に 3 巡。
    for round in 0..3 {
        let shown = issue_show(&mut presenter, &mut world, 0);
        assert!(
            !shown,
            "headless の表示層は表示を一度も確立していないので可視化は実らない\
             （この前提が変わったらこの檻の書き方を見直すこと・round={round}）"
        );
        assert_eq!(
            snapshot(&world),
            before,
            "表示の発行が鎖の計画を書き換えた（再表示から重なりへ作用する経路が生えている・round={round}）"
        );
        assert_eq!(
            dirty(&world),
            Some(false),
            "表示の発行が差分の印を立てた（適用系が空振りで起こされる・round={round}）"
        );

        presenter.apply(
            &mut world,
            PresentCommand::Hide {
                target: balloon_target(0),
                reply: None,
            },
        );
        assert_eq!(
            snapshot(&world),
            before,
            "中身の絵の消去が鎖の計画を書き換えた（round={round}）"
        );
        assert_eq!(
            dirty(&world),
            Some(false),
            "中身の絵の消去が差分の印を立てた（round={round}）"
        );
    }

    // 中身の見え方が実際に触られたことの確認（可視は付かないが、装着済みの target で
    // ある＝表示層の照会が答える相手として実在している）。
    assert_eq!(
        presenter.target_visible(balloon_target(0)),
        Some(false),
        "檻が触っていた target が表示層に実在していない（入力が空振りしていた）"
    );

    // 指令の相を回し直しても同じ——中身の見え方は鎖の入力ではない。
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        snapshot(&world),
        before,
        "中身の見え方の変化が指令の相を経由して鎖の計画へ回り込んでいる"
    );
    assert_eq!(
        dirty(&world),
        Some(false),
        "中身の見え方の変化で指令の相が計画を公開し直している"
    );

    // 対照——窓の在庫が動けば計画は確かに変わる（観測点が生きていることの証明）。
    spawn_scopes(&mut world, &[0, 1, 2]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_ne!(
        snapshot(&world),
        before,
        "窓が増えても計画が動かない＝計画を読む口が壊れている（上の不変は恒真だった）"
    );
    assert_eq!(
        dirty(&world),
        Some(true),
        "窓が増えても差分の印が立たない＝印を読む口が壊れている"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 撤去の完了——退役した引き金への参照が配線層から消えている
// ---------------------------------------------------------------------------

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
///
/// **対照は必ずこの側へ当てる**。落とす前の本文に当てると、説明文に綴りがあるだけで
/// 「在る」と読み、コード行を全部消しても緑のまま通る。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 退役した引き金の綴りが、配線層のどこにも（説明文にも）残っていない。
///
/// 名前を説明文に残すと、後から来た人が「まだ在る仕組み」として読む。引き金は要件を
/// 1 つも担わずに退役したので、名前ごと消すのが正しい。旧受け口と Z 順の起床の旗も同じ
/// ——この相は重なりの機構へ 1 バイトも触れない。
#[test]
fn the_retired_reshow_trigger_leaves_no_trace_in_the_visibility_phase() {
    let raw = include_str!("balloon_visibility_phase.rs");
    let code = code_only(raw);

    // 較正——説明文を落とす処理が本文まで落としていない（落としていれば何を消しても緑）。
    assert!(
        code.contains("fn issue_show("),
        "説明文を落とす処理が本文まで落としている（この檻は恒真になっている）"
    );
    assert!(
        code.contains("presenter.show_target(world, target)"),
        "表示の発行の字面が本文に無い（檻の前提が崩れている）"
    );

    for needle in ["note_balloon_shown", "wants_group_follow_on_show"] {
        assert!(
            !raw.contains(needle),
            "退役した引き金の綴り `{needle}` が配線層に残っている（説明文も含めて消すこと）"
        );
    }
    for needle in ["ZOrderGroups", "ZOrderChainPlan", "tick_wake::ZORDER"] {
        assert!(
            !code.contains(needle),
            "重なりの機構の綴り `{needle}` が配線層の本文に在る（この相は重なりへ触れない）"
        );
    }
}
