# Requirements Document

## Project Description (Input)

**誰の問題か**: 全 spec の実装者・レビュアー・brief 保守（棚卸）。

**現状**: areka のソースファイルが肥大している（最大 8,472 行＝`crates/areka/src/placement/follow.rs`）。**本 spec の動機は単純で、1 ファイルの行数が大きすぎることそのもの**である。実測の結果、肥大の 7〜8 割はソースファイル内に同居しているテストコード（`#[cfg(test)] mod tests { ... }` ブロック。以下「テストモジュール」）であり、本番ロジック単体は大半が 500〜1,000 行で健全。つまり本番ロジックが健全な大きさでも、テストモジュールが同居することでファイル全体が 4,000〜8,000 行に達し、エディタ・diff・レビュー・git マージのすべてで扱いづらくなっている。副次的に、同一ファイルへ複数 spec のテストモジュールが積まれることで干渉台帳の「同一ファイル異ハンク」衝突が増幅され、テストモジュールの中を指す file:line アンカーもテストコードの増減のたびにずれる。

> 注（2026-08-07 実測による是正）: 本 spec 起票時の brief は「テストモジュールが本体の途中に挿入されて本番アンカーがずれる」ことを主因に挙げていたが、全数走査の結果、必須対象 48 本のテストモジュールは**すべてファイル末尾**にあり interleaved テストモジュールは 0 件であった。実例として引かれた PR#100 `ce86995` の `input_events/balloon.rs` +155 ドリフトも、ハンク単位では**本番本体 +24／テストモジュール内 +132** であり、+132 はテストコード内のアンカーにのみ効く。したがって「テストモジュールが本番本体のアンカーを動かす」という因果は成立しない。動機は上記のとおり**行数そのもの**である（この是正は本 spec の実施可否・スコープを変えない）。

**何が変わるべきか**: `#[cfg(test)]` のテストモジュールを本番ファイルの外の専用テストファイルへ機械的に移設し、加えて本番本体が実際に太い `follow.rs`（本体 1,996 行）と `frame.rs`（本体 1,497 行）の 2 本のみを責務単位のサブモジュールへ分割する。テストコードの内容・網羅は 1 行も変えず、`cargo test --workspace` 全緑かつテスト総数不変・公開 API 不変・挙動変更ゼロを受け入れの下限とする。以後の新規テストモジュールは本番ファイルの外へ書く運用を steering に明文化する。テストハーネス一本化・テスト間の状態汚染の是正（`test-cage-determinism` W6.9 の領分）と、`follow.rs`／`frame.rs` 以外の本番本体分割は範囲外。

## Introduction

本 spec は areka ワークスペースの**ファイル構造のみ**を是正する。動かすのは「コードの置き場所」であって「コードの中身」でも「本番のふるまい」でもない。受け入れの中心は 3 つの不変量——**テスト総数不変**・**公開 API 不変**・**挙動変更ゼロ**——であり、この 3 つが証跡で示せない変更は本 spec の成果物ではない。

**成果の定義は「1 ファイルの行数」で測る**。本番ファイル・テストファイルの区別なく 1 ファイル 1,000 行以下を目安とし、テストコードを別ファイルへ出しただけで 6,476 行のテストファイルが残る結果は成果として認めない（2026-08-07 の試算＝分離のみでは最大ファイル 8,472 → 6,476 行・1,000 行超のファイル 54 → 39 本にとどまる）。この目標のためにテストのモジュールパスが変わることは許容し、その代わり検証はテスト名の一致ではなく**テスト本数の一致とテスト本文の一致**で行う（Requirement 1.7／1.8／2.9）。

**対象は areka リポジトリの全 Rust ソースファイル**である。すなわち全クレートの `src/` に加え、`tests/`（統合テスト）・`examples/`・`benches/`・`build.rs` を含む、リポジトリ配下の `*.rs` 全数を対象とする（Markdown・TOML 等の非 Rust ファイルは対象外）。なお 2026-08-07 の全域実測時点で `benches/` と `build.rs` は**リポジトリ内に 1 件も存在しない**（将来の追加に備えた列挙であり、現時点の実作業は `src/`・`tests/`・`examples/` に閉じる）。

