# 是正の対証跡（task 6.3・2026-08-21）

## 0. 本書の位置づけ

本書は **task 6.3「各是正について是正前の失敗と是正後の通過を実行で示す」の成果物**である。要件 7.3
「When 是正を投入したとき, the 回帰テスト shall 是正前のコードで失敗し是正後に通過することを、確定
台帳で確定した各機序について実行で示す」に対し、**是正前の赤と是正後の緑を 1 件ずつ対で並べる**。

- **本書は合否を主張する文書ではない。** 実機の合否は手順書（`signoff-procedure.md`）の様式で
  task 7.3 が出す。本書が言うのは「どの是正が、どの量を、いくつからいくつへ動かしたか」だけである。
- **記録は 2 種類あり、書き分ける。**
  - **【記録の引用】**——是正タスクの実装時（2026-08-20 以前）に採られた赤の記録。出所はコミット本文・
    テストファイルの doc・確定台帳・是正前の基準値であり、**本タスクで実行し直したものではない**。
  - **【本タスクで実行】**——2026-08-21 に現在のコードへ対して走らせた結果。逐語で貼る。
- **是正前のコードは復元していない。** タスク文の明示要件により、過去のコミットへ戻す破壊的な操作
  （`git checkout <sha>`・`git reset`・`git stash`・ワークツリーの切替）は 1 度も行っていない。
  代わりに **⑴ 記録の引用**と **⑵ 現在のコードへのミューテーション**の 2 本立てで示す。
- **ミューテーションが必要なのは、赤を出せるコミットが存在しないからでもある。** 4 件の対テストは
  いずれも**是正と同一のコミットで初めてリポジトリへ入った**（§6 の表）。ゆえに「テストが在って是正が
  無い」木は履歴上に 1 つも無く、過去へ戻っても赤は再現できない。是正を無効化する変異だけが、
  今日のコードに対して同じ赤を作れる。

### 実行環境

| 項目 | 値 |
| --- | --- |
| ブランチ | `claude/areka-p0-dpi-atomicity-7b3efa`（HEAD `6a20f504`・群 5 と 6.1／6.2 が着地済み） |
| コマンド | `cargo test -p areka -- --test-threads=1`（`--bins` なし）／`cargo test -p wintf` |
| 実行日 | 2026-08-21 |

`--test-threads=1` を使うのは、失敗テスト名の帰属を確実に採るためである（`tasks.md` の
`## Implementation Notes`「変異検査の作法」⑶）。

---

## 1. 4 件と確定台帳の突合（要件 7.3 の「各機序」）

要件 7.3 は「確定台帳で確定した**各機序**について」と言う。台帳（`mechanism-ledger.md` §2）は L1〜L9 を
登記しているので、**4 件で尽きているか**を 1 行ずつ当たった。

| 台帳 | 機序 | 台帳の状態 | 是正 | 本書での扱い |
| --- | --- | --- | --- | --- |
| L1 | 一括書込が窓ごとに 1 枚ずつ進む | 確定 | C8 の候補（採否は task 7.1） | **本書の対象外**。是正そのものが未確定ゆえ対テストが存在しない。設計 C8 の候補表が「7.3 の対テスト（是正前赤／後緑）」列を持ち、task 7.2 が採用案について埋める |
| L2 | 経路 A（OS 提案位置の同期書込）が 0 回 | 確定（真の 0） | **不要**（維持すべき状態） | **本書の対象外**。是正が無いので対にならない。0 でなくなったら回帰として task 7.3 が扱う（台帳 §9） |
| **L3** | 作業領域源が起動時固定で追随しない | 確定 | task 5.1・5.2 | **件① 作業領域追随**（§2） |
| **L4** | 連鎖を一度確定したら解き直さない | 確定 | task 5.6 | **件④ 連鎖再解決**（§5） |
| **L5** | 同一 hwnd への 2 指令が合流しない | 確定 | task 5.3 | **件② 合流**（§3） |
| **L6** | Z 書込が先行したときの 2 段書込経路 | 確定（経路の存在） | task 5.4 | **件③ 整合待ち**（§4） |
| L7 | 窓書込 1 回の内訳 | 部分的に確定 | C8 の候補（採否は task 7.1） | **本書の対象外**（L1 と同じ理由） |
| L8 | 積み上げから一括書込までの間 | 部分的に確定 | 前半は `draw-load-parity` へ申し送り／後半は未特定 | **本書の対象外**。本仕様は最適化しない（台帳 §5） |
| L9 | 実機専用の上限の確定値 | 確定 | 判定器へ反映済み | **本書の対象外**。台帳 §1 のとおり L9 は**裁定**であって機序ではない |

