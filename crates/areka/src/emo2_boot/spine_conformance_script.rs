//! 適合一周走行の台本と期待列（areka-P0-emo2-conformance-e2e・design D2）。
//!
//! 本ファイルは**逐語のデータだけ**を置く。段の駆動（注入と有界待ち）は
//! `spine_conformance_support.rs`（design D6）が持ち、
//! 突き合わせの判定は `spine_conformance_lap_tests.rs`（design D1）が持つ。ゆえに本ファイルには
//! **駆動も判定も 1 行も無い**（`#[test]` も `assert` も置かない）。期待値を 1 か所へ集めることで、
//! 上流の裁定が変わったときの追随点が 1 つになる（design「File Structure Plan」）。
//!
//! # 期待値の出所は正典ではなく**実装**である（design D2 の表題）
//!
//! 参照列の中身は ukadoc の記述ではなく、実際に組み立てている純関数の逐語から採る。各定数・
//! 各期待要素の直上に、読み取った `file:line` を注記してある。正典と実装が食い違う箇所
//! （Ref1/Ref2 の省略など）は**実装側**を写す——本仕様は確かめる側であって裁定を覆さない。
//!
//! # 送らないことの期待（design D2「送らないことの期待」）
//!
//! 次の 3 群は「送らない」ことが期待である。本ファイルはそれらを[`expected_calls`]へ**書かない**
//! ——それだけで期待になる。判定は列の**等値**照合であり部分一致・包含判定を用いないため
//! （R2.5・design D3）、期待列に無い呼出が 1 件でも現れれば走行は赤になる。検査そのものは
//! `spine_conformance_lap_tests.rs` の領分であり、本ファイルは 1 行も検査しない。
//!
//! 1. 会話の発火と時報（`OnTalk`／`OnHour`）——適合対象が毎秒の変化通知の内部で自発生成するため、
//!    ベースウェア側が送ると二重になる（`crates/areka-kanade/src/schedule/events.rs:60` が
//!    送出許可集合から恒久的に除いている）。
//! 2. 更新系のイベント群とバルーン変更のイベント——適合対象の辞書に受け口は在るが、契機
//!    （ネットワーク更新・バルーンの切替）が本走行では発生しない。送出許可集合
//!    （同 `:70-82`）にそもそも載っていない。
//! 3. 正典の選択確定イベント（`OnChoiceSelectEx`／`OnChoiceSelect`）——適合対象の選択肢 ID は
//!    すべて `On` 始まりであり、実装は同名イベントを 1 段だけ直接発火して正典段を先行させない
//!    （`crates/areka-kanade/src/schedule/choice.rs:58-66` の `plan_cascade` が `On` 始まりを
//!    `CascadePlan::Named` へ写す）。
//!
//! # 消費側がまだ居ないこと
//!
//! 本ファイルの項目を読むのは一周テスト（design D1）であり、その走行本体は後続タスク
//! （`tasks.md` 3.1）が置く。それまでは未使用のため、モジュール全体へ理由付きで
//! `allow(dead_code)` を置く（黙らせるのではなく、消費側が来る時期を明記する）。
//! 〈段名・表示指令〉の期待列も、走行からの採取を伴うため同じ相（`tasks.md` 3.2）で
//! 本ファイルへ追補する。

// 消費側（一周テスト本体・design D1）は tasks.md 3.1 で置かれる。それまでは本ファイルの
// 公開面をどこも読まないため、未使用警告を理由付きで抑止する。
#![allow(dead_code)]

use super::conformance_support::{DisplayProjection, RecordedStatus};
use super::{ExitKind, RecordedCall, ScriptedShioriBackend, ScriptedShioriHandle, shell_target};

// ===========================================================================
// 段の注入時刻の区間（design D1 の段表・D3「段の区間はテストが先に宣言する定数」）
// ===========================================================================

/// 段 1 つぶんの注入時刻の区間（逐語宣言・走行のたびに変わらない）。
///
/// `begin_ms` は前段の `limit_ms` 以上から始まり、注入時刻は `limit_ms` を超えない
/// （design D6 の不変条件）。`limit_ms` に達したら以後は注入せず観測だけを待つ——注入時刻が
/// 観測を追い越すと待っている条件そのものが壊れるためである（`spine.rs:333-341` の実測）。
pub(super) struct LapStage {
    /// 段名（表示指令の台帳の第 1 要素にもなる・design D3）。
    pub(super) name: &'static str,
    /// 注入時刻の下限（この段が最初に注入しうる時刻）。
    pub(super) begin_ms: u64,
    /// 注入時刻の上限（頭打ち）。
    pub(super) limit_ms: u64,
}

/// 1 回の注入で注入時刻が進む刻み（毎秒の変化通知の「1 秒相当」に合わせる）。
///
/// `KanadeMsg::Tick { now }` は 1 秒相当の Tick として定義されている
/// （`crates/areka-kanade/src/msg.rs:122-123`）。
pub(super) const TICK_STEP_MS: u64 = 1_000;

