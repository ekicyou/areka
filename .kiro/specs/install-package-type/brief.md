# Brief: install-package-type

> 起票: 2026-10-23（棚卸㉙・覚え書きの「`install.txt` の `type,package`」から格上げ）。
> 版の段は「いつか」・区分 C（今は書庫ごと断り、記録も残る＝黙って壊れはしない。正典にある入れ方の 1 つが使えないだけ）。

## Problem

- **利用者**: SSP 専用の「複数インストール用パッケージ」（1 つの書庫に、ゴースト 2 体や、ゴーストとバルーンなど、それぞれ自分の `install.txt` を持つ中身を束ねたもの）を areka に投げ込むと、書庫ごと断られる。SSP なら束ねた中身が順に入る。
- **作者**: 束ねて配る形を、areka の利用者向けには使えない。

## Current State

- **正典（ukadoc・2026-10-23 に MCP で引いた）**:
  - 「type,種別」: 種別の並びの最後に `package`「（SSPのみ）複数インストール用パッケージ」。
  - 「directory,ディレクトリ名」: 「省略不可/type,packageの場合のみ不要。」
  - 「bootghost,ディレクトリ名」（2.7.52）: 「type,packageのみ有効。initial_install.narまたはnar関連付けからSSPを起動した際に、起動すべきゴーストのディレクトリ名を指定する。」
  - 「UKADOC Project インストール」の「(SSP専用)複数インストール用パッケージ」: 最上位にパッケージ用の `install.txt` を置き、その下の中身のフォルダごとに中身用の `install.txt`（と `developer_options.txt`）を置く。「パッケージ用および個別の内容用それぞれに、別のinstall.txtが必要」。単体の書庫にできるものは何でも束ねられるが、`shell` や `supplement` のように入れる相手が決まっているものは、相手の違うものを混ぜないこと。
  - 中身を入れる順・中身の 1 つが断られたときの残り・ゴーストへの知らせ方は、ukadoc の写しでは引けなかった（要件の段で SSP で測る）。
