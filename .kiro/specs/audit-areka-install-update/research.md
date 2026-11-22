# ギャップ分析: audit-areka-install-update（2026-11-22）

## 1. 要約

- **この回でしたこと**: 受け持つ範囲の数え直し（要件 1.6）、brief と要件の行番号の当て直し（要件 1.7）、預かった件 1〜8 の今のコードとの照合（要件 5）、仮置きの議題 1〜7 の事実の確かめと推奨、作業の形とタスク数の見込み。
- **この回でしなかったこと（要件 1.1〜1.5 との差）**: `/ponytail:ponytail-audit` そのものは、この回の依頼（「audit の実施は実装の仕事・ギャップ分析は準備と作業の形」）に従ってかけていない。要件 1.1〜1.5 と brief の Approach（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）は、ギャップ分析の報告に指摘の全件の一覧が載ることを前提にしている。完了した見直し 30 本は、どれもギャップ分析の段で audit をかけている（例 `completed/audit-areka-input-menu/research.md` の 4 節＝77 件）。この差は議題 0 として出す（9 節）。
- **数と行番号**: 要件の表（124 ファイル・本番 10,315 行・テスト 25,838 行）と一致した。brief と要件に書いた行番号は 1 つも動いていない。
- **預かった件**: 件 1〜8 は全部まだ在る。件 4・件 5 は既にある共通の道具（`areka_runtime::spine::join_bounded`・`log_capture_kit::capture_lines`）に置き換えるだけで済む。件 6 の 2 つの写しは、既にある公開の補助（`areka_session::reload::test_support::boot_context`）を読むだけで消せる見込み（持ち主の違うファイルには手を入れない）。
- **議題 1（`NO_BODY` と本体の「窓口が無い」受け口の差）の推奨**: そろえない。差は `url` の腕だけで、どちらも本番の道では通らず、どちらも記録を残す。そろえる向きはどちらも、持ち主の違うファイル（本体の `main.rs` の `body_cue_sinks`・その字面を読む `runtime_tests/zorder_wiring_tests.rs`）か依存の向き（`areka-session` から本体の型は作れない）にぶつかる。ただし `NO_BODY` の doc の「（窓口が無いときの答えと同じ）」は `url` の腕で事実と違うので、持ち主の `audit-areka-app` へ doc の直しを申し送る。

## 2. 数え直し（要件 1.6）

着手のときの HEAD は `32a86bd78`（親 `1b3ede541`＝要件の表の `main`）。範囲の `.rs` を `wc -l` で数えた。テストは名前が `_tests.rs`・`_test_support.rs` で終わるもの。

| crate | ファイル | 本番 | テスト |
|---|---|---|---|
| 本体（`install/`・`update/`・`mcp/`・`roots/`・`roots.rs`・`session_tests/readme_body_tests.rs`） | 104 | 42 ファイル・8,417 行 | 62 ファイル・22,689 行 |
| `areka-session`（`roots/`・`readme/`・`readme.rs`・`readme_tests.rs`） | 20 | 9 ファイル・1,898 行 | 11 ファイル・3,149 行 |
| 合計 | 124 | 51 ファイル・10,315 行 | 73 ファイル・25,838 行 |

要件の Introduction の表と一致する。範囲に `.rs` でないファイルは無い。1,000 行に近いもの（上から）: `update/worker_tests.rs` 967・`install/startup_tests.rs` 854・`mcp/get_expression_table_tests.rs` 808・`install/desk_tests.rs` 788・`mcp/dump_surface_tests.rs` 721・`update/desk.rs` 706（本番でいちばん大きい）。越えるものは 0。`update/worker_tests.rs` と `install/startup_tests.rs` へテストを足す直しは、新しいファイルへ分けることになる（要件 6.1）。

## 3. 行番号の当て直し（要件 1.7）

