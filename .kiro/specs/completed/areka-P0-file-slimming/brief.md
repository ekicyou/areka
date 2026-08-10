# Brief: areka-P0-file-slimming

> **起票 2026-08-06**（`/kiro-discovery` 棚卸⑥セッション・開発者が「ソースコード 1 file 当たりの行数が肥大しているのでは」と提起）。
> 本 brief は**実測（2026-08-06・main `686ff10` 相当）を全て内包**する。別セッションはこの brief 単体で再開できる。
> **ウェーブ**: **W5.95**（単独・W6 実装より前）。今は実装が 1 本も走っていないため衝突相手ゼロ＝先行が最安。後送（W6.95 案）は W6〜W6.9 の全ウェーブがテストコード挿入によるアンカードリフト税を払い続けるため**却下**（開発者裁定 2026-08-06）。
> **並走**: 本 spec の実装中、W6 各 spec の `/kiro-start`（要件フェーズ・コード非接触）は文書フェーズ先行の規律で並走可。W6 実装は本 spec 着地後に design 前 rebase（既存規律）で新レイアウトを吸収する。

## Problem

**誰の問題か**: 全 spec の実装者・レビュアー・brief 保守（棚卸）。

ソースファイルが肥大している。最大 8,472 行（follow.rs）。実測の結果、**肥大の 7〜8 割は ファイル内テストモジュール（`#[cfg(test)]` テスト）**であり、本番ロジック単体は大半が 500〜1,000 行で健全。問題はテストモジュールと本体の同居構造そのものにある:

1. **アンカードリフトの主因**: spec がテストモジュールを本体の途中に挿入すると、後続行の全アンカーがずれる。実例＝col（PR#100）が `input_events/balloon.rs` へテスト +210 行を挿入し、bindoption-exclusivity brief の監視アンカーが +155 ドリフト（棚卸⑥で検出・追記(60)）。棚卸のたびに全 brief の file:line 再監査が必要になる税が構造的に発生している。
2. **同一ファイル干渉の増幅器**: 干渉台帳の「同一ファイル異ハンク」衝突（presenter.rs 直列鎖・cage⇄vis 等)の一部は、複数 spec 分のテストモジュールが同じファイルに積まれることで発生・悪化する。
3. **編集・レビューの人間工学**: 4,000〜8,000 行のファイルはエディタ・diff・レビューの全てで扱いづらい。

### 実測（起票時 2026-08-06 → 群 1〜7 着地 2026-08-09 `6c5cb70` → 群 8 完了 2026-08-10 `5e36218`）

**全域値は 3 時点とも同一条件のスキャナ `verification/Measure-TestModules.ps1`（`.kiro/` 除外済みの版）による全数再計測である。更新前の出所はタスク 1.1 のコミット済み成果物 `verification/scan_raw.csv` ／ `scan_summary.txt`、上位ファイルの per-file 値は下表の起票時実測（main `686ff10` 相当）である。群 8 完了時点の値はタスク 8.13 が採り直した。**

#### リポジトリ全域

| 指標 | 更新前（2026-08-07 `f05537e`） | 群 7 着地（2026-08-09 `6c5cb70`） | **更新後**（2026-08-10 `5e36218`） |
|---|---:|---:|---:|
| 走査した `.rs` ファイル数 | 619 | 793 | **872** |
| 総行 | 257,134 | 258,342 | **259,154** |
| 本番本体 | 158,122 | 158,660 | **148,713** |
| テストコード | 99,012 | 99,682 | **110,441** |
| **最大ファイル行数** | **8,472**（`placement/follow.rs`） | 2,503（`emo2_boot/spine.rs`） | **986**（`pilot/examples/pilot-clickthrough-alpha-toggle/main.rs`） |
| **1,000 行を超えるファイル数** | **54** | 13 | **0** |
| ファイル内テストモジュールが 500 行を超えるファイル数（＝必須対象） | 49 | 0 | **0** |

ファイル数 619 → 872（+253）の内訳は、群 1〜6 が新設したテストファイル 164 本＋本番サブモジュール 10 本、群 8 が新設した 79 本（テストファイル 52 本＋サンプル・本番サブモジュール 27 本）である。総行の +2,020 は、ファサード・接続宣言・各ファイルの `use` ヘッダの増分であり、**移設・分割したコードの本文は 1 行も変えていない**（要件 2.4）。タスク 8.13 が全数を再実行して確認した内訳は、群 1〜6 の **49 本が 49/49 一致**、群 8 のうちテストを含む 8.1〜8.8 が **12 実行すべて exit 0**、テストを 1 つも含まない本体分割 8.9〜8.12 が各タスクの純移動の機械証明（説明のつかない差異 0）である。

