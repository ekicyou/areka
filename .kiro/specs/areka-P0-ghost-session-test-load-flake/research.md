# ギャップ分析: areka-P0-ghost-session-test-load-flake

- 実施日: 2026-10-05
- 方法: 確定した requirements.md と、今のコード（main `44fc0a61` から分かれたこのブランチ）を Grep・Read で静的に読み比べた。cargo のビルドやテストは回していない（赤の再現は要件 1 の手順で着手後に行う）。
- 読んだもの: spec.json・requirements.md・brief.md・steering（product・tech・structure・roadmap の該当の行）・待ちの部品と、それを使うテストのファイル。

## 1. 要約

- **待ちの部品は 3 か所に散らばり、どれも成否の真偽だけを返す。** `emo2_boot/spine.rs` の `spin_wait_until`（30 秒）・`run_bounded`／`join_bounded`（10 秒・中で panic）、足場 `emo2_boot/ghost_switch_test_support.rs` の `wait_steady`（20 秒）と、`spin_wait_until` の上に乗る `pump_until`・`pump_talking_until`・`pump_input_until`。ほかにテストのファイルが自前で持つ締切つきの待ちが 10 か所ほどある。どれも「何を待って・何秒待って・相手が進んだか」を出さない（要件 3 の欠け）。
- **自分たちで CPU を食っている見込みが強い。** `spin_wait_until` は最初の 100 万回を `yield_now` の空回しで回す。足場の `pump_*` は 1 回ごとに ECS の段を回すので、1 回が数〜数十マイクロ秒かかり、100 万回を使い切る前に 30 秒の締切に届きうる（＝ sleep へ落ちずに締切まで 1 コアを焼く。要実測）。起こし直しのテストの自前の待ちは sleep へ落ちる段すら無い。多数のテストが同時に待つと、待っている相手（kanade・dispatcher・作業プールの各スレッド）を自分たちで飢えさせる（要件 2.5 の欠け）。
- **台詞の時計の追い越しの疑いがある。** `pump_talking_until` は実時間 1 ms ごとに合成の時計を 100 ms 進め、上限が無い。kanade には合成の時計で数える締切（送り出しの台詞の再生完了待ち `close_talk_deadline_ms` ＝ 30,000 ms ＝ Tick 300 回）がある。相手のスレッドが飢えている間も時計だけが進むと、この締切が先に切れて「打ち切って降ろす」へ流れ、記録の並びや error の数の比較が赤になりうる。`spine.rs` の doc 自身が同じ型の失敗（注入時刻が観測を追い越して条件が壊れる）を記録しており、締切を延ばしても直らない種類である。目印のログ `change_deadline_exceeded` があるので、再現の手順で白黒を付けられる。
- **展開の後片付けの os error 5 は、作業フォルダの札（ロック）の閉じる順と、札を持たない `gc-` の木に起因する、プロセスをまたぐ競合の見込みが高い。** 失敗は標準エラーへの表示だけで赤にならない。テストの赤の原因というより「同じ負荷の下で一緒に出る雑音」と見られる（要調査）。
- **直し方は、部品を `spine.rs` の子のファイルへ出して呼び名を保ったまま中身を変える形（下の案 C）が、触るファイルを最も少なくできる。** 呼び名を保てば、足場を使う 28 本と `spin_wait_until` を使う 15 本は、呼び出しの形を変えずに済むものが大半になる。

## 2. 数え直し（要件 1.4 の下調べ）

起票時の数と比べた結果。着手時にもう一度数えて記録する（要件 1.4）。

