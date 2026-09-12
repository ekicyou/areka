//! 「戻す操作」の公開・装着時の 2 層の差し込み・記録の 1 度化の決定論テスト
//! （タスク 4.2・要件 10.2／10.3／10.4／10.5／10.7・4.1・6.3・13.1／13.4／13.5）。
//!
//! [`super`]（`state_decoration_tests.rs`）の子モジュールとして接続してあり、台本を組む
//! 補助（`font`・`text`・`st`・`look_of`）を共有する。1 ファイル 1,000 行の見張りに余裕を
//! 残すため、タスク 4.1 の試験とはファイルを分けている。
//!
//! 記録の件数は `log-capture-kit`（ワークスペース唯一の捕捉機構）で数える。「出ない」を
//! 主張する試験には、同じ窓・同じ発行点で「出る側」を 1 件置いた対照を添えてある。

use super::*;

// ══════════════════════ タスク 4.2（戻す操作の公開・記録の 1 度化） ══════════════════════
//
// | § | 内容 | 要件 |
// |---|---|---|
// | §8 | 「戻す操作」を 1 つの関数として公開（スコープ指定・全スコープ） | 10.3, 10.5 |
// | §9 | 装着時に 2 層を差し込む口（旧い既定と同値なら追随） | 4.1 |
// | §10 | 記録は「キーと値」ごとに 1 台詞 1 度・台本の先頭で数え直す | 13.1, 13.4 |
// | §11 | 上下付きが有効なまま文字が追記されたときの記録 | 6.3 |
// | §12 | `\f[disable]` も一括の戻し（`\f[default]` との対称） | 10.2, 10.4 |
// | §13 | 記録なしで失敗を飲み込む経路が無いことの字面検査 | 13.5 |
// | §14 | **本番の**一括の戻しが項目を列挙しないことの字面検査 | 10.4 |

use log_capture_kit::{CapturedEvent, capture};

use crate::look::{LookLayers, Script};

/// 捕捉した記録のうち、本文に `needle` を含む `warn` の件数。
fn warns(events: &[CapturedEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.message().contains(needle))
        .count()
}

/// 装飾を 1 つずつ載せた状態（見た目・所有外キーの両方が既定から外れている）。
fn state_with_look_and_unowned() -> TextLayerState {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&font("0", &["align", "center"]));
    state.apply_cue(&text("0", "あ"));
    state
}

/// 装着で差し込む 2 層（既定の大きさ 20・赤文字＝正典の既定と違う値）。
fn attached_layers() -> LookLayers {
    LookLayers::from_balloon(
        vec!["Meiryo".to_owned()],
        20.0,
        (255, 0, 0),
        (255, 255, 255),
        (0, 0, 255),
    )
}

// ------------------------------------ §8 「戻す操作」の公開（R10.3／R10.5）

/// 一括の戻し（`\f[default]`）と戻す操作（`reset_decoration`）が同じ結果になる。
#[test]
fn the_bulk_reset_and_the_reset_operation_agree() {
    let mut by_tag = state_with_look_and_unowned();
    let mut by_call = state_with_look_and_unowned();

    by_tag.apply_cue(&font("0", &["default"]));
    by_call.reset_decoration(Some(&ActorKey::from("0")));

    assert_eq!(
        st(&by_tag, "0").current_look(),
        st(&by_call, "0").current_look()
    );
    assert_eq!(
        st(&by_tag, "0").unowned_vocab(),
        st(&by_call, "0").unowned_vocab()
    );
    assert_eq!(
        st(&by_call, "0").current_look(),
        &st(&by_call, "0").look_layers().default,
        "戻す操作は既定の見た目へ戻す"
    );
    assert!(
        st(&by_call, "0").unowned_vocab().is_empty(),
        "所有外キーの保持も戻す操作に含まれる"
    );
}

/// 全スコープの戻しは 1 回の呼び出しで両方のスコープを戻す（R10.5）。
#[test]
fn the_reset_operation_covers_every_scope_in_one_call() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&font("1", &["italic", "1"]));

    state.reset_decoration(None);

    assert!(!st(&state, "0").current_look().bold);
    assert!(!st(&state, "1").current_look().italic);
}

