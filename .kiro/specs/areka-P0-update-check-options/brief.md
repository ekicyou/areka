# Brief: areka-P0-update-check-options

> 2026-10-04 棚卸㉑で起票（roadmap「覚え書き」の「更新のオプションと `OnUpdateCheck*`」を格上げ）。**段はその他**。前提は `network-update-canon-order`。

## Problem

- **ゴースト作者**: `\![updatebymyself,checkonly]` のように更新にオプションを付けると、areka は何もしない。「新しい版があるか確かめるだけ」を作れず、`OnUpdateCheckComplete` などの 4 語も届かない。
- **ゴースト作者**: シェルやバルーンの更新先を SHIORI から差し替える `other_homeurl_override` に応えない。
- **利用者**: 説明書 `dist/README.txt` の既知の制限に「更新のオプション（確かめるだけ・試すだけ・やり直し）は受けません」「ゴーストがシェル・バルーンの更新先を差し替える仕組みには応えません」と書いてある。
- 正典:
  - `\![update,更新対象(,オプション…)]`（https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdate_2c_66f4_65b0_5bfe_8c61_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1）は「オプション指定はSSPのみ。更新オプション指定を参照。」。
  - `OnUpdateCheckComplete`（https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateCheckComplete:1）は「ネットワーク更新のチェックに成功した際に発生。」。
  - `OnUpdateCheckResult`（https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateCheckResult:1）は「更新チェックのみオプションをつけた時に最後に発生。」。
  - `other_homeurl_override`（https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#other_homeurl_override:1）は「更新対象の更新先URL(homeurl)を強制的に置き換える。」。

## Current State

- 台本の受け口 `crates/areka/src/emo2_boot/update_cue.rs` の定数 `UPDATE_OPTIONS`（`checkonly`・`testonly`・`recovery`）と、接頭辞 `--option=` を見つけると、更新を始めずに記録だけ残す（完了 `network-update` の要件 1.8・10.8＝「α では受けない」）。
- kanade の `crates/areka-kanade/src/schedule/events.rs` の、ネットワーク更新の 19 語の表のコメントは「`checkonly` の系の `OnUpdateCheck*` 4 語・`OnUpdateResultExplorer`・作る側の `OnUpdatedata*` は載せない（要件 2.15）」。
- `other_homeurl_override` は SHIORI に聞いていない。更新先は対象の descript の `homeurl` だけで決まる。
- `network-update-canon-order` の brief は、更新のオプション・`OnUpdateCheck*`・`other_homeurl_override` を「本 spec が持たない物（引受先なし）」と明記している。

## Desired Outcome

- `checkonly` を付けた更新は、取ってこずに確かめるだけで終わり、`OnUpdateCheckComplete`／`OnUpdateCheckFailure` と最後の `OnUpdateCheckResult`（`\![updateother]` なら `OnUpdateCheckResultEx`）を正典の順で送る。
- `testonly`・`recovery` は、正典の意味を確かめた上で、応えるか「応えない」を記録で示すかを決めて実装する。
- シェル・バルーンの更新の前に `other_homeurl_override` を SHIORI に聞き、空でない答えなら更新先を置き換える。
- 説明書の既知の制限から該当の 2 行を消せる。

## Approach

- 受け口（`update_cue.rs`）でオプションを読み、更新の要求（`crate::update::RawUpdateRequest`）にオプションの欄を持たせる。
- 手続き（`crates/areka/src/update/procedure.rs`）で「確かめるだけ」の分岐を作る。エンジン `crates/areka-update/` に「差分を数えて止める」口があるか確かめ、無ければ足す。
- `OnUpdateCheck*` の 4 語を kanade の汎用の入口の表へ足す。
- `other_homeurl_override` は SHIORI の資源の問い合わせ（kanade の `schedule/resources.rs` の集合）へ 1 語足し、手続きが更新先を決める前に聞く。

## Scope

- **In**:
  - `checkonly`・`testonly`・`recovery`・`--option=…` の受け取り（`\![updatebymyself]`・`\![update]`・`\![updateother]` の 3 つ）。
  - `OnUpdateCheckComplete`・`OnUpdateCheckFailure`・`OnUpdateCheckResult`・`OnUpdateCheckResultEx` の送信。
  - `other_homeurl_override` の問い合わせと置き換え。
  - 決定論のテスト（偽の取得先）・網羅台帳の行（`sakura-script.toml`・`shiori.toml`）・`dist/README.txt` の 2 行。
- **Out**:
  - 自動更新・定期の更新（`OnUpdateProcessExec` の `auto`）。
  - `OnUpdateResultExplorer` と、作る側の `OnUpdatedata*`（roadmap「予約」の `createupdatedata`）。
  - 本体の更新 `\![update,platform]`（予約）。
  - `\![execute,install,url,URL,homeurl]`（議題 2 で同居させるか決める）。

## Boundary Candidates

- 台本の受け口（オプションの読み取り）。
- 手続き（確かめるだけの分岐と、送るイベントの順）。
- SHIORI の資源の問い合わせ（`other_homeurl_override`）。

## Out of Boundary

- 更新の発生順序そのもの（MD5 の取り直し・台詞の終わりを待つ・総括の 2 段）＝`network-update-canon-order`。本 spec はその上に「確かめるだけ」の枝を足す。
- 取得と展開の仕組み（`areka-update` の中身）。口を 1 つ足す以上は変えない。