**結論: 是正を持つ確定機序は L3・L4・L5・L6 の 4 件で尽きており、本書の 4 件と過不足なく一致する。**
残る確定機序（L1・L7）は是正候補が未採用ゆえ 7.3 の対を作れない状態であり、**引受先は task 7.2**
（設計 C8 候補表の「7.3 の対テスト」列・tasks.md 7.2 の「候補表に記した対テストのうち決定論で測れる
部分を実装し」）である。放置ではない。

設計の「### 是正前失敗・是正後通過（7.3）」が挙げる 4 件（合流・`WorkAreaResnap`・`ChainRealign`・hold）
とも一致する。ただし設計は件①を `WorkAreaResnap`（＝task 5.2 の経路名）1 語で呼んでいるのに対し、
**台帳 L3 の是正は task 5.1 と 5.2 の 2 つ**である。本書は両方を件①の内側に置き、それぞれ別の赤と
別の緑を対にした（§2）。設計が挙げる量「diff −48→0」は task 5.1 側のものである。

---

## 2. 件① 作業領域追随（台帳 L3・task 5.1／5.2）

作業領域源は起動時に 1 度だけ作られ、以後どのフレームでも作り直されなかった。タスクバーの高さは
論理寸で宣言され物理 px では拡大率に比例するので、拡大率を下げると真の作業領域下端は下がる。
起動時の値が焼き付いたままだと、下端吸着のキャラ窓は古い下端へ接地し続ける。

是正は 2 段ある。**5.1** が源を実行時に作り直し、**5.2** が「拡大率は動かず作業領域だけが動いた」
フレーム（`Changed<DPI>` が 1 件も立たない）を拾う。

### 2.1-a 是正前の赤【記録の引用】——task 5.1

| 項目 | 内容 |
| --- | --- |
| 量 | 接地点差（`ground_y − wa_bottom`）が **−48px**（あるべき値 0） |
| 記録の所在 | `crates/areka/src/emo2_boot/frame_work_area_sync_tests.rs:8`／`:11`／`:77-78`（module doc と test doc）。コミット `3a4c6696` 本文「接地点差 −48px → 0」「2 源を潰す変異では 3 本のテストが赤になる＝観測は生きている」 |
| 実機の裏付け | `mechanism-ledger.md:36`（L3 の行）「実機は接地点差 −48px が 192→96 の 6/6」／根拠の詳細は同 §3.3（:81 以降）／`baseline-2026-08-20.md:116`（§3.2 ⑺）「192→96 の 6 遷移すべてで **−48**、96→192 の 6 遷移すべてで 0」 |

### 2.1-b 是正前の赤【記録の引用】——task 5.2

| 項目 | 内容 |
| --- | --- |
| 量 | 接地点 Y が**旧下端 1444** に留まる（あるべき値＝新下端 1492） |
| 記録の所在 | `crates/areka/src/emo2_boot/frame_work_area_resnap_tests.rs:13`（module doc）「是正前は接地点が旧下端に留まって赤くなり」／`:101`（test doc）「接地点は旧下端に留まる」／`:118`（失敗メッセージが「是正前は旧下端 {old_bottom} に留まる」を刷る） |
| 由来 | コミット `b3491d4f` 本文「拡大率が変わらず作業領域だけ変わったフレームで、下端吸着のキャラ窓を現寸のまま新しい下端へ 1 回だけ書き直す経路を、拡大率の相の直後に置いた」 |

### 2.2 是正後の緑【本タスクで実行】

```
$ cargo test -p areka lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write -- --test-threads=1
     Running unittests src\main.rs (target\debug\deps\areka-a7688e48475ebf69.exe)

running 1 test
test emo2_boot::frame::work_area_sync_tests::lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1172 filtered out; finished in 0.00s
```

