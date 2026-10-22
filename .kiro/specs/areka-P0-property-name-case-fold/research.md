# ギャップ分析: areka-P0-property-name-case-fold

> 分析日 2026-10-22・本ブランチ（main `6e0ea8d8` の上・要件の初期化 `d5935e85`）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。行数は参考（`key.rs` 411・`reader.rs` 584・`actor.rs` 799・`actor_tests.rs` 364・`shiori_host.rs` 628・`get_property_tests.rs` 135）。
> 本書は情報と選択肢を出すもので、決めるのは設計の段（と要件ディスカッション）。

## 1. 要約

- **名前を鍵にする所はもともと 1 本にまとまっている。** 鏡像（`MirrorImage`）の点付きの区画へ書く道 3 本（静的な値の `PublishStatic`・自由な名前の `Set`〔`StoreWrite`〕・保存の投影 `PersistPut`／起動時の `build_initial_image`）と、読む道 1 本（`SylphyaReader::resolve_dotted`）は、どれも `parse_dotted` → `PropPath::to_canonical_string` の同じ綴りで鍵を作る（保存の投影だけは `PersistKey::to_canonical_key` の小文字の定数）。ここで半角の英字を小文字へ畳めば、読み（要件 1）・自由な名前の書きと読み（要件 3.3・3.4）・保存された値（要件 4）が同時にそろう。**保存の側（`persist/`）は 1 行も触らずに済む**（保存の名前はもともと小文字だけ）。
- **足りないのは 2 か所だけ。** ⑴ 鍵を作る所の畳み（`key.rs`）、⑵ 書き込みの仕分け `classify_set` の 1 段目（`SET_EFFECTIVE` を生の文字列と完全一致で比べる）の畳み（`actor.rs`）。2 段目の正準語彙の判定 `is_canonical_vocab` は `parse_dotted` の結果の名前を見るので、⑴ を `parse_dotted` の中で行えば自動で追従する。読む口（MCP の `get_property`・SHIORI の `GetProperty`）のコードは変えない。
- **選択肢は「どこで畳むか」の 3 案。** 案 A＝`parse_dotted` の中（各部分の名前と括弧の中の名前を畳んで `PropPath` を作る）、案 B＝`to_canonical_string` だけで畳む（`PropPath` は元の綴りを保つ）、案 C＝入口ごとに文字列を畳む。要件 1.6（`INDEX(abc)` を `index(abc)` と同じに「無い名前」）の一致は、`index(…)` の見分けまで畳むかで決まり、案 A は自然に満たし、案 B は見分けを別に畳む必要がある（§5 の端 1）。
- **要件の前提と食い違う事実が 1 つある。** SHIORI の `GetProperty` の実装は 3 つ目がある（`crates/areka-ghost/src/shiori_inproc.rs` の `InProcHost`・自前の `HashMap` で完全一致）。また本番のゴーストの結線は 32bit の仲介（`ShioriWiring::Helper`）で、sylphya を読む `ShioriHostSink` はデモ（環境変数で開く）とテストの入口だけが使う。＝**本番で動くゴーストの SHIORI から sylphya の読み手へ届く `GetProperty` は今は無い**。要件 2.2・2.4 は決定論テストで満たせるが、実機確認は MCP の口だけになる（§2.3）。
- **規模 S・リスク 低。** 外部の依存は足さない（標準の `str::to_ascii_lowercase`／`eq_ignore_ascii_case` で足り、兄弟の決まり `mcp-ghost-name-match` も `eq_ignore_ascii_case` を使う）。調べ物は 1 点だけ（全角の英字を SSP が畳むか＝未測定・測る手段が乏しい）。

## 2. 現状の調査

### 2.1 名前を鍵にする所（書く道 3 本・読む道 1 本）

