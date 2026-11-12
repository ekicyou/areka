# ギャップ分析: audit-sylphya-actor

> 2026-11-06・枝 `claude/kiro-start-audit-sylphya-actor-1becb5`（HEAD `8178f7f7`＝`main` の `73c5b25d` に spec の初期化のコミットを 1 つ足しただけ。`git merge-base HEAD origin/main` が `73c5b25d` と一致＝今の `main` を取り込み済み）。
> 物差しは ponytail のプラグインの `ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.3/skills/ponytail-audit/SKILL.md`・起票の 5.3.2 から上がっているが、見る項目と 3 つの組は同じ）。
> 作法のうち 2 つを外した（要件 1.3・1.4）: 「最大 20 件」は外して全件を載せる。「読まなかった所」は残さない（範囲の 46 ファイルを全部、1 行目から最後の行まで読んだ）。
> 読み方: 範囲が小さいので読み手を分けず、1 人で全部のファイルを読んだ。載せた指摘はどれもファイルと行を開いて確かめ、外から使われているかはツリー全体を grep して確かめた（文字列での参照・ほかのクレートがソースの本文を読む検査も含む）。実機でしか確かめられないものは、その旨を書いた。

## 1. 範囲の測り直し（要件 1.2）

| | 起票のとき（main `e3c2cbe4`） | 棚卸㉟（main `18e4cb30`） | 今（`73c5b25d`） |
|---|---|---|---|
| ファイルの数 | 36 | 46 | **46**（Rust 43＋`Cargo.toml` 3。fixture のファイルは無い） |
| コードの行（名前の決まり） | 6,447 | 約 6,900（檻を含む） | **6,368**（22 ファイル） |
| テストの行（名前の決まり） | 3,190 | — | **5,498**（21 ファイル・`_tests.rs`／`_cage.rs`／`tests/`／`tests.rs`／`test_log_capture.rs`） |
| `Cargo.toml` | — | — | 64 行 |
| 合計 | — | — | 11,930 行 |

- クレートごと: `areka-sylphya` 31 ファイル（コード 5,095・テスト 3,790・`Cargo.toml` 28）・`areka-actor` 10（944・1,042・24）・`areka-talk` 5（329・666・12）。
- 数え方の注: コードのファイルの中に `#[cfg(test)] mod tests` が 2,169 行ある（最初の `#[cfg(test)]` から末尾まで）。これをテストに回すと、本番のコードは約 4,200 行・テストは約 7,670 行。
- 1,000 行の近く: `persist/persist_tests.rs` 879・`actor.rs` 841。足すなら新しいファイルへ（要件 5.1）。
- 棚卸㉟の後に範囲のファイルは変わっていない（`18e4cb30..73c5b25d` の差分は roadmap だけ）。

## 2. この範囲がしていること・想定した負荷

- **何をするか**: `areka-sylphya` は「名前で引ける値」の掲示板。書き手のアクター 1 本が写し（`MirrorImage`）を作り直して差し替え、読み手（`SylphyaReader`）は待たずに今の写しを読む。窓の位置・最後に使ったゴーストなどの記憶を `profile\areka\sylphya.toml`（TOML）に、読んで重ねて書く。`areka-actor` はアクターの土台（名前付きスレッド・返信の口・UI スレッドのポンプへ届ける橋）。`areka-talk` は kanade と sakura の間で渡すトークの約束の型と、台本の出どころの語彙。
- **想定した負荷**: 利用者は 1 人・ゴーストは 1〜数体。記憶のファイルは利用者が手で直すことがあり、版の違う areka（上げた後に戻す）が同じファイルを読むことがある。areka のプロセスは同時に 2 つ以上動きうる（多重起動を止める仕組みはツリーに無い・`audit-ghost-catalog` の調べと同じ）。UI スレッドは 1 本で、その上の記録（ログ）は実機の確かめの主な証拠になる。

## 3. 指摘の一覧（全 22 件）

組の名は audit の 3 つ: **Must fix（必ず直す）**・**Should fix（直したほうがよい）**・**Nice to have（できれば）**。番号は 3 つの組を通して 1 から。

- 数: **Must fix 2 件・Should fix 8 件・Nice to have 12 件**。
- 「公開の形・範囲外」の欄: 直すと 3 つのクレートの公開の形（範囲の外から使われている名前・型・関数の形）が変わるか、範囲の外のファイルに手が及ぶか（要件 1.6）。「C18 で止まる」は要件 6.1〜6.3 の約束を破らないと直せないもの。
- 「残す候補」の欄: 直さずに残してよい 3 種類（⑴ 開発者の裁定・完了した spec の要件に反する直し・⑵ わざと置いた先送りの印・⑶ SSP／ukadoc の正典に関わる振る舞いの変更）に当たるかもしれないもの。決めるのは要件の討議（要件 2.2）。「—」は当たらない。
- ⑶ に当たりそうな指摘は無かった（どの直しも areka の中の保存・記録・テストの話で、ゴーストから見える正典の振る舞いを変えない）。そのため ukadoc は引いていない。

### 3.1 Must fix（2 件）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 1 | `persist/mod.rs:282`（説明）・`:309-313`・`:351-367`（`read_existing`）・`persist/format.rs:235-263`（`read_toml_str`） | **読めなかった記憶のファイルを、読めた分だけで上書きして消す。** 保存は「今のファイルを読んで、足して、書く」。ところが今のファイルが ⑴ 読めない（ウイルス対策の一時の掴み・権限）⑵ TOML として壊れている（手で直したときの打ち間違い）⑶ `format-version` が 1 でない（新しい areka で書いた後に古い areka へ戻した）とき、読んだ結果を「空」とみなして続け、書き込む 1 件だけのファイルで置き換える。起動のたびに起動中の印（`write_session_mark`）を App スコープへ書くので、戻した最初の起動で、最後に使ったゴースト・根の並びが消える。Ghost スコープも同じで、窓の位置・最後のバルーン／シェルが消える。`format.rs` の記録は「file preserved」と言うが、保存の道がそれを破る | `read_existing` が「無い」と「在るが使えない」を分ける。使えないときは書かずに `PersistOutcome::Degraded(PersistFailure { cause: PersistFailureCause::Io(理由の文) , .. })` を返す（既存の語彙のまま・新しい variant は足さない）。`format.rs` に「版が合い、読めたか」を返す crate 内の関数を足し、`read_toml_str` はそれを包む。縛るテスト: `FakePersistIo` に `format-version = 2` の中身・壊れた TOML・`fail_next_read` を置いて `save_scope` し、中身が変わらず `Degraded` になること | 公開の形は変えない（`PersistFailureCause` に variant を足すと `crates/areka/src/alert.rs:223`・`write_failure.rs:56` の網羅の match が壊れる＝C18 で止まるので足さない）。範囲の外のコードに手は及ばないが、利用者には書けなかったことの知らせ（`alert.rs`）が出るようになる | ⑴ の疑いは薄い（完了した `areka-P0-sylphya` の要件 6.3 は「読めないときは不在として寛容に縮退し、起動を妨げない」で、上書きは求めていない。要件 6.4 は旧形式を判別できること）。読めない物を「書かずに残す」か「別の名前へ退けてから書く」かは討議で決める |
| 2 | `areka-actor/src/ui.rs:99-123`（`spawn_ui` の受信のタスク） | **UI アクターの記録の区間が、待ちをまたいで入りっぱなしになる。** タスクの頭で `span.enter()` し、その取っ手を持ったまま `rx.recv().await` で待つ。待ちで UI スレッドへ制御を返しても区間は「入った」ままなので、その後に UI スレッドで出るほかの記録（窓の処理・ほかのタスク）が全部 `actor{actor="emo-text"}` の区間の中に見える。`tracing` の説明が名指しで禁じている使い方。実機の確かめは記録を読んで判断するので、どのアクターの記録かを取り違える | `async move { .. }.instrument(span)`（`tracing::Instrument`・`tracing` は既に依存＝依存は増えない）にし、`_enter` を消す。区間は poll の間だけ入る | 公開の形は変えない・範囲の外に手は及ばない。縛るテストの作り方は調べが要る（§9 R1・`log-capture-kit` の捕捉は区間を運ばない） | — |