「テストコード」の定義は 3 時点とも同一——**ファイル内に残る `#[cfg(test)] mod` ブロックの行数 ＋ `#[cfg(test)]` ＋ パス属性で接続された分離済みテストファイルの総行**である（両者に重複が無いことを毎回確認している。更新後は 29,389 ＋ 81,052 ＝ 110,441・パス接続 216 本の中に `#[cfg(test)] mod` ブロックは 0 行）。

> **⚠ テストコードが +10,759 と大きく増えて見えるのは、コードが増えたからではなく分類が是正されたからである。** 群 8 は、スキャナの定義が**本番本体として数えていたテストコードを可視化した**。内訳は再現可能な 2 つの数で説明できる: ファイル内ブロックが 30,091 → 29,389（**−702** ＝ `shiori_inproc.rs` の 446 ＋ `common/mod.rs` の `mod smoke` 256 が外部ファイルへ出た分）、パス接続ファイルが 164 本 69,591 行 → 216 本 81,052 行（**+11,461** ＝ 群 8 が新設した 52 本の総行にちょうど一致）。この 11,461 行の大半は、**`tests/` 配下で `#[cfg(test)]` を書かないファイル**（`choice_test` 1,563・`mouse_test` 1,116・`close_test` 1,009・`dola` の `runtime_test` 1,102）と、**歴史的形式で分離済みだったファイル**（`spine.rs` 2,503・`decode_tests.rs` 1,395・`golden_tests.rs` 1,356）に元から在ったテストコードである——いずれも旧定義では本番本体に計上されていた。**これは `verification/notes.md` §39.2 の見落とし出所 ① と ② そのものであり、行数の是正と同時に計測の是正でもある。**

#### 1,000 行を超えるファイルは 0 本になった（旧「残る理由」の表を差し替え）

**群 7 着地時点では 13 本が 1,000 行を超えて残っており、この節はその 13 本に「残る理由」を与えていた。開発者裁定（2026-08-09・`verification/notes.md` §39）によりその理由づけは全て却下され、13 本すべてが群 8（タスク 8.1〜8.12）で分割された。** 却下の根拠は要件 1.7 の宣言——「この目安は本番ファイルとテストファイルの双方に等しく適用する。本 spec の基準は「1 ファイルの行数」であって「本番ファイルの行数」ではない」——が上位であり、要件 4.5 の禁止条項と要件 1.4 の除外条項がこれを上書きすると読んだのが誤りだったことによる（要件 4.5 は改訂・要件 1.4 は「除外するのは移設だけで分割ではない」と明確化）。

| 行数（群 7 時点） | ファイル | 旧「残る理由」——**いずれも却下** | 却下の出所（§39.2） | 担当 | 分割後の最大 |
|---:|---|---|---|---|---:|
| 2,503 | `crates/areka/src/emo2_boot/spine.rs` | 要件 1.4 の除外（親が既に分離済み） | ② 要件 1.4 の読み違え | 8.1 | **794**（8 本） |
| 1,657 | `crates/areka-kanade/tests/kanade/common/mod.rs` | ファイル内テストモジュール 256 行（要件 1.5 の必須対象外） | ① 検出条件が届かない | 8.2 | **373**（8 本） |
| 1,563 | `crates/areka-kanade/tests/kanade/choice_test.rs` | `#[cfg(test)] mod` を持たない（要件 1.1 の対象外） | ① | 8.3 | **334**（8 本） |
| 1,434 | `crates/areka-emo-text/examples/emo-text-layer.rs` | サンプル・本体分割の範囲外（要件 4.5） | ③ 要件 4.5 が 1.7 を上書きすると読んだ | 8.10 | **783**（7 本） |
| 1,395 | `crates/areka-parsers/src/shell/decode_tests.rs` | 分離済みの歴史的形式（要件 1.4） | ② | 8.5 | **346**（10 本） |
| 1,356 | `crates/areka-emo-compose/src/golden_tests.rs` | 分離済みの歴史的形式（要件 1.4） | ② | 8.6 | **294**（8 本） |
| 1,168 | `crates/areka/examples/emo-present.rs` | サンプル・本体分割の範囲外（要件 4.5） | ③ | 8.11 | **374**（9 本） |
| 1,116 | `crates/areka-kanade/tests/kanade/mouse_test.rs` | `#[cfg(test)] mod` を持たない（要件 1.1 の対象外） | ① | 8.4 | **525**（5 本） |
| 1,102 | `crates/dola/tests/cue/runtime_test.rs` | `#[cfg(test)] mod` を持たない（要件 1.1 の対象外） | ① | 8.7 | **326**（6 本） |
| 1,066 | `crates/areka-emo-present/src/presenter.rs` | 要件 4.5 が `follow.rs`／`frame.rs` 以外の分割を範囲外としている | ③ | 8.9 | **278**（7 本） |
| 1,063 | `crates/areka/examples/collision-probe.rs` | サンプル・本体分割の範囲外（要件 4.5） | ③ | 8.12 | **330**（8 本） |
| 1,018 | `crates/areka-ghost/src/shiori_inproc.rs` | ファイル内テストモジュール 446 行（要件 1.5 の必須対象外） | ①（閾値未満・要件 1.5 で併せ移設） | 8.8 | **581**（4 本） |
| 1,009 | `crates/areka-kanade/tests/kanade/close_test.rs` | `#[cfg(test)] mod` を持たない（要件 1.1 の対象外） | ① | 8.4 | **520**（4 本） |

