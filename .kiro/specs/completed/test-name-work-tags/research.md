# ギャップ分析: test-name-work-tags

- 測った木: `claude/kiro-start-test-name-work-tags-c34897` の HEAD `5088b891`（main `00877299` に spec の初期化のコミット 1 つを足したもの・コードは main と同じ）。測った日 2026-11-22。
- 測り方: `git grep`（作業の木の中）と、名の一覧から新旧の名を作って比べる python の使い捨ての式（scratchpad に置いた・コミットしない）。テストは走らせていない。

## 1. 分析の要約

- 札つきのテストの名は要件どおり **28 ファイル・282 本**（`fn t_[a-z]+[0-9]+[a-z]?_`・本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本を除く）と、数字を持たない札の 2 本（`t_zbc_`・`t_zic_`）。札を外した後の名が同じファイルの中でぶつかる組は **0**、本番の関数や他の識別子とぶつかる名も **0**。別のモジュールで同じ名になる組は要件 1.3 の 4 組のほかに **2 組**ある（`t_zbc_`／`t_zic_` の 1 組と、`t_zcc09_` と札の無い既存のテストの 1 組）。
- 名を文字列で並べるのは本体の `placement_tests/` の 2 ファイルの表だけで（68 の名・`check_table` が `"\n#[test]\nfn {name}("` の字面で照合）、`crate_source!` で他の crate のテストのファイルを読む。表が読む 10 ファイルの改名は、表の直しと同じコミットに入れる必要がある。tools/・.github/・nextest の設定・スキルに、テストを名で選ぶ所は無い。
- 要件が挙げていない名指しが見つかった: **札だけの短い書き方**（`t_zgp20`・`t_zpd10`・`t_k5` ほか）が 24 行、`t_` の無い札（`zwi01／02` ほか）が 8 行、大文字の作業番号（`T-K5`・`T-R8`・`T-N10` ほか）が 70 行。要件 5.3 の検索（古い名の全体）はこれらを拾わない。
- 実装の形は「改名表を 1 つ作り、それで全体の名を機械で置き換え、短い書き方だけを手で直す」形が最も少ない手数で漏れが無い。規模は S〜M、危険度は低い。
- 要件の段で決めることは 8 件（本書 7 節）。中心は、短い書き方と大文字の作業番号をこの spec で直すかと、境界の外のファイルに残る名指しの扱い。

## 2. 今の状態

### 2.1 札つきの名の数え（要件 2・5.4 の「改める前の数え」）

`git grep -cE "fn t_[a-z]+[0-9]+[a-z]?_" -- crates` ＝ **29 ファイル・284 本**（brief と同じ）。内訳は brief の Current State と 1 本も違わない。このうち本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本（`t_zwi05_…`・`t_zwi08_…`）は境界の外で、この spec が改めるのは 28 ファイル・282 本。

数字を持たない札の形 `fn t_[a-z]+_`（数字を持つ形を除く）の当たりは 4 つ:

| 場所 | 綴り | 扱い |
|---|---|---|
| `crates/areka/src/placement_tests/zorder_group_branch_coverage_tests.rs:377` | `t_zbc_the_table_covers_every_branch_with_executed_tests` | 改める（要件 1.5） |
| `crates/areka/src/placement_tests/zorder_inversion_branch_coverage_tests.rs:391` | `t_zic_the_table_covers_every_branch_with_executed_tests` | 改める（要件 1.5） |
| `crates/areka/src/placement_tests/zorder_branch_coverage_test_support.rs:79` | `t_only_in_a_comment`（文字列の中） | 改めない（検査の見本） |
| 同 `:85` | `t_real_one`（文字列の中） | 改めない（検査の見本） |

同じファイルの 105〜117 行には `"t_one"`・`"t_gone"` の文字列もある（`fn` が前に無いので上の式は当たらない・これも見本）。

札の種類は 200 を越える（`c1`〜`c5`・`k1`〜`k9`・`r1`〜`r8`・`rs01`〜`rs11`・`zgp1`〜`zgp28`・`zdb01`〜`zdb18`・`zwo01`〜`zwo11` ほか）。`c1`・`c2`・`i4`・`r4`・`r8`・`v1` などは 1 つの札を複数のテストが持つ（例: `t_c2_` は 7 本）。

### 2.2 札を外した名のぶつかり

改名の表（282＋2 本）を作り、次を確かめた。

- **同じファイルの中**: 札を外した名どうしの重なり 0。札を外した名と、同じファイルの他の `fn` の名（テストの道具・本番の関数）の重なり 0。`const`・`static`・`struct`・`enum`・`mod`・`type`・`let` の名との重なり 0。
- **同じ crate の中の識別子**: 札を外した名が、同じ crate の `src/` のどこかに識別子として既に在るのは 1 つだけ（下の表の ⑹）。`use super::*` で取り込んだ本番の関数を、改名したテストの名が隠す形は 0。
- **Rust の予約語**になる名・数字で始まる名: 0。いちばん短い新しい名も 12 字を越える（短いのは検査の見本の `t_real_one` → `one` だけで、これは改めない）。

別のモジュールで同じ名になる組（コンパイルは通る・`cargo test <名>` の絞り込みで両方が走る）:

| # | 新しい名 | ファイル（古い名の札） | 要件 1.3 に在るか |
|---|---|---|---|
| ⑴ | `executing_scope_does_not_change_the_directive` | `areka-runtime` の `window_state_cue_tests.rs`・`zorder_cue_tests.rs` | 在る |
| ⑵ | `disconnected_receiver_is_recorded_and_does_not_panic` | 同上 | 在る |
| ⑶ | `clone_reaches_the_same_receiver` | 同上 | 在る |
| ⑷ | `version_moves_only_when_the_contents_move` | `areka-placement` の `zorder_group_ledger_state_tests.rs`（`t_zgl15`）・`zorder_group_ledger_rebase_tests.rs`（`t_zrb9`） | 在る |
| ⑸ | `the_table_covers_every_branch_with_executed_tests` | 本体の `placement_tests/zorder_group_branch_coverage_tests.rs`（`t_zbc`）・`zorder_inversion_branch_coverage_tests.rs`（`t_zic`） | **無い** |
| ⑹ | `unassigned_scopes_join_behind_every_group_in_ascending_scope_order` | `areka-placement` の `zorder_chain_compose_tests.rs`（`t_zcc09`・純関数の檻）と、札の無い既存の `zorder_chain_tail_order_tests.rs:167`（実窓のテスト）。両方とも `zorder_chain_compose.rs` の子のモジュール（兄弟） | **無い** |

⑷ は名指しにも効く: `zorder_group_ledger_state_tests.rs:20` の doc コメントは `t_zgl15_…` を、本番の `zorder_group_ledger.rs:667` のコメントは `t_zrb9` を名指しし、今は札で 2 本を見分けている。札を外すと同じ名になるので、名指しの側でファイルを添えないと、どちらのテストかが読めない。

⑹ は、同じ鎖の後ろの並びを「台帳と合成の純関数」と「実窓」の 2 段で確かめる兄弟で、中身が違う。同じ名にすると `cargo test unassigned_scopes_join…` で実窓のテスト（`zorder_chain_tail_order_tests.rs`）も走る。

### 2.3 名を名指す所（古い名の全体で当たるもの）

`git grep -nwF`（改める 284 本の古い名の全体・歴史の場所と本 spec の文書を除く・`fn` の定義の行を除く）の当たり:

| 場所 | 行数 | 中身 |
|---|---|---|
| `crates/areka/src/placement_tests/zorder_group_branch_coverage_tests.rs` | 50 | 表（`COVERAGE`）の文字列 |
| `crates/areka/src/placement_tests/zorder_inversion_branch_coverage_tests.rs` | 19 | 表の文字列 |
| `crates/areka-placement/src/zorder_property_deferral_tests.rs` | 9 | doc コメントのリンク（6・20・23・25・27・29・77・123・126 行） |
| `crates/areka-placement/src/spawn_assembly_tests.rs:154` | 1 | doc コメント |
| `crates/areka-placement/src/resolver_balloon_keyword_tests.rs:54` | 1 | doc コメントのリンク |
| `crates/areka-placement/src/zorder_group_ledger_state_tests.rs:20` | 1 | `//!` のリンク |
| `crates/areka-placement/src/zorder_group_ledger_tests.rs:357` | 1 | doc コメントのリンク |
| `crates/areka-runtime/src/frame/zorder_drain_projection_tests.rs:531` | 1 | リンクの定義行（`[`t_zdp10`]: t_zdp10_…`） |
| `doc/COMPAT_ARCHITECTURE.md` | 3 | 226 行（`t_zdb13`〜`t_zdb17` の 5 本）・354 行（`t_rs01`・`t_rs02`・`t_rs05` の 3 本）・412 行（`t_c3`・`t_l2`・`t_rs01` の 3 本と「旧 `t_c3_char_n_prefix_adds_scope_n`」） |
| `.kiro/specs/extra-character-windows/brief.md:133` | 1 | 3 本 |
| `.kiro/specs/zorder-property/brief.md:94` | 1 | `t_zpd40_…` |

