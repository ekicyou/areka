# Brief: background-result-tick-wake

> 起票: 2026-10-17（`areka-P0-user-data-root` の完了時の棚卸で、範囲の外の議題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-user-data-root/tasks.md` の Implementation Notes の「4.2 の写しの答えは起床の旗を立てず、次の回（遅くても心拍 30 コマ）で拾う…背景の結果の全般に旗を立てるかは範囲外の議題」と、同タスク 4.2 の点検。区分 C（バグでない持ち越し）。行数とファイルの位置は起票のとき（`user-data-root` のブランチ・2026-10-17）のもの＝着手時に引き直す。

## Problem

- **利用者**: tick の門を入れた形（`AREKA_TICK_GATE=1`）で、ゴーストの切り替えの写しが終わってから新しいゴーストが立ち上がるまで、最大で心拍 1 回ぶん（30 コマ＝60Hz で約 0.5 秒）遅れうる。切り替えの資産の組み立て・kanade からの切り替えの知らせなど、ほかの背景の結果も同じ。
- **開発者**: 背景のスレッドが出した結果が、起床の旗（`tick_wake::mark`）を立てない。門は「旗が無い」と正しく判断して省略し、結果は次に誰かが旗を立てた回か心拍の回で拾われる。設計の大原則「時刻は正確に扱う」（待ち時間は丸めない）から外れる。

## Current State

- tick の門は既定で切（`crates/wintf/src/ecs/world/tick_gate.rs`・既定 OFF・`AREKA_TICK_GATE=1|0` で上書き）。**今の既定の形では毎回回るので遅れは出ない**。遅れが出るのは門を入れた形（A/B の測り・`tick-gate-adoption` が既定を入れると決めた後）だけ。
- 心拍は `TICK_HEARTBEAT_FRAMES = 30`（同 53 行）。起動から 600 コマは無条件で回る（`TICK_GATE_WARMUP_FRAMES`）。
- 旗を立てずに結果を渡している背景の結果（起票のときに見えたもの・着手時に洗い出す）:
  - `crates/areka/src/emo2_boot/switch_copy.rs`（115 行・`user-data-root` の切り替えの写し・44 行の `std::thread::spawn` → `tx.send`）。
  - `crates/areka/src/emo2_boot/switch_assets.rs`（253 行・切り替えの資産の組み立て・141 行の `std::thread::Builder` → 148 行の `tx.send`）。
  - `crates/areka/src/emo2_boot/change_cue.rs`（kanade からの切り替えの知らせ）。
  - ほかの候補: `install/desk.rs`・`install/fetch_url.rs`・`roots/desk.rs`・`mcp/dump_surface.rs`・`emo2_boot/spine_wait.rs`・`shiori_host.rs` のスレッド（結果を ECS の刻みで拾うものだけが対象）。
- areka の側で旗を立てる生産者は `crates/areka/src/tick_gate_config_producers_tests.rs` の `AREKA_PRODUCERS`（今 7 行）が字面で見張る。旗を立てるファイルを足したら、この表に行を足さないと赤になる。

## Desired Outcome

- 背景のスレッドが ECS の刻みで拾われる結果を送るとき、送った直後に起床の旗を立てる（または立てない理由を記録する）。どの結果が対象かの一覧を要件で決め、全部をそろえる。
- 門を入れた形で、背景の結果が届いてから拾われるまでの遅れが心拍に頼らない（次の画面更新で拾われる）ことを、決定論テストで固定する。
- 旗を立てる新しいファイルが `AREKA_PRODUCERS` に載り、字面の見張りが抜けを捕まえる。

## Approach

1. 背景の結果の一覧を作る（送る所・拾う所・今の拾われ方＝次の旗か心拍か）。
2. 旗の種類を決める: 今ある旗（`PRESENT`・`REARM` など）を使い回すか、背景の結果の旗を wintf の `tick_wake.rs` に足すか（足すなら wintf の旗の表と名前の一覧も触る）。
3. 送る側で `tick_wake::mark` を呼ぶ形を推す（受け取り口を 1 か所にまとめる形と比べて要件で選ぶ）。
4. 門を入れた形の決定論テスト（偽の背景の結果を送り、次の刻みで拾われる）と、`AREKA_PRODUCERS` の行を足す。

## Scope

- **In**: 背景の結果の一覧・送る側の旗の立て方・（要るなら）旗の種類の追加・`AREKA_PRODUCERS` の行・門を入れた形の決定論テスト。
- **Out**: 門を既定で入れるか（`tick-gate-adoption`）・心拍の回数そのもの・「今」の読み取りの時計（`instant-to-interrupt-ticks`）・背景の処理の中身（写し・組み立て・インストール）。

## Boundary Candidates

- 背景のスレッドの送り口（`tx.send` の直後）と、ECS の刻みの受け取り口（`try_recv`）の境目。
- areka の旗（`crates/areka/src/`）と wintf の旗の表（`crates/wintf/src/ecs/world/tick_wake.rs`）。
- 字面の見張り（`tick_gate_config_producers_tests.rs`）。

## Out of Boundary

- 門の判断の純関数（`tick_gate.rs`）の規則。
- 背景のスレッドの起こし方・終わらせ方。

## Upstream / Downstream

- **Upstream**: ✅ `user-data-root`（2026-10-17 完了・切り替えの写しを足した）。
- **Downstream**: `tick-gate-adoption`（保留・門を既定で入れるかを決める）＝門を入れる前にこの spec が着地していれば、背景の結果の遅れが A/B の測りに混ざらない。

## Existing Spec Touchpoints

- **Extends**: 完了 `user-data-root`（切り替えの写し）・tick の門（`AREKA_PRODUCERS` の見張り）。
- **Adjacent**（同じ時に走らせない相手）:
  - `instant-to-interrupt-ticks`（v0.2.0・据え置き）: `tick_wake.rs` の時計を替える。この spec が旗の種類を `tick_wake.rs` に足す形を選んだら、同じファイル＝直列。
  - `tick-gate-adoption`（v0.2.0・保留）: 門と生産者の見張りを触る・計測を汚すので他と並べない。この spec はその前に置く。
  - `emo2_boot` の結線の列（`switch_assets.rs`）: `extra-character-windows`・`mcp-reload` が同じファイルを触る＝同じウェーブに置かない。
  - `multi-ghost-runtime`（v0.1.0）: 切り替えと箱の結線を作り直す＝`switch_copy.rs`・`switch_assets.rs` の照合が要る。

## Constraints

- 段は**その他**・区分 C・版に縛らない（道具と手入れ）。v0.2.0 の間に挟むのは、`instant-to-interrupt-ticks`・`tick-gate-adoption` が走っていないときで、`tick_wake.rs` に触らない形を選んだときだけ（版の段の「v0.2.0 の間に挟むときの注意」）。
- 「時刻は正確に扱う」: 遅れを心拍で丸めない・拾う回を引き上げる案（心拍を短くする）は出さない。
- 1 フレーム遅らせる解は取らない。
- 決定論テスト網羅は必達。旗はプロセスで 1 組しか無い＝「立っていないはず」は主張しない（見張りの表の頭の注意）。
- 依存は足さない。
- 規模の見込み: S〜M（5〜9。一覧 1・旗の種類 0〜2・送る側 2〜4・テスト 2）。
- 要件定義のモデル: Opus。

## 優先度の区分

C（バグでない持ち越し。今の既定〔門は切〕では遅れは出ず、門を入れる前に片づけておくもの）。

## 2026-10-31 棚卸㉜の再測定（main d374f901）
- **前提の変化**: tick の門 `crates/wintf/src/ecs/world/tick_gate.rs`（222 行・心拍 30 コマは 53 行目）は変わらず、既定は今も切。起床の旗の表 `tick_wake.rs` は 374→375 行（✅ `shell-mouse-cursor` が `WM_MOUSEHOVER` を足し、✅ `stay-on-top-directives` が説明に 1 語）。旗の見張り `crates/areka/src/tick_gate_config_producers_tests.rs` は 279 行・`AREKA_PRODUCERS` は 7→8 行（`stay-on-top-directives` の `emo2_boot/window_state_cue.rs`＝送る側で旗を立てる先例が 1 つ増え、`move_cue.rs`・`talk_lifecycle.rs`・`zorder_cue.rs` と並ぶ）。候補の行数: `emo2_boot/switch_copy.rs` 115→131（✅ `readonly-mode`）・`switch_assets.rs` 253→257（✅ `descript-window-keys`）・`change_cue.rs` 119・`switch_cue.rs` 123・`reload_cue.rs` 106・`crates/areka/src/mcp/mod.rs` 152（64 行目 `drain`）・`crates/areka-mcp/src/server.rs` 207→211（✅ `mcp-strict-errors`）。
- **インストールの窓口（確かめた）**: ✅ `install-before-boot` の着地で `crates/areka/src/install/desk.rs`（592 行）の重なりは解けた。窓口の `drain`（103 行・Input の段）は背景の 2 つから `try_iter` で拾う＝インストールの係のスレッドの問い（`install/worker.rs` 78 行ほか・`ask` 経由）と、ファイルを選ぶスレッドの `send_pick`（`desk.rs` 239 行）。どちらも旗を立てない＝本文の「ほかの候補」の `install/desk.rs` は対象で、`worker.rs` も触る。
- **触るファイル**: 一覧で対象と決めた送る側（`emo2_boot/{switch_copy,switch_assets,change_cue,switch_cue,reload_cue}.rs`・`install/{desk,worker}.rs` のうち）・MCP の受け口（`areka-mcp/src/server.rs` か `areka/src/mcp/mod.rs`）・`tick_gate_config_producers_tests.rs`・旗の種類を足すなら `tick_wake.rs`。
- **規模**: 6〜10 のまま（インストールの窓口は起票の 5〜9 の「ほかの候補」に入っていた）。
- **先に要るもの**: 働きの上では無い（先頭）。v0.1.0 の `shell-scale-linkage` とは送る側のファイルで重ならないが、向こうが `\![get,property]` の受け手を `window_state_cue.rs` の形で足して旗を立てると `AREKA_PRODUCERS`（`tick_gate_config_producers_tests.rs`）に行を足す＝向こうの触るファイルが決まってから照らす。同じウェーブに置けない相手: `multi-ghost-runtime`（切り替えの結線）・`mcp-shiori-query`・`mcp-user-response`（`mcp/mod.rs`）・`check-script-arg-checks`（`change_cue.rs`・`switch_cue.rs`）・`makoto-reload-directives`（`reload_cue.rs`）・`tick_wake.rs` を触るなら `instant-to-interrupt-ticks`（v0.2.0）と直列。
- **机・待ち**: 静かな机・前後の全体テストの比べ・長い負荷の走行・人の待ちは要らない（門を入れた形の決定論のテスト）。依存は足さない＝1 本だけの席は取らない。`areka-test-binary-stability` を測っている間は着地させない（同じテストの実行ファイルの足場に触る見込み）。
- **優先度の区分**: C のまま（バグでない持ち越し・門を入れる前に片づけるもの）。
- **要件定義のモデル**: Opus のまま（今ある旗と、送る側で旗を立てる先例に乗る）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 本文の「ほかの候補」に MCP の受け口と読み直しの知らせ（`emo2_boot/reload_cue.rs`・`reload.rs`）が無い（棚卸㉘で見つけた・本文は直していない）。本文の `AREKA_PRODUCERS`「今 7 行」は 8 行。