### 3.2 Should fix（8 件）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 3 | `persist/io.rs:67-80`（`temp_path_for`）・`:127-148`（`commit`）・`persist/mod.rs:309-318` | **2 つの書き手が同じ記憶のファイルを同時に書くと、片方の更新が消えるか中身が混ざる。** 一時ファイルの名前が `sylphya.toml.tmp` に決め打ちで、読んで重ねて書く間に錠が無い。areka が 2 つ同時に動き、両方が App スコープを書くと（起動中の印・最後のゴースト・根の並び）、⑴ 後から書いた方が先の方の更新を消す ⑵ A の `File::create` の後に B が同じ一時ファイルを作り直して切り詰め、A の `rename` が B の書きかけを本体へ移す＝壊れた TOML になり、次の読みで全部「不在」になる（#1 と重なると全部消える）。注は「M1 は単一プロセス前提・競合が要るならロックを入れる」と限界を書いている | 書き手の間を錠で 1 本ずつにする（`std` の `OpenOptionsExt::share_mode(0)` で `sylphya.toml.lock` を作ってから読み・書き・確定まで持つ＝`audit-ghost-catalog` の #1 と同じ形・依存は増えない）。一時の名前にプロセス番号を混ぜる案は、要件 8.3 の読み方（注の「PID/時刻を混ぜない」）に触れる | 公開の形は変えない。`io.rs` の一時のパスの入口（`TempPath`・`temp_path_guard_test.rs` が見る物）は足さず変えない（要件 6.3）＝本番の `temp_path_for` は入口ではない | ⑴（完了した `areka-P0-sylphya` の要件 8.3「非決定な外部環境を暗黙に直読しない」を注が「一時の名前に PID を混ぜない」と読んでいる）・⑵（注に限界を書いた先送り。今の負荷＝2 つのプロセスが同時に書くことが本当に起きるかは実機で確かめていない・§9 R2）。v0.2.0 の `sylphya-sqlite-kvs` が「アプリ層の書き手が何人でも消えない」を受け持つので、そこへ回す案もある（要件 6.4・6.5） |
| 4 | `actor.rs:532-549`（`SylphyaPublisher::barrier`） | **反映を待つ口に期限が無い。** 説明は「有界: 呼び側がタイムアウトを課す」と言うが、課す手段が無い（中は `rx.recv()` の無期限の待ち）。`crates/areka/src/install/desk.rs:377-378` は UI スレッド（ECS の World）から呼ぶので、書き手のアクターが遅い書き込み（`sync_all` の最中・ウイルス対策・遅いドライブ）にいる間、UI が止まる | `barrier_within(timeout: Duration) -> Result<(), ReplyError>` を足す（`recv_timeout`・期限切れは `ReplyError::Timeout`・`warn!`）。説明の「呼び側が課す」を本当の形に直す。呼び手の乗り換えは範囲の外 | 足すだけ＝C18 の中で直せる。呼び手（`crates/areka/src/install/desk.rs`・`areka-ghost/src/sylphya_wiring.rs:179`・`crates/areka/src/shiori_host.rs`）の乗り換えは範囲の外（`crates/areka` は C18 で触れない）＝後の spec へ回すかを決める | — |
| 5 | `areka-actor/src/ui.rs:103-122`（受信のループの 3 つの枝） | **UI アクターの「処理の失敗でも止まらない」「送り手が全部いなくなったら終わる」の 2 つの枝に、落ちるテストが無い。** `ui.rs` の単体テストは `UiSender` だけを試し、実走のポンプの試験（`tests/toy_ui_pump_test.rs`）は `Continue` と `Break` しか通らない。`Err` を返したら止めてしまう直し間違いや、`Err(Closed)` で回り続ける直し間違いが緑のまま通る | `toy_ui_pump_test.rs` に 1 本足す: 1 件目で `Err` を返し、2 件目の返信が届くこと。全 `UiSender` を落としたらタスクが終わること（`JoinHandle` で観測） | 公開の形は変えない・範囲の外に手は及ばない | — |
| 6 | `areka-sylphya/src/ledger_key_determinism_tests.rs:1-381`（全体）・`lib.rs:77-79`・`vocab/dotted.rs:296-355` | **同じことを確かめるテストが 2〜3 か所にある。** このファイルは自分の頭の説明（`:3-8`）で「個々の不変量は各生産ファイルの tests でも檻化済み」と認め、`key.rs` の `form1〜5`・`err_*`・`determinism_*`、`flat.rs`・`dotted.rs`・`shiori_resource.rs`・`vocab/mod.rs` の件数と中身の試験を写している。SET 有効群の 25 件の手書きの並びは `dotted.rs:317-348` と `ledger_key_determinism_tests.rs:289-316` の 2 か所にあり、表を直すたびに 2 か所を同じに直す。`criterion_backing_layer_five_taxonomy_exists` は配列の字面の長さが 5 かを見るだけで、落ちようがない | ファイルごと消し、写しでない 1 件（ext のイベント名がどの解決の表にも入らない＝`:398-419`）だけを `dotted.rs` の tests へ移す。`lib.rs` の `mod` の行も消す | 公開の形は変えない・範囲の外に手は及ばない（ツリーにこのファイル名を名指す物は無い） | ⑴（完了した `areka-P0-sylphya` のタスク 2.5 の成果物が「criterion ごとに 1 群として在ること」だった） |
| 7 | `areka-talk/src/tests.rs:12-481` | **ほとんどのテストが、コンパイラが作る派生（`Copy`・`Clone`・`PartialEq`・`Hash`）と、構造体に入れた値が出てくることを確かめるだけ。** 例: `talk_command_single_stream_preserves_batch_order`（`:352`）は `Vec` の中の順を見るだけでチャネルを通らない。`choice_waiting_timeout_directive_vocabulary_is_representable`（`:429`）はテストの中で定義した関数を試している。どれも壊しようがなく、型の欄を変えると何十行も直す手間だけが残る | 意味のある 3 本（`StartTalk::new` の既定＝`:72`・`:82`・`:90`）を残して消す（約 400 行） | 公開の形は変えない・範囲の外に手は及ばない | ⑴ の疑いは薄い（完了した spec の要件は「型を 1 つにまとめる」で、派生の試験そのものは求めていない） |
| 8 | `asker.rs:31-65`・`value.rs:41-83`・`vocab/mod.rs:56-96`・`key.rs:358-371`・`reader.rs:438-464`・`mirror.rs:128-136`・`:240-248`・`actor_tests.rs:42-51`・`:326-390`・`persist/format.rs:505-529`・`persist/io.rs:332-343`・`persist/persist_tests.rs:519-552` | **落ちようのないテストが sylphya にも多い。** 派生の `Eq`／`Ord` を確かめるもの、純粋な関数を同じ引数で 2 回呼んで同じかを見るもの（`determinism_*`・`apply_is_deterministic_across_variants`）、`Send` を型で確かめるだけのもの（`runtime_command_sink_trait_is_reserved`）、空の像が空か（`empty_image_has_all_partitions_empty`）。直し間違いを 1 つも捕まえない | 消す（約 250 行）。落ちうる中身を持つもの（例 `reader.rs` の `determinism_dotted_value_present_stable` は値のある枝を見る）は残すか、近くの意味のあるテストへ畳む | 公開の形は変えない・範囲の外に手は及ばない | ⑴（完了した `areka-P0-sylphya` の要件 2.5・9.1「決定論の檻」の criterion に紐づけて置かれている） |
| 9 | `areka-actor/tests/spike_ui_pump.rs:1-173` | **一度きりの確かめ（スパイク）が、置き換わった後も残っている。** 自分の説明（`:31-32`）で「後続の `spawn_ui`・toy(b) は PRIMARY 組合せを確定してよい」と結論を出し終えている。今は `tests/toy_ui_pump_test.rs` が本物の `spawn_ui` で同じ往復を試す。ポンプと心拍のコードも 2 つのファイルで写し合っている | ファイルを消す | 公開の形は変えない。ツリーでこのファイルを名指すのは完了した spec の文書だけ（`completed/areka-P0-actor-foundation/`・直さない） | ⑴（完了した `areka-P0-actor-foundation` の要件 8.2・8.3 の成果物だった） |
| 10 | `actor.rs:107-115`（`RuntimeCommandSink`）・`:468-469`（`SylphyaInit::runtime_sink`）・`:717-723` | **本番で一度も使われない差し替えの口。** ツリーの `runtime_sink:` は範囲の外に 32 か所あり、全部 `None`。`Some` を入れるのは自分のテストの `PanicSink`（アクターを panic させる道具）だけ。`RuntimeCommandSink` を実装する型は範囲の外に 0 | 型と欄を消し、`Effect::RuntimeCommandReserved` の配線の枝を `warn!` 済みの何もしない枝にする。panic の檻は別の道具（例 `PersistIo` の panic する偽物）で作り直す | **C18 で止まる**（`SylphyaInit` の欄を消すと範囲の外の 32 か所の構造体の字面が壊れる。うち `crates/areka` に多数） | ⑵（説明に「M1 は型予約のみ・α 後に seriko 等へ橋渡し」と理由を書いた予約の口） |

