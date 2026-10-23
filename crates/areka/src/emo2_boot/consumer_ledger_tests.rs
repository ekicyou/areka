use areka_parsers::sakura::JUMP_TAG_CARRIER;
use areka_sakura::contract::FONT_TAG_CARRIER;

use super::*;

/// 正準台帳に `move`・`bind` が共に登記されている → 照会で `move`→[`CommandConsumer::MoveSink`]・
/// `bind`→[`CommandConsumer::Seriko`] が引ける（両登録の共存・Observable の中核・task 7.2）。
#[test]
fn canonical_has_move_and_bind_registered() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.consumer_of("move", None),
        Some(CommandConsumer::MoveSink),
        "正準台帳は move（選別子なし）→ MoveSink を登記している（R2.2）"
    );
    assert_eq!(
        ledger.consumer_of("bind", None),
        Some(CommandConsumer::Seriko),
        "正準台帳は bind（選別子なし）→ Seriko を登記している（task 7.2・R2.2）"
    );
}

/// 正準台帳は重なり指定・重なり解除の 2 組を [`CommandConsumer::ZOrderSink`] として登記する
/// （名前＋選別子の粒度＝要件 11.2/11.3・task 3.2）。受け口 `ZOrderCueSink` が自己選別する
/// 組（`("set","zorder")`・`("reset","zorder")`）とちょうど一致していること。
#[test]
fn canonical_registers_both_zorder_pairs() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::ZOrderSink),
        "正準台帳は (set, zorder) → ZOrderSink を登記している（要件 11.2）"
    );
    assert_eq!(
        ledger.consumer_of("reset", Some("zorder")),
        Some(CommandConsumer::ZOrderSink),
        "正準台帳は (reset, zorder) → ZOrderSink を登記している（要件 11.2）"
    );
}

/// 未登記のコマンド名は選別子の有無によらず `None`（自己選別モデルでは未知名は良性未処理・
/// 要件 11.2）。
#[test]
fn unregistered_name_is_none() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.consumer_of("noexist", None),
        None,
        "登記のないコマンド名は None（自己選別モデルの良性未処理・要件 11.2）"
    );
    assert_eq!(
        ledger.consumer_of("noexist", Some("zorder")),
        None,
        "選別子を伴っても、名前ごと未登記なら None（要件 11.2）"
    );
}

/// 名前まるごとの登記（選別子なし）は、第 1 引数が何であってもその名前の全出現を担当する。
/// `MoveCueSink` が名前だけを見て自己選別している実体（`move_cue.rs` の `name != "move"`）と
/// 揃えるための規則。**両側から挟む**: 第 1 引数の無い出現も、有る出現も同じ担当を引く。
#[test]
fn selectorless_registration_owns_every_first_parameter() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.consumer_of("move", None),
        Some(CommandConsumer::MoveSink),
        "第 1 引数の無い move は MoveSink の担当"
    );
    assert_eq!(
        ledger.consumer_of("move", Some("--x=10")),
        Some(CommandConsumer::MoveSink),
        "第 1 引数を伴う move も同じ MoveSink の担当（名前まるごとの登記は第 1 引数を問わない）"
    );
}

/// 選別子つきの登記は、その選別子ちょうどの出現だけを担当する。名簿に無い第 1 引数と、
/// 第 1 引数の無い出現は、どちらも担当なし＝良性未処理（要件 11.2）。
/// **両側から挟む**: 名簿にある `zorder` は引けて、名簿に無い `windowstate` と裸の `set` は
/// 引けない。
#[test]
fn selector_bearing_registration_owns_only_listed_selectors() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::ZOrderSink),
        "名簿にある選別子は引ける"
    );
    assert_eq!(
        ledger.consumer_of("set", None),
        None,
        "第 1 引数の無い裸の set は担当なし（名前まるごとの登記ではない・要件 11.2）"
    );
    assert_eq!(
        ledger.consumer_of("set", Some("windowstate")),
        None,
        "名簿に無い第 1 引数は担当なし＝将来の担当のための余地（要件 11.3）"
    );
    assert_eq!(
        ledger.consumer_of("reset", None),
        None,
        "裸の reset も同様に担当なし（要件 11.2）"
    );
}

