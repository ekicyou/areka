# X1-V: 横断プロジェクト設定 × 脆弱性レビューと非破壊対策（依存監査・設定の脆弱性点検）

- status: no-change
- commit: （境界内設定ファイルの変更ゼロ。docs コミット — 断片 X1-V.md ＋ proposals.md への P73〜P75 追記のみ）

本セルは X1 領域（横断プロジェクト設定）の V セルであり、**全60セルタスクの最後**。性質は**非挙動変更**（脆弱性点検＋挙動非破壊な対策のみ）。design.md「Security Considerations」L516 が X1 領域の V 観点へ内包すると定める **依存監査（`cargo audit` 相当）** を主点検とし、依存固定・`.gitignore`・`.gitmodules` の安全性、および P71（publish=true）の公開リスク観点を点検した。結論は **no-change**（実在し挙動非破壊対策が妥当な脆弱性は存在せず、挙動を変える候補＝依存パッチ更新・Cargo.lock 追跡方針・cargo audit の CI 導入はすべて P73〜P75 として記録）。

## 観点・基準・範囲

- セルID: X1-V（領域 X1「横断プロジェクト設定」 × 観点 V「脆弱性」）。
- requirements（source 番号）: 1.4（横断設定を独立領域化）・2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更を伴う対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V。X1-T/X1-S 完了済みの上で実行）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations（L512-516、特に L516「依存監査は X1 領域の V 観点で `cargo audit` 相当の調査を行い、依存更新は挙動影響を評価のうえ慎重に適用」）、Revalidation Triggers（L48-52、公開ポリシー変更＝R2.9 前提が崩れる）、レビューマトリクス X1 行定義（L168）・V 観点（L178）、R2.6 の依存監査内包判断（L182）、セル断片様式（L440）・提案記録様式（L453）。
- 境界（boundary）: ルート `Cargo.toml`・`crates/*/Cargo.toml`・`.gitignore`・`.gitmodules`・`.vscode/` の設定ファイルのみ。**ソース/テストコード・`vendors/` 配下は一切触れていない。**
- 起点: X1-S 適用後のクリーンなワークツリー（親検証済みベースライン 1713 passed / 0 failed / 32 ignored）。

## 依存監査の手段と結果（本セルの明示的完了条件）

### 手段: `cargo audit` が導入済み → 実行（手動アドバイザリ突き合わせは補助）

- **`cargo audit` は導入済み**（`cargo audit --version` = `cargo-audit-audit 0.22.2`）。未導入時の代替（依存一覧＋公開アドバイザリ突き合わせ）ではなく、ツール本体を実行した。
- 実行: `cargo audit`（RustSec Advisory DB を `c:\rust\cargo\advisory-db` から取得、**1132 件のアドバイザリをロード**、`Cargo.lock` の **300 クレート依存**をスキャン）。
- 各検出について `cargo tree --workspace -i <crate>` で**到達経路（プロダクション or dev-only）を実測**し、`c:\rust\cargo\advisory-db` のローカル advisory ファイルを直接読んで影響条件を確認した（推測ではなく一次情報で裏取り）。

### 結果: 脆弱性（vulnerability）0 件 / 情報的警告（informational）5 件

`cargo audit` の集計は **`error: N vulnerabilities found` ではなく `warning: 5 allowed warnings found`**（＝**脆弱性 0 件**、情報的警告のみ）。検出 5 件の内訳（実測）:

| ID | クレート@版 | 区分 | 到達経路（`cargo tree -i` 実測） | プロダクション混入 | 判定 |
|----|------------|------|-------------------------------|------------------|------|
| RUSTSEC-2026-0097 | `rand@0.10.0` | unsound（情報的） | `dola`（直接 dep）→ wintf → areka。dev-dep でも wintf | **あり**（areka.exe に含む） | 条件未成立で**到達不能**＝現状安全。パッチ更新は挙動影響評価のうえ P73 |
| RUSTSEC-2026-0097 | `rand@0.9.2` | unsound（情報的） | `pasta_core@0.1.6`（vendored submodule）→ dola → wintf → areka | **あり**（areka.exe に含む） | 同上（条件未成立）。供給元は `vendors/pasta`（境界外・R1.5 で改変禁止）。P73 |
| RUSTSEC-2024-0436 | `paste@1.0.15` | unmaintained（情報的） | `image`（**wintf の dev-dependency**）→ ravif → rav1e → paste（proc-macro） | **なし**（dev-only） | 出荷バイナリに非混入。proc-macro でビルド時のみ。現状安全。P74 参照 |
| RUSTSEC-2026-0105 | `core2@0.4.0` | unmaintained + yanked（情報的） | `image`（**wintf の dev-dependency**）→ ravif → rav1e → bitstream-io → core2 | **なし**（dev-only） | 同上（dev-only）。現状安全。P74 参照 |

#### RUSTSEC-2026-0097（rand unsoundness）— 最重要・プロダクション混入だが到達不能（一次情報で裏取り）

唯一プロダクションへ混入する検出。ローカル advisory（`advisory-db/crates/rand/RUSTSEC-2026-0097.md`）を直接読み、**UB 発火に必要な全条件**を確認した。advisory 記載の発火条件（全て成立して初めて UB）:

1. rand の **`log` feature と `thread_rng` feature が有効**であること
2. **カスタムロガー（`impl log::Log`）が定義**されていること
3. そのカスタムロガーが **`rand::rng()`（旧 `thread_rng()`）を呼び `RngCore`/`TryRng` メソッドを実行**すること
4. その最中に `ThreadRng` が**再シード**（64KB 生成ごと）すること
5. trace レベルログ有効、または warn レベル＋getrandom がシード供給不能

本ワークスペースでの実測（推測なし）:

- **条件1 不成立（rand の `log` feature オフ）**: `Cargo.lock` の `rand@0.10.0` / `rand@0.9.2` の `dependencies` に **`log` は不在**（0.10.0 = chacha20/getrandom/rand_core、0.9.2 = rand_chacha/rand_core のみ。`grep` で rand の依存に `"log"` ゼロ）。rand の `log` feature が無効＝**rand はログレコードを一切発行しない**ため、再シード時にカスタムロガーへ再入する経路そのものが存在しない。なお `log` クレート自体はツリーに存在する（`tracing-log` ← `tracing-subscriber`、および `image` dev-dep 経由）が、advisory が要求するのは **rand 自身の `log` feature**（rand が log を出すこと）であり、ツリーに log が居ることとは無関係。
- **条件2 不成立（カスタムロガー不在）**: ワークスペース全域で `set_logger` / `set_boxed_logger` / `impl log::Log` / `impl Log for` の**ヒット 0 件**（grep 実測）。本プロジェクトは `tracing` / `tracing-subscriber` を使用し、`log` クレートのカスタムロガー機構を使わない。`tracing-log` ブリッジは log レコードを tracing へ吸い上げる消費側であり、「`rand::rng()` を呼ぶカスタムロガー」ではない。
- `rand::rng()` の実利用箇所（参考、grep 実測）: `dola/src/runtime/facade.rs:345`（プロダクション・loop_offset 乱数）と `wintf/examples/dcomp_demo.rs:134`（example のみ）。いずれもカスタムロガー内ではない通常呼び出しで、条件3（ロガー内からの再入）を満たさない。

→ **条件1・2が独立に不成立**であり、UB（aliased mutable reference）の発火経路は存在しない。RUSTSEC-2026-0097 は本ワークスペースでは**到達不能＝現状安全**と判定（informational/unsound advisory であり、CVE 級の無条件 RCE 等ではない点も併記）。挙動非破壊の対策（コード変更）は不要。パッチ更新（下記 P73）は別途。

#### dev-only 検出（paste / core2）— 出荷バイナリに非混入

