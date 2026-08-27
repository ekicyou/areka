# Brief: areka-P0-scope-zorder-pinning

`\![set,zorder,スコープID,...]`（バルーン込み b/s 記法対応）によるスコープ窓 Z 順ピン留め＋`\![reset,zorder]`＋descript `seriko.zorder` 読み口。2026-08-27 起票（`/kiro-discovery` Path C・開発者要望）。

## Problem

ゴースト作者（emo2 開発者本人）がスクリプトから「どのスコープの窓がどれより手前か」を確定的に制御できない。現状の areka は W6 `ghost-window-zorder`（PR#107）でバルーン⇄キャラ窓の**同一スコープ内隣接**（案 A Win32 owner）を構造保証したが、**スコープ間の上下関係は「ユーザの操作次第」＝非強制**が要件（同 spec 要件 3）であり、SSP 正典タグ `\![set,zorder]` は未対応（`set,zorder`／`reset,zorder` はリポジトリ全体で 0 件・2026-08-27 grep 実測）。

## ukadoc 正典（2026-08-27 採取・全文は ukadoc MCP）

- **`\![set,zorder,スコープID,スコープID,...]`**（SSP 2.3.77・s,b 記法は 2.7.34）: 指定スコープの窓を**左から順に手前側**へ配置。例 `\![set,zorder,1,0]` → \1 が必ず \0 より手前。
  - **バルーン込み記法**: `\![set,zorder,balloon1,surface1,balloon0,surface0]`／省略形 `b1,s1,b0,s0` → \1バルーン - \1立ち絵 - \0バルーン - \0立ち絵 の順に**厳密制御**。
  - **複数グループ**: スコープ ID の違うタグを複数回実行するとグループが増える（`\![set,zorder,1,0]` `\![set,zorder,3,2]`）。グループ間の相対順は規定なし。
  - **再ペアはエラー**: 既にペアにしたスコープ ID を含むタグの実行はエラー。組み替えは reset してからやり直す。
  - ID 数に上限なし（ただし重い・2〜3 個推奨）。**ゴースト終了まで有効**（永続化なし）。
- **`\![reset,zorder]`**（2.3.77）: 指定を解除し **descript であらかじめ指定された分にリセット**。
- **descript `seriko.zorder,スコープID,...`**（shell descript・2.4）: 「`\![set,zorder,...]` の descript 版。タグを実行しなくてもあらかじめ設定できる」。
- **プロパティ `currentghost.seriko.zorder`**（2.8.78・[SET有効]）: 現在の設定状態の read/write。グループ内カンマ・グループ間セミコロン・手前から順。明示指定モード `s0,b0,s1;s2,b2` と従来数値モード `0,1;2,3` は**排他**（混在不可）。書込は**完全置換**（タグの追加式と異なる）・空文字列で全解除・要素 2 未満のグループは無視。

## Current State

