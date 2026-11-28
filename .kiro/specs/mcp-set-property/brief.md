# Brief: mcp-set-property

> 出どころ（2026-10-21 開発者・`/kiro-discovery`・`sylphya-sqlite-kvs` と同じ起票）: sylphya の保存を sqlite へ移すと、`sylphya.toml` を手で書き換える使い方ができなくなる。同日の答え「MCP の set で書く」＝手書きをやめ、areka の MCP に名前で書く道具を足す（AI からも開発者からも書ける）。`shell-balloon-scale-setting` の「設定画面ができるまでは `sylphya.toml` を手で書く」もこの道具で書く形へ改める。分け方＝**2 本**（`sylphya-sqlite-kvs` → 本 spec）・版＝**v0.2.0**。

## Problem

開発者と AI は、areka が覚えている値や property の値を、外から名前で書き換えられない。今は `sylphya.toml` を手で開いて書く道だけがある。`sylphya-sqlite-kvs` がファイルを sqlite に変えると、その道も無くなる。設定画面（windows-reactor 待ち）ができるまでのあいだ、表示倍率（`shell-balloon-scale-setting`）のような設定を試す手が無くなる。

## Current State

- areka の MCP（`crates/areka-mcp/src/tools/`）は SSP の道具の定義を逐語で写して持つ（`get_property.rs` の `DEFINITION`＝`doc/ssp-mcp/tools-list-ssp-2.9.05.json`）。SSP の MCP に `set_property` は無い。
- areka 独自の道具の前例: `check_script`（`tools_own_tests.rs`・`tools_own_socket_tests.rs`）。
- `get_property` は動いているゴーストの `SylphyaReader` を読む（`crates/areka/src/mcp/get_property.rs`）。
- 書く口はゴーストの側の `SylphyaPublisher::set`（`actor.rs:463`）。今は表に無い鍵をメモリーにだけ書き、覚える鍵（`PersistKey`）へは届かない。`sylphya-sqlite-kvs` の後は、鍵の表が決める住み場所へ名前で届く。
- stdio の橋渡し（`areka-mcp-bridge`）は道具の一覧を `tools.json` にキャッシュする（`cache.rs`）。道具を足すと一覧が変わる。

## Desired Outcome

- areka の MCP に `set_property` がある。鍵の名前・層（アプリ・ゴースト・シェル・バルーン）・値・対象のゴースト（`get_property` と同じ `ghost_name`）を受け、`sylphya-sqlite-kvs` の口で書く。
- 書いてよくない鍵（鍵の表で書けないもの）は、理由付きの誤りで返す。書けなかったとき（ファイルが書けない・`readonly-mode`）も誤りで返し、記録を残す。
- 覚える鍵に書いた値は、次の起動でも効く。`get_property` で書いた値をすぐ読み返せる。
- `shell-balloon-scale-setting` の brief の「`sylphya.toml` を手で書く」を「`set_property` で書く」へ改める。

## Approach

`check_script` と同じ「areka 独自の道具」の形で `set_property` を足す。定義は `get_property` の引数の名前にそろえ（`property_name`・`ghost_name`）、`value` と `layer` を足す。中身は `sylphya-sqlite-kvs` の名前での set を呼ぶだけで、MCP の側に鍵の判断を持たない。

- **採らなかった案**: 起動オプションで値を渡す（一度きりで、動いている最中に試せない）。手書き用の TOML を残す（読む口が 2 つになる＝開発者が選ばなかった）。

## Scope

- **In**: `set_property` の定義と引数の検査・書く先への配線・書けない鍵と書けないときの誤り・橋渡しのキャッシュの一覧が変わることの扱い・`shell-balloon-scale-setting` の brief の改め・決定論の試験（道具の一覧・引数の検査・書いて読み返す）と実機（実 emo2）での確かめ。
- **Out**: 鍵の表と住み場所の判断（`sylphya-sqlite-kvs`）・SHIORI の SetProperty の振る舞い・SSP の MCP に無い他の道具・設定画面。

## Boundary Candidates

- MCP の道具の面（定義・引数・誤り）
- KVS の名前での set への配線（判断は KVS の側）

## Out of Boundary

- KVS の作り・鍵の表（`sylphya-sqlite-kvs`）
- 表示倍率の値の意味と使い道（`shell-balloon-scale-setting`）

## Upstream / Downstream

