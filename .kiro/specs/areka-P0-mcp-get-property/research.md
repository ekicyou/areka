# ギャップ分析: areka-P0-mcp-get-property

> 実測は 2026-10-04・本ブランチ（main `e2a373b5` の上に spec の初期化 1 コミット）。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 本書は分析と選択肢を並べるもので、決定はしない（決めるのは要件ディスカッションと設計）。

## 1. 分析の要約

- **足りないのは 2 か所だけ**: ⑴ ゴーストの実行系（`GhostRuntime`）の記憶の読み手（`SylphyaReader`）を外から借りる読み口が無い。⑵ `get_property` の処理の中身がダミー（`NG:not implemented yet`）。ほかの部品（宛先の解決・答えの形・答えの記録・読み手の「値／見つからない」の 2 択・問い手の組み方・ログの捕まえ方・本物の実行系を起こすテストの土台）はすべて既にある。
- **要件 5 の「触る 3 ファイル」で閉じられる**ことを実物で確かめた。置き場（`GhostSlot`）と `GhostSession::runtime()` は `pub(crate)`、`GhostRuntime::mount()` と `ghost_asker_id` は `pub`、`areka` は `areka-sylphya` に直接依存済み、`log-capture-kit` は dev 依存済み＝`mod.rs`・`resolve.rs`・`ghost_session.rs`・`Cargo.toml` を触らずに済む。
- **「実行系が無い」道（要件 3.1）は本番の汲む系からは来ない**。来るのは振り分け（`dispatch`）をテストから直に呼ぶときだけ。ただし共有のテスト `get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure` が、空の World でまさにこの道を通し「名前の解決の失敗の 2 文言のどちらでもない」ことを求める＝この分岐は省けない。
- **主な分かれ目はテストの組み方**（純粋な関数に切り出して作った鏡像で試すか、本物の実行系〔`SwitchRig`〕を起こして試すか、その組み合わせか）。特に要件 4.1⑸「問い手の選び方」は、問い手を選ぶ配線そのものを通さないと檻にならない。
- 規模 **S**（3〜5 タスク）・危険 **低**。調べ残しは小さい（下の「調べが要る点」）。

## 2. 今あるもの（要件ごとの対応表）

