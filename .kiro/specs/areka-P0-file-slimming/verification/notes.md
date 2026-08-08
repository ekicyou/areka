# areka-P0-file-slimming — 着手条件の記録と実装登記

- 作成: タスク 1.1「対象一覧の全域再計測と着手条件の記録」
- 計測日: 2026-08-07
- ブランチ: `claude/areka-p0-file-slimming-64d065` / HEAD `f05537e`
- 本ファイルは (a) 着手条件の記録（要件 1.2 / 1.4 / 1.5 / 3.1 / 3.5 / 5.4 / 5.6）と、(b) 実装中の所見を集める**登記ファイル**（要件 5.2 / 5.5）を兼ねる。追記義務は §7 冒頭に定める。

## 1. 再計測の条件と再現手順

再計測は `verification/Measure-TestModules.ps1` で行う。スクリプトを成果物として残すことで、以下の数値はすべて再現可能である。

```
pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Measure-TestModules.ps1
```

出力: `scan_raw.csv`（全 619 ファイルの生計測）・`target_inventory.csv`（必須対象）・`excluded_inventory.csv`（分離済み）・`scan_summary.txt`（集計と診断カウンタ）。

計測定義（research.md §2.1 / §2.2 / §2.4 と同一条件）:

- **対象** = リポジトリ配下の全 `*.rs`。`target/`・`vendors/`・`.git/`・`.claude/worktrees/` を除外。実測 **619 ファイル / 総行 257,134**。`crates/` 外に `*.rs` は 0 件、`benches/` と `build.rs` も 0 件（要件 1.1 が列挙する 5 種のうち 2 種は空集合）。
- **テストコード行** = 各 `#[cfg(test)]` 属性行から、対応する `mod` ブロックの閉じ波括弧行までの行数（属性行・閉じ括弧行を含む）の総和。
- 文字列リテラル・raw 文字列・文字リテラル・行コメント・ブロックコメント（入れ子対応）の**内部**に現れる `#[cfg(test)]` や波括弧は計上しない。
  - この除外は実効性がある: `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` は `#[cfg(test)]` の文字列一致が 11 行あるが、うち `:2242` はコメント本文中の言及であり、真のテストモジュールは 10 個である。状態追跡を行わない計測は 1 モジュール過大になる。
- 宣言のみ（`#[cfg(test)] mod <name>;`）はテストコード行に**計上しない**。既に本番ファイル外へ分離済みである証跡として別集計する（§3）。
- `#[cfg(test)]` が `mod` 以外の項目に付くものは移設対象外として別集計する（40 件・§2.3）。

### 手検証（3 ファイル・スクリプト出力と目視の突合）

| ファイル | スクリプト出力 | 目視確認 |
|---|---|---|
| `crates/areka/src/placement/follow.rs` | 総行 8,472 / 本体 1,996 / テスト 6,476 / 1 モジュール（`tests`:1997-8472） | `#[cfg(test)]` は `:1997` の 1 箇所のみ、次行が `mod tests {`、ファイル末尾 8472 行目が閉じ波括弧。8472 − 1997 + 1 = 6,476 で一致 |
| `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` | 総行 2,574 / 本体 483 / テスト 2,091 / 10 モジュール | 属性の出現行 321・473・652・945・1116・1415・1707・2033・2248・2347 がスクリプトの開始行と完全一致（`:2242` はコメント中の言及で正しく除外）。最終モジュール `s7_second_boot_record_present` は 2347-2574 = 228 行。10 モジュールの合計 2,091 で一致 |
| `crates/shiori-host32-host/src/lifecycle.rs` | 総行 548 / 本体 271 / テスト 277 / 1 モジュール（`tests`:272-548） | `:272` が `#[cfg(test)]`、次行が `pub(crate) mod tests {`、548 行目が閉じ波括弧。548 − 272 + 1 = 277 で一致。要件の注記が指摘する「初回スキャナが取りこぼした `pub(crate) mod`」を本再計測は正しく捕捉している |

## 2. 再計測結果と設計値の突合

### 2.1 必須対象（要件 1.2）— 完全一致・乖離なし

| 指標 | 設計値（design / requirements） | 再計測 | 判定 |
|---|---:|---:|---|
| 必須対象ファイル数 | 49 | **49** | 一致 |
| テストコード合計 | 68,921 | **68,921** | 一致 |
| 内訳 `src/` | 48 本 / 66,830 行 | 48 本 / 66,830 行 | 一致 |
| 内訳 `tests/` | 1 本 / 2,091 行 | 1 本 / 2,091 行 | 一致 |
| 対象クレート数 | 12 | **12** | 一致 |
| 複数テストモジュールを持つ対象 | 8 | **8** | 一致 |

`target_inventory.csv` の 49 行は、パス・総行数・テストコード行数・テストモジュール数のすべてが research.md §2.2 の表および design.md §File Structure Plan と一致する。**ファイル単位の乖離は 0 件**であり、要件 1.2 が求める「差分の理由」を要する項目は必須対象については存在しない。

複数テストモジュールの 8 本（モジュール数）: `spine_e2e_test.rs`(10)・`areka/src/main.rs`(7)・`emo2_boot/move_cue.rs`(4)・`shiori-host32-helper/src/main.rs`(4)・`emo-text/src/choice.rs`(3)・`sylphya/src/actor.rs`(3)・`emo-text/src/actor.rs`(2)・`kanade/src/schedule/mod.rs`(2)。

対象クレート 12: `areka`(13) / `areka-emo-text`(7) / `areka-kanade`(6) / `areka-emo-present`(4) / `areka-ghost`(4) / `areka-emo-compose`(3) / `areka-seriko`(3) / `wintf`(4) / `areka-sakura`(2) / `areka-sylphya`(1) / `dola`(1) / `shiori-host32-helper`(1)。

### 2.2 全域合計 — テストコード行に **−91 行**の乖離あり（理由確定）

| 区分 | research §2.1 | 再計測 | 差 |
|---|---:|---:|---:|
| `crates/*/src/**` テストコード行 | 92,868 | 92,777 | **−91** |
| `crates/*/tests/**` テストコード行 | 5,437 | 5,437 | 0 |
| `crates/*/examples/**` テストコード行 | 798 | 798 | 0 |
| 合計 | 99,103 | 99,012 | **−91** |
| ファイル数 | 619 | 619 | 0 |
| 総行数 | 257,134 | 257,134 | 0 |
| ファイル内テストモジュールブロック数 | 263 | 262 | **−1** |
| ブロックを持つファイル数 | 220 | 219 | **−1** |

**理由（ファイル単位で確定）**: `crates/dola/src/runtime/clock.rs:31` の `#[cfg(all(test, target_os = "windows"))] mod tests { ... }`（L31-121・**91 行**）。research 時点のスキャナは `cfg(...)` の中に `test` を含む変種も計上していたのに対し、本再計測は要件 1.1 の文言どおり厳密に `#[cfg(test)]` のみを計上した。差は 91 行・1 ブロック・1 ファイルであり、上表の 3 つの差分（−91 / −1 / −1）と**過不足なく一致**する。他に差の出所は無い。

**影響**: 無し。当該ファイルはテストコード 91 行で 500 行の閾値を大きく下回り、いずれの計測定義でも必須対象に入らない。またリポジトリ全域で `#[cfg(all(test, ...))]` 形式はこの 1 箇所のみである（スクリプトの診断カウンタ `loose_cfg` = 1、独立の `#\[cfg\(all\(test` 検索でも 1 件）。したがって必須対象 49 本の一覧は不変であり、本 spec のスコープ・作業量・受け入れ条件のいずれも変わらない。

### 2.3 その他の照合

| 項目 | 設計 / research | 再計測 | 判定 |
|---|---:|---:|---|
| 非 `mod` `#[cfg(test)]` 項目 | 40 件 | **40 件** | 一致（設計判断 #3 の「全数残置」の母数が確定） |
| 宣言のみ（分離済み）サイト数 | 55 箇所 | **55 箇所** | 一致 |
| 宣言元ファイル数 | 26 ファイル | **30 ファイル** | **不一致（+4）** |
| 行頭以外に現れる `#[cfg(test)]` | 記載なし | **0 件** | 属性はすべて行頭。行頭アンカーの計測で取りこぼしは無い |

宣言元ファイル数の +4 について: 独立検証として複数行正規表現による全数検索を行い、`55 occurrences across 30 files` を得た。箇所数 55 は research と一致するので、**30 が正しく research の「26 宣言ファイル」は集計誤り**である。宣言サイトは移設対象ではなく除外側（要件 1.4）の根拠にすぎないため、本 spec の作業内容には影響しない。

### 2.4 閾値判定の安定性

テストコード 500 行**以下**の最大は `crates/areka-seriko/src/bind.rs` = 485 行、500 行**超**の最小は `crates/areka-ghost/src/ticker.rs` = 504 行。486〜503 行の帯にファイルは 1 本も存在しない。すなわち閾値付近に境界ぎりぎりのファイルは無く、対象一覧は計測定義の細部（属性行を含めるか等）の揺れに対して安定している。

## 3. 除外一覧（要件 1.4）— 既に本番ファイル外へ分離済み

`excluded_inventory.csv` に全数を記録した。

- **55 本**のテストファイルが、親モジュールから `#[cfg(test)] mod <name>;`（うち `pub(crate) mod` 3 件）で条件つきに宣言され、既に本番ファイルの外に在る。宣言元は 30 ファイル、すべて `crates/*/src/` 配下。合計 19,582 行。
- 実体ファイルの解決に失敗したものは **0 件**（`(unresolved)` 行なし）。
- 要件 1.4 のとおり、これら 55 本は**移設対象から除外し、そのまま維持する**。案 C（`#[path]` 形式）へ揃え直す作業は行わない（design.md §移設方式の裁定・research R-1 の裁定）。
- 必須対象 49 本との**重複は 0 件**（分離済みテストファイル自身は `#[cfg(test)] mod` ブロックを内包しないため、閾値判定に掛からない）。
- うち 8 本は 500 行を超える: `areka/src/emo2_boot/spine.rs`(2,503)・`areka-parsers/src/shell/decode_tests.rs`(1,395)・`areka-emo-compose/src/golden_tests.rs`(1,356)・`areka-emo-compose/src/composer_tests.rs`(703)・`areka-emo-compose/src/log_firing_tests.rs`(664)・`areka-parsers/src/shell/validation_tests.rs`(639)・`areka-parsers/src/sakura/decode_tests.rs`(586)・`dola/src/runtime/interpolator/tests.rs`(539)。これらは既に分離済みであり本 spec の移設対象ではない（1,000 行目安に対するテーマ分割も本 spec は行わない）。

## 4. テストコード 500 行以下のファイルの任意移設（要件 1.5 / 設計判断 #10）

- テストコードが 1〜500 行のファイルは **170 本**（テストコード合計 30,091 行）存在する。要件 1.5 はこれらの併せ移設を「許容」するが必須とはしない。
- **裁定どおり任意移設は行わない（対象 0 件）**。差分を最小に保つことを優先し、同一ディレクトリに移設済みファイルと未移設ファイルが混在する状態は、要件 6.1 / 6.2 で steering に明文化する新規テストの配置規律によって将来自然に収束させる（design.md §裁定一覧 #10）。
- したがって本 spec が触る本番ファイルは必須対象 **49 本**に限定される。

## 5. 移設方式と本番ファイルのパス変更（要件 3.1 / 3.5）

- **採用方式 = 案 C**: 同一ディレクトリのフラット兄弟テストファイル＋パス属性による接続。接続宣言は全対象で同一文言とする。

```rust
#[cfg(test)]
#[path = "<テストファイル名>"]
mod <テストモジュール名>;
```

- **命名規約**: テストファイル名 = `<stem>_<テストモジュール名>.rs`、置き場所は本番ファイルと同一ディレクトリ。stem は本番ファイル名（拡張子除く）で、`mod.rs` は親ディレクトリ名、`main.rs` / `lib.rs` は読み替えなし。パス属性はモジュール root ファイルでは文法上省略可能だが常に明示する（`src/` と `tests/` で文言まで同一の単一方式を保つため）。
- 裁定根拠は design.md §移設方式の裁定（設計判断 #1）の対比表による。要点のみ再掲: 案 A はパス属性で読み込まれたファイルの配下で素の `mod` 宣言が E0583 となり単一方式として成立しない。案 B は本番ファイル 44 本のパスが変わる。
- **本番ファイルのパスが変わるファイルは 0 本**。要件 3.5 が求める「パスが変わる本番ファイルの全数一覧」は**空集合**であり、他 spec の brief に記載された file:line アンカーがファイルパスごと無効化される事象は発生しない。`git blame` の断絶も、`crates/areka/examples/` のパス属性 include への影響も生じない。
- 再計測はこの前提を裏づける。必須対象 49 本の形の内訳は PLAIN 43 / MODRS 3 / ROOT 2 / PATHMOD 1 であり、案 C ではこの 4 形すべてについて本番ファイルを移動しない。

## 6. 着手条件の確認（要件 5.4 / 5.6）

### 6.1 正典ブランチ（要件 5.6）

- 現在のブランチ = `claude/areka-p0-file-slimming-64d065`（HEAD `f05537e`）。要件 5.6 が定める本 spec の正典ブランチと一致する。
- `main`（`247d48a`）に対し 15 コミット先行・0 コミット遅れ。`git diff --name-only main...HEAD -- crates` は 0 ファイル（本 spec はまだ本番コードに一切触れていない）。
- 重複ワークツリー `claude/areka-p0-file-slimming-e4f098` は要件 5.6 のとおり破棄済み。`git worktree list` にもローカルブランチ一覧（17 本）にも存在しないことを確認した。

### 6.2 他 spec 実装ブランチの不在＝W5.95 の空白（要件 5.4）

実行したコマンドと出力の要点:

| コマンド | 出力の要点 |
|---|---|
| `git worktree list` | **3 本のみ**。`C:/home/maz/git/areka` `247d48a` [main] ／ `.../areka-p0-file-slimming-64d065` `f05537e`（本 spec）／ `.../epic-kepler-bdbee8` `ce7d165` |
| `git diff --stat main...claude/epic-kepler-bdbee8` | **出力空**（main との差分 0 ファイル）。epic-kepler は実装を保持していない |
| `git for-each-ref refs/heads` ＋ 各ブランチの `rev-list --left-right --count` | ローカルブランチ 17 本。main と本ブランチを除く 15 本はいずれも**ワークツリー未接続**（どこにもチェックアウトされていない）かつ main より遅れている |
| `git diff --name-only main...<branch> -- crates` | 先行差分が `crates/` に及ぶ 8 本はすべて**完了済み spec** の squash マージ後の残骸（choice-interact / choice-select-events / collision-dpi / dpi-window-vanish / emo-dpi-scaling / kero / gpu-test-crash / requirements-review）。いずれも `.kiro/specs/completed/` に対応ディレクトリが実在する |
| `.kiro/specs/` の列挙 | active spec は 16 本。本 spec 以外にブランチを持つ active spec は 0 本 |

→ **他 spec の実装ブランチの同時進行 = 0 本**。要件 5.4 の前提（W5.95＝実装ウェーブの空白期）は成立している。

## 7. 登記（実装中の所見）

**追記義務**: 以後の**全タスク**は、当該クレートのコミット（要件 7.1 のクレート単位の論理コミット）を行う**前に**、そのクレートで発見した所見を本節へ追記しなければならない。対象は要件 5.2 が定める「壊れたテストモジュール・不正なテストモジュール・テスト間で状態が汚染されているテストモジュール」および移設中に判明した既存の不整合であり、**必ず file:line を添える**。本 spec ではいずれも修正せず、所有 spec へ送る（要件 5.1 / 5.2）。要件 5.5 の一時除外、および設計が許容する「僅少超過での単一維持」の理由も本節へ登記する。

### 7.1 送付所見（所有 spec へ送る）

| # | 所見 | file:line | 送り先 | 状態 |
|---|---|---|---|---|
| 1 | 本番構造体に埋め込まれたテスト注入シーム `fail_next_render`（フィールド定義・初期化・分岐の 3 箇所）。要件 5.1 が禁じる注入シームの変更に当たるため本 spec では触れない | `crates/areka-emo-text/src/viewbox_draw.rs:117,147,485` | `areka-P0-test-cage-determinism`（W6.9） | 登記済（設計時に確定・未送付） |
| 2 | `cargo test -p areka --examples` が main 時点で既にコンパイルエラー（E0433・テストモジュール内の `crate::` 参照）。移設の前後で変わらない既存状態であり、証跡採取は `--exclude areka` で運用する | `crates/areka/src/placement/spawn.rs:879`（先行裁定の注記は `:871`） | `areka-P0-test-cage-determinism`（W6.9） | 登記済（設計時に確定・未送付） |

### 7.2 記録のみ（是正しない）

