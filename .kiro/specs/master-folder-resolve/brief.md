# Brief: master-folder-resolve

> 決まりの正本: `doc/ideas/play-layer-folders-2026-11-28.md` の 2.4。

## Problem

- **劇の作者**: 劇（play）は、設計の上でゴーストと同じ入れ物で、自分の master を `play/<名>/master/` に置く。いまの areka は master を `ghost/master` 決め打ちで探すので、劇を読めない。
- **areka を作る人**: master の道筋（`<入れ物>/ghost/master`）が本番のコードに散らばっていて、master をどこに置くかの決まりを 1 か所で変えられない。

## Current State

- `ghost/master` を名指す本番のファイルは約 50 本（`crates/` の `*.rs`、テストを除く）。内訳は `areka-session` 12・`areka` 8・`areka-ghost` 6・`areka-runtime` 5・`areka-placement` 4・`pilot` 4・`areka-parsers` 3・`sample-ghost-kit` 3・`shiori-abi` 2、ほかに 1 本ずつの crate が 4 つ。
- master を解く関数は 1 つにまとまっていない。
- 記憶の根は `profile_areka_root(<ゴースト>/ghost/master)`（`crates/areka-ghost/src/sylphya_wiring.rs:102`・`crates/areka-session/src/boot_resolve.rs:381`・`:400`）。
- SSP のゴーストは `ghost/master` だけを持つ（ukadoc の Ghost の `type`: 「SSPではghost/masterにあるdescript.txtならghostと識別される」）。

## Desired Outcome

- master フォルダを、`<入れ物>/master` → `<入れ物>/ghost/master` の順に解く関数が 1 つある。ゴーストにも劇にも同じに当たる。
- 移るのは SHIORI だけでなく、master フォルダの中身全部（SHIORI の DLL・ゴーストの `descript.txt`・辞書・`profile/`）である。
- master を名指していた本番の箇所が、すべてこの関数を通る。記憶の根も、この関数で解いた master の上に作る。
- 両方あるときは `<入れ物>/master` を採り、記録に残す（黙って片方を捨てない）。
- SSP のゴーストは `/master` を持たないので、今までどおり `ghost/master` が引かれる。1 体だけのいまの使い方は、今と同じに動く。
- `/master` を使うゴーストは SSP では動かない、と作者向けの文書に書いてある。

## Approach

境目は「入れ物の根 → master フォルダ」を引く関数 1 つ。先にこの関数を作り、呼び手を寄せる。新しい置き場の規則はこの関数の中だけで決まり、呼び手は規則を知らない。

## Scope

- **In**: master を解く関数・本番の呼び手の寄せ・記憶の根の作り方・両方あるときの記録・作者向けの文書の 1 節・決定論テスト。
- **Out**: 劇の入れ物（`play-package`）・`type,play` の書庫・素材の探す順と識別子（`ghost-material-metadata`）・配下のゴースト。

## Boundary Candidates

- 解く関数（純粋な判断：在る／無いの組から master を選ぶ）とファイルシステムの読み。
- 呼び手の寄せ（起動の解決・読み直し・切り替え・インストールの宛先・記憶の根・テストの道具）。

## Out of Boundary

- `play/` の目録と劇の起動。
- 配下の `ghost/` を配下のゴーストとして読むこと（劇の入れ物の持ち物）。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: `play-package`（劇の master を解く）・`ghost-material-metadata`（入れ物の中を探す）・`multi-ghost-runtime`（ゴーストごとの master）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `multi-ghost-runtime`（同じ起動・切り替えのファイルを触る）・`dev-folder-alias`（探す順の先頭のエイリアス）・完了 `areka-P0-baseware-root-list`（根の並び）。

## Constraints

- 既定の動作を変えない（SSP のゴーストは今と同じ）。
- 触るファイルが多いので、同じファイルを触る spec とは同じウェーブに置かない。
- 確かめていない: 第三者のゴーストで、根に `master` という名前のフォルダを偶然持つもの。要件の段で検体を開いて確かめる。
- 区分 A。