/// 戻す操作は既に追記済みの文字の見た目を変えない（R10.7）。
#[test]
fn the_reset_operation_does_not_touch_glyphs_already_placed() {
    let mut state = state_with_bold_then_text();
    state.reset_decoration(Some(&ActorKey::from("0")));

    assert!(look_of(&state, "0", 0).bold, "追記済みの文字は太字のまま");
    assert!(
        !st(&state, "0").current_look().bold,
        "以降の文字には効かない"
    );
}

/// 台本の先頭（全消去）も同じ戻す操作を通る——見た目の結果が一致する（R10.3）。
#[test]
fn the_talk_start_reaches_the_same_reset_result() {
    let mut by_clear_all = state_with_look_and_unowned();
    let mut by_call = state_with_look_and_unowned();

    by_clear_all.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    by_call.reset_decoration(None);

    assert_eq!(
        st(&by_clear_all, "0").current_look(),
        st(&by_call, "0").current_look()
    );
    assert_eq!(
        st(&by_clear_all, "0").unowned_vocab(),
        st(&by_call, "0").unowned_vocab()
    );
    // 絶対値の錨——2 つを突き合わせるだけだと、戻しを丸ごと no-op にしても両方が
    // 「戻っていない同じ値」で一致して緑になる。
    assert_eq!(
        st(&by_clear_all, "0").current_look(),
        &st(&by_clear_all, "0").look_layers().default,
        "台本の先頭の戻しは既定の見た目へ戻す"
    );
}

// ------------------------------------------- §9 装着時の 2 層の差し込み（R4.1）

/// 現在の見た目が旧い既定と同値なら、装着で差し込んだ新しい既定へ追随する。
#[test]
fn attaching_layers_moves_an_untouched_look_to_the_new_default() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));

    state.set_look_layers(&ActorKey::from("0"), attached_layers());

    let actor = st(&state, "0");
    assert_eq!(actor.current_look(), &actor.look_layers().default);
    assert_eq!(actor.current_look().height, 20.0);
    assert_eq!(actor.current_look().color, (255, 0, 0));
}

/// 明示された見た目は装着で上書きされない（旧い既定と同値でないので追随しない）。
#[test]
fn attaching_layers_keeps_an_explicit_look() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));

    state.set_look_layers(&ActorKey::from("0"), attached_layers());

    let actor = st(&state, "0");
    assert!(actor.current_look().bold, "明示した太字が残る");
    assert_eq!(
        actor.current_look().color,
        (0, 0, 0),
        "明示された見た目は丸ごと保たれる（新しい既定へ移らない）"
    );
    assert_eq!(
        actor.look_layers().default.color,
        (255, 0, 0),
        "2 層そのものは差し替わる"
    );
}

/// 装着より先に `\f` が届いた窓を**記録が捕まえる**（タスク 9.4 の裁定）。
///
/// cue のドレインは非同期で、`text_slot_view` が `None` の間は装着が次フレームへ委ねられる
/// （`crates/areka/src/emo2_boot/frame/attach.rs::connect_balloon_text` の `None` の腕）。
/// この窓で `\f[...]` が先に届くと、直上の
/// [`attaching_layers_keeps_an_explicit_look`] が固定している追随ガードが成立せず、
/// バルーン定義の既定（大きさ・色・フォント名）が**その台詞のあいだ届かない**——以後の
/// 文字が素の既定 12px で描かれる。次の台詞頭の `ClearAll` で自然治癒する。
///
/// **挙動の是正は本仕様では行わない**（引受先＝`areka-P0-emo-text-canon-residue`）。
/// 正しく直すには「作者が明示した項目だけを新しい既定へ載せ替える」3 者併合が要り、
/// それは要件 10.4 の「項目を列挙しない」（後続仕様が [`crate::look::TextLook`] へ足した
/// 項目も自動で戻しと追随に含まれる）と衝突する設計判断を伴う。ここで固定するのは
/// **黙って落ちない**ことだけ——既定が届かなかったことが記録に 1 件残る。
#[test]
fn attaching_over_an_explicit_look_records_that_the_defaults_could_not_land() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));

    let ((), events) = capture(|| state.set_look_layers(&ActorKey::from("0"), attached_layers()));

    let actor = st(&state, "0");
    assert_eq!(
        actor.current_look().height,
        12.0,
        "窓の実害——バルーン定義の 20px が現在の見た目へ届かない"
    );
    assert_eq!(
        warn_count(&events),
        1,
        "黙って落とさない: 既定が届かなかったことが 1 件記録される"
    );
}

