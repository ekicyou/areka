# ギャップ分析: mcp-scope-rect-property

> 2026-10-23・本ブランチ（main `15d8b0f3` の上・spec の初期化 `93b8d452`）の実物で調べた。行番号はこの時点のもの。
> 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` のウェーブ C11 の表）。
> この文書は情報と選択肢を並べるもので、決めは要件ディスカッションと設計で行う。
> **2026-10-23 の要件ディスカッションで、1・3 b・5・9 節の推し（案 A＝毎フレーム比べてプッシュ）は退けられた。決まった形は 10 節**（矩形はプル・「スコープ → 窓」の表はプッシュ・出ていない窓も矩形を答える）。1〜9 節は調べた時点の記録として残す。

## 1. まとめ

- **読む道はもう在る。足りないのは「値を出す所」と「値を集める所」の 2 つ。** MCP の `get_property`（`crates/areka/src/mcp/get_property.rs:43-49`）とゴーストの道（`crates/areka/src/shiori_host.rs:250-259`）は、どちらも `SylphyaReader::resolve_dotted_str`（`crates/areka-sylphya/src/reader.rs:90`）を呼ぶだけ。値の置き場に 2 つの名前の値が載れば、読む側は 1 行も変えずに答えられる。
- **窓の矩形の正本は UI スレッドの World に在る。** 各窓の `WindowPos`（物理の画素・位置と大きさ）と、スコープから窓を引く表 `GhostWindows`（`crates/areka/src/placement/spawn.rs:305-324`）。ただし「画面に出ているか」は OS の窓では分からず、表示の側 `EmoPresenter::target_visible`（`crates/areka-emo-present/src/presenter/read.rs:225`）が正本。
- **値の置き場に足りないもの**: ⑴ 刻々と変わる値を出す口（今は `publish_static`・`publish_shiori`・`set`・`persist_put` だけ・`crates/areka-sylphya/src/actor.rs:473-503`）、⑵ 値を**消す**手段（効果の一覧 `Effect` に消す効果が無い・`actor.rs:181-243`）、⑶ 他の値と混ざらない区画（`MirrorImage` は 4 区画・`crates/areka-sylphya/src/mirror.rs:33-44`）。
- **推しの形は「UI スレッドで毎フレーム矩形を集めて比べ、変わったときだけ値の置き場へ丸ごと出す」**（案 A）。「読む時に問い合わせる」（案 B）はゴーストの道の再入の約束と食い違う。ただし案 A には「出してから読めるまでの間」（要件 4.6）と「ウェーブの約束で触れないファイル」の 2 つの詰め所がある（下の 5・6 節）。
- **規模 M（8〜12 タスク）・危うさ 中。** 窓の外枠は、今の面に出ていない着せ替えの層の分まで面の画像より広く取られることがある（下の 3 節 a）。

## 2. 今あるもの（調べた場所）

### 2.1 読む道（変えなくてよい）

| 場所 | 中身 |
|---|---|
| `crates/areka/src/mcp/get_property.rs:25-51` | MCP の `get_property`。`GhostSlot` から実行系を取り、`resolve_dotted_str` の答えが `Value` なら素のまま、`NotFound` なら `NG:Cannot find such property name.`（48 行目）。**UI スレッドの Input 段で World を持って走る**（`crates/areka/src/mcp/mod.rs:49-53` で `Input` 段に登録、`64-75` の `drain` が `&mut World` で振り分け）。 |
| `crates/areka/src/shiori_host.rs:250-259` | ゴーストの道の `GetProperty`。同じ `resolve_dotted_str`。**任意のスレッドから同期で呼ばれ、他のスレッドへ問い合わせない約束**（再入の約束・`shiori_host.rs:29-36`・`218-220`）。値を書く側の反映は「有界の遅れ」を認めている（`shiori_host.rs:24`）。 |
| `crates/areka-sylphya/src/reader.rs:90-104・131-150` | 名前を `parse_dotted` で読み、正準の文字列（`currentghost.scope(0).rect` の形・英字は小文字）にして、点付きの per-asker 区画 → 大域区画の順に引く。読み方は「読みのロック → `Arc` の複製 → 表を引く」だけ（`mirror.rs:86-99`）。書式が読めない名前は `NotFound`（要件 3.7 はこのままで満たす）。 |
| `crates/areka-sylphya/src/actor.rs:141-175` | ゴーストが `currentghost.*` へ SET しても「正準の語彙で SET 無効」（`NotSettable`）になり書かれない。2 つの名前の値をゴーストが上書きする道は無い。 |

### 2.2 値の置き場（足す所）

| 場所 | 中身 |
|---|---|
| `crates/areka-sylphya/src/mirror.rs:1-20・33-44・105-108` | 掲示板の作り。1 枚の不変の写し（`epoch` 付き）を、書き手（アクターの 1 本のスレッド）が新しい写しに差し替える。区画は `flat_per_asker`・`flat_global`・`dotted_global`・`dotted_per_asker` の 4 つ。 |
| `crates/areka-sylphya/src/actor.rs:48-97` | 書き手へのメッセージ `SylphyaMsg`。`PublishStatic`・`PublishShiori`・`Set`・`PersistPut`・`Barrier`・`Close`。 |
| `crates/areka-sylphya/src/actor.rs:181-243・618-701` | 判断の中核 `SylphyaCore::apply` が返す効果 `Effect` と、それを写しへ当てる受信ループ。**効果はすべて「書く」で、キーを消す効果が無い**。 |
| `crates/areka-sylphya/src/actor.rs:536-543・555-600` | 出す口は投げっぱなし（`mpsc` の送り）。反映はアクターのスレッドが後で行う。`barrier()` で反映を待てるが、待つのは呼び手のスレッド。 |
| `crates/areka-ghost/src/sylphya_wiring.rs:107-120`・`crates/areka-ghost/src/runtime.rs:234-241` | 値の置き場は**ゴーストの実行系ごとに 1 つ**起こされる。ゴーストの切替・`reload` で実行系を作り直すと、空の写しの新しい置き場になる（要件 6.2・6.3・6.5 は作りの上で自然に満たす）。 |

### 2.3 窓の矩形の持ち主（読むだけ）

| 場所 | 中身 |
|---|---|
| `crates/areka/src/placement/spawn.rs:305-324` | `GhostWindows`（World の Resource）。スコープ番号 → キャラクターの窓・バルーンの窓。ゴーストが持つスコープの窓は起動時に全部作る（無い番号は `None`＝要件 3.1）。 |
| `crates/areka/src/placement/follow/window_move.rs:1097-1118` | `window_rect_of`: `WindowPos` から矩形を組む先例（位置が `CW_USEDEFAULT` の印、大きさが 0 以下なら「まだ無い」）。負の座標は正しい値として通す。**同じ判定を新しいファイルで書き直すか、これを公開して使うかは設計で**（このファイルは 1,221 行で、ウェーブの他の 2 本が「触らない」と約束している）。 |
| `crates/areka/src/placement/follow/window_move.rs:633-760` | 窓の位置を書く唯一の口 `enqueue_window_set_pos`。OS へ移動の命令を積み、同時に `WindowPos` を**変更の印を立てずに**（`bypass_change_detection`）先に書き換える（731-735 行）。 |
| `crates/wintf/src/ecs/window_proc/window_pos.rs:136-205` | OS からの `WM_WINDOWPOSCHANGED`。自分が出した移動の返りは印を立てずに書き、外からの変化（拡大率の違う画面への移動など）は印を立てて書く。値はクライアントの座標（ゴーストの窓は枠の無い `WS_POPUP` なので窓の外枠と同じ・`spawn.rs:595-599`）。 |
| `crates/wintf/src/runtime/mod.rs:137` | プロセスはモニタごとの DPI 対応 v2。`WindowPos` は物理の画素・主の画面の左上を原点とする仮想画面の座標＝SSP の数の系と同じ（要件 1.3）。 |
| `crates/areka-emo-present/src/presenter/read.rs:210-227`・`presenter/hub.rs:215-237` | 「画面に出ているか」の正本 `target_visible`。`\s[-1]`（`Hide`）は**絵を隠すだけで OS の窓は残る**（窓は `WS_VISIBLE` のまま・位置も大きさも変わらない）。最初の `ShowSurface` の前は `None`。 |
| `crates/areka/src/emo2_boot/frame/wiring.rs:55-57` | 表示の側 `presenter` を持つ `Emo2Wiring` の欄は `pub(in crate::emo2_boot)`＝**`emo2_boot` の中からしか読めない**。毎フレームの処理の間だけ World から外され、終わりに戻される（`emo2_boot/frame.rs:298・399`）。 |
| `crates/areka/src/emo2_boot/target_map.rs:19-27` | スコープ → 表示の的（`shell_target`・`balloon_target`）。 |
| `crates/areka/src/placement/diag.rs:380` | `WindowMoveRecord`（窓ごとの物理の位置・大きさ・スコープ）。記録の行で、値の出どころにはならない（観測は既定で無音）。 |

### 2.4 World へ書き手を差す先例

- `PersistWiring`（`crates/areka/src/placement/persist.rs:500-503`）: 今のゴーストの値の置き場への出す口（`SylphyaPublisher` の複製）を World の NonSend として持つ。起動のたび・実行系を作り直すたびに差し直される（`crates/areka/src/main.rs:805-807`・`ghost_session_runtime.rs:24・59`）。**新しい集め手は、この出す口をそのまま読める**（新しく差す物を増やさずに済む）。
- 毎フレームの系の登録の先例: `ghost_session::register_systems`（`crates/areka/src/ghost_session.rs:63-91`）が `FrameFinalize` 段へ 2 つの系を足している（84-90 行）。

## 3. 要件の段から預かった 2 点の答え

### a. 窓の外枠が「見えている面の画像」より大きく取られることがあるか

**ある。** キャラクターの窓の大きさは、面の**合成の外形**を拡大率で掛けた物理の大きさで、毎フレームそこへ合わせ直される（`crates/areka/src/emo2_boot/frame/drain_resnap.rs:213-246`・`physical_size` の説明 `crates/areka-emo-present/src/presenter/read.rs:42-67`）。その外形は次の決めで算出される（`crates/areka-emo-compose/src/plan_extent.rs:12-25`）:

- **その面の全部の定義の層**（全 element ＋ **今オンかオフかに関係なく全部の着せ替えのアニメーションの最初のコマ**）の和集合。
- **全く透明な element も原寸が分かっていれば外形に入る**。

したがって、着せ替えの部品が本体の外へはみ出して置かれている面では、その部品が**オフでも**窓はその分だけ広い（「着せ替えのオンオフで窓の大きさが変わらないように」という決め・要件 6.5 の裁定）。また、窓の中の α が 0 の所はもともと見えない（当たりは α ≥ 0.5 の画素だけ）。

さらに次の 2 つも「窓の外枠 ≠ 見えている絵」になる:
- `\s[-1]` で隠したスコープ: 絵は消えるが OS の窓は同じ位置・同じ大きさで残る（`presenter/hub.rs:215-237`・`spawn.rs:595-599`）。要件 3.2 の「画面に出ていない」は、OS の窓ではなく表示の側の可視（`target_visible`）で見分けるしかない。
- 起動の直後で最初の面が出る前: 窓は配置の見積もりの大きさで作られる（`spawn.rs:439-480` の `window_pos(...)`）。要件 3.4 の「まだ置かれていない」は、表示の側の `target_visible` が `None`/`false` の間と読むのが素直。

バルーンの窓は、バルーンの面の物理の大きさへ合わせ直される（`crates/areka/src/emo2_boot/frame/dpi.rs:159-162` の `resize_window_keep_position`）。バルーンの面より広く取る作りは見つからなかった。

→ 要件の議題 3（「窓の外枠」か「見えている画像の範囲」か）は**諮る対象になった**。SSP が着せ替えのオフの部品の分を窓に含めるかは測っていない（Research Needed R2）。

### b. 「変わるたびに出す」か「読む時に問い合わせる」か — 今のコードで楽な所・難しい所

**変わるたびに出す（推し）**

- 楽な所:
  - 値の置き場が最初から「書き手が出し、読み手は待たずに読む」作り（`mirror.rs:1-7`・`86-108`）。メッセージを 1 種、効果を 1〜2 種足すだけで、読む道（`reader.rs`）と MCP・ゴーストの両方の入口は変えずに済む（要件 6.6・7.2）。
  - 出す口は World に既に在る（`PersistWiring.publisher`・`placement/persist.rs:500-503`）。実行系を作り直すと差し直されるので、新しいゴーストの置き場へ出せる。
  - 矩形の材料（`GhostWindows`・`WindowPos`・`target_visible`）はどれも UI スレッドの World に在り、1 フレームの最後に一度に読めば、キャラクターとバルーンが同じ時点の値で揃う（要件 4.5・4.7）。
- 難しい所:
  - **変わったことを印で知れない**。位置を書く唯一の口が印を立てずに書き（`window_move.rs:731-735`）、OS からの返りも印を立てずに書く（`wintf/.../window_pos.rs:144-149`）。可視の変化も表示の側の中の値。→ `Changed<WindowPos>` には頼れず、**毎フレーム、前に出した値と比べる**形になる（窓は数枚なので軽い）。
  - **消す手段が無い**（`actor.rs:181-243`）。窓が消えた・バルーンが隠れたときに値なしへ戻す（要件 7.4・3.6）には、新しい効果が要る。「動く値の区画を丸ごと差し替える」メッセージにすれば、消すことも、キャラクターとバルーンを同時に揃えることも 1 回で済む。
  - **出してから読めるまでの間**。出す口は投げっぱなしで、反映はアクターのスレッド（`actor.rs:536-543`）。フレーム N の最後に出した値が、フレーム N＋1 の Input 段の MCP の読みまでに写しへ載っている保証は無い（ふつうは数 μs で載るが、要件 4.6 は「約束」）。今のゴーストの道は「有界の遅れ」を認めている（`shiori_host.rs:24`）。下の 5 節の案 A-1〜A-3 で詰める。
  - **作り直した置き場への出し直し**。`reload` は窓をそのまま残して実行系だけを作り直す（`ghost_session_runtime.rs:87` の `rebuild_runtime_in_place`）。窓が動いていなければ「前と同じ」なので比べる形だと出さない → 新しい置き場が空のままになる。集め手は「出す先が替わった」ことを知る手段を持つ必要がある（`PersistWiring` の差し直しの印・あるいは読み手で今の写しを見て比べる、など）。
  - **表示の側は `emo2_boot` の中からしか読めない**（`frame/wiring.rs:57`）。集め手は `emo2_boot` の中に置くことになり、`emo2_boot/mod.rs` に `mod` の 1 行が要る（6 節の約束と当たる）。

**読む時に問い合わせる**

- 楽な所:
  - MCP の道だけなら非常に楽。MCP の `get_property` は既に UI スレッドで `&mut World` を持って走る（`mcp/mod.rs:49-75`・`get_property.rs:25`）。`GhostWindows`・`WindowPos` をその場で読め、出す・比べる・消すの仕組みが要らず、「読んだ時点の値」が厳密に返る（要件 4.6 が自動で満たされる）。
- 難しい所:
  - **ゴーストの道で成り立たない**。`GetProperty` は任意のスレッドから同期で呼ばれ、他のスレッドへ問い合わせない約束（`shiori_host.rs:29-36`・`218-220`）。World は UI スレッドの物で、そこへ問い合わせると新しい待ち合わせ（返りを待つ）の約束になり、要件 5.1・5.2 と食い違う（要件の議題 1 の推しの理由と同じ）。
  - MCP だけ World を直接読み、ゴーストの道は読めない（または別の値）という形は要件 6.6（両方の道で同じ値）と要件 7.2（後から足す値も両方の道で読める）に反する。
  - 後の 6 本（`shell-scale-linkage` など）が同じ口に値を足すときも、名前ごとに World の読み方を MCP の側へ書くことになり、「口が 1 つ」（要件 7.1）にならない。

## 4. 要件ごとの対応表

| 要件 | 今あるもの | 足りないもの | 印 |
|---|---|---|---|
| 1.1〜1.2 キャラクターの窓の矩形 | `GhostWindows`・`WindowPos` | 集めて値の置き場へ出す所 | Missing |
| 1.3〜1.5 数の系と形 | `WindowPos` が物理の画素・仮想画面の座標（DPI 対応 v2） | 「左,上,右,下」の文字列にする純関数（右＝左＋幅） | Missing（小） |
| 1.6 窓の外枠で答える | `WindowPos`＝窓の外枠（枠なしの窓） | 外枠が見えている絵より広い場面がある（3 節 a） | Constraint（議題） |
| 1.7 重なった 2 窓 | 同じ値になる（特別な処理は要らない） | — | — |
| 2.1〜2.3 バルーンの矩形 | `GhostWindows::balloon_window`・`balloon_target` | 同上 | Missing |
| 3.1 無いスコープ | `GhostWindows` に無い番号は `None` | 値を出さない | — |
| 3.2〜3.4 出ていない・まだ置かれていない | `target_visible`（`None`/`false`） | 可視を読む所（`emo2_boot` の中だけ） | Missing |
| 3.5 値なしの答え | MCP・ゴーストの道とも `NotFound` で既存の文 | — | — |
| 3.6 前の値を残さない | — | 値を消す効果 | Missing |
| 3.7 書式が読めない名前 | `resolve_dotted_str` が `NotFound` | — | — （`scope(00)` は `scope(0)` と同じ名前に読まれる＝R4） |
| 4.1〜4.5 動いた後 | どの経路も最後は `WindowPos` に載る（areka の書く口・OS の返り） | フレームの最後に比べて出す所 | Missing |
| 4.6 画面の更新の後の読み | — | 出してから読めるまでの間の約束 | Unknown（5 節） |
| 4.7 混ぜない | 1 つの名前の値は 1 つの文字列で、写しの差し替えは丸ごと | — | — |
| 5.1〜5.3 読みは待たない | 読みはロック内の `Arc` 複製だけ | 出す側も UI スレッドを待たせない形 | Constraint |
| 5.4 記録を流さない | アクターは書く効果で記録を出さない | 新しい効果も `debug!` 以下に | Constraint |
| 6.1〜6.3・6.5 ゴーストごと・保存しない | 置き場は実行系ごと・保存は `PersistPut` を通るものだけ | 作り直した置き場への出し直し | Missing |
| 6.4 保存しない | 新しい区画を保存の投影（`build_initial_image`・`actor.rs:612-627`）に入れなければ保存されない | — | — |
| 6.6 両方の道で同じ値 | 両方とも同じ読み手 | — | — |
| 7.1〜7.4 口 | — | メッセージ・効果・区画・読み手の引く順 | Missing |
| 8 決定論のテスト | 先例: `actor_actor_integration_tests.rs`（`barrier` で反映を待つ・時計なし）・headless の World に `GhostWindows` と `WindowPos` を置く placement のテスト群 | 表示の側の可視を偽で差す口 | Missing |
| 9 実機と文書 | `doc/pasta-real-run.md:181・184・185・239`・`property.toml:401-412・739-750` | 改める文 | Missing（文書） |

## 5. 実装の案

### 案 A: 値の置き場に「動く値の区画」を足し、UI スレッドが毎フレーム比べて出す（拡張）

- sylphya: `MirrorImage` に区画を 1 つ足す（例: 点付きの動く値の区画）。メッセージを 1 種足す（例: 区画を丸ごと差し替える `PublishLive`）。効果を 1 種足す（区画の差し替え）。読み手は、引く順に新しい区画を入れる（per-asker か大域か・どの順かは設計で）。保存の投影には入れない。
- areka: `emo2_boot` に新しいファイルを 1 つ足し、毎フレームの系で `GhostWindows` を回して、キャラクターの窓とバルーンの窓の `WindowPos` と表示の側の可視から「名前 → 4 つの数の文字列」の表を作り、前に出した表と違うときだけ `PersistWiring.publisher` で出す。
- 長所: 読む道を変えない・後の 6 本が同じ表に値を足すだけで済む（`shell-scale-linkage` の `scaling`・`currentmonitor.dpi` もここで作れる）・ゴーストの道の約束に触れない。
- 短所: 出してから読めるまでの間（要件 4.6）・出す先が替わったことの検知・`emo2_boot/mod.rs` の 1 行。

4.6 の詰め方は 3 つ考えられる:

| 小案 | 中身 | 長所 | 短所 |
|---|---|---|---|
| A-1 投げっぱなしのまま | 今の作りどおりアクターに任せる | 変えるものが一番少ない・UI スレッドを待たせない | 「画面の更新の後に受けた読み」に古い値が返る隙間が理屈の上で残る（ふつうは数 μs）。要件 4.6 の文を「有界の遅れ」に寄せるか、テストで `barrier` を使って確かめる形になる |
| A-2 出した後に `barrier()` で待つ | UI スレッドが反映を待つ | 4.6 を厳密に満たす | 変わったフレームごとに UI スレッドがアクターを待つ（要件 5.3「窓の移動・描画を遅らせない」と当たりうる）。アクターが落ちていると `Err` で戻るだけなので止まりはしない |
| A-3 動く値だけ別の写しにし、UI スレッドが直接差し替える | 動く値の区画を、アクターを通さない別の `SharedMirror` 相当にして、UI スレッドが唯一の書き手になる。読み手は 2 つの写しを読む | 出した時点で読める＝4.6 を厳密に満たす・待たない | 値の置き場の「書き手はアクター 1 本」の作りに 2 本目の書き手を足す（区画を分けるので衝突はしない）。`SylphyaReader::new` の形が変わる（呼び手 24 か所・多くはテスト）。`sylphya-sqlite-kvs`（v0.2.0）の作り直しと形を合わせる必要がある |

### 案 B: 読む時に問い合わせる（新しい口）

- MCP の `get_property` で、名前が 2 つの名前なら World から直接組んで答える。ゴーストの道は UI スレッドへ問い合わせて返りを待つ、または答えない。
- 長所: MCP は厳密に今の値。出す・比べる・消すの仕組みが要らない。
- 短所: ゴーストの道の再入の約束（`shiori_host.rs:29-36`）と要件 5.1・5.2・6.6・7.1・7.2 に食い違う。要件定義のモデルを Fable へ戻す条件（requirements.md の議題 1）に当たる。後の 6 本の土台にならない。

### 案 C: 組み合わせ（MCP は World を直接、ゴーストの道は案 A の置き場）

- MCP の読みは `get_property.rs` で World から組み（4.6 が厳密）、同じ組み方の関数で毎フレーム置き場へも出す（ゴーストの道用）。
- 長所: MCP の自動試験が一番欲しい「直後に読んで新しい値」が厳密になる。
- 短所: 同じ時点で 2 つの道の値が違う隙間が残る（要件 6.6 に当たる）。読む道が 2 本になり、要件 7.2（読む道を変えずに足せる）にも当たる。`mcp/get_property.rs` はウェーブの ⑦ が「触らない」と約束した側で、④ の持ち物だが、名前の振り分けを足すと膨らむ。

## 6. ウェーブ C11 の約束との当たり（roadmap.md 189 行目）

④ `mcp-scope-rect-property` に許された所: `crates/areka-sylphya/src/{actor,mirror,reader}.rs`・`vocab/dotted.rs`・`crates/areka-ghost/src/{sylphya_wiring,runtime}.rs`・新しい `emo2_boot` のファイル・`crates/areka/src/mcp/get_property.rs`・`property.toml` の行・`doc/pasta-real-run.md`。約束: sylphya の `persist/` に触らない・`main.rs`・`emo2_boot/mod.rs`・`emo2_boot/frame.rs` に触らない。

当たる所:

1. **新しい `emo2_boot` のファイルには `emo2_boot/mod.rs` の `mod` 宣言が 1 行要る**（`emo2_boot/mod.rs:15-62` の並び）。表示の側の欄が `pub(in crate::emo2_boot)`（`frame/wiring.rs:57`）なので、`emo2_boot` の外（例: `mcp/` や `placement/`）に置くと読めない。逃げ道は `#[path]` で別の場所から読む形だが、`emo2_boot` の子にはならないので欄が読めない。→ **「`mod` の 1 行だけ足す」を約束の例外として開発者に諮るか、`frame/wiring.rs` に読み取りの関数を 1 つ足すか**（`frame/wiring.rs` は誰の持ち物の表にも無いが、⑥ の `frame.rs` の子）。
2. **毎フレームの系の登録の場所**。先例は `ghost_session::register_systems`（`ghost_session.rs:63-91`）で、④ の表に無い（他の 7 本の表にも無い＝重なりは無い）。`emo2_boot` の中の既存の登録関数（`register_emo2_frame_system`）は `emo2_boot/mod.rs` か `frame.rs` に在り、どちらも約束の内。→ `ghost_session.rs`（917 行）に 1 行足すのを許すか、を諮る。
3. `window_rect_of`（`placement/follow/window_move.rs:1104`）は私有。① と ⑥ が「触らない」と約束した 1,221 行のファイルで、公開に変えるのも避けたい → 新しいファイルで同じ判定（`CW_USEDEFAULT` の印・大きさ 0 以下）を書く。判定の重複は小さいが、片方だけ直る危うさがある（`structure.md` の「複製は本文の同一性を壊す」）。
4. 案 C は `mcp/get_property.rs` を大きくする。④ の持ち物なので約束には当たらないが、⑦ の案 2 の足し分が次のウェーブで同じファイルに来る。
5. `PersistWiring` を読むのは `placement/persist.rs` の公開の欄を読むだけで、⑥ の持ち物のファイルを書き換えない。

