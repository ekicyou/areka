# 設計バリデーション: areka-P0-shiori-loadu

> 2026-09-23・本ブランチで `design.md`（確定）を requirements.md・research.md・brief.md・steering・実コードと突き合わせた。非対話（開発者への質問 0 件）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 判定: **GO**（クリティカルイシュー **0 件**・軽微な所見 6 件）。

## レビュー要約

設計は変更を `crates/shiori-host32-helper/src/shiori_proxy.rs` の 1 ファイルに閉じ、判断（`choose_init_entry`）・符号化（`encode_with_codepage`）・呼出（`encode_alloc_and_load`）を別々の関数に切って独立に檻へ入れる形で、要件 1〜9 の全 ID が設計要素に写っている。設計が実コードに置く前提（`resolve` クロージャの順序・`EntryNotFound("load")` を固定する既存テスト・`ansi_encode` の署名・`Drop` の `let _ =`・`#![allow(dead_code)]`・fixture の glob メンバ・文書の現物の形）は全て実物と一致した。実装へ進んでよい。

## 検証した設計の主張（実コードとの突合）

| 設計の主張 | 実物 | 判定 |
|---|---|---|
| `ansi_encode` は `WideCharToMultiByte(CP_ACP, 0, …, PCSTR::null(), None)` を 2 回呼び検出無し | `fn ansi_encode`（`shiori_proxy.rs`）のとおり。`lpUsedDefaultChar` は両方 `None` | 一致 |
| `LoadFn`／`UnloadFn` の戻りは Rust `bool` | `type LoadFn`／`type UnloadFn` の定義行のとおり | 一致 |
| 既存テストが `EntryNotFound("load")` を固定 | `kernel32_yields_entry_not_found`（`mod tests`）が `sym == "load"` を assert | 一致。設計の「`loadu`→`load` を任意で引いて `choose_init_entry` を通し、その後で `unload`・`request`」の順なら kernel32 は `(None, None)`→`"load"` で無改変のまま緑 |
| `Drop` は `let _ = (self.unload)()` | `impl Drop for ShioriByteProxy` のとおり | 一致。`u8` 化しても本文不変 |
| `struct ShioriByteProxy` の `load` 欄は読む者が無い | `crates/shiori-host32-helper/src/*.rs` に `proxy.load` の読み手 0 件（`#![allow(dead_code)]` も冒頭に在る） | 一致 |
| `main.rs` は 0 行変更 | `handle_message` の `InboundAction::TriggerLoad` の枝は `ShioriByteProxy::load(&dll_path, &s.load_dir)` と `load_result_to_ack`（ジェネリック）だけ。署名不変なら触らない | 一致 |
| `windows` 0.62.2 で `MultiByteToWideChar(cp, flags, &[u8], Option<&mut [u16]>) -> i32` が使える | `Globalization/mod.rs` の `pub unsafe fn MultiByteToWideChar` のとおり。ただし第 2 引数は `MULTI_BYTE_TO_WIDE_CHAR_FLAGS`（newtype）で、`WideCharToMultiByte` の `u32` とは違う | ほぼ一致（所見 1） |
| `Win32_Globalization` は helper で有効 | `crates/shiori-host32-helper/Cargo.toml` の `features` に在る | 一致 |
| 往復比較は ACP=932 でも 65001 でも正しい | 932: 既定文字 `?` も best-fit（`U+00A5`→`0x5C` 等）も戻すと元と違うので `lossy=true`、表に在る字は WC→MB→WC で戻る（同じ Unicode に 2 つのバイト列がある字も、符号化が選んだ方を復号すれば同じ Unicode）。65001: `lpDefaultChar`／`lpUsedDefaultChar` は既に NULL なので関数は失敗せず、往復は恒等で `lossy=false` | 正しい |
| `u8` で受ければ Win32 `BOOL`（`i32`）の DLL と Rust `bool` の DLL の両方で安全 | i686 cdecl の戻りは EAX。`i32` の exporter は EAX 全体を書き（AL=下位バイト）、Rust `bool` の exporter は AL だけを保証する（上位 24 bit は不定）。受け側 `u8` は AL しか読まないので両方で定義済みの値になる。逆に `i32` で受けると pasta（Rust `bool`）の不定な上位 bit を読んでしまう＝`u8` が正しい最小の選択 | 正しい（所見 5 に天井を記す） |
| 新 fixture クレートは既存 fixture・`resolve_testdll` 6 か所・host e2e を 0 変更で残せる | ルート `Cargo.toml` は `members = ["crates/*"]`。出力名 `shiori_loadu.dll` は既存の `shiori.dll` と衝突しない。host の `tests/*_e2e.rs`・`src/*.rs` に helper の stderr を読む箇所は 0 件（`Stdio`／`[helper]` の grep 0 件）なので新しい `eprintln!` 2 行が既存 e2e を壊さない。新 env `HOST32_TESTDLL_LOADU_*` は既存 fixture が読まない | 一致（要件 6.8 成立） |
| 兄弟テストファイルの `#[path]` 接続 | steering `structure.md` の「テスト分離の命名規約」（`<stem>_<テーマ>.rs`＋`#[path]` 必須）と一致。`use super::*` で `shiori_proxy` の私有項目に届く（既存 `mod tests` の私有 `resolve_testdll` には届かないので自前で持つ設計は正しい） | 一致 |
| 文書計画が現物どおりに実行できる | COMPAT §8 は「項目・裁量・根拠・出典 spec」4 列の 1 表で末尾追記可。host README「手順（コピペ可）」に `cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc` の行が在り次行追記可。`structure.md`「Test DLL Fixture Crates」節が在る。台帳 `[entry."ukadoc:spec_dll"]` の `note` に「壊れ方:」「ログ:」「根拠の場所:」「内容:」の段落が在り、冒頭注釈「群 14c」も在る。`ukadoc-survey` に `report`／`report-summary` が在り、`report/*.md` は `loadu` を含まない（撮り直しで数字が変わらない見込みは妥当）。`crates/ukadoc-survey/tests/` に `briefing-shiori.md` を読む検査は無い | 実行可能（所見 2） |