/// 一意性檻: 同じ名前と同じ選別子の組を二重登録しようとすると [`LedgerError::Duplicate`]
/// で検出できる（1 出現＝高々 1 担当・要件 11.3）。選別子なしの組でも同じ。
#[test]
fn duplicate_registration_is_detected() {
    let mut ledger = ConsumerLedger::new();
    ledger
        .try_register("move", None, CommandConsumer::MoveSink)
        .expect("初回登記は成功する");
    let err = ledger
        .try_register("move", None, CommandConsumer::Seriko)
        .expect_err("同一の組の二重登録は Err で検出される（1 出現=高々 1 担当・要件 11.3）");
    assert_eq!(
        err,
        LedgerError::Duplicate {
            name: "move".to_string(),
            selector: None,
        },
        "重複は Duplicate{{name, selector}} として観測可能"
    );
    // 二重登録は最初の登記を上書きしない（担当は据え置き）。
    assert_eq!(
        ledger.consumer_of("move", None),
        Some(CommandConsumer::MoveSink)
    );
}

/// **完了条件の中核（要件 11.3）**: 同じコマンド名の**別の第 1 引数**は、後から**別の担当**として
/// 登記できる。重なり指定を本機能が持ったまま、将来 `set` の別のサブコマンドに別の担当が
/// 付けられる余地が型で残っていること。
#[test]
fn different_selector_under_same_name_registers_as_another_consumer() {
    let mut ledger = ConsumerLedger::canonical();
    ledger
        .try_register("set", Some("windowstate"), CommandConsumer::Seriko)
        .expect("同名・別選別子は重複でない（将来の担当の余地・要件 11.3）");

    // 2 つの選別子が別々の担当を引く（相互に混ざらない）。
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::ZOrderSink),
        "先の登記は後から足した選別子に押しのけられない"
    );
    assert_eq!(
        ledger.consumer_of("set", Some("windowstate")),
        Some(CommandConsumer::Seriko),
        "後から足した選別子は自分の担当を引く（要件 11.3 の余地）"
    );
}

/// 上のテストと**対で**置く: 同じコマンド名の**同じ第 1 引数**は後から別担当にできない。
/// 「別の選別子なら足せる」が「何でも足せる」に化けていないことを、隣り合わせで示す。
#[test]
fn same_selector_under_same_name_is_rejected() {
    let mut ledger = ConsumerLedger::canonical();
    let err = ledger
        .try_register("set", Some("zorder"), CommandConsumer::Seriko)
        .expect_err("同名・同選別子の再登記は Err（1 出現=高々 1 担当・要件 11.3）");
    assert_eq!(
        err,
        LedgerError::Duplicate {
            name: "set".to_string(),
            selector: Some("zorder".to_string()),
        },
        "同じ組の再登記は Duplicate として観測可能"
    );
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::ZOrderSink),
        "既登記の担当は据え置き（上書きしない）"
    );
}

/// 相互排他（選別子つき → 選別子なしの順）: `("set","zorder")` が居るところへ
/// 名前まるごとの `("set", なし)` は足せない。足せてしまうと重なり指定の 1 出現に
/// 2 つの担当が作用し「1 出現＝高々 1 担当」（要件 11.3）が破れる。
#[test]
fn selectorless_after_selector_bearing_conflicts() {
    let mut ledger = ConsumerLedger::new();
    ledger
        .try_register("set", Some("zorder"), CommandConsumer::ZOrderSink)
        .expect("初回登記は成功する");
    let err = ledger
        .try_register("set", None, CommandConsumer::Seriko)
        .expect_err("選別子つきが居るところへ名前まるごとの登記は入れない（要件 11.3）");
    assert_eq!(
        err,
        LedgerError::SelectorConflict {
            name: "set".to_string(),
            existing: Some("zorder".to_string()),
            incoming: None,
        },
        "排他違反は SelectorConflict として、名前と両側の選別子を名指しで観測できる"
    );
    // 拒否された登記は表に載らない（既存の担当だけが残る）。
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::ZOrderSink)
    );
    assert_eq!(ledger.consumer_of("set", None), None);
}