要件と brief が挙げた所と一致する。ただし **`zorder-property/brief.md` は 94 行が古い名の全体を持つ**（要件 4.5 が挙げる 59・81 行は札だけの短い書き方・次の 2.4）。steering・`doc/` のほかのファイル・`tools/`・`.github/`・`.claude/` に当たりは無い。

表の照合（`zorder_branch_coverage_test_support.rs` の `check_table`）は、表の名ごとに、`crate_source!` で読んだテストのファイルから注釈を落とした本文に `"\nfn {name}("` と `"\n#[test]\nfn {name}("` が在るかを見る。表が読む札つきのファイルは 10 本（`areka-placement` の `zorder_group_ledger_tests.rs`・`zorder_group_ledger_state_tests.rs`・`zorder_group_ledger_written_order_tests.rs`・`zorder_chain_compose_tests.rs`・`zorder_chain_compose_written_order_tests.rs`、`areka-runtime` の `zorder_cue_tests.rs`・`frame/zorder_descript_tests.rs`・`frame/zorder_drain_tests.rs`・`frame/zorder_drain_projection_tests.rs`・`frame/zorder_drain_written_order_tests.rs`）。表に在る札つきの名は 68。この 10 本のどれかを改めたコミットで表を直さないと、本体のテストが赤になる（要件 4.1 の「途中のコミットでも赤にならない」が実際に効く所）。

### 2.4 要件が挙げていない名指し（札だけの短い書き方ほか）

要件 5.3 の検索は「古い名の全体」で、次の 3 つの書き方を拾わない。札が名から消えると、これらは何も指さなくなる。

**(a) 札だけの短い書き方 `t_<札>`（後ろに名が続かない）— 24 行**（`git grep -nP "\bt_[a-z]+[0-9]+[a-z]?(?!_[a-z])\b"`・`areka-sakura` の局所変数 `t_wait1`・`t_wait2` を除く）

| 場所 | 行 | 札 |
|---|---|---|
| `crates/areka-placement/src/zorder_group_ledger.rs`（**本番のファイル**のコメント） | 667 | `t_zrb9` |
| `crates/areka-placement/src/zorder_group_ledger_tests.rs` | 8・9・10・24・25・315・472 | `t_zgp1`〜`t_zgp14`・`t_zgp18`〜`t_zgp20`・`t_zgp22`〜`t_zgp25`・`t_zgp27`・`t_zgp28`（**範囲の書き方**）・`t_zgp20`・`t_zgp22`・`t_zgp28`・`t_zwo06`・`t_zwo11` |
| `crates/areka-placement/src/zorder_group_ledger_written_order_tests.rs` | 167 | `t_zgp3` |
| `crates/areka-placement/src/zorder_property_deferral_tests.rs` | 268・307・308・453・487 | `t_zpd12`・`t_zpd10`・`t_zpd30` |
| `crates/areka-runtime/src/consumer_ledger_test_support.rs` | 28 | `t_zcs1／t_zcs2` |
| `crates/areka-runtime/src/frame/zorder_drain_projection_tests.rs` | 522・531 | `t_zdp10`（リンクの見出し） |
| `crates/areka-runtime/src/frame_schedule_tests.rs` | 14 | `t_zwi06` |
| `crates/areka-runtime/src/window_state_cue_tests.rs` | 324 | `t_zcs14` |
| `crates/areka-runtime/src/zorder_cue_tests.rs` | 148 | `t_zcs4` |
| `crates/areka/src/runtime_tests/zorder_wiring_tests.rs`（**境界の外**） | 11 | `t_zwi06`（`areka-runtime` の改める側のテスト） |
| `doc/COMPAT_ARCHITECTURE.md` | 196 | `t_k5` |
| `.kiro/specs/zorder-property/brief.md` | 59・81 | `t_zpd40`（要件 4.5 が挙げる所） |

短い書き方の札は、どれも 1 本のテストだけが持つ札（`c2` のような複数持ちの札は短い書き方で使われていない）なので、札 → 新しい名の全体は 1 対 1 に引ける。範囲の書き方（`t_zgp1`〜`t_zgp14` ほか）だけは名の全体へ置き換えられず、文を書き直す必要がある。

**(b) `t_` の無い札 — 8 行**

- `crates/areka-runtime/src/zorder_wiring_tests.rs` 7・8・11・18 行: `zwi01／02`・`zwi06／07`・`zwi09`（同じファイルの改める側）と `zwi05`・`zwi08`（本体の境界の外の 2 本）。
- `crates/areka/src/runtime_tests/zorder_wiring_tests.rs`（**境界の外**）4・5・10・13 行: `zwi05`・`zwi08`（同じファイルの 2 本）と `zwi01／02`・`zwi06／07`・`zwi09`（`areka-runtime` の改める側）。

**(c) 大文字の作業番号 `T-<札>` — 70 行**（`git grep -nE "\bT-[A-Z]{1,4}[0-9]{1,2}[a-z]?\b" -- crates`）

| 場所 | 行数 |
|---|---|
| `crates/areka-placement/src/resolver_balloon_keyword_tests.rs` | 23（`T-K1`〜`T-K9`・`T-R8`。節の見出しのコメントと doc コメントの頭） |
| `crates/areka-placement/src/resolver_resolve_tests.rs` | 22 |
| `crates/areka-placement/src/resolver_resolve_balloon_tests.rs` | 10 |
| `crates/areka-placement/src/resolver_resolve_free_tests.rs` | 8 |
| `crates/areka-placement/src/resolver_union_tests.rs` | 4 |
| `crates/areka-runtime/src/zorder_wiring_tests.rs:182` | 1（`T-N10`＝`frame_schedule_tests.rs` の `t_n10_…` 2 本を指す） |
| `crates/areka-emo-present/src/presenter_dpi_scale_tests.rs:24`・`presenter_test_support.rs:229` | 2（`T-N3`・この spec の範囲の crate の外） |

(c) は名ではなく、設計の作業文書のテストの番号を注釈に書いたもの。`CLAUDE.md` の `## Comments`（spec・要件・タスクの番号をコードに書かない）に同じ理由で当たる。札を名から外すと、`T-K5` と `fn k5_…` を結ぶ手がかりも消える（今は `T-K5` → `t_k5_…` と字面で結べる）。

`doc/COMPAT_ARCHITECTURE.md:412` の「テスト T-C3」は、完了 spec の設計の番号を歴史として書く文で、テストの名の名指しではない（要件 4.8 の「旧 `t_c3_…`」と同じ扱いにできる）。

### 2.5 名で選ぶ仕組み・名に依る作り

- `tools/`（`load-flake.ps1` は `-Filter` を外から受け取るだけ）・`.github/workflows/`・`.claude/agents/`・`.claude/skills/` に、改める名やその札で絞り込む所は無い。nextest の設定ファイルは無い。
- 改める 28＋2 ファイルに、テストの名から一時フォルダやログの名を作るもの（`module_path!`・`function_name`・`thread::current().name()`）は無い。`#[ignore]`・`#[should_panic]` も無い。マクロで名を作る所（`macro_rules!`・`concat!`・`stringify!`）も無い。スナップショットのファイル（`.snap`）も無い。
- `config.rs` は本番のファイルで、311 行目までが本番、`#[cfg(test)] mod tests`（317 行〜）の中に札つきの 20 本がある。`mod limit_tests;`（314 行・`config_limit_tests.rs`）の宣言は名を変えない。
- 改める前後の長さ: いちばん長い新しい名は 77 字。名が短くなるので 1,000 行を越えるファイルは出ない（いちばん長い `zorder_chain_compose_tests.rs` が 720 行）。ただし表の行は、名が短くなると `rustfmt` が複数行の組を 1 行へ畳むことがある（整形の段のため、改名の後に `cargo fmt` が要る・差分は行の畳みを含む）。