## Upstream / Downstream

- **Upstream**: 完了 `network-update`・完了 `update-engine`・`network-update-canon-order`（同じ手続きの順を組み替えるので先に入る）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: 完了 `network-update` の要件 1.8・2.15・10.8（「α では受けない」とした部分）。
- **Adjacent**: `network-update-canon-order`（`update/`・`update_cue.rs`・kanade の `schedule/steady.rs`）・`balloon-lifecycle-events`／`property-query-channels`（kanade の列）・`mcp-kanade-tools`。

## Constraints

- ukadoc の「更新オプション指定」の節は MCP の snapshot に無い。要件の段で URL と逐語の引用（15 語以内）で中身を確かめてから決める。
- ログの無い失敗の経路を作らない。応えないオプションは `warn!` で名前を残す。
- 常時のテストは x64 と偽の境界で回し、ネットワークへ出ない。テストは兄弟ファイルへ。1 ファイル 1,000 行以下。

## 2026-10-04 棚卸㉑で起票

- **出どころ**: roadmap「覚え書き」の「更新のオプションと `OnUpdateCheck*`」（10-02 登記）と、説明書の既知の制限の 2 行（更新のオプション・更新先の差し替え）。
- **規模**: M（10〜14 タスク）。
- **前提**: `network-update-canon-order`（同じ `crates/areka/src/update/` と `update_cue.rs` を組み替える＝直列）。
- **触るファイル**: `crates/areka/src/emo2_boot/update_cue.rs`・`crates/areka/src/update/{mod,procedure,desk}.rs`・`crates/areka-update/src/`（確かめるだけで止める口）・`crates/areka-kanade/src/schedule/{events,resources}.rs`・`doc/ukadoc-coverage/ledger/{sakura-script,shiori}.toml`・`dist/README.txt`。
- **共有しうる相手**: `network-update-canon-order`（直列）・kanade の列（`mouse-drag-events`・`balloon-lifecycle-events`・`property-query-channels`）・`dist/README.txt` を触る配布の spec。
- **議題**:
  1. `testonly`・`recovery` の意味（ukadoc の「更新オプション指定」の節を URL と逐語で引く）。応えるか、記録だけにするか。
  2. `\![execute,install,url,URL,homeurl]`（https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_7a2e_5225_5d:1 の種別 `homeurl`＝「そのURLをhomeurlとみなしてネットワーク更新を実行する」）を同居させるか。
  3. `other_homeurl_override` を聞くのは、シェル・バルーンだけか、`\![updateother]` の全対象か（正典の Reference0 は「shell,balloon,headline,plugin,languageなど」）。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（10〜14 タスク）。切る: なし。
- 前提の状態: **待ち＝`network-update-canon-order`**（未着手）。同じ `crates/areka/src/update/` と `update_cue.rs` を組み替えるので直列のまま。
- 崩れた前提／古くなった位置:
  - 起票の指し先はすべてそのまま: `emo2_boot/update_cue.rs` の `UPDATE_OPTIONS`（`checkonly`・`testonly`・`recovery`）と接頭辞 `--option=`（受けて記録だけ残す）・kanade の `schedule/events.rs` の更新の表のコメント（`OnUpdateCheck*` 4 語は載せない・要件 2.15）・`dist/README.txt` の「■ 既知の制限」の 2 行（更新のオプション・更新先の差し替え）。`crates/areka/src/update/`・`update_cue.rs`・`crates/areka-update/` は C3 で 0 行。
  - kanade の `schedule/events.rs` は `mouse-drag-events` でドラッグの 2 語が足されて 733 行。本 spec が 4 語を足しても上限には遠い。`schedule/resources.rs`（359 行）は棚卸㉑の PR でコメント 1 行だけ変わった（`SEAM(M2…)` が `SEAM(α 後…)` に）＝`ALLOWED_RESOURCE_IDS` へ 1 語足す形はそのまま。
  - `events.rs` と `resources.rs` は kanade の進行の列のファイル。本 spec は列に名前が無いが、`balloon-lifecycle-events`（`schedule/events.rs`）・`sakura-time-critical`・`property-query-channels`・`script-security-level`（10-05 起票・kanade の列）と同じウェーブに置くなら、表の別の行に足すだけかを着手の前に照合する。
- 触るファイル（並走の照合用）: 起票のまま＝`crates/areka/src/emo2_boot/update_cue.rs`・`crates/areka/src/update/{mod, procedure, desk}.rs`・`crates/areka-update/src/`（確かめるだけで止める口）・`crates/areka-kanade/src/schedule/{events, resources}.rs`・`doc/ukadoc-coverage/ledger/{sakura-script, shiori}.toml`・`dist/README.txt`（「■ 既知の制限」の 2 行）。
- 共有しうる相手: `network-update-canon-order`（直列）・上の kanade の列の spec（`events.rs`）・「■ 既知の制限」を触る配布の列（`winget-manifest-submission`・`release-code-signing`・`install-live-target-hazards`）・`coverage-roadmap-refresh`（台帳）。
- 議題（答えで作業が変わるものだけ）: 起票のまま 3 つ。
- 見つけた穴: なし。