## 7. Research Needed（設計の段へ持ち越す）

- **R1（要件の議題 2）**: SSP が、出ていないバルーン・`\s[-1]` のスコープ・ゴーストに無いスコープの `rect` に何を答えるか。SSP の MCP で測る（今日は SSP が起きていなかった）。
- **R2（要件の議題 3 の材料）**: 着せ替えの部品がオフで本体の外へはみ出す面で、SSP の窓（と `rect`）がその分を含むか。areka は含む（3 節 a）。ぱすたの面 1000 に本体の外へはみ出す着せ替えがあるかも、areka の記録の窓の大きさと SSP の `831x1026`（150%）で比べられる。
- **R3**: SSP の段 1 で `scope(0)` と `scope(1)` が同じ矩形だったのは、2 つの面の外形がたまたま同じだからか。areka の記録の `\0` と `\1` の窓の大きさで確かめる（実機の確かめ要件 9.2 の読み方に効く）。
- **R4**: `currentghost.scope(00).rect`・`scope(+1)` など、正準化で `scope(0)` に寄る書き方を SSP がどう扱うか（areka の読み手は `ByIndex` に読んで同じ名前にする）。ukadoc は書いていない。要件 3.7 の文に当たるかだけを確かめる（小）。
- **R5**: 4.6 の約束の強さ（5 節の A-1〜A-3）。MCP の読みは次のフレームの Input 段、出すのは前のフレームの最後、の間にアクターの反映が間に合わない場面が実際にあるか。決めは要件の文の強さ次第。
- **R6**: 集め手を置く段（`FrameFinalize` など）と、その段で表示の側（`Emo2Wiring`）が World に戻っていること。`emo2_frame_system` は `Update` 段で外して戻す（`frame.rs:298・399`）ので、それより後の段なら読める見込み。ドラッグ（Input 段）・`\![move]`（Update 段）・OS の返り（窓の手続き）のどれもそれより前に `WindowPos` へ載る。

