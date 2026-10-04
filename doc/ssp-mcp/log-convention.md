# ログの履歴の取り決め（`get_log` の 5 つの種別）

areka は、ログ（`tracing` のマクロで出す行）のうち意味のある出来事をアプリの中に残し、MCP のツール `get_log` で読めるようにしている。本書は**ログを出す側**のための取り決めで、どう出せばどの種別に残るかを書く。履歴のコード（`crates/areka/src/log_history.rs`）を読まずに正しい行を書けることを目指す。

- **出来事**: `tracing::info!` や `tracing::warn!` などで 1 回出した行。出来事は「レベル」（error・warn・info・debug・trace）と「target」（行の出どころの名前。書かなければモジュールのパス、`target: "…"` で明示もできる）を持つ。
- **記録**: 履歴に残った出来事 1 件。通し番号・時刻・種別・名・表示の語・本文を持つ。
- **取り決めの target**: 本書で約束した 2 つの target `areka::log::script` と `areka::log::error`。この target の行だけは、欄 `ghost`・`label` を記録の名と表示の語として読む。

`get_log` の結果の 1 行は `#<番号> <yyyy/mm/dd hh:mm> [<表示の語>] <名> : <本文>` の形になる（SSP と同じ書式）。

## 1. 5 つの種別と振り分けの規則

種別は SSP の `get_log` の `log_type` と同じ 5 つ。

| 種別 | 残るもの |
|---|---|
| `error` | warn 以上の出来事（どのクレート・どのモジュールが出したものでも）と、`areka::log::error` へ info 以上で出した出来事 |
| `script` | `areka::log::script` へ info 以上で出した出来事（再生した台本） |
| `network` | 取得の通信の info の出来事 |
| `update` | 更新・インストールの info の出来事 |
| `status` | 起動・終了・ゴーストの切り替えの節目の info の出来事 |

1 つの出来事は 1 つの種別にだけ入る。次の順に当て、最初に当たったものに決まる。

1. レベルが debug・trace の出来事は残さない（取り決めの target でも残さない）。
2. target が `areka::log::script` なら `script`（warn や error で出しても `error` には入らない）。
3. target が `areka::log::error`、またはレベルが warn 以上なら `error`。
4. レベルが info なら、下の規則の表の target を上から照合し、最初に当たった行の種別。
5. どれにも当たらなければ残さない（通し番号も進まない）。

target の照合は 2 通りある。

- **下も含む**: その target 自身か、`::` で区切った下のモジュールに当たる。`areka::update` は `areka::update::desk` に当たり、`areka::updater` には当たらない。
- **完全一致**: 字が完全に同じ target だけに当たる。`areka` は下のモジュールまで含めるとアプリ本体の info が全部 `status` になるので、完全一致にしている。

## 2. 規則の表

取り決めの target の 2 行と、info の行を振り分ける 13 行。判定の順に並べた（`network` の行は `update` の行より先に当てる）。この表はテスト（`crates/areka/src/log_history_convention_tests.rs`）が読み、実装の規則と 1 行でも食い違うと赤になる。表を変えるときは実装（`log_history.rs` の `RULES`・`TARGET_SCRIPT`・`TARGET_ERROR`）も同時に変える。

<!-- log-rules:begin -->
| 種別 | target | 照合 |
|---|---|---|
| script | `areka::log::script` | 完全一致 |
| error | `areka::log::error` | 完全一致 |
| network | `areka::install::fetch_url` | 下も含む |
| network | `areka_update::winhttp` | 下も含む |
| network | `areka_update::fetch` | 下も含む |
| update | `areka_update` | 下も含む |
| update | `areka::update` | 下も含む |
| update | `areka::install` | 下も含む |
| status | `areka` | 完全一致 |
| status | `areka::boot_config` | 下も含む |
| status | `areka::boot_resolve` | 下も含む |
| status | `areka::ghost_session` | 下も含む |
| status | `areka::emo2_boot::ghost_switch` | 下も含む |
| status | `ghost-boot` | 下も含む |
| status | `ghost-shutdown` | 下も含む |
<!-- log-rules:end -->

- `ghost-boot`・`ghost-shutdown` は `crates/areka-ghost/src/runtime.rs` が `target: "…"` で明示している target で、ゴースト自体の起動と終了の節目。
- `areka_update::winhttp`・`areka_update::fetch` は、今は 1 行もログを出していない。取得の行を足したときの受け皿である。

## 3. 取り決めの欄と種別ごとの既定

取り決めの target（`areka::log::script`・`areka::log::error`）の行だけ、次の 2 つの欄を読む。

| 欄 | 意味 | 入れる値 |
|---|---|---|
| `ghost` | 記録の `<名>`。`get_log` の `ghost_name` での絞り込みはこの名と完全に一致する記録を返す | その行が属するゴーストの名前。`get_active_ghost_list` が返す値と同じもの |
| `label` | 記録の `[<表示の語>]`。表示が変わるだけで、種別は変わらない | SSP が送り手ごとに出す語（`Ghost:OnBoot`・`SSTP(Local,Auth)` など） |

