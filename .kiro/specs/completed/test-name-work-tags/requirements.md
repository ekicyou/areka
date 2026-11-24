# Requirements Document

## Project Description (Input)

開発者と、テストを読み書きするセッションが困っている。テストの関数の名の頭に、設計の作業文書のテストの番号の札（`t_i4_`・`t_c3_`・`t_l2_`・`t_rs01_` など）が残っている。札の意味は spec の設計にしか無く、spec は完了でアーカイブされ、文書は流れていくので、名を読んでも札の意味は分からない。CLAUDE.md の `## Comments` は、spec・要件・タスクの番号をコードに書かない決まりで、名にも同じ理由が当たる。新しく足すテストが周りの名に合わせて札を写し、札は増え続けている。

今の数え（2026-11-22・main `00877299`・`fn t_[a-z]+[0-9]+[a-z]?_` の検索）は 29 ファイル・284 本（`areka-placement` 20 ファイル・`areka-runtime` 8 ファイル・本体 1 ファイル）。名を名指す側は、コードのコメント（`spawn_assembly_tests.rs:154` ほか）、名を文字列で並べる本体の `placement_tests/` の 2 ファイル、`doc/COMPAT_ARCHITECTURE.md` §8 の判定の欄、未完了の spec の brief（`extra-character-windows`・`zorder-property`）に在る。札を名に付ける決まりはどこにも書かれていない。

直した後の姿: テストの名から札が外れ、名を変えたテストの名指しは新しい名を指す。古い名が残るのは歴史の場所（`.kiro/specs/completed/`・`doc/archive/`・`.kiro/steering/roadmap-history.md`）だけ。テストの本文・期待値・数は変わらず、全体テストは緑のまま。

出どころは `audit-areka-input-menu` のかけ直しの audit の指摘（同 spec の research.md 12.3）。優先度の区分は A（見直しの指摘の取り残し・開発者「とにかく指摘事項は全部直す方針です」）。

## Introduction

この spec は、テストの関数の名の頭に残る作業番号の札を外し、その名を名指す所を新しい名へ直すためのものである。あわせて、札だけの短い書き方（`t_zgp20` ほか）と、注釈に書かれた設計の作業番号（`T-K5`・`zwi01／02` ほか）も直し、札がまた付かないよう CLAUDE.md の決まりに 1 語を足す（2026-11-22 要件の討議・全議題を推奨案で承認）。変えるのはテストの名と、その名指しと注釈だけで、テストの中身・本番の振る舞いは何も変えない。

棚卸㊷の決めにより、本体の `crates/areka/src/runtime_tests/zorder_wiring_tests.rs` の札つきの 2 本はこの spec から外し、本体の残りを受け持つ `audit-areka-app` へ渡した（同じウェーブ C25 の `session-crate-split` がこのファイルの `include_str!` のパスを直すため）。そのため、この spec が改める札つきの名は、`fn t_[a-z]+[0-9]+[a-z]?_` の形で 28 ファイル・282 本と、数字を持たない札の 2 本（要件 1.5）になる。

この spec はウェーブ C25 で `session-crate-split`・`dola-cue-test-builder` と並んで走る。並走の約束（roadmap の「ウェーブ編成」の C25 の行）を守る（要件 6）。

## Boundary Context