### 2.6 前の spec の型（使い回せるもの）

- テストの名の一覧の比べ方: `session-module-gather` の `baseline-test-names.txt`（`cargo test --workspace --no-fail-fast -- --list | grep ': test$' | tr -d '\r' | sed -E 's/: test$//; s/^.*:://' | LC_ALL=C sort`）と、`runtime-module-untangle` の `test-name-tails-before.txt`／`-after.txt`。この spec では、改める前の一覧を改名表で写したものと改めた後の一覧が一致することで、要件 3.2（本数が同じ）と「名だけが変わった」を同時に示せる。
- 0 件の主張の較正（要件 5.4）: 同じ式を改める前の木（`00877299`）へ当てて 284 を出す形は、これまでの spec の「0 件を書く前に較正」の型と同じ。

### 2.7 並走の相手との重なり（要件 6）

- `session-crate-split`: 本体の `runtime_tests/zorder_wiring_tests.rs` を直す。この spec はそのファイルに触らない約束（要件 6.1）。ただしそのファイルの 4・5・10・11・13 行は、`areka-runtime` 側の改める名（`t_zwi06`・`zwi01／02`・`zwi06／07`・`zwi09`）を名指している（2.4 の (a)(b)）。触らないと、この spec の着地の後もそのファイルに古い札が残る。
- `dola-cue-test-builder`: 触るファイルの中に `crates/areka-runtime/src/consumer_ledger_test_support.rs` がある（向こうは 126 行からの `carrier_cue`・`actor_carrier_cue`）。この spec が触りうるのは同じファイルの 28 行のコメント（`t_zcs1／t_zcs2`）だけで、約 100 行離れた別の塊。要件 6.2 はこのファイルを挙げていない。
- `zorder-property`・`zorder-chain-residue`・`extra-character-windows`: 3 本とも今は brief だけで、作業の木も無い（要件 6.4 の「走っていれば後に回す」は今は当たらない・着手のときに見直す）。
- `doc/COMPAT_ARCHITECTURE.md` §8: 名の行 226・354・412 に加え、短い書き方の 196 行（`t_k5`）がある。196 行が `session-crate-split` のパスの行と重なるかは、向こうの差分を見ないと分からない（向こうのパスの行は本体の `crates/areka/src/session/…` を指す行で、196 行は `crates/areka-placement/src/resolver.rs` を指す・重なる見込みは低い）。

## 3. 要件と今の資産の対応

| 要件 | 今の資産・状態 | 区分 |
|---|---|---|
| 1.1 札を外す | 機械で外せる（`t_` ＋英小文字と数字の続き＋`_`）。284 本の改名表は作れる | 足りている |
| 1.2 同じモジュールでぶつかる | 0 組 | 足りている |
| 1.3 別のモジュールで同じ名 | 要件の 4 組のほかに 2 組（2.2 の ⑸⑹） | **Constraint**（数え直しが要る・⑹ は兄弟のモジュール） |
| 1.5 数字を持たない札 | 2 本。検査の見本の文字列と区別が要る | 足りている |
| 1.6 札でない頭 | `spine_`・`real_`・`row1_` は札の式に当たらない（`row1_` は `t_` で始まらない） | 足りている |
| 2.4 足された札の数え直し | 式は brief のとおり使える | 足りている |
| 3.1〜3.4 振る舞いを変えない | 名に依る作りは無い（2.5）。`config.rs` は `mod tests` の中だけ | 足りている |
| 3.2 本数が同じ | 前の spec の名の一覧の型が使える（2.6） | 足りている |
| 3.5 整形を含む全体テスト | 表の行が畳まれる見込み（`cargo fmt` が要る） | Constraint |
| 4.1 同じコミットで名指しを直す | 表が他の crate の 10 ファイルを読む（2.3）。コミットの切り方が縛られる | **Constraint** |
| 4.2 コメントの名指し | 古い名の全体の 14 行は要件どおり。**札だけの短い書き方 24 行・`t_` の無い札 8 行・大文字の作業番号 70 行は要件に無い** | **Missing**（範囲の決めが要る） |
| 4.4 COMPAT §8 | 226・354・412 行は要件どおり。**196 行の `t_k5` が要件に無い** | **Missing** |
| 4.5 未完了の spec の文書 | `zorder-property/brief.md` は 94 行が名の全体、59・81 行が短い書き方。要件は 59・81 行だけを挙げる | **Missing**（行の取り違え） |
| 4.8 「旧 `t_c3_…`」 | 412 行。同じ行の「T-C3」も歴史の書き方 | 足りている |
| 5.1〜5.3 最後の確かめ | 式は作れる。5.3（古い名の全体）は短い書き方を拾わない | **Constraint**（式を足すか決める） |
| 6.1 本体は `placement_tests/` だけ | 本体の `runtime_tests/zorder_wiring_tests.rs` に改める側の名指しが残る | **Constraint** |
| 6.2 dola-cue の 3 ファイル | ほかに `consumer_ledger_test_support.rs:28` のコメントが向こうの持ち物 | **Constraint** |
| 本番のファイル | `zorder_group_ledger.rs:667` のコメントが `t_zrb9` を名指す（要件 3.3 は本番の「関数・型・定数」を変えないとだけ書く） | **Unknown**（触ってよいかの決め） |

## 4. 実装の形の案

### 案 A: 改名表を作り、機械で全部を置き換える（スクリプト）

- 中身: 改める前の木から改名表（ファイル・古い名・新しい名・札）を作る。その表で、テストのファイルの `fn` の行、表の文字列、コメント、`doc/`、brief の名の全体を、単語の境で 1 つずつ置き換える（`re.sub(r'\b' + 古い名 + r'\b', 新しい名)` を表の行ごとに）。改名表は実装の記録に残し、最後の確かめの「古い名」の名簿にも使う。
- 良い所: 284 本の手の打ち間違いが 0。同じ表が置き換えと確かめの両方に使える。札を「外すだけ」なので表の作り方も 1 行の式で済む。
- 悪い所: 短い書き方（札だけ・範囲・`t_` 無し・大文字）は表の名の全体と一致しないので拾えない。⑷ の名指しのように、札で 2 本を見分けていた所は文脈を読んで直す必要がある。
- 書き込みの道具が逆斜線を化かす件（記憶の「書いたらバイトで確かめる」）は、置き換える名に逆斜線が無いので当たらない。表のファイルの他の文字列（`\![set,zorder]` ほか）を壊さないよう、行ごとの置き換えに限る。

### 案 B: ファイルごとに手で直す（実装係がファイルを読んで直す）

- 中身: ファイルを 1 本ずつ開き、名と名指しを直す。
- 良い所: 文脈が要る名指し（範囲の書き方・⑷）も同じ手で直せる。
- 悪い所: 284 本＋名指し 100 行余りを手で打つので、打ち間違い（綴りの 1 字の違い）を表の検査とコンパイルでしか拾えない。コメントの打ち間違いは何も拾わない。トークンの量も多い。

### 案 C: 機械の置き換え＋短い書き方だけ手で（A と B の組み合わせ）

- 中身: 名の全体（`fn` の行・表・コメント・文書の 2.3 の全部）は案 A の表で機械に置き換える。短い書き方（2.4 の (a)(b)、範囲に決めたなら (c)）は、札 → 新しい名の全体の対応を同じ表から引いて手で書き直す。範囲の書き方は文を書き直す。
- 良い所: 打ち間違いの起きやすい所を機械に任せ、文脈の要る所だけ人の手にする。短い書き方の札はどれも 1 本だけが持つ札なので、表から 1 対 1 に引ける。
- 悪い所: 2 段になる。手で直す所が範囲の決め（7 節の 1・2）で大きく変わる。

### コミットとタスクの切り方の候補

表の検査が他の crate のファイルを読むので、**表が読む 10 ファイルとその 2 つの表は同じコミット**に入れる必要がある（要件 4.1）。それ以外は自由に切れる。