### 3.3 Nice to have（12 件）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 11 | `areka-actor/tests/toy_worker_test.rs:158-181`・`:62-244` | `panicking_body_join_is_panicked` が**プロセス全体の panic の受け手**を外してから戻す。同じテストのバイナリで並んで走るほかのテストがその間に落ちると、落ちた理由の文が出ない。ほかの 5 本も `spawn.rs` の単体テスト（`:274-388`）と同じこと（Break・切断・`Err` の後も続く・panic の join）を確かめる写し | panic の受け手の付け外しを消す（無くても結果は同じ）。`spawn.rs` の単体テストと重なる分は片方に寄せる（公開の口だけで試す toy(a) の役目を残すなら、`spawn.rs` 側を消す） | 公開の形は変えない・範囲の外に手は及ばない | ⑴（toy(a) は完了した `areka-P0-actor-foundation` の要件 8.1 の成果物） |
| 12 | `areka-sylphya/src/test_log_capture.rs:1-85` | 共有の道具 `log_capture_kit::capture` の結果を、中身の同じ自前の型へ写し直すだけの包み。同じクレートの `persist_failure_tests.rs`・`withhold_tests.rs` は道具を直に使う＝捕捉の書き方が 1 つのクレートに 2 つある | 包みを消し、`assert_logged` だけを 10 行ほどの関数として残すか、`log_capture_kit::CapturedEvent` を直に使う | 公開の形は変えない（`pub(crate)`）。`crates/log-capture-kit/src/event.rs:98`・`event_tests.rs:335` の注が「`areka-sylphya` の `assert_logged`」を名指す＝消すならその注が古くなる（範囲の外・`audit-test-kits` の持ち物） | — |
| 13 | `actor_actor_integration_tests.rs:210-224`（`SharedFakeIo`） | `impl PersistIo for Arc<T>`（`persist/io.rs:48-58`・`audit-ghost-catalog` が足した）があるので、この包みは要らない | `Arc<FakePersistIo>` を直に `Box::new` で渡す | 公開の形は変えない。範囲の外の同じ包み 3 つ（`crates/areka/src/boot_resolve_tests.rs:422` ほか・brief の記録）は本体の見直しの持ち物＝触らない | — |
| 14 | `actor_actor_criteria_cage.rs:17-26`（`spawn_with_mirror_observer`） | 本番の起動の配線（初期像→`SharedMirror`→`spawn_actor`→`run_actor`）をテストの中で写している。本番の `spawn_sylphya_with_failures` を直しても、この檻は写しを試し続ける | `spawn_sylphya_with_failures` の中身を「部品と `SharedMirror` を返す」crate 内の関数へ移し、本番と檻の両方がそれを呼ぶ | 公開の形は変えない | — |
| 15 | `actor.rs:88-96`（`PersistPut::reply`）・`:747-765` | 外から使えない返信の欄。送り口 `tx` は非公開で、`reply: Some` を送る公開の関数が無い（`persist_put` は常に `None`）。使うのは自分のテストだけ。`None` を返す `error!` の枝（`:755-763`）は起きない | 欄と枝を消す。テスト（`actor_failure_sink_tests.rs:322-345`）は失敗の受け口か `load_scope` で結果を見る | `SylphyaMsg::PersistPut` は範囲の外で 0 か所＝消してよい（要件 6.1 の「外から使われている名前」に当たらない） | — |
| 16 | `areka-actor/src/ui.rs:64-70`・`:165-177`（`UiSpawnError::NotUiThread`） | 返されることの無いエラー。`spawn_ui` はいつも `Ok` を返すのに、呼び手は起きない `Err` を扱う（`crates/areka/src/emo2_boot/mod.rs:220` の `SpawnUi`・`areka-emo-text/src/actor.rs:655`） | 戻り値を `(UiSender, JoinHandle)` にし、エラー型を消す | **C18 で止まる**（`spawn_ui` の署名と `UiSpawnError` を範囲の外の `areka-emo-text`・`crates/areka` が使う） | ⑵（説明に「検出できた場合の戻り値として予約」と理由を書いた予約の口） |
| 17 | `mirror.rs:38-39`（`MirrorImage::flat_global`）・`reader.rs:70`・`:130` | 本番で誰も書かない区画。アクターの効果に書き込む物が無く、読みは毎回空の表を引き、トークの写しは毎回空の表を複製する | 欄と、読みの 2 か所の引きを消す | 範囲の外で `flat_global` を使う所は 0（範囲の外は `MirrorImage::empty()` と `dotted_global` だけを使う）＝消してよい | ⑵（説明に「将来の大域フラット語彙用に確保」と理由を書いた区画） |
| 18 | `vocab/flat.rs:107-118`（`FlatEntry::layer`・`m1`）・`vocab/mod.rs:7-54`（`BackingLayer`・`M1Status`・`DegradePolicy::NotFound`・`SetSemantics`）・`vocab/dotted.rs:102-136` | 本番のコードが一度も読まない語彙の欄。`layer`・`m1` を読むのはテストだけ、`DegradePolicy::NotFound` を作る道は無い、`SET_EFFECTIVE` の 2 番目の要素は 25 件全部 `RuntimeCommand`（テストがそう固定する）で情報を運ばない | 読まない欄と variant を消し、`SET_EFFECTIVE` を名前だけの表にする | **範囲の外がソースの本文を読む**: `crates/ukadoc-survey` の証拠の抜き出しが `vocab/` の表の形（`&[(&str, SetSemantics)]`・`&[FlatEntry]`）を名指し（`evidence/resolve.rs:25`）、`crates/areka/src/placement/zorder_property_deferral_tests.rs` が `vocab/` の公開の const の名前を文字列で数える。const の名前は変えず、欄だけを消すなら両方とも壊れない見込み（設計で確かめる） | ⑵（語彙完備＝「語彙は全部持ち、配線は 0 から追う」の実装規律で置いた語彙） |
| 19 | `persist/format.rs:105-121`（`FormatDoc::is_all_absent`） | テストからしか呼ばれない公開の関数（範囲の外で 0） | `#[cfg(test)]` の補助へ移すか、テストで `== FormatDoc::default()` と比べて消す | 範囲の外で 0＝消してよい | — |
| 20 | `actor.rs:665`（`run_actor` の `shared.load().successor()`） | 知らせ 1 通ごとに、写しの 4 つの表を全部複製する。書き換えの無い知らせ（反映の待ち・停止・書けない名前の SET・204 の記録）でも複製して捨てる。反映の待ちは SET や記録のたびに来る | 効果に書き換えが 1 つも無ければ複製しない（`effects` を先に見る） | 公開の形は変えない | — |
| 21 | `actor.rs:260-271`（`SylphyaCore`） | 状態を持たない型と `new()`。`apply` は自由な関数で足りる | `pub fn apply(msg: &SylphyaMsg) -> Vec<Effect>` にする | **C18 で止まる**（`SylphyaCore::new().apply(..)` を `crates/areka/src/placement/zorder_property_deferral_tests.rs:245` が使う） | — |
| 22 | `key.rs:124-128`（`parse_seg`） | 閉じた括弧の後ろに文字が続く名前（`scope(0)x`）を「括弧が閉じていない」（`UnclosedParen`）と報告する。記録の誤りの種類が実物と食い違い、手掛かりを取り違える | 末尾が `)` でなくても `(` の後ろに `)` が在れば別の種類で報告する（`KeyParseError` に variant を足す・範囲の外で 0 か所＝足してよい）か、`EmptySegment` と同じく位置だけを正しく言う既存の variant に寄せる | `KeyParseError` は範囲の外で 0 か所 | — |

- 消せる見込み（Lean）: 約 −1,300 行（#6 約 380・#7 約 400・#8 約 250・#9 173・#11・#12・#13・#15・#17〜#19 の小さな分）、依存は −0。

## 4. 指摘にしなかったもの（確かめた結果）