- **Upstream**: `sylphya-sqlite-kvs`（名前での set・鍵の表・層）。
- **Downstream**: `shell-balloon-scale-setting`（v0.4.0・設定画面ができるまでの書く手）・予約「設定画面と設定ファイル」。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `mcp-kanade-tools`・`mcp-receiver-bugs` が作った道具の形に乗る）
- **Adjacent**: `sylphya-sqlite-kvs`・`shell-balloon-scale-setting`（brief を改める）・`bridge-store-user-root`（橋渡しのキャッシュ）

## Constraints

- v0.2.0・`sylphya-sqlite-kvs` の着地の後。
- SSP の道具の定義は逐語で写す決まりがある。areka 独自の道具は SSP の道具と名前が衝突しないこと（SSP が後で `set_property` を出したら、その定義に合わせ直す）。
- 規模の見立て: XS〜S（4〜7 タスク）。

## 2026-11-28 棚卸㊺の再測定（main 7a590832）
- **前提の変化**: 待っていた `sylphya-sqlite-kvs` と見直し 2 本（`audit-mcp-update-crates`・`audit-areka-install-update`）、crate 分け（`runtime-module-untangle`〜`session-crate-split`）は全部済んだ。名前で書く口は `SylphyaReader::kvs(問い手)`（`crates/areka-sylphya/src/reader.rs` 60 行）で得る `Kvs` の `set(層, 鍵, 値)`（`kvs.rs` 63 行）。層は `PersistScope` の 4 つ（アプリ・ゴースト・シェル・バルーン）、結果は `WriteOutcome` の 4 つ（書けた・断った・書けなかった・控えた）。書いてよいかは鍵の表 `keytable.rs` の `by_name` が決める＝MCP の側に鍵の判断を持たない形がそのまま取れる。本番でこの口を使う所はまだ無い（本 spec が最初の使い手）。`get_property` の読み（`resolve_dotted_str`）は層を重ねて読む＝書いた値をすぐ読み返せる。
- **Current State の古い所**: 書く口 `SylphyaPublisher::set`（今 `actor.rs` 533 行）は返事も層も無い、ゴーストごとのメモリーへの書き＝本 spec が使う口ではない。`sylphya.toml` は無くなり、置き場は各層の `profile\areka\sylphya.sqlite` になった（古いファイルは一度だけ取り込まれる）。
- **触るファイル**: `crates/areka-mcp/src/tools/` の新しい道具のファイルと `mod.rs`（232・`OWN_TABLE` と `ToolCall`）と道具の一覧のテスト（`tools_own_tests.rs` ほか）・接続の指示文 `crates/areka-mcp/src/handler.rs`（独自の道具を名指す）・本体の `crates/areka/src/mcp/` の新しいファイルと振り分けの `mod.rs`（155・`get_property.rs` 55 行に倣う）・`doc/ssp-mcp/areka-tools.md`（独自の道具の正本）・`.kiro/specs/shell-balloon-scale-setting/brief.md`。橋渡しのキャッシュ `crates/areka-mcp-bridge/src/cache.rs` は一覧が変わることの扱いしだい。依存は足さない。
- **規模**: XS〜S（4〜7）のまま。
- **先に要るもの**: 無し。
- **同じウェーブに置けない相手**: 候補の 7 本の中には無い。`property-query-channels` とは `doc/ssp-mcp/areka-tools.md` だけを分け合いうる（向こうは 112 行目の受け取り手の数）＝その行から数を外しておけば重ならない。`balloon-text-visual-tree` が触る本体の `mcp/dump_balloon.rs` には触らない。sylphya（`crates/areka-sylphya/`）は呼ぶだけで直さない。
- **優先度の区分**: A（開発者「MCP の set で書く」）。
- **要件定義のモデル**: Opus（areka 独自の道具 `check_script` の前例と、sylphya の今の口に乗る）。
- **分割の案**: なし。
- **見つけた穴**: ⑴ `Kvs::set` は番人の返事を待つ（錠に当たると最大 1 秒）＝UI スレッドの上で呼ばず、`check_script` と同じく別のスレッドへ渡し、本体の `mcp/mod.rs` の `later` で答える形を要件に書く。⑵ 読み取りだけの走りでは結果が「控えた」になる＝Desired Outcome の「`readonly-mode` は誤りで返す」とどう答えるかを要件の段で決める。⑶ 層を重ねて読むので、下の層（アプリ）へ書いても上の層に値があれば読み返しはそちらになる＝答えに書いた層を添えるかを要件で決める。⑷ `\![set,property]` の受け口（`property-query-channels`）と同じ口・同じ断り方にそろえる。