| 件 | brief／要件の行 | 今の行（`32a86bd78`） |
|---|---|---|
| 件 1 | `install/desk.rs` 381 の `barrier()`・`areka-sylphya/src/actor.rs` 562 の `barrier_within` | 381・562（動いていない） |
| 件 2 | `areka-session/src/log_history.rs` 131（`areka_session::readme`）・136（`UNKEPT_TARGETS`） | 131・136 |
| 件 3 | `install/mod.rs` 132・`update/mod.rs` 224・`mcp/mod.rs` 48 | 132・224・48 |
| 件 4 | `install/names_tests.rs` 29 | 29 |
| 件 5 | `areka-session/src/readme/readme_cue_tests.rs` 24 | 24 |
| 件 6 | `install/desk_tests.rs` 730・`readme/opener_submit_tests.rs` 31・寄せ先の候補 `reload_test_support.rs` 17 | 730・31・17 |
| 件 7 | `install/desk_tests.rs` 385・554・`update/desk_resolve_tests.rs` 558 | 385・554・558 |
| ほか | `install/judge.rs` 90（`doc/COMPAT_ARCHITECTURE.md` の名指し）・`areka-session/src/roots/user_root.rs` の `writable_ghost` の doc | 90・199〜200（完了のときに直した形のまま） |

## 4. 今のコードの形（Current State）

- **層と向き**: 本体の `install/`・`update/`・`mcp/`・`roots/{desk,edit,pick}` は、`BODY_SYSTEMS`（`main.rs` 360）の口の束を通して `areka-session` から呼ばれる（`after_input` → `register_body_after_input` → `install::register`・`update::register`・`mcp::register`）。`areka-session` の `roots/{mod,resolve,report,user_root}` と `readme*` は本体から引かれる。本体の型を `areka-session` は知らない（依存の向き）。
- **窓口の型**: インストール・更新とも「台本の受け口（`*_cue.rs`）→ 生の要求の送出端 → UI スレッドの窓口（`desk.rs` の `drain`・Input の段）→ 背景のスレッド（`worker.rs`）」の同じ形。背景のスレッドは窓口へ頼み（`DeskAsk`）を送り、答えを待つ。終了は `exit_wait::register_gate` の門と `discard_for_exit`。
- **記録の分け方**: `log_history.rs` の `classify` は warn 以上を出所によらず「error」に数え、出所の前方一致で分けるのは info だけ。今 `update_cue`・`readme_cue` に `info!` は無い（`install_cue` の info は `readonly_refused` の 1 行で、`UNKEPT_TARGETS` で残さない）。
- **unsafe**: 15 か所（`install/pick.rs`・`roots/pick.rs`・`readme/os_port.rs`・`mcp/dump_surface_gpu_test_support.rs`）。各ファイルの `SAFETY` の数は unsafe の数と同じ（中身の正しさは audit で読む）。
- **先送りの印**: 範囲に `shortcut:`・`ponytail:` の注釈と `#[allow(...)]` は 0（要件 2.2 の 2 種類目に当たる候補は今のところ無い）。
- **audit で出そうな所の手がかり（audit ではない・件数に数えない）**:
  - コードとテストに要件の番号を書いた所が 5 ファイル 23 か所（`readme_tests.rs` 12・`roots/edit_tests.rs` 5・`readme/readme_cue_tests.rs` 4・`readme/readme_cue.rs` 1・`install/desk_record_tests.rs` 1）。CLAUDE.md の `## Comments` に反する。
  - `install/fetch_url.rs` の `spawn_download` は取得のスレッドを手放す（「手放す＝待たない」）。終了の門に乗らない。
  - 本番の `let _ =` が 4 か所（`install/procedure.rs` 142・`install/worker.rs` 248・`mcp/dump_surface.rs` 252・同 426〔`#[cfg(test)]`〕）。
  - テストの期限の無い `join()` が多い（`update/worker_tests.rs`・`install/worker_tests.rs`・`roots/desk_tests.rs` ほか）。件 4 と同じ種類。

## 5. 預かった件 1〜8 の照合（要件 5）