- **In scope**:
  - `crates/areka-placement/src/` の 20 ファイルと `crates/areka-runtime/src/` の 8 ファイル（`window_state_cue_tests.rs`・`zorder_cue_tests.rs`・`zorder_wiring_tests.rs`・`frame_schedule_tests.rs`・`frame/zorder_descript_tests.rs`・`frame/zorder_drain_tests.rs`・`frame/zorder_drain_projection_tests.rs`・`frame/zorder_drain_written_order_tests.rs`）の札つきのテストの名（一覧の正本は brief の Current State）。
  - 本体の `crates/areka/src/placement_tests/zorder_group_branch_coverage_tests.rs`・`zorder_inversion_branch_coverage_tests.rs`＝テストの名を文字列で並べる表と、両ファイルの札つきのテスト各 1 本（`t_zbc_…`・`t_zic_…`）。
  - 名指しの直し: コードのコメント（doc コメントの中のリンクを含む。棚卸㊷で見つけた所＝`spawn_assembly_tests.rs:154`・`resolver_balloon_keyword_tests.rs:54`・`zorder_group_ledger_state_tests.rs:20`・`zorder_group_ledger_tests.rs:357`・`zorder_property_deferral_tests.rs` の 6・20〜29・77・123・126 行・`crates/areka-runtime/src/frame/zorder_drain_projection_tests.rs:531`）、`doc/COMPAT_ARCHITECTURE.md` §8 の 226・354・412 行、未完了の spec の文書（`.kiro/specs/extra-character-windows/brief.md:133`・`.kiro/specs/zorder-property/brief.md` の 94 行＝名の全体・59・81 行＝札だけの短い書き方）、steering（今は該当なし・着手のときに数え直す）。
  - 札だけの短い書き方の名指し（research.md 2.4 の (a)・24 行＝本番のファイル `zorder_group_ledger.rs:667` のコメントと `doc/COMPAT_ARCHITECTURE.md:196` を含む）と、注釈の設計の作業番号（同 (b)(c)＝`t_` の無い札 `zwi01／02` ほかと、大文字の `T-K5` ほか 70 行・`crates/areka-emo-present/` の 2 行を含む）（要件 4.9〜4.11）。
  - `dola-cue-test-builder` の持ち物 `crates/areka-runtime/src/consumer_ledger_test_support.rs` の 28 行のコメント（許す重なり・要件 6.5）。
  - `audit-areka-app` の brief への申し送り（要件 6.6）。
  - CLAUDE.md の `## Comments` の 1 行（要件 7）。
  - 最後の確かめの検索（要件 5）。
- **Out of scope**:
  - テストの本文・期待値・置き場の直し、テストのファイル名の変更。
  - 札でない頭（`spine_`・`real_` など意味のある語）。`crates/areka-emo-atlas/src/normalize_rule_tests.rs` の `row1_`〜`row7_`（同じファイルの `Normalizer::plan` の表の行を指す名で、作業文書の番号ではない）。
  - 本体の `crates/areka/src/runtime_tests/zorder_wiring_tests.rs` の札つきの 2 本（`t_zwi05_…`・`t_zwi08_…`）＝`audit-areka-app` の持ち物。
  - `crates/areka/src/placement_tests/zorder_branch_coverage_test_support.rs` の検査の見本の文字列（`t_gone`・`t_real_one`・`t_only_in_a_comment`）＝表の検査そのものを試す見本で、テストの名ではない。
  - 歴史の場所（`.kiro/specs/completed/`・`doc/archive/`・`.kiro/steering/roadmap-history.md`）の書き換え。
  - 札をまた足さないための照合のテスト（決まりの文の 1 語だけを足す・要件 7）。
  - 本体の `crates/areka/src/runtime_tests/zorder_wiring_tests.rs` の 4・5・10・11・13 行の、この spec が改める名の名指し（`audit-areka-app` へ申し送る・要件 6.6）。
  - `doc/COMPAT_ARCHITECTURE.md` のファイルの改名（`doc-compat-architecture-rename` の持ち物）。
  - 名指しの行に在る、名のほかの古い記述（例: `zorder-property/brief.md` の 59・81・94 行が書くファイルの古い道 `crates/areka/src/placement/…`）。