/// 一周の全段（design D1「段・注入・完了条件」の表と同順・同名）。
///
/// # 区間の採寸（task 3.1 の実測・レビュー指摘を受けた改訂 2）
///
/// 区間の幅は「注入時刻をどこまで進めてよいか」であると同時に、刻み [`TICK_STEP_MS`] で割った
/// **注入の予算**でもある。段の待ちには 2 つの相が混ざっており、予算を食うのは片方だけである。
///
/// - **⑴ 着地待ち**——投函した入力が実スレッドを何段も渡って新しい会話を起こすまで。実測 1〜2 反復
///   だが、高負荷では桁で伸びる。ここは `StageSink::may_advance_clock` が偽を返すあいだ
///   **注入時刻を据え置いたまま**再生側 Tick を投函し続けるので、**予算を 1 本も食わない**。
/// - **⑵ 再生を進める相**——起きた会話を占有終端の先まで進めるまで。一周で最長の占有区間は
///   実測 0.65 秒＝**1 本**で足りる。予算が要るのはここだけである。
/// - **⑶ 余韻**——完了条件が成立した後、実 async の着地を待つ観測だけの相。
///   `WaitInjection::DispatcherTickThenObserve` が注入をやめるので、ここも**予算を食わない**。
///
/// ゆえに再生を伴う段の幅は **20 秒＝20 本**で足りる（必要 2〜4 本に対し 5〜10 倍）。初版は
/// この 3 相を分けずに幅だけを広げ（100 秒）、⑶ が予算を食い尽くして次段の状態を壊し、改訂 1 は
/// ⑶ を止めたものの ⑴ が予算を食い尽くして再生が凍った。幅ではなく**相の切り分け**が効く。
///
/// - **自発会話・会話中の抑止だけ幅 1 秒＝1 本**。この 2 段は kanade へ毎秒の変化通知を 1 本
///   投函するだけで、待ちには**何も注入しない**（`WaitInjection::Idle`）。再生を 1 ミリ秒も
///   進めないので、会話の占有が待っている間に終わってしまう競争が**構造的に存在しない**。
/// - **段の間隔は 2 秒**。段が変わると注入時刻は次の段の下限へ跳ぶが、跳んだ先で最初に投函されるのは
///   各段の `once`（選択確定など）であり、再生側 Tick はその後ろに並ぶ。ゆえに跳びが選択待ちの
///   成否に効くことはない。
///
/// **「サブメニューと戻り」だけ区間が 2 つある**。この段は選択肢を 2 つ確定するが、選択肢 ID は
/// 直前に再生中の台本の `\q` 帳簿に含まれていなければ弾かれる
/// （`crates/areka-kanade/src/schedule/steady.rs:262`）ため、2 つを連続反復で投函できない。段名は
/// 同じまま区間だけを割り、各々を選択待ちまで再生し切らせる（段は 10・区間は 11）。
pub(super) const LAP_STAGES: &[LapStage] = &[
    LapStage {
        name: "起動",
        begin_ms: 0,
        limit_ms: 0,
    },
    LapStage {
        name: "装着",
        begin_ms: 0,
        limit_ms: 20_000,
    },
    LapStage {
        name: "自発会話",
        begin_ms: 22_000,
        limit_ms: 23_000,
    },
    LapStage {
        name: "会話中の抑止",
        begin_ms: 25_000,
        limit_ms: 26_000,
    },
    LapStage {
        name: "撫で",
        begin_ms: 28_000,
        limit_ms: 48_000,
    },
    LapStage {
        name: "メニュー",
        begin_ms: 50_000,
        limit_ms: 70_000,
    },
    LapStage {
        name: "選択確定",
        begin_ms: 72_000,
        limit_ms: 92_000,
    },
    LapStage {
        name: "サブメニューと戻り",
        begin_ms: 94_000,
        limit_ms: 114_000,
    },
    LapStage {
        name: "サブメニューと戻り",
        begin_ms: 116_000,
        limit_ms: 136_000,
    },
    LapStage {
        name: "位置調整",
        begin_ms: 138_000,
        limit_ms: 158_000,
    },
    LapStage {
        name: "終了",
        begin_ms: 160_000,
        limit_ms: 180_000,
    },
];

/// 一周を通じた注入時刻の上限（最終段の頭打ち＝[`LAP_STAGES`] の最後の `limit_ms`）。
///
/// この値が 1 時間（3,600,000 ms）に満たないことが、毎秒の変化通知の Ref0 が走行を通じて
/// `"0"` で一定になる根拠である（Ref0 は注入時刻を時へ割った値＝
/// `crates/areka-kanade/src/schedule/events.rs:173`）。180,000 ms は 1 時間の 20 分の 1 に満たない。
pub(super) const LAP_INJECT_LIMIT_MS: u64 = 180_000;

// ===========================================================================
// 毎秒の変化通知の応答本数（task 2.2 の規則・本数を駆動器から借りない）
// ===========================================================================

/// 一周を通じて投函されうる毎秒の変化通知の**上限本数**（段の上限だけから導く）。
///
/// # 導き方（この 3 行だけで閉じる）
///
/// 1. 注入時刻は単調増加し、[`LAP_INJECT_LIMIT_MS`] を超えない（design D6 の不変条件）。
/// 2. 1 回の注入で注入時刻は [`TICK_STEP_MS`] 進み、上限に達したら以後は注入しない。
/// 3. ゆえに投函回数は多くとも `上限 / 刻み` 回＋起点の 1 回である。
///
/// **駆動器が実際に何本投げるかは参照しない**（借りると、駆動器を書き換えたときに台本が
/// 黙って追随して規律が消える）。実際の本数は必ずこの上限以下になる。
pub(super) const MAX_SECOND_CHANGE_CALLS: usize = (LAP_INJECT_LIMIT_MS / TICK_STEP_MS) as usize + 1;

