# Brief: areka-P0-install-companion-canon

> 2026-10-02 `/kiro-discovery` で起票（`alpha-release-signoff` の完了の手順の中・開発者指示「あ、起票はあとでやってくれますよね。「実装完了を承認」スキルは最後に実施しますし。その前提で、今は実装に戻ってください。」）。roadmap「alpha-release-signoff の持ち越し」節。出どころは `alpha-release-signoff` の完成判定 `verification/alpha-completion.md` §6 と受入記録 `verification/acceptance-record.md` §8.8（セッション「pasta棚卸」〔`pasta_check` の同梱バルーンの対応〕からの問い合わせで見つかった）。**正典（ukadoc）との違いを埋める spec**。本文のソースの指し先は起票時（`c430480d`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストとシェルの作者**: ukadoc どおりに書いた `install.txt` の同梱（`balloon*.directory` など）が、areka では違う結果になる——読まれないはずの番号まで入る、階層付きの取り出し元が書けない、パス区切りや `..` を含む値で**インストール全体が断られる**。SSP で入る書庫が areka では入らない。
- **利用者**: そのような書庫を落とすと、`OnInstallFailure` で終わる。

## Current State

正典は ukadoc「Install設定」（https://ssp.shillest.net/ukadoc/manual/descript_install.html ・2026-10-02 に読んだ）。areka の読み手は `crates/areka-nar/src/manifest.rs`・`names.rs`・`plan.rs`。

| | ukadoc（逐語） | areka（起票時） |
|---|---|---|
| ⑴ 番号付きの同梱の探索 | 「探索は無印→0→1→2…の順に行われ、見つからない番号が出た時点で打ち切られる。」「そのため欠番を作ってはいけない。例えばballoon0とballoon2だけを書いた場合、balloon1が無い時点で打ち切られるため、balloon2は読まれない。」「無印と0は別のものとして扱われる。」 | 欠番で打ち切らない。`balloon` の直後が数字だけの接頭辞をすべて読み（`numbered`・`classify`）、宛先を接頭辞のバイト順の表に集める（`collect_companions` の `BTreeMap`）＝`balloon10` が `balloon2` より先に並ぶ。無印と 0 は別に扱う（正典と同じ） |
| ⑵ `*.source.directory` の階層付きの値 | 「SSP 2.9.00以降は、extra\bal1 のようにアーカイブ内の階層を辿る相対パスも指定できる。」「区切りは「\」「/」のどちらでもよい。」 | 1 階層の名前の検査（`names.rs` の `is_valid_one_level_name`）に掛け、インストール全体を断る（`InvalidDirectoryName`）。取り出し元の判定（`plan.rs` の `in_folder`）も 1 階層の前提 |
| ⑶ `*.directory` のパス区切り | 「ここに指定できるのは1階層のディレクトリ名だけで、パス区切りは使えない（「_」に置換される）。」 | 同じ検査で断る |
| ⑷ `*.source.directory` の `..` | 「「..」による上位階層への参照はできない（取り除かれる）。」 | 同じ検査で断る（`..` は末尾がドットとして撥ねられる） |

- ukadoc は 2.9.00 より前の形も書いている——「2.9.00未満では区切りが「_」に置換されるため、1階層のディレクトリ名しか指定できない。」
- 一周への影響は無かった（α の検体 3 体の `install.txt` はどれも 4 点のどれも使わない＝受入記録 §8.8）。開発者の裁定（2026-10-02）「どちらも A で進めて」＝α では直さず持ち越し、説明書には書かない。
- **起動時の同梱バルーンの選び方（問いとして記録・決めていない）**: ukadoc の同時インストールの節は「なお、ゴーストやシェルに紐づくバルーンとして設定されるのは最初の1個だけである。」「2個目以降はインストールされるだけで、そのゴーストの標準バルーンにはならない。」と書く。areka の起動時のバルーンの解決（`crates/areka/src/boot_config.rs` の `resolve_balloon_for_ghost` の「同梱」の段）は `crates/areka-ghost/src/catalog.rs` の `companion_balloon` を使い、**無印の `balloon.directory` だけを読む**（「番号付きの鍵は読まない」）。番号付きだけで書いたゴースト（α の検体 `claudia` は `balloon0`・`balloon1`）では、「最初の 1 個」の `balloon0` が紐づかず、次の段（唯一・既定・無作為）へ進む。「紐づく」が起動時の既定のバルーンの意味か、シェルに紐づくバルーンを areka がどう扱うかは、要件の段で ukadoc を読み直して開発者に尋ねる。