/// 較正の負の側——**装着が先**の正常な順序では記録が 0 件（恒真な警告になっていない）。
///
/// 初回装着（現在の見た目が旧い既定と同値＝追随する）と、`\f` の後の再装着
/// （k 再追従。既定はもう素の既定ではないので窓ではない）の**両方**で 0 件を断言する。
#[test]
fn attaching_in_the_normal_order_records_nothing() {
    let mut state = TextLayerState::default();
    state.apply_cue(&text("0", "あ"));

    let ((), first) = capture(|| state.set_look_layers(&ActorKey::from("0"), attached_layers()));
    state.apply_cue(&font("0", &["bold", "1"]));
    let ((), again) = capture(|| state.set_look_layers(&ActorKey::from("0"), attached_layers()));

    let actor = st(&state, "0");
    assert!(actor.current_look().bold, "明示した太字は残る");
    assert_eq!(
        actor.current_look().height,
        20.0,
        "正常な順序ならバルーン定義の既定が届いている"
    );
    assert_eq!(warn_count(&first), 0, "初回装着で警告は出ない");
    assert_eq!(warn_count(&again), 0, "再装着（k 再追従）でも警告は出ない");
}

/// 捕まえた記録のうち `warn` の件数。
fn warn_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .count()
}

/// 装着の後の戻す操作は、差し込んだ新しい既定へ戻る。
#[test]
fn the_reset_operation_uses_the_attached_layers() {
    let mut state = TextLayerState::default();
    state.apply_cue(&font("0", &["bold", "1"]));
    state.set_look_layers(&ActorKey::from("0"), attached_layers());

    state.reset_decoration(Some(&ActorKey::from("0")));

    assert_eq!(st(&state, "0").current_look().color, (255, 0, 0));
    assert!(!st(&state, "0").current_look().bold);
}

// ------------------------------- §10 記録は「キーと値」ごとに 1 台詞 1 度（R13.1／R13.4）

/// 同じ不正な値を 3 度繰り返しても記録は 1 件。
#[test]
fn the_same_bad_value_is_recorded_once_per_talk() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        for _ in 0..3 {
            state.apply_cue(&font("0", &["bold", "yes"]));
        }
    });

    assert_eq!(
        warns(&events, "\\f の指定を適用できない"),
        1,
        "同じキーと値の記録が積み上がっている"
    );
}

/// 対照——値が違えば別の記録になる（捕捉が空振りしていないことの確認も兼ねる）。
#[test]
fn a_different_bad_value_is_recorded_separately() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["bold", "yes"]));
        state.apply_cue(&font("0", &["bold", "no"]));
    });

    assert_eq!(warns(&events, "\\f の指定を適用できない"), 2);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.field_str("value") == Some("yes"))
            .count(),
        1,
        "記録は値ごとに引ける"
    );
}

/// 台本の先頭をまたぐと、同じ不正な値がもう 1 度記録される。
#[test]
fn the_record_returns_after_the_talk_starts_again() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["bold", "yes"]));
        state.apply_cue(&font("0", &["bold", "yes"]));
        state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
        state.apply_cue(&font("0", &["bold", "yes"]));
        state.apply_cue(&font("0", &["bold", "yes"]));
    });

    assert_eq!(
        warns(&events, "\\f の指定を適用できない"),
        2,
        "台詞ごとに 1 件（台本の先頭で記録済みの集合が空にならないと 1 件になる）"
    );
}

/// 語彙のみの項目・スタイルシートの語・アンカー色も同じ 1 度化を通る（R13.1）。
#[test]
fn the_vocabulary_only_records_are_also_capped_at_one() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        for _ in 0..2 {
            state.apply_cue(&font("0", &["outline", "1"]));
            state.apply_cue(&font("0", &["height", "x-large"]));
            state.apply_cue(&font("0", &["color", "default.anchor"]));
        }
    });

    assert_eq!(warns(&events, "\\f の指定を適用できない"), 3);
}

// ----------------------------------- §11 上下付きが有効なまま文字が追記された（R6.3）

