# Brief: areka-P0-mcp-reload

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントは辞書やシェルを書き換えたら、ゴーストを再読み込みして結果を見る。SSP の `reload` は ghost／shiori／shell／balloon／makoto／descript の 6 対象で、同じことは台本の `\![reload,…]` でもできる（ukadoc・MCP の `sakurascript` からも使える）。areka には再読み込みという操作そのものが無い。

## Current State

- 再読み込みは 1 つも無い（`makoto-dll-host` の brief も「reload という概念自体が未実装」と書く）。
- いちばん近いのはゴーストの再起動＝切替の経路（`crates/areka/src/emo2_boot/ghost_switch.rs` の `switch_to`・`boot_into`、完了 `ghost-restart-unit`）。正典の `\![reload,ghost]` は「自分自身へのゴースト切り替えと、SHIORI イベントが発生しないことを除けばほぼ同じ」。
- シェル・バルーンの載せ替えは α の `shell-balloon-switch`（`\![reload,shell|balloon]` を範囲外にしている）が作る。

## Desired Outcome

- 6 対象を 1 つのエンジンの操作として作り、**MCP の `reload` と台本の `\![reload,ghost／shiori／shell／balloon／descript]`（`\![reload,descript,パラメータ]`・旧 `\![reloadsurface]` を含む）の両方**から呼べる。
- 読み込みの失敗はエラーログ（`mcp-log-history` の error 種別）へ出る（SSP: 「Errors while loading are recorded to the error log」）。
- **makoto**: `makoto-dll-host`（α 後）が着地するまでは `NG:` で返してログを出す縮退の口を置き、同 spec の brief へ「MCP の reload makoto はこの口を埋める」と申し送る（記憶 defer-canon-with-full-vocabulary-and-tracking-spec）。

## Approach

`shell-balloon-switch` と切替の経路が作った載せ替えの部品を、「同じものへ載せ替え・SHIORI イベントを出さない」形で呼ぶ。shiori だけの載せ替え（シェルは残す）と descript の読み直しは要件で ukadoc を引いて意味を確定する。

## Scope

- **In**: 5 対象（ghost／shiori／shell／balloon／descript）の再読み込み・`\![reload,…]` の消費者（`\!` の汎用キャリアの台帳へ登記）・MCP `reload` の中身・makoto の縮退の口・ukadoc 網羅台帳の `owner`・決定論テスト。
- **Out**: makoto の本体（`makoto-dll-host`）・`\![reload,aigraph]`（AI グラフは無い）。

## Boundary Candidates

- 載せ替えの部品（既存・`shell-balloon-switch`）と、再読み込みの操作（新）と、2 つの入口（台本・MCP）の境。

## Out of Boundary