| # | 事項 | file:line | 扱い |
|---|---|---|---|
| 1 | テストモジュールの**後方**に本番コードが残る全域唯一のファイル（`impl std::fmt::Debug for EcsWorld` 5 行）。テストコード 123 行で 500 行閾値未満につき対象外 | `crates/wintf/src/ecs/world/mod.rs:710-714` | 記録のみ・是正しない |
| 2 | `#[cfg(all(test, target_os = "windows"))]` を使う全域唯一のファイル。§2.2 の −91 行乖離の出所。テストコード 91 行で対象外 | `crates/dola/src/runtime/clock.rs:31` | 記録のみ・是正しない |

### 7.3 他 spec との衝突による一時除外（要件 5.5）

**運用**: 着手後に他 spec の実装が必須対象ファイルへ着地した場合、当該ファイルの移設を**強行しない**。下表へ「ファイル・着地コミット・除外日・理由」を登記したうえで、当該ファイルを移設対象から一時的に外す。`target_inventory.csv` の行は削除せず本表を突合先とし、最終検証（テスト本数一致・本文一致）の母数から当該ファイル分を除外した根拠として用いる。

着手時点で該当は無い。

| ファイル | 着地コミット | 除外日 | 理由 |
|---|---|---|---|
| （現時点で該当なし） | — | — | — |

### 7.4 僅少超過で単一維持したテストファイル（design §テーマ分割ポリシー）

design はテーマ分割について「自然な境界が無ければ単一維持を許容し理由を記録する」と定めている（例: `placement/resolver.rs` の 1,011 行）。単一維持を選んだ場合は下表へ理由を記録する。

着手時点で該当は無い。

| テストファイル | 行数 | 単一維持の理由 |
|---|---:|---|
| （現時点で該当なし） | — | — |

## 8. 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `Measure-TestModules.ps1` | 再計測スクリプト（上記の数値をすべて再現する） |
| `scan_raw.csv` | 全 619 ファイルの生計測（500 行フィルタの再現元） |
| `target_inventory.csv` | 必須対象 49 ファイル |
| `excluded_inventory.csv` | 既に分離済みのテストファイル 55 本 |
| `scan_summary.txt` | 区分別集計・対象集計・診断カウンタ |
| `notes.md` | 本ファイル（着手条件の記録＋登記） |

## 9. 32bit ヘルパ成果物の事前ビルド（タスク 1.2・要件 2.1）

- 実施日: 2026-08-07 / ブランチ `claude/areka-p0-file-slimming-64d065`
- 位置づけ: design.md §Migration Strategy（段階着地計画）順 0「準備」の一項（i686 成果物ビルド）、および §検証 / EvidencePipeline が前提とする「最終全緑は i686 host-32 成果物ビルド後に取る」に対応する環境前提。本番コード（`crates/**`）・`Cargo.toml` には一切触れていない。
- design.md §Allowed Dependencies に「i686 host-32 成果物の事前ビルド（`cargo test --workspace` 全緑の既存 DoD 前提——本 spec は変更しない）」と明記された依存の実体化にあたる。

### 9.1 着手前の状態（この作業が取り除いた失敗）

両成果物とも不在であり、`cargo test -p shiori-host32-host` は exit **101** で停止した。最初に落ちるのは `tests/error_paths.rs` で、パニック本文は成果物の探索先とビルド手順をそのまま示している。

```
thread 'helper_abnormal_exit_is_detected_nonblocking' panicked at crates\shiori-host32-host\tests\error_paths.rs:114:5:
i686 helper exe が見つかりません（探索先: <repo>\target\i686-pc-windows-msvc\{debug,release}\shiori-host32-helper.exe）。
PowerShell で先に i686 helper をビルドしてください（Git Bash 不可）:
  cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
```

内訳: `src/lib.rs` のユニット 87 件は成果物不在でも全て緑。`tests/error_paths.rs` が 1 passed / 1 **failed** となり、cargo はここで打ち切るため後続の 4 本の E2E バイナリ（`lifecycle_cyclic_e2e` / `lifecycle_kill_e2e` / `shiori_load_e2e` / `shiori_request_e2e`）は実行に到達しない。

**正確な記録**: 失敗していたのは「実行」であって「列挙」ではない。成果物不在の状態でも `cargo test -p shiori-host32-host -- --list` は exit **0** で全 96 件を列挙できた（`--list` / `--no-run` はコンパイルまでで、ヘルパを起動しないため）。タスクの完了状態にある「列挙・実行」のうち、実際にヘルパ不在で落ちていたのは実行側のみである。

### 9.2 ビルドコマンド（PowerShell で実行）

```powershell
cargo build -p shiori-host32-helper  --target i686-pc-windows-msvc   # 49.07s / exit 0
cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc   #  7.77s / exit 0
```

`cargo` の出力はファイルへリダイレクトし、終了コードは `$LASTEXITCODE` で別途確認すること。`tee` / `tail` へパイプするとパイプライン末尾のコマンドの終了コードが返り、cargo の失敗が隠れる。

### 9.3 生成物

| 成果物 | 絶対パス | サイズ | 更新時刻 | PE machine |
|---|---|---:|---|---|
| helper 実行ファイル | `<repo>\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` | 271,872 B | 2026-08-07 23:21:27 | `0x014C`（i386 / 32bit） |
| テスト用 SHIORI DLL | `<repo>\target\i686-pc-windows-msvc\debug\shiori.dll` | 155,648 B | 2026-08-07 23:21:35 | `0x014C`（i386 / 32bit） |

`<repo>` = `C:\home\maz\git\areka\.claude\worktrees\areka-p0-file-slimming-64d065`。PE ヘッダの machine 値を実読して 32bit であることを確認済み（x64 でビルドしてしまう取り違えの検出）。`target/` は git 管理外のため、この作業によるワークツリーの追跡ファイルへの変更は本ファイルの追記のみである。

なお `shiori-host32-testdll` の i686 ビルドは `#[warn(linker_messages)]` 由来の警告を 1 件出すが、これは既存の状態であり本 spec 以前から変わらない。要件 2.6 の警告件数比較は x64 の `cargo build` を対象とするので、この 1 件は比較の母数に入らない。

### 9.4 ツールチェーン

| 項目 | 値 |
|---|---|
| 既定ツールチェーン | `stable-x86_64-pc-windows-msvc`（active・default） |
| ホスト | `x86_64-pc-windows-msvc` |
| 追加ターゲット | `i686-pc-windows-msvc`（`rustup target list --installed` に存在・追加インストール不要） |
| rustup home | `c:\rust\up` |

### 9.5 再現手順（PowerShell 必須）

1. **PowerShell（pwsh）で実行する。Git Bash では実行しない。** Git Bash の PATH には GNU coreutils の `link.exe` が入っており、これが MSVC の `link.exe` を隠す。i686 のリンク段でリンカが取り違えられビルドが失敗する。9.1 のパニック本文自身が「Git Bash 不可」と明示している。
2. リポジトリルート（本ワークツリー）で 9.2 の 2 コマンドを順に実行する。
3. 9.3 の 2 ファイルの存在を確認する。
4. `cargo test -p shiori-host32-host` が exit 0 になることを確認する。
5. 探索先を変えたい場合のみ、環境変数 `HOST32_HELPER_EXE` で exe パスを明示できる（既定は `target/i686-pc-windows-msvc/{debug,release}/` の探索）。

前提として `vendors/pasta` サブモジュールが初期化済みであること（本ワークツリーでは初期化済み）。

### 9.6 完了状態の証跡

ビルド後の `cargo test -p shiori-host32-host` は **exit 0**。

| テストバイナリ | 結果 |
|---|---|
| `unittests src\lib.rs` | ok. 87 passed / 0 failed |
| `tests\error_paths.rs` | ok. 2 passed / 0 failed |
| `tests\lifecycle_cyclic_e2e.rs` | ok. 2 passed / 0 failed |
| `tests\lifecycle_kill_e2e.rs` | ok. 1 passed / 0 failed |
| `tests\shiori_load_e2e.rs` | ok. 2 passed / 0 failed |
| `tests\shiori_request_e2e.rs` | ok. 2 passed / 0 failed |
| Doc-tests | 0 tests |

合計 **96 件・失敗 0**。9.1 で落ちていた `helper_abnormal_exit_is_detected_nonblocking` を含め全て緑になった。

`shiori_load_e2e` の出力には `LoadLibraryFailed(HRESULT(0x8007007E))` の行が現れるが、これは `load_e2e_success_fail_survival` が意図的に観測している失敗経路（LOAD 失敗後も helper が生存することの確認）であり、テストは緑である。異常ではない。

### 9.7 ヘルパ以外の消費側 — 既定のワークスペース実行では未到達

`resolve_helper_exe` 相当を持つテストは host32-host 以外に 3 本あるが、いずれも環境変数ゲートで、ヘルパ解決に到達する前に早期 return する。

| ファイル | ゲート |
|---|---|
| `crates/areka-kanade/tests/kanade/real_helper_test.rs:160` | `HOST32_PASTA_DLL` 未設定なら return |
| `crates/areka-ghost/tests/ghost/real_pasta_test.rs:153` | 同上 |
| `crates/areka-ghost/tests/ghost/snapshot_capture_test.rs` | `HOST32_PASTA_DLL` ＋ `AREKA_SNAPSHOT_OUT` の両設定時のみ動作 |

→ 成果物不在で無条件に落ちていたのは `shiori-host32-host` のみであり、そこが緑になった時点でヘルパ不在起因の失敗はワークスペースから消えている。

### 9.8 `shiori-host32-helper` 自身のテストのアーキテクチャ依存（要件 2.2 への含意・記録のみ）

`shiori-host32-helper` は本 spec の対象クレート 12 のうちの 1 つ（`main.rs` 595 行・4 モジュール）であるため、その挙動を実測して記録する。

| 実行 | 結果 |
|---|---|
| `cargo test -p shiori-host32-helper`（既定 x64） | exit 0 — **20 passed / 0 failed / 3 ignored** |
| `cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` | exit 0 — **23 passed / 0 failed / 0 ignored** |

x64 で `ignored` になる 3 件は `shiori_proxy::tests::testdll_drop_invokes_courtesy_unload` ／ `shiori_proxy::tests::testdll_request_roundtrip_get_and_notify` ／ `loopback_tests::loopback_hello_request_proxy_driven_and_bounded_loop`。32bit の `shiori.dll` を `LoadLibrary` するため x64 プロセスでは `BAD_EXE_FORMAT` になるという構造上の制約で、実装側が実行時に `ignore` 理由つきでスキップしている（失敗ではない）。本 spec では**是正しない**。

**要件 2.2（前後のテスト総数完全一致）への含意**: この 3 件は「x64 では ignored / i686 では passed」と、**ターゲットによって passed 件数が変わる**。したがって EvidencePipeline の移設前後スナップショットは**同一ターゲットで採取**しなければ総数一致の判定が壊れる。既定（x64）で揃えれば 3 件は前後とも ignored で安定し、比較は成立する。この 3 件は移設対象モジュール（`main.rs` の 4 モジュール・`shiori_proxy.rs`）と重なるため、当該クレートのコミット時に本注意を再確認すること。

## 10. 移設前スナップショットの採取（タスク 1.3・要件 2.2 / 2.3 / 2.6）

design §検証 / EvidencePipeline（証跡パイプライン）の採取手順 1〜3 を、実装開始直前に一度だけ実行した記録。
以後の実装作業（タスク 2 以降）は、この 3 ファイルを基準値として比較される。

- 採取日: 2026-08-07
- ブランチ: `claude/areka-p0-file-slimming-64d065` / HEAD `6289b70`（タスク 1.2 のコミット時点。作業ツリーの変更は verification/ の新規 3 ファイル＋本 notes.md への追記のみ。`before_build_warnings.txt` の記録 sha と一致する）
- 実行シェル: **PowerShell（pwsh）**。Git Bash は使わない（§9.5 の理由——GNU `link.exe` が MSVC `link.exe` を隠す）
- ツールチェーン: `cargo 1.97.1 (c980f4866 2026-06-30)` / `rustc 1.97.1 (8bab26f4f 2026-07-14)` / `stable-x86_64-pc-windows-msvc (default)`

### 10.1 3 本の採取結果

| # | コマンド | exit | 保存先 | 行数 |
|---:|---|---:|---|---:|
| 1 | `cargo test --workspace --no-fail-fast -- --list` | 0 | `verification/before_default.txt` | **4,790** |
| 2 | `cargo test --workspace --exclude areka --all-targets --no-fail-fast -- --list` | 0 | `verification/before_alltargets.txt` | **4,105** |
| 3 | `cargo build --workspace --all-targets` | 0 | `verification/before_build_warnings.txt` | 254（集計値 5 個＋警告原文 23 行＋raw stderr 187 行） |

3 本とも **exit 0**。失敗・未達は無い。

### 10.2 リストファイルの生成手順（移設後も同一手順で再現すること）

1. cargo の **stdout** のみを対象とする（`--list` 出力は stdout・進捗と警告は stderr）。
2. 正規表現 `: test$` に一致する行だけを残す。除外されるのはユニット単位の集計行（`N tests, 0 benchmarks`）と空行のみ。
3. **序数（ordinal）比較で整列**する。`[Array]::Sort($arr, [System.StringComparer]::Ordinal)` を用いる。
   PowerShell の `Sort-Object` 既定はカルチャ依存比較でありアンダースコアと英字の順序が変わるため使わない。
4. UTF-8（BOM 無し）で書き出す。

**重複行は除去しない。** 同一の完全修飾名が複数のテストバイナリに現れることがあり、要件 2.2 が問うのは総数であるため、比較は集合ではなく**多重集合（行数込み）**で行う。実測の重複は以下:

- `before_default.txt`: 4,790 行 / 相異なる名前 4,787（`log_capture::tests::` の 3 本が各 2 回）
- `before_alltargets.txt`: 4,105 行 / 相異なる名前 4,097（上記 3 本＋`ipc::tests::` の 5 本が各 2 回）

集計行の総和（`N tests` の合計）は 1 が 4,790、2 が 4,105 で、抽出行数と一致する。抽出漏れは無い。

### 10.3 3 本立てが実際に相互補完していることの実測

design が 3 本立てを要求する根拠を、採取結果そのもので裏づけた。

| 観測 | 実測値 |
|---|---:|
| (1) にのみ在るテスト行 | 711 |
| (2) にのみ在るテスト行 | 26 |
| (1) の doctest 行（`<path>.rs - <item> (line N): test` 形式） | 37 |
| (2) の doctest 行 | **0** |

- **(2) が doctest を落とす**ことは実測で確認（37 → 0）。`--all-targets` は doctest を含まないという design の前提どおり。
- **(2) にのみ在る 26 行はすべて `crates/pilot/examples/**` のテストモジュール**である。相異なる名前で 21 本。
  裏取り: `crates/pilot/examples/shiori-host-32/process_host.rs:254`（`exit_kind_classification_table`）・
  `crates/pilot/examples/shiori-host-32/shiori_proxy.rs:273`（`ansi_encode_ascii_is_byte_equivalent`）・
  `crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs:921`（`center_is_opaque`）。
  これらは既定の `--list`（1）に **0 件**しか現れない。examples 被覆に (2) が必要であることの直接証跡。
- (1) にのみ在る 711 行は doctest 37 行と、(2) が `--exclude areka` で落とした `crates/areka` のテストの合計。
  検算: 4,790 − 4,105 + 26 = 711。
- (3) は (2) が除外した `crates/areka` の examples を**非 test モード**でビルドして被覆する。既存 E0433 は test モード限定の
  コンパイルエラー（`crates/areka/src/placement/spawn.rs:879`・先行裁定注記 `:871`）であるため、(3) は exit 0 で通る。
  この既存エラーは本 spec の担当ではない（§7.1 の送付所見）。

### 10.4 `#[cfg_attr(not(target_arch = "x86"), ignore)]` の 3 件は `--list` に現れる（要件 2.2 への含意・§9.8 の続き）

§9.8 で「x64 では ignored / i686 では passed」と記録した 3 件について、`--list` 出力への現れ方を実測で確定した。

**結論: 現れる。** `--list` は `ignore` 属性の有無を区別せず、無印で `: test` 行として列挙する。
`before_default.txt` に以下の 3 行が実在する:

```
loopback_tests::loopback_hello_request_proxy_driven_and_bounded_loop: test
shiori_proxy::tests::testdll_drop_invokes_courtesy_unload: test
shiori_proxy::tests::testdll_request_roundtrip_get_and_notify: test
```

さらに、この 3 件が**ターゲットによってリスト membership を変えない**ことも実測した:

| 実行 | `: test` 行数 | 差分 |
|---|---:|---|
| `cargo test -p shiori-host32-helper -- --list`（既定 x64・exit 0） | 23 | — |
| `cargo test -p shiori-host32-helper --target i686-pc-windows-msvc -- --list`（exit 0） | 23 | **差分ゼロ**（`Compare-Object` 出力なし） |