| 道 | 定義 | 鍵の作り方 | 今の綴り |
|---|---|---|---|
| 静的な値（`baseware.name`・`baseware.version`） | `actor.rs` の `SylphyaCore::apply` の `PublishStatic` 腕 → `canonicalize_or_raw` | `parse_dotted` → `to_canonical_string`（読めなければ生の名前） | 供給元 `crates/areka-ghost/src/sylphya_wiring.rs` の `publish_ghost_statics` は小文字だけ |
| 自由な名前の書き込み | `apply` の `Set` 腕・`SetClass::StoreWrite` | 同上（読めなければ警告＋生の名前で保存） | 書き手しだい（大小を保つ） |
| 保存の投影 | `apply` の `PersistPut` 腕・`build_initial_image` | `PersistKey::to_canonical_key`（`format!` と文字列の定数） | 小文字だけ（`areka.window.scope(N).x` ほか 9 族） |
| 読み | `reader.rs` の `SylphyaReader::resolve_dotted_str` → `resolve_dotted` → `resolve_dotted_canonical` | `parse_dotted` → `to_canonical_string` → ゴーストごとの区画 → 全体の区画を完全一致で引く | 聞き手しだい |

- 鏡像の点付きの区画へ書くのは `run_actor`（`Effect::SetDottedGlobal`／`SetDottedPerAsker`）と `build_initial_image` だけ（ほかの書き手は 0・`grep dotted_global|dotted_per_asker`）。
- `PersistKey::to_canonical_key` の説明は「`parse_dotted(k).unwrap().to_canonical_string() == k`」を不変として持ち、`persist/persist_tests.rs` の `canonical_key_round_trips_with_to_canonical_string` が固定している。小文字へ畳む変更はこの不変を崩さない（もともと小文字）。
- `parse_dotted` の呼び手は `areka-sylphya` の中だけ（`reader.rs`・`actor.rs` 3 か所）。`PropPath`・`Selector`・`parse_dotted` は `lib.rs` から公開されているが、ほかのクレートからの利用は 0。
- `parse_seg` は `index(…)` の見分けを `name == "index"` の完全一致で行う（非数値なら `KeyParseError::BadIndex`）。

### 2.2 書き込みの仕分け（`actor.rs` の `classify_set`）

1. `SET_EFFECTIVE.iter().any(|(k, _)| *k == key)`＝**生の文字列**の完全一致 → `RuntimeCommand`。
2. `is_canonical_vocab(key)`＝`parse_dotted` の根の名前が `DOTTED_ROOTS` に・葉の名前が `GENERIC_PROP_NAMES` に（`contains` の完全一致）→ `NotSettable`。
3. それ以外 → `StoreWrite`。

- 語彙の表 3 つ（`crates/areka-sylphya/src/vocab/dotted.rs`）の英字はすべて小文字（`char*.bind.menu`・`shiori.変数名` の非英字はそのまま）。
- 穴の再確認: `CURRENTGHOST.NAME` は 2 で根 `CURRENTGHOST`・葉 `NAME` がどちらも外れ 3 へ、`MENU` は 1 を外れ、2 も外れて 3 へ落ちる（静的に確認・テストは 0）。
- `Effect::RuntimeCommandReserved`／`Effect::NotSettable` は**生の `key`** を運び、`RuntimeCommandSink::dispatch` も生の `key` を受ける（sink は未配線＝`runtime_sink: None`）。記録（`tracing::warn!`）も生の `key`。
- 書き込みの入口: `ShioriHostSink::SetProperty`・`ShioriHostSink::set_property_value`（`crates/areka/src/shiori_host.rs`）と、テストの `SylphyaPublisher::set`。カウンタの族（`crates/areka-ghost/src/prop_sink.rs` の `PropSetCueSink`）は `PersistPut` の型の決まった名前で書き、仕分けを通らない。

### 2.3 読む口（要件の「2 つ」と、見つかった 3 つ目）