- **ペア機構（拡張元）**: `crates/wintf/src/ecs/window/zorder_pair.rs`（語彙 `KeepDirectlyAbove`:47・`ExpectedOrder`:66・`ReassertZOrder`:124・`OwnerLink`:142・判断純関数 `decide_pair_fix`:335）＋ `zorder_pair_establish.rs:97`（owner 張り）＋ `zorder_pair_maintain.rs:342`（維持系・`InsertSpec` は `HWND_TOP` へ写像 :180-183）＋ sink/diag。現行は**ペア 2 窓専用**（`KeepDirectlyAbove.peer` 単数）＝ N 窓一列のグループ語彙が無い。
- **areka 側結線**: `placement/spawn.rs:533`（バルーンへ `KeepDirectlyAbove` 付与）・`:631-644` `wire_zorder_pair()`（`FrameFinalize` へ establish→maintain chain）。scope→窓の正本は `GhostWindows`（`spawn.rs:291`・`char_window`/`balloon_window`）＝ **`s{n}`/`b{n}` 表記はこの 2 本の引きで即解決**。
- **`\!` 汎用キャリアは転写済み**: `decode_passthrough_bang`（`areka-parsers/src/sakura/decode.rs:313-326`）→ `GenericCommand` アーム（`areka-sakura/src/compile.rs:174-181`）→ `CueCommand::Custom`（`dola/src/cue/command.rs:163-166`）。**parsers/sakura/dola は無改変で `\![set,zorder,...]` が sink まで届く**。消費側の同型実例＝`move_cue.rs:486-556`（開封→名前自己選別→純関数 parse→mpsc）。
- **消費者一意性台帳**: `emo2_boot/consumer_ledger.rs:96-105`（現在 `move`/`bind` の 2 行）。**⚠ 粒度はコマンド名のみ**——`"set"` に単一消費者を登記するとその 1 消費者が全 `\![set,*]` サブコマンドの分配点になる。`sakura-time-directives`（M2 ゲート）の `set,balloonwait` は compile 側 allowlist で層が違い現時点で非衝突だが、粒度は要件で裁定のこと。
- **descript は生転記済み**: `placement/config.rs:103-104` `zorder_raw`（`seriko.zorder` 生転記・実挙動なし）・`:133` で shell KV から取得済み。**キー追加不要＝読む口（parse＋起動時適用）を足すだけ**。⚠ **解釈の食い違いを訂正のこと**: `completed/areka-P0-ghost-window-zorder/brief.md:10` は「SERIKO レイヤ順」と注記したが、ukadoc と `completed/areka-P0-window-placement/design.md:67` は「**`\![set,zorder]` の descript 版＝窓 Z 順**」で確定（ukadoc が正）。
- **プロパティ側は未整地**: `areka-sylphya/src/vocab/dotted.rs` の SET 有効群 21 項に `seriko.zorder` は無く、`currentghost` ルート枝は M1 では NOT_FOUND 縮退・SET 有効群も型シーム予約のみ。**実導出は M1 スコープ外が妥当**（下記 Out）。
- **正典衝突**: `completed/areka-P0-ghost-window-zorder/requirements.md:62-68`（要件 3＝スコープ間の上下関係を強制しない）・design.md:473（スコープ間に owner リンクを張らない）。ukadoc 原文は「ユーザの操作次第」でありスクリプト明示指定を禁じてはいない → **「既定＝非強制／タグ実行後＝当該グループのみピン留め」の二状態化**が想定形（要件裁定事項・COMPAT §8 へ登記。現在 §8 に z-order の行はゼロ）。
- **未消費シーム**: `ReassertZOrder` は供給者 2 系統中 1 つ欠落（vis が再表示後 insert を消費せず着地＝roadmap W6 申し送り⑴）。本 spec のグループ張り直し機構は**このシームの供給者になれる**（一石二鳥候補）。

## Desired Outcome

1. `\![set,zorder,1,0]`（数値モード）と `\![set,zorder,b1,s1,b0,s0]`（明示モード）が ukadoc 意味論どおり動く: 左から手前・複数グループ・再ペアはエラー系（SSP 準拠の扱いを要件で確定）・ゴースト終了まで有効。
2. `\![reset,zorder]` で descript 既定へ戻る（descript 無指定なら全解除＝非強制状態へ）。
3. shell descript `seriko.zorder` が起動時に適用される（`zorder_raw` の読む口）。
4. 既定状態（タグ未実行・descript 無指定）は現行どおり**非強制**＝完了 spec 要件 3 の挙動を変えない。同一スコープ内のバルーン隣接（`KeepDirectlyAbove`）はグループ指定と両立。
5. 判断分岐は純関数化して決定論檻で全網羅（`decide_pair_fix` と同型・共有ハーネス log-capture-kit / temp-path-kit 使用）＋実機サインオフ（実 emo2・実 DPI・有界 auto-exit＋ログ grep）。

## Approach

**案 A（推奨）: ペア機構の上位に「グループ語彙」を新設**。`ZOrderGroup`（手前から順の窓 Entity 列・出所＝タグ/descript の別）を Resource/Component として新設し、維持系がグループ内隣接を `InsertAfter` 連鎖の是正指令（既存 `SetWindowPosCommand` funnel 経由）へ写像。establish/maintain の chain に組み込み、`ReassertZOrder` を張り直しトリガとして共有。sink（`zorder_cue.rs` 新設・`move_cue.rs` 同型）がタグを純関数 parse → UI スレッドへ。
- 却下 **案 B（owner 連鎖でスコープ間を構造保証）**: design.md:473「スコープ間に owner リンクを張らない」の覆しになり、owner 一組は間に窓を挟めない制約（記憶 windows-setwindowpos-insert-after）でグループの自由順と矛盾。重い割に表現力が落ちる。
- 却下 **案 C（毎 tick 無条件 SetWindowPos 強制）**: canonical-not-minimal 違反・`dlp` が確定させた「tick の 98% は表示に変化なし」に逆行。

## Scope