- 別のゴースト・シェル・バルーンへの切替（既存）。
- ファイルの変更を見張って自動で読み直すこと。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`・`shell-balloon-switch`（α・載せ替えの部品）。
- **Downstream**: `makoto-dll-host`（makoto の口を埋める）。

## Existing Spec Touchpoints

- **Extends**: `makoto-dll-host`（brief へ申し送りを追記済み・2026-09-29）。
- **Adjacent**: `shell-balloon-switch`・完了 `ghost-restart-unit`・`ghost_session.rs`。

## Constraints

- 既存の終了経路と切替の決定論テストを 1 本も落とさない（roadmap の保存義務）。
- 規模 M〜L。kanade を触る必要が出たら同じウェーブの `mcp-kanade-tools` と重なる＝design で触らない形を先に探し、触るなら `mcp-kanade-tools` の後へ回す（干渉台帳）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。

## 2026-10-03 C4 の候補（10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。C3 に入れなかった理由: `mcp-kanade-tools` と kanade を分け合いうる。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: L（15〜20 タスク・20 をまたぎうる）。またいだときの切り方の案: ⒜ 本 spec＝`ghost`・`shell`・`balloon`・`descript` の 4 対象＋MCP の口＋台本の消費者＋makoto の縮退（14〜17）、⒝ `\![reload,shiori]`（SHIORI だけ載せ替え・シェルは残す）を別 spec（`reload-shiori`・S〜M）へ。⒝ は実行系（`areka-ghost/src/runtime.rs` の SHIORI アクター）を作り直すので触る所が別系統。順は ⒜ → ⒝。
- 前提の状態: `mcp-tool-entrances`（PR#223）・`shell-balloon-switch`（PR#205）は着地済み。前提は満たす。`shell-balloon`（PR#227）も着地済み。
- 崩れた前提／古くなった位置:
  - ダミーの場所: アプリ本体側 `crates/areka/src/mcp/reload.rs` の `handle`（`NG:not implemented yet` の 1 文）と `reload_tests.rs`（同じ文言を期待＝書き換える）。プロトコル側 `crates/areka-mcp/src/tools/reload.rs` は定義と `Args { target: String, ghost_name: Option<String> }` まで完成。`target` の値の検査（`NG:Unknown …`）はアプリ本体側が返す約束（tool-entrances の design）。
  - **「同じゴーストへ載せ替える」既存の道がある**: `network-update` が作った `crates/areka/src/update/desk.rs` の `fn reload` が、`request_ghost_switch(world, SwitchRequest { ghost: GhostSpec::Folder(同じフォルダ), raise_event: false, origin: ChangeOrigin::Automatic, boot_event })`（`emo2_boot/ghost_switch.rs`）で同じゴーストへ切り替えている。`\![reload,ghost]` はこの形に載る。
  - ただし「SHIORI イベントを出さない」には起動側の口が足りない: 起動の由来 `BootOrigin`（`crates/areka-kanade/src/change.rs`）は `Plain`／`ChangedFrom`／`Halted`／`Updated { id, references }` で、**起動の知らせを 1 つも送らない由来が無い**（`Updated` は別のイベントを送る）。降ろす側も `take_down` が `session.shutdown(CloseReason::System)` を呼ぶ。→ **kanade（`change.rs` の `BootOrigin`・`schedule/boot.rs`）を触る見込みが高い**＝kanade の進行の列に入る。
  - シェル・バルーンの載せ替えは `emo2_boot/shell_balloon_switch.rs`（台詞の切れ目で差し替え・背景で資産を組む）と `\![change,shell|balloon]` の消費者（`emo2_boot/consumer_ledger.rs` の `SwitchSink`・`emo2_boot/change_cue.rs`）。`\![reload,…]` の消費者は 0 件（台帳に `reload` の行が無い）。
  - **`\![reload,balloon]` の持ち主が 2 つある**: `balloon-canon-residue` の brief の項目 4 も `\![reload,balloon]` を持つ（「切替の仕組みに載せる」）。本 spec と二重。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/mcp/reload.rs`・`reload_tests.rs`
  - **新規**の消費者（例 `crates/areka/src/emo2_boot/reload_cue.rs`＋兄弟テスト）・`emo2_boot/consumer_ledger.rs`（`reload` の行）・`emo2_boot/mod.rs`（`mod` と系の登録）
  - `emo2_boot/ghost_switch.rs`（同じゴーストへの載せ替えの入口・891 行）・`emo2_boot/shell_balloon_switch.rs`（同じシェル／バルーンの組み直し）
  - `crates/areka-kanade/src/change.rs`（知らせなしの起動の由来）・`schedule/boot.rs`・降ろす側の終了の扱い（`schedule/close.rs` か `msg.rs`）
  - ⒝ を含めるなら `crates/areka-ghost/src/runtime.rs`
  - `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![reload,…]` の行の `owner`
- 議題（答えで作業が変わるものだけ）:
  - `\![reload,balloon]` をどちらが持つか（本 spec か `balloon-canon-residue` か）。片方の brief から外す。
  - `\![reload,shiori]` を本 spec に含めるか ⒝ へ切るか（含めると `runtime.rs` を `mcp-get-property` と分け合う）。
  - 知らせなしの起動を kanade に足すか（足すなら `mcp-kanade-tools` と同時に走らせない）、kanade に触らず UI 側だけで済ませる形があるか。