/// 応答の待ち行列へ足す余裕。**必ず正**とする。
///
/// 行列が尽きると台本受け口はその場で落ちる（`spine.rs:245`／`:267` の `panic!`）。余裕が 0 だと
/// 段の境目で 1 本余分に投函されただけで受け口が落ち、「何が違ったか」が読めない失敗になる。
/// 逆に余った応答は 1 件も記録に残らない（記録は呼出の側だけを積む＝`spine.rs:232-245`）ので
/// 害が無く、余分な**呼出**の方は記録に残って列の等値照合が読みやすい失敗として拾う
/// （design D2「余裕を足しても余計な呼出は記録に残る」）。
pub(super) const SECOND_CHANGE_SLACK: usize = 8;

/// 毎秒の変化通知に積む応答の本数（照会側・片道側で各々この本数）。
///
/// 照会（会話可）と片道（会話中）は台本受け口では**別の待ち行列**であり
/// （`spine.rs:145` の `get` と `:158` の `notify` が別の表へ積む）、どちらが何本消費されるかは
/// 会話の占有状況で決まる。ゆえに両方へ同じ上限＋余裕を積む。
pub(super) const SECOND_CHANGE_RESPONSES: usize = MAX_SECOND_CHANGE_CALLS + SECOND_CHANGE_SLACK;

// ===========================================================================
// 交信の逐語値（実装から読み取った定数）
// ===========================================================================

/// `OnFirstBoot` の Ref0（消滅回数の 10 進文字列）。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:124-129`（`vanish_count.to_string()`）。
/// spine ハーネスは起動のたびに永続状態の置き場を消してから起動する（`spine.rs:498`）ため、
/// 消滅回数は常に 0 である。
pub(super) const FIRST_BOOT_VANISH_COUNT: &str = "0";

/// `OnBoot` の Ref0（シェル名）。
///
/// 出所: `crates/areka-ghost/src/config.rs:32`（`resolve_shell_name`）が
/// `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt:3` の `name` を
/// 読む。同ファイルは冒頭で `charset,UTF-8` を宣言しており、復号は宣言に従う
/// （`crates/areka-parsers/src/charset/decode.rs:26-45`）ので、下の逐語がそのまま Ref0 になる。
pub(super) const SHELL_NAME: &str = "「コンフィズリー」＆「City-Pop'n」";

/// `basewareversion` の Ref0（ベースウェアの版）。
///
/// 出所: `crates/areka-ghost/src/config.rs:33` が `env!("CARGO_PKG_VERSION")` を渡す。同じ
/// ワークスペース版番号（`Cargo.toml:8` の `version = "0.0.1"` を全クレートが `version.workspace`
/// で共有する）ゆえ、本クレートで同じマクロを展開した値と一致する。逐語の数字を書き写すと
/// 版を上げたときに黙って陳腐化するため、**同じ源**を展開して固定する。
pub(super) const BASEWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `basewareversion` の Ref1（ベースウェアの名）。
///
/// 出所: `crates/areka-kanade/src/msg.rs:307`（`KanadeConfig::new` の既定 `"areka"`）。
pub(super) const BASEWARE_NAME: &str = "areka";

/// 毎秒の変化通知の Ref0（注入時刻を時へ割った値）。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:173`（`now.0 / 3_600_000`）。
/// [`LAP_INJECT_LIMIT_MS`] が 1 時間に満たないため走行を通じて一定である。
pub(super) const SECOND_CHANGE_HOURS: &str = "0";

/// 毎秒の変化通知の Ref1（見切れ）の M1 固定値。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:39`（`REF1_OFFSCREEN_M1`）。
pub(super) const SECOND_CHANGE_OFFSCREEN: &str = "0";

/// 毎秒の変化通知の Ref2（重なり）の M1 固定値。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:42`（`REF2_OVERLAP_M1`）。
pub(super) const SECOND_CHANGE_OVERLAP: &str = "0";

/// 毎秒の変化通知の Ref3（会話が**始められる**とき）。照会（GET）と対になる。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:172-174`（`talk_playable` が真なら `"1"`）。
pub(super) const SECOND_CHANGE_PLAYABLE: &str = "1";

/// 毎秒の変化通知の Ref3（会話中）。片道（NOTIFY）と対になる。
///
/// 出所: 同上（`talk_playable` が偽なら `"0"`・`crates/areka-kanade/src/schedule/events.rs:188-194`
/// が片道側を構成する）。
pub(super) const SECOND_CHANGE_BUSY: &str = "0";

/// マウス系 Ref2（移動ではホイール回転量・二重クリックでは正典が常に "0" と定める）。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:48`（`REF2_WHEEL_M1`）と同 `:302`
/// （二重クリック側の直値 `"0"`）。M1 ではどちらも `"0"` で一致する。
pub(super) const MOUSE_REF2: &str = "0";

/// マウス系 Ref6（入力デバイス種）の M1 固定値。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:53`（`REF6_DEVICE_MOUSE`）。
pub(super) const MOUSE_DEVICE: &str = "mouse";