- **SET の仕分けの 2 つの癖**（`classify_set` が葉の名前だけで正準語彙とみなす・`SET_EFFECTIVE` が末尾形で実キーと突き合わされない＝`vocab/dotted.rs:153-169`）: 正典に根拠が無いことを記録し、引受先（`property-query-channels`・brief の実在を確かめた）を名指している＝わざと置いた先送り。今の負荷で困る場面（`\![set,property]` の口）はまだ開いていない。
- **括弧の中に `.` を含む名前は読めない**（`key.rs:21`・`ghostlist(Ver.2).name` は `UnclosedParen`）: 設計の決め（単純な分割）として明記。今の読み手（MCP `get_property`）が引く `ghostlist(...)` の値は sylphya に無く、結果はどちらにしても `NotFound`。
- **SHIORI の照会が 204 に変わっても前の値が残る**（`PublishShiori { value: None }` は消さない）: 照会は起動の 1 回だけ（`areka-kanade/src/schedule/boot.rs:75-88`）で、同じ書き手で照会し直す道が無い＝起きる場面が無い。
- **起動中の印の空文字**: 写しには `""` が載り、ファイルでは「無し」になるが、印を写し経由で書くのは非空の名前だけ（消すのは `boot_resolve.rs` の直の `save_scope`）＝食い違いが見える場面が無い。
- **SET の自由な名前が増え続ける**: 書く口（`ShioriHostSink::SetProperty`）は本番のゴーストの道に無い（デモとテストだけ・完了した `areka-P0-property-name-case-fold` の要件 36 行目）。
- **ソースの本文を読む 2 本のテスト**（`dotted_set_ledger_tests.rs:204-248`）: 1 本は完了した `areka-P0-sylphya-set-ledger` の要件 3.3／7.2 の決め、もう 1 本は `crates/ukadoc-survey` が読む `// ukadoc:` の注の数の約束＝本当の契約を守っている。
- **`FakePersistIo` が本番のクレートで `pub`**: 範囲の外の 21 ファイル（`areka-ghost`・`crates/areka` のテスト）が使う＝`#[cfg(test)]` にできない。
- **`spawn_actor` がスレッドを作れないと panic する**（`spawn.rs:133-139`）: 記録を残してから落ちる、と説明に書いた決め（致命に限る panic の規律どおり）。
- **`withhold.rs` の鍵の小文字化**: 本番は「全部控える」の 1 本道で、根の判定はテストの中だけ。
- `#[allow(dead_code)]`・`shortcut:`・`ponytail:` の印は範囲に 1 つも無い（grep で確かめた）。⑵ の候補は、理由を説明に書いた予約の口（#3・#10・#16・#17・#18）だけ。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今の資産 | ギャップ |
|---|---|---|
| 1（洗い出し） | 本書 §1・§3 | 済み（全 46 ファイルを読んだ・22 件） |
| 2（討議で確定） | 本書 §3 の「残す候補」と §8 の議題の種 | 討議で決める: ⑴ の候補 7 件・⑵ の候補 5 件・C18 で止まる 3 件（**Constraint**） |
| 3（直し切る） | 範囲の 3 クレート | #1〜#22 の直し（**Missing**）。呼び手の乗り換え（#4）は範囲の外（**Constraint**） |
| 4（テストで縛る） | `FakePersistIo`（故障の注入）・`log_capture_kit`・`toy_ui_pump_test.rs` の実走のポンプ | #1 の縛り（**Missing**）・#2 の縛りの作り方（**Unknown**＝区間を見る道具が無い）・#5（**Missing**） |
| 5（守る決まり） | 1,000 行の番人・`error!`＋`Err`・依存 0 | `actor.rs` 841 行＝足すなら新しいファイルへ（**Constraint**）。#2・#3 は依存を足さずに直せる |
| 6（C18 の約束） | 棚卸㉟の約束 ⑴・⑤ | #10・#16・#21 は公開の形を変えないと直せない（**Constraint**）。#3 は `io.rs` の一時の入口に触れない形で直す |
| 7（仕上げ） | `tools/test-all.ps1` | かけ直しの audit と既定の枝との差分の確かめ（実装の後） |

- 範囲の外のファイルが範囲の文を当てにしている所（直すときに壊さない）: ⑴ `doc/ukadoc-coverage/ledger/property.toml` が `actor.rs:345-350`・`reader.rs:158-164` の行番号と記録の文（`"SET on non-settable canonical vocab: ..."`・`"dotted resolution not found"`）を引く（行番号は既に古い＝今は `actor.rs:375-380`・`reader.rs:159-165`・持ち主は `audit-ukadoc-survey`）＝記録の文と宛先は変えない ⑵ `crates/ukadoc-survey` が `vocab/dotted.rs`・`vocab/shiori_resource.rs`・`lib.rs` の本文を読む ⑶ `vocab/dotted.rs` の `doc/COMPAT_ARCHITECTURE.md` を名指す 2 行（`:64`・`:80`）は変えない（要件 5.7）。

## 6. 1,000 行の番人

- 範囲で 1,000 行を越えるファイルは無い。近いのは `persist/persist_tests.rs` 879・`actor.rs` 841。#1 の縛るテストは `persist_tests.rs` に足すと 950 前後になる見込み＝新しい兄弟のテストファイル（例 `persist_unreadable_tests.rs`）へ置く。

## 7. 実装の進め方の案

| 案 | 中身 | 良い | 悪い |
|---|---|---|---|
| A 今のファイルで直す | 指摘ごとに今のファイルで直す。新しいファイルは #1 のテストの兄弟 1 つだけ | 層と決まりのまま・差分が小さい・消す指摘が多いので行が減る | C18 で止まる 3 件（#10・#16・#21）と #4 の呼び手の乗り換えは、この spec の中では済まない |
| B 止まる分を後のウェーブへ | A に加え、#10・#16・#21 と #4 の呼び手は、C18 の後（本体の crate 分けの後）に同じ範囲で直すか、受け持ちの spec の brief へ件として書き足す（要件 6.4・6.5） | 約束を破らない・この spec は C18 の中で閉じる | 回し先の spec の実在を確かめ、brief に書き足す手間がある |
| C 組ごとに 2 本へ分ける | Must・Should をこの spec、Nice を同じ範囲の 2 本目へ | 危ない所が早く着地する | タスクは 20 に届かない見込み（下）＝分ける理由が弱い |

- 規模の見立て: 大きいタスクで 10〜13 本（#1 と縛るテスト・#2・#3・#4・#5・消すテストの束 #6〜#9・#11〜#15・語彙と区画 #17〜#19・#20・#22・かけ直し）。20 を越えない見込み＝分けなくてよい（要件 2.6）。
- Effort: **M**（直しの多くは消すだけ。#1・#3 は保存の道の振る舞いを変えるので、テストと実機の確かめが要る）。
- Risk: **Medium**（#1・#3 は利用者の記憶の扱いを変える。ほかは Low）。

## 8. 討議に渡す議題の種（要件 2）

1. **#1 読めない記憶のファイルの扱い**: ⒜ 書かずに残し `Degraded` を返す（ファイルは手で直せば戻る・書けなかった知らせが出る） ⒝ 別の名前（例 `sylphya.toml.broken`）へ退けてから新しく書く。⒜ は利用者に知らせが続けて出る、⒝ は名前の決めと退けた物の掃除が要る。どちらも公開の形は変えない。⑴ の疑い（完了した `areka-P0-sylphya` の要件 6.3・6.4）が無いことの確認も。
2. **#3 2 つの書き手の衝突**: ⒜ この spec で錠を入れる ⒝ v0.2.0 の `sylphya-sqlite-kvs`（「アプリ層の書き手が何人でも消えない」を受け持つ）へ回す ⒞ ⑵ として残す。⑴ の候補（要件 8.3 の読み方）・⑵ の候補（注に限界を書いた先送り）。
3. **C18 で止まる 3 件（#10・#16・#21）**: 公開の形を変えないと直せない。⒜ 本体の crate 分けの後に同じ範囲の 2 本目で直す ⒝ ⑵ として残す（#10・#16 は理由を書いた予約の口） ⒞ 後のウェーブの受け持ちの spec の brief へ書き足す（要件 6.4・6.5）。
4. **#4 の呼び手の乗り換え**: 口（`barrier_within`）を足すのはこの spec。UI スレッドの呼び手（`crates/areka/src/install/desk.rs`）の乗り換えは範囲の外＝回し先（本体の見直しの spec のどれか）を決める。
5. **⑴ の候補のテスト（#6・#8・#9・#11）**: 完了した spec の成果物として置かれた檻・スパイク・toy(a)。中身が写しか落ちようがないので消す推奨。完了した spec の要件を覆すことになるかの確認。
6. **⑵ の候補の語彙と区画（#17・#18）**: 語彙完備の規律で置いた欄と区画。消すか残すか。#18 は範囲の外の 2 つの読み手（`ukadoc-survey`・`crates/areka` のテスト）を壊さない形でだけ直せる。

## 8.1 設計に回す事項（要件の討議の仕分け・2026-11-06）

- B1（#2）: 区間の出入りを縛るテストの形（§9 R1）。
- B2（#1）: 書けなかった知らせの理由の文（§9 R3）と、縛るテストを置く新しい兄弟のファイル（§6）。
- B3（#3）: 錠を入れる場合の形（錠のファイルの名前・待つ上限・待ち切れないときの `Degraded`・同じプロセスの中の書き手どうしも 1 本ずつにする）。
- B4（#14・#20・#22・#12）: 本番と檻が同じ組み立てを呼ぶ形・書き換えの無い知らせで写しを作らない判定・`parse_seg` の誤りの種類の選び方・捕捉の包みの消し方。
- B5（#18 を残す場合）: §9 R4 は要らなくなる。消す場合だけ確かめる。

## 9. 調べが要るもの（Research Needed）

