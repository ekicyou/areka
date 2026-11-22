# ギャップ分析: dola-cue-test-builder

> 調べた時点: ブランチ `claude/kiro-start-dola-cue-test-builder-294439`（`09234de8` ＝ main `00877299` ＋ spec の初期化だけ）。分析の型は `kiro-validate-gap` の `rules/gap-analysis.md`（今の姿 → 要件の実現の見込み → 案 A/B/C → 規模と危うさ → 設計の段への申し送り）。

## 1. 分析の要約

- **前例はそろっている**: ワークスペースの `test-support` 機能（`areka-placement`・`areka-input`・`areka-runtime`）が、`[features] test-support = [...]` ＋ `#[cfg(any(test, feature = "test-support"))] pub mod test_support;` ＋ 使い手の `[dev-dependencies]` に同じ依存を `features = ["test-support"]` で書く形で 3 つある。`dola` に同じ形を足せば、要件 2（既定の公開の面を変えない）を満たせる。
- **5 つの写しの引数の形は全部同じ**（演者 `&str`・名 `&str`・トークン `&[&str]`、演者を取らない 2 つは `"0"` を固定で渡す）。`dola` の組み立てを `(actor: &str, name: &str, tokens: &[&str]) -> TalkCue` にすれば、5 つとも本文 1 行で委ねられる。型の変換（`String`・`Vec`・seriko の `scope` の型）の食い違いは無い。
- **`Cargo.lock` は変わらない見込み**: 使い手 3 crate はどれも既に `dola` に依存している（ghost は dev、seriko・runtime は本番）。機能は `Cargo.lock` に載らないので、機能の行と dev の行を足しても錠は動かない（実装の後に `--locked` のビルドで確かめる）。
- **本体 `crates/areka/` に触らずに済む**: 本体は `areka-runtime` を dev で `features = ["test-support"]` 付きで引いている。runtime の `test-support` に `"dola/test-support"` を足せば、本体の 5 ファイルが使う `areka_runtime::consumer_ledger::test_support::carrier_cue` はそのまま引ける。
- **残る裁定**: 置き場（機能の下の関数か、常に公開の組み立てか）・組み立ての名と形（自由関数か `TalkCue` の付属関数か）・docs.rs の表に機能を足すか・README の書き方・同じファイルの中だけの写し 3 つを 1 行の名で残すか直に呼ぶか。

## 2. 今の姿

### 2.1 合図の型と正準形（`dola`）

| 物 | 在処 | 要点 |
|---|---|---|
| `TalkCue` | `crates/dola/src/cue/command.rs:288` | 公開のフィールド 4 つ（`at`・`actor`・`command`・`duration`）。`impl TalkCue` は 1 つも無い |
| `CueCommand::command_carrier(name: impl Into<String>, tokens: Vec<String>)` | 同 `:183` | 正準形 `Custom { command, params: Array([String…]) }` を組む唯一の口 |
| `CueCommand::as_command_carrier(&self) -> Option<(&str, Vec<&str>)>` | 同 `:195` 付近 | 正準形の開き口 |
| `ActorKey` | 同 `:22`、`From<&str>`（`:36`）・`From<String>`（`:42`） | |
| 正準形の檻 | `crates/dola/src/cue/command_tests.rs:489〜` | `command_carrier` の往復・空のトークン・ワイヤの形・非正準の `None` を既に固定 |
| 公開の面 | `crates/dola/src/cue/mod.rs:22〜32` | `command` と `sink` は私有、型は `pub use` で出す。`runtime`・`schedule`・`sheet` は `pub mod` |

- `crates/dola/Cargo.toml` の `[features]` は `toml = ["dep:toml"]` だけ。`[package.metadata.docs.rs] features = ["toml"]`。`publish = true`。
- `crates/dola/README.md` は英語。`toml` 機能の説明が本文に 2 か所ある（機能の説明の前例）。
- 大きさ: `command.rs` 305 行・`command_tests.rs` 586 行（1,000 行の線に遠い）。

### 2.2 5 つの写し（引数の形の照合）