/// 相互排他（選別子なし → 選別子つきの順）: 上と**逆の順**でも同じく拒否される。
/// 片方の順序しか押さえない檻は、順序に依存した実装（後勝ちの上書き等）を素通りさせる。
#[test]
fn selector_bearing_after_selectorless_conflicts() {
    let mut ledger = ConsumerLedger::new();
    ledger
        .try_register("set", None, CommandConsumer::Seriko)
        .expect("初回登記は成功する");
    let err = ledger
        .try_register("set", Some("zorder"), CommandConsumer::ZOrderSink)
        .expect_err("名前まるごとが居るところへ選別子つきの登記は入れない（要件 11.3）");
    assert_eq!(
        err,
        LedgerError::SelectorConflict {
            name: "set".to_string(),
            existing: None,
            incoming: Some("zorder".to_string()),
        },
        "逆順でも排他違反は SelectorConflict として観測できる"
    );
    // 拒否された登記は表に載らない。名前まるごとの担当だけが残り、その担当は
    // 第 1 引数によらず引ける。
    assert_eq!(
        ledger.consumer_of("set", None),
        Some(CommandConsumer::Seriko)
    );
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::Seriko),
        "名前まるごとの登記が残っている以上、zorder も同じ担当が引かれる"
    );
}

/// 排他違反の記録は、どの名前のどの選別子どうしが衝突したかを本文で名指しする
/// （黙って諦めない・要件 8.3 の規律）。
#[test]
fn selector_conflict_message_names_the_offending_pair() {
    let err = LedgerError::SelectorConflict {
        name: "set".to_string(),
        existing: Some("zorder".to_string()),
        incoming: None,
    };
    let text = err.to_string();
    assert!(text.contains("set"), "本文にコマンド名が載る: {text}");
    assert!(text.contains("zorder"), "本文に既存の選別子が載る: {text}");
    assert!(
        text.contains("なし"),
        "本文に「選別子なし」の側も載る（片側だけでは衝突が読めない）: {text}"
    );
}

/// 別のコマンド名どうしは互いに干渉しない（排他は同一名の中だけの規則）。
/// 排他の実装が名前をまたいで効いてしまうと、正準台帳がそもそも組めなくなる。
#[test]
fn exclusion_applies_only_within_the_same_name() {
    let mut ledger = ConsumerLedger::new();
    ledger
        .try_register("set", Some("zorder"), CommandConsumer::ZOrderSink)
        .expect("(set, zorder) の登記");
    ledger
        .try_register("move", None, CommandConsumer::MoveSink)
        .expect("別名 move の名前まるごと登記は (set,zorder) と衝突しない");
    ledger
        .try_register("reset", Some("zorder"), CommandConsumer::ZOrderSink)
        .expect("別名 reset の選別子つき登記も衝突しない");
    assert_eq!(
        ledger.consumer_of("move", None),
        Some(CommandConsumer::MoveSink)
    );
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::ZOrderSink)
    );
    assert_eq!(
        ledger.consumer_of("reset", Some("zorder")),
        Some(CommandConsumer::ZOrderSink)
    );
}