/// 移動の Ref5（押下ボタン）。移動はボタン押下を伴わないため常に `"0"`。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:262`（直値 `"0"`）。
pub(super) const MOUSE_MOVE_BUTTON: &str = "0";

/// 二重クリックの Ref5（押下ボタン・左）。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:293-296`（左 `"0"`／右 `"1"`）。
/// 本走行は左の二重クリックでメニューを開く。
pub(super) const MENU_CLICK_BUTTON: &str = "0";

/// `OnClose` の Ref0（終了の由来）。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:200-206` が `reason.as_ref_str()` を載せ、
/// `crates/areka-kanade/src/msg.rs:40` が `CloseReason::User` を `"user"` と綴る。
/// 正典の Ref1／Ref2（スコープ番号）は単一スコープの M1 では省略される（同 `events.rs:199`）。
pub(super) const CLOSE_REASON: &str = "user";

// ===========================================================================
// 撫で・二重クリックの注入値（当たり領域は実 fixture の逐語）
// ===========================================================================

/// マウス入力 1 点ぶんの注入値（座標・話者・当たり領域）。
///
/// 座標は**縮約後のサーフェス px**（作者定義の合成座標系）であり、当たり領域が解決された空間と
/// 同一である（`crates/areka-kanade/src/schedule/events.rs:238-246` の「座標空間」節）。
/// kanade は当たり領域名を意味解釈せず不透明に転写する（同 `:235-236`）。
pub(super) struct MouseProbe {
    /// Ref0（ローカル x 座標）。
    pub(super) x: i64,
    /// Ref1（ローカル y 座標）。
    pub(super) y: i64,
    /// Ref3（話者＝対象スコープ・本体 0／相方 1）。
    pub(super) scope: u32,
    /// Ref4（当たり領域の識別子）。
    pub(super) region: &'static str,
}

/// 撫で（本体側）の注入値。実 fixture の当たり判定矩形の中心を採る。
///
/// 出所: `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt:23`
/// （`collision0,93,62,271,130,Head`＝本体 `\0` 側）。中心は (182, 96)。
pub(super) const STROKE_SAKURA: MouseProbe = MouseProbe {
    x: 182,
    y: 96,
    scope: 0,
    region: "Head",
};

/// 撫で（相方側）の注入値。
///
/// 出所: 同 `surfaces.txt:415-418`（`surface.append10,2100-2110,2200-2210` の
/// `collision1,82,163,140,186,Bust`＝相方 `\1` 側）。中心は (111, 174)。
/// 本体側と当たり領域名を変えてあるのは、2 件の記録が話者だけでなく領域でも区別できることを
/// 期待列の側で見えるようにするためである（design 適合検証項目表 項目 6 の 4 か所のうち 2 か所）。
pub(super) const STROKE_KERO: MouseProbe = MouseProbe {
    x: 111,
    y: 174,
    scope: 1,
    region: "Bust",
};

/// メニューを開く二重クリックの注入値（本体側・胸）。
///
/// 出所: 同 `surfaces.txt:24`（`collision1,133,270,229,326,Bust`＝本体 `\0` 側）。中心は (181, 298)。
/// 撫で（本体・頭）と座標も領域も違えてあるので、2 つの記録が取り違えられない。
pub(super) const MENU_CLICK: MouseProbe = MouseProbe {
    x: 181,
    y: 298,
    scope: 0,
    region: "Bust",
};

// ===========================================================================
// 選択肢（実物 menu.pasta の形をそのまま写す）
// ===========================================================================

/// メインメニューの選択肢 ID（おしゃべり頻度）。
///
/// 出所: `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic/menu.pasta:15`。
pub(super) const CHOICE_TALK_INTERVAL_MENU: &str = "Onおしゃべり頻度メニュー";
/// メインメニューの選択肢 ID（エモの位置調整）。出所: 同 `menu.pasta:15`。
pub(super) const CHOICE_MOVE_MENU: &str = "Onエモの位置調整メニュー";
/// メインメニューの選択肢 ID（閉じる）。出所: 同 `menu.pasta:15`。
pub(super) const CHOICE_MENU_CLOSE: &str = "Onメニュー閉じる";
/// サブメニューの「もどる」の選択肢 ID。出所: 同 `menu.pasta:33`／`:62`。
pub(super) const CHOICE_MAIN_MENU: &str = "Onメインメニュー";
/// 位置調整サブメニューの「調整」の選択肢 ID。出所: 同 `menu.pasta:62`。
pub(super) const CHOICE_MOVE_APPLY: &str = "Onエモの位置調整選択";

/// 選択肢の表示ラベル（おしゃべり頻度）。出所: 同 `menu.pasta:15`。
pub(super) const LABEL_TALK_INTERVAL_MENU: &str = "おしゃべり頻度";
/// 選択肢の表示ラベル（もどる）。出所: 同 `menu.pasta:33`／`:62`。
pub(super) const LABEL_MAIN_MENU: &str = "もどる";
/// 選択肢の表示ラベル（エモの位置調整）。出所: 同 `menu.pasta:15`。
pub(super) const LABEL_MOVE_MENU: &str = "エモの位置調整";
/// 選択肢の表示ラベル（調整）。出所: 同 `menu.pasta:62`。
pub(super) const LABEL_MOVE_APPLY: &str = "調整";