## 7.1 SSP の実測（2026-10-23 19:50 頃・R1・R4 の答え）

SSP 2.9.23・窓際のぱすたさん（手すきの状態＝`get_status` が空・バルーンは出ていない）。SSP の MCP の `get_property` で読んだ。`\s[-1]` は 1 回だけ送り、すぐ `\0\s[1000]\e` で元へ戻した（reload・設定の変更・ゴーストの切替はしていない）。

| 聞いた名前 | 状態 | SSP の答え |
|---|---|---|
| `currentghost.scope(0).rect` | 面 1000 を出している | `1710,376,2818,1744` |
| `currentghost.scope(1).rect` | 面 10 を出している | `1710,376,2818,1744`（scope(0) と同じ） |
| `currentghost.balloon.scope(0).rect` | **バルーンは出ていない** | `1790,492,2662,1392`（値がある） |
| `currentghost.balloon.scope(1).rect` | **バルーンは出ていない** | `838,376,1710,1276`（値がある） |
| `currentghost.scope(0).rect` | **`\0\s[-1]` の後**（`surface.num` は `-1`） | `1710,376,2818,1744`（隠す前と同じ） |
| `currentghost.balloon.scope(0).rect` | `\0\s[-1]` の後 | `1790,492,2662,1392`（同じ） |
| `currentghost.scope(2).rect` | ゴーストに無いスコープ（`scope.count` は `2`） | **`0,0,0,0`** |
| `currentghost.scope(-1).rect` | 負の番号 | **`0,0,0,0`** |
| `currentghost.balloon.scope(2).rect` | ゴーストに無いスコープ | `NG:Cannot find such property name.` |
| `currentghost.scope(x).rect` | 番号でない | `NG:Cannot find such property name.` |
| `currentghost.scope(00).rect` | 前ゼロ | `1710,376,2818,1744`（scope(0) と同じ・R4） |