| 写し | 形 | 演者 | 本文 | 呼び手 |
|---|---|---|---|---|
| ghost `prop_sink.rs:118`（`#[cfg(test)] mod tests` の中の私有） | `carrier_cue(actor: &str, name: &str, tokens: &[&str])` | 引数 | `TalkCue { at: 0.0, actor: ActorKey::from(actor), command: command_carrier(..), duration: 0.0 }` | 同じファイル 5 か所 |
| ghost `prop_get_sink_tests.rs:18`（私有） | `carrier(name: &str, tokens: &[&str])` | `"0"` 固定 | 同上 | 同じファイル 8 か所ほど |
| seriko `actor_test_support.rs:124`（`pub(super)`・`actor.rs:812` で `#[cfg(test)]`） | `carrier_cue(scope: &str, name: &str, tokens: &[&str])` | 引数（名が `scope` なだけで `&str`） | 同じファイルの `cue(0.0, scope, command_carrier(..), 0.0)`（`lib_test_support.rs:124` の `cue` は `TalkCue` を直に組むだけ） | seriko の 5 ファイル |
| runtime `consumer_ledger_test_support.rs:126・131`（`pub`・`consumer_ledger.rs:389` で `cfg(any(test, feature = "test-support"))`） | `carrier_cue(name: &str, args: &[&str])`／`actor_carrier_cue(actor: &str, name: &str, args: &[&str])` | `carrier_cue` は `"0"` で `actor_carrier_cue` へ委ねる | `actor_carrier_cue` が本文を持つ | runtime 12・本体 5 |
| runtime `frame_window_link_drain_tests.rs:31`（私有） | `carrier(actor: &str, name: &str, tokens: &[&str])` | 引数 | 同上の直書き | 同じファイル 6 か所ほど |

- 呼び手の数を grep で数え直した: runtime 12・seriko 5・本体 5 ＝ 22（要件と一致）。`crates/areka-sakura/src/compile_sheet_tests.rs` の 1 件はテストの名の語だけ。
- 5 つとも `(&str, &str, &[&str])` に読み替えられる。seriko の `scope` も `&str`。`Vec`・`String` を取る写しは無い。
- 写しを消すと使われなくなる取り込みがある: runtime `frame_window_link_drain_tests.rs:7` の `ActorKey`・`CueCommand`（`TalkCue` は `move_scope1` の戻りで残る）。ghost の 2 ファイルと runtime の `consumer_ledger_test_support.rs` は、同じファイルの他のテスト（`CueCommand::Text`・`broken_custom_cue`）が使うので残る。seriko の `actor_test_support.rs` は `cue`・`CueCommand` を他の組み立て（`emote_cue`・`noncanonical_custom_cue`）が使うので残る。

### 2.3 ワークスペースの `test-support` 機能の前例

| crate | 機能の行 | 開く口 | 使い手の書き方 |
|---|---|---|---|
| `areka-placement` | `test-support = []` | `lib.rs:89` `#[cfg(any(test, feature = "test-support"))] pub mod test_support;`（ほか `reseed.rs`・`measure.rs` にも同形） | runtime・areka-input・本体の `[dev-dependencies]` に `areka-placement = { path = "../areka-placement", features = ["test-support"] }`（runtime は `[dependencies]` にも同じ依存を機能なしで持つ＝本番の表の同じ依存に機能を足すだけ） |
| `areka-input` | `test-support = []` | 同形 | 本体の dev |
| `areka-runtime` | `test-support = ["dep:shiori-host32-host", "dep:crash-report-kit", "areka-placement/test-support"]` | `consumer_ledger.rs:389` ほか | 本体・areka-input の dev。**他 crate の `test-support` を前へ送る前例（`areka-placement/test-support`）がここに在る** |

- 根の `Cargo.toml` は `resolver = "2"`。dev の依存の機能は、dev の依存を組むとき（`cargo test`・examples）だけ本番の依存と合わさる。本番の配布物は `tools/package.ps1:580` の `cargo build --locked --release -p areka -p areka-mcp-bridge` で組むので、`dola/test-support` は入らない（runtime の `test-support` が本体の本番のビルドに入らないのと同じ理屈・同じ前例）。
- `cargo test --workspace`（`tools/test-all.ps1:75`）の 1 回の中では、機能が合わさって `dola` は `test-support` 付きの 1 つに組まれる。これは既存の前例（placement・runtime）と同じで、本番の振る舞いは変わらない（足すのは関数 1 つで、既存の型や判定は変わらない）。
- 公開の crate に機能を足す前例は `dola` の `toml` だけ。テスト用の機能を持つ公開の crate は、ワークスペースにはまだ無い（`wintf`・`wintf-winmsg-executor` にも無い）。