| 要件 | 使える既存の部品 | 状態 |
|---|---|---|
| 1.1 素の値で答える | `areka_mcp::tools::outcome::value`（`crates/areka-mcp/src/tools/outcome.rs`・`OK:` なし・本文 1 つ・`is_error: false`） | ある |
| 1.2 空の値は空の本文 | `outcome::value("")` は `ToolContent::Text("")` を 1 つ持つ形（`is_error: false`） | ある |
| 1.3 宛先のゴーストの記憶から、そのゴーストの問い手として読む | 記憶＝`GhostRuntime` の私有の欄 `sylphya_reader`（`crates/areka-ghost/src/runtime.rs` の `pub struct GhostRuntime`）。問い手＝`areka_ghost::sylphya_wiring::ghost_asker_id(&runtime.mount().shiori.dir)`（起動の手順 `boot_with_origin` と `crates/areka/src/install/names.rs` の `publish_to` が同じ組み方） | **読み口が無い（Missing）**。問い手は組める |
| 1.4 名前を手直ししない | `SylphyaReader::resolve_dotted_str`（`crates/areka-sylphya/src/reader.rs`）に受け取ったまま渡せば足りる | ある（下の注 1） |
| 1.5 その場で答える | `resolve_dotted_str` は鏡像の読みロック→`Arc` の複製→表引きだけ（送受信・ファイル・他スレッドへの問い合わせ無し） | ある |
| 1.6 切替の後は今のゴースト | 汲む系 `drain`（`crates/areka/src/mcp/mod.rs`）が要求 1 件ごとに `resolve::active` を読み直す。処理の中で置き場から実行系を引けば、その時点の今のゴースト | ある（配線） |
| 2.1〜2.3 値の無い名前は `NG:Cannot find such property name.` | `DottedResolution::NotFound`（`crates/areka-sylphya/src/value.rs`）。空文字は `parse_dotted` の `KeyParseError::Empty`、`a..b`・末尾の点は `EmptySegment` → `warn!` を残して `NotFound` | ある（文言は処理側で書く） |
| 3.1 実行系が見つからないとき | 置き場の読み方は `world.get_non_send::<GhostSlot>()?.0.as_ref()?.runtime()`（`install/names.rs` の `current_runtime` と同じ 1 行。ただしあちらは `pub(super)` で借りられない） | 文言と `warn!` を書く |
| 3.3 panic しない | 読み手は書式の誤りでも `NotFound` に倒れる。置き場の欠落は `Option` で拾える | ある |
| 4.1⑷ `warn!` 1 件の確認 | `log_capture_kit::capture`（`crates/log-capture-kit/src/capture.rs`・`areka` の dev 依存）。手作りの subscriber は見張りのテスト（`log-capture-kit` の走査）が赤にする | ある |
| 4.1⑸ 問い手の選び方 | 本物の実行系を起こす土台 `SwitchRig`（`crates/areka/src/emo2_boot/ghost_switch_test_support.rs`・`mcp_tests.rs` が既に使用）＋`runtime.sylphya_publisher().set(asker, 自由な名前, 値)`→`barrier()` でゴーストごとの点付きの値を作れる | ある（組み方は選択肢・下の 4 節） |
| 4.3 ダミーの期待の書き換え | `get_property_tests.rs` の `answers_not_implemented_yet_with_an_empty_world` | 書き換え対象 |
| 4.4 共有のテストを変えずに緑 | `mcp_tests.rs` の上記の共有テストは、空の World・作った `ActiveGhost` で `get_property` を通す。要件 3.1 の文言 `NG:Property system is not available` は 2 文言のどちらとも違うので緑のまま | 確認済み |
| 4.6 実機確認 | 前例＝`completed/areka-P0-mcp-tool-entrances/verification/signoff.md`（配布形の `areka.exe`・`claude mcp add --transport http` を project スコープ・curl・`RUST_LOG=…,areka_mcp=debug`・有界の自動終了） | 手順の前例あり |
| 5.2 読み口 1 本だけ | `GhostRuntime` には同じ型の読み口（`kanade()`・`dispatcher()`・`sylphya_publisher()`・`mount()`・`shiori_probe()`）が並ぶ。`sylphya_reader` という名前の関数はまだどこにも無い（衝突なし）。`runtime.rs` は 783 行 | 足すだけ |

**注 1（名前の英字の大小）**: SSP は `BASEWARE.NAME` を通すが、areka の読み手は大小を区別する（`doc/ssp-mcp/survey.md` §7.3 の 4）。要件 1.4 は「手直ししない」なので、`get_property` だけ畳むと SHIORI の `GetProperty`・`%property[]` と食い違う。直すなら読み手の側で全経路をそろえる話で、本 spec の範囲外（要件 Boundary の Out of scope）。

## 3. 実物で確かめた事実（設計の前提になるもの）