```
$ cargo test -p areka a_work_area_only_change_lands_the_ground_point_on_the_new_bottom_in_one_write -- --test-threads=1
     Running unittests src\main.rs (target\debug\deps\areka-a7688e48475ebf69.exe)

running 1 test
test emo2_boot::frame::work_area_resnap_tests::a_work_area_only_change_lands_the_ground_point_on_the_new_bottom_in_one_write ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1172 filtered out; finished in 0.00s
```

### 2.3 同一の主張であることの担保

- **テスト名は変わっていない。** 両ファイルとも履歴上のコミットは 1 つだけで（§6 の表）、是正と同時に
  着地してから 1 度も書き換わっていない。記録が名指しする関数と、上で緑になった関数は同一である。
- **測っている量も同一である。** 5.1 側の失敗メッセージは差を刷り（下の赤で `差 -48px`）、緑の側は同じ
  `diff` が 0 であることを主張する。5.2 側は接地点 Y そのものを刷る（赤で `left: 1444`・緑は 1492 と一致）。
- **探針が退化していないことは別テストが押さえている。** `frame_work_area_sync_tests.rs:114-121` の
  `the_two_scale_levels_really_move_the_work_area_bottom` が「合成レイアウトの 2 水準で下端が実機と同じ
  48px 動く」ことを固定するので、差が 0 になるのは是正が効いたからであって、探針が潰れたからではない。

### 2.4 今日の載荷【本タスクで実行】

#### M-A1: 作業領域源の実行時同期を無効化する

`crates/areka/src/emo2_boot/frame/work_area_sync.rs` の `sync_monitor_snapshot_with` へ 1 行入れ、
源を作り直さずに `None` を返させる（＝task 5.1 以前の状態）。

```rust
    if !monitors.is_empty() { return None; }   // ← 挿入した 1 行（この位置で monitors は必ず非空）
    let next = MonitorSources::from_monitors(monitors);
```

```
$ cargo test -p areka lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write -- --test-threads=1
running 1 test
test emo2_boot::frame::work_area_sync_tests::lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write ... FAILED

---- emo2_boot::frame::work_area_sync_tests::lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write stdout ----

thread '...' (6476) panicked at crates\areka\src\emo2_boot\frame_work_area_sync_tests.rs:95:5:
assertion `left == right` failed: 接地点が新しい作業領域下端に載っていない（差 -48px・是正前の値は -48px）
  left: -48
 right: 0

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1172 filtered out; finished in 0.60s
```

**`left: -48` は記録された是正前の量と一致する。** 同じ変異のままクレート全体を走らせると:

```
$ cargo test -p areka -- --test-threads=1
test result: FAILED. 1121 passed; 51 failed; 1 ignored; 0 measured; 0 filtered out; finished in 16.71s
```

#### M-A2: 作業領域変化を契機とする再スナップを無効化する

同ファイルの `resnap_for_work_area_change` を無操作にする（＝task 5.2 以前の状態）。

```rust
    if !change.current.work_areas.is_empty() || change.current.work_areas.is_empty() { return; }
    let mut targets: Vec<Entity> = Vec::new();
```

```
$ cargo test -p areka a_work_area_only_change_lands_the_ground_point_on_the_new_bottom_in_one_write -- --test-threads=1
running 1 test
test emo2_boot::frame::work_area_resnap_tests::a_work_area_only_change_lands_the_ground_point_on_the_new_bottom_in_one_write ... FAILED

thread '...' (28384) panicked at crates\areka\src\emo2_boot\frame_work_area_resnap_tests.rs:115:9:
assertion `left == right` failed: scope=0: 接地点が新しい作業領域下端に載っていない（是正前は旧下端 1444 に留まる）
  left: 1444
 right: 1492
```

```
$ cargo test -p areka -- --test-threads=1
test result: FAILED. 1157 passed; 15 failed; 1 ignored; 0 measured; 0 filtered out; finished in 20.31s
```

**`left: 1444` は記録された「旧下端 1444」と一致する。**

> **なぜ 2 つの変異を別々に当てたか。** 接地点 **Y** は経路 (b) で二重に守られている（`tasks.md` の
> `## Implementation Notes`「接地点 Y は経路 (b) で二重に守られている」）——拡大率の相の再射影と
> 作業領域再スナップの両方が新下端へ載せるので、**片方だけ潰しても他方が回復させる**腕がある。
> ゆえに 2 段の是正を 1 つの変異でまとめて測ることはできない。上の 2 件はそれぞれ自分の担当フレーム
> （拡大率が動くフレーム／拡大率が動かず作業領域だけ動くフレーム）を測っており、互いの回復に隠れない。

