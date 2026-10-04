# Brief: areka-P0-install-companion-reading

> 2026-10-03 `/kiro-discovery`（再入）で起票。`install-companion-canon` を要件の段で 3 本へ分けた 1 本目（開発者指示「現地点で対応していない機能に影響するのであれば、ロードマップを調整し、複数specへの分解で対応して欲しい。本specは「実施せず、別specに引き継ぐ」」）。**書庫を入れるときの `install.txt` の同梱の読み方を ukadoc に揃える**。起動時にどのバルーンを使うかは `ghost-standard-balloon`、シェルに紐づくバルーンは `shell-companion-balloon` が持つ。
> 前身の資料: `completed/areka-P0-install-companion-canon/`（brief と書きかけの requirements.md。要件 1〜4・6〜8 はこの spec の下書きとして読み直せる・要件 5 は `ghost-standard-balloon` へ）。ソースの指し先は 2026-10-03（main `d4f9e93d`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストとシェルの作者**: ukadoc どおりに書いた `install.txt` の同梱（`balloon*.directory` など）が、areka では違う結果になる。
  - 読まれないはずの番号まで入る。
  - 階層付きの取り出し元が書けない。
  - パス区切りや `..` を含む値で、**インストール全体が断られる**。
  - その結果、SSP で入る書庫が areka では入らない。
- **利用者**: そのような書庫を落とすと `OnInstallFailure` で終わる。

## Current State

正典は ukadoc「Install設定」（https://ssp.shillest.net/ukadoc/manual/descript_install.html ）。areka の読み手は `crates/areka-nar/src/manifest.rs`・`names.rs`・`plan.rs`。

| | ukadoc（逐語） | areka（2026-10-03） |
|---|---|---|
| ⑴ 番号付きの同梱の探索 | 「探索は無印→0→1→2…の順に行われ、見つからない番号が出た時点で打ち切られる。」「無印と0は別のものとして扱われる。」 | 欠番で打ち切らない。`numbered`（`manifest.rs`）は基の名前の後に数字が何桁続いても受ける。`collect_companions` は接頭辞をキーにした `BTreeMap` に集めるのでバイト順になり、`balloon10` が `balloon2` より先に来る |
| ⑵ `*.source.directory` の階層付きの値 | 「SSP 2.9.00以降は、extra\bal1 のようにアーカイブ内の階層を辿る相対パスも指定できる。」「区切りは「\」「/」のどちらでもよい。」 | `check_one_level` で断る（`InvalidDirectoryName`）。取り出し元の判定 `plan.rs` の `in_folder` は先頭の 1 要素だけを比べ、`companion_placement` は 1 要素だけを剥がす |
| ⑶ `*.directory` のパス区切り | 「ここに指定できるのは1階層のディレクトリ名だけで、パス区切りは使えない（「_」に置換される）。」 | 同じ検査で断る |
| ⑷ `*.source.directory` の `..` | 「「..」による上位階層への参照はできない（取り除かれる）。」 | 同じ検査で断る |

- **並びは知らせにも流れる**: 入った物の並び（本体 → 同梱）が `OnInstallCompleteEx` の各 Reference の並びと、旧 `OnInstallComplete` の Reference2（先頭の同梱）になる。経路は `plan.rs` → `install.rs` の `commit_all` → `install/procedure.rs` の `installed_items` → `install/judge.rs`。このため、正典の順へ直すと知らせの並びも正典になる（`procedure.rs`・`judge.rs` は無改変で済む見込み）。
- **取り出し元の文字列を引く場所がほかに 2 つある**: `crates/areka/src/install/terms.rs` の `nested_terms`（`format!("{}/{file}", source_directory)`）と `crates/sample-ghost-kit/examples/fold-samples.rs`。
- **同梱できる種類はバルーンだけ**（`BALLOON_PREFIX`）。`headline`・`plugin` などは `UnsupportedCompanionKind` の警告になる。
- α の検体 3 体の `install.txt` は、4 点のどれも使っていない。

## Desired Outcome