/// 下付きが有効な間に文字を 3 度追記しても記録は 1 件。
#[test]
fn appending_text_while_a_script_is_active_is_recorded_once() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["sub", "1"]));
        for _ in 0..3 {
            state.apply_cue(&text("0", "あ"));
        }
    });

    assert_eq!(warns(&events, "上下付きが有効なまま文字を追記した"), 1);
}

/// 対照——上下付きが無効な間の追記は記録されない（同じ窓に「出る側」を 1 件置く）。
#[test]
fn appending_text_without_a_script_records_nothing() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&text("0", "あ"));
        // 出る側の対照——同じ窓・同じ発行点で 1 件は必ず出る。
        state.apply_cue(&font("0", &["sup", "1"]));
        state.apply_cue(&text("0", "い"));
    });

    assert_eq!(
        warns(&events, "上下付きが有効なまま文字を追記した"),
        1,
        "上下付きを立てた後の追記だけが記録される"
    );
}

/// 上下付きを外せば以降の追記は記録されない。
#[test]
fn turning_the_script_off_stops_the_record() {
    let (state, events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["sub", "1"]));
        state.apply_cue(&font("0", &["sub", "0"]));
        state.apply_cue(&text("0", "あ"));
        state
    });

    assert_eq!(warns(&events, "上下付きが有効なまま文字を追記した"), 0);
    assert_eq!(st(&state, "0").current_look().script, Script::None);
}

/// 台本の先頭をまたぐと、上下付きの追記の記録ももう 1 度出る。
#[test]
fn the_script_record_returns_after_the_talk_starts_again() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["sup", "1"]));
        state.apply_cue(&text("0", "あ"));
        state.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
        state.apply_cue(&font("0", &["sup", "1"]));
        state.apply_cue(&text("0", "い"));
    });

    assert_eq!(warns(&events, "上下付きが有効なまま文字を追記した"), 2);
}

/// 0 文字の追記は記録を生まない（表を汚さないのと同じ地点）。
#[test]
fn appending_zero_glyphs_records_nothing() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["sub", "1"]));
        state.apply_cue(&text("0", ""));
    });

    assert_eq!(warns(&events, "上下付きが有効なまま文字を追記した"), 0);
}

/// 2 種類の記録は互いに素な鍵空間を持つ——一方が他方を無記録で飲み込まない（R13.5）。
///
/// `\f[sub,追記]` は値が正しい真偽の語でないので「`\f` の指定を適用できない」側の記録を作り、
/// その綴りは追記点の記録が使うキー（`sub`）と字面で近い。鍵空間を共有していた形
/// （どちらも `"sub=追記"`）では先に立った方が後から来た方を飲み込み、片方が 0 件になった。
/// 値（`追記`）は台本作者が自由に書ける文字列なので、これは到達可能な欠陥である。
#[test]
fn the_two_record_kinds_do_not_share_a_key_space() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["sub", "1"]));
        state.apply_cue(&font("0", &["sub", "追記"]));
        state.apply_cue(&text("0", "あ"));
    });

    assert_eq!(
        warns(&events, "\\f の指定を適用できない"),
        2,
        "`\\f[sub,1]`（語彙のみ）と `\\f[sub,追記]`（値が真偽の語でない）で値ごとに 1 件（R13.1）"
    );
    assert_eq!(
        warns(&events, "上下付きが有効なまま文字を追記した"),
        1,
        "下付きが有効なままの追記の記録（R6.3・鍵空間を共有すると消える）"
    );
}

/// 同じ種別の中でも、鍵は（キー, 値）の組であって 1 本の綴りではない（R13.4／R13.5）。
///
/// `\f[a=b,c]`（キーは `a=b`）と `\f[a,b=c]`（キーは `a`）は別々の失敗だが、鍵を
/// `format!("{key}={value}")` へ平坦化すると**どちらも `"a=b=c"`** になり、2 件目が無記録で
/// 飲み込まれる。`=` はキー側にも値側にも現れうる——どちらも台本作者が自由に書ける文字列
/// なので、これは到達可能な欠陥である。件数だけでなく両方の `key` 欄も確かめる。
#[test]
fn a_record_key_is_the_pair_not_a_flattened_spelling() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["a=b", "c"]));
        state.apply_cue(&font("0", &["a", "b=c"]));
    });

    assert_eq!(
        warns(&events, "\\f の指定を適用できない"),
        2,
        "平坦化した鍵では 2 件目が飲み込まれる"
    );
    let keys: Vec<&str> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .filter_map(|e| e.field_str("key"))
        .collect();
    assert_eq!(
        keys,
        ["a=b", "a"],
        "別々のキーの失敗がそれぞれ引ける（件数だけでは取り違えを見張れない）"
    );
}