| 件 | 今の形 | 直し方の見込み（設計で決める） | 範囲の外への及び |
|---|---|---|---|
| 1 `barrier()` | `record_installed`（`install/desk.rs` 360）は窓口の `drain`＝UI スレッドで呼ばれ、期限の無い `barrier()` で書き手を待つ。本番で `barrier()` を UI から呼ぶ所はほかに `shiori_host.rs` があるが範囲の外 | `barrier_within(期限)` へ。期限切れは `barrier_within` 自身が `error!` を 1 件出すので、呼び手は今の `install_record_unsettled` の `error!` を残して続ける。期限の値は Research Needed（R2） | 無し（`areka-sylphya` は読むだけ） |
| 2 `UNKEPT_TARGETS` | `update_cue`・`readme_cue` は `info!` を持たない＝今は決めることが無い | 議題 2 | `log_history.rs` は `audit-areka-app` |
| 3 呼び手の説明 | 3 か所とも「呼び手は `ghost_session::register_systems`」。実際は `register_systems` → `BODY_SYSTEMS.after_input`（`main.rs` 375〜380 の `register_body_after_input`）→ 各 `register` | doc を「`BODY_SYSTEMS` の `after_input` から」に直す（1 行ずつ） | 無し |
| 4 `let _ = join()` | `names_tests.rs` 29 は `parts.handle.join()` の結果を捨てる。同じ crate に `areka_runtime::spine::join_bounded("sylphya", parts.handle).expect(..)` の前例（`runtime_tests/frame/switch_finish_tests.rs` 300）。本体の dev 依存は `areka-runtime` の `test-support` を開けている | 1 行の置き換え | 無し |
| 5 `capture_logs` の写し | 中身は `log_capture_kit::capture_lines(LineFormat::LevelTargetFields, f)` を包む 3 行。`areka-runtime` の同名の補助は `#[cfg(test)]` で外の crate から使えない。`areka-session` のほかのテスト（`ghost_quit_tests.rs` ほか）は `capture_lines` を直に呼ぶ | 私有の包みを消し、9 か所の呼びを `capture_lines(..).1` に（`areka-session` の流儀に合わせる） | 無し（`audit-areka-app` の brief の「サ」にも挙がる＝持ち主はこの spec と知らせる） |
| 6 `boot_context` の写し | 2 つとも `reload::test_support::boot_context(temp, root)`（`pub`・`cfg(any(test, feature = "test-support"))`）とほぼ同じ。違い: `desk_tests` はバルーン `B`・プロファイルが `root` の下、`opener_submit_tests` はゴースト `g`・バルーン `b`・`GhostRoute::Memory`。`desk_tests` の呼びは 7 か所とも `"A"` | 既にある補助を読むだけで 2 つとも消せる見込み（検体の名をそろえる）。違いがテストの結果に効かないかは R3 | 無し（`reload_test_support.rs` は読むだけ） |
| 7 `BODY_SYSTEMS` の 1 行 | 3 本とも、定常到達の知らせ（`on_steady`・`ghost_switch::on_notice`）が本体の口を通る道を試すテストで、1 行はその前提の名指し。寄せ先の候補は範囲の中にある（`install/desk_tests.rs` の `Rig::new`・`update/desk_resolve_tests.rs` の `world()`） | 議題 5 | 無し |
| 8 `NO_BODY` の差 | 6 節 | 議題 1 | 直す本文は `audit-areka-app` |

audit の指摘との重なり（要件 1.5）は、audit をかけていないので書けない（議題 0）。

## 6. 議題 1 の事実（`NO_BODY` と本体の「窓口が無い」受け口）

- `NO_BODY.cue_sinks`（`areka-session/src/body_systems.rs` 71）は `DiscardSink` を 2 つ返す。`DiscardSink::emit` は何もしない。束が無いことは `BodySystems::of` の `error!(body_systems_missing)` で記録が残る（`#[cfg(not(test))]`）。
- 本体の `body_cue_sinks`（`main.rs` 424〜432）は `InstallCueSink::new(install::desk::raw_sender(world))` と `UpdateCueSink::new(update::desk::raw_sender(world))`。窓口が無いと `raw_sender` は受信端の無い送出端（`mpsc::channel().0`）を返す。
- そのときの振る舞いの差:
  - `path` の腕: 送れずに `warn!(install_cue_send_failed)`。`NO_BODY` は黙って捨てる（ただし束の無いことは error で残る）。
  - `url` の腕: `fetch_for` が本物の取得（`fetch_url::spawn_download`）を起こし、**ネットワークから書庫を落としてから** `warn!(install_fetch_send_failed)`。`NO_BODY` は取りに行かない。読み取りだけ（readonly）なら取りに行かない点は同じ。
  - 更新の受け口: 送れずに `warn!`。`NO_BODY` は黙って捨てる。