- 切り方 1（ファイルの群ごと・8〜10 タスク）: ① 改める前の名の一覧と改名表を取る ② 表が読む 10 ファイル（`areka-placement` の 5・`areka-runtime` の 5）と本体の 2 つの表（`t_zbc_`・`t_zic_` を含む）を 1 コミット ③ `areka-placement` の zorder の残り 2 ファイル（`zorder_group_ledger_rebase_tests.rs`・`zorder_property_deferral_tests.rs`）と `zorder-property/brief.md` ④ 配置の解決（`resolver_*` 5 ファイル）⑤ 設定と再配置（`config.rs`・`config_limit_tests.rs`・`reseed_tests.rs`・`balloon_limit_tests.rs`・`source_tests.rs`）と §8 の 354・412 行と `extra-character-windows/brief.md` ⑥ 窓の作成（`spawn_*` 3 ファイル）⑦ `areka-runtime` の残り（`window_state_cue_tests.rs`・`zorder_wiring_tests.rs`・`frame_schedule_tests.rs`）と §8 の 226 行 ⑧ 短い書き方の名指し ⑨ 最後の確かめと名の一覧の比べ。
- 切り方 2（機械の置き換えを 1 コミットで・4〜5 タスク）: ① 改名表 ② 名の全体を全部のファイルで一度に置き換え（1 コミット）③ 短い書き方を手で ④ 最後の確かめ。手数は最少。ただしタスクが 1 つ大きくなり、レビューの単位が粗い。
- どちらでも、各コミットで `cargo test -p areka --bin areka placement_tests::zorder` の表の検査と、改めた crate のテストを通す。`cargo fmt` は各コミットで当てる。

## 5. 規模と危険度

- **規模: S〜M**（284 本の名と 100 行余りの名指し。機械の置き換えなら 1〜3 日に収まる。手で直す範囲を (c) の 70 行まで広げると M の側）。
- **危険度: 低**。名に依る作りが無く（2.5）、同じモジュールでのぶつかりも 0。危ないのは「表の検査が他の crate を読む」ことによる途中のコミットの赤と、短い書き方の取り残しだけで、どちらも検索で見える。

## 6. 設計の段へ持ち越す調べ

- **Research Needed**: 名の一覧の比べ（要件 3.2）を、名の末尾だけで比べるか、モジュールのパス付きで比べるか。2.2 の ⑴〜⑹ の同じ名があるので、末尾だけの多重集合では「同じ名の 2 本のどちらか」が入れ替わっても見えない。モジュールのパス付きの一覧（`cargo test --workspace -- --list` の `: test$` の行）に改名表を当てて比べる形なら見える。
- **Research Needed**: `rustfmt` が表の行を畳む量（改名の後に `cargo fmt --check` の差分で数える）。1,000 行の上限には影響しないが、差分の読みやすさに効く。
- **Research Needed**: 着手のときの数え直し（要件 2.4）で、C25 の相手の着地により札つきの名や名指しの行が動いていないか（特に `session-crate-split` が本体の `runtime_tests/zorder_wiring_tests.rs` のコメントを動かすか）。

## 7. 要件の討議へ出す決めごと

1. **札だけの短い書き方（2.4 の (a)・24 行）をこの spec で直すか。** 要件 4.5 は `zorder-property/brief.md` の `t_zpd40` だけを挙げるが、コードのコメントに同じ書き方が 20 行余りある。札が名から消えると何も指さなくなる。直すなら、範囲の書き方（`t_zgp1`〜`t_zgp14` ほか・`zorder_group_ledger_tests.rs` の 8〜10・24・25 行）を名の全体の並びにするか、文で言い直すかも決める。
2. **大文字の作業番号（2.4 の (c)・`T-K1` ほか 70 行）と `t_` の無い札（(b)・`zwi01／02` ほか）を範囲に入れるか。** 同じ `## Comments` の理由に当たり、札を名から外すと `T-K5` ↔ `t_k5_…` の字面の結びも消える。入れると規模が M の側へ寄る。入れないなら、`/kiro-discovery` で別に起票するか、残す訳を記録するか。`areka-emo-present` の 2 行（`T-N3`）は範囲の crate の外。
3. **別のモジュールで同じ名になる組を、要件 1.3 の 4 組から 6 組へ数え直すか、その 2 組を別の名にするか。** ⑸ `t_zbc_`／`t_zic_` の表の検査（同じ形のテストが 2 つの表に 1 本ずつ）と、⑹ `t_zcc09_` と札の無い実窓のテスト（兄弟のモジュール・中身が違う）。⑹ は `cargo test <名>` で実窓のテストも走る。
4. **⑷ `version_moves_only_when_the_contents_move` の 2 本の名指しの書き方。** 今は札（`t_zgl15`・`t_zrb9`）で見分けている。名指しにファイルを添える（`zorder_group_ledger_rebase_tests.rs` の `version_moves_…`）か、どちらかの名に中身を表す語を足すか（要件 1.2 はぶつかる時だけ足すと書き、ここはぶつからない）。
5. **境界の外のファイルに残る、改める側の名指しをどうするか。** 本体の `runtime_tests/zorder_wiring_tests.rs` の 4・5・10・11・13 行は `areka-runtime` の改める名（`t_zwi06`・`zwi01／02`・`zwi06／07`・`zwi09`）を名指す。要件 6.1 で触れない。案: (i) `audit-areka-app` の brief へ申し送り、向こうが 2 本の改名と同じコミットで直す (ii) `session-crate-split` の着地の後にこの spec が直す（約束の変更が要る）(iii) 要件 5.3 の確かめの例外として記録する。逆向きに、`areka-runtime` の `zorder_wiring_tests.rs:18` は本体の 2 本（`zwi05`・`zwi08`）を名指す（こちらは着地の後も本体の 2 本が札を持つので、今の書き方のままでも指せる）。
6. **本番のファイルのコメント（`zorder_group_ledger.rs:667` の `t_zrb9`）を直してよいか。** 要件 3.3 は本番の関数・型・定数を変えないとだけ書き、コメントには沈黙。直さないと要件 5.3 を短い書き方まで広げたときに当たりが残る。
7. **要件 4.5 の行の取り違えを直すか。** `zorder-property/brief.md` で名の全体を持つのは 94 行で、59・81 行は短い書き方。要件の Boundary Context と 4.5 は 59・81 行だけを挙げる。同じく `doc/COMPAT_ARCHITECTURE.md` 196 行の `t_k5` を要件 4.4 の行に足すか。
8. **`dola-cue-test-builder` の持ち物 `consumer_ledger_test_support.rs:28` のコメントに触るか。** 向こうが直すのは 126 行からの別の塊で、行は重ならない（約 100 行離れる）。要件 6.2 の 3 ファイルのほかの接触として「許す重なり」に足すか、触らずに申し送るか。

あわせて、要件 5.3 の確かめの式を、古い名の全体のほかに札だけの短い書き方（`\bt_[a-z]+[0-9]+[a-z]?\b`）まで広げるか（1・6 の決めに従う）。

## 8. 設計の段の調べ（2026-11-22・`/kiro-spec-design`）

- 調べの型: **軽い調べ**（既にあるテストの名を変えるだけ・新しい依存・新しい本番のコードは無い）。外の資料は引いていない。下の調べ物は手元の `git grep` と、scratchpad の使い捨ての写しへ当てた `rustfmt` だけ。
- 使ったスキル: `kiro-spec-design`（手順と規則）だけ。設計・画面・アクセシビリティの領域のスキルは当たらない（テストの名と注釈と文書の直しだけ）。
- 測った木: `claude/kiro-start-test-name-work-tags-c34897` の HEAD `1fb7f500`（main `00877299` に spec の文書のコミット 4 つ・コードは main と同じ）。

### 8.1 数えの確かめ

- `git grep -cE "fn t_[a-z]+[0-9]+[a-z]?_" -- crates` ＝ 29 ファイル・284 本（ギャップ分析と同じ）。
- 古い名の全体（`fn t_…` の 284 本）に重なりは無い（`sort | uniq -d` が 0）。古い名を 1 つ決めれば、どのファイルのどのテストかが 1 つに決まる＝名の一覧の比べでは、名の末尾を改名表で写すだけで前後が対応する。
- 名指しの札の持ち主（改める群を決めるのに使った）: `zcs1`・`zcs2`・`zcs4`・`zcs14`＝`areka-runtime` の `zorder_cue_tests.rs`（`window_state_cue_tests.rs:324` と `consumer_ledger_test_support.rs:28` が名指す）・`i4`＝`spawn_clickthrough_tests.rs` と `spawn_follow_pipeline_tests.rs`・`rs01`・`rs02`・`rs05`＝`reseed_tests.rs`・`c3`＝`config.rs`・`l2`＝`config_limit_tests.rs`・`k5`＝`resolver_balloon_keyword_tests.rs`・`zgl15`＝`zorder_group_ledger_state_tests.rs`・`zrb9`＝`zorder_group_ledger_rebase_tests.rs`・`zdb13`〜`zdb17`＝`frame/zorder_descript_tests.rs`・`n10`＝`frame_schedule_tests.rs`・`zwi06`＝`areka-runtime` の `zorder_wiring_tests.rs`・`zpd*`＝`zorder_property_deferral_tests.rs`・`zdp10`＝`frame/zorder_drain_projection_tests.rs`。
- `crates/areka-emo-present/` の `T-N3`（2 行）が指すのは `display_tests.rs` の `surface_size_matches_scaled_extent_within_one_pixel`（丸めの一致を表で固定するテスト・札は無い）。
- CLAUDE.md の決まりの行は 97 行目の英語の 1 行（`Do not reference `.kiro` specs, requirement IDs, or task numbers in code. …`）。要件 7.1 の「`.kiro` の spec・要件・タスクの番号をコードに書かない」はこの行の意味を書いたもの。
- steering（`roadmap.md`・`roadmap-history.md` を除く）・`doc/`（`COMPAT_ARCHITECTURE.md` を除く）・`tools/`・`.github/`・`.claude/` に改める名の名指しは無い（ギャップ分析と同じ）。