`ignore` は「列挙されるが実行されない」を意味するだけで、テストの**存在**を消さない。したがってこの 3 件に限れば
リスト比較はターゲット差の影響を受けない（§9.8 が懸念した passed 件数の 20 対 23 という差は、`--list` ではなく実行結果側の話である）。

**それでも移設前後の 2 スナップショットは同一ホストターゲット（既定の x64）で採取しなければならない。**
理由は `ignore` ではなく `#[cfg(...)]` である——`cfg` でゲートされたテストはターゲットが変わると
リストから**消える／現れる**ため、ターゲットを跨いだ比較は要件 2.2 の「総数完全一致」判定を無意味にする。
本 spec の全スナップショットは既定ターゲット `x86_64-pc-windows-msvc` で採取する。タスク 7.1 の移設後採取も同一とすること。

### 10.5 警告集計の方法（要件 2.6・タスク 7.2 が同一手順で再現すること）

`before_build_warnings.txt` の `[TALLY METHOD]` セクションに手順を逐語で埋め込んである。要点:

- 対象は **stderr のみ**（cargo の警告は stderr へ出る）。
- 2 種類の行を別々に数える。
  - **SUMMARY 行**: `^warning: ` + 「バッククォート囲みのクレート名」+ `(ユニット) generated N warnings` に一致する行。
  - **DIAG 行**: `^warning: ` に一致し、SUMMARY 行ではないもの（個別警告の見出し行）。
- SUMMARY 行から `generated N warnings` の N を総和して `GENERATED_SUM`、`(N duplicates)` の N を総和して `DUPLICATES` とし、
  `NET = GENERATED_SUM − DUPLICATES` を求める。`NET` と `DIAG_COUNT` の一致が集計の健全性チェックになる。

**移設前の基準値（この 5 数値が比較対象）**:

| 指標 | 値 |
|---|---:|
| `DIAG_COUNT` | **16** |
| `SUMMARY_COUNT` | 7 |
| `GENERATED_SUM` | 22 |
| `DUPLICATES` | 6 |
| `NET` | **16** |

ユニット別内訳（原文はファイル内 `[PER-UNIT TALLY]`）。**7.2 の比較対象は上記 5 数値のみ**であり、
下表の `duplicates` 列の帰属先と行順は比較対象に含めない——同一コミットの再ビルドで duplicates 3 件が
`shiori-host32-testdll` の `(lib)` と `(lib test)` の間を移動し行順も変わることを実測済み（`generated` 列は再現する）:

| ユニット | generated | duplicates |
|---|---:|---:|
| `shiori-host32-helper` (bin, test) | 3 | 0 |
| `shiori4-testdll` (lib) | 1 | 0 |
| `areka-seriko` (lib test) | 4 | 0 |
| `shiori-host32-testdll` (lib test) | 3 | 0 |
| `areka` (bin "areka") | 4 | 0 |
| `shiori-host32-testdll` (lib) | 4 | 3 |
| `shiori-host32-helper` (bin) | 3 | 3 |

内訳の性質（16 件の DIAG の中身）: `"cdecl" is not a supported ABI for the current target` × 6、
`unused std::ops::ControlFlow that must be used` × 4、デッドコード系（`CommandConsumer`・`LedgerError`・`ConsumerLedger`・
`new`/`try_register`/`consumer_of`/`canonical`）× 4、`linker stdout: ...`（MSVC link.exe の import library 生成通知）× 2。

集計上の注意（7.2 で踏むこと）:

- `linker stdout:` の 2 件は本文にワークツリー絶対パスと日本語ロケールの MSVC 出力を含む。**比較は本文一致ではなく件数**で行う。
- cargo はキャッシュ済み（fresh）ユニットの警告も再生する。本採取時点でワークスペースは (1)(2) のビルドで温まっており、
  それでも 7 ユニット分の警告がすべて再生された。フルリビルドでも fresh でも件数は同一になる。
- `[RAW STDERR]` セクション（全 187 行）は証跡であって比較対象ではない。`Compiling` / `Fresh` 行はキャッシュ状態で変動する。

### 10.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `verification/before_default.txt` | 手順 1 のテスト名リスト・4,790 行・序数整列済み |
| `verification/before_alltargets.txt` | 手順 2 のテスト名リスト・4,105 行・序数整列済み |
| `verification/before_build_warnings.txt` | 手順 3 の警告集計・基準値 5 数値＋ユニット別内訳＋警告原文＋raw stderr |
| `verification/notes.md` | 本節（§10）を追記 |

`crates/**`・`Cargo.toml`・spec 本体ドキュメントには一切触れていない。

## 11. 検証ツールの整備（タスク 1.4・要件 1.8 / 2.3 / 2.4 / 2.9）

design §検証 / EvidencePipeline の判定契約を実行可能なスクリプトとして用意し、
既知の一致ケース・不一致ケースの双方で期待どおり判定することを確認した記録。

- 整備日: 2026-08-07
- ブランチ: `claude/areka-p0-file-slimming-64d065` / HEAD `48d0b5d`（タスク 1.3 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**。文字列整列は `[System.StringComparer]::Ordinal` を用いる（§10.2 手順 3 と同一）
- `crates/**`・`Cargo.toml`・spec 本体ドキュメントには一切触れていない

### 11.1 成果物

| ファイル | 役割 |
|---|---|
| `verification/Compare-RelocatedTests.ps1` | 移設前後のテスト本文一致検証（行頭空白非依存・項目単位） |
| `verification/Test-MappingBijection.ps1` | 旧→新テスト名対応表の全単射検証・フラグメント結合 |
| `verification/Compare-TestLists.ps1` | 対応表を旧リストへ適用した結果と新リストの多重集合突合 |
| `verification/test_name_mapping.csv` | 対応表の器（ヘッダ行のみ・データ行はクレート単位タスクが追記） |
| `verification/mapping/` | クレート単位の対応表フラグメント置き場（`README.txt` に規約を逐語で記載） |
| `verification/RustParse.ps1` | 上記が共有する Rust 字句正規化・項目パース（dot-source 用ライブラリ） |
| `verification/Test-VerificationTools.ps1` | 上記 3 本の自己検証ドライバ（一致／不一致ケースの判定マトリクス） |
| `verification/fixtures/` | 既知ケースの入力（`relocate/` 7 本・`lists/` 5 本・`mapping/` 5 本＋フラグメント 2 組） |

### 11.2 後続タスクの呼び出し方（この記載どおりに実行すること）

いずれも終了コードで判定する。**0 = 合格 / 1 = 不合格 / 2 = 引数不正**。
以下の `$V` は `\.kiro/specs/areka-P0-file-slimming/verification` を指す。

**(a) 本文一致検証（タスク 2.x の各ファイル移設直後・要件 2.4）**

```powershell
# 単一ファイルへの移設（テーマ分割なし）
pwsh -File $V/Compare-RelocatedTests.ps1 `
    -Commit <移設前コミット> `
    -OriginalPath crates/areka-seriko/src/resolver.rs `
    -RelocatedPath crates/areka-seriko/src/resolver_tests.rs

# テーマ分割（複数ファイル）— 複数指定は**カンマ区切りの 1 引数**で渡す
pwsh -File $V/Compare-RelocatedTests.ps1 `
    -Commit <移設前コミット> `
    -OriginalPath crates/areka/src/placement/follow.rs `
    -RelocatedPath "crates/areka/src/placement/follow_test_support.rs,crates/areka/src/placement/follow_anchor_tests.rs,crates/areka/src/placement/follow_drag_tests.rs"
```

- `-OriginalPath` は**リポジトリ相対パス**（`/` 区切り）。`-Commit` を与えると `git show <commit>:<path>`
  で読む——移設後の作業ツリーには既にテストモジュールが無いため、実運用では `-Commit` を必ず指定する。
  読み取り専用の git のみを使う（`checkout`／`restore` は行わない）。
- 元ファイルに複数のテストモジュールがある場合は既定で全ブロックを合算する。名前で絞るときは `-ModuleName`。
- 一致すると**何も出力せず** exit 0。件数を目で見たいときだけ `-Detail` を付ける
  （`MATCH: test fn 133=133 / helper item 57=57 / mod block 1 / files 3` の形式）。
- `-Commit` を省略すると作業ツリーの元ファイルを読む（フィクスチャ検証用の経路）。

**(b) 対応表の全単射検証（テーマ分割を行ったクレートごと・要件 2.9）**

```powershell
pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/<crate>.csv
```

**(c) 最終照合（タスク 7.1）— フラグメント結合 → 対応表検証 → リスト突合**

```powershell
# 1. 全フラグメントを結合して単一の対応表を書き出す（検証 PASS のときだけ書き出される）
pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping -Out $V/test_name_mapping.csv

# 2. 既定リスト（doctest 込み）
pwsh -File $V/Compare-TestLists.ps1 `
    -Before $V/before_default.txt -After $V/after_default.txt -Mapping $V/test_name_mapping.csv

# 3. examples 込みリスト
pwsh -File $V/Compare-TestLists.ps1 `
    -Before $V/before_alltargets.txt -After $V/after_alltargets.txt -Mapping $V/test_name_mapping.csv
```

- `after_*.txt` は §10.2 の手順（`: test$` 抽出・序数整列・UTF-8 BOM 無し・**重複行を残す**）で採取すること。
  採取ターゲットは移設前と同じ既定の `x86_64-pc-windows-msvc`（§10.4 の理由）。
- `Compare-TestLists.ps1` は行数一致（要件 2.2）と対称差の双方を報告する。両方が満たされたときのみ `RESULT: PASS`。
- 対応表の行が `-Before` に一度も現れない場合は `UNUSED MAPPING ROWS` として報告する。既定では不合格にしない
  ——examples のテストは `_alltargets` 側にしか現れず、`_default` 側の照合では未使用になるのが正常であるため。
  2 本のリストを跨いでも未使用の行が残る場合は対応表の誤りなので、その行を修正する（`-StrictUnusedMappings` で不合格化できる）。

**(d) 検証ツール自体の自己検証（ツールに手を入れたら必ず再実行）**

```powershell
pwsh -File $V/Test-VerificationTools.ps1            # 判定マトリクスのみ
pwsh -File $V/Test-VerificationTools.ps1 -ShowOutput # 各ケースの出力も表示
```

### 11.3 対応表フラグメントの規約

- 置き場: `verification/mapping/<crate>.csv`（Cargo のパッケージ名そのまま・1 クレート 1 ファイル・ディレクトリを掘らない）
- 列: `old_fqn,new_fqn,reason`（結合先と同一・同一順序）。`reason` は `theme_split` のみ
- テーマ分割でモジュールパスが変わったテスト関数のみ 1 行を持つ。完全修飾名が変わらない移設は行を持たない
- 行の並びは問わない（検証は集合として行う）。結合時に `old_fqn` の序数順へ整列して書き出す
- 逐語の規約は `verification/mapping/README.txt`

### 11.4 本文一致検証が吸収する差分／吸収しない差分

要件 2.4 が許容する機械的調整だけを吸収する。それ以外は 1 文字でも不一致として報告する。

| 差分 | 扱い | 根拠 |
|---|---|---|
| 行頭の空白（一律 4 スペース de-indent を含む） | 吸収（各行を lstrip） | 2.4 |
| 行末の空白 | 吸収（`git diff -w` 相当） | 2.4 |
| 行頭の可視性修飾（`pub` / `pub(crate)` / `pub(super)` / `pub(in …)`）の付与 | 吸収 | 2.4 |
| `use` 項目の追加・変更・分散 | 吸収（`use` 項目を突合対象から外す） | 2.4 |
| モジュール接続宣言（`mod X;`）の追加 | 吸収（突合対象から外す） | 2.4 |
| 入れ子 `mod X { … }` がファイル root へ持ち上がること | 吸収（入れ子を平坦化して比較） | 2.4 |
| 項目の順序変更・複数ファイルへの分散 | 吸収（テスト関数は識別子キーで対応付け・非テスト項目は本文の多重集合で対応付け） | 1.8 / 2.9・design §EvidencePipeline |
| アサーション・入力値・期待値の変更 | **不一致**（`[TEST-BODY]`・差分行を提示） | 2.4 |
| テスト関数の欠落・追加 | **不一致**（`[TEST-MISSING]` / `[TEST-EXTRA]`） | 2.2 / 2.4 |
| テスト関数の改名 | **不一致**（欠落＋追加として現れる） | 2.9 |
| 属性・コメントの変更 | **不一致**（項目本文の一部として比較される） | 2.4 |
| 非テスト項目（ヘルパ・定数・型・`impl`）の欠落・変更 | **不一致**（`[ITEM-MISSING]` / `[ITEM-EXTRA]`） | 2.4 |

補足: 移設先ファイルの**行数**は判定に用いない。空行の増減と項目の再配置は 2.4 の許容範囲であり、
判定はあくまで項目本文の一致に置く。1 ファイル 1,000 行の目安（1.7）は別の指標であって本スクリプトの対象外。

#### 検出できない差分（盲点・タスク 1.4 のレビューで実測確定）

以下の 2 つは本スクリプトが **MATCH と誤判定する**。移設作業者はこの 2 点を手で守ること。

| 盲点 | なぜ検出できないか | 代償措置 |
|---|---|---|
| **複数行文字列リテラルの内部**の行頭空白が変わる | 正規化が各行を無条件に lstrip するため、文字列の**中身**である行頭空白も消える。テストの入力値が変わっているのに一致と判定される | **代償措置なし**（完全修飾名が変わらないのでリスト照合でも捕まらない）。下記の唯一の該当箇所を手で守る |
| 平坦化される入れ子 `mod X { … }` に付いた**属性**（例 `#[cfg(target_arch = "…")]`） | 平坦化時に `mod` 項目自身の属性行を記録しないため、テストがコンパイルから外れても一致と判定される | `cargo test -- --list` からテストが消えるので **`Compare-TestLists.ps1` が捕捉する**（実測確認済み） |

**必須対象 49 本を全走査した結果、第 1 の盲点に該当する行はリポジトリ全域で 1 箇所のみ**:

- `crates/wintf/src/ecs/window_proc/window_pos.rs:1137` — `\` 継続でない行が続く複数行文字列で、17 個の行頭空白が**リテラルの中身**になっている。
  当該ファイルを移設するタスク（2.1・ウィンドウ基盤クレート）は、この行の行頭空白を 1 文字も増減させずに移すこと。
  de-indent はテストモジュールブロック**全体**に一律 4 スペース適用されるが、この行だけは対象外として扱う
  （4 スペース削ると文字列の中身が変わる＝要件 2.4 違反になる）。移設後に当該行を目視で突合し、結果を登記すること。
- 他の候補 13 行はすべて `\` 継続（Rust が行頭空白を除去する）か単一行 raw 文字列であり、影響しない。

### 11.5 既知の一致ケース・不一致ケースでの判定確認（完了状態の証跡）

`Test-VerificationTools.ps1` の 24 ケースが全て期待どおり（`RESULT: PASS  (24/24 ケースが期待どおり)`）。

| ケース | 対象 | 期待 | 実 | 判定 |
|---|---|---:|---:|---|
| `RELOC-OK-DEINDENT` | 純粋な 4 スペース de-indent のみ | 0 | 0 | OK |
| `RELOC-BAD-ASSERT` | 期待値を 1 文字変更（`, 7)` → `, 8)`） | 1 | 1 | OK |
| `RELOC-BAD-DROPPED` | テスト関数 1 本を丸ごと欠落 | 1 | 1 | OK |
| `RELOC-BAD-RENAMED` | テスト関数 1 本を改名 | 1 | 1 | OK |
| `RELOC-OK-THEMESPLIT` | 3 ファイルへ分散＋順序入れ替え＋`pub(super)` 付与 | 0 | 0 | OK |
| `MAP-OK` | 正しい対応表 1 行 | 0 | 0 | OK |
| `MAP-DUP-OLD` | `old_fqn` の重複 | 1 | 1 | OK |
| `MAP-DUP-NEW` | `new_fqn` の重複 | 1 | 1 | OK |
| `MAP-BAD-IDENT` | 末尾セグメント（関数識別子）が旧新で相違 | 1 | 1 | OK |
| `MAP-BAD-REASON` | `reason` が `theme_split` 以外 | 1 | 1 | OK |
| `MAP-FRAG-OK` | フラグメント 2 本の結合（衝突なし） | 0 | 0 | OK |
| `MAP-FRAG-COLLIDE` | フラグメント間でキー衝突 | 1 | 1 | OK |
| `MAP-EMPTY-CONTAINER` | ヘッダのみの `test_name_mapping.csv` | 0 | 0 | OK |
| `MAP-FRAGDIR-EMPTY` | フラグメント 0 件の `mapping/` | 0 | 0 | OK |
| `MAP-MERGE-OUT` | `-Out` による結合書き出し | 0 | 0 | OK |
| `MAP-MERGE-ROUNDTRIP` | 結合結果を単体で再検証 | 0 | 0 | OK |
| `LIST-OK-IDENTICAL` | 同一リスト（重複行込み） | 0 | 0 | OK |
| `LIST-BAD-MISSING` | テスト 1 本の欠落 | 1 | 1 | OK |
| `LIST-BAD-RENAMED-NOMAP` | 対応表なしで名前が変わった | 1 | 1 | OK |
| `LIST-OK-RENAMED-WITHMAP` | 同じ変更＋正しい対応表 1 行 | 0 | 0 | OK |
| `LIST-BAD-DUPCOLLAPSE` | 相異なる名前は同じで重複回数だけ減少 | 1 | 1 | OK |
| `LIST-SMOKE-REALDATA-DEFAULT` | 実データ `before_default.txt` を両側に | 0 | 0 | OK |
| `LIST-SMOKE-REALDATA-ALLTARGETS` | 実データ `before_alltargets.txt` を両側に | 0 | 0 | OK |
| `TOKENIZER-EQUIV` | 字句解析の持ち上げ等価性（§11.6） | 0 | 0 | OK |

実データスモークの出力（フィクスチャではなく §10 の移設前スナップショットそのもの）:

```
BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
AFTER       : before_default.txt  (4790 行 / 相異なる 4787)
MAPPING     : 0 行 (1 ファイル) / 適用 0 行 / 未使用 0 行
LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
RESULT: PASS