### 2.4 公開前の確認（`tools/crates-io.ps1`）

- 機能を特別に扱う判定は無い（判定は「一覧」「欄」「理由」「版」「公開の段の形」「大きさ」）。包むのは `cargo package --no-verify --allow-dirty --locked --offline -p dola -p wintf -p wintf-winmsg-executor`（`-Verify` のときは `cargo publish --dry-run`＝既定の機能で組む）。
- 空の機能 `test-support = []` は依存を持たないので、包むことにも `--locked` にも響かない見込み。
- `cargo package` は `Cargo.toml` を正規化して包む（コメントは `Cargo.toml.orig` にだけ残る）。**機能の行のコメントは crates.io の利用者には見えず、見えるのは README と機能の名の一覧だけ**（要件 2.5 の「機能の行の説明」はリポジトリの読み手向け、README が公開の説明の本体になる）。
- `tools/test-all.ps1:76` の `cargo test -p dola --features toml` の段は変わらない。`test-support` だけを付けた `dola` の単独の組み立ての段は無いが、`cargo test --workspace` の中で `dola` が非テストの lib として `test-support` 付きで組まれるので、`cfg(feature = "test-support")` の枝はそこで必ずコンパイルされる。

### 2.5 見張りのテスト

- `crates/log-capture-kit/tests/sample_path_guard_test.rs`・`workspace_scan/mod.rs` は `crates/**/Cargo.toml` の `[dependencies]` にテスト専用の kit が無いかを見る。`dola` は kit でなく、足すのは機能の行と dev の行なので当たらない。
- `TalkCue {` や `command_carrier(` の字面を数える見張りは無い（grep で 0）。写しのファイルを `include_str!` で読む見張りも無い。
- `crates/ukadoc-survey/tests/consistency/non_vacuity.rs:81` は `crates/dola/src/lib.rs` を名指す。`lib.rs` を触らない置き方なら響かない（`cue/mod.rs` か `cue/` の新しいファイルに置く限り触らない）。

## 3. 要件と資産の対応

| 要件 | 今ある物 | ギャップ |
|---|---|---|
| 1.1 組み立てを 1 つ | `command_carrier`（コマンドだけ） | **Missing**: `TalkCue` 全体（at 0・長さ 0・演者）を組む口が `dola` に無い |
| 1.2 トークンの順・空・`--key=value` | `command_carrier` が保証し、`command_tests.rs:495〜` が固定 | 無い（委ねるだけで満たす） |
| 1.3 往復 | `as_command_carrier` と既存の檻 | 無い（組み立ての結果に対して 1 本足す） |
| 1.4 演者の既定を持たない【A2】 | ― | 無い（引数で必ず取る形にするだけ） |
| 1.5 `dola` のテストで固定 | `command_tests.rs` の流儀 | **Missing**: 組み立ての檻。兄弟の `_tests.rs` に置く流儀に合わせる |
| 2.1〜2.3 `test-support` 機能【A1】 | 前例 3 crate | **Missing**: `dola` の `[features]` の行・`cfg` の口・使い手 3 crate の dev の行・runtime の前送り |
| 2.4 `crates-io.ps1` が通る | 機能を特別に扱わない | **Constraint**: 空の機能なら通る見込み。実装の後に回して確かめる |
| 2.5 Cargo.toml の説明と README | README に `toml` の説明の前例 | **Missing**: 説明の文。**Constraint**: Cargo.toml のコメントは包むと消える（README が公開の説明） |
| 2.6 外の crate を足さない・`Cargo.lock` 不変 | 3 crate とも既に `dola` に依存 | 無い見込み（**Research Needed**: 実装後に `--locked` で確かめる） |
| 3.1〜3.4 写しの本文を消す | 5 つの写し | 引数の形が全部同じなので 1 行で委ねられる |
| 3.5 呼び手 22 ファイルに触らない | 名を残せば触らない | 無い |
| 3.6 同じファイルの中だけの写し | ghost 2・runtime 1 | **Unknown（設計で決める）**: 1 行の名で残すか、直に呼ぶか |
| 4.1〜4.3 振る舞いを変えない | ― | 無い（型・判定・非正準の組み立てに触らない） |
| 5.1〜5.6 範囲 | ― | `crates/areka/`・根の `Cargo.toml`・`Cargo.lock` に触らない置き方が在る（2.3） |