- **Adjacent expectations**:
  - `session-crate-split`（同じウェーブ）は本体の `runtime_tests/zorder_wiring_tests.rs` を直し、`crates/areka-placement/`・`crates/areka-runtime/`（パスの名指しのコメントのほか）・本体の `placement_tests/` に触らない。`doc/COMPAT_ARCHITECTURE.md` §8 では向こうがパスの行、こちらが名の行（別々の既にある行＝許す重なり）を直す。
  - `dola-cue-test-builder`（同じウェーブ）は、呼び手の名（runtime の `carrier_cue`・`actor_carrier_cue` ほか）を残す形を取り、`crates/areka-runtime/src/{window_state_cue_tests,zorder_cue_tests,zorder_wiring_tests}.rs` に触らない。名も消す形が選ばれたときは向こうが C25 から外れる。`consumer_ledger_test_support.rs` は、向こうが 126 行からの塊を、こちらが 28 行のコメントだけを直す（許す重なり・要件 6.5）。
  - `audit-areka-app`（C26 の見込み）は、本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本をこの spec と同じ外し方で改め、同じコミットでそのファイルの名指し（この spec が改めた `areka-runtime` の名を指す 4・5・10・11・13 行）を新しい名へ直す（要件 6.6 の申し送り）。
  - `zorder-property`・`zorder-chain-residue`・`extra-character-windows` は、この spec の着地の後に新しい名を読む。

## Requirements

### Requirement 1: 札の外し方

**Objective:** As a 開発者, I want テストの名から作業番号の札だけを外したい, so that 名を読めば何を確かめるテストかが分かり、札の意味を探しに spec の文書を辿らなくて済む

#### Acceptance Criteria

1. The 改名 shall テストの関数の名の頭の札（`t_` と、英小文字と数字の続きと、その後ろの `_`）を外し、残りの綴りをそのまま新しい名とする（例: `t_c3_char_n_prefix_does_not_add_scope_n` → `char_n_prefix_does_not_add_scope_n`）
2. If 札を外した名が、同じモジュールの中の別のテストや関数の名とぶつかる, then the 改名 shall 名の後ろにそのテストの中身を表す語を足して、ぶつからない名にする（2026-11-22 の数えでは、ぶつかる組は 0）
3. If 札を外した名が、別のファイル（別のモジュール）のテストの名と同じになる, then the 改名 shall その同じ名をそのまま許す（2026-11-22 の数えでは 6 組＝`window_state_cue_tests.rs` と `zorder_cue_tests.rs` の 3 組、`zorder_group_ledger_rebase_tests.rs` と `zorder_group_ledger_state_tests.rs` の 1 組、本体の `placement_tests/` の `t_zbc_`・`t_zic_` の 1 組、`zorder_chain_compose_tests.rs` の `t_zcc09_…` と札の無い既存の `zorder_chain_tail_order_tests.rs` のテストの 1 組・research.md 2.2）
4. The 改名 shall 札の外の語の綴りを変えない（言い回しの直しをこの spec で行わない）
5. Where 札が数字を持たない形（本体の `placement_tests/` の `t_zbc_`・`t_zic_`）, the 改名 shall 同じ規則で外す
6. The 改名 shall 札でない意味のある頭（`spine_`・`real_`・`row1_` など）を持つ名を変えない

### Requirement 2: 改める名の範囲

**Objective:** As a 開発者, I want 札つきの名を範囲の中で漏れなく改めたい, so that 札を写す手本が残らず、新しいテストが札を写し続けることが止まる

#### Acceptance Criteria

1. The 改名 shall `crates/areka-placement/src/` の 20 ファイルと `crates/areka-runtime/src/` の 8 ファイルの、札つきのテストの名を全部改める（2026-11-22 の数えで 282 本）
2. The 改名 shall 本体の `crates/areka/src/placement_tests/zorder_group_branch_coverage_tests.rs`・`zorder_inversion_branch_coverage_tests.rs` の札つきのテスト各 1 本を改める
3. The 改名 shall 本体の `crates/areka/src/runtime_tests/zorder_wiring_tests.rs` に触らない
4. When 着手のときに札の形の検索で数え直す, the 改名 shall 起票の後に足された札つきのテスト（上の範囲のファイルのほか、要件 6 の約束の相手の持ち物でないファイルに在るもの）も同じ規則で改め、数え直しの結果を実装の記録に残す

### Requirement 3: 振る舞いを変えない

**Objective:** As a 開発者, I want 名だけを変え、テストが確かめる中身は 1 つも変えたくない, so that 改名で檻が弱まったり、テストが黙って消えたりしない

