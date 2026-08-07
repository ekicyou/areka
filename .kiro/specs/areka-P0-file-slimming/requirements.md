# Requirements Document

## Project Description (Input)

**誰の問題か**: 全 spec の実装者・レビュアー・brief 保守（棚卸）。

**現状**: areka のソースファイルが肥大している（最大 8,472 行＝`crates/areka/src/placement/follow.rs`）。**本 spec の動機は単純で、1 ファイルの行数が大きすぎることそのもの**である。実測の結果、肥大の 7〜8 割は in-file の檻（`#[cfg(test)]` テストモジュール）であり、本番ロジック単体は大半が 500〜1,000 行で健全。つまり本番ロジックが健全な大きさでも、檻が同居することでファイル全体が 4,000〜8,000 行に達し、エディタ・diff・レビュー・git マージのすべてで扱いづらくなっている。副次的に、同一ファイルへ複数 spec の檻が積まれることで干渉台帳の「同一ファイル異ハンク」衝突が増幅され、檻の中を指す file:line アンカーも檻の増減のたびにずれる。

> 注（2026-08-07 実測による是正）: 本 spec 起票時の brief は「檻が本体の途中に挿入されて本番アンカーがずれる」ことを主因に挙げていたが、全数走査の結果、必須対象 48 本の檻は**すべてファイル末尾**にあり interleaved 檻は 0 件であった。実例として引かれた PR#100 `ce86995` の `input_events/balloon.rs` +155 ドリフトも、ハンク単位では**本番本体 +24／檻内 +132** であり、+132 は檻内アンカーにのみ効く。したがって「檻が本番本体のアンカーを動かす」という因果は成立しない。動機は上記のとおり**行数そのもの**である（この是正は本 spec の実施可否・スコープを変えない）。

**何が変わるべきか**: `#[cfg(test)]` の檻を本番ファイルの外の専用テストファイルへ機械的に移設し、加えて本番本体が実際に太い `follow.rs`（本体 1,996 行）と `frame.rs`（本体 1,497 行）の 2 本のみを責務単位のサブモジュールへ分割する。檻の内容・網羅は 1 行も変えず、`cargo test --workspace` 全緑かつテスト総数不変・公開 API 不変・挙動変更ゼロを受け入れの下限とする。以後の新規檻は本番ファイルの外へ書く運用を steering に明文化する。テストハーネス一本化・毒化是正（`test-cage-determinism` W6.9 の領分）と、`follow.rs`／`frame.rs` 以外の本番本体分割は範囲外。

## Introduction

本 spec は areka ワークスペースの**ファイル構造のみ**を是正する。動かすのは「檻の位置」であって「檻の中身」でも「本番のふるまい」でもない。受け入れの中心は 3 つの不変量——**テスト総数不変**・**公開 API 不変**・**挙動変更ゼロ**——であり、この 3 つが証跡で示せない変更は本 spec の成果物ではない。

**対象は areka リポジトリの全 Rust ソースファイル**である。すなわち全クレートの `src/` に加え、`tests/`（統合テスト）・`examples/`・`benches/`・`build.rs` を含む、リポジトリ配下の `*.rs` 全数を対象とする（Markdown・TOML 等の非 Rust ファイルは対象外）。

対象規模のうち `crates/*/src/**/*.rs` については 2026-08-07 の全数実測が済んでいる: 387 ファイル・総 189,190 行のうち `#[cfg(test)] mod {...}` ブロックが 92,868 行（49.1%）。**檻が 500 行を超えるファイルは 48 本**（当該 48 本の檻合計 66,830 行・檻 1,000 行超は 26 本）で、brief 実測表の 16 本はその部分集合である。`src/` 外（`tests/`・`examples/`・`benches/`・`build.rs`）を含む全域の実測値は Requirement 1.2 の再計測で確定させる。

> 注: 全体合計は 2 度の独立計測で 92,591 → 92,868（+277）へ訂正された。出所は `crates/shiori-host32-host/src/lifecycle.rs:272` の `#[cfg(test)] pub(crate) mod tests`（277 行）で、初回スキャナが `pub(crate) mod` を取りこぼしていた。当該檻は 500 行未満のため**必須対象 48 本の一覧は不変**。