| 数えたもの | 数え方 | 今の数 | 起票時の数 | 差の理由 |
|---|---|---|---|---|
| 足場 `ghost_switch_test_support` を名指しするファイル | `git grep -l ghost_switch_test_support -- crates/areka/src` | 28（定義の本体と、宣言の `emo2_boot/mod.rs` を含む） | 28 | 一致 |
| `spin_wait_until` を名指しするファイル | `git grep -l spin_wait_until -- crates/areka/src` | 16（定義の `spine.rs` を含む＝呼ぶ側は 15） | 20 | 起票時の 20 は「`spin_wait_until` **または** 締切の定数 `SPIN_WAIT`」を数えた数。`SPIN_WAIT` だけを名指しするファイル 4 本（`spine_display_tests.rs`・`spine_hold_support.rs`・`spine_move_cue_tests.rs`・`spine_settle_tests.rs`）を足すと 20 で一致する。`crates/areka-ghost/tests/` の 2 本はコメントの中の言及だけ |
| 本番のファイルの中の利用 | 同上 | 1（`mcp/dump_surface.rs` の `#[cfg(test)]` の `WaitAnswer`） | — | 本番の処理からは呼ばれていない |
| `run_bounded`／`join_bounded` を名指しするファイル（`crates/areka/src`） | `git grep -l "run_bounded\|join_bounded"` | 7（`spine.rs`・足場・`spine_talk_close_tests.rs`・`ghost_session_restart_tests.rs`・`session_end_sync_send_tests.rs`・`install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`） | 7 | 一致。ただし `install/` の 2 本は同じ名前の**別の定義**（自前の 10 秒・30 秒） |
| `wait_steady` を呼ぶファイル | `git grep -c wait_steady` | 20（うち `frame_ghost_quit_switch_tests.rs` は**同じ名前の自前の定義**＝受け口を外して 20 秒まで別の通知を読み飛ばす） | — | — |
| `pump_talking_until` を呼ぶファイル | 同上 | 8（足場を除く） | — | — |
| `pump_until`／`pump_input_until` を呼ぶファイル | 同上 | 9（足場を除く） | — | — |

テストのファイルが自前で持つ締切つきの待ち（`Instant::now() +` か `recv_timeout(` を持つもの・`crates/areka/src` のテスト側）:

| ファイル | 締切 | 待ち方 |
|---|---|---|
| `ghost_session_restart_tests.rs` の `run_input_until` | 10 秒 | `yield_now` だけの空回し（sleep へ落ちない）。毎回 `Input` の段を回す |
| `ghost_session_restart_tests.rs` の `shutdown_bounded` | 20 秒 | `run_bounded`（期限切れは panic「possible hang」） |
| `emo2_boot/frame_ghost_quit_switch_tests.rs` の `wait_steady` | 20 秒 | `recv_timeout`（眠って待つ・CPU は食わない） |
| `emo2_boot/frame_ghost_quit_logsink_tests.rs`・`emo2_boot/install_cue_tests.rs` | 30 秒（`BOUND`） | 各自 |
| `emo2_boot/spine_close_wiring_tests.rs`（2 か所）・`spine_display_tests.rs`・`spine_move_cue_tests.rs`・`spine_seriko_loop_tests.rs`（4 か所）・`spine_talk_close_tests.rs`（2 か所） | 30 秒（`SPIN_WAIT`）・10 秒 | 合成の Tick を注入しながら待つ形（200 µs の sleep をはさむ）。合成の時計に上限が無いものがある（例: `spine_close_wiring_tests.rs` の終了の握手の待ち） |
| `emo2_boot/frame/switch_tests.rs` | 5 秒 | `yield_now` の空回し・中で assert |
| `install/desk_pick_tests.rs`・`install/fetch_url_tests.rs` | 10 秒・30 秒 | 自前の `join_bounded`（`recv_timeout`） |
| `mcp/get_expression_table_tests.rs` | 20 秒 | `recv_timeout` |
| `menu/trigger_tests.rs`・`menu/trigger_show_tests.rs`・`thread_roles_tests.rs` | 各自 | 対象の族の外（赤の観測なし）。数え上げにだけ載せる |

同じ名前の部品は他の crate にもある（`areka-ghost`・`areka-kanade`・`areka-actor` の `run_bounded`／`join_bounded` が 10 本ほど）。これらは別の実行ファイルで、赤の観測も無いので、対象の族の外と見てよいかは設計で決める（下の論点 7）。

## 3. 今あるもの（そのまま使える型）

- **締切つきの待ちの規律と、その根拠の実測**（`spine.rs` の `SPIN_WAIT`・`SPIN_YIELD_BUDGET`・`BACKOFF_SLEEP` の doc）。「反復の回数で打ち切らない」「長引いたら CPU を返す」「注入する合成の時計は観測を追い越してはならない（締切では直らない）」が既に文書になっている。今回の直しはこの規律を足場の `pump_*` と自前の待ちへ広げる仕事と言える。
- **時計を差し替える継ぎ目**（`settle_bounded_with`・`spine_conformance_support.rs` の `run_stage_with`）。「締切を越えたら何と言って落ちるか」を、実時間を待たずに決定論で檻に入れる型が既にある。待ちの部品の失敗の文言（要件 3）を決定論のテストで固定するのに、そのまま使える。
- **型つきの失敗**（`spine_conformance_support.rs` の `StageFailure`）。失敗を `bool` でなく「何がどうだったか」を持つ値で返す先例。
- **合成の時計に頭打ちを置く先例**（`spine_conformance_support.rs` の `limit_ms`・`injection_capacity`）。`pump_talking_until` の時計に上限を置くときの型になる。
- **眠って待つ待ち**（足場の `wait_steady` と `frame_ghost_quit_switch_tests.rs` の `wait_steady` は `recv_timeout`）。CPU を食わない待ちの手本。
- **偽の SHIORI の記録**（`ScriptedShioriHandle::non_status_calls`）。呼び出しの数は単調に増えるので「相手が進んだか」の目印に使える候補。
- **機械の負荷を測る道具**（`tools/perf/check-quiet.ps1`）。マシン全体の CPU の平均・最大と、重いプロセスの有無を記録できる。再現の手順で「どれだけ重かったか」を残すのに使える（ただし静かかどうかの判定用に作られており、負荷を作る道具ではない）。
- **全体の見張り**（`crates/log-capture-kit/tests/` の `file_length_guard_test.rs`・`temp_path_guard_test.rs`・`sample_path_guard_test.rs`）。要件 7 の守る対象。

