# Brief: ghost-call

> **0.1.0 の段**（2026-10-15 `/kiro-discovery`「版の段」）。開発者「複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」。roadmap の「予約」の「多重ゴースト」を、土台の `multi-ghost-runtime` と本 spec の 2 本に分けた。本 spec は、利用者と台本が他のゴーストを**呼ぶ**働きと、呼ぶ・呼ばれる・周りへの知らせのイベントを持つ。下の「推す形」と「議題」は、要件の段で覆してよい。

## Problem

- `multi-ghost-runtime` が入っても、利用者が 2 体目を起こす手段が無い。SSP の利用者は、メニューの「他のゴーストを呼ぶ」で並べる。
- 台本の `\![call,ghost,…]` で相方のゴーストを呼ぶゴーストがある（ukadoc の記述例は `\![call,ghost,Emily/Phase4.5]`）。今の areka は `\![call,ghost]` を知らない命令として扱う。
- 呼ばれたゴーストは `OnGhostCalled` で、呼んだゴーストは `OnGhostCallComplete` で、ほかのゴーストは `OnOtherGhostBooted`・`OnOtherGhostClosed`・`OnOtherGhostChanged` で、互いの出入りを知る。今はどれも送らない。

## Current State（main `46123b00`・起票時の調べ）

- **`\![call,ghost]`**: `crates/areka/src/emo2_boot/consumer_ledger.rs`（943 行）に `("change","ghost")` → `ChangeSink` と、シェル・バルーンの着替えの行は在るが、`("call","ghost")` の行は無い。`emo2_boot/change_cue.rs`（119 行）は change/ghost だけを拾う。`check_script` は表に無い `\!` の名前を「知らない命令」と答える（roadmap「`\!` の名前を足す spec の共有の席」＝表に行を足し `consumer_ledger_agreement_tests.rs` に見本を足す）。
- **名前の解決**: 完了 `ghost-change-name-resolution` が `\![change,ghost,…]` のゴースト名（`lastinstalled`・`random` を含む）を解決する。同 spec の要件は `\![call,ghost,…]` を「多重ゴースト・α 後」として範囲外にした＝解決の関数を使い回せる見込み。
- **イベント**: `OnGhostCalling`・`OnGhostCalled`・`OnGhostCallComplete`・`OnOtherGhostTalk` は本番のコードに無い。`OnOtherGhostBooted` は `crates/areka/src/reference_brain.rs` に文字列で出るだけ。kanade が持つのは切り替えの `OnGhostChanging`／`OnGhostChanged`（`areka-ghost/src/runtime.rs`）。
- **メニュー**: `menu/ghost_frame.rs`（74 行）が切り替えの一覧を作る（チェックは `BootContext.current`）。`crates/areka-sylphya/src/vocab/shiori_resource.rs` に `callghosthistorybutton.caption`・`callghostrootbutton.caption` の語彙だけが在る。
- **MCP**: `get_active_ghost_list` は `multi-ghost-runtime` が全員を返す形にする。

### ukadoc（正典）

- `\![call,ghost,ゴースト名(,--option=raise-event)]`: ゴースト名のゴーストを呼ぶ。`lastinstalled`・`random` も可。`--option=raise-event` を付けると、メニューからの操作と同じく `OnGhostCalling` が通知される（付けなければ通知しない・SSP 2.4.74 以降）。
- `OnGhostCalling`（呼ぶ側・呼ぶ指示のとき）: Reference0＝呼ぶゴーストの本体側の名前・1＝`manual`／`automatic`・2＝ゴースト名・3＝パス。
- `OnGhostCalled`（呼ばれた側）: Reference0＝呼んだゴーストの本体側の名前・1＝呼び出し時のスクリプト・2＝名前・3＝パス・7＝呼ばれたゴーストのシェル名。SSP では 204 なら続けて `OnBoot`。
- `OnGhostCallComplete`（呼んだ側・呼ばれたゴーストが起動スクリプトを終えた後）: Reference0・1・2・7。
- `OnOtherGhostBooted`（無関係なゴースト・`OnGhostCallComplete` と同じとき）・`OnOtherGhostClosed`（複数起動中に他が終了）・`OnOtherGhostChanged`（他のゴーストの間で切り替わった）。

## Desired Outcome