読めること:
- SSP は「画面に出ているか」で答えを変えない。面を隠したスコープも、出ていないバルーンも、**窓の矩形をそのまま答える**（OS の窓が在る限り、その矩形）。
- ゴーストに無いキャラクターのスコープは `0,0,0,0`、無いバルーンのスコープは `NG`。
- R2（オフの着せ替えの部品の分を窓に含むか）は、ぱすたさんに外へはみ出す着せ替えが無いので測れなかった。

## 8. 規模と危うさ

- **規模: M（8〜12 タスク）**。sylphya の口（メッセージ・効果・区画・読み手・檻）で 3〜4、集め手（純関数の組み立て・毎フレームの比べと出し直し・偽の可視の口）で 3〜4、MCP とゴーストの道の通しのテスト 1〜2、実機の確かめと文書 1〜2。
- **危うさ: 中**。値の置き場は既にある作りに足すだけで、依存も足さない。ただし ⑴ 出してから読めるまでの間（要件 4.6）の約束の強さ、⑵ 作り直した置き場への出し直しの漏れ、⑶ ウェーブの約束（`emo2_boot/mod.rs`・登録の場所）の 3 つは設計で必ず決める必要がある。

## 9. 設計の段への申し送り

- 推しの形は案 A。4.6 の詰め方（A-1〜A-3）と、ウェーブの約束の 2 点（`mod` の 1 行・系の登録の場所）は要件ディスカッションで諮る。
- 動く値の区画は「丸ごと差し替え」にすると、消す（7.4）・揃える（4.5・4.7）・作り直した置き場への出し直し（6.3）が 1 つの形で済む。後の spec が値を足すときは、集め手の表に行を足す形になる（集め手の作りを後の 6 本が使える形にしておく）。
- 「画面に出ているか」は表示の側の `target_visible` で決め、OS の窓の有無では決めない（3 節 a）。
- `Changed<WindowPos>` に頼らない（2.3 の印を立てない書き込み）。毎フレーム比べる。