/// 正準台帳の構築は重複・排他違反なしで成功する（内部整合＝一意性の検査が緑）。9 件が
/// 共存しても檻は保たれ、既登記の組の再登記は [`LedgerError::Duplicate`] で検出され、
/// 別名の追加は独立に成功する（task 7.2・要件 11.3）。
///
/// **母数も判定する**（タスク 9.4・7.3 の申し送り）——件数を数えずに個々の登記だけを
/// 見ていると、登記が黙って増えても減っても緑のままになる。件数は逐語で固定し、
/// 増減は本檻と本 doc の 2 か所を明示的に編集させる。
#[test]
fn canonical_builds_without_duplicate() {
    // canonical() は内部 try_register（36 行）が Ok（重複なら expect が panic する）。
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.entry_count(),
        36,
        "正準台帳の登記は 36 件（move／bind／(set,zorder)／(reset,zorder)／運搬名 \\f／\
         (open,readme)／(open,file)／(open,browser)／(open,explorer)／(open,editor)／\
         (open,mailer)／運搬名 \\j／(enter,nouserbreakmode)／(leave,nouserbreakmode)／\
         (change,ghost)／(change,shell)／(change,balloon)／(execute,install)／(execute,resetwindowpos)／\
         updatebymyself／update／updateother／(set,choicetimeout)／\
         areka.prop.set／(set,balloontimeout)／(reload,ghost)／(reload,shiori)／(reload,shell)／\
         (reload,balloon)／(reload,descript)／reloadsurface／(set,alignmenttodesktop)／\
         (set,alignmentondesktop)／(set,sticky-window)／(reset,sticky-window)／(execute,resetballoonpos)）——増減させたら本檻と doc の 2 か所を編集すること"
    );
    // 待ち時間の指定は受け口 LifecycleSink に結び付く（areka-P0-balloon-lifecycle-events 要件 9.10）。
    assert_eq!(
        ledger.consumer_of("set", Some("balloontimeout")),
        Some(CommandConsumer::LifecycleSink)
    );
    assert_eq!(
        ledger.consumer_of("move", None),
        Some(CommandConsumer::MoveSink)
    );
    assert_eq!(
        ledger.consumer_of("bind", None),
        Some(CommandConsumer::Seriko)
    );
    assert_eq!(
        ledger.consumer_of("set", Some("zorder")),
        Some(CommandConsumer::ZOrderSink)
    );
    assert_eq!(
        ledger.consumer_of("reset", Some("zorder")),
        Some(CommandConsumer::ZOrderSink)
    );

    // 36 件共存下でも一意性は保たれる: 既登記の組 bind の再登記は Duplicate で
    // 検出される。
    let mut ext = ledger.clone();
    let err = ext
        .try_register("bind", None, CommandConsumer::MoveSink)
        .expect_err("既登記の組 bind の再登記は Err で検出される（要件 11.3）");
    assert_eq!(
        err,
        LedgerError::Duplicate {
            name: "bind".to_string(),
            selector: None,
        },
        "36 件共存下でも重複は Duplicate{{name, selector}} として観測可能"
    );
    // 既登記の担当は据え置き（上書きしない）。
    assert_eq!(ext.consumer_of("bind", None), Some(CommandConsumer::Seriko));

    // 相異なる名前は独立に登記できる（別名は重複でない）。
    ext.try_register("resize", None, CommandConsumer::Seriko)
        .expect("別名 resize の追加は重複でない");
    assert_eq!(
        ext.consumer_of("resize", None),
        Some(CommandConsumer::Seriko)
    );
}

/// task 7.3／要件 2.4: 正準台帳は文字装飾（`\f`）を文字レンダリング層の担当として登記する。
///
/// 登記のキーは運搬名そのもの（[`FONT_TAG_CARRIER`]）——`\f` は第 1 引数（`bold`／
/// `color` …）で担当が分かれないので選別子なしの 1 行である。実行時の選別は
/// `state_decoration.rs` が同じ運搬名で自己選別して行い、本表は宣言だけを持つ。
#[test]
fn canonical_registers_the_font_tag_for_the_text_layer() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.consumer_of(FONT_TAG_CARRIER, None),
        Some(CommandConsumer::TextLayer),
        "正準台帳は '\\f'（選別子なし）→ TextLayer を登記している（要件 2.4）"
    );
    assert_eq!(
        ledger.consumer_of(FONT_TAG_CARRIER, Some("bold")),
        Some(CommandConsumer::TextLayer),
        "選別子なしの登記なので第 1 引数によらず同じ担当（要件 11.3）"
    );
}

/// task 4.2／要件 4.5 と areka-P0-open-external-tags 要件 1.5・9.2・9.3: 正準台帳は
/// `open` の 6 組（`readme`・`file`・`browser`・`explorer`・`editor`・`mailer`）と運搬名 `\j`
/// （選別子なし）を、開く系の受け口（説明書を含む）の担当として登記する。
///
/// `open` は名前＋第 1 引数で登記する——作り付けの窓（`help` など）は将来 別の担当へ割り当て
/// られるよう、名前まるごとでは登記しない。`\j` は第 1 引数が行き先そのもの（URL）なので
/// 選別子なし。受け口 `ReadmeCueSink` が自己選別する組とちょうど一致していること。
#[test]
fn canonical_registers_the_open_family_for_the_readme_sink() {
    let ledger = ConsumerLedger::canonical();
    for selector in ["readme", "file", "browser", "explorer", "editor", "mailer"] {
        assert_eq!(
            ledger.consumer_of("open", Some(selector)),
            Some(CommandConsumer::ReadmeSink),
            "正準台帳は (open, {selector}) → ReadmeSink を登記している（要件 9.2）"
        );
    }
    assert_eq!(
        [
            ledger.consumer_of(JUMP_TAG_CARRIER, None),
            ledger.consumer_of(JUMP_TAG_CARRIER, Some("http://example.com/")),
        ],
        [Some(CommandConsumer::ReadmeSink); 2],
        "運搬名 \\j（選別子なし）→ ReadmeSink・第 1 引数によらず同じ担当（要件 1.5）"
    );
    assert_eq!(
        ledger.consumer_of("open", Some("help")),
        None,
        "作り付けの窓の open,help は担当なし＝将来の担当のための余地（要件 9.3）"
    );
    assert_eq!(
        ledger.consumer_of("open", None),
        None,
        "第 1 引数の無い裸の open は担当なし（名前まるごとの登記ではない・要件 9.3）"
    );
}