1. 右クリックメニューに「他のゴーストを呼ぶ」の一覧が出て、選ぶと、そのゴーストが今のゴーストの隣で起動する。もう動いているゴーストは選べない（または印が付く・議題 1）。
2. 台本の `\![call,ghost,名前]`（`lastinstalled`・`random` を含む）で同じことが起き、`--option=raise-event` の有無で `OnGhostCalling` の通知が ukadoc どおりに分かれる。
3. 呼ぶ・呼ばれる・周りへの 6 つのイベント（`OnGhostCalling`・`OnGhostCalled`・`OnGhostCallComplete`・`OnOtherGhostBooted`・`OnOtherGhostClosed`・`OnOtherGhostChanged`）が、ukadoc の Reference の並びで届く。`OnGhostCalled` が 204 なら `OnBoot` に落ちる。
4. 呼べなかったとき（名前が無い・SHIORI が起きない）は、呼んだゴーストの台詞で伝わる（メッセージボックスを出さない・既定の失敗の伝え方に揃える）。
5. `check_script` が `\![call,ghost]` を知っている命令として答える。

## Approach

**推す形**（要件の段で覆してよい）: `multi-ghost-runtime` の「席を 1 つ足して起こす」口の上に、メニューと台本の入口を載せる。名前の解決は完了 `ghost-change-name-resolution` の関数を使い回す。周りへの知らせは、席の集まりを回して 1 体ずつ送る（送る順は議題 2）。

## Scope

- **In**: メニューの「他のゴーストを呼ぶ」／`\![call,ghost,…]` と `--option=raise-event`／6 つのイベント／呼べなかったときの伝え方／`check_script` の表と見本／網羅台帳の行（`sakura-script.toml`・`shiori-events` の該当行）・`doc/COMPAT_ARCHITECTURE.md` §8。
- **Out**: 複数の席の持ち方と 1 体の終了・すべて終了（`multi-ghost-runtime`）／`OnOtherGhostTalk`・`\![set,otherghosttalk]`・`\![raiseother]`（0.3.0 の覚え書き「ゴースト同士の掛け合い」）／`OnOtherGhostVanished`（消滅は予約）／呼んだ履歴のメニュー（`callghosthistorybutton`・議題 3）／`activeghostlist` のプロパティ（`property-catalog-lists`）。

## Boundary Candidates

- 呼ぶ入口（メニューの枠・台本の受け取り手）。
- 名前の解決（使い回し）。
- イベントの組み立てと配り先（呼ぶ側・呼ばれる側・周り）。

## Out of Boundary

- 席の集まり・止まる流れ・記憶（`multi-ghost-runtime`）。
- 他のゴーストの台詞を聞く・他のゴーストへイベントを投げる働き。

## Upstream / Downstream

- **Upstream**: `multi-ghost-runtime`（先に着地）・完了 `ghost-change-name-resolution`・完了 `mcp-author-tools`（`check_script` の表）。
- **Downstream**: 0.3.0 の覚え書き「ゴースト同士の掛け合い」・`property-catalog-lists`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `\![…]` の名前を足す spec（`consumer_ledger.rs` の共有の席・互いに直列）・`mcp-reload`（`consumer_ledger.rs`）・タスクトレイの 3 本（windows-reactor 待ち・同じ表に行を足す）。

## Constraints

- `consumer_ledger.rs` は 943 行＝足す前に中のテスト（約 500 行）を兄弟のファイルへ出す（roadmap の上限の欄）。
- メッセージボックスを出さない。失敗は既定の伝え方（ゴーストの台詞）に揃える。
- 決定論テスト網羅・ログ無し失敗経路の禁止。

## 議題（要件の段で決める）

1. もう動いているゴーストをメニューでどう見せるか（出さない・選べない・印を付ける）。
2. 周りへの知らせの順（呼んだ側の `OnGhostCallComplete` と、無関係なゴーストの `OnOtherGhostBooted` は「同じとき」＝どちらを先に送るか）。
3. 呼んだ履歴のメニュー（`callghosthistorybutton`）を本 spec に入れるか。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）
- 前提の変化: 10-15 の起票の後の初めての測定。版の段で **v0.1.0**（`multi-ghost-runtime` の後）。ソースは起票の時から変わっていない。
- 確かめたこと: `emo2_boot/consumer_ledger.rs`（943 行）に `call`・`ghost` の行は無い。`emo2_boot/change_cue.rs` は 119 行。`OnGhostCalling`・`OnGhostCalled`・`OnGhostCallComplete` は本番のコードに無く、`OnOtherGhostBooted` は `reference_brain.rs`（515 行）の検査の文字列 1 か所だけ。`menu/ghost_frame.rs` 74・`menu/mod.rs` 348。語彙は `crates/areka-sylphya/src/vocab/shiori_resource.rs`（291）に在る。
- 触るファイル（見込み）:
  - `consumer_ledger.rs`（先に中のテストを兄弟へ出す）と `consumer_ledger_agreement_tests.rs`・新しい受け取り手（`emo2_boot/` の新しいファイルか `change_cue.rs`）。
  - `menu/{mod,ghost_frame}.rs` と新しい枠。
  - kanade の `src/change.rs`（起動の由来＝今 4 種に「呼ばれた」を足す。`OnGhostCalled` が 204 なら `OnBoot` へ続く）と `schedule/boot.rs`（345）。
  - 周りへ送る所（`multi-ghost-runtime` が作る席の集まり）・台帳 `sakura-script.toml` と SHIORI のイベントの台帳の行・COMPAT §8。