/// 鍵の第 3 要素は値の**列**であって、カンマで繋いだ 1 本の綴りではない（R13.4／R13.5）。
///
/// 引用符はトークンの中のカンマを守る（`areka-parsers` の
/// `lexer_tests.rs::quoted_arg_protects_comma` が `\s["a,b"]` → 引数 `["a,b"]` を固定して
/// いる）。よって `\f[bold,"x,y"]`（値 1 個・真偽の語でない）と `\f[bold,x,y]`（値 2 個・
/// 個数違い）は**理由まで違う別々の失敗**なのに、鍵を `values.join(",")` へ平坦化すると
/// どちらも `"x,y"` になり、2 件目が無記録で飲み込まれる。件数だけでなく 2 つの `reason`
/// が違うことも確かめる（件数だけでは取り違えを見張れない）。
#[test]
fn a_record_key_keeps_the_value_list_not_a_joined_spelling() {
    let ((), events) = capture(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&font("0", &["bold", "x,y"]));
        state.apply_cue(&font("0", &["bold", "x", "y"]));
    });

    assert_eq!(
        warns(&events, "\\f の指定を適用できない"),
        2,
        "平坦化した鍵では 2 件目が飲み込まれる"
    );
    let reasons: Vec<&str> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .filter_map(|e| e.field_str("reason"))
        .collect();
    assert_eq!(reasons.len(), 2, "理由が 2 件そろう");
    assert_ne!(
        reasons[0], reasons[1],
        "値 1 個（語が真偽でない）と値 2 個（個数違い）は別の理由——同じ鍵へ潰れていない"
    );
}

// ------------------------------------- §12 `\f[disable]` も一括の戻し（R10.2／R10.4）

/// `\f[disable]` は無効表示の見た目へ丸ごと合わせ、所有外キーの保持も空にする
/// （`\f[default]` との対称・タスク 4.2 の裁定）。
#[test]
fn the_disable_tag_is_a_bulk_reset_too() {
    let mut state = TextLayerState::default();
    state.set_look_layers(&ActorKey::from("0"), attached_layers());
    state.apply_cue(&font("0", &["bold", "1"]));
    state.apply_cue(&font("0", &["align", "center"]));

    state.apply_cue(&font("0", &["disable"]));

    let actor = st(&state, "0");
    assert_eq!(
        actor.current_look(),
        &actor.look_layers().disable,
        "全項目が無効表示の見た目に合う"
    );
    assert!(
        actor.unowned_vocab().is_empty(),
        "一括の戻しなので所有外キーの保持も空になる（`\\f[default]` と対称）"
    );
}

// ---------------------------------------------------- §13 記録なしの失敗経路が無い（R13.5）