## 4. 要件ごとの対応と欠け

| 要件 | 今あるもの | 欠け | 種別 |
|---|---|---|---|
| 1.1〜1.3 再現の手順 | `check-quiet.ps1`（負荷の測定だけ）・`tools/test-all.ps1`（`-j 4`） | 負荷を作り、ワークツリーの `target\` の下だけで動き、自分が起こした子プロセスだけを止める手順が無い。赤を回ごとに集めて記録する形も無い | Missing |
| 1.4 数え上げ | 上の 2 節 | 着手時の数え直しと記録 | — |
| 2.1 進み続けている限り赤にしない | 締切は総時間だけで決まる（30・20・10 秒） | 「進みの有無」で打ち切りを決める仕組みが無い | Missing |
| 2.2 到達で判定・確かめを弱めない | 呼び手の `done` は状態の到達で書かれている | `wait_steady` は「先に別の通知が来たら `false`」で、その通知が何だったかを捨てる | Constraint |
| 2.3 sleep・延長・1 フレーム遅らせで直さない | — | 直しの選び方の制約 | Constraint |
| 2.5 待っている間に CPU を占めない | `spin_wait_until` は 100 万回の空回しの後に 1 ms の sleep | `pump_*` は 1 回が重く、空回しの段で締切まで行きうる。`run_input_until`（起こし直し）・`frame/switch_tests.rs` は sleep へ落ちない | Missing |
| 3.1 文言に「何を・何秒・進んだか」 | `run_bounded`・`join_bounded` だけが「何を・何秒」を出す（進みは出さない） | 他の部品は `bool` だけ。呼び手は待ちの結果を大きな組の比較に混ぜるので、赤の文言から待ちの打ち切りか判定の食い違いかが読めない | Missing |
| 3.2 止まったら「止まった」と言う | — | 進みの目印と、その無い時間を数える仕組み | Missing |
| 3.3 どの待ちにも上限 | 全部に上限はある | 形を変えても上限を保つこと | Constraint |
| 4.1〜4.4 本物の競合 | — | 再現の結果次第。候補は下の 5 節 | Unknown |
| 5.1〜5.3 os error 5 | `sample-ghost-kit/src/devroot.rs` の札と掃除の仕組み・兄弟テスト | 原因の確定と、同時に走っても妨げない形 | Unknown |
| 6.1〜6.4 確かめ | — | 再現の手順を前後で同じ条件で回す段取り | Missing |
| 7.1 1,000 行 | `spine.rs` はちょうど 1,000 行・足場は 426 行・`ghost_session_restart_tests.rs` は 424 行 | `spine.rs` へは 1 行も足せない。部品を外へ出すのが先 | Constraint |
| 7.2 一時パスの窓口 | `temp-path-kit`・`sample-ghost-kit` | 再現の手順の一時ファイルは「テスト」でなくスクリプトの置き場の話。見張りの対象かを確かめる | Constraint |
| 7.3 部品の形を変えたら全利用者を移す | — | 呼び名を保てば移す本数を減らせる（下の案 C） | Constraint |

## 5. 赤の原因の候補（再現の手順で確かめるもの）

1. **締切の総時間切れ（負荷で遅いだけ）。** 30・20・10 秒の締切に、飢えた相手が間に合わない。brief の観測（`spin_wait_until` の 30 秒の期限切れ・「spine ghost shutdown did not complete」10 秒）はこれに当たる。
2. **自分たちの空回しによる飢え。** 同じ実行ファイルの中で多数のテストが同時に `yield_now` で回り、相手のスレッドの実行の機会を奪う。`--test-threads=1` で緑になった観測と合う。`spine.rs` の doc に「純 yield ＋ 30 秒期限で無関係な 3 テストが巻き添え」の実測がある。
3. **合成の時計の追い越し（締切では直らない種類）。** `pump_talking_until` と `spine_close_wiring_tests.rs` などの注入の待ちは、合成の時計に上限が無い。kanade の合成の締切（送り出しの台詞の再生完了待ち 30,000 ms＝Tick 300 回＝静かな机で約 0.3 秒の実時間）が、相手が飢えている間に切れうる。切れると `change_deadline_exceeded` の error が出て台詞を打ち切って降ろすので、error の数・記録の並びの比較が赤になる。brief の「`boot_event_tests` の記録の並びの比較の赤」はこの型の可能性がある（ただしその試験の切替は `raise_event: false` で送り出しの台詞を流さないので、別の合成の締切が絡むかは要調査）。再現の回ごとに `change_deadline_exceeded` などの記録を拾えば白黒が付く。
4. **本番の順序の取り違え（本物の競合）。** 今の静的な読みでは決定的な候補は見つかっていない。例: kanade は `basewareversion` の NOTIFY の完了の後に `Steady` を出すので（`areka-kanade/src/schedule/boot.rs`）、「`Steady` の後に `basewareversion` が記録に載る」並びの取り違えは起きない作り。再現で 1〜3 に当たらない赤が残ったときだけ、ここを掘る。

## 6. os error 5 の見立て（要件 5 の下調べ）

- 場所: `crates/sample-ghost-kit/src/devroot.rs`。作業フォルダ `target\nar-samples\work\<プロセス識別子>-<連番>\` は、隣の札（`.lock`・削除を共有しない形で開いたまま）で生死を示す。取得のたびに `sweep` が棚を走査し、消せた札の木を「持ち主が居ない残骸」として `gc-…` へ `rename` してから消す。`rename` の失敗は `report_cleanup` が標準エラーへ「…の退避に失敗した（次の走行で回収する）」と出すだけで、赤にしない。
- 起きうる組（静的な読みからの見込み）:
  1. **`WorkDir` の破棄の順。** `Drop` は ⑴ 札を閉じる ⑵ 木を消す ⑶ 札を消す の順。⑴ と ⑶ の間に別のプロセスの `sweep` が走ると、札を消せてしまい（持ち主が居ないと判定）、持ち主がまだ消している最中の木を `rename` しようとする。消している途中のファイルは「削除待ち」で掴まれているので、Windows は `rename` を拒み os error 5 になる。札を閉じる理由は「札そのものを消すため」なので、⑵ 木を消す → ⑴ 札を閉じる → ⑶ 札を消す の順へ入れ替えれば、この窓は閉じる見込み。
  2. **札を持たない `gc-` の木。** 退避した `gc-…` の木には札が無いので、持ち主が `remove_dir_all` している最中に、別のプロセスの `sweep` が「札の無い残骸」として同じ木の退避を試み、同じ理由で拒まれる。
  3. **ウイルス対策・検索の索引が新しく写した木を掴む。** 写した直後の木は走査されやすい。この場合はテストの側の原因ではない（要件 5.3）。
- 誰が使うか: `SampleRoot::acquire`（とその内側の `fresh_root`）を呼ぶファイルは areka で 11 本（取得の呼び出しは約 20 か所）、ワークスペース全体で約 13 crate（examples を除く）。同じ `target\nar-samples\work\` を、全体テストの x64 段で同時に走る複数のテストの実行ファイルが共有する。取得のたびに棚全体の走査と木の丸ごとの複写が走るので、これ自体も負荷の高い時の I/O を増やす。
- 赤との関わり: 退避の失敗は表示だけなので、それ自体はテストを赤にしない。同じ回の赤と一緒に出るのは「同じ負荷の下で一緒に起きる」からの見込みが高い（因果でなく相関）。要件 5.1 の記録では、再現の回ごとに os error 5 の有無と赤の有無を並べて残すと判断できる。
- 触る場所: 直すなら `sample-ghost-kit`（テスト専用の crate）。兄弟テスト `a_sweeper_running_alongside_never_disturbs_a_staging_tree_or_a_live_copy`・`the_lease_refuses_deletion_while_it_is_held` が今の不変条件の檻なので、それを緑のまま保つ。`sample_path_guard_test.rs` の見張りにも触れないように直す。

## 7. 直し方の案

### 案 A: 今の部品の中身だけを変える（呼び名も形も保つ）

- `spin_wait_until` の空回しの予算を「回数」から「時間」（例: 最初の数 ms だけ空回し）へ変え、`pump_*` が sleep へ落ちるようにする。`run_input_until`（起こし直し）と `frame/switch_tests.rs` の空回しにも同じ下限を入れる。
- 締切を越えたとき、部品の中で「何を・何秒・進んだか」を出してから `false` を返す（標準エラーへ出す）。
- 良い点: 呼び手は 1 行も変わらない。差分が最小。
- 悪い点: `bool` を返す形のままなので、呼び手の大きな組の比較に混ざった赤の文言は今と同じ（文言は標準エラーの別の場所に出る）。「進んだか」を部品が知る手段が無い（呼び手が目印を渡す形が要る）。`spine.rs` は 1,000 行ちょうどなので、中身を変えるだけでも先に外へ出す必要がある。

### 案 B: 新しい待ちの部品を作り、全部の呼び手を移す

- 新しいファイル（例: `emo2_boot/spine_wait.rs`）に、`what`（何を待つか）・進みの目印・締切を受け取り、届かなければ文言つきで panic する（または型つきの失敗を返す）部品を置く。眠って待てるところ（受け口の `recv_timeout`・偽の SHIORI の記録の更新の知らせ）は眠って待つ。
- 呼び手を全部（足場の `pump_*`・15 本の `spin_wait_until`・自前の待ち）新しい形へ移す。
- 良い点: 要件 2・3 をいちばん素直に満たす。赤の文言が待ちの打ち切りを名指しする。
- 悪い点: 触るファイルが 30 本を超えうる。待ちの結果を大きな組の比較に混ぜている呼び手は、確かめの内容を変えないように書き直す手間が要る（要件 7.3）。

### 案 C: 部品を `spine.rs` の子のファイルへ出し、呼び名を保ったまま中身を差し替え、足場と自前の待ちだけ形を変える（混合）

- `spine.rs` の待ちの部品（`SPIN_WAIT`・`SPIN_YIELD_BUDGET`・`BACKOFF_SLEEP`・`spin_wait_until`・`run_bounded`・`join_bounded`、必要なら `settle_bounded`）を `spine.rs` の中の `#[path]` の子のモジュールへ出し、`spine.rs` から同じ名前で出し直す（`pub(crate) use …`）。**`emo2_boot/mod.rs` を触らずに済む**（並走の spec の持ち物）。呼び手の `use` も変わらない。
- 新しい部品の中で: 空回しを時間で区切る（要件 2.5）・締切を越えたら「何を・何秒・進んだか」を出す（要件 3.1）。進みの目印は、受け取れる部品（足場の `pump_*` は `SwitchRig` が偽の SHIORI の記録の数・受け口の通知の数を見られる）から渡し、受け取れない古い呼び手は「進みは不明」と書く。
- 足場の `pump_talking_until` の合成の時計に頭打ちを置く（相手が追いつくまで進めない形）。`wait_steady` は先に来た別の通知を文言に残す。
- `ghost_session_restart_tests.rs` の `run_input_until` などの自前の待ちは、新しい部品へ寄せる。
- 良い点: 触るファイルを、`spine.rs`・新しい子のファイル・足場・自前の待ちを持つ数本に抑えられる。28 本と 15 本の呼び手の多くは変えずに済む。
- 悪い点: `bool` を返す古い形を残すと、要件 3.1 の「失敗の文言に含める」を部品の中の出力で満たすことになる（呼び手の比較の文言には載らない）。それで足りるかは論点 2 で決める。