対象規模は 2026-08-07 の全域実測で確定している:

| 区分 | ファイル | 総行 | `#[cfg(test)] mod` テストコード行 |
|---|---:|---:|---:|
| `crates/*/src/**` | 387 | 189,190 | 92,868 |
| `crates/*/tests/**` | 198 | 53,798 | 5,437 |
| `crates/*/examples/**` | 34 | 14,146 | 798 |
| **合計** | **619** | **257,134** | **99,103** |

**テストモジュールが 500 行を超えるファイルは全域で 49 本**（テストコード合計 68,921 行）。内訳は `src/` の 48 本（テストコード合計 66,830 行・brief 実測表の 16 本はその部分集合）＋ `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` 1 本（2,574 行・テストモジュール 2,091 行・テストモジュール 10 個）である。

> 注: 全体合計は 2 度の独立計測で 92,591 → 92,868（+277）へ訂正された。出所は `crates/shiori-host32-host/src/lifecycle.rs:272` の `#[cfg(test)] pub(crate) mod tests`（277 行）で、初回スキャナが `pub(crate) mod` を取りこぼしていた。当該テストモジュールは 500 行未満のため**必須対象 48 本の一覧は不変**。

移設先の形式には既存前例がある。`crates/*/src/` 配下には `#[path = "..."]` 形式は 1 件も無く、素の `#[cfg(test)] mod <name>;`（宣言のみ・実体は別ファイル）が **26 宣言ファイル・55 箇所**（うち 3 箇所は `pub(crate) mod` 宣言）で既に使われている（`areka-parsers`・`dola/runtime`・`wintf/ecs`・`crates/areka/src/emo2_boot/mod.rs` の `spine` 等）。未採用なのがテストコード量の多いクレート群（`areka`・`areka-emo-*`・`areka-kanade`・`areka-sakura`・`areka-seriko`）である。

## Boundary Context

- **In scope**: areka リポジトリの全 Rust ソースファイル（全クレートの `src/`・`tests/`・`examples/`・`benches/`・`build.rs`）のうちテストモジュールが 500 行を超えるもののテスト分離／1,000 行を超えるテストファイルのテーマ単位分割／`crates/areka/src/placement/follow.rs` と `crates/areka/src/emo2_boot/frame.rs` の本番本体分割／移設方式の単一裁定と一貫適用／新規テストモジュールの配置規律の steering 明文化／移設後の行数実測の brief 更新／移設前後のテスト総数一致の証跡採取。
- **Out of scope**: テストコードの内容変更・追加・削除／テストハーネスの一本化・共有化・テスト間の状態汚染の是正・時刻注入シームの変更（`test-cage-determinism` W6.9 の領分）／`follow.rs`・`frame.rs` 以外の本番本体分割／関数分割・責務移動・ロジック書き換えを伴うリファクタ／他 spec の brief に記載された file:line アンカーの更新／テストモジュールが 500 行以下のファイルの必須移設。
- **Adjacent expectations**: `test-cage-determinism` は本 spec 着地後の**新レイアウト上で**作業する（本 spec はテストモジュールの**位置**のみを所有し、テストモジュールの**中身**は `test-cage-determinism` が所有する）。W6 以降の各 spec は design 前 rebase（既存規律）で新レイアウトを吸収し、本 spec が各 spec の brief を書き換えて回ることはしない。`cargo test --workspace` の全緑判定には i686 の host-32 成果物が事前に用意されていることを前提とする（既存の DoD 前提であり本 spec が変更するものではない）。

## Requirements

### Requirement 1: テストモジュールと本番本体の分離

**Objective:** As a spec 実装者, I want 本番ロジックのファイルに `#[cfg(test)]` のテスト本体が同居していない状態, so that テストの追加・削除が本番コードの行アンカーをずらさなくなる

#### Acceptance Criteria