1. **読み手の 2 択と SHIORI 側の区別。** `ShioriHostSink::GetProperty`（`crates/areka/src/shiori_host.rs`）は `Value(v)` を成功、`NotFound` を `SHIORI_E_PROPERTY_NOT_FOUND` にする。`get_property` はこの写し方をそのまま `outcome::value(v)`／`outcome::ng("Cannot find such property name.")` に写せばよい。
2. **SHIORI 側の記憶は別物。** 本番以外の経路（`shiori_session.rs`・`shiori_demo.rs`）の `ShioriHostSink` は `spawn_app_sylphya_sink` で**別の sylphya を 1 本起こして**使う。ゴーストの実行系の記憶とは別の実体（要件の「brief の記述を実物で引き直して改めた点」1 と一致）。
3. **今の本番の記憶の中身。** 大域の点付きの区画に `baseware.name`＝`areka`・`baseware.version`（`publish_ghost_statics` が `publish_static` の dotted で載せる）と、永続から読み戻した `areka.*` の名前（`spawn_sylphya` の初期の像・`PersistPut`）。**ゴーストごとの点付きの区画に本番で書く人は今いない**（書くのは `SylphyaPublisher::set` の自由な名前＝`SetClass::StoreWrite` だけで、その呼び手はアプリ側の別 sylphya）。よって要件 1.3 の「ゴーストごとの値が勝つ」は今の本番では観測できず、テストで作って確かめるしかない。
4. **テストで値を作る道。** ゴーストごと＝`publisher.set(問い手, "test.key", 値)`（根が正準の 10 本〔`DOTTED_ROOTS`〕や葉が正準の名前だと `NotSettable` で書かれない＝自由な名前を使う）。大域＝`publisher.publish_static(問い手, vec![], vec![(名前, 値)])`（どの名前でも大域へ着地）。どちらも投函だけなので `barrier()` で反映を待つ。純粋に組むなら `MirrorImage::empty()`（欄はすべて `pub`）を `SharedMirror::new` で包み `SylphyaReader::new` に渡せる（`reader.rs` の内部テストと同じ作り方）。
5. **「実行系が無い」道の到達性。** `resolve::active` は置き場が無い・空・実行系が無い単位（`GhostSession::for_test`）で `None` を返し、`Omitted::UseActive` の解決は `NG:Specified ghost is not active` で `handle` の手前で終わる。`drain` は単一スレッドで、`active` を読んでから `handle` を呼ぶまでの間に World を変える処理は挟まらない＝**本番の汲む系からは来ない**。来るのは `dispatch` を直に呼ぶテスト（共有テスト・本 spec のテスト）だけ。それでも共有テストがこの道の答えを縛るので、分岐は要る。
6. **`ActiveGhost` と置き場の実行系の対応。** `ActiveGhost` は名前とルートフォルダ（`GhostSession::ghost_dir` の絶対化）だけを持つ。置き場は 1 つ（`GhostSlot(Option<GhostSession>)`）で、本番では `ActiveGhost` は常に置き場の実行系に対応する。問い手は `ActiveGhost.root` でなく `runtime.mount().shiori.dir`（シェルではなく SHIORI のフォルダ）から組む必要がある。
7. **起動直後の短い窓。** `publish_ghost_statics` は投函だけで、反映は sylphya のアクターが後でする（最初の会話の前は prefetch の sink の `barrier` で揃う）。置き場に実行系が入った直後のごく短い間に `baseware.name` を聞くと `NotFound` になりうる。実用上は MCP の呼び出しが届く頃には反映済み。
8. **答えの記録は既にある。** 橋（`crates/areka-mcp/src/tools/bridge.rs` の `ReplyTo::send`）が `tool`・`ghost`・`is_error`・`text` の `debug!` を 1 行残す。処理側で普通の答えに記録を足さない（要件 Boundary）。
9. **SSP の実測との突き合わせ**（survey §7.1）: 空文字・`baseware..name`・`baseware.name.`・前後に空白・`no.such.thing` はどれも areka でも `NotFound`（`parse_dotted` の誤り、または鏡像に無い）＝同じ文言になる。違うのは英字の大小（注 1）と数字の括弧の読み方（`property-catalog-lists` の持ち物）だけ。

## 4. 実装の選択肢

### 選択肢 A: 処理を `handle` 1 本に書き、テストはすべて本物の実行系（`SwitchRig`）で通す