## 8. 規模と危うさ

- 規模: **M**（3〜7 日）。部品の切り出しと中身の差し替えは S だが、負荷の下での再現を前後で回す手順（要件 1・6）と、os error 5 の調べと直し（要件 5）が乗る。案 B なら L 寄り。
- 危うさ: **中**。直す部品の型（締切・空回し・時計の頭打ち）は `spine.rs` に先例と実測がある。一方、赤は負荷次第で出たり出なかったりし、再現しない（要件 6.4 の据え置き）か、本物の競合が見つかって本番のファイルへ波及する（要件 4.3 の止まって報告）かが、着手するまで読めない。

## 9. 設計で決めること（要件の討議への論点）

1. **直し方の向き。** 待ちを「眠って待つ観測の待ち」へ置き換える／締切を越えたときの文言で負荷と欠陥を見分ける／両方、のどれを本命にするか（要件は両方を許す）。上の案 A・B・C のどれを土台にするかと対になる。
2. **失敗の文言をどこに出すか。** `bool` を返す今の形を残して部品の中で標準エラーへ出すか、部品が文言つきで panic する（または型つきの失敗を返す）形へ変えて呼び手を移すか。後者は赤の文言そのものが待ちの打ち切りを名指しするが、待ちの結果を大きな組の比較に混ぜている呼び手（例: 切替の失敗から既定ゴーストへ戻るテスト）の書き直しが要る。
3. **「相手が進んだ」の目印を何にするか。** 候補は ⑴ 偽の SHIORI の記録の数（`ScriptedShioriHandle`） ⑵ kanade の通知の数 ⑶ 待っている条件のもとになる値の変化（呼び手が渡す） ⑷ 記録されたログの件数。部品が目印を受け取る形にすると、部品の呼び出しの形が変わる。
4. **「止まった」と言うまでの時間と、要件 2.1 の両立。** 相手のスレッドが飢えて「進まない」時間は、負荷の下では長くなりうる。進みの無い時間で「止まった」と言うなら、その長さをどう取るか（総時間の上限とは別に）。総時間の上限（要件 3.3）を何秒にするか。
5. **CPU を占めない待ち方（要件 2.5）。** ⑴ 空回しの予算を時間で区切る ⑵ 偽の SHIORI の記録や受け口の通知で眠って待つ（テスト側の部品に知らせを足す） ⑶ 同時に生きている足場（ゴーストを起こすテスト）の数を実行ファイルの中で数本に絞る、のどれを取るか。⑶ は「全体テストの並列度を変えない」の外にあるが、赤を隠すことにならないかを決める必要がある。
6. **合成の時計の頭打ち。** `pump_talking_until`（と `spine_close_wiring_tests.rs` などの注入の待ち）の時計に上限を置くか、置くなら「相手が前の Tick を処理し終えるまで次を進めない」か「1 回の待ちで進める合成の時間の上限」か。再現の手順で `change_deadline_exceeded` が出るかを先に見てから決める手もある。
7. **対象の範囲。** ⑴ 他の crate の同名の部品（`areka-ghost`・`areka-kanade` などの `run_bounded`／`join_bounded`）を入れるか ⑵ `install/` の自前の `join_bounded` を入れるか ⑶ `emo2_boot/frame/switch_tests.rs`（`frame/` の下のテストのファイル・本番のファイルではない）の 5 秒の空回しを入れるか。`frame/` の下は並走の spec の持ち物の場所なので、テストのファイルでも触ってよいか確かめが要る。
8. **待ちの部品の置き場。** `spine.rs` の子のモジュール（`emo2_boot/mod.rs` を触らない）にするか、`emo2_boot/` の兄弟のモジュール（`emo2_boot/mod.rs` に宣言を 1 行足す＝並走の spec の持ち物に触る）にするか。
9. **再現の手順の形。** 負荷の作り方（CPU を回すだけの子プロセスを論理 CPU の数だけ・または別の置き場での `cargo build`）・回す範囲（`cargo test -p areka --bin areka` の全体か、族の名前で絞るか）・回数と 1 回の上限・スクリプトを `tools/` に置くか spec の記録に手順として書くだけか・負荷の記録に `check-quiet.ps1` を使うか。`areka-test-threads-av` と同時に走らせない段取りも含む。
10. **os error 5 を直すか、相関の記録で閉じるか。** 見込みの原因（札を閉じる順・`gc-` の木に札が無い）を直すなら、`sample-ghost-kit` の破棄の順を入れ替える・`gc-` の木にも札を持たせる、のどちらか（または両方）。ウイルス対策が原因と分かれば記録して外す（要件 5.3）。