// ===========================================================================
// 台本（応答表）
// ===========================================================================

/// 利用者名の照会に返す値。展開されたこの値が表示に載り、生の記法は漏れない（R3.10）。
///
/// 起動系列の利用者名の先取り照会（`crates/areka-kanade/src/schedule/boot.rs:64-66`）の応答が
/// プロパティ機構へ着地し、台本中の `%username` がこの値へ展開される
/// （`crates/areka-sakura/src/sysvar.rs:65-68`）。台本が値を与えるので、値なしの既定
/// （同 `:45` の `DEFAULT_USERNAME`）へは落ちない。
pub(super) const USERNAME: &str = "アヒル";

/// 起動挨拶（`OnBoot` の応答）。**生の `%username` を含む**のが要点である（R3.10）。
///
/// 記法がそのまま表示へ漏れれば表示指令の列で捕まり、展開されていれば
/// [`USERNAME`] が載る。どちらであるかを台本の側で先に決めておくために、生の記法を逐語で書く。
pub(super) const BOOT_TALK: &str = r"\0\s[0]%username、おはよう。\e";

/// 自発会話（会話可のときの毎秒の変化通知の応答）。
pub(super) const IDLE_TALK: &str = r"\0\s[0]ひまやなあ。\e";

/// 撫で（本体側）の応答。
pub(super) const STROKE_SAKURA_TALK: &str = r"\0\s[0]くすぐったいって。\e";

/// 撫で（相方側）の応答。
pub(super) const STROKE_KERO_TALK: &str = r"\1\s[10]……なに。\e";

/// メインメニュー（二重クリックの応答）。実物 `menu.pasta:15` の形をそのまま写す
/// ——選択肢 3 つ・改行 1 つ・`\_l` による字下げ。
pub(super) const MAIN_MENU_TALK: &str = concat!(
    r"\0\s[0]なんでっか？",
    r"\n\q[おしゃべり頻度,Onおしゃべり頻度メニュー]",
    r"\n\q[エモの位置調整,Onエモの位置調整メニュー]",
    r"\_l[5em,2lh]\q[閉じる,Onメニュー閉じる]",
    r"\e",
);

/// おしゃべり頻度サブメニュー（`Onおしゃべり頻度メニュー` の応答）。
/// 実物 `menu.pasta:33` の形（選択肢 3 つ＋「もどる」）を写す。
pub(super) const TALK_INTERVAL_MENU_TALK: &str = concat!(
    r"\0\s[0]どのくらいがええ？",
    r"\n\q[しゃべくり,Onしゃべくりおしゃべり]",
    r"\n\q[ほどよく,Onほどよくおしゃべり]",
    r"\n\q[たまーに,Onたまーにおしゃべり]",
    r"\_l[5em,2lh]\q[もどる,Onメインメニュー]",
    r"\e",
);

/// 位置調整サブメニュー（`Onエモの位置調整メニュー` の応答）。
/// 実物 `menu.pasta:62` の形（選択肢 1 つ＋「もどる」）を写す。
pub(super) const MOVE_MENU_TALK: &str = concat!(
    r"\0\s[0]エモの立ち位置を変えるで。",
    r"\n\q[調整,Onエモの位置調整選択]",
    r"\_l[5em,2lh]\q[もどる,Onメインメニュー]",
    r"\e",
);

/// 位置調整の実施（`Onエモの位置調整選択` の応答）。
///
/// 移動量は実物 `menu.pasta:65` の逐語（`\1\![move,-353,,,0,base,base]`）をそのまま用いる。
/// 既存の決定論テストが同じ逐語で着地を固定している（`spine_move_cue_tests.rs:89`）。
pub(super) const MOVE_APPLY_TALK: &str = r"\1\![move,-353,,,0,base,base]\e";

/// 終了挨拶（`OnClose` の応答）。**`\-`（終了指令）で終わることが要点**である。
///
/// `\e` で終わると再生完了は「終了拒否」として扱われ、運行は定常運転へ戻って解放が起きない
/// （`crates/areka-kanade/src/schedule/close.rs:15-17`）。実物も終了パターンの末尾で
/// ゴースト終了を出す（`fixtures/emo2/ghost/master/dic/boot.pasta:95`）。
pub(super) const CLOSE_TALK: &str = r"\0\s[0]またね。\-";