- **areka**:
  - 種別を読む所は `crates/areka-nar/src/manifest.rs`（491 行）の `parse_kind`（223 行目）。`ghost`・`shell`・`supplement`・`balloon` の 4 つだけを受け、ほかは全部 `RefuseReason::UnsupportedType { found }`（230 行目）で断る。`package` を名指しで断る腕は無く、「知らない種別」の 1 つとして落ちる。種別の型 `InstallKind`（同じファイルの 90 行目）も 4 つだけ。
  - `install.txt` を読む入口 `parse_manifest`（179 行目）は、種別の次に `name` と `directory` を必ず要るものとして読む。パッケージの `install.txt` は `directory` を持たないので、種別だけを受けても次で断られる。
  - 製品の側の振り分けは `crates/areka/src/install/judge.rs`（260 行）の 121〜137 行目で、`UnsupportedType { found: Some(_) }` を `OnInstallFailure` の語 `unsupported` に写す。
  - 1 つの書庫で 3 つ以上入ったときの知らせ `OnInstallCompleteEx` の送り手は既にある（`crates/areka/src/install/procedure.rs`・`judge.rs`）。ukadoc「OnInstallComplete」は「1回のインストールで同時に入った物のうち先頭の2件しか通知されない」「3件以上が同時にインストールされうる場合は、OnInstallCompleteExを使う事」。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml`: `bootghost` の行（3939 行目）は absent・持ち主なしで、注記に「束ねた形を解く仕様はまだ起票されていない」「起票されたらこの行の担当を書くこと」。`type` の行（4024 行目）と `directory` の行（3967 行目）は implemented・持ち主 `areka-P0-nar-install`。

## Desired Outcome

- パッケージの書庫を投げ込むと、束ねた中身が 1 つずつ、今の 4 種別と同じ手順（同意の確かめ・上書きの確かめ・ゴーストへの知らせ）で入る。
- 入った結果を、1 回のインストールとしてゴーストへ知らせる（どのイベントにどう並べるかは SSP で測って決める）。
- `bootghost` を読み、起動の入口からの入れ方（`startup-install-option` の `initial_install.nar`・`--install`）がパッケージを入れたときに、起こすゴーストの名前として渡す。
- 中身の 1 つが断られたときに残りをどうするか（続けるか・全部やめるか）を、SSP と比べて決めてある。
- 決定論のテストで「ゴースト 2 体のパッケージ」「ゴーストとバルーンのパッケージ」「中身の `install.txt` が欠けたパッケージ」「中身の 1 つが断られるパッケージ」を固定する。

## Approach

- 最初の段は SSP で測る（開発者の機械の SSP の MCP と投げ込み）: 入れる順・失敗したときの残り・イベントの形・中身の中にまたパッケージがあるときの扱い。
- 読み手は、最上位の `install.txt` がパッケージだと分かったら、下のフォルダごとに今の `parse_manifest` を回す（読み手の決まりを 2 つ持たない）。展開と書き込みは、今の `areka-nar` の手順を中身ごとに使う。

## Scope

- **In**: `type,package` の受け入れ・中身ごとの `install.txt` の読み・中身ごとの入れ方・結果の知らせ・`bootghost` の読み取りと起動の入口への受け渡し・網羅台帳の行（`bootghost` の持ち主・`type` の行の注記）・決定論のテスト・実機の確かめ（パッケージの書庫を作って投げ込む）。
- **Out**: `developer_options.txt`（書庫を作る道具が読む・roadmap の「予約」の `createnar`）・起動の入口そのもの（`startup-install-option`）・`plugin`・`headline`・`language`・`calendar skin`・`calendar plugin` の種別（今のまま断る）。

## Boundary Candidates

- `install.txt` の読み手（`crates/areka-nar/src/manifest.rs`）と、書庫の中の置き場所の計画（`crates/areka-nar/src/plan.rs`）
- 製品の側の手順と知らせ（`crates/areka/src/install/procedure.rs`・`judge.rs`）

## Out of Boundary

- 今の 4 種別の入れ方の決まり（同意・上書き・`refresh`）は変えない。

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-nar-install`（読み手と手順）。
- **Downstream**: `startup-install-option`（`bootghost` を使う）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-nar-install` の決まりに種別を 1 つ足す。
- **Adjacent**: `startup-install-option`（`install/judge.rs`・`install/desk.rs`）・`readonly-mode`（`crates/areka/src/install/` と `crates/areka-nar/src/install.rs` で書き込みを止める）・`areka-test-binary-stability`（`areka-nar` のテストの束ね）。

## Constraints

- 1 ファイル 1,000 行（`manifest.rs` 491・`crates/areka-nar/src/install.rs` 666・`procedure.rs` 521）。
- 決定論のテストは必達。実機の根と一時フォルダはワークツリーの `target\` の下だけ。
- 失敗はメッセージボックスでなく既定のゴーストの台詞で伝える決まりは、そのまま（インストールの警告は今の例外のまま）。

## 2026-10-23 棚卸㉙の測り

- **触るファイル**: `crates/areka-nar/src/{manifest.rs 491, plan.rs 398, lib.rs 359}` と兄弟のテスト・`crates/areka/src/install/{procedure.rs 521, judge.rs 260}` と兄弟のテスト・`doc/ukadoc-coverage/ledger/assets.toml` の 2〜3 行。
- **規模**: S〜M（5〜8 タスク。SSP の測り 1・読み手 2・手順と知らせ 2・`bootghost` 1・実機 1）。覚え書きの XS〜S は「断り方だけ」の見立てで、束ねた中身を入れるところまでやると広がる。
- **先に要るもの**: 働きの上では無い。`bootghost` を起動の入口へ渡すところだけ `startup-install-option` の後（あちらが入口を作る）。同じウェーブに置けない相手（v0.1.0）: `startup-install-option`（`install/judge.rs`）・`readonly-mode`（`crates/areka/src/install/`・`crates/areka-nar/src/install.rs`）。SSP での測りは開発者の機械（SSP を動かす＝許しを取る）。
- **優先度の区分**: C。
- **要件定義のモデル**: Opus（今の 4 種別の手順を中身ごとに回すだけ・先例あり）。
- **議題の候補**: ⑴ 中身の 1 つが断られたときに残りを入れるか（SSP で測る）。⑵ 結果をどのイベントに載せるか（中身ごとに `OnInstallComplete` を出すか、1 回の `OnInstallCompleteEx` にまとめるか）。⑶ 中身の同意（`accept`）と上書きの確かめを中身ごとに聞くか。⑷ 中身の中にまたパッケージがあるとき（断るか）。
