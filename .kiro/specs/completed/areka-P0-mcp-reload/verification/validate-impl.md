# 機能の検証（kiro-validate-impl・2026-10-19）

対象: `areka-P0-mcp-reload`・HEAD `e8c52efb`・main との merge-base `623a3923`（＝取り込み `d2e9343e` の第 2 親＝今の main）。差分は `git diff 623a3923..HEAD`（68 ファイル・main の取り込み分は含まない）。

## Validation Report
- DECISION: **NO-GO**（直しは 1 か所・1 行の削除で済む見込み）
- MECHANICAL_RESULTS:
  - Tests: **FAIL** — `pwsh -NoProfile -File tools/test-all.ps1` → 終了コード 1。11 段のうち赤は `x64 workspace tests`（`cargo test --workspace --no-fail-fast -j 4`・終了コード 101）の 1 段だけ。通過 12,432・失敗 2・無視 72（142 本のテストの実行ファイル）。ほかの 10 段（i686 の成果物のビルド・fmt --check・i686 のテスト・crates.io・文字コード・リリースノート・俯瞰・helper の揃え）は緑。
    - 赤の 2 本: `log-capture-kit` の `tests/file_length_guard_test.rs` の `every_over_limit_exception_is_still_over_the_limit`（210 行）と `the_measurement_is_not_vacuous_and_matches_the_allow_table`（267 行）。理由はどちらも「例外表に載っている `crates/areka-seriko/src/bind.rs` が 375 行で上限内」。
    - 原因は本 spec: タスク 8.3 の差し戻しで `bind.rs`（1,043 行）の中のテストを兄弟 `bind_tests.rs` へ出して 375 行にしたが、1,000 行の檻の例外表（`file_length_guard_test.rs` 79〜81 行）から `bind.rs` の行を消していない。揺れではなく決まって赤になるので、回し直しはしていない（触っていないクレートの揺れの回し直しの規則には当たらない）。
    - 本 spec が触ったクレートのテストはすべて緑（例: `areka` の単体 3,335 通過・`tests/mcp_get_log_real_run.rs` 1 通過・`tests/smoke_boot_loop_exit.rs` 5 通過）。
  - TBD/TODO grep: **CLEAN**（`git diff 623a3923..HEAD -- . ':!.kiro'` で足した行に TBD/TODO/FIXME/HACK/XXX は 0 件）
  - Secrets grep: **CLEAN**（足した行に `password=`・`api_key=`・`secret=`・`token=` は 0 件）
  - Smoke boot: **PASS** — 同じ全体テストの中で `cargo build` 済みの `areka` を起こす `tests/smoke_boot_loop_exit.rs`（5 本）と `tests/mcp_get_log_real_run.rs`（1 本）が緑。実機の証跡は `verification/8.2-real-machine.md`（5 つの読み直し・Risk 2 つ）と `8.3-shell-carry.md`（面と着せ替えの持ち越し）。GUI は自分では起こしていない。
- INTEGRATION:
  - Cross-task contracts: **整合**。2 つの入口（`reload_cue.rs` の `ReloadCueSink` → `ReloadRx` → `reload::drain`、MCP の `mcp/reload.rs::answer`）は `reload::request_reload` 1 本を通る。ghost は `accept_switch`（`raise_event: false`）→ `switch_to` が `boot_origin_override` で `BootOrigin::Reloaded{Ghost}` を選ぶ → kanade `on_prefetch_reply` が `boot_root` を見ずに basewareversion へ直行（`boot_reload_silent`）。shiori は `AwaitTalkGap`＋`OnDestroy(reload)` → `poll_destroy` → `start_switch` → `switch_to` の `wants_runtime_only` で `rebuild_runtime_in_place`（`Reloaded{Shiori}`→`OnInitialize` の Ref0＝`reload`）。終わりは予約を下ろす全部の出口が `end_switch` → `reload::on_switch_end`（例外は `session_end.rs` の 1 か所＝終了の門 `discard_for_exit` が拾う）。shell／balloon／descript は `accept_skin` → `frame/switch.rs` の `skin_is_reload`・`skin_applied`・`skin_phase_ended`。3.8 の持ち越しは `capture_for_reload`（降ろす前）→ `hand_over`（`boot_into` の成功の後に `Emo2Wiring.shell_carry`）→ `frame/attach.rs` の装着の後に `shell_carry::replay`。MCP の答えは `Accepted` の直後に `log_history::last_id()` を 1 度読む。実機 8.2・8.3 の記録の並びもこの順と一致する。
  - Shared state consistency: **整合**。印は `ReloadInFlight`（`Switching{reserved}`・`AwaitingDestroy`・`Skins`）1 つで、窓口自身の予約かは `reserved == SwitchInFlight.target.dir` で照合。仕込んだ `#[allow(dead_code)]` は差分の足した行に 0 件（全部外れた）。
  - Boundary audit: **ほぼ整合・記録済みの補正あり**。13.3 の禁止のファイル（kanade の `msg.rs`・`actor.rs`・`schedule/mod.rs`・`schedule/change.rs`・`crates/areka-mcp/**`・`mcp/mod.rs`・`mcp/resolve.rs`・`doc/ssp-mcp/areka-tools.md`・`Cargo.toml` 全部・`Cargo.lock`）の差分は 0 行。`crates/areka/src/main.rs` も差分 0 行（965 行）。13.1 の外に出たのは、設計の「13.1 の一覧の補正」の表（8.3 の 6 行を含む）・Notes 3.3 の裁定・8.1 の記録にある `areka-kanade/src/lib.rs`（`ReloadScope` の再輸出 1 語）と `input_events/balloon.rs`・`choice_drain.rs`（doc コメントだけ）で、どれも記録済み。**直しで `crates/log-capture-kit/tests/file_length_guard_test.rs` が新たに 13.1 の外に加わる**（下の REMEDIATION）。