BEFORE      : before_alltargets.txt  (4105 行 / 相異なる 4097)
AFTER       : before_alltargets.txt  (4105 行 / 相異なる 4097)
MAPPING     : 0 行 (1 ファイル) / 適用 0 行 / 未使用 0 行
LINE COUNT  : before 4105 / after 4105 -> 一致 (Requirement 2.2)
RESULT: PASS
```

重複行の扱いが §10.2 の実測（4,790 行 / 相異なる 4,787）と一致しており、多重集合として比較していることが確かめられる。

### 11.6 字句解析の持ち上げの等価性（`Measure-TestModules.ps1` との突合）

`RustParse.ps1` は `Measure-TestModules.ps1`（タスク 1.1）の字句正規化ルーチンを逐語で持ち上げたものである。
「2 本目の弱い解析器」を作っていないことを、タスク 1.1 の実測結果そのもので裏づけた。

- 方法: `scan_raw.csv` の各行について実ファイルを読み、`RustParse.ps1` が求めたテストモジュールの
  行範囲（`名前:開始-終了(行数)` の連結）が `modules` 列と**文字列として完全一致**することを要求する。
- 結果: **走査 619 ファイル / テストモジュールを持つ 219 ファイル / 不一致 0**。

> **このケースが有効なのは移設前のツリーに限る。** `TOKENIZER-EQUIV` は「現在の作業ツリー」を
> `scan_raw.csv`（移設前の実測）と突合するため、タスク 2.1 以降でテストモジュールを本番ファイルの外へ
> 出した瞬間から**必ず赤になる**——これは欠陥ではなく仕様である（本番ファイルにブロックが無くなる一方、
> CSV は `tests:X-Y(N)` を記載したままになる）。
> 移設開始後に `Test-VerificationTools.ps1` を再実行する場合は、`TOKENIZER-EQUIV` の失敗は
> **想定どおりとして扱い、残る 23 ケースが全て期待どおりであることをもって合格**とすること。
> 字句解析の等価性そのものを再確認したいときは、移設前コミット（`48d0b5d`）のツリーに対して実行する。

### 11.7 実データでの動作確認（フィクスチャ以外の証跡）

移設前コミット（この時点の `HEAD` = `48d0b5d`＝タスク 1.3 のコミット）から `git show` で読み出し、`#[cfg(test)] mod` ブロックの本体を
4 スペース de-indent して書き出したものと突合した。実運用と同一の読み出し経路である。

| 対象 | モジュール数 | テストコード行 | 結果 |
|---|---:|---:|---|
| `crates/areka/src/placement/follow.rs` | 1 | 6,476 | `MATCH: test fn 133=133 / helper item 57=57 / mod block 1 / files 1` |
| `crates/areka/src/emo2_boot/frame.rs` | 1 | 3,163 | `MATCH: test fn 56=56 / helper item 56=56 / mod block 1 / files 1` |
| `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` | 10 | 2,091 | `MATCH: test fn 15=15 / helper item 40=40 / mod block 10 / files 10` |

不一致側も実データで確認した。

- **1 文字改変**: `follow.rs` の切り出し本文で `assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });`
  の数値を変えたところ、`[TEST-BODY] テスト関数 move_window_to_updates_window_pos_physical_px の本文が一致しません`
  と該当行の before / after を提示して exit 1。
- **テーマ分割の模擬**: `follow.rs` の 223 項目を 3 ファイルへ巡回配分し（順序が完全に入れ替わる）、
  さらに全行を 8 スペースで再インデントしたうえで突合したところ
  `MATCH: test fn 133=133 / helper item 57=57 / mod block 1 / files 3` で exit 0。
  項目単位の対応付けが順序とインデントの双方に依存しないことの直接証跡。

### 11.8 運用上の注意

- `pwsh -File` は引数を常に単一文字列として渡すため、複数ファイルの指定は**カンマ区切りの 1 引数**にする
  （`-RelocatedPath "a.rs,b.rs,c.rs"`）。各スクリプトが内部でカンマ展開する。
- 文字列の整列は必ず `[System.StringComparer]::Ordinal`。`Sort-Object` の既定はカルチャ依存で
  アンダースコアと英字の順序が変わる（§10.2 手順 3 と同じ理由）。
- `Compare-RelocatedTests.ps1` は git を**読み取りにしか使わない**（`show` のみ）。作業ツリーを書き換える
  git コマンドは実行しない。
- 判定は必ず終了コードで行う。合格時に何も出力しないのが `Compare-RelocatedTests.ps1` の既定であり、
  「出力が無い＝実行されていない」ではない。
- **終了コード 2（引数不正）は 1（不一致）と必ず区別すること。** パスの打ち間違いで 2 が返っているのを
  「本文が一致しなかった」と読み違えると、直す必要のないテストを直しにいってしまう。
  タスク 1.4 のレビュー時点では 3 スクリプトとも `$ErrorActionPreference = 'Stop'` により
  `Write-Error` が即時終了して exit 1 になっていた欠陥があり、`-ErrorAction Continue` を付けて是正済み
  （`Compare-TestLists.ps1` 4 箇所・`Compare-RelocatedTests.ps1` 4 箇所・`Test-MappingBijection.ps1` 1 箇所）。
- **計測スクリプトの走査範囲から `.kiro/` を除外済み。** `verification/fixtures/relocate/*.rs` は本物の
  `#[cfg(test)] mod` ブロックを含むため、除外しないとリポジトリ全域の実測へ混入する
  （実測: 619 → 627 ファイル・テストコード 99,012 → 99,043 行）。`Measure-TestModules.ps1` の
  `$excludeRe` に `^\.kiro/` を追加してあり、除外後は `scan_raw.csv`・`target_inventory.csv`・
  `excluded_inventory.csv` の 3 本がタスク 1.1 のコミット版とバイト一致で再現することを確認済み。
  タスク 7.4 の実測やり直しは必ずこの修正後のスクリプトで行うこと。

### 11.9 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `verification/Compare-RelocatedTests.ps1` | 本文一致検証スクリプト |
| `verification/Test-MappingBijection.ps1` | 全単射検証・フラグメント結合スクリプト |
| `verification/Compare-TestLists.ps1` | リスト照合スクリプト |
| `verification/RustParse.ps1` | 共有の字句正規化・項目パース |
| `verification/Test-VerificationTools.ps1` | 自己検証ドライバ（24 ケース） |
| `verification/test_name_mapping.csv` | 対応表の器（ヘッダ行のみ） |
| `verification/mapping/README.txt` | フラグメント規約 |
| `verification/fixtures/**` | 既知の一致ケース・不一致ケースの入力 |
| `verification/notes.md` | 本節（§11）を追記 |

## 12. 3 階層モジュール解決の事前スモーク（タスク 1.5・要件 2.7 / 4.1 / 4.3）

### 12.1 何を測ったか

`design.md:532`（Migration Strategy 順 0「3 階層解決の事前スモーク」）が要求する測定。
`research.md:161-175`（§2.3.3）の実測は「`#[path]` 読込ファイル → その子」の **2 階層**までであり、
`follow.rs` の本体分割（タスク 6.1・D1 ファサード再輸出型）が必要とする **3 階層**は
言語意味論からの推論にとどまっていた。本節はこれを実測へ置き換える。

対象の取り込み鎖（file:line で確認済み）:

```
crates/areka/examples/window-placement.rs:107   #[path = "../src/placement/mod.rs"] mod placement;   ← 1 階層目
crates/areka/examples/collision-probe.rs:231    #[path = "../src/placement/mod.rs"] mod placement;   ← 同上
crates/areka/src/placement/mod.rs:24            pub mod follow;                                      ← 2 階層目（§2.3.3 で実測済）
crates/areka/src/placement/follow.rs            mod <サブモジュール>;                                ← 3 階層目（本節で実測）
```

事前状態: `crates/areka/src/placement/follow.rs` は 8,472 行のフラットファイルで、
`crates/areka/src/placement/follow/` ディレクトリは存在しない（`ls crates/areka/src/placement/` で確認）。

### 12.2 仮置きした内容（実施後に全て撤去済）

| ファイル | 内容 |
|---|---|
| `crates/areka/src/placement/follow/smoke3.rs`（新規） | `pub(crate) const SMOKE3_LEVELS: u32 = 3;` のみ。`crate::` パスを 1 件も持たない（design §本体分割の制約 1） |
| `crates/areka/src/placement/follow.rs`（追記 7 行・`use` ブロック直後 L52 の後ろ） | `mod smoke3;` と `#[allow(dead_code)] pub(crate) const SMOKE3_PROBE: u32 = smoke3::SMOKE3_LEVELS;` |

到達性の証明形として `pub(crate) const` の初期化子からサブモジュールの項目を参照する形を採った。
理由: (i) `mod` 宣言だけでは「宣言はしたが経路が通っているか」を分離できず、
モジュールパス `smoke3::SMOKE3_LEVELS` の解決まで含めて初めて 3 階層が通ったと言えるため。
(ii) `pub(crate) use` の再輸出形は未使用時に `unused_imports` 警告を生み、
要件 2.6（警告非増加）の測定を濁す。`#[allow(dead_code)]` を項目に直付けする形なら
警告を 1 件も増やさずに到達性だけを測れる（実測でも警告増加 0 件・下記 12.4）。
(iii) 既存関数の中へ参照を差し込む形は本番コードへの侵襲が大きく、撤去の完全性が担保しにくい。

### 12.3 否定対照（どこを探しているかを確定させた測定）

3 階層が「通った」ことより、**どのディレクトリを探しているか**が本質。
`#[path]` で読み込まれたモジュールの子は、そのファイル自身のディレクトリへ解決される
（§2.3.3 の 1 行目＝`tests/dom/a.rs` の `mod sub;` は `tests/dom/sub.rs` を探し `tests/dom/a/sub.rs` は探さない）。
この規則が `follow.rs` にも及ぶなら、3 階層目は `src/placement/follow/` ではなく
**`src/placement/`（兄弟位置）**へ解決されてしまい、D1 ファサード形は成立しない。
そこで肯定側の緑を採る前に、2 本の否定対照でこの分岐を潰した。

**否定対照 1（ファイルをどこにも置かない）** — `mod smoke3;` のみを宣言して example ビルド:

```
cargo build -p areka --examples     → exit 101
error[E0583]: file not found for module `smoke3`
  --> crates\areka\examples\..\src\placement\follow.rs:53:1
   = help: to create the module `smoke3`, create file
           "crates\areka\examples\..\src\placement\follow\smoke3.rs" or
           "crates\areka\examples\..\src\placement\follow\smoke3\mod.rs"
```

コンパイラが探索した候補が `...\src\placement\follow\smoke3.rs` であると
エラーメッセージ自身が名指ししている。`...\src\placement\smoke3.rs` は候補に挙がっていない。

**否定対照 2（兄弟位置へ置く）** — `crates/areka/src/placement/smoke3.rs` を作成して再ビルド:

```
cargo build -p areka --examples     → exit 101
error[E0583]: file not found for module `smoke3`
   = help: to create the module `smoke3`, create file
           "crates\areka\examples\..\src\placement\follow\smoke3.rs" or ...
```

兄弟位置のファイルは拾われない＝`#[path]` 直読みモジュール（`placement/mod.rs`）に適用される
「親ディレクトリへ解決」の特例は、その先の通常 `mod` 宣言（`follow.rs`）へは伝播しない。
`follow.rs` の子は通常規則どおり **file-stem ディレクトリ `follow/`** へ解決される。
これが D1 ファサード形の成立条件そのものである。

### 12.4 計測（BEFORE / WITH-SMOKE / AFTER-REMOVAL）

すべて PowerShell、出力はファイルへリダイレクトして `$LASTEXITCODE` を別途取得（`tee` 経由にしない）。
警告件数は出力中の行頭 `warning` 行数（サマリ行 1 本を含む）。

| 相 | コマンド | exit | 警告行 |
|---|---|---:|---:|
| BEFORE | `cargo build -p areka --examples` | 0 | 0 |
| 否定対照 1（ファイル無し） | `cargo build -p areka --examples` | **101**（E0583） | — |
| 否定対照 2（兄弟位置） | `cargo build -p areka --examples` | **101**（E0583） | — |
| WITH-SMOKE | `cargo build -p areka --examples` | **0** | **0** |
| WITH-SMOKE | `cargo build -p areka` | **0** | 5（内訳は既存 dead_code 4 件＋サマリ 1 行・`smoke3` の出現 0 件） |
| AFTER-REMOVAL | `cargo build -p areka --examples` | 0 | 0 |
| AFTER-REMOVAL | `cargo build -p areka` | 0 | 5（WITH-SMOKE と同一） |

AFTER-REMOVAL は BEFORE と exit・警告件数ともに完全一致。
`cargo build -p areka` の既存 4 警告（`CommandConsumer` / `LedgerError` / `ConsumerLedger` /
その associated items が never used）は仮置きと無関係な既存分であり、
仮置き中の出力に `smoke3` を含む警告は 1 件も無い（要件 2.6 の観点で増分 0）。

なお本節は `cargo build`（非テストモード）で測っている。
`cargo test -p areka --examples`（テストモード）は `crates/areka/src/placement/spawn.rs:879` の
既存 E0433 で赤であり、本節の測定対象ではない（当該欠陥には触れていない）。

### 12.5 撤去の完全性

仮置きの撤去は Edit による原文復元とファイル削除のみで行った（`git checkout`／`restore`／`reset`／
`stash`／`clean` は不使用）。撤去後の確認:

```
git status --porcelain -uall -- crates   → 出力なし（空）
git diff --stat -- crates                → 出力なし（空）
git status --porcelain -uall             → 出力なし（空・リポジトリ全域）
Test-Path crates\areka\src\placement\follow   → False
```

`crates/**` に差分は 1 件も残っていない。

### 12.6 結論

**3 階層解決は成立する。**

`example（#[path] include）→ src/placement/mod.rs → follow.rs → follow/<サブモジュール>.rs` の
3 階層は `cargo build -p areka --examples` で緑（exit 0・警告増加 0）。
かつ否定対照 2 本により、3 階層目の解決先が `follow.rs` 自身の file-stem ディレクトリ
`src/placement/follow/` であることが確定した（兄弟位置ではない）。

したがって:

- タスク 6.1 の `follow.rs` 本体分割は **D1 ファサード再輸出型のまま進めてよい**（要件 4.1 / 4.3）。
  設計 §本体分割の 5 サブモジュール（`anchor` / `drag_follow` / `window_move` / `work_area` / `visibility`）は
  この形で example ビルドを壊さない。
- design §本体分割の制約 5（examples の追随＝ファサード維持により編集不要）は前提が実測で裏づけられた。
  ただし制約 1（`crate::` パス不使用）は新サブモジュール側でも必ず守ること——
  example は placement 木を私有 include するため、`crate::` を 1 件でも書けば example ビルドが即赤になる。
- 実行時挙動（要件 2.7）への影響は無い。本節は最終状態で本番コードの差分 0 件である。

差し戻し（ファサード形の裁定へ戻す）は**不要**。