- 規模: 10〜14 タスク。
- 先に要るもの: `multi-ghost-runtime`（働き）。
- 同じウェーブに置けない相手（24 本の中）: `multi-ghost-runtime`・`mcp-reload`（`consumer_ledger.rs`・kanade の `src/change.rs`・`schedule/boot.rs`＝向こうも起動の由来を 1 種足す）・`mcp-strict-errors` と `script-security-level`（`consumer_ledger.rs`）・`baseware-root-list`（`menu/ghost_frame.rs`・`menu/mod.rs`）・`mcp-kanade-tools`（イベントの許可の表 `schedule/events.rs` に足すとき）。
- 優先度の区分: A（開発者の「複数ゴーストの同時起動」）。
- 要件定義のモデル: Opus。イベントと Reference の並びは ukadoc が決め、名前の解決は完了 `ghost-change-name-resolution` を使い回す。議題 2（送る順）は ukadoc の「同じとき」の 2 通りの並べ方で、骨格の分かれ目ではない。
- 分割の案: 切らない。
- 見つけた穴・古くなった記述: 本文に kanade の `src/change.rs`・`schedule/boot.rs`（`OnGhostCalled` から `OnBoot` への落ち方）が無い＝`mcp-reload` と直列になることが書かれていない。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 版の段で「v0.1.0 の余力の枠」の 2 本目（`multi-ghost-runtime` の後）。`mcp-reload` が着地した（10-22）＝棚卸㉕の「`mcp-reload` と直列」は解けた。
  - あちらは kanade の起動の出どころ `BootOrigin`（`crates/areka-kanade/src/change.rs`・188 行）に読み直しの 1 種 `Reloaded` を足した。この spec は同じ型に「呼ばれた」を 1 種足す＝前例の形に沿って足すだけ。
  - `emo2_boot/consumer_ledger.rs` は 943 → **472 行**に割られた（テストは兄弟の `consumer_ledger_tests.rs` 555 行・`consumer_ledger_agreement_tests.rs` 385 行）。本文の Constraints の「足す前に中のテストを兄弟へ出す」は済んでいる。
  - `\![call,ghost]` の行は今も無い。`OnGhostCalling`・`OnGhostCalled`・`OnGhostCallComplete`・`OnOtherGhostClosed`・`OnOtherGhostChanged` は本番のコードに 0 件、`OnOtherGhostBooted` は `crates/areka/src/reference_brain.rs`（515 行）の検査の文字列 1 か所だけ（変わらず）。