## 4. 実現の案

### 案 A: 既存を延ばす ― `dola` に `test-support` 機能を足し、機能の下に自由関数を置く（要件の【決め 1】の形・brief の ⑵）

- 置き場: `crates/dola/src/cue/mod.rs` に `#[cfg(any(test, feature = "test-support"))] #[path = "test_support.rs"] pub mod test_support;`、中身は新しい `crates/dola/src/cue/test_support.rs` に関数 1 つ（例 `pub fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue`）。檻は兄弟の `test_support_tests.rs`（または `command_tests.rs` へ足す）。
- `Cargo.toml`: `dola` に `test-support = []`。ghost の dev の `dola` の行に `features = ["test-support"]`。seriko の `[dev-dependencies]` に `dola = { path = "../dola", features = ["test-support"] }` を足す（本番の表の同じ依存に機能を足すだけ・placement の前例）。runtime は `test-support` に `"dola/test-support"` を足し、`[dev-dependencies]` に `dola` の行を足す（自身の `cargo test -p areka-runtime` は自分の `test-support` を開けないので dev の行が要る）。
- 写し: 5 つとも本文を `dola::cue::test_support::carrier_cue(actor, name, tokens)` の 1 行にする（演者を取らない 2 つは `"0"` を渡す）。
- ✅ ワークスペースの前例と同じ形（`test_support` の名の口・`cfg(any(test, feature))`）。読み手が迷わない。
- ✅ 既定の公開の面は不変（docs.rs の表にも足さなければ、docs.rs の文書も今と同じ）。
- ✅ 本体・`Cargo.lock`・根の `Cargo.toml` に触らない。
- ❌ 4 crate の `Cargo.toml` に触る（どれも機能の行か dev の 1 行）。
- ❌ crates.io の機能の一覧に `test-support` の名が出る（中身は docs.rs に出ない）。

### 案 B: 新しく組む ― `TalkCue` に常に公開の組み立てを足す（brief の ⑴）

- 置き場: `crates/dola/src/cue/command.rs` の `TalkCue` の下に `impl TalkCue { pub fn command_carrier(actor: impl Into<ActorKey>, name: …, tokens: …) -> Self }` のような付属関数。機能の行は要らない。
- ✅ 触る `Cargo.toml` が 0（`dola` も使い手も）。差分が最も小さい。
- ✅ 本番のコードも使える（ただし今は本番に使い手が無い）。
- ❌ 要件 2.1〜2.3・2.5（【決め 1】）と食い違う。公開の API が本番で使わない口のぶん増え、crates.io の利用者に向けて「at 0・長さ 0」の意味（瞬時の合図）を約束することになる。
- ❌ 一度公開すると消すのは破壊的な変更になる（`dola` は 0.x なので許されはするが、版の段で扱う手間が増える）。

### 案 C: 混ぜる ― 機能の下に置くが、名は `TalkCue` の付属関数にする

- 置き場: `command.rs` に `#[cfg(any(test, feature = "test-support"))] impl TalkCue { pub fn carrier(actor: &str, name: &str, tokens: &[&str]) -> Self }`（または `test_support.rs` の中の `impl TalkCue`）。
- ✅ 呼び手が `TalkCue::carrier(..)` と型から引ける（取り込みが `TalkCue` だけで済み、写しのファイルで使われなくなる取り込みが減る）。
- ✅ 公開の面は案 A と同じく機能の裏。
- ❌ ワークスペースの前例（`test_support` の名のモジュール）と形が違い、`cfg` の付いた `impl` は「どこで開くか」が grep で見つけにくい。
- ❌ 将来 `TalkCue` に本番の組み立てを足すとき、名の衝突を考える必要が出る。

### 捨てた案

- **brief の ⑶（新しい leaf の crate）**: 要件 5.3・C25 の約束（`Cargo.lock` の席は `session-crate-split`）で取らない。
- **seriko の `cue` 経由を残す**: seriko の写しだけ `cue(0.0, scope, …, 0.0)` を通しているが、`cue` は `TalkCue` を直に組むだけなので、`dola` の組み立てへ委ねても結果は同じ。`cue` は他の組み立てが使うので消さない。