---

## 3. 件② 合流（台帳 L5・task 5.3）

同一 tick・同一窓へ積まれたジオメトリ指令が畳まれず、バルーン窓が 1 遷移で 2 回書かれていた。

### 3.1 是正前の赤【記録の引用】

| 項目 | 内容 |
| --- | --- |
| 量 | バルーン窓の書込回数 **2 本**（決定論の上限 1） |
| 記録の所在 | `crates/wintf/src/ecs/window/command_coalesce_tests.rs:6`（module doc）「是正前は 2 本＝決定論の上限 1 に対する違反」／`:154-155`（test doc）「バルーン窓の 2 本が 12 遷移 × 2 スコープ＝24 件の違反になっていた」 |
| 由来 | コミット `67bdd640` 本文「バルーン窓の書込が 2 回から 1 回になった」 |
| 実機の裏付け | `mechanism-ledger.md:38`（L5 の行）「実機は `merged_into_seq` が 77/77 で番兵」／根拠の詳細は同 §3.5（:103 以降）／`baseline-2026-08-20.md:114`（§3.2 ⑵）「決定論の上限 1 に対して 12 遷移 × 2 スコープ ＝ **24 件の違反**」 |

### 3.2 是正後の緑【本タスクで実行】

```
$ cargo test -p wintf a_balloon_window_is_written_once_per_transition -- --test-threads=1
     Running unittests src\lib.rs (target\debug\deps\wintf-da1bdd47b1e07db9.exe)

running 1 test
test ecs::window::command::command_coalesce_tests::a_balloon_window_is_written_once_per_transition ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 754 filtered out; finished in 0.00s
```

### 3.3 同一の主張であることの担保

- テスト名は変わっていない（ファイルの履歴コミットは `67bdd640` の 1 つだけ）。
- **測っている量は「窓ごとの書込回数」であり、赤・緑とも同じ `writes_per_window` を数えている**——設計
  C8 の B-2a 行が挙げる対テスト「決定論: `writes_per_window` 2→1（キュー検査）」そのものである。
- 探針は実機の 1 遷移・1 スコープをそのまま写している（キャラ窓へ `DpiReproject` 1 本、バルーン窓へ
  `KeepPositionResize` と `BalloonFollow` の 2 本）。キャラ窓が元から 1 本であることを同じテストが対照
  として先に主張するので、「全部消えて 1 になった」形では緑にならない。

### 3.4 今日の載荷【本タスクで実行】

#### M-B: 合流を無効化する

`crates/wintf/src/ecs/window/command.rs` の `coalesce_geometry` で、合流先を 1 度も探させない。

```rust
    let target = if false && is_coalescible(&cmd) {
```

```
$ cargo test -p wintf a_balloon_window_is_written_once_per_transition -- --test-threads=1
running 1 test
test ecs::window::command::command_coalesce_tests::a_balloon_window_is_written_once_per_transition ... FAILED

thread '...' (32004) panicked at crates\wintf\src\ecs\window\command_coalesce_tests.rs:184:5:
assertion `left == right` failed: バルーン窓の書込は寸と位置を畳んで 1 本になる: [SetWindowPosCommand { hwnd: HWND(0xfffffec1), x: 100, y: 200, width: 382, height: 684, flags: SET_WINDOW_POS_FLAGS(20), hwnd_insert_after: None, tag: WriteTag { origin: "DpiReproject", scope: Some(0), kind: "shell" } }, SetWindowPosCommand { hwnd: HWND(0xfffffec3), x: 500, y: 200, width: 336, height: 240, flags: SET_WINDOW_POS_FLAGS(20), hwnd_insert_after: None, tag: WriteTag { origin: "KeepPositionResize", scope: Some(0), kind: "balloon" } }, SetWindowPosCommand { hwnd: HWND(0xfffffec3), x: 482, y: 210, width: 0, height: 0, flags: SET_WINDOW_POS_FLAGS(21), hwnd_insert_after: None, tag: WriteTag { origin: "BalloonFollow", scope: Some(0), kind: "balloon" } }]
  left: 2
 right: 1
```