- 要件の討議での補足（2026-10-05）:
  - 論点 2: テストの実行器は、赤になったテストのスレッドの標準出力・標準エラーを、失敗の報告にまとめて出す。部品の中で文言を出しても、その文言は要件 3.1 の「失敗の文言」として赤の報告に載る。panic の文言へ入れるかは、読みやすさと書き直す量の兼ね合いで決める。
  - 論点 6: 要件 2.6（注入の時刻は観測を追い越さない）として要件に入った。設計で決めるのは頭打ちの形だけ。
  - 論点 5 の ⑶（足場の同時の数を絞る）: 要件 2.7 として、待ち方の直しの後に残った赤に限り、文言の証拠付きで許した。
  - 新しい論点: 全体テストの所要時間が「目立って延びた」とする目安（要件 6.5）。眠って待つ・時計の頭打ち・同時の数の絞りは、どれも所要時間を延ばしうる。
  - 要件 6.4: 再現しなくても、静的に確かめた待ち方の欠け（2.5・2.6・3）は直す（開発者の裁定）。
  - 論点 7: 赤の観測が無い待ちは範囲の外として要件に明記した（再現で赤になったものだけ加える）。`frame/switch_tests.rs` が再現で赤になったときは、本番のファイルではないので触ってよい。

## 10. 設計へ持ち越す調べもの（Research Needed）