/// areka-P0-balloon-break 要件 8.4: 正準台帳は中断の無効化の 2 組を旗の受け口の担当として
/// 登記する。
///
/// 登記のキーは名前＋第 1 引数——`enter`／`leave` は第 1 引数で担当が分かれる名前であり、
/// `onlinemode` など別の第 1 引数は担当なしのまま残る。受け口 `NoUserBreakCueSink` が
/// 自己選別する組とちょうど一致していること。
#[test]
fn canonical_registers_both_no_user_break_pairs() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        ledger.consumer_of("enter", Some("nouserbreakmode")),
        Some(CommandConsumer::UserBreakSink),
        "正準台帳は (enter, nouserbreakmode) → UserBreakSink を登記している（要件 5.1）"
    );
    assert_eq!(
        ledger.consumer_of("leave", Some("nouserbreakmode")),
        Some(CommandConsumer::UserBreakSink),
        "正準台帳は (leave, nouserbreakmode) → UserBreakSink を登記している（要件 5.2）"
    );
    assert_eq!(
        ledger.consumer_of("enter", Some("onlinemode")),
        None,
        "名簿に無い第 1 引数は担当なし（受け口も拾わない・要件 8.2）"
    );
    assert_eq!(
        ledger.consumer_of("enter", None),
        None,
        "第 1 引数の無い裸の enter は担当なし（名前まるごとの登記ではない）"
    );
}

/// areka-P0-ghost-shell-balloon-switch 要件 1.10 と areka-P0-shell-balloon-switch 要件 1.15:
/// 正準台帳は `change` を第 1 引数で 3 組に分けて登記する——`(change, ghost)` はゴーストの切替の
/// 受け口、`(change, shell)`・`(change, balloon)` はシェル・バルーンの切替の受け口。裸の `change`
/// （名前まるごとの登記）は無く、名簿に無い第 1 引数は担当なし。
#[test]
fn canonical_registers_change_ghost_shell_balloon_and_no_bare_change() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        [
            ledger.consumer_of("change", Some("ghost")),
            ledger.consumer_of("change", Some("shell")),
            ledger.consumer_of("change", Some("balloon")),
            ledger.consumer_of("change", None),
            ledger.consumer_of("change", Some("Shell")),
        ],
        [
            Some(CommandConsumer::ChangeSink),
            Some(CommandConsumer::SwitchSink),
            Some(CommandConsumer::SwitchSink),
            None,
            None,
        ],
        "(change,ghost)・(change,shell)・(change,balloon) の 3 組が登記され、裸の change は無い"
    );
}

/// areka-P0-ghost-install 要件 1.5〜1.7: install の受け口は `(execute, install)` の 1 組だけを
/// 担当する。`execute` の第 1 引数のうち表に載るのは `install` と、窓の位置の受け口の
/// `resetwindowpos`（areka-P0-move-canon 要件 8.7）・`resetballoonpos`（areka-P0-window-link-directives
/// 要件 7.1）の 3 つだけで、他の第 1 引数・裸の
/// `execute` は登記しない。
#[test]
fn canonical_install_sink_owns_only_execute_install() {
    let ledger = ConsumerLedger::canonical();
    assert_eq!(
        [
            ledger.consumer_of("execute", Some("install")),
            ledger.consumer_of("execute", Some("resetwindowpos")),
            ledger.consumer_of("execute", Some("resetballoonpos")),
            ledger.consumer_of("execute", Some("http-get")),
            ledger.consumer_of("execute", None),
        ],
        [
            Some(CommandConsumer::InstallSink),
            Some(CommandConsumer::MoveSink),
            Some(CommandConsumer::MoveSink),
            None,
            None
        ],
        "install の受け口は (execute,install) だけ・(execute,resetwindowpos|resetballoonpos) は窓の位置の受け口\
         （execute の他の第 1 引数・裸の execute は担当なし）"
    );
}