**`left: 2 / right: 1` は記録された「2 本 → 1 本」そのものである。** キャラ窓 1 本とバルーン窓 2 本＝
3 本という内訳も、記録された実機の 1 遷移・1 スコープの形と字面で一致する。

```
$ cargo test -p wintf --lib -- --test-threads=1
test result: FAILED. 736 passed; 19 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.44s
```

**同じ変異は areka 側の多フレーム駆動（task 6.1）でも赤になる**——遷移フレームの窓書込が 4 本
（2 スコープ × キャラ／バルーン）ではなく **8 本**になる。

```
$ cargo test -p areka transition_atomicity_tests -- --test-threads=1
test emo2_boot::frame::transition_atomicity_tests::the_atomicity_cases_run_against_a_multi_monitor_work_area_table ... ok
test emo2_boot::frame::transition_atomicity_tests::transition_is_atomic_at_120 ... FAILED
test emo2_boot::frame::transition_atomicity_tests::transition_is_atomic_at_192 ... FAILED

thread '...' panicked at crates\areka\src\emo2_boot\frame_transition_atomicity_tests.rs:347:5:
assertion `left == right` failed: 遷移フレームの窓書込が 4 本（2 スコープ × キャラ／バルーン）ではない: [...]
  left: 8
 right: 4

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 1170 filtered out; finished in 0.58s
```

---

## 4. 件③ 整合待ち（台帳 L6・task 5.4）

Windows は `WM_DPICHANGED`（窓の拡大率）と `WM_DISPLAYCHANGE`（モニタ表）の順序を保証しない。
拡大率通知が先に届く順序では、窓は**新しい寸のまま旧作業領域下端へ**接地した中間矩形で 1 度書かれ、
表が追いついたフレームでもう 1 度書き直される——要件 5.8 が名指しで禁じる 2 段書込である。

### 4.1 是正前の赤【記録の引用】

| 項目 | 内容 |
| --- | --- |
| 量 | 待ちフレームで窓書込が出る（`y + height = 1444`＝**旧下端**の中間矩形）。経路 (a) が **2 段**書込になる |
| 記録の所在 | `crates/areka/src/emo2_boot/frame_dpi_sync_hold_tests.rs:13`（module doc）「是正前は旧下端 1444 の中間矩形が出て赤くなり」／`:144`（節見出し「完了条件そのもの（是正前の赤）」）／`:177`（失敗メッセージ「表が追いつく前に窓書込が出ている（旧下端 {old_bottom} の中間矩形）」） |
| 由来 | コミット `fb39419d` 本文「是正前は y+height=1444＝新寸のまま旧下端へ書き、次フレームで再スナップがもう 1 本書いていた」 |
| 台帳 | `mechanism-ledger.md:39`（L6 の行・経路の 5 点）／根拠の詳細は同 §3.6（:116 以降）。**本機序の証跡クラスは静的構造である**——実機 12 遷移はすべて表更新が先だったので、この順序は実機採取では 1 度も発火していない（同 §3.6）。ゆえに 7.3 の対テストは決定論の側にしか作れない |

### 4.2 是正後の緑【本タスクで実行】

```
$ cargo test -p areka a_scale_notice_ahead_of_the_table_lands_in_one_write_without_the_old_bottom -- --test-threads=1
     Running unittests src\main.rs (target\debug\deps\areka-a7688e48475ebf69.exe)

running 1 test
test emo2_boot::frame::dpi_sync_hold_tests::a_scale_notice_ahead_of_the_table_lands_in_one_write_without_the_old_bottom ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1172 filtered out; finished in 0.00s
```

### 4.3 同一の主張であることの担保

- テスト名は変わっていない（ファイルの履歴コミットは `fb39419d` の 1 つだけ）。
- **赤と緑は同じ 2 つの量を測る**——⑴ 待ちフレームの窓書込件数（赤で 2 件・緑で 0 件）と ⑵ 解除
  フレームでのキャラ窓の書込件数と載っている下端（緑で 1 件・新下端 1492）。設計が挙げる
  「hold（経路 (a) の 2 段→1 段）」はこの 2 量の対である。
- **零件を主張する前に条件の成立を固定している**（同ファイル `:164-173`）——待ちフレームで
  `window_dpi=192 / table_dpi=Some(96)` になっていることと、待ち札が実際に付いたことを先に主張する。
  ゆえに「駆動が死んでいたから 0 だった」形では緑にならない。