| 口 | 定義 | 読み手 | 本番での到達 |
|---|---|---|---|
| MCP `get_property` | `crates/areka/src/mcp/get_property.rs` の `handle` | `runtime.sylphya_reader().resolve_dotted_str` | ✅ 配布形で到達（`mcp-get-property` の実機確認済み） |
| SHIORI `GetProperty`（sylphya） | `crates/areka/src/shiori_host.rs` の `ShioriHostSink_Impl::GetProperty` | `self.reader.resolve_dotted_str` | ⚠ 使うのは `shiori_demo.rs`（環境変数で開くデモ）と `shiori_session.rs` の `activate_with_timeout`（`#[allow(dead_code)]`・テスト専用）だけ |
| SHIORI `GetProperty`（**3 つ目**） | `crates/areka-ghost/src/shiori_inproc.rs` の `InProcHost_Impl::GetProperty` | **sylphya を読まない**。自前の `RefCell<HashMap<String, HSTRING>>` を完全一致で引く | ⚠ `ShioriWiring::InProc`（x64 の SHIORI4 を同じプロセスに読む結線）でだけ使う。本番の結線は `ShioriWiring::Helper`（`crates/areka/src/main_ghost_wiring_tests.rs` が `Helper` を固定）。説明文に「α 後の native 消費時に host 注入シームごと再設計」とある |

- 本番の `Helper` 結線（SHIORI3 の DLL を 32bit の仲介で動かす）には `GetProperty` の呼び戻しが無い。＝今の配布形で、動くゴーストの SHIORI が sylphya の読み手へ問い合わせる道は無い。要件 2.2・2.4・5.4 は `ShioriHostSink` の決定論テスト（`spawn_test_sylphya_sink`）で満たせる。実機確認（要件 6.1）は MCP の口だけになる。
- `InProcHost` は要件の「今ある読む口は 2 つ」に数えられていない。sylphya の外の別の店なので、本 spec の畳みは届かない。範囲に入れるか（入れるなら `areka-ghost` の `shiori_inproc.rs` へ触る・同じ畳みを当てるか sylphya へ委ねる形に変える）、範囲の外として記録するかは要件ディスカッションの議題（§8 の 5）。
- 台本の `%property[]` は `vocab/flat.rs` の構文記録と `ledger_key_determinism_tests.rs` に名前があるだけで読む口は無い（要件の実物 1 と一致）。`\![get,property]` も無い。
- 起動時の読み `crates/areka-ghost/src/runtime.rs`（`areka.boot.count`・`areka.vanish.count` を `resolve_dotted_str` で）は小文字で聞くので影響なし。

### 2.4 保存

- `persist/{mod,format,io}.rs` は TOML の型の決まった項目として読み書きし、自由な名前は保存ファイルに入らない（要件の実物 4 と一致）。
- 読む側で畳めば `AREKA.BOOT.COUNT` も引ける。保存の書き方・読み戻しは変えずに要件 4.1〜4.3 を満たせる（ウェーブ C10 の約束「保存した名前は畳まない＝`persist/` に触らない」と一致）。要件 4.2 のテストは `FakePersistIo` で `spawn_sylphya` を立てて読む既存の形（`actor_actor_integration_tests.rs`）で書け、`persist/` のファイルに触れない。

### 2.5 テストの置き場と今の檻