/// areka-P0-network-update 要件 1.5〜1.8: 正準台帳は `updatebymyself`・`update`・`updateother` の
/// 3 つを選別子なしで更新の受け口の担当として登記する（第 1 引数は更新対象で担当を分けない）。
#[test]
fn canonical_registers_the_three_update_names_for_the_update_sink() {
    let ledger = ConsumerLedger::canonical();
    for name in ["updatebymyself", "update", "updateother"] {
        assert_eq!(
            [
                ledger.consumer_of(name, None),
                ledger.consumer_of(name, Some("ghost")),
                ledger.consumer_of(name, Some("--shell=S")),
            ],
            [Some(CommandConsumer::UpdateSink); 3],
            "{name}: 選別子なしの登記なので第 1 引数によらず更新の受け口が担当"
        );
    }
    assert_eq!(
        ledger.consumer_of("updatex", None),
        None,
        "似た名前は担当なし"
    );
}

/// areka-P0-mcp-reload 要件 11.1・11.2: 正準台帳は `reload` を第 1 引数で 5 組
/// （`ghost`・`shiori`・`shell`・`balloon`・`descript`）に分け、旧い `reloadsurface` を選別子なしで、
/// どれも読み直しの受け口の担当として登記する。`makoto`・`aigraph`・裸の `reload`・大小違いは
/// 担当なしで、`(reload, makoto)` は後から別の担当として足せる（本 spec の登記が塞がない）。
#[test]
fn canonical_registers_reload_rows_and_leaves_makoto_open() {
    let ledger = ConsumerLedger::canonical();
    for selector in ["ghost", "shiori", "shell", "balloon", "descript"] {
        assert_eq!(
            ledger.consumer_of("reload", Some(selector)),
            Some(CommandConsumer::ReloadSink),
            "(reload,{selector}) は読み直しの受け口"
        );
    }
    assert_eq!(
        ledger.consumer_of("reloadsurface", None),
        Some(CommandConsumer::ReloadSink),
        "reloadsurface は選別子なしで読み直しの受け口"
    );
    for selector in [Some("makoto"), Some("aigraph"), Some("Ghost"), None] {
        assert_eq!(
            ledger.consumer_of("reload", selector),
            None,
            "(reload,{selector:?}) は担当なし"
        );
    }
    let mut ext = ledger.clone();
    ext.try_register("reload", Some("makoto"), CommandConsumer::Seriko)
        .expect("(reload,makoto) は後から足せる（要件 11.2）");
}

/// areka-P0-mcp-reload 要件 13.5: `bind` の行の名前は拾う側（seriko）と共有する定数そのもの。
#[test]
fn canonical_bind_row_uses_the_seriko_constant() {
    assert_eq!(
        ConsumerLedger::canonical().consumer_of(areka_seriko::BIND_COMMAND, None),
        Some(CommandConsumer::Seriko)
    );
}

/// areka-P0-window-link-directives 要件 1.4・3.1・3.5・7.1: 吸着・窓の組・バルーン位置初期化の 5 組は
/// 窓の位置の受け口の担当で、名前と第 1 引数の掛け合わせ（`reset,alignmenttodesktop` など）は担当なし。
#[test]
fn canonical_registers_the_five_window_link_pairs_for_the_move_sink() {
    let ledger = ConsumerLedger::canonical();
    for (name, selector) in [
        ("set", "alignmenttodesktop"),
        ("set", "alignmentondesktop"),
        ("set", "sticky-window"),
        ("reset", "sticky-window"),
        ("execute", "resetballoonpos"),
    ] {
        assert_eq!(
            ledger.consumer_of(name, Some(selector)),
            Some(CommandConsumer::MoveSink),
            "({name},{selector}) は窓の位置の受け口"
        );
    }
    for (name, selector) in [
        ("reset", "alignmenttodesktop"),
        ("reset", "alignmentondesktop"),
        ("execute", "sticky-window"),
        ("set", "resetballoonpos"),
    ] {
        assert_eq!(
            ledger.consumer_of(name, Some(selector)),
            None,
            "({name},{selector}) は担当なし"
        );
    }
}