/// 一周走行の台本受け口を組む（応答の待ち行列を id ごとに積む）。
///
/// 既存のビルダー（`spine.rs:145` の照会・`:158` の片道・`:167` の解放）だけを使う
/// ——新しい仕組みを発明しない（R2.6）。応答は id ごとの待ち行列で、呼出のたびに先頭から
/// 1 件消費される。**行列が尽きるとその場で受け口が落ちる**（`spine.rs:245`／`:267`／`:277`）
/// ため、本数の決まらない毎秒の変化通知には [`SECOND_CHANGE_RESPONSES`] 本を積む。
///
/// 利用者名の照会は明示的に台本化する。明示しなければビルダーの既定（`spine.rs:186-188` の
/// 値なし応答）が補われ、`%username` が既定名へ落ちて R3.10 の展開を確かめられなくなる。
pub(super) fn lap_backend() -> (ScriptedShioriBackend, ScriptedShioriHandle) {
    let mut builder = ScriptedShioriBackend::builder()
        // ── 起動系列（design D2 の起動 1〜5・順序は R3.1） ──
        .notify("OnInitialize", Ok(()))
        .get("username", Ok(Some(USERNAME.to_string())))
        // 初回起動の照会は応答なし（204）で通常起動へ落とす——既存 spine の標準台本と同形。
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(BOOT_TALK.to_string())))
        .notify("basewareversion", Ok(()))
        // ── 撫で（本体・相方の 2 件・design D1 の撫で段） ──
        .get("OnMouseMove", Ok(Some(STROKE_SAKURA_TALK.to_string())))
        .get("OnMouseMove", Ok(Some(STROKE_KERO_TALK.to_string())))
        // ── メニュー（二重クリック 1 件） ──
        .get("OnMouseDoubleClick", Ok(Some(MAIN_MENU_TALK.to_string())))
        // ── 選択確定・サブメニューと戻り・位置調整（いずれも選択肢 ID と同名の照会） ──
        .get(
            CHOICE_TALK_INTERVAL_MENU,
            Ok(Some(TALK_INTERVAL_MENU_TALK.to_string())),
        )
        .get(CHOICE_MAIN_MENU, Ok(Some(MAIN_MENU_TALK.to_string())))
        .get(CHOICE_MOVE_MENU, Ok(Some(MOVE_MENU_TALK.to_string())))
        .get(CHOICE_MOVE_APPLY, Ok(Some(MOVE_APPLY_TALK.to_string())))
        // ── 終了握手（照会 → 終了挨拶 → 解放） ──
        .get("OnClose", Ok(Some(CLOSE_TALK.to_string())))
        .unload(Ok(ExitKind::Clean));

    // ── 毎秒の変化通知（照会側・会話可） ──
    // 先頭の 1 件だけが自発会話を返し、以後は応答なし（204）で新しい会話を始めない。
    // 先頭が自発会話になるのは、会話が始められるとき（照会）にしかこの行列を消費しないためである。
    builder = builder.get("OnSecondChange", Ok(Some(IDLE_TALK.to_string())));
    for _ in 1..SECOND_CHANGE_RESPONSES {
        builder = builder.get("OnSecondChange", Ok(None));
    }
    // ── 毎秒の変化通知（片道側・会話中） ──
    // 片道は応答スクリプトを運べない型であり、会話中の割り込みは構造的に起きない。
    for _ in 0..SECOND_CHANGE_RESPONSES {
        builder = builder.notify("OnSecondChange", Ok(()));
    }

    builder.build()
}

// ===========================================================================
// 送出の期待列（呼出の別・id・参照列の 3 要素・design D2 の表の逐語）
// ===========================================================================

/// 照会（GET）1 件ぶんの期待。
fn get(id: &str, references: &[&str]) -> RecordedCall {
    RecordedCall::Get {
        id: id.to_string(),
        references: references.iter().map(|r| r.to_string()).collect(),
    }
}

/// 片道（NOTIFY）1 件ぶんの期待。
fn notify(id: &str, references: &[&str]) -> RecordedCall {
    RecordedCall::Notify {
        id: id.to_string(),
        references: references.iter().map(|r| r.to_string()).collect(),
    }
}

/// マウス系の参照列 7 本を組む（移動と二重クリックで**同一の並び**である）。
///
/// 出所: `crates/areka-kanade/src/schedule/events.rs:256-264`（移動）と `:299-307`（二重クリック）。
/// 違うのは Ref5 だけで、移動は常に `"0"`、二重クリックは押下ボタンを綴る。
/// 当たり領域が無いとき（`None`）は空文字で埋めて**位置を保つ**——選択関連の付随参照列とは
/// 逆の規約である（同 `:316-323`）。本走行は常に領域ありで注入するのでその分岐へは入らない。
fn mouse_references(probe: &MouseProbe, button: &str) -> Vec<String> {
    vec![
        probe.x.to_string(),
        probe.y.to_string(),
        MOUSE_REF2.to_string(),
        probe.scope.to_string(),
        probe.region.to_string(),
        button.to_string(),
        MOUSE_DEVICE.to_string(),
    ]
}