## 10. 要件ディスカッションの決め（2026-10-23）と、設計の段への申し送り（改め）

決めの本文は `requirements.md` 末尾の表。ここでは 1〜9 節のどこが変わるかと、設計で詰める所を並べる。

### 10.1 決まった形（案 D: プル＋材料のプッシュ）

- **矩形はプル**: sylphya の読み手が、名前（`currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect` の形）に登録された読み手を、読まれた時に呼ぶ。読み手は areka の側が登録し、OS の窓の矩形（`GetWindowRect` の類）を 1 回の問い合わせで取る。sylphya は OS のコードを持たない（完了 spec の要件 8.3 の注入シーム）。
- **材料はプッシュ**: 「ゴーストごとのスコープ → キャラクターの窓・バルーンの窓」の表を、窓を作る・壊す・ゴーストを替える・`reload` の時点で、読み手が引ける所へ出す。
- **「出ているか」は要らない**: SSP の実測（7.1 節）で、面を隠した窓・出ていないバルーンも窓の矩形を答えると分かった。表示の側の `target_visible` を読む必要が無くなり、`emo2_boot` の中に集め手を置く理由（2.3 節・6 節 1）が消える見込み。
- **無いスコープ**: キャラクターは `0,0,0,0`、バルーンは値なし（SSP と同じ）。
- **要件 4.6・4.7**: 読んだ時点の OS の値を 1 回の問い合わせで返すので、5 節の A-1〜A-3 の詰めは要らない。
- **退けた物**: 毎フレーム比べる集め手（5 節の案 A）・`WindowPos` を正本にすること（一次情報源ではなく、拡大率の違う画面へ移るときに補正した値を先に書く・`wintf/.../window_pos.rs` の中心保持の補正）・値を消す効果（矩形を写しへ載せないので要らない）。