`paste@1.0.15`（RUSTSEC-2024-0436 unmaintained）と `core2@0.4.0`（RUSTSEC-2026-0105 unmaintained + yanked）は**いずれも `image`（wintf の `[dev-dependencies]`、`crates/wintf/Cargo.toml:40` `image = "0.25.9"`）経由のみ**で混入する（`cargo tree -i` 実測: image → ravif → rav1e → {paste(proc-macro) / bitstream-io → core2}）。`image` は X1-T が「全件実消費の dev-dep（統合テスト・examples・in-source test）」と確認済みで、**プロダクション `areka.exe` には含まれない**。`paste` は proc-macro（ビルド時展開のみ・実行時コードなし）、`core2` は AVIF デコード経路のテスト依存。いずれも出荷物に影響せず、かつ供給元 `image`/`ravif`/`rav1e` は外部依存（R1.5 で改変禁止）。**現状安全**（テスト・example ビルドのためにのみ存在）。

## 依存固定の安全性点検

- **`*` ワイルドカード固定はゼロ**（ルート `[workspace.dependencies]`・各クレート `[dependencies]` を全行確認）。緩すぎる指定によるサプライチェーンリスク（任意の将来版を引く）は**なし**。
- 固定の形態（cargo の caret 既定）: `human-panic="2.0.6"` / `rand="0.10.0"` / `windows="0.62.2"` 等の x.y.z 指定はいずれも**完全固定（`=`）ではなく caret 範囲**（例 `rand="0.10.0"` は `>=0.10.0, <0.11.0` を許容）。`thiserror="2"` / `tracing="0.1"` / `serde="1"` は major のみ指定で範囲が広いが、いずれも広く使われる安定クレートで一般的かつ許容範囲（緊急の固定強化を要する実害なし）。`image="0.25.9"` が dev-dep として paste/core2 を推移的に引く。
- **Cargo.lock のコミット有無（サプライチェーン上の主要所見）**: `.gitignore` 行2が `Cargo.lock` を除外しており、**Cargo.lock は git に追跡されていない**（`git ls-files Cargo.lock` 該当なし。ローカルには 72621 バイトで実在）。`git log --all -- Cargo.lock` も空＝**一度も追跡されたことがなく**、`.gitignore` への `Cargo.lock` 追加は**初回コミット f189c1b（プロジェクト基盤）からの意図的設定**。バイナリ生成ワークスペース（`areka` は `[[bin]]`）では Cargo 自身のガイダンス上ロックファイルのコミットが推奨され、未追跡だと推移的依存（上記 rand 等）の版が固定されず各環境/CI で caret 範囲の最新へ解決されうる（再現性・サプライチェーン固定性の観点）。ただし「Cargo.lock を追跡する」変更は **build が解決する依存集合を全消費者に対して固定する＝ビルド再現性の方針変更**であり、初回からの意図的除外を覆す挙動相当の変更のため、本 V セルでは適用せず **P75** に記録（後述）。

## `.gitignore` / `.gitmodules` の安全性点検

### `.gitignore`（`target` / `Cargo.lock` / `tmpclaude*` / `.vs/` / `*_test.txt` / `*_dump.txt`）