/// 失敗の分類（design.md「Error Handling」の分類表）と実装の腕を字面で突き合わせる。
///
/// 値の比較では「記録なしで飲み込む経路」を見張れない——記録を落としても値は同じだからである。
/// ゆえに `apply_font_args` の `match` の腕を 1 つずつ数え、各腕が記録へ落ちることを固定する。
///
/// **この試験の天井**——見張れるのは「`ARMS` に挙げた腕が消えていないこと」「挙げた各腕の
/// 切片に記録があること」「catch-all が無いこと」の 3 つだけである。腕の切片は「次に現れる
/// 既知の腕の開始位置」までなので、**既知の 2 腕の間に無記録の腕が新設されると、直前の腕の
/// 切片がそれを飲み込んで緑のままになる**。`Note` に variant を足す形の追加はコンパイラが
/// 網羅性で捕まえるが、`Ok(Some(...))` 以外の形（たとえば `Err` を種別で分ける）で腕を
/// 挟むと素通りする。腕を増やしたら `ARMS` にも足すこと。
#[test]
fn no_failure_path_is_swallowed_without_a_record() {
    // （腕の綴り, その腕が通る記録）——`Ok(None)` だけが「失敗でないので記録なし」。
    const ARMS: [(&str, Option<&str>); 6] = [
        ("Ok(None) => {}", None),
        ("Ok(Some(Note::Unowned)) =>", Some("tracing::debug!")),
        (
            "Ok(Some(Note::VocabularyOnly { key })) =>",
            Some("self.warn_once("),
        ),
        (
            "Ok(Some(Note::StylesheetKeyword)) =>",
            Some("self.warn_once("),
        ),
        (
            "Ok(Some(Note::AnchorColorAsDefault)) =>",
            Some("self.warn_once("),
        ),
        ("Err(issue) =>", Some("self.warn_once(")),
    ];
    // `ARMS` の長さは配列型が固定しているので実行時に数え直す意味は無い（コンパイル時に
    // 恒真）。分類表との対応は、下の走査が各綴りを実装から引けることで担保する。

    let src = include_str!("state_decoration.rs");
    // 走査面は `apply_font_args` の `match` 1 つ——次の私有メソッドの手前で切る
    // （切らないと以降の関数の catch-all を拾って恒偽になる）。
    let body = src
        .split_once("match apply_font_tag(")
        .expect("apply_font_args の分岐が読めるはず")
        .1;
    let body = body
        .split_once("\n    fn ")
        .expect("apply_font_args の次に私有メソッドがあるはず")
        .0;

    let starts: Vec<usize> = ARMS
        .iter()
        .map(|(pattern, _)| {
            body.find(pattern)
                .unwrap_or_else(|| panic!("腕 `{pattern}` が実装から消えている"))
        })
        .collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();

    for (index, (pattern, recorder)) in ARMS.iter().enumerate() {
        let start = starts[index];
        let end = sorted
            .iter()
            .copied()
            .find(|at| *at > start)
            .unwrap_or(body.len());
        let arm = &body[start..end];
        match recorder {
            Some(call) => assert!(
                arm.contains(call),
                "腕 `{pattern}` が記録（{call}）なしで失敗を飲み込んでいる"
            ),
            None => assert!(
                !arm.contains("warn_once") && !arm.contains("tracing::"),
                "腕 `{pattern}` は失敗ではないので記録を持たない"
            ),
        }
    }

    assert!(
        !body.contains("_ =>"),
        "catch-all の腕があると `Note` の追加が黙って無記録で通る"
    );
}

// -------------------------- §14 本番の一括の戻しは項目を列挙しない（R10.4）

/// 本番の「一括の戻し」の実体（`ActorTextState::reset_look_to`）が見た目を**丸ごと**
/// 置き換えていることを字面で固定する。
///
/// **`look.rs` 側の字面検査では本番を見張れない**——`look_font_tag_tests.rs::the_bulk_reset_replaces_the_look_as_a_whole_without_listing_items`
/// が走査する `look.rs::apply_font_tag` の `key == "default"`／`"disable"` の 2 腕は、
/// `apply_font_args` が先に掴んで戻す操作へ回すため**本番経路から到達しない**
/// （`apply_font_args` の doc とタスク 4.2 の裁定）。本番の実体は本ファイルの
/// `reset_look_to` であり、そこを項目の列挙へ書き換えても向こうの検査は緑のままになる。
///
/// 値の比較でも見張れない——今日の [`crate::look::TextLook`] の項目集合では、列挙形と
/// 丸ごと置き換えは同じ結果になる。後続仕様が項目を足したときに戻しから漏れるかどうかは
/// 字面にしか現れない。
#[test]
fn the_production_bulk_reset_replaces_the_look_as_a_whole() {
    let src = include_str!("state_decoration.rs");
    let body = src
        .split_once("fn reset_look_to(&mut self, layer: TextLook) {")
        .expect("一括の戻しの共通実体 reset_look_to が読めるはず")
        .1;
    let body = body
        .split_once("\n    }")
        .expect("reset_look_to の閉じ括弧があるはず")
        .0;
    assert!(
        body.contains("self.decor.current = layer;"),
        "本番の一括の戻し（reset_look_to）が丸ごとの置き換えでなくなっている——\
         項目を列挙すると後続仕様が足した項目が戻しから漏れる。本体:{body}"
    );
    assert!(
        !body.contains("self.decor.current."),
        "本番の一括の戻し（reset_look_to）が `current` の項目を名指しで書き換えている——\
         項目を列挙すると後続仕様が足した項目が戻しから漏れる。本体:{body}"
    );
}