1. The areka リポジトリ shall 全 Rust ソースファイル（全クレートの `src/`・`tests/`・`examples/`・`benches/`・`build.rs`）のうち `#[cfg(test)]` テストモジュールの合計が 500 行を超える全ファイルについて、そのテストモジュールの本体を当該ファイルの外にある専用のテストファイルへ移した状態を持つ。
2. The file-slimming 実装 shall テスト分離の必須対象一覧を設計時にリポジトリ全域で全数再計測して確定する。2026-08-07 の全域実測では **49 ファイル**（テストコード合計 68,921 行）が該当し、`src/` の 48 本＋`crates/areka-ghost/tests/ghost/spine_e2e_test.rs` である。設計時の再計測がこれと乖離する場合は差分の理由を示す。
3. When 1 つのファイルが複数の `#[cfg(test)]` テストモジュールを持つとき（必須対象 49 本のうち 8 本が該当・最大は `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の 10 個、次いで `crates/areka/src/main.rs` の 7 個）, the file-slimming 実装 shall それらを当該ファイル専用のテストモジュールの置き場へ集約する。なお 2026-08-07 の全数実測により、必須対象 48 本のテストモジュールは**すべてファイル末尾に連続配置**されており、本体の途中に挿入された interleaved テストモジュールは 0 件である（ワークスペース全体でも `crates/wintf/src/ecs/world/mod.rs` の 1 件のみ・テストモジュール 500 行未満で対象外）。
4. Where テストモジュールが既に本番ファイルの外に在る（例: `crates/areka/src/emo2_boot/spine.rs`＝親の `#[cfg(test)] mod spine;` で外部から gate されている）, the file-slimming 実装 shall 当該ファイルを移設対象から除外し、そのまま維持する。
5. Where ファイルの `#[cfg(test)]` テストモジュールが 500 行以下であるとき, the file-slimming 実装 shall 当該ファイルの移設を必須としない。ただし同一ディレクトリ・同一モジュール群の一貫性のために併せて移設することを許容する。
6. Where `#[cfg(test)]` が `mod` ブロック以外の項目（`use`・`fn`・`impl`・フィールド等）に付いているとき, the file-slimming 実装 shall 当該項目の扱い（残置するかテストファイルへ移すか）を設計で裁定し、全対象へ一貫適用する。
7. When 移設先のテストファイルが 1,000 行を超えるとき, the file-slimming 実装 shall 当該テストコードをテーマ単位のサブモジュール（複数ファイル）へ分割する。**この目安は本番ファイルとテストファイルの双方に等しく適用する**——本 spec の基準は「1 ファイルの行数」であって「本番ファイルの行数」ではない。分割はテーマの境界を壊してまで満たす強制値ではないが、6,476 行のテストファイルが残る結果は本 spec の成果として認めない。
8. When テストコードのサブモジュール分割によりテスト名（モジュールパス）が変わるとき, the file-slimming 実装 shall Requirement 2 の検証をテスト名の一致ではなく**テスト本数の一致とテスト本文の一致**で行う（Requirement 2.4 を参照）。

### Requirement 2: 移設の無変更性（テスト網羅・公開 API・挙動の保全）

**Objective:** As a レビュアー, I want 大量の機械的差分がテストコードの中身も本番挙動も 1 行たりとも変えていないことを証跡で確認できる, so that ファイル横断の巨大な移動をリスクなく受け入れられる

#### Acceptance Criteria