- 大小を固定するテストは 0（`crates/areka-sylphya`・`crates/areka`・`crates/areka-ghost` のテストで大文字を混ぜた点付きの名前を渡す所は 0・`grep` で確認）。＝畳んでも既存のテストは赤にならない見込み（要件 5.5）。
- 前後の空白を削らないことを固定する唯一の檻: `get_property_tests.rs` の `reads_values_through_the_ghost_own_asker_on_a_real_runtime`（名前 7 つを 1 本の `assert_eq!` で突き合わせる）。`BASEWARE.NAME` はこの並びへ 1 つ足すのが最短（要件 5.3）。
- 書式として読めない名前の檻: `key.rs` の `err_*` 群・`reader.rs` の `dotted_str_malformed_is_not_found_no_panic`・`ledger_key_determinism_tests.rs`。
- 書き込みの仕分けの檻: `actor_tests.rs` の `classify_set_*`・`apply_set_*`、`vocab/dotted_set_ledger_tests.rs`、`crates/areka/src/placement/zorder_property_deferral_tests.rs`（いずれも小文字）。
- **置き場の決まり**: `structure.md`「新規のテストモジュールは本番ファイルの中に本体を書かない（同じディレクトリの兄弟ファイル）」。`key.rs`・`reader.rs` は本体の中に `mod tests` を持つ古い形なので、新しいテストは兄弟ファイル（例 `key_case_fold_tests.rs` のような名前）へ置く。1 ファイル 1,000 行の目安に近いファイルは無い（`actor.rs` 799 が最大）。
- 決定論テストの手本: SHIORI の `GetProperty` は `shiori_host.rs` の `set_then_barrier_then_get_property_is_visible_and_missing_key_errors`（`set → barrier → get`）。

### 2.6 記録の文書

- `doc/COMPAT_ARCHITECTURE.md` §8（122 行目から・表の末尾へ追記する運用）。隣に置ける既存の行: 「SET 無効な正準語彙への書込挙動」（145 行目付近）。対照になる既存の行: 設定の語の大小を「小文字の完全一致のみ」とする行（`windowposition.x` のキーワード・重なり指定の語・6 値の語）と、「区別しない」とする行（ファイル名から面の番号を読むとき）。プロパティの名前は「区別しない」側になる＝語ごとに決まりが違う理由（SSP の実測に合わせた）を 1 句書くと後から迷わない。
- `doc/ssp-mcp/survey.md` §7.3 の 4（119 行目）に着地の注記を足す。同じ節の 6（121 行目）には 10-06 の追記がもう入っている（brief の「偽のまま」は古い）。

## 3. 要件と資産の対応

| 要件 | 使える資産 | ギャップ |
|---|---|---|
| 1.1・1.2 読みの大小違い（各部分・括弧の中） | `parse_dotted`／`to_canonical_string`／`resolve_dotted_canonical` | **Missing**: 鍵を作る所の畳み |
| 1.3 全角・かなは別 | `to_ascii_lowercase`（ASCII だけ畳む） | **Constraint**: `str::to_lowercase`（Unicode）を使うと全角の英字まで畳む＝使ってはいけない。**Research**: SSP が全角を畳むかは未測定 |
| 1.4 前後の空白 | 今は削らない | なし（`trim` を足さないことを守る） |
| 1.5 値は畳まない | 値は鏡像の値のまま返る | なし |
| 1.6 読めない大文字の名前 | `parse_seg` の `index` の見分け | **Missing（案による）**: `index(…)` の見分けを大小を区別せずに（§5 の端 1） |
| 1.7 ゴーストごとが先 | `resolve_dotted_canonical` の順 | なし（鍵がそろえば順はそのまま） |
| 2.1 MCP | `get_property::handle` | なし（コードは変えない・テストの追加だけ） |
| 2.2・2.4 SHIORI | `ShioriHostSink::GetProperty` | なし（テストの追加だけ）。**Constraint**: 本番の結線から届かない・`InProcHost` は別の店（§2.3） |
| 2.3 無い名前 | 今の答え | なし |
| 3.1 書けない名前の大小違い | `is_canonical_vocab` | **Missing**: 根・葉の比べの畳み（案 A なら自動） |
| 3.2 書ける名前の大小違い | `classify_set` 1 段目 | **Missing**: `SET_EFFECTIVE` の比べの畳み |
| 3.3・3.4 自由な名前 | `StoreWrite` の正準化 | **Missing**: 鍵の畳み（読みと同じ所を通る） |
| 3.5 隠さない | 書けない名前は書かない | 3.1 が満たされれば自動 |
| 4.1〜4.3 保存 | `PersistKey::to_canonical_key`（小文字） | なし（`persist/` は不変） |
| 5.1〜5.4 決定論テスト | 兄弟テストの手本 | **Missing**: 新しいテスト（§2.5 の置き場の決まり） |
| 5.5 既存の期待 | 大文字を使う既存テスト 0 | なし |
| 6.1 実機確認 | `mcp-get-property` の `verification/signoff.md` の curl 手順・`real-run-kit`（`doc/testing.md`） | **Unknown**: 手作業で記録するか、`real-run-kit` の試験で書くか |
| 6.2・6.3 記録 | §8・survey §7.3 | **Missing**: 行の追記 |