## 5. 規模と危うさ

- **規模: S**（1〜2 日・3〜5 タスク）: `dola` の関数 1 つと檻、`Cargo.toml` 4 つの行、写し 5 つの 1 行化、README の数行。前例どおりで、新しい仕組みは無い。
- **危うさ: 低**: 既存の型・判定・本番の経路に触らない。気をつけるのは (1) 写しを消した後の使われない取り込み（警告）、(2) runtime の dev の `dola` の行の書き忘れ（`cargo test -p areka-runtime` 単独で赤になる）、(3) `Cargo.lock` が動かないことの確かめ、の 3 つで、どれもビルドで分かる。

## 6. 設計の段への申し送り

### 6.1 決めること（要件の討議の議題の種）

1. **置き場**: 要件の討議の【決め 1】で案 B（常に公開）は外れた。案 A（機能の下の自由関数）と案 C（機能の下の付属関数）のどちらにするかを設計で決める。
2. **組み立ての名と引数の形**: 名（`carrier_cue`・`command_carrier_cue`・`TalkCue::carrier` ほか）と、引数を `(&str, &str, &[&str])` に固定するか、`impl Into<ActorKey>`・`impl Into<String>`・`IntoIterator` に広げるか。5 つの写しは全部 `(&str, &str, &[&str])` なので、固定の形で 1 行の委譲が成り立つ。
3. **演者の既定**: 要件の討議の【決め 2】で確定済み（`dola` は既定を持たず、runtime の `carrier_cue` と ghost の `carrier` が `"0"` を渡す）。設計で決めることは無い。
4. **docs.rs の表**: `[package.metadata.docs.rs] features` に `test-support` を足すか（足すと docs.rs に組み立てが載る＝公開の文書の面が増える。足さなければ docs.rs は今と同じ）。
5. **README の書き方**: README（英語）に `test-support` の節を足すときの置き場と言い方（`toml` の説明の型に合わせて、「テストだけが使う組み立てを開ける・本番では付けない」を書く）。Cargo.toml のコメントは包むと消えるので、公開の説明は README が担う。
6. **同じファイルの中だけの写し 3 つ（要件 3.6）**: ghost の `prop_sink.rs` の `carrier_cue`・`prop_get_sink_tests.rs` の `carrier`、runtime の `frame_window_link_drain_tests.rs` の `carrier` を、1 行の名で残すか、呼び手が `dola` の組み立てを直に呼ぶ形にするか。1 行で残すほうが差分が小さく、直に呼ぶほうが名の層が 1 つ減る（`prop_get_sink_tests.rs` は呼び手ごとに `"0"` を書き足すことになる）。
7. **檻の置き場**: 新しい兄弟の `_tests.rs` か、既存の `command_tests.rs` の正準形の節へ足すか。

### 6.2 確かめが要るもの（Research Needed）

- `Cargo.lock` が動かないこと: 機能の行と、既に依存のある crate への dev の行だけなので動かない見込み。実装の後に `git diff --exit-code Cargo.lock` と `--locked` のビルドで確かめる。
- `tools/crates-io.ps1` の包むだけの形が空の機能 `test-support = []` で通ること（判定に機能の扱いは無いので通る見込み）。
- `cargo test -p areka-runtime`・`-p areka-seriko`・`-p areka-ghost`・`-p dola` を単独で回して、dev の行の書き忘れが無いこと（`cargo test --workspace` だと機能が合わさって書き忘れが隠れる）。

## 7. 状態と次の段

- 分析の型: 既存の前例（`test-support` 機能 3 crate）と 5 つの写しの引数の照合を中心に、`crates-io.ps1`・`test-all.ps1`・`package.ps1`・見張りのテストを読んだ。外のクレートの調べは要らなかった（新しい依存は無い）。
- 次: 要件の討議（`/kiro-requirements-discussion dola-cue-test-builder`）で 6.1 の議題を決め、その後 `/kiro-design dola-cue-test-builder`。

---

# 設計の段の記録（2026-11-22・`/kiro-spec-design -y`）