- 触るファイル: `consumer_ledger.rs`（行 1 つ）と `consumer_ledger_agreement_tests.rs`（見本 1 つ）・受け取り手（`emo2_boot/change_cue.rs` 119 行か新しいファイル）・`menu/mod.rs`（368）と `menu/ghost_frame.rs`（75）と新しい枠・kanade の `change.rs` と `schedule/boot.rs`（379）・周りへ送る所（`multi-ghost-runtime` が作る席の集まり）・網羅台帳 `sakura-script.toml` と `shiori.toml` の行・`doc/COMPAT_ARCHITECTURE.md` §8。
- 規模: 10〜14 タスク（変わらず）。
- 先に要るもの: `multi-ghost-runtime`（働き・未完了）。
- 同じウェーブに置けない相手（v0.1.0）: `consumer_ledger.rs` を触る `mcp-strict-errors`・`script-security-level`・`stay-on-top-directives`・`window-link-directives`・`script-small-tags`・`descript-window-keys`（表の別の行でも同じファイル）／kanade の `change.rs`・`schedule/boot.rs` を触る `script-security-level`・`mouse-click-wheel-events`／`menu/mod.rs` を触る `popup-menu-shell-look`。網羅台帳は別の行なら並べてよいが、行の状態を動かすときは `briefing.md` の数を同じ時に書き換える相手と照らす。
- 1 本だけの席: 取らない。人の待ち・静かな机・前後の全体テストの比べ: 要らない（実機で 2 体目を呼ぶ確かめは AI が回せる）。
- 優先度の区分: A（開発者「複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」）。
- 要件定義のモデル: Opus（イベントと Reference の並びは ukadoc が決め、名前の解決は完了 `ghost-change-name-resolution` を使い回し、起動の出どころへの 1 種の追加は `Reloaded` の前例に沿う。送る順の議題 2 は ukadoc の「同じとき」の並べ方の選びで、骨格の分かれ目ではない）。
- 分割の案: 切らない。
- 見つけた穴・古くなった記述: 本文の「Current State」の `consumer_ledger.rs`（943 行）と Constraints の「テストを兄弟へ出す」は古い（済んだ）。本文の Adjacent の `mcp-reload` は着地済み。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: 10-22 から、この spec の働きの部分は変わっていない。`\![call,ghost]` の行は受け取り手の台帳（`emo2_boot/consumer_ledger.rs`）に今も無く、呼ぶ・呼ばれる・周りの 6 イベントは本番のコードに 0 件（`OnOtherGhostBooted` は `crates/areka/src/reference_brain.rs` の検査の文字列 1 か所だけ）。網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` には `OnGhostCalling` などの行が既に在る（状態を動かすだけ）。台帳は `window-link-directives`（10-23）が行を足して 472→496 行。kanade の起動の出どころ `BootOrigin`（`crates/areka-kanade/src/change.rs` 188 行）は変わらず、前例の `Reloaded`・`Updated` に沿って「呼ばれた」を足せる。`script-security-level` が足した、出どころで止めるタグの表（`emo2_boot/script_restriction.rs`）に `\![call,ghost]` は無い。
- **触るファイル**: 棚卸㉘のとおり（`consumer_ledger.rs` の行 1 つ・`consumer_ledger_agreement_tests.rs`（403）の見本 1 つ・受け取り手（`emo2_boot/change_cue.rs` 119 行か新しいファイル）・`menu/mod.rs`（373）と `menu/ghost_frame.rs`（75）と新しい枠・kanade の `change.rs` と `schedule/boot.rs`（381）・周りへ送る所・網羅台帳の行・`doc/COMPAT_ARCHITECTURE.md` §8）。
- **規模**: 10〜14 タスクのまま。
- **先に要るもの**: `multi-ghost-runtime`（働き・未完了。分けたときは後半の「止まり方と記憶」）。同じウェーブに置けない相手（未完了）: `consumer_ledger.rs` の `stay-on-top-directives`・`script-small-tags`・`descript-window-keys`・`shell-scale-linkage`・`balloon-communicatebox`・`mcp-strict-errors`（案 ⑵）／kanade の `change.rs` の `ondestroy-on-exit`・`network-update-canon-order`・`property-query-channels`／`schedule/boot.rs` の `mouse-click-wheel-events`／`menu/mod.rs` の `balloon-communicatebox`・`popup-menu-shell-look`・`popup-menu-residue`・タスクトレイの 2 本／`menu/ghost_frame.rs` の `menu-thumbnail-preview`。1 本だけの席・人の待ち・静かな机・全体テストの比べは要らない。
- **優先度の区分**: A のまま（開発者「複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」）。
- **要件定義のモデル**: Opus のまま（イベントと Reference の並びは ukadoc、名前の解決は完了 `ghost-change-name-resolution` の使い回し、起動の出どころの 1 種は前例どおり。周りへ送る順は ukadoc の「同じとき」の並べ方の選びで、新しい順の保証は `multi-ghost-runtime` の後半が持つ）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**: 棚卸㉘の「同じウェーブに置けない相手」の `script-security-level`・`window-link-directives` は 10-23 に着地した（外す）。`consumer_ledger.rs` の行数（472）は 496 に読み替える。

## 2026-10-30 `startup-install-option` からの申し送り
- 起動オプション `--option callghost`（`--install` で入れた最後のゴーストを起こす）は、いまはメニューの「ゴースト」と同じ切り替え（`GhostSpec::Folder`・知らせあり・Reference1 `manual`）で今のゴーストと入れ替える（`crates/areka/src/install/startup.rs` の `callghost_request`・要件討議 議題 1）。
- 正典の「呼び出される」は 2 体目として呼ぶ形なので、この spec で 2 体目を呼ぶ働きが着地したら、`callghost` をその形（今のゴーストを残して入れたゴーストを呼ぶ）へ移すこと。移すときは `startup.rs` の `callghost_request` と `decide_call_ghost`、`dist/README.txt` の `--option callghost` の行を直す。