## 4. 実装の選択肢

### 案 A: `parse_dotted` の中で畳む（既存の部品を広げる）

- `parse_seg` が各部分の名前と `ByName` の中身を `to_ascii_lowercase` して `PathSeg` を作る。`index` の見分けは畳んだ名前で行う。`classify_set` の 1 段目は `eq_ignore_ascii_case`（または畳んだ文字列）で比べる。
- 触る所: `key.rs`（数行）・`actor.rs`（1 段目の比べ 1 行）。`reader.rs` は変更 0。
- ✅ 畳む所が 1 か所で、読み・自由な名前の書き・正準語彙の判定・`index` の見分けが全部同じ決まりにそろう（要件 1.6 も自然に満たす）。
- ✅ `to_canonical_string` の冪等の檻（`canonical_string_is_idempotent_for_canonical_inputs`）は入力が小文字なのでそのまま緑。
- ❌ `PropPath` から元の綴りが消える。後の spec が `shiori.変数名` の変数名の部分を SHIORI へ元の綴りで渡したいとき（細部 7）、`PropPath` からは取り戻せない（生の文字列を別に持つ必要が出る）。
- ❌ `parse_dotted` は公開の関数なので、意味（「区切る」から「区切って畳む」へ）が変わる。今の外部の呼び手は 0。

### 案 B: `to_canonical_string` だけで畳む（`PropPath` は元の綴りを保つ）

- 鍵の文字列を作る所でだけ畳む。正準語彙の判定（`is_canonical_vocab`）と `index` の見分け（`parse_seg`）は別に `eq_ignore_ascii_case` へ直す。
- 触る所: `key.rs`（`to_canonical_string` と `parse_seg` の `index` の比べ）・`actor.rs`（1 段目と 2 段目の比べ）。
- ✅ `PropPath` が元の綴りを保つので、後の spec が値を出すときに元の綴りを使える余地が残る。
- ✅ 手で組んだ `PropPath`（公開の構造体）を `resolve_dotted` へ渡しても畳まれる。
- ❌ 比べる所が 3 か所に分かれ、1 か所でも畳み忘れると決まりが割れる（`index` の見分けを畳み忘れると §5 の端 1 が起きる）。
- ❌ `to_canonical_string` の説明「`parse_dotted` と往復整合（冪等）」が「小文字の入力に限って冪等」へ変わる＝説明文の改め。

### 案 C: 入口で文字列ごと畳む

- `resolve_dotted_str`・`classify_set`・`StoreWrite` の正準化・`canonicalize_or_raw` の各入口で `key.to_ascii_lowercase()` してから今の処理へ渡す。
- ✅ 既存の関数の意味を変えない。読めない名前を生のまま保存するときも畳まれる（§5 の端 2 がそろう）。
- ❌ 入口が 4 か所以上に散り、`resolve_dotted(&PropPath)` を直接呼ぶ道（`runtime.rs` はしていないが公開されている）は畳まれない。後の spec（`property-query-channels` の台本の口）が新しい入口を足すたびに畳みを足す必要がある＝brief の「畳む場所を読み手の側の 1 か所に」に反する。

### 比べ