- `runtime.rs`: `pub fn sylphya_reader(&self) -> &SylphyaReader` を 1 本。
- `get_property.rs`: 置き場から実行系を引く → 無ければ `warn!`＋`NG:Property system is not available` → あれば問い手を組んで `resolve_dotted_str` → 2 択を写して `reply.send`。
- テスト: ⑷ は空の World。⑴⑵⑶⑸ は `SwitchRig` で起こし、`set`／`publish_static`＋`barrier` で値を作ってから `handle` を呼ぶ。
- ✅ 本番と同じ配線（置き場→実行系→`mount().shiori.dir`→問い手）を全部踏む＝「檻は到達する経路を踏ませよ」に沿う。
- ✅ 本番のコードは最短（関数 1 本）。
- ❌ `SwitchRig` は emo2 の検体の複製・COM・ゴーストの起動と降ろしを伴い、1 本ごとに重い（`mcp_tests.rs` の既存 2 本と同じ重さ）。4 本立てると時間が延びる（1 本にまとめれば軽くなるが、1 つのテストに分岐を 4 つ詰めることになる）。

### 選択肢 B: 判断を純粋な関数に切り出し、作った鏡像で試す（本物の実行系は使わない）

- `get_property.rs` に、例えば「読み手・問い手（またはその元の SHIORI のフォルダ）・名前 → 答え」の純粋な関数を置き、`handle` は置き場から引いてそれを呼ぶだけの薄い配線にする。
- テスト: ⑴⑵⑶⑸ は `MirrorImage` を手で組んだ読み手で、⑷ は空の World。
- ✅ 速く、決定論が強い。Windows の重い土台が要らない。
- ✅ 関数の引数を「SHIORI のフォルダ」にして中で `ghost_asker_id` を呼べば、⑸ の「そのゴーストの問い手を使う」の規則まで純粋に檻に入る。
- ❌ 「置き場の実行系の `mount().shiori.dir` を渡す」という配線そのもの（例えば誤って `ActiveGhost.root` を渡す間違い）は檻に入らない。
- ❌ 引数が問い手そのものだと、⑸ は sylphya の「ゴーストごと→全体」の順を試すだけになり、`areka-sylphya` の既存テストの重ね撃ちになる。

### 選択肢 C: B の純粋な関数＋本物の実行系のテスト 1 本（混合）

- ⑴⑵⑶ と ⑸ の規則は B の純粋なテスト、⑷ は空の World。加えて `SwitchRig` で 1 本だけ、「本物の実行系のゴーストの問い手に載せた値が返り、別の問い手の値は返らない」を通す（⑸ の配線の檻）。
- ✅ 判断の分岐は速く固定し、配線の要所（問い手を組む元）だけ本物で踏む。
- ❌ テストが 2 系統になり、読み手に「純粋な関数と `handle` のどちらで何を固定しているか」を説明する手間が要る。
- ❌ 純粋な関数を `get_property.rs` に置くか子モジュールにするか（要件 5.1 は子モジュールなら可）を決める必要がある。

### 比べ

| 観点 | A | B | C |
|---|---|---|---|
| 本番の配線を踏む | 全部 | 踏まない | 要所だけ |
| テストの重さ | 重い | 軽い | 中 |
| 本番のコードの量 | 最小 | 関数 1 本ふえる | 関数 1 本ふえる |
| ⑸ の檻の強さ | 強い | 規則まで（配線は外） | 強い |
| 触るファイル（要件 5.1） | 3 つ | 3 つ | 3 つ |

どれを選んでも触るファイルは要件 5.1 の 3 つで閉じる。

## 5. 規模と危険

- **規模: S**（3〜5 タスク: 読み口 1 本／処理の中身／テストの書き換えと追加／実機確認）。既存の型をなぞるだけで、新しい依存も新しい仕組みも無い。
- **危険: 低**。読み手は同期・待ち無しで、UI スレッドを止める道が無い。触る共有の境界は `runtime.rs` の読み口 1 本で、同じウェーブ C3 の他の spec に `runtime.rs` を触るものは居ない（roadmap の C3 の行。`runtime.rs` を触る `property-query-channels`・`makoto-dll-host` は別の系列で C3 に居ない）。