- 見つけた穴: `\![reload,balloon]` の持ち主の二重（上記）。
- **裁定（棚卸㉑・roadmap の裁定 5）**: `\![reload,balloon]` は本 spec が持つ（`balloon-canon-residue` の項目 4 から移した）。切替の仕組み（`shell_balloon_switch.rs`）に載せる。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模（タスク数）と切るかどうか: L（15〜20）のまま。20 をまたいだときだけ前回の案（`\![reload,shiori]` を別 spec へ）で切る。それ以上は削らない。
- 前提の状態: 満たす。C3 は本 spec の触るファイル（`emo2_boot/{ghost_switch,shell_balloon_switch,consumer_ledger,mod}.rs`・kanade の `src/change.rs`・`schedule/boot.rs`）を 1 行も変えていない（`634032f6..f26aa1c1` の差分 0）。`reload` は今も `NG:not implemented yet` で答える 4 本の 1 つ（`crates/areka/src/mcp/reload.rs` の `handle`）。
- 崩れた前提／古くなった位置:
  - 起動の由来 `BootOrigin`（kanade の `src/change.rs` の列挙の定義）は今も `Plain`／`ChangedFrom`／`Halted`／`Updated` の 4 つ＝知らせを送らない由来は無い（前回どおり）。
  - 同じゴーストへの載せ替えの既存の道は `crates/areka/src/update/desk.rs` の `fn reload` のまま。
  - `\![reload,makoto]` は `makoto-reload-directives` も持つ（同 brief）。本 spec は縮退の `NG:` の口だけ、という分担は変わらない。消費者の台帳 `consumer_ledger.rs` の `reload` の行を 2 本が書くので、後着が行を足し直す。
  - kanade の進行の列では `mcp-kanade-tools` の後ろだが、kanade で触るのは `src/change.rs`（`BootOrigin` の変種 1 つ）と `schedule/boot.rs`（その腕）だけ。`mcp-kanade-tools` が台本つきの結果を `src/change.rs` に置かず、`schedule/boot.rs` にも触らないと約束すれば、**2 本は触るファイルの重なり 0 で並べられる**。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/mcp/reload.rs`・`reload_tests.rs`
  - 新規 `crates/areka/src/emo2_boot/reload_cue.rs`＋兄弟テスト・`emo2_boot/consumer_ledger.rs`（859 行）・`emo2_boot/mod.rs`（883 行）・`emo2_boot/ghost_switch.rs`（891 行）・`emo2_boot/shell_balloon_switch.rs`（442 行）
  - `crates/areka-kanade/src/change.rs`（175 行）・`schedule/boot.rs`（345 行）・降ろす側の扱い（`schedule/close.rs` か `msg.rs`。`msg.rs` に触ると `mcp-get-status`／`mcp-kanade-tools` と重なる）
  - ⒝ を含めるなら `crates/areka-ghost/src/runtime.rs`
  - `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![reload,…]` の行
- 議題（答えで作業が変わるものだけ）: 前回の 2 つ（`\![reload,shiori]` を含めるか・知らせなしの起動を kanade に足すか）。加えて、降ろす側を `msg.rs` に触らずに済ませるか（済めば MCP の kanade の 2 本と並べられる）。
- 見つけた穴: なし。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - `\!` の受け取り手の台帳 `crates/areka/src/emo2_boot/consumer_ledger.rs` は 859 行から 943 行になった（`open-external-tags`・`mcp-author-tools`・`balloon-lifecycle-events` が足した）。うち約 510 行はファイルの中のテストの塊＝行を足す前に兄弟のテストファイルへ出す（1 タスク）。
  - `mcp-author-tools` の `check_script` は同じ台帳を引いて「誰も拾わない `\!`」を答える。`reload` の行を足すと `check_script` の答えも変わる。台帳の行と各受け口の選別が合っていることを固定するテスト `consumer_ledger_agreement_tests.rs` にも行を足す。
  - `ghost-standard-balloon` がバルーンの決め方を「ゴーストの descript の指定 → 記憶 → 同梱」の順にした（`emo2_boot/ghost_switch.rs` は 902 行）。`\![reload,balloon]` は今のバルーンのフォルダを読み直すだけで選び直しはしない、でよいかを要件で確かめる。
  - 降ろす側は kanade に足すものが無いと分かった: 切替の依頼の「イベントを送らない」指定（`ChangeRequest` の `raise_event` が偽）で、`OnGhostChanging` も `OnClose` も送らずに降ろす道が既にある（kanade の `schedule/change.rs` の切替を始める関数）。棚卸㉒の議題「降ろす側を `msg.rs` に触らずに済ませるか」の答えは「済む」。kanade で要るのは、起動の由来（`src/change.rs` の `BootOrigin`・今も 4 種）に「知らせを送らない」1 種と、`schedule/boot.rs` でそれを受ける所だけ。
  - `emo2_boot/mod.rs` は 883 行から 912 行になった。
- 触るファイル:
  - `crates/areka/src/mcp/reload.rs`・`reload_tests.rs`
  - 新規 `crates/areka/src/emo2_boot/reload_cue.rs`＋兄弟テスト（同じゴースト・同じシェル・同じバルーンへ載せ替える入口もここか、その子に置く）
  - `emo2_boot/consumer_ledger.rs`（943）・`consumer_ledger_agreement_tests.rs`・`emo2_boot/mod.rs`（912）・`emo2_boot/ghost_switch.rs`（902）・`emo2_boot/shell_balloon_switch.rs`（442）
  - kanade の `src/change.rs`（175）・`schedule/boot.rs`（345）
  - `\![reload,shiori]` を含めるなら `crates/areka-ghost/src/runtime.rs`（788）
  - 台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![reload,…]` の行・`doc/ssp-mcp/areka-tools.md` の受け取り手の数の 1 行