### 4.4 今日の載荷【本タスクで実行】

#### M-C: 整合待ちを無効化する（常に進む＝待たない）

`crates/areka/src/placement/dpi_sync.rs` の純判定 `dpi_sync_decision` を常に `Proceed` へ倒す
（＝task 5.4 以前の状態）。

```rust
    if window_dpi != u32::MAX { return DpiSyncDecision::Proceed; }
    let Some(table_dpi) = table_dpi else {
```

```
$ cargo test -p areka a_scale_notice_ahead_of_the_table_lands_in_one_write_without_the_old_bottom -- --test-threads=1
running 1 test
test emo2_boot::frame::dpi_sync_hold_tests::a_scale_notice_ahead_of_the_table_lands_in_one_write_without_the_old_bottom ... FAILED

thread '...' (27204) panicked at crates\areka\src\emo2_boot\frame_dpi_sync_hold_tests.rs:175:5:
表が追いつく前に窓書込が出ている（旧下端 1444 の中間矩形）: [SetWindowPosCommand { hwnd: HWND(0x100), x: 1266, y: 70, width: 868, height: 1374, flags: SET_WINDOW_POS_FLAGS(20), hwnd_insert_after: None, tag: WriteTag { origin: "DpiReproject", scope: Some(0), kind: "char" } }, SetWindowPosCommand { hwnd: HWND(0x110), x: 854, y: 45, width: 0, height: 0, flags: SET_WINDOW_POS_FLAGS(21), hwnd_insert_after: None, tag: WriteTag { origin: "BalloonFollow", scope: Some(0), kind: "balloon" } }]
```

**出た中間矩形は `y=70`・`height=1374` ゆえ `y + height = 1444`＝記録された旧下端そのものである。**
新寸（`height=1374`＝192 水準）のまま旧下端（96 水準の 1444）へ接地しており、コミット `fb39419d` 本文が
書いた「新寸のまま旧下端へ書き」の形と字面で一致する。

```
$ cargo test -p areka -- --test-threads=1
test result: FAILED. 1155 passed; 17 failed; 1 ignored; 0 measured; 0 filtered out; finished in 20.86s
```

---

## 5. 件④ 連鎖再解決（台帳 L4・task 5.6）

起動時の連鎖確定は一度きりで、確定後は解き直さない。拡大率が変わると全スコープの幅が k 倍に変わり、
各窓は下端中央を保ったまま置き直されるので、隣接していた 2 体のあいだに幅変化の半分の和だけ隙間が開く。

### 5.1 是正前の赤【記録の引用】

| 項目 | 内容 |
| --- | --- |
| 量 | 隣接ペアの隙間 **359px**（あるべき値 0） |
| 記録の所在 | `crates/areka/src/emo2_boot/frame_chain_realign_tests.rs:9`（module doc）「実機は 200%→100% で **359px**（幅 764→382 と 672→336 の左端差 `191 + 168`）だった」／`:12`「是正前は隙間 359 が残って赤くなり」／`:72-75`（定数 `GAP_WITHOUT_REALIGN: i32 = 359` とその doc）／`:225`（test doc）「是正前は左端差の和（実機 359px の決定論版）が残る」 |
| 実機の裏付け | `mechanism-ledger.md:37`（L4 の行）「実機は第 1 段の 100% で二体の隙間 359px」／根拠の詳細は同 §3.4（:92 以降）／`reobservation-2026-08-15.md:129`「**隙間 359px**（各窓が中央保存で縮んだ幅の半分の和 = (764−382)/2 + (672−336)/2 = 191+168）。200% へ戻すと隙間 0 に戻る」 |
| 注意 | 台帳 §3.4 が明示するとおり、**是正前の採取の `chain_realigned=0` は根拠に使わない**（観測点が task 5.6 で初めて入るので、消灯による 0 である・要件 8.5） |

### 5.2 是正後の緑【本タスクで実行】

```
$ cargo test -p areka the_gap_returns_after_a_scale_change_and_the_realign_closes_it -- --test-threads=1
     Running unittests src\main.rs (target\debug\deps\areka-a7688e48475ebf69.exe)

running 1 test
test emo2_boot::frame::chain_realign_tests::the_gap_returns_after_a_scale_change_and_the_realign_closes_it ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1172 filtered out; finished in 0.00s
```