| | 畳む所の数 | 要件 1.6 | 元の綴りを残す（細部 7） | 触るファイル |
|---|---|---|---|---|
| A | 1（＋仕分け 1 段目） | 自然に満たす | 残らない | `key.rs`・`actor.rs` |
| B | 3 | 見分けを別に畳めば満たす | 残る | `key.rs`・`actor.rs` |
| C | 4 以上 | 満たす | 残る（入口の手前の文字列） | `reader.rs`・`actor.rs` |

どの案でも `reader.rs` の読む順・`persist/`・読む口 2 つのコードは変えない。

## 5. 境界の端（設計で決めて檻に入れる所）

1. **`index(abc)` を自由な名前として書いた値**: 今は `index(abc)` は読めない名前なので、`StoreWrite` が警告して**生の名前 `index(abc)` で保存する**（`apply` の `StoreWrite` 腕）。案 B で `index` の見分けを畳み忘れると、`INDEX(abc)` は `ByName` として読めて鍵 `index(abc)` になり、生で保存された値に当たってしまう（`index(abc)` で聞くと「無い名前」・`INDEX(abc)` で聞くと値がある＝要件 1.6 に反する）。案 A・C では起きない。
2. **読めない名前の生の保存**: 案 A・B では生の名前は畳まれずに保存されるが、読む側は読めない名前を常に「無い名前」にするので見える違いは無い。案 C では畳まれて保存される。どちらでも要件の答えは変わらない（記録の綴りだけ違う）。
3. **記録と効果の綴り**: `NotSettable`・`RuntimeCommandReserved` の効果と警告の記録に、書かれた綴りを残すか正準の綴りを残すか。後で `RuntimeCommandSink` に配線する spec（seriko など）が名前で振り分けるなら、正準の綴りを渡すほうが受け手が畳まずに済む。今は sink 未配線なので答えは見えない。
4. **葉の名前だけで正準語彙とみなす判定**（`vocab/dotted.rs` の `SOUND_PROP_NAMES` の説明にある areka 固有の作り）: 畳むと `myplugin.NAME` も「書けない名前」になる（今は `myplugin.name` が書けない名前で、`myplugin.NAME` は自由な名前）。要件 3.1 どおりの結果だが、この作りの是正は `property-query-channels` の持ち物のまま。
5. **`SET_EFFECTIVE` の末尾形**: 1 段目は全体の文字列を末尾形の表と比べるので、`currentghost.surface.num` のような実際の書き込みの名前は今も 1 段目に当たらない（既知・`property-query-channels` の持ち物）。本 spec は比べを畳むだけで、この形は変えない。

## 6. 並走と触るファイル

- 本 spec が触る見込み: `crates/areka-sylphya/src/key.rs`・`actor.rs`（＋案 C なら `reader.rs`）と新しい兄弟のテスト、`crates/areka/src/mcp/get_property_tests.rs`（名前を 1 つ足す）、`crates/areka/src/shiori_host.rs` のテスト（または新しい兄弟のテストファイル）、`doc/COMPAT_ARCHITECTURE.md` §8（追記）、`doc/ssp-mcp/survey.md` §7.3。
- ウェーブ C10（`roadmap.md` 179 行目）の約束「④＝`crates/areka-sylphya/src/{key,actor,reader}.rs` と兄弟のテスト・survey §7.3・§8（追記）／保存した名前は畳まない（`persist/` に触らない）」と一致する。③ `ghost-inner-balloon` は「sylphya の `persist/` に触らない」と約束済み。`readonly-mode`（`persist/{io,mod}.rs`）は C10 に入っておらず次のウェーブの候補＝今は重ならない。
- C10 のほかの 7 本の触るファイルに `crates/areka/src/mcp/get_property*.rs`・`shiori_host.rs` は無い（⑤ `offline-surface-dump` は `mcp/dump_surface_image.rs`）。
- 重なりうるのは §8 の表の**末尾への追記**だけ（② `window-link-directives`・③ `ghost-inner-balloon` も §8 に追記する）。行の追加どうしの衝突なので、後に着地する側が並べ直せば済む。
- 3 つ目の `GetProperty`（`InProcHost`）を範囲に入れると `crates/areka-ghost/src/shiori_inproc.rs` に触る。C10 でこのファイルを挙げる spec は無い。
- 後の版の `sylphya-sqlite-kvs`（v0.2.0）は `actor.rs`・`reader.rs`・`persist/` を作り直す。本 spec が先に着地し、畳む所（案で決まる 1〜3 か所）を向こうの要件が引き継ぐ形（要件の Adjacent expectations と同じ）。