**13 本・合計 17,450 行が 92 本へ分かれた**（元の 13 本はハブ／ファサードとして残り、新設は 79 本）。**残る最大は 986 行**（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs`）で、以下 974（`areka-emo-text/src/draw.rs`）・962（`areka/src/main.rs`）・960（`areka-ghost/src/runtime_tests.rs`）・950・945・942・937 と続く。**1,000 行超は 0 本**で、要件 1.7 を全域で充足する。

1,000 行超 0 本はタスク 8.13 が**独立な 2 通りの物差し**で確認した（`verification/notes.md` §53.5）: (a) `git ls-files --cached --others --exclude-standard -- '*.rs'` の 880 本（`target/`・`.git/` は gitignore と git 内部で除外・`.kiro/` のフィクスチャ 8 本を**含む**）と、(b) 上記スキャナの 872 本（`target｜vendors｜.git｜node_modules｜.claude/worktrees｜.kiro` を除外）。いずれも **0 本**。populate されている `vendors/pasta` サブモジュール（`.rs` 160 本・最大 961 行）を足しても 0 は崩れない。**ファイル内テストモジュールが 500 行を超えるファイルも 0 本**（最大 485 行）。

#### 起票時に挙げた上位ファイルの内訳

**「更新後」列は群 8 完了時点（`5e36218`）の値である。** 群 7 着地（`6c5cb70`）から変わったのは `presenter.rs`（タスク 8.9）と `spine.rs`（タスク 8.1）の 2 行だけで、他の 14 行は群 8 が 1 バイトも触れていない。

| ファイル | 更新前 総行 | 更新前 本番本体 | 更新前 テスト | 更新後 本番（本／最大） | 更新後 テスト（本／最大） |
|---|---:|---:|---:|---:|---:|
| `crates/areka/src/placement/follow.rs` | 8,472 | **1,997** | 6,475 | 6 ／ 701（計 2,119） | 12 ／ 950（計 6,594） |
| `crates/areka-emo-present/src/presenter.rs` | 5,417 | 1,043 | 4,374 | **7 ／ 278（計 1,135）** | 8 ／ 827（計 4,474） |
| `crates/areka/src/emo2_boot/frame.rs` | 4,660 | **1,498** | 3,162 | 6 ／ 394（計 1,693） | 9 ／ 608（計 3,284） |
| `crates/areka-emo-text/src/layout.rs` | 3,294 | 750 | 2,544 | 1 ／ 764 | 5 ／ 768（計 2,558） |
| `crates/areka-kanade/src/schedule/steady.rs` | 3,286 | 904 | 2,382 | 1 ／ 918 | 4 ／ 831（計 2,396） |
| `crates/areka-emo-text/src/viewbox_draw.rs` | 3,090 | 786 | 2,304 | 1 ／ 803 | 6 ／ 685（計 2,339） |
| `crates/areka-emo-text/src/actor.rs` | 2,967 | 858 | 2,109 | 1 ／ 880 | 6 ／ 681（計 2,130） |
| `crates/areka/src/input_events/balloon.rs` | 2,825 | 830 | 1,995 | 1 ／ 847 | 6 ／ 753（計 2,030） |
| `crates/areka-sakura/src/drive.rs` | 2,808 | 531 | 2,277 | 1 ／ 542 | 4 ／ 837（計 2,299） |
| `crates/areka/src/emo2_boot/spine.rs` | 2,503 | （全量テストスパイン） | 2,503 | — | **8 ／ 794（計 2,555）** |
| `crates/areka-seriko/src/actor.rs` | 2,331 | 485 | 1,846 | 1 ／ 493 | 3 ／ 928（計 1,852） |
| `crates/areka-emo-present/src/balloon.rs` | 2,264 | 633 | 1,631 | 1 ／ 644 | 4 ／ 685（計 1,638） |
| `crates/areka-emo-compose/src/plan.rs` | 2,203 | 668 | 1,535 | 1 ／ 678 | 3 ／ 942（計 1,548） |
| `crates/areka-kanade/src/schedule/mod.rs` | 2,176 | 670 | 1,506 | 1 ／ 687 | 2 ／ 882（計 1,489） |
| `crates/areka/src/placement/mod.rs` | 1,899 | 564 | 1,335 | 1 ／ 575 | 4 ／ 567（計 1,351） |
| `crates/areka-emo-compose/src/scale.rs` | 1,778 | 468 | 1,310 | 1 ／ 478 | 3 ／ 809（計 1,310） |

<details>
<summary>起票時（2026-08-06）の実測表・原文</summary>

| ファイル | 総行 | 本番本体 | テストモジュール |
|---|---:|---:|---:|
| `crates/areka/src/placement/follow.rs` | 8,472 | **1,997** | 6,475 |
| `crates/areka-emo-present/src/presenter.rs` | 5,417 | 1,043 | 4,374 |
| `crates/areka/src/emo2_boot/frame.rs` | 4,660 | **1,498** | 3,162 |
| `crates/areka-emo-text/src/layout.rs` | 3,294 | 750 | 2,544 |
| `crates/areka-kanade/src/schedule/steady.rs` | 3,286 | 904 | 2,382 |
| `crates/areka-emo-text/src/viewbox_draw.rs` | 3,090 | 786 | 2,304 |
| `crates/areka-emo-text/src/actor.rs` | 2,967 | 858 | 2,109 |
| `crates/areka/src/input_events/balloon.rs` | 2,825 | 830 | 1,995 |
| `crates/areka-sakura/src/drive.rs` | 2,808 | 531 | 2,277 |
| `crates/areka/src/emo2_boot/spine.rs` | 2,503 | （全量テストスパイン） | 2,503 |
| `crates/areka-seriko/src/actor.rs` | 2,331 | 485 | 1,846 |
| `crates/areka-emo-present/src/balloon.rs` | 2,264 | 633 | 1,631 |
| `crates/areka-emo-compose/src/plan.rs` | 2,203 | 668 | 1,535 |
| `crates/areka-kanade/src/schedule/mod.rs` | 2,176 | 670 | 1,506 |
| `crates/areka/src/placement/mod.rs` | 1,899 | 564 | 1,335 |
| `crates/areka-emo-compose/src/scale.rs` | 1,778 | 468 | 1,310 |

注: frame.rs はテストモジュールが本体に**散在**（:299-345 帯等）しており、trailing だけでなく interleaved のテストモジュールも対象。

</details>

注（更新後の訂正）: 上の「散在」は起票時の見立てである。2026-08-07 の全数実測で、必須対象 49 本に interleaved のテストモジュールは **0 件**と確定した（要件 1.3）。また起票時の `follow.rs` 本番本体 1,997 ／ テスト 6,475 は、厳密計測（ブロック外枠を移設側に数える定義）では 1,996 ／ 6,476 であり、要件 4.1 以降はこの値を採っている。

## Current State

- テストモジュールは [[areka-bin-crate-internal-tests-in-crate]] の規律で in-crate 配置——ただしこの規律は「**in-crate**」であって「**in-file**」ではない。`#[cfg(test)] mod tests;`（別ファイル）でも `super::` 経由の私有アクセスは保たれ、規律違反にならない。
- テストモジュールの実体は決定論テスト・ログテスト・property テストなど多様で、`capture_logs` 等の共有ハーネスを含む（ハーネス一本化は `test-cage-determinism` W6.9 の領分＝本 spec は触らない）。
- 本体が実際に太いのは `follow.rs`（1,997）と `frame.rs`（1,498）の 2 本のみ。**〔起票時の記述。実装後はこの 2 本ともファサード分割済み。厳密計測では `follow.rs` の本番本体は 1,996 が正。群 7 着地時点では本番本体が 1,000 行を超えるファイルが `presenter.rs`（1,066）1 本だけ残っていたが、これも群 8（タスク 8.9）でファサード分割され、**現在は本番本体・ファイル総行のいずれでも 1,000 行を超えるファイルは 0 本**である——上の実測表を参照。〕**