移設先の形式には既存前例がある。`crates/*/src/` 配下には `#[path = "..."]` 形式は 1 件も無く、素の `#[cfg(test)] mod <name>;`（宣言のみ・実体は別ファイル）が **26 宣言ファイル・55 箇所**（うち 3 箇所は `pub(crate) mod` 宣言）で既に使われている（`areka-parsers`・`dola/runtime`・`wintf/ecs`・`crates/areka/src/emo2_boot/mod.rs` の `spine` 等）。未採用なのが檻の重いクレート群（`areka`・`areka-emo-*`・`areka-kanade`・`areka-sakura`・`areka-seriko`）である。

## Boundary Context

- **In scope**: areka リポジトリの全 Rust ソースファイル（全クレートの `src/`・`tests/`・`examples/`・`benches/`・`build.rs`）のうち檻が 500 行を超えるものの檻分離／`crates/areka/src/placement/follow.rs` と `crates/areka/src/emo2_boot/frame.rs` の本番本体分割／移設方式の単一裁定と一貫適用／新規檻の配置規律の steering 明文化／移設後の行数実測の brief 更新／移設前後のテスト総数一致の証跡採取。
- **Out of scope**: 檻の内容変更・追加・削除／テストハーネスの一本化・共有化・毒化是正・時刻注入シームの変更（`test-cage-determinism` W6.9 の領分）／`follow.rs`・`frame.rs` 以外の本番本体分割／関数分割・責務移動・ロジック書き換えを伴うリファクタ／他 spec の brief に記載された file:line アンカーの更新／檻が 500 行以下のファイルの必須移設。
- **Adjacent expectations**: `test-cage-determinism` は本 spec 着地後の**新レイアウト上で**作業する（本 spec は檻の**位置**のみを所有し、檻の**中身**は cage が所有する）。W6 以降の各 spec は design 前 rebase（既存規律）で新レイアウトを吸収し、本 spec が各 spec の brief を書き換えて回ることはしない。`cargo test --workspace` の全緑判定には i686 の host-32 成果物が事前に用意されていることを前提とする（既存の DoD 前提であり本 spec が変更するものではない）。

## Requirements

### Requirement 1: 檻と本番本体の分離

**Objective:** As a spec 実装者, I want 本番ロジックのファイルに `#[cfg(test)]` のテスト本体が同居していない状態, so that テストの追加・削除が本番コードの行アンカーをずらさなくなる

#### Acceptance Criteria

1. The areka リポジトリ shall 全 Rust ソースファイル（全クレートの `src/`・`tests/`・`examples/`・`benches/`・`build.rs`）のうち `#[cfg(test)]` 檻の合計が 500 行を超える全ファイルについて、その檻の本体を当該ファイルの外にある専用のテストファイルへ移した状態を持つ。
2. The file-slimming 実装 shall 檻分離の必須対象一覧を設計時にリポジトリ全域で全数再計測して確定する。`crates/*/src/` 限定の 2026-08-07 実測では 48 ファイル（brief 実測表の 16 ファイルを含む・檻合計 66,830 行）が該当しており、全域の再計測結果は最低でもこの 48 本を包含する。`src/` 外で新たに該当したファイルは一覧へ加える。
3. When 1 つのファイルが複数の `#[cfg(test)]` 檻を持つとき（必須対象 48 本のうち 7 本が該当・最大は `crates/areka/src/main.rs` の 7 檻）, the file-slimming 実装 shall それらを当該ファイル専用の檻の置き場へ集約する。なお 2026-08-07 の全数実測により、必須対象 48 本の檻は**すべてファイル末尾に連続配置**されており、本体の途中に挿入された interleaved 檻は 0 件である（ワークスペース全体でも `crates/wintf/src/ecs/world/mod.rs` の 1 件のみ・檻 500 行未満で対象外）。
4. Where 檻が既に本番ファイルの外に在る（例: `crates/areka/src/emo2_boot/spine.rs`＝親の `#[cfg(test)] mod spine;` で外部から gate されている）, the file-slimming 実装 shall 当該ファイルを移設対象から除外し、そのまま維持する。
5. Where ファイルの `#[cfg(test)]` 檻が 500 行以下であるとき, the file-slimming 実装 shall 当該ファイルの移設を必須としない。ただし同一ディレクトリ・同一モジュール群の一貫性のために併せて移設することを許容する。
6. Where `#[cfg(test)]` が `mod` ブロック以外の項目（`use`・`fn`・`impl`・フィールド等）に付いているとき, the file-slimming 実装 shall 当該項目の扱い（残置するか檻ファイルへ移すか）を設計で裁定し、全対象へ一貫適用する。

