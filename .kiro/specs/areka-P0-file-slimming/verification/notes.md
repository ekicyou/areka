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
3. **序数（ordinal）比較で整列**する。**必ず `[string[]]` に型付けしてから** `[Array]::Sort($arr, [System.StringComparer]::Ordinal)` を用いる:

   ```powershell
   $arr = [string[]]@(Get-Content $raw | Where-Object { $_ -match ': test$' })
   [Array]::Sort($arr, [System.StringComparer]::Ordinal)
   ```

   PowerShell の `Sort-Object` 既定はカルチャ依存比較でありアンダースコアと英字の順序が変わるため使わない。

   > **`[string[]]` の型付けを省くと黙ってカルチャ順になる**（タスク 3.5 のレビューで根本原因を特定）。
   > パイプライン出力（`… | Where-Object …`）の各要素は `PSObject` に包まれており、
   > `object[]` のまま `[Array]::Sort` へ渡すと `StringComparer.Ordinal` の内部 `as string` キャストが失敗して
   > **既定のカルチャ比較器へ静かにフォールバックする**。`$arr[0].GetType()` は `System.String` を返すので
   > 目視では気づけない。本 spec の採取はすべてパイプライン出力なので、この罠は常に踏みうる。
   >
   > 実測（同一配列を 1 コマンド内で 2 通りに整列・4,790 行）:
   > 序数 `8468B087…C0E9` ／ `Sort-Object` `9C75E1B7…F20F` ／ **1,806 位置が相違・多重集合は同一**。
   > 分岐点は index 179 の `bake::tests::…`（序数が先）と `bake_entry_tests::…`（カルチャが先）＝`::`(0x3A) 対 `_`(0x5F)。
   >
   > **較正の落とし穴**: コミット済み `before_default.txt` をハッシュして基準値と一致しても、
   > あのファイルは既に整列済みなので**整列器そのものは 1 度も動いていない**——符号化と改行しか検証できない。
   > 較正するなら**未整列の生出力を 2 通りに整列して digest が割れること**を確かめること。
4. UTF-8（BOM 無し）で書き出す。
5. **改行は CRLF・末尾に改行 1 つ**（`[IO.File]::WriteAllLines($path, $arr, (New-Object Text.UTF8Encoding($false)))` が既定でこの形になる）。

> **手順 5 を後から追加した理由（タスク 3.3 レビュー）**: 当初この手順は改行形式を固定していなかったため、
> 各タスクが記録した中間リストの SHA256 が担当者ごとに再現しなかった（BOM 有無・CRLF/LF・末尾改行の有無で
> 4 通りに割れる）。**判定そのものは `Compare-TestLists.ps1` の行単位比較で行っており影響を受けない**が、
> 記録した値が第三者に再現できないのは要件 2.3 の趣旨に反する。以後、リストの SHA256 を証跡に載せる場合は
> 必ず手順 5 の形で採ること。タスク 3.2 §16 と 3.3 §17 に載る中間リストのハッシュは
> **この規定より前に採られた参考値**であり、正規の witness は
> コミット済み `before_default.txt`（4,790 行・SHA256 `77F03656B507D72DB4A5D9E5D75DC4849C16A92B6E371F718F8887F7EB43D2AD`）
> との `Compare-TestLists.ps1` 照合結果そのものである。

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

## 17. クレート単位のテスト分離とテーマ分割: `areka-sakura`（タスク 3.3・要件 1.1 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `2048941`（タスク 3.2 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 下表の本番 2 ファイル＋新規テストファイル 7 本＋`verification/mapping/areka-sakura.csv`＋本ファイル以外には一切触れていない（`Cargo.toml`・他クレート・spec 本体ドキュメントは無変更）

### 17.1 移設した 2 ファイル（design §File Structure Plan の `crates/areka-sakura` と完全一致）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 新テストファイル | 新ファイル行 | 本番 残行 |
|---|---:|---|---|---|---:|---:|
| `src/drive.rs` | 2,808 | `tests`（531-2808・テストコード 2,278） | **テーマ分割 ×3 ＋共有ヘルパ** | `drive_test_support.rs` ／ `drive_delivery_tests.rs` ／ `drive_lifecycle_tests.rs` ／ `drive_choice_tests.rs` | 136 / 837 / 672 / 654 | 542 |
| `src/compile.rs` | 1,867 | `tests`（322-1867・テストコード 1,546） | **テーマ分割 ×2 ＋共有ヘルパ** | `compile_test_support.rs` ／ `compile_arm_tests.rs` ／ `compile_sheet_tests.rs` | 57 / 891 / 602 | 330 |

- 2 ファイルとも**テストモジュールは 1 個・ファイル末尾に連続配置**（`scan_raw.csv:168,170` の実測どおり・ズレ 0）。モジュール名はいずれも `tests`。
- **新テストファイル 7 本はすべて 1,000 行以下**（最大 `compile_arm_tests.rs` の 891 行）。僅少超過で単一維持したファイルは **0 件**（§7.4 への追記は不要）。
- 2 ファイルとも非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は 1 件も存在しない（`scan_raw.csv` の当該 2 行の `nonmod_count` が 0）。
- `#[cfg(test)]` 行は §13〜§16 と同じく**元位置に据え置き**、テーマ分割で増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` の挿入は drive 11・compile 8 の計 19 行）。
- 同クレートの残る 5 本（`sysvar.rs` 68／`duration.rs` 50／`error.rs` 46／`contract.rs` 0／`lib.rs` 0）はテストコード 500 行以下ゆえ要件 1.5 で非必須・設計判断 #10 により任意移設は行わない（無変更）。

接続宣言（7 モジュールとも同一文言・design §移設方式の裁定 案 C）:

    #[cfg(test)]
    #[path = "<stem>_<モジュール名>.rs"]
    mod <モジュール名>;

### 17.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

**タスク文の「スクリプト解釈と実行駆動という 2 つの関心」ヒントは、ファイル境界の説明としては当たっているが、
ファイル内のテーマ境界は与えない。** 実測すると解釈＝`compile.rs`・駆動＝`drive.rs` と**クレート内の 2 ファイルに
そのまま 1 対 1 対応**しており（`compile.rs` の公開項目は `compile` / `append_epilogue` / `CompiledTalk` のみ、
`drive.rs` の `pub` 項目は `spawn_talk` 1 本のみ——`TalkHandle` は `contract.rs` の定義を戻り値型として
私有 `use` で取り込んでいるだけ）、ヒントに従うだけでは 2,278 行と 1,546 行のテストファイルが
2 本残る。よって**ファイル内の実シームはヒントの外に求めた**——採ったのは**本番 API の継ぎ目**である。

**`drive.rs`** — 本番は talk アクター 1 本で、`TalkDriver::handle` が `SakuraMsg` を 4 腕へ配る
（`on_start` :186 ／ `on_tick` :246（＋`settle_after_tick` :312・`notify_choice_waiting_if_newly_waiting` :355）／
`on_close` :407 ／ `on_resolve_choice` :447、終端は `send_done` :509 / `send_interrupted` :517。行番号は移設前）。
テストモジュール内部には**バナー区画が 5 本**ある（1622 = task 7.2 完了 horizon ／ 1898 = task 5.2 ResolveChoice ／
2202 = task 10.1 配送列 ／ 2343 = task 10.3 未知コマンド名 ／ 2465 = task 3.2 ChoiceWaiting）。
この 5 区画はいずれも上記 4 腕のどれか 1 つに閉じており、**1 本も跨がずに束ねられる**。
残る先頭群（644-1620・バナー無しの 16 テスト）だけが 2 テーマに割れるが、ここはバナーが 1 本も無いため
区画を割ってはいない。

| 新モジュール（ファイル） | 移設前の行範囲 | 束ねたバナー区画 | 対象の本番項目 | テスト数 |
|---|---|---|---|---:|
| `test_support`（`drive_test_support.rs`） | 540-642 ／ 1622-1641 | task 7.2 バナー（`NEG_WINDOW` に付属） | —（3 テーマから参照される 17 ヘルパ項目） | 0 |
| `delivery_tests`（`drive_delivery_tests.rs`） | 682-996 ／ 1179-1366 ／ 1560-1620 ／ 2202-2463 | task 10.1・task 10.3 | `on_tick` の配送（アンカー刻印・未 due の保留・同一 at の FIFO・broadcast fan-out・envelope duration・未知コマンドキャリアの素通し） | 10 |
| `lifecycle_tests`（`drive_lifecycle_tests.rs`） | 644-680 ／ 998-1177 ／ 1368-1558 ／ 1643-1896 | task 7.2 | `on_start`（空 sheet 即完了・二重 Start ガード）・`on_close`（中断 ACK・自然終端後の Close）・`send_done`（占有 horizon gated の発火時刻・talk_id エコー・受信端 drop 耐性）＝**`TalkDone` を誰に・いつ・高々 1 回返すか** | 12 |
| `choice_tests`（`drive_choice_tests.rs`） | 1898-2200 ／ 2465-2807 | task 5.2・task 3.2 | `on_resolve_choice`（barrier 停止・解決・不一致 id・Armed 誤投函）と `notify_choice_waiting_if_newly_waiting`（`ChoiceWaiting` の通算 1 回性） | 10 |

**ヘルパ参照関係による裏取り**（全 17 ヘルパ項目の参照行を全数走査）: `TalkNotice`（＋`From` 実装 2 本）・
`recv_done`・`RecordingSink`（＋実装 2 本）・`NoopSink`（＋実装）・`two_sinks`・`ChannelSink`（＋実装）・
`commands`・`NEG_WINDOW` の 17 項目は**すべて 2 テーマ以上から参照される**（`ChannelSink` は delivery と choice、
`commands` は delivery と lifecycle、`NEG_WINDOW` は lifecycle と choice、残りは 3 テーマ全部）。
逆に `MENU_SCRIPT`・`drive_menu_to_barrier`・`menu_relative_horizon` の 3 項目は choice 専用で
（参照行 1938/1982/2024/2072/2086/2478/2499/2519/2660/2726・1948/1992/2034/2111/2510/2670/2739・2525/2681）、
同ファイルに残した。この分布が上記の境界を裏づける。

**task 7.2 バナーが `test_support` へ移った理由**: 本文一致検証は項目の直前コメントを（空行を挟んでも）当該項目の
本文の一部として比較する。移設前の 1622-1641 は「バナー 17 行＋空行＋doc 1 行＋`const NEG_WINDOW`」で**1 項目**であり、
`NEG_WINDOW` が lifecycle と choice の両方から参照される以上、集約先は `test_support` しかない（複製は `ITEM-EXTRA`）。
バナー本文は「負の窓」という `NEG_WINDOW` そのものの技法説明なので、帰属としても不自然ではない。文言は 1 文字も変えていない。

**`compile.rs`** — 本番は `compile`（`Instruction` 列の走査転写）と `append_epilogue`（末尾 carrier 付加・純関数）の
2 公開関数だけで、テストは 43 本。バナーは 6 本（839 = task 5.2 D 焼き込み ／ 1016 = task 4.1 Choice/Cursor ／
1139 = task 4.2 Move/GenericCommand/SystemVar＋barrier ／ 1401 = task 4.4 決定論 ／ 1687 = task 4.2 append_epilogue）。
`append_epilogue` 単独ではテスト 180 行にしかならず 2 テーマの一方にはならないため、
`compile` 側を**「個々の命令アームが cue へどう写るか」**と**「台本（`CueSheet`）全体としての性質」**へ割り、
`append_epilogue`（末尾 horizon・同時刻 barrier の後ろへの安定挿入）を後者に含めた。バナー区画は 1 本も跨いでいない。

| 新モジュール（ファイル） | 移設前の行範囲 | 束ねたバナー区画 | 対象の本番項目 | テスト数 |
|---|---|---|---|---:|
| `test_support`（`compile_test_support.rs`） | 330-383 | — | —（両テーマから参照される 4 ヘルパ） | 0 |
| `arm_tests`（`compile_arm_tests.rs`） | 401-469 ／ 555-590 ／ 603-663 ／ 721-765 ／ 1016-1399 ／ 1401-1685 | task 4.1・task 4.2（アーム＋barrier）・task 4.4 | `compile` の `match instruction` 各腕（`Surface`／`BalloonSurface`／`SpeakerScope`／`NewLine`／`Clear`／`Choice`／`Cursor`／`Move`／`GenericCommand`／`SystemVar`／catch-all）の不透明転写と、走査後に付く選択待ち barrier。task 4.4 はその同じ腕（メニュー・キャリア・sysvar）の包括的な決定論固定 | 23 |
| `sheet_tests`（`compile_sheet_tests.rs`） | 385-399 ／ 471-553 ／ 592-601 ／ 665-719 ／ 767-837 ／ 839-1014 ／ 1687-1866 | task 5.2・task 4.2（append_epilogue） | 命令を跨いで決まる台本全体の性質——`offset`／`duration` の累積（先頭待ちの保存・単調増加・D 焼き込み・`Wait` の第一級化）・`ClearAll` 前置・`End`/`Quit` の切詰めと `TalkEndReason`・`start_time` の有限非負非減少・同一入力の決定性、および `append_epilogue` | 20 |

**ヘルパ参照関係による裏取り**: `compile`（1 引数ブリッジ）・`command_of`・`cue_eq`・`assert_clear_all_prefix_and_rest` の
4 本は両テーマから参照される（`cue_eq` は arm 側 1499・sheet 側 781/1720）ため `test_support` へ集約。
`representative_instructions`（参照 771 のみ）は sheet 専用、`barrier_of`（参照 1106/1315/1347/1395/1529）は arm 専用、
`relative_horizon`（参照 1755/1782/1808/1831/1839）は sheet 専用ゆえ、それぞれ当該テーマファイルに残した。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: `drive_test_support.rs` の 11 箇所
（`TalkNotice`・`recv_done`・`RecordingSink`＋`new`／`records`・`NoopSink`・`two_sinks`・`ChannelSink`＋フィールド `tx`・
`commands`・`NEG_WINDOW`）と `compile_test_support.rs` の 4 関数へ `pub(super)` を付与した（付与のみ・本文は無変更）。
`ChannelSink.tx` はテーマ側がリテラル構築するためフィールドにも付与が要る。複製は 1 件も作っていない。

**バナーの帰属**: 本文一致検証は項目の直前コメントを当該項目の本文の一部として比較する。したがってバナーは
**元と同じ項目へ付属したまま**移した。`compile.rs` の task 4.2 append_epilogue バナー（1687-1688）は移設前も
`use areka_talk::EpilogueCommand;` に付属する `use` 項目の一部（＝比較対象外）だったため、移設後も同じ `use` の
直前に置いて帰属を変えていない。他の 9 本（drive 5・compile 4）はいずれも直後の項目（テスト関数・`const`・ヘルパ）に
付属したまま移動しており、文言は 1 文字も変えていない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 各テーマファイルの先頭に移設元の `use` 群を置き、そのファイルで実際に使う
項目だけへ絞った（絞る前のビルドで `unused_imports` が 17 件出て要件 2.6 に反したため）。落としたのは
`compile_test_support`＝`text_playback_duration`／`{NewLineRatio, SurfaceArg}`／`Duration`、
`compile_arm_tests`＝`Duration`、`compile_sheet_tests`＝`SystemVarSnapshot`、
`drive_test_support`＝`TalkId`／`text_playback_duration`／`TryRecvError`、
`drive_delivery_tests`＝`RecvTimeoutError`／`Instant`、`drive_lifecycle_tests`＝`TalkCue`／`RecvTimeoutError`／
`TryRecvError`／`{Arc, Mutex}`／`Instant`、`drive_choice_tests`＝`TryRecvError`／`{Arc, Mutex}`／`Instant`。
`use` 項目は §11.4 のとおり本文一致検証の対象外である。**可視性・`use` 以外の調整は 1 件も必要なかった。**

### 17.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 2 ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ
（行コメント・ブロックコメント（入れ子）・通常文字列・raw 文字列（`#` の数）・バイト文字列・
文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と
「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は、既知の唯一の該当箇所
`crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691` を**盲点 1 件**として検出し、同ファイルの
`:382`・`:429`・`:614`・`:690` を **`\` 継続 4 件**として正しく切り分けることで確認した
（実測出力: `継続行 5 件: 382,429,614,690,691 / 盲点該当 1 件: 691`）。

| ファイル | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---:|---:|
| `crates/areka-sakura/src/drive.rs`（移設前） | 1（`:2538`） | **0** |
| `crates/areka-sakura/src/compile.rs`（移設前） | 0 | **0** |
| 新テストファイル 7 本（移設後） | 1（`drive_choice_tests.rs:385`） | **0** |

**結論: 担当 2 ファイルに複数行文字列リテラルは 1 箇所しかなく、それは `\` 継続（`drive.rs:2537` の行末が `\`）である。**
Rust は `\` 改行に続く行頭空白を除去するため、一律 4 スペース de-indent はリテラルの中身を変えない。
念のため当該 2 行を目視で突合した——移設前 `drive.rs:2537-2538` と移設後 `drive_choice_tests.rs:384-385` は
**「移設前の行から先頭 4 文字を除いたものと移設後の行がバイト同値」**であることをプログラム比較で確認済み（両行とも True）。
§11.4 第 1 の盲点の該当行は **0 件**であり、例外処理は不要だった。

### 17.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 2048941 -OriginalPath <本番> -RelocatedPath "<新テスト群>" -Detail`

| 対象 | 出力 | exit |
|---|---|---:|
| `drive.rs` → 4 ファイル | `MATCH: test fn 32=32 / helper item 17=17 / mod block 1 / files 4` | 0 |
| `compile.rs` → 3 ファイル | `MATCH: test fn 43=43 / helper item 7=7 / mod block 1 / files 3` | 0 |

**(a2) 行単位の多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、
それとは独立に「移設前ブロック本体を一律 4 スペース de-indent した行の多重集合」と「新テストファイル群の行の多重集合」を
（空行を除いて）突合した。差分はすべて要件 2.4 が許容する調整（可視性付与・`use`）だけであり、
テスト本文の行は 1 行も増減・改変していない。

| 対象 | 元 | 新 | 消えた行 | 増えた行 | 内訳 |
|---|---:|---:|---:|---:|---|
| `drive.rs` | 2,088 | 2,106 | 12 | 30 | 消 = `pub(super)` を付ける前の 11 行＋落とした `use` 1 本 ／ 増 = `pub(super)` 付き 11 行＋新設 `use` 19 本 |
| `compile.rs` | 1,449 | 1,456 | 4 | 11 | 消 = `pub(super)` 前の 4 行 ／ 増 = `pub(super)` 付き 4 行＋新設 `use` 7 本 |

de-indent の分類（移設した全行）: **(a) ちょうど −4 スペース**または**(b) バイト同値（空行）**のいずれか。
これ以外に分類される行は **0 件**（検出したら停止する設計で、全 7 ファイルとも検出 0）。
内訳は `drive_test_support` 114/3・`drive_delivery_tests` 765/55・`drive_lifecycle_tests` 608/46・
`drive_choice_tests` 595/40・`compile_test_support` 51/0・`compile_arm_tests` 836/26・
`compile_sheet_tests` 557/17（形式: −4 スペース行 / 空行）。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-sakura.csv`

    PASS: 全単射 OK / 行数 75 / 相異なる old_fqn 75 / 相異なる new_fqn 75 / フラグメント 1
      - areka-sakura.csv: 75 行

exit 0。75 行の内訳は `compile::tests::*` → `compile::arm_tests::*` 23 行／`compile::tests::*` → `compile::sheet_tests::*` 20 行／
`drive::tests::*` → `drive::delivery_tests::*` 10 行／`drive::tests::*` → `drive::lifecycle_tests::*` 12 行／
`drive::tests::*` → `drive::choice_tests::*` 10 行。`reason` は全行 `theme_split`、末尾セグメント（関数識別子）は旧新で同一。
cargo が実際に印字する完全修飾名にクレート名の接頭辞は付かない（`before_default.txt` の実データで確認済み——
`compile::tests::` 43 行・`drive::tests::` 32 行が移設前に実在する）。
既存フラグメントとの結合検証（`-Path $V/mapping`）も
`PASS: 全単射 OK / 行数 232 / 相異なる old_fqn 232 / 相異なる new_fqn 232 / フラグメント 3` で exit 0（キー衝突なし）。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list` → `: test$` 抽出 →
`[Array]::Sort($arr, [System.StringComparer]::Ordinal)` → UTF-8 BOM 無し・重複行を残す）でリストを採取し、
コミット済み `before_default.txt` と**タスク 3.1・3.2・3.3 の 3 フラグメント全部**を渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after_default_task33.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 232 行 (1 ファイル) / 適用 232 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 232 行・未使用 0 行**。移設後リストの SHA256 は
`63994B62B8F9CBA31EF7CF6DDF07C20DD6098A8D2D183590C0976B7BF03C0E75`
（§16 と同じく中間リストファイル自体はコミットしない。再現手順は §10.2 のとおり）。
整列は `Sort-Object` を使わず `[System.StringComparer]::Ordinal` で行った（§11.8）。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、
対応表を**使わない**多重集合の対称差で確かめた。

| 検査 | 結果 |
|---|---|
| `before_default.txt` と移設後リストの対称差（対応表なし）: 消えた行 / 現れた行 / 全フラグメント行数 | **232 / 232 / 232**（三者一致） |
| タスク 3.1・3.2 のフラグメントだけを `before_default.txt` へ適用して復元した「本タスク着手直前のリスト」と移設後リストの対称差: 消えた行 / 現れた行 / 本タスクの対応表行数 | **75 / 75 / 75**（三者一致） |
| 消えた行がすべて `old_fqn` に在るか | **True** |
| 現れた行がすべて `new_fqn` に在るか | **True** |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |
| `old_fqn` と `new_fqn` が同一の行（＝変わっていない名前を載せた水増し） | **0** |
| 末尾セグメント（関数識別子）が旧新で相違する行 | **0** |
| `reason` が `theme_split` 以外の行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。

移設後リストのモジュール別内訳（`cargo test -p areka-sakura -- --list`・本クレート 88）:
`compile::arm_tests` 23 ／ `compile::sheet_tests` 20 ／ `drive::delivery_tests` 10 ／ `drive::lifecycle_tests` 12 ／
`drive::choice_tests` 10 ／ `duration::tests` 5 ／ `error::tests` 3 ／ `sysvar::tests` 5。
移設前の `compile::tests` 43・`drive::tests` 32・`duration::tests` 5・`error::tests` 3・`sysvar::tests` 5 と本数が一致する。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-sakura --no-fail-fast` → **exit 0**。
**88 passed / 0 failed / 0 ignored**（lib 88 ＋ 統合 0 ＋ doctest 0）。
移設前の独立導出値と一致する: 移設前コミット `2048941` の `git show` に対する `#[test]` 属性行の全数は
`compile.rs` 43・`drive.rs` 32 で、`before_default.txt` の `compile::tests::` 43 行・`drive::tests::` 32 行と一致し、
これに `duration`5・`error`3・`sysvar`5 を足した 88 が移設後の実測値と完全一致する。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-sakura --all-targets` → exit 0・**警告行 0 件**
（`warning:` で始まる行が 1 行も出ない）。`before_build_warnings.txt` の `[PER-UNIT TALLY]` は本クレートに
1 件も割り当てていない（基準値 0）ので、増加ゼロ。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。ユニット別 generated 件数（7 ユニット）も多重集合として一致し、
`areka-sakura` のサマリ行は移設前後とも 0 件である。

**(f) 本番本体の無変更** — 移設前コミット `2048941` の各本番ファイルの先頭〜旧 `#[cfg(test)]` 行の直前までを
現作業ツリーと逐行突合し、**2 ファイルとも不一致 0**（drive 1-530 ／ compile 1-321）。

| ファイル | `git diff --numstat` |
|---|---|
| `drive.rs` | 挿入 11 ／ 削除 2,277 |
| `compile.rs` | 挿入 8 ／ 削除 1,545 |

2 ファイル合計 `2 files changed, 19 insertions(+), 3822 deletions(-)`。挿入 19 = 新設した接続宣言のうち
元位置に据え置いた `#[cfg(test)]` 行 2 本を除いた行数である。

**(g) 完了状態の直接確認** — 2 本番ファイルに残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言のみ:
`drive.rs:531-542` ／ `compile.rs:322-330`。テストモジュール本体は 1 行も残っていない。

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 10 パスのみ:
変更 2 本（本番ファイル）＋未追跡 8 本（新テストファイル 7 本＋`verification/mapping/areka-sakura.csv`）。
`crates/**` の他ファイル・`Cargo.toml` への差分は 0 件。

### 17.5 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（`areka-sakura` の 2 ファイル・75 テスト・テストコード 3,824 行）では、修正を要する
壊れたテスト・不正なテスト・テスト間の状態汚染は 1 件も発見しなかった。所有 spec への送付所見は 0 件である。**

以下は「調べたが問題なし／既存のまま据え置き」と確定した記録（次に触る者が同じ調査を繰り返さないための控え。是正は行わない）:

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | `static` / `thread_local!` / `std::env::set_var` / `unsafe` / `std::thread::sleep` / `#[ignore]` / `#[should_panic]` / `OnceLock` の使用 | 担当 7 テストファイル全域 | **0 件**（全走査で確認。`sleep` の 1 件の文字列一致は `drive_delivery_tests.rs:178` の「実時計・sleep 非依存」と書いたコメント）。プロセスグローバルな状態に触れるテストは 1 件も無い。`drive` 系は talk ごとに独立スレッド＋独立 mpsc チャンネルを張るだけで、共有状態は持たない |
| 2 | 実時計に依存する観測窓（`recv_timeout`） | `drive_test_support.rs:38-50`（`recv_done` の deadline）・`drive_test_support.rs:137`（`NEG_WINDOW = 200ms`）・`drive_lifecycle_tests.rs` と `drive_choice_tests.rs` の負の窓 9 箇所 | **問題なし・記録のみ**。正常系では「送信が起きない」ことを 200ms の timeout で示し、バグ系では窓内に必ず届く両方向決定的な設計であることを、移設前のバナーが明記している（そのバナーは `drive_test_support.rs:118-134` に同文で残っている）。5 秒側の `recv_timeout(...).unwrap_err()` 2 箇所（`drive_choice_tests.rs:558,650`）は `handle.actor.join()` 済みで送信端が drop されているため `Disconnected` が即返り、5 秒待つことはない（クレート全体の実行時間 0.41 秒が裏づけ） |
| 3 | 有界スピン待ち（`for _ in 0..1000` ＋ `std::thread::yield_now()`） | `drive_delivery_tests.rs:114-128`（`undue_cues_are_withheld_until_their_at_is_reached`） | **既存・記録のみ（是正しない）**。反復上限が壁時計でなく回数で決まっているため、極端に負荷の高いマシンでは理論上 1,000 回のスピンで cue 着弾を取り逃す余地がある（取り逃すと `assert!(wait_seen, ...)` が落ちる＝偽陽性の赤）。移設前から同一コードであり、テスト本文の変更は要件 2.4 違反になるため触らない。同一ファイルの前後の barrier は `recv_timeout(5s)` でブロックしており、この 1 箇所だけが `try_recv` スピンである |
| 4 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | `compile_arm_tests.rs:5` ／ `compile_sheet_tests.rs:4` | **1 件発生・接続側で解決済み（テストロジックは無変更）**。`compile.rs` のテストモジュールは本番 `compile` と同名の 1 引数ブリッジ関数を定義しており、移設前は「グロブ `use super::*;` を同一モジュール内の明示定義が shadow する」ことで解決していた。テーマ分割でブリッジを `test_support` へ出すと、`use super::*;`（本番 `compile`）と `use super::test_support::*;`（ブリッジ）の**グロブ同士が衝突**して E0659（曖昧）＋E0061（引数不足）が 98 件出る。明示 import（`use super::test_support::{assert_clear_all_prefix_and_rest, command_of, compile, cue_eq};`）はグロブより優先されるため、これで解決した。**同名 shadow ヘルパを持つテストモジュールをテーマ分割する後続タスク（`areka` / `areka-emo-text` / `areka-kanade` 等）は同じ罠を踏むので、共有ヘルパは明示 import で受けること。** `drive` 側は本番と同名のヘルパが無いためグロブのままで足りている |

