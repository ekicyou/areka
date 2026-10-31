# 調べの記録: seriko-pattern-calls

## 1. ギャップ分析（2026-10-30・main `b9c153c2b` の作業木）

### 1.1 分析の要約

- **語の見分けの口はもうある。** 7 語は `areka-emo-compose` の `world.rs` の `targets_animation_id`（311 行目）、`move` は `method.rs` の `is_move_name`（189 行目）で、綴りの比べ方を 1 本（`canonical_method_name`）に揃えて見分けている。ぶら下がりの検査・外形・入れ子の辺・描けないコマの報告はもう 8 語を飛ばす。**8 語を重ね方として読むのは 2 か所だけ**＝seriko の `table.rs` 311 行目（表を作るとき・ここが「．さくら」の 27 件の WARN の出どころ）と、emo-compose の `plan.rs` 535 行目（着せ替えの pattern0 を**合成のたびに** `ComposeMethod::from_name` で引く＝pattern0 に 8 語を書いた着せ替えでは、合成のたびに WARN が出る）。
- **欠けているのは「働き」の全部。** ⑴ 候補の並び `(0,1)` は行を割る `split_csv`（`lexer.rs` 195 行目）で 2 つの欄に割れ、`Pattern` は相手を `surface_id: i64` の 1 つしか持てない。⑵ seriko の再生は「経過時刻 → 今のコマ」を毎回計算するだけ（`timeline.rs` の `frame_at`）で、「コマを通り過ぎた」という出来事を持たない＝呼び出しを起こす場所が無い。⑶ `PatternState` は面のずれを運べない。⑷ 着せ替え（`interval,bind`）は **pattern0 しか描かない**（`plan.rs` 505 行目の決めごと）ので、`insert` が意味を持つ「着せ替えの何コマ目かに差し込む」場所がそもそも無い。
- **呼び出しの相手は表に載ったアニメーションだけ。** seriko の表（`AnimationTable`）は interval が `random`・`bind+random`・`always`・`runonce`・`periodic`・`talk`・`sometimes`・`rarely` のものしか載せない。「．さくら」の相手（animation0・1 は `rarely`/`sometimes`）は載っているので足りるが、`never`・`bind`・未知の語のアニメーションは表に無い＝呼べない。
- **候補の方針**は 3 つ（§1.5）。どれも読み手・語の見分け・seriko の再生・合成の 4 層に手が入る。違いは「呼び出しの出来事を seriko の再生のどこで起こすか」と「`move`・`insert` を合成へどう運ぶか」。
- **調べが要る点**は、要件 8 の M1〜M5 に加えて M6（`bind` の 2 コマ目以降の描き方）・M7（`base` と `move` の重なり）・M8（書き出しの面の写しに `move` を入れるか）の 3 つ（§1.7）。

### 1.2 今の作り（触る所の地図）

| 層 | ファイル（行数） | 今していること | この spec との関係 |
| ---- | ---- | ---- | ---- |
| 読み手（字句） | `crates/areka-parsers/src/shell/lexer.rs`（222） | `split_csv` が `[ ]` の中のカンマだけを守って欄に割る。見出しの行と本体の行の両方で使う | `(0,1)` が `(0`・`1)` に割れる（要件 2.2） |
| 読み手（意味） | `crates/areka-parsers/src/shell/decode.rs`（598） | `decode_animations` が `animationN.patternM` の欄 1 を `DrawMethod`（原文のまま）、欄 2 を `surface_id: i64`（読めなければ 0）、欄 3〜5 を待ち・X・Y に写す。`animation` で始まらない行（旧書式）は飛ばす | 欄 2 の読みと並びの読みを足す所。旧書式は範囲の外（要件の Out of scope） |
| 読み手（型） | `crates/areka-parsers/src/shell/model.rs`（301） | `Pattern { index, method, surface_id, wait, x, y }` | 相手の並びを持つ欄が無い。この型を構造体の書き方で作る所は約 24 か所（テストが大半・emo-atlas・emo-compose・emo-present・seriko にまたがる） |
| 描けない element定義 | `crates/areka-parsers/src/shell/undrawn.rs` | `overlay`・`base`・`balloon` 以外の element定義の行を原文で並べる。読み込みの入口が 1 行 1 件の `warn!` を出す | element定義に 7 語・`move` を書いたときの記録（要件 5.3・6.6）は、今の一般の文言の `warn!` で既に 1 度だけ出ている |
| 語の見分け | `crates/areka-emo-compose/src/method.rs`（420） | `ComposeMethod::from_name`（未知の語で `warn!`）・`known_method`・`is_implemented_name`（記録なし）・`is_move_name` | 8 語を `from_name` に通さない形へ |
| 語の見分け | `crates/areka-emo-compose/src/world.rs`（414） | `targets_animation_id`（7 語）・`dangling_pattern_targets`（7 語を相手の無いコマの検査から外す） | 7 語の正本。`crates/areka-emo-atlas/src/manifest.rs` の `bind_only_paths`（156 行目）が同じ 7 語を写して持つ（emo-atlas は emo-compose を参照できない） |
| 外形・入れ子 | `plan_extent.rs`（198）・`nesting.rs`（434）・`plan_bounds.rs`・`plan_issues.rs` | 8 語を辺・外形・描けないコマの報告から外す | 変更は要らない見込み（窓の大きさは静的な外形で決まり、`PatternState` に依らない＝要件 7.5 の「窓を変えない」は今のまま満たす） |
| 合成 | `crates/areka-emo-compose/src/plan.rs`（706） | `flatten_surface` が ⑴ 面の静的な element（土台）を offset (0,0) で積み、⑵ 有効な着せ替えの pattern0・`always`・`PatternState` の今のコマを animation の番号の順に積む。コマは `is_implemented()`（今は `Overlay` だけ）を通ったものだけ描く | `move`（土台をずらす）・`insert`（差し込む）を受ける所。535 行目の `from_name` は合成のたびに走る |
| 合成の入力 | `crates/areka-emo-compose/src/pattern.rs`（324） | `PatternState`（animation の番号 → 今のコマ 1 枚か「消えている」・部品の欄・動く絵の欄）。内部は非公開の昇順の表で、emo-present の合成の覚え（`ComposeKey`）の鍵に入る | ずれを運ぶ欄が無い。非公開の欄を足すのは外へ壊れない（`PatternFrame` を作る所は 48 か所あるので、`PatternFrame` 自体を変えるのは重い） |
| 切り抜き | `crates/areka-emo-compose/src/blit.rs`（782） | 合成の絵 `[0, w) × [0, h)` の外は画素ごとに捨てる（要件 6.2 のテストあり） | `move,0,10,0,9999` で窓の外へ出た絵が見えない（要件 7.5）は今の切り抜きで満たす見込み |
| 再生の表 | `crates/areka-seriko/src/table.rs`（901） | `LoopAnimation { id, trigger, frames: Vec<LoopFrame> }`。`LoopFrame { surface_id, picture, method: ComposeMethod, wait_ms, x, y }`。表に載せるのは引き金を持つアニメーションだけ | WARN の出どころ（311 行目）。`LoopFrame` を作る所は 8 か所（3 ファイル）と少ない |
| 再生の時刻 | `crates/areka-seriko/src/timeline.rs`（564） | `frame_at`／`current_frame_index`＝累積の待ちから今のコマを引く純粋な関数。`seeded_rng`（SplitMix64）・`should_fire` | 呼び出しのコマの待ちを 0 に数える（要件 3.1「待たずに」）所 |
| 再生の統括 | `crates/areka-seriko/src/looper.rs`（924） | `LoopRuntime::on_tick`＝毎秒の抽選 → 再生中の全アニメーションを `frame_at` で進める → `PatternState` へ差分。再生の状態は `Playback { started_at_ms }` だけ（経過から毎回計算） | 呼び出しを起こす所。`-1` の停止（`put_top_play` の `Stopped`）・面の切り替え（`on_surface_changed`）がずれを戻す時（M3）の候補 |
| 部品の再生 | `crates/areka-seriko/src/parts.rs`（893）・`parts_trigger.rs` | 入れ子の部品の animation を別の時計で進める | 部品の中の呼び出し・`move` を受けるかは決めごと（§1.6 の 6） |
| 書き出し | `crates/areka-seriko/src/settle.rs`・`crates/areka/src/offline_dump/render.rs` | 面を `runonce` の待ちの合計の最大まで進めた状態を書き出す（乱数は常に外れ） | `surface12` を書き出すと `runonce` の終わり＝`move,0,10,0,9999` の後の絵になる（M8） |
| 起動 | `crates/areka/src/emo2_boot/mod.rs` 539 行目・`spine.rs` 708 行目 | `SerikoLoopConfig { shell_table, balloon_tables, rng: seeded_rng(seed) }` | 選びの乱数を別に持つなら注入の口が要る（要件 3.6・3.7）。`SerikoLoopConfig { … }` を書く所は 31 か所（テストが大半） |
| 読み込みの記録 | `crates/areka-emo-present/src/shell_target.rs`（865）・`emo-compose/src/plan_issues.rs` | 面の表を 1 度なめた純粋な報告（`ShapeIssue`）を、入口の `load_shell_target` が 1 件 1 行の `warn!` にする | 要件 5・6 の「読み込みのときに 1 度だけ」の記録の置き場の手本 |