- 通る道: 窓口は `after_input`（`register_systems` の中・どのゴーストを起こすより前）で据わり、終了の `discard_for_exit` も窓口を外さない（待ち行列を捨てるだけ）。よって本番で「窓口が無い」受け口も `NO_BODY` も通らない。通るのは結線の誤りとテストの足場（`SwitchRig::new`）だけ。
- 字面の檻: `runtime_tests/zorder_wiring_tests.rs` 56（`audit-areka-app` の持ち物）が `body_cue_sinks` の 2 行を字面で照合している。本体の側を変えるとこのテストも直すことになる。
- 試し: `session_tests/body_systems_tests.rs` の `missing_body_systems_logs_and_answers_like_no_desk` は、束の無いときの error 1 行・`update_busy`／`install_busy` が偽・受け口の数 2 を固定する。受け口の中身（捨てるか送るか）は固定していない。
- doc の食い違い: `NO_BODY` の doc「何もしない・偽・捨てる受け口 2 つ（窓口が無いときの答えと同じ）」は、`url` の腕と記録の有無で事実と違う。

## 7. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今ある物 | 足りない物・制約 |
|---|---|---|
| 1 洗い出し | 数え直しと行番号は済み（2・3 節） | **Missing**: audit の指摘の全件の一覧（1.1〜1.5）。議題 0 |
| 2 確定 | 議題 1〜7 の事実と推奨（9 節） | audit の一覧が無いと 2.1（確定した一覧を書き加える）ができない |
| 3 直し切る | 既にある共通の道具（`join_bounded`・`capture_lines`・`reload::test_support`・`barrier_within`） | 直す中身は audit 次第 |
| 4 テスト | 偽の取得の口（`StartFetch`・`MakeFetch`）・偽の SHIORI の土台（`switch_rig`）・`target` の下の根（`TempPath`）・GPU の足場（`mcp/dump_*_gpu_test_support.rs`） | **Unknown**: 件 1 の期限切れを決定論で起こす方法（R2）。`update/worker_tests.rs`（967）・`install/startup_tests.rs`（854）へは足せない＝新しいファイル |
| 5 預かった件 | 5 節 | 件 2・8 は議題。件 8 の本文は持ち主が違う |
| 6 決まり | 1,000 行の番人・`error!`＋`Err`・UI の外の重い仕事の形は今のコードが守っている | **Constraint**: 依存を足せない（C26 ⑵）。`fetch_url` の手放しのスレッドを終了の道に乗せる直しが出たら、終了の待ちの形（`exit_wait`＝`audit-areka-app`）に及ぶかを設計で見る |
| 7 並走 | 3 本の持ち物は brief と roadmap の C26 の約束 ⑴〜⑷ で 0 重なり | **Constraint**: `main.rs`・`body_systems.rs`・`log_history.rs`・`reload_test_support.rs`・`zorder_wiring_tests.rs` は `audit-areka-app` |
| 8 副作用 | 決定論の偽の口がそろっている | 実機・外のホストは使わない |
| 9 かけ直し | 前例の作法（`completed/audit-areka-input-menu/research.md` 12 節） | 打ち切りと Fable の総括の決めは要件に書いてある |

## 8. 進め方の案

- **案 A（既にある物へ寄せる・推す）**: 預かった件は既にある共通の道具に置き換え、audit の指摘は持ち主のファイルの中で「いちばん小さな完全な変更」で直す。新しいファイルは 1,000 行の番人と、テストを分ける所だけ。利点: 並走の約束に収まる・差分が小さい。弱み: 写しの 1 か所へのまとめ（件 6 の残り）は 3 本の着地の後まで残る。
- **案 B（共通の足場を範囲の中に新しく作る）**: `boot_context`・`BODY_SYSTEMS` の据え・ログの捕捉を範囲の中の `_test_support.rs` へ集める。利点: 範囲の中の写しは 0。弱み: 範囲の外にも同じ写しが残り（`menu/ghost_frame_tests.rs`・`ghost_switch_tests.rs`・`reload_test_support.rs`）、寄せ先がもう 1 つ増える。既にある `reload::test_support` と二重になる。
- **案 C（本体の口の形を変える）**: 件 8 で `raw_sender` を `Option` にして、窓口が無ければ受け口を捨てる形にする、など。弱み: `main.rs` と字面の檻（`audit-areka-app`）に及び、本番の振る舞いは変わらない。

推す＝案 A。

**規模と危うさ**: 規模 L（1〜2 週）＝本番 10,315 行・テスト 25,838 行を読み、前例の比（`audit-areka-input-menu` は本番 4,851 行で 77 件・18 小タスク、`audit-emo2-boot-spine` は 102 件・23 小タスク）から指摘は 80〜130 件の見込み。危うさ 中＝既にある型に沿う直しが大半だが、終了の道（取得のスレッド・背景のスレッドの答え待ち）と UI スレッドの待ち（件 1）に触り、持ち主の違うファイルに接する所が 5 つある。