## 13. クレート単位のテスト分離: `wintf`（タスク 2.1・要件 1.1 / 1.6 / 2.4 / 2.8 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `264bac2`
- 実行シェル: **PowerShell（pwsh 7）**
- `crates/wintf` の対象 4 ファイル＋新規テストファイル 4 本以外には一切触れていない（`Cargo.toml`・他クレート・spec 本体ドキュメントは無変更）

### 13.1 移設した 4 ファイル（design §File Structure Plan の `crates/wintf` と完全一致）

| 本番ファイル | 移設前 総行 | テストモジュール | 新テストファイル | 新ファイル行 | 本番 残行 |
|---|---:|---|---|---:|---:|
| `crates/wintf/src/ecs/window_proc/window_pos.rs` | 1,160 | `tests`（445-1160） | `window_pos_tests.rs` | 713 | 447 |
| `crates/wintf/src/ecs/clickthrough/controller.rs` | 1,092 | `tests`（456-1092） | `controller_tests.rs` | 634 | 458 |
| `crates/wintf/src/ecs/window_proc/dpi_helpers.rs` | 746 | `tests`（149-746） | `dpi_helpers_tests.rs` | 595 | 151 |
| `crates/wintf/src/ecs/layout/systems/monitor_systems.rs` | 1,050 | `tests`（482-1050） | `monitor_systems_tests.rs` | 566 | 484 |

- 4 ファイルとも**テストモジュールは 1 個・ファイル末尾に連続配置**（design の実測どおり・ズレ 0）。モジュール名はいずれも `tests`。
- 4 ファイルとも新テストファイルが **1,000 行以下**のため、テーマ分割は不要（要件 1.7）。
  完全修飾名は 1 件も変わらないため、**`mapping/wintf.csv` は作成しない**（対応表の行 0 件・§11.3 の規約どおり）。
- 4 ファイルとも `#[cfg(test)]` は**テストモジュールの 1 箇所のみ**。非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は
  この 4 ファイルには 1 件も存在しない（wintf の該当 4 件は `ecs/widget/bitmap_source/systems.rs:49`・`task_pool.rs:91,97`・
  `runtime/window_registry.rs:99` で、いずれも本タスクの対象外ファイル）。
- **可視性・`use`・モジュール接続の調整は 1 件も必要なかった**（要件 2.8 の発動なし）。`use super::*;` を含む既存 import が
  そのまま有効であり、本番ファイルの差分は「テストモジュールブロックの削除」＋「接続宣言 2 行の追加」のみ。

接続宣言（4 ファイルとも同一文言・design §移設方式の裁定 案 C）:

```rust
#[cfg(test)]
#[path = "<stem>_tests.rs"]
mod tests;
```

### 13.2 §11.4 の盲点（複数行文字列リテラル内の行頭空白）への対処と目視突合

`crates/wintf/src/ecs/window_proc/window_pos.rs:1137`（移設前）＝リポジトリ全域で唯一の該当行。
一律 4 スペース de-indent の**対象外**として扱い、行頭空白 17 個を 1 文字も増減させずに移した。

- 直前行 `:1136` は `\` 継続**ではない**行であるため、`:1137` の行頭 17 空白は文字列リテラルの中身である
  （一方 `:1135` は `\` 継続なので `:1136` の行頭空白は Rust が除去する＝de-indent してよい）。

移設前（`git show 264bac2:crates/wintf/src/ecs/window_proc/window_pos.rs`）:

```
1135 [lead=16] |                "dpi={dpi}: 書かない判定でハンドラが未処理（None）を返している\|
1136 [lead=17] |                 ——`DefWindowProcW` が提案矩形を適用し、その中の `SetWindowPos` が|
1137 [lead=17] |                 同期的に窓を動かすため源断ちが無効化する: {outcome:?}"|
```

移設後（`crates/wintf/src/ecs/window_proc/window_pos_tests.rs`）:

```
 689 [lead=12] |            "dpi={dpi}: 書かない判定でハンドラが未処理（None）を返している\|
 690 [lead=13] |             ——`DefWindowProcW` が提案矩形を適用し、その中の `SetWindowPos` が|
 691 [lead=17] |                 同期的に窓を動かすため源断ちが無効化する: {outcome:?}"|
```

**目視突合の結果: 合格。** `:1135`→`:689` と `:1136`→`:690` は −4、`:1137`→`:691` は **±0**（17 のまま）。
文字列リテラルの値はバイト等価であり、要件 2.4 違反は発生していない。
なお結果として `:690`（13）と `:691`（17）の見た目のインデントは揃わないが、これは意図した正しい状態である
（揃えると文字列の中身が変わる）。**この行を今後さわる者は同じ理由で行頭空白を保存すること。**

### 13.3 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 264bac2 -OriginalPath <本番> -RelocatedPath <新テスト> -Detail`

| 対象 | 出力 | exit |
|---|---|---:|
| `window_pos.rs` | `MATCH: test fn 18=18 / helper item 16=16 / mod block 1 / files 1` | 0 |
| `controller.rs` | `MATCH: test fn 22=22 / helper item 12=12 / mod block 1 / files 1` | 0 |
| `dpi_helpers.rs` | `MATCH: test fn 22=22 / helper item 8=8 / mod block 1 / files 1` | 0 |
| `monitor_systems.rs` | `MATCH: test fn 13=13 / helper item 11=11 / mod block 1 / files 1` | 0 |

**(b) テスト名リストの不変（要件 2.2 / 2.9・完了状態）** — `cargo test -p wintf --no-fail-fast -- --list` を移設前後で採取し、
`: test$` 抽出・`[StringComparer]::Ordinal` 整列（§10.2 と同一手順）:

```
before = 1088 行 / after = 1088 行
Compare-Object → 差分なし
SHA256 = 634A79061DE29A7CF15ED36BB6A9A50A864AF4767F731FD97C8077ED90CAD335（before / after 一致）
```

本タスクは完全修飾名が 1 件も変わらないため、対応表を介さない**素のバイト一致**が成立している。

**(c) クレート緑（要件 7.2）** — `cargo test -p wintf` → **exit 0**。
11 ターゲット合計 **1,062 passed / 0 failed / 26 ignored**（= `--list` の 1,088 と整合）。

**(d) 本番本体の無変更** — 移設前コミットの各本番ファイル 1 行目〜（旧 `#[cfg(test)]` 行の直前）までを
現作業ツリーと逐行突合し、**不一致 0**（window_pos 1-444 / controller 1-455 / dpi_helpers 1-148 / monitor_systems 1-481）。
`git diff --stat -- crates/wintf` = `4 files changed, 8 insertions(+), 2516 deletions(-)`
（挿入 8 = 4 ファイル × `#[path = …]` + `mod tests;` の 2 行。`#[cfg(test)]` 行は元位置のまま据え置きのため差分に現れない）。

**(e) 完了状態の直接確認** — 4 本番ファイルに残る `cfg(test)` / `mod tests` の出現はすべて接続宣言のみ:
`window_pos.rs:445,447`・`controller.rs:456,458`・`dpi_helpers.rs:149,151`・`monitor_systems.rs:482,484`。
テストモジュール本体は 1 行も残っていない。

**(f) 警告（要件 2.6 の予備確認）** — `cargo build -p wintf --all-targets` → exit 0・`warning` 行 **0 件**（増加なし）。

**(g) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 8 パスのみ:
変更 4 本（上表の本番ファイル）＋未追跡 4 本（新テストファイル）。`crates/**` の他ファイル・`Cargo.toml` への差分は 0 件。

### 13.4 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（`crates/wintf` の対象 4 ファイル・テストコード 2,508 行）では、修正を要する壊れたテスト・
不正なテスト・テスト間の状態汚染は 1 件も発見しなかった。** 送付所見は 0 件である。

以下は「調べたが問題なし」と確定した 2 点の記録（次に触る者が同じ調査を繰り返さないための控え。所有 spec への送付は不要）:

| 観測 | file:line（移設後） | 判定 |
|---|---|---|
| 本番の thread_local ドラッグ状態（`crate::ecs::drag`）をテストから書き換える唯一の箇所 | `crates/wintf/src/ecs/clickthrough/controller_tests.rs:352-395` | **問題なし**。`std::panic::catch_unwind` で本体を包み、`:390` の `reset_to_idle()` を panic 経路でも必ず通してから `resume_unwind` する。汚染は残らない |
| `capture_under_filter`（スレッドローカル dispatcher 差し替え）による多スレッド実行器の取りこぼし（既知の盲点） | `crates/wintf/src/ecs/layout/systems/monitor_systems_tests.rs:194-202` | **問題なし**。`run_apply` が `ExecutorKind::SingleThreaded` を明示しており、盲点は既に塞がれている（`:194-196` に理由がコメントで明記済み） |

その他、`controller_tests.rs:177-215` の `create_test_hwnd` は実 Win32 窓を生成・破棄する（各テストが自前で `destroy_test_hwnd`）。
本 spec 着手前から存在する構造であり、移設で変わっていない。

### 13.5 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/wintf/src/ecs/window_proc/window_pos_tests.rs` | 新規（713 行） |
| `crates/wintf/src/ecs/clickthrough/controller_tests.rs` | 新規（634 行） |
| `crates/wintf/src/ecs/window_proc/dpi_helpers_tests.rs` | 新規（595 行） |
| `crates/wintf/src/ecs/layout/systems/monitor_systems_tests.rs` | 新規（566 行） |
| 上記に対応する本番 4 ファイル | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体は無変更） |
| `verification/notes.md` | 本節（§13）を追記 |
| `verification/mapping/wintf.csv` | **作成しない**（FQN 変化 0 件・対応表の行を持たない） |

## 14. クレート単位のテスト分離: `areka-sylphya` / `dola` / `shiori-host32-helper`（タスク 2.2・要件 1.1 / 1.3 / 1.6 / 2.4 / 2.8 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `bfbdfb5`
- 実行シェル: **PowerShell（pwsh 7）**
- 下表の本番 3 ファイル＋新規テストファイル 8 本と本ファイル以外には一切触れていない（`Cargo.toml`・他クレート・spec 本体ドキュメントは無変更）

### 14.1 移設した 3 クレート・3 ファイル（design §File Structure Plan と完全一致）

| クレート | 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 新テストファイル | 新ファイル行 | 本番 残行 |
|---|---|---:|---|---|---:|---:|
| `areka-sylphya` | `src/actor.rs` | 1,587 | `tests`（675-1028）／`actor_integration_tests`（1036-1379）／`actor_criteria_cage`（1420-1587） | `actor_tests.rs`／`actor_actor_integration_tests.rs`／`actor_actor_criteria_cage.rs` | 351 / 341 / 165 | 730 |
| `dola` | `src/cue/command.rs` | 1,089 | `tests`（332-1089） | `cue/command_tests.rs` | 755 | 334 |
| `shiori-host32-helper` | `src/main.rs` | 1,114 | `resolve_param_tests`（517-568）／`classify_tests`（570-660）／`load_ack_tests`（662-689）／`loopback_tests`（691-1114） | `main_resolve_param_tests.rs`／`main_classify_tests.rs`／`main_load_ack_tests.rs`／`main_loopback_tests.rs` | 49 / 88 / 25 / 421 | 531 |

- 3 ファイルとも**テストモジュールはファイル末尾に連続配置**（design の実測どおり・ズレ 0）。本体の途中に挿入されたテストモジュールは 0 件。
- 新テストファイル 8 本はすべて **1,000 行以下**（最大 `cue/command_tests.rs` の 755 行）のため、テーマ分割は 1 件も行わない（要件 1.7）。
- **完全修飾名は 8 モジュールすべてで 1 件も変わらない**（1 テストモジュール＝1 テストファイル・設計判断 #2）。したがって
  `mapping/areka-sylphya.csv`・`mapping/dola.csv`・`mapping/shiori-host32-helper.csv` は**いずれも作成しない**（対応表の行 0 件・§11.3 の規約どおり）。
- `shiori-host32-helper/src/main.rs` は**バイナリ入口**であるため、stem は読み替え規則により **`main`**（design §移設方式の裁定・`main.rs` / `lib.rs` は読み替えなし）。よってテストファイル名は `main_<モジュール名>.rs`。
- `dola/src/cue/command.rs` はサブディレクトリモジュール配下だが、接続規約どおりテストファイルは**同一ディレクトリ** `crates/dola/src/cue/` に置く。
- **可視性・`use`・モジュール接続の調整は 1 件も必要なかった**（要件 2.8 の発動なし）。各テストモジュール冒頭の `use super::*;` を含む既存 import がそのまま有効。

接続宣言（8 モジュールとも同一文言・design §移設方式の裁定 案 C）:

```rust
#[cfg(test)]
#[path = "<stem>_<テストモジュール名>.rs"]
mod <テストモジュール名>;
```

`#[cfg(test)]` 行は §13（タスク 2.1）と同じく**元位置に据え置き**、その下 2 行だけを新設する。

### 14.2 非 `mod` `#[cfg(test)]` 項目の残置（設計判断 #3）

`shiori-host32-helper/src/main.rs` の 2 件（design §Supporting References が `:418,424` として列挙した自由関数
——属性行は `:417,423`）は `impl` ブロック内のテスト専用 inherent メソッドであり、`:427` で閉じる本体側の項目である。
**移設せず元位置に残置**した。移設後も同一行番号（`main.rs:417,423`）に在る。
`areka-sylphya/src/actor.rs`・`dola/src/cue/command.rs` には非 `mod` `#[cfg(test)]` 項目は 1 件も無い。

### 14.3 テストモジュールに付いた doc コメント（`///`）の扱い — 本タスクで確定した規則

`areka-sylphya/src/actor.rs` の 2 モジュールは、`#[cfg(test)]` 行の**直前**に doc コメントを持つ:

| モジュール | doc コメント（移設前の行範囲） | 行数 |
|---|---|---:|
| `actor_integration_tests` | `crates/areka-sylphya/src/actor.rs:1030-1035` | 6 |
| `actor_criteria_cage` | `crates/areka-sylphya/src/actor.rs:1381-1419` | 39 |

**裁定: doc コメントは接続宣言に付けたまま本番ファイルへ残す（テストファイルへは移さない）。**

理由:

1. `///` は `#[doc = "…"]` すなわち `mod` 項目の**属性**であって、設計判断 #2 が言う「モジュールブロック外のコメントバナー」（`spine_e2e_test.rs` の `// ===== S2 … =====` 形）ではない。移設単位の定義は「`#[cfg(test)]`＋**付随属性**＋`mod <name> { … }`」であり、属性は項目に付随する。項目そのもの（`mod actor_integration_tests`）は接続宣言として本番ファイルに残るのだから、属性もそこに残るのが整合する——タスク 2.1 が `#[cfg(test)]` 行を元位置に据え置いたのと同一の扱いである。
2. ファイル先頭へ移すには `///` を `//!`（内部 doc）へ書き換えねばならず、これは要件 2.4 が禁じる**コメントの変更**にあたる。`///` のままファイル先頭に置くと直後の `use super::*;` を文書化する別物になる。
3. 残置ならバイト等価で、intra-doc リンク（`[`send_after_death_logs_warn_not_silent`]` 等）の解決先も同一モジュールのままで変わらない。

`dola`・`shiori-host32-helper` の 5 モジュールには doc コメントは付いていない（`#[cfg(test)]` の直前は空行）。
**今後 doc コメント付きテストモジュールを移設するタスクは本裁定に従うこと。**

### 14.4 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 3 ファイルの独自走査

Implementation Notes の指示（「各タスクは自分の担当ファイルで独自に走査し直すこと」）に従い、
本タスクの 3 ファイルを字句状態追跡つきの独立スキャナ（コメント・raw 文字列・エスケープを追跡）で全走査した。
スキャナの妥当性は、既知の該当箇所である `crates/wintf/src/ecs/window_proc/window_pos_tests.rs:689-691`
（§13.2 の移設後の位置）を正しく検出することで確認済み。

| ファイル | 複数行にまたがる文字列リテラル | 盲点該当（`\` 継続でない行） |
|---|---|---|
| `crates/areka-sylphya/src/actor.rs` | 0 件 | **0 件** |
| `crates/dola/src/cue/command.rs` | 0 件 | **0 件** |
| `crates/shiori-host32-helper/src/main.rs` | 1 件（移設前 `:719-721`） | **0 件** |

唯一の複数行文字列 `main.rs:719-721`（移設後 `main_loopback_tests.rs:27-29`）は、継続する 2 行がいずれも
**直前行の末尾 `\` による継続**である（`:719` 末尾 `\`／`:720` 末尾 `\`）。Rust は `\`＋改行の直後の行頭空白を除去するため、
一律 4 スペース de-indent（lead 12/13/13 → 8/9/9）を適用しても**文字列の値はバイト等価**である。目視で突合済み。

**結論: 本タスクの 3 ファイルに §11.4 第 1 の盲点の該当行は 1 件も無い。**

### 14.5 検証（すべて実測・終了コードで判定・クレート単位）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit bfbdfb5 -OriginalPath <本番> -RelocatedPath "<新テスト群>" -Detail`