- **R1（#2）**: 区間が入りっぱなしでないことを縛るテストの作り方。`log-capture-kit` の捕捉は区間を運ばないので、テストの中で区間の出入りを数える最小の `Subscriber`（`spawn_log_capture_tests.rs` の `NotAWindow` と同じ型の自前の受け手・依存 0）をポンプのスレッドの既定に据える形を設計で確かめる。
- **R2（#3）**: 2 つの areka のプロセスが同じ App スコープを同時に書く道が今あるか（2 つ目の起動の `write_session_mark` と 1 つ目の定常の印・根の並びの書き換え）。実機の確かめは開発者の机でしか取れない。
- **R3（#1）**: 書けなかったときの知らせ（`crates/areka/src/alert.rs` の文）が「読めない記憶を守った」場面でも伝わる文になるか。範囲の外の文は変えられない（C18）ので、`PersistFailureCause::Io` に入れる理由の文で足りるかを設計で確かめる。
- **R4（#18）**: `ukadoc-survey` の抜き出しが `FlatEntry` の欄を減らしても同じ要素を拾うか（「要素ごとの最初の文字列」の規則なので拾う見込み）・`zorder_property_deferral_tests.rs` の公開の const の名簿が変わらないか。

---

## 10. 設計の段の調べ（2026-11-06・`/kiro-spec-design -y`）

- **Discovery の種類**: light（今のクレートの直し・新しい依存なし・外の API の調べなし）。読み手は分けず、設計の 1 人が範囲の該当ファイルと範囲の外の呼び手を開いて確かめた。
- **使った道具・決まり**: `kiro-spec-design` の rules（design-principles・design-discovery-light・design-synthesis・design-review-gate）・ponytail（いちばん小さい完全な変更）・形の手本は完了した `audit-ghost-catalog` の design.md。

### 10.1 設計に回した事項の答え（§8.1）

- **B1・R1（#2）**: 区間の出入りを数える自前の受け手（`tracing::Subscriber` の最小の実装・`enter` で +1・`exit` で −1）をポンプのスレッドの既定に据え、handler の中で 1・ポンプの filter（poll の外）で 0 を見る。前例は `crates/areka-actor/src/spawn_log_capture_tests.rs` の `NotAWindow`・`crates/log-capture-kit` の 6 つの自作の受け手。呼び出し地点の関心がプロセス全体に焼き付く機序（`log-capture-kit` の crate doc）を避けるため、テストは 1 本だけの独り立ちのバイナリ `tests/ui_span_test.rs` に置く。`tracing::Instrument` は `tracing` 0.1.44 の `lib.rs` で無条件に公開されている（`pub use self::instrument::Instrument;`）＝依存も機能の指定も増えない。
- **B2・R3（#1）**: 範囲の外の知らせ（`crates/areka/src/alert.rs`）は `PersistFailureCause::Io(reason)` を「理由: {reason}」の 1 行でそのまま出す。前後の行は「記憶を書けないので、ゴーストを起こさずに起動を止めました。」（起動の前）か「記憶を書けなくなったので、areka を終了しました。」（走っている間・`write_failure.rs` の `fatal`）と「直し方: この置き場を書ける状態にしてください。…」。理由の文に「どのファイルが・なぜ使えないか・中身を消していないこと」を入れれば足りる（文面は design.md の #1 の節）。縛るテストの置き場は `persist/persist_unreadable_tests.rs`（`persist_tests.rs` 879 行に足さない）。
- **B3（#3）**: 錠は 2 段＝プロセスの中の `static SAVE_GATE: Mutex<()>` と、`PersistIo` に既定つきで足す `lock_for_update`（`FsPersistIo` だけが `sylphya.toml.lock` を `share_mode(0)`＋`FILE_FLAG_DELETE_ON_CLOSE` で開く）。待つ上限 1 秒・10 ms ごとに取り直す・取れなければ `error!`＋`Degraded(Io)`。一時の名前 `sylphya.toml.tmp` と `temp_path_for` は変えない。`temp_path_guard_test.rs` の走査語は一時の入口の 2 つの綴り（`concat!` で組んだ語）だけで、錠のパスを宛先から組む形は当たらない（`TempPath` の 3 か所も変えない）。
- **B4（#14・#20・#22・#12）**: #14＝`spawn_parts`（crate 内）を本番と檻が呼ぶ。#20＝後継の像を `Option` で遅らせ、書き換える 4 つの枝だけが作る。#22＝`KeyParseError::TextAfterParen` を足す（範囲の外で `KeyParseError` は 0 か所＝足してよい）。#12＝`test_log_capture.rs` は消さずに共有の型の上の 2 つの関数へ縮める（`crates/log-capture-kit/src/event.rs` の注が「`areka-sylphya` の `assert_logged`」を名指すので、名を残せば範囲の外の注が古びない）。
- **B5・R4（#18）**: #18 を残すと決めたので調べは要らない。

### 10.2 設計で確かめた事実（Research Log）

- `PersistIo` の範囲の外の実装は 20（どれも `crates/areka` のテストの写しで、`read`・`commit`（一部 `withholds`）だけを書く）。既定つきの口を足しても書き直しは要らない。門に止める偽の IO は範囲の外に無い（`SAVE_GATE` で他のテストを止め続ける心配が無い）。
- `save_scope` の範囲の外の呼び手は `crates/areka/src/boot_resolve.rs`（`save_app`）と `roots/edit.rs` の 2 つ。どちらも UI スレッドから呼ぶ＝錠の待ちの上限 1 秒がそのまま UI の待ちの上限になる。
- 範囲の外で `MirrorImage::flat_global`・`FormatDoc::is_all_absent`・`SylphyaMsg::PersistPut`・`KeyParseError` を使う所は 0（grep）。`MirrorImage { .. }` の字面の組み立ても範囲の外に 0。`barrier()` は範囲の外で 20 か所以上が使う（消さない）。
- `wintf_winmsg_executor::JoinHandle` は `Future`（0.0.5 の `lib.rs`）＝`spawn_local` で待つ見張りのタスクを作れる（#5 の「全送り手が落ちたら終わる」の観測）。
- `run_inbox` は toy(a)（`tests/toy_worker_test.rs`）の echo worker の中で使われる＝`spawn.rs` の単体テスト 4 本は toy(a) の 5 本と同じ規約を見る写し（#11）。
- 行番号の訂正（§3 の時点の番号の一部が古かった）: #6 の「写しでない 1 件」は `ledger_key_determinism_tests.rs` の `criterion_ext_event_names_reserved_only_not_resolution_vocab`（321〜341 行目）・#15 のテストは `actor_failure_sink_tests.rs` の `spawn_without_receiver_behaves_as_today`（71〜92 行目）。
- 範囲の外のテストの fixture に `format-version` を持たない `sylphya.toml`（`"old = 1"`・`"new = 2"`）を置く物がある（`crates/areka-user-data` の 4 ファイル・`crates/areka-mcp-bridge/src/config_prepare_tests.rs`）。どれも記憶のフォルダの写しと置き場の決めを見るテストで、sylphya の保存は通らない見込み。全体テスト（T11）で確かめる（赤なら、その範囲の spec のセッションと申し合わせる＝要件 6.6）。

### 10.3 合成（design-synthesis）

- **一般化**: #1 と #3 は同じ「読んで重ねて書く」の 2 つの穴（読めなかった分を消す・2 人が交互に混ざる）。1 か所（`save_scope`）で「錠を取る → 読めるか決める → 書く」の順にまとめた。
- **作るか借りるか**: 錠は OS の共有の決まり（`share_mode(0)`）をそのまま使う（`areka-nar` の棚の錠と同じ形・依存 0）。期限つきの待ちは既存の `ReplyReceiver::recv_timeout`。区間は `tracing::Instrument`。新しく作る仕組みは無い。
- **簡素化**: 錠の型・新しい理由の variant・理由の enum は作らない（錠は `std::fs::File` のまま返し、理由は文字列で `PersistFailureCause::Io` に入れる）。`test_log_capture.rs` は消さずに縮める。範囲の外のファイルの変更は 0。

### 10.4 設計の審査（design-review-gate）

- 機械の確かめ: requirements.md の 41 の番号がすべて design.md に在る・境界の 4 節が埋まっている・ファイルの構成に具体的なパス・登場する部品（`spawn_parts`・`post_barrier`・`lock_within`・`SAVE_GATE`・`EnterDepth`・`run_pump_until`）はすべてファイルに割り当て済み。
- 直した所（1 回目）: ⑴ 錠の期限切れの理由の文が、本当に書けないフォルダ（拒み 5）でも「ほかの areka」と言う誤りを、最後の失敗で分けた ⑵ #4 の縛るテストが、直しを外すと赤でなく止まる形だったのを、別のスレッドで呼んで期限つきで待つ形へ ⑶ プロセスの中の錠が同じバイナリの他のテストを待たせることを前提に書いた。2 回目で新しい指摘なし＝通過。

### 10.5 開発者に確かめたい点（設計の討議へ）