- `pump_*` の 1 回あたりの時間を測り、100 万回の空回しが締切の 30 秒の中で使い切れるか（sleep へ落ちるか）を確かめる。
- 足場の切替で、kanade・dispatcher のどの合成の締切が `pump_talking_until` の時計で切れうるかの一覧（送り出しの台詞の 30,000 ms のほか、選択肢の待ちの既定 30,000 ms など）。
- `ghost_switch_boot_event_tests.rs` の記録の並びの赤が、どの比較で落ちたか（呼び出しの数か、並びか）。brief の観測には失敗の文言が残っていないので、再現の手順で取り直す。
- os error 5 が出た回に、どのプロセスのどの木だったか（`gc-` の木か・作業フォルダか）。表示の文言にパスが出るので、再現の回の標準エラーを残せば分かる。
- `--test-threads=1` で緑になるかを、同じ負荷の条件で確かめ直す（起票時の見立て・未確認）。

## 11. 設計の段の調べと決定（2026-10-05）

- 調べ方: 軽い調べ（既存の仕組みの拡張）。待ちの部品・足場・dispatcher・kanade の締切・`sample-ghost-kit` の後片付けを、ファイルと定義の名前で読んだ。cargo のビルドとテストは回していない（机を他のセッションと共有しているため）。外部の依存は足さないので、外の調べは無い。