| クレート | 出力 | exit |
|---|---|---:|
| `areka-sylphya` | `MATCH: test fn 34=34 / helper item 13=13 / mod block 3 / files 3` | 0 |
| `dola` | `MATCH: test fn 29=29 / helper item 1=1 / mod block 1 / files 1` | 0 |
| `shiori-host32-helper` | `MATCH: test fn 17=17 / helper item 1=1 / mod block 4 / files 4` | 0 |

**(b) テスト名リストの不変（要件 2.2 / 2.9・完了状態）** — `cargo test -p <crate> --no-fail-fast -- --list` を移設前後で採取し、
`: test$` 抽出・`[StringComparer]::Ordinal` 整列（§10.2 と同一手順）。採取ターゲットは前後とも既定の `x86_64-pc-windows-msvc`（§10.4）:

| クレート | before 行数 | after 行数 | `Compare-Object` | SHA256（before = after） |
|---|---:|---:|---|---|
| `areka-sylphya` | 171 | 171 | 差分なし | `023A89718C7725917801459586D834CDAD33A9DF1F012130E8DE01B1F13AF75A` |
| `dola` | 638 | 638 | 差分なし | `D42F7CA3082F1112C1E8B2C48D12FED40B4562FF870489A001E5E76101B57BF8` |
| `shiori-host32-helper` | 23 | 23 | 差分なし | `9CA4B3CE62AEBC1F4DA9B2FFD7743FB6EACA192533F58A29629A2FA147ED79B5` |

3 クレートとも対応表を介さない**素のバイト一致**（本タスクの対応表の行は 0 件）。

> **是正（タスク 2.2 レビュー）**: `dola` 行の SHA256 は当初 `A488140F…248C3D` と記録していたが、これは
> `Sort-Object`（カルチャ依存）で整列した値であり、§10.2 が禁じる手順だった。`[StringComparer]::Ordinal` で
> 採り直した正しい値 `D42F7CA3…B57BF8` へ差し替えてある（`areka-sylphya` と `shiori-host32-helper` の 2 行は
> 当初から Ordinal で正しく、変更していない）。
> なお `dola` の**移設前**リストは現ツリーからは再生成できないため、before = after の根拠は
> 次のワークスペース水準の照合に置く——移設後の `cargo test --workspace --no-fail-fast -- --list` が
> コミット済み `before_default.txt` と**バイト一致**（4,790 行・SHA256
> `77F03656B507D72DB4A5D9E5D75DC4849C16A92B6E371F718F8887F7EB43D2AD`）。
> これは §10.2 の手順そのままであり `dola` を含む全クレートを覆う。

`shiori-host32-helper` の 23 行は §10.4 の実測値と一致し、アーキテクチャゲート 3 件
（`loopback_tests::loopback_hello_request_proxy_driven_and_bounded_loop`・`shiori_proxy::tests::testdll_drop_invokes_courtesy_unload`・
`shiori_proxy::tests::testdll_request_roundtrip_get_and_notify`）は移設前後とも `--list` に現れている。
`#[cfg_attr(not(target_arch = "x86"), ignore = "…")]` 属性には一切手を触れていない（要件 2.4）。

**(c) クレート緑（要件 7.2）**

| クレート | コマンド | exit | 結果 |
|---|---|---:|---|
| `areka-sylphya` | `cargo test -p areka-sylphya` | 0 | **171 passed / 0 failed / 0 ignored**（＋doctest 0） |
| `dola` | `cargo test -p dola` | 0 | 7 ターゲット合計 **637 passed / 0 failed** ＋ doctest 1 ignored（= `--list` の 638 と整合） |
| `shiori-host32-helper` | `cargo test -p shiori-host32-helper` | 0 | **20 passed / 0 failed / 3 ignored**（x64 のアーキテクチャゲート 3 件＝§9.8 のとおり緑） |

**(d) 警告非増加（要件 2.6）** — `cargo build -p <crate> --all-targets`

| クレート | exit | 行頭 `warning` 行（移設前実測） | 同（移設後） |
|---|---:|---:|---:|
| `areka-sylphya` | 0 | 0 | **0** |
| `dola` | 0 | 0 | **0** |
| `shiori-host32-helper` | 0 | 5（`"cdecl" is not a supported ABI` ×3 ＋サマリ 2 行） | **5**（同一） |

`shiori-host32-helper` の 5 行は `before_build_warnings.txt` の基準値（`(bin, test)` generated 3 ／ `(bin)` generated 3（3 duplicates））と一致する。

念のためワークスペース全域でも突合した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16`。
§10.5 の移設前基準値 5 数値と**完全一致**（増加ゼロ）。

**(e) 本番本体の無変更** — 移設前コミット `bfbdfb5` の各本番ファイルへ「テストモジュール本体（`mod X {` 〜 閉じ `}`）を
接続宣言 2 行へ置換」という機械規則のみを適用した期待形を生成し、現作業ツリーと逐行突合した。**3 ファイルとも不一致 0**
（`actor.rs` 期待 730 = 実 730 ／ `command.rs` 期待 334 = 実 334 ／ `main.rs` 期待 531 = 実 531）。
すなわち本番本体・モジュール間の空行・doc コメントは 1 文字も変わっていない。

| クレート | `git diff --stat` |
|---|---|
| `areka-sylphya` | `1 file changed, 6 insertions(+), 863 deletions(-)`（挿入 6 = 3 モジュール × 2 行） |
| `dola` | `1 file changed, 2 insertions(+), 757 deletions(-)`（挿入 2 = 1 モジュール × 2 行） |
| `shiori-host32-helper` | `1 file changed, 8 insertions(+), 591 deletions(-)`（挿入 8 = 4 モジュール × 2 行） |

削除行数はいずれも「テストコード行 − `#[cfg(test)]` 行数」（866−3 / 758−1 / 595−4）と一致する。

**(f) 完了状態の直接確認** — 3 本番ファイルに残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言と残置項目のみ:
`actor.rs:675-677,685-687,728-730`／`command.rs:332-334`／`main.rs:517-519,521-523,525-527,529-531`
（＋ `main.rs:417,423` の残置 `#[cfg(test)]` 自由関数、`main.rs:33` の本番 `mod shiori_proxy;`）。
テストモジュール本体は 1 行も残っていない。

**(g) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 11 パスのみ:
変更 3 本（上表の本番ファイル）＋未追跡 8 本（新テストファイル）。`crates/**` の他ファイル・`Cargo.toml` への差分は 0 件。

### 14.6 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（3 クレート・8 テストモジュール・テストコード 2,219 行）では、修正を要する壊れたテスト・
不正なテスト・テスト間の状態汚染は 1 件も発見しなかった。所有 spec への送付所見は 0 件である。**

以下は「調べたが問題なし」と確定した記録（次に触る者が同じ調査を繰り返さないための控え。送付不要）:

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | プロセスグローバルな tracing subscriber（interest-keeper）を `set_global_default` で常駐させる仕組みをテストから使う 3 箇所 | `crates/areka-sylphya/src/actor_tests.rs:255,276`・`crates/areka-sylphya/src/actor_actor_criteria_cage.rs:138,152`（実体は `crates/areka-sylphya/src/test_log_capture.rs`） | **問題なし**。`capture` は `with_default` でスレッドローカルに subscriber を差し込み、global の bare registry は `event` が no-op。捕捉列がテスト間で混ざらないことがモジュール doc（`test_log_capture.rs:1-46`）に明記され、並列負荷下の `Interest::never` 焼き付きも interest-keeper で構造的に到達不能化済み。移設で何も変わらない |
| 2 | 実スレッドを spawn する統合檻（`spawn_sylphya` → `publish` → `barrier` → `join`）。同期は barrier と join のみで `thread::sleep` 皆無 | `crates/areka-sylphya/src/actor_actor_integration_tests.rs` 全域・`actor_actor_criteria_cage.rs:125-165` | **問題なし**。ログ捕捉はいずれも `join` 後のテストスレッド上で行われ、クロススレッド取りこぼしの窓が無い |
| 3 | `std::env::temp_dir()` 配下に `SystemTime` 由来のユニーク名でディレクトリを作り、testdll をコピーする | `crates/shiori-host32-helper/src/main_loopback_tests.rs:98`（後始末は `:420` の `let _ = std::fs::remove_dir_all(&load_dir);`） | **テスト間汚染は無し**（名前がユニークで他テストと共有しない）。ただし後始末はテスト本体末尾の best-effort であり、途中の assert 失敗時は一時ディレクトリが残る（RAII ガード無し）。x64 では当該テストは `ignore` のため既定実行では発生しない。**本 spec 着手前から存在する構造で、移設で変わっていない。是正しない・記録のみ** |
| 4 | `std::env::var("HOST32_TESTDLL_DLL")` の読み取り | `crates/shiori-host32-helper/src/main_loopback_tests.rs:9` | **問題なし**。読み取りのみで `set_var` は無く、他テストの環境を書き換えない |

### 14.7 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-sylphya/src/actor_tests.rs` | 新規（351 行） |
| `crates/areka-sylphya/src/actor_actor_integration_tests.rs` | 新規（341 行） |
| `crates/areka-sylphya/src/actor_actor_criteria_cage.rs` | 新規（165 行） |
| `crates/dola/src/cue/command_tests.rs` | 新規（755 行） |
| `crates/shiori-host32-helper/src/main_resolve_param_tests.rs` | 新規（49 行） |
| `crates/shiori-host32-helper/src/main_classify_tests.rs` | 新規（88 行） |
| `crates/shiori-host32-helper/src/main_load_ack_tests.rs` | 新規（25 行） |
| `crates/shiori-host32-helper/src/main_loopback_tests.rs` | 新規（421 行） |
| 上記に対応する本番 3 ファイル | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体・doc コメントは無変更） |
| `verification/notes.md` | 本節（§14）を追記 |
| `verification/mapping/*.csv` | **作成しない**（3 クレートとも FQN 変化 0 件・対応表の行を持たない） |

コミットは要件 7.1 に従い**クレート単位の 3 コミット**へ分ける（`areka-sylphya` ／ `dola` ／ `shiori-host32-helper`）。

## 15. クレート単位のテスト分離とテーマ分割: `areka-emo-compose`（タスク 3.1・要件 1.1 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `7599c90`
- 実行シェル: **PowerShell（pwsh 7）**
- 下表の本番 3 ファイル＋新規テストファイル 7 本＋`verification/mapping/areka-emo-compose.csv`＋本ファイル以外には一切触れていない（`Cargo.toml`・他クレート・spec 本体ドキュメントは無変更）
- **本 spec で初めてテーマ分割を行うタスク**であり、対応表フラグメント（§11.3）に実データの行が入るのも本タスクが最初である

### 15.1 移設した 3 ファイル（design §File Structure Plan の `crates/areka-emo-compose` と完全一致）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 新テストファイル | 新ファイル行 | 本番 残行 |
|---|---:|---|---|---|---:|---:|
| `src/plan.rs` | 2,203 | `tests`（668-2203・テストコード 1,536） | **テーマ分割 ×2 ＋共有ヘルパ** | `plan_test_support.rs` ／ `plan_ops_tests.rs` ／ `plan_extent_tests.rs` | 139 / 942 / 467 | 678 |
| `src/scale.rs` | 1,778 | `tests`（468-1778・テストコード 1,311） | **テーマ分割 ×2 ＋共有ヘルパ** | `scale_test_support.rs` ／ `scale_resample_tests.rs` ／ `scale_ratio_tests.rs` | 2 / 499 / 809 | 478 |
| `src/fold.rs` | 1,132 | `tests`（265-1132・テストコード 868） | 単純移設（FQN 不変） | `fold_tests.rs` | 865 | 267 |

- 3 ファイルとも**テストモジュールは 1 個・ファイル末尾に連続配置**（design の実測どおり・ズレ 0）。モジュール名はいずれも `tests`。
- **新テストファイル 7 本はすべて 1,000 行以下**（最大 `plan_ops_tests.rs` の 942 行）。僅少超過で単一維持したファイルは **0 件**（§7.4 への追記は不要）。
- 3 ファイルとも非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は 1 件も存在しない（`scan_raw.csv` の該当 3 行が示すとおり、`#[cfg(test)]` の出現はテストモジュール 1 箇所のみ）。
- `#[cfg(test)]` 行は §13 / §14 と同じく**元位置に据え置き**、テーマ分割で増えるモジュールぶんの宣言だけを新設する（`git diff` の挿入は plan 8 行・scale 8 行・fold 2 行の計 18 行）。

接続宣言（7 モジュールとも同一文言・design §移設方式の裁定 案 C）:

```rust
#[cfg(test)]
#[path = "<stem>_<モジュール名>.rs"]
mod <モジュール名>;
```

### 15.2 要件 1.4 による除外（本タスクで**触っていない** 3 ファイル）

`crates/areka-emo-compose/src` には既に本番ファイル外へ分離済みのテストファイルが 3 本ある。素の
`#[cfg(test)] mod X;`（`lib.rs:166-176`）で宣言される歴史的形式であり、要件 1.4 により**除外・そのまま維持**した。
案 C へ揃え直していないし、1,000 行超（`golden_tests.rs`）のテーマ分割もしていない。

| ファイル | 行数 | 宣言元 |
|---|---:|---|
| `golden_tests.rs` | 1,356 | `lib.rs:169-170` |
| `composer_tests.rs` | 703 | `lib.rs:172-173` |
| `log_firing_tests.rs` | 664 | `lib.rs:175-176` |

同様に `log_capture.rs`（テスト専用ハーネス・`lib.rs:166-167`）と、`lib.rs:178-194` の `contract_tests`（テストコード 17 行・
500 行以下ゆえ要件 1.5 で非必須・設計判断 #10 により任意移設は行わない）も無変更である。

### 15.3 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

design は本 2 ファイルを「テーマ分割 ×約 2」と見積もっている。実装時に各モジュールの内部構造
（コメントバナー・対象関数のまとまり・ヘルパの参照関係）を確認した結果、**両ファイルとも本番 API の継ぎ目に沿って
ちょうど 2 テーマへ割れる**ことを確認し、見積りどおり 2 テーマ＋共有ヘルパの構成とした。関数の中は 1 箇所も割っておらず、
識別子の改名も 0 件である（要件 2.9）。

**`plan.rs`** — テストモジュール内部には `// ── task N ── …` のバナーが 6 本あり、
それぞれが本番の 2 系統の関数へ素直に対応する。バナーの区切りを 1 本も跨がずに 2 テーマへ束ねられる。

| 新モジュール（ファイル） | 移設前の行範囲 | 束ねたバナー区画 | 対象の本番関数 | テスト数 |
|---|---|---|---|---:|
| `test_support`（`plan_test_support.rs`） | 681-809 | `// ---- 合成モデルビルダ ----`（10 ヘルパ） | —（共有土台） | 0 |
| `ops_tests`（`plan_ops_tests.rs`） | 811-1403 ＋ 1864-2202 | 冒頭の task 4 群・task 5.2・task 5.3・task 11.2・task 7.2 | `push_static_element_ops` / `derive_ops` / `flatten_surface`（命令列の導出） | 28 |
| `extent_tests`（`plan_extent_tests.rs`） | 1405-1862 | task 5.4・task 5.5 | `compute_extent` / `build_plan`（静的外形と計画組み立ての 3 分類） | 12 |