- **機密の誤除外漏れ**: 現状、機密ファイル（`.env`・`*.pem`・`*.key`・`secrets.*`・credentials・`*.local.toml` 等）の除外パターンは**ない**。ただし点検の結果、**機密に該当する追跡ファイルは 0 件**（`git ls-files` を該当パターンで grep してヒットなし）、ワーキングツリーにも機密様のファイルは存在しない。本プロジェクトはデスクトップマスコット基盤で、鍵・認証情報・ローカル機密を扱う構成が現時点で不在。
- **判定（churn 回避）**: 機密除外パターンの追記は task 本文が「投入してよい対策」に挙げるが、**実在する機密の漏洩経路がない**（追跡ファイルゼロ・ツリーに機密なし）ため、これは将来の仮想に対する投機的ハードニング（karpathy §2「Nothing speculative」/「No handling for impossible scenarios」、§3「broken でないものを触らない」）に該当する。初回コミット以来 `.gitignore` はビルド成果物・スクラッチに限定する意図的構成であり、**現状安全＝対策不要**と判定し追記しない（churn 回避）。実在の機密が将来導入される際にその時点で除外を整備するのが妥当。
- **必要物の誤除外**: `target`（成果物）・`Cargo.lock`（上記 P75 で別途論点）・`tmpclaude*`/`.vs/`/`*_test.txt`/`*_dump.txt`（スクラッチ・デバッグ一時物）はいずれも除外が正当で、ソース/テストの誤除外は**なし**（X1-T/X1-S がテスト構成・整理観点でも健全と確認済み）。

### `.gitmodules`（`vendors/pasta` → `https://github.com/ekicyou/pasta.git`）

- サブモジュール 1 件のみ。URL は **`https://`（HTTPS）** で、平文 `git://` や検証回避を伴うプロトコルではない（中間者リスクの観点で HTTPS は妥当）。参照先 `ekicyou/pasta` は本リポジトリ作者と同一 org（`ekicyou/areka`）で、`[patch.crates-io] pasta_core = { path = "vendors/pasta/crates/pasta_core" }`（ルート Cargo.toml:31-32）の patch 元として正当に結線。
- `.gitmodules` 自体はコミット固定（ピン）を持たず、固定 commit は親リポジトリの gitlink（tree エントリ）が保持する設計＝git submodule の標準。S8 ゲート（submodule 初期化済み）は X1-T が確認済み。**安全性点検上の是正不要**。なお `pasta_core`（→ rand@0.9.2 を引く）は `vendors/` 配下＝R1.5 で本ループ改変禁止のため、rand@0.9.2 の更新は pasta 側の領分（P73 で言及）。

## P71（publish=true）の公開リスク観点（補足判断・記録のみ）

X1-S が記録した **P71（areka/dola/wintf の各 `[package]` が `publish = true` で `[workspace.package] publish = false` を上書き）** について、V 観点（脆弱性・公開リスク）から補足判断した。`publish = true` は各クレートが `cargo publish` 可能＝crates.io へ誤公開され得る状態を意味し、(a) 誤公開（未成熟なバイナリ基盤クレートの意図しない公開）、(b) 公開物への機密同梱（ただし上記のとおり現状ツリーに機密なし・`include`/`exclude` 未指定で既定の VCS 管理ファイルが対象）の公開リスク観点が存在する。ただし `publish` フラグの是正（true→false）は design.md Revalidation Triggers L52「公開ポリシー変更（`publish = false` 解除）＝R2.9 の非推奨コード削除前提が崩れる」に該当する**挙動相当の設定変更**であり、かつ意図（将来公開予定 or テンプレート由来の惰性）をコードから断定できない。**X1-V でも修正は適用せず、P71 の記録を維持**（意図確認は P71 の suggestion どおり別途）。本 V セルの追加判断は「公開リスク観点でも P71 の意図確認を要する」点の補足に留め、新規採番はしない（P71 参照）。

## 適用した挙動非破壊対策

**なし**（境界内設定ファイルの変更ゼロ）。依存監査の検出 5 件はいずれも (a) プロダクション混入分（rand）は発火条件未成立で到達不能・現状安全、(b) dev-only 分（paste/core2）は出荷物非混入・現状安全であり、**挙動を変えずに投入すべき対策コードが存在しない**。依存固定は `*` ゼロで健全、`.gitignore`/`.gitmodules` は機密漏洩経路なしで現状安全。挙動を変える候補（依存パッチ更新・Cargo.lock 追跡・cargo audit の CI 導入）はすべて proposals（P73〜P75）へ。境界（設定ファイルのみ・挙動非破壊）と karpathy 原則（churn 回避・投機的変更の不採用）を遵守して**変更ゼロ**とした。