1. #1 の帰結: 使えない記憶のファイルが在ると、`crates/areka` の今の扱いにより、起動の前の書き込みなら起動を止め、走っている間の書き込みなら areka を終える（どちらも理由の窓が出る）。完了した `areka-P0-sylphya` の要件 6.3 の「起動継続を妨げない」は読む側（`load_scope`）で今も守る。推奨: このまま（記憶を消す代わりに止まる・要件の討議の決めのとおり）。
2. 空のファイル（中身が空白だけ）は守る中身が無いので上書きする。推奨: このまま。
3. 錠の待ちの上限 1 秒（UI スレッドの直の保存も同じ上限）。推奨: このまま。

## 11. 設計の討議で確定したこと（2026-11-06・開発者「承認」＝4 議題とも推奨案）

1. #1 で使えない記憶のファイルがあるときに areka が止まる（起動の前は起動を止め、走っている間は終わる）のを受け入れる。完了した `areka-P0-sylphya` の要件 6.7 は、書く側の失敗について完了した `areka-P0-write-failure-notice` の 10-16 の裁定「書けない＝致命」で上書きされている。足すのは 2 つ。⑴ 読みの共有違反と拒み（os error 32・5）は最大 1 秒取り直す。⑵ 知らせの決まった一文の合わない件を `audit-areka-app` の brief へ回す。
2. #2 の縛るテストのため、`crates/log-capture-kit/tests/with_default_guard_test.rs` の例外の表に 1 行足し、件数を 6 にする。表に placement の 3 行があるので、`placement-crate-split` と申し合わせてから触る。
3. #3 の錠の待ちは 1 秒。
4. 空か空白だけの記憶のファイルは上書きする。

## 12. かけ直しの結果

### 1 回目（数える指摘 7 件・全部直した）

1. （Must fix）読み取りだけの走りが、使えない記憶のファイル（新しい版・壊れた TOML）で起動を止めるようになった → `save_scope` で控える道かを先に見て、控える道なら `read_existing` の失敗で止めず、warn! を 1 件出して空の土台で続ける（この spec の前と同じ寛容な読み・何も書かない）。縛るテスト `withhold_tests.rs` の `save_scope_inside_does_not_stop_on_an_unusable_existing_file`（`format-version = 2` のファイルで `Withheld`・バイトで同じ）。design.md の「控えに入った中身は areka が書いた物」の誤りも直した。
2. （Should fix）書き込みの最後の `rename` が一時の掴み（os error 32・5）で待たずに失敗する → 取り直しを小さな関数 `io::retry_transient` に出し、`commit` の `rename` と `read_existing` の読みの両方がそれを使う（決まりは同じ・`LOCK_RETRY` ごとに `LOCK_WAIT` まで）。縛るテスト `io_lock_tests.rs` の `retry_transient_retries_a_sharing_violation_only`。
3. （Should fix）版が 1 のままの知らない表・型の違う値を保存のたびに消す → 保存の土台を読んだファイルの表（`toml::Table`）のまま持ち、`FormatDoc` の値だけを重ねて書く（`format::to_toml_string_over`）。消す道はもともと無い（起動中の印は空文字を書いて「無し」にするので、上書きで足りる）。縛るテスト `persist_unreadable_tests.rs` の `unknown_tables_and_values_survive_a_save`（`[foo]`・`[last]` の知らない key・整数の `count` が残る）。
4. （Should fix）落ちようのないテスト 2 本 → `format.rs` の `last_only_is_not_all_absent` と `persist_window_adjacent_tests.rs` の `adjacent_only_doc_is_not_all_absent` を消した。
5. （Nice to have）数だけを見るテスト 5 本 → `flat_vocab_has_26_entries`・`dotted_roots_has_10_entries`・`generic_prop_names_has_17_entries`・`set_effective_has_25_entries`・`menu_group_consistent_with_generic_names` を消した（中身の一致のテストが数も決める）。
6. （Nice to have）`ui.rs` の async-channel だけを見るテスト → `dropping_all_senders_closes_the_channel` を消した（本物の `spawn_ui` は `tests/toy_ui_pump_test.rs` が縛る）。
7. （Nice to have）記録の捕捉の書き方が 2 つ → `persist_failure_tests.rs` と `withhold_tests.rs` の 2 か所を `crate::test_log_capture::capture` に替えた。

ついでに古くなった注 2 つを直した: `io.rs` の「M1 は単一プロセス前提・錠は要るとき足す」（錠はもう在る）と、`persist/mod.rs` の冒頭の保存の結果の並び（錠が取れない・既存が使えない・`Withheld` を足した）。

数えない再出（design.md「直さない・回した指摘」の件・直さない）: #18 の本番が読まない語彙（`FlatEntry::layer`・`m1`・`BackingLayer`・`M1Status`・`DegradePolicy::NotFound`・`SetSemantics`・`SET_EFFECTIVE` の 2 番目の要素）・#10 の `RuntimeCommandSink`／`SylphyaInit::runtime_sink`・#16 の `UiSpawnError::NotUiThread`・#21 の `SylphyaCore`・#4 の `barrier_within` の本番の呼び手 0・#1 の決まった一文（`crates/areka/src/alert.rs`・範囲の外）。

### 2 回目（数える指摘 3 件・全部直した）

1. （Should fix）起動の読み（`load_scope`）だけが一時の掴み（os error 32・5）で待たず、覚えていたゴーストを忘れて既定のゴーストで書き替える → `load_scope` の読みも `io::retry_transient(LOCK_WAIT, ..)` を通す（1 行）。取り直した後も読めなければ、今と同じく warn! を出して「何も覚えていない」として返す（読む側の寛容な不在はそのまま）。縛るテスト `persist_unreadable_tests.rs` の `a_transient_sharing_violation_on_load_is_retried`（1 回目の読みだけ 32 を返す偽の IO で、保存した `LastGhost` が読める。取り直しを外すと空になって赤）。
2. （Nice to have）`format::to_toml_string` が本番で使われず、テストのためだけの公開の関数になった → ツリー全体（`crates`・`tools`）で呼び手がこのファイルのテストだけなのを確かめて消し、テスト 5 か所は `to_toml_string_over(toml::Table::new(), &doc)` にした。説明の注は `to_toml_string_over` へ移した。
3. （Nice to have）錠の取り直しの輪（`lock_within`）が `retry_transient` と同じ決まりをもう 1 つ持っていた → `retry_transient` に待ちの上限を引数で渡す形にし（本番の呼び手は `LOCK_WAIT`）、`lock_within` はそれを使って、期限切れの最後の失敗が共有違反（32）のときだけ今と同じ `TimedOut` の文（「ほかの areka…」）に替える。拒み（5）とほかの失敗はそのまま返す。振る舞いと文は変わらない（`io_lock_tests.rs` の錠のテストが緑のまま）。

古くなった注も直した: `vocab/flat.rs`・`vocab/dotted.rs` の「件数檻: `….len() == N`」3 か所（1 回目の 5 で消したテストを指していた）は、今も並びを縛るテストの名（`flat_vocab_tokens_match_exactly`・`dotted_roots_match_exactly`・`generic_prop_names_match_exactly`）へ。`persist/format.rs` の「全値が String/Integer/Table なので直列化は失敗しない」は、土台が読んだファイルの表になったので「読んだ TOML の表とこの関数が足す値からは失敗しない」へ。`persist/io.rs` の `LOCK_WAIT` の注は、使う所（記憶のファイルの読み・確定の `rename`・更新の間の錠）へ。

design.md の古い行も直した: `read_existing` の注と戻り値（2 つ組の既定 `Ok(Default::default())`）、取り直しの説明（`io::retry_transient` の名と、使う 4 か所 `read_existing`・`commit` の `rename`・`load_scope`・`lock_within`）、`lock_within` の説明、流れの図（控える走りでは使えない既存でも warn! を出して空の土台に重ね、`io.commit` で控えの中身に置いて `Withheld`。同じ走りの後の `load_scope` はその控えを読む）、#19 の行（`persist_window_adjacent_tests.rs` の `adjacent_only_doc_is_not_all_absent` と `format.rs` の `last_only_is_not_all_absent` は 1 回目の 4 で消した）。

数えない再出（1 回目と同じ・直さない）: #18 の本番が読まない語彙・#10 の `RuntimeCommandSink`／`SylphyaInit::runtime_sink`・#16 の `UiSpawnError::NotUiThread`・#21 の `SylphyaCore`・#4 の `barrier_within` の本番の呼び手 0・#1 の決まった一文（範囲の外）。確かめて指摘にしなかったもの: `merge_table` の重ね方・`parse_toml_str` の 2 つ組の呼び手・控える道の寛容な読み・`retry_transient` の待ち方・`to_toml_string_over` の直列化の失敗の道（起きる入力が無い）・数のテストを消した後も中身の一致のテストが数を決めていること・記録の捕捉の書き方・手で書いた 0 で始まる窓の ID（この spec の前からの振る舞い）。

