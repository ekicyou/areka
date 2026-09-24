# Brief: areka-P0-nar-install-hardening

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票（台帳 #52）。完了 spec `areka-P0-nar-install` の最終検証が「引受先の無い先送り（担当を決めていただきたい）」として挙げ、`areka-P0-ghost-install` の brief へ申し送られていた 4 件のうち、**`crates/areka-nar/src/` の中だけで閉じる 3 件**を切り出した。
> 切り出しの理由は 2 つ。⑴ 3 件とも「第三者が作った `.nar` を受け取る口の堅さ」の話で、性質がバグ修正である（開発者の優先度＝バグ修正 → α の機能）。⑵ `ghost-install` は α の直列経路の 5 番目で着手が遠く、想定タスクも 20〜25 本で上限の境界に在る。先に抜けば `ghost-install` が軽くなり、穴も早く塞がる。
> 出どころの正本: `.kiro/specs/completed/areka-P0-nar-install/validation-report.md` の「引受先の無い先送り」の節と、`.kiro/specs/areka-P0-ghost-install/brief.md` の「2026-09-19 `nar-install` 実装完了からの申し送り（4 件）」。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: 見知らぬ作者の `.nar` を窓へ落とす第三者。

`areka-nar` は名前の検査（絶対パス・親への上り・区切りの正規化）と総量の上限（`container` の `MAX_TOTAL_DECLARED_SIZE` ＝ 1 GiB）を持つ。抜けているのは次の 3 つである。

1. **1 要素あたりの名前・パスの長さに上限が無い。** 30 万文字の名前が受理される。完了 spec の実装メモ自身が「設計の穴」と書いている。長すぎる名前は OS のファイル作成で落ちるはずだが、**どこで・どんな記録を残して落ちるかを誰も決めていない**。
2. **巻き戻せなかったとき、利用者の元の木がどこに生き残っているかを知る手段が無い。** 失敗の型（`NarError`）は巻き戻せたかどうか（`rolled_back`）と詰まった宛先のパスは持つが、元の木が `<根>/.nar-work/<プロセス識別子>-<連番>/old-<k>/` に残っていることは、失敗の記録の `work` の欄にしか出ない。**データを失ったように見えて、実は残っている**のに利用者へ伝えられない。
3. **失敗の記録の `work` の欄が、確定の段の失敗では一度も検査されていない。** 全数対応のテストの 13 の固定入力はすべて作業フォルダを掘る前（`WorkArea::create` の手前）で拒否されるので、テストが `work` について主張しているのは「空であること」だけである（`crates/areka-nar/src/lib_vocabulary_tests.rs`）。2 の情報が正しく出ることを保証するものが無い。

## Current State

- `areka-nar` に依存しているのは `sample-ghost-kit` だけで、製品（`crates/areka`）はまだ呼んでいない（`git grep -l areka-nar -- 'crates/*/Cargo.toml'`）。**いま直せば、呼び手の追随が 0 で済む。**
- `areka-nar/src` に正典の URL を指すコメント行（`// ukadoc:`）は 0 行（同じ検索は `crates/areka-parsers/src/package/resolve.rs` で 12 行当たる）。正典への紐付けの追記は網羅台帳の更新と一緒に `ghost-install` が行う（本仕様は触らない）。

## Desired Outcome

完了時に次が真になっている。

1. 1 要素あたりの名前・パスの長さに上限が在り、超えた `.nar` は**作業フォルダを掘る前に**、理由と該当の要素を載せた失敗で拒まれる。宛先は無傷。
2. 確定に失敗して巻き戻せなかったとき、元の木の在りかが失敗の値から取り出せる（呼び手が利用者へ見せられる）。
3. 確定の段の失敗を実際に起こす決定論テストが在り、`work` の欄の中身を検査している。

## Approach

`areka-nar` の既存の検査の並びに 1 つ足し、既存の失敗の型に 1 欄足し、既存のテストに確定の段の失敗の入力を足す。新しい仕組みは作らない。

要件段階で決めること（裁定候補 1 件）: **上限の値と数え方**。Windows の `MAX_PATH`（260）は展開先の根の長さに左右されるので、`.nar` の中の相対パスだけで決まる値にする必要がある。推すのは「1 要素の相対パス全体で何文字まで」を 1 つの定数で持つ形（値は検体 6 体の実測の最大に十分な余裕を載せて決める）。areka の裁量なので `doc/COMPAT_ARCHITECTURE.md` §8 へ 1 行登記する。

確定の段の失敗の起こし方: 宛先の中のファイルを開いたままにして入れ替えを失敗させる（`nar-install` の設計の申し送り「使用中の宛先」と同じ現象）。テスト内で自分が開いたハンドルだけを使い、外のプロセスに頼らない。

## Scope

- **In**: 長さの上限と拒否／失敗の値に元の木の在りかを載せる／確定の段の失敗の決定論テストと `work` の欄の検査／`doc/COMPAT_ARCHITECTURE.md` §8 の 1 行。
- **Out**: 網羅台帳 `descript_install` の 11 行を実装済みへ動かすこと・正典 URL のコメント行（`ghost-install` に残す。**本仕様は `doc/ukadoc-coverage/` に触らない**）／利用者への見せ方（`ghost-install`）／`type,package`（roadmap #41）／SHIORI の解放（呼び手の仕事）。

## Boundary Candidates

- 受け口の検査（`areka-nar` の名前の検査の並び）
- 失敗の値（`NarError` と失敗の記録）

## Out of Boundary

- `sample-ghost-kit`・`fold-samples`・検体（振る舞いが変わらないことを確かめるだけ。検体 6 体の最長の名前が上限に掛からないこと）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-nar-install`。
- **Downstream**: `areka-P0-ghost-install`（申し送り 4 件のうち 3 件が消え、残るのは台帳 11 行の 1 件）・`areka-P0-update-engine`（確定の部品を使う場合）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-nar-install` の要件を上書きしない（足すだけ）。要件段階で、同 spec の要件のどれにも反しないことを 1 行ずつ確かめる。
- **Adjacent**: `areka-P0-update-engine` が `areka-nar` の確定の部品を公開して使うと決めた場合だけ `crates/areka-nar/src/` を共有する＝**本仕様を先に着地させる**（XS〜S で早く閉じる）。`doc/COMPAT_ARCHITECTURE.md` §8 は `areka-P0-shiori-loadu` も行を足す＝表の末尾への追記どうしの競合で、後着が取り込む。

## Constraints

- 規模 **XS〜S**（タスク 3〜5 本）。
- 失敗の型へ欄を足すと、`PartialEq` の意味が変わっても既存テストは欄を外しても緑のまま通りうる（記憶 correct-behaviour-can-have-no-cage-at-all）。足した欄は、その欄を外すと赤になるテストで必ず固定する。
- 失敗の記録の重複判定の鍵に、連結した綴りを使わない（記憶 record-dedup-keys-never-concatenate）。
- 既存の全数対応のテスト（13 の固定入力）を 1 本も弱めない。足すのは入力だけ。