> 調べた時点: ブランチ `claude/kiro-start-dola-cue-test-builder-294439`（`716b279d`）。main は `20cbb202` へ進んだが、間に入ったのは roadmap の文書だけで、この spec が触るファイルは動いていない。

## 8. 要約

- **発見の型**: 軽い発見（Extension）。新しい外の依存は無く、外の調べ（WebSearch）は要らない。前例（ワークスペースの `test-support` 機能 3 crate）と 5 つの写しの今の字面を読み直して、ギャップ分析の事実を確かめた。
- **確かめた事実**:
  - 5 つの写しの本文と取り込みは §2.2 のとおり。写しを消して使われなくなる取り込みは runtime `frame_window_link_drain_tests.rs:7` の `ActorKey`・`CueCommand` だけ（同じファイルの他の行に出ない）。ghost の 2 ファイルは `ActorKey`・`CueCommand` を `CueCommand::Text` の合図でまだ使う。
  - seriko の写しの呼び手 5 ファイルは `use super::test_support::*;`（`actor.rs:813` の `#[cfg(test)] #[path = "actor_test_support.rs"] mod test_support;`）のグロブで引く。グロブは再輸出（`pub(super) use`）も拾うので、写しの本文を再輸出 1 行に替えても呼び手は変わらない。
  - runtime の呼び手は `use crate::consumer_ledger::test_support::{carrier_cue, …}`・`actor_carrier_cue` を名で引き、本体の 5 ファイルは `areka_runtime::consumer_ledger::test_support::carrier_cue` を名で引く。名が残れば、本文が関数でも再輸出でも呼び手は変わらない。
  - `dola` の `cue/mod.rs` は `command` を私有のモジュールに置き、型を `pub use` で出す。組み立てのモジュールは `cue/` の中に新しいファイルで足せ、`crates/dola/src/lib.rs`（`ukadoc-survey` の見張りが名指す）に触らない。
  - `crates/dola/README.md` は英語で、`toml` 機能の説明が About の段落と Key features の箇条に 1 つずつ在る。
  - 全体テストの段（`tools/test-all.ps1`）は `cargo test --workspace`・`cargo test -p dola --features toml`・`tools/crates-io.ps1` の 3 つがこの spec に響く。どれも足す段は要らない。
  - 本番の配布物は `tools/package.ps1` の `cargo build --locked --release -p areka -p areka-mcp-bridge` で組む。dev の依存の機能は本番の組み立てに入らない（runtime の `test-support` と同じ理屈）。

## 9. 設計で決めたこと（§6.1 の議題への答え・設計の討議で確かめる）

| 議題（§6.1） | 決めた形 | 理由 |
|---|---|---|
| 1 置き場 | 案 A＝`dola::cue::test_support` モジュール（`#[cfg(any(test, feature = "test-support"))] pub mod test_support;`）の自由関数 | ワークスペースの前例（placement・input・runtime）と同じ形で、`test_support` の字面で grep すれば開く口が見つかる。案 C（`cfg` 付きの `impl TalkCue`）は前例と形が違い、将来の本番の組み立てと名がぶつかりうる |
| 2 名と引数 | `pub fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue` | 5 つの写しが全部この形。`impl Into<…>` へ広げても使い手が 0 で、型推論の曖昧さだけが増える（Simplification）。名は写し 3 つ（ghost `prop_sink.rs`・seriko・runtime）と同じ `carrier_cue` |
| 3 演者の既定 | 要件の【決め 2】のまま（`dola` は既定を持たない） | ― |
| 4 docs.rs の表 | `[package.metadata.docs.rs] features` は `["toml"]` のまま（`test-support` を足さない） | 足すと docs.rs の文書の面がテスト専用の口のぶん増え、要件 2.2 の「既定の公開の面を今と同じに」と向きが逆になる。機能の存在は README が伝える |
| 5 README | About の段落の後に `test-support` の短い段落を 1 つ足す（英語・`toml` の説明と同じ調子） | Key features の箇条は利用者向けの能力の一覧で、テスト専用の口は載せる類でない |
| 6 同じファイルの中だけの写し | 引数の形が `dola` と同じもの（ghost `prop_sink.rs` の `carrier_cue`・runtime `frame_window_link_drain_tests.rs` の `carrier`）は関数を消して `use` 1 行で名を受ける（後者は `as carrier`）。演者を取らない ghost `prop_get_sink_tests.rs` の `carrier(name, tokens)` は `"0"` を渡す 1 行の関数で残す | 呼び手の行を 1 つも変えずに本文が消える。`prop_get_sink_tests.rs` を直に呼ぶ形にすると呼び手 10 行ほどに `"0"` を書き足すことになる |
| 2 と 6 の続き（呼び手の名の取り次ぎ） | 引数の形が同じ名（seriko の `carrier_cue`・runtime の `actor_carrier_cue`）は再輸出 1 行（`pub(super) use …;`・`pub use … as actor_carrier_cue;`）。runtime の `carrier_cue(name, args)` は今の本文 `actor_carrier_cue("0", name, args)` のまま | 再輸出は本文を持たない 1 行で、名と引数の形がそのまま残る（要件 3.2）。関数で包むと同じ署名を 2 か所に書くことになる |
| 7 檻の置き場 | 新しい兄弟 `crates/dola/src/cue/test_support_tests.rs`（`test_support.rs` から `#[cfg(test)] #[path = "test_support_tests.rs"] mod tests;`） | `structure.md` の兄弟の規則（`foo.rs` → `foo_tests.rs`）どおり。`command_tests.rs` は `command` の檻で、組み立ての檻を混ぜると持ち主がぼやける |