### 3 回目（数える指摘 2 件・全部直した）

2 回目の 3 件の直しに誤りは見つからなかった。2 件とも前から在って 1・2 回目が拾わなかった、ほかのテストに丸ごと含まれるテスト。

1. （Nice to have）記憶の保存のテストに、後から足した上位のテストに含まれる古いテストが 3 本 → `persist_tests.rs` の `load_tolerates_read_failure`・`save_commit_failure_is_degraded_and_leaves_prior_intact`・`none_root_save_is_degraded_no_panic` を消した。消す前に、残るテストが同じことを全部見ているのを確かめた: `load_read_failure_emits_warn_and_degrades` は準備が同じで「読みの失敗で空」を見る（ほかに warn! も）。`save_commit_failure_emits_error_log` は準備が同じで `Degraded` と「前の中身は 1 のまま」を見る（ほかに error! も）。`persist_failure_tests.rs` の `scope_without_root_returns_no_root` は置き場の無い保存の結果を `Degraded(PersistFailure { NoRoot, dir: None, keys })` まで丸ごと比べる（スコープは Ghost でなく Shell だが、置き場が全部無いときの道はスコープで分かれない）。panic しないことは、どのテストも走り切ることで見ている。中身の見出し「IO 障害の寛容縮退」はその下が空になったので一緒に消した。
2. （Nice to have）読み口の「決定論」のテストが、同じ純粋な関数を同じ引数で呼び比べるだけ → `reader.rs` の `determinism_dotted_value_present_stable` を消し、それを指す注を「決定論は純粋な関数を同じ引数で呼ぶだけなので檻を置かない」の 1 行にした。タスク 1.3 では「値の在る枝を見る読み口のテストは残す」としたが、消した後も値の在る枝は `dotted_value_present_per_asker`・`dotted_value_present_global`・`dotted_per_asker_precedes_global`・`dotted_str_roundtrips_with_canonical_stored_key`・`per_asker_non_mixing_flat_and_dotted` が縛る。読みが写しを書き替えないことは `pull_tests.rs` の `reentrant_read_does_not_stall_and_does_not_store` が縛る。

消した 4 つの名は、ツリー全体（`.kiro` を除く）で残りが 0 なのを grep で確かめた。

数えない再出（1・2 回目と同じ・直さない）: #18 の本番が読まない語彙・#10 の `RuntimeCommandSink`／`SylphyaInit::runtime_sink`・#16 の `UiSpawnError::NotUiThread`・#21 の `SylphyaCore`・#4 の `barrier_within` の本番の呼び手 0・#1 の決まった一文（範囲の外）。

確かめて数えなかったもの: 錠を持つ時間とほかのプロセスの錠の待ち。1 回目の 2 の直しから、書き手は錠を持ったまま最大 1 秒（読み）＋1 秒（rename）眠りうる。ほかのプロセスの錠の待ちは 1 秒なので、2 つの areka が同じ App の記憶をほぼ同時に保存し、そこへウイルス対策の 1 秒近い掴みが重なると、後の方が `TimedOut` で `Degraded` になる。ただ普段の一時の掴みはミリ秒で、3 つが重なることは実際にはまず起きないので数えない（直すなら錠の待ちを `2 * LOCK_WAIT` 以上にするだけ）。ほかに `retry_transient` の待ちの引数・`lock_within` の `map_err`・`load_scope` の取り直し・`to_toml_string` の削除も確かめて、誤りは無かった。

### 4 回目（数える指摘 2 件・全部直した）

3 回目の 2 件の直しに誤りは見つからなかった。2 件とも前から在って 1〜3 回目が拾わなかった、落ちようのない／ほかに丸ごと含まれるテスト。不具合・危険・遅い道は見つからなかった。

1. （Nice to have）写しのテストに、標準の表（`BTreeMap`）へ手で入れて同じ表から読み戻すだけの、落ちようのないテストが 2 本 → `mirror.rs` の `per_asker_partitions_are_independent`・`dotted_partitions_hold_canonical_string_keys` を消した。消す前に、本物の読み口を通るテストが同じことを見ているのを確かめた: per-asker の区画が混ざらないことは `reader.rs` の `flat_other_asker_does_not_leak` と `per_asker_non_mixing_flat_and_dotted`（フラット・点付きとも両方向）が、点付きの区画を正準の文字で引けることは `dotted_value_present_per_asker`・`dotted_value_present_global`・`dotted_str_roundtrips_with_canonical_stored_key` が縛る。
2. （Nice to have）出どころの語の組の並べ方のテスト 2 本が、同じファイルの定数のテストに丸ごと含まれる → `origin_tests.rs` の `render_head_only`・`render_external_sstp` を消した。消す前に確かめた: `constants_render_and_level` は `GHOST`（`{ head: Internal, rest: &[] }`）で `"internal"`、`MCP_NOTIFY`（`{ head: External, rest: &[Sstp] }`）で `"external,sstp"` を見ており、組も文字も同じ。

ついでに注 3 つを直した（数えなかったもの・下の「確かめて数えなかったもの」の件）: `vocab/flat.rs` の `FLAT_VOCAB`、`vocab/dotted.rs` の `DOTTED_ROOTS`・`GENERIC_PROP_NAMES` の「並びはテスト … が縛る」は、テストが集合か並べ替えた列にしてから比べるので並びは縛っていない。「顔ぶれは（`DOTTED_ROOTS` は顔ぶれと重複の無さは）テスト … が縛る・並びは縛らない」に替えた。

消した 4 つの名は、ツリー全体（`.kiro` を除く）で残りが 0 なのを grep で確かめた。

数えない再出（1〜3 回目と同じ・直さない）: #18 の本番が読まない語彙（それを見る `set_effective_all_runtime_command`・`only_four_tokens_are_m1_derived` も含む）・#10 の `RuntimeCommandSink`／`SylphyaInit::runtime_sink`・#16 の `UiSpawnError::NotUiThread`・#21 の `SylphyaCore`・#4 の `barrier_within` の本番の呼び手 0・#1 の決まった一文（範囲の外）。

確かめて数えなかったもの: 3 回目の直し（消した 3 本の主張は残るテストが今も全部持つ・`reader.rs` の注の 1 行）。`mirror.rs` の残りの 4 本（#17 で残すと決めた物）。`pull_tests.rs` の登録し直しの 2 本（層が違い、複製の側から登録し直す向きは片方だけが持つ）。`shiori_resource_ids_has_159_entries`（この表には中身の一致のテストが無く、159 を決めるのはこの 1 本だけ）。上の注 3 つ（コードの振る舞いは変わらない）。記憶のファイルの読み書きの道・錠を持つ時間・`run_actor`・`parse_seg`・`spawn_actor`・`run_inbox`・`ui.rs`・`origin.rs`（誤った結果になる入力は無かった）。スレッドの配線の揺れを見る 20 回の繰り返しのテスト 2 本。

### 5 回目（数える指摘 3 件・全部直した・ここで打ち切り）

4 回目の 2 件の直しに誤りは見つからなかった（消した 4 本の名を指す所はツリーに残っていない）。不具合・危険・遅い道は見つからなかった。3 件とも前から在って 1〜4 回目が拾わなかった、ほかのテストに丸ごと含まれるテスト。

1. （Nice to have）働き手のアクターの結線テストのうち 2 本が、同じファイルの別の 2 本に丸ごと含まれる → `areka-actor/tests/toy_worker_test.rs` の `echo_round_trip_matches_payload`・`close_then_join_completes_deterministically` と、それを並べる冒頭の注の (i)・(ii) の 2 行を消した。消す前に確かめた: `backlog_after_close_is_discarded_and_reply_dropped` は同じ `spawn_echo_worker` に `Close` を送り、期限つきの join が `Ok` になるのを見る（ほかに積み残しの返事が `Dropped` になることも）。`handler_err_does_not_stop_receive_loop` は `spawn_actor`・`run_inbox` の続ける枝・`reply_channel` の往復を通り、返った文字 `"after-error"` を比べる。
2. （Nice to have）UI アクターの往復のテスト 1 本が、手前に `Fail` を 1 件足しただけの隣のテストに丸ごと含まれる → `areka-actor/tests/toy_ui_pump_test.rs` の `toy_b_worker_to_ui_pump_echo_round_trip` を消した。隣の `a_handler_error_does_not_stop_the_ui_actor` は同じ `echo_round_trip_after` を `vec![UiMsg::Fail]` で呼ぶだけで、その後の往復・`"ping"` の比べ・`Close`・期限は同じ。
3. （Nice to have）出どころの語の組の並べ方のテストが 1 本、4 回目と同じ理由で丸ごと含まれる → `areka-talk/src/origin_tests.rs` の `render_internal_raise` を消した。頭が `internal` で 1 語つなぐ組は `property_constant_renders_internal_property` が `"internal,property"` で、`raise` の綴りは `render_all_rest_words_in_order` が 7 語の並びの中で見ている。