### 17.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-sakura/src/drive_test_support.rs` | 新規（136 行・共有ヘルパ 17 項目） |
| `crates/areka-sakura/src/drive_delivery_tests.rs` | 新規（837 行・10 テスト） |
| `crates/areka-sakura/src/drive_lifecycle_tests.rs` | 新規（672 行・12 テスト） |
| `crates/areka-sakura/src/drive_choice_tests.rs` | 新規（654 行・10 テスト） |
| `crates/areka-sakura/src/compile_test_support.rs` | 新規（57 行・共有ヘルパ 4 本） |
| `crates/areka-sakura/src/compile_arm_tests.rs` | 新規（891 行・23 テスト） |
| `crates/areka-sakura/src/compile_sheet_tests.rs` | 新規（602 行・20 テスト） |
| 上記に対応する本番 2 ファイル | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体は無変更） |
| `verification/mapping/areka-sakura.csv` | 新規（75 行・全単射検証済み） |
| `verification/notes.md` | 本節（§17）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-sakura`）とする。

## 18. クレート単位のテスト分離とテーマ分割: `areka-ghost`（ライブラリ側）（タスク 3.4・要件 1.1 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `470164a`（タスク 3.3 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 下表の本番 3 ファイル＋新規テストファイル 5 本＋`verification/mapping/areka-ghost.csv`＋本ファイル以外には一切触れていない。**`crates/areka-ghost/tests/**`（統合テストツリー・`spine_e2e_test.rs` を含む）はタスク 3.5 の領分であり本タスクでは 1 行も変更していない**（`git status --porcelain -uall` で確認済み）

### 18.1 移設した 3 ファイル（design §File Structure Plan の `crates/areka-ghost` の `src/` 3 本と完全一致）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 新テストファイル | 新ファイル行 | 本番 残行 |
|---|---:|---|---|---|---:|---:|
| `src/dispatcher.rs` | 1,856 | `tests`（421-1856・テストコード 1,436） | **テーマ分割 ×2 ＋共有ヘルパ** | `dispatcher_test_support.rs` ／ `dispatcher_slot_tests.rs` ／ `dispatcher_choice_tests.rs` | 63 / 613 / 767 | 429 |
| `src/runtime.rs` | 1,613 | `tests`（651-1613・テストコード 963） | 単純移設 | `runtime_tests.rs` | 960 | 653 |
| `src/ticker.rs` | 823 | `tests`（320-823・テストコード 504） | 単純移設 | `ticker_tests.rs` | 501 | 322 |

- 3 ファイルとも**テストモジュールは 1 個・ファイル末尾に連続配置**（`scan_raw.csv:77,81,87` の実測どおり・ズレ 0）。モジュール名はいずれも `tests`。
- **新テストファイル 5 本はすべて 1,000 行以下**（最大 `runtime_tests.rs` の 960 行）。僅少超過で単一維持したファイルは **0 件**（§7.4 への追記は不要）。
- 3 ファイルとも非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は 1 件も存在しない（`scan_raw.csv` の当該 3 行の `nonmod_count` が 0）。テストモジュールに付いた `///` doc コメントも 0 件（Implementation Notes の §14.3 規則は本タスクでは発動しない）。
- `#[cfg(test)]` 行は §13〜§17 と同じく**元位置に据え置き**、増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` の挿入は dispatcher 8・runtime 2・ticker 2 の計 12 行）。
- 同クレートの残る `src/` 8 本（`shiori_inproc.rs` 446／`prop_sink.rs` 407／`sink.rs` 284／`sylphya_wiring.rs` 189／`config.rs` 134／`relay.rs` 115／`shiori_wiring.rs` 89／`lib.rs`・`test_log_capture.rs` 0）はテストコード 500 行以下ゆえ要件 1.5 で非必須・設計判断 #10 により任意移設は行わない（無変更）。
- `crates/areka-ghost/tests/ghost/spine_e2e_test.rs`（2,091 行・10 モジュール）は**タスク 3.5 の担当**。本タスクでは触っていない。

接続宣言（5 モジュールとも同一文言・design §移設方式の裁定 案 C）:

    #[cfg(test)]
    #[path = "<stem>_<モジュール名>.rs"]
    mod <モジュール名>;

### 18.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

**`dispatcher.rs`** — テーマ分割が要るのは本ファイルのみ（他 2 本は 1,000 行未満で単純移設）。

タスク 3.1〜3.3 ではコメントバナーが「作業時系列の見出し」で thematic でなかったため本番 API の継ぎ目へ退避したが、
**本ファイルではバナーが唯一 1 本しかなく、しかもそれが本番 API の継ぎ目とちょうど一致する**。
移設前 `dispatcher.rs:1097-1107` の
`// ── task 3.3: 選択系 3 アームの中継意味論と時刻換算（design C9・DD-9/DD-11・R1.3/5.5/7.2/7.5） ──`
がそれで、区画の切れ目は `DispatcherState::handle`（`:122`）が配る 7 アームのうち
**選択系 3 アーム**（`on_resolve_choice` `:154` ／ `on_cancel_choice` `:194` ／ `on_choice_waiting` `:228`）と
**それ以外**（`on_start` `:282` ／ `on_done` `:314` ／ `on_tick` `:337` ／ `on_close` `:358` ／
`close_active_if_any` `:365`、および inbox 境界の `From` 実装 5 本 `:51-95`）の境目に一致する。
よって本ファイルに限りバナー由来の境界と本番 API シーム由来の境界が同一であり、両者は互いの裏取りになっている。
（行番号はすべて移設前 `470164a`。）

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support`（`dispatcher_test_support.rs`） | 431-487 | —（2 テーマから参照される 3 ヘルパ項目＋その impl 2 本） | 0 |
| `slot_tests`（`dispatcher_slot_tests.rs`） | 489-1096 | inbox 境界の `From` 変換（`TalkCommand`／`ChoiceWaiting` の無損失写像）と、単一 slot の運行——差し替え（`on_start`＋`close_active_if_any`）・stale `Done` 棄却（`on_done`）・停止時の内側 join（`on_close`）・経過秒換算の Tick 中継（`on_tick`）・完了転送と slot 解放・`system_vars` provider の per-talk 凍結 | 8 |
| `choice_tests`（`dispatcher_choice_tests.rs`） | 1097-1855 | 選択系 3 アーム——`on_resolve_choice`（一致中継・不一致棄却・送出失敗継続）・`on_cancel_choice`（Close 転送＋slot 維持・不一致棄却・送出失敗継続）・`on_choice_waiting`（ms 換算・不一致棄却・`base_now` 未確定の warn 防御・転送失敗継続）と実 talk e2e 2 本 | 12 |

**ヘルパ参照関係による裏取り**（テストモジュール内の全ヘルパ項目 16 件の参照行を全数走査。行番号は移設前）:

- **2 テーマから参照される 3 項目**（→ `test_support` へ集約）:
  `test_system_vars`（定義 `:436`／参照 slot 側 `:593,671,752,791,893`・choice 側 `:1158,1707,1799`）・
  `run_bounded`（定義 `:446`／参照 slot 側 `:648,727,765,868,975,1086`・choice 側 `:1774,1846`）・
  `RecordingSink`（定義 `:462`＋`impl RecordingSink :466`＋`impl CueSink for RecordingSink :480`／
  参照 slot 側 `:587,588,665,666,746,747,785,886,887,1009`・choice 側 `:1704,1705,1796,1797`）。
- **slot 専用 1 項目**（同ファイルに残置）: `ChannelSink`（定義 `:492`＋`impl CueSink :496`／参照 `:786,1010` の 2 箇所とも slot 側）。
- **choice 専用 9 項目**（同ファイルに残置）: `spawn_probe_talk`（`:1112`）・`spawn_vanished_talk`（`:1130`）・
  `StateFixture`（`:1143`）・`state_fixture`（`:1150`）・`impl StateFixture`（`:1166`・`feed`）・`occupy`（`:1180`）・
  `release`（`:1189`）・`MENU_SCRIPT`（定義 `:1200`／参照 `:1714,1806`）・
  `relay_choice_waiting`（定義 `:1460`／参照 `:1504,1536`）。参照行はいずれも `:1097` 以降にしか現れない。

この分布は上記の境界と完全に整合する（境界を跨ぐヘルパは 3 件だけで、それらは `test_support` に集約した）。

**バナーの帰属**: 本文一致検証は項目の直前コメントを（空行を挟んでも）当該項目の本文の一部として比較する。
唯一のバナー `:1097-1107` は移設前も直後の `spawn_probe_talk`（choice 専用ヘルパ）に付属する本文の一部であり、
移設後も `dispatcher_choice_tests.rs` の同じ位置（同ファイル先頭の `use` ヘッダ直後）にそのまま置いた。文言は 1 文字も変えていない。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: `dispatcher_test_support.rs` の 5 箇所
（`test_system_vars`・`run_bounded`・`struct RecordingSink`・`RecordingSink::new`・`RecordingSink::records`）へ
`pub(super)` を付与した（付与のみ・本文は無変更）。フィールド `records` は同モジュール内の impl からしか触られないため
付与不要（テーマ側は `.records()` メソッド経由・参照 `:888` のみ）。複製は 1 件も作っていない。

**Implementation Notes の E0659 罠（タスク 3.3 §17.5）への対処**: 共有ヘルパは**明示 import** で受けた
（`use super::test_support::{RecordingSink, run_bounded, test_system_vars};`）。加えて誤結合の有無を実測で確認した——
移設前 `dispatcher.rs` を全数走査した結果、`test_system_vars`／`run_bounded`／`RecordingSink` の 3 識別子は
**テストモジュールの内側にしか出現せず**（親モジュール `dispatcher` の項目・`use` 由来の名前
（`DispatcherMsg`・`ActiveTalk`・`DispatcherState`・`spawn_dispatcher`・`ControlFlow`・`Sender`・`ActorHandle`・
`reply_channel`・`run_inbox`・`spawn_actor`・`KanadeMsg`・`MonotonicMs`・`ChoiceWaiting`・`CueSink`・`SakuraMsg`・
`StartTalk`・`TalkCommand`・`TalkDone`・`TalkHandle`・`TalkId`・`spawn_talk`・`SystemVarSource`・`BootCueSink`）とは
1 件も衝突しない）、`use super::*;` が同名を供給する余地は無い。同一シグネチャの黙った差し替えは起き得ない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 各新ファイルの先頭に移設元の `use` 群を置き、そのファイルで実際に使う項目だけへ絞った
（絞る前のビルドで `unused_imports` が 7 件出て要件 2.6 に反したため）。落としたのは
`dispatcher_test_support`＝`test_log_capture` 3 項目／`CueCommand`・`TalkEndReason`／`mpsc`（`self`）／`tracing::Level`、
`dispatcher_slot_tests`＝`test_log_capture` 3 項目／`sync_channel`／`std::thread`／`tracing::Level`、
`dispatcher_choice_tests`＝`CueCommand`・`TalkCue`／`sync_channel`／`std::thread`。
`runtime_tests.rs`・`ticker_tests.rs` は単純移設のため `use` ヘッダを 1 行も変えていない。
`use` 項目は §11.4 のとおり本文一致検証の対象外である。**可視性・`use` 以外の調整は 1 件も必要なかった**（要件 2.8 の追加調整 0 件）。

### 18.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 3 ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ
（行コメント・入れ子ブロックコメント・通常文字列・raw 文字列（`#` の数）・バイト文字列・
文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と
「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は、既知の唯一の該当箇所
`crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691` を**盲点 1 件**として検出し、同ファイルの
`:382`・`:429`・`:614`・`:690` を **`\` 継続 4 件**として正しく切り分けることで確認した
（実測出力: `継続行 5 件: 382,429,614,690,691 / 盲点該当 1 件: 691`）。

| ファイル | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---|---:|
| `crates/areka-ghost/src/dispatcher.rs`（移設前） | 2（`:249` は本番本体・`:771` はテストモジュール内） | **0** |
| `crates/areka-ghost/src/runtime.rs`（移設前） | 8（`:271`・`:349` は本番本体／`:847`・`:860`・`:1222`・`:1223`・`:1224`・`:1306` はテストモジュール内） | **0** |
| `crates/areka-ghost/src/ticker.rs`（移設前） | 0 | **0** |
| 新テストファイル 5 本（移設後） | 7（`dispatcher_slot_tests.rs:289` ／ `runtime_tests.rs:195,208,570,571,572,654`） | **0** |

**結論: 担当 3 ファイルの複数行文字列リテラルはすべて `\` 継続であり、§11.4 第 1 の盲点の該当行は 0 件。**
Rust は `\` 改行に続く行頭空白を除去するため、一律 4 スペース de-indent はリテラルの中身を変えない。
移設されたのは 7 行（dispatcher `:771` → `dispatcher_slot_tests.rs:289` ／ runtime `:847,860,1222,1223,1224,1306`
→ `runtime_tests.rs:195,208,570,571,572,654`）で、いずれも
**「移設前の行から先頭 4 文字を除いたものと移設後の行がバイト同値」**であることを §18.4 (a2) の全行分類で確認済み
（この 7 行を含む全移設行が「ちょうど −4 スペース」に分類されている）。例外処理は不要だった。

### 18.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 470164a -OriginalPath <本番> -RelocatedPath "<新テスト群>" -Detail`

| 対象 | 出力 | exit |
|---|---|---:|
| `dispatcher.rs` → 3 ファイル | `MATCH: test fn 20=20 / helper item 16=16 / mod block 1 / files 3` | 0 |
| `runtime.rs` → 1 ファイル | `MATCH: test fn 16=16 / helper item 15=15 / mod block 1 / files 1` | 0 |
| `ticker.rs` → 1 ファイル | `MATCH: test fn 18=18 / helper item 2=2 / mod block 1 / files 1` | 0 |

3 本とも exit **0**（引数不正の 2 ではないことを、故意にパスを誤らせた対照実行が exit 2 を返すことで確認済み）。

**(a2) 行単位の分類と多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、
それとは独立に、移設した**全行**を位置対応で「(a) ちょうど −4 スペース」「(b) バイト同値（空行）」へ分類し、
どちらでもない行を全件提示させた。

| 新ファイル | −4 スペース行 | 空行 | その他 |
|---|---:|---:|---:|
| `dispatcher_test_support.rs` | 47 | 5 | **5**（すべて `pub(super)` 付与＝要件 2.4 が明示的に許容する可視性調整） |
| `dispatcher_slot_tests.rs` | 551 | 56 | 0 |
| `dispatcher_choice_tests.rs` | 700 | 59 | 0 |
| `runtime_tests.rs` | 844 | 116 | 0 |
| `ticker_tests.rs` | 433 | 68 | 0 |

「その他」5 行の全数は
`fn test_system_vars()` ／ `fn run_bounded<F: …>` ／ `struct RecordingSink {` ／ `fn new() -> Self {` ／
`fn records(&self) -> …` の先頭への `pub(super) ` 付与のみで、それ以外の文字は 1 字も違わない。

行の多重集合突合（空行を除く。元＝移設前ブロック本体を一律 4 スペース de-indent した行の多重集合）:

| 本番ファイル | 元 | 新 | 消えた行 | 増えた行 | 内訳 |
|---|---:|---:|---:|---:|---|
| `dispatcher.rs` | 1,310 | 1,320 | 6 | 16 | 消 = `pub(super)` を付ける前の 5 行＋落とした `use std::sync::mpsc::{self, sync_channel};` 1 本 ／ 増 = `pub(super)` 付き 5 行＋新設・複製された `use` 11 本 |
| `runtime.rs` | 844 | 844 | **0** | **0** | 完全一致（`use` も含め 1 行も動かしていない） |
| `ticker.rs` | 433 | 433 | **0** | **0** | 完全一致 |

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-ghost.csv`

    PASS: 全単射 OK / 行数 20 / 相異なる old_fqn 20 / 相異なる new_fqn 20 / フラグメント 1
      - areka-ghost.csv: 20 行

exit 0。20 行の内訳は `dispatcher::tests::*` → `dispatcher::slot_tests::*` 8 行／
`dispatcher::tests::*` → `dispatcher::choice_tests::*` 12 行。`reason` は全行 `theme_split`、
末尾セグメント（関数識別子）は旧新で同一。**`runtime.rs`・`ticker.rs` は完全修飾名が変わらないため行を持たない**
（`runtime::tests::*` 16 本・`ticker::tests::*` 18 本は移設前後で同名）。
移設前 `before_default.txt` に `dispatcher::tests::` が **20 行**実在し、対応表の `old_fqn` 20 件すべてが
そこに存在することを照合済み（不在 0）。
既存フラグメントとの結合検証（`-Path $V/mapping`）も
`PASS: 全単射 OK / 行数 252 / 相異なる old_fqn 252 / 相異なる new_fqn 252 / フラグメント 4` で exit 0（キー衝突なし）。

> 注: `runtime::tests::` は `before_default.txt` に 22 行あるが、これは他クレート（`dola` の `runtime` 等）の
> 同名モジュールを含む合計である。cargo の完全修飾名にはクレート名の接頭辞が付かないため、リスト照合は
> §10.2 のとおり**多重集合**として行われ、この重なりは判定に影響しない（本クレートぶんは 16 本）。
> タスク 3.5 は同じ `areka-ghost.csv` へ追記することになるが、`spine_e2e_test.rs` は 1 モジュール＝1 ファイルの
> 個別移設で完全修飾名が変わらないため、追加行は 0 行になる見込みである。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list` → stdout のみ → `: test$` 抽出 →
`[Array]::Sort($arr, [System.StringComparer]::Ordinal)` → UTF-8 BOM 無し・CRLF・末尾改行 1 つ・重複行を残す）で
リストを採取し、コミット済み `before_default.txt` と**タスク 3.1〜3.4 の 4 フラグメント全部**を渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after_default_task34.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 252 行 (4 ファイル) / 適用 252 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 252 行・未使用 0 行**。移設後リストの SHA256 は
`8468B087D4748BEC6DE24A9E08CF78996918413AC764C3680582C4D5B705C0E9`
（§10.2 手順 3〜5 の形式＝序数整列・UTF-8 BOM 無し・CRLF・末尾改行で採取。
中間リストファイル自体はコミットしない）。

> **是正（タスク 3.5 で発覚・親が訂正）**: ここには当初 `9C75E1B7…F20F` と記録していたが、
> これは同じ 4,790 行を **`Sort-Object`（カルチャ依存比較）** で整列したファイルのハッシュだった。
> §10.2 手順 3 が指定する `[System.StringComparer]::Ordinal` で採り直すと `8468B087…C0E9` になる
> （タスク 3.5 の実装者が出所を特定し、親が独立に再計算して確認）。
> **リストの内容（行の多重集合）は同一**であり `Compare-TestLists.ps1` の判定は整列順に依存しないため、
> 要件 2.2 / 2.3 の充足には影響しない。§14.5(b) に続き 2 度目の同種の取り違えである。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、
対応表を**使わない**多重集合の対称差で確かめた。

| 検査 | 結果 |
|---|---|
| `before_default.txt` と移設後リストの対称差（対応表なし）: 消えた行 / 現れた行 / 全フラグメント行数 | **252 / 252 / 252**（三者一致） |
| タスク 3.1〜3.3 のフラグメントだけを `before_default.txt` へ適用して復元した「本タスク着手直前のリスト」と移設後リストの対称差: 消えた行 / 現れた行 / 本タスクの対応表行数 | **20 / 20 / 20**（三者一致） |
| 消えた行がすべて `old_fqn` に在るか | **True**（例外 0） |
| 現れた行がすべて `new_fqn` に在るか | **True**（例外 0） |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |
| `old_fqn` と `new_fqn` が同一の行（＝変わっていない名前を載せた水増し） | **0** |
| 末尾セグメント（関数識別子）が旧新で相違する行 | **0** |
| `reason` が `theme_split` 以外の行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。

移設後リストの当該モジュール別内訳（`cargo test -p areka-ghost --lib -- --list`）:
`dispatcher::slot_tests` 8 ／ `dispatcher::choice_tests` 12 ／ `runtime::tests` 16 ／ `ticker::tests` 18。
移設前の `dispatcher::tests` 20・`runtime::tests` 16・`ticker::tests` 18 と本数が一致する。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-ghost --no-fail-fast` → **exit 0**。
**147 passed / 0 failed / 0 ignored**（lib 107 ＋ 統合テスト（`tests/ghost.rs` ツリー）40 ＋ doctest 0）。
移設前の独立導出値と一致する: 移設前コミット `470164a` の `git show` に対する `#[test]` 属性行の全数は
`src/` 全 12 ファイルで **107**（config 4・dispatcher 20・prop_sink 9・relay 3・runtime 16・shiori_inproc 19・
shiori_wiring 2・sink 5・sylphya_wiring 11・ticker 18）、`tests/` ツリー全 9 ファイルで **40**。
**統合テストツリー（`spine_e2e_test.rs` を含む）は無変更のまま 40 本すべて緑である。**

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-ghost --all-targets` → exit 0。
`areka-ghost` に帰属する警告は **0 件**（出た警告は依存の `shiori4-testdll` (lib) 1 件のみで、
`before_build_warnings.txt` の `[PER-UNIT TALLY]` に既に載っている基準内の警告）。基準値も `areka-ghost` へは 0 件の割当。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。ユニット別 generated 件数（7 ユニット）も多重集合として一致し、
`areka-ghost` のサマリ行は移設前後とも 0 件である。

**(f) 本番本体の無変更** — 移設前コミット `470164a` の各本番ファイルの先頭〜旧 `#[cfg(test)]` 行の直前までを
現作業ツリーと逐行突合し、**3 ファイルとも不一致 0**（dispatcher 1-420 ／ runtime 1-650 ／ ticker 1-319）。

| ファイル | `git diff --numstat` |
|---|---|
| `dispatcher.rs` | 挿入 8 ／ 削除 1,435 |
| `runtime.rs` | 挿入 2 ／ 削除 962 |
| `ticker.rs` | 挿入 2 ／ 削除 503 |

3 ファイル合計 `3 files changed, 12 insertions(+), 2900 deletions(-)`。挿入 12 = 新設した接続宣言（9＋3＋3＝15 行）のうち
元位置に据え置いた `#[cfg(test)]` 行 3 本を除いた行数である。

**(g) 完了状態の直接確認** — 3 本番ファイルに残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言のみ:
`dispatcher.rs:421-429` ／ `runtime.rs:651-653` ／ `ticker.rs:320-322`。`#[test]` は 1 件も残っていない。
テストモジュール本体は 1 行も残っていない。

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 9 パスのみ:
変更 3 本（本番ファイル）＋未追跡 6 本（新テストファイル 5 本＋`verification/mapping/areka-ghost.csv`）。
**`crates/areka-ghost/tests/` 配下の差分は 0 件**。他クレート・`Cargo.toml`・`tasks.md`・spec 本体ドキュメントも無変更。

### 18.5 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（`areka-ghost` の `src/` 3 ファイル・54 テスト・テストコード 2,903 行）では、修正を要する
壊れたテスト・不正なテストは 1 件も発見しなかった。所有 spec への送付所見は 0 件である。**

以下は「調べたが問題なし／既存のまま据え置き」と確定した記録（次に触る者が同じ調査を繰り返さないための控え。是正は行わない）:

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | `static` / `thread_local!` / `std::env::set_var` / `unsafe` / `std::thread::sleep` / `#[ignore]` / `OnceLock` の使用 | 担当 5 テストファイル全域 | **0 件**（全走査で確認）。`#[should_panic]` は 1 件（`ticker_tests.rs:57`・`starting_at_panics_on_zero_interval`＝正当な契約テスト）。プロセスグローバルな可変状態をテスト側で持つものは無い |
| 2 | 一時ディレクトリ名が**テスト関数名だけ**でユニーク化されており、プロセス/実行をまたぐと同一パスになる | `runtime_tests.rs:55-59`（`unique_temp_dir`）＋利用 8 箇所（`:138,183,242,285,403,466,577,874`）。同型の既存実装が `config.rs:84-88` にもある | **既存・記録のみ（是正しない）**。1 プロセス内ではタグ（テスト関数名）が相異なるためテスト間の衝突は起きず、各テストは使用前後に `remove_dir_all` するので前回実行の残骸も掃われる。ただし**同一マシンで `cargo test` を 2 プロセス同時に走らせる**（例: x64 と i686 を並走させる・別シェルで同時実行する）と同じパスを共有し、片方の事前 `remove_dir_all` がもう片方の fixture を消し得る。関数名の「unique」は**プロセス内の一意性**であってグローバル一意ではない。テスト本文の変更は要件 2.4 違反になるため触らない。**要件 5.2 の送付対象ではない**——5.2 が拾うのは「テスト**間**で状態が汚染されているテストモジュール」であり、本件は 1 プロセス内では汚染が起きない**プロセス間**の危険だからである（8 タグが相異なることをレビューが実測確認済み）。将来この決定論の穴に手を入れるとすれば `test-cage-determinism`（W6.9）の領分だが、本 spec からの送付所見としては起票しない |
| 3 | 実 DLL 経路の観測が**壁時計デッドライン**（10 秒）で括られており、注入 simulated time（`now += 1`）が観測を待たずに前進する | `runtime_tests.rs:634-651`（`inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll`） | **既存・記録のみ（是正しない）**。当該箇所には「デッドラインは宙吊り防止の上限にすぎず、talk timeline を進めるのは注入 Tick のみ」という設計意図がコメントで明記されており、`base_now` は slot 占有後の初回 Tick で刻印されるため `now` の先行前進は換算結果を壊さない。とはいえ `LoadLibraryW`＋`CreateInstance` が 10 秒を超える極端な負荷では偽陽性の赤になり得る（回数上限ではなく壁時計で括っている唯一の箇所）。移設前から同一コードであり、テスト本文の変更は要件 2.4 違反になるため触らない |
| 4 | 「送信が起きない」ことを短い timeout で示す負の窓 | `ticker_tests.rs:427`（200ms・catch-up が複数境界を 1 回へ畳むことの固定） | **問題なし・記録のみ**。時計は注入値のまま（次デッドライン未満）で追加発火が構造的に起き得ない状態を作ってから測っており、両方向決定的。他の `recv_timeout` はすべて 5 秒の宙吊り防止上限で、正常系では即座に届く |
| 5 | プロセスグローバルな tracing subscriber（leak された interest-keeper） | `test_log_capture.rs:1-40`（本タスクの担当ファイルではない・`INTEREST_KEEPER`）。利用は `dispatcher_choice_tests.rs` の `capture` 9 箇所 | **問題なし・記録のみ（是正しない）**。並列負荷下で `Interest::never` が焼き付く確率欠陥を根治するために意図して常駐させているもので、モジュール doc に不変条件（「本モジュールより先に別のグローバル subscriber を設定してはならない」）と違反時の大声 panic まで明記されている。テスト間の状態汚染ではなく汚染の**予防**機構であり、本 spec でも `test-cage-determinism` でも撤去対象ではない |
| 6 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | — | **0 件**。共有ヘルパ 3 項目（＋impl 内メソッド 2 本）への `pub(super)` 付与と `use` の絞り込みだけで通った。§17.5 #4 が警告した同名 shadow ヘルパによる E0659 は本ファイルには存在しない（§18.2 の全数照合で確認） |

### 18.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-ghost/src/dispatcher_test_support.rs` | 新規（63 行・共有ヘルパ 3 項目＋impl 2 本） |
| `crates/areka-ghost/src/dispatcher_slot_tests.rs` | 新規（613 行・8 テスト） |
| `crates/areka-ghost/src/dispatcher_choice_tests.rs` | 新規（767 行・12 テスト） |
| `crates/areka-ghost/src/runtime_tests.rs` | 新規（960 行・16 テスト・単純移設） |
| `crates/areka-ghost/src/ticker_tests.rs` | 新規（501 行・18 テスト・単純移設） |
| 上記に対応する本番 3 ファイル | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体は無変更） |
| `verification/mapping/areka-ghost.csv` | 新規（20 行・全単射検証済み。タスク 3.5 が同ファイルへ追記する） |
| `verification/notes.md` | 本節（§18）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-ghost` ライブラリ側）とする。
同クレートの統合テストツリー（タスク 3.5）は別コミットになる。

## 19. 統合テストツリーのテスト分離: `areka-ghost`（タスク 3.5・要件 1.1 / 1.3 / 1.7 / 2.4 / 2.8 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `585cd5d`（タスク 3.4 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 触ったのは `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` ＋ 新規テストファイル 10 本 ＋ 本ファイルのみ。
  入口 `tests/ghost.rs`・同ツリーの他 7 ファイル・`src/**`・`Cargo.toml`・spec 本体ドキュメントは **1 行も変更していない**
  （`git status --porcelain -uall` で確認）。`verification/mapping/areka-ghost.csv` も **20 行のまま無変更**——本タスクは
  1 テストモジュール＝1 テストファイルの個別移設で完全修飾名が一切変わらないため、対応表の追加行は **0 行**である
  （§18.4 (b) の予測どおり）。

### 19.1 移設した 1 ファイル・10 テストモジュール（design §File Structure Plan の `crates/areka-ghost` の `tests/` 1 本と完全一致）

移設前 `spine_e2e_test.rs` は 2,574 行・テストモジュール 10 個・テストコード 2,091 行（要件 1.2 の実測値と一致）。
10 モジュールはいずれも **ファイル末尾側に連続配置**され、共有 fixture（L1-314）より後ろにのみ在る。

| # | テストモジュール | 移設前ブロック（`#[cfg(test)]`〜`}`） | ブロック行 | バナー行範囲 | 本体行範囲 | 新テストファイル | 新ファイル行 |
|---:|---|---|---:|---|---|---|---:|
| 1 | `tests` | 321-459 | 139 | 316-319 | 323-458 | `spine_e2e_test_tests.rs` | 141 |
| 2 | `broadcast_relevance_partition` | 473-643 | 171 | 461-472 | 475-642 | `spine_e2e_test_broadcast_relevance_partition.rs` | 181 |
| 3 | `s1_boot_success` | 652-934 | 283 | 645-651 | 654-933 | `spine_e2e_test_s1_boot_success.rs` | 288 |
| 4 | `s2_connect_failure` | 945-1103 | 159 | 936-944 | 947-1102 | `spine_e2e_test_s2_connect_failure.rs` | 166 |
| 5 | `s3_helper_liveness_detected` | 1116-1386 | 271 | 1105-1115 | 1118-1385 | `spine_e2e_test_s3_helper_liveness_detected.rs` | 280 |
| 6 | `s4_close_handshake` | 1415-1667 | 253 | 1388-1414 | 1417-1666 | `spine_e2e_test_s4_close_handshake.rs` | 278 |
| 7 | `s5_close_deadline` | 1707-2006 | 300 | 1669-1706 | 1709-2005 | `spine_e2e_test_s5_close_deadline.rs` | 336 |
| 8 | `s6_full_disconnect` | 2033-2236 | 204 | 2008-2032 | 2035-2235 | `spine_e2e_test_s6_full_disconnect.rs` | 227 |
| 9 | `global_log_probe` | 2248-2330 | 83 | 2238-2247 | 2250-2329 | `spine_e2e_test_global_log_probe.rs` | 91 |
| 10 | `s7_second_boot_record_present` | 2347-2574 | 228 | 2332-2346 | 2349-2573 | `spine_e2e_test_s7_second_boot_record_present.rs` | 241 |

- ブロック行の合計は **2,091**（要件 1.2・design の「テスト行 2,091」と完全一致）。
- **新テストファイル 10 本はすべて 1,000 行以下**（最大 `…_s5_close_deadline.rs` の 336 行）。テーマ分割は不要
  （design §テーマ分割ポリシー: 本ファイルは複数テストモジュールの個別ファイル化だけで全ファイル 1,000 行以下に収まる 3 本のうちの 1 本）。
- 移設後の `spine_e2e_test.rs` は **354 行**（共有 fixture L1-314 ＋ 接続宣言 10 組 40 行）。`git diff --numstat` は
  `20 insertions(+) / 2240 deletions(-)`（挿入 20 = 新設した `#[path]` 10 行 ＋ `mod X;` 10 行。`#[cfg(test)]` 10 行と
  区切りの空行 10 行は元位置の行と一致するため差分に現れない）。
- **共有 fixture は本体に残置**（design §File Structure Plan の規定どおり）: `E2E_BOUND`(:22)・`spin_pumping_ticks`(:48)・
  `RecordedCall`(:73)・`ScriptedShioriBackendBuilder`(:89)・`ScriptedShioriBackend`(:170)・`ScriptedShioriHandle`(:252)・
  `RecordingSink`(:282) とその impl 群。行番号は移設前後で同一（§19.5 (f)）。

接続宣言（10 モジュールとも同一文言・design §移設方式の裁定 案 C・**`src/` 側と文言まで同一**）:

    #[cfg(test)]
    #[path = "spine_e2e_test_<モジュール名>.rs"]
    mod <モジュール名>;

**統合テストツリー内の冗長 `#[cfg(test)]` は落とさず残した**（設計判断 #13）。`tests/` 配下はクレート全体が test ターゲット
であり `#[cfg(test)]` は常に真だが、除去は要件 2.4 が禁じる属性の変更に当たり、次回の行数計測（テストモジュール判定式）の
一貫性も壊すためである。移設前後の `#[cfg(test)]` の出現数はともに **11**——内訳は属性 10 件（＝接続宣言 10 組）と、
`global_log_probe` のバナー内でリテラルとして言及されている 1 件（移設前 `:2242` → `spine_e2e_test_global_log_probe.rs:5`）。
移設後の `spine_e2e_test.rs` に残る `#[test]` の文字列 1 件は先頭 module doc（`:9`）の記述であり、移設前から同一・テスト実体ではない。

### 19.2 モジュール解決の実測（要件 3.1 / 3.2・research §2.3.3 の 2 階層規則を本ツリーで再確認）

`spine_e2e_test.rs` 自身が入口 `tests/ghost.rs:17-18` から `#[path = "ghost/spine_e2e_test.rs"]` で読み込まれるため、
その子モジュールの `#[path]` 値（裸のファイル名）が **どのディレクトリを基準に解決されるか** を否定対照で確定させた。
いずれも一時的に宣言を書き換えて `cargo test -p areka-ghost --test ghost --no-run` を実行し、**Edit で厳密に復元**した
（破壊的 git は不使用。復元後に `NEGCTRL` 文字列がリポジトリ全域に 0 件であること、`git diff` に痕跡が無いことを確認済み）。

| 対照 | 書き換えた宣言 | rustc の出力（要点） | 判ること |
|---|---|---|---|
| A | `#[path = "spine_e2e_test_tests_NEGCTRL.rs"]` | `error: couldn't read `crates\areka-ghost\tests\ghost\spine_e2e_test_tests_NEGCTRL.rs`: 指定されたファイルが見つかりません。 (os error 2)` / `--> crates\areka-ghost\tests\ghost\spine_e2e_test.rs:318:1` | 裸のファイル名は **宣言ファイル `spine_e2e_test.rs` 自身のディレクトリ `tests/ghost/`** を基準に解決される（入口 `tests/` でも `tests/ghost/spine_e2e_test/` でもない） |
| B | `#[path]` を外した素の `mod tests;` | `error[E0583]: file not found for module `tests`` / `= help: to create the module `tests`, create file "crates\areka-ghost\tests\ghost\tests.rs" or "crates\areka-ghost\tests\ghost\tests\mod.rs"` | 素の `mod` も同じディレクトリを見る。ゆえに `<stem>_<モジュール名>.rs` 命名（案 C）を素の `mod` で表現することは**できない**——`#[path]` の明示が必須である |

対照 B は research §2.3.3 が測った E0583 を本ツリーで再現したものであり、案 A（素の `mod`）が単一方式として成立しない
という裁定の裏取りになっている。両対照とも exit 101（コンパイルエラー）で、復元後は exit 0・全緑に戻る。

### 19.3 バナーと doc コメントの分類（設計判断 #2 と Implementation Notes の `///` 規則の適用）

10 モジュールすべてで、`#[cfg(test)]` の直前にあるのは **`//` 行コメントのバナーブロック**（`// ===== S2: … =====` の
見出し行＋そのモジュールが何を固定するかの説明行）である。`///`（doc コメント）や `//!` は **1 件も無い**——
機械判定（バナー候補行が `///` / `//!` で始まったら例外送出）でも 0 件を確認した。よって Implementation Notes の
「`///` doc コメント付きテストモジュールは接続宣言側へ残置する」規則（§14.3）は本タスクでは発動せず、
設計判断 #2 のとおり **10 本すべてのバナーを対応するテストファイルの先頭へ同伴**させた。

| # | モジュール | バナー行数 | 分類 | 移設後の位置 |
|---:|---|---:|---|---|
| 1 | `tests` | 4 | `//` バナー（モジュール間見出し） | ファイル先頭 L1-4 |
| 2 | `broadcast_relevance_partition` | 12 | 同上 | L1-12 |
| 3 | `s1_boot_success` | 7 | 同上 | L1-7 |
| 4 | `s2_connect_failure` | 9 | 同上 | L1-9 |
| 5 | `s3_helper_liveness_detected` | 11 | 同上 | L1-11 |
| 6 | `s4_close_handshake` | 27 | 同上 | L1-27 |
| 7 | `s5_close_deadline` | 38 | 同上 | L1-38 |
| 8 | `s6_full_disconnect` | 25 | 同上 | L1-25 |
| 9 | `global_log_probe` | 10 | 同上 | L1-10 |
| 10 | `s7_second_boot_record_present` | 15 | 同上 | L1-15 |

バナー 158 行は **全行がバイト同値**（行頭空白・文言・行数のいずれも不変・10 モジュールとも不一致 0）。
バナーの直後に空行を 1 行置き（移設前もバナーと `#[cfg(test)]` の間に空行があった形を保つ）、その後に de-indent した本体を置いた。
本文一致検証はバナーを見ない——比較対象は移設前の「`{` の次の行〜`}` の前の行」であり、バナーはブロックの外側だからである。
移設後のファイルでもバナーは先頭の `use` 項目に付属し、`use` 項目は §11.4 のとおり突合対象から外れる（10 モジュールとも
本体の先頭項目は `use` である）。したがってバナーの同伴は判定に影響せず、かつ 1 文字も失われていない。

### 19.4 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ（行コメント・入れ子ブロックコメント・
通常文字列・raw 文字列（`#` の数）・バイト文字列・文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で
文字列リテラルの内部にいる行」と「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は §18.3 と同じ既知ケース
`crates/wintf/src/ecs/window_proc/window_pos_tests.rs` で確認済み（実測出力: `継続行 5 件: 382,429,614,690,691 / 盲点該当 1 件: 691`）。

| ファイル | 複数行文字列の継続行 | 盲点該当（`\` 継続でない行） |
|---|---|---:|
| `tests/ghost/spine_e2e_test.rs`（移設前・2,574 行） | 27（`790,895,1043,1073,1074,1271,1288,1336,1337,1338,1551,1588,1642,1660,1842,1882,1937,1980,1999,2189,2204,2218,2231,2321,2322,2510,2557`） | **0** |
| 新テストファイル 10 本（移設後） | 27（s1 `145,250` ／ s2 `107,137,138` ／ s3 `166,183,231,232,233` ／ s4 `163,200,254,272` ／ s5 `173,213,268,311,330` ／ s6 `181,196,210,223` ／ `global_log_probe` `83,84` ／ s7 `178,225`） | **0** |
| `tests/ghost/spine_e2e_test.rs`（移設後・354 行＝共有 fixture のみ） | 0 | **0** |

**結論: 担当ファイルの複数行文字列リテラルはすべて `\` 継続であり、§11.4 第 1 の盲点の該当行は 0 件。**
継続行の本数も移設前後で 27 = 27 と一致する。Rust は `\` 改行に続く行頭空白を除去するため、一律 4 スペース de-indent は
リテラルの中身を変えない。例外処理は不要だった（§19.5 (a2) の全行分類でも、これら 27 行を含む全移設行が
「ちょうど −4 スペース」に分類されている）。

### 19.5 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 585cd5d -OriginalPath crates/areka-ghost/tests/ghost/spine_e2e_test.rs -RelocatedPath "<新 10 本をカンマ区切り>" -Detail`

    MATCH: test fn 15=15 / helper item 40=40 / mod block 10 / files 10

exit **0**。加えて `-ModuleName` で 1 モジュール対 1 ファイルへ絞った 10 本の個別照合もすべて exit 0:

| モジュール | 出力 |
|---|---|
| `tests` | `MATCH: test fn 6=6 / helper item 0=0 / mod block 1 / files 1` |
| `broadcast_relevance_partition` | `MATCH: test fn 2=2 / helper item 3=3 / mod block 1 / files 1` |
| `s1_boot_success` | `MATCH: test fn 1=1 / helper item 4=4 / mod block 1 / files 1` |
| `s2_connect_failure` | `MATCH: test fn 1=1 / helper item 4=4 / mod block 1 / files 1` |
| `s3_helper_liveness_detected` | `MATCH: test fn 1=1 / helper item 4=4 / mod block 1 / files 1` |
| `s4_close_handshake` | `MATCH: test fn 1=1 / helper item 4=4 / mod block 1 / files 1` |
| `s5_close_deadline` | `MATCH: test fn 1=1 / helper item 4=4 / mod block 1 / files 1` |
| `s6_full_disconnect` | `MATCH: test fn 1=1 / helper item 4=4 / mod block 1 / files 1` |
| `global_log_probe` | `MATCH: test fn 0=0 / helper item 7=7 / mod block 1 / files 1` |
| `s7_second_boot_record_present` | `MATCH: test fn 1=1 / helper item 6=6 / mod block 1 / files 1` |

故意にパスを誤らせた対照実行は **exit 2**（引数不正）を返し、不一致の 1 と区別できることを確認した
（Implementation Notes の「2 を 1 と読み違えない」に対応）。

**(a2) 行単位の分類と多重集合突合（スクリプトより強い独自検証）** — 移設した全行を位置対応で
「ちょうど −4 スペース」「バイト同値（空行）」へ分類し、どちらでもない行を全件提示させた。

| 新ファイル | −4 スペース行 | 空行 | その他 | バナーのバイト不一致 |
|---|---:|---:|---:|---:|
| `…_tests.rs` | 113 | 23 | **0** | 0 |
| `…_broadcast_relevance_partition.rs` | 159 | 9 | **0** | 0 |
| `…_s1_boot_success.rs` | 262 | 18 | **0** | 0 |
| `…_s2_connect_failure.rs` | 140 | 16 | **0** | 0 |
| `…_s3_helper_liveness_detected.rs` | 244 | 24 | **0** | 0 |
| `…_s4_close_handshake.rs` | 231 | 19 | **0** | 0 |
| `…_s5_close_deadline.rs` | 275 | 22 | **0** | 0 |
| `…_s6_full_disconnect.rs` | 182 | 19 | **0** | 0 |
| `…_global_log_probe.rs` | 72 | 8 | **0** | 0 |
| `…_s7_second_boot_record_present.rs` | 205 | 20 | **0** | 0 |
| **合計** | **1,883** | **178** | **0** | **0** |

「その他」は **全モジュールで 0 件**——すなわち **可視性の付与も `use` の改変も本タスクでは 1 件も発生していない**
（要件 2.8 の追加調整 0 件）。1 テストモジュール＝1 テストファイルで `super` の指す先が変わらないため、
`use super::*;` を含む import 群がそのまま有効だったことによる。共有ヘルパの切り出し（`test_support`）も不要のため、
Implementation Notes の E0659 罠（同名 shadow ヘルパのグロブ衝突・§17.5）は本タスクでは発生し得ない。

行の多重集合突合（空行を除く。元＝移設前ブロック本体を一律 4 スペース de-indent した行の多重集合）:
**元 1,883 / 新 1,883 / 消えた行 0 / 増えた行 0**（完全一致）。

**(b) 対応表（要件 2.9）— 追加行 0** — 1 テストモジュール＝1 テストファイルの個別移設ゆえ、10 モジュールの
モジュールパスは移設前後で不変であり、テスト完全修飾名は 1 件も変わらない。`verification/mapping/areka-ghost.csv` は
タスク 3.4 の **20 行のまま無変更**（本タスクは行を追加しない）。

**(b2) 直接証跡: `spine_e2e_test::` の 15 件がバイト同値** — 移設前 `before_default.txt` から抽出した
`spine_e2e_test::` で始まる 15 行と、移設後リストから抽出した 15 行を `Compare-Object` で突合し **差分ゼロ**。
15 件の内訳は `tests::` 6 ／ `broadcast_relevance_partition::` 2 ／ `s1`〜`s7` 各 1（`global_log_probe` は
テスト関数を持たない支援モジュール）。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

§10.2 の手順（`cargo test --workspace --no-fail-fast -- --list` → stdout のみ → `: test$` 抽出 →
`[Array]::Sort($arr, [System.StringComparer]::Ordinal)` → UTF-8 BOM 無し・CRLF・末尾改行 1 つ・重複行を残す）で
移設後リストを採取し、コミット済み `before_default.txt` と**タスク 3.1〜3.4 の 4 フラグメント全部（252 行・無変更）**を
渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after_default_task35.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 252 行 (4 ファイル) / 適用 252 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 252 行・未使用 0 行**（本タスクが対応表を 1 行も増やしていないことと整合）。
移設後リストの SHA256 は `8468B087D4748BEC6DE24A9E08CF78996918413AC764C3680582C4D5B705C0E9`
（§10.2 手順 5 の形式＝UTF-8 BOM 無し・CRLF・末尾改行 1 つ・序数整列。中間リストファイル自体はコミットしない）。

**(c2) 反証: 対応表を使わない対称差** — `before_default.txt` と移設後リストの多重集合対称差は
**消えた行 252 / 現れた行 252**（＝全フラグメント行数 252 と三者一致）。さらに、コミット済み `before_default.txt` に
4 フラグメントを適用して再構成したリストは、移設後の実採取リストと **完全同値**（`Compare-Object` 差分なし・
SHA256 も同一）。すなわち本タスクによる名前の変化は **0 件**であることが、対応表に依らずに示されている。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-ghost --no-fail-fast` → **exit 0**。
**147 passed / 0 failed / 0 ignored**（lib 107 ＋ 統合テスト（`tests/ghost.rs` ツリー）40 ＋ doctest 0）。
§18.4 (d) が記録した移設前の 147（lib 107 ＋統合 40）と**完全一致**。統合テスト 40 本のうち
`spine_e2e_test::` は 15 本で、10 の新ファイルすべてから期待どおり収集されている
（`#[cfg(test)]` が統合テストターゲットで真であることの実行時確認でもある）。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-ghost --all-targets` → exit 0。
`areka-ghost` に帰属する警告は **0 件**（出た警告は依存の `shiori4-testdll` (lib) の linker stdout 1 件のみで、
`before_build_warnings.txt` の `[PER-UNIT TALLY]` に既に載っている基準内の警告）。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。ユニット別 generated 件数（7 ユニット）も多重集合として一致する。

**(f) 共有 fixture の無変更と外部参照の不変（design §File Structure Plan）** — 移設前コミット `585cd5d` の
`spine_e2e_test.rs` の L1-314（module doc ＋ `use` ＋ `E2E_BOUND` ＋ `spin_pumping_ticks` ＋ `ScriptedShioriBackend` 一式 ＋
`RecordingSink` 一式）を現作業ツリーの L1-314 と逐行突合し、**バイト不一致 0**。行番号も動いていない。
外部からの参照 4 箇所はいずれも文言・解決先とも不変で、当該 4 ファイルに `git` 差分は無い:

| 参照元 | 行 | 参照 |
|---|---:|---|
| `crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` | 49 | `use crate::spine_e2e_test::RecordingSink;` |
| `crates/areka-ghost/tests/ghost/real_pasta_test.rs` | 39 | `use crate::spine_e2e_test::RecordingSink;` |
| `crates/areka-ghost/tests/ghost/snapshot_capture_test.rs` | 106 | `use crate::spine_e2e_test::RecordingSink;` |
| `crates/areka-ghost/tests/ghost/sylphya_integration_test.rs` | 46 | `use crate::spine_e2e_test::RecordingSink;` |

モジュール内部からの `super::` 参照（`super::E2E_BOUND` 11 箇所・`super::spin_pumping_ticks` 6 箇所・
`super::global_log_probe::install` 1 箇所・`use super::*;` 8 箇所）も、`super` の指す先が
`spine_e2e_test` のままであるため 1 件も書き換えていない。

**(g) 完了状態の直接確認** — 移設後 `spine_e2e_test.rs` に残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて
接続宣言のみ（`:316-318`・`:320-322`・`:324-326`・`:328-330`・`:332-334`・`:336-338`・`:340-342`・`:344-346`・
`:348-350`・`:352-354` の 10 組）。テストモジュール本体は 1 行も残っていない。

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 11 パスのみ:
変更 1 本（`tests/ghost/spine_e2e_test.rs`）＋未追跡 10 本（新テストファイル）。
`tests/ghost.rs`・同ツリーの他 7 ファイル・`src/**`・`verification/mapping/**`・`Cargo.toml`・`tasks.md`・
spec 本体ドキュメントはいずれも無変更。否定対照（§19.2）の痕跡も 0 件（`NEGCTRL` の全域 grep が 0 ヒット）。

### 19.6 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（統合テストツリー 1 ファイル・10 テストモジュール・15 テスト・テストコード 2,091 行）では、
修正を要する壊れたテスト・不正なテストは 1 件も発見しなかった。所有 spec への送付所見は 0 件である。**

以下は「調べたが問題なし／既存のまま据え置き」と確定した記録（次に触る者が同じ調査を繰り返さないための控え。是正は行わない）:

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | プロセスグローバルな tracing subscriber を `OnceLock` ＋ `set_global_default` で常駐させる支援モジュール | `spine_e2e_test_global_log_probe.rs:68`（`static BUFFER`）・`:75-91`（`install()`）。利用は `spine_e2e_test_s7_second_boot_record_present.rs:109`（`super::global_log_probe::install()`） | **問題なし・記録のみ（是正しない）**。kanade アクタースレッドで発火する `boot_gate skip_first_boot` ログをスレッドローカル捕捉では捕えられないために意図して常駐させているもので、`install()` の `expect` メッセージに不変条件（「本モジュールより先に別の global subscriber を設定してはならない」）が明記されている。**この不変条件が現に成立していることを実測で確認した**——`crates/areka-ghost/tests/` 全域を走査した結果、`set_global_default` ／ `subscriber::set_default` ／ `with_default` ／ `.init()` を呼ぶ箇所は本モジュール以外に **0 件**。フィルタ無しの capture-all ゆえ Interest の `Never` 焼き付きも起こさない。テスト間の状態汚染ではなく汚染の**予防**機構であり、`test-cage-determinism` への送付対象ではない |
| 2 | 一時ディレクトリ名がテスト関数名だけでユニーク化されており、プロセス/実行をまたぐと同一パスになる | `spine_e2e_test_s1_boot_success.rs:17-21`／`…_s2_connect_failure.rs:22-26`／`…_s3_helper_liveness_detected.rs:25-29`／`…_s4_close_handshake.rs:37-41`／`…_s5_close_deadline.rs:48-52`／`…_s6_full_disconnect.rs:39-43`（いずれも `unique_temp_dir`） | **既存・記録のみ（是正しない）**。§18.5 #2（`runtime_tests.rs:55-59`・`config.rs:84-88`）と**同型**の観測である。モジュール名（`s1`〜`s6`）とテスト関数名で 1 プロセス内の一意性は保たれるため、テスト間の衝突は起きない。危険なのは同一マシンで `cargo test` を 2 プロセス同時に走らせた場合（プロセス**間**）であり、要件 5.2 が拾う「テスト**間**の状態汚染」には当たらない。**同ファイル群のうち `s7` だけは `std::process::id()` を混ぜて硬化済み**（`…_s7_second_boot_record_present.rs:32-40`）——同一ツリー内で流儀が割れている点も含めて記録に留める。テスト本文の変更は要件 2.4 違反になるため触らない |
| 3 | 有界スピンの壁時計安全弁 60 秒（`E2E_BOUND`）と、それを直接使う `Instant` デッドライン | `spine_e2e_test.rs:22`（定数・共有 fixture に残置）・`:54`（`spin_pumping_ticks` 内）／`super::E2E_BOUND` の参照は新 7 ファイルに 11 箇所（`…_s1…:277`・`…_s2…:69`・`…_s3…:68,144`・`…_s4…:266`・`…_s5…:251,324`・`…_s6…:83`・`…_s7…:156,206,231`） | **問題なし・記録のみ**。定義位置（`:19-22`）のコメントに「意味論 deadline は MonotonicMs 仮想時間で注入されるためこの壁時計値はテスト意味論に影響せず、workspace 並列負荷の飢餓による偽赤のみを防ぐ」と明記されており、兄弟 e2e（inproc/real_pasta/snapshot_capture = 60s）と規約が揃っている。仮想時刻を進めるのは注入 Tick のみで、壁時計は宙吊り検出の上限にすぎない |
| 4 | `static` / `thread_local!` / `std::env::set_var` / `unsafe` / `std::thread::sleep` / `#[ignore]` / `#[should_panic]` の使用 | 担当 11 ファイル全域 | `static` は #1 の `BUFFER` 1 件のみ。`thread_local!` ／ `set_var` ／ `unsafe` ／ `sleep` ／ `#[ignore]` ／ `#[should_panic]` は **0 件**（全走査で確認）。プロセスグローバルな可変状態は #1 の観測用バッファのみである |
| 5 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | — | **0 件**。1 テストモジュール＝1 テストファイルで `super` の指す先が変わらないため、`use super::*;` を含む import 群がそのまま有効だった（§19.5 (a2) の「その他 0 行」がその機械的証跡） |
| 6 | §18.4 (c) が記録した移設後リストの SHA256 `9C75E1B7…F20F` が §10.2 手順 3・5 の形式で再現しない | `verification/notes.md` §18.4 (c) | **記録のみ（是正しない・他タスクの記録であり本 spec の受け入れには無影響）**。本タスクで出所を特定した——当該ハッシュは同じ 4,790 行を **`Sort-Object`（カルチャ依存比較）で整列**したファイルのものであり（実測で完全一致を確認）、§10.2 手順 3 が指定する `[System.StringComparer]::Ordinal` 整列では `8468B087…C0E9` になる。**リストの内容（行の多重集合）は同一**であり、`Compare-TestLists.ps1` の判定は行単位の多重集合突合ゆえ整列順に依存しないため、要件 2.2 / 2.3 の充足には影響しない。§18.4 (c) の本文が「整列は `Sort-Object` を使わず `Ordinal` で行った」と述べている点と記録値が食い違っているだけであり、**本文の是正はタスク 7.1／完了時にまとめて判断する**（Implementation Notes の「リストの SHA256 は補助証跡・正規の witness は `Compare-TestLists.ps1` の照合結果」に従い、本タスクの witness は §19.5 (c) の `RESULT: PASS` である） |

### 19.7 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-ghost/tests/ghost/spine_e2e_test_tests.rs` | 新規（141 行・6 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_broadcast_relevance_partition.rs` | 新規（181 行・2 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_s1_boot_success.rs` | 新規（288 行・1 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_s2_connect_failure.rs` | 新規（166 行・1 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_s3_helper_liveness_detected.rs` | 新規（280 行・1 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_s4_close_handshake.rs` | 新規（278 行・1 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_s5_close_deadline.rs` | 新規（336 行・1 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_s6_full_disconnect.rs` | 新規（227 行・1 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs` | 新規（91 行・テスト 0・S7 支援モジュール） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test_s7_second_boot_record_present.rs` | 新規（241 行・1 テスト） |
| `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` | 末尾の 10 テストモジュールブロックを接続宣言 10 組へ置換（共有 fixture L1-314 は無変更・2,574 → 354 行） |
| `verification/notes.md` | 本節（§19）を追記 |

コミットは要件 7.1 に従い **`areka-ghost` 統合テストツリーの 1 コミット**（同クレートのライブラリ側＝タスク 3.4 とは別コミット）とする。
これで design §File Structure Plan の `crates/areka-ghost`（`src/` 3 本＋`tests/` 1 本）は全数着地した。

## 20. クレート単位のテスト分離とテーマ分割: `areka-kanade`（タスク 4.1・要件 1.1 / 1.3 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `7c57594`（タスク 3.5 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 触ったのは下表の本番 6 ファイル ＋ 新規テストファイル 12 本 ＋ `verification/mapping/areka-kanade.csv` ＋ 本ファイルのみ。
  `crates/areka-kanade/tests/**`（統合テストツリー）・`Cargo.toml`・他クレート・spec 本体ドキュメントは **1 行も変更していない**（`git status --porcelain -uall` で確認）

### 20.1 移設した 6 ファイル（design §File Structure Plan の `crates/areka-kanade` と完全一致）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 新テストファイル | 新ファイル行 | テスト数 | 本番 残行 |
|---|---:|---|---|---|---:|---:|---:|
| `src/schedule/steady.rs` | 3,286 | `tests`（904-3286・テストコード 2,383） | **テーマ分割 ×3 ＋共有ヘルパ** | `steady_test_support.rs` ／ `steady_flow_tests.rs` ／ `steady_choice_tests.rs` ／ `steady_choice_timeout_tests.rs` | 106 / 736 / 723 / 831 | 0 / 27 / 20 / 21 | 918 |
| `src/schedule/mod.rs` | 2,176 | `tests`（670-1554・885）＋`log_firing_tests`（1567-2176・610） | 個別ファイル化のみ | `schedule_tests.rs` ／ `schedule_log_firing_tests.rs` | 882 / 607 | 33 / 33 | 687 |
| `src/schedule/boot.rs` | 1,406 | `tests`（289-1406・1,118） | **テーマ分割 ×2 ＋共有ヘルパ** | `boot_test_support.rs` ／ `boot_sequence_tests.rs` ／ `boot_reply_branch_tests.rs` | 46 / 556 / 518 | 0 / 8 / 12 | 299 |
| `src/actor.rs` | 1,318 | `tests`（371-1318・948） | 単純移設 | `actor_tests.rs` | 945 | 22 | 373 |
| `src/shiori/real.rs` | 903 | `tests`（281-903・623） | 単純移設 | `shiori/real_tests.rs` | 620 | 17 | 283 |
| `src/schedule/events.rs` | 993 | `tests`（411-993・583） | 単純移設 | `schedule/events_tests.rs` | 580 | 28 | 413 |

- 6 ファイルとも移設前の行範囲は `target_inventory.csv` / `scan_raw.csv` の実測（`crates/areka-kanade` の 6 行）と**完全一致**（ズレ 0）。テストモジュールはすべてファイル末尾に連続配置。
- **新テストファイル 12 本はすべて 1,000 行以下**（最大 `steady_choice_timeout_tests.rs` の 831 行）。**僅少超過で単一維持したファイルは 0 件**（§7.4 への追記は不要）。
- 6 ファイルとも非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は 1 件も存在しない（`scan_raw.csv` の当該 6 行の `nonmod_count` が 0）。
- **`schedule/mod.rs` の stem はモジュール root 規則により親ディレクトリ名 `schedule`**（design §移設方式の裁定）。よって `schedule/schedule_tests.rs`・`schedule/schedule_log_firing_tests.rs` となる。
- `log_firing_tests` に付いた `///` doc コメント（移設前 `mod.rs:1556-1566`・11 行）は **Implementation Notes の §14.3 規則どおり接続宣言側へ残置**した（現 `mod.rs:674-684`）。移設前後でバイト一致を確認済み（§20.4 (f)）。
- `#[cfg(test)]` 行は §13〜§19 と同じく**元位置に据え置き**、増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` の挿入は steady 11・boot 8・mod 4・actor 2・real 2・events 2 の計 29 行／削除 7,138 行）。
- 同クレートの残る `src/` 6 本（`msg.rs` 388／`schedule/close.rs` 350／`schedule/choice.rs` 237／`status.rs` 143／`schedule/resources.rs` 121／`talk.rs` 18）はテストコード 500 行以下ゆえ要件 1.5 で非必須・設計判断 #10 により任意移設は行わない（無変更）。`schedule/log_capture.rs`（274 行）は親 `mod.rs:31-32` の `#[cfg(test)] pub(crate) mod log_capture;` で**既に本番ファイル外へ分離済み**＝要件 1.4 の除外対象（無変更）。
- `crates/areka-kanade/tests/**`（`choice_test.rs` 1,563 行・`common/mod.rs` 1,657 行ほか計 11 本）は `#[cfg(test)] mod` ブロックを持つのが `common/mod.rs` の `smoke`（256 行）1 件のみで 500 行以下。要件 1.1 の対象外（無変更）。

接続宣言（12 モジュールとも同一文言・design §移設方式の裁定 案 C）:

    #[cfg(test)]
    #[path = "<stem>_<モジュール名>.rs"]
    mod <モジュール名>;

### 20.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

**バナーは thematic ではなく作業時系列の見出しだった。** `steady.rs` の `// ====` バナー 5 本はいずれも
「タスク 4.1」「タスク 4.3」「タスク 4.4」「タスク 4.5」「タスク 4.6」という**タスク番号**を見出しに掲げており
（移設前 `steady.rs:1620,1688,2325,2464,2897`）、`boot.rs` の 2 本も「タスク 6.2」「タスク 5.3」である
（`boot.rs:892,1151`）。タスク 3.1〜3.3 と同じく**本番 API の継ぎ目**を第一基準に採り、
ヘルパ参照関係の全数走査で裏取りした。結果としてバナー位置は本番シームと一致したので、両者は互いの裏取りになっている。

**`steady.rs`** — 本番 API（移設前）は 3 群に分かれる:
(i) 定常運行 `on_tick`（`:675`）・`on_reply`（`:726`）・`on_talk_done`（`:827`）・`on_close_request`（`:851`）・
`begin_close`（`:871`）・`on_mouse`（`:78`）、
(ii) 選択の受領とカスケード `on_choice`（`:218`）・`on_cascade_reply`（`:352`）・`resolve_choice`（`:474`）・
`on_choice_waiting`（`:138`）・`choice_phase_label`（`:494`）、
(iii) 選択肢タイムアウト `fire_choice_timeout_if_due`（`:528`）・`on_timeout_reply`（`:593`）。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support`（`steady_test_support.rs`） | 911-913, 915-925, 927-943, 984-994, 1688-1700, 1702-1717, 1719-1727, 1729-1732, 2325-2337 | —（3 テーマから参照される 9 ヘルパ項目） | 0 |
| `flow_tests`（`steady_flow_tests.rs`） | 945-982, 996-1686 | pump ゲート表駆動（`on_tick`）・talk 調停（`on_reply`）・origin 別 reply 政策（置換／DD-6 防御）・`TalkDone` の 2 値ルーティング（`on_talk_done`）・`CloseRequest`（`on_close_request`／`begin_close`）・マウス GET 発行（`on_mouse`）・`ActiveTalk.script` の保持 | 27 |
| `choice_tests`（`steady_choice_tests.rs`） | 1734-2323, 2339-2462 | 選択確定の受領と棄却分岐（`on_choice`）・カスケード応答（`on_cascade_reply`・`resolve_choice`）・選択待ち中の実行状態導出（`on_choice_waiting`・`choice_phase_label`） | 20 |
| `choice_timeout_tests`（`steady_choice_timeout_tests.rs`） | 2464-2895, 2897-3285 | 選択肢タイムアウト（`fire_choice_timeout_if_due`・`on_timeout_reply`）・解除後の棄却・帳簿の掃除・選択起因の失敗例外（DD-12） | 21 |

**`boot.rs`** — 本番 API は `boot_start`（`:41`）／`on_reply`（`:51`）／`on_prefetch_reply`（`:131`）／
`to_baseware_version`（`:221`）／`record_pending_close`（`:277`）。テストは「起動シーケンス全体を通す群」と
「reply の枝（prefetch 応答・初回ゲート分岐）を細かく踏む群」に割れる。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support`（`boot_test_support.rs`） | 294-296, 298-301, 303-319, 321-337 | —（2 テーマから参照される 4 ヘルパ項目） | 0 |
| `sequence_tests`（`boot_sequence_tests.rs`） | 339-890 | 起動シーケンス通し（`boot_start`＋`on_reply` の主経路）・talk_id の一意性と単調性・boot 中の `CloseRequest` 保留・`to_baseware_version` の greeting 追跡 | 8 |
| `reply_branch_tests`（`boot_reply_branch_tests.rs`） | 892-1405 | username リソース照会 prefetch（`on_prefetch_reply`）と初回ゲート分岐＋epilogue 添付（`on_reply` の first_boot 枝） | 12 |

**ヘルパ参照関係による裏取り**（テストモジュール内の全ヘルパ項目 17＋13 件の参照行を全数走査。行番号は移設前）:

- `steady.rs` の**3 テーマ／2 テーマから参照される 9 項目**（→ `test_support` へ集約）:
  `config`（定義 `:911`／参照 flow 20・choice 25・timeout 34 箇所）・
  `steady_none`（`:915`／flow 14・choice 1（`:1777`））・`steady_some`（`:927`／flow 11・choice 2・timeout 2＋`steady_waiting` 内 2）・
  `assert_no_second_change`（`:984`／flow 4・timeout 1（`:2536`））・`choice_input_of`（`:1688`／choice 13・timeout 2）・
  `steady_with_ledger`（`:1702`／choice 17・timeout 14）・`expect_get_call`（`:1719`／choice 6・timeout 7）・
  `expect_ledger`（`:1729`／choice 8・timeout 5）・`status_wire`（`:2325`／choice 3・timeout 2）。
- `steady.rs` の**単一テーマ専用 8 項目**（同ファイルに残置）: flow 側＝`assert_shiori`（`:945`）・`mouse_move_input`（`:1492`）・
  `mouse_dbl_input`（`:1502`）・`active_script`（`:1620`）・`started_script`（`:1638`）／timeout 側＝`steady_waiting`（`:2475`）・
  `step_capturing`（`:2498`）・`assert_choice_invariant`（`:2905`）。参照行はいずれも自テーマの範囲にしか現れない。
- `boot.rs` の**2 テーマから参照される 4 項目**（→ `test_support`）: `config`（`:294`）・`initial`（`:298`）・
  `assert_get`（`:303`／参照 sequence 3・branch 2（`:916,1268`））・`assert_notify`（`:321`／sequence 4・branch 1（`:944`））。
- `boot.rs` の**branch 専用 9 項目**（同ファイルに残置）: `drive_to_prefetch`（`:899`）・`resource_outcome_of`（`:923`）・
  `is_username_get`（`:1127`）・`is_onfirstboot_get`（`:1135`）・`is_onboot_get`（`:1143`）・`reply`（`:1157`）・
  `config_not_first_boot`（`:1165`）・`config_with_epilogue`（`:1172`）・`start_talk_of`（`:1182`）。

**バナーの帰属（本タスクで確定した扱い）**: 本文一致検証（`RustParse.ps1:319-331`）は、直前の空行を読み飛ばしたうえで
**先行コメント行を後続項目の本文の一部**として扱う。したがって `steady.rs` の `// ====` バナー 2 本
——`:1688-1690`（「選択確定の受領検証とカスケード駆動」）と `:2325-2327`（「選択待ち中の実行状態導出」）——は
それぞれ共有ヘルパ `choice_input_of`・`status_wire` の本文に属する。**バナーだけをテーマ側へ残すと本文不一致になる**ため、
2 本とも該当ヘルパに同伴させて `steady_test_support.rs` へ移した（文言は 1 文字も変えていない）。
残る 3 本のバナー（`:1620-1626` ActiveTalk.script ／ `:2464-2472` 選択肢タイムアウト ／ `:2897-2901` 帳簿の掃除）と
`boot.rs` の 2 本（`:892-896` prefetch ／ `:1151-1155` 初回ゲート分岐）は `use` 項目または自テーマの項目に属するため、
当該テーマファイルの元位置にそのまま置いた。`use` 項目は §11.4 のとおり本文一致検証の対象外である。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: `steady_test_support.rs` の 9 関数と
`boot_test_support.rs` の 4 関数の**シグネチャ行の先頭にのみ** `pub(super)` を付与した（付与のみ・本文は無変更）。
複製は 1 件も作っていない。

**Implementation Notes の E0659 罠（タスク 3.3 §17.5）への対処**: 共有ヘルパは**明示 import** で受けた
（`use super::test_support::{…};`）。加えて誤結合の有無を実測で確認した——移設前の本番スコープ
（`steady.rs:1-903` ／ `boot.rs:1-288`）を全数走査した結果、13 個の共有ヘルパ識別子のうち
**モジュール名前空間の項目と衝突するものは 0 件**である。唯一 `config` だけが本番側に出現するが
（`steady.rs:129,144,170` ／ `boot.rs` 17 箇所）、いずれも**関数の仮引数・局所束縛の名前**であって
モジュール項目ではないため `use super::*;` が供給する余地はない（本番 `boot.rs:24` の
`fn step(state: State, input: Input, config: &KanadeConfig)` が代表例）。同一シグネチャの黙った差し替えは起き得ない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 各テーマファイルの先頭に移設元の `use` 群を置き、
そのファイルで実際に使う項目だけへ絞った（絞る前のビルドで `unused_imports` が 2 件出て要件 2.6 に反したため）。
落としたのは `boot_test_support`＝`crate::schedule::{step, ActiveTalk}`（1 行まるごと）と
`steady_choice_tests`＝`crate::talk::TalkEndReason` の 2 件のみ。
単純移設の 5 本（`actor_tests`・`real_tests`・`events_tests`・`schedule_tests`・`schedule_log_firing_tests`）は
`use` ヘッダを 1 行も変えていない。**可視性・`use` 以外の調整は 1 件も必要なかった**（要件 2.8 の追加調整 0 件）。

### 20.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 6 ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ
（行コメント・入れ子ブロックコメント・通常文字列・raw 文字列（`#` の数）・バイト文字列・
文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と
「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は、既知の唯一の該当箇所
`crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691` を**盲点 1 件**として検出し、同ファイルの
`:382`・`:429`・`:614`・`:690` を **`\` 継続 4 件**として正しく切り分けることで確認した
（実測出力: `継続行 5 件: 382,429,614,690,691 / 盲点該当 1 件: 691`）。

| ファイル群 | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---:|---:|
| 移設前の本番 6 ファイル（`7c57594` から `git show`） | **0** | **0** |
| 移設後の新テストファイル 12 本 | **0** | **0** |

**結論: 担当 6 ファイルには複数行にまたがる文字列リテラルが 1 件も存在せず、§11.4 第 1 の盲点の該当行は 0 件。**
例外処理は不要だった。

### 20.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 7c57594 -OriginalPath <本番> -RelocatedPath "<新テスト群>" -Detail`

| 対象 | 出力 | exit |
|---|---|---:|
| `schedule/steady.rs` → 4 ファイル | `MATCH: test fn 68=68 / helper item 17=17 / mod block 1 / files 4` | 0 |
| `schedule/mod.rs` → 2 ファイル | `MATCH: test fn 66=66 / helper item 16=16 / mod block 2 / files 2` | 0 |
| `schedule/boot.rs` → 3 ファイル | `MATCH: test fn 20=20 / helper item 13=13 / mod block 1 / files 3` | 0 |
| `actor.rs` → 1 ファイル | `MATCH: test fn 22=22 / helper item 10=10 / mod block 1 / files 1` | 0 |
| `shiori/real.rs` → 1 ファイル | `MATCH: test fn 17=17 / helper item 14=14 / mod block 1 / files 1` | 0 |
| `schedule/events.rs` → 1 ファイル | `MATCH: test fn 28=28 / helper item 6=6 / mod block 1 / files 1` | 0 |

6 本とも exit **0**（引数不正の 2 ではない。exit 2 は `-OriginalPath` にパス誤りを与えた対照実行でのみ発生する）。

**(a2) 行単位の分類と多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、
それとは独立に、移設した**全行**を「(a) ちょうど −4 スペース」「(b) バイト同値（空行）」へ分類し、
どちらでもない行を全件提示させた。

| 新ファイル | −4 スペース行 | 空行 | 非適合行 | 総行（＝上記＋新設ヘッダ） |
|---|---:|---:|---:|---:|
| `actor_tests.rs` | 822 | 123 | **0** | 945 |
| `shiori/real_tests.rs` | 567 | 53 | **0** | 620 |
| `schedule/events_tests.rs` | 542 | 38 | **0** | 580 |
| `schedule/schedule_tests.rs` | 823 | 59 | **0** | 882 |
| `schedule/schedule_log_firing_tests.rs` | 557 | 50 | **0** | 607 |
| `schedule/steady_test_support.rs` | 95 | 2 | **0** | 106 |
| `schedule/steady_flow_tests.rs` | 666 | 63 | **0** | 736 |
| `schedule/steady_choice_tests.rs` | 691 | 23 | **0** | 723 |
| `schedule/steady_choice_timeout_tests.rs` | 789 | 32 | **0** | 831 |
| `schedule/boot_test_support.rs` | 41 | 0 | **0** | 46 |
| `schedule/boot_sequence_tests.rs` | 520 | 32 | **0** | 556 |
| `schedule/boot_reply_branch_tests.rs` | 478 | 36 | **0** | 518 |

**移設した全行が例外なく「ちょうど −4 スペース」または空行**であり、非適合行は 1 行も無い。

行の多重集合突合（空行を除く。元＝移設前ブロック本体を一律 4 スペース de-indent した行の多重集合）:

| 本番ファイル | 元 | 新 | 消えた行 | 増えた行 | 内訳 |
|---|---:|---:|---:|---:|---|
| `actor.rs` | 822 | 822 | **0** | **0** | 完全一致（`use` も含め 1 行も動かしていない） |
| `shiori/real.rs` | 567 | 567 | **0** | **0** | 完全一致 |
| `schedule/events.rs` | 542 | 542 | **0** | **0** | 完全一致 |
| `schedule/mod.rs`（`tests`） | 823 | 823 | **0** | **0** | 完全一致 |
| `schedule/mod.rs`（`log_firing_tests`） | 557 | 557 | **0** | **0** | 完全一致 |
| `schedule/steady.rs` | 2,245 | 2,262 | 9 | 26 | 消 = `pub(super)` を付ける前の共有ヘルパ 9 シグネチャ ／ 増 = `pub(super)` 付き 9 行 ＋ 新設・複製された `use` 17 行（`use super::*;` ×3・`use crate::msg::ShioriCall;` ×2・`use crate::schedule::step;` ×2・`use crate::talk::TalkEndReason;` ×1・`use super::test_support::{…}` 3 組 9 行） |
| `schedule/boot.rs` | 1,041 | 1,046 | 4 | 9 | 消 = `pub(super)` を付ける前の共有ヘルパ 4 シグネチャ ／ 増 = `pub(super)` 付き 4 行 ＋ `use super::*;` ×2・`use crate::schedule::{step, ActiveTalk};` ×1・`use super::test_support::{…}` ×2 |

差分はすべて要件 2.4 が明示的に許容する調整（可視性付与・`use` の追加／複製）だけで説明がつき、
説明のつかない行は 0 件である。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-kanade.csv`

    PASS: 全単射 OK / 行数 88 / 相異なる old_fqn 88 / 相異なる new_fqn 88 / フラグメント 1
      - areka-kanade.csv: 88 行

exit 0。88 行の内訳は `schedule::steady::tests::*` → `flow_tests` 27／`choice_tests` 20／`choice_timeout_tests` 21 の計 68 行と、
`schedule::boot::tests::*` → `sequence_tests` 8／`reply_branch_tests` 12 の計 20 行。`reason` は全行 `theme_split`、
末尾セグメント（関数識別子）は旧新で同一。**`actor.rs`・`shiori/real.rs`・`schedule/events.rs`・`schedule/mod.rs` は
完全修飾名が変わらないため行を持たない**（`actor::tests` 22・`shiori::real::tests` 17・`schedule::events::tests` 28・
`schedule::tests` 33・`schedule::log_firing_tests` 33 は移設前後で同名）。
移設前 `before_default.txt` に `schedule::steady::tests::` が **68 行**・`schedule::boot::tests::` が **20 行**実在し、
対応表の `old_fqn` 88 件すべてがそこに存在することを照合済み（不在 0）。
既存フラグメントとの結合検証（`-Path $V/mapping`）も
`PASS: 全単射 OK / 行数 340 / 相異なる old_fqn 340 / 相異なる new_fqn 340 / フラグメント 5` で exit 0（キー衝突なし）。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list` → stdout のみ → `: test$` 抽出 →
`$arr = [string[]]@(…)` へ型付け → `[Array]::Sort($arr, [System.StringComparer]::Ordinal)` →
UTF-8 BOM 無し・CRLF・末尾改行 1 つ・重複行を残す）でリストを採取し、
コミット済み `before_default.txt` と**タスク 3.1〜4.1 の 5 フラグメント全部**を渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after_default_task41.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 340 行 (5 ファイル) / 適用 340 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 340 行・未使用 0 行**。移設後リストの SHA256 は
`8B29C296097FDEC7BC897944641F17B5D32CB724C2F56C0115DEEEF46A9DFD34`
（§10.2 手順 3〜5 の形式。中間リストファイル自体はコミットしない）。

**整列器の較正（Implementation Notes の ⚠ 項目）**: コミット済みファイルのハッシュ照合では整列器が動かないため、
**同一の未整列生出力（4,790 行）を序数と `Sort-Object` の 2 通りに整列**して digest が割れることを先に確かめた:
序数 `8B29C296…FD34` ／ `Sort-Object` `5B8A9CC0…F882` ／ **1,806 位置が相違・多重集合は同一**。
§10.2 の実測（1,806 位置）と一致しており、序数比較器が実際に働いていることの直接証跡である。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、
対応表を**使わない**多重集合の対称差で確かめた。

| 検査 | 結果 |
|---|---|
| `before_default.txt` と移設後リストの対称差（対応表なし）: 消えた行 / 現れた行 / 全フラグメント行数 | **340 / 340 / 340**（三者一致） |
| うち本タスクぶん（`schedule::(steady\|boot)::tests::` の消滅 ／ 5 新モジュールの出現） | **88 / 88**（自クレートの対応表 88 行と一致） |
| 消えた行がすべて `old_fqn` に在るか | **True**（例外 0） |
| 現れた行がすべて `new_fqn` に在るか | **True**（例外 0） |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |
| `old_fqn` と `new_fqn` が同一の行（＝変わっていない名前を載せた水増し） | **0** |
| 末尾セグメント（関数識別子）が旧新で相違する行 | **0** |
| `reason` が `theme_split` 以外の行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。

移設後リストのモジュール別内訳（`cargo test -p areka-kanade --lib -- --list`・計 279）:
`actor::tests` 22 ／ `msg::tests` 14 ／ `schedule::boot::reply_branch_tests` 12 ／ `schedule::boot::sequence_tests` 8 ／
`schedule::choice::tests` 19 ／ `schedule::close::tests` 11 ／ `schedule::events::tests` 28 ／
`schedule::log_firing_tests` 33 ／ `schedule::resources::tests` 8 ／ `schedule::steady::choice_tests` 20 ／
`schedule::steady::choice_timeout_tests` 21 ／ `schedule::steady::flow_tests` 27 ／ `schedule::tests` 33 ／
`shiori::real::tests` 17 ／ `status::tests` 5 ／ `talk::tests` 1。
移設前の `schedule::steady::tests` 68・`schedule::boot::tests` 20 と本数が一致する（27+20+21=68／8+12=20）。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-kanade --no-fail-fast` → **exit 0**。
**326 passed / 0 failed / 0 ignored**（lib 279 ＋ 統合テスト（`tests/kanade.rs` ツリー）47 ＋ doctest 0）。
移設前の独立導出値と一致する: 移設前コミット `7c57594` の `git show` に対する `#[test]` 属性行の全数は
`src/` 12 ファイルで **279**（actor 22・msg 14・boot 20・choice 19・close 11・events 28・mod 66・resources 8・
steady 68・real 17・status 5・talk 1）、`tests/` ツリー 10 ファイルで **47**（boot 1・choice 10・close 7・
common 4・failure 4・full_run 1・mouse 11・prefetch 2・real_helper 1・steady 6）。
環境変数ゲートつきの `tests/kanade/real_helper_test.rs` の 1 本も移設前と同じく passed（ignored ではない）。
**統合テストツリーは無変更のまま 47 本すべて緑である。**

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-kanade --all-targets` → exit 0・**警告 0 件**
（`areka-kanade` に帰属する警告は移設前基準値でも 0 件の割当）。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。ユニット別 generated 件数（7 ユニット）も多重集合として一致する。

**(f) 本番本体の無変更** — 移設前コミット `7c57594` の各本番ファイルの先頭〜旧 `#[cfg(test)]` 行までを
現作業ツリーと逐行突合し、**6 ファイルとも不一致 0**（actor 1-371 ／ real 1-281 ／ events 1-411 ／
mod 1-670 ／ steady 1-904 ／ boot 1-289）。加えて `mod.rs` が保存した `log_firing_tests` の
`///` doc コメント＋空行＋`#[cfg(test)]`（移設前 1555-1567 → 現 673-685・13 行）も**逐行で不一致 0**。

| ファイル | `git diff --numstat` |
|---|---|
| `schedule/steady.rs` | 挿入 11 ／ 削除 2,379 |
| `schedule/boot.rs` | 挿入 8 ／ 削除 1,115 |
| `schedule/mod.rs` | 挿入 4 ／ 削除 1,493 |
| `actor.rs` | 挿入 2 ／ 削除 947 |
| `shiori/real.rs` | 挿入 2 ／ 削除 622 |
| `schedule/events.rs` | 挿入 2 ／ 削除 582 |

6 ファイル合計 `6 files changed, 29 insertions(+), 7138 deletions(-)`。

**(g) 完了状態の直接確認** — 6 本番ファイルに残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言のみ:
`steady.rs:904-918`（4 モジュール）／`boot.rs:289-299`（3 モジュール）／`mod.rs:670-672,685-687`（2 モジュール・
既存の `mod.rs:31-32` の `log_capture` 宣言は要件 1.4 の除外分で無変更）／`actor.rs:371-373` ／
`shiori/real.rs:281-283` ／ `schedule/events.rs:411-413`。**`#[test]` は 6 ファイルとも 1 件も残っていない。
テストモジュール本体は 1 行も残っていない。**

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 19 パスのみ:
変更 6 本（本番ファイル）＋未追跡 13 本（新テストファイル 12 本＋`verification/mapping/areka-kanade.csv`）。
**`crates/areka-kanade/tests/` 配下の差分は 0 件**。他クレート・`Cargo.toml`・`tasks.md`・spec 本体ドキュメントも無変更。

### 20.5 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（`areka-kanade` の `src/` 6 ファイル・221 テスト・テストコード 7,150 行）では、修正を要する
壊れたテスト・不正なテストは 1 件も発見しなかった。所有 spec への送付所見は 0 件である。**

以下は「調べたが問題なし／既存のまま据え置き」と確定した記録（次に触る者が同じ調査を繰り返さないための控え。是正は行わない）:

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | `static` 項目 / `thread_local!` / `std::env::set_var` / `unsafe` / `std::thread::sleep` / `#[ignore]` / `OnceLock` / `#[should_panic]` / `Instant::now` / `SystemTime` / ファイル入出力の使用 | 担当 12 テストファイル全域 | **0 件**（全走査で確認）。`static` の文字列一致 8 件はすべて `&'static str` のライフタイム表記であって項目ではない（`actor_tests.rs:503,610`・`schedule_tests.rs:368,394,423`・`real_tests.rs:21` ほか）。プロセスグローバルな可変状態も一時ファイルもテスト側には無く、時刻はすべて `MonotonicMs` の注入値である |
| 2 | プロセスグローバルな tracing subscriber（leak された interest-keeper） | `crates/areka-kanade/src/schedule/log_capture.rs:149,159`（本タスクの担当ファイルではない・`INTEREST_KEEPER`）。利用は `schedule_tests.rs`・`schedule_log_firing_tests.rs`・`steady_choice_timeout_tests.rs` の `capture` 呼出 | **問題なし・記録のみ（是正しない）**。並列負荷下で `Interest::never` が焼き付く確率欠陥を根治するために意図して常駐させているもので、モジュール doc（`:10-42`）に不変条件（「本モジュールより先に別のグローバル subscriber を設定してはならない」）と違反時の大声 panic まで明記されている。テスト間の状態汚染ではなく汚染の**予防**機構であり、本 spec でも `test-cage-determinism` でも撤去対象ではない。`areka-ghost` の `test_log_capture.rs`（§18.5 #5）と同型 |
| 3 | 要件 1.4 の除外ファイル `schedule/log_capture.rs`（274 行） | `crates/areka-kanade/src/schedule/mod.rs:31-32` の `#[cfg(test)] pub(crate) mod log_capture;` | **除外・無変更**。既に本番ファイル外へ分離済みで `#[cfg(test)] mod` ブロックを持たない。`excluded_inventory.csv` の登載どおり |
| 4 | 統合テストツリーの巨大ファイル（`tests/kanade/choice_test.rs` 1,563 行・`common/mod.rs` 1,657 行・`mouse_test.rs` 1,116 行・`close_test.rs` 1,009 行） | `crates/areka-kanade/tests/kanade/**` | **対象外・記録のみ**。`#[cfg(test)] mod` ブロックを持たない（`common/mod.rs` の `smoke` 256 行のみが該当し 500 行以下）ため要件 1.1 の「同居テストモジュールの外出し」という操作が定義できない。design §Non-Goals が明示的に対象外としているカテゴリ（`choice_tests.rs` と同型） |
| 5 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | — | **0 件**。共有ヘルパ 13 項目への `pub(super)` 付与と `use` の絞り込み（2 件）だけで通った。§17.5 #4 が警告した同名 shadow ヘルパによる E0659 は本クレートには存在しない（§20.2 の全数照合で確認） |

### 20.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-kanade/src/actor_tests.rs` | 新規（945 行・22 テスト・単純移設） |
| `crates/areka-kanade/src/shiori/real_tests.rs` | 新規（620 行・17 テスト・単純移設） |
| `crates/areka-kanade/src/schedule/events_tests.rs` | 新規（580 行・28 テスト・単純移設） |
| `crates/areka-kanade/src/schedule/schedule_tests.rs` | 新規（882 行・33 テスト・個別ファイル化） |
| `crates/areka-kanade/src/schedule/schedule_log_firing_tests.rs` | 新規（607 行・33 テスト・個別ファイル化） |
| `crates/areka-kanade/src/schedule/steady_test_support.rs` | 新規（106 行・共有ヘルパ 9 項目） |
| `crates/areka-kanade/src/schedule/steady_flow_tests.rs` | 新規（736 行・27 テスト） |
| `crates/areka-kanade/src/schedule/steady_choice_tests.rs` | 新規（723 行・20 テスト） |
| `crates/areka-kanade/src/schedule/steady_choice_timeout_tests.rs` | 新規（831 行・21 テスト） |
| `crates/areka-kanade/src/schedule/boot_test_support.rs` | 新規（46 行・共有ヘルパ 4 項目） |
| `crates/areka-kanade/src/schedule/boot_sequence_tests.rs` | 新規（556 行・8 テスト） |
| `crates/areka-kanade/src/schedule/boot_reply_branch_tests.rs` | 新規（518 行・12 テスト） |
| 上記に対応する本番 6 ファイル | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体は無変更） |
| `verification/mapping/areka-kanade.csv` | 新規（88 行・全単射検証済み） |
| `verification/notes.md` | 本節（§20）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-kanade`）とする。

## 21. クレート単位のテスト分離とテーマ分割: `areka-emo-present`（小規模 3 ファイル）（タスク 4.2・要件 1.1 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `60e0b22`（タスク 4.1 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 触ったのは下表の本番 3 ファイル ＋ 新規テストファイル 6 本 ＋ `verification/mapping/areka-emo-present.csv` ＋ 本ファイルのみ。
  **`crates/areka-emo-present/src/presenter.rs`（テストコード 4,375 行）はタスク 4.3 の担当ゆえ 1 行も触っていない**。
  `crates/areka-emo-present/tests/`・`Cargo.toml`・他クレート・spec 本体ドキュメントも無変更（`git status --porcelain -uall` で確認）

### 21.1 移設した 3 ファイル（design §File Structure Plan の `crates/areka-emo-present` の 4 本のうち `presenter.rs` を除く 3 本）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 新テストファイル | 新ファイル行 | テスト数 | 本番 残行 |
|---|---:|---|---|---|---:|---:|---:|
| `src/balloon.rs` | 2,264 | `tests`（633-2264・テストコード 1,632） | **テーマ分割 ×3 ＋共有ヘルパ** | `balloon_test_support.rs` ／ `balloon_series_tests.rs` ／ `balloon_model_tests.rs` ／ `balloon_target_tests.rs` | 197 / 685 / 338 / 418 | 0 / 24 / 6 / 7 | 644 |
| `src/cache.rs` | 1,100 | `tests`（194-1100・907） | 単純移設 | `cache_tests.rs` | 904 | 16 | 196 |
| `src/scale.rs` | 876 | `tests`（228-876・649） | 単純移設 | `scale_tests.rs` | 646 | 18 | 230 |

- 3 ファイルとも移設前の行範囲は `target_inventory.csv` / `scan_raw.csv` の実測（`crates/areka-emo-present` の該当 3 行）と**完全一致**（ズレ 0）。テストモジュールはすべてファイル末尾に連続配置・1 ファイル 1 モジュール。
- **新テストファイル 6 本はすべて 1,000 行以下**（最大 `cache_tests.rs` の 904 行）。**僅少超過で単一維持したファイルは 0 件**（§7.4 への追記は不要）。
- 3 ファイルとも非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は 1 件も存在しない（`scan_raw.csv` の当該 3 行の `nonmod_count` が 0）。`scale.rs:21` の `#[cfg(test)]` はモジュール doc コメント本文中の文字列であって属性ではない。
- テストモジュールに `///` doc コメントは付いていない（Implementation Notes の §14.3 規則は本タスクでは発動しない）。
- `#[cfg(test)]` 行は §13〜§20 と同じく**元位置に据え置き**、増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` の挿入は balloon 11・cache 2・scale 2 の計 15 行／削除 3,185 行）。
- 移設先 4 ファイルの合計 1,638 行は、移設前ブロック本体 1,629 行 − 元の `use` ヘッダ 6 行 ＋ 各テーマファイルの新ヘッダ 15 行 で説明がつく（項目間の空行 20 本は元の配置のまま保存）。
- 同クレートの残る `src/` は `presenter.rs`（**タスク 4.3 の担当**）と、テストコード 500 行以下ゆえ要件 1.5 で非必須・設計判断 #10 により任意移設を行わない 4 本（`chain.rs` 159／`mount.rs` 198／`command.rs` 92／`lib.rs` 0）である（いずれも無変更）。`tests/swapchain_spike.rs`（273 行）は `#[cfg(test)] mod` ブロックを持たず要件 1.1 の対象外（無変更）。

接続宣言（6 モジュールとも同一文言・design §移設方式の裁定 案 C）:

    #[cfg(test)]
    #[path = "<stem>_<モジュール名>.rs"]
    mod <モジュール名>;

### 21.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

**本ファイルのバナーは作業時系列ではなく thematic だった**（タスク 4.1 §20.2 とは逆）。`balloon.rs` の
`// ──` バナー 7 本はいずれも「檻 1: scope→接頭辞優先連鎖の導出」「檻 2-5: 面 ID 単位の連鎖探索」
「檻 6: 採用接頭辞からの面別上書きファイル名導出」「公開 API `resolve_balloon_faces`」「檻 7: R6.2 warn の発火条件」
「檻 8: scope 別バルーン定義の 2 層マージ」「後方互換の非回帰」という**対象の本番項目名を掲げた見出し**であり
（移設前 `balloon.rs:929,1056,1293,1340,1443,1714,2063`）、本番 API の継ぎ目と 1 対 1 に対応していた。
先行タスクと同じく**本番 API の継ぎ目を第一基準**に採り、ヘルパ参照関係の全数走査で裏取りした。

**本番 API（移設前 `balloon.rs:1-632`）は 3 群に分かれる**:
(i) **系列解決** `SeriesFamily`（`:62`）・`BALLOON_FAMILY`（`:72`）・`SeriesPrefix`（`:80`）・`ChainTier`（`:89`）・
`prefix_chain`（`:143`）・`ResolvedFace`（`:191`・`override_file_name` は `impl`（`:203`）内）・`face_id_of`（`:233`）・
`select_faces`（`:265`）・`enumerate_file_names`（`:309`）・`resolve_balloon_faces`（`:358`）、
(ii) **scope 別バルーン定義の 2 層マージ** `read_decoded`（`:417`）・`read_descript_layer`（`:425`）・
`read_face_override_layer`（`:446`）・`load_scope_balloon_model`（`:499`）、
(iii) **表示ターゲット構築** `synthetic_surfaces_txt`（`:541`）・`build_balloon_target`（`:566`）・
`build_balloon_target_from_faces`（`:585`）。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support`（`balloon_test_support.rs`） | 642-684, 1443-1576, 1714-1726 | —（2〜3 テーマから参照される 3 群の共有ヘルパ） | 0 |
| `series_tests`（`balloon_series_tests.rs`） | 737-785, 929-1054, 1056-1291, 1293-1338, 1340-1441, 1578-1589, 1591-1694 | 系列を明示した面判定・接頭辞優先連鎖の導出（`prefix_chain`・`SeriesFamily` 表データ）・面 ID 単位の連鎖探索（`select_faces`・`face_id_of`）・採用接頭辞からの上書きファイル名導出（`ResolvedFace::override_file_name`）・公開解決 API（`resolve_balloon_faces`）・R6.2 縮退 warn の発火条件 | 24 |
| `model_tests`（`balloon_model_tests.rs`） | 1728-1736, 1738-2061 | scope 別バルーン定義の 2 層マージ（`load_scope_balloon_model`＋`read_*_layer`）——per-scope 実値・未指定キーの継承・上書き層不在の `debug!` 縮退・不在以外の I/O エラーの `warn!`・基層読取失敗・確定値の `info!` 記録 | 6 |
| `target_tests`（`balloon_target_tests.rs`） | 686-689, 691-735, 787-830, 832-870, 872-927, 1696-1712, 2063-2143, 2145-2263 | synthetic surfaces.txt の転記一致（`synthetic_surfaces_txt`）・parse→bake→World の全経路構築（`build_balloon_target`・`build_balloon_target_from_faces`）・面 0 不在の `EmptyComposition` 縮退・本仕様適用前の固定接頭辞列挙（神託）との後方互換非回帰 | 7 |

**design の初期見積 `×約 2` に対し `×3 ＋ 共有ヘルパ` を採った理由**: design は
「他 22 モジュールのテーマ名は実装時に各モジュールの内容から決定し、旧→新テスト名対応表に記録する」として
テーマ名・本数を実装時裁定に委ねており、`×約 2` は概算である。本ファイルの本番 API は上記のとおり
**3 つの継ぎ目に明確に割れており**、2 分割にすると (ii) 定義マージ と (iii) ターゲット構築 という
互いに独立した継ぎ目を、行数を満たす目的だけで 1 ファイルへ融合することになる（両者が共有するヘルパは
`emo2_balloon_root` と `TempDir` の 2 件のみで、いずれも 3 テーマ横断ゆえ `test_support` へ出る）。
design §テーマ分割ポリシーの判定基準は「テストモジュール内部の既存構造（対象関数のまとまり・コメントバナー・
ヘルパの参照関係）に従い」であり、本数ではない。3 分割はバナー 7 本・本番 API 3 群・ヘルパ参照グラフの
3 者すべてと整合する唯一の分け方である。

**ヘルパ参照関係による裏取り**（テストモジュール内の全ヘルパ項目 23 件の参照行を全数走査。行番号は移設前）:

- **3 テーマから参照される群**（→ `test_support`）: `TEMP_COUNTER`（`:646`）＋`TempDir`（`:649`・`impl` `:653`・
  `impl Drop` `:680`）——参照は series 7 箇所（`:1346,1364,1382,1408,1600,1645,1666`）・model 3 箇所
  （`:1863,1931,1973`）・target 4 箇所（`:793,839,1700,2157`）。
- **2 テーマから参照される群 その 1**（→ `test_support`）: ログ捕捉ハーネス 6 項目——`CapturedEvent`（`:1455`）・
  `impl CapturedEvent`（`:1460`）・`FieldGrab`（`:1472`）・`impl Visit`（`:1474`）・`CaptureSubscriber`（`:1483`）・
  `impl Subscriber`（`:1485`）・`InterestProbe`（`:1514`）・`impl Subscriber`（`:1516`）・`ensure_interest_probes`（`:1546`）・
  `capture_events`（`:1565`）。`capture_events` の参照は series 3 箇所（`:1605,1649,1671`）・model 4 箇所
  （`:1874,1941,1980,2019`）。`CapturedEvent` の型名・`.level`・`.fields`・`.field()` は series（`:1583,1584,1623,1625,1630,1635,1656,1689,1693`）と
  model（`:1890,1893,1904,1910,1917,1946,1949,1956,1962,1989,1992,2003,2021,2024,2025,2033,2039,2043,2049,2053,2054,2055,2057`）の双方から直に触られる。
- **2 テーマから参照される群 その 2**（→ `test_support`）: `emo2_balloon_root`（`:1723`）——参照は model 3 箇所
  （`:1749,1818,2017`）・target 2 箇所（`:883,2221`）。
- **単一テーマ専用 10 項目**（当該テーマファイルに残置）: series 側＝`chain_pairs`（`:932`・参照 5）・
  `selected`（`:1059`・参照 12）・`default_fallback_warns`（`:1579`・参照 2）／
  model 側＝`emo2_face0`（`:1729`・参照 5・すべて model 内）／
  target 側＝`opaque_1x1`（`:687`・参照 3）・`pre_spec_faces`（`:2078`・参照 3）・`opaque`（`:2096`・参照 2）・
  `scope_digest`（`:2111`・参照 4）。参照行はいずれも自テーマの範囲にしか現れない。

**バナーの帰属（§20.2 で確定した扱いの適用）**: 本文一致検証（`RustParse.ps1:319-331`）は、直前の空行と
コメント行を読み飛ばして最初のコード行を探すため、**先行コメント塊は後続項目の本文の一部**になる。
したがって共有ヘルパの直前にあるバナー 3 本——`:642-644`（一時ディレクトリ）・`:1443-1451`（檻 7: R6.2 warn の
発火条件）・`:1714-1715`（檻 8: 2 層マージ）——はそれぞれ `TEMP_COUNTER`・`CapturedEvent`・`emo2_balloon_root` の
本文に属する。**バナーだけをテーマ側へ残すと本文不一致になる**ため、3 本とも該当ヘルパに同伴させて
`balloon_test_support.rs` へ移した（文言は 1 文字も変えていない。現 `:5-7`・`:50-58`・`:186-187`）。
残る 4 本（`:929` 檻 1／`:1056` 檻 2-5／`:1293` 檻 6／`:1340` 公開 API／`:2063-2070` 後方互換）は
自テーマの項目に属するため、当該テーマファイルの元位置にそのまま置いた。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: `balloon_test_support.rs` の
`TempDir`（struct＋`path` フィールド＋`new`/`path`/`touch`/`write` の 4 メソッド）・
`CapturedEvent`（struct＋`level`/`fields` の 2 フィールド＋`field` メソッド）・`capture_events`・`emo2_balloon_root` の
**計 12 行の先頭にのみ** `pub(super)` を付与した（付与のみ・本文は無変更）。`TEMP_COUNTER`・`FieldGrab`・
`CaptureSubscriber`・`InterestProbe`・`ensure_interest_probes` は `test_support` の内部からしか参照されないため
可視性を変えていない。**複製は 1 件も作っていない**（`ITEM-EXTRA` 回避・Implementation Notes の集約規則どおり）。

**Implementation Notes の E0659 罠（タスク 3.3 §17.5）への対処**: 共有ヘルパは**明示 import** で受けた
（`use super::test_support::{…};`）。加えて誤結合の有無を実測で確認した——移設前の本番スコープ
（`balloon.rs:1-632`）を全数走査した結果、`test_support` が持つ 9 識別子
（`TEMP_COUNTER`・`TempDir`・`CapturedEvent`・`FieldGrab`・`CaptureSubscriber`・`InterestProbe`・
`ensure_interest_probes`・`capture_events`・`emo2_balloon_root`）のうち、
**本番モジュールの名前空間と衝突するものは 0 件**である（各識別子の本番スコープ出現数がすべて 0）。
同一シグネチャの黙った差し替えは起き得ない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 各テーマファイルの先頭に `use super::*;` と、そのファイルで実際に
使う項目だけの `use` を置いた（絞る前のビルドで `unused_imports` が 3 件出て要件 2.6 に反したため）。
落としたのは `series_tests`＝`areka_emo_atlas::MemoryDecoder`（`super::*` 経由で本番の import が届くため不要）・
`model_tests`＝`std::path::PathBuf`・`target_tests`＝`std::path::PathBuf` の 3 件のみで、
移設前ヘッダの 4 本（`areka_emo_atlas::{ElementId, MemoryDecoder}`・`areka_parsers::shell::parse`・
`std::path::PathBuf`・`std::sync::atomic::{AtomicU32, Ordering}`）は**いずれも 1 本も失われず**、
必要なテーマファイルへ 1 回ずつ配置されている。単純移設の 2 本（`cache_tests`・`scale_tests`）は
`use` ヘッダを 1 行も変えていない。**可視性・`use` 以外の調整は 1 件も必要なかった**（要件 2.8 の追加調整 0 件）。

### 21.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 3 ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ
（行コメント・入れ子ブロックコメント・通常文字列・raw 文字列（`#` の数）・バイト文字列・
文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と
「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は、既知の唯一の該当箇所
`crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691` を**盲点 1 件**として検出し、同ファイルの
`:382`・`:429`・`:614`・`:690` を **`\` 継続 4 件**として正しく切り分けることで確認した
（実測出力: `継続行 5 件: 382,429,614,690,691 / 盲点該当 1 件: 691`）。

| ファイル群 | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---:|---:|
| 移設前の本番 3 ファイル（`60e0b22` から `git show`） | **0** | **0** |
| 移設後の新テストファイル 6 本 | **0** | **0** |
| 移設後の本番 3 ファイル | **0** | **0** |

**結論: 担当 3 ファイルには複数行にまたがる文字列リテラルが 1 件も存在せず、§11.4 第 1 の盲点の該当行は 0 件。**
例外処理は不要だった。

### 21.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 60e0b22 -OriginalPath <本番> -RelocatedPath "<新テスト群>" -Detail`

| 対象 | 出力 | exit |
|---|---|---:|
| `cache.rs` → 1 ファイル | `MATCH: test fn 16=16 / helper item 9=9 / mod block 1 / files 1` | 0 |
| `scale.rs` → 1 ファイル | `MATCH: test fn 18=18 / helper item 14=14 / mod block 1 / files 1` | 0 |
| `balloon.rs` → 4 ファイル | `MATCH: test fn 37=37 / helper item 23=23 / mod block 1 / files 4` | 0 |

3 本とも exit **0**。**引数不正の 2 と取り違えていないことを対照実行で確認した**——
`-OriginalPath crates/areka-emo-present/src/nonexistent.rs` を与えると
`fatal: path ... does not exist in '60e0b22'` を出して **exit 2** になる（不一致の 1 ではない）。

**(a2) 行単位の分類と多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、
それとは独立に、移設した**全行**を「(a) ちょうど −4 スペース」「(b) 空行」へ分類し、
どちらでもない行を全件提示させた（新設した `use` ヘッダ行は分類の対象外）。

| 新ファイル | −4 スペース行 | 空行 | 非適合行 | 新ヘッダ | 総行 |
|---|---:|---:|---:|---:|---:|
| `balloon_test_support.rs` | 159 | 22 | **12** | 4 | 197 |
| `balloon_series_tests.rs` | 637 | 45 | **0** | 3 | 685 |
| `balloon_model_tests.rs` | 311 | 24 | **0** | 3 | 338 |
| `balloon_target_tests.rs` | 380 | 33 | **0** | 5 | 418 |
| `cache_tests.rs` | 834 | 70 | **0** | 0 | 904 |
| `scale_tests.rs` | 598 | 48 | **0** | 0 | 646 |

`balloon_test_support.rs` の非適合 **12 行はすべて `pub(super)` を付与した宣言行**であり（§21.2 の可視性調整）、
要件 2.4 が明示的に許容する調整である。それ以外の移設行は例外なく「ちょうど −4 スペース」または空行。

行の多重集合突合（空行を除く。元＝移設前ブロック本体を一律 4 スペース de-indent した行の多重集合。新側は
`use` ヘッダを含む全行）:

| 本番ファイル | 元 | 新 | 消えた行 | 増えた行 | 内訳 |
|---|---:|---:|---:|---:|---|
| `cache.rs` | 834 | 834 | **0** | **0** | 完全一致（`use` も含め 1 行も動かしていない） |
| `scale.rs` | 598 | 598 | **0** | **0** | 完全一致 |
| `balloon.rs` | 1,504 | 1,510 | 12 | 18 | 消 = `pub(super)` を付ける前の 12 宣言行 ／ 増 = `pub(super)` 付き 12 行 ＋ 複製された `use super::*;` 3 行 ＋ 新設した `use super::test_support::{…};` 3 行 |

差分はすべて要件 2.4 が明示的に許容する調整（可視性付与・`use` の追加／複製）だけで説明がつき、
説明のつかない行は 0 件である。移設前ヘッダの `use` 4 本は 1 本も消えていない（消えた 12 行に `use` は無い）。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-emo-present.csv`

    PASS: 全単射 OK / 行数 37 / 相異なる old_fqn 37 / 相異なる new_fqn 37 / フラグメント 1
      - areka-emo-present.csv: 37 行

exit 0。37 行の内訳は `balloon::tests::*` → `series_tests` 24／`model_tests` 6／`target_tests` 7。
`reason` は全行 `theme_split`、末尾セグメント（関数識別子）は旧新で同一。
**`cache.rs`・`scale.rs` は完全修飾名が変わらないため行を持たない**（`cache::tests` 16・`scale::tests` 18 は移設前後で同名）。
移設前 `before_default.txt` に `balloon::tests::` が **37 行**実在し、対応表の `old_fqn` 37 件すべてが
そこに存在することを照合済み（不在 0）。既存フラグメントとの結合検証（`-Path $V/mapping`）も
`PASS: 全単射 OK / 行数 377 / 相異なる old_fqn 377 / 相異なる new_fqn 377 / フラグメント 6` で exit 0（キー衝突なし）。
**本フラグメントはタスク 4.3 が `presenter.rs` 分を追記できる状態（同一 3 列・`old_fqn` 序数順）で残してある。**

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list`（exit 0）→ stdout のみ → `: test$` 抽出 →
`$arr = [string[]]@(…)` へ型付け → `[Array]::Sort($arr, [System.StringComparer]::Ordinal)` →
UTF-8 BOM 無し・CRLF・末尾改行 1 つ・重複行を残す）でリストを採取し、
コミット済み `before_default.txt` と**タスク 3.1〜4.2 の 6 フラグメント全部**を渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after_default_task42.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 377 行 (6 ファイル) / 適用 377 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 377 行・未使用 0 行**。移設後リストの SHA256 は
`A311FC84A2A4ED828C6729521A25B122D991F529B1CF70A6756433AA9B8FBABA`
（§10.2 手順 3〜5 の形式。中間リストファイル自体はコミットしない）。

**整列器の較正（Implementation Notes の ⚠ 項目）**: コミット済みファイルのハッシュ照合では整列器が動かないため、
**同一の未整列生出力（4,790 行）を序数と `Sort-Object` の 2 通りに整列**して digest が割れることを先に確かめた:
序数 `A311FC84…BABA` ／ `Sort-Object` `FE2479FA…5D6E` ／ **1,806 位置が相違・多重集合の差は 0**。
§10.2 の実測（1,806 位置）と一致しており、序数比較器が実際に働いていることの直接証跡である。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、
対応表を**使わない**多重集合の対称差で確かめた。

| 検査 | 結果 |
|---|---|
| `before_default.txt` と移設後リストの対称差（対応表なし）: 消えた行 / 現れた行 / 全フラグメント行数 | **377 / 377 / 377**（三者一致） |
| うち本タスクぶん（`balloon::tests::` の消滅 ／ `balloon::(series\|model\|target)_tests::` の出現） | **37 / 37**（自クレートの対応表 37 行と一致） |
| 消えた行がすべて `old_fqn` に在るか | **True**（例外 0） |
| 現れた行がすべて `new_fqn` に在るか | **True**（例外 0） |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |
| `old_fqn` と `new_fqn` が同一の行（＝変わっていない名前を載せた水増し） | **0** |
| 末尾セグメント（関数識別子）が旧新で相違する行 | **0** |
| `reason` が `theme_split` 以外の行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。

移設後リストのモジュール別内訳（`cargo test -p areka-emo-present --lib -- --list`・計 127）:
`balloon::model_tests` 6 ／ `balloon::series_tests` 24 ／ `balloon::target_tests` 7 ／ `cache::tests` 16 ／
`chain::tests` 1 ／ `command::tests` 5 ／ `mount::tests` 3 ／ `presenter::tests` 47 ／ `scale::tests` 18。
移設前の `balloon::tests` 37 と本数が一致する（24+6+7=37）。`presenter::tests` 47 は**無変更**（タスク 4.3 の担当）。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-emo-present --no-fail-fast` → **exit 0**。
**128 passed / 0 failed / 0 ignored**（lib 127 ＋ 統合テスト `tests/swapchain_spike.rs` 1 ＋ doctest 0）。
移設前の独立導出値と一致する: 移設前コミット `60e0b22` の `git show` に対する `#[test]` 属性行の全数は
`src/` 8 ファイルで **127**（balloon 37・cache 16・chain 1・command 5・lib 0・mount 3・presenter 47・scale 18）、
`tests/` で **1**（swapchain_spike 1）。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-emo-present --all-targets` → exit 0・**警告 0 件**
（`areka-emo-present` に帰属する警告は移設前基準値でも 0 件の割当）。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。ユニット別 generated 件数（7 ユニット・多重集合 {1,3,3,3,4,4,4}）も一致する。

**(f) 本番本体の無変更** — 移設前コミット `60e0b22` の各本番ファイルの先頭〜旧 `#[cfg(test)]` 行までを
現作業ツリーと逐行突合し、**3 ファイルとも不一致 0**（balloon 1-633 ／ cache 1-194 ／ scale 1-228）。

| ファイル | `git diff --numstat` |
|---|---|
| `balloon.rs` | 挿入 11 ／ 削除 1,631 |
| `cache.rs` | 挿入 2 ／ 削除 906 |
| `scale.rs` | 挿入 2 ／ 削除 648 |

3 ファイル合計 `3 files changed, 15 insertions(+), 3185 deletions(-)`。

**(g) 完了状態の直接確認** — 3 本番ファイルに残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言のみ:
`balloon.rs:633-644`（4 モジュール）／`cache.rs:194-196`（1 モジュール）／`scale.rs:228-230`（1 モジュール）。
`scale.rs:21` の `#[cfg(test)]` はモジュール doc コメント本文中の文字列であって属性ではない（移設前から同一）。
**`#[test]` は 3 ファイルとも 1 件も残っていない。テストモジュール本体は 1 行も残っていない。**

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 10 パスのみ:
変更 3 本（本番ファイル）＋未追跡 7 本（新テストファイル 6 本＋`verification/mapping/areka-emo-present.csv`）。
**`crates/areka-emo-present/src/presenter.rs` の差分は 0 件**（`git diff --stat` が空）。
`crates/areka-emo-present/tests/`・他クレート・`Cargo.toml`・`tasks.md`・spec 本体ドキュメントも無変更。
新規に導入した `TODO` / `FIXME` / `TBD` は 0 件。

### 21.5 登記（要件 5.2）— 壊れたテスト・状態汚染の所見

**本タスクの範囲（`areka-emo-present` の `src/` 3 ファイル・71 テスト・テストコード 3,188 行）では、
壊れたテスト・不正なテストは 1 件も発見しなかった。所有 spec への送付所見は 1 件**（下表 #2・ハーネス重複）
**であり、これは要件 5.1 により本 spec では是正しない。**

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | プロセス寿命で常駐する tracing の interest probe dispatcher 2 個（`OnceLock` 保持・`Dispatch::new` 済み） | `crates/areka-emo-present/src/balloon_test_support.rs:121,153-163`（`InterestProbe` / `ensure_interest_probes`）ならびに同型の `crates/areka-emo-present/src/scale_tests.rs:368,398-408` | **問題なし・記録のみ（是正しない）**。callsite の interest キャッシュに `never` が焼き付く確率欠陥を潰すために意図して常駐させているもので、`enabled()` は偽・`event()` は no-op ゆえ他テストの観測へ副作用を与えない。テスト間の状態汚染ではなく汚染の**予防**機構であり、`areka-ghost` の `test_log_capture.rs`（§18.5 #5）・`areka-kanade` の `log_capture.rs`（§20.5 #2）と同型 |
| 2 | **同一クレート内に同型のログ捕捉ハーネスが 3 重に存在する**（`CapturedEvent` / `FieldGrab` / `CaptureSubscriber` の 3 点セット） | `crates/areka-emo-present/src/balloon_test_support.rs:62,79,90`（本タスクで `test_support` へ集約）／`crates/areka-emo-present/src/scale_tests.rs:304,327,338`／`crates/areka-emo-present/src/presenter.rs:3669,3678,3689`（**本タスク非担当・無変更**） | **送付所見 →`test-cage-determinism`（W6.9）**。重複の理由は移設前 `balloon.rs:1449-1451` のコメントに明記されている——「同 crate `presenter.rs` の tests に同型のものが在るが、あちらは test-local な private 型ゆえ本モジュールから参照できない。新規 dev-dependency を足さない方針ゆえ、`tracing` 本体のみで最小構成を再現する」。**テストハーネスの一本化・共有化は要件 5.1 が本 spec に禁じている**ため是正しない。なお本 spec の移設は 3 重を 3 重のまま保っている（増やしても減らしてもいない）——`balloon` 側は 1 クレート内で `test_support` へ集約したのみ。将来 `presenter.rs` のハーネスをクレート共通のテストヘルパへ持ち上げれば 3 → 1 に畳める |
| 3 | 一時ディレクトリ fixture が `std::env::temp_dir()` 配下へ実ファイルを作る | `crates/areka-emo-present/src/balloon_test_support.rs:10,17-27,44-47`（`TEMP_COUNTER` ＋ `TempDir::new` ＋ `impl Drop`） | **問題なし・記録のみ**。ディレクトリ名は `areka-emo-present-balloon-{プロセス id}-{単調カウンタ}` で並列実行でも衝突せず、`Drop` で再帰削除される。プロセス間・テスト間の共有状態にならない |
| 4 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | — | **0 件**。共有ヘルパ 4 項目（宣言行 12 本）への `pub(super)` 付与と `use` の絞り込み（3 件）だけで通った。§17.5 #4 が警告した同名 shadow ヘルパによる E0659 は本クレートには存在しない（§21.2 の全数照合で確認） |
| 5 | 要件 1.5 で非必須のファイル（`chain.rs` 159／`mount.rs` 198／`command.rs` 92 のテストコード） | `crates/areka-emo-present/src/{chain,mount,command}.rs` | **非必須・無変更**。設計判断 #10 により任意移設は行わない |

### 21.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-emo-present/src/cache_tests.rs` | 新規（904 行・16 テスト・単純移設） |
| `crates/areka-emo-present/src/scale_tests.rs` | 新規（646 行・18 テスト・単純移設） |
| `crates/areka-emo-present/src/balloon_test_support.rs` | 新規（197 行・共有ヘルパ 3 群・テスト 0） |
| `crates/areka-emo-present/src/balloon_series_tests.rs` | 新規（685 行・24 テスト） |
| `crates/areka-emo-present/src/balloon_model_tests.rs` | 新規（338 行・6 テスト） |
| `crates/areka-emo-present/src/balloon_target_tests.rs` | 新規（418 行・7 テスト） |
| 上記に対応する本番 3 ファイル（`balloon.rs`・`cache.rs`・`scale.rs`） | 末尾のテストモジュールブロックを接続宣言へ置換（本番本体は無変更） |
| `verification/mapping/areka-emo-present.csv` | 新規（37 行・全単射検証済み・タスク 4.3 が `presenter.rs` 分を追記できる状態） |
| `verification/notes.md` | 本節（§21）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-emo-present` の小規模 3 ファイル分）とする。
`presenter.rs`（タスク 4.3）は同クレートの別コミットとして続く。

## 22. クレート単位のテスト分離とテーマ分割: `areka-emo-present`（`presenter.rs`）（タスク 4.3・要件 1.1 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `91d59e0`（タスク 4.2 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 触ったのは `crates/areka-emo-present/src/presenter.rs` ＋ 新規テストファイル 8 本 ＋ `verification/mapping/areka-emo-present.csv`（追記のみ）＋ 本ファイルのみ。
  タスク 4.2 の成果物（`balloon*.rs` 4 本・`cache_tests.rs`・`scale_tests.rs` と対応する本番 3 ファイル）は **1 行も触っていない**。
  `crates/areka-emo-present/tests/`・`Cargo.toml`・他クレート・`tasks.md`・spec 本体ドキュメントも無変更（`git status --porcelain -uall` で確認）。

### 22.1 移設した 1 ファイル

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 本番 残行 |
|---|---:|---|---|---:|
| `src/presenter.rs` | 5,417 | `tests`（1043-5417・テストコード 4,375・ブロック本体 4,372） | **テーマ分割 ×7 ＋共有ヘルパ** | 1,066 |

- 移設前の行範囲は `target_inventory.csv` / `scan_raw.csv` の実測（`tests:1043-5417(4375)`）と**完全一致**（ズレ 0）。テストモジュールはファイル末尾に連続配置・1 ファイル 1 モジュール。
- 非 `mod` `#[cfg(test)]` 項目（設計判断 #3 の残置対象 40 件）は 1 件も存在しない（`scan_raw.csv` の `nonmod_count` が 0）。テストモジュールに `///` doc コメントも付いていない（§14.3 規則は発動しない）。
- `#[cfg(test)]` 行は §13〜§21 と同じく**元位置（1043 行）に据え置き**、増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` = 挿入 23 ／ 削除 4,374）。

| 新テストファイル | 行数 | テスト数 |
|---|---:|---:|
| `presenter_test_support.rs`（`test_support`） | 401 | 0 |
| `presenter_display_tests.rs`（`display_tests`） | 601 | 7 |
| `presenter_compose_input_tests.rs`（`compose_input_tests`） | 423 | 3 |
| `presenter_read_accessor_tests.rs`（`read_accessor_tests`） | 451 | 9 |
| `presenter_dpi_scale_tests.rs`（`dpi_scale_tests`） | 827 | 11 |
| `presenter_resize_report_tests.rs`（`resize_report_tests`） | 265 | 4 |
| `presenter_refresh_and_log_tests.rs`（`refresh_and_log_tests`） | 815 | 9 |
| `presenter_fractional_scale_tests.rs`（`fractional_scale_tests`） | 691 | 4 |
| **計** | **4,474** | **47** |

**新テストファイル 8 本はすべて 1,000 行以下**（最大 `presenter_dpi_scale_tests.rs` の 827 行）。**僅少超過で単一維持したファイルは 0 件**（§7.4 への追記は不要）。同クレートの既存テストファイル 6 本（タスク 4.2）も 1,000 行以下のまま無変更（最大 `cache_tests.rs` 904 行）。

**本番 `presenter.rs` は 1,066 行で 1,000 行の目安を 66 行超える**。これは本番本体 1,042 行＋接続宣言 24 行であり、**要件 4.5 と design §Non-Goals が `follow.rs`・`frame.rs` 以外の本番本体分割を明示的に禁じている**ため意図どおりである（design §Requirements Traceability 4.5 が `presenter.rs 1,042 行等は分割しない` と名指ししている）。本タスクは本番本体を 1 行も動かしていない（§22.4 (f)）。

接続宣言（8 モジュールとも同一文言・design §移設方式の裁定 案 C）:

    #[cfg(test)]
    #[path = "presenter_<モジュール名>.rs"]
    mod <モジュール名>;

### 22.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

**バナーは作業時系列ではなく thematic だった**（タスク 4.2 §21.2 と同じ・タスク 4.1 §20.2 とは逆）。
`// ──` バナー 10 本のうち **9 本が見出しの先頭に対象（本番 API 名・観測契約）を掲げており**、
タスク番号は括弧内の出所表記にとどまる——`CurrentSurfaceRead: 現サーフェス id 状態のライフサイクル固定（Task 2・…）`（`:2354`）・
`DPI 追従（k 適用の単一漏斗）: タスク 3.2／3.3 の檻`（`:2595`）・`表示成立点の状態照合＝窓寸 reconcile 報告（タスク 3.4・…）`（`:3405`）・
`表示成立点 info ログ（設計 D10・要件 6.1/6.3）の檻`（`:3657`）・`applied_scale／refresh_scale（タスク 3.5・design Flow 2）`（`:3796`）・
`要件 2.3（多層コンテンツの単一 k 一貫拡大）の実表示檻`（`:4696`）・`hit_region_client の配線と縮退の檻（タスク 3.2・…）`（`:4976`）・
フィクスチャ 2 本（`:1064` GPU/WUC ／ `:1195` ComposedSurface 生成補助）。
タスク番号を先頭に掲げるのは `task 6.3: 端数 k（5/4）の実表示・…`（`:4305`）の 1 本のみである。
先行タスクと同じく**本番 API の継ぎ目を第一基準**に採り、ヘルパ参照関係の全数走査で裏取りした。結果としてバナー位置は本番シームと一致した。

**本番 API（移設前 `presenter.rs:1-1042`）の継ぎ目**:
(i) 表示の成立と縮退 `apply`（`:332`）・`apply_show`（`:360`）・`apply_hide`（`:617`）・`apply_invalidate`（`:640`）・`read_back`（`:1013`）、
(ii) 照会契約 `TextSlotView`（`:172`・`slot`/`window`/`surface_size`/`physical_size`/`scale`）・`target_physical_size`（`:704`）、
(iii) k の再適用と報告 `applied_scale`（`:731`）・`applied_ratio`（`:744`）・`refresh_scale`（`:793`）・`take_pending_resize`（`:880`）、
(iv) 読み取りアクセサ `current_surface_id`（`:891`）・`hit_region`（`:909`）・`hit_region_client`（`:953`）——本番でも隣接 3 連である。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support` | 1064-1278, 2013-2077, 2267-2283, 4976-5062 | —（2 テーマ以上から参照される共有フィクスチャ 21 項目群） | 0 |
| `display_tests` | 1280-1863 | 表示成立と golden 一致・解決不能 id の非破壊 skip・0×0 縮退の Hidden 化・`Hide`→再表示のキャッシュ復帰・`text_slot_view` の基礎契約（`apply_show`／`apply_hide`／`read_back`／`TextSlotView`） | 7 |
| `compose_input_tests` | 1865-2011, 2079-2259, 2261-2265, 2285-2352 | 合成入力（bind 集合・pattern・面 id）の差分が `ComposeKey` を通って表示へ届くこと＝キャッシュキーの回帰防止と文字スロットの安定 | 3 |
| `read_accessor_tests` | 2354-2593, 5219-5416 | 読み取りアクセサ 3 連（`current_surface_id` のライフサイクル・`hit_region`／`hit_region_client` の値契約と正常縮退） | 9 |
| `dpi_scale_tests` | 2595-3403 | DPI 追従（政策の窓単位保持・k=2/1 の実拡大表示・照会契約 `scale`／`physical_size`／`target_physical_size` の丸め権威・k のキャッシュキー参加・`native_size` の追随） | 11 |
| `resize_report_tests` | 3405-3655 | 表示成立点の窓寸 reconcile 報告（`take_pending_resize`：変化時の報告・べき等・初回報告・失敗時の非報告） | 4 |
| `refresh_and_log_tests` | 3657-3713, 3715-3794, 3796-4303, 5063-5074, 5076-5217 | 観測ログ捕捉ハーネスとその全消費者——D10 表示成立点 info ログ・`applied_scale`／`refresh_scale`（非実行の証明にログを使う 5 本を含む）・`hit_region_client` の防御 warn の発火条件 | 9 |
| `fractional_scale_tests` | 4305-4974 | 端数 k（5/4）の実表示バイト・αマスクの寸と内容・縮小方向の追従・多層コンテンツ（bind／pattern）の単一 k 一貫拡大 | 4 |

**design の初期見積 `×約 5` に対し `×7 ＋ 共有ヘルパ` を採った理由**: design は「他 22 モジュールのテーマ名は実装時に各モジュールの内容から決定し、旧→新テスト名対応表に記録する」としてテーマ名・本数を実装時裁定に委ねており、`×約 5` は概算である。本モジュールはテストコード 4,375 行・テスト 47 本で本 spec 最大の単一ブロックであり、5 分割では 1 ファイルあたり平均 875 行・最大が確実に 1,000 行を超える。上表のとおり本番 API の継ぎ目は 7 つに割れており、バナー 10 本もその 7 つへ 1 対 1 に畳める。

**`refresh_and_log_tests` が 2 つのバナーをまたぐ理由（本タスク固有の構造的制約）**: 移設前 `presenter.rs:3689` の
`struct CaptureSubscriber(std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>>);` は**タプル構造体**であり、
利用側は `cap.0.lock()` として**フィールド 0 を直に読む**（移設前 `:3737, 4001, 4047, 4122, 4192, 4285, 5138` の 7 箇所）。
この 7 箇所は D10 ログ・`refresh_scale`・`hit_region_client` の 3 バナーにまたがる。
ハーネスを `test_support` へ出すとタプルフィールドに可視性修飾が要るが、それは
**行頭ではなく行内**（`struct CaptureSubscriber(pub(super) std::sync::Arc<…>);`）にしか書けず、
本文一致検証の正規化（§11.4・`RustParse.ps1:494` は**行頭**の `pub(...)` のみ除去する）が吸収できない。
実際に試したところ `[ITEM-MISSING]`／`[ITEM-EXTRA]` の 2 件で exit 1 になった。
名前付きフィールドの `CapturedEvent`（`level`／`fields`）は行頭付与で吸収されるが、タプルフィールドだけは構造的に吸収できない。
そこで**ハーネス（`CapturedEvent`／`FieldGrab`／`CaptureSubscriber` と 2 つの `impl`）とその全消費者を 1 モジュールに保ち、
ハーネスには可視性修飾を 1 文字も加えない**方針を採った。結果として `refresh_and_log_tests` は
「捕捉ハーネスを共有する檻の集合」という 1 つの主題で閉じており、`applied_absent_warn_count`（ハーネス依存の述語ヘルパ）も同居する。
これは要件 5.1（テストハーネスの一本化・共有化をしない）とも整合する——3 重のハーネスは 3 重のまま、位置だけを動かしている。

**ヘルパ参照関係による裏取り**（テストモジュール内の全ヘルパ項目 39 件の参照行を全数走査。行番号は移設前）:

- **多テーマから参照 → `test_support`**（21 項目）: `make_world_with_gpu`（`:1072`・7 テーマ）・`spawn_window_with_dpi`（`:1094`・8 テーマ）・
  `set_window_dpi`（`:1099`・4）・`scaled_golden`（`:1109`・3）・`ScaledGolden`（`:1127`・`scaled_golden` と fractional の 2）・
  `scaled_golden_with`（`:1143`・同 2）・`px_at`（`:1169`）・`show_ok`（`:1177`・5）・`elem`（`:1199`・4）・`surface`（`:1208`・6）・
  `shell_of`（`:1218`・6）・`build_target_assets`（`:1235`・6）・`build_two_face_assets`（`:2021`・5）・`pattern_overlay_at`（`:2271`・2）・
  `hit_coll`（`:4988`）・`build_collision_only_assets`（`:5004`）・`attach_hit_target`（`:5037`・2）・`force_current_surface`（`:5046`・2）・
  `force_applied`（`:5055`）。
  うち `px_at`・`hit_coll`・`build_collision_only_assets`・`force_applied` の 4 項目は現時点で**単一テーマからしか参照されない**が、
  いずれも複数テーマ共有の連続フィクスチャ塊（`:1064-1278` の GPU/golden 群、`:4976-5062` の当たり判定群）の内側にあり、
  かつ同じ塊の他項目（`scaled_golden_with`／`attach_hit_target`）から呼ばれるため、塊ごと `test_support` へ置いた。
- **単一テーマ専用（当該テーマファイルに残置）**: display 側＝`build_assets_with_valid_and_empty`（`:1395`）／
  compose 側＝`build_target_assets_with_bind`（`:1870`）・`build_target_assets_with_pattern`（`:2200`）・`pattern_overlay`（`:2263`）／
  dpi 側＝`build_two_sized_face_assets`（`:3193`）／refresh_and_log 側＝`CapturedEvent`（`:3669`）・`FieldGrab`（`:3678`）＋`impl Visit`（`:3680`）・
  `CaptureSubscriber`（`:3689`）＋`impl Subscriber`（`:3691`）・`has_display_success_log`（`:3802`）・`applied_absent_warn_count`（`:5064`）／
  fractional 側＝`surface_entity_of`（`:4315`）・`mask_dims`（`:4325`）・`arrangement_size`（`:4332`）・`build_alpha_varying_assets`（`:4346`）・
  `LAYERED_BIND_AT`（`:4705`）・`LAYERED_PATTERN_AT`（`:4707`）・`LAYERED_PART_SIZE`（`:4709`）・`build_layered_assets`（`:4727`）。
  参照行はいずれも自テーマの範囲にしか現れない。

**バナーの帰属（§20.2 で確定した扱いの適用）**: 本文一致検証（`RustParse.ps1:319-331`）は直前の空行とコメント行を読み飛ばして
最初のコード行を探すため、**先行コメント塊は後続項目の本文の一部**になる。したがって:
`:1064`（GPU/WUC フィクスチャ）→ `make_world_with_gpu` ／ `:1195`（ComposedSurface 生成補助）→ `elem` ／
`:3657`（表示成立点 info ログ）→ `CapturedEvent` ／ `:4305`（task 6.3 端数 k）→ `surface_entity_of` ／
`:4696`（多層コンテンツ）→ `LAYERED_BIND_AT` ／ `:4976`（hit_region_client）→ `use areka_parsers::shell::{Collision, CollisionName};`
の 6 本はヘルパ側の本文に属し、それぞれ該当ヘルパへ同伴させた（文言は 1 文字も変えていない）。
残る 4 本（`:2354` CurrentSurfaceRead ／ `:2595` DPI 追従 ／ `:3405` 窓寸 reconcile 報告 ／ `:3796` applied_scale／refresh_scale）は
後続のテスト関数またはヘルパの本文に属し、当該テーマファイルの元位置にそのまま置いた。
`:4976` が束ねられる先は `use` 項目であり本文一致検証の対象外だが、当たり判定フィクスチャ塊の見出しであるため同塊とともに `test_support` へ移した。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: `presenter_test_support.rs` の **21 行の先頭にのみ** `pub(super)` を付与した
（関数 16・`struct ScaledGolden` 1・`ScaledGolden` のフィールド 4）。付与のみで本文は無変更。
`hit_coll`・`build_collision_only_assets` は `test_support` 内部からしか参照されないため可視性を変えていない。
**複製は 1 件も作っていない**（`ITEM-EXTRA` 回避・Implementation Notes の集約規則どおり）。

**Implementation Notes の E0659 罠（タスク 3.3 §17.5）への対処**: 共有ヘルパは**明示 import** で受けた（`use super::test_support::{…};`・グロブは使っていない）。
加えて誤結合の有無を実測で確認した——移設前の本番スコープ（`presenter.rs:1-1042`）を全数走査した結果、
`test_support` が公開する 17 識別子のうち**本番モジュールの名前空間と衝突するものは 0 件**である。
同一シグネチャの黙った差し替えは起き得ない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 各ファイルの先頭に `use super::*;` と、そのファイルで実際に使う項目だけの `use` を置いた
（絞る前のビルドで `unused_imports` が 45 件出て要件 2.6 に反したため、`cargo build --message-format=json` の
`unused_imports` 診断が指す識別子だけを機械的に落とした）。移設前ヘッダの `use` 項目は**1 つも失われていない**
（各項目が必要なファイルへ 1 回以上配置されている）。**可視性・`use` 以外の調整は 1 件も必要なかった**（要件 2.8 の追加調整 0 件）。

### 22.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ
（行コメント・入れ子ブロックコメント・通常文字列・raw 文字列（`#` の数）・バイト文字列・
文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と
「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は、既知の唯一の該当箇所
`crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691` を**盲点 1 件**として検出し、同ファイルの
`:382`・`:429`・`:614`・`:690` を **`\` 継続 4 件**として正しく切り分けることで確認した
（実測出力: `continuation=5 382,429,614,690,691 / blind=1 691`）。

| 対象 | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---|---:|
| 移設前 `presenter.rs`（`91d59e0` から `git show`） | **5**（`:5171, 5215, 5407, 5412, 5413`） | **0** |
| 移設後の新テストファイル 8 本 | **5**（`presenter_read_accessor_tests.rs:442, 447, 448` ／ `presenter_refresh_and_log_tests.rs:769, 813`） | **0** |
| 移設後の `presenter.rs` | **0** | **0** |

**結論: 担当ファイルの複数行文字列リテラル 5 件はすべて `\` 継続であり、§11.4 第 1 の盲点の該当行は 0 件。**
`\` 継続では Rust が行頭空白を除去するため、一律 4 スペース de-indent は文字列の中身を変えない。例外処理は不要だった。
移設前 5 行と移設後 5 行は 1 対 1 に対応する（`5171→refresh_and_log:769`・`5215→refresh_and_log:813`・
`5407,5412,5413→read_accessor:442,447,448`）。

### 22.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 91d59e0 -OriginalPath crates/areka-emo-present/src/presenter.rs -RelocatedPath "<新テスト 8 本>" -Detail`

    MATCH: test fn 47=47 / helper item 39=39 / mod block 1 / files 8

exit **0**。**引数不正の 2 と取り違えていないことを対照実行で確認した**——`-OriginalPath crates/areka-emo-present/src/nonexistent.rs`
を与えると `fatal: path ... does not exist in '91d59e0'` を出して **exit 2** になる（不一致の 1 ではない）。

**(a2) 行単位の分類と多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、
それとは独立に、移設した**全行**を分類した。移設前ブロック本体（`1045-5416`・4,372 行）の各行を一律 4 スペース de-indent した
多重集合（空行を除く 4,000 行）と、新テストファイル 8 本の全行の多重集合（空行を除く 4,076 行）を突合:

| 検査 | 結果 |
|---|---|
| 「ちょうど −4 スペース」または空行で説明できない移設行 | **0**（生成時の分類器で `OTHER = 0` を確認） |
| 消えた行（元にあって新に無い） | **25** |
| 増えた行（新にあって元に無い） | **101** |

- **消えた 25 行**の内訳: `pub(super)` 付与前の宣言行 **21**（関数 16・`struct ScaledGolden` 1・フィールド 4）＋
  移設前ヘッダの `use` 行 **4**（`use areka_parsers::shell::{` とその継続 2 行・`use wintf::ecs::{Arrangement, GraphicsCore, HitTest, HitTestMode, Visual, WucGraphicsResource};`）。
- **増えた 101 行**の内訳: `pub(super)` 付き宣言行 **21** ＋ 8 ファイルへ再配置・複製された `use` ヘッダ行 **80**
  （`use super::*;` ×7・`use super::test_support::{…};` の展開行・`};` ×10 ほか）。

差分はすべて要件 2.4 が明示的に許容する調整（可視性付与・`use` の追加／分散）だけで説明がつき、説明のつかない行は 0 件である。
移設前ヘッダの `use` 項目は 1 つも失われていない（消えた 25 行に含まれる `use` 4 行はいずれも各テーマファイルで
必要な項目だけに絞った形へ書き換わっている）。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-emo-present.csv`

    PASS: 全単射 OK / 行数 84 / 相異なる old_fqn 84 / 相異なる new_fqn 84 / フラグメント 1
      - areka-emo-present.csv: 84 行

exit 0。**タスク 4.2 の 37 行に本タスクの 47 行を追記して 84 行**（`old_fqn` 序数順に整列して書き戻し・既存 37 行は無改変）。
47 行の内訳は `presenter::tests::*` → `display_tests` 7／`compose_input_tests` 3／`read_accessor_tests` 9／`dpi_scale_tests` 11／
`resize_report_tests` 4／`refresh_and_log_tests` 9／`fractional_scale_tests` 4。`reason` は全行 `theme_split`、
末尾セグメント（関数識別子）は旧新で同一。移設前 `before_default.txt` に `presenter::tests::` が **47 行**実在し、
対応表の `old_fqn` 47 件すべてがそこに存在することを照合済み（不在 0）。
既存フラグメントとの結合検証（`-Path $V/mapping`）も
`PASS: 全単射 OK / 行数 424 / 相異なる old_fqn 424 / 相異なる new_fqn 424 / フラグメント 6` で exit 0（キー衝突なし）。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list`（exit 0）→ stdout のみ → `: test$` 抽出 →
`$arr = [string[]]@(…)` へ型付け → `[Array]::Sort($arr, [System.StringComparer]::Ordinal)` →
UTF-8 BOM 無し・重複行を残す）でリストを採取し、コミット済み `before_default.txt` と**タスク 3.1〜4.3 の 6 フラグメント全部**を渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after_default_task43.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 424 行 (6 ファイル) / 適用 424 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 424 行・未使用 0 行**。移設後リストの SHA256 は
`37547413DAE70BE311B85AFFD4D503A6ED1796010FB8884F1796F60538F2682B`（中間リストファイル自体はコミットしない）。

**整列器の較正（Implementation Notes の ⚠ 項目）**: コミット済みファイルのハッシュ照合では整列器が動かないため、
**同一の未整列生出力（4,790 行）を序数と `Sort-Object` の 2 通りに整列**して digest が割れることを先に確かめた:
序数 `37547413…682B` ／ `Sort-Object` `6E51B3D9…3A12` ／ **1,806 位置が相違・多重集合の差は 0**。
分岐点は index 179 の `bake::tests::blit_verbatim_correctness`（序数が先）と `bake_entry_tests::all_transparent_is_empty_entry_not_error`（カルチャが先）で、
§10.2 の実測（1,806 位置・index 179・同一の分岐ペア）と完全に一致する。序数比較器が実際に働いていることの直接証跡である。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、
対応表を**使わない**多重集合の対称差で確かめた。

| 検査 | 結果 |
|---|---|
| `before_default.txt` と移設後リストの対称差（対応表なし）: 消えた行 / 現れた行 / 全フラグメント行数 | **424 / 424 / 424**（三者一致） |
| うち本タスクぶん（`presenter::tests::` の消滅 ／ 7 テーマモジュールの出現） | **47 / 47**（本タスクの追記 47 行と一致） |
| 消えた行がすべて `old_fqn` に在るか | **True**（例外 0） |
| 現れた行がすべて `new_fqn` に在るか | **True**（例外 0） |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |
| `old_fqn` と `new_fqn` が同一の行（＝変わっていない名前を載せた水増し） | **0** |
| 末尾セグメント（関数識別子）が旧新で相違する行 | **0** |
| `reason` が `theme_split` 以外の行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-emo-present --no-fail-fast` → **exit 0**。
**128 passed / 0 failed / 0 ignored**（lib 127 ＋ 統合テスト `tests/swapchain_spike.rs` 1 ＋ doctest 0）。
タスク 4.2 の実測値（128）と一致する。移設後の lib 内訳は
`balloon::model_tests` 6 ／ `balloon::series_tests` 24 ／ `balloon::target_tests` 7 ／ `cache::tests` 16 ／
`chain::tests` 1 ／ `command::tests` 5 ／ `mount::tests` 3 ／ `scale::tests` 18 ／
`presenter::compose_input_tests` 3 ／ `presenter::display_tests` 7 ／ `presenter::dpi_scale_tests` 11 ／
`presenter::fractional_scale_tests` 4 ／ `presenter::read_accessor_tests` 9 ／ `presenter::refresh_and_log_tests` 9 ／
`presenter::resize_report_tests` 4 の計 127。移設前の `presenter::tests` 47 と本数が一致する（7+3+9+11+4+9+4=47）。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-emo-present --all-targets` → exit 0・**警告 0 件**。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、
§10.5 の移設前基準値 5 数値と**完全一致**。ユニット別 generated 件数（7 ユニット・多重集合 {1,3,3,3,4,4,4}）も一致し、
`areka-emo-present` に帰属する警告行は 0 件である。

> 集計時の注意（後続タスクへ）: SUMMARY 行の正規表現は §10.5 の逐語どおり `generated \d+ warnings?` と
> **末尾 `?` まで含める**こと。`shiori4-testdll` (lib) は警告 1 件ゆえ cargo が単数形 `generated 1 warning` を出力し、
> `?` を落とすとこの 1 行が DIAG 側へ回って 5 数値が 17/6/21/6/15 へずれる（本タスクで一度踏んだ）。

**(f) 本番本体の無変更** — 移設前コミット `91d59e0` の `presenter.rs` 先頭〜旧 `#[cfg(test)]` 直前まで（1-1042 行）を
現作業ツリーと逐行突合し、**不一致 0**。`git diff --numstat` は `23 / 4374`（挿入 23 ＝ 接続宣言 24 行 − 元位置据え置きの
`#[cfg(test)]` 1 行 ／ 削除 4,374 ＝ テストコード 4,375 行 − 同 1 行）。

**(g) 完了状態の直接確認** — `presenter.rs` に残る `cfg(test)` / `#[path]` / `mod …;` の出現はすべて接続宣言のみ
（`:1043-1066` の 8 モジュール）。**`#[test]` は 1 件も残っていない。テストモジュール本体は 1 行も残っていない。**

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 10 パスのみ:
変更 2 本（`crates/areka-emo-present/src/presenter.rs`・`verification/mapping/areka-emo-present.csv`）＋未追跡 8 本（新テストファイル）。
タスク 4.2 の成果物（`balloon*.rs` 4 本・`cache_tests.rs`・`scale_tests.rs`・`balloon.rs`・`cache.rs`・`scale.rs`）の差分は **0 件**。
`crates/areka-emo-present/tests/`・他クレート・`Cargo.toml`・`tasks.md`・spec 本体ドキュメントも無変更。
新規に導入した `TODO` / `FIXME` / `TBD` は 0 件。

### 22.5 登記（要件 5.2）— 壊れたテスト・テスト間の状態汚染の所見

**本タスクの範囲（`presenter.rs` の 47 テスト・テストコード 4,375 行・本 spec 最大の単一テストモジュール）では、
壊れたテスト・不正なテストは 1 件も発見しなかった。所有 spec への送付所見は 1 件**（下表 #1・ハーネス重複＝
タスク 4.2 §21.5 #2 で既登記のものの追跡）**であり、これは要件 5.1 により本 spec では是正しない。**

| # | 観測 | file:line（移設後） | 判定 |
|---|---|---|---|
| 1 | **同一クレート内に同型のログ捕捉ハーネスが 3 重に存在する**（`CapturedEvent` / `FieldGrab` / `CaptureSubscriber` の 3 点セット）。本タスクが動かしたのは 3 本目の位置のみ | `crates/areka-emo-present/src/presenter_refresh_and_log_tests.rs:25,34,45`（本タスクで移設・**中身は 1 文字も変えていない**）／既登記の 2 本 = `balloon_test_support.rs:62,79,90`・`scale_tests.rs:304,327,338` | **送付所見 →`test-cage-determinism`（W6.9）**。タスク 4.2 §21.5 #2 の追跡。移設前 `balloon.rs:1449-1451` のコメントが理由を明記している——「同 crate `presenter.rs` の tests に同型のものが在るが、あちらは test-local な private 型ゆえ本モジュールから参照できない」。**要件 5.1 がテストハーネスの一本化・共有化を本 spec に禁じている**ため是正しない。本 spec の移設は 3 重を 3 重のまま保っている（増やしても減らしてもいない） |
| 2 | `CaptureSubscriber` が**タプル構造体**で、利用側が `cap.0` としてフィールドを直に読む（移設前 7 箇所） | `crates/areka-emo-present/src/presenter_refresh_and_log_tests.rs:45`（定義）と同ファイル内 7 箇所の `cap.0.lock()` | **問題なし・記録のみ（是正しない）**。ただし**本 spec のテーマ分割にとっては構造的制約**である——タプルフィールドの可視性修飾は行内にしか書けず、本文一致検証の正規化（行頭 `pub(...)` のみ除去）が吸収できないため、ハーネスを共有ヘルパへ出すと必ず不一致になる。本タスクはハーネスと全消費者を 1 モジュールに保つことで回避した（§22.2）。**後続タスク（`areka`・`areka-emo-text` 等）も同型のタプルハーネスを持つ可能性があるので、テーマ境界を引く前に `.0` 参照の分布を確認すること** |
| 3 | テストが本番構造体の**私有フィールドを直接書き換える**（`presenter.targets.get_mut(..).current_surface_id = …` / `.applied = …`） | `crates/areka-emo-present/src/presenter_test_support.rs:386,395`（`force_current_surface` / `force_applied`） | **問題なし・記録のみ**。移設前 `presenter.rs:4978-4983` のコメントが「現行の公開 API 経由では到達不能な防御分岐（DD-5）に、私有状態の直接構築だけが到達できる」と理由を明記している。in-source テストの特権であり、テスト間で共有される状態ではない（各テストが `EmoPresenter::new()` から組む） |
| 4 | 各テストが実 GPU（`GraphicsCore::new()` の HARDWARE デバイス）と WUC コンポジタを生成し、テストスレッドごとに `CoInitializeEx(COINIT_MULTITHREADED)` を呼ぶ | `crates/areka-emo-present/src/presenter_test_support.rs:24-37`（`make_world_with_gpu`） | **問題なし・記録のみ**。`S_FALSE`／`RPC_E_CHANGED_MODE` を無視する形で冪等に書かれており、apartment 不変（`DQTAT_COM_NONE`）ゆえ他テストの観測へ副作用を与えない。プロセス大域の tracing subscriber は使わず `with_default`（スレッドローカル）で捕捉しているため、並列実行でログが混線しない |
| 5 | 移設で可視性・`use`・モジュール接続の追加調整が要るケース（要件 2.8） | — | **0 件**。共有ヘルパ 17 項目（宣言行 21 本）への `pub(super)` 付与と `use` の絞り込みだけで通った。§17.5 #4 が警告した同名 shadow ヘルパによる E0659 は本ファイルには存在しない（§22.2 の全数照合で確認） |
| 6 | 本番 `presenter.rs` が移設後も 1,066 行で 1,000 行の目安を超える | `crates/areka-emo-present/src/presenter.rs` | **意図どおり・是正しない**。本番本体 1,042 行＋接続宣言 24 行。要件 4.5 と design §Non-Goals が `follow.rs`・`frame.rs` 以外の本番本体分割を明示的に禁じており、design §Requirements Traceability 4.5 が `presenter.rs 1,042 行等は分割しない` と名指ししている。**クレート完了状態の「テストファイルがすべて 1,000 行以下」には抵触しない**（本番ファイルであってテストファイルではない） |

### 22.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-emo-present/src/presenter_test_support.rs` | 新規（401 行・共有フィクスチャ 21 項目・テスト 0） |
| `crates/areka-emo-present/src/presenter_display_tests.rs` | 新規（601 行・7 テスト） |
| `crates/areka-emo-present/src/presenter_compose_input_tests.rs` | 新規（423 行・3 テスト） |
| `crates/areka-emo-present/src/presenter_read_accessor_tests.rs` | 新規（451 行・9 テスト） |
| `crates/areka-emo-present/src/presenter_dpi_scale_tests.rs` | 新規（827 行・11 テスト） |
| `crates/areka-emo-present/src/presenter_resize_report_tests.rs` | 新規（265 行・4 テスト） |
| `crates/areka-emo-present/src/presenter_refresh_and_log_tests.rs` | 新規（815 行・9 テスト・ログ捕捉ハーネス同居） |
| `crates/areka-emo-present/src/presenter_fractional_scale_tests.rs` | 新規（691 行・4 テスト） |
| `crates/areka-emo-present/src/presenter.rs` | 末尾のテストモジュールブロックを接続宣言 8 本へ置換（本番本体 1-1042 行は無変更） |
| `verification/mapping/areka-emo-present.csv` | 追記（37 → 84 行・全単射検証済み・既存 37 行は無改変） |
| `verification/notes.md` | 本節（§22）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-emo-present` の `presenter.rs` 分）とする。
これで `areka-emo-present` の必須対象 4 ファイルはすべて移設完了であり、
同クレートのテストファイルは 14 本すべてが 1,000 行以下である。

## 23. クレート単位のテスト分離とテーマ分割: `areka-emo-text`（レイアウト系 3 ファイル）（タスク 4.4・要件 1.1 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `7372699`（タスク 4.3 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 触ったのは `crates/areka-emo-text/src/layout.rs`・`viewbox.rs`・`viewbox_draw.rs` の 3 本 ＋ 新規テストファイル 16 本 ＋ `verification/mapping/areka-emo-text.csv`（新規）＋ 本ファイルのみ。
  同クレートのタスク 4.5 担当分（`actor.rs`・`draw.rs`・`choice.rs`・`state.rs`）と既存の分離済みファイルは **1 行も触っていない**（`git status --porcelain -uall` で確認・§23.4 (h)）。

### 23.1 移設した 3 ファイル（design §File Structure Plan の `crates/areka-emo-text` 7 本のうちレイアウト系 3 本）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 本番 残行 |
|---|---:|---|---|---:|
| `src/layout.rs` | 3,294 | `tests`（750-3294・テストコード 2,545・ブロック本体 2,542） | **テーマ分割 ×4 ＋共有ヘルパ** | 764 |
| `src/viewbox.rs` | 2,498 | `tests`（750-2498・テストコード 1,749・ブロック本体 1,746） | **テーマ分割 ×4 ＋共有ヘルパ** | 764 |
| `src/viewbox_draw.rs` | 3,090 | `tests`（786-3090・テストコード 2,305・ブロック本体 2,302） | **テーマ分割 ×5 ＋共有ヘルパ** | 803 |

- 移設前の行範囲は `target_inventory.csv` / `scan_raw.csv` の実測（`tests:750-3294(2545)` / `tests:750-2498(1749)` / `tests:786-3090(2305)`）と**完全一致**（ズレ 0）。3 本ともテストモジュールはファイル末尾に連続配置・1 ファイル 1 モジュール。
- `#[cfg(test)]` 行は §13〜§22 と同じく**元位置に据え置き**、増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` = `14/2544`・`14/1748`・`17/2304`。挿入は接続宣言行数 − 据え置きの `#[cfg(test)]` 1 行、削除はテストコード行数 − 同 1 行）。
- テストモジュールに `///` doc コメントは付いていない（§14.3 規則は発動しない）。

| 新テストファイル | 行数 | テスト数 |
|---|---:|---:|
| `layout_test_support.rs`（`test_support`） | 95 | 0 |
| `layout_wrap_tests.rs`（`wrap_tests`） | 731 | 20 |
| `layout_visible_window_tests.rs`（`visible_window_tests`） | 262 | 10 |
| `layout_segmented_tests.rs`（`segmented_tests`） | 768 | 16 |
| `layout_cursor_tests.rs`（`cursor_tests`） | 702 | 13 |
| `viewbox_test_support.rs`（`test_support`） | 106 | 0 |
| `viewbox_axis_tests.rs`（`axis_tests`） | 147 | 8 |
| `viewbox_dirty_tests.rs`（`dirty_tests`） | 582 | 14 |
| `viewbox_plan_commit_tests.rs`（`plan_commit_tests`） | 704 | 13 |
| `viewbox_choice_marker_tests.rs`（`choice_marker_tests`） | 224 | 4 |
| `viewbox_draw_test_support.rs`（`test_support`） | 172 | 0 |
| `viewbox_draw_frame_render_tests.rs`（`frame_render_tests`） | 502 | 8 |
| `viewbox_draw_choice_hover_tests.rs`（`choice_hover_tests`） | 230 | 2 |
| `viewbox_draw_oracle_regression_tests.rs`（`oracle_regression_tests`） | 246 | 2 |
| `viewbox_draw_live_diff_tests.rs`（`live_diff_tests`） | 685 | 10 |
| `viewbox_draw_png_dump_tests.rs`（`png_dump_tests`） | 504 | 2 |
| **計** | **6,660** | **122** |

**新テストファイル 16 本はすべて 1,000 行以下**（最大 `layout_segmented_tests.rs` の 768 行）。**僅少超過で単一維持したファイルは 0 件**（§7.4 への追記は不要）。本番 3 本も 764 / 764 / 803 行で 1,000 行以下に収まった。

接続宣言（16 モジュールとも同一文言・design §移設方式の裁定 案 C）:

    #[cfg(test)]
    #[path = "<stem>_<モジュール名>.rs"]
    mod <モジュール名>;
### 23.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

**バナーの分類は 3 本で割れた**——`layout.rs` は**混在**（前半 thematic／後半 chronological）・`viewbox.rs` は **thematic**・`viewbox_draw.rs` は**バナーがほぼ無い**（`════` 2 本のみ・区切りは項目ごとの `///` doc コメント）。
したがって 3 本とも**本番 API の継ぎ目を第一基準**に採り、テストモジュール内ヘルパの参照関係を全数走査して裏取りした。

#### (1) `layout.rs` — バナー 16 本は**混在**（前半 thematic・後半 chronological）

`// ──` バナー 16 本のうち前半 10 本は要件番号ベースの thematic 見出し（`R4.5: FixedMetrics の決定論仮想値`（`:799`）・
`R2.1/2.2/2.4: \_l カーソル座標 → image px 換算`（`:820`）・`R6.1: 横書き`（`:938`）・`R6.2: 日本語縦書き（vertical_rl）`（`:1016`）・
`R6.3: vertical_lr`（`:1062`）・`改行マーカー（NewLine{ratio}）`（`:1116`）・`縮退・境界`（`:1177`）・
`遅延意味論（deferred newline）の判断分岐`（`:1347`）・`R2.5 系/R11.6: 決定論`（`:1641`）・
`3.2 R7.1/7.2/7.4/7.5: あふれ判定とスクロール可視窓`（`:1643`））であるのに対し、
後半 6 本は**作業時系列**の見出しである（`Task 4.1: 塊先決による折返し`（`:1957`）・`Task 4.2: 長大塊の文字単位縮退`（`:2299`）・
`Task 4.4: 保留改行との整合`（`:2459`）・`Task 4.2: pending-cursor 遅延実体化`（`:2719`）・`Task 4.3: 換算表完全性`（`:2887`）・
`Task 4.2: \_l 縮退 4 分岐の actor ごと warn-once`（`:3132`））——`Task 4.2` が 3 か所に散らばり、`4.4` が `4.3` より前に来ている。
**chronological 側はそのままではテーマにならない**ため、Implementation Notes の指示どおり本番 API の継ぎ目へ落とした。

**本番 API（移設前 `layout.rs:1-749`）の継ぎ目**:
(i) metrics 注入口 `GlyphMetrics`（`:74`）・`FixedMetrics`（`:105`）・`FIXED_LINE_BOX_RATIO`（`:112`）と折返し／行送りの中核 `LayoutEngine::layout`（`:235`）・`layout_inner`（`:292`）・`finish_line`（`:593`）、
(ii) 塊先決 `WrapPlan`（`:191`）・`segment_advance_sum`（`:567`）、
(iii) あふれ・可視窓 `VisibleWindow`（`:172`）・`LayoutEngine::visible_window`（`:512`）、
(iv) `\_l` カーソル `cursor_to_image_px`（`:650`）・`CursorDegrade`（`:678`）・`CursorWarnGuard`（`:697`）・`warn_cursor_degrade`（`:715`）・`layout_with_cursor_warn`（`:268`）。
この 4 つがそのまま 4 テーマになった（後半の chronological バナー 6 本は (ii) と (iv) へ 2:4 に畳める）。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support` | 770-797, 1641-1698 | —（2 テーマ以上から参照される共有フィクスチャ 7 項目） | 0 |
| `wrap_tests` | 799-818, 938-1639 | metrics 仮想値・折返し閾値・3 方向の軸読み替え・改行マーカーの行送り・遅延改行（累算／蒸発／リビール進行）・縮退境界（`GlyphMetrics`／`FixedMetrics`／`LayoutEngine::layout`／`finish_line`） | 20 |
| `visible_window_tests` | 1700-1955 | あふれ判定とスクロール可視窓（`LayoutEngine::visible_window`／`VisibleWindow`・3 方向のスクロール方向・飽和・端数行送り） | 10 |
| `segmented_tests` | 1957-2717 | 塊先決ワードラップ（`WrapPlan::Segmented`／`segment_advance_sum`・境界値・長大塊の char 縮退・保留改行との順序・prefix 安定性） | 16 |
| `cursor_tests` | 820-936, 2719-3293 | `\_l` カーソル（`cursor_to_image_px` の換算表・レイアウト経由の遅延実体化・per-axis 合成・`CursorWarnGuard` の actor ごと warn-once） | 13 |

**design の初期見積 `×約 3` に対し `×4 ＋ 共有ヘルパ` を採った理由**: design は「他 22 モジュールのテーマ名は実装時に各モジュールの内容から決定し、旧→新テスト名対応表に記録する」としてテーマ名・本数を実装時裁定に委ねており、`×約 3` は概算である。本番 API の継ぎ目が 4 つに割れており、3 分割にすると `wrap_tests`＋`cursor_tests` か `wrap_tests`＋`visible_window_tests` を束ねることになって 1 ファイルが 1,000 行を超える（前者 1,433 行・後者 1,097 行）。

**ヘルパ参照関係による裏取り**（テストモジュール内の非テスト項目 11 件の参照行を全数走査。行番号は移設前）:

- **多テーマから参照 → `test_support`**（7 項目）: `IMAGE`（`:770`・4 テーマ）・`model`（`:773`・4）・`glyphs`（`:789`・2＋`test_support` 内部）・`inline_positions`（`:794`・3）・`model_rect`（`:1641`・2）・`broken_lines`（`:1665`・2）・`window_for`（`:1677`・2）。
  後ろの 3 項目は可視窓テスト群の直前に定義されているが、`wrap_tests` の `materialized_newline_near_full_triggers_overflow`（`:1492`）が 3 つとも呼ぶ（同テストの doc が可視窓側の `trailing_pending_newline_does_not_trigger_overflow` と対を成すと明記している）ため、**複製せず** `test_support` へ集約した（`ITEM-EXTRA` 回避・Implementation Notes の集約規則どおり）。
- **単一テーマ専用（当該テーマファイルに残置）**: `plan`（`:1957`）・`flat_glyphs`（`:1972`）＝ `segmented_tests` 専用／`WarnCounter`（`:3132`）＋`impl tracing::Subscriber for WarnCounter`（`:3139`）＝ `cursor_tests` 専用。参照行はいずれも自テーマの範囲にしか現れない。

#### (2) `viewbox.rs` — バナー 19 本は **thematic**

`// ──` バナー 19 本はすべて対象（本番 API 名・要件番号）を掲げる thematic 見出しであり、タスク番号を先頭に置くものは 1 本も無い
（`R5.1–5.3: 軸写像（横=y・縦=x）と符号素通し`（`:776`）・`DD11/R6.4/R8.2: 真位置と量子化`（`:821`）・`R7.5 系: 純関数・契約点の初期状態・blit 軸写像委譲`（`:867`）・
`3.2 R2.2/3.2/3.3/4.2: ダーティ導出（露出帯 ∪ 変化行 ∪ 全域）の檻`（`:912`）・`3.3 R2.3/4.3: plan/commit 二相`（`:1559`）・
`6.1 R4.4: 行指紋の hover 印（choice_marker）`（`:2284`）ほか）。総括節の `(a)`〜`(e)`（`:1855`/`:1927`/`:2020`/`:2106`/`:2212`）だけは列挙記号だが、
いずれも `plan/commit 二相` 節の内側にある小見出しで、独立したテーマにはならない。**バナー位置は本番シームと一致した。**

**本番 API（移設前 `viewbox.rs:1-749`）の継ぎ目**:
(i) 軸写像・量子化 `block_axis_vector`（`:93`）・`ScrollState`（`:56`）・`resolve_position`（`:147`）・`blit_vector`（`:160`）、
(ii) ダーティ導出 `line_fingerprint`（`:588`）・`CommittedLine`（`:389`）・`resident_rect`（`:633`）・`exposure_band`（`:674`）・`expand_guard_clamp`（`:724`）・`is_backward_shrink`（`:424`）・`LineOverhang`（`:327`）・`DIRTY_GUARD_IMG_PX`（`:306`）、
(iii) plan/commit 二相 `ScrollPlanner::plan`（`:168`）・`plan_with_overhangs`（`:191`）・`commit`（`:263`）・`request_clear`（`:293`）・`FramePlan`（`:70`）、
(iv) 行指紋の hover 印（`line_fingerprint` の `choice_marker` 経路）。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support` | 912-980, 1828-1853 | —（2 テーマ以上から参照される共有フィクスチャ 8 項目） | 0 |
| `axis_tests` | 770-910 | 軸写像と符号素通し・真位置からの直接量子化・純関数性と契約点の初期状態（`block_axis_vector`／`resolve_position`／`blit_vector`／`ScrollState`） | 8 |
| `dirty_tests` | 982-1557 | ダーティ導出（露出帯 ∪ 変化行 ∪ 全域）・行指紋・オーバーハング拡張・整数格子拡張とクランプ・back 全被覆（`line_fingerprint`／`resident_rect`／`exposure_band`／`expand_guard_clamp`／`DIRTY_GUARD_IMG_PX`） | 14 |
| `plan_commit_tests` | 1559-1826, 1855-2282 | plan/commit 二相（純粋計画・確定・Clear・失敗フレーム再試行）と、その上に載る総括檻 (a)〜(e)（軸写像 e2e・長スクロールのドリフトなし・ダーティ 5 ケース一式・back 全被覆総当り・二相反復同一性） | 13 |
| `choice_marker_tests` | 2284-2497 | 行指紋の hover 印（`choice_marker` の set／clear／switch が当該行の指紋だけを変える） | 4 |

**design の初期見積 `×約 2` に対し `×4 ＋ 共有ヘルパ` を採った理由**: 2 分割でも 1,000 行には収まる（783 / 939 行）が、`plan_commit_tests` 側が 939 行と目安すれすれになるうえ、本番 API の継ぎ目は 4 つに割れておりバナーもその 4 つに整列している。design が「テーマの境界を壊してまで満たす強制値ではない」としているのは**分割しない側**の逃げ道であって、自然な境界で細かく割ることを禁じてはいない。

**ヘルパ参照関係による裏取り**（非テスト項目 19 件を全数走査。行番号は移設前）:

- **多テーマから参照 → `test_support`**（8 項目）: `phys`（`:920`・3 テーマ）・`broken_lines`（`:938`・2）・`canvas_for`（`:950`・2）・`window`（`:974`・3）・`commit_initial`（`:1828`・2）・`expect_update`（`:1847`・2）。
  加えて `IMAGE`（`:912`）と `model_rect`（`:925`）は**テストからは `dirty_tests` 側でしか参照されない**が、いずれも `canvas_for` の内部から呼ばれる（`:950` の本文）ため、`canvas_for` と分離できず塊ごと `test_support` へ置いた（§22.2 の「共有フィクスチャ塊は塊ごと動かす」と同じ扱い）。
- **単一テーマ専用（当該テーマファイルに残置）**: `HORIZONTAL_OFFSET`／`VERTICAL_RL_OFFSET`／`VERTICAL_LR_OFFSET`（`:770-774`）＝ `axis_tests`／`covers_block_axis_fully`（`:1530`）＝ `dirty_tests`／`plan_canvas`（`:1559`）・`assert_long_scroll_is_drift_free`（`:1927`）・`assert_back_fully_covered`（`:2106`）＝ `plan_commit_tests`／`run_content`（`:2284`）・`glyph_resident`（`:2303`）・`choice_resident`（`:2312`）・`derive_dirty_for_hover`（`:2371`）＝ `choice_marker_tests`。

#### (3) `viewbox_draw.rs` — バナーは 2 本だけ（分類の対象にならない）

テストモジュール内に `// ──` 形式のバナーは **0 本**で、`// ════` の大見出しが 2 本あるのみ（`live-diff pixel 等価主檻（task 10・…）`（`:1922`）・
`目視診断（PNG ダンプ・#[ignore]・…）`（`:2595`））。残りの区切りは項目ごとの `///` doc コメントである。
したがってバナーからテーマは導けず、**本番 API の継ぎ目だけを基準**に引いた（2 本のバナーはそのうち 2 つの境界と一致した）。

**本番 API（移設前 `viewbox_draw.rs:1-785`）の継ぎ目**:
(i) フレーム描画契約 `ViewboxExecutor::render`（`:207`）・`DrawStats`（`:78`）・`request_clear`（`:182`）・`scroll_state`（`:170`）・`ensure_format`（`:507`）・`degrade_if_needed`（`:544`）・`full_domain_update`（`:598`）・`plan_inconsistency`（`:619`）、
(ii) Choice／hover 描画 `ChoiceDraw`（`:672`）・`ChoiceHover`（`:678`）・`color_f`（`:686`）・`highlight_rect`（`:704`）・`segment_text_range`（`:764`）・`expand_overhang_for_band`（`:737`）。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support` | 815-968 | —（5 テーマすべてが使う headless リグと画素ヘルパ 9 項目） | 0 |
| `frame_render_tests` | 970-1008, 1231-1683 | `render` の観測可能な完了状態 1〜4 と縮退・失敗経路（初回全域描画・NoChange・可視窓移動の blit と露出帯・typewriter 現在行のみ・`request_clear` の FullClear・font 変更の全域縮退・`plan_inconsistency` の述語・デバイス失敗の再試行安全） | 8 |
| `choice_hover_tests` | 1010-1229 | Choice 住人の素描画が GlyphRun とピクセル同一であること／hover セグメントの塗りと解除リセット（`ChoiceDraw`／`ChoiceHover`／`highlight_rect`／`segment_text_range`） | 2 |
| `oracle_regression_tests` | 1685-1920 | oracle（全域再描画）との byte 一致で固定した 2 件の実欠陥回帰（行間の文字欠け診断・DD-9 行内縮小の退避インク未クリア） | 2 |
| `live_diff_tests` | 1922-2593 | live-diff pixel 等価主檻（`LiveDiffRig` で oracle と viewbox を同時駆動・3 方向 byte 等価・端数 k の許容差・大サイズ／プロポーショナル／Yu Gothic UI 実フォント・注入 divergence の検出） | 10 |
| `png_dump_tests` | 2595-3089 | 目視診断の PNG ダンプ 2 本（`#[ignore]`・自前 PNG エンコーダ 6 ヘルパ同居） | 2 |

**design の初期見積 `×約 3` に対し `×5 ＋ 共有ヘルパ` を採った理由**: 3 分割では 1 ファイルが確実に 1,000 行を超える（テストコード 2,302 行・共有リグ 154 行を差し引いても平均 716 行で、`live_diff_tests` 672 行と `png_dump_tests` 495 行はどちらも他と併合できない大きさ）。上表の 5 つは目的（描画契約の檻／Choice 素描画／実欠陥の回帰固定／oracle 等価の主檻／目視診断）が互いに独立している。

**ヘルパ参照関係による裏取り**（非テスト項目 23 件を全数走査。行番号は移設前）:

- **多テーマから参照 → `test_support`**（9 項目）: `Rig`（`:828`）＋`impl Rig`（`:836`）は **5 テーマすべて**が使う／`glyph_items`（`:914`・4）・`build`（`:919`・3）・`opaque_count`（`:944`・3）・`geo_model`（`:878`・3）。
  `make_dispatcher_and_compositor`（`:815`）は `impl Rig::new` からのみ呼ばれ、`live_diff_model_font`（`:891`）と `block_axis_ink_span`（`:949`）は現時点で `live_diff_tests` からしか参照されないが、3 つとも `:815-968` の連続したリグ／画素ヘルパ塊の内側にあり（`live_diff_model_font` の doc は直前の `geo_model` の説明から続いている）、塊ごと `test_support` へ置いた（§22.2 と同じ扱い）。
- **単一テーマ専用（当該テーマファイルに残置）**: `multiline_items`（`:1419`）＝ `frame_render_tests`／`as_choice_canvas`（`:1010`）・`count_bgra_in_x_band`（`:1100`）＝ `choice_hover_tests`／`LiveDiffRig`（`:1922`）＋`impl LiveDiffRig`（`:1963`）・`run_live_diff_scenario`（`:2209`）・`run_live_diff_scenario_on`（`:2217`）・`run_live_diff_nonunit_scale`（`:2304`）＝ `live_diff_tests`／`diag_crc32`（`:2595`）・`diag_adler32`（`:2618`）・`diag_png_chunk`（`:2628`）・`diag_encode_png_rgba`（`:2638`）・`diag_composite_rgba`（`:2673`）・`diag_bottom_ink_row`（`:2719`）＝ `png_dump_tests`。

#### (4) 3 本に共通する裁定

**タプル構造体ヘルパの分布確認（Implementation Notes・§22.2 の制約）**: テーマ案を立てる前に 3 ファイルのテストモジュール内の構造体を全数走査した結果、**タプル構造体は 1 件も無い**
（`WarnCounter`（`layout.rs:3132`）・`Rig`（`viewbox_draw.rs:828`）・`LiveDiffRig`（`viewbox_draw.rs:1922`）はいずれも名前付きフィールド）。§22.2 の「利用側ごと 1 テーマに収める」制約は本タスクでは発動しなかった。

**バナーの帰属（§20.2 で確定した扱いの適用）**: 本文一致検証（`RustParse.ps1:319-331`）は直前の空行とコメント行を読み飛ばして最初のコード行を探すため、**先行コメント塊は後続項目の本文の一部**になる。したがってバナーは常に直後の項目と同じテーマファイルへ運ばれた。共有ヘルパ側へ束ねられたのは 2 本——`viewbox.rs:912`（`ダーティ導出の檻` 見出し → `const IMAGE`）と `viewbox_draw.rs:1922`（`live-diff pixel 等価主檻` → `struct LiveDiffRig`。ただし `LiveDiffRig` は `live_diff_tests` 専用なので同テーマ内に留まる）である。文言は 1 文字も変えていない。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: 3 つの `*_test_support.rs` の**宣言行の先頭にのみ** `pub(super)` を付与した——`layout` 7 行（`const` 1・関数 6）・`viewbox` 8 行（`const` 1・関数 7）・`viewbox_draw` 14 行（`struct Rig` 1・そのフィールド 4・`impl Rig` の inherent メソッド 2・自由関数 7）。付与のみで本文は無変更。**複製は 1 件も作っていない**。

**Implementation Notes の E0659 罠（タスク 3.3 §17.5）への対処**: 生成した 16 ファイルに**グロブ import は 1 件も無い**（`use …::*;` の出現 0）。共有ヘルパは `use super::test_support::{…};` の明示 import で受けた。加えて誤結合の有無を実測で確認した——`test_support` が公開する識別子（layout 7・viewbox 8・viewbox_draw 8）のうち、対応する本番モジュールの名前空間と衝突するものは **3 本とも 0 件**である。同一シグネチャの黙った差し替えは起き得ない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 移設前ヘッダの `use` 項目を各テーマファイルへ配り、そのファイルで実際に使う項目だけに絞った（絞る前のビルドで `unused_imports` が多数出て要件 2.6 に反したため、`cargo build --message-format=json` の `unused_imports` 診断が指す識別子だけを機械的に落とした。**トレイトメソッド解決に必要な `GlyphMetrics` のような「識別子が本文に現れない import」を落とさないよう、全項目を配ってから診断で削る向きに揃えた**）。移設前ヘッダの `use` 項目は**1 つも失われていない**（各項目が必要なファイルへ 1 回以上配置されている）。**可視性・`use` 以外の調整は 1 件も必要なかった**（要件 2.8 の追加調整 0 件）。
### 23.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 3 ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ（行コメント・入れ子ブロックコメント・通常文字列・raw 文字列（`#` の数）・バイト文字列・文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は、既知の唯一の該当箇所 `crates/wintf/src/ecs/window_proc/window_pos_tests.rs:691` を**盲点 1 件**として検出し、同ファイルの `:382`・`:429`・`:614`・`:690` を **`\` 継続 4 件**として正しく切り分けることで確認した（実測出力: `continuation=5 382,429,614,690,691 / blind=1 691`）。

| 対象 | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---|---:|
| 移設前 `layout.rs`（`7372699` から `git show`） | **0** | **0** |
| 移設前 `viewbox.rs` | **1**（`:1997`） | **0** |
| 移設前 `viewbox_draw.rs` | **6**（`:1786, 1847, 2203, 2581, 2893, 3071`） | **0** |
| 移設後の新テストファイル 16 本 | **7**（`viewbox_plan_commit_tests.rs:419` ／ `viewbox_draw_oracle_regression_tests.rs:112, 173` ／ `viewbox_draw_live_diff_tests.rs:295, 673` ／ `viewbox_draw_png_dump_tests.rs:308, 486`） | **0** |
| 移設後の本番 3 ファイル | **0** | **0** |

**結論: 担当 3 ファイルの複数行文字列リテラル 7 件はすべて `\` 継続であり、§11.4 第 1 の盲点の該当行は 0 件。**
`\` 継続では Rust が行頭空白を除去するため、一律 4 スペース de-indent は文字列の中身を変えない。例外処理は不要だった。
移設前 7 行と移設後 7 行は 1 対 1 に対応する（`viewbox:1997→plan_commit:419` ／ `viewbox_draw:1786,1847→oracle_regression:112,173` ／ `2203,2581→live_diff:295,673` ／ `2893,3071→png_dump:308,486`）。

### 23.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 7372699 -OriginalPath <本番> -RelocatedPath "<新テストファイル群>" -Detail`

    layout.rs        : MATCH: test fn 59=59 / helper item 11=11 / mod block 1 / files 5   (exit 0)
    viewbox.rs       : MATCH: test fn 39=39 / helper item 19=19 / mod block 1 / files 5   (exit 0)
    viewbox_draw.rs  : MATCH: test fn 24=24 / helper item 23=23 / mod block 1 / files 6   (exit 0)

3 本とも exit **0**。**引数不正の 2 と取り違えていないことを対照実行で確認した**——`-OriginalPath crates/areka-emo-text/src/nonexistent.rs` を与えると `fatal: path ... does not exist in '7372699'` を出して **exit 2** になる（不一致の 1 ではない）。

**(a2) 行単位の分類と多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、それとは独立に、移設した**全行**を分類した。移設前ブロック本体の各行を一律 4 スペース de-indent した多重集合（空行を除く）と、新テストファイル群の全行の多重集合（空行を除く）を突合:

| 本番ファイル | ブロック本体 / 非空行 | 新ファイル計 / 非空行 | 「ちょうど −4 スペース」または空行で説明できない行 | 消えた行 | 増えた行 |
|---|---|---|---:|---:|---:|
| `layout.rs` | 2,542 / 2,443 | 2,558 / 2,462 | **0** | 10 | 29 |
| `viewbox.rs` | 1,746 / 1,631 | 1,763 / 1,647 | **0** | 14 | 30 |
| `viewbox_draw.rs` | 2,302 / 2,167 | 2,339 / 2,206 | **0** | 16 | 55 |

- **`layout.rs`**: 消えた 10 行 = `pub(super)` 付与前の宣言行 **7**（`const IMAGE` / `fn model(` / `fn glyphs(` / `fn inline_positions(` / `fn model_rect(` / `fn broken_lines(` / `fn window_for(`）＋ 移設前ヘッダの `use` 行 **3**（`use super::{` とその継続 2 行）。増えた 29 行 = `pub(super)` 付き宣言行 **7** ＋ 5 ファイルへ再配置・複製された `use` 行 **22**。
- **`viewbox.rs`**: 消えた 14 行 = 宣言行 **8**（`const IMAGE` / `fn phys(` / `fn model_rect(` / `fn broken_lines(` / `fn canvas_for(` / `fn window(` / `fn commit_initial(` / `fn expect_update(`）＋ `use` 行 **6**（`use super::{`・継続 4 行・`};`）。増えた 30 行 = 宣言行 **8** ＋ `use` 行 **22**。
- **`viewbox_draw.rs`**: 消えた 16 行 = 宣言行 **14**（`struct Rig {` 1・そのフィールド 4・`impl Rig` の `fn new(` / `fn attach(` 2・自由関数 7）＋ `use` 継続行 **2**。増えた 55 行 = `pub(super)` 付き宣言行 **14** ＋ `use` 行 **41**。

差分はすべて要件 2.4 が明示的に許容する調整（可視性付与・`use` の追加／分散）だけで説明がつき、説明のつかない行は **3 本とも 0 件**である。移設前ヘッダの `use` 項目は 1 つも失われていない。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-emo-text.csv`

    PASS: 全単射 OK / 行数 122 / 相異なる old_fqn 122 / 相異なる new_fqn 122 / フラグメント 1
      - areka-emo-text.csv: 122 行

exit 0。**本タスクで新規作成した 122 行**（`old_fqn` 序数順に整列。タスク 4.5 が同クレートの残り 4 ファイル分を同じフラグメントへ追記できるよう、末尾に追記可能な形で置いた）。
内訳は `layout::tests::*` → `wrap_tests` 20／`visible_window_tests` 10／`segmented_tests` 16／`cursor_tests` 13、`viewbox::tests::*` → `axis_tests` 8／`dirty_tests` 14／`plan_commit_tests` 13／`choice_marker_tests` 4、`viewbox_draw::tests::*` → `frame_render_tests` 8／`choice_hover_tests` 2／`oracle_regression_tests` 2／`live_diff_tests` 10／`png_dump_tests` 2。
`reason` は全行 `theme_split`、末尾セグメント（関数識別子）は旧新で同一。移設前 `before_default.txt` に `layout::tests::` 59 行・`viewbox::tests::` 39 行・`viewbox_draw::tests::` 24 行（計 122）が実在し、対応表の `old_fqn` 122 件すべてがそこに存在することを照合済み（不在 0）。
既存フラグメントとの結合検証（`-Path $V/mapping`）も
`PASS: 全単射 OK / 行数 546 / 相異なる old_fqn 546 / 相異なる new_fqn 546 / フラグメント 7` で exit 0（キー衝突なし）。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list`（exit 0）→ stdout のみ → `: test$` 抽出 → `$arr = [string[]]@(…)` へ型付け → `[Array]::Sort($arr, [System.StringComparer]::Ordinal)` → UTF-8 BOM 無し・重複行を残す）でリストを採取し、コミット済み `before_default.txt` と**タスク 3.1〜4.4 の 7 フラグメント全部**を渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after_default_task44.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 546 行 (7 ファイル) / 適用 546 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 546 行・未使用 0 行**。移設後リストの SHA256 は `E4604A1161219E857AE8702A98DD108FC93371C7538569E156BDDC4340E171AA`（中間リストファイル自体はコミットしない）。

**整列器の較正（Implementation Notes の ⚠ 項目）**: コミット済みファイルのハッシュ照合では整列器が動かないため、**同一の未整列生出力（4,790 行）を序数と `Sort-Object` の 2 通りに整列**して digest が割れることを先に確かめた:
序数 `E4604A11…71AA` ／ `Sort-Object` `663D17D0…543D` ／ **1,806 位置が相違**。
分岐点は index 179 の `bake::tests::blit_verbatim_correctness`（序数が先）と `bake_entry_tests::all_transparent_is_empty_entry_not_error`（カルチャが先）で、§10.2 の実測（1,806 位置・index 179・同一の分岐ペア）と完全に一致する。序数比較器が実際に働いていることの直接証跡である。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、対応表を**使わない**多重集合の対称差で確かめた。

| 検査 | 結果 |
|---|---|
| `before_default.txt` と移設後リストの対称差（対応表なし）: 消えた行 / 現れた行 / 全フラグメント行数 | **546 / 546 / 546**（三者一致） |
| うち本タスクぶん（`layout::tests::` `viewbox::tests::` `viewbox_draw::tests::` の消滅 ／ 13 テーマモジュールの出現） | **122 / 122**（本タスクの追記 122 行と一致） |
| 消えた行がすべて `old_fqn` に在るか | **True**（例外 0） |
| 現れた行がすべて `new_fqn` に在るか | **True**（例外 0） |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |
| `old_fqn` と `new_fqn` が同一の行（＝変わっていない名前を載せた水増し） | **0** |
| 末尾セグメント（関数識別子）が旧新で相違する行 | **0** |
| `reason` が `theme_split` 以外の行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-emo-text --no-fail-fast` → **exit 0**。
**406 passed / 0 failed / 2 ignored**（lib 376＝374 passed＋2 ignored ＋ 統合テスト 9 本 32 ＋ doctest 0）。
移設前との本数一致は、**独立に導出した `#[test]` 属性の全数**で裏づけた——`git ls-tree 7372699 crates/areka-emo-text/` の `.rs` 26 本を `git show` で読んで `^\s*#\[test\]\s*$` を数えると **408**、移設後の作業ツリー 42 本でも **408** で一致する（cargo の実行総数 406 passed + 2 ignored = 408 とも一致）。
移設後の lib 内訳（`--lib -- --list` の集計）は
`layout::wrap_tests` 20 ／ `layout::visible_window_tests` 10 ／ `layout::segmented_tests` 16 ／ `layout::cursor_tests` 13（計 59＝移設前 `layout::tests` 59）、
`viewbox::axis_tests` 8 ／ `viewbox::dirty_tests` 14 ／ `viewbox::plan_commit_tests` 13 ／ `viewbox::choice_marker_tests` 4（計 39＝移設前 `viewbox::tests` 39）、
`viewbox_draw::frame_render_tests` 8 ／ `viewbox_draw::choice_hover_tests` 2 ／ `viewbox_draw::oracle_regression_tests` 2 ／ `viewbox_draw::live_diff_tests` 10 ／ `viewbox_draw::png_dump_tests` 2（計 24＝移設前 `viewbox_draw::tests` 24）。
残る 254 本（`actor` 33・`state` 59・`draw` 29・`choice` 50・`region` 22・`canvas` 12・`writing` 11・`wrap` 9・`segment` 9・`sink` 8・`surface` 6・`lib` 6）はタスク 4.5 以降の担当で、本タスクでは無変更。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-emo-text --all-targets` → exit 0・**警告 0 件**。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、§10.5 の移設前基準値 5 数値と**完全一致**。
SUMMARY 行の正規表現は §10.5 逐語どおり `generated \d+ warnings?`（末尾 `?` 込み・タスク 4.3 の申し送り）を用いた。
ユニット別 generated 件数（7 ユニット・多重集合 {1,3,3,3,4,4,4}）も一致し、`areka-emo-text` に帰属する警告行は 0 件である。

**(f) 本番本体の無変更** — 移設前コミット `7372699` の各ファイル先頭〜旧 `#[cfg(test)]` 直前まで（`layout.rs` 1-749／`viewbox.rs` 1-749／`viewbox_draw.rs` 1-785）を現作業ツリーと逐行突合し、**3 本とも不一致 0**。
とくに `viewbox_draw.rs` の**非 `mod` `#[cfg(test)]` 4 項目は元位置にバイト同一で生存**している（属性行 `:116` → フィールド `fail_next_render: bool,`（`:117`）／属性行 `:146` → 初期化 `fail_next_render: false,`（`:147`）／属性行 `:153` → `fn inject_render_failure(&mut self)`（`:154`）／属性行 `:484` → 分岐 `if self.fail_next_render {`（`:485`））。design §Supporting References の全数表と行番号まで一致する（ズレ 0）。
`layout.rs`・`viewbox.rs` には非 `mod` `#[cfg(test)]` 項目は 1 件も無い（`scan_raw.csv` の `nonmod_count` が 0）。

**(g) 完了状態の直接確認** — 3 本の本番ファイルに残る `mod …;` / `#[path]` の出現はすべて接続宣言のみ（`layout.rs:750-764` の 5 モジュール・`viewbox.rs:750-764` の 5 モジュール・`viewbox_draw.rs:786-803` の 6 モジュール）。**`#[test]` は 1 件も残っていない。テストモジュール本体は 1 行も残っていない。** 残る `#[cfg(test)]` は接続宣言のぶんと、`viewbox_draw.rs` の設計判断 #3 による残置 4 項目だけである。

**(h) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 20 パスのみ:
変更 3 本（`layout.rs`・`viewbox.rs`・`viewbox_draw.rs`）＋未追跡 17 本（新テストファイル 16 本＋`verification/mapping/areka-emo-text.csv`）。
タスク 4.5 担当の 4 ファイル（`actor.rs`・`draw.rs`・`choice.rs`・`state.rs`）・`crates/areka-emo-text/tests/`・`examples/`・他クレート・`Cargo.toml`・`tasks.md`・spec 本体ドキュメントは無変更。
新規に導入した `TODO` / `FIXME` / `TBD` は 0 件。
### 23.5 登記（要件 5.2）— 壊れたテスト・テスト間の状態汚染の所見

**本タスクの範囲（3 ファイル・122 テスト・テストコード 6,599 行）では、壊れたテスト・不正なテストは 1 件も発見しなかった。所有 spec への送付所見は 2 件**（下表 #1・#2）**であり、いずれも要件 5.1 により本 spec では是正しない。**

| # | 観測 | file:line | 判定 |
|---|---|---|---|
| 1 | **本番構造体に埋め込まれたテスト注入シーム `fail_next_render`**。`ViewboxExecutor` の私有フィールド（`#[cfg(test)]` 付き）・`new()` 内の初期化・注入用 inherent メソッド `inject_render_failure`・`render` 内の失敗分岐の 4 点セットで、本番構造体の状態としてテスト専用の可変フラグを持つ | `crates/areka-emo-text/src/viewbox_draw.rs:117`（フィールド `fail_next_render: bool,`）・`:147`（`new()` の初期化 `fail_next_render: false,`）・`:154`（`fn inject_render_failure(&mut self)`）・`:485`（`render` 内の分岐 `if self.fail_next_render {`）——それぞれ直前行 `:116` / `:146` / `:153` / `:484` が `#[cfg(test)]`。**本タスクの移設で 1 バイトも動いていない**（§23.4 (f) で逐行突合済み） | **送付所見 →`test-cage-determinism`（W6.9）**。design §Error Handling が「設計時点で確定済みの送付所見」として名指ししているもの（design §Supporting References の非 `mod` `#[cfg(test)]` 40 件のうち 4 件＝構造体フィールド 2・自由関数扱い 1・分岐 1）。**設計判断 #3 が 40 件全数残置を裁定している**ため本番ファイルに据え置いた。唯一の消費者は `viewbox_draw_frame_render_tests.rs:467` の `device_failure_mid_render_is_retry_safe_front_unchanged_no_commit`（`exec.inject_render_failure();`・リポジトリ全域で唯一の呼出） で、同テストは注入後に必ず 1 フレーム消費してフラグを落とすが、**フラグの寿命が本番オブジェクトの寿命に等しい**点は変わらない（`Rig` を使い捨てにする現在の書き方では顕在化しない） |
| 2 | **層規律の構造檻の被覆が、本 spec の移設によって黙って縮む**。`pure_layer_modules_have_no_windows_imports` は `include_str!("layout.rs")` 等で**本番ファイルのテキストそのもの**を読んで `windows::` 等の禁止パターンを探すため、テストモジュールを外へ出した時点で**移設したテストコードは走査対象から外れる**（`layout.rs` 2,545 行・`viewbox.rs` 1,749 行ぶんが被覆から抜けた。`choice.rs`・`state.rs` も同檻の対象なのでタスク 4.5 でさらに縮む） | `crates/areka-emo-text/src/lib.rs:173-183`（`PURE_SOURCES` の 9 エントリ。うち本タスクの担当は `:179` `layout.rs` と `:181` `viewbox.rs`） | **送付所見 →`test-cage-determinism`（W6.9）／タスク 4.5・7.x への申し送り**。**現時点で実害は 0**——移設した純粋層テストファイル 10 本（`layout_*` 5・`viewbox_*` 5）を全数走査した結果、禁止パターン（`use windows` / `windows::` / `windows_core` / `windows_numerics` / `extern crate windows`）の該当は **0 件**であり、`windows` の文字列が現れるのは `viewbox_test_support.rs:83` のコメント 1 行だけ（禁止パターンには一致しない）。**是正しない理由**: `PURE_SOURCES` はテストの入力値そのものであり、エントリを増やすことは要件 2.4 が禁じる「入力値の変更」に当たる。加えて `lib.rs` は本タスクの担当外（テストモジュール 65 行・必須対象でない）。**縮んだ被覆を戻すなら `<stem>_*_tests.rs` を `PURE_SOURCES` へ足す 1 行追加で済む**ことを申し送る |
| 3 | 目視診断の PNG ダンプ 2 本が**カレントディレクトリへファイルを書き出す**（`AREKA_DIAG_OUT` 未指定時の既定が `"."`） | `crates/areka-emo-text/src/viewbox_draw_png_dump_tests.rs:152` と `:385`（`std::env::var("AREKA_DIAG_OUT").unwrap_or_else(\|_\| ".".to_string())`）・書き込みは `:319`・`:355`・`:483` | **問題なし・記録のみ（是正しない）**。両テストとも `#[ignore = "PNG ダンプ（ファイル副作用・目視診断用・明示実行のみ）"]`（`:150`・`:383`）が付いており既定実行では走らない（`cargo test -p areka-emo-text` の `2 ignored` はこの 2 本）。副作用が属性で明示されているのは正しい書き方である |
| 4 | 各テストが実 GPU（`GraphicsCore::new()`）と WUC コンポジタを生成し、テストスレッドごとに dispatcher queue を作る | `crates/areka-emo-text/src/viewbox_draw_test_support.rs:21`（`make_dispatcher_and_compositor`・ASTA 第一候補／NONE 保険）・`:41`（`Rig::new`） | **問題なし・記録のみ**。`presenter.rs`（§22.5 #4）と同じ方針で、apartment 生成に失敗しても保険経路へ落ちる冪等な書き方。`Rig` はテストごとに新規生成され、テスト間で共有される状態は無い |
| 5 | ログ捕捉に**プロセス大域の subscriber を使っていない** | `crates/areka-emo-text/src/layout_cursor_tests.rs:544`（`struct WarnCounter`・名前付きフィールド）・`:600`・`:669`（`tracing::subscriber::with_default`） | **問題なし・記録のみ**。スレッドローカルの `with_default` で捕捉しているため並列実行で混線しない。§22.5 #1 が `areka-emo-present` で登記した「同型ハーネスの重複」は本クレートでは 1 本のみで、重複は無い |
| 6 | `yugothic_real_fixture_matches_oracle_byte_for_byte` が**実インストールフォント（Yu Gothic UI）と実 fixture ファイル**に依存する | `crates/areka-emo-text/src/viewbox_draw_live_diff_tests.rs:566` | **問題なし・記録のみ**。doc（`:562-563`）が「fixture が読めない環境（font 不在等）でも oracle↔viewbox は font に依らず一致するため頑健」と縮退条件を明記しており、環境差で赤くならない設計になっている |
| 7 | 移設で可視性・`use`・モジュール接続の**追加**調整が要るケース（要件 2.8） | — | **0 件**。共有ヘルパへの `pub(super)` 付与（宣言行 計 29 本）と `use` の配り直しだけで 3 本とも通った。§17.5 #4 が警告した同名 shadow ヘルパによる E0659 は本 3 ファイルには存在しない（§23.2 (4) の全数照合で確認）。グロブ import は 1 件も生成していない |

### 23.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-emo-text/src/layout_test_support.rs` | 新規（95 行・共有フィクスチャ 7 項目・テスト 0） |
| `crates/areka-emo-text/src/layout_wrap_tests.rs` | 新規（731 行・20 テスト） |
| `crates/areka-emo-text/src/layout_visible_window_tests.rs` | 新規（262 行・10 テスト） |
| `crates/areka-emo-text/src/layout_segmented_tests.rs` | 新規（768 行・16 テスト） |
| `crates/areka-emo-text/src/layout_cursor_tests.rs` | 新規（702 行・13 テスト） |
| `crates/areka-emo-text/src/viewbox_test_support.rs` | 新規（106 行・共有フィクスチャ 8 項目・テスト 0） |
| `crates/areka-emo-text/src/viewbox_axis_tests.rs` | 新規（147 行・8 テスト） |
| `crates/areka-emo-text/src/viewbox_dirty_tests.rs` | 新規（582 行・14 テスト） |
| `crates/areka-emo-text/src/viewbox_plan_commit_tests.rs` | 新規（704 行・13 テスト） |
| `crates/areka-emo-text/src/viewbox_choice_marker_tests.rs` | 新規（224 行・4 テスト） |
| `crates/areka-emo-text/src/viewbox_draw_test_support.rs` | 新規（172 行・共有リグ 9 項目・テスト 0） |
| `crates/areka-emo-text/src/viewbox_draw_frame_render_tests.rs` | 新規（502 行・8 テスト） |
| `crates/areka-emo-text/src/viewbox_draw_choice_hover_tests.rs` | 新規（230 行・2 テスト） |
| `crates/areka-emo-text/src/viewbox_draw_oracle_regression_tests.rs` | 新規（246 行・2 テスト） |
| `crates/areka-emo-text/src/viewbox_draw_live_diff_tests.rs` | 新規（685 行・10 テスト） |
| `crates/areka-emo-text/src/viewbox_draw_png_dump_tests.rs` | 新規（504 行・2 テスト・自前 PNG エンコーダ同居） |
| `crates/areka-emo-text/src/layout.rs` | 末尾のテストモジュールブロックを接続宣言 5 本へ置換（本番本体 1-749 行は無変更・残 764 行） |
| `crates/areka-emo-text/src/viewbox.rs` | 同上（接続宣言 5 本・本番本体 1-749 行は無変更・残 764 行） |
| `crates/areka-emo-text/src/viewbox_draw.rs` | 同上（接続宣言 6 本・本番本体 1-785 行は無変更・非 `mod` `#[cfg(test)]` 4 項目もバイト同一で生存・残 803 行） |
| `verification/mapping/areka-emo-text.csv` | 新規（122 行・全単射検証済み・タスク 4.5 が追記できる形） |
| `verification/notes.md` | 本節（§23）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-emo-text` のレイアウト系 3 ファイル分）とする。残る 4 ファイル（`actor.rs`・`draw.rs`・`choice.rs`・`state.rs`）はタスク 4.5 の担当であり、同クレートのテストファイルがすべて 1,000 行以下になるのはタスク 4.5 の完了時点である（本タスクで生成した 16 本はすべて 1,000 行以下）。

## 24. クレート単位のテスト分離とテーマ分割: `areka-emo-text`（アクター・描画・選択肢・状態の 4 ファイル）（タスク 4.5・要件 1.1 / 1.3 / 1.6 / 1.7 / 1.8 / 2.4 / 2.8 / 2.9 / 3.1〜3.3 / 7.1 / 7.2）

- 実施日: 2026-08-08
- ブランチ: `claude/areka-p0-file-slimming-64d065` / 移設前コミット `246ddb9`（タスク 4.4 のコミット時点）
- 実行シェル: **PowerShell（pwsh 7）**
- 触ったのは `crates/areka-emo-text/src/actor.rs`・`draw.rs`・`choice.rs`・`state.rs` の 4 本 ＋ 新規テストファイル 16 本 ＋ `verification/mapping/areka-emo-text.csv`（**追記のみ**）＋ 本ファイルのみ。
  タスク 4.4 が生成した 16 本と `layout.rs`・`viewbox.rs`・`viewbox_draw.rs`・`lib.rs`・その他既存ファイルは **1 行も触っていない**（`git status --porcelain -uall` で確認・§24.4 (i)）。
- **本タスクの完了をもって `areka-emo-text` クレートのテスト分離は完了**する（design §File Structure Plan の 7 本のうちタスク 4.4 の 3 本＋本タスクの 4 本）。

### 24.1 移設した 4 ファイル（design §File Structure Plan の `crates/areka-emo-text` 7 本のうち残る 4 本）

| 本番ファイル | 移設前 総行 | テストモジュール（移設前の行範囲） | 扱い | 本番 残行 |
|---|---:|---|---|---:|
| `src/actor.rs` | 2,967 | `tests`（858-939・82）＋`runtime_tests`（941-2967・2,027） | `tests` は**個別ファイル化のみ**（FQN 不変・対応表 0 行）／`runtime_tests` は**テーマ分割 ×4 ＋共有ヘルパ** | 880 |
| `src/draw.rs` | 2,293 | `tests`（964-2293・1,330） | **テーマ分割 ×2 ＋共有ヘルパ** | 974 |
| `src/choice.rs` | 1,749 | `tests`（537-1121・585）・`style_resolve_tests`（1129-1339・211）・`decorate_tests`（1347-1749・403） | **個別ファイル化のみ**（3 本とも 1,000 行以下・FQN 不変・対応表 0 行） | 547 |
| `src/state.rs` | 1,630 | `tests`（457-1630・1,174） | **テーマ分割 ×3 ＋共有ヘルパ** | 471 |

- 移設前の行範囲は `target_inventory.csv` / `scan_raw.csv` の実測（`tests:858-939(82)|runtime_tests:941-2967(2027)` / `tests:964-2293(1330)` / `tests:537-1121(585)|style_resolve_tests:1129-1339(211)|decorate_tests:1347-1749(403)` / `tests:457-1630(1174)`）と**完全一致**（ズレ 0）。4 本ともテストモジュールはファイル末尾に連続配置。
- **タスクの指示どおり「1 モジュール＝1 ファイル」を先に適用し、それでも 1,000 行を超えるモジュールだけをテーマ分割した。** その結果 `choice.rs` はテーマ分割ゼロ・対応表 0 行、`actor.rs` の `tests`（82 行）も FQN が変わらないため対応表 0 行である（設計判断 #2 のとおり）。
- `#[cfg(test)]` 行は §13〜§23 と同じく**元位置に据え置き**、増えるモジュールぶんの宣言だけを新設した（`git diff --numstat` = `16/2103`（actor）・`8/1327`（draw）・`6/1208`（choice）・`11/1170`（state））。
- テストモジュールに `///` doc コメントは付いていない（§14.3 規則は発動しない）。`choice.rs` のモジュール間バナー 2 本（`1123-1128` タスク 5.3 ／ `1341-1346` タスク 5.4）は `//` 行コメントであり、設計判断 #2 のとおり**対応するテストファイルの先頭へバイト同値で同伴**させた（§19.3 と同じ扱い）。

| 新テストファイル | 行数 | テスト数 |
|---|---:|---:|
| `actor_tests.rs`（`tests`・FQN 不変） | 79 | 4 |
| `actor_test_support.rs`（`test_support`） | 131 | 0 |
| `actor_runtime_frame_tests.rs`（`runtime_frame_tests`） | 556 | 9 |
| `actor_choice_contract_tests.rs`（`choice_contract_tests`） | 681 | 10 |
| `actor_clear_atomicity_tests.rs`（`clear_atomicity_tests`） | 372 | 4 |
| `actor_scale_refresh_tests.rs`（`scale_refresh_tests`） | 311 | 6 |
| `draw_test_support.rs`（`test_support`） | 84 | 0 |
| `draw_format_metrics_tests.rs`（`format_metrics_tests`） | 470 | 20 |
| `draw_oracle_tests.rs`（`oracle_tests`） | 792 | 9 |
| `choice_tests.rs`（`tests`・FQN 不変） | 582 | 25 |
| `choice_style_resolve_tests.rs`（`style_resolve_tests`・FQN 不変） | 215 | 13 |
| `choice_decorate_tests.rs`（`decorate_tests`・FQN 不変） | 407 | 12 |
| `state_test_support.rs`（`test_support`） | 57 | 0 |
| `state_cue_apply_tests.rs`（`cue_apply_tests`） | 637 | 24 |
| `state_reveal_tests.rs`（`reveal_tests`） | 280 | 16 |
| `state_cursor_coord_parse_tests.rs`（`cursor_coord_parse_tests`） | 202 | 19 |
| **計** | **5,856** | **171** |

**新テストファイル 16 本はすべて 1,000 行以下**（最大 `draw_oracle_tests.rs` の 792 行）。**僅少超過で単一維持したファイルは 0 件**（§7.4 への追記は不要）。本番 4 本も 880 / 974 / 547 / 471 行で 1,000 行以下に収まった。

接続宣言（16 モジュールとも同一文言・design §移設方式の裁定 案 C）:

    #[cfg(test)]
    #[path = "<stem>_<モジュール名>.rs"]
    mod <モジュール名>;

### 24.2 テーマ境界の裁定（design §テーマ分割ポリシー・手順①）

**バナーの分類は 4 本で割れた**——`actor.rs`（`runtime_tests`）は**作業時系列（chronological）**・`draw.rs` は**混在**（前半 4 本が thematic・後半 3 本がタスク番号先頭）・`state.rs` は **thematic**（要件番号ベース）・`choice.rs` は**モジュール間バナー 2 本のみ**（テーマ分割をしないので分類の対象外）。
Implementation Notes の指示どおり、chronological / 混在の 2 本は**本番 API の継ぎ目を第一基準**に採り、thematic の `state.rs` も同じく本番 API で裏取りした。いずれもテストモジュール内ヘルパの参照関係を全数走査して確認している。

#### (1) `actor.rs` の `runtime_tests` — セクション見出し 13 本は **chronological**（順序も乱れている）

セクション見出しコメント 13 本（`// ══` 8 本・`// ──` 2 本・`// task 9.x:` の素コメント 3 本）の見出しはタスク番号順に並んでおらず、**`task 7.2`（モジュール冒頭）→ 8.1 → 8.2 → 9.1 → 9.2 → 9.3 → 8.3 → 9.4 → 7.1** と前後する
（移設前 `actor.rs:1593` `task 8.1`・`:1683` `task 8.2`・`:1766` `task 9.1`・`:1908` `task 9.2`・`:2075` `task 9.3`・`:2302` `task 8.3`・`:2466` `task 9.4`・`:2666` `task 7.1`。
冒頭 3 本 `:1102` / `:1259` / `:1320` はタスク番号を持たず、モジュール冒頭コメント（`:943-946`）が掲げる「task 7.2 の檻」の内訳見出しである）。
**chronological はそのままではテーマにならない**ため本番 API の継ぎ目へ落とした。

**本番 API（移設前 `actor.rs:1-857`）の継ぎ目**:
(i) 束縛と解決 `TextSlotBinding`（`:47`・`new` `:84`・`from_view` `:116`）・`ResolvedBalloonText`（`:134`・`resolve` `:152`）——これは 82 行の `mod tests` が単独で担当しており**分割不要**、
(ii) アクターループ／cue ドレイン `spawn_emo_text`（`:562`）・`apply_cue`（`:422`）、
(iii) フレーム提示 `present_frame`（`:598`）・`present_actor`（`:642`）・`draw_stats`（`:495`）、
(iv) 選択肢契約 `inject_choice_hover`（`:507`）・`choice_hit_rows`（`:530`）・`choice_active`（`:541`）・`ChoiceHitRow`（`:179`）、
(v) `apply_cue` の Clear/ClearAll 経路（ライフサイクル無効化）、
(vi) k 再追従 `refresh_actor_scale`（`:344`）と内側シーム `refresh_actor_binding`。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support` | 943-946, 1022-1100, 1593-1607, 1683-1699 | —（共有フィクスチャ 9 項目——うち 8 項目へ `pub(super)`・`REVEAL_INTERVAL` は内部） | 0 |
| `runtime_frame_tests` | 975-1020, 1102-1592 | 実 pump 上のドレイン終了 3 経路（`spawn_emo_text`・R1.2/1.3/1.4）と `present_frame` の未解決 actor 蓄積＋スキップ＋再試行・装着と注入時刻駆動の進行・`draw_stats`・`clear_all`（＝モジュール冒頭が宣言する「task 7.2 の檻」そのもの） | 9 |
| `choice_contract_tests` | 1609-1681, 1701-2300 | 選択肢契約 API（`inject_choice_hover`／`choice_hit_rows`／`choice_active`）とその提示・字下げ描画・hover 画素（SquareFill と矩形反転縮退） | 10 |
| `clear_atomicity_tests` | 2302-2664 | `apply_cue(Clear/ClearAll)` の原子的無効化（hover リセット＋ヒット行スナップショット無効化＋FullClear 提示後の画素消滅の同一フレーム観測） | 4 |
| `scale_refresh_tests` | 2666-2966 | `refresh_actor_scale`／`refresh_actor_binding` の k 再追従（同値 noop・image 空間不変・churn ガード・未登録 actor の noop） | 6 |

**design の初期見積 `×約 2〜3` に対し `×4 ＋ 共有ヘルパ` を採った理由**: design は「他 22 モジュールのテーマ名は実装時に各モジュールの内容から決定し、旧→新テスト名対応表に記録する」としてテーマ名・本数を実装時裁定に委ねており、`×約 2〜3` は概算である。上記 (ii)+(iii) はモジュール冒頭コメントが 1 つの檻（task 7.2）として束ねているので 1 テーマへまとめたが、選択肢まわり（`1593-2665`）は**それだけで 1,073 行**あり 1 ファイルに収まらない。`apply_cue` の Clear/ClearAll 経路（(v)）で割るのが本番 API 上の唯一の自然な継ぎ目であり、これで 681 / 372 行になる。3 分割では必ず 1,000 行超が出る。

**ヘルパ参照関係による裏取り**（`runtime_tests` 内の非テスト項目 14 件の参照行を全数走査。行番号は移設前）:

- **多テーマから参照 → `test_support`**（`pub(super)` 付与 8 項目）: `cue`（`:1031`・4 テーマ）・`geo_model`（`:1055`・4）・`spawn_reserved_slot`（`:1082`・3）・`opaque_count`（`:1098`・4）・`choice_cue`（`:1596`・2）・`com_world`（`:1687`・3）。
  `pump_until_idle`（`:1046`・`runtime_frame_tests` のみ）と `cursor_model`（`:1070`・`choice_contract_tests` のみ）は現時点で単一テーマからしか参照されないが、**`1022-1100` の「テスト土台」連続ヘルパ塊の内側**にあり、同塊の他 4 項目が多テーマ共有であるため塊ごと `test_support` へ置いた（§22.2・§23.2 と同じ扱い）。`REVEAL_INTERVAL`（`:1027`）は `cue`／`choice_cue` の内部からしか参照されないため `test_support` 内部に留め、可視性は変えていない。
- **単一テーマ専用（当該テーマファイルに残置）**: `LevelCounter`（`:977`）＋`impl tracing::Subscriber for LevelCounter`（`:982`）＋`with_log_cage`（`:1007`）＝ `runtime_frame_tests` 専用（参照は `:1110, 1165, 1217, 1234, 1270, 1308` の 6 箇所ですべて `1102-1592` の内側）。
  この 3 項目は `:975-1020` の**「ログ檻」バナーで区切られた独立した塊**であり、テスト土台塊（`:1022-1100`）とは別単位なので**塊ごと `runtime_frame_tests` へ移した**（可視性は 1 文字も付けていない）。／`ink_in_band`（`:2469`）＝ `clear_atomicity_tests` 専用（参照 `:2516, 2533, 2622, 2660`）／`NATIVE`（`:2676`）＝ `scale_refresh_tests` 専用（参照 `:2693`〜`:2957`）。
  **判定は項目単位ではなく塊単位で行った**——塊の全項目が単一テーマなら塊ごとテーマファイルへ、塊に多テーマ項目が混じるなら塊ごと `test_support` へ、という規則である。

#### (2) `draw.rs` — バナー 7 本は**混在**（前半 4 本 thematic・後半 3 本がタスク番号先頭）

前半 4 本は対象を掲げる thematic 見出し（`R4.1/R4.2: フォント解決とフォールバック（純粋部・COM 不要）`（`:1055`）・`方向レシピ: writing_mode 解釈結果→DirectWrite 設定の一意導出`（`:1148`）・`R10.3: 文字装飾／disable.font.* は型シームのみ`（`:1187`）・`COM 検証（headless DWrite・デバイス非依存・窓不要）`（`:1202`））であるのに対し、
後半 3 本はタスク番号を先頭に掲げる（`task 6.2 R4.5: DWriteMetrics——計測専用 probe TextLayout`（`:1284`）・`task 6.3 R3.1/R7.3: DrawExecutor——可視窓の全域再描画`（`:1516`）・`task 6.4 R4.5/R6.1–6.3/R7.5: probe/描画行 TextLayout の送り幅一致 invariant`（`:2098`））。
番号は 6.2→6.3→6.4 と昇順で本番 API の並びとも一致するため、**本番 API の継ぎ目を第一基準**に採り、バナー位置がそれと矛盾しないことを確認した。

**本番 API（移設前 `draw.rs:1-963`）の継ぎ目**:
(i) 解決系（実デバイス不要——DWrite factory だけで完結）`ResolvedFont`（`:152`/`:168`）・`DirectionRecipe`（`:240`/`:251`）・`create_text_format`（`:302`）・`try_create_format`（`:334`）・`FontDisableSeam`（`:144`）・`DWriteMetrics`（`:365`/`:381`）・`impl GlyphMetrics for DWriteMetrics`（`:435`）・`measure_line_box_ratio`（`:493`）、
(ii) 比較専用オラクル（実 GPU／WUC 必須）`DrawExecutor`（`:692`/`:707`）とその支持である `LineLayoutStore`（`:580`）・`measure_line_overhang`（`:659`）・`create_target_bitmap`（`:930`）。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support` | 991-1053, 1317-1325 | —（2 テーマとも使うモデル生成・ログ檻・既定 metrics の 6 項目） | 0 |
| `format_metrics_tests` | 1055-1315, 1327-1514 | 解決系（`ResolvedFont` の ukadoc 既定フォールバック・`DirectionRecipe` の写像表・`TextEffects`／`FontDisableSeam` の型シーム・`create_text_format` の headless 生成と既定再試行・`DWriteMetrics` の probe 規約／キャッシュ／行ピッチ／実 font face metrics） | 20 |
| `oracle_tests` | 1516-2292 | 比較専用オラクル `DrawExecutor`（全域再描画の単調増加と Clear 復帰・残渣なし・確定行キャッシュ・スケール一点適用・スクロール・住人シーム skip）と、その実描画経路で測る probe/描画一致 invariant | 9 |

**design の初期見積 `×約 2` どおり `×2 ＋ 共有ヘルパ` を採った**。境界は「実デバイスを要さない解決系」対「実 GPU/WUC 上のオラクル描画」という本番 API の継ぎ目に一致する。`task 6.4` の invariant 檻（`:2098-2292`）は `DrawExecutor::line_layout` を実際に駆動する（`drawn_line_cluster_widths`（`:2132`）が `DrawRig` を要求する）ため、probe 側ではなく **オラクル側**へ入れた。

**ヘルパ参照関係による裏取り**（非テスト項目 19 件を全数走査。行番号は移設前）:

- **多テーマから参照 → `test_support`**（`pub(super)` 付与 4 項目）: `model_with_font`（`:992`・両テーマ。`:2126` の `invariant_font` からも呼ばれる）・`empty_font`（`:1005`・両テーマ。`:2071`／`:2240`）・`with_log_cage`（`:1040`・両テーマ。`:1062`〜`:1504` と `:2034`／`:2046`）・`default_metrics`（`:1318`・両テーマ。`:1333`〜`:1502` と `:2062`）。
  `LevelCounter`（`:1010`）＋`impl tracing::Subscriber`（`:1015`）は `with_log_cage` の内部からしか参照されないため `test_support` 内部に留め、可視性を変えていない。
- **単一テーマ専用（当該テーマファイルに残置）**: `read_family_name`（`:1205`）・`manual_probe_advance`（`:1294`）＝ `format_metrics_tests`／`make_dispatcher_and_compositor`（`:1542`）・`DrawRig`（`:1554`）＋`impl DrawRig`（`:1561`）・`geo_model`（`:1604`）・`glyph_items`（`:1617`）・`render_items`（`:1623`）・`opaque_count`（`:1652`）・`ink_min`（`:1657`）・`INVARIANT_CASES`（`:2112`）・`invariant_font`（`:2120`）・`drawn_line_cluster_widths`（`:2132`）＝ `oracle_tests`。

#### (3) `state.rs` — バナー 26 本は **thematic**（要件番号ベース）

`// ──` / `// ══` バナーはすべて要件番号または対象 API を掲げており、タスク番号を先頭に置くものは 1 本も無い（`R2.1: Text 追記`（`:504`）・`R2.2: NewLine 改行（ratio 転写）`（`:551`）・`R2.3: Clear 全消去`（`:571`）・`R1.6: actor 別振り分け`（`:646`）・`R10.5: 上書きガードなし`（`:682`）・`R2.4/R2.5: 純粋・決定論`（`:696`）・`Choice/Cursor 実消費（W4 choice-render・タスク 1.2）`（`:729`）・`══ typewriter リビール進行（注入時刻駆動・R3／R7 系）`（`:1136`）・`══ 服従契約の縮退（1.2/7.3）と honor no-op（2.2/7.5）`（`:1356`）・`══ \_l 座標語彙のパース（parse_cursor_coord・語彙全形の網羅・タスク 1.1）`（`:1430`）ほか）。**`══` 大見出し 3 本のうち 2 本（`:1136`・`:1430`）がそのままテーマ境界になった（残る `:1356`「服従契約の縮退」は reveal_tests の内部に位置する）。**

**本番 API（移設前 `state.rs:1-456`）の継ぎ目**:
(i) cue 適用と状態 `TextLayerState`（`:302`/`:307`）・`ActorTextState`（`:269`/`:278`）・`ChoiceSpan`（`:254`）・`TextItem`（`:71`）・`TextLayerConfig`（`:56`/`:61`）、
(ii) リビール進行 `RevealSchedule`（`:198`/`:203`）、
(iii) `\_l` 座標語彙のパース `parse_cursor_coord`（`:148`）・`CursorCoord`（`:108`）・`CursorUnit`（`:124`）——純粋・全域関数で状態に触れない。

| 新モジュール（ファイル） | 移設前の行範囲 | 対象の本番項目 | テスト数 |
|---|---|---|---:|
| `test_support` | 464-502, 1136-1150 | —（2 テーマ以上から参照される共有ヘルパ 5 項目） | 0 |
| `cue_apply_tests` | 504-1134 | cue 適用による状態遷移（Text 追記・NewLine・Clear/ClearAll・actor 別振り分けと遅延生成・後出し優先・決定論・Choice/Cursor 実消費・`TextLayerConfig` 既定値） | 24 |
| `reveal_tests` | 1152-1428 | typewriter リビール進行（`RevealSchedule`: r_i 式・at 下限・リビール中の後出し反映・決定論・境界・服従契約の縮退と honor no-op） | 16 |
| `cursor_coord_parse_tests` | 1430-1629 | `parse_cursor_coord` の語彙全形（Omitted／Absolute Px・Em・Lh・Percent／Relative／Invalid／全域性） | 19 |

**design の初期見積 `×約 2` に対し `×3 ＋ 共有ヘルパ` を採った理由**: 2 分割（`1430` で割る）でも 1,000 行には収まる（925 / 200 行）が、**925 行は目安 1,000 のすぐ下**であり、本 spec の主眼が「1 ファイルの行数」である以上そこで止める理由がない。本番 API の継ぎ目は 3 つに割れており `══` 大見出し 3 本もそれに整列しているので、自然な境界で 3 分割した（637 / 280 / 202 行）。design が「テーマの境界を壊してまで満たす強制値ではない」としているのは**分割しない側**の逃げ道であって、自然な境界で細かく割ることを禁じてはいない（§23.2 (2) と同じ判断）。

**ヘルパ参照関係による裏取り**（非テスト項目 9 件を全数走査。行番号は移設前）:

- **多テーマから参照 → `test_support`**（`pub(super)` 付与 5 項目）: `cue`（`:473`・`cue_apply`＋`reveal`）・`cue_dur`（`:487`・同）・`items_of`（`:497`・同）・`reveal_times_of`（`:1143`・同——`cue_apply` 側の `:804, 896, 993, 1049, 1050` が Choice/Cursor 実消費の「リビール不変」を主張するために呼ぶ）・`REVEAL_INTERVAL`（`:468`——`cue`（`test_support`）と `choice_cue`（`cue_apply_tests`）の双方から参照されるため `test_support` へ）。
- **単一テーマ専用（当該テーマファイルに残置）**: `WarnCounter`（`:732`）＋`impl tracing::Subscriber for WarnCounter`（`:736`）・`choices_of`（`:755`）・`choice_cue`（`:764`）＝ `cue_apply_tests` 専用（参照はいずれも `:1134` 以前）。

**`══ typewriter リビール進行` バナーの帰属**: 本文一致検証（`RustParse.ps1:319-331`）は直前の空行とコメント行を読み飛ばして最初のコード行を探すため、**先行コメント塊は後続項目の本文の一部**になる。移設前 `:1136-1141` のバナー塊は直後の `reveal_times_of`（`:1143`）に属するので、同ヘルパとともに `state_test_support.rs` へ運ばれた（§22.2 で `:4976` を `test_support` へ運んだのと同じ扱い・文言は 1 文字も変えていない）。同じ理由で `actor.rs` の `:1593`（task 8.1）と `:1683`（task 8.2）のバナーも `choice_cue`／`com_world` に付随して `actor_test_support.rs` にある。

#### (4) 4 本に共通する裁定

**タプル構造体ヘルパの分布確認（Implementation Notes・§22.2 の制約）**: テーマ案を立てる前に 4 ファイルのテストモジュール内の構造体を全数走査した結果、**タプル構造体は 1 件も無い**（`LevelCounter`（`actor.rs:977`／`draw.rs:1010`）・`DrawRig`（`draw.rs:1554`）・`WarnCounter`（`state.rs:732`）はいずれも名前付きフィールド）。§22.2 の「利用側ごと 1 テーマに収める」制約は本タスクでも発動しなかった。

**共有ヘルパの可視性（要件 2.4 が許容する機械的調整）**: 3 つの `*_test_support.rs` の**宣言行の先頭にのみ** `pub(super)` を付与した——`actor` 8 行（関数 8）・`draw` 4 行（関数 4）・`state` 5 行（`const` 1・関数 4）の計 **17 行**。付与のみで本文は無変更。`test_support` の内部からしか参照されない項目（`actor` の `REVEAL_INTERVAL`／`draw` の `LevelCounter`＋`impl`）には**付けていない**。**複製は 1 件も作っていない**（`ITEM-EXTRA` 回避・Implementation Notes の集約規則どおり）。

**Implementation Notes の E0659 罠（タスク 3.3 §17.5）への対処**: **共有ヘルパは全て明示 import で受けた**（`use super::test_support::{…};`・グロブは 1 件も使っていない）。新規 16 本に現れるグロブは移設前から在った `use super::*;` の 7 件だけ（`choice_*` 3 本＋`state_*` 4 本）で、これは元のテストモジュールが持っていたものをそのまま運んだものである。誤結合の有無も実測で確認した——`state` の `test_support` が公開する 5 識別子（`REVEAL_INTERVAL`／`cue`／`cue_dur`／`items_of`／`reveal_times_of`）と `state.rs` のモジュールスコープ（`BTreeMap`・`ActorKey`・`CueCommand`・`TalkCue`・`TextLayerConfig`・`TextItem`・`CursorCoord`・`CursorUnit`・`parse_cursor_coord`・`RevealSchedule`・`ChoiceSpan`・`ActorTextState`・`TextLayerState`）に**衝突は 0 件**。`actor`／`draw` はグロブを使っておらず構造的に発生しない。`choice_*` 3 本はグロブを持つが `test_support` を作っていないので同様に発生しない。

**`use` ヘッダの調整（要件 2.4 / 2.6）**: 移設前ヘッダの `use` 項目を各テーマファイルへ全数配ってから、`cargo build --message-format=json` の `unused_imports` 診断（MachineApplicable 提案）が指す識別子だけを機械的に落とした（**全項目を配ってから診断で削る向き**——タスク 4.4 が確立した順序。トレイトメソッド解決に必要な「識別子が本文に現れない import」を落とさないため）。**移設前ヘッダの `use` 項目は 1 つも失われていない**（機械照合: `actor` 39／`draw` 59／`choice` 21／`state` 4 の leaf 識別子すべてが移設後のいずれかのファイルに 1 回以上現れる・欠落 0）。

**要件 2.8 の追加調整（可視性・`use`・モジュール接続）**: **1 件**。`draw_oracle_tests.rs` に `use wintf::com::dwrite::DWriteTextLayoutExt;` を **1 行追加**した——移設前は `draw.rs:1291` のモジュール中途 `use` が `manual_probe_advance`（probe 側）と `drawn_line_cluster_widths`（オラクル側）の双方へ効いていたが、テーマ分割で 2 ファイルに分かれたためオラクル側に届かなくなり `E0599 get_cluster_metrics` が出た。要件 2.8 のとおり**テストロジックには一切触れず `use` の追加だけで解決**した（本文一致検証は `use` 項目を突合対象から外すので判定には影響しない）。これ以外の可視性・接続の追加調整は 0 件。

### 24.3 §11.4 の盲点（複数行文字列リテラル内の行頭空白）— 担当 4 ファイルの独自走査

Implementation Notes の指示に従い、担当ファイルを字句状態追跡つきの独立スキャナ（行コメント・入れ子ブロックコメント・通常文字列・raw 文字列（`#` の数）・バイト文字列・文字リテラルとライフタイムの判別・エスケープを追跡し、「行頭時点で文字列リテラルの内部にいる行」と「直前行が `\` 継続か」を判定する）で全走査した。スキャナの妥当性は既知の唯一の該当箇所 `crates/wintf/src/ecs/window_proc/window_pos_tests.rs` で確認済み（実測出力: `continuation=5 382,429,614,690,691 / blind=1 691`——継続 5 件のうち盲点該当は `:691` のみ、という §13.2／§23.3 と同一の切り分け）。

| 対象 | 複数行にまたがる文字列リテラルの継続行 | 盲点該当（`\` 継続でない行） |
|---|---|---:|
| 移設前 `actor.rs`（`246ddb9` から `git show`） | **0** | **0** |
| 移設前 `draw.rs` | **3**（`:2185, 2280, 2286`） | **0** |
| 移設前 `choice.rs` | **0** | **0** |
| 移設前 `state.rs` | **0** | **0** |
| 移設後の新テストファイル 16 本 | **3**（`draw_oracle_tests.rs:685, 780, 786`） | **0** |
| 移設後の本番 4 ファイル | **0** | **0** |

**結論: 担当 4 ファイルの複数行文字列リテラル 3 件はすべて `\` 継続であり、§11.4 第 1 の盲点の該当行は 0 件。**
`\` 継続では Rust が行頭空白を除去するため、一律 4 スペース de-indent は文字列の中身を変えない。例外処理は不要だった。移設前 3 行と移設後 3 行は 1 対 1 に対応する（`draw:2185, 2280, 2286 → draw_oracle_tests:685, 780, 786`）。

### 24.4 検証（すべて実測・終了コードで判定）

**(a) 本文一致検証（要件 2.4）** — `pwsh -File $V/Compare-RelocatedTests.ps1 -Commit 246ddb9 -OriginalPath <本番> -RelocatedPath "<新テストファイル群>" -Detail`

    actor.rs   : MATCH: test fn 33=33 / helper item 14=14 / mod block 2 / files 6   (exit 0)
    draw.rs    : MATCH: test fn 29=29 / helper item 19=19 / mod block 1 / files 3   (exit 0)
    choice.rs  : MATCH: test fn 50=50 / helper item 15=15 / mod block 3 / files 3   (exit 0)
    state.rs   : MATCH: test fn 59=59 / helper item  9=9  / mod block 1 / files 4   (exit 0)

4 本とも exit **0**。**引数不正の 2 と取り違えていないことを対照実行で確認した**——`-OriginalPath crates/areka-emo-text/src/nonexistent.rs` を与えると `fatal: path ... does not exist in '246ddb9'` を出して **exit 2** になる（不一致の 1 ではない）。

**(a2) 行単位の分類と多重集合突合（スクリプトより強い独自検証）** — 本文一致検証は項目単位・行頭空白非依存であるため、それとは独立に、移設した**全行**を分類した。移設前ブロック本体（`choice.rs` は同伴バナー 2 塊 12 行を含む）の各行を一律 4 スペース de-indent した多重集合（空行を除く）と、新テストファイル群の全行の多重集合（空行を除く）を突合:

| 本番ファイル | 移設元 非空行 | 新ファイル計 非空行 | 「ちょうど −4 スペース」または空行で説明できない行 | 消えた行 | 増えた行 |
|---|---:|---:|---:|---:|---:|
| `actor.rs` | 1,950 | 1,973 | **0** | 15 | 38 |
| `draw.rs` | 1,249 | 1,266 | **0** | 4 | 21 |
| `choice.rs` | 1,117 | 1,117 | **0** | **0** | **0** |
| `state.rs` | 1,023 | 1,028 | **0** | 5 | 10 |

- **`actor.rs`**: 消えた 15 行 = `pub(super)` 付与前の宣言行 **8**（`cue` / `pump_until_idle` / `geo_model` / `cursor_model` / `spawn_reserved_slot` / `opaque_count` / `choice_cue` / `com_world`）＋ 移設前ヘッダの `use` 行 **7**。増えた 38 行 = `pub(super)` 付き宣言行 **8** ＋ 6 ファイルへ再配置・複製された `use` 行 **30**。
- **`draw.rs`**: 消えた 4 行 = 宣言行 **4**（`model_with_font` / `empty_font` / `with_log_cage` / `default_metrics`）。増えた 21 行 = `pub(super)` 付き宣言行 **4** ＋ `use` 行 **17**（うち 1 行が要件 2.8 の `DWriteTextLayoutExt` 追加）。
- **`choice.rs`**: **完全一致（差分ゼロ）**。個別ファイル化のみで可視性付与も `use` の配り直しも要らなかったため、移設した 1,117 行すべてが「ちょうど −4 スペース」または同伴バナーのバイト同値である。
- **`state.rs`**: 消えた 5 行 = 宣言行 **5**（`REVEAL_INTERVAL` / `cue` / `cue_dur` / `items_of` / `reveal_times_of`）。増えた 10 行 = `pub(super)` 付き宣言行 **5** ＋ `use` 行 **5**（`use super::*;` の 3 複製＋`test_support` 明示 import 2）。

差分はすべて要件 2.4 が明示的に許容する調整（可視性付与・`use` の追加／分散）と、要件 2.8 の `use` 追加 1 行だけで説明がつき、説明のつかない行は **4 本とも 0 件**である。

**(b) 対応表フラグメントの全単射検証（要件 2.9）** — `pwsh -File $V/Test-MappingBijection.ps1 -Path $V/mapping/areka-emo-text.csv`

    PASS: 全単射 OK / 行数 239 / 相異なる old_fqn 239 / 相異なる new_fqn 239 / フラグメント 1
      - areka-emo-text.csv: 239 行

exit 0。**タスク 4.4 の 122 行は 1 バイトも触らず、本タスクの 117 行を末尾へ追記した**（`git diff --numstat` = `117 0`・`git show 246ddb9:` の 123 行（ヘッダ＋122 行）と現ファイル先頭 123 行が完全一致）。
内訳は `actor::runtime_tests::*` → `runtime_frame_tests` 9／`choice_contract_tests` 10／`clear_atomicity_tests` 4／`scale_refresh_tests` 6（計 29）、
`draw::tests::*` → `format_metrics_tests` 20／`oracle_tests` 9（計 29）、
`state::tests::*` → `cue_apply_tests` 24／`reveal_tests` 16／`cursor_coord_parse_tests` 19（計 59）。
**`actor::tests::*`（4 本）と `choice::tests::*`／`choice::style_resolve_tests::*`／`choice::decorate_tests::*`（計 50 本）は完全修飾名が変わらないため 1 行も持たない**（設計判断 #2）。
`reason` は全行 `theme_split`、末尾セグメント（関数識別子）は旧新で同一。移設前 `before_default.txt` に対応表の `old_fqn` 117 件すべてが実在することを照合済み（不在 0）。
既存フラグメントとの結合検証（`-Path $V/mapping`）も
`PASS: 全単射 OK / 行数 663 / 相異なる old_fqn 663 / 相異なる new_fqn 663 / フラグメント 7` で exit 0（キー衝突なし）。

**(c) 対応表適用後のテスト名リスト一致（要件 1.8 / 2.2）— ワークスペース水準**

移設後に §10.2 の手順（`cargo test --workspace --no-fail-fast -- --list`（exit 0）→ stdout のみ → `: test$` 抽出 → `$arr = [string[]]@(…)` へ型付け → `[Array]::Sort($arr, [System.StringComparer]::Ordinal)` → UTF-8 BOM 無し・重複行を残す）でリストを採取し、コミット済み `before_default.txt` と**タスク 3.1〜4.5 の 7 フラグメント全部**を渡して突合した:

    BEFORE      : before_default.txt  (4790 行 / 相異なる 4787)
    AFTER       : after2.txt  (4790 行 / 相異なる 4787)
    MAPPING     : 663 行 (7 ファイル) / 適用 663 行 / 未使用 0 行
    LINE COUNT  : before 4790 / after 4790 -> 一致 (Requirement 2.2)
    RESULT: PASS

exit 0。**適用 663 行・未使用 0 行**。移設後リストの SHA256 は `03328FE72AD3B25EC1549ED548590C52B7AC2E7CA14E3E7DAD0298D34BD51270`（中間リストファイル自体はコミットしない）。

**整列器の較正（Implementation Notes の ⚠ 項目）**: コミット済みファイルのハッシュ照合では整列器が動かないため、**同一の未整列生出力（4,790 行）を序数と `Sort-Object` の 2 通りに整列**して digest が割れることを先に確かめた:
序数 `03328FE7…1270` ／ `Sort-Object` `A8AB93FE…9BAB` ／ **1,806 位置が相違**。
分岐点は index 179 の `bake::tests::blit_verbatim_correctness`（序数が先）と `bake_entry_tests::all_transparent_is_empty_entry_not_error`（カルチャが先）で、§10.2 の実測（1,806 位置・index 179・同一の分岐ペア）と完全に一致する。序数比較器が実際に働いていることの直接証跡である。

**(c2) 対応表そのものへの反証（自分の表を疑う検証）** — 対応表が「実際に起きた変化」と過不足なく一致することを、対応表を**使わない**多重集合の対称差で確かめた。

| 検査 | 結果 |
|---|---|
| `before_default.txt` と移設後リストの対称差（対応表なし）: 消えた行 / 現れた行 / 全フラグメント行数 | **663 / 663 / 663**（三者一致） |
| 本タスクの追記 117 行のうち `old_fqn` が実際に消えた行にある数 | **117 / 117** |
| 本タスクの追記 117 行のうち `new_fqn` が実際に現れた行にある数 | **117 / 117** |
| タスク 4.4 の 122 行が依然として整合している数 | **122 / 122** |
| 消えた行がすべて `old_fqn` に在るか | **True**（例外 0） |
| 現れた行がすべて `new_fqn` に在るか | **True**（例外 0） |
| `old_fqn` なのに実際には消えていない行 | **0** |
| `new_fqn` なのに実際には現れない行 | **0** |
| `old_fqn` と `new_fqn` が同一の行（＝変わっていない名前を載せた水増し） | **0** |
| 末尾セグメント（関数識別子）が旧新で相違する行 | **0** |
| `reason` が `theme_split` 以外の行 | **0** |

すなわち対応表は「実際に変わった名前だけ」を「実際に変わったとおりに」記載しており、水増しも取りこぼしも無い。
なお `actor::tests::*` と `choice::*` が対応表に 1 行も無いことは、この対称差でも裏づけられている——**消えた 663 行のどれにも `actor::tests::` / `choice::tests::` / `choice::style_resolve_tests::` / `choice::decorate_tests::` の emo-text 分は含まれない**（`cargo test -p areka-emo-text --lib -- --list` で `actor::tests` 4／`choice::tests` 25／`choice::style_resolve_tests` 13／`choice::decorate_tests` 12 が移設後も同名で実在する）。

**(d) クレート緑（要件 7.2）** — `cargo test -p areka-emo-text --no-fail-fast` → **exit 0**。
**406 passed / 0 failed / 2 ignored**（lib 376＝374 passed＋2 ignored ＋ 統合テスト 9 本 32 ＋ doctest 0）。タスク 4.4 の実績と完全一致。
移設前との本数一致は、**独立に導出した `#[test]` 属性の全数**で裏づけた——`git ls-tree -r 246ddb9 crates/areka-emo-text/` の `.rs` を `git show` で読んで `^\s*#\[test\]\s*$` を数えると **408**、移設後の作業ツリーでも **408** で一致する（cargo の実行総数 406 passed + 2 ignored = 408 とも一致）。
移設後の lib 内訳（`--lib -- --list` の集計・**35 モジュール 376 本**）のうち本タスク担当分は
`actor::tests` 4 ／ `actor::runtime_frame_tests` 9 ／ `actor::choice_contract_tests` 10 ／ `actor::clear_atomicity_tests` 4 ／ `actor::scale_refresh_tests` 6（計 33＝移設前 `actor::tests` 4＋`actor::runtime_tests` 29）、
`draw::format_metrics_tests` 20 ／ `draw::oracle_tests` 9（計 29＝移設前 `draw::tests` 29）、
`choice::tests` 25 ／ `choice::style_resolve_tests` 13 ／ `choice::decorate_tests` 12（計 50＝移設前と**モジュール名ごと完全一致**）、
`state::cue_apply_tests` 24 ／ `state::reveal_tests` 16 ／ `state::cursor_coord_parse_tests` 19（計 59＝移設前 `state::tests` 59）。

**(e) 警告非増加（要件 2.6）** — `cargo build -p areka-emo-text --all-targets` → exit 0・**警告 0 件**。
ワークスペース全域でも §10.5 の手順で再集計した——`cargo build --workspace --all-targets` → exit 0、
`DIAG_COUNT = 16` / `SUMMARY_COUNT = 7` / `GENERATED_SUM = 22` / `DUPLICATES = 6` / `NET = 16` で、§10.5 の移設前基準値 5 数値と**完全一致**。
SUMMARY 行の正規表現は §10.5 逐語どおり `generated \d+ warnings?`（末尾 `?` 込み・タスク 4.3 の申し送り）を用いた。

**(f) 本番本体の無変更** — 移設前コミット `246ddb9` の各ファイル先頭〜旧 `#[cfg(test)]` 直前まで（`actor.rs` 1-857／`draw.rs` 1-963／`choice.rs` 1-536／`state.rs` 1-456）を現作業ツリーと逐行突合し、**4 本とも不一致 0**。
とくに `draw.rs` の**非 `mod` `#[cfg(test)]` 16 項目は元位置にバイト同一で生存**している。属性行 → 項目行の対応は
`:74→:75`／`:76→:77`／`:105→:106`／`:107→:108`／`:109→:110`／`:111→:112`／`:113→:114`／`:115→:116`／`:117→:118`（`use` 宣言 9 件）・
`:429→:430`（`fn cached_probe_count`）・`:539→:541`（`struct FormatKey`）・`:691→:692`（`pub struct DrawExecutor`）・`:706→:707`（`impl DrawExecutor`）・
`:893→:894`（`fn line_layout_creations`）・`:929→:930`（`fn create_target_bitmap`）・`:942→:943`（`fn none_err`）で、
design §Supporting References の全数表（`draw.rs:75,77,106,108,110,112,114,116,118`／`:430,894,930,943`／`:707`／`:541`／`:692`）と**行番号まで完全一致**（ズレ 0・設計判断 #3 のとおり全数残置）。
`actor.rs`・`choice.rs`・`state.rs` には非 `mod` `#[cfg(test)]` 項目は 1 件も無い（`scan_raw.csv` の `nonmod_count` が 0 で、実測でも接続宣言以外の `#[cfg(test)]` は 0）。

**(g) 完了状態の直接確認** — 4 本の本番ファイルに残る `mod …;` / `#[path]` の出現はすべて接続宣言のみ（`actor.rs:858-880` の 6 モジュール・`draw.rs:964-974` の 3 モジュール・`choice.rs:537-547` の 3 モジュール・`state.rs:457-471` の 4 モジュール）。**`#[test]` は 4 本とも 0 件。`mod X { … }` 形式のテストモジュール本体は 1 行も残っていない。** 残る `#[cfg(test)]` は接続宣言のぶんと、`draw.rs` の設計判断 #3 による残置 16 項目だけである。

**(h) クレート単位の完了状態（1.7）** — `crates/areka-emo-text` 配下の `src/**`・`tests/**`・`examples/**` の全 `.rs` を実測した結果、**1,000 行を超えるファイルは `examples/emo-text-layer.rs`（1,434 行）ただ 1 本**である。同ファイルは `#[cfg(test)] mod` を 1 つも持たない（`scan_raw.csv` の `test_lines=0` / `module_count=0`）ため要件 1.1 の対象外であり、design §Non-Goals が明示的に範囲外としている「テストモジュールを持たない巨大ファイル」に当たる（**テストファイルではない**）。したがって**当該クレートのテストファイルはすべて 1,000 行以下**——最大は `tests/attach_wiring_test.rs` の 873 行、`src/` 側の最大は `draw.rs` の 974 行（本番）と `draw_oracle_tests.rs` の 792 行（テスト）である。

**(i) 作業ツリーの範囲** — `git status --porcelain -uall` は本節追記前の時点で下記 21 パスのみ:
変更 5 本（`actor.rs`・`choice.rs`・`draw.rs`・`state.rs`・`verification/mapping/areka-emo-text.csv`）＋未追跡 16 本（新テストファイル）。
タスク 4.4 担当の 3 ファイルと生成 16 本・`lib.rs`・`crates/areka-emo-text/tests/`・`examples/`・他クレート・`Cargo.toml`・`tasks.md`・spec 本体ドキュメントは無変更。
新規に導入した `TODO` / `FIXME` / `TBD` は 0 件。

### 24.5 登記（要件 5.2）— 壊れたテスト・テスト間の状態汚染の所見

**本タスクの範囲（4 ファイル・171 テスト・移設前テストコード 5,812 行）では、壊れたテスト・不正なテストは 1 件も発見しなかった。所有 spec への送付所見は 1 件（下表 #1）で、これは §23.5 #2 の拡張であり新規所見ではない。** いずれも要件 5.1 により本 spec では是正しない。

#### §23.5 #2 への追記（拡張・新規所見を立てない）

**タスク 4.4 が §23.5 #2 で登記した「`include_str!` で本番ファイル本文を読む構造テストの被覆が移設で黙って縮む」件は、本タスクで予告どおり `choice.rs` と `state.rs` にも及んだ。** 同エントリを次のとおり拡張する（判定・送り先・是正しない理由は §23.5 #2 のまま変更なし）。

- 対象檻: `crates/areka-emo-text/src/lib.rs:172` `pure_layer_modules_have_no_windows_imports`。`PURE_SOURCES`（`:173-183`・9 エントリ）のうち**本タスクの担当は `:174` `choice.rs` と `:175` `state.rs` の 2 件**（タスク 4.4 の担当は `:179` `layout.rs` と `:181` `viewbox.rs`）。
- 縮んだ量: `choice.rs` **1,749 → 547 行（−1,202）**・`state.rs` **1,630 → 471 行（−1,159）**。タスク 4.4 分と合わせ、**同テストの走査対象は累計 −6,625 行**になった（タスク 4.4 が記録した −4,294 は「テストコード行」の値で、`include_str!` が読むファイル全体の増減としては **−4,264**＝`layout.rs` 3,294→764 と `viewbox.rs` 2,498→764 が正しい。指標を揃えて再計算した値が −6,625 である。タスク 4.5 レビューの指摘による訂正）。`PURE_SOURCES` の残る 5 エントリ（`writing.rs`・`region.rs`・`segment.rs`・`canvas.rs`・`wrap.rs`）はテストモジュールが 500 行以下で本 spec の必須対象外のため、`areka-emo-text` における縮小はここで打ち止めである。
- **禁止パターンの再走査（本タスクの義務）**: `choice.rs`／`state.rs` から外へ出した内容（新規 7 本＝`choice_tests.rs`・`choice_style_resolve_tests.rs`・`choice_decorate_tests.rs`・`state_test_support.rs`・`state_cue_apply_tests.rs`・`state_reveal_tests.rs`・`state_cursor_coord_parse_tests.rs`）を全数走査した結果、禁止パターン（`use windows` / `windows::` / `windows_core` / `windows_numerics` / `extern crate windows`）の該当は **0 件**。移設後の `choice.rs`・`state.rs` 本体も **0 件**。`windows` という文字列が現れるのは `state_cue_apply_tests.rs:353`・`:375` と `state_reveal_tests.rs:92` の 3 箇所だけで、いずれもスライスの `.windows(2)` 呼び出しであり禁止パターンには一致しない。**実害は 0 件のまま**である。
- **是正しない理由（§23.5 #2 から不変）**: `PURE_SOURCES` はテストの入力値そのものであり、エントリを増やすことは要件 2.4 が禁じる「入力値の変更」に当たる。加えて `lib.rs` は本タスクの担当外（テストモジュール 65 行・必須対象でない）。**縮んだ被覆を戻すなら `<stem>_*_tests.rs` を `PURE_SOURCES` へ足す追加で済む**ことを引き続き申し送る（タスク 7.3 の steering 追記・タスク 7.5 の送付が受け皿）。

#### 記録のみ（是正しない）

| # | 観測 | file:line | 判定 |
|---|---|---|---|
| 1 | 上記 §23.5 #2 の拡張（`choice.rs`／`state.rs` ぶんの被覆縮小） | `crates/areka-emo-text/src/lib.rs:174`（`choice.rs`）・`:175`（`state.rs`） | **送付所見 →`test-cage-determinism`（W6.9）**。§23.5 #2 と同一の所見であり**新規には立てない** |
| 2 | 実 GPU（`GraphicsCore::new()`）と WUC コンポジタを生成し、テストスレッドごとに dispatcher queue を作るテストが `actor`／`draw` の双方にある | `crates/areka-emo-text/src/actor_test_support.rs:119`（`com_world`）・`:121`（`CoInitializeEx(None, COINIT_MULTITHREADED)`）・`crates/areka-emo-text/src/draw_oracle_tests.rs:42`（`make_dispatcher_and_compositor`・ASTA 第一候補／NONE 保険）・`:62`（`DrawRig::new`） | **問題なし・記録のみ**。§22.5 #4／§23.5 #4 と同じ方針で、apartment 生成に失敗しても保険経路へ落ちる冪等な書き方。`DrawRig`／`com_world` はテストごとに新規生成され、テスト間で共有される状態は無い |
| 3 | ログ捕捉に**プロセス大域の subscriber を使っていない**（`tracing::subscriber::with_default` のスレッドローカル捕捉） | `crates/areka-emo-text/src/actor_runtime_frame_tests.rs:21`（`struct LevelCounter`・名前付きフィールド）・`crates/areka-emo-text/src/draw_test_support.rs:31`（同型）・`crates/areka-emo-text/src/state_cue_apply_tests.rs:235`（`struct WarnCounter`）・`crates/areka-emo-text/src/layout_cursor_tests.rs:544`（同・タスク 4.4 生成分） | **送付所見 →`test-cage-determinism`（W6.9）**。捕捉機構そのものは健全で、`with_default` のスレッドローカル捕捉ゆえ並列実行で混線しない。送付理由は**同型のログ捕捉ハーネスが本クレートだけで 4 本重複していること**（`actor`／`draw`／`state`／`layout_cursor_tests.rs`）——§21.5 が `areka-emo-present` の 3 本重複を送付所見として登記したのと**同じ形**であり、分類を揃える。一本化は要件 5.1 が明示的に禁じる領分ゆえ本 spec では行わず、4 本を 4 本のまま保った。**タスク 7.5 はこの行を送付対象として拾うこと**（記録のみに分類したままだと転送されない）。なお §23.5 #5（タスク 4.4）は「本クレートでは 1 本のみで重複は無い」と述べているが、これは当時すでに誤りだった（`246ddb9` 時点で `actor.rs:977`・`draw.rs:1010`・`state.rs:732` に同型が存在）——本行が訂正する |
| 4 | `draw.rs` の比較専用オラクル `DrawExecutor` は**本番コードでありながら `#[cfg(test)]` で丸ごとゲートされている**（struct・impl・支持関数 3 本・`use` 9 本の計 16 項目） | `crates/areka-emo-text/src/draw.rs:692`（`pub struct DrawExecutor`）・`:707`（`impl DrawExecutor`）・`:541`（`struct FormatKey`）・`:430`／`:894`（テスト観測用 inherent メソッド）・`:930`／`:943`（支持自由関数）・`:75,77,106,108,110,112,114,116,118`（専用 `use`） | **問題なし・記録のみ（是正しない）**。設計判断 #3 が 40 件全数残置を裁定しており、本タスクでも 1 バイトも動かしていない（§24.4 (f) で逐行突合済み）。本番経路は `ViewboxExecutor` へ移行済みで、`DrawExecutor` は live-diff 比較のオラクルとして意図的に保全されているもの（`draw.rs:36` 以降の module doc「全域再描画オラクル」節に明記） |
| 5 | 移設で可視性・`use`・モジュール接続の**追加**調整が要るケース（要件 2.8） | `crates/areka-emo-text/src/draw_oracle_tests.rs:5`（`use wintf::com::dwrite::DWriteTextLayoutExt;`） | **1 件**（§24.2 (4) に詳述）。共有ヘルパへの `pub(super)` 付与（宣言行 計 17 本）と `use` の配り直し以外に必要だったのはこの 1 行のみ。§17.5 #4 が警告した同名 shadow ヘルパによる E0659 は本 4 ファイルには存在しない（§24.2 (4) の全数照合で確認）。新規のグロブ import は 1 件も生成していない |

### 24.6 本タスクの成果物

| ファイル | 内容 |
|---|---|
| `crates/areka-emo-text/src/actor_tests.rs` | 新規（79 行・4 テスト・**FQN 不変**） |
| `crates/areka-emo-text/src/actor_test_support.rs` | 新規（131 行・共有フィクスチャ 9 項目・テスト 0） |
| `crates/areka-emo-text/src/actor_runtime_frame_tests.rs` | 新規（556 行・9 テスト・ログ檻ハーネス同居） |
| `crates/areka-emo-text/src/actor_choice_contract_tests.rs` | 新規（681 行・10 テスト） |
| `crates/areka-emo-text/src/actor_clear_atomicity_tests.rs` | 新規（372 行・4 テスト） |
| `crates/areka-emo-text/src/actor_scale_refresh_tests.rs` | 新規（311 行・6 テスト） |
| `crates/areka-emo-text/src/draw_test_support.rs` | 新規（84 行・共有ヘルパ 4 項目＋ログ檻 2 項目・テスト 0） |
| `crates/areka-emo-text/src/draw_format_metrics_tests.rs` | 新規（470 行・20 テスト） |
| `crates/areka-emo-text/src/draw_oracle_tests.rs` | 新規（792 行・9 テスト・要件 2.8 の `use` 追加 1 行） |
| `crates/areka-emo-text/src/choice_tests.rs` | 新規（582 行・25 テスト・**FQN 不変**） |
| `crates/areka-emo-text/src/choice_style_resolve_tests.rs` | 新規（215 行・13 テスト・**FQN 不変**・モジュール間バナー同伴） |
| `crates/areka-emo-text/src/choice_decorate_tests.rs` | 新規（407 行・12 テスト・**FQN 不変**・モジュール間バナー同伴） |
| `crates/areka-emo-text/src/state_test_support.rs` | 新規（57 行・共有ヘルパ 5 項目・テスト 0） |
| `crates/areka-emo-text/src/state_cue_apply_tests.rs` | 新規（637 行・24 テスト） |
| `crates/areka-emo-text/src/state_reveal_tests.rs` | 新規（280 行・16 テスト） |
| `crates/areka-emo-text/src/state_cursor_coord_parse_tests.rs` | 新規（202 行・19 テスト） |
| `crates/areka-emo-text/src/actor.rs` | 末尾のテストモジュール 2 本を接続宣言 6 本へ置換（本番本体 1-857 行は無変更・残 880 行） |
| `crates/areka-emo-text/src/draw.rs` | 同上（接続宣言 3 本・本番本体 1-963 行は無変更・非 `mod` `#[cfg(test)]` 16 項目もバイト同一で生存・残 974 行） |
| `crates/areka-emo-text/src/choice.rs` | 同上（接続宣言 3 本・本番本体 1-536 行は無変更・残 547 行） |
| `crates/areka-emo-text/src/state.rs` | 同上（接続宣言 4 本・本番本体 1-456 行は無変更・残 471 行） |
| `verification/mapping/areka-emo-text.csv` | **追記**（+117 行・計 239 行・全単射検証済み・タスク 4.4 の 122 行はバイト不変） |
| `verification/notes.md` | 本節（§24）を追記 |

コミットは要件 7.1 に従い**クレート単位の 1 コミット**（`areka-emo-text` の残る 4 ファイル分）とする。**本コミットをもって `areka-emo-text` クレートのテスト分離とテーマ分割は完了**し、当該クレートのテストファイルはすべて 1,000 行以下になった（design §File Structure Plan の 7 本すべて着地・新規テストファイル計 32 本）。