1. When テスト分離および本体分割の全作業が完了したとき, the areka ワークスペース shall `cargo test --workspace` が全緑（exit 0・失敗 0 件）になる。
2. When 移設の前後で `cargo test --workspace` の実行テスト数を比較したとき, the areka ワークスペース shall 実行されたテスト総数が完全に一致する。
3. The file-slimming 実装 shall 移設前と移設後のテスト実行数の証跡を採取し、両者の一致を示す。証跡の無い「全緑」の申告は完了根拠として認めない。
4. The file-slimming 実装 shall テストコードの内容——テスト関数の識別子・アサーション・入力値・期待値・属性・コメント——を変更しない。移設に伴い機械的に必要となる `use`／可視性／モジュール接続の調整、および `mod tests { ... }` ブロックをファイルの module root へ出すことで一律に発生するインデント（de-indent）の調整のみを許容する。したがって本項の検証は空白非依存の比較（`git diff -w` 相当・行頭空白を正規化した完全一致）で行う。
5. When 移設が完了したとき, the areka ワークスペース shall クレート外から観測できる公開 API（型・関数・トレイト・モジュールパス・可視性）を移設前と同一に保つ。
6. When `cargo build` を実行したとき, the areka ワークスペース shall 移設前に対して警告件数を増やさない。
7. The file-slimming 実装 shall 本番ロジックの実行時挙動を変更しない。
8. If 移設によって既存のテストモジュールがコンパイル不能になったとき, the file-slimming 実装 shall テストモジュールのテストロジックを書き換えるのではなく、可視性・import・モジュール接続の側で解決する。
9. Where Requirement 1.7 のサブモジュール分割によりテストのモジュールパスが変わるとき, the file-slimming 実装 shall テスト名（完全修飾名）の変更を許容する。許容するのは**所属モジュールパスの変化のみ**であり、テスト関数の識別子そのものの改名は許容しない。この場合、移設前後のテスト名一覧を「旧完全修飾名 → 新完全修飾名」の対応表として証跡に残し、対応表が全単射（漏れ・重複なし）であることをもって Acceptance Criteria 2 の本数一致を裏づける。

### Requirement 3: 移設方式の単一裁定と所在の予測可能性

**Objective:** As a 新規テストを書く実装者, I want どの本番ファイルのテストモジュールがどのファイルに在るかを規則から一意に導ける, so that テストモジュールを探す・置く判断に迷わず、分離した構造が偶発的に崩れない

#### Acceptance Criteria

1. The file-slimming 実装 shall テストモジュールの移設方式を 1 つに裁定し、全ての移設対象へ同一方式を適用する。方式ごとに使い分けることはしない。
2. When 本番ファイルのパスが与えられたとき, the areka ワークスペース shall そのテストファイルのパスを規則のみから一意に決定できる命名規約を満たす。
3. The file-slimming 実装 shall テストファイルから本番本体の私有項目へ従来どおり到達できる状態を保つ（in-crate 配置規律の維持——外部 `tests/` 統合テストへの移動は本 spec の移設先ではない）。
4. When 移設方式を裁定するとき, the file-slimming 実装 shall 候補方式を対比したうえで裁定根拠を記録する。対比には少なくとも、`crates/*/src/` に既に 55 箇所（26 宣言ファイル）存在する素の `#[cfg(test)] mod <name>;` 前例と、`crates/*/src/` に既存前例が 1 件も無い `#[path]` 指定形式の双方を含める。
5. If 裁定した移設方式が本番ファイル自体のパスを変更するとき（例: `foo.rs` → `foo/mod.rs`）, the file-slimming 実装 shall パスが変わる本番ファイルの全数一覧を設計で明示する（他 spec のアンカーが一度ずれる範囲がこの一覧に等しいため）。

### Requirement 4: 本番本体の分割（2 ファイル限定）

**Objective:** As a 実装者・レビュアー, I want 突出して太い本番本体が責務単位に分かれている, so that エディタ・diff・レビューで扱える大きさになる

#### Acceptance Criteria

1. The file-slimming 実装 shall `crates/areka/src/placement/follow.rs`（本番本体 1,996 行）と `crates/areka/src/emo2_boot/frame.rs`（本番本体 1,497 行）の本番本体を、責務単位のサブモジュールへ分割する。
2. The file-slimming 実装 shall 分割後の 1 ファイルあたり 1,000 行以下を目安とする（Requirement 1.7 と同一の目安であり、本番ファイル・テストファイルの双方に適用する）。これは指針であり、責務シームを壊してまで満たす強制値ではない。
3. When 分割が完了したとき, the areka ワークスペース shall 分割前に公開されていた項目を同一の可視性で提供し続け、呼び出し側の変更をモジュールパスの追随に限る。
4. The file-slimming 実装 shall 分割に際して関数の分割・統合、責務の移動、ロジックの書き換えを行わない。許容するのは項目の移動とモジュールパスの追随のみとする。
5. The file-slimming 実装 shall `follow.rs`・`frame.rs` 以外のファイルの本番本体を分割しない（本番本体 500〜1,000 行の水準は健全と判定済み）。
6. When 分割対象 2 本（テストコード 6,476 行・3,163 行）のテストコードを移設するとき, the file-slimming 実装 shall Requirement 1.7 の 1,000 行目安に従い、単一の巨大なテストファイルへ集約せず、分割後のサブモジュールに対応するテーマ単位で複数ファイルへ配置する。いずれの配置でも Requirement 2 のテスト本数不変・本文不変を満たす。