- COVERAGE:
  - Requirements mapped: **13/13 節**（受け入れ基準 66 のうち 65 をタスクの `_Requirements:` が持ち、新しい 3.8 は 8.3 が持つ）。
  - Coverage gaps: 10.4（makoto の brief への申し送り）だけがタスクに無いが、tasks.md 188 行と design.md 386 行が `/kiro-complete` の手順で行うと決めている（今の `.kiro/specs/areka-P0-makoto-reload-directives/brief.md` には節はまだ無い＝完了のときに足す）。
- DESIGN:
  - Architecture drift: 無し。設計の Flow 1〜4 と ShellCarry の節のとおり。文書側の食い違い（`ScriptReload::parse`→`parse_script`・`ReloadRaw`→`ScriptReload`・`RuntimeBootInputs`→`Emo2BootInputs`・`rebuild_runtime_in_place -> bool`→`RuntimeRebuild`・543／478–480／512／518／680 行・ReloadDesk の「NoContext」）は Notes が `/kiro-complete` で直すと記録済み。
  - Dependency direction: 違反なし（kanade は areka を見ない・seriko は `BIND_COMMAND` を `lib.rs` の `pub use` で出すだけ・`areka-emo-present` は読み取りの口 1 つ）。
  - File Structure Plan vs actual: **一致**（新しいファイル 17 本と兄弟のテストが計画どおり）。変更した `.rs` で 1,000 行を超えるものは 0（最大は `ghost_switch.rs` 921・`ghost_session.rs` 910・`frame_attach_tests.rs` 905）。