/// 一周で送られる呼出の期待列（`ScriptedShioriHandle::non_status_calls()` と等値で突き合わせる）。
///
/// 死活の問い合わせは取り出し口の側で除かれている（`spine.rs:302-311`）ため、本列には現れない。
/// 各要素の直上に、その段と参照列の出所を注記してある。
pub(super) fn expected_calls() -> Vec<RecordedCall> {
    vec![
        // ── 起動 1: 初期化の通知。M1 にリロードの概念が無いので参照は無い。
        //    出所: events.rs:110-116。
        notify("OnInitialize", &[]),
        // ── 起動 2: 利用者名の先取り照会。参照は無い。
        //    出所: resources.rs:55-60（`references: Vec::new()`）・発行点は boot.rs:64-66。
        get("username", &[]),
        // ── 起動 3: 初回起動の照会。Ref0＝消滅回数。
        //    出所: events.rs:124-129。
        get("OnFirstBoot", &[FIRST_BOOT_VANISH_COUNT]),
        // ── 起動 4: 通常起動の照会。Ref0＝シェル名（Ref6/7 は M1 では省略）。
        //    出所: events.rs:135-141。
        get("OnBoot", &[SHELL_NAME]),
        // ── 起動 5: ベースウェア版の通知。Ref0＝版・Ref1＝名（Ref2 は省略）。
        //    出所: events.rs:146-155。
        notify("basewareversion", &[BASEWARE_VERSION, BASEWARE_NAME]),
        // ── 自発会話（会話可）: 照会で送られ Ref3 が "1"。
        //    出所: events.rs:171-195（会話が始められるときは照会・Ref3="1"）。
        get(
            "OnSecondChange",
            &[
                SECOND_CHANGE_HOURS,
                SECOND_CHANGE_OFFSCREEN,
                SECOND_CHANGE_OVERLAP,
                SECOND_CHANGE_PLAYABLE,
            ],
        ),
        // ── 会話中の抑止: 同じ参照の並びのまま片道になり Ref3 が "0" になる。
        //    片道は応答スクリプトを運べない型ゆえ、新しい会話は構造的に始まらない。
        //    出所: events.rs:169-170・:188-194。
        notify(
            "OnSecondChange",
            &[
                SECOND_CHANGE_HOURS,
                SECOND_CHANGE_OFFSCREEN,
                SECOND_CHANGE_OVERLAP,
                SECOND_CHANGE_BUSY,
            ],
        ),
        // ── 撫で（本体側）: 参照 7 本・Ref3＝話者・Ref4＝当たり領域・Ref5 は常に "0"。
        RecordedCall::Get {
            id: "OnMouseMove".to_string(),
            references: mouse_references(&STROKE_SAKURA, MOUSE_MOVE_BUTTON),
        },
        // ── 撫で（相方側）: 話者と当たり領域だけが本体側と異なる。
        RecordedCall::Get {
            id: "OnMouseMove".to_string(),
            references: mouse_references(&STROKE_KERO, MOUSE_MOVE_BUTTON),
        },
        // ── メニュー: 同じ 7 本。Ref2 は "0"・Ref5 は押下ボタン（左＝"0"）。
        RecordedCall::Get {
            id: "OnMouseDoubleClick".to_string(),
            references: mouse_references(&MENU_CLICK, MENU_CLICK_BUTTON),
        },
        // ── 選択確定: 選択肢 ID そのものが照会の id になる。付随参照列は空ゆえ参照は 1 本も無い
        //    （空文字で埋めない＝マウス系とは逆の規約・events.rs:385-396・:316-323）。
        //    正典の選択確定イベントは**先行しない**（choice.rs:58-66 が `On` 始まりを 1 段へ写す）。
        get(CHOICE_TALK_INTERVAL_MENU, &[]),
        // ── サブメニューと戻り: 「もどる」→ メインメニュー、続けて「エモの位置調整」。
        get(CHOICE_MAIN_MENU, &[]),
        get(CHOICE_MOVE_MENU, &[]),
        // ── 位置調整: 「調整」を確定すると応答が移動の指令を運ぶ。
        get(CHOICE_MOVE_APPLY, &[]),
        // ── 終了: 照会で送られ Ref0 は由来のみ（正典の Ref1／Ref2 は M1 では省略される）。
        //    出所: events.rs:197-206。
        get("OnClose", &[CLOSE_REASON]),
        // ── 解放: ちょうど 1 度だけ（R3.9）。列の等値照合が件数もそのまま固定する。
        RecordedCall::Unload,
    ]
}

// ===========================================================================
// 〈段名・表示指令〉の期待列（design D3「判定の本体」・task 3.2）
// ===========================================================================

/// 一周で届く表示指令の期待列（段名つき・`spine_conformance_judge.rs` が等値で突き合わせる）。
///
/// # なぜ 2 行しかないのか（実測・R12.5 の記録）
///
/// 一周で表示指令を生む段は **2 つだけ**である。台本の応答は `STROKE_KERO_TALK`
/// （＝相方の `\1\s[10]`）を除いてすべて `\0\s[0]` を指し、その面は起動挨拶が既に表示している
/// ——面が変わらない指定は表示指令を生まない。実物 `menu.pasta` の応答がそう書かれているため
/// であり、台本の都合ではない。ゆえに起動・自発会話・会話中の抑止・メニュー・選択確定・
/// サブメニューと戻り・位置調整・終了の 8 段は **0 件**である。
///
/// design D3 はこの列の完全一致を「判定の本体」と書くが、**2 行では R2.4 の判定は設計が想定する
/// より実質的に弱い**。段の順序と内容を機械で証明したと言えるのは、交信の列（16 行）と進行状態の
/// 列（15 行）を合わせた 3 列の等値であって、この列だけではない。表示経路の被覆は既存の兄弟テスト
/// （`spine_display_tests.rs`・`spine_seriko_loop_tests.rs`・`spine_talk_close_tests.rs`）が正本
/// として持つ。**この弱さを埋めるために架空の表示指令を足すことはしない**——期待列は実装が実際に
/// 出すものの写しでなければ、退行の検出器として働かない。
pub(super) fn expected_display() -> Vec<(&'static str, DisplayProjection)> {
    vec![
        // ── 装着: 起動挨拶 `\0\s[0]` が本体（scope0）のキャラ窓へ面 0 を出す。
        //    表示対象の採番は `target_map.rs` の `shell_target`（キャラ窓＝2*scope）。
        (
            "装着",
            DisplayProjection::Show {
                target: shell_target(0).0,
                surface: 0,
            },
        ),
        // ── 撫で: 相方側の応答 [`STROKE_KERO_TALK`]（`\1\s[10]`）が相方（scope1）のキャラ窓へ
        //    面 10 を出す。本体側の応答 [`STROKE_SAKURA_TALK`] は `\0\s[0]` ＝表示中の面ゆえ 0 件。
        (
            "撫で",
            DisplayProjection::Show {
                target: shell_target(1).0,
                surface: 10,
            },
        ),
    ]
}