- ⑴〜⑷ が ukadoc どおりに読まれる。
  - 番号は無印 → 0 → 1 → 2… の順に探し、見つからない番号で打ち切る。
  - `*.source.directory` は `\` と `/` の相対パスで書庫の中の階層を辿れる。`..` は取り除く。
  - `*.directory` のパス区切りは `_` に置き換える。
- 入る順と知らせの並びが、正典の探索順になる。
- 置き換え・取り除きの後も、安全の検査（`names.rs`・`plan.rs`）は今の強さのまま効く。置き換えた結果が予約名や長すぎる名前なら、今どおり断る。
- 読み替えたこと（置き換え・取り除き・打ち切り・読まなかった鍵）が、ログに 1 件ずつ残る。
- **`*.directory` を `_` へ置き換える規則を、起動の側（`ghost-standard-balloon`）も同じ関数で使える場所に置く**。`areka-ghost` は `areka-nar` に依存していない（`names.rs` の `is_valid_one_level_name` は `pub(crate)`）ので、置き場所は設計で決める。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `descript_install` の `*.directory`・`*.source.directory` の行が実装に合っている。

## Approach

- 読み手（`manifest.rs`）で ukadoc の読み替えを行い、安全の検査（`names.rs`）は読み替えた後の値に掛ける＝検査を緩めずに、入口だけ正典に揃える。
- 取り出し元の階層の判定（`plan.rs` の `in_folder`・剥がす数）を、正規化した相対パスの前方一致へ広げる。正規化した形を `/` 区切りにすれば `terms.rs` は無改変で済む見込み。
- 記録は `ManifestWarning`（`error.rs`）に種類を足す。areka の側（`lib.rs` が `outcome.warnings` を回す）は変えずに済む。
- 前身の書きかけの要件で決めた形（要件の段で覆してよい）:
  - SSP 2.9.00 以降の形を採る。2.9.00 未満の「区切りを `_` に置き換える」形は採らない。
  - `*.source.directory` が無いときは、`_` へ置き換えた後の `*.directory` の名前を取り出し元にする。
  - `..` と空の区切りを取り除いて何も残らなければ、インストール全体を断る（ukadoc は沈黙）。
  - 先頭に 0 を付けた綴り（`balloon01`）は読まない。

## Scope

- **In**:
  - ⑴〜⑷ の読み替えと、並びの順。
  - 読み替えの記録。
  - `_` への置き換えの関数を、起動の側からも使える場所に置くこと。
  - 決定論のテスト。書庫を組む既存の検体の道具で、欠番・`balloon10`・階層付き・区切り・`..`・取り除いて空・置き換えた結果が予約名、の各場面を作る。
  - `terms.rs`・`fold-samples.rs` の追随。
  - 網羅台帳の 2 行と `doc/COMPAT_ARCHITECTURE.md` §8（完了 `nar-install` の要件 3.9・3.12 の読み方を上書きすること、ukadoc が沈黙するところの決め）。
- **Out**:
  - 起動時にどのバルーンを使うか（`ghost-standard-balloon`）と、シェルに紐づくバルーン（`shell-companion-balloon`）。
  - `type,package` と `developer_options.txt`（覚え書き「配布物を束ねる／作る側の 3 件」）。
  - 本体の `directory` の扱い。ukadoc は区切りについて書いていないので、今の「断る」を変えない。
  - 同梱の種類を増やすこと（`plugin*` など）。
  - 使用中のフォルダへの上書き（`install-live-target-hazards`）。

## Boundary Candidates

- `install.txt` の読み手（`manifest.rs` の `classify`・`numbered`・`collect_companions`）
- 名前の安全の検査（`names.rs`）と取り出し元の判定（`plan.rs`）
- 取り出し元の文字列の利用者（`install/terms.rs`・`fold-samples.rs`）

## Out of Boundary

- 書庫の安全な展開の規則（シンボリックリンク・絶対パス・長さの上限 200）は変えない（完了 `nar-install`・`nar-install-hardening`）。
- `crates/areka-nar/src/install.rs` の確定の手順と、`crates/areka/src/install/` の `terms.rs` 以外（`install-live-target-hazards`）。
- `crates/areka-ghost/src/catalog.rs`・`crates/areka/src/boot_resolve.rs`・`boot_config.rs`（`ghost-standard-balloon`）。

## Upstream / Downstream

- **Upstream**: 完了 `nar-install`・`nar-install-hardening`・`ghost-install`・`baseware-root-layout`。
- **Downstream**:
  - `ghost-standard-balloon`: 「最初の 1 個」＝この spec の探索順で最初に見つかった同梱。名前の `_` 置き換えも同じ関数を使う。
  - `shell-companion-balloon`。
  - pasta の側の `pasta-check-bundled-balloon`（本リポジトリの外）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `nar-install` の要件の読み方の上書きは `doc/COMPAT_ARCHITECTURE.md` §8 に記す）。
- **Adjacent**: `install-live-target-hazards`（同じ `areka-nar` とインストールの手続き＝触るファイルを着手時に照合）・`ghost-standard-balloon`（`_` の置き換えの関数を共有する）。

## Constraints

- 意味論は ukadoc から輸入する（SSP の実測には合わせない）。
- 第三者の書庫を受ける口なので、安全の検査を緩めない。
- 決定論のテスト網羅は必達。ログの無い失敗の経路を作らない。
- `crates/areka-nar/src/plan_tests.rs` は 916 行なので、新しいテストは兄弟の新しいファイルへ置く。
- 検体・一時フォルダはワークツリーの `target\` の下だけ。
- 段は**その他**・ウェーブ **C2-⑥**（前身の席を引き継ぐ）・規模 S〜M（7〜11 タスク）・Fable 推奨。


## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S〜M（7〜11 タスク）。変わらず。
- 前提の状態: 前提の spec は無い＝満たす。C2-⑥ の席で未着手（`.kiro/specs/areka-P0-install-companion-reading/` は brief だけ）。C2 の他の 6 本のうち未着手は `mouse-drag-events` だけで、触るファイルの重なりは 0（向こうは kanade と `input_events/mod.rs`）。
- 崩れた前提／古くなった位置:
  - 起票（main `d4f9e93d`）の後に `crates/areka-nar/` へ入ったのは `Cargo.toml` の版の 1 行だけ。`manifest.rs`（`classify`・`numbered`・`collect_companions`・`check_one_level` が `directory`・`<prefix>.directory`・`<prefix>.source.directory` の 3 か所で呼ばれる）・`names.rs`・`plan.rs`（`companion_placement`・`in_folder`）は brief の記述のまま。`crates/areka/src/install/terms.rs` は棚卸⑳（PR#211・利用条件の文の切り詰めが絵文字を割る件）で 7 行変わったが、`nested_terms` の `format!("{}/{file}", companion.source_directory)` は同じ。`crates/sample-ghost-kit/examples/fold-samples.rs` の `strip_folder(relative, &companion.source_directory)` も同じ。
  - **`_` への置き換えの関数の置き場所**: `areka-nar` と `areka-ghost` はどちらも `areka-parsers` に依存し、互いには依存しない（`crates/areka-ghost/Cargo.toml`・`crates/areka-nar/Cargo.toml`）。`areka-parsers` に置けば新しい依存の辺は 0 本で、`ghost-standard-balloon`（`catalog.rs`）・`shell-companion-balloon` もそのまま使える。`areka-ghost` の `catalog.rs` はすでに `areka_parsers::kv::parse_kv` で `install.txt` を読んでいる。＝勝ちが明白なので議題にせず設計で決めてよい。
  - `install-live-target-hazards` と同じ `crates/areka/src/install/` を使うが、こちらが触るのは `terms.rs`（と `terms_tests.rs`）だけ、向こうは `judge.rs`・`procedure.rs`・`overwrite.rs`・`desk.rs`＝ファイルの重なり 0。条件は「`procedure.rs`・`judge.rs` を無改変で済ませる」（brief の見込み）を守ること。`crates/areka-nar/src/install.rs`（確定の手順）にはどちらも触らない。
- 触るファイル（並走の照合用）:
  - `crates/areka-nar/src/{manifest.rs 431, names.rs, plan.rs, error.rs}`・新規の兄弟テスト（`plan_tests.rs` 916・`manifest_companion_tests.rs` 528 は伸ばしすぎない）
  - `crates/areka-parsers/src/` の新規の小さなモジュール（`_` への置き換え・`..` の取り除き）
  - `crates/areka/src/install/terms.rs`・`terms_tests.rs`
  - `crates/sample-ghost-kit/examples/fold-samples.rs`
  - `doc/ukadoc-coverage/ledger/assets.toml`（`descript_install` の `*.directory`・`*.source.directory` の 2 行）・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: 棚卸の時点で新しいものは無し（brief の「前身の書きかけの要件で決めた形」4 点を要件の段で確かめる）。
- 見つけた穴: 無し（brief の ⑴〜⑷ そのものが正典とのずれ＝本 spec の仕事）。

## 2026-10-04 ウェーブ C3-⑪（棚卸㉑）

- 段は「その他」。C3 は 11 本並走（`roadmap.md`「ウェーブ編成」の C3 の行が正本）。着手は最新の main から。
- 同じウェーブの約束: `crates/areka-parsers/src/lib.rs` と `shell/` に触らない（`_` への置き換えの関数は `areka-nar` か `areka-parsers` の既存の子の下へ）。`crates/areka/src/install/` は `terms.rs` だけで、`procedure.rs`・`judge.rs` は無改変。

## 2026-10-04 要件の討議での決め（`*.directory` の区切りは置き換えない）

- 開発者の決め: 同梱の `*.directory` に区切りを含む値は、ukadoc の「パス区切りは使えない」を採って**今どおり断る**。括弧の中の「`_` に置換される」は採らない（「バグを受け入れる必要はないと思う」）。
- 上の Current State の ⑶、Desired Outcome の「`*.directory` のパス区切りは `_` に置き換える」と「置き換える規則を起動の側も使える場所に置く」、Approach の「`*.source.directory` が無いときは `_` へ置き換えた後の名前」、Scope の「`_` への置き換えの関数」、再測定の節の `crates/areka-parsers/src/` の新規モジュールは、どれも取り下げた。正本は `requirements.md`。
- 設計の討議の決め: `crates/sample-ghost-kit/examples/fold-samples.rs` は広げない（階層付きの取り出し元を持つ検体が 0 体）。Scope の「`fold-samples.rs` の追随」は取り下げ。