### 8.2 `rustfmt` が表の行を畳む量（6 節の 2 つ目の調べ）

本体の 2 つの表のファイルを scratchpad へ写し、札を外して `rustfmt --edition 2024` を当てた（コミットしない）。

| ファイル | 前 | 後 |
|---|---|---|
| `zorder_group_branch_coverage_tests.rs` | 379 行 | 379 行（畳み 0） |
| `zorder_inversion_branch_coverage_tests.rs` | 393 行 | 389 行（組 1 つ前後が 1 行へ畳まれる） |

畳みは小さく、差分は読める。`fn` の行は短くなるだけで折り返しは生まれない。各コミットで `cargo fmt --all` を当てればよい。

### 8.3 名の一覧の比べ方（6 節の 1 つ目の調べ）

`cargo test --workspace --no-fail-fast -- --list` の `: test$` の行を、**モジュールのパスつき**で取る。改める前の一覧の各行の最後の名を改名表で写し（古い名は全体で重ならないので 1 対 1）、並べ直したものが改めた後の一覧と 1 行も違わないことで、要件 3.2（crate ごとの本数）と「名だけが変わった」を一度に示す。名の末尾だけの多重集合で比べると、別のモジュールで同じ名になる 6 組（2.2）の入れ替わりが見えないので取らない。

### 8.4 要件 5.5 の式が範囲の中で当てる所（例外の記録の下書き）

`\bt_[a-z]+[0-9]+[a-z]?` と `\bT-[A-Z]{1,4}[0-9]{1,2}[a-z]?\b` を、`crates/`・`doc/`・steering・未完了の spec の文書（歴史の場所を除く）へ当てた今の当たりのうち、改めた後も残るもの:

| 場所 | 当たり | 扱い（設計の仮定・議題 3） |
|---|---|---|
| 本体の `runtime_tests/zorder_wiring_tests.rs` 11 行 | `t_zwi06` | 要件 5.5 の例外（申し送った行） |
| 同ファイルの `fn t_zwi05_…`・`fn t_zwi08_…` | 2 本 | 要件 5.1 の例外（`audit-areka-app` の持ち物） |
| `doc/COMPAT_ARCHITECTURE.md:412` | 「旧 `t_c3_char_n_prefix_adds_scope_n`」・「テスト T-C3」 | 要件 5.5 の例外（歴史の文・要件 4.8） |
| `crates/areka-sakura/` の局所変数 | `t_wait1`・`t_wait2` | 要件 5.5 の例外（テストの名でない） |
| `.kiro/specs/extra-character-windows/brief.md:131` | 「完了 `window-placement` の設計 DD6・テスト T-C3」 | **要件 5.5 に挙げられていない**。412 行と同じ歴史の文（完了 spec の設計の番号を書く） |
| `.kiro/specs/completed/audit-areka-app/brief.md:159` | `t_zwi05_…`・`t_zwi08_…` | **要件 5.5 に挙げられていない**。札をまだ持つ本体の 2 本を正しく指す申し送り |
| `.kiro/steering/roadmap.md` 514・2033・2039 行 | `t_i4_`・`t_c3_`・`t_l2_`・`t_rs01_` | **要件 5.5 に挙げられていない**。この spec の行の説明で、札の例を書くもの（テストの名指しではない） |
| この spec の文書（`.kiro/specs/test-name-work-tags/`） | 多数 | 要件 5.3 と同じく除く（要件 5.5 は書いていない） |

## 9. 設計の決め（2026-11-22）

### 決め 1: 改名表を 1 つ作り、名の全体は機械で、短い書き方は手で（ギャップ分析の案 C）

- **Context**: 284 本の名と、名の全体の名指し 69＋20 行余り、短い書き方・`t_` の無い札・大文字の作業番号 100 行余り（要件 1・4）。
- **Alternatives**: 案 A（全部を機械で）＝短い書き方を拾えない。案 B（全部を手で）＝打ち間違いをコメントでは何も拾わない。IDE の名の変更（rust-analyzer の rename）＝文字列の表・文書・コメントを直さず、1 本ずつの手の操作になる。
- **Selected**: 案 C。改名表（ファイル・古い名・新しい名・札）を着手のときに木から作り、表の行ごとに単語の境で置き換える。短い書き方は同じ表の札から 1 対 1 に引いて手で書き直す。範囲の書き方と大文字の作業番号は文を手で直す。
- **Rationale**: 打ち間違いの起きやすい所を機械に任せ、文脈の要る所だけを手にする。同じ表が置き換えと最後の確かめ（要件 5.3 の古い名の名簿）の両方に使える。
- **Follow-up**: 置き換えはバイトで行い（文字コードと改行を保つ）、書いた後にバイトで確かめる（逆斜線を含む表の他の文字列を壊さない）。

### 決め 2: コミットは「改める群」ごと・群の名指しは場所を問わず同じコミット

- **Context**: 要件 4.1（あるファイルの名を改めるコミットで、その名指しを全部直す）と、本体の表が他の crate の 10 ファイルを読むこと（2.3）。
- **Selected**: 改める 28＋2 ファイルを 6 つの群に分け、1 群 1 コミットにする。群の名指しは、どのファイルに在っても（他の群のファイル・本番のファイルのコメント・`doc/`・brief）同じコミットで直す。表が読む 10 ファイルと本体の 2 つの表は同じ群（G1）。
- **Alternatives**: 全部を 1 コミット（ギャップ分析の切り方 2）＝手数は最少だが、レビューの単位が 284 本で粗く、brief の見込み（8〜12 タスク）にも合わない。議題 1。
- **Trade-offs**: `doc/COMPAT_ARCHITECTURE.md` の 412 行のように、1 行が 2 つの群の名を持つ所は 2 つのコミットで少しずつ直る（どのコミットでも、その時点で在る名を指す）。

### 決め 3: 記録の置き場

- 改名表は spec の文書の隣に `rename-table.tsv` として残す（完了でアーカイブへ移る・後の spec が古い名から新しい名を引ける）。
- 名の一覧の前後・置き換えの使い捨ての式は作業の木の `target/test-name-work-tags/` に置き、コミットしない。比べの結果（行数・差の 0）と式は `research.md` の新しい節へ書く。議題 2。

### 決め 4: 注釈の作業番号の直し方

- 節の見出しや doc コメントの頭の `T-K1:` ほかは、番号だけを消して説明の文を残す。
- 他のテストを指す働きを持つもの（`frame_schedule_tests.rs`（T-N10）・`display_tests.rs` の T-N3・「`resolver_resolve_balloon_tests.rs` の T-R8 群」）は、ファイルとテストの新しい名で書き直す。いくつものテストを番号でまとめて指す所（T-R8 群・`t_zgp1`〜`t_zgp14`）は、名を並べず、中身の言葉で言い直す（要件 4.9）。
- `areka-runtime` の `zorder_wiring_tests.rs:18` が本体の 2 本を指す「（zwi05）」「（zwi08）」は、番号を消し、すでに在る中身の言葉とファイルの道で指す形を残す（本体の 2 本はこの spec の着地の後も札を持つので、新しい名が無い）。議題 4。

## 10. 合成（design-synthesis）の結果