#### Acceptance Criteria

1. The 改名 shall テストの本文・期待値・属性（`#[test]`・`#[ignore]` など）・テストを組み込む `mod` の宣言を変えない
2. The 改名 shall 改名の前後で、各 crate のテストの本数を同じに保つ
3. The 改名 shall 本番のコード（テストでない関数・型・定数）を変えない（`crates/areka-placement/src/config.rs` は同じファイルの中のテストの `mod` の名だけを改める。本番のファイルで変えてよいのは、テストを名指すコメントだけ＝`zorder_group_ledger.rs:667` ほか）
4. The 改名 shall テストのファイルの名と置き場を変えず、1,000 行を越えるファイルを作らない
5. When 全部の改名と名指しの直しが済んだ, the 全体テスト shall 整形と静的検査の段を含めて緑で通る

### Requirement 4: 名指しを新しい名へ直す

**Objective:** As a テストを読み書きするセッション, I want 名を変えたテストを指す所が新しい名を指していてほしい, so that 文書やコメントから辿った名でテストが見つかる

#### Acceptance Criteria

1. When あるファイルのテストの名を改める, the 変更 shall 同じコミットで、そのテストの名指し（コードのコメント・名を文字列で並べる表・文書）を新しい名へ直す（途中のコミットでも表の検査が赤にならない）
2. The 名指しの直し shall コードのコメントの中の名指し（doc コメントの中のリンクを含む）を新しい名へ直す
3. The 名指しの直し shall 本体の `placement_tests/` の 2 ファイルの表が並べるテストの名を新しい名へ直し、表の検査のテスト（`check_table` を呼ぶもの）が緑で通る
4. The 名指しの直し shall `doc/COMPAT_ARCHITECTURE.md` §8 の、名を変えたテストを名指す行（226・354・412 行と、札だけの短い書き方の 196 行）の名だけを直し、その行のほかの記述と、ほかの行を変えない
5. The 名指しの直し shall 未完了の spec の文書（`extra-character-windows/brief.md`・`zorder-property/brief.md`）の名指しを新しい名へ直す。札だけの短い書き方（`zorder-property/brief.md` の `t_zpd40`）も、新しい名の全体へ直す
6. If 着手のときの数え直しで、steering やほかの未完了の spec の文書に名指しが見つかる, then the 名指しの直し shall それも新しい名へ直す
7. The 名指しの直し shall 歴史の場所（`.kiro/specs/completed/`・`doc/archive/`・`.kiro/steering/roadmap-history.md`）を書き換えない
8. The 名指しの直し shall `doc/COMPAT_ARCHITECTURE.md` 412 行が今は無いテストの昔の名として書く「旧 `t_c3_char_n_prefix_adds_scope_n`」と、同じ行の「テスト T-C3」を書き換えない（今は無いテストを歴史として書く文で、改名の対象のテストの名指しではない）
9. The 名指しの直し shall 札だけの短い書き方（`t_zgp20`・`t_zpd10` ほか・後ろに名が続かないもの）を、その札を持つテストの新しい名の全体へ直す。いくつもの札を範囲で書く所（`t_zgp1`〜`t_zgp14` ほか）は、番号を使わない文で言い直す
10. The 名指しの直し shall 注釈に書かれた設計の作業番号（大文字の `T-K5`・`T-R8`・`T-N10`・`T-N3` ほかと、`t_` の無い札 `zwi01／02` ほか）を消す。テストを指す働きを持つ所は、そのテストの新しい名で書き直す（範囲は `crates/areka-placement/`・`crates/areka-runtime/`・`crates/areka-emo-present/` の 2 行。本体の `runtime_tests/zorder_wiring_tests.rs` は要件 6.6）
11. Where 札を外した後に別のファイルの同じ名のテストが在り、名だけではどちらを指すか読めない（`version_moves_only_when_the_contents_move` の 2 本ほか）, the 名指しの直し shall 名にファイルの名を添えて書く（例: 「`zorder_group_ledger_rebase_tests.rs` の `version_moves_only_when_the_contents_move`」）