- **In**: `\![set,zorder]`（数値/明示 b,s 両モード・複数グループ・再ペアエラー系）・`\![reset,zorder]`・`zorder_raw` 読む口＋起動時適用・グループ維持系（wintf 語彙＋是正）・consumer_ledger 登記（粒度裁定込み）・COMPAT §8 登記（二状態化＋完了 spec 要件 3 との関係・`seriko.zorder` 解釈訂正）・決定論檻＋実機サインオフ。
- **Out**: `currentghost.seriko.zorder` の実導出（語彙・書式・排他規則を**縮退シームとして登記**し M2 追跡＝defer-canon 4 点セット。sylphya SET 有効群への追加は要件段階で要否裁定）・`\v`／`set,windowstate`／topmost 系（zorder_pair 先送り語彙檻 9 語の既決）・バルーン表示ライフサイクル・窓の位置/寸法。

## Boundary Candidates

- **parse/sink 層**（areka `emo2_boot/zorder_cue.rs` 新設＋ledger）⇄ **グループ語彙・維持系**（wintf `zorder_pair*` 拡張）⇄ **descript 読む口**（`placement/config.rs` の消費側＋spawn 時適用）。3 者は純関数境界で切れる。
- グループ⇄ペアの調停（`KeepDirectlyAbove` とグループ順の合成規則——明示モードでペア隣接が自明に含意される場合・数値モードでバルーンが追随する場合）は wintf 側 1 箇所に閉じる。

## Out of Boundary

- SSP の「大量指定は重い」に対する性能最適化（是正指令の発行数削減は設計内で常識的に・計測 spec 化はしない）。
- グループ間の相対順序の規定（ukadoc に規定なし＝非強制のまま）。
- owner 構造（案 A）そのものの変更。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-ghost-window-zorder`（ペア機構・案 A owner・`HWND_TOP` 正典・sink-observed 証跡方式）・`\!` 汎用キャリア（parsers/sakura/dola 無改変が不変条件）・`GhostWindows`（scope→窓正本）・`completed/areka-P0-test-cage-determinism`（log-capture-kit / temp-path-kit 共有ハーネス）。
- **Downstream**: `emo2-conformance-e2e`（W7・実機一周で本タグの目視確認候補＋W6 申し送り⑴「再表示直後のバルーン隣接」を本 spec の `ReassertZOrder` 供給で先に消化できる可能性）・M2 の `currentghost.seriko.zorder` 実導出。

## Existing Spec Touchpoints

- **Extends**: なし（完全新規・完了 spec への申し送り登記もゼロ＝2026-08-27 実測）。
- **Adjacent**:
  - `present-write-coherence`（W6.95 同居）: ファイル素。弱接触 2 点＝⑴ pwc brief:154 が `zorder_pair_maintain.rs` を**観測対象**（改変せず）として名指し ⑵ COMPAT §8 へ両者追記（行が別）。
  - `balloon-offset-dpi`（W6.95 同居）: ファイル素。**唯一の実質リスク**＝bod が `enqueue_window_set_pos`（`follow/window_move.rs:452-544`・`SWP_NOZORDER` ハードコード）の署名を変える場合のみ同一関数衝突。**本 spec は同 funnel を触らない**（維持系は既存どおり `SetWindowPosCommand` を直接発行）を設計不変条件とする。
  - `areka-P0-sakura-time-directives`（M2 ゲート）: `\![set,*]` サブコマンド粒度の裁定を共有（compile 側 allowlist と sink 台帳の層違いを要件で明文化）。
  - 先送り語彙檻: `zorder_pair_deferred_vocabulary_tests.rs:69-101` と areka 側兄弟——新設本番ファイルは**両肺の `PRODUCTION_FILES` へ同時追加**（:76-79 の規定）。

## Constraints

- Rust 2024・render/window は UI スレッド固定・維持系は `FrameFinalize` 常駐（tick 門とは独立）。dola は語彙フリー＝無改変。typed コマンド新設禁止（汎用キャリア 1 本）。
- ログ無し失敗経路の禁止（error!+Err）・判断分岐のみ檻に入れる・注入 sim time は観測を追い越さない。
- 実機検証の定石: 隣接判定は「最も近い**可視**の隣」（既定 IME 窓が owner 直上に居座る・記憶 windows-default-ime-window-sits-above-owner）・挿入位置に topmost 窓を渡さない（`HWND_TOP` 正典）・絶対パス起動＋`AREKA_APP_SMOKE_EXIT_MS`＋`RUST_LOG` grep。案 A ではクリックで fix は出ない（証跡は sink-observed）。
- 新設ファイルは 1,000 行目安内（追記(79) の漂流に足さない）。file:line 引用は design 前に再実測（本 brief のアンカーは 2026-08-27 実測）。