- **一般化**: 要件 4 の名指しの直し（コメント・表・`doc/`・brief）は、どれも「改名表の 1 行の古い名 → 新しい名」の同じ置き換えで、場所の違いは除く場所（歴史の場所・この spec の文書）だけ。短い書き方も「札 → 改名表の 1 行」の同じ引きになる。1 つの表と 1 つの決まり（群の名指しは同じコミット）で全部を覆う。
- **作るか使うか**: `git grep`（数えと確かめ）・python の正規表現（バイトの置き換え）・`cargo fmt`・`cargo test -- --list` を使う。新しい道具は作らない。IDE の rename は文字列と文書を直さないので取らない。
- **削ったもの**: 置き換えの式のコミット（使い捨て）・札を見張る照合のテスト（要件 7.2）・名に言い回しの直しを足すこと（要件 1.4）。

## 11. 危険と手当て

- 表が読む 10 ファイルの改名と表の直しが別のコミットに分かれると本体のテストが赤 → G1 を 1 コミットにし、そのコミットで本体の表の検査を通す。
- 短い書き方の取り残し → 群ごとに、その群の札の短い書き方を検索してから閉じ、最後に要件 5.5 の式で全体を数える。
- 並走の相手と同じファイルを触る（`consumer_ledger_test_support.rs:28`・§8 の行） → 触る前に `dola-cue-test-builder` のセッションへ知らせる。§8 は名の行だけ。
- `cargo test <名>` で別のモジュールの同じ名が一緒に走る（⑴〜⑹） → 要件 1.3 で許した。名指しの文ではファイルの名を添える（要件 4.11）。

## 12. 設計の討議の決め（2026-11-22）

設計の議題 4 件を推奨案で確定した（design.md の「設計の決めた事」）: ⑴ 群ごとのコミット（G0〜G8 の 9 タスク）⑵ 改名表だけをコミットし、名の一覧と置き換えの道具は `target/test-name-work-tags/` に置く ⑶ 式 4・5 の例外に、`extra-character-windows/brief.md:131` の歴史の文・`audit-areka-app/brief.md` の本体の 2 本の名・`roadmap.md` のこの spec の行・この spec の文書を訳つきで足す ⑷ `areka-runtime` の `zorder_wiring_tests.rs:18` の「（zwi05）」「（zwi08）」は番号だけを消す。

あわせて、設計の検証（design-validation.md・GO）の 3 件は自明な直しとして design.md に入れた（§8 の隣り合う行の衝突の解き方・式 4・5 の `-I` と表の数の主張をやめること・群の閉じ方に式 4・5 を足すこと）。

## 13. 着手のときの数え直し（G0・2026-11-22）

- 測った木: `claude/kiro-start-test-name-work-tags-c34897` の HEAD `3829a00b`（`origin/main` の先頭 `7cb95c3e` を取り込み済み・`git fetch origin` の後に `HEAD..origin/main` は 0 本）。`00877299` から main に足された 2 本（#496・#497）は steering と roadmap の文書だけで、範囲のファイルに触らない。
- 使い捨ての式: `target/test-name-work-tags/rename.py`（`build`・`apply <群>`・`refs <群>`）・`mklist.py`（名の一覧）・`sumrefs.py`（名指しの数え）。コミットしない。

### 13.1 並走の相手と、範囲のファイルを触る別の spec

| spec | 状態（`git worktree list`・`git branch -a`・`origin/main..<枝>` の差分） | 扱い |
|---|---|---|
| `session-crate-split` | 枝 `claude/kiro-start-session-crate-split-56c35d` が要件の討議まで（コードの差分 0）。main に着地していない | 取り込むものは無い |
| `dola-cue-test-builder` | 枝 `claude/kiro-start-dola-cue-test-builder-294439` がタスクの生成と実装の 1 コミット（`crates/dola/README.md` だけ）。main に着地していない | 取り込むものは無い。`consumer_ledger_test_support.rs` はまだ動いていない |
| `zorder-property`・`zorder-chain-residue`・`extra-character-windows` | `.kiro/specs/` に brief だけ。枝も作業の木も無い（`claude/areka-p0-zorder-scope-87951c` は 10-19 の完了 spec の古い枝） | 走っていない。後に回す群・ファイルは 0（要件 6.4） |

### 13.2 札つきの名の数え（要件 2.4）

- 式: `fn t_[a-z]+[0-9]+[a-z]?_[a-z]\w*\(` の定義（`crates/` の `git ls-files` の `.rs`）と、名で足す `t_zbc_the_table_covers_every_branch_with_executed_tests`・`t_zic_the_table_covers_every_branch_with_executed_tests`。除くファイル: 本体の `runtime_tests/zorder_wiring_tests.rs`・`zorder_branch_coverage_test_support.rs`。
- 結果: **30 ファイル・284 本**（282＋2）。起票の後に足された札つきのテストは 0、`#[test]` が付かない定義は 0、C25 の相手の持ち物のファイルに在るものは 0、G1〜G6 の一覧に無いファイルで見つかったもの（場所で群を決めたもの）は 0。

| 群 | 本数 | ファイルごとの本数 |
|---|---|---|
| G1 | 146 | `areka-placement`: `zorder_group_ledger_tests.rs` 23・`zorder_group_ledger_state_tests.rs` 21・`zorder_group_ledger_written_order_tests.rs` 10・`zorder_chain_compose_tests.rs` 17・`zorder_chain_compose_written_order_tests.rs` 5／`areka-runtime`: `zorder_cue_tests.rs` 16・`frame/zorder_descript_tests.rs` 19・`frame/zorder_drain_tests.rs` 11・`frame/zorder_drain_projection_tests.rs` 14・`frame/zorder_drain_written_order_tests.rs` 8／本体: `placement_tests/zorder_group_branch_coverage_tests.rs` 1・`zorder_inversion_branch_coverage_tests.rs` 1 |
| G2 | 17 | `zorder_group_ledger_rebase_tests.rs` 9・`zorder_property_deferral_tests.rs` 8 |
| G3 | 42 | `resolver_resolve_tests.rs` 15・`resolver_balloon_keyword_tests.rs` 12・`resolver_resolve_free_tests.rs` 6・`resolver_resolve_balloon_tests.rs` 6・`resolver_union_tests.rs` 3 |
| G4 | 47 | `config.rs` 20・`config_limit_tests.rs` 2・`reseed_tests.rs` 14・`balloon_limit_tests.rs` 9・`source_tests.rs` 2 |
| G5 | 12 | `spawn_cleanup_tests.rs` 6・`spawn_follow_pipeline_tests.rs` 3・`spawn_clickthrough_tests.rs` 3 |
| G6 | 20 | `window_state_cue_tests.rs` 13・`zorder_wiring_tests.rs` 5・`frame_schedule_tests.rs` 2 |

crate ごと: `areka-placement` 194・`areka-runtime` 88・`areka` 2。design.md の File Structure Plan の本数と 1 本も違わない。

### 13.3 改名表を作るときの検査（要件 1.1〜1.6）

- 改名表: `.kiro/specs/test-name-work-tags/rename-table.tsv`（見出し 1 行＋284 行・UTF-8・LF・群とファイルと古い名で並べた）。`new` は `old` から頭の `t_<英小文字と数字>_` を外した残り。
- 古い名の重なり 0・同じファイルの他の `fn`／`const`／`static`／`struct`／`enum`／`mod`／`type`／`trait`／`union` の名とのぶつかり 0（語を足した行 0・要件 1.2）・予約語と数字で始まる名 0。札の式は `spine_`・`real_`・`row1_` に当たらない（要件 1.6）。
- 検査が止まることの確かめ: 予約語の名簿へ `clone_reaches_the_same_receiver` を足して `build` を走らせると `BLOCKED: bad new name clone_reaches_the_same_receiver` で止まり、表のファイルを書かない。
- 別のファイルで同じ名になる組（改めた名どうしと、既にある `#[test]` の名・要件 1.3）: **6 組**で、2.2 の ⑴〜⑹ と同じ組。ただし ⑶ `clone_reaches_the_same_receiver` は、2.2 の 2 ファイルのほかに本体の `crates/areka/src/session/readme/readme_cue_tests.rs:166` の札の無い既存のテスト（`session-module-gather`・#494 で今の道へ移ったもの）とも同じ名で、3 本になる。コンパイルは通り、要件 1.3 で許す形。名指しにファイルを添える対象（要件 4.11）は、この名の名指しが無いので増えない。

### 13.4 改める前の名の一覧（要件 3.2）