### 11.1 持ち越した調べものへの答え

| 調べもの（10 節） | 答え | 根拠 |
|---|---|---|
| `pump_*` の 1 回の時間・100 万回の空回しを使い切れるか | 実測は実装へ持ち越し。静的には、`pump_*` は 1 回ごとに ECS の段を回すので、足場の待ちは終わるまで空回しのままと読める（sleep へ落ちる前に待ちが終わるか 30 秒に届く） | `SwitchRig::pump_until`・`pump_talking_until`・`pump_input_until` が `spin_wait_until` の条件の中で `run_ghost_quit_phase`・`world.run_schedule(Input)` を呼ぶ |
| 足場の時計で切れうる合成の締切の一覧 | **0 件**。足場のゴーストの kanade は Tick を受け取らない | dispatcher の `DispatcherState::on_tick` は再生中の台詞へ経過秒を渡すだけ。`KanadeMsg::Tick` を送るのは `areka-ghost/src/ticker.rs` と `spine_conformance_support.rs` の `StageSink for SpineHarness` だけ。足場は `TickerMode::Disabled` |
| `ghost_switch_boot_event_tests.rs` の赤がどの比較か | 再現の手順で取り直す（設計では決められない） | — |
| os error 5 がどの木か | 再現の記録の標準エラーの行のパスで見分ける。見分けの表を design の「WorkDirCleanup」に置いた | `devroot.rs` の `report_cleanup` はパスを出す |
| `--test-threads=1` で緑になるか | 再現のスクリプトの引数 `-TestThreads 1` で対照を取る | — |

ギャップ分析の候補 3（足場で `change_deadline_exceeded` が出て赤になる）は、上の 2 行目により足場については成り立たない。`spine_*_tests.rs` の自前の注入の待ちも dispatcher の Tick だけを送る（`SpineHarness::inject_dispatcher_tick`）。

### 11.2 設計の決定