消した 4 つの名は、ツリー全体（`.kiro` を除く）で残りが 0 なのを grep で確かめた。足したテストは無い。

数えない再出（1〜4 回目と同じ・直さない）: #18 の本番が読まない語彙（それを見る `set_effective_all_runtime_command`・`only_four_tokens_are_m1_derived` も含む）・#10 の `RuntimeCommandSink`／`SylphyaInit::runtime_sink`・#16 の `UiSpawnError::NotUiThread`・#21 の `SylphyaCore`・#4 の `barrier_within` の本番の呼び手 0・#1 の決まった一文（範囲の外）。

5 回目は「必ず直す」0 件・本番の振る舞いに関わる指摘 0 件だったので、その回の指摘を直したところで打ち切った（audit-seriko・audit-kanade と同じ区切り方）。回ごとの数える指摘は 7→3→2→2→3 で、残りは全部直し済み。数えない再出は design.md「直さない・回した指摘」の 6 件だけ。

## 13. 既定の枝との差分の確かめ（要件 7.5・2026-11-12）

比べた相手: `git fetch origin main` の後の `origin/main`（`96bd5b12`）。枝との分かれ目（merge-base）も `96bd5b12` で、差分は `git diff origin/main...HEAD`（HEAD は `aa606ebe`）。6 つとも約束どおりだった。

1. 触らないファイルの差分が空（要件 6.2・6.8・5.5）
   - 回したもの: `git diff --stat origin/main...HEAD -- crates/areka/ crates/areka-placement/ crates/log-capture-kit/tests/file_length_guard_test.rs crates/log-capture-kit/tests/temp_path_guard_test.rs crates/wintf/src/ecs/window_proc/ tools/perf/fixtures/ tools/perf/fixtures-loop/ vendors/ Cargo.lock`
   - 結果: 出力は 0 行（空）。`crates/areka-placement/` も含めて差分は無い。
2. 差分のファイルの名が範囲と spec と例外の表のファイルだけ（要件 6.7）
   - 回したもの: `git diff --name-only origin/main...HEAD` から `crates/areka-sylphya/`・`crates/areka-actor/`・`crates/areka-talk/`・`.kiro/specs/audit-sylphya-actor/` の下を除いた残り。
   - 結果: 変わったファイルは全部で 48。残りは 5 つ。
     - `crates/log-capture-kit/tests/with_default_guard_test.rs`: 差分は例外の表への 1 件（`crates/areka-actor/tests/ui_span_test.rs` と理由の文・4 行の組）と、件数 `ALLOWED_DIRECT_CALLS_COUNT` の 5 → 6 だけ。
     - `.kiro/specs/audit-areka-app/brief.md`・`.kiro/specs/audit-areka-install-update/brief.md`・`.kiro/specs/audit-emo2-boot-spine/brief.md`・`.kiro/specs/sylphya-sqlite-kvs/brief.md`: 要件の討議で、直さずに回した指摘（#1 の決まった一文・#4 の呼び手・#16・#10 と #21）を回し先の brief へ書き足したコミット（`4176d4ea`・`dfeaacec`）の変更。意図した変更として数える（tasks.md の Implementation Notes 8.3）。
   - ほかに思いがけないファイルは無かった。
3. 保存の口のファイルに、一時のパスの入口と `TempPath` の行の増減が無い（要件 6.3）
   - 回したもの: `git diff origin/main...HEAD -- crates/areka-sylphya/src/persist/io.rs` の増えた行と消えた行から `temp_dir(`・`process::id(`・`TempPath` を探す。あわせて既定の枝と今のファイルで同じ綴りの数を比べる。
   - 結果: 増減した行に当たりは 0 件。数は既定の枝・今とも `temp_dir(` 0・`process::id(` 0・`TempPath` 5 で同じ（今の 321・323・332・349・363 行目）。`io.rs` 自体は錠（3.1）と取り直しの `retry_transient`（かけ直し 1・2 回目）で 105 行増えて 22 行減ったが、この確かめが見るのは上の綴りの行だけ。錠の実のファイルのテスト 2 本は兄弟の `persist/io_lock_tests.rs` に置いたので（Implementation Notes 3.1）、`TempPath` の行はそちらにだけ在る。
4. 公開の形（要件 6.1）
   - 回したもの: 3 つのクレートの `src` の差分から、消えた行と増えた行のうち `pub` で始まる物・列挙の値・トレイトの関数を拾った。消した名は `git grep -F <名> -- '*.rs' ':!crates/areka-sylphya' ':!crates/areka-actor' ':!crates/areka-talk'` で 3 つのクレートの外の Rust のファイルを数えた。
   - 消した公開の物は 4 つ。どれも 3 つのクレートの外の Rust のファイルで 0 か所:
     - `MirrorImage::flat_global`（欄）: 0 ファイル。
     - `FormatDoc::is_all_absent`: 0 ファイル。
     - `SylphyaMsg::PersistPut` の `reply` の欄: `PersistPut` の綴りが 0 ファイル（外で `SylphyaMsg` を使うのは `crates/areka-placement/src/zorder_property_deferral_tests.rs` の `SylphyaMsg::Set` だけ）。
     - `persist::format::to_toml_string`: 設計の表の 3 つに無い 4 つ目。かけ直しの 2 回目の 2 で消した（`persist` も `format` も `pub mod` なので、外から `areka_sylphya::persist::format::to_toml_string` で呼べた公開の関数だった）。外の Rust のファイルで 0 ファイル。消したときにも `crates`・`tools` 全体で呼び手がこのファイルのテストだけなのを確かめている。代わりの `to_toml_string_over` は `pub(crate)` で、公開の形には足していない。
     - 当たりが在るのは `.kiro/specs/completed/` の下の古い記録だけ（設計の文・テストの一覧）。コードではないので数えない。
   - 足した公開の物は 3 つで、設計のとおり:
     - `PersistIo::lock_for_update`: 既定の本体（`Ok(None)`）つき。外で `PersistIo` を実装している所は `git grep -n "impl.*PersistIo for" -- crates ':!crates/areka-sylphya'` で 19 か所（`crates/areka-placement` 11・`crates/areka` 8・どれもテストかテストの道具）。どれも `lock_for_update` を書かずに既定の本体を使うので書き換えは要らず、確かめ 1 のとおり 2 つのクレートの差分は空。
     - `SylphyaPublisher::barrier_within`。
     - `KeyParseError::TextAfterParen { at: usize }`: 外で `KeyParseError::` の値を書く所は 0 か所。
   - そのほか: `read_toml_str` は名も形も前のまま。増えた `pub(crate)` の物（`spawn_parts`・`to_toml_string_over`・`parse_toml_str`・`LOCK_WAIT`・`LOCK_RETRY`・`is_transient`・`retry_transient`）はクレートの外から見えない。消えた `pub(crate)` の `CapturedEvent` とその欄（#12）、`areka-actor` の `run_bounded`（#11・テストの中の非公開の関数）も公開の形ではない。
5. ドット名の語彙のファイルの 2 行（要件 5.7）
   - 回したもの: `git diff origin/main...HEAD -- crates/areka-sylphya/src/vocab/dotted.rs` の増えた行と消えた行から `COMPAT_ARCHITECTURE` を数える。あわせて既定の枝と今のファイルで `COMPAT_ARCHITECTURE` の在る行を見る。
   - 結果: 増減した行の当たりは 0 件。既定の枝・今とも 64 行目と 80 行目で、文は同じ（差分の塊は 10・27・221・249・293・372 行目あたりで、2 行に掛からない）。
6. 語彙の台帳が引く 2 つの記録の文と target が残る
   - 回したもの: `git grep -F -e 'SET on non-settable canonical vocab: warn and drop (call still Ok, no write)' -e 'dotted resolution not found' -- crates/areka-sylphya/src` と、それぞれの `target:`。
   - 結果: `crates/areka-sylphya/src/actor.rs` 378 行目の `warn!` が同じ文で、target は `LOG_TARGET`＝`"areka_sylphya::actor"`（40 行目・前と同じ値）。`crates/areka-sylphya/src/reader.rs` 157 行目の `debug!` が同じ文で、target は `"areka_sylphya::reader"`。
   - 気づいたこと（直していない・範囲の外・`/kiro-complete` の棚卸へ）: `doc/ukadoc-coverage/ledger/property.toml` は場所を `reader.rs:158-164`・`actor.rs:345-350` と行番号でも書く。`actor.rs` の方は既定の枝でも 379 行目で、この spec の前から合っていない。`reader.rs` の方は既定の枝の 163 行目から 157 行目へ動いたので、書いてある範囲（158-164）から少し外れた（`debug!` の始まりは 153 行目）。台帳は範囲の外で、約束は文と target なので、ここでは直さない。行番号を見る番人はツリーに無い。