## 7. 規模とリスク

- **規模: S（1〜3 日）**。変更は数行〜十数行の畳みと、決定論テスト（読み 9 場合・書き 5 場合・2 つの口）と、配布形での実機確認 1 回と、文書 2 か所。新しい部品・依存・設定は無い。brief の見立て S〜M（5〜9 タスク）と合う。
- **リスク: 低**。既存の形（正準の綴り 1 本）を広げるだけで、既存のテストに大文字の名前は 0。気を付ける点は ⑴ `to_lowercase` と `to_ascii_lowercase` の取り違え（全角まで畳む）、⑵ 畳む所の取りこぼし（案 B・C）、⑶ `trim` を足さないこと（`get_property_tests.rs` が赤になる）。

## 8. 設計の段（と要件ディスカッション）への申し送り

### 決める点

1. **畳む所**: 案 A（`parse_dotted` の中）・案 B（`to_canonical_string` だけ）・案 C（入口ごと）のどれにするか。答えで触るファイルと檻の数が変わる。
2. **`PropPath` に元の綴りを残すか**（細部 7 の `shiori.変数名` の余地）。残すなら案 B か C。残さないなら案 A で、後の spec は生の文字列を別に持つ。
3. **`index(…)` の見分けを大小を区別せずに行うか**。要件 1.6 の例（`INDEX(abc)`）の一致はこれで決まる（案 A は自然に、案 B は別に直す）。
4. **書き込みの仕分けの記録・効果・`RuntimeCommandSink` へ渡す名前の綴り**（書かれた綴りか正準の綴りか）。答えで `apply` の効果を突き合わせるテストの期待が変わる。
5. **3 つ目の `GetProperty`（`InProcHost`・sylphya の外の自前の店）を範囲に入れるか**。要件の実物 1「今ある読む口は 2 つ」と食い違う事実。範囲の外なら Boundary Context の Out of scope に 1 行と、§8 の行に「`InProcHost` は別の店で対象外」を書く。入れるなら `areka-ghost` の `shiori_inproc.rs` に触る（本番では未使用の結線）。
6. **SHIORI の `GetProperty` の確かめは決定論テストだけでよいか**。本番の結線（`Helper`）には sylphya へ届く `GetProperty` が無く、`ShioriHostSink` はデモとテストの入口だけが使う。要件 6.1 は MCP だけなので変わらないが、要件 2.2 の「ゴーストの SHIORI が」を満たす証拠の種類（決定論テストのみ）を設計に明記する。
7. **要件 6.1 の実機確認の書き方**: `mcp-get-property` の `verification/signoff.md` の curl の手順を踏む手作業の記録か、`real-run-kit`（`doc/testing.md`・`#[ignore]` の実機テスト）で書くか。

### 調べる点

- **Research Needed（任意）**: SSP が全角の英字（`Ａ`／`ａ`）の大小を同じとみなすか。survey §7.1 の実測は半角の英字だけ。確かめるには全角の英字を名前に含むゴースト（括弧の中の名前）が要り、手近な手段が無い。細部 1 は「半角だけ」で決めており、外れても「SSP で当たる名前が areka で外れる」側に倒れる（逆の危険は無い）。
- そのほかに外部の依存・版の調べ物は無い。