### 10.2 設計で詰める所

1. 読み手の登録の形（名前の形で引く表・per-asker か大域か・読み手の型〔`Fn(&PropPath) -> Option<String>` の類〕・`SylphyaReader` の作り方への影響〔呼び手 24 か所〕）と、`resolve_dotted_str` の中で読み手をいつ引くか（写しより先か後か）。
2. 「スコープ → 窓」の表をどこに置き、誰が出すか（`GhostWindows` を作るのは ⑥ の持ち物 `placement/spawn.rs`。新しいファイルで `GhostWindows` の差し替えを見て出せるか・`reload` の `rebuild_runtime_in_place` で新しい sylphya へ出し直す道）。触るファイルと理由を挙げる（要件の Boundary Context）。
3. 窓の印（HWND）を他のスレッドから使うときの扱い（ゴーストの道の読み手はどのスレッドから呼ばれてもよい・窓が壊れた後の印で読んだら「窓を持たない」と同じ答え〔要件 3.5〕）。
4. `GetWindowRect` が窓を持つスレッドを待たないことの確かめ（Win32 の作りからの判断で、まだ実測していない）と、DPI 対応 v2 のプロセスで物理の画素が返ることの実機テスト（要件 8.5）。
5. `reader.rs` の冒頭・`mirror.rs` の `SharedMirror::load` のコメントの直し（要件 7.8）。
6. ウェーブの約束（6 節）の当たりの組み直し: 集め手が要らなくなったので、`emo2_boot/mod.rs` の `mod` の 1 行・`ghost_session.rs` の登録が要るかを改めて調べる。

## 11. 設計の段の調べと決め（2026-10-23・`/kiro-spec-design -y`）

### 11.1 まとめ