### 5.3 同一の主張であることの担保

- テスト名は変わっていない（ファイルの履歴コミットは `2c9293d4` の 1 つだけ）。
- **測る量は「隣接ペアの隙間」で赤・緑とも同一である。** 探針は実機の実表示寸をそのまま使う
  （高水準 764x1094 と 672x596・低水準はちょうど半分）ので、`359` は合わせ込んだ定数ではなく
  レイアウトから導かれる値である（`1509 − (814 + 336) = 359`＝左端差 `191 + 168`）。
- **緑が空虚でないことを、同じテスト本体が先に固定している**（`:234-244`）——解き直しを走らせる**前に**
  「遷移が実際に幅を半分にして隙間 `GAP_WITHOUT_REALIGN` を開けた」ことを主張する。したがって隙間 0 は
  「そもそも開かなかった」ではなく「開いたものを閉じた」である。
- **是正の効果を測る檻と、武装条件の特異性を測る檻は別ファイルである**（`tasks.md` の
  `## Implementation Notes`「是正の効果を測る檻は、その是正を起こす条件の特異性を固定しない」）。
  後者は `frame_chain_realign_arm_tests.rs` が持つ。本書が対にするのは前者の量である。

### 5.4 今日の載荷【本タスクで実行】

#### M-D: 遷移後の連鎖再解決を無効化する

`crates/areka/src/placement/chain_realign.rs` の `realign_chain_once_with` を、武装していても
何もせずに返させる（＝task 5.6 以前の「一度確定したら解き直さない」状態）。

```rust
    if world.get_resource::<ChainRealignPending>().is_some() {
        return;
    }
    if world.get_resource::<ChainRealignPending>().is_none() {
        return;
    }
```

```
$ cargo test -p areka the_gap_returns_after_a_scale_change_and_the_realign_closes_it -- --test-threads=1
running 1 test
test emo2_boot::frame::chain_realign_tests::the_gap_returns_after_a_scale_change_and_the_realign_closes_it ... FAILED

thread '...' (6404) panicked at crates\areka\src\emo2_boot\frame_chain_realign_tests.rs:248:5:
assertion `left == right` failed: 遷移後の連鎖の解き直しが効いていない（隣接ペアの隙間が残っている・要件 6.1）
  left: 359
 right: 0
```

**`left: 359 / right: 0` は記録された実機 359px の決定論版そのものである。**

```
$ cargo test -p areka -- --test-threads=1
test result: FAILED. 1163 passed; 9 failed; 1 ignored; 0 measured; 0 filtered out; finished in 20.76s
```

---

## 6. 対テストの由来（是正前のコミットが存在しないことの裏取り）

| 対テストのファイル | 履歴上のコミット | 同コミットが入れた是正 |
| --- | --- | --- |
| `crates/areka/src/emo2_boot/frame_work_area_sync_tests.rs` | `3a4c6696` のみ | task 5.1 作業領域源の実行時同期 |
| `crates/areka/src/emo2_boot/frame_work_area_resnap_tests.rs` | `b3491d4f` のみ | task 5.2 作業領域変化を契機とする再スナップ |
| `crates/wintf/src/ecs/window/command_coalesce_tests.rs` | `67bdd640` のみ | task 5.3 合流 |
| `crates/areka/src/emo2_boot/frame_dpi_sync_hold_tests.rs` | `fb39419d` のみ | task 5.4 整合待ち |
| `crates/areka/src/emo2_boot/frame_chain_realign_tests.rs` | `2c9293d4` のみ | task 5.6 連鎖再解決 |

（`git log --oneline -- <ファイル>` で 1 件ずつ確認した。）

各対テストは**是正と同一のコミットで初めて現れ、以後 1 度も書き換わっていない**。ゆえに:

- **テストが在って是正が無い木は履歴上に存在しない。** 過去のコミットへ戻しても赤は再現できない
  （タスク文が破壊的な操作を禁じているのとは別に、そもそも取れない）。