- `cargo test -p areka-placement -p areka-runtime -p areka --no-fail-fast -- --list` の `: test` の行を、テストの束（crate とターゲット）とモジュールのパスつきで `target/test-name-work-tags/tests-before.txt` に取った（コミットしない）。改める名を持つのはこの 3 crate だけなので、名の一覧はこの 3 crate に限った（`areka-emo-present` は注釈だけを直す）。
- 本数: 全 3,967（`areka` 1,898・`areka-placement` 1,019・`areka-runtime` 1,050）。改名表の古い名 284 は一覧にちょうど 1 度ずつ在る。

### 13.5 名指しの当たり（要件 4.6）

`rename.py refs <群>`（歴史の場所・この spec の文書・本体の `runtime_tests/zorder_wiring_tests.rs` を除く `git ls-files` の全部へ、群の古い名の全体と、群の中で 1 本だけが持つ札の短い書き方を当てる）から `fn` の定義の行を除いた当たり:

| 群 | 行数 | 場所 |
|---|---|---|
| G1 | 85 | 本体の 2 つの表 50・19、`zorder_group_ledger_tests.rs` 8・9・10・24・25・315・357・472、`zorder_group_ledger_written_order_tests.rs:167`、`zorder_group_ledger_state_tests.rs:20`、`frame/zorder_drain_projection_tests.rs` 522・531、`window_state_cue_tests.rs:324`、`zorder_cue_tests.rs:148`（`t_zcs4`）、`consumer_ledger_test_support.rs:28`、`doc/COMPAT_ARCHITECTURE.md:226` |
| G2 | 18 | `zorder_property_deferral_tests.rs` 6・20・23・25・27・29・77・123・126・268・307・308・453・487、`zorder_group_ledger.rs:667`、`.kiro/specs/zorder-property/brief.md` 59・81・94 |
| G3 | 2 | `resolver_balloon_keyword_tests.rs:54`、`doc/COMPAT_ARCHITECTURE.md:196` |
| G4 | 3 | `doc/COMPAT_ARCHITECTURE.md` 354・412、`.kiro/specs/extra-character-windows/brief.md:133` |
| G5 | 1 | `spawn_assembly_tests.rs:154` |
| G6 | 1 | `frame_schedule_tests.rs:14` |

- design.md の「名指しの見込み」に無かった当たりは `zorder_cue_tests.rs:148` の `t_zcs4`（G1 のファイルの中・2.4 には在る）の 1 行。G1 で直す。
- steering（`roadmap.md` を含む）・ほかの未完了の spec の文書の当たりは 0（`roadmap.md` の `t_i4_` ほかは複数のテストが持つ札の例で、名指しの式に当たらない・決めた事 3）。
- 式 4（札だけの短い書き方）の当たりは 24 行、式 5（大文字の作業番号）の当たりは `crates/` に 70 行で、2.4 の数えと同じ。

## 14. 最後の確かめの記録（G8・2026-11-22）

- 測った木: 枝 `claude/kiro-start-test-name-work-tags-c34897` の HEAD `122c4140a`＝G0〜G7 の `0f2a4d9fa` に `origin/main` の `c7b705aff`（#498 `dola-cue-test-builder`）を取り込んだマージ（merge-base は `c7b705aff`。`audit-areka-app/brief.md` の競合は両方の項目を残して解いた）。取り込む前の `0f2a4d9fa` でも同じ測りをした（14.7）。較正の木は `00877299`（改める前・§13 の測りと同じ名の一覧の元）。
- 式 3 の名簿: 改名表の `old` の 284 行（`target/test-name-work-tags/` の外の使い捨てファイル）。式 4・5 の範囲は `crates/`・`doc/`・`.kiro/steering/`・`.kiro/specs/`（歴史の場所 `.kiro/specs/completed/`・`doc/archive/`・`.kiro/steering/roadmap-history.md` を除く）。
- 数は「行」（`git grep -n` の行数）と「当たり」（`git grep -o` の数・1 行に 2 つ在れば 2）を分けて書く。

### 14.1 式 1〜5 と数（改めた後の木 HEAD `122c4140a` ／ 改める前の木 `00877299`）

| 式 | 当てたもの | 改めた後（HEAD） | 改める前（較正） | 期待 |
|---|---|---|---|---|
| 1（5.1） | `git grep -nE "fn t_[a-z]+[0-9]+[a-z]?_" -- crates` | **2 行**（本体の `runtime_tests/zorder_wiring_tests.rs` の 29 行目 `t_zwi05_…`・65 行目 `t_zwi08_…`） | 284 行・29 ファイル（うち本体の `runtime_tests/` が 2） | 284／本体の 2 本だけ |
| 2（5.2） | `git grep -nE "fn t_[a-z]+_" -- crates` から式 1 の当たりを除く | **2 行**（`placement_tests/zorder_branch_coverage_test_support.rs` の 79 行 `t_only_in_a_comment`・85 行 `t_real_one`＝検査の見本の文字列） | 4 行（上の 2 行と `zorder_group_branch_coverage_tests.rs:377`・`zorder_inversion_branch_coverage_tests.rs:391`） | 4／見本だけ |
| 3（5.3） | 名簿を `git grep -nwF -f` で歴史の場所と本 spec の文書を除く全体へ | **0 行** | 372 行（`fn` の定義 284＋名指し 88。名指し 88 は §2.3 の 50・19・9・1・1・1・1・1・3・1・1 と同じ内訳） | 0／284＋名指し |
| 4（5.5） | `git grep -nIP "\bt_[a-z]+[0-9]+[a-z]?"`（`t_wait1`・`t_wait2` を含む） | この spec の文書を除き **12 行・5 ファイル・17 当たり**（下の 14.2） | 式 1 の 284 行を除き 119 行。うち札だけの短い書き方（`(?!_[a-z])\b` の形・`t_wait` を除く）は **24 行** | §2.4 の 24 行 |
| 5（5.5） | `git grep -nIP "\bT-[A-Z]{1,4}[0-9]{1,2}[a-z]?\b"` | この spec の文書を除き **2 行・2 ファイル・3 当たり**（`COMPAT_ARCHITECTURE.md:412` に `T-C3` が 2 つ・下の 14.2） | 72 行（`crates/` が **70** 行＋`doc/COMPAT_ARCHITECTURE.md:412`・`extra-character-windows/brief.md:131`） | §2.4 の 70 行 |

- この spec の文書（`.kiro/specs/test-name-work-tags/`）の当たりは HEAD で数えて、式 4 が 365 行・式 5 が 34 行。

- 式 3 の改める前の 372 行は、`git grep` が `00877299` の木へ当てた数。歴史の場所（`.kiro/specs/completed/` の 30 ファイル・`doc/archive/` と `roadmap-history.md` は当たり 0）の当たりは、除いたうえでの数。除いた当たりは書き換えない（要件 5.3）。
- 式 4 の「札だけの短い書き方」を改めた後の木へ当てると、当たりは本体の `runtime_tests/zorder_wiring_tests.rs:11` の 1 行だけ（24 行 → 1 行。残りの 23 行は G1・G2・G6 で直した）。
- 札の無い札（`zwi01`〜`zwi09`）は式の外。改めた後の `crates/` の当たりは本体の `runtime_tests/zorder_wiring_tests.rs` の 4・5・10・11・13 行だけで、`audit-areka-app/brief.md` の申し送りの持ち物。`T-N3`・`T-N10` の当たりは 0。

### 14.2 式 4・5 の当たりと例外の表の突き合わせ（この spec の文書を除く）

HEAD `122c4140a` の当たり。式 4 の 12 行の内訳は `zorder_wiring_tests.rs` 3 行（3 当たり）・`compile_sheet_tests.rs` 4 行（4）・`COMPAT_ARCHITECTURE.md` 1 行（1）・`audit-areka-app/brief.md` 1 行（2）・`roadmap.md` 3 行（7）。