### Requirement 2: 移設の無変更性（テスト網羅・公開 API・挙動の保全）

**Objective:** As a レビュアー, I want 大量の機械的差分が檻の中身も本番挙動も 1 行たりとも変えていないことを証跡で確認できる, so that ファイル横断の巨大な移動をリスクなく受け入れられる

#### Acceptance Criteria

1. When 檻分離および本体分割の全作業が完了したとき, the areka ワークスペース shall `cargo test --workspace` が全緑（exit 0・失敗 0 件）になる。
2. When 移設の前後で `cargo test --workspace` の実行テスト数を比較したとき, the areka ワークスペース shall 実行されたテスト総数が完全に一致する。
3. The file-slimming 実装 shall 移設前と移設後のテスト実行数の証跡を採取し、両者の一致を示す。証跡の無い「全緑」の申告は完了根拠として認めない。
4. The file-slimming 実装 shall 檻の内容——テスト名・アサーション・入力値・期待値・属性・コメント——を変更しない。移設に伴い機械的に必要となる `use`／可視性／モジュール接続の調整、および `mod tests { ... }` ブロックをファイルの module root へ出すことで一律に発生するインデント（de-indent）の調整のみを許容する。したがって本項の検証は空白非依存の比較（`git diff -w` 相当・行頭空白を正規化した完全一致）で行う。
5. When 移設が完了したとき, the areka ワークスペース shall クレート外から観測できる公開 API（型・関数・トレイト・モジュールパス・可視性）を移設前と同一に保つ。
6. When `cargo build` を実行したとき, the areka ワークスペース shall 移設前に対して警告件数を増やさない。
7. The file-slimming 実装 shall 本番ロジックの実行時挙動を変更しない。
8. If 移設によって既存の檻がコンパイル不能になったとき, the file-slimming 実装 shall 檻のテストロジックを書き換えるのではなく、可視性・import・モジュール接続の側で解決する。

### Requirement 3: 移設方式の単一裁定と所在の予測可能性

**Objective:** As a 新規テストを書く実装者, I want どの本番ファイルの檻がどのファイルに在るかを規則から一意に導ける, so that 檻を探す・置く判断に迷わず、分離した構造が偶発的に崩れない

#### Acceptance Criteria

1. The file-slimming 実装 shall 檻の移設方式を 1 つに裁定し、全ての移設対象へ同一方式を適用する。方式ごとに使い分けることはしない。
2. When 本番ファイルのパスが与えられたとき, the areka ワークスペース shall その檻ファイルのパスを規則のみから一意に決定できる命名規約を満たす。
3. The file-slimming 実装 shall 檻ファイルから本番本体の私有項目へ従来どおり到達できる状態を保つ（in-crate 配置規律の維持——外部 `tests/` 統合テストへの移動は本 spec の移設先ではない）。
4. When 移設方式を裁定するとき, the file-slimming 実装 shall 候補方式を対比したうえで裁定根拠を記録する。対比には少なくとも、`crates/*/src/` に既に 55 箇所（26 宣言ファイル）存在する素の `#[cfg(test)] mod <name>;` 前例と、`crates/*/src/` に既存前例が 1 件も無い `#[path]` 指定形式の双方を含める。
5. If 裁定した移設方式が本番ファイル自体のパスを変更するとき（例: `foo.rs` → `foo/mod.rs`）, the file-slimming 実装 shall パスが変わる本番ファイルの全数一覧を設計で明示する（他 spec のアンカーが一度ずれる範囲がこの一覧に等しいため）。

### Requirement 4: 本番本体の分割（2 ファイル限定）

**Objective:** As a 実装者・レビュアー, I want 突出して太い本番本体が責務単位に分かれている, so that エディタ・diff・レビューで扱える大きさになる

#### Acceptance Criteria