## 10. 合成（design-synthesis）の結果

- **一般化**: 写しの 5 つは「演者・名・トークンから正準のキャリアの合図を組む」1 つの働きの変種で、違いは演者 `"0"` を固定で渡すか否かだけ。働きは `dola` の 1 関数、演者の既定は呼び手の名の側（【決め 2】）に置けば全部が包める。正準でない合図（`noncanonical_custom_cue`・`broken_custom_cue`）や時刻の違う合図へ広げる一般化は、要件の外なので取らない。
- **作るか借りるか**: 借りるものは無い（外の crate の話ではない）。中身は既存の `CueCommand::command_carrier` に委ね、`TalkCue` の 4 つのフィールドを埋めるだけ。
- **削ったもの**: 引数の総称化（`impl Into`）・演者の既定を持つ 2 つめの関数・`TalkCue` の付属関数・docs.rs への掲載・`cue/mod.rs` の型の表への行の追加（表は「提供する型」の一覧で、機能の裏の関数 1 つは載せる類でない）・新しい段の `test-all.ps1` への追加（`cargo test --workspace` と各 crate の単独の確かめで足りる）。

## 11. 危うさと手当て

| 危うさ | 手当て |
|---|---|
| runtime の dev の `dola` の行を書き忘れると `cargo test -p areka-runtime` 単独だけが赤（`--workspace` では機能が合わさって隠れる） | 実装の確かめで `-p dola`・`-p areka-ghost`・`-p areka-seriko`・`-p areka-runtime`・`-p areka` を単独で回す |
| runtime の `test-support` に `"dola/test-support"` を書き忘れると、本体のテストが `areka_runtime::consumer_ledger::test_support` を組めない | `cargo test -p areka` を単独で回す |
| 本番の組み立てに `dola/test-support` が入る | `cargo tree -p areka -e normal,features -i dola` の出力に `test-support` が無いことを確かめる |
| `Cargo.lock` が動く | `git diff --exit-code Cargo.lock`（機能と、既に在る依存への dev の行だけなので動かない見込み） |
| 写しを消した後の使われない取り込み | ビルドの警告を 0 にする（runtime `frame_window_link_drain_tests.rs` の `ActorKey`・`CueCommand` が当たる） |
| テストの数が変わる | 付け替えの前と後で各 crate の `cargo test -- --list` の数を比べる（`dola` だけ檻のぶん増える） |

## 12. 参照した規則とスキル

- `.claude/skills/kiro-spec-design/rules/design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`・`design-principles.md`
- `.kiro/steering/structure.md` の Unit Tests の兄弟の規則（`foo.rs` → `foo_tests.rs`）・グロブと明示 import の解決順（E0659 の注意）・`areka-runtime` の Features の節
- `CLAUDE.md` の `## Comments`（コードと `Cargo.toml` のコメントに spec・要件・タスクの番号を書かない・`pub` の項目の doc の 1 行目は日本語）
- 外のスキル（設計・アクセシビリティ・UX）は、画面を持たないテストの支えの整理なので当たらない