- **テスト名の変更も無い。** 記録が名指しする関数名と、今日緑になった関数名は同一である。
- 是正タスクは「対テストを先に書き、是正前の赤を実行で記録してから是正を入れる」手順で進んでおり
  （tasks.md 5.1／5.3／5.4／5.6 の各バレット）、その赤は**作業ツリー上でのみ存在した**。逐語の実行
  出力そのものはリポジトリに残っていないので、本書は §2〜§5 の【記録の引用】で量と出所を示し、
  **同じ量が今日のコードでも変異によって再現すること**を【本タスクで実行】で示した。

---

## 7. 変異の復元（バイト単位）

変異を当てた 4 ファイルは、変異前に採った控えから**バイト単位で**戻した。

```
$ sha256sum -c <控え>/sha256.txt
crates/areka/src/emo2_boot/frame/work_area_sync.rs: OK
crates/wintf/src/ecs/window/command.rs: OK
crates/areka/src/placement/dpi_sync.rs: OK
crates/areka/src/placement/chain_realign.rs: OK
```

| ファイル | sha256（変異前＝復元後） |
| --- | --- |
| `crates/areka/src/emo2_boot/frame/work_area_sync.rs` | `c6e298488a00d2d7630a515a951d3cc9ded5c0aee2e003acf133166c19fb53c2` |
| `crates/wintf/src/ecs/window/command.rs` | `703575839daa82af949e55fab98aa39be12336cfc80018ed762e09dd2f2e5b34` |
| `crates/areka/src/placement/dpi_sync.rs` | `0578ba8cabd3c16fac682368d02899ddc44f612220598f48df663978eec556a8` |
| `crates/areka/src/placement/chain_realign.rs` | `34e4efa51d3a70a4fd99af90ae5b8b7d78265391f29e59ad793ceb0cbed48d22` |

```
$ git status --short
 M crates/areka/src/placement/follow_visibility_balloon_wiring_tests.rs

$ git diff --stat
（出力なし）
```

**残る 1 行は変異とは無関係である。** `follow_visibility_balloon_wiring_tests.rs` は本タスクの前から
` M` と表示されるが blob は HEAD と一致し `git diff` は空である（stat キャッシュの残骸。`git hash-object`
と `git ls-files -s` と `git rev-parse HEAD:<path>` の 3 者が `782f027c4376a2c1af1215d2d6dca1794710a032`
で一致する）。task 5.5 の rustfmt 巻き添え是正（`tasks.md:317`）の際に生じたもので、本タスクでは
このファイルに触れていない。

---

## 8. 全体走行（復元後・本タスクで実行）

```
$ cargo test -p areka -- --test-threads=1
test result: ok. 1172 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 18.99s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.11s
```

```
$ cargo test -p wintf
test result: ok. 755 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.40s
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 97 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.18s
test result: ok. 170 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.67s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 26 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

ワークスペース全体の走行は本タスクの範囲外である（i686 の host-32 成果物が要る＝task 6.4 の持ち分）。

---

## 9. 一覧（4 件の対）

| # | 是正 | 台帳 | 是正タスク | 量 | 是正前 | 是正後 | 対テスト |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ① | 作業領域追随（源の同期） | L3 | 5.1 | 接地点差 px | **−48** | **0** | `work_area_sync_tests::lowering_the_scale_lands_the_ground_point_on_the_new_work_area_bottom_in_one_write` |
| ① | 作業領域追随（再スナップ） | L3 | 5.2 | 接地点 Y | **1444**（旧下端） | **1492**（新下端） | `work_area_resnap_tests::a_work_area_only_change_lands_the_ground_point_on_the_new_bottom_in_one_write` |
| ② | 合流 | L5 | 5.3 | バルーン窓の書込回数 | **2** | **1** | `command_coalesce_tests::a_balloon_window_is_written_once_per_transition` |
| ③ | 整合待ち（hold） | L6 | 5.4 | 待ちフレームの窓書込 | **2 件**（旧下端 1444 の中間矩形） | **0 件**／解除フレームで 1 回 | `dpi_sync_hold_tests::a_scale_notice_ahead_of_the_table_lands_in_one_write_without_the_old_bottom` |
| ④ | 連鎖再解決 | L4 | 5.6 | 隣接ペアの隙間 px | **359** | **0** | `chain_realign_tests::the_gap_returns_after_a_scale_change_and_the_realign_closes_it` |

**4 件すべてについて、是正前の量が【記録の引用】と【今日の変異による実行】の両方で一致し、
是正後の量が【今日の実行】で規約値になっている。**