- 欄が無いときは、種別ごとの既定を使う。
- `ghost`・`label` は `%` を付けて渡すか、文字列のまま渡す（どちらも引用符の付かない値になる）。`?` を付けると引用符つきの値が名になり、`ghost_name` で当たらなくなる。
- 取り決めの target でない行の `ghost`・`label` は取り決めの欄として読まず、ほかの欄と同じく本文に残る（例: 本文の末尾に ` ghost="emo2"`）。

| 種別 | 既定の表示の語 | 既定の名 |
|---|---|---|
| `error` | `Error` | `[SYSTEM]` |
| `script` | `SSTP` | `[SYSTEM]` |
| `network` | `Info` | `[SYSTEM]` |
| `update` | `Info` | `[SYSTEM]` |
| `status` | `STAT` | `STAT` |

## 4. 本文の作り方

- 本文は、出来事のメッセージの後に、残りの欄を出来事に書いた順で ` 名前=値` の形で続けたもの。取り決めの target の行を除き、標準出力の行の `<target>: ` より後ろと同じ形になる（取り決めの target の行は次の項目のとおり `ghost`・`label` を除く）。
  - 値は `{:?}` の形で書く。文字列で渡した欄は引用符つきで、逆斜線は 2 つになる。`%` を付けた欄は整形した文のまま。
  - 取り決めの欄（取り決めの target の行の `ghost`・`label`）は本文から除く。
  - 名前が `log.` で始まる欄（`log` クレートから橋渡しされた行に付く `log.target`・`log.file` など）は除く。
  - 残りの欄が無ければメッセージだけ、メッセージが無ければ欄だけ。
- **台本はメッセージで出す**。メッセージなら逆斜線は 1 つのまま残る（`\0\s[0]こんにちは\n\![raise,OnTest]`）。欄として渡すと引用符と 2 つの逆斜線が付く。
- 本文が改行を含んでもよい。`get_log` の結果では、2 行目以降がタブで始まる。

## 5. 上限・`RUST_LOG`・通し番号

- **件数の上限**: 種別ごとに 1,000 件。ある種別が 1,000 件あるときに次の記録が来ると、その種別のいちばん古い 1 件を捨てる。他の種別は減らない。
- **本文の上限**: 4,096 文字（バイト数でなく文字数）。超えた分は捨て、末尾に ` ...(truncated)` を付ける。
- **`RUST_LOG` と独立**: 履歴に残るかどうかは、上の規則だけで決まる。`RUST_LOG=warn` で標準出力から消えた info の行も、規則の表に当たれば履歴に残る。`RUST_LOG` で特定のクレートだけに絞っても、他のクレートの warn 以上は `error` に残る。
- **通し番号**: アプリの起動ごとに 1 から始まり、種別を問わず記録 1 件ごとに 1 ずつ増える（全種別で 1 本）。捨てた記録の番号は使い回さない。履歴はアプリの中にだけあり、再起動で空に戻る。
- **時刻**: 記録した時点の現地時刻を分まで。

## 6. 書き方の例

再生した台本を `script` に残す行:

```rust
tracing::info!(
    target: "areka::log::script",
    ghost = %ghost_name,
    label = "SSTP(Local,Auth)",
    "{script}"
);
```

`get_log` の結果では `#12 2026/10/04 21:05 [SSTP(Local,Auth)] えも2 : \0\s[0]こんにちは\e` のようになる。

`error` へ直接残す行（warn でも info でもよい。`label` を省いたので表示の語は `Error`）:

```rust
tracing::warn!(
    target: "areka::log::error",
    ghost = %ghost_name,
    "[GHOST/Script] 台本の解釈に失敗した: {reason}"
);
```

- 出す側は、本書の target と欄の名前を書くだけでよい。履歴のコードの型や関数は使わない（クレートをまたいでも書ける）。
- **取り決めの target の行は info 以上で出す**。debug・trace で出した行は残らない。台本の行は info なので、`RUST_LOG` が info を通すとき（既定）は標準出力にも 1 行出る。

## 7. 履歴に届かないもの

- debug・trace の出来事（取り決めの target でも）。
- 規則の表のどの target にも当たらない info の出来事（例: `areka::alert` の問いかけの行、`areka::emo2_boot` の info）。
- `tracing` も `log` クレートも通らない出力（`println!`・`eprintln!`・パニックの報告）。
- `log` クレートで出した info 以下の行。`log` クレートの warn 以上は `error` に残る（target は `log` として届くが、warn 以上はどの target でも `error` なので変わらない）。
- スパン（`tracing::span!`）。履歴に残るのは出来事だけ。

## 8. 「最後に振った番号」の口

areka の中の処理は `crate::log_history::last_id()` で、いま最後に振った通し番号（1 件も無ければ 0）を読める。

1. 確かめたい操作（台本を流す・イベントを送るなど）の**前に** `last_id()` を読む。
2. 操作をする。
3. 読んだ番号を `get_log` の `since_id` に渡す。番号がそれより大きい記録、つまり操作より後の記録だけが返る。

出した行は、出したスレッドの上でその場で履歴に積まれる。出し終えた後に届いた `get_log` は、その行を取りこぼさない。