## proposals.md へ回した候補（P73・P74・P75・新規採番）

- **P73**（新規・source X1-V）: `rand` の RustSec advisory 対応パッチ更新（`rand@0.10.0 → 0.10.1`、`rand@0.9.2 → 0.9.4`）。RUSTSEC-2026-0097 の patched 版（`>=0.10.1` / `>=0.9.3`）への更新。`cargo update --dry-run` で 0.10.1 / 0.9.4 が解決可能と実測。現状は発火条件未成立で到達不能だが防御的に追随する価値あり。ただし (a) rand@0.9.2 は `pasta_core`（`vendors/` 配下・R1.5 改変禁止）が引くため更新は pasta 側の領分、(b) Cargo.lock が未追跡（後述 P75）のため `cargo update` の効果が永続コミットされない、(c) パッチでも挙動影響（乱数列の互換性等）の評価を要する、ため本ループでは適用せず記録。
- **P74**（新規・source X1-V）: 依存監査（`cargo audit`）の CI 導入。CI 新設は本ループ対象外（design.md Non-Goals / X1-T の CI 欠落所見）だが、informational/unmaintained 検出（paste/core2 等の dev 依存含む）を継続的に検知する基盤として、CI 新設仕様（P70・X1-T の CI 申し送りと統合）に `cargo audit` ジョブを含めることを提案。
- **P75**（新規・source X1-V）: Cargo.lock の追跡方針の確定。バイナリ生成ワークスペース（`areka` は `[[bin]]`）として Cargo.lock を git 追跡しビルド再現性・推移的依存の固定性を確保するか、現状の未追跡（初回コミットからの意図的設定）を維持するかの方針確定。追跡開始は build が解決する依存集合を全消費者へ固定する挙動相当の変更のため記録のみ。P71（publish 方針）と併せて公開/配布ポリシーの単一の真実源を確定するのが望ましい。

既知 proposals の再確認（重複採番なし・参照に留めた）:
- **P71**（X1-S）: publish=true 上書き。本 V セルで公開リスク観点を補足判断したが既知のため参照のみ（新規採番せず）。
- **P72**（X1-S）: `[profile.release]` 最適化。成果物特性を変える設定で本 V セルの脆弱性観点でも触れない（参照のみ）。
- **P70**（X1-T）: S2 が非既定 feature を実行しない。P74（cargo audit CI）の統合先候補として参照。

## verification (S2)

- **BEFORE**: 親検証済みベースライン（X1-S 直後 = 1713 passed / 0 failed / 32 ignored、クリーンワークツリー）を信頼し省略（本セルは設定点検タスクで境界内設定ファイルの変更ゼロ＝依存変更も適用していない。親指示「BEFORE S2 は省略可」に従う）。
- **AFTER（全量・必須）**:
  - `cargo build --workspace` → **成功**（exit 0）。
  - `cargo test --workspace --no-fail-fast` → **1713 passed / 0 failed / 32 ignored**（全 test result 行を awk 合算で実測、FAILED 行ゼロ）。ベースライン 1713/0/32 と**完全一致・増減ゼロ**（設定ファイル変更なしのため当然）。
  - 依存監査: `cargo audit`（0.22.2）実行 = **脆弱性 0 / 情報的警告 5**（`warning: 5 allowed warnings found`、`error: vulnerabilities found` なし）。`cargo tree --workspace -i {rand@0.10.0,rand@0.9.2,paste,core2,log}` で到達経路を実測。
