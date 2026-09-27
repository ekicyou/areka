# Brief: areka-P0-ghost-change-name-resolution

> 2026-09-26 `/kiro-discovery` 再入（棚卸⑰）で起票。`areka-P0-ghost-shell-balloon-switch`の再測定で想定タスクが 18〜21 本に張り付いたため、**切替先の名前解決**を S の別仕様として切り出した。切る場所は「名前解決」——`ghost-shell-balloon-switch` は名指しの `\![change,ghost,名]` だけを受け、解決できない名前は「該当なし → 無視＋`warn!`」で受ける。本仕様がその腕を `random`／`sequential`／`lastinstalled` の解決に置き換え、`\+`／`\_+` を同じ経路へ繋ぐ。
> 本文の file:line は**起票時の実測値**（2026-09-26・main `13b72893`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: `.nar` を入れた直後に「入れたゴーストに交代します」と言うゴーストの利用者。里々 wiki の定石（`satori:ゴースト切り替え`「インストールしたゴーストに即チェンジまたは呼び出し（ＳＳＰ専用）」）は `OnInstallComplete` から `\![change,ghost,lastinstalled]` を出す。`ghost-shell-balloon-switch` だけでは `lastinstalled` は「該当なし」で黙って（`warn!` だけで）何も起きず、**入れて替える α の一周がゴーストの台本からは完結しない**。

同じ経路の仲間として、`\+`（ランダムに他のゴーストへ切替）・`\_+`（次のゴーストへ切替）・`\![change,ghost,random|sequential]` も今日は何も起きない。`\+`／`\_+` は字句解析で `Bare("+")`／`Bare("_+")` に切れ、`crates/areka-parsers/src/sakura/decode.rs` の `decode_bare` の既定の腕で `Raw` になり、`crates/areka-sakura/src/compile.rs` の catch-all が捨てる（`ghost-shell-balloon-switch` の棚卸⑰の再測定 項目 9）。

## Current State

- 列挙は完了 `areka-P0-baseware-root-layout` の `areka_ghost::catalog`（`BasewareRoot`・`list_ghosts`・`GhostEntry`）が持つ。`random`／`sequential` はその上の純関数 1 本で足りる。
- 「最後にインストールしたゴースト」の記録はどこにも無い（`crates/` で `lastinstalled` は 0 件の見込み・着手時に確かめる）。正典は「SSP を一度終了した場合は無効」＝**プロセスの中だけの記録**でよい（永続化しない）。書く側は `areka-P0-ghost-install`。
- 切替の経路そのもの（`SwitchRequest`・`\![change,ghost]` の受け口 `emo2_boot/change_cue.rs`・kanade の切替の相）は `ghost-shell-balloon-switch` が作る。

## Desired Outcome

1. `\![change,ghost,random]` と `\+` で、**今のゴースト以外**の中から 1 体を選んで切り替わる（正典 `\+`「ランダムに他のゴーストに切り替わる。このスクリプトによる切り替えではSHIORIイベントOnGhostChangingは通知されない」）。候補が 0 体なら無視＋`warn!`。
2. `\![change,ghost,sequential]` と `\_+` で、目録の並びで「次」のゴーストへ切り替わる（末尾なら先頭へ）。`\_+` も `OnGhostChanging` を送らない。
3. `\![change,ghost,lastinstalled]` で、同じプロセスの中で最後に入れたゴーストへ切り替わる。入れていなければ無視＋`warn!`（正典「SSPを一度終了した場合は無効」）。**受け皿（記録の置き場所と書く口）を本仕様が用意し、`ghost-install` はそこへ書くだけ**。
4. 切替の経路は `ghost-shell-balloon-switch` と同じ 1 本（本仕様は名前を名指しへ解決して `ghost-shell-balloon-switch` の経路へ渡すだけ・kanade に触らない）。

## Approach