1. The file-slimming 実装 shall `crates/areka/src/placement/follow.rs`（本番本体 1,996 行）と `crates/areka/src/emo2_boot/frame.rs`（本番本体 1,497 行）の本番本体を、責務単位のサブモジュールへ分割する。
2. The file-slimming 実装 shall 分割後の 1 ファイルあたり本番本体 1,000 行以下を目安とする。これは指針であり、責務シームを壊してまで満たす強制値ではない。
3. When 分割が完了したとき, the areka ワークスペース shall 分割前に公開されていた項目を同一の可視性で提供し続け、呼び出し側の変更をモジュールパスの追随に限る。
4. The file-slimming 実装 shall 分割に際して関数の分割・統合、責務の移動、ロジックの書き換えを行わない。許容するのは項目の移動とモジュールパスの追随のみとする。
5. The file-slimming 実装 shall `follow.rs`・`frame.rs` 以外のファイルの本番本体を分割しない（本番本体 500〜1,000 行の水準は健全と判定済み）。
6. When 分割対象 2 本（檻 6,476 行・3,163 行）の檻を移設するとき, the file-slimming 実装 shall その檻を単一の檻ファイルへ集約するか分割後のサブモジュール単位へ配置するかを設計で裁定し、いずれを選んだ場合も Requirement 2 のテスト総数不変・内容不変を満たす。

### Requirement 5: 隣接 spec の領分の非侵襲

**Objective:** As a `test-cage-determinism` および各領域を所有する spec の担当, I want 本 spec が檻の位置だけを動かし中身と他 spec の文書に触れない, so that 自 spec の前提・観測条件・作業が壊れない

#### Acceptance Criteria

1. The file-slimming 実装 shall テストハーネスの一本化・共有化、毒化の是正、時刻注入シームの変更を行わない。
2. If 移設の過程で壊れた檻・不正な檻・毒化した檻を発見したとき, the file-slimming 実装 shall 本 spec では修正せず、所見を file:line 付きで登記し、所有 spec（`test-cage-determinism` または当該領域の所有 spec）へ送る。
3. The file-slimming 実装 shall 他 spec の brief に記載された file:line アンカーを書き換えない（新レイアウトの吸収は各 spec の design 前 rebase または次回棚卸に委ねる）。
4. When 実装へ着手するとき, the file-slimming 実装 shall 他 spec の実装ブランチが同時進行していないこと（W5.95＝実装ウェーブの空白期）を確認する。
5. If 着手後に他 spec の実装が同一ファイルへ着地したとき, the file-slimming 実装 shall 当該ファイルの移設を強行せず、衝突を登記したうえで対象から一時的に外す。

### Requirement 6: 規律の明文化と実測の更新

**Objective:** As a 今後の spec 実装者, I want 新規檻の置き場が steering に明記され、現在の行数実測が最新である, so that 分離した構造が再び崩れず、次の棚卸が古い数字を引かない

#### Acceptance Criteria

1. When 本 spec が完了したとき, the areka ワークスペース shall steering（実装規律）に「新規の `#[cfg(test)]` 檻は本番本体と同じファイルに書かず、裁定された檻ファイルへ書く」旨の記述を含む。
2. The file-slimming 実装 shall 当該 steering 記述に、Requirement 3 で裁定した檻ファイルのパス命名規約を参照可能な形で含める。
3. When 移設と分割が完了したとき, the file-slimming 実装 shall 移設後の行数実測（総行・本番本体・檻）を brief の実測表として更新する。
4. The file-slimming 実装 shall 実測の更新において、更新前の値と更新後の値を対比可能な形で残す。

### Requirement 7: 段階的な着地と検証の粒度

**Objective:** As a レビュアー, I want 巨大な単一コミットではなくクレート単位の論理コミットで届く, so that 差分をクレート単位で検証でき、問題箇所を切り分けられる

#### Acceptance Criteria

1. The file-slimming 実装 shall 檻分離をクレート単位の論理コミットへ分割し、随時コミットする。全変更を単一の巨大コミットへまとめない。
2. When 各クレートの檻分離をコミットするとき, the file-slimming 実装 shall 当該クレートのテストが全緑であることを確認したうえでコミットする。
3. The file-slimming 実装 shall 檻分離のコミットと本番本体分割のコミットを分ける。