- **土台は案 C。** 待ちの芯（`wait_until`／`wait_recv`）を `emo2_boot/spine_wait.rs`（`spine.rs` の子のモジュール）に 1 つ置き、古い呼び名（`spin_wait_until`・`run_bounded`・`join_bounded`）は形を保った薄い包みにする。`emo2_boot/mod.rs` に触らない。
- **打ち切りは「進みの無い時間」で決める。** 進みの目印（単調に増える数）が 30 秒（`SPIN_WAIT`）増えなければ「止まった」、増え続けて 300 秒（`WAIT_CAP`）に届けば「進んではいた」、目印の無い待ちは今と同じ総時間 30 秒で「進みは不明」。秒数を延ばすだけの直しではない（基準を変える）。
- **進みの目印（論点 3）**: 足場は「起こした回数＋偽の SHIORI が受けた呼び出しの総数」。本番のコードに数え口を足さない。作業プールの進み（`ghost_session_restart_tests.rs` の `run_input_until`）は数える口がテストの側に無いので「目印なし」のまま共通の部品へ寄せる。
- **CPU（論点 5）**: 空回しの予算を回数（1,000,000 回）から時間（`DENSE_SPIN` ＝ 60 ms）へ。値は `SETTLE_MIN` の doc の実測（`yield_now` 5,000 回＝無負荷 0.31 ms）から今の予算を時間に直したもの。速い待ちは今と同じ、重い待ち（足場）だけが 60 ms で CPU を返す。
- **文言の出し場所（論点 2）**: 芯は型つきの失敗 `WaitFailure` を返す。`bool` を返す包みは標準エラーへ 1 行出してから `false`。待ちが `false` を返すことを期待するテストは 0 件（`git grep` で確かめた）なので、出した文言は必ず赤の報告と一緒に出る。
- **合成の時計の頭打ちの形（論点 6・要件 2.6）**: 数値の頭打ちを置かない。足場の時刻の受け手は届いた順に消化する台詞だけで、合成の締切を持つ kanade は Tick を受け取らない。これを決まりにして檻（相手を `HoldAt` で固めたまま時計を 30,000 ms 以上進めても、切替が同じ並びで終わり error が 0 件）で固定する。
  - 採らなかった形 1（前の Tick の処理の知らせを待つ）: `areka-ghost` の dispatcher と `areka-sakura` の再生に数え口が要る。足場に守るべき合成の締切が無いので、本番に手を入れる理由が立たない。
  - 採らなかった形 2（1 回の待ちで進める合成の時間の上限）: 台詞の着地の前に時計が上限へ届くと台詞が凍り、負荷次第の赤を新しく作る（`StageSink::may_advance_clock` の doc の実測と同じ型）。
  - **設計の討議で確かめる点**: この決定は要件 2.6 の目的（合成の締切だけが切れる赤を作らない）を構造で満たすが、「知らせを待ってから次を送る」形ではない。その形を求めるなら境界を本番のコードへ広げる必要がある。
- **足場の同時の数（要件 2.7）**: 直しの後の再現で `［進んではいた］` の赤が残り、記録したときだけ、`SwitchRig::new` に許可（同時の数の初期値は論理 CPU の半分）を入れる。
- **再現の手順（論点 9）**: `tools/load-flake.ps1`。負荷は CPU を回すだけの子プロセス（既定は論理 CPU × 2）、範囲は `cargo test -p areka --bin areka` の全部（同じ実行ファイルの中の取り合いを再現するため）、5 回、1 回 30 分まで。出力は `target\load-flake\`。記録は spec の `load-repro.md`。
- **os error 5（論点 10）**: `WorkDir` の破棄の順（札を閉じてから木を消す）と、札の無い `gc-` の木が、doc の決まり「木が見えている間は札が開いている」を破っている（静的に確かめた）。ただし原因の確定は要件 6.4 により再現が要るので、直しは再現の記録が同時の利用を示したときだけ入れる。
- **所要時間の目安（要件 6.5・新しい論点）**: 直した後が直す前の 1.10 倍を超えたら「目立って延びた」。対象の族は 3 回ずつの中央値（前の 3 回の最大が線より上なら最大を線にする）、全体テストは 1 回ずつ。延びたら `DENSE_SPIN` を先に調整する。

### 11.3 まとめ直し（一般化・作るか使うか・削るもの）

- 一般化: 3 か所に散らばっていた待ち（空回し・受け口・別スレッドの完了）を、打ち切りの決め方 1 つにまとめた。違いは「どう待つか（条件を回すか・受け口で眠るか）」だけ。
- 作るか使うか: 外の crate は使わない（`std` で足りる）。時計を差し替える継ぎ目と型つきの失敗は、既にある型（`settle_bounded_with`・`StageFailure`）に倣う。
- 削ったもの: 進みの目印のための中継スレッド・本番の数え口・合成の時計の数値の頭打ち・全部の呼び手の書き換え（案 B）。足場を使う 28 ファイルは呼び出しの形が変わらない。

### 11.4 危うさ

- 進みの目印が見ていない長い処理（SHIORI の呼び出しを伴わない合流など）が 30 秒を超えると `［止まった］` と出る。直した後の再現で出たら目印を足す。
- `DENSE_SPIN` の値しだいで所要時間が延びる。目安（1.10 倍）で見張る。
- 同じウェーブの約束の外で触るのは `tools/load-flake.ps1`（新規）と、条件つきの `crates/sample-ghost-kit/src/devroot.rs`（＋兄弟のテスト）。後者は触る前に他の spec と重ならないことを確かめる。