- `\+`／`\_+` は **areka-sakura の compile で汎用キャリア `\![change,ghost,random]`／`\![change,ghost,sequential]` と同じ命令へ写す**（新しい typed 命令を作らない＝`\!` は汎用キャリア 1 本の原則。消費者台帳 `consumer_ledger.rs` に行を足さずに済む）。decode で `Raw` に落ちる腕を直すか compile で拾うかは設計で決める。
- 名前解決は `areka-ghost` の新ファイル（`catalog.rs` 303 行の隣）に純関数で置く（`resolve_change_target(entries, current, name, last_installed) -> Option<GhostEntry>` の形）。乱数は注入（決定論テスト）。
- `ghost-shell-balloon-switch` の `change_cue.rs` の「解決できない名前」の腕を、この純関数の呼び出しに置き換える。

## Scope

- **In**: `\+`／`\_+` の字句・compile の写し／`random`／`sequential`／`lastinstalled` のゴースト名の解決／`lastinstalled` の受け皿（プロセス内の記録と書く口）／網羅台帳 `sakura-script.toml` の `\+`・`\_+` と `\![change,ghost…]` の備考・生成物
- **Out**: シェル・バルーンの `random`／`lastinstalled`（`shell-balloon-switch` の議題 ⑷。正典 `\![change,shell]` の `lastinstalled` は原文が「最後にインストールしたゴースト」と書く＝`shell-balloon-switch` で読み方を裁定する）／`\![call,ghost,…]`（多重ゴースト・α 後）／`lastinstalled` を書く側（`ghost-install`）／`sequential` の順を「ゴーストエクスプローラの下側」に合わせること（areka に無い）

## Boundary Candidates

- 字句と compile（`areka-parsers`・`areka-sakura`）
- 名前解決の純関数（`areka-ghost`）
- 受け口の腕の置き換え（`crates/areka/src/emo2_boot/change_cue.rs`＝`ghost-shell-balloon-switch` が作る）

## Out of Boundary

- 切替の握手・降ろして起こし直す仕組み（`ghost-shell-balloon-switch`）
- シェル・バルーン切替（`shell-balloon-switch`）
- インストール（`ghost-install`）

## Upstream / Downstream

- **Upstream**: `areka-P0-ghost-shell-balloon-switch`（`SwitchRequest`・`change_cue.rs`・切替の経路）／完了 `areka-P0-baseware-root-layout`（目録）
- **Downstream**: `areka-P0-ghost-install`（`lastinstalled` の受け皿へ書く）・`areka-P0-alpha-release-signoff`（入れて替える一周）

## Existing Spec Touchpoints

- **Extends**: `ghost-shell-balloon-switch` の受け口（解決できない名前の腕）
- **Adjacent**: `areka-P0-shell-balloon-switch`（**同じウェーブで並走する**＝下の Constraints の接触制限を守る）

## Constraints

- **`shell-balloon-switch` と並走する条件（共有ソース 0）**: 本仕様は `consumer_ledger.rs`・`boot_resolve.rs`・`boot_config.rs`・`emo2_boot/mod.rs`・`ghost_session.rs`・kanade・`menu/mod.rs` に**触らない**。触るのは `areka-parsers/src/sakura/decode.rs`（391 行）・`areka-sakura/src/compile.rs`（346 行）・`areka-ghost` の新ファイル・`emo2_boot/change_cue.rs`（`ghost-shell-balloon-switch` が作り、`shell-balloon-switch` は触らない＝`shell-balloon-switch` は別ファイル `switch_cue.rs`）とそれぞれの兄弟テストだけ。網羅台帳 `sakura-script.toml` は `shell-balloon-switch` と同じファイルの**別の行**（`\+`・`\_+` 対 `\![change,shell|balloon…]`）、生成物（`report/*.md`）は手で直さず合流後に生成器で作り直す。着手時に `ghost-shell-balloon-switch` の実物（`change_cue.rs` の名前と形）で接触ファイルを再測定し、重なりが出たら `shell-balloon-switch` の後へ直列に戻す。
- 決定論テスト網羅（乱数は注入・`list_ghosts` の並びは偽の目録で固定）。ログ無し失敗経路の禁止（候補 0・記録なしは `warn!`）。
- 規模 **S（4〜5 タスク）**。要件は Opus で足りる（正典が語を決めており、裁量は `sequential` の順と「今のゴーストを候補から外す」の 2 点）。