## Desired Outcome

- **テストモジュールの兄弟ファイル分離**: 上表の全ファイルで、`#[cfg(test)]` テストモジュールを兄弟ファイル（例 `follow.rs` → `follow_tests.rs` を `#[cfg(test)] #[path = "follow_tests.rs"] mod tests;` で接続、またはディレクトリモジュール化 `follow/mod.rs`＋`follow/tests.rs`——方式は設計で統一裁定）へ移設。**テストコードの内容・網羅は 1 行も変えない**。
- **本体分割（2 本のみ）**: `follow.rs`（1,997）と `frame.rs`（1,498）の本番本体を責務単位のサブモジュールへ分割。目安＝1 ファイル 1,000 行以下（強制ではなく指針）。
- **不変量**: `cargo test --workspace` 全緑＋**テスト総数不変**（移設でテストモジュールを 1 本も失わない——[[areka-log-cage-harness-blindspots]] の教訓＝「無いこと」は静かに壊れる）。公開 API 不変。挙動変更ゼロ。
- 以後の新規テストモジュールは兄弟ファイルへ書く運用を steering（実装規律）へ 1 行明文化。

## Approach

**機械的移設に徹する**（ロジック変更・テスト改善・ハーネス統一は全て Out）:

1. 移設方式を 1 つに統一裁定（`#[path]` 兄弟ファイル方式 vs ディレクトリモジュール化——import 追随コスト・`#[path]` の既知の癖〔[[harness-shell-quirks]] は examples 限定の話であり src 内は無関係〕を設計で比較）。
2. テスト分離: 上表 16 ファイル＋α（500 行超のテストモジュールを持つ残りは設計時に全数再計測して確定）。interleaved テストモジュール（frame.rs :299-345 帯等）は同じ兄弟ファイルへ集約。
3. 本体分割: follow.rs・frame.rs のみ。責務シームは既存のフェーズ構造（frame.rs は 7 フェーズ・follow.rs は追従/遷移/persist 系）に沿う。
4. 検証: テスト総数の前後比較（`cargo test --workspace` の実行数一致）＋全緑＋`cargo build` 警告増ゼロ。