## クリティカルイシュー

**0 件**。「既存アーキテクチャとの不整合」「要件の取りこぼし」「実装経路の不明」「不釣り合いな複雑さ」のいずれにも該当する項目は見つからなかった。要件 1.1〜9.6 の全 ID が Requirements Traceability の表に設計要素付きで載り、テストは要件 6 が求める範囲（判断表・UTF-8 固定列・検出の決定論・i686 実読 2 本・既存テスト無改変）を群 A〜D で被覆している。

## 軽微な所見（設計ディスカッションでの即修正候補）

1. **`MultiByteToWideChar` の第 2 引数は newtype。** 設計の疑似コード `MultiByteToWideChar(cp, 0, &bytes, …)` はそのままでは型が合わない（`MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0)` が要る）。`WideCharToMultiByte` の `dwflags: u32` とは違う。実装時に気付く 1 行だが、設計の「Technology Stack」の記述を直しておくと迷わない。
2. **`briefing-shiori.md` の直し先が設計の列挙より 2 か所多い。** 設計は群 14c の「判断の根拠の場所」「共通 `note`」と「足りない物」⑴ を挙げるが、同じ「足りない物」の直前の **「今ある物」**（「`load`・`unload`・`request` の 3 つを名前で引いてから呼んでいる」）と、群 14c 末尾および「ukadoc へのフィードバック候補」4 の **2026-09-06 の書き直しの経緯段落**（「『3 つ』は areka の助け手が名前で引いている入口の数でしかない」）も、実装後は事実と違う文になる。経緯段落は日付付きの記録なので「（2026-09-23 以降は 4 つ全てを引く）」の 1 句を足す程度でよい。
3. **新設 env の数は 2 つでなく 3 つ。** 「Allowed Dependencies」は `HOST32_TESTDLL_LOADU_RECORD`・`_FAIL` の「2 つのみ」と書くが、群 D の `resolve_loadu_testdll` は `HOST32_TESTDLL_LOADU_DLL`（テスト側が読む所在の上書き）も使う。要件 9.2（本番 env 0 追加）には触れないが、数を揃える。
4. **観測の 2 行（要件 3.1・4.1〜4.3）に決定論の檻が無い。** `eprintln!` は同一プロセスのテストでは捕まえられず、host e2e は要件 6.8 で無改変。判定（`lossy`・`InitEntry`）は群 A〜C で固定されるので、行そのものは配線として実機の grep（7.1〜7.3）に委ねるのが本プロジェクトの規律（判断分岐のみ檻に入れる）に沿う。設計の Testing Strategy にその旨を 1 行明記しておくと、実装レビューで「檻が無い」と再び議題にならない。
5. **`u8` 受けの天井を Safety 根拠に書く。** C 製 DLL が `BOOL` として下位バイト 0・上位 bit 非 0 の値（例 `0x100`）を返すと失敗と判定される。正典どおり `TRUE`(1)／`FALSE`(0) を返す限り問題無く、`i32` 受けは pasta（Rust `bool`）で壊れるので選択は正しい。`transmute` の Safety 文にこの天井を 1 文残す。
6. **既存テスト `ansi_encode_mixed_japanese_is_multibyte` は既定コードページが 65001 の機械で元から赤。** 本仕様の変更とは無関係（CP_ACP のバイト列が UTF-8 と一致するため）だが、要件 6.8「無改変で緑」は CP932 機での話であることを実装記録に 1 行残しておく。

## 設計の強み

- **判断・符号化・呼出の 3 分割と `InitEntry` の型設計。** fn ポインタを 1 つしか持たない `InitEntry` により「両方を呼ぶ経路」が型の上で存在せず（要件 1.5）、`choose_init_entry` の 4 行表が DLL 無しで x64 常時の檻に入る（1.8・6.3）。`encode_with_codepage(cp, …)` にコードページを引数で通した結果、機械の既定コードページに縛られない検出テスト（20127／65001）が成立する（6.5）。
- **凍結面を本当に触らない差分設計。** `EntryNotFound("load")` の名札・`ansi_encode` の署名・`ProxyError` の variant・`main.rs`・親との受け渡し・既存 fixture の全てを不変に保ち、2 本目の fixture の戻りを意図的に `i32` にして既存 fixture の `bool` と対にすることで、要件 5.1 の両側（`BOOL` 製と 1 バイト製）を偽 DLL だけで踏める。

## 最終判定

**GO**。既存アーキテクチャとの不整合は無く、要件は全 ID が設計要素とテスト（または実機手順）に写り、実装経路は 1 ファイル＋新クレート 1 つ＋兄弟テスト 1 つに閉じて明確。残るリスクは軽微な所見 6 件で、いずれも設計ディスカッションで数行の修正か実装時の 1 行で片付く。

次の手順: 設計ディスカッション（`kiro-design-discussion`）で所見 1〜3 を design.md に反映し、所見 4〜5 は実装時の doc／Safety 文で拾う。その後 `/kiro-spec-tasks areka-P0-shiori-loadu`。