// ===========================================================================
// 進行状態の期待列（design D3「進行状態の台帳」・R3.8・task 3.2）
// ===========================================================================

/// 進行状態の wire 値（会話中のみ）。
///
/// 出所: `crates/areka-kanade/src/status.rs:58-61`（`ExecutionState::Talking` の綴り）と
/// 同 `:190-199`（複数あるときはカンマ連結・空集合はヘッダ行なし）。
pub(super) const STATUS_TALKING: &str = "talking";

/// 進行状態の wire 値（会話中かつ選択待ち）。
///
/// 選択待ちの間も会話の枠は占有されたままなので、`choosing` は単独では現れず必ず `talking` と
/// 複合になる（`crates/areka-kanade/src/status.rs:211-216`）。連結順は正典順（`talking` が先）。
pub(super) const STATUS_TALKING_CHOOSING: &str = "talking,choosing";

/// 進行状態の記録 1 件ぶんの期待。`None`＝ヘッダ行を出さない（記録の欠落ではない）。
fn status(id: &str, status: Option<&str>) -> RecordedStatus {
    RecordedStatus {
        id: id.to_string(),
        status: status.map(str::to_string),
    }
}

/// 一周で送られる呼出に載る進行状態の期待列（`ScriptedShioriHandle::status_calls()` と等値）。
///
/// 交信の列から**解放を除いた**並びと 1 対 1 に対応する（解放は進行状態を運ばない型である
/// ＝`spine.rs` の `unload` は記録の第 2 系統へ書かない）。
///
/// # この列だけが選択待ちを見せる（R3.8）
///
/// 会話中は毎秒の変化通知の別（照会か片道か）と Ref3 でも読める。**しかし選択待ちは Ref3 では
/// 会話中と区別できない**——Ref3 の源は `talk_active` だけだからである
/// （`crates/areka-kanade/src/schedule/events.rs:171-180`）。選択起源の 4 呼出が
/// [`STATUS_TALKING_CHOOSING`] を運ぶことは、この列でしか固定できない。
pub(super) fn expected_statuses() -> Vec<RecordedStatus> {
    vec![
        // ── 起動系列 1〜4: 運行は起動相にあり、会話も選択待ちも立たない＝ヘッダ行なし。
        status("OnInitialize", None),
        status("username", None),
        status("OnFirstBoot", None),
        status("OnBoot", None),
        // ── 起動系列 5: **起動挨拶を先に起動してから**送るので会話中である。
        //    `OnBoot` が応答を返した経路では、フェーズを `BootVersion{talk: Some(_)}` へ確定して
        //    から送出時点のスナップショットを撮る（`crates/areka-kanade/src/schedule/boot.rs:226-228`
        //    ・同 `:275-280`）。204 で返る経路なら非アクティブになるが、本走行の台本は
        //    [`BOOT_TALK`] を返すので会話中側を通る。
        status("basewareversion", Some(STATUS_TALKING)),
        // ── 自発会話（会話可）: 会話が始められる＝会話中でない。
        status("OnSecondChange", None),
        // ── 会話中の抑止: 直前の応答が再生中＝会話中。
        status("OnSecondChange", Some(STATUS_TALKING)),
        // ── 撫で 2 件: 自発会話の再生が続いたまま入力が届く。
        status("OnMouseMove", Some(STATUS_TALKING)),
        status("OnMouseMove", Some(STATUS_TALKING)),
        // ── メニュー: 撫での応答（相方側）がまだ再生中のうちに二重クリックが届く。
        //    撫で段は会話の**起動**までで完了し、以後は注入を止めて観測だけを続けるので、
        //    再生時刻は 1 ミリ秒も進まないまま次段へ渡る（会話中にメニューが開くこと自体が
        //    R1.5 の「結合と一周でしか現れない事象」である）。
        status("OnMouseDoubleClick", Some(STATUS_TALKING)),
        // ── 選択起源の 4 呼出: 選択待ちは会話の枠を占有したまま成立する（複合値）。
        //    出所: `crates/areka-kanade/src/schedule/steady.rs:292-294`（選択確定の発火は
        //    `snapshot_with_choice(true)` を明示的に渡す）。
        status(CHOICE_TALK_INTERVAL_MENU, Some(STATUS_TALKING_CHOOSING)),
        status(CHOICE_MAIN_MENU, Some(STATUS_TALKING_CHOOSING)),
        status(CHOICE_MOVE_MENU, Some(STATUS_TALKING_CHOOSING)),
        status(CHOICE_MOVE_APPLY, Some(STATUS_TALKING_CHOOSING)),
        // ── 終了: 終了系列は運行を `Unloading` へ移してから発火する＝全状態が非アクティブ。
        //    出所: `crates/areka-kanade/src/schedule/mod.rs:484-492`。
        status("OnClose", None),
    ]
}