- OWNERSHIP: **LOCAL**
- UPSTREAM_SPEC: N/A
- BLOCKED_TASKS: 無し（28 タスクすべて `[x]`・`_Blocked:_` 0 件）。Notes の残り（`/kiro-complete` でやること）: 13.1 への補正の追随（`areka-kanade/src/lib.rs`・8.3 の 6 ファイル）・design.md の文言直し・10.4 の申し送り・`log_history.rs` の `last_id()` の古い allow とコメント（13.1 の外・申し送り）・`frame/switch_reload_tests.rs:76` の clippy `clone_on_copy` 1 件・起票の候補 3 件（8.2）と sylphya の消す口（2.6）。
- REMEDIATION:
  1. `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表から `("crates/areka-seriko/src/bind.rs", "着手前から超過している本番ソース。…")` の 1 項目（78〜81 行の括弧ごと）を消す。表に件数や説明の文がほかにあれば合わせる。
  2. このファイルは要件 13.1 の一覧の外なので、design.md「要件 13.1 の一覧の補正」の表と tasks.md の 8.3 の Notes に 1 行足す（理由: `bind.rs` を 1,000 行以下へ分けた結果、例外表の項目が古くなった）。
  3. 確かめ: `cargo test -p log-capture-kit --test file_length_guard_test`（緑 6 本）。全体テストは `/kiro-complete` の 1 回に任せてよい（ほかの段と本 spec のクレートは今回緑）。
  4. その後 `/kiro-validate-impl areka-P0-mcp-reload` の再判定で GO の見込み（ほかの点検はすべて通っている）。

## Verification Result
- STATUS: NOT_VERIFIED
- CLAIM_TYPE: FEATURE_GO
- CLAIM: areka-P0-mcp-reload の 28 タスクが合わさって完成し、マージへ進める
- EVIDENCE: `pwsh -NoProfile -File tools/test-all.ps1` 終了コード 1（x64 の全体テストで `log-capture-kit` の 1,000 行の檻が 2 本赤・本 spec の 8.3 の分割が原因）。スモーク・網羅・結線・設計・ブロックは上のとおり通過。
- GAPS: 全体テストの赤 1 か所。
- NOTES: REMEDIATION 1〜3 の後に再判定。

---

# 再検証（2026-10-19・直しのコミット `61835a65`）

上の 1 回目（HEAD `e8c52efb`）の記録はそのまま残す。開発者の決まり（2 回目からは赤だったテストだけ）に従い、全体テストは回し直していない。1 回目の全体テストのほかの段と、本 spec が触ったクレートの緑は、そのまま有効とする。

## 直しの範囲

`git diff e8c52efb..61835a65 --stat` → 3 ファイル（3 行足し・5 行消し）。これ以外の変更は無い。
- `crates/log-capture-kit/tests/file_length_guard_test.rs`: 例外表 `OVER_LIMIT_ALLOWED` から `crates/areka-seriko/src/bind.rs` の項目（4 行）を消し、`OVER_LIMIT_ALLOWED_COUNT` を 10 → 9 にした。消したのはこの 2 か所だけ（テストの検査の中身は変えていない）。
- `.kiro/specs/areka-P0-mcp-reload/design.md`: 「要件 13.1 の一覧の補正」の表に、このファイルの行を 1 行足した。
- `.kiro/specs/areka-P0-mcp-reload/tasks.md`: 8.3 の Notes に 1 行足した。

## 赤だったテストだけを回し直した

`cargo test -p log-capture-kit --test file_length_guard_test` → 終了コード 0。**6 通過・0 失敗**。1 回目に赤だった 2 本は、同じ実行ファイルの中で緑になった。
- `every_over_limit_exception_is_still_over_the_limit` … ok
- `the_measurement_is_not_vacuous_and_matches_the_allow_table` … ok
- ほかの 4 本（`dropping_a_known_exception_turns_the_guard_red`・`no_source_file_exceeds_the_line_limit_outside_the_allow_table`・`the_over_limit_allow_table_has_no_duplicate_entries`・`the_over_limit_allow_table_declares_its_own_size_and_reasons`）も ok。

## Validation Report（再検証）
- DECISION: **GO**
- MECHANICAL_RESULTS:
  - Tests: **PASS**。1 回目は `tools/test-all.ps1` で 11 段のうち 10 段が緑、赤は 1 段（`log-capture-kit` の 2 本だけ）だった。その 2 本を含む `cargo test -p log-capture-kit --test file_length_guard_test` が、`61835a65` で終了コード 0・6 通過。直しはテストの表だけで、ほかのクレートのコードは差分 0 行なので、1 回目の緑はそのまま使える。
  - TBD/TODO grep: CLEAN（直しの差分にも 0 件）
  - Secrets grep: CLEAN
  - Smoke boot: PASS（1 回目のまま。`smoke_boot_loop_exit` 5 本・`mcp_get_log_real_run` 1 本と、実機の 8.2・8.3）
- INTEGRATION: 1 回目のまま整合（Cross-task contracts・Shared state・Boundary audit）。13.1 の外に出たファイルに `file_length_guard_test.rs` が 1 本加わり、design.md の補正の表と tasks.md の Notes に記録された。
- COVERAGE: 13/13 節。10.4 は `/kiro-complete` の手順で行う（記録済み）。
- DESIGN: ずれは無い・依存の向きの違反は無い・File Structure Plan と一致。
- OWNERSHIP: LOCAL ／ UPSTREAM_SPEC: N/A
- BLOCKED_TASKS: 無し（28 タスクすべて `[x]`）
- REMEDIATION: 無し。`/kiro-complete` へ申し送ること（1 回目と同じ）: 13.1 の一覧を補正に追随させる（`areka-kanade/src/lib.rs`・8.3 の 6 ファイル・今回の `file_length_guard_test.rs`）・design.md の文言を直す・10.4 の申し送りを書く・`log_history` の `last_id` の古い allow・`frame/switch_reload_tests.rs:76` の clippy の警告 `clone_on_copy` 1 件・起票の候補（8.2 の 3 件・2.6 の sylphya の件）。

## Verification Result（再検証）
- STATUS: VERIFIED
- CLAIM_TYPE: FEATURE_GO
- CLAIM: areka-P0-mcp-reload の 28 タスクを合わせて、機能として完成している（`61835a65`）
- EVIDENCE:
  - 1 回目の全体テスト（`e8c52efb`）: 10/11 段が緑で、赤は 2 本だけ。
  - その 2 本を `61835a65` で回し直し、緑（6 通過・終了コード 0）。
  - 直しの範囲: 3 ファイル・テストの表と spec 文書だけ。
  - スモーク・網羅・結線・設計・ブロックはすべて通過。
- GAPS: 直した後のコミットで全体テストを通しては回していない（決まりどおり。`/kiro-complete` の全体テストの 1 回で通しを取る）。
- NOTES: `/kiro-complete` へ進める。