- **調べの種類**: 軽い調べ（今ある sylphya と窓の作りへの足し）。新しい依存は足さない。外の資料は Win32 の `GetWindowRect` の説明だけを引き直した。
- **使った手引き**: `kiro-spec-design` の決まり（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と、このセッションの「最小で全部を満たす」方針（ponytail）。ほかのスキルは使っていない（画面・使い勝手の作りが無い）。
- **要の発見**:
  1. 読み手を **sylphya の読み口（`SylphyaReader`）に登録する**形にすると、読み口を作る関数 `SylphyaReader::new` の形も、アクター（`actor.rs`）も、MCP の `mcp/get_property.rs` も、ゴーストの道の `shiori_host.rs` も変えずに済む。読み口の複製はどれも同じ登録の表を共有する。
  2. 「スコープ → 窓」の表の中身は OS の窓の印（HWND）で、sylphya は OS を知らない（要件 7.2）。だから表は **sylphya に登録した読み手が持つ置き場**へ出す。sylphya はその中身を読まない。
  3. 実行系を作るたびに必ず通る口 `ghost_session_runtime.rs` の `rewire_runtime_consumers`（初回の起動・切替・`reload` の 3 つとも、実行系を置き場 `GhostSlot` へ付ける**前**に呼ばれる）に 1 行足せば、新しい sylphya に読み手を登録し、その時点の表を同期で載せられる。`reload` の出し直しはこれで済む。

### 11.2 10.2 節の 6 点の答え

| # | 問い | 答え | 根拠 |
|---|---|---|---|
| 1 | 読み手の登録の形・引く順 | 名前の「形」（選択子を `(*)` に寄せた正準の文字列＝`currentghost.scope(*).rect`）で引く表を `SylphyaReader` の中に持つ（`Arc<RwLock<BTreeMap<形, Arc<dyn PullSource>>>>`）。読み手の型は `fn read(&self, path: &PropPath) -> Option<String>`。区画は大域（sylphya は実行系ごとに 1 つなので問い手で分ける要が無い）。**写しの区画を先に引き、無いときだけ読み手を引く**（読み手を足しても今ある答えが変わらない＝要件 7.5 が作りで満たされる）。`SylphyaReader::new` の形は変えない（呼び手 24 か所に触らない） | `reader.rs` の `resolve_dotted_canonical`・`mirror.rs`・呼び手の数は 10.2 節 |
| 2 | 表の置き場と出し手 | 表は areka の新しいファイル `ghost_session_scope_rect.rs` の置き場（`RwLock<ScopeWindowTable>`）に置き、sylphya に登録した 2 つの読み手（キャラクター用・バルーン用）が共有する。出し手は ⑴ `rewire_runtime_consumers` からの結線（実行系を作るたび・同期で今の表を載せる）⑵ `FrameFinalize` の系（`GhostWindows` が変わった・窓の印が付いた〔`Added<WindowHandle>`〕・外れた〔`RemovedComponents<WindowHandle>`〕ときだけ表を組み直し、違えば差し替える）。⑥ の持ち物 `placement/spawn.rs` は読むだけ（公開の `GhostWindows::scopes`・`char_window`・`balloon_window`） | `ghost_session_runtime.rs` の `rewire_runtime_consumers`・`rebuild_runtime_in_place`・`ghost_session.rs` の `boot_wired`・`register_systems`・先例 `placement::spawn::register_ghost_windows_click_through`（`Added<WindowHandle>` を `FrameFinalize` で読む） |
| 3 | HWND を他のスレッドから使う扱い | 表には HWND の生の値（`isize`）だけを置く。読み手は値を写し取ってから表の鍵を外し、OS へ 1 回聞く。壊れた窓の印なら `GetWindowRect` が失敗する＝「窓を持たない」と同じ答え（要件 3.5）。窓の entity が消えたとき、wintf は `WM_CLOSE` を**投函**するだけ（`window_handle.rs` の `on_window_handle_remove`）なので、OS の窓はその直後の数フレームだけ残りうる。表からは同じフレームの `FrameFinalize` で外す | `crates/wintf/src/ecs/window/window_handle.rs` |
| 4 | `GetWindowRect` は待たないか・物理の画素か | Microsoft の説明は送るメッセージを書いていない（`GetWindowText` のように窓のスレッドへ送る関数は説明に書く）＝窓を持つスレッドの返事を待たない読み取り。**「GetWindowRect is virtualized for DPI」**＝呼んだスレッドの DPI の扱いで値が変わる。プロセスはモニタごとの v2（`wintf/src/runtime/mod.rs` の `SetProcessDpiAwarenessContext`）だが、ゴーストの道の呼び手のスレッドが自分の扱いを変えている場合に備え、本物の読み手は呼ぶ間だけスレッドの扱いを v2 にして戻す（`SetThreadDpiAwarenessContext`・先例 `crates/areka/tests/window_link_real_run.rs`）。右と下は窓の外の最初の画素（「the bottom-right coordinates of the returned rectangle are exclusive」）。枠の見えない縁を含むことがあるのは枠のある窓だけで、ゴーストの窓は枠の無い `WS_POPUP`（`spawn.rs` の `window_style`） | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect （2026-10-23 に引き直した）。実機の確かめは要件 8.5 の `#[ignore]` テスト |
| 5 | コメントの直し | `reader.rs` の冒頭の「OS 呼出…一切行わない」と `mirror.rs` の `SharedMirror::load` の説明を、要件 2.7・8.3 の言い方（他のスレッドを待たない・遅れうる値源を載せない・外の環境は登録された読み手を通す）へ直す。`load` 自身が鍵と `Arc` の複製しかしないことは事実なので残す | 要件 7.8 |
| 6 | ウェーブの約束 | 集め手が要らなくなり `emo2_boot` に置く理由が消えた。**`emo2_boot/mod.rs`・`emo2_boot/frame.rs`・`main.rs`・⑥ の持ち物には触らない**。触るのは `ghost_session.rs`（`mod` の宣言 1 つと系の登録 1 行）と `ghost_session_runtime.rs`（結線 1 行）で、どちらも C11 のどの spec の表にも無い | `roadmap.md` の C11 の行 |

