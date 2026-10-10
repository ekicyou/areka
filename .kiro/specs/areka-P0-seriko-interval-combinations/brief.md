# Brief: areka-P0-seriko-interval-combinations

> 2026-10-10 棚卸㉓で `areka-P0-seriko-trigger-intervals` から切り出した。同 spec が 22〜27 タスクで上限の 20 を超え、しかもこの部分だけが合成（emo-compose）まで及ぶため。もとは `animated-image-playback` の要件討議（2026-10-05・議題 4）で `seriko-trigger-intervals` が引き受けた仕事である。

## Problem

- **シェルの作者**: interval に `bind+always`（着せ替えが有効な間だけ繰り返す）のような、`always` を含む `+` の組み合わせを書いても、何も動かず、何も言われない。areka は元の綴りを添えた控えめな記録を残すだけで、その記録は既定の水準では見えない。
- **開発者**: 網羅台帳の `always` の行は、この穴のために「縮退・黙って壊れる」のままになっている。

正典の出どころ（ukadoc の文書 MCP で引いた見出し。文は要約）:

- `animation*.interval,インターバル`（descript_shell_surfaces・https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation*.interval%2C%E3%82%A4%E3%83%B3%E3%82%BF%E3%83%BC%E3%83%90%E3%83%AB ）: SSP だけ、`+` で区切って並べると組み合わせを指定できる。**正典が書くのはこの 1 文だけ**で、組める語・語の順・語の数・数値つきの語の書き方・組んだときの意味のどれも書いていない（MCP で `bind+` を引いても 0 件）。
- `always`（同・https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#always ）: そのサーフェスである間、繰り返し再生する。

## Current State

今の木（main `ee3af616`）で確かめたこと。細かい名前は、切り出し元の brief（`.kiro/specs/areka-P0-seriko-trigger-intervals/brief.md`）の「`animated-image-playback` からの申し送り」と「繰り返しの仕組みの形」が正本（この spec がその 2 つの節を引き継ぐ）。

- 読み手: 綴りを型へ写す `normalize_interval`（`crates/areka-parsers/src/shell/decode.rs`）は、`bind`・`random`・`bind+random` を完全一致で見分け、ほかは元の綴りのまま `Interval::Other` に写す。`bind+always` は `Interval::Other("bind+always")` になる（語は落ちていない）。
- `always` の単独かを見る 1 関数: `is_always_interval`（`crates/areka-emo-compose/src/nesting.rs`・小文字の完全一致）。合成・見える部品・seriko の表が共有する。
- 着せ替えの種類かを見る 1 関数: `is_bind_interval`（`crates/areka-emo-compose/src/plan.rs`・型の `Bind` と `BindRandom` だけが真）。
- この 2 関数を引く場所: 重ねる対象を選ぶ `flatten_surface`（`plan.rs`）・経過 0 の絵を描く `always_rest_target`（`plan_always.rs`）・外形を数える `flatten_extent`（`plan_extent.rs`）・面ごとの部品の 1 行 `SurfaceParts` の `bind_ids` と `always_rest`（`nesting.rs`）。
- seriko: 繰り返しの引き金 `LoopTrigger::Always`（`crates/areka-seriko/src/table.rs`）は着せ替えの印を持たない。部品の門 `gate`（`parts.rs`）と、一番上のサーフェスの刻み `on_tick`（`looper.rs`）が、着せ替えの番人と `always` を別々の腕で扱う。表を組む `AnimationTable::from_world_and_films` は、組み合わせを `Interval::Other` の腕で記録して採らない。
- 網羅台帳: `ukadoc:descript_shell_surfaces:always:1` は `degraded`。担当の欄は今 `areka-P0-seriko-trigger-intervals`（この spec へ付け替える）。

## Desired Outcome

- `bind+always` のアニメーションが、着せ替えが有効な間だけ繰り返し再生される。無効の間は、経過 0 の絵も描かれない。
- `always` を含むほかの組み合わせは、要件で決めた読み方で動くか、読めない綴りとして**見える水準の記録**を残す（「書いたのに何も動かず、何も言われない」を無くす）。
- 外形と、見える部品の集まりが、組み合わせでも単独のときと同じ決まりで求まる。

## Approach

- 「`always` を含むか」と「着せ替えの種類か」を、組み合わせの綴りでも答えられるようにする。2 つの関数を引く場所の全部（上の Current State の一覧）に同じ答えを通す。
- `+` の綴りを分ける場所は 1 つにする。読み手で型に写す形と、`Interval::Other` の綴りを下流の 1 関数で分ける形（読み手に触らない）を設計で比べる。
- 繰り返しの引き金に着せ替えの印を持たせ、着せ替えが無効の間は時計を作らず、経過 0 の絵も描かない。
- 時計・繰り返しの計算は `animated-image-playback` のものをそのまま使う。3 つ目の時計を作らない。

## Scope