**タスク数の見込み**: 小タスク 20〜26（brief の 18〜28 の中）。内訳の見込み: audit の実施と記録 1（議題 0 で実装へ回す場合）・預かった件 1〜8 が 3〜4・指摘の直しが区画（インストール・更新・MCP・根・説明書）ごとに 2〜3 で 11〜15・テストの分け直しと番人 1〜2・かけ直しと差分の確かめ 2〜3・申し送りと記録 1。

## 9. 要件の討議への持ち越し

### 議題（答えで作業が変わるもの）

0. **audit をどの段でかけるか**（要件 1.1〜1.5・2.1）。今回のギャップ分析は audit をかけていない。
   - 推す＝**要件の討議の前に、同じギャップ分析の続きとして audit をかけ、この報告へ書き足す**（要件の文は変えない）。理由: 要件 2.1 は確定した指摘の一覧を書き加えることを求め、議題にすべき 3 種類（要件 2.2）も一覧が無いと選べない。完了した見直し 30 本と brief の Approach（2026-10-19 の開発者の決め）がこの形。
   - 別案: audit を実装の最初のタスクへ回し、要件 1 と 2.1 を「最初のタスクで洗い出し、議題は実装の途中で仰ぐ」に書き換える。設計が指摘ごとの直し方を決められず、20 タスクの見込みも当て推量になるので推さない。
1. **`NO_BODY` と本体の「窓口が無い」受け口の差**（要件 5.8）。
   - 推す＝**そろえない**。`NO_BODY` の doc の「（窓口が無いときの答えと同じ）」を「本体の窓口の無いときとは `url` の腕が違う（本体は取りに行ってから送れずに warn・こちらは取りに行かない）」に直すか、括弧を消すことを、持ち主の `audit-areka-app` へ申し送る（`session_tests/body_systems_tests.rs` は今のまま）。理由: ⑴ 両方とも本番の道では通らない（6 節）⑵ 両方とも記録が残る（`body_systems_missing` の error／`install_cue_send_failed`・`install_fetch_send_failed` の warn）＝黙った失敗ではない ⑶ `NO_BODY` を本体へ寄せるのは依存の向きでできない ⑷ 本体を `NO_BODY` へ寄せる（窓口が無ければ取りに行かない）と `raw_sender` の形と `main.rs` の `body_cue_sinks`・字面の檻を変えるが、本番の振る舞いは 1 つも変わらない。
   - 別案: 窓口が無いときに `url` の腕が取りに行くことを無駄として直す（`InstallCueSink` が送出端の生死を知る形）。std の `Sender` は受信端の生死を送らずに知れないので、窓口の有無を受け口へ渡す形の変更になり、`main.rs` に及ぶ。audit が同じ所を指したときにだけ、その指摘として扱う。
2. **`update_cue`・`readme_cue` に `info!` を足すときの記録**（要件 5.2）。
   - 推す＝**受け口には `info!` を足さない（受理は `debug!`・断りは `warn!` の今の流儀を保つ）。足す直しが要るときに限り、その出所を `UNKEPT_TARGETS` へ足すことを `audit-areka-app` へ申し送る**（仮置きのまま）。理由: 受け口の受理の記録は、窓口と開く口が既に `info!` で残す記録（`update/desk.rs` の `update_*`・`readme/opener.rs` 101）と重なり、利用者の記録に同じ出来事が 2 行並ぶ。
3. **申し送る相手が先に着地したとき、自分で直してよいか**（要件 7.2）。
   - 推す＝**仮置きのまま（相手が `completed/` へ移った後なら `main` を取り込んで自分で直し、記録に残す）**。ただし相手がまだ走っているあいだは触らずに申し送る。理由: 申し送りの宛先が無くなった件を持ち主の無い宿題にしない（brief の Constraints と同じ）。