### Requirement 5: 最後の確かめ

**Objective:** As a 開発者, I want 札が残っていないことと古い名の名指しが残っていないことを、検索の結果で確かめたい, so that 「全部外した」という主張が人の目でなく数えで裏付けられる

#### Acceptance Criteria

1. When 全部の改名が済んだ, the 確かめ shall `crates/` の札の形の検索（`fn t_[a-z]+[0-9]+[a-z]?_`）の当たりが、本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本だけであることを示す
2. When 全部の改名が済んだ, the 確かめ shall 数字を持たない札の形の検索（`fn t_[a-z]+_`）の当たりが、`zorder_branch_coverage_test_support.rs` の検査の見本の文字列だけであることを示す
3. When 全部の改名が済んだ, the 確かめ shall 改めたテストの古い名のどれもが、リポジトリの中で歴史の場所とこの spec の文書のほかに当たらないことを示す
4. The 確かめ shall 検索の式・当たりの数・当たった場所を実装の記録に残す（0 件の主張には、同じ式が改める前の数え〔284 本〕を当てることを添えて、式が当たる力を持つことを示す）
5. When 全部の改名が済んだ, the 確かめ shall 札だけの短い書き方の検索（`t_[a-z]+[0-9]+[a-z]?`）と大文字の作業番号の検索（`T-[A-Z]{1,4}[0-9]{1,2}[a-z]?`）の `crates/`・`doc/`・steering・未完了の spec の文書での当たりが、記録した例外（本体の `runtime_tests/zorder_wiring_tests.rs` の申し送った行・`doc/COMPAT_ARCHITECTURE.md:412` の歴史の文・テストの名でない局所変数の `t_wait1`・`t_wait2`）だけであることを示す

### Requirement 6: 同じウェーブの相手との約束（C25）

**Objective:** As a 並走するセッション, I want この spec が約束の外のファイルに触らないでいてほしい, so that 3 本の PR が互いにぶつからずに着地できる

#### Acceptance Criteria

1. The 改名 shall 本体の crate では `crates/areka/src/placement_tests/` の 2 ファイルだけに触る（`session-crate-split` が直す `runtime_tests/`・`session/` ほかに触らない）
2. The 改名 shall `crates/areka-runtime/src/{window_state_cue_tests,zorder_cue_tests,zorder_wiring_tests}.rs` ではテストの名と名指しだけを変え、合図の組み立ての呼び出し（`carrier_cue` ほか）とその名に触らない（`dola-cue-test-builder` の約束）
3. The 名指しの直し shall `doc/COMPAT_ARCHITECTURE.md` §8 では名の行だけを直し、`session-crate-split` が直すパスの行に触らない
4. If 着手のときに、範囲のファイルを触る別の spec（`zorder-property`・`zorder-chain-residue` ほか）が走っている, then the 改名 shall そのファイルを後に回し、そのことを実装の記録に残す
5. The 名指しの直し shall `dola-cue-test-builder` の持ち物 `crates/areka-runtime/src/consumer_ledger_test_support.rs` では 28 行のコメントだけを直し（向こうが直す 126 行からの塊に触らない・許す重なり）、手を付ける前にそのことを向こうのセッションへ知らせる
6. The 変更 shall `.kiro/specs/completed/audit-areka-app/brief.md` へ、本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本の改名と同じコミットで、そのファイルの 4・5・10・11・13 行の名指しを、この spec が改めた新しい名へ直すことを申し送る

### Requirement 7: 札をまた付けない決まり

**Objective:** As a 開発者, I want 作業番号を名に付けない決まりを書いておきたい, so that 設計の文書のテストの番号が、また実装のテストの名へ写されない

#### Acceptance Criteria

1. The 変更 shall CLAUDE.md の `## Comments` の「`.kiro` の spec・要件・タスクの番号をコードに書かない」の行に、テストの名にも書かないことを足す（1 行の中の語の足しだけ・ほかの行を変えない）
2. The 変更 shall 札を見張る照合のテストを足さない