## 2026-09-26 申し送り（`ghost-shell-balloon-switch` の要件討議から）

- **「解決できない名前」の腕の所在は `change_cue.rs` ではない。** 名前の突き合わせは目録（`catalog::list_ghosts`＝fs I/O）を読むので、talk スレッドの受け口 `emo2_boot/change_cue.rs` ではなく **UI 側（フレームの相）の取り出し**で行う。受け口は名前を無変形で運ぶだけ（`ghost-shell-balloon-switch` 要件 1.7）。本仕様が置き換える腕は UI 側の取り出しにあり、着手時に実物（取り出しのファイル名と形）で接触ファイルを再測定すること。上の Constraints の `change_cue.rs` の記述はこの申し送りで読み替える。 → 棚卸⑱: 実物は `crates/areka/src/emo2_boot/ghost_switch.rs` の `resolve_switch_target`（下の節）。

## 2026-09-27 棚卸⑱の再測定（main 5a232d2f）

> `ghost-shell-balloon-switch`（PR#192）の着地後。上の節の file:line と「`change_cue.rs` の腕」は再測定前の見立てなので、以下を正とする。

**判定: `shell-balloon-switch` と完全並走できる（触るソースの重なり 0）。** 本仕様の触るソースは 2〜3 ファイル＋兄弟テストに閉じる。`/kiro-start` はいま始められる。

### 崩れた／確かめた前提