**`scale.rs`** — バナーは 2 本あるが、いずれも「task 6.1: 純関数の全網羅」「collision-dpi-hittest task 1」という
**作業単位（時系列）**の見出しで、テーマ名の材料にならない。一方で対象関数は `resample`（自由関数・双一次再標本化）と
`ScaleRatio`（型とそのメソッド `new`/`mul`/`as_f32`/`scale_len`/`scaled_extent`/`unscale_coord`）の 2 系統に
きれいに分かれ、**ヘルパの参照関係もこの境界と一致する**（`surface_of`・`gray`・`gray_bytes`・`oracle`・
`assert_matches_oracle` の 5 ヘルパは `resample` 系テストからしか参照されない）。よって対象関数のまとまりを
テーマ境界に採った。バナー 2 本は直後の項目に付属したまま移動しており、文言は 1 文字も変えていない。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support`（`scale_test_support.rs`） | 473-474 | `const AUTHOR_DPI`（両テーマから参照される唯一の共有項目） | 0 |
| `resample_tests`（`scale_resample_tests.rs`） | 476-818 ＋ 1120-1271 | `resample`（＋専用ヘルパ 5 本） | 11 |
| `ratio_tests`（`scale_ratio_tests.rs`） | 820-1118 ＋ 1273-1777 | `ScaleRatio` の各メソッド | 27 |

`fold.rs` は移設後 865 行で 1,000 行以下ゆえテーマ分割せず、`mod tests` のまま `fold_tests.rs` へ単純移設した
（完全修飾名 `fold::tests::*` は 24 件とも不変・対応表の行を持たない）。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: `plan_test_support.rs` の 10 関数と
`scale_test_support.rs` の 1 定数へ `pub(super)` を付与した（付与のみ・本文は無変更）。テーマモジュールからは
`use super::test_support::*;` で参照する。付与した可視性は本文一致検証の正規化が吸収する（§11.4）。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 各テーマファイルの先頭には移設元モジュールの `use` 群を置き、
そのファイルで実際に使う項目だけへ絞った（絞らないと `unused_imports` 警告が増えて要件 2.6 に反する）。
`use` 項目は §11.4 のとおり本文一致検証の対象外である。**可視性・`use` 以外の調整は 1 件も必要なかった**（要件 2.8 の発動なし）。

### 15.4 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 3 ファイルの独自走査

Implementation Notes の指示に従い、担当 3 ファイルを字句状態追跡つきの独立スキャナ
（行コメント・ブロックコメント・通常文字列・raw 文字列・バイト文字列・文字リテラル・エスケープを追跡し、
「行頭時点で文字列リテラルの内部にいる行」と「直前行が `\` 継続か」を判定する）で全走査した。
スキャナの妥当性は、既知の唯一の該当箇所 `crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691`（§13.2）を
盲点として、同ファイルの `:382`・`:429`・`:614`・`:690` を `\` 継続として正しく切り分けることで確認済み。

| ファイル | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---:|---:|
| `crates/areka-emo-compose/src/plan.rs` | 0 | **0** |
| `crates/areka-emo-compose/src/scale.rs` | 0 | **0** |
| `crates/areka-emo-compose/src/fold.rs` | 0 | **0** |

**結論: 担当 3 ファイルには複数行文字列リテラルが 1 件も無く、§11.4 第 1 の盲点の該当行は 0 件。**
よって全行へ一律 4 スペース de-indent を適用してよく、例外処理は不要だった。

### 15.5 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4・テーマ分割の同一性）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 7599c90 -OriginalPath <本番> -RelocatedPath "<新テスト群>" -Detail`

| 対象 | 出力 | exit |
|---|---|---:|
| `plan.rs` → 3 ファイル | `MATCH: test fn 40=40 / helper item 19=19 / mod block 1 / files 3` | 0 |
| `scale.rs` → 3 ファイル | `MATCH: test fn 38=38 / helper item 6=6 / mod block 1 / files 3` | 0 |
| `fold.rs` → 1 ファイル | `MATCH: test fn 24=24 / helper item 17=17 / mod block 1 / files 1` | 0 |

テーマ分割は項目の再配置と順序変更を伴うが、テスト関数は識別子キーで 1:1、ヘルパ項目は正規化本文の多重集合で
突合されるため（§11.4）、3 件とも exit 0＝本文完全一致である。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-emo-compose.csv`

```
PASS: 全単射 OK / 行数 78 / 相異なる old_fqn 78 / 相異なる new_fqn 78 / フラグメント 1
  - areka-emo-compose.csv: 78 行
```

exit 0。78 行の内訳は `plan::tests::*` → `plan::ops_tests::*` 28 行／`plan::tests::*` → `plan::extent_tests::*` 12 行／
`scale::tests::*` → `scale::resample_tests::*` 11 行／`scale::tests::*` → `scale::ratio_tests::*` 27 行。
`reason` は全行 `theme_split`、末尾セグメント（関数識別子）は旧新で同一である。`fold.rs` の 24 件は FQN 不変ゆえ行を持たない。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list` → `: test$` 抽出 →
`[Array]::Sort($arr, [System.StringComparer]::Ordinal)` → UTF-8 BOM 無し・重複行を残す）でリストを採取し、
コミット済み `before_default.txt` と対応表経由で突合した:

```
BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
AFTER       : after_default_task31.txt  (4790 行 / 相異なる 4787)
MAPPING     : 78 行 (1 ファイル) / 適用 78 行 / 未使用 0 行
LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
RESULT: PASS
```

exit 0。**適用 78 行・未使用 0 行**——対応表の全行が移設前リストの実在する行に当たり、適用結果が移設後リストと
多重集合として完全一致した。移設後リストの SHA256 は `E312A434FFBAD0A1BF842FF697D8AE488C5942E3C34FC4BCB59F837C9F0616A5`
（`before_default.txt` の `77F03656…43D2AD` と異なるのは 78 件のモジュールパスが変わったためで、対応表適用後は一致する）。
整列は `Sort-Object` を使わず `[System.StringComparer]::Ordinal` で行った（§11.8）。

> リストは**ワークスペース全域**で突合した（クレート単位へ絞るより強い形）。なお `scale::` で始まる行は
> ワークスペース全体で 56 行あり、本クレートの 38 行に加えて他クレートの `scale` モジュール由来が 18 行含まれる。
> 56 行に**重複する完全修飾名は 1 件も無い**ことを確認済みで、対応表の 78 行が他クレートの行を巻き込む余地は無い。

移設後リストのモジュール別内訳（本クレートぶん・`cargo test -p areka-emo-compose -- --list`・合計 216）:
`plan::ops_tests` 28 ／ `plan::extent_tests` 12 ／ `scale::resample_tests` 11 ／ `scale::ratio_tests` 27 ／
`fold::tests` 24（残りは無変更の 12 モジュール）。移設前の `plan::tests` 40・`scale::tests` 38・`fold::tests` 24 と本数が一致する。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-emo-compose` → **exit 0**。
**216 passed / 0 failed / 0 ignored**（＋doctest 0）。移設前の 216 と一致。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-emo-compose --all-targets` → exit 0・`warning` 行 **0 件**。
`before_build_warnings.txt` の `[PER-UNIT TALLY]` に `areka-emo-compose` の行は無く（基準値 0）、増加ゼロである。
念のためワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。

**(f) 本番本体の無変更** — 移設前コミット `7599c90` の各本番ファイルの先頭〜旧 `#[cfg(test)]` 行の直前までを
現作業ツリーと逐行突合し、**3 ファイルとも不一致 0**（plan 1-667 ／ scale 1-467 ／ fold 1-264）。

| ファイル | `git diff --stat` |
|---|---|
| `plan.rs` | `1541 +--------` （挿入 8・削除 1,533） |
| `scale.rs` | `1316 +--------` （挿入 8・削除 1,308） |
| `fold.rs` | `869 +--------` （挿入 2・削除 867） |

3 ファイル合計 `3 files changed, 18 insertions(+), 3708 deletions(-)`。挿入 18 = 新設した接続宣言のうち
元位置に据え置いた `#[cfg(test)]` 行 3 本を除いた行数である。

**(g) 完了状態の直接確認** — 3 本番ファイルに残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言のみ:
`plan.rs:668-670,672-674,676-678` ／ `scale.rs:468-470,472-474,476-478` ／ `fold.rs:265-267`。
テストモジュール本体は 1 行も残っていない。

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 11 パスのみ:
変更 3 本（本番ファイル）＋未追跡 8 本（新テストファイル 7 本＋`verification/mapping/areka-emo-compose.csv`）。
`crates/**` の他ファイル（`golden_tests.rs`・`composer_tests.rs`・`log_firing_tests.rs` を含む）・`Cargo.toml` への差分は 0 件。

### 15.6 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（`areka-emo-compose` の 3 ファイル・102 テスト・テストコード 3,715 行）では、修正を要する
壊れたテスト・不正なテスト・テスト間の状態汚染は 1 件も発見しなかった。所有 spec への送付所見は 0 件である。**

以下は「調べたが問題なし」と確定した記録（次に触る者が同じ調査を繰り返さないための控え。送付不要）:

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | `tracing` ログ捕捉の呼び出し 6 箇所（plan 2・scale 4） | `crates/areka-emo-compose/src/plan_ops_tests.rs:755,795`・`scale_resample_tests.rs:296,319`・`scale_ratio_tests.rs:139,150`（実体は `crates/areka-emo-compose/src/log_capture.rs:59-67`） | **問題なし**。`capture_logs` は `tracing::subscriber::with_default` でスレッドローカルに subscriber を差し込み、`set_global_default` を使わない（`log_capture.rs:1-19` に明記）。compose パイプラインは完全同期でログは呼び出しスレッド上で発火するため、クロージャ復帰時点で捕捉が完結し、並行実行でも他テストと混ざらない |
| 2 | ファイルシステムを読む唯一のテスト経路（emo2 fixture の `surfaces.txt`） | `crates/areka-emo-compose/src/fold_tests.rs:671-676`（`emo2_shell()`） | **問題なし**。`CARGO_MANIFEST_DIR` 起点の絶対パスを組み立てての**読み取り専用**。書き込み・一時ディレクトリ生成・後始末は無く、テスト間で共有される可変状態を作らない |
| 3 | `static` / `thread_local` / `std::env::set_var` / `unsafe` / `sleep` / `#[ignore]` / `#[should_panic]` の使用 | 担当 7 テストファイル全域 | **0 件**（全走査で確認）。プロセスグローバルな状態に触れるテストは 1 件も無い |
| 4 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | — | **発生せず**。`use super::*;` を含む既存 import がそのまま有効で、必要だったのは共有ヘルパへの `pub(super)` 付与（11 項目）と各テーマファイルの `use` ヘッダの絞り込みのみ |

### 15.7 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-emo-compose/src/plan_test_support.rs` | 新規（139 行・共有ヘルパ 10 本） |
| `crates/areka-emo-compose/src/plan_ops_tests.rs` | 新規（942 行・28 テスト） |
| `crates/areka-emo-compose/src/plan_extent_tests.rs` | 新規（467 行・12 テスト） |
| `crates/areka-emo-compose/src/scale_test_support.rs` | 新規（2 行・共有定数 1 本） |
| `crates/areka-emo-compose/src/scale_resample_tests.rs` | 新規（499 行・11 テスト） |
| `crates/areka-emo-compose/src/scale_ratio_tests.rs` | 新規（809 行・27 テスト） |
| `crates/areka-emo-compose/src/fold_tests.rs` | 新規（865 行・24 テスト） |
| 上記に対応する本番 3 ファイル | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体は無変更） |
| `verification/mapping/areka-emo-compose.csv` | **新規・本 spec で最初の対応表フラグメント**（78 行・全単射検証済み） |
| `verification/notes.md` | 本節（§15）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-emo-compose`）とする。

## 16. クレート単位のテスト分離とテーマ分割: `areka-seriko`（タスク 3.2・要件 1.1 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `1021ccf`（タスク 3.1 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 下表の本番 3 ファイル＋新規テストファイル 7 本＋`verification/mapping/areka-seriko.csv`＋本ファイル以外には一切触れていない（`Cargo.toml`・他クレート・spec 本体ドキュメントは無変更）

### 16.1 移設した 3 ファイル（design §File Structure Plan の `crates/areka-seriko` と完全一致）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 新テストファイル | 新ファイル行 | 本番 残行 |
|---|---:|---|---|---|---:|---:|
| `src/actor.rs` | 2,331 | `tests`（485-2331・テストコード 1,847） | **テーマ分割 ×2 ＋共有ヘルパ** | `actor_test_support.rs` ／ `actor_dispatch_tests.rs` ／ `actor_bind_loop_tests.rs` | 84 / 840 / 928 | 493 |
| `src/state.rs` | 1,576 | `tests`（519-1576・テストコード 1,058） | **テーマ分割 ×2 ＋共有ヘルパ** | `state_test_support.rs` ／ `state_surface_tests.rs` ／ `state_bind_pattern_tests.rs` | 10 / 483 / 566 | 527 |
| `src/looper.rs` | 939 | `tests`（376-939・テストコード 564） | 単純移設（FQN 不変） | `looper_tests.rs` | 561 | 378 |

- 3 ファイルとも**テストモジュールは 1 個・ファイル末尾に連続配置**（`scan_raw.csv:175,178,181` の実測どおり・ズレ 0）。モジュール名はいずれも `tests`。
- **新テストファイル 7 本はすべて 1,000 行以下**（最大 `actor_bind_loop_tests.rs` の 928 行）。僅少超過で単一維持したファイルは **0 件**（§7.4 への追記は不要）。
- 3 ファイルとも非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は 1 件も存在しない（`scan_raw.csv` の該当 3 行が示すとおり `#[cfg(test)]` の出現はテストモジュール 1 箇所のみ）。
- `#[cfg(test)]` 行は §13〜§15 と同じく**元位置に据え置き**、テーマ分割で増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` の挿入は actor 8・state 8・looper 2 の計 18 行）。
- 同クレートの残る 5 本（`bind.rs` 485／`table.rs` 323／`timeline.rs` 313／`resolve.rs` 186／`output.rs` 159）はテストコード 500 行以下ゆえ要件 1.5 で非必須・設計判断 #10 により任意移設は行わない（無変更）。

接続宣言（7 モジュールとも同一文言・design §移設方式の裁定 案 C）:

```rust
#[cfg(test)]
#[path = "<stem>_<モジュール名>.rs"]
mod <モジュール名>;
```

### 16.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

design は本 2 ファイルを「テーマ分割 ×約 2」と見積もっている。実装時に各モジュールの内部構造
（コメントバナー・対象関数のまとまり・ヘルパの参照関係）を確認した結果、**両ファイルとも見積りどおり
ちょうど 2 テーマ＋共有ヘルパへ割れる**ことを確認した。関数の中は 1 箇所も割っておらず、識別子の改名も 0 件である（要件 2.9）。

**`actor.rs`** — テストモジュール内部には長いバナーが 5 本ある（635／919／1205／1359／1984）。いずれも
`// Task 4.1: 停止経路・異常系…` のように**作業単位（時系列）の見出しに主題の説明が併記された混成形**である。
`scale.rs`（§15.3）のように主題の材料がまったく無いわけではないので、バナー区画は 1 本も跨がずに束ね、
**束ね方は本番 API の継ぎ目で決めた**。`handle_message` の分岐は「面 id を切り替える経路」と
「面 id を変えずに現在面を `emit_display` から再発行する経路」に割れ、後者が bind 適用（BindSet の変化）と
Tick（アニメコマの進行）の 2 本である。

| 新モジュール（ファイル） | 移設前の行範囲 | 束ねたバナー区画 | 対象の本番項目 | テスト数 |
|---|---|---|---|---:|
| `test_support`（`actor_test_support.rs`） | 492-500 ／ 653-670 ／ 1933-1982 | —（両テーマから参照される 6 ヘルパ） | — | 0 |
| `dispatch_tests`（`actor_dispatch_tests.rs`） | 502-633 ／ 635-651 ／ 672-1357 | 冒頭群・Task 4.1・Task 4.4・Task 6.2 | `SerikoSink`（結線契約）・`spawn_seriko`（Close／disconnect／停止後 emit）・`handle_message` の面切替分岐（シェル面／バルーン面）と担当外 cue の honor | 19 |
| `bind_loop_tests`（`actor_bind_loop_tests.rs`） | 1359-1931 ／ 1984-2330 | Task 6.1/6.3（bind）・Task 6.1（Tick） | `handle_message` の bind キャリア分岐と Tick 腕。いずれも**表示中の面 id を変えずに現在面を再発行する**経路 | 15（うち入れ子 `tick_loop_tests` 4） |

**ヘルパ参照関係による裏取り**（全 16 ヘルパの呼び出し行を全数走査）: 両テーマから参照されるのは
`emote_cue`・`tiny_resolver`・`fresh_states`・`inert_runtime`・`capture_logs_flow`・`capture_logs` の **6 本のみ**で、
`assert_cue_sink_contract`・`entityref_cue`・`balloon_cue`・`text_cue`・`wait_cue` の 5 本は `dispatch_tests` 専用、
`bind_carrier_cue`・`named_carrier_cue`・`noncanonical_custom_cue`・`arm_bind_resolver`・`eye_mustselect_resolver` の
5 本は `bind_loop_tests` 専用である（5 対 5 の対称・境界を跨ぐ専用ヘルパ 0 本）。この分布が上記の境界を裏づける。

入れ子モジュール `tick_loop_tests`（移設前 2001-2330・テスト 4 本）は**項目としてまるごと**
`bind_loop_tests` へ移した（入れ子を解体していない）。完全修飾名は
`actor::tests::tick_loop_tests::*` → `actor::bind_loop_tests::tick_loop_tests::*` へ変わり、対応表に 4 行を持つ。

**`state.rs`** — バナーは 4 本（787／1013／1208／1331）あり、いずれも
`// ---- apply_balloon（バルーン面・別 map 同居…） ----` のように**本番 API 名そのもの**を主題にした真正のテーマ見出しである。
`ScopeStates` の可変 API は「面 id を切り替える 2 本（`apply` / `apply_balloon`）」と
「面 id を変えずに再発行の要否を判定する 3 本（`apply_bind` / `apply_bind_exclusive` / `commit_pattern`）」へ割れ、
バナー区画の並びがそのままこの 2 群に対応する。