### Requirement 5: 隣接 spec の領分の非侵襲

**Objective:** As a `test-cage-determinism` および各領域を所有する spec の担当, I want 本 spec がテストコードの位置だけを動かし中身と他 spec の文書に触れない, so that 自 spec の前提・観測条件・作業が壊れない

#### Acceptance Criteria

1. The file-slimming 実装 shall テストハーネスの一本化・共有化、テスト間の状態汚染の是正、時刻注入シームの変更を行わない。
2. If 移設の過程で壊れたテストモジュール・不正なテストモジュール・テスト間で状態が汚染されているテストモジュールを発見したとき, the file-slimming 実装 shall 本 spec では修正せず、所見を file:line 付きで登記し、所有 spec（`test-cage-determinism` または当該領域の所有 spec）へ送る。
3. The file-slimming 実装 shall 他 spec の brief に記載された file:line アンカーを書き換えない（新レイアウトの吸収は各 spec の design 前 rebase または次回棚卸に委ねる）。
4. When 実装へ着手するとき, the file-slimming 実装 shall 他 spec の実装ブランチが同時進行していないこと（W5.95＝実装ウェーブの空白期）を確認する。2026-08-07 実測では他 spec の実装ブランチは 0 本であり、前提は成立している。
5. If 着手後に他 spec の実装が同一ファイルへ着地したとき, the file-slimming 実装 shall 当該ファイルの移設を強行せず、衝突を登記したうえで対象から一時的に外す。
6. The file-slimming 実装 shall 本 spec の正典ブランチを `claude/areka-p0-file-slimming-64d065`（要件・ギャップ分析・要件ディスカッションの全成果を保持）とする（開発者裁定 2026-08-07）。同一 spec の重複ワークツリー `claude/areka-p0-file-slimming-e4f098`（`f657d84`＝並行実行された古い `/kiro-start` の産物・内容は本ブランチの版に包含・未コミット作業なし）は**開発者の指示により 2026-08-07 に破棄済み**（ワークツリー削除＋ブランチ削除）。

### Requirement 6: 規律の明文化と実測の更新

**Objective:** As a 今後の spec 実装者, I want 新規テストモジュールの置き場が steering に明記され、現在の行数実測が最新である, so that 分離した構造が再び崩れず、次の棚卸が古い数字を引かない

#### Acceptance Criteria

1. When 本 spec が完了したとき, the areka ワークスペース shall steering（実装規律）に「新規の `#[cfg(test)]` テストモジュールは本番本体と同じファイルに書かず、裁定されたテストファイルへ書く」旨の記述を含む。
2. The file-slimming 実装 shall 当該 steering 記述に、Requirement 3 で裁定したテストファイルのパス命名規約を参照可能な形で含める。
3. When 移設と分割が完了したとき, the file-slimming 実装 shall 移設後の行数実測（総行・本番本体・テストモジュール）を brief の実測表として更新する。
4. The file-slimming 実装 shall 実測の更新において、更新前の値と更新後の値を対比可能な形で残す。

### Requirement 7: 段階的な着地と検証の粒度

**Objective:** As a レビュアー, I want 巨大な単一コミットではなくクレート単位の論理コミットで届く, so that 差分をクレート単位で検証でき、問題箇所を切り分けられる

#### Acceptance Criteria

1. The file-slimming 実装 shall テスト分離をクレート単位の論理コミットへ分割し、随時コミットする。全変更を単一の巨大コミットへまとめない。
2. When 各クレートのテスト分離をコミットするとき, the file-slimming 実装 shall 当該クレートのテストが全緑であることを確認したうえでコミットする。
3. The file-slimming 実装 shall テスト分離のコミットと本番本体分割のコミットを分ける。