4. **`boot_context` の写しを 1 か所へまとめる仕事の持ち主**（要件 5.6）。
   - 推す＝**この spec の 2 つの写しは、既にある `areka_session::reload::test_support::boot_context` を読む形で 0 にする（読むだけ・持ち主のファイルに手を入れない）。残る写しはほかの 2 本の持ち物だけ（`menu/ghost_frame_tests.rs`＝`audit-areka-app`、`ghost_switch_tests.rs`＝`audit-emo2-boot-switch` は自分の `ghost_switch_test_support.rs` へ寄せる予定）なので、この spec から後の持ち主を立てない。同じ補助を使えることを 2 本のセッションへ知らせる（要件 7.10）**。3 本の着地の後に写しが 2 つ以上残っていれば、次の棚卸（`/kiro-next`）で拾う。要件 5.6 の文は「記録に残すまで」から「既にある補助へ寄せて範囲の中の写しを 0 にする」へ書き換える。R3 が外れた（検体の違いがテストの結果に効く）ときは仮置きの形へ戻す。
5. **`BODY_SYSTEMS` の 1 行を足した 3 本のテスト**（要件 5.7）。
   - 推す＝**寄せない（3 本の 1 行を残し、理由を記録に残す）**。理由: 1 行は「本体の口を通る道を試す」という前提を名指す明示の 1 行で、寄せ先（`Rig::new`・`world()`）へ入れると同じ足場を使うほかのテスト（口が無いときの倒れ方・窓口の有無を見るもの）の前提も黙って変わる。減るのは 1 か所あたり 1 行。
6. **20 タスクを越える見込みのときに 2 本目へ切るか**（要件 2.6）。
   - 推す＝**切らない（仮置きのまま）**。理由: 2026-11-16 の開発者の決め（20 タスクは discovery の分け方の目安で spec の上限ではない・一度切り出した後は規模を理由に分けない）が brief の文より後。見込みは 20〜26（8 節）。
7. **`mcp/mod.rs` 48 行の説明を、ファイルの持ち主のこの spec が直すこと**（要件 5.3）。
   - 推す＝**仮置きのまま（この spec が `install/mod.rs` 132・`update/mod.rs` 224 と同じ直しで直し、`audit-areka-app` のセッションへ直したことを知らせる）**。文は「`BODY_SYSTEMS` の `after_input` から」の形。

### Research Needed（設計で確かめる）

- **R1**: 議題 0 で audit をギャップ分析の続きにかけるとき、範囲が 36,000 行あるので、前例（`audit-areka-input-menu`＝本番は書き手が全行・テストは下請け 3 つに分けて全行）と同じ分け方で全行を読めるか（区画ごとに 4〜6 の下請け）。
- **R2**: 件 1 の期限の値と、期限切れを決定論で起こす方法。`areka-sylphya` の檻は止めの偽の IO（`actor_barrier_within_tests.rs` の `GatedFirstCommit`・私有）を使う。本体のテストから今のゴーストの記憶の書き手へ偽の IO を差せる公開の口が在るか（無ければ、`record_installed` の待ちを関数へ切り出して `barrier_within` の結果を受ける側だけを試す形）。期限の値は、UI を止めてよい長さとして `exit_wait::EXIT_WAIT_LIMIT`（3 秒）などの既にある値に合わせるか。
- **R3**: 件 6 で `reload::test_support::boot_context` に置き換えたとき、バルーンの名（`B`／`b` → `kaku`）・プロファイルの置き場（`root` の下 → `temp` の下）・`GhostRoute::Memory` → `Default` の違いが、`install/desk_tests.rs` の 7 本と `readme/opener_submit_tests.rs` の 1 本の結果に効かないか。
- **R4**: `install/fetch_url.rs` の手放しの取得のスレッドを audit が指したとき、終了の門（`WorkGate`・`exit_wait`）に乗せる直しが `areka-session` の `exit_wait`（`audit-areka-app`）に手を入れずにできるか。

### 申し送りの見込み（要件 7.10・audit の前に分かっている分）

- `audit-areka-app` へ: 議題 1 の `NO_BODY` の doc の直し。議題 2 で `info!` を足すときの `UNKEPT_TARGETS`。同 brief の「サ」（`readme/readme_cue_tests.rs` の `capture_logs`）と「シ」（`install/desk_tests.rs`・`readme/opener_submit_tests.rs` の `boot_context`）はこの spec が直すこと。`reload::test_support::boot_context` を範囲の外の写しの寄せ先にも使えること。
- `audit-emo2-boot-switch` へ: `reload::test_support::boot_context` が使えること（`ghost_switch_tests.rs` 84 の写しの寄せ先の選びに）。