## Scope

- **In**: テストモジュールの兄弟ファイル移設（全クレート・500 行超テストモジュール）・follow.rs/frame.rs の本体分割・移設方式の統一裁定・運用規律 1 行の steering 追記・移設後の行数実測表の brief 更新。
- **Out**: テストコードの内容変更・追加・削除／ハーネス一本化・テスト間の状態汚染の是正（`test-cage-determinism` W6.9 の領分）／follow.rs・frame.rs 以外の本体分割（500〜1,000 行の本体は健全）／リネーム以上のリファクタ（関数分割・責務変更）。

## Boundary Candidates

- テストモジュール移設（クレート単位で独立・並列可能な機械作業）
- follow.rs 本体分割（placement 系）
- frame.rs 本体分割（emo2_boot 系)

## Out of Boundary

- `test-cage-determinism` の全領分（capture_logs 統一・状態汚染・注入シーム）。本 spec が先行するため、cage は W6.9 着手時に**新レイアウト上で**作業する（cage brief のアンカーは cage 着手時再監査が既存義務＝追加コストなし）。
- 各 spec の brief アンカー更新——本 spec 着地後の最初の棚卸（または各 spec の design 前 rebase）で吸収する。本 spec が全 brief を書き換えて回ることはしない。

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手可能・実装ウェーブ空白期が観測条件）。
- **Downstream**: W6 以降の**全 spec**（slim なファイルと安定アンカーの恩恵）・`test-cage-determinism`（新レイアウト上で作業）・`emo2-conformance-e2e`（着手時 brief 全面再監査で新レイアウト吸収）。

## Existing Spec Touchpoints

- **Extends**: なし（新規境界）。
- **Adjacent**: `test-cage-determinism`（テストモジュールの**位置**は本 spec・テストモジュールの**中身**は cage——この線引きが境界の核）。W6 の 5 spec（文書フェーズのみ並走・コード非接触）。

## Constraints

- Rust 2024・Windows 専用。挙動変更ゼロ・公開 API 不変・テスト総数不変が受け入れの下限。
- [[obsolete-vs-broken-test-policy]]: 移設中に壊れたテストモジュールを見つけても本 spec では**直さない**（登記して cage または所有 spec へ送る）。
- [[areka-commit-as-you-go]]: クレート単位の論理コミットで随時コミット（巨大 1 コミット禁止）。
- 実装は機械作業だが、[[kiro-verify-completion]] のとおりテスト数一致の証跡を移設前後で採取すること。