「．さくら」（`target/nar-samples/cache-2/dot_sakura-*/ghost/dot_sakura/shell/master/surfaces.txt`・Shift_JIS・`descript` の `version,1`）の該当の行:

- `surface0`・`1`・`2`・`3`・`7` の `animation2.interval,rarely`／`sometimes` ＋ `animation2.pattern0,alternativestart,(0,1)`（1 コマだけ・待ちの欄なし）。相手の animation0・1 は自分も `rarely`／`sometimes` で抽選される＝**再生中の相手を呼ぶことが普通に起きる**（M1 がそのまま効く）。
- `surface12`: `animation0.interval,runonce` ＋ `pattern0,base,11`（待ち・X・Y の欄なし）＋ `pattern1〜11,move,0,10,0,{6,12,24,48,96,144,192,240,288,384,9999}`。
- `surface22`: `runonce` ＋ `base,0`・`move`×2・`base,2`・`move`×9（最後は `9999`）。
- `start`・`stop`・`alternativestop`・`parallel*`・`insert` は「．さくら」に無い。

### 1.3 要件と今の作りの対応（Missing／Unknown／Constraint）

| 要件 | 今の作り | 区分 | 中身 |
| ---- | ---- | ---- | ---- |
| 1.1・1.4 重ね方として扱わない・WARN 0 | `table.rs` 311・`plan.rs` 535 が `from_name` を通す | Missing | 8 語を `from_name` の前で見分けて外す。`plan.rs` 535 は合成のたびに走るので、ここも外さないと要件 5.4（繰り返さない）にも触れる |
| 1.2 綴りの比べ方 | `canonical_method_name` 1 本 | 既存で足りる | 8 語の表を 1 か所に置き、`targets_animation_id`・`is_move_name` をそこから引く形が素直 |
| 1.3 絵を重ねない | 呼び出しのコマは `PatternState` に入り `Unknown` で描かれない（結果だけ合っている） | Missing | `FinishedResidual` の 1 コマだけのアニメーション（「．さくら」の animation2）は今 `Unknown` のコマを残す。働きのコマは欄へ置かない形にする |
| 1.5 ほかの語は今のまま | — | Constraint | `known_method` の表と `Unknown` の記録は残す |
| 2.1 欄 2 を番号で | `surface_id: i64`・読めなければ 0 | Missing（一部） | 「読めない」と「0」を見分けられない（`field_i64` が 0 に倒す）＝要件 2.5 の記録が出せない |
| 2.2・2.3 並びを 1 つに | `split_csv` は `[ ]` だけ守る。`[0,1]`・`(0.1)`・`[0.1]` は今も 1 欄、`(0,1)` だけ割れる | Missing | §1.5 の読み手の 2 案 |
| 2.4 欄が欠けた行 | 欠けた欄は 0 になり行は 1 コマとして残る | 既存で足りる | 待ちを無視する所は再生の側 |
| 2.5 読めない欄の記録 | 無い | Missing | 読み込みの報告の型に 1 種足す（§1.5） |
| 2.6 ほかの行の読みを変えない | `split_csv` は見出しの行と element定義・collision定義の行にも使う | Constraint | 字句で `( )` を守ると、名前に `(` を含むファイル（閉じない括弧）で行の残りが 1 欄に潰れる |
| 2.7 版の宣言に依らない | `decode_animations` は版を見ない | 既存で足りる | — |
| 3.1〜3.5 開始 | 無い。再生は経過から今のコマを毎回計算し、「コマに着いた」出来事を持たない | Missing | 刻みと刻みの間に通り過ぎた呼び出しのコマを拾う仕組みが要る（刻みが遅れても、呼び出しの時刻はコマの予定の時刻＝「時刻は正確に」の原則） |
| 3.6・3.7 選びの偶然 | 乱数は 1 本（`config.rng`）を抽選が使う | Missing・Constraint | 同じ 1 本から引くと抽選の結果がずれる。別の乱数を注入する口が要る |
| 4.1〜4.5 停止 | `-1` の停止は `put_top_play` の `Stopped` の枝（コマ除去＋再生除去） | 一部あり | 同じ枝を相手のアニメーションへ当てる関数に切り出せる |
| 5.1 相手が無い | `dangling_pattern_targets` は 7 語を外すだけ | Missing | 「今の面の animation に相手の番号が無い」を報告する純粋な照会が要る。`never` など表に載らない相手をどう数えるかは決めごと |
| 5.2 巡回 | 無い | Missing | 1 つの時刻の中で始めたアニメーションの集合を持って 2 度目を断る |
| 5.3 着せ替え・element定義の使えない語 | element定義は `undrawn.rs` の一般の `warn!` が既に出る。着せ替えの側は無い | 一部あり | 着せ替えの側は読み込みの報告に足す。element定義の文言を語ごとに変えるかは決めごと |
| 5.4 1 度だけ | 読み込みの報告（`shape_issues`）は 1 度だけの手本がある | 既存の型を使う | `plan.rs` 535 の合成ごとの `from_name` は外す |
| 6.1〜6.7 insert | 着せ替えは pattern0 だけ描く（`plan.rs` 505 行目）。`insert` の行は `nesting.rs` の辺からも外れている | Missing（大） | 着せ替えの 2 コマ目以降を描かないので、「A のコマの位置に B を差し込む」の「位置」が pattern0 にしか無い。§1.6 の 4 |
| 7.1〜7.4 move | 無い。`PatternState` にずれの欄が無い。土台は `push_static_element_ops` が offset (0,0) で積む | Missing | ずれを運ぶ欄と、土台の層にだけ足す所 |
| 7.5 窓を変えない | 窓の大きさは静的な外形（`compute_extent`）・切り抜きあり | 既存で足りる見込み | 実機で確かめる |
| 7.6 戻す時と範囲 | — | Unknown | M3・M4 |
| 8 測り | — | Unknown | §1.7 |
| 9.1〜9.3 決定論のテスト | 「．さくら」は `sample-ghost-kit` の `SAMPLES` に登記済み。WARN の数えは emo-compose の `log_capture.rs` の型がある | 既存の型を使う | 選びを決め打つ注入の口（3.7）が要る |
| 9.4 実機 | 根・検体は `target\` の下 | Constraint | 「面の写し」を書き出しで取るなら M8 の決めが効く |
| 10 台帳・§8 | 台帳の 8 行は `status = "absent"`（例: 6030 行目 `alternativestart`・7248 行目 `move`） | Missing | 文書の更新だけ |

### 1.4 今の作りの決まりで、この spec が守るもの

- 読み手は「原文を写すだけ・意味は下流」（`model.rs` の `DrawMethod`）。並びを数の列にまで解くか、原文の文字列で運ぶかは §1.5 の決めごと。
- 合成の入力 `PatternState` は emo-present の合成の覚えの鍵に入る＝欄を足すなら昇順で `Eq` が安定する形に。
- seriko は実行中に `EmoWorld` を見ない（表を作るときの写しだけを引く）。呼び出しの相手の解決は表を作るときに済ませる形が合う。
- 抽選の乱数の消費の順は決まっていて（`looper.rs` の D-7）、テストが期待値を焼き込んでいる＝選びが同じ乱数を引くと既存のテストの期待がずれる（要件 3.6）。
- 「時刻は正確に扱う」の原則: 刻みが遅れても、呼ばれたアニメーションの開始の時刻は呼び出しのコマの予定の時刻にする（刻みの時刻にしない）。
- 1 ファイル 1,000 行の目安。`looper.rs` 924・`table.rs` 901・`parts.rs` 893・`actor.rs` 843・`shell_target.rs` 865 は上限に近い＝足す分は兄弟の新しいファイルへ。
- 新しいテストは本番ファイルの中に書かず兄弟ファイルへ（`structure.md` の Test Naming Conventions）。

### 1.5 実装の方針の候補

#### 読み手（並びの読み）の 2 案

- **R-a 字句で守る**: `split_csv` で `( )` の中のカンマも守る。
  - ✅ 1 か所の直し。`[ ]` と同じ扱いで読み手の説明が揃う。
  - ❌ 見出しの行・element定義（ファイル名）・collision定義にも効く。閉じない `(` を含むファイル名で行の残りが 1 欄に潰れる＝要件 2.6 に触れうる。守る範囲を「欄の頭が `(`」に限れば和らぐが、字句の層が語の意味を知ることになる。
- **R-b 意味の層で継ぎ直す**: `decode_animations` で欄 1 が並びを取る 4 語のときだけ、欄 2 以降を `,` で継ぎ直して閉じ括弧までを 1 つの並びとして読む。
  - ✅ ほかの行に一切触れない（要件 2.6 を構造で満たす）。並びの語は待ち・X・Y を無視するので、継ぎ直しで欄がずれても失うものが無い。
  - ❌ 字句と意味の 2 か所に「括弧」の知識が分かれる（`[ ]` は字句、`( )` は意味）。

#### 型（相手の運び方）の 2 案

- **T-a 読み手で数の列まで解く**: `Pattern` に相手の欄（例: 番号の列と「読めなかった」の印）を足す。
  - ✅ 下流は数を引くだけ。要件 2.5 の「読めない」を読み手の時点で見分けられる。
  - ❌ `Pattern` を書く約 24 か所を直す。読み手が語の意味（どの語が並びを取るか）を知る。
- **T-b 原文の欄 2 を文字列で運ぶ**: `Pattern` に欄 2 の原文（並びは継ぎ直した原文）を足し、数への読みは下流（語の見分けと同じ emo-compose の 1 か所）が持つ。
  - ✅ 「読み手は写すだけ」の決まりに沿う。数の読み方（`.` 区切り・`[ ]`）を語の表の隣に置ける。
  - ❌ 直す所の数は T-a と同じ。下流で読むたびに解くので、表を作るときに 1 度だけ解くよう気をつける。

#### 再生（働き）の 3 案

- **案 A 今の作りを広げる**: `table.rs` で `LoopFrame` に「働き」（描く／開始／停止／差し込み／ずらし と相手の番号）を足し、`looper.rs` の進行（`put_top_play` の周り）で、前の刻みから今の刻みまでに通り過ぎた働きのコマを拾って相手を始める・止める。`Playback` に「どのコマまで処理したか」を足す。
  - ✅ 新しいファイルが少ない。今の進行の 1 本の道を通る。
  - ❌ `looper.rs`（924）・`table.rs`（901）が 1,000 行を超える。進行の関数に出来事の処理が混ざり、`frame_at` の「経過だけで決まる」性質が崩れる。
- **案 B 新しい部品に分ける**: 語の分類（8 語・並びの読み）を emo-compose の新しいファイルに、seriko 側は「働きのコマの出来事を集めて当てる」新しいファイル（例: 呼び出しの係）に置く。進行は今の `frame_at` のまま、出来事だけを別の純粋な関数（前の経過・今の経過 → 通り過ぎた働きのコマの列）で求め、統括がそれを当てる。
  - ✅ 「経過だけで決まる」コマの計算を壊さない。決定論のテストを関数単位で書ける。上限に近いファイルへ足す行が少ない。
  - ❌ 統括と新しい係の境目（再生の表・`PatternState` の差分・乱数）を設計で決める必要がある。
- **案 C 混ぜる**: 語の分類と並びの読み・読み込みの報告は案 B のように新しいファイルへ、開始・停止は `looper.rs` の既存の道（再生の追加・`-1` の停止の枝）を小さく切り出して使い、`move`・`insert` は合成の側（`plan.rs`・`pattern.rs`）を広げる。
  - ✅ 既存の停止の規則（要件 3.5・4.1「`-1` と同じ見え方」）をそのまま使える。
  - ❌ 触るファイルが一番多い（読み手 2・emo-compose 3〜4・seriko 3〜4・emo-present 1）。

#### `move` の運び方の 2 案

- **M-a 今のコマの欄で運ぶ**: `move` のコマも今のコマとして `PatternState` の欄に置き、合成が「その animation の今のコマが `move` なら土台をずらす」と読む。
  - ✅ 欄を足さない。`-1` で欄が消えればずれも戻る＝M3 が「停止で戻る」なら自然に合う。2 つ目の `move` が前を置き換える（要件 7.2）も欄の置き換えで満たす。
  - ❌ 同じ animation の `base` のコマ（`surface22` の `base,2` の後に `move`）が `move` に置き換わって消える＝`draw-methods-canon` が `base` を描くようになったとき食い違う（M7）。
- **M-b ずれの欄を足す**: `PatternState` に非公開の「土台のずれ」の欄（一番上・部品ごと）を足し、seriko が戻す時（M3）の規則で書き換える。
  - ✅ `base` の働きとずれが独立する。戻す時を測りどおりに決められる。
  - ❌ seriko 側に「ずれの持ち主のアニメーション」の記録が要る。

#### `insert` の 2 案

- **I-a pattern0 の差し込みだけ**: 今の「着せ替えは pattern0 だけ描く」を変えず、pattern0 が `insert,B` の着せ替え A を「A の位置で B の pattern0 を描く」と読む。
  - ✅ 合成の規則の変更が小さい。
  - ❌ 正典の例（A の後ろの部分と前の部分の**間**に B を挟む）は A に 3 コマ以上が要り、pattern0 だけでは表せない＝要件 6.1 の目的を満たさない。
- **I-b 着せ替えの全コマを描く**: 着せ替えのコマを pattern0 だけでなく全部重ねるよう合成を広げ、その列の中で `insert` を展開する。
  - ✅ 正典の例どおりに動く。
  - ❌ 着せ替えの描き方の今の決めごと（`plan.rs` 505 行目・「pattern1 を土台に採ると閉じ目のコマがベースの目を覆う」の実機の欠陥への手当て）に触れる。`bind+random` の再生と純 `bind` の描き方を分けて考える必要がある。M6 の測りが先。

### 1.6 設計で決めること（要件の議論へ渡す）

1. **並びの読みの置き場**（R-a 字句／R-b 意味の層）。要件 2.6 を構造で守れるのは R-b。
2. **相手の運び方**（T-a 数の列／T-b 原文の文字列）と、`Pattern` を書く約 24 か所の直し方。
3. **呼ばれる相手の範囲**: 表（引き金を持つアニメーション）に載ったものだけを相手にするか、表に載らない `never`・未知の interval のアニメーションも「呼ばれるだけの相手」として表に載せるか。要件の Adjacent は「`never` は相手が無いのと同じ扱い」とするが、要件 5.1 の記録（「相手が無い」）を `never` の相手に出すと、`seriko-script-triggers` の着地の前は正しく書いたシェルでも記録が出る。
4. **`insert` の範囲**（I-a pattern0 だけ／I-b 着せ替えの全コマ）。I-b は着せ替えの描き方を広げる＝この spec の大きさ（12〜18）を超えうる。「．さくら」は `insert` を使わない。M6 の測りの結果で決める。
5. **`move` の運び方**（M-a 今のコマの欄／M-b ずれの欄）。`base` との重なり（M7）と戻す時（M3）で決まる。
6. **部品とバルーンの面**: 入れ子の部品（`parts.rs`）の animation・バルーンの面の animation に書かれた 7 語と `move` も同じ規則で動かすか、一番上のシェルの面だけにするか。今の再生は「一番上と部品は同じ規則」を守っている。`parts.rs` は 893 行。
7. **選びの乱数の注入の口**: `SerikoLoopConfig` に欄を足す（31 か所の直し）か、`LoopRuntime::with_clock` と同じ形の後付けの口にするか。本番の種は抽選の種から派生させるか、別に取るか（info! に出す再現の手がかりも）。
8. **呼び出しの時刻**: 刻みが遅れたとき、呼ばれたアニメーションの開始を「呼び出しのコマの予定の時刻」にする（「時刻は正確に」の原則）。1 回の刻みで複数の呼び出しのコマを通り過ぎた・呼ばれた先の pattern0 がまた呼び出し、の連なりを 1 つの刻みの中で解く。
9. **読み込みの記録の置き場**: 要件 2.5・5.1・5.3・6.4〜6.6 の記録を、`plan_issues.rs` の `ShapeIssue` に種類を足して `shell_target.rs` の入口で出すか、seriko の表を作るとき（`table.rs`・今 WARN を出している所）に出すか。前者は読み込み 1 回につき 1 度が既に保証されている。element定義に書いた語は今 `undrawn.rs` の一般の文言で出ている＝語ごとの文言に変えるか。
10. **`plan.rs` 535 の合成ごとの `from_name`**: 8 語を外すだけでなく、ほかの未知の語でも合成のたびに WARN が出る今の作りを、この spec で表を作るときの 1 度へ寄せるか（`animated-surface-log-flood` と同じ所に触れる）。
11. **書き出しの面の写し**（M8）: `settle_surface` は `runonce` の待ちの合計の最大まで進める＝`surface12` の写しは `move,…,9999` の後（絵が窓の外）になる。書き出しに `move` を入れるか、要件 9.4 の「面の写し」をどう撮るか。
12. **emo-atlas の 7 語の写し**（`manifest.rs` の `bind_only_paths`）: 8 語の表を 1 か所に寄せたあとも写しを残すか（emo-atlas は emo-compose を参照できない）。

### 1.7 調べが要る点（Research Needed・SSP の MCP での測りは開発者の許しを得てから）

要件 8 の M1〜M5 に加えて、測りの候補:

| 印 | 測ること | 効く所 | 検体の形 |
| ---- | ---- | ---- | ---- |
| M1 | 再生中の相手を `start`・`parallelstart`・`alternativestart` で呼んだとき（頭から／続ける／捨てる） | 要件 3・「．さくら」の animation0・1（自分も抽選される） | 長いコマ列の animation を `always` で回し、別の animation から途中で `start` |
| M2 | `alternativestop` の選ぶ範囲 | 要件 4.3 | 並びの片方だけ再生中の状態で呼ぶ |
| M3 | `move` のずれが戻る時（停止・`runonce` の終わり・面の切り替え） | 要件 7.6・決めごと 5 | `surface12` の形を短くしたもの＋`-1` で終わる版 |
| M4 | `move` でずれる範囲（土台だけ／ほかの animation の層・当たり判定も） | 要件 7.6 | 土台＋`always` の重ね＋当たり判定を持つ面 |
| M5 | 着せ替えを `start` 系・`stop` 系の相手に指したとき | 要件 5.3 | `bind` の animation を `start` |
| **M6（新）** | 純 `bind` の着せ替えの pattern1 以降を SSP が重ねて描くか、`insert` を pattern1 以降に書いたとき差し込まれるか | 決めごと 4（I-a／I-b） | 3 コマの着せ替え A（後ろ・`insert,B`・前）と B |
| **M7（新）** | 同じ animation の `base,N` の後の `move` が、置き換えた土台をずらすか（`surface22` の形） | 決めごと 5 | `surface22` を短くしたもの |
| **M8（新）** | SSP の面の書き出し（`dump_surface`）が `runonce` の `move` を写すか | 決めごと 11・要件 9.4 | 「．さくら」の `surface12` |

ほかに外部の依存の調べは要らない（新しいクレートは足さない・`tokio` 等の並行の約束にも触れない）。

### 1.8 大きさと危うさ

- **大きさ: L（1〜2 週）**。読み手・語の見分け・seriko の再生（刻みの間の出来事・連なり・巡回・別の乱数）・合成（ずれ・差し込み）・読み込みの記録・決定論のテスト・実機の 1 回と、4 つのクレートに手が入る。`insert` を I-b にすると L の上の端。I-a か範囲の外へ出すなら M に近づく。
- **危うさ: 中**。どれも今の型（表を作るときに 1 度解く・刻みで進める・読み込みの報告を 1 度出す）の上に足せる。危うさの元は、⑴ 刻みの間に通り過ぎたコマを拾う仕組みが今の「経過だけで決まる」作りに無い新しい型であること、⑵ 着せ替えの描き方（pattern0 だけ）の決めごとに `insert` が触れること、⑶ 上限に近いファイルが多く兄弟のファイルへの分け方が要ること、⑷ M1・M3・M4・M6 の答えで再生の規則が変わること。

### 1.9 設計の段への申し送り

- 推しの方向（決めではない）: 並びは **R-b**（ほかの行を構造で守る）、語の分類と並びの数への読みは emo-compose の新しいファイル 1 つに寄せ、`targets_animation_id`・`is_move_name` をそこから引く。再生は **案 B か C**（`frame_at` を壊さず、刻みの間に通り過ぎた働きのコマを別の純粋な関数で求める）。`move` と `insert` は M3・M6・M7 の測りの後に決める。
- 同じウェーブに置けない相手（brief の 2026-10-24 の再測定のまま）: `animated-surface-log-flood`（`plan.rs`・`pattern.rs`・`lib.rs`）・`shell-mouse-cursor`（`areka-parsers/src/shell/mod.rs` に新しいファイルを足すとき）・`mcp-strict-errors` の「再生まで印を運ぶ」案（seriko の `actor.rs`）。版の外と分け合うファイル: `extent-element-offset`（`plan_extent.rs`）・`seriko-trigger-lottery-gaps`（seriko の `table.rs`・`looper.rs`・`parts.rs`・`actor.rs`）。
- 確かめの土台: 「．さくら」は `sample-ghost-kit` の `SAMPLES` に登記済み。WARN の数えは `areka-emo-compose/src/log_capture.rs` の型。選びの決め打ちは乱数の注入の口（決めごと 7）で行う。

## 2. 要件の議論の決め（2026-10-30・開発者「承認」＝全議題を推奨案で確定）

| 議題 | 決め | 要件への反映 |
| ---- | ---- | ---- |
| 1 旧 SERIKO 1.x の書式 | 別の spec `seriko-legacy-pattern-format` に分ける・`/kiro-discovery` で起票 | Out of scope に確定を記した |
| 2 `never` の相手 | この spec で「呼ばれるだけの相手」として表に載せる・`seriko-script-triggers` は `\i` の口だけ（同 brief に申し送り） | 要件 11 を新設・5.1・Adjacent・10.1 |
| 3 ID の欄の名前 | 番号として読めない欄として何もせず 1 度記録（今の要件 2.5 のまま） | 変更なし |
| 4 `insert` | 範囲に残し、M6 の測りどおり正典の形で作る | 要件 6.8（A で追加済み）のまま |
| 5 部品とバルーンの面 | 一番上の面と同じ規則で働かせる | 要件 3.8・In scope |
| 6 SSP の測り（M1〜M8） | この議論の後、このセッションで続けて測る・reload は毎回許しを得る・結果はこのファイルへ | 要件 8.2 のまま |
| 7 `plan.rs` の合成ごとの `from_name` | 表を作るときの 1 度へ寄せ、8 語以外の未知の語も 1 度に（`animated-surface-log-flood` の brief に申し送り） | 要件 5.5・In scope |

§1.6 の決めごと 3（呼ばれる相手の範囲）は議題 2 で「`never` は載せる・未知の interval は載せない」に決まった。決めごと 6（部品とバルーン）は議題 5 で「同じ規則」に決まった。決めごと 10 は議題 7 で「全部の語を表を作るときの 1 度へ」に決まった。残る §1.6 の決めごとは設計の段で解く。

## 3. SSP の測り（2026-10-30・議題 6・開発者「はい、測って」）

### 3.1 手順と検体

- SSP: `C:\wintools\ssp\ssp.exe` 2.9.26（FileVersion `2, 9, 26, 3000`）。
- 検体: 使い捨てのテスト用ゴースト `calltest`。作り方は `ssp-measure/make_calltest.py`（PIL で 200×200 の画像を作り、`surfaces.txt` を書く）。置き場はワークツリーの `target\seriko-pattern-calls-ssp\calltest\` だけ。SSP の `ghost\` には何も置いていない。
  - 絵: `surface0.png`（灰色の土台）・`surface50.png`（濃い灰色の土台）・101 赤 (20,20)-(99,99)・102 緑 (60,60)-(139,139)・103 青 (100,100)-(179,179)・104 黄 (140,20)-(189,69)・199 透明。
  - 着せ替え: `sakura.bindgroup3`（既定 0）・4〜7（既定 1）。カテゴリはそれぞれ別の名前にした（後述）。
- 測り方は 2 つ。
  1. **オフラインダンプ**（ゴーストを起こさない・SSP の環境に何も書かない）:
     - `ssp.exe --offline-dump <calltest> --dump-surface-list … --dump-output-dir … --dump-error-log …`
     - 呼び出しの元を `--dump-animation 9` で再生し、コマごとの合成を写した。`animation9` は `never` で、待ちだけのコマ（透明の 199）を時計にしている。
     - SSP のヘルプの起動オプション一覧（https://ukatech.shillest.net/ukadoc-ssp/functions/option/ ）に「start・alternativestart・parallelstart で起動された別のアニメーションは連番の対象にならない」とあり、実際にも同じ Warning が出た。ただし合成の中では、呼ばれたアニメーションも動いていた（M1 の結果でわかる）。
  2. **動いている SSP**（`move` を見るため）:
     - `ssp.exe --option readonly,dontloadmodule --ghost <calltest の絶対パス>` で起こした（2.8.99 以降の一時の読み込みフォルダ・設定は保存されない）。
     - SSP の MCP で `\s[N]`・`\i[9]` を送り、窓の矩形（`currentghost.scope(0).rect`）と、窓のまわりの画面の写し（PowerShell の `CopyFromScreen`・画面の操作はしない）で絵の位置を測った。
     - 実効の倍率は `currentghost.scope(0).scaling` = `200,200`（200×200 の絵が 400×400 で出る）。
     - reload は 1 度も打っていない。測り終えて `\-` で閉じた。
  - 同じウェーブの兄弟（Stay-on-top directives・Balloon scroll arrows）には、起こす前と閉じた後に知らせた。
- 気づいた癖（この spec の外。記録だけ）:
  - SSP は、同じカテゴリの中では既定で有効になる着せ替えを 1 つにした（カテゴリ `test` に 4〜7 を置くと 4 だけが有効になった）。
  - SSP の MCP に 1 本の台本で `\s[0]\s[24]…` と送ると、2 つ目の `\s` が効かないことがあった（`\s` を 1 本ずつ送れば効く）。
  - 画像の左上の画素の色は抜き色として扱われた（黒の目印が透明になった）。

### 3.2 結果

| 印 | 測ったこと | 結果 | 根拠 |
| ---- | ---- | ---- | ---- |
| M1 | 再生中の相手を `start`・`alternativestart,(1)`・`parallelstart,(1)` でもう一度呼ぶ | **pattern0 から頭でやり直す**（3 語とも同じ） | 相手 = 赤 0〜12 秒・緑 12〜24 秒・その後 `-1`。9 秒に呼び直す。15 秒が赤・25 秒が緑（続けるなら 15 秒が緑・25 秒は何も無い）。オフラインダンプの面 10〜12 |
| M2 | `alternativestop,(1,2)`。1 は再生中（`always`）、2 は再生していない | **並びの全部から選ぶ**。再生していない 2 を選んだときは何も起きない | 8 回のうち、1 が止まったのが 5 回・残ったのが 3 回（止める前は 8 回とも再生中）。オフラインダンプの面 110〜117 |
| M3 | `move` のずれが戻る時 | **アニメーションの終わり・`stop`・`-1` では戻らない**。面を出したとき（同じ面の出し直しを含む）にだけ戻る | 動いている SSP の面 21（呼ばれた側が `move` の後に終わる）・22（1 秒後に `stop`）・23（1 秒後に `-1`）で、2.2 秒までずれたまま。`\s[0]`・`\s[23]` の出し直しで元へ戻った |
| M4 | `move` でずれる範囲 | **キャラクター全体**。土台・element（黄）・ほかのアニメーションの層（`runonce` の緑）・当たり判定（Head の枠）が、そろって同じだけずれる。絵は欠けない（窓ごとずれる） | 動いている SSP の面 24。`\![enter,collisionmode]` で枠も写した。灰色・緑・黄が、どれも y で +40 |
| M4 の付記 1 | ずれの単位 | **表示の倍率がかからない**。`move,0,0,0,40` が、倍率 200% でも画面の 40 画素だった（重ねるコマの座標と絵は 2 倍で出る） | 面 20・24 |
| M4 の付記 2 | デスクトップの下の端に揃える置き方（SSP の既定）とのかね合い | **下へのずれは、揃えに打ち消されて見えない**。窓の矩形は上へ 40 動き、画面の絵の位置は変わらなかった。`\![set,alignmenttodesktop,free]` で揃えを外すと、下へ 40 ずれて見えた | 面 20（揃えあり・なしの両方） |
| M5 | 着せ替え（`bind`）を呼び出しの相手にする | 無効の着せ替えを `start` すると、その絵が出る。有効の着せ替えを `stop` しても消えない | オフラインダンプの面 25・26 |
| M6 | 純 `bind` の pattern1 以降・pattern1 の `insert` | **全部のコマを pattern の番号の順に重ねて描く**。pattern1 の `insert,7` は、その位置に 7 を差し込む（赤の上・青の下に緑）。7 の本来の位置では描かない。7 が無効なら何も描かない | オフラインダンプの面 27・28・35（35 は bindgroup4 の上で、7 が無効・有効の両方） |
| M7 | `base,50` の後の `move` | **差し替えた土台もずれる**。`base` のコマは、後の `move` のコマで消えない | 動いている SSP の面 29（濃い灰色が +40） |
| M8 | 書き出しの写しに `move` が入るか | **入らない**。オフラインダンプ（`runonce` を 1 周させた状態）にも MCP の `dump_surface`（「before scaling and transparency」）にも、ずれは写らない | オフラインダンプの面 20〜24・29 と、動いている SSP の `dump_surface` |

### 3.3 ukadoc との食い違いと、要件へのはね返り

- ukadoc の `move` は「ベースレイヤの表示位置を、元の表示位置から指定座標分ずらす」。SSP は、土台の層だけでなくキャラクター全体（ほかの層・当たり判定）をずらし、窓ごと動かしていた（M4）。
- 要件 8.1 は「M1〜M7 を SSP と同じにする」。ただし M4 とその付記 1・2 は、ukadoc の字面と、areka の決めごと（DPI 追従・作業領域の中に置く）にかかるので、開発者に確かめる（§4）。
- M1・M2・M3・M5・M6・M7・M8 は要件に写した（要件 3.9・4.3・5.6・5.7・6.8・7.6・9.4・9.6）。

## 4. `move` の議論の決め（2026-10-30・開発者「承認」＝3 議題とも推奨案）

| 議題 | 決め | 要件への反映 |
| ---- | ---- | ---- |
| 1 ずれる範囲 | SSP に合わせ、キャラクター全体（すべての層と当たり判定）を窓ごとずらす。窓の大きさは変えず、絵を欠けさせない | 要件 7.1・7.5・見出し |
| 2 単位と表示の倍率 | ほかのコマの座標と同じに倍率をかける（DPI 追従の決めごと）。SSP は倍率をかけないので、違いを §8 に記す | 要件 7.1・10.2 |
| 3 下の端に揃える置き方 | ずらした窓の位置にも、今ある置き方（揃え直し・作業領域の中に置く決まり）を後から当てる。下揃えでは揃えが勝つ＝SSP と同じ見え方 | 要件 7.8・9.4 |

設計への申し送り: `move` は合成の命令（`plan.rs`）ではなく、窓の位置のずれとして置き方（`crates/areka/src/placement/`・`emo2_boot` の揃え直し）へ運ぶ形が素直。§1.6 の決めごと 5（M-a／M-b）は、この形を前提に設計の段で解く。当たり判定も窓ごと動くので、当たり判定の座標は直さなくてよい見込み。

## 5. 設計の調べと決め（2026-10-30・`/kiro-spec-design -y`）

### 5.1 調べの範囲

- 種類: 今ある仕組みの拡張（light discovery）。外部の依存の調べは要らない（新しいクレートを足さない）。
- 使った手引き: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`。steering は `tech.md`・`structure.md`（テストの置き場と 1,000 行の目安）・`logging.md`。
- コードの調べ（下の 5.2）は、seriko の再生と、`move` から窓の位置までの道の 2 つを、読むだけの調べの係に分けて行い、要約だけを受けた。合成・読み手・提示の境目は直接読んだ。

### 5.2 わかったこと

- **seriko の出口は `DisplayCommand` の 1 本だけ**（`output.rs`・`#[non_exhaustive]` でない）。areka の `PresentBridge`（`emo2_boot/adapter.rs`）が `PresentCommand` へ写し、UI のフレームの drain（`frame/drain_resnap.rs`）が提示へ当てる。位置を運ぶ道は無い。だから、ずれは新しい variant で運ぶ。`Show` と同じ FIFO に載るので、順が崩れない。
- **窓の位置の正本は `WindowPos` だけ**。「元の位置」の考えは無い。書き込みは `enqueue_window_set_pos` に集まる。下揃えは `project_anchor`、見える範囲の守りは `apply_visibility_guard` が受け持つ。`\![move]` の倍率は `scale_signed` と `applied_ratio`。当たり判定は窓の中の座標なので、窓と一緒に動く。バルーンは `follow_balloon` で「キャラ窓 ＋ 相対 offset」を追う。
  - だから、ずれは「当てた分」の記録を部品に持つ。元の位置は「今の位置 − 当てた分」で求める。
  - 保存とドラッグの終わりだけは、ずれを引くか捨てる（そうしないと、起動のたびにずれが積もる）。
- **同じ面の出し直しは、looper へ届かない**。`states.apply` が `Unchanged` を返し、`on_surface_changed` を呼ばない。だから、戻す口（`on_surface_shown`）は actor が `Changed`／`Unchanged` の両方で呼ぶ。
- **過去の時刻に始まった再生を入れる先例がある**。`Armed::poll`・`fire_top_triggers` が `Playback { started_at_ms: 過去 }` を入れる。だから、呼び出しの開始は「予定の時刻の `Playback` を入れる」だけで、今の進め方（経過から今のコマを計算する）を変えずに済む。
- **今の進行の段は、再生中の番号の写しを先に取る**。この刻みで始まった相手は、次の刻みまで描かれない。だから、`run_calls` を写しの前に置く（1 フレーム遅らせない）。
- **一番上と部品の進め方は、2 本に分かれている**（`looper.rs` の `put_top_play` と `parts.rs` の `progress`）。だから、働きのコマの当て方は 1 つの純粋な関数（`calls.rs`）にし、`CallPlays` の 2 つの実装で両方から呼ぶ（実装が 2 つ本当にあるので口を作る）。
- **`SerikoLoopConfig { … }` を書く所は 23 か所**（本番 2・テスト 21）。選びの乱数を後から差す口も考えたが、本番の構築は `spawn_seriko_clocked` の中にあるので、結局 `config` に欄が要る。だから、欄を足して 23 か所を機械的に直す。
- **読み込みのときの記録は 2 つの型がある**。
  - emo-compose の `shape_issues`: シェルだけ。`shell_target.rs` が出す。バルーンの読み込みは呼ばない。
  - seriko の表を作るときの `warn!`: シェルは読み込みにつき 1 度、バルーンはスコープの読み込みごとに 1 度。
  - だから、8 語の記録（insert を含む）は seriko の表を作るとき（`table_calls.rs`）に集める。そうすると、シェルとバルーンの両方で 1 度ずつ出る。8 語以外の未知の語の記録は今のまま（`shape_issues`・表の `from_name`）。
- **合成の D9-3 の門**（`plan.rs` の `is_bind_animation`・`nesting.rs` の ③）は、`bind` と `bind+random` の両方で、無効な着せ替えのコマを止めている。このままでは 5.6（無効の純 `bind` を `start` で描く）を満たせない。純 `bind` は今まで seriko のコマを持たなかったので、門を `bind+random` だけに狭めても、今の守りは崩れない。
- **外形**（`plan_extent.rs`）は、着せ替えの pattern0 と `always` の全部のコマを数えている。純 `bind` の全部のコマを描くなら、外形も全部を数える（窓の大きさは静的なまま・7.5）。
- **`settle_surface` は出力を捨てる**（`Discard`）。ずれを `PatternState` の外に持てば、書き出しには構造の上で写らない（9.6・M8）。

### 5.3 まとめの 3 つの見方（design-synthesis）

- **一般化**:
  - 8 語の見分けと相手の欄の読みは、1 つの関数の組（`pattern_word`・`call_targets`）にする。seriko の表・合成の手順・入れ子の辺・外形・emo-atlas の写しは、その答えだけを使う。
  - 着せ替えの描き方（全部のコマ・`insert`）は 1 つの手順の表（`BindSteps`）にし、合成と入れ子の辺が同じ表を読む。
- **作るか借りるか**: 外の部品で解ける所は無い。乱数は今の `seeded_rng` を 2 本目に使う。待ち行列は標準の `BinaryHeap` を使う。窓の当て方は今の `project_anchor`・`apply_visibility_guard`・`move_window_with_route`・`scale_signed` をそのまま使う。
- **削ったもの**:
  - ずれを `PatternState` に入れる案。emo-present の合成の覚えの鍵が `move` のコマごとに外れる。並走の `animated-surface-log-flood` の持ち場（`pattern.rs`・`cache.rs`）にも触れる。
  - areka の側に別の mpsc を作る案。`Show` との順が崩れる。
  - `LoopFrame` に「見える描くコマ」の欄を足す案。後ろ向きに探す小さな関数 `visible_frame` で足りる。
  - `settle_time` を呼び出しの連なりまで延ばす案。要件に無い。

### 5.4 設計の決め

| 決めごと（§1.6） | 決め | 理由 |
|------|------|------|
| 1 並びの読みの置き場 | R-b の変形: `decode_animations` が、pattern定義の欄 2 が `(` で始まるときだけ `)` まで継ぎ直す（語は見ない） | ほかの行を構造で守る（2.6）。読み手が語の意味を知らずに済む |
| 2 相手の運び方 | T-b: `Pattern.target`（欄 2 の原文）。数への読みは emo-compose の `call_targets` | 「読み手は写すだけ」。約 24 ファイルの構造体の書き方は機械的に直す |
| 3 呼ばれる相手の範囲 | `never` と純 `bind` を `CallOnly` で表に載せる。未知の interval は載せない | 議題 2・M5（無効の着せ替えも `start` で描く） |
| 4 `insert` の範囲 | I-b: 有効な純 `bind` は全部のコマを描き、どのコマの `insert` も展開する。`bind+random` は pattern0 だけ | M6。`bind+random` の 2 コマ目以降を静的に描くと、閉じ目が目を覆う実機の欠陥に戻る |
| 5 `move` の運び方 | M-a でも M-b でもなく、seriko が (scope, 面の段) のずれを持ち、`DisplayCommand::MoveShift` で提示へ、置き方が窓へ。働きのコマは欄を書き換えない（`visible_frame`） | §4 の決め（窓ごと・置き方へ運ぶ）。M7（`base` が消えない）と M3（終わりで戻らない）を構造で満たす |
| 6 部品とバルーン | 同じ `run_calls` を `CallPlays` の 2 つの実装で呼ぶ。部品の `move` もキャラクター全体のずれ。バルーンの面のずれはバルーン窓に当てる | 議題 5（同じ規則） |
| 7 選びの乱数 | `SerikoLoopConfig.pick_rng`（23 か所を直す）。本番は別の `RandomState` の種。`info!` に 2 つの種 | 3.6・3.7。後から差す口は本番の構築の中に届かない |
| 8 呼び出しの時刻 | 予定の時刻 `開始 ＋ 累積の待ち`（`always` は周ごと）の待ち行列。`run_calls` は写しの前・`refresh` でも呼ぶ | 時刻は正確に・1 フレーム遅らせない |
| 9 記録の置き場 | 8 語の記録は seriko の表を作るとき（`table_calls.rs`）。element定義は今の `undrawn.rs` のまま | シェルとバルーンの両方で 1 度ずつ出る |
| 10 `plan.rs` の合成ごとの `from_name` | `BindSteps` を読み込みのときに作り（記録なしの `quiet_method`）、合成は手順を読むだけ | 議題 7。未知の語の記録は `shape_issues` が 1 度出す |
| 11 書き出しの写し | ずれを `PatternState` の外に持つので、書き出しには写らない | M8・9.6 |
| 12 emo-atlas の写し | 写しを残し、`move` を足す | emo-atlas は emo-compose を引けない |

### 5.5 正典が黙る所の areka の読み（§8 へ）

- 「着せ替え」は純 `bind`（`interval,bind`）と読む。要件 5.3・5.6・5.7・6 の括弧書きのとおりである。`bind+random` は抽選で動く animation として、7 語と `move` を `random` と同じに働かせる。`insert` は差し込まない（6.6 の記録の対象）。
- `always` を `stop` したら、止めた時刻から周の頭で始め直す（`always` は interval がいつも再生する規則なので、止めた後の次の開始がその時刻に来る＝4.5）。SSP では測っていない。
- 差し込む着せ替え A が無効で展開されなかったとき、差し込まれる B は本来の位置に描く。2 つの着せ替えが同じ B を差し込むときは、描く順で先に当たった方だけが描く。SSP では測っていない（M6 は A が有効のときだけ）。
- 利用者のドラッグの終わりで、ずれは捨てる（置いた所が新しい元の位置）。位置を保存するときは、ずれを引いた位置を保存する。
- 3.6 の読み: 選びは抽選の乱数を引かない。呼び出しが animation を再生中にすれば、その animation が抽選から外れるのは今の規則どおりである。

### 5.6 危うさと手当て

- **純 `bind` の全部のコマを描く変更で、黄金のテストの期待が変わりうる** → M6 の測りを根拠に期待を直し、直したテストの名を作業の記録に残す。
- **`looper.rs`・`parts.rs`・`actor.rs` が 1,000 行に近い** → 本体を `calls.rs`・`table_calls.rs` に置き、足すのは呼び出しの数行と欄だけにする。
- **並走の `animated-surface-log-flood` と `plan.rs`・`lib.rs` が重なる** → `build_plan` の引数を変えない。`plan.rs` は着せ替えの静的な寄与の塊を 1 つの呼び出しに替えるだけにする。`lib.rs` は `mod` 2 行と `pub use` だけにする。`pattern.rs`・`cache.rs` には触らない。
- **置き方の書き手が多い** → ずれの記録は「当てた分」だけを持ち、ほかの書き手は今のまま窓ごと動かす。引く所は保存の 2 か所とドラッグの終わりだけに限る。

## 6. 設計の議論の決め（2026-10-30・開発者「承認」＝6 議題とも推奨案）

| 議題 | 決め | 反映 |
| ---- | ---- | ---- |
| 1 `always` を `stop` した後 | 面を出し直すまで止めたまま（SSP の測り M2: 待ち 0 で赤を出す `always` を止めた 0.1 秒後も赤が出なかった） | 要件 4.5・設計の Calls と §8 の一覧・テスト |
| 2 要件 3.6 の読み | 選びは抽選の乱数を引かない。呼び出しを持たないシェルでは、同じ種なら抽選の結果は今と同じ | 要件 3.6 の文を改めた |
| 3 「着せ替え」の範囲 | 純 `bind` だけ。`bind+random` は `random` と同じに扱い、7 語と `move` が働く | 設計のまま |
| 4 差し込む側が無効・2 つの差し込み手 | 設計の読み（無効なら B は本来の位置・描く順で先の方だけが差し込む）で仮に確定。タスクに入る前に SSP のオフラインダンプで測って合わせる（2026-10-30 に測って確かめた＝§6.1） | 設計の BindSteps に注記 |
| 5 ドラッグと保存 | ドラッグの終わりでその窓のずれを 0 にする。保存はずれを引いた元の位置 | 設計のまま |
| 6 組の相手 | 一緒に動かさない（動くのはそのキャラ窓とそのバルーンだけ） | 設計のまま |

### 6.1 議題 4 の測り（2026-10-30・SSP 2.9.26 のオフラインダンプ）

検体は §3.1 の `calltest` に 2 つの面を足したもの（`target\seriko-pattern-calls-ssp\calltest\`・面の定義は下のとおり）。Balloon scroll arrows が SSP を使い終えてから測った。

| 面 | 形 | 結果 | 読み |
| ---- | ---- | ---- | ---- |
| 40 | 無効の純 `bind` 3（赤・`insert,7`・青）と、有効の純 `bind` 7（緑） | 緑が描かれた（赤と青は無い） | 差し込む側が無効なら、B は本来の位置に描く＝設計の読みどおり |
| 41 | 有効の純 `bind` 4（赤・`insert,7`・青）と 6（`insert,7`・黄）、有効の 7（緑） | 赤の上に緑・緑の上に青（重なりは青）・黄あり | 描く順で先の 4 だけが差し込む。6 も差し込むなら、緑が青の上に来る＝設計の読みどおり |