1. **置き換える腕の所在は `crates/areka/src/emo2_boot/ghost_switch.rs`（699 行）の純関数 `resolve_switch_target(entries: &[GhostEntry], spec: &GhostSpec) -> Option<SwitchTarget>`**。呼び手は同ファイルの唯一の入口 `request_ghost_switch(world, req: SwitchRequest) -> SwitchVerdict` で、`None` を `warn!(ghost_switch_unknown)`・`SwitchVerdict::NotFound` にしている（降ろさず kanade へ何も送らない）。`change_cue.rs`（`ChangeCueSink`・117 行）は第 2 引数の名前を**無変形**で `ChangeRequestRaw { name, raise_event }` に載せて送るだけ（09-26 の申し送りどおり）。`random`／`sequential`／`lastinstalled` は今日そのまま `GhostSpec::Name("random")` になって `NotFound` に落ちる。`resolve_switch_target` の doc に「今のゴーストも除外しない」と明記されており、`random`／`sequential` は今のゴースト（`BootContext.current.ghost.folder: Option<String>`・`request_ghost_switch` の中で `ctx.current.ghost.folder` として手に入る）を要するので、**本仕様は `resolve_switch_target` の手前で名前を名指し（`GhostSpec::Folder`）へ解く 1 段を足す**か、`resolve_switch_target` の引数に今のフォルダ名と `lastinstalled` の記録を足す。どちらでも呼び手は `request_ghost_switch` の 1 か所。
2. **既存テストの書き換えが 1 本**: `ghost_switch_tests.rs` の `unknown_names_warn_once_and_send_nothing` が `["Nobody", "random", "lastinstalled"]` の 3 つを「該当なし」として固定している。`random` は候補が居れば解けるようになるので、この 3 つ組を `Nobody` だけに縮め、`random`／`lastinstalled` の判断は兄弟の新テストファイル（`ghost_switch.rs` の末尾に `#[cfg(test)] #[path = "ghost_switch_resolve_tests.rs"]` を 1 つ足す形＝既存の 3 つと同型）へ置く。`ghost_switch_tests.rs` は 545 行で足せる余地はあるが、上の形が `shell-balloon-switch` と衝突しない（同じファイルに触るのは本仕様だけなので衝突はしないが、置き場を分けたほうが読みやすい）。
3. **`\+`／`\_+` の落ち方は brief どおり**: `crates/areka-parsers/src/sakura/decode.rs`（391 行）の `decode_bare` の既定の腕 `other => decode_passthrough_bare(other)` が `Instruction::Raw("\\+")` を作り、`crates/areka-sakura/src/compile.rs`（346 行）の catch-all `other => debug!("M-boot 外タグを無視")` が捨てる。写す場所の候補は 2 つで、**decode の `decode_bare` に `"+" => Instruction::GenericCommand { name: "change", raw_args: ["ghost","random"] }`／`"_+" => …["ghost","sequential"]` の 2 腕を足すのが最小**（同じ関数に `"f" => Instruction::Font { args: vec![] }`＝「裸の綴りを角括弧付きと同じ受け皿へ載せる」前例が在る。正典が `\+`＝「`\![change,ghost,random]` と同様」と言い切っているので別名の転記であって意味づけではない）。この形なら `compile.rs` に触らず、`ChangeCueSink` にもそのまま届く。`raise_event` は付かないので `OnGhostChanging` は送られない（正典 `\+`／`\_+` の「このスクリプトによる切り替えでは SHIORI イベント OnGhostChanging は通知されない」と一致）。compile 側で `Raw("\\+")` を文字列で見分ける形は取らない（`Raw` の中身に意味を見るのは転記層の約束に反する）。
4. **`lastinstalled` の受け皿は無い**（製品コードに `lastinstalled` は 0 件・テストの 1 件だけ）。プロセス内の記録でよいので、**`ghost_switch.rs` に `Resource` 1 つ（仮 `LastInstalledGhost(String)`）を定義し、`request_ghost_switch` が `world.get_resource()` で読む**（無ければ「入れていない」＝`warn!`）。書く口は `world.insert_resource(LastInstalledGhost(folder))` の 1 行で、`ghost-install` がそれを呼ぶ。据え付けの結線は要らない（無いときは `None`）。**新しいファイルも、`emo2_boot/mod.rs`・`ghost_session.rs`・`main.rs` への行も要らない**。
5. **`sequential` の並び**: `catalog::list_ghosts`（`crates/areka-ghost/src/catalog.rs`）は「フォルダ名の昇順」を返し、メニューの「ゴースト」枠（`menu/ghost_frame.rs`）もその並びで子を出す。「次」＝この並びの次（末尾なら先頭）で利用者の見えるメニューの並びと一致する＝裁定は要らない。今のゴーストが並びに無いとき（argv でフォルダの外のゴーストを指定して起きた・`folder` が `None`）は先頭へ、を設計で決める。
6. **kanade は触らない（確認）**: 解けた名前は `GhostSpec::Folder` として `request_ghost_switch` → `KanadeMsg::ChangeGhost(ChangeRequest { target: ChangeTarget { name, sakura_name, dir }, origin, raise_event })` に載り、`OnGhostChanging`／`OnGhostChanged` の Ref は kanade が `ChangeTarget` から組む。`\![change,ghost,random,--option=raise-event]` は `ChangeCueSink` が `params[2..]` を option として読むので今日の形のまま通る。
7. **台帳**: `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\+`（`_5c_2b`）・`\_+`（`_5c__2b`）は `absent`・owner 空・B2 で、備考が上の落ち方をそのまま書いている。`\![change,ghost,…]` の行は `implemented`・owner `areka-P0-ghost-shell-balloon-switch` で、備考が「`random`／`sequential`／`lastinstalled` の解決は `areka-P0-ghost-change-name-resolution` の持ち場」と本仕様を名指しし、`same-feature` の link で `\+`・`\_+` を指す。本仕様は `\+`・`\_+` を実装済み（owner＝本仕様）に、`\![change,ghost,…]` の備考の 1 文を「解決済み」へ改める。
8. **純関数を areka-ghost の新ファイルに置く案（上の Approach）は取り下げてよい**: `crates/areka-ghost/src/lib.rs` に `pub mod` 1 行が要り、`shell-balloon-switch` が `lib.rs` に触らない約束（向こうの brief の「3 つの約束」3）で守られてはいるが、`ghost_switch.rs` の中（699 → 760 行程度）に置けば触るクレートが 1 つ減る。目録の型 `GhostEntry`（`identity.folder`・`identity.name`・`dir`）は既に `ghost_switch.rs` が使っている。
9. **数**: `ghost_switch.rs` 699・`ghost_switch_tests.rs` 545・`change_cue.rs` 117・`decode.rs` 391・`decode_tests.rs` 624・`compile.rs` 346・`catalog.rs` 324。