## Desired Outcome

- ⑴〜⑷ が ukadoc どおりに読まれる: 番号は無印 → 0 → 1 → 2… の順に探し、見つからない番号で打ち切る（読まなかった鍵は記録に残す）／`*.source.directory` は `\` と `/` の相対パスで書庫の中の階層を辿れる・`..` は取り除く／`*.directory` のパス区切りは `_` に置き換える。
- 置き換え・取り除きの後も、書庫の外へ書き出さない安全の検査（`names.rs`・`plan.rs`）は今の強さのまま効く（置き換えた結果が予約名や長すぎる名前なら今どおり断る）。
- 読み替えたこと（置き換え・取り除き・打ち切り）はログに 1 件ずつ残る。
- 起動時の同梱バルーンの選び方は、問いへの答えに従って直すか、今のまま理由を `doc/COMPAT_ARCHITECTURE.md` §8 に記す。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `descript_install` の該当の行が実装に合っている。

## Approach

- 読み手（`manifest.rs`）で ukadoc の読み替えを行い、安全の検査（`names.rs`）は読み替えた後の値に掛ける＝検査を緩めずに入口だけ正典に揃える。
- 取り出し元の階層の判定（`plan.rs` の `in_folder`）を、正規化した相対パスの前方一致へ広げる。
- 2.9.00 より前の形（`*.source.directory` の区切りを `_` に置き換える）と 2.9.00 以降の形のどちらに揃えるかは要件で決める（推しは今の ukadoc が本文に書く 2.9.00 以降）。

## Scope

- **In**: ⑴〜⑷ の読み替え・並びの順・読み替えの記録・決定論のテスト（書庫を組む既存の検体の道具で、欠番・`balloon10`・階層付き・区切り・`..` の各場面）・起動時の同梱バルーンの問いの答えの実装か記録・網羅台帳の更新・`doc/COMPAT_ARCHITECTURE.md` §8。
- **Out**: `type,package` と `developer_options.txt`（登記だけの行「配布物を束ねる／作る側の 3 件」）・本体の `directory` の扱い（ukadoc は区切りについて書いていない＝今の「断る」を変えない）・同梱の種類を増やす（`plugin*` など今は扱わない種別）・使用中のフォルダへの上書き（`install-live-target-hazards`）。

## Boundary Candidates

- `install.txt` の読み手（`manifest.rs` の `classify`・`numbered`・`collect_companions`）
- 名前の安全の検査（`names.rs`）と取り出し元の判定（`plan.rs`）
- 起動時の同梱バルーンの選び方（`areka-ghost` の `catalog.rs`・`areka` の `boot_config.rs`）

## Out of Boundary

- 書庫の安全な展開の規則（シンボリックリンク・絶対パス・長さの上限 200）は変えない（完了 `nar-install`・`nar-install-hardening`）。

## Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。完了 `nar-install`・`nar-install-hardening`・`ghost-install`・`baseware-root-layout` の上に建つ。
- **Downstream**: pasta の側の `pasta-check-bundled-balloon`（pasta のリポジトリ・本リポジトリの外）は ukadoc に従って書くので、本 spec の着地で areka でも同じ結果になる。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `nar-install` の要件 3.12〜3.16 の読み方を上書きするなら `doc/COMPAT_ARCHITECTURE.md` §8 に記す）。
- **Adjacent**: `install-live-target-hazards`（同じ `areka-nar` とインストールの手続き＝触るファイルを着手時に照合）・登記だけの行「配布物を束ねる／作る側の 3 件」。

## Constraints

- 意味論は ukadoc から輸入する（SSP の実測には合わせない）。ukadoc の原文に誤記の疑いがあれば兄弟の項目の型で読み、字面どおりだと作者の意図に反するときだけ開発者へ上げる。
- 第三者の書庫を受ける口＝安全の検査を緩めない。決定論のテスト網羅は必達。ログの無い失敗の経路を作らない。
- 検体・一時フォルダはワークツリーの `target\` の下だけ。