- **In**:
  - `always` を含む `+` の組み合わせ（`bind+always` と、語の順の違い `always+bind`）。
  - 読めない組み合わせの記録を、見える水準にすること。
  - 決定論のテスト（着せ替えの有効・無効で、再生・経過 0 の絵・外形がどうなるか）・網羅台帳の `always` の行・`doc/COMPAT_ARCHITECTURE.md` §8（正典が黙っている所の決めごと）・実機の確かめ。
- **Out**（別途）:
  - `always` の単独（`animated-image-playback`・完了）。`bind+random`（実装済み）。
  - `talk`・`runonce`・`periodic` の単独（`seriko-trigger-intervals`）。`yen-e`・`never`・`\i`（`seriko-script-triggers`）。
  - `always` を含まない組み合わせ（`bind+runonce` など）は、議題 1 の答えで In か Out かが決まる。

## Boundary Candidates

- `+` の綴りを分ける 1 か所（読み手か、合成の 1 関数か）。
- 合成の 4 ファイル（`nesting.rs`・`plan.rs`・`plan_always.rs`・`plan_extent.rs`）。
- seriko の引き金と門（`table.rs`・`parts.rs`・`looper.rs`）。

## Out of Boundary

- 繰り返しの計算と時計の作り（`crates/areka-seriko/src/timeline.rs` の `lap_of`・`always_at`、`PartClocks`）。使うだけで変えない。
- 外形の求め方そのもの（`extent-element-offset`）。この spec は「数える対象に組み合わせを入れる」所だけを触る。

## Upstream / Downstream

- **Upstream**: `seriko-trigger-intervals`（seriko の `table.rs`・`looper.rs`・`parts.rs` を分け合う。議題 1 で `always` を含まない組み合わせを入れるなら、働きの上でも要る）。完了 `animated-image-playback`（繰り返しの仕組み）。
- **Downstream**: 無し。

## Existing Spec Touchpoints

- **Extends**: `seriko-trigger-intervals` の範囲から、`always` を含む組み合わせを引き取る（もとは `animated-image-playback` の要件 4.9 が残したもの）。
- **Adjacent**（同じファイルを触るので直列）:
  - `extent-element-offset`・`animated-image-import`（`plan_extent.rs`）。
  - `draw-methods-canon`（`plan.rs`・`nesting.rs`・`plan_always.rs` の「描ける語か」の門）。
  - `seriko-script-triggers`（seriko の `table.rs`・`looper.rs`）。

## Constraints

- 時刻は正確に扱う（待ち時間を丸めない・更新が遅れたら過ぎた時間の分だけ進める）。1 フレーム遅らせる解は取らない。
- 着せ替えのオン・オフで外形が変わらない決まり（外形は有効な着せ替えの集まりに依らない）を保つ。
- 動かないシェルの合成の回数を増やさない（表に `always` も動く絵の子も無ければ足した道を通らない、という今の門 `AnimationTable::is_continuous()` を保つ）。
- テストは兄弟のファイルへ。1 ファイル 1,000 行以下。ログの無い失敗の経路を作らない。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**:
  - `crates/areka-emo-compose/src/{nesting,plan,plan_always,plan_extent}.rs` と兄弟のテスト
  - `crates/areka-seriko/src/{table,parts,looper}.rs` と兄弟のテスト（引き金の型 `LoopTrigger` が `table.rs` に在るので、切り出しの案の `parts.rs`・`looper.rs` に `table.rs` を足した）
  - 読み手で型に写す形を採るときだけ `crates/areka-parsers/src/shell/{model.rs, decode.rs}`
  - `doc/ukadoc-coverage/ledger/assets.toml` の `always` の行（担当の付け替えも）・`doc/COMPAT_ARCHITECTURE.md` §8
- **規模**: 4〜6 タスク（議題 1 で `always` を含まない組み合わせも入れるなら +1〜2）。
- **先に要るもの**: `seriko-trigger-intervals`。ファイルの順として `extent-element-offset`・`animated-image-import`・`draw-methods-canon`・`seriko-script-triggers` と直列（どれが先でもよいが、同じウェーブに置かない）。
- **優先度の区分**: A（開発者 10-05「口パクの `talk` が動かない」→ 優先度を高へ。切り出し元の区分を引き継ぐ）。
- **要件定義のモデル**: Fable。正典は組み合わせについて 1 文しか書いておらず、組める語・語の順・数値の位置・意味を areka が決めることになる（決めたことは §8 に記す）。
- **議題**（答えで作業が変わるものだけ）:
  1. `always` を含まない組み合わせ（`bind+runonce`・`bind+talk,数値`・`bind+periodic,数値` など）を、この spec で読むか。`+` を分ける場所は 1 つなので同じ仕組みで読める見込みだが、語ごとに「着せ替えが無効の間はどうするか」を決める必要がある。
  2. 数値つきの語を含む組み合わせの綴り（数値をどの位置に書くか。`bind+random,4` は今の読み手が読める唯一の形）。正典は黙っている。
  3. 3 語以上の組み合わせと、意味が食い違う組み合わせ（例: `always+never`）をどこまで読み、読まないものをどの水準で記録するか。