- **増減内訳**: ±0。変更は `proposals.md`（仕様ドキュメント）と本断片のみで、cargo のビルド/テスト/成果物・境界内設定ファイルに一切影響しない。1713/0/32 維持。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy --workspace` は exit 非ゼロ（wintf lib が 20 errors + 112 warnings、dola lib 21 warnings で停止）。上位: `collapsible_if`×68・`type_complexity`×30・**`not_unsafe_ptr_arg_deref`×20（error 級）**・`question_mark`(let-else)×8・`too_many_arguments`×6（8/7・11/7・15/7 含む）・`derivable_impls`×3 等。X1-T（156件規模）・X1-S と同傾向。
- `not_unsafe_ptr_arg_deref` の `error:` 表示は **clippy 既定の `#[deny(...)]`**（プロジェクト設定ではない。境界内設定ファイルに `clippy.toml`/`[lints]`/deny 指定が**不在**であることを X1-T/X1-S に続き再確認）。raw ポインタ deref する pub 関数が `unsafe` 未マークという**ソースコード**条件で、COM/winproc 系の既レビュー領分。
- 全件**ソースコード**の指摘で X1-V 境界（設定ファイルのみ）外。S2（build/test）は緑であり S3 はブロッカーとしない規約に従い記録のみ。境界内設定に是正すべき lint 構成はなし。

## flaky

- 既知フレーキー `wintf tests/ecs cue_performance_test::bench_pop_ready_empty_queue`（壁時計ベンチ負荷依存・W8-T1 で解消済み）は本セルの全量実行で初回から pass（failed=0）。本セルは境界内設定・コード変更ゼロで cue キュー timing と無関係。**flaky 発生なし。** 再実行不要。

## 自己レビュー

- **依存監査結果の記録（本セルの明示的完了条件）を充足**: 手段（`cargo audit` 0.22.2 を実行・1132 advisories・300 crate 依存スキャン）と結果（脆弱性 0 / 情報的警告 5）、各検出の到達経路（`cargo tree -i` 実測）、最重要 RUSTSEC-2026-0097 の発火条件を advisory ファイル直読＋ワークスペース実測（rand の `log` feature オフ・カスタムロガー不在）で**到達不能と裏取り**、dev-only 検出（paste/core2 は `image` dev-dep 経由で出荷物非混入）を実測で確定。すべて一次情報（cargo audit 出力・Cargo.lock・cargo tree・advisory-db ファイル・grep）に基づき推測なし。
- 設定の脆弱性点検: 依存固定（`*` ゼロ・caret 既定・Cargo.lock 未追跡を `git ls-files`/`git log --all` で実測）、`.gitignore`（機密追跡ファイル 0 件を実測→機密除外追記は投機的ハードニングのため karpathy 原則で不採用）、`.gitmodules`（HTTPS・同一作者・patch 結線正当）を点検。P71（publish）は公開リスク観点を補足し記録維持。
- **境界遵守**: 変更は `proposals.md`（提案台帳）＋本断片のみ。ルート/各クレート Cargo.toml・`.gitignore`・`.gitmodules`・`.vscode/` の**境界内設定ファイルは一切改変せず**、ソース/テスト・`vendors/`・機能spec文書も不変。tasks.md 未更新・コミット未作成。
- **件数の実測整合**: AFTER S2 = 1713/0/32（ベースライン一致・±0）。cargo audit = 5 warnings / 0 vulnerabilities。proposals 新規採番 P73〜P75（末尾 P72 を確認のうえ連番）。すべて cargo 実測・git 実測と一致。
- 結論: 横断プロジェクト設定の脆弱性耐性は高い。依存監査の唯一のプロダクション混入検出（rand unsoundness）は**発火条件が独立に2つ不成立で到達不能**、残りは dev-only で出荷物非混入。依存固定は緩い `*` ゼロで健全、`.gitignore`/`.gitmodules` に機密漏洩経路なし。実在し挙動非破壊対策が妥当な脆弱性は皆無のため**変更ゼロ（no-change）**とし、挙動を変える候補（rand パッチ更新 P73・cargo audit CI 化 P74・Cargo.lock 追跡方針 P75）と既知 P71（publish 公開リスク）は記録に留めて churn を回避した。本セルをもって全60セルタスクの点検が完了する。
