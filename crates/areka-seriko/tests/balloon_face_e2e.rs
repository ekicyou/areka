//! バルーン面切替の決定論的エンドツーエンド観測（spec: areka-P0-balloon-face-cue
//! 要件 5.1/5.2/5.3/5.4）。
//!
//! fixture のさくらスクリプトを **直接入力**し、`parse → compile → cue → seriko → 表示指令発行`
//! までの貫通経路を、注入した `SakuraMsg::Tick` のみで（sleep/polling を一切用いず）決定論的に
//! 観測する統合テスト。公開 API のみで組む。
//!
//! 同期は `common::run_core` のとおり（sleep/polling なし・要件 5.3）。
//!
//! # 経路の要所
//!
//! - `BalloonSurface`（`\b`）は `cue_target_of` が `Some(CueTarget::Shell)` へ分類する＝
//!   表示系（seriko）が action する（テキスト状態機械へは行かない・要件 3.2）。broadcast ゆえ
//!   全 cue は両 sink へ届くが、text 系は本 E2E の観測対象外——破棄専用の [`NullTextSink`] で足りる。
//! - 解決テーブルは空 alias 表（`BTreeMap::new()`）で足りる。数値 key（`"0"`/`"2"`/`"-1"`）は
//!   alias を引かず数値枝で解決できる（`resolve` / `resolve_balloon_key` の数値枝）。

use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::ActorKey;
use areka_seriko::{BindResolver, DisplayCommand};
use std::time::Duration;

mod common;
use common::*;

/// 1 シナリオを貫通駆動し、seriko が発行した表示指令列（送信順）を返す（同期は [`run_core`] のとおり）。
///
/// 空の別名の表でも数値の鍵は数値の枝で解ける。静的な着せ替えは空（バルーンに着せ替えは無く、
/// ここで見るシェルの Show も binds 空で足りる）。
fn run_scenario(script: &str, ticks: &[f64]) -> Vec<DisplayCommand> {
    run_core(
        BindSet::from_ids([]),
        BindResolver::empty(),
        script,
        ticks,
        Box::new(NullTextSink),
    )
}

/// シナリオ1（要件 5.1）: `\b[ID]` を含む fixture script 直入力で「面切替指令（ID 指定）」を
/// mock 表示 sink 上に観測する。
///
/// `\b[2]\e` → seriko が `ShowBalloon{scope:"0", surface_id:2}` をちょうど 1 件発行する
/// （既定 scope は `\p[n]` 未指定ゆえ "0"・compile が転写）。
#[test]
fn balloon_show_is_observed_end_to_end() {
    let records = run_scenario(r"\b[2]\e", &[0.0]);
    assert_eq!(
        records,
        vec![DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "\\b[2] は ShowBalloon{{scope:0, surface_id:2}} を 1 件観測させる（5.1）"
    );
}

/// シナリオ2（要件 5.2）: `\b[-1]`（非表示センチネル）を含む fixture script 直入力で
/// 「非表示指令」を mock 表示 sink 上に観測する。`\w1`（=50ms）を跨ぐ Tick 時刻で駆動する。
///
/// `\b[2]\w1\b[-1]\e` → Tick(0.0) で `ShowBalloon{2}`、Tick(0.05) で `HideBalloon` を順に観測。
/// `\w1` の時刻は compile と同一の `Duration::from_millis(50).as_secs_f64()` で導出し、10 進
/// 直書き（0.05）の f64 表現誤差で境界包含判定（`at <= tick`）を誤らないようにする。
#[test]
fn balloon_hide_is_observed_end_to_end() {
    // `\w1`＝1×50ms。compile 側の累積と同一計算で導出する（リテラル直書きの誤差回避）。
    let at_hide = Duration::from_millis(50).as_secs_f64();
    let records = run_scenario(r"\b[2]\w1\b[-1]\e", &[0.0, at_hide]);
    assert_eq!(
        records,
        vec![
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("0"),
                surface_id: 2,
                pattern: PatternState::default(),
            },
            DisplayCommand::HideBalloon {
                scope: ActorKey::from("0"),
            },
        ],
        "\\b[2]→\\b[-1] は ShowBalloon(2)→HideBalloon の順に観測させる（5.2）"
    );
}

/// シナリオ3（要件 1.2 の E2E 面）: レガシー裸形 `\bN` がブラケット形 `\b[N]` と**同一**の
/// 表示指令を観測させる。
///
/// `\b2\e` → `ShowBalloon{2}`。ブラケット形 `\b[2]\e`（シナリオ1）と同じ字面の観測列に固め、
/// 裸形が本文数字を漏らさず（`\b2` の `2` は面数字として消費される）同一経路へ乗ることを固定する。
#[test]
fn bare_balloon_matches_bracket_form_end_to_end() {
    let bare = run_scenario(r"\b2\e", &[0.0]);
    assert_eq!(
        bare,
        vec![DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "裸形 \\b2 は ShowBalloon{{2}} を観測させる（1.2 の E2E 面）"
    );
}

/// シナリオ4（要件 4.3）: 同一面への冪等な再指定は表示指令を再発行しない。
///
/// `\b[2]\b[2]\e`（両 cue とも at=0.0）→ Tick(0.0) で両 cue が発火するが、2 件目は per-scope
/// 状態不変ゆえ発行が抑止され、`ShowBalloon{2}` はちょうど 1 件のみ観測される（per-scope の冪等ガード）。
#[test]
fn same_balloon_twice_emits_once_end_to_end() {
    let records = run_scenario(r"\b[2]\b[2]\e", &[0.0]);
    assert_eq!(
        records,
        vec![DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "同一面 \\b[2] の冪等な再指定は ShowBalloon を 1 件のみ発行する（4.3）"
    );
}

/// シナリオ5（要件 4.6）: シェル面切替とバルーン面切替の混在で、双方が独立に記録される。
///
/// `\s[0]\b[2]\e`（両 cue とも at=0.0・記述順 FIFO）→ Tick(0.0) で `\s[0]`（シェル面＝seriko の
/// `Emote` 経路・数値 "0"→Show(0)）と `\b[2]`（バルーン面＝`BalloonSurface` 早期分岐）が順に
/// 発火し、`Show{scope:"0", surface_id:0, binds:空}` と `ShowBalloon{scope:"0", surface_id:2}`
/// が互いに干渉せず送信順どおりに記録される（別 map・別発行）。
#[test]
fn shell_and_balloon_mixed_recorded_independently_end_to_end() {
    let records = run_scenario(r"\s[0]\b[2]\e", &[0.0]);
    assert_eq!(
        records,
        vec![
            DisplayCommand::Show {
                scope: ActorKey::from("0"),
                surface_id: 0,
                binds: BindSet::from_ids([]),
                pattern: PatternState::default(),
            },
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("0"),
                surface_id: 2,
                pattern: PatternState::default(),
            },
        ],
        "シェル面 Show(0) とバルーン面 ShowBalloon(2) が独立に記録される（4.6）"
    );
}