### 11.3 引き直して分かった細かい点

- **書式の読み方**（`key.rs` の `parse_dotted`）: `scope(00)` は `ByIndex(0)`（要件 1.9 は今の読み手のまま満たす）。`scope(-1)`・`scope(x)`・`scope()`・`scope(+1)` はどれも `ByName`（中身はそれぞれ `-1`・`x`・空・`+1`）で、書式の誤りにはならない。u32 を超える番号（`scope(99999999999)`）は `BadIndex` で読み手が `warn!` を残して値なしにする（今の振る舞いのまま）。
  - → 読み手の側で、`ByName` の中身が「`-` と 1 つ以上の数字」なら**負の番号**（要件 3.1・キャラクターは `0,0,0,0`・バルーンは値なし）、それ以外の `ByName` は**書式の誤り**（要件 3.6・値なし）と読む。`scope(+1)` を SSP がどう答えるかは測っていない（値なしにする）。
- **`GhostWindows` が変わる時**: 作る時（`spawn_ghost_windows`）・1 つの窓を外す時（`remove_entry_of`）・既定の位置を追う時（`window_move.rs` の既定位置の追随・`drain_resnap.rs` の確定・`reset_pos.rs`）。後の 3 つは拡大率の遷移などシステムの置き直しの時だけで、ドラッグの毎フレームではない。表の中身（窓の印）は変わらないので、比べて差し替えない。
- **切替**: `app_exit::close_windows_for_restart` が窓の entity を消し、`GhostWindows` を外す。窓の印が外れた（`RemovedComponents<WindowHandle>`）ことで表が空になる。新しい実行系の結線は、その時点の `GhostWindows`（新しい窓の一式・印はまだ付いていないことがある）から表を組む。
- **網羅台帳の証拠**: `ukadoc-survey` の検査は `implemented` の行に証拠を 1 件以上求める（`check/finding.rs` の `ImplementedWithoutEvidence`）。証拠はソースの `// ukadoc: <URL>` の行で、URL はカタログ（`doc/ukadoc-coverage/catalog.toml`）の形 `https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.scope_28ID_29.rect:1`・`…#currentghost.balloon.scope_28ID_29.rect:1`（括弧は `_28`・`_29`）。
- **テストの道具**: `emo2_boot/ghost_switch_test_support.rs` の `SwitchRig` は本番の `boot_ghost` を通る（＝`rewire_runtime_consumers` を通る）。窓は `spawn_ghost_windows` で窓なしに組め、偽の印（`frame_test_support.rs` の `fake_handle`）を付けられる。偽の OS の読み手は World の資源（`ScopeRectReadSeam`）で差す。

### 11.4 選んだ形と退けた形（表の置き場）

| 案 | 中身 | 良い所 | 困る所 | 決め |
|---|---|---|---|---|
| X 表を写しに載せる | アクターへのメッセージで表を出し、写しの新しい区画に置く。読み手は写しから表を読む | 「書き手はアクター 1 本」を守る | 出してから写しに載るまでの間がある（実行系を付けた直後に読み手が居ない・表が空＝`0,0,0,0` が出ず「無い名前」になる隙間が要件 3.3・6.3 に当たる）。HWND を sylphya の型に載せるには型を消す（`Any`）か文字列にするしかない。`actor.rs`・`mirror.rs` の区画・効果の型が広がる | 退けた |
| Y 読み手の置き場に表を置き、読み口に同期で登録する | 本設計 | 登録も表の差し替えも同期＝実行系を置き場へ付ける前に揃う。sylphya は OS の型を知らない。アクターに触らない | 読み口に 2 つ目の鍵（登録の表）が増える（`SharedMirror` と同じ「読みの鍵 → `Arc` の複製」の形で、待ち合わせにはならない） | **採った** |

### 11.5 設計の合成（synthesis）

- **一般化**: 2 つの名前は「名前の形に読み手を 1 つ登録する」の 2 例。口は形の文字列で引く表 1 つにした。後の spec（`shell-scale-linkage` の `scope(ID).scaling` など）は読み手を足すだけ（要件 7.4）。読み手に問い手（`AskerContext`）は渡さない（今の 2 つの名前で要らない・足すときは再確認の引き金）。
- **作るか借りるか**: 置き場の差し替えは標準の `RwLock`（`SharedMirror` と同じ）。`arc-swap` などの依存は足さない。OS の矩形は `GetWindowRect`（`windows` クレートの既にある機能 `Win32_UI_WindowsAndMessaging`・`Win32_UI_HiDpi`）。
- **削ったもの**: 毎フレームの集め手・可視の読み取り（SSP の実測で要らない）・値を消す効果・写しの新しい区画・アクターのメッセージ・`mcp/get_property.rs` の振り分け・`emo2_boot` の新しいファイル。表の比べは「変わった印が立ったときだけ」で、毎フレームは比べない。

### 11.6 危うさと手当て

- **窓の印の使い回し**: 壊れた窓の HWND の値を OS が別の窓に使い回すと、表の古い値が別の窓を指す。印は同じフレームの `FrameFinalize` で外すので、残るのは数フレーム。HWND には使い回しを遅らせる数が入っている。→ 受け入れる（設計の Risks に書く）。
- **閉じている途中の窓**: `WM_CLOSE` の投函から窓が実際に壊れるまでの間に、表から外す前の読みが来ると、閉じかけの窓の矩形を返しうる。切替の間は実行系が外れている（MCP は「実行系が無い」で答える）ので、ふつうは届かない。→ 受け入れる。
- **ほかの C11 の spec と `ghost_session.rs` の `register_systems`**: 表では誰も挙げていないが、系の登録を 1 行足す spec が出ると隣り合う行の競合になりうる（行が別なので手で直せる大きさ）。