| 当たり | 式 | 例外の表の項 | 訳 |
|---|---|---|---|
| `crates/areka/src/runtime_tests/zorder_wiring_tests.rs:11`（`t_zwi06`） | 4 | 要件 5.5 の例外（本体の申し送った行） | 本体のこのファイルは境界の外（要件 2.3）。指す先は `areka-runtime` の `zorder_wiring_tests.rs` の改めたテストで、`audit-areka-app/brief.md` の申し送りで本体の 2 本の改名と同じコミットで直す |
| 同 `:29`・`:65`（`fn t_zwi05_…`・`fn t_zwi08_…`） | 4 | 要件 5.1 の例外（札つきの 2 本） | `audit-areka-app` の持ち物。名の全体が式 1 の当たり 2 行と同じ |
| `doc/COMPAT_ARCHITECTURE.md:412`（`旧 t_c3_char_n_prefix_adds_scope_n`） | 4 | 要件 5.5 の例外（歴史の文・要件 4.8） | 「旧」と書く過去の名。今のテストの名指しではない |
| 同 `:412`（`テスト T-C3` が 2 か所） | 5 | 同上 | 完了 spec の設計の番号を歴史として書く文 |
| `crates/areka-sakura/src/compile_sheet_tests.rs` の 60・62・71・79（`t_wait1`・`t_wait2`） | 4 | 要件 5.5 の例外（局所変数） | テストの名ではない。時刻を持つ `let` の変数名 |
| `.kiro/specs/extra-character-windows/brief.md:131`（`テスト T-C3`） | 5 | 決めた事 3 で足した例外 | `COMPAT_ARCHITECTURE.md:412` と同じ歴史の文（完了 `window-placement` の設計の番号）。書き換えない |
| `.kiro/specs/completed/audit-areka-app/brief.md:159`（`t_zwi05_…`・`t_zwi08_…`） | 4 | 決めた事 3 で足した例外 | まだ札つきで在る本体の 2 本を正しく指す申し送り（既にある 159 行の項目。この spec が足した 1 項目は札の字面を持たず当たらない） |
| `.kiro/steering/roadmap.md` の 389・1790・1796 行（`t_i4_`・`t_c3_`・`t_l2_`・`t_rs01_`） | 4 | 決めた事 3 で足した例外 | この spec の行と出どころの説明で、札の例を書くもの。テストの名指しではない（行番号は main の #497 で §8.4 の 514・2033・2039 から 390・1791・1797 へ、#498 でさらに 1 行ずつ上へ動いた） |
| `.kiro/specs/test-name-work-tags/`（HEAD で式 4 が 365 行・式 5 が 34 行） | 4・5 | 決めた事 3 で足した例外 | 要件 5.3 と同じく、この spec の文書は古い名・札を説明のために書く |

当たりのうち例外の表に無いものは 0。`.kiro/specs/zorder-property/brief.md` の 59・81・94 行は G2 で直り、当たりは 0。#498 が足した文（`audit-areka-app/brief.md` の 6 行・`dola-interrupt-ticks/brief.md` の 4 行・`roadmap.md`・`crates/` の 15 ファイル）に新しい当たりは 0。`dola-cue-test-builder` の文書は `.kiro/specs/completed/` へ移り、歴史の場所として除く。

### 14.3 名の一覧の比べ（要件 3.1・3.2・1.3）

- 取り方: 改める前と同じ `cargo test -p areka-placement -p areka-runtime -p areka --no-fail-fast -- --list`（標準出力と標準エラーを 1 本に）を HEAD `122c4140a` で走らせ、`target/test-name-work-tags/mklist.py` で整えた（束・モジュールのパスつき・並べ直し）。`tests-after.txt`（コミットしない）。`mklist.py` を改める前の生の出力へ当て直すと `tests-before.txt` とバイトで同じ（取り方が再現する）。
- 比べ: `tests-before.txt` の各行の最後の名を改名表の `new` で写し（写した行は 284）、並べ直した `tests-before-mapped.txt` と `tests-after.txt` を `diff` した。**違う行 0・両方に片方だけ在る行 0**。
- #498 との突き合わせ: `git diff 7cb95c3e8 c7b705aff` が 3 crate で触ったのは `areka-runtime` の `consumer_ledger_test_support.rs`（手伝いの `actor_carrier_cue` を `dola::cue::test_support::carrier_cue` の再輸出へ）・`frame_window_link_drain_tests.rs`（手伝いの `carrier` を消して同じ再輸出を使う）・`Cargo.toml` だけで、`#[test]` の足し・消し・改名は 0。足したテストは `dola` の `cue/test_support_tests.rs` で、名の一覧の 3 crate の外。違う行 0 はこれと合う。
- crate ごとの本数（前＝後）: `areka` 1,898・`areka-placement` 1,019・`areka-runtime` 1,050・計 3,967。一覧の中の重複行は前も後も 0。
- 改めた後の一覧に改名表の `old` の名は 0。改名表の `new` は 284 行で、異なる名は 279（別のファイルで同じ名になる組のため 5 つの名が 2 行ずつ・要件 1.3）。改名した 284 行は一覧に 1 行ずつ在る。一覧で `new` の名を持つ行は 286 で、多い 2 行は改める前から同じ名で在った札の無いテスト＝`zorder_chain_tail_order_tests` の `unassigned_scopes_join_behind_every_group_in_ascending_scope_order` と本体の `readme_cue_tests` の `clone_reaches_the_same_receiver`（§13.3）。

### 14.4 触らない所（要件 2.3・3.3・6.1）

`git diff --name-only origin/main...HEAD`（`origin/main`＝merge-base＝`c7b705aff`）は 47 ファイルで、取り込む前（merge-base `7cb95c3e8`）と同じ並び。

- この spec の文書 7（`design-validation.md`・`design.md`・`rename-table.tsv`・`requirements.md`・`research.md`・`spec.json`・`tasks.md`）。
- `crates/areka-placement/src/` の 22 ファイル・`crates/areka-runtime/src/` の 9 ファイル・`crates/areka-emo-present/src/` の 2 ファイル（G1〜G7 の範囲）。
- 本体 `crates/areka/` は `src/placement_tests/zorder_group_branch_coverage_tests.rs`・`zorder_inversion_branch_coverage_tests.rs` の **2 ファイルだけ**。`crates/areka/src/runtime_tests/` の差分は **0**。
- 文書: `CLAUDE.md`（1 行の差分）・`doc/COMPAT_ARCHITECTURE.md`・`.kiro/specs/completed/audit-areka-app/brief.md`（申し送り）・`extra-character-windows/brief.md`・`zorder-property/brief.md`。
- 歴史の場所（`.kiro/specs/completed/`・`doc/archive/`・`.kiro/steering/roadmap-history.md`）の差分は **0**。
- 本番のファイルの差分: `zorder_group_ledger.rs` はコメントの 1 行（667 行・`t_zrb9` を新しい名へ）だけ。`config.rs` は 317 行の `mod tests` の中（336 行以降・40 行）だけ。ほかの `crates/` の差分は `_tests.rs`・`_test_support.rs` のファイルだけ。
- #498 と重なったファイル: `consumer_ledger_test_support.rs` は 28 行目の名指しが改めた名（`set_zorder_becomes_a_set_directive_carrying_the_element_tokens`・`reset_zorder_becomes_a_reset_directive`＝`zorder_cue_tests.rs` の 65・78 行に在る）のまま残り、#498 の直し（127 行あたりの再輸出）と並ぶ。`window_state_cue_tests.rs` は #498 が触らず、改名表の 13 行がどれも新しい名の `fn` 1 つ・古い名 0。

### 14.5 行数（要件 3.4）

触ったファイルの行数の最大は HEAD で `crates/areka-emo-present/src/presenter_test_support.rs` の 909 行。次いで `zorder_group_ledger.rs` 729・`zorder_chain_compose_tests.rs` 720・`config.rs` 711。どのファイルも 1,000 行以下（`.kiro/steering/roadmap.md` は今回の差分に入っていない）。

### 14.6 全体テスト（要件 3.5）

- `pwsh -NoProfile -File tools/test-all.ps1 -Format -License` を、取り込む前の `0f2a4d9fa` で排他テストの机を持った回に 1 度回し、終了コード 0・「all steps green」（2026-11-22）。
- #498 を取り込んだ後の `6b90d9e39` でも、最後の確かめ（`/kiro-validate-impl`）で排他テストの机を持った回に 1 度回し、終了コード 0・「all steps green」（2026-11-22）。
- 要件 6.5 の知らせ: G1 に手を付ける前に、`dola-cue-test-builder` のセッションへ「`consumer_ledger_test_support.rs` の 28 行のコメントの名だけを直す・126 行からの塊に触らない・cue のテストは `fn` の行とコメントだけを変える」と送った。

### 14.7 取り込む前の測り（`0f2a4d9fa`）

式 1〜3 は HEAD と同じ数（2・2・0 行）。式 4 はこの spec の文書を除き 12 行・17 当たりで、違いは `roadmap.md` の行番号（390・1791・1797）だけ。式 5 は 2 行・3 当たり。名の一覧（`target/test-name-work-tags/tests-after-0f2a4d9fa.txt`）は HEAD の `tests-after.txt` とバイトで同じ。触ったファイルは同じ 47。