- 規模: L（17〜22 タスク）。`\![reload,shiori]` を除くと 14〜17、含めると 17〜22。
- 先に要るもの: 働きの前提は着地済み。
  - 同じウェーブに置けない相手: `script-security-level`（`schedule/boot.rs`）・`makoto-reload-directives`・`script-impact-tiers`・`mcp-strict-errors`（`consumer_ledger.rs`）・`shell-companion-balloon`・`balloon-canon-residue`（切替のファイルを挙げている）・`makoto-dll-host`（`runtime.rs`＝`\![reload,shiori]` を含めるときだけ）。
  - `mcp-kanade-tools` とは、向こうが kanade の `src/change.rs`・`schedule/boot.rs` に触らなければ重なり 0。`mcp-strict-errors` より先に着地させる（案内文の「not implemented yet」の 1 文を消す順）。
- 優先度の区分: A（開発者の指示「ssp mcp tool の完全移植のための spec 群を立ち上げて」）。
- 要件定義のモデル: Fable（SHIORI だけの載せ替えと descript の読み直しの意味を ukadoc から決める・切替の最中に届いたときの順序）。
- 分割の案: 今は切らない（20 をはっきり超えてはいない）。要件の段で数えて 20 を超えたら、前から決めてあるとおり `\![reload,shiori]`（実行系の SHIORI だけを載せ替える・`runtime.rs`）だけを別の spec へ出す。それ以上は削らない。
- 見つけた穴・古くなった記述: 棚卸㉒の行数（859・883・891）は古い。`doc/ssp-mcp/areka-tools.md` の手書きの「今 23 組」は受け取り手を足すたびに古くなる（数を書かずコードを指す形に直す候補）。

## 2026-10-10 棚卸㉓の申し送り

- **`consumer_ledger.rs` に触るついでの片付け 1 件**（完了 `mcp-author-tools` の `research.md` の 13 節が残したもの）。`\!` の対応表 `ConsumerLedger::canonical`（`crates/areka/src/emo2_boot/consumer_ledger.rs`）の `bind` の行は、名前を文字列 `"bind"` で直に書いている。拾う側の seriko（`crates/areka-seriko/src/actor.rs`）も自分の中で `"bind"` と直に比べていて、2 つを結ぶ共通の定数が無い。ほかの内部の名前（`FONT_TAG_CARRIER`・`PROP_SET_CUE_NAME`・`JUMP_TAG_CARRIER`）は定数を分け合っている。
- 不具合ではない（今は 2 つの綴りが合っている）。本 spec で対応表に `reload` の行を足すときに、seriko に名前の定数を 1 つ置いて表から引く形へ直す（1 タスクの中で済む大きさ）。seriko のファイルに触りたくないウェーブなら、やらずに次に対応表へ触る spec へ送ってよい。