| 新モジュール（ファイル） | 移設前の行範囲 | 束ねたバナー区画 | 対象の本番項目 | テスト数 |
|---|---|---|---|---:|
| `test_support`（`state_test_support.rs`） | 523-530 | —（両テーマから参照される 2 ヘルパ） | — | 0 |
| `surface_tests`（`state_surface_tests.rs`） | 532-1011 | 冒頭群（`apply`）・`apply_balloon` | `ScopeStates::apply` / `apply_balloon`（面の状態機械・相互独立） | 22 |
| `bind_pattern_tests`（`state_bind_pattern_tests.rs`） | 1013-1575 | `apply_bind`・`apply_bind_exclusive`・`commit_pattern`/`current_pattern` | `apply_bind` / `apply_bind_exclusive` / `commit_pattern` / `current_pattern` / `shown_slots` | 23 |

- `shown_slots_enumerates_only_shown_shell_and_balloon`（移設前 1550-1575）は `commit_pattern` バナー区画の末尾に置かれているため、
  バナーを跨がずそのまま `bind_pattern_tests` へ入れた（`shown_slots` は Tick 経路の入口として pattern 群と同居しているのが元の構造）。
- 同様に `apply_surface_switch_clears_stored_pattern` / `apply_balloon_surface_switch_clears_stored_pattern` は名前こそ `apply` 系だが
  「面切替で格納 pattern がクリアされる」ことの檻であり、元から `commit_pattern` 区画に在る。区画どおり `bind_pattern_tests` へ入れた。
- ヘルパ参照関係の裏取り: `binds_1100_1207`（546,560,627,651,676 と 1407,1491）・`empty_states`（両テーマの全域）が両テーマ参照ゆえ `test_support` へ。
  `pat`（1366-1533 でのみ参照）は `bind_pattern_tests` 専用ゆえ同ファイルに残した。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: `actor_test_support.rs` の 6 関数と
`state_test_support.rs` の 2 関数へ `pub(super)` を付与した（付与のみ・本文は無変更）。テーマモジュールからは
`use super::test_support::*;` で参照する。設計判断の申し送り（Implementation Notes「共有ヘルパは項目が 1 件でも集約する」）に従い、
複製は 1 件も作っていない（複製すると本文一致検証が `ITEM-EXTRA` を出す）。

**バナーの帰属**: 本文一致検証は項目の直前コメントを当該項目の本文の一部として比較する。したがってバナーは
**元と同じ種類の項目へ付属したまま**移した。actor の Task 4.1 バナー（635-646）は移設前も
`use crate::resolve::SurfaceResolver;` に付属する `use` 項目の一部（＝比較対象外）だったため、移設後も
`use areka_emo_compose::{BindSet, PatternState};` の直前へ置いて帰属を変えていない。他の 4 本
（Task 4.4／Task 6.2／Task 6.1・6.3／Task 6.1 Tick）と state の 3 本（787／1013／1208）は直後のテスト関数・
ヘルパ・入れ子 `mod` に付属したまま移動しており、文言は 1 文字も変えていない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 各テーマファイルの先頭に移設元の `use` 群を置き、そのファイルで実際に使う項目だけへ絞った。
絞る前のビルドでは `actor_dispatch_tests.rs` に `unused_imports` が 3 件出て要件 2.6（警告非増加）に反したため、
`use crate::resolve::SurfaceResolver;`・`use crate::state::ScopeStates;`・`use std::collections::{BTreeMap, BTreeSet};` の 3 行を落とした
（いずれも `use super::*;` で同名が入るか、当該ファイルでは未使用）。`use` 項目は §11.4 のとおり本文一致検証の対象外である。
入れ子 `tick_loop_tests` の `use super::*;` は親（`bind_loop_tests`）のグロブ取り込みを素通しで解決でき、追加調整は不要だった。
**可視性・`use` 以外の調整は 1 件も必要なかった**（要件 2.8 の発動なし）。

### 16.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 3 ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ
（行コメント・ブロックコメント（入れ子）・通常文字列・raw 文字列・バイト文字列・文字リテラルとライフタイムの判別・
エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と「直前行が `\` 継続か」を判定する）で全走査した。
スキャナの妥当性は、既知の唯一の該当箇所 `crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691`（§13.2）を
**盲点 1 件**として検出し、同ファイルの `:382`・`:429`・`:614`・`:690` を **`\` 継続 4 件**として正しく切り分けることで確認した
（実測出力: `継続行 5 件: 382,429,614,690,691 / 盲点該当 1 件: 691`）。

| ファイル | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---:|---:|
| `crates/areka-seriko/src/actor.rs`（移設前） | 0 | **0** |
| `crates/areka-seriko/src/state.rs`（移設前） | 0 | **0** |
| `crates/areka-seriko/src/looper.rs`（移設前） | 0 | **0** |
| 新テストファイル 7 本（移設後） | 0 | **0** |

**結論: 担当 3 ファイルには複数行文字列リテラルが 1 件も無く、§11.4 第 1 の盲点の該当行は 0 件。**
よって全行へ一律 4 スペース de-indent を適用してよく、例外処理は不要だった。

### 16.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 1021ccf -OriginalPath <本番> -RelocatedPath "<新テスト群>" -Detail`

| 対象 | 出力 | exit |
|---|---|---:|
| `actor.rs` → 3 ファイル | `MATCH: test fn 34=34 / helper item 24=24 / mod block 1 / files 3` | 0 |
| `state.rs` → 3 ファイル | `MATCH: test fn 45=45 / helper item 3=3 / mod block 1 / files 3` | 0 |
| `looper.rs` → 1 ファイル | `MATCH: test fn 18=18 / helper item 12=12 / mod block 1 / files 1` | 0 |

`actor.rs` のヘルパ 24 件は、テストモジュール直下の 16 件と入れ子 `tick_loop_tests` の 8 件の合計である
（`ParseItems` は入れ子 `mod` を平坦化して比較するため両者が 1 つの多重集合に入る）。

**(a2) 行単位の多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、
それとは独立に「移設前ブロック本体を一律 4 スペース de-indent した行の多重集合」と「新テストファイル群の行の多重集合」を
（空行を除いて）突合した。差分は下表のとおり**すべて要件 2.4 が許容する調整（可視性付与・`use`）だけ**であり、
テスト本文の行は 1 行も増減・改変していない。

| 対象 | 元 | 新 | 消えた行 | 増えた行 | 内訳 |
|---|---:|---:|---:|---:|---|
| `actor.rs` | 1,700 | 1,708 | 8 | 16 | 消 = `pub(super)` を付ける前の `fn` 行 6 本＋落とした `use` 2 本 ／ 増 = `pub(super) fn` 6 本＋新設 `use` 10 本 |
| `state.rs` | 931 | 935 | 2 | 6 | 消 = `pub(super)` 前の `fn` 行 2 本 ／ 増 = `pub(super) fn` 2 本＋新設 `use` 4 本 |
| `looper.rs` | 499 | 499 | **0** | **0** | 完全一致（単純移設ゆえ調整ゼロ） |

de-indent の分類（移設した全行）: **(a) ちょうど −4 スペース**または**(b) バイト同値（空行）**のいずれか。
これ以外に分類される行は **0 件**（アセンブラが検出したら停止する設計で、全 7 ファイルとも検出 0）。
内訳は `looper_tests` 499/62・`actor_test_support` 65/3（＋`pub(super)` 6）・`actor_dispatch_tests` 767/43・
`actor_bind_loop_tests` 855/50・`state_test_support` 5/0（＋`pub(super)` 2）・`state_surface_tests` 420/39・
`state_bind_pattern_tests` 503/36（形式: −4 スペース行 / 空行）。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-seriko.csv`

```
PASS: 全単射 OK / 行数 79 / 相異なる old_fqn 79 / 相異なる new_fqn 79 / フラグメント 1
  - areka-seriko.csv: 79 行
```

exit 0。79 行の内訳は `actor::tests::*` → `actor::dispatch_tests::*` 19 行／`actor::tests::*` → `actor::bind_loop_tests::*` 11 行／
`actor::tests::tick_loop_tests::*` → `actor::bind_loop_tests::tick_loop_tests::*` 4 行／
`state::tests::*` → `state::surface_tests::*` 22 行／`state::tests::*` → `state::bind_pattern_tests::*` 23 行。
`reason` は全行 `theme_split`、末尾セグメント（関数識別子）は旧新で同一である。`looper.rs` の 18 件は FQN 不変ゆえ行を持たない。
既存フラグメントとの結合検証（`-Path $V/mapping`）も `PASS: 全単射 OK / 行数 157 / … / フラグメント 2` で exit 0（キー衝突なし）。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list` → `: test$` 抽出 →
`[Array]::Sort($arr, [System.StringComparer]::Ordinal)` → UTF-8 BOM 無し・重複行を残す）でリストを採取し、
コミット済み `before_default.txt` と**タスク 3.1・3.2 両方のフラグメント**を渡して突合した:

```
BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
AFTER       : after_default_task32.txt  (4790 行 / 相異なる 4787)
MAPPING     : 157 行 (2 ファイル) / 適用 157 行 / 未使用 0 行
LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
RESULT: PASS
```

exit 0。**適用 157 行・未使用 0 行**。移設後リストの SHA256 は `99EF337A4322D91E2EFFE1DD1BF1799C7CB86E0F3111F228153704E456775B1E`。
整列は `Sort-Object` を使わず `[System.StringComparer]::Ordinal` で行った（§11.8）。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、
対応表を**使わない**多重集合の対称差で確かめた。手順は (i) `before_default.txt` へタスク 3.1 のフラグメントだけを適用して
本タスク着手直前のリストを復元し、(ii) それと移設後リストの対称差を取る:

| 検査 | 結果 |
|---|---|
| 消えた行数 / 現れた行数 / 本タスクの対応表行数 | **79 / 79 / 79**（三者一致） |
| 消えた行がすべて `old_fqn` に在るか | **True** |
| 現れた行がすべて `new_fqn` に在るか | **True** |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。

移設後リストのモジュール別内訳（`cargo test -p areka-seriko -- --list`・本クレート該当分 97）:
`actor::dispatch_tests` 19 ／ `actor::bind_loop_tests` 11 ／ `actor::bind_loop_tests::tick_loop_tests` 4 ／
`state::surface_tests` 22 ／ `state::bind_pattern_tests` 23 ／ `looper::tests` 18。
移設前の `actor::tests` 30・`actor::tests::tick_loop_tests` 4・`state::tests` 45・`looper::tests` 18 と本数が一致する。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-seriko --no-fail-fast` → **exit 0**。
**203 passed / 0 failed / 0 ignored**（lib 177 ＋ 統合 5+8+1+9+3 ＋ doctest 0）。
移設前の独立導出値と一致する: 移設前 `cargo test -p areka-seriko -- --list` の `: test$` 行は **203 行**、
うち統合テストバイナリ 26 本を差し引いた lib ぶんが 177 本で、移設後の内訳と完全一致する。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-seriko --all-targets` → exit 0・
`warning: areka-seriko (lib test) generated 4 warnings`。`before_build_warnings.txt` の
`[PER-UNIT TALLY]` が本クレートに割り当てる基準値 **4 件**と同数で、増加ゼロ。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。

**(f) 本番本体の無変更** — 移設前コミット `1021ccf` の各本番ファイルの先頭〜旧 `#[cfg(test)]` 行の直前までを
現作業ツリーと逐行突合し、**3 ファイルとも不一致 0**（actor 1-484 ／ state 1-518 ／ looper 1-375）。

| ファイル | `git diff --numstat` |
|---|---|
| `actor.rs` | 挿入 8 ／ 削除 1,846 |
| `state.rs` | 挿入 8 ／ 削除 1,057 |
| `looper.rs` | 挿入 2 ／ 削除 563 |

3 ファイル合計 `3 files changed, 18 insertions(+), 3466 deletions(-)`。挿入 18 = 新設した接続宣言のうち
元位置に据え置いた `#[cfg(test)]` 行 3 本を除いた行数である。

**(g) 完了状態の直接確認** — 3 本番ファイルに残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言のみ:
`actor.rs:485-493` ／ `state.rs:519-527` ／ `looper.rs:376-378`。テストモジュール本体は 1 行も残っていない。

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 11 パスのみ:
変更 3 本（本番ファイル）＋未追跡 8 本（新テストファイル 7 本＋`verification/mapping/areka-seriko.csv`）。
`crates/**` の他ファイル・`Cargo.toml` への差分は 0 件。

### 16.5 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（`areka-seriko` の 3 ファイル・97 テスト・テストコード 3,469 行）では、修正を要する
壊れたテスト・不正なテスト・テスト間の状態汚染は 1 件も発見しなかった。所有 spec への送付所見は 0 件である。**

以下は「調べたが問題なし／既存のまま据え置き」と確定した記録（次に触る者が同じ調査を繰り返さないための控え。是正は行わない）:

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | `tracing` ログ捕捉ハーネスの呼び出し 18 箇所（`dispatch_tests` 8・`bind_loop_tests` 10） | `crates/areka-seriko/src/actor_dispatch_tests.rs:75,261,330,387,488,546,713,767`・`actor_bind_loop_tests.rs:94,283,324,361,402,437,472,520,552,811`（実体は `crates/areka-seriko/src/actor_test_support.rs:38-49`（`capture_logs_flow`）と `:51-84`（`capture_logs`）） | **問題なし**。`capture_logs` は `tracing::subscriber::with_default` でスレッドローカルに subscriber を差し込み、`set_global_default` を使わないため並行実行でも他テストと混ざらない。ハーネスが見られないのは**別スレッドで発火するログ**だけだが、18 箇所の内訳を全数確認したところ、包んでいるのは同期 `handle_message`（15）・`CueSink::emit`（2＝`actor_dispatch_tests.rs:75,387`・いずれもテストスレッド上の同期呼び出し）・`sink.send_tick`（1）で、**`spawn_seriko` を包んでいる呼び出しは 0 件**。したがって「ログが出ないこと」を主張する assert（`matches("level=WARN").count() == 0` 等）が捕捉漏れで空虚に真になる経路は存在しない。この制約は元のテストモジュール自身がバナーで明記しており、移設後は `actor_dispatch_tests.rs:140-151` に同文で残っている |
| 2 | `handle_message` の戻り値 `ControlFlow` を受け取らない呼び出し 4 件＝`unused_must_use` 警告 4 件 | `crates/areka-seriko/src/actor_bind_loop_tests.rs:147,156,211,219`（移設前は `actor.rs:1498,1507,1562,1570`） | **既存・記録のみ（是正しない）**。移設前から同一の 4 件で、`before_build_warnings.txt` が `areka-seriko (lib test) generated 4 warnings` として基準値に織り込んでいるものと同一である。本 spec が縛るのは件数の非増加（要件 2.6）だけであり、`let _ =` の付与はテスト本文の変更＝要件 2.4 違反になるため行わない |
| 3 | `static` / `thread_local!` / `std::env::set_var` / `unsafe` / `sleep` / `#[ignore]` / `#[should_panic]` / `OnceLock` の使用 | 担当 7 テストファイル全域 | **0 件**（全走査で確認。`static` の 7 件の文字列一致はすべて `'static` 境界かコメント内の「static = {1100, 1207}」表記、`sleep` の 6 件はすべて「sleep 不使用」と書いたコメント）。プロセスグローバルな状態に触れるテストは 1 件も無い |
| 4 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | — | **発生せず**。必要だったのは共有ヘルパへの `pub(super)` 付与（8 項目）と各テーマファイルの `use` ヘッダの絞り込みのみ。入れ子 `tick_loop_tests` の `use super::*;` も無調整で解決した |

### 16.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-seriko/src/actor_test_support.rs` | 新規（84 行・共有ヘルパ 6 本） |
| `crates/areka-seriko/src/actor_dispatch_tests.rs` | 新規（840 行・19 テスト） |
| `crates/areka-seriko/src/actor_bind_loop_tests.rs` | 新規（928 行・15 テスト。うち入れ子 `tick_loop_tests` 4） |
| `crates/areka-seriko/src/state_test_support.rs` | 新規（10 行・共有ヘルパ 2 本） |
| `crates/areka-seriko/src/state_surface_tests.rs` | 新規（483 行・22 テスト） |
| `crates/areka-seriko/src/state_bind_pattern_tests.rs` | 新規（566 行・23 テスト） |
| `crates/areka-seriko/src/looper_tests.rs` | 新規（561 行・18 テスト） |
| 上記に対応する本番 3 ファイル | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体は無変更） |
| `verification/mapping/areka-seriko.csv` | 新規（79 行・全単射検証済み） |
| `verification/notes.md` | 本節（§16）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-seriko`）とする。