### 触るソースファイル（本仕様・今の実物）

`crates/areka/src/emo2_boot/ghost_switch.rs`（名前の解決・`lastinstalled` の記録の型と読み）・`ghost_switch_tests.rs`（3 つ組の縮め）・兄弟の新テスト `ghost_switch_resolve_tests.rs`（仮）／`crates/areka-parsers/src/sakura/decode.rs`（`decode_bare` の 2 腕）・`decode_tests.rs`。`compile.rs`・`consumer_ledger.rs`・`emo2_boot/mod.rs`・`ghost_session.rs`・`boot_config.rs`・`boot_resolve.rs`・`menu/*`・kanade・`areka-ghost` は **0**。
別枠（重なりに数えない）: `doc/ukadoc-coverage/ledger/sakura-script.toml`（`\+`・`\_+`・`\![change,ghost]` の 3 行＝`shell-balloon-switch` の `\![change,shell|balloon]` とは別の行）・生成物 `report/*.md` と手書きの `briefing.md`／`roadmap-draft.md` の数（**後に main へ入る側が生成器で作り直す**）・`doc/COMPAT_ARCHITECTURE.md` §8（`sequential` の並びと「今のゴーストを候補から外す」の 1 行）・`.kiro/steering/roadmap.md`。

### `shell-balloon-switch` との重なり

**0**。向こうが触るのは `switch_cue.rs`・`frame/switch.rs`・`menu/{shell,balloon}_frame.rs`（新規）と `emo2_boot/{mod,assets,frame,consumer_ledger}.rs`・`ghost_session.rs`・`boot_resolve.rs`・`menu/mod.rs`・`areka-ghost/src/{runtime,catalog}.rs`・kanade `schedule/events.rs`・present・seriko・text。本仕様の 2〜3 ファイルとは 1 つも重ならない。**危ない点は 1 つだけ**: 向こうが `ConfigInputs`／`CurrentGhost`／`GhostDecision` に欄を足すと、`ghost_switch.rs` の `boot_into` の構造体リテラルが壊れて向こうも `ghost_switch.rs` を触ることになる。向こうの brief に「欄を足さない」を約束として書いた。合流の順は問わないが、後に入る側が台帳の生成物を作り直す。

### 想定タスク数・Fable 要否

**4〜6（S のまま）**: ① `decode_bare` の 2 腕とテスト ② 名前の解決の純関数（`random`＝今のゴースト以外から注入した乱数で 1 体・`sequential`＝昇順の次・`lastinstalled`＝記録）と決定論テスト ③ `request_ghost_switch` への組み込み・`lastinstalled` の `Resource`・既存テストの縮め ④ 台帳と §8 とロードマップ ⑤ 実機 1 周（`\+` で emo2 → 他へ・`\![change,ghost,lastinstalled]` は記録を手で入れた World の決定論テストで代える＝`ghost-install` が無いので実機は無理）。**要件は Opus で足りる**（開発者に聞く議題 0。`sequential` の並びはメニューと同じ昇順で決まり、「今のゴーストを外す」は正典 `\+` の「他のゴースト」で決まる。`\![change,ghost,random]` に `--option=raise-event` を付けたときの `OnGhostChanging` は正典どおり送る）。

### `ghost-shell-balloon-switch` からの申し送り（完了 spec の design「Revalidation Triggers」）

- `SwitchRequest`／`GhostSpec` の形を前提にしている spec が他にも在る（`ghost-install`）。本仕様が `GhostSpec` に腕を足す（例 `Random`／`Sequential`／`LastInstalled`）形と、`request_ghost_switch` の手前で `Folder` へ解く形のどちらでもよいが、**`GhostSpec` を変えるなら `ghost-install` の brief へ申し送る**。
- 台帳 `sakura-script.toml` の `\![change,ghost,…]` の備考が本仕様を名指ししている（上の 7）。実装したら備考を追随させる。
- `ghost_switch.rs` の `request_ghost_switch` は「予約が在れば Busy」を先に判定するので、`\+` を連打しても 2 通目以降は `warn!(ghost_switch_busy)` で無視される（本仕様で変えない）。