## 6. 設計の段で決めること（要件ディスカッションへの申し送り）

1. **テストの組み方**（4 節の A／B／C）。特に要件 4.1⑸「問い手の選び方」を、本物の実行系（`SwitchRig`）で踏ませるか、純粋な関数の規則で済ませるか。
2. **読み口の形**。`sylphya_reader(&self) -> &SylphyaReader`（借りる）か、複製を返すか。名前は既存の `sylphya_publisher()` に合わせる案が自然。問い手は `mount()` から組めるので読み口は 1 本で足りる（要件 5.2）。
3. **`ActiveGhost` と置き場の実行系を突き合わせるか**。本番では置き場は 1 つで常に一致する。突き合わせ（例: ルートフォルダの一致）を足すと本番で来ない分岐が 1 つ増える。足さないなら `handle` は `ghost` を記録（`warn!` の欄）にだけ使う。
4. **要件 3.1 の `warn!` の形**。欄の名前（宛先のゴースト＝`listed_value` 相当か `root` か・渡された名前＝`property_name`）と、`event` の欄を付けるか（`ghost_switch_notice_tests.rs` は `event` の欄で絞る型）。テストは `log_capture_kit::capture` で数える。
5. **英字の大小の食い違い（注 1）** → 要件ディスカッション 議題 1 で開発者が起票を裁定し、`property-name-case-fold` を起票した（2026-10-04）。本 spec は大小の扱いをテストで固定せず（要件 4.7）、設計と完了のときにその brief へ申し送りを書く（要件 5.6）。
6. **`areka.*`（永続の内部の名前）が MCP から読めること** → 要件ディスカッションで要件の Boundary（Adjacent expectations）に「隠す仕組みは足さない」と明記して閉じた（設計で扱う分岐なし）。

## 7. 調べが要る点（Research Needed）

- **実機確認の段取り**: SSP が動いていると 9801/9821 を取り合う（早い者勝ち）。前例どおり SSP を止めて、配布形の `areka.exe` を `target\` の下に展開して確かめるのが安全。emo2 の descript の `name` は前例で `えも？？` と出ている（`ghost_name` を渡すなら確かめ直す）。
- **起動直後の短い窓**（3 節の 7）: 実機確認は起動が落ち着いてから（会話の始まりの後）呼べば足りる見込み。設計で「起動の直後に `NotFound` になりうる」を書き残すかどうか。
- **空の本文の届き方**: `outcome::value("")` は空の text を 1 つ持つ。SSP の実測（idle の `currentghost.status`）は「空の本文・`isError: false`」で、Claude Code 側の見え方に差が無いかは、値が空の名前が本番に無いため実機では確かめられない（決定論テストで答えの形だけ固定する）。

## 8. 要件ディスカッションで足した設計の申し送り（2026-10-04）

1. **実機確認で SSP と並べるか止めるか**。SSP は 9801 と 9821 の両方を握るので、SSP を動かしたまま areka を起こすと、areka は隣の口（+1〜+9）へ逃げる（`mcp-server-core` の早い者勝ちの規則）。その場合は `claude mcp add` に実際の口を渡す。止めて 9801 を取らせる前例の手順と、どちらで採るかを設計の検証計画で決める。実機の根は `target\` の下だけ。
2. **起動直後の短い窓**（7 節）を、設計の「既知の振る舞い」に書き残すか。実機確認の手順は「会話の始まりの後に呼ぶ」で足りる見込み。
3. **`ghost_name` の照合の食い違い**（survey §7.4・大小・本体側名・前後の空白・空文字）→ 要件ディスカッション 議題 2 で開発者が起票を裁定し、`mcp-ghost-name-match` を起票した（2026-10-04）。本 spec のテストは照合を固定せず（要件 4.8）、実装の最終段階でその brief へ申し送りを書く（要件 5.7）。設計では、処理が `ActiveGhost` を何に使うか（6 節の 3）を申し送りに書ける形で決める。
