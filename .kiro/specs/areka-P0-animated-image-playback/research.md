# ギャップ分析: areka-P0-animated-image-playback

> 2026-10-05・本ブランチ（main `ec072853` の上）のコードを読んで書いた。ビルドとテストは回していない（読んだだけ）。
> コードは「何の定義か＋ファイル」で指す。設計・実装の着手時に引き直すこと。
> この文書は材料と選択肢を並べるもので、決定はしない。決めるのは要件討議と設計である。
>
> **2026-10-05 設計の段の追記**: この文書の 1〜8 章はギャップ分析（main `ec072853` の上）のままで、一部が古くなった。⑴ `import` は棚卸㉒で `areka-P0-animated-image-import` へ切り出した。`import` についての記述（1 章の 3 項目め・2.5 節の pattern定義の行・3 章の表の 5.x の行・3.2 節・4 章の各案の `import` の文・6 章の 6・8 章の最後の項）は**引き渡し済みの材料**であり、本 spec は使わない（消さずに残す）。⑵ 議題 1〜5 は要件討議で全部決着した（requirements.md）。⑶ 設計が選んだ形と、その根拠は末尾の「設計の段の調査」に在る。

## 1. まとめ

- **土台はほぼ揃っている**。全コマはアトラスに載り（`areka-emo-atlas` の `AtlasTable::animation`）、子のサーフェスの時計（`areka-seriko` の `PartClocks`）は「親を切り替えても巻き戻らない・見えない間も進む・シェルの差し替えで捨てる」を既に満たす。足りないのは「繰り返す」「表示されたらすぐ始める」「コマの絵を名指しする」の 3 つである。
- **brief の案（動く絵を子サーフェスへ分解する）をそのまま載せると、今のコードのままでは 4 か所でつまずく**（3 章）。①2 枚目以降のコマはファイル名で引けない ②外形の計算がアニメーションのコマを数えないので、動く絵だけのサーフェスは外形 0×0 になり非表示へ落ちる ③子の時計は刻み（16 ミリ秒ごと）でしか生まれないので、表示の最初の 1 回は絵が欠ける ④作者の番号と当たらないサーフェス番号の空間が今は無い。どれも設計で解けるが、解き方で触るファイルが変わる。
- **`import` は読み手（`areka-parsers`）から要る**。pattern定義の第 3 欄は数として読まれるので、ファイル名は 0 に化けて消える。brief の「触るファイル」に `areka-parsers` の pattern定義は入っていない。
- **バルーン（議題 1）は「読み込みは届くが、時計が届かない」**。子の時計はシェル面だけで回り、鍵に面の種類（シェル／バルーン）を持たない。動かすなら時計の鍵と、隠れている間の合成の止め方に手が入る。
- **議題 2〜4 は、今の仕組みとの相性で重さが大きく違う**（5 章）。議題 3 の仮の案（丸めずに飛ばす）と議題 4 の仮の案（駆動しない）は今の関数の振る舞いそのままで、足す仕事が無い。議題 2 の「守る」は時計の開始の時刻だけで求まり軽い。議題 2 の別案 ⑵（表示し直すたびに頭から）は、子の時計の決まりと正面から当たる。
- 規模は **M〜L**、リスクは **中**（外形・最初の 1 回・合成のし直しの回数の 3 点が設計の山）。

## 2. 今あるもの（要件ごとの手がかり）

### 2.1 読み込みとアトラス（前提・変えない）

| 何 | 定義 | 中身 |
| --- | --- | --- |
| コマの一覧 | `AtlasTable::animation`（`crates/areka-emo-atlas/src/table.rs`） | 動く絵の親の `ElementId` でだけ `Some(&Animation)`。本番の呼び手は 0 か所（要件の実測どおり） |
| コマの並び | `Animation`（同上） | `frames`（0 番は親自身）・`delays_ms`（同じ長さ）・`loop_count` |
| 繰り返し回数 | `LoopCount`（同上） | `Infinite` か `Finite(NonZeroU32)` |
| 鍵からの逆引き | `AtlasTable::resolve`・`AtlasTable::with_frames`（同上） | 2 枚目以降のコマは親と同じ鍵を持ち、`resolve` は親（いちばん小さい番号）だけを返す。**2 枚目以降のコマをファイル名で引く口は無い**（`ElementId` で直接引くしかない） |
| 外形のもとになる寸法 | `AtlasEntry.original`（同上） | 各コマは絵の全体の寸法で揃っている（brief の申し送り）ので、どのコマでも同じ値 |
| 試験用の読み手 | `MemoryDecoder::insert_animated`（`crates/areka-emo-atlas/src/decode.rs`） | 実ファイルも Windows の拡張機能も要らずに動く絵を焼ける（要件 9.3 に使える） |
| 検体 | `crates/areka-emo-atlas/src/testdata/animated/README.md` | 12 個・全部 8×8 以下の単色。回数つきは `basic.apng`（合計 2 回）・`alpha.webp`（合計 3 回・待ち時間 100・0・70＝**待ち時間 0 のコマを含む**）。終わりなしは `rgb.apng` ほか |

### 2.2 アニメーションの表と時計（`areka-seriko`）

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| 表に採る語 | `AnimationTable::from_world`（`crates/areka-seriko/src/table.rs`） | `random`・`bind+random`・`sometimes`・`rarely` だけ。`always` は `Interval::Other("always")` として届き、元の綴りつきの `debug!` で落ちる |
| 引き金の型 | `LoopTrigger`（同上） | `Random{k}`・`BindRandom{k}` の 2 つ。`k == 0` を弾く検査が `let (Random{k} \| BindRandom{k}) = trigger` の形で書かれており、**3 つ目の引き金を足すとここがコンパイルで止まる**（直す場所が分かりやすい） |
| 表の材料 | 同上 | **`EmoWorld` だけを受け取り、アトラスは受け取らない**。動く絵のコマの数・待ち時間は表を作る場所から見えない |
| コマの進み方 | `frame_at`・`FrameStatus`（`crates/areka-seriko/src/timeline.rs`） | 開始からの経過と待ち時間の累積で今のコマを決める純粋な関数。**繰り返しの考えが無い**（末尾で止まるか、`-1` で消える）。待ち時間 0 が続くと同じ時刻を共有し、後ろのコマが今のコマになる（＝通り過ぎたコマは出ない） |
| 一番上の再生 | `LoopRuntime::on_tick`・`Playback`（`crates/areka-seriko/src/looper.rs`） | 始まるのは 1000 ミリ秒の境目の抽選だけ。`on_surface_changed` がサーフェスの切り替えで再生を全部捨てる |
| 子の時計 | `PartClocks`・`PartAnim`（`crates/areka-seriko/src/parts.rs`） | 鍵は（スコープ, 部品の番号, animation の番号）。`Playing{started_at_ms}` は開始の時刻を持ち続け、見えない間も進んだ扱い。`clear` はシェルの表の差し替えでだけ呼ばれる |
| 子の時計が生まれる場所 | `PartClocks::advance`（同上） | 刻みの、しかも境目を跨いだ回の抽選でだけ生まれる。`PartClocks::peek`（切り替えの直後）は時計を作らない |
| 子の経路を通すか | `AnimationTable::has_animated_parts`（`table.rs`）・`LoopRuntime::on_tick` の `with_parts`（`looper.rs`） | 動く部品が無い表では子の経路を 1 行も通らない。**通すのはシェル面だけ**（`*slot == Slot::Shell`） |
| 刻み | `spawn_loop_ticker`（`crates/areka-ghost/src/ticker.rs`） | 16 ミリ秒 |
| 同じコマなら出さない | `ScopeStates::commit_pattern`（`crates/areka-seriko/src/state.rs`） | 新しい `PatternState` が前と同じなら指令を出さない。**要件 7.4（コマが替わらない更新では合成し直さない）はこの 1 か所が既に担っている** |

### 2.3 合成（`areka-emo-compose`）

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| 今のコマの入れ物 | `PatternState`・`PatternFrame`（`crates/areka-emo-compose/src/pattern.rs`） | 一番上の欄（animation の番号 → コマ）と部品の欄（部品の番号 → animation の番号 → コマ）。**コマが指せるのはサーフェスの番号だけ**（絵を直接は指せない） |
| 描画メソッドの名前 | `ComposeMethod::from_name`・`known_method`（`crates/areka-emo-compose/src/method.rs`） | `import` は表に無く、`warn!` を出して `Unknown` へ。`is_implemented` は `Overlay` だけ |
| 命令の組み立て | `flatten_surface`・`push_static_element_ops`（`crates/areka-emo-compose/src/plan.rs`） | element定義 → 着せ替えの pattern0 とコマ、の順に積む。数字だけの element定義は子へ再帰する。コマも pattern0 も `Overlay` だけ通す |
| 外形 | `compute_extent`・`flatten_extent`（同上） | 数えるのは「element定義の絵（子へは再帰）」と「**着せ替えの種類**の animation の pattern0」だけ。**着せ替えでない animation のコマは数えない**。コマ（`PatternState`）は見ないので、アニメーションで外形は動かない |
| 外形 0×0 | `build_plan`（同上） | 数えるものが無いサーフェスは `EmptyComposition`。表示の側（`crates/areka-emo-present/src/presenter/show.rs` の `apply_show`）はこれを非表示へ縮める |
| 絵の束縛 | `bind_atlas`（`crates/areka-emo-compose/src/atlas_bind.rs`） | element定義のファイル名を `AtlasTable::resolve` で 1 度だけ引く。ファイル名でしか引かない |
| 面の画像の土台 | `apply_base_images`（`crates/areka-emo-compose/src/base_image.rs`） | `element0` の無いサーフェスに `surface<数字>.png` を層 0 の画像の element として足す。`element0` が在れば使わない（＝要件 1.3 は今のままで満たされる） |
| 見える部品 | `NestTable::visible_parts`（`crates/areka-emo-compose/src/nesting.rs`） | element定義の子・有効な着せ替えの pattern0 の先・コマの先をたどる |

### 2.4 表示と起動の結線

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| 合成の結果を覚える席 | `ComposeCache`・`ComposeKey`・`CAPACITY`（`crates/areka-emo-present/src/cache.rs`） | 鍵は（サーフェス番号, 着せ替え, `PatternState` の全体）。席は 3・古い順に追い出す。定数の変更は裁定を通すと書いてある |
| シェルの読み込み | `build_shell_target_with_boxes`（`crates/areka-emo-present/src/shell_target.rs`） | 焼く一覧は「`surfaces.txt` の element定義の絵＋土台に使う面の画像」。面の画像は「`element0` だけを持つサーフェス」を一覧に足す形で焼いている（`base_image_surface`）。`ManifestDeriver::derive`（`crates/areka-emo-atlas/src/manifest.rs`）は element定義の絵だけを集める |
| バルーンの読み込み | `build_balloon_target_from_faces`・`synthetic_surfaces_txt`（`crates/areka-emo-present/src/balloon.rs`） | 面 1 つを「`element0,overlay,<実ファイル名>,0,0` だけのサーフェス」に書き直し、シェルと同じ読み手・同じ `bake`・`EmoWorld::build` へ通す |
| 表を作る場所 | `crates/areka/src/emo2_boot/assets.rs` のシェルの資産を作る関数（`loop_table`）・`build_balloon_assets`（スコープごとの `balloon_tables`） | どちらも `AnimationTable::from_world(&emo_world)`。アトラスは同じ場所に在る（渡してはいない） |
| バルーンの見える・見えない | `apply_show` の説明（`crates/areka-emo-present/src/presenter/show.rs`）・`crates/areka/src/emo2_boot/frame/attach.rs` の `VisibilityOwnership::External` | 外が持ち主の対象では、表示の指令は合成・配置までを行い、見えるようにはしない |

### 2.5 読み手（`areka-parsers`）

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| element定義 | `Element`（`crates/areka-parsers/src/shell/model.rs`）・`decode_elements`（`decode.rs`） | 描画メソッドの欄を持たない。第 2 欄が `overlay` の行だけ採り、他は記録なしに読み飛ばす（要件 1.1 の「`overlay` の element定義」はこのままで合う） |
| pattern定義 | `Pattern`（`model.rs`）・`decode_animations`・`field_i64`（`decode.rs`） | 第 3 欄を `i64` の `surface_id` に入れる。**数でない欄は 0 になる**。ファイル名を入れる欄は無い |
| interval | `normalize_interval`（`decode.rs`）・`Interval`（`model.rs`） | 完全一致で `bind`・`random`・`bind+random` だけを見分ける。他は `Interval::Other(元の綴り)`。`bind+always` は `Other("bind+always")` になる |

## 3. 要件と今のコードの差（Requirement-to-Asset Map）

印: **無**＝仕組みが無い／**縛**＝今の作りが縛りになる／**不明**＝設計で調べる／**済**＝今のままで満たす

| 要件 | 使えるもの | 差 |
| --- | --- | --- |
| 1.1・1.2 自動アニメーション | `AtlasTable::animation`・子の時計・`apply_base_images`（`surface*.png` を element にする） | **無**: 動く絵の element をコマの列へつなぐ場所。**縛**: 表（`AnimationTable::from_world`）はアトラスを知らない |
| 1.3 `element0` が在る | `apply_base_images` | **済** |
| 1.4 位置と重ね順 | `push_static_element_ops` | 置き換え方しだい（下の 4 章） |
| 1.5・1.6 待ち時間・累積 | `frame_at`・`PartAnim::Playing` | **無**: 繰り返し。累積で決める形は既に在る |
| 1.7 外形を変えない | `flatten_extent` | **縛**: 着せ替えでない animation のコマを数えない。動く絵を「コマだけのサーフェス」にすると外形から消える（絵がそれだけのサーフェスは 0×0 → 非表示） |
| 1.8 当たり判定 | `SurfaceMaster.collisions`（静的） | 矩形は変わらない。**不明**: 透明な画素でクリックを通す判定は合成結果のバイトから作る（`CacheEntry.mask`）ので、コマごとに形が変わる。要件の「当たり判定の領域」が矩形だけを指すのかを設計で確かめる |
| 1.9 子・pattern の先でも同じ | `PartClocks`・`NestTable::visible_parts` | 子へ分解する案なら自然に満たす |
| 1.10 作者のアニメーションと並ぶ | 一番上の欄と部品の欄は別 | **済**（欄が分かれている） |
| 1.11 GIF・縮んだ絵 | `animation` が `None` | **済** |
| 1.12 作者の番号を変えない | — | **縛**: サーフェス番号は全部 `u32`（`SurfaceMaster.id`・`PatternFrame.surface_id`・`ElementKind::Surface`）。作者が使わない空間は今は無い |
| 2.1 終わりなし | — | **無** |
| 2.2・2.3 回数つき（仮） | `LoopCount`・`PartAnim::Playing` | 開始の時刻と合計の時間から「N 回を過ぎたら最後のコマ」が求まる（状態を足さずに済む） |
| 2.4 待ち時間 0（仮） | `frame_at` | **済に近い**: 今の関数が既に「同じ時刻のコマは後ろが勝つ」 |
| 2.5・4.6 合計 0 | — | **無**: 繰り返しを「経過を合計で割った余り」で作ると 0 で割る。表を作るときの検査が要る（`k == 0` の検査と同じ場所・同じ形） |
| 2.6 乱数を使わない | `frame_at` は純粋 | **済** |
| 3.1 初めての表示で 1 枚目から | `PartClocks::advance` | **縛**: 時計は境目の抽選でしか生まれない。「見えたら始める」が無い |
| 3.2〜3.6 巻き戻さない・揃う・スコープ別・捨てる | `PartClocks` の鍵と `clear` | 子へ分解し、同じ画像ファイルを 1 つの子にまとめれば**済** |
| 3.7 見えない間は描き直さない | `advance` は見える部品だけ触る | **済** |
| 4.1 抽選を待たずに始める | — | **無**: 一番上（`LoopRuntime::on_tick`）と子（`PartClocks::advance`）の 2 か所に要る |
| 4.2 繰り返す | — | **無**（同じ 2 か所） |
| 4.3 切り替えで止めて頭から | `on_surface_changed` | **済**（一番上の再生は切り替えで捨てられる） |
| 4.4 子では巻き戻らない | `PartClocks` | **済** |
| 4.5 途中の `-1` | `FrameStatus::Stopped` | **縛**: 今は時計を消す。繰り返しでは「消してから頭へ戻る」に変える |
| 4.7 ほかの語と並ぶ・抽選を変えない | 乱数の消費の順が決まっている（`looper.rs` の冒頭の説明） | `always` が乱数を引かなければ、既存の決定論テストの期待値は動かない（確かめる点） |
| 4.8・4.9 ほかの語・組み合わせ | `AnimationTable::from_world` の `Other` の腕 | **済**（`always` を完全一致で拾えば、`bind+always` は今までどおり落ちる） |
| 5.1〜5.5 `import` | — | **無**: 読み手の欄（ファイル名）・描画メソッドの名前・コマの展開。**縛**: `PatternFrame` は絵を直接指せない |
| 5.6 ファイルが無い | `bake` の失敗の一覧（`ShellTarget.bake_errors`） | 焼けなかった絵は一覧に載り `warn!` が出る。pattern定義の側の記録は新しく要る |
| 5.8 `import` だけが名指しする絵 | `base_image_surface` の手口（`shell_target.rs`） | 同じ手口（その絵だけを持つサーフェスを焼く一覧に足す）で `ManifestDeriver` を変えずに済む見込み |
| 5.9 上限 | `bake` | **済**（同じ `bake` を通れば効く） |
| 6（仮） バルーン | 同じ読み手・同じ `bake`・スコープごとの表 | **縛**: 子の経路はシェル面だけ。`PartClocks` の鍵に面の種類が無い（同じスコープのシェルとバルーンで番号が重なると時計が混ざる）。5 章の議題 1 |
| 7.1・7.2・7.7 動かないシェル | `has_animated_parts` の門・`PatternState` が空なら前と同じ | 動く絵・`always`・`import` が 0 のときに新しい経路を 1 行も通らない門を保てば**済** |
| 7.4 | `commit_pattern` | **済** |
| 7.5・7.6 回数と席 | `CAPACITY = 3` | **不明**: コマが 4 枚以上の動く絵は、コマが替わるたびに席から外れて CPU で合成し直す。数字は測って決める |
| 8 記録 | 表を作るときに 1 回だけ記録する型（`k == 0`・コマが空） | 同じ場所に足せる。**注意**: シェルの表は起動で 1 回、差し替えのたびに 1 回作られる（「読み込み 1 回につき 1 回」に合う）。面の表（`ShellTarget::build_world`）はスコープの数だけ組まれるので、記録をそちらに置くとスコープの数だけ出る |
| 9 テスト | `MemoryDecoder::insert_animated`・時刻と乱数を外から渡す檻（`looper_tests.rs` ほか）・`log_capture_kit` | 型は揃っている |
| 10 記録と申し送り | 網羅台帳の 3 項目（`doc/ukadoc-coverage/ledger/assets.toml` の `element*`＝縮退・`import`＝absent・`always`＝語彙のみ）・`doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」と、その中の「動く絵（APNG・動く WebP）の読み込み」の節 | 書き足す場所は在る |

### 3.1 brief の案をそのまま載せたときにつまずく 4 か所

brief の Approach 1 は「コマを 1 枚ずつ持ち `always` で順に指すサーフェスを作り、動く絵の element を、その子サーフェスを指す element に置き換える」である。

1. **2 枚目以降のコマを element にできない**。`bind_atlas` はファイル名で引き、`AtlasTable::resolve` は親しか返さない。コマ 1 枚ごとのサーフェスを作るなら、`ElementId` を直接束縛する口が要る。
2. **外形から消える**。子サーフェスが「element定義を持たず、`always` の animation のコマだけ」だと、`flatten_extent` は何も数えない。親に他の絵が在れば外形が縮むだけ（要件 1.7 に反する）、動く `surface0.png` だけのサーフェスなら 0×0 で非表示になる。逆に 1 枚目のコマを element として残すと、透明な部分を持つ絵で 1 枚目が後ろに透けて見える（コマは重ね済みの全体の絵なので、下に何も敷いてはいけない）。
3. **最初の表示で絵が欠ける**。サーフェスを表示する指令（`ScopeStates::apply`）は空のコマで出る。子の時計は次の刻みまで生まれず、`refresh_parts`（`PartClocks::peek`）は時計を作らない。静止画なら出ていた絵が、最長 16 ミリ秒のあいだ出ない。
4. **番号の空間**。サーフェス番号は `u32` の全域を作者が使える。合成したサーフェスを `u32` の上の方へ置くなら「作者が使っていない」ことを読み込みで確かめる必要があり、`\s[番号]` で直接呼べてしまう。面の一覧をなめる処理（`EmoWorld::dangling_pattern_targets`・`NestReport`・箱の表の畳み込み `fold_boxes`）にも現れる。上限いっぱい（1,024 コマ）の絵 1 つで 1,025 個のサーフェスが増え、面の表はスコープの数だけ組まれる。

### 3.2 `import` の今の読まれ方（直さないと残る副作用）

> 【引き渡し済み】この節は `areka-P0-animated-image-import` の材料である（2026-10-05 棚卸㉒）。本 spec は `import` に触らない。

`animation0.pattern0,import,glow.png,100,10,20` は、今は `method = "import"`・`surface_id = 0`（ファイル名が数でないため）・`wait = 100` と読まれる。その結果:

- `EmoWorld::dangling_pattern_targets`（`crates/areka-emo-compose/src/world.rs`）は「サーフェス 0 を指すコマ」と数える。サーフェス 0 が無いシェルでは「相手の面が無い」の `warn!` が出る。
- `ManifestDeriver::resolve_indirect` と `NestReport` もサーフェス 0 への参照としてたどる。
- 欄 2 がサーフェス番号でない描画メソッドを除く判定（`targets_animation_id`・`world.rs`）に `import` は入っていない。

`import` を入れるときは、読み手にファイル名の欄を足したうえで、この 3 か所から `import` の行を外す必要がある。

## 4. 実装の進め方の候補

### 案 A: 子サーフェスへ分解する（brief・開発者確定の方向）

- 読み込みの後、動く絵 1 つにつき「コマ 1 枚ずつのサーフェス」と「それを `always` で順に指す子サーフェス」を合成し、動く絵を指す element を子サーフェスを指す element（`ElementKind::Surface`）へ置き換える。
- `always` は `LoopTrigger` に足し、`frame_at` に繰り返しの版を足す。`import` は表を作るときに「冒頭の待ち＋コマの列」へ展開する。
- **触る場所**: `areka-seriko` の `table.rs`・`timeline.rs`・`looper.rs`・`parts.rs`／`areka-emo-compose` の `plan.rs`（外形）・`atlas_bind.rs`（`ElementId` の直接の束縛）・`world.rs` か `fold.rs`（合成サーフェスを足す場所）・`method.rs`／`areka-emo-present` の `shell_target.rs`／`areka-parsers` の `model.rs`・`decode.rs`（`import`）／`crates/areka/src/emo2_boot/assets.rs`。
- ✅ 開発者の決めた形そのもの。時計は 1 種類（animation の時計）のまま。要件 3（巻き戻さない・揃う・スコープ別・捨てる）と 1.9（子でも同じ）が子の時計の決まりからそのまま出る。
- ❌ 3.1 の 4 か所を全部解く必要がある（`ElementId` の束縛・外形・最初の 1 回・番号の空間）。サーフェスの数が最大で絵 1 つあたり 1,025 個増える。`plan.rs` は 821 行で、1 ファイル 1,000 行の上限に近い。

### 案 B: 「絵のコマ番号」の欄を足す（サーフェスは合成しない）

- `PatternState` に 3 つ目の欄（動く絵 → 今のコマ番号）を足し、`push_static_element_ops` が element の `ElementId` を `AtlasTable::animation(id).frames[番号]` へ差し替える。時計は `areka-seriko` の中に「動く絵の時計」（鍵＝スコープ × 画像）として持つ。
- ✅ サーフェスも番号の空間も合成しない。欄が空なら 0 番（親）＝静止画と同じ絵なので、最初の 1 回が欠けない。外形は親の寸法のまま（`flatten_extent` を変えない）。バルーンにも同じ欄がそのまま届く。
- ❌ brief の確定事項（子サーフェスへ分解・3 つ目の時計は作らない）から離れる。時計は seriko の中に在るが、animation の時計とは別の種類が 1 つ増える。`import` は「pattern定義から絵を置く」ので、この欄だけでは描けず、結局コマが絵を指す口が別に要る。合成の鍵（`ComposeKey`）が欄 1 つぶん大きくなる。

### 案 C: A の骨格のまま、コマが絵を直接指せるようにする（折衷）

- 動く絵 1 つにつき合成するサーフェスは **1 個だけ**（子サーフェス）。その `always` の animation のコマは、サーフェス番号でなく絵（`ElementId`）を直接指す（`LoopFrame`・`PatternFrame` の指す先を「サーフェス番号か、絵か」の 2 通りにする）。`import` のコマも同じ口で絵を指す。
- 外形は、子サーフェスに「この絵の寸法」を持たせて `flatten_extent` に数えさせる。最初の 1 回は、`always` の時計を「見えた時点で始める」規則にして `peek` でもコマを出す（または時計が無い間は 0 番を出す）ことで埋める。
- ✅ 開発者の形（子サーフェス・時計は 1 種類）を保ちつつ、3.1 の ①（ファイル名で引けない）とサーフェスの数の膨らみが消える。`import` と自動アニメーションが同じ口を使う。
- ❌ `PatternFrame` の形が変わる（`NestTable::visible_parts`・`flatten_surface`・`pattern_frame` など、コマの `surface_id` を読む場所を全部見直す）。番号の空間（3.1 の ④）は絵 1 つにつき 1 個ぶん残る。外形と最初の 1 回は A と同じく設計で解く。

### 3 案に共通して要るもの

- `always` の引き金と繰り返し（`table.rs`・`timeline.rs`・`looper.rs`・`parts.rs`）。合計 0 の検査。
- 表を作る場所へアトラスを渡す（`AnimationTable::from_world` の呼び手は起動・差し替え・テストに在る）。または、動く絵の事実を面の表（`EmoWorld`）の側に先に写しておく。
- `import` の読み手の欄と、3.2 の 3 か所の手当て、`import` だけが名指しする絵を焼く一覧へ足すこと。
- 動かないシェルで新しい経路を通らない門（`has_animated_parts` と同じ考え方）と、それを数えるテスト。

## 5. 議題ごとの材料

### 議題 1: バルーンの面の動く絵を動かすか（要件 6）

- **文書の食い違いの実物**:
  - brief（本 spec・2026-10-01）の Desired Outcome と Scope の In に「バルーンの面に置いた動く絵も動く」。
  - 完了 `areka-P0-animated-image-decode` の requirements.md（`.kiro/specs/completed/areka-P0-animated-image-decode/requirements.md`）の Out of scope に「バルーンの絵を動かすこと。…コマを時間で切り替える仕組みは本 spec でも後続でも作らない。画面に出るのは 0 番のコマ 1 枚である」。同じ文書の要件 5.4 は「バルーンの絵に動く APNG・WebP が置かれているときは、シェルの絵と同じに読んで全コマを焼き、今までの鍵で 0 番のコマを出す（議題 4 の裁定『バルーンも同じ』・2026-10-04）」。冒頭には「討議で足した 2 件＝バルーンの動く絵〔要件 5.4〕」とあり、**2026-10-04 の討議でバルーンを話題にしたうえでの文面**である（brief より後）。
- **正典**: C1（`element*` の項）が名指しするのは `surface*.png` と element定義で、どちらもシェルの `surfaces.txt` の項。ukadoc MCP を「APNG」で引いた 4 件（`element*`・`import`・シェルのファイル構成・SSP ヘルプの画像ビューワ）に、バルーンの絵のアニメーションを定める記述は無い。SSP ヘルプの画像ビューワの表は APNG・WebP を「SSP 全域（シェルやバルーンの素材などにも使える）」と書くが、これは読める形式の話で、バルーンで動くとは書いていない。
- **コードの現状**:
  - 読み込み: バルーンの面は `element0,overlay,<実ファイル名>` のサーフェスとしてシェルと同じ `bake` を通る（`build_balloon_target_from_faces`）。全コマは既にアトラスに在る。
  - 表: スコープごとに `AnimationTable::from_world` で作られる（`build_balloon_assets`）。面の表は `EmoWorld::build`（面の画像の土台の手順を通らない版）で組まれる。
  - 時計: 子の経路はシェル面だけ（`LoopRuntime::on_tick` の `with_parts`・`LoopRuntime::refresh_parts`）。`PartClocks` の鍵に面の種類が無い。バルーンの面の切り替え（`ScopeStates::apply_balloon`）は部品の欄を載せ直さない。
  - 見える・見えない: バルーンは外が持ち主の対象なので、表示の指令で勝手に見えるようにはならない（要件 6.3 は今の作りで守られる見込み）。一方、seriko はバルーンの窓が隠れているかを知らない（知っているのは `\b[番号]` で決まる面の状態だけ）。**隠れている間もコマが替われば合成までは走る**（要件 6.4 を満たすには止める仕組みが要る）。
- **裁定で変わる仕事**:
  - 「動かす」: 子の時計をバルーン面でも回す（鍵に面の種類を足す）・バルーンの面の切り替えで部品の欄を載せ直す・隠れている間の合成を止める・バルーンの検体と実機の確かめ。案 B なら時計の鍵と隠れている間の止め方だけで済む。
  - 「動かさない」: バルーンの側は何も足さない。案 A・C では「動く絵を子サーフェスへ置き換える手順」をバルーンの面の表に掛けないだけで変更 0 になる。完了 spec の文面とも合う。
- **見ておく点**: `emo2` を含むリポジトリ内の検体のバルーンに動く絵は 0 枚（brief の申し送り「今のリポジトリの検体には動く絵が 0 枚」）。どちらに裁定しても今ある検体の見た目は変わらない。

### 議題 2: 自動アニメーションでファイルの繰り返し回数を守るか（要件 2.2・2.3）

- **正典**: `import` の項だけが「繰り返し回数は無視される」と書く（C4）。`element*` の項（C1）は回数に触れない。ukadoc MCP を「繰り返し回数」で引いて当たるのは `import` の項と `\![set,tasktrayicon,…,--runcount=繰り返し回数]`（通知領域のアイコン・「指定がない場合は無限に繰り返す」）と YAYA の 1 件だけで、自動アニメーションの回数を定める記述は無い。
- **コードの現状**: 読み込みは `LoopCount::Infinite`／`Finite(n)` を渡すだけ。検体に回数つきが 2 種在る（`basic.apng`＝合計 2 回、`alpha.webp`＝合計 3 回）ので、どの案でもテストの材料は在る。
- **案ごとの重さ**:
  - 仮の案（守る・止まったら動かし直さない）: 子の時計の `Playing{started_at_ms}` から「経過 ≥ 合計の時間 × N なら最後のコマ」と求まる。状態を足さない。時計はシェルの差し替えまで捨てられないので、要件 2.3（サーフェスを切り替えても動かし直さない）は子の時計の決まりからそのまま出る。**作者から見ると「起動して数秒で止まり、表情を替えても二度と動かない」絵になる**点は、裁定のときに伝える材料。
  - 別案 ⑴（無視して常に繰り返す）: 仕組みが `always` 1 つで済み、回数の分岐とそのテストが要らない。`import` の扱い（正典が無視と明記）と揃う。
  - 別案 ⑵（表示し直すたびに頭から）: 子の時計は「見えなくなった・また見えた」を覚えない作り（見えない部品には触らない）。この案は時計に「見えていたか」を持たせる必要があり、要件 3（巻き戻さない）とも完了 `areka-P0-surface-element-nesting` の時計の決まりとも当たる。
- **一番上のサーフェス自身の `always`**（要件 4.3）は切り替えで頭から始まる。自動アニメーションを「子の `always`」として作ると子の決まり（巻き戻らない）になり、同じ `always` でも置き場所で振る舞いが違う。これは要件どおりだが、対応表（要件 10.2・10.3）に並べて書く材料になる。

### 議題 3: 待ち時間 0・極端に短い待ち時間（要件 2.4）

- **正典**: 黙っている（ukadoc MCP で `element*`・`import` の項に待ち時間の下限の記述は無い）。
- **コードの現状**:
  - 読み込みは 0 を 0 のまま渡す（brief の申し送り）。検体 `alpha.webp` は待ち時間 100・0・70 で、0 のコマを持つ。
  - `frame_at` は今でも「待ち時間 0 が続くと同じ時刻を共有し、後ろのコマが今のコマ」になる（`timeline.rs` のテスト `frame_at_all_zero_waits_selects_last_index` が固定している）。**仮の案（丸めない・通り過ぎたコマは飛ばす）は、今の関数の振る舞いそのまま**である。
  - 刻みは 16 ミリ秒なので、16 ミリ秒より短いコマは丸めなくても画面には出ない（飛ばされる）。
  - 合計が 0 の絵は、繰り返しを余りで求めると 0 で割る。要件 2.5 の検査が要る（どの案でも）。
- **別案（短い待ち時間を決めた値へ引き上げる）の重さ**: 表を作るときに 1 か所で値を書き換えるだけで軽い。ただし (a) ファイルどおりの速さでなくなる（要件 2 の Objective と当たる）(b) 引き上げる値と閾値に正典の根拠が無い (c)「絵を見る道具の多くがそうしている」は本分析では裏を取っていない（**要調査**・設計で一次の資料を当たる）。
- **性能の面**: 待ち時間が短くコマが 4 枚以上の絵は、刻みのたびに席から外れて合成し直す（`CAPACITY = 3`）。引き上げは合成の回数を減らす効果も持つが、それは要件 7.5 の測定で数字を見てから決められる。

### 議題 4: `bind+always` などの組み合わせ（要件 4.9）

- **正典**: C6「SSPのみ+区切りで列挙する事で組み合わせ指定が可能」。組み合わせの相手や順を限る記述は無い（`always+bind` の順も書ける）。ukadoc MCP は 1 語の部分一致だけなので「bind+」では 0 件、材料は C6 だけである。
- **コードの現状**:
  - 読み手（`normalize_interval`）は `bind`・`random`・`bind+random` を**完全一致**で見分ける。`bind+always` は `Interval::Other("bind+always")` になる。
  - 表（`AnimationTable::from_world`）は `Other` の中身を完全一致で見る（`sometimes`・`rarely`）。`always` も完全一致で足せば、`bind+always` は今までどおり元の綴りつきの `debug!` で落ちる。**仮の案（駆動しない・変更 0）は足す仕事が無い**。
  - 「着せ替えの種類か」の判定は 3 か所に在る: 合成の `is_bind_interval`（`plan.rs`・pattern0 の静的な絵と外形）・`NestTable` の `bind_ids`（`nesting.rs`）・seriko の着せ替えの番人（`parts.rs` の `gate`・`looper.rs` の抽選と進行）。どれも `Interval::Bind`／`BindRandom` と `LoopTrigger::BindRandom` を型で見ている。
- **別案（`bind+always` だけ入れる）の重さ**: 引き金を 1 つ足し（着せ替えが有効な間だけ繰り返す）、上の 3 か所に「これも着せ替えの種類」と教える。`Other("bind+always")` を文字列で見るか、読み手の `Interval` に型を足すか（`#[non_exhaustive]` なので足せる）を決める。`+` の順の違い（`always+bind`）と、3 つ以上の組み合わせをどこまで読むかも決めることになる。この範囲は後続 `areka-P0-seriko-trigger-intervals`（残りの語と組み合わせを引き受ける）と重なる。

## 6. 設計で調べること（要調査）

1. **外形の数え方**（要件 1.4・1.7）: 動く絵を置き換えた後も外形が静止画のときと同じになる形。roadmap に `extent-element-offset`（外形が画像の element定義の X,Y を数えないバグ・「本 spec の前が望ましい」）が未着手で載っている。本 spec は「静止画のときと同じ」を守ればよく、そのバグは直さないが、同じ関数（`flatten_extent`）を触るので順序を確かめる。
2. **最初の 1 回の絵**（要件 3.1・7.1）: 表示の指令が出てから最初の刻みまでの間に、動く絵が欠けないこと。`always` の時計を「見えた時点」で始める規則を、一番上（`LoopRuntime`）と子（`PartClocks::advance`・`peek`）の両方でどう書くか。
3. **合成サーフェスの番号の空間**（要件 1.12）: `u32` の中で作者と分ける方法、`\s[番号]` で呼ばれたときの扱い、面の一覧をなめる処理（相手の無いコマ・入れ子の報告・箱の表）から外すかどうか。
4. **合成し直す回数と席の数**（要件 7.3〜7.6）: コマが 4 枚以上の絵で、コマが替わるたびに合成し直す時間が 16 ミリ秒に収まるか。前後の数字の採り方は `areka-P0-recompose-budget` の測り方（`cache.rs` の冒頭が引く `remeasure-2026-08-15.md`）に合わせる。
5. **クリックを通す判定の形**（要件 1.8）: 合成結果の透明度から作る判定がコマごとに変わることを、要件の「当たり判定の領域」に含めるか。
6. **`import` の細部**: 末尾のコマの後（抽選の語のとき、最後のコマを残すか消すか）・1 つの animation に `import` とほかの pattern定義が混ざるとき（要件 5.10）・`surface.append` の行にだけ現れる `import` の絵（roadmap の覚え書きに「`surface.append` の行にしか現れない絵のファイル名は焼かれない」が在る）。
7. **`always` が乱数を引かないこと**（要件 4.7）: 既存の決定論テスト（まばたき）の乱数の消費の順が動かないことを確かめる。
8. **起動の時間**: 動く絵を持つシェルは採寸と資産の組み立てで 2 回焼かれる（brief の表）。roadmap の `placement-measure-bake-once`（未着手・「本 spec の前が望ましい」）との順序。本 spec の検体は 8×8 なので、検体では数字に出ない。
9. **短い待ち時間を引き上げる道具の実例**（議題 3 の別案を採る場合だけ）。

## 7. 規模とリスク

- **規模: M〜L**（brief の見立て 15〜20 タスクのまま）。`always` と繰り返しは既存の型の延長で小さい。自動アニメーションの置き換え・外形・最初の 1 回、`import` の読み手からの追加、性能の測定がそれぞれ中くらい。議題 1 が「動かす」なら上限側。
- **リスク: 中**。新しい技術は無く、土台（全コマ・子の時計）は完了済み。山は ①外形と最初の 1 回（静止画のときと同じ絵・同じ寸法を保つ）②合成のし直しの回数（測るまで分からない）③`emo2` の既存の照合を書き換えずに通すこと（門の置き方）の 3 つ。
- **並走の注意**: 同じ合成の場所を触る `balloon-element-order`・`self-alpha-declaration`・`element-base-method`・`element-clipping-option`・`extent-element-offset` とは同時に走らせない（roadmap の列の決まり）。`crates/areka/src/emo2_boot/spine.rs` は 1,000 行ちょうどで、表を作る呼び出しを持つ。ここの署名を変えるなら行数に注意する。

## 8. 設計へ渡すこと

- 進め方は、開発者確定の「子サーフェスへ分解」を骨格にした **案 A か案 C** が brief と合う。案 C は 3.1 の ① とサーフェスの数を消せるが、コマの形（`PatternFrame`）を変える。案 B は brief の確定事項から離れるので、採るなら開発者の裁定が要る。
- 要件討議で先に決まると設計が軽くなる順: **議題 1**（バルーン＝時計の鍵と範囲が変わる）→ **議題 2**（回数＝仕組みが 1 つで済むか）→ 議題 4 → 議題 3（後ろの 2 つは仮の案なら足す仕事が無い）。
- 要件には無いが設計で必ず触る場所: `areka-parsers` の pattern定義（`import` のファイル名の欄）と、3.2 の 3 か所。brief の「触るファイル」に足す。


---

# 設計の段の調査（2026-10-05・main `82607b5f` を取り込んだ後）【選んだ形は却下・末尾の「作り直し」が上書き】

> コードを読んで確かめた（ビルドとテストは回していない）。コードは「何の定義か＋ファイル」で指す。

## Summary

- **Feature**: `areka-P0-animated-image-playback`
- **Discovery Scope**: Extension（今ある合成と seriko への足し込み。外部の依存 0・軽い調査）
- **Key Findings**:
  - brief の案（動く絵をコマ 1 枚ずつのサーフェスと子サーフェスへ分解する）は、同じウェーブの約束（`plan.rs` に触らない）の下では要件を満たせない。コマが届くまで絵が丸ごと消える場面（バルーンの装着・シェルの差し替えの最初の表示）が残り、外形も数えられない。
  - 合成の入口 `Composer::compose_into`（`crates/areka-emo-compose/src/lib.rs`）は約束の外に在り、`build_plan` を呼ぶのはここ 1 か所である。命令の絵の番号をここで今のコマへ替えれば、位置・重ね順・外形は静止画のときのまま、コマだけが替わる。
  - シェルの表示は必ず seriko から出る（装着は出さない）ので、seriko は見えているシェルの面をいつも知っている。バルーンは逆で、装着が面 0 を確立し、見える・見えないは可視性の相だけが決める。seriko へ窓の見える・見えないを知らせる線が 1 本要る。

## Research Log

### 子サーフェスへ分解する形が約束の下で成り立つか

- **Context**: brief の Approach 1 と、同じウェーブ C4 の約束（`plan.rs`・`fold.rs`・`method.rs`・`areka-emo-present`・`assets.rs` に触らない）の両方を満たす形を探した。
- **Sources Consulted**: `flatten_surface`・`push_static_element_ops`・`flatten_extent`・`build_plan`（`crates/areka-emo-compose/src/plan.rs`）／`NestTable::visible_parts`（`nesting.rs`）／装着の手順（`crates/areka/src/emo2_boot/frame/attach.rs`）／`RebasedShow`（`crates/areka-seriko/src/output.rs`）／`ScopeStates::apply`・`apply_balloon`（`state.rs`）。
- **Findings**:
  - 合成は、コマ（`PatternState`）が空のとき、着せ替えでない animation の絵を 1 枚も描かない。静的に描かれるのは element定義の絵と、有効な着せ替えの pattern0 だけ。
  - コマが静的な絵を「置き換える」のは、有効な着せ替え（番号が着せ替えの集合に在る）の pattern0 に対してだけ。着せ替えの集合は seriko と装着が持つ値で、合成の側から足せない。しかも着せ替えの層は element定義の後に積まれるので、element定義の番号の順（要件 1.4）を保てない。
  - element定義の絵は必ず描かれ、コマはその上に重なるだけ。1 枚目を element として残すと、コマ（重ね済みの全体の絵）の透明な所から 1 枚目が見える。
  - 外形は「束縛の在る画像の element の原寸」「element定義の子」「全部の着せ替えの pattern0 の先」だけを数える。着せ替えでない animation のコマだけのサーフェスは 0×0。
  - 空のコマで合成される場面が実在する: バルーンの装着（面 0・空のコマ・見える・見えないは外が持ち主）／シェルとバルーンの差し替えの最初の表示（`RebasedShow` はコマを運ばない）／起動の採寸。
  - 装着はシェルの最初の表示を出さない（注記「シェルは初回 ShowSurface を attach で発行しない」）。seriko は `\b[番号]` が来るまでバルーンの面を知らない。
- **Implications**: 「コマが届かなければ絵が無い」という状態の持ち方そのものが、1 フレーム遅らせる解を取らない決まりと当たる。「何も届かなければ 1 枚目・届けばそのコマ」という持ち方に変える必要がある。`plan.rs` の今の決まりの中にはその持ち方を表す手段が無い。

### 試して捨てた抜け道

- **使われない着せ替えの animation を子サーフェスに足して外形に数えさせる**: 外形は取れるが、作者の `bindgroup` の番号と当たると 1 枚目が重なって見える。外形の計算の癖に寄りかかるので、後続 `areka-P0-extent-element-offset` が外形を直すと壊れうる。コマが届くまで絵が無い問題は解けない。
- **置き換えの前の外形を覚えておき、`Composer` でその外形を使う**: 外形は解けるが、コマが届くまで絵が無い問題は解けない。
- **アトラスに「原寸だけ持つ空の絵」を足す**: 外形は解けるが、読み込みとアトラスは本 spec の範囲外（要件の Boundary Context）。コマが届くまで絵が無い問題は解けない。
- **seriko にシェルの最初の面も知らせる**: シェルは seriko が出すまで表示されないので要らなかった。

### バルーンの窓の見える・見えないを seriko が知る方法

- **Context**: 要件 6.1（見えている間は動く）・6.4（隠れている間は描き直さない）・2.3（隠れていたバルーンが出たら回数つきは始め直す）。
- **Sources Consulted**: `run_balloon_visibility_phase`・`issue_actions`（`crates/areka/src/emo2_boot/balloon_visibility_phase.rs`）／`run_status_report_phase`（`crates/areka/src/emo2_boot/frame/status_report.rs`）／`EmoPresenter::target_visible`・`current_surface_id`（`crates/areka-emo-present/src/presenter/read.rs`）／`GhostSession::seriko_sink`（`crates/areka/src/ghost_session.rs`）。
- **Findings**: 見えているかの真実は表示層の照会。フレームの終わりに照会から組を作り、前と違うときだけ運行の側へ送る相が既に在る。seriko の送り手は `GhostSession` だけが持ち、差し替えの相が同じ口を借りている。
- **Implications**: 同じ形の相を 1 つ足し、seriko へ（開いているか・面の番号）を送る。可視性の判断には触らない。

### 既存の決まりを壊さないか

- **Findings**:
  - バルーンの面の表は今は必ず空（`crates/areka-emo-present/src/balloon.rs` の `synthetic_surfaces_txt` が `element0` だけのサーフェスを作る）。バルーンの面の進行を広げても、今ある振る舞いは変わらない。
  - 乱数の消費の順は、抽選の輪の対象の並びで決まる。`Always` を乱数を引く前に飛ばし、抽選の対象（`shown_slots`）を変えなければ、既存の決定論テストの期待値は動かない。
  - `ComposeKey`（`crates/areka-emo-present/src/cache.rs`）は `PatternState` を丸ごと持ち等しさで比べるので、欄を足しても鍵は正しく分かれる。perf の記録の `key_hash`（`crates/areka-emo-present/src/presenter/timing.rs` の `compose_key_hash`）は一番上の欄しか混ぜておらず、足す欄も混ざらない（今も部品の欄は混ざっていない）。
  - `AtlasBinding` を読むのは `plan.rs` だけ。束縛を別のコマへ付け替えても、ほかの読み手は居ない。
  - `crates/areka/src/emo2_boot/spine.rs` は 1,000 行ちょうど。テストの接続宣言は `mod.rs`（883 行）に置く。
  - `areka-seriko` は `areka-emo-atlas` に依存していない。動く絵の事実を数と文字列だけの型で渡せば、`Cargo.toml` を触らずに済む。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
| --- | --- | --- | --- | --- |
| 子サーフェスへ分解（brief の案） | コマ 1 枚ずつのサーフェス＋ `always` で指す子サーフェス | 手で書いた定義と同じ形 | 約束の下では不成立（空のコマで絵が消える・外形・番号の空間・`\s[番号]` で呼べる） | `plan.rs` を直せば成り立つが、約束を解く必要がある |
| 絵のコマの欄＋合成の入口で差し替え（採用） | `PatternState` に「動く絵 → コマの番号」を足し、`Composer::compose_into` が命令の絵を替える | 空の欄＝ 1 枚目なので絵が欠けない・外形と重ね順は `plan.rs` のまま・番号を作らない・約束の 7 か所に触らない | brief の言い方（子サーフェス）から離れる。`import` は別に命令を出す所が要る | 時計は部品の時計と同じ型・同じ持ち主 |
| コマが絵を直接指す（ギャップ分析の案 C） | `PatternFrame` の指す先を「サーフェスか絵か」に広げる | `import` と口を共有できる | `plan.rs` の変更が必須（約束に反する） | 不採用 |

## Design Decisions

### Decision: 動く絵は「絵のコマの欄」で運び、合成の入口で差し替える

- **Context**: 要件 1.1〜1.13・7.1 と、同じウェーブの約束。
- **Alternatives Considered**:
  1. 子サーフェスへ分解 — 上の表のとおり不成立。
  2. 約束を解いて `plan.rs` を直す — 開発者の判断が要り、同じウェーブの `areka-P0-element-base-method` と場所が重なる。
- **Selected Approach**: `bind_atlas` の直後に動く絵の事実（`FilmFacts`）を面の表へ載せ、seriko の時計が `PatternState` の絵のコマの欄を進め、`Composer::compose_into` が `build_plan` の後に命令の絵の番号を今のコマへ替える。
- **Rationale**: 「何も届かなければ 1 枚目」が状態の持ち方から出るので、絵が欠ける場面が 0 になる。外形・位置・重ね順は静止画と同じ計算のまま。サーフェスの番号を作らないので、要件 1.12 が形から満たされる。
- **Trade-offs**: brief の「子サーフェス」の言い方から離れる（設計討議の議題）。ギャップ分析（上の 8 章）は、これに近い案 B を「採るなら開発者の裁定が要る」と書いている。perf の記録の `key_hash` は欄を数えない。
- **Follow-up**: 設計討議で開発者の確認を取る。roadmap の本 spec の行の言い方を直す。

### Decision: 繰り返しは「経過を周期で割る」1 つの計算で決める

- **Context**: 要件 1.6・2.1・2.2・2.4・2.9・4.2（時刻は正確に扱う・開発者裁定 2026-10-05）。
- **Selected Approach**: `lap_of`（何周目か・周の頭からの経過）を共有し、`always_at`（pattern定義の待ち＝出す前の待ち）と `film_frame_at`（画像の待ち＝出しておく時間）がその上に載る。状態は開始の時刻だけ。
- **Rationale**: 今のコマが「刻みの時刻 − 開始の時刻」だけで決まるので、刻みの遅れを持ち越す状態が無い。回数つきの「止まった」も経過から求まり、状態を足さない。
- **Trade-offs**: 待ち時間の数え方の違いは 2 つの関数に分かれる（共有するのは周の計算だけ）。

### Decision: 時計は「見えた最初の刻み」で生まれ、無い間は経過 0 として見る

- **Context**: 要件 3.1・4.1 と、1 フレーム遅らせる解を取らない決まり。seriko は刻みでしか時刻を知らない。
- **Alternatives Considered**:
  1. 表示の指令のときに、直前の刻みの時刻で時計を作る — 刻みが 1 度も来ていないと時刻が無い。始まりが最大 1 刻み早まる。
  2. seriko に時計を持たせる — アクターの作り方が変わり、決定論テストの注入の口が増える。
- **Selected Approach**: 表示の指令には経過 0 の絵（動く絵は 1 枚目・`always` は待ち 0 のコマ）を載せ、時計は次の刻みの時刻で作る。
- **Rationale**: 絵は切り替えの指令と同時に出る（0 フレーム）。規則が 1 つで、時刻が無い場合の分岐が要らない。
- **Trade-offs**: 1 枚目が最大 1 刻みぶん長く見える（1 度きり・積み上がらない）。設計討議の議題 2。

### Decision: 回数つきの絵の「始め直し」は、見えなくなった時計を捨てることで作る

- **Context**: 要件 2.3・2.7・2.8。
- **Selected Approach**: 回数つきの絵の時計は、評価（刻み・切り替えの直後・窓の知らせ）で「見えていない」と分かったら捨てる。終わりなしの絵と `always` の時計は捨てない。
- **Rationale**: 「見えていたか」を別に覚えずに済む（時計が在る＝途切れずに見えている）。始め直すかどうかが、繰り返し回数だけで決まる（要件 2.8）。

### Decision: 記録は seriko の表が出す

- **Context**: 要件 8.4（読み込み 1 回につき 1 回）。面の表はスコープの数だけ組まれる。
- **Selected Approach**: 合成の側は事実（`FilmFacts::skipped`）を返すだけ。`AnimationTable::from_world` が `warn!` を出す。
- **Rationale**: シェルの表は読み込み 1 回につき 1 回だけ作られる（`crates/areka/src/emo2_boot/assets.rs` の今の呼び方）。

### 設計の 3 つの見直し（まとめる・作るか使うか・削る）

- **まとめる**: 動く絵・一番上の `always`・部品の `always` は「見えたら乱数なしで始まり、経過を周期で割る」同じ問題。計算と「時計なし＝経過 0」の規則を 1 つにした。シェルとバルーンは面の種類を鍵に持つだけで同じ経路。
- **作るか使うか**: 外部のライブラリで解く部分は無い。今ある `frame_at`・`PartAnim`・`commit_pattern`・`NestTable::visible_parts`・届けの相の形をそのまま使う。
- **削る**: サーフェスの番号の割り当て・`\s[番号]` の遮り・外形を覚える表・シェルの最初の面の知らせは、採った形では要らないので作らない。合成の結果を覚える席の数は測るまで変えない。

## Risks & Mitigations

- brief の案から離れたことを開発者が認めない — 設計討議の最初の議題にする。別の道（約束を解いて `plan.rs` を直す）の材料は上の Research Log に在る。
- バルーンの面のコマが替わるたびに文字の層が作り直される — E2E と実機で確かめる。起きたら `areka-emo-present` の側の直しになるので、止めて報告する。
- コマが 4 枚以上の絵で合成が 16 ミリ秒に収まらない — 測って数字を残し、開発者へ報告する（席の数は変えない）。
- `plan.rs` の「命令が出る条件」が同じウェーブの spec で変わる — 束縛の付け替えの前提を design.md の Revalidation Triggers に挙げた。マージの後にテストを回す。

## References

- [descript_shell_surfaces `element*`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#element*%2C%E6%8F%8F%E7%94%BB%E3%83%A1%E3%82%BD%E3%83%83%E3%83%89%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D%2CX%E5%BA%A7%E6%A8%99%2CY%E5%BA%A7%E6%A8%99) — 自動アニメーションの正典（requirements.md の C1〜C3）
- [descript_shell_surfaces `always`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#always) — C5
- `.kiro/specs/completed/areka-P0-animated-image-decode/` — コマ・待ち時間・繰り返し回数の渡し方
- `.kiro/specs/completed/areka-P0-surface-element-nesting/` — 部品の時計の決まり


---

# 設計の段の調査（作り直し・2026-10-05 設計討議の裁定の後）

> 上の「設計の段の調査」が選んだ形（合成の後で絵の番号を差し替える）は、設計討議で**却下された**。開発者「いや、本質的な案で設計せよ。スコープが膨らむなら関係しそうな他セッションと調整。」上の節は、子サーフェスの形が今のコードのどこでつまずくかの材料として残す（「Research Log」の事実は今も正しい）。選んだ形・Design Decisions は、この節が上書きする。
> 同じウェーブの約束（`plan.rs`・`areka-emo-present` ほかに触らない）は設計の縛りから外れた。触るファイルは design.md の「触るファイルと並走の重なり」に全部挙げた。

## Summary

- **Feature**: `areka-P0-animated-image-playback`
- **Discovery Scope**: Extension（合成・seriko・表示層・結線にまたがる）
- **Key Findings**:
  - 子サーフェスの形の 4 つのつまずきは、どれも「`always` というものを合成が知らない」ことから来る。`always` を合成・外形・見える部品の 3 か所で根から扱えば、手書きの `always` と自動の分解が同じ決まりで動く。
  - pattern定義が指せるのは番号だけなので、コマ 1 枚ごとのサーフェスを作らない限り、子の定義は読み手の型（`Animation`）では書けない。部品を指す鍵に 2 つ目の種類を足し、子のコマは絵を直接指す形にした。番号の空間の問題はこれで形から消える。
  - バルーンを出すとき、表示層は最後の入力で通し直す作りになっている（`show_target`）。隠れている間の指令を「入力の記録だけ替える」にすれば、出た瞬間に正しいコマが見え、隠れている間の合成は 0 回になる。

## Research Log

### 子の形の 2 つを比べる

- **Context**: 「コマ 1 枚ごとにサーフェス」と「絵 1 つにつき子 1 つ・コマは絵を直接指す」のどちらが、自動で書かれた定義として本当か。
- **Findings**:
  - 読み手の `Pattern.surface_id` は整数で、サーフェスの番号しか運べない。面の表の `SurfaceMaster.animations` は読み手の `Animation` の列である。この型のまま子を書くには、コマ 1 枚ごとに番号つきのサーフェスが要る。
  - 番号つきのサーフェスは、面の索引（`SurfaceIndex`）・`surface_ids` をなめる 7 か所（相手の無いコマ・入れ子の報告・箱の表・当たり判定の持ち込み・箱の束の番号・見える部品・seriko の表）・`\s[番号]` の全部に現れる。上限いっぱいの絵 1 つで 1,025 個。
  - 正規化した型（seriko の `LoopFrame`・合成の `PatternFrame`）は読み手の型と別なので、子の定義を正規化した形で持てば、コマは絵を直接指せる。
- **Implications**: 絵 1 つにつき子 1 つを採る。子は面の索引に載せず、別の鍵（`PartKey::Film`）で指す。seriko の表では子は `always` を 1 本持つ部品そのものになり、部品の時計の同じ経路を通る。

### 「載っていない」と「消えている」を分ける必要

- **Context**: コマが無ければ経過 0 を描く、と決めると、`always` の途中の終わりのコマ（`-1`）の後に「何も出さない」を表せなくなる。
- **Findings**: `PatternState` の欄は「在る・無い」の 2 値。経過 0 のコマが在る `always`（最初の待ちが 0）に `-1` が続く定義は普通に書ける（出す → 待つ → 消す → 待つ → 繰り返す）。
- **Implications**: 欄の意味を 3 つにする（載っていない＝経過 0・コマ・消えている）。seriko は経過 0 と同じコマを載せないことにすると、空の `PatternState` が「全部が経過 0」と等しくなり、合成の鍵が揃う。`PatternFrame` の型と今の関数の署名は変えずに、関数を足して表す（表示層と `areka` のテストの書き換えを 0 にするため）。

### 時刻をどこまで正確にできるか

- **Sources Consulted**: `spawn_loop_ticker`・`LoopTickerConfig`（`crates/areka-ghost/src/ticker.rs`）／`SerikoMsg`・`spawn_seriko`（`crates/areka-seriko/src/actor.rs`）／`show_target`（`crates/areka-emo-present/src/presenter/visibility.rs`）／`apply_show`（`presenter/show.rs`）／`SerikoLoopConfig` を書き下している所（`mod.rs`・`spine.rs`・seriko のテスト）。
- **Findings**:
  - 刻みの時計は差し替えられるクロージャ（既定は OS の起動からのミリ秒）。seriko に同じ時計を渡せば、合図を処理した時刻で時計を作れる。`SerikoLoopConfig` に欄を足すと `spine.rs`（1,000 行ちょうど）の書き下しが壊れるので、時計つきの起動を別の関数にして、今の起動は署名を変えずに残す。
  - 窓の見える・見えないは UI スレッドが決める。送り手が同じ時計を読んで知らせに載せれば、知らせの到着が遅れても開始の時刻は正確になる。
  - `show_target` は最後の入力（`last_show`）で `apply_show` を通し直す。`apply_show` は外から所有される対象でも合成までは必ず行う。
  - 残る遅れは 2 つ: seriko → UI のスレッドの境（全部の指令に同じだけ掛かる）と、コマの替わり目を見つける刻み（16 ミリ秒）。後ろは刻みの出し手を「次の切り替えの時刻で起こす」形に作り替えれば消せるが、今のアニメーション全部に関わる。
- **Implications**: 時計の開始は出来事の時刻にする。隠れている対象の合成は先送りする。刻みの作り替えは開発者に問う（design.md の Open Questions 1）。

### 外形と `areka-P0-extent-element-offset`

- **Findings**: `flatten_extent` の画像の element の行は、累積のオフセットに原寸を足すだけで、その element 自身の X,Y を足していない（同 spec が直すバグ）。動く絵の子を別の行で数えると、直しが片方にしか入らない。
- **Implications**: 画像と動く絵の子を同じ 1 行で数える。順は本 spec が先。roadmap の「`flatten_extent` は `extent-element-offset` の持ち物」の約束を直す。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
| --- | --- | --- | --- | --- |
| 合成の後で絵の番号を差し替える（前の版） | `PatternState` に絵のコマの欄・`Composer` で差し替え | 触るファイルが少ない | 自動の分解が定義にならない。手書きの `always` と別の経路 | **却下**（開発者裁定 2026-10-05） |
| コマ 1 枚ごとにサーフェス | 読み手の型のまま書ける | 手書きと字面まで同じ | 番号の空間・1,025 個・面の一覧の全部に現れる | 不採用 |
| 絵 1 つにつき子 1 つ・コマは絵を直接指す（採用） | 鍵の種類を足す・子は `always` を 1 本持つ | 番号を作らない・大きさが増えない・`import` が同じ口に載る・手書きの `always` と同じ時計と計算 | 欄の読みに種類が増える（`Cell`）。`plan.rs` を同じウェーブの spec と分け合う | — |

## Design Decisions

### Decision: `always` を合成が知る（経過 0・外形・見える部品）

- **Context**: つまずき a・b。
- **Selected Approach**: コマの欄に何も無い `always` は、経過 0 の pattern を定義から描く。外形は全部の pattern の和集合。見える部品にも経過 0 の先を入れる。見分けは `is_always_interval`、経過 0 は `rest_index` の 1 関数ずつ。
- **Rationale**: 最初の指令から絵が出る（0 フレーム）。手書きと自動に同じ決まりが効く。合計 0 の `always` は時計なしで描ける。
- **Trade-offs**: 今 `always` を書いているシェルは外形が広がりうる（Open Questions 3）。

### Decision: 部品を指す鍵に種類を足す（`PartKey::Film`）

- **Context**: つまずき c・d、要件 1.12。
- **Alternatives Considered**: `u32` の上の方を予約する（作者が書ける・`\s` で呼べる）／コマごとのサーフェス。
- **Selected Approach**: 子は番号を持たず、面の索引に載せない。`ElementKind::Film` は読み分け（`element_kind`）からは出ない。
- **Rationale**: 当たる・隠す・呼べる道が型から無くなる。遮る検査が要らない。

### Decision: 1 つの計算 `always_at` を手書きと子が共有する

- **Selected Approach**: 子の pattern の待ちを「出す前の待ち」に直し（0, d0, d1, …）、最後のコマを出しておく時間は周期に入れる。回数は引き金が持つ。
- **Rationale**: 関数が 1 つで済む。回数つきの始め直しも「回数を持つ時計は見えなくなったら捨てる」という、子に限らない決まりになる。

### Decision: 時計の開始は出来事の時刻・隠れている対象は合成を先送り

- **Context**: 検証の指摘 3（1 フレーム遅らせる解・時刻は正確に）。
- **Selected Approach**: `SerikoClock` を刻みと seriko に同じものを渡す。窓の知らせは送り手が時刻を載せる。`apply_show` は、隠れている・外から所有される・面と着せ替えが同じ指令を、入力の記録の差し替えだけで済ませる。
- **Rationale**: 窓が出た瞬間に正しいコマが見える。隠れている間の合成は 0 回。回数つきの絵は、窓が出たフレームの時刻から始まる。
- **Trade-offs**: 隠れている間も指令は流れる（Open Questions 2）。

### Decision: 窓の知らせは届けの相（`status_report.rs`）に置く

- **Context**: 検証の指摘 1。
- **Alternatives Considered**: 新しい相（`frame.rs`・`frame/wiring.rs` に触る）／可視性の相が出す・隠すときに送る（`\b[-1]` など別の道を拾えない・`areka-P0-balloon-lifecycle-events` の持ち物）。
- **Selected Approach**: 表示層の照会の差で拾う。照会・台帳・ゴーストごとの作り直しを既に持つ届けの相に足す。
- **Rationale**: 真実が 1 つ（表示層）で、変える道がいくつ在っても 1 か所で拾える。

### 設計の 3 つの見直し（まとめる・作るか使うか・削る）

- **まとめる**: 手書きの `always`（一番上・部品）と動く絵の子は、同じ引き金・同じ計算・同じ経過 0 の決まり。シェルとバルーンは鍵に面の種類を持つだけ。
- **作るか使うか**: 部品の入口・見える部品・部品の時計・`commit_pattern`・`show_target` の通し直し・届けの相を、そのまま使う。
- **削る**: 番号の割り当てと遮る検査・コマごとのサーフェス・動く絵だけの計算（`film_frame_at`）・新しい相・表示層での時刻の計算は作らない。

## Risks & Mitigations

- `plan.rs` を `areka-P0-element-base-method` と分け合う — 取り込みの順を先に決める。外形の移動はどちらか一方だけが行う。
- 合成の先送りの条件を広げすぎる — 表示層のテストで条件の縁（面が違う・見えている・命令で見える対象）を固定する。
- 欄の読みの種類が増え、合成と見える部品の求め方がずれる — 同じ検体で「合成が描いた先」と「見える部品」を突き合わせるテストを置く（今ある `nesting_fixture_tests.rs` の型）。
- コマが 4 枚以上の絵で合成が 16 ミリ秒に収まらない — 測って報告する（席の数は変えない）。


## 再検証（2026-10-05）を受けた直し

> 作り直した版の再検証（`design-validation.md`）と、ほかのセッションから得た事実を受けて、上の「作り直し」の節の決定を次のとおり改める・足す。

### Decision: 合成の先送りは指令の入口に置き、`last_show` は変えない（上の「隠れている対象は合成を先送り」を改める）

- **Context**: 再検証の指摘 1。`apply_show` の頭で `last_show` のコマを差し替える形だと、①`show_target` の通し直しが同じ条件に掛かって合成されない ②`last_show` で合成の覚えを引く読み戻し（`EmoPresenter::last_shown`・`read_back`、MCP の `dump_balloon`）が外れる。
- **Sources Consulted**: `apply_show` を呼ぶ 4 か所（`presenter/hub.rs` の `ShowSurface` の腕・`visibility.rs` の `show_target`・`refresh.rs` の `refresh_scale`・`replace.rs`）／`last_shown`（`snapshot.rs`）／`read_back`（`read.rs`）／`dump_balloon`（`crates/areka/src/mcp/dump_balloon.rs` は `last_shown` が絵を返さないと失敗を返す）。
- **Selected Approach**: `last_show` は「最後に表示が成立した入力」のまま。預かったコマは対象ごとの別の欄に置く。預かるのは外からの指令の入口（`hub.rs`）だけで、`apply_show` は変更 0。`show_target` は預かったコマで `apply_show` を直接通す。預かったときも応答は必ず 1 回返す。
- **Rationale**: 通し直しが自分で預かる道が形から無い。読み戻しは今までどおり成立した絵を返す（完了 `areka-P0-mcp-dump-images` の後退 0）。

### Decision: 隠れていた対象を出すとき、回数つきの絵は必ず経過 0 から（再検証の案 A）

- **Context**: 再検証の指摘 3。閉じた知らせ → seriko の戻しの指令、の往復が着く前に窓が出ると、止まった最後のコマが 1〜2 フレーム見える。1 フレーム遅らせる解は取らない。
- **Selected Approach**: `show_target` が通し直すコマから、回数つきの子（面の表の `FilmSheet.laps` が在る子）の欄を外す。欄が無い＝経過 0。
- **Rationale**: 表示層は時刻も seriko も要らず、出た最初のフレームから 1 枚目が出る。
- **Trade-offs**: 飛んでいる途中の古い指令が後から着く僅かな窓は残る（design.md「残る遅れ」）。指令に世代の印を付ければ消せるが、指令の型の全部に関わるので足さない。

### Decision: 刻みと、隠れたバルーンへの指令は「決めたこと」にする

- **Context**: 前の版で開発者への問いにしていた 2 件。開発者の決まり「時刻は正確に扱う・更新が遅れたら過ぎた時間の分だけ進める」（2026-10-05）で決まっていると読む。
- **Selected Approach**: 刻みは今の 16 ミリ秒のまま（今のコマは経過から正確に決まる・遅れは積み上がらない）。隠れているバルーンへも指令は流れる（合成・描画 0 回）。未決として残すのは、手書きの `always` の外形（和集合か）の 1 件だけ。

### 調整で確かめた事実（ほかのセッションから・design.md の同名の節が正本）

- `areka-P0-element-base-method` は `plan.rs` に触らない。先に main へ入る。このウェーブで `plan.rs` の重なりは 0。
- `areka-P0-balloon-lifecycle-events` は `emo2_boot/mod.rs` の 1〜3 行だけ（今ある `clock: TalkClock` の写しを渡す）。その `clock` の名前・型・持ち主を変えない。2026-10-05 の実装の頭で順を入れ替え、本 spec の後に main へ入る（同 spec はまだ要件の段だった）。
- `areka-P0-extent-element-offset` は未着手。本 spec が先。`flatten_extent` を `plan_extent.rs` へ移したことを申し送る。

### 軽い直し（設計書へ入れたもの）

`LoopTrigger` の腕を決め打つ所は 3 か所／`NestTable` の行を載せる条件に 2 欄を足す／面の切り替え・着せ替えの変化の直後の `refresh` でも回数つきの時計を捨てる／出来事の時刻は `last_seen` に入れない／時計の開始の言い方（台本の合図は処理した時刻・窓の知らせは送った時刻）／子の命令は置いた element の描画メソッドを運ぶ／`bind_atlas` は面の表につき 1 度／`flatten_extent` は移すだけで既存の食い違いは直さない／対応表に 2 行／テスト 3 本（突き合わせ・周の境目・`NestReport`）＋読み戻しと閉じてすぐ出すテスト／バルーンの表の差し替えの後の時計は次の刻みで生まれる。


## 出番の世代（2026-10-05 設計討議の裁定「何でもかんでも先送りはよくない。２」を受けて）

> 上の「再検証を受けた直し」が「残る遅れ」に残した「飛んでいる途中の古い指令」を、本 spec の中で解く。あわせて、手書きの `always` の外形は和集合で決定とする（開発者に示して異議なし）。

### 読んで確かめたこと

- seriko の表示の指令 `DisplayCommand`（`crates/areka-seriko/src/output.rs`）は 5 種（`Show`・`Hide`・`ShowBalloon`・`HideBalloon`・`Rebased`）。出口は `emit_display`（`actor.rs`）の 1 か所で、発行先は 1 本の FIFO。`Rebased` は「定義を替えた点に 1 件並ぶ合図」で、世代（`epoch`）は UI が出した差し替えの依頼の番号を seriko がそのまま返す形である。
- 橋渡し `map_display_command`（`crates/areka/src/emo2_boot/adapter.rs`）は `DisplayCommand` を網羅して突き合わせ、`PresentCommand`（`crates/areka-emo-present/src/command.rs`）へ写す。`ShowSurface` を書き下している所は表示層と `areka` のテストに多数在る。
- シェルの面の切り替え・非表示・着せ替え・定義の差し替えは、どれも seriko が合図を受けて決め、同じ出口から順に出す。時計を捨てるきっかけを UI スレッドが先に知るのは、外から所有される対象（バルーン）を出す・隠すときだけ。

### Decision: 合図を FIFO の 1 点に並べ、表示層が「追い付くまで」を知る

- **Alternatives Considered**:
  1. 全部の表示の指令に世代の欄を足す — `ShowBalloon`・`ShowSurface` の書き下し（テストに多数）が全部変わる。シェルの指令にも意味の無い欄が付く。
  2. `Rebased` の `epoch` を使い回す — `epoch` は定義の差し替えの依頼の番号で、荷物の突き合わせに使われている。出す・隠すのたびに進めると、差し替えの突き合わせが壊れる。
  3. 世代を全体で 1 つにする — あるスコープのバルーンを出しただけで、ほかのスコープの指令まで「追い付いていない」になる。
- **Selected Approach**: 世代は表示層が外から所有される対象ごとに持ち、隠れていた対象を出すたびに進める。窓の知らせが世代を運び、seriko は新しい世代を受けたら合図 `StageAck` を先に 1 件出してから時計を作り直す。表示層は、合図が今の世代に追い付くまで、その対象へ着く指令から回数つきの子の欄を外す（指令は捨てない）。今ある指令の型は変えない。
- **Rationale**: 「出番を知る前に決めたコマ」と「知った後に決めたコマ」の境が、FIFO の中の合図 1 件で決まる（`Rebased` と同じ並びの使い方）。指令ごとの印が要らない。捨てずに欄だけ外すので、面の番号の切り替えや終わりなしの絵のコマは失われない。効くのはバルーンだけで、動く絵の無い面の表では外す欄が無いので何も起きない（`emo2` の指令の列・絵は今と同じ）。
- **Trade-offs**: 表示層と seriko の命令に 1 種ずつ足す（`PresentCommand::StageAck`・`DisplayCommand::StageAck`）。橋渡しに腕が 1 つ増える。

### シェルの側に同じ窓は在るか

- 無い。コマを決める者と、時計を捨てるきっかけ（`\s`・`\s[-1]`・着せ替え・定義の差し替え）を知る者が同じスレッド（seriko）で、指令は同じ出口から順に出る。面の切り替えの前の刻みで決めたコマは、切り替えの指令より前に並び、表示層は順に当てるだけである。定義の差し替えは `Rebased` が FIFO の 1 点に並ぶ。今日の時点で古い指令が後から着く窓は見つからなかった。出番の世代はシェルには足さない。

### Decision: 手書きの `always` の外形は全部の pattern の和集合（決定）

- **Rationale**: 自動で作った子と手書きの `always` は同じに振る舞う。表示されている間ずっと見える絵が外形で切れてはいけない。画像の element の X,Y の直しは `areka-P0-extent-element-offset` へ申し送る。

### 残る遅れ

- スレッドの境（seriko が決めたコマを UI が当てるまで）と、16 ミリ秒の刻みだけ。出し直しの直後は、seriko が始め直した後のコマが着くまで経過 0 の絵のまま（古い絵は出ない）。

## 実装前の引き直し（2026-10-06・main `3d6b0629` 取り込み後・タスク 1.1）

> 取り込みのコミットは `96be31d2`（`origin/main` の `3d6b0629`＝`areka-P0-element-base-method` の squash を含む）。`areka-P0-balloon-lifecycle-events` は待たない（2026-10-05 開発者裁定）。本ブランチの分かれ目 `54d2abc3` から `3d6b0629` までに入ったコードの変更は `element-base-method` の 1 本だけで、触ったのは `areka-parsers` の `shell/{decode,model,mod,undrawn}.rs`・`areka-emo-compose` の `fold.rs`（注記だけ）と `method.rs`（注記だけ）・`areka-emo-present` の `shell_target.rs`（`load_shell_target` に `warn!` 1 つとテストの接続）と、その兄弟のテスト。

### 引き直した所と結果

**食い違いは 0 件**。design.md「触るファイルと並走の重なり」の行を、取り込んだ後のコードで 1 つずつ読み直した。

| 所 | design の前提 | 取り込んだ後の実物 | 結果 |
| --- | --- | --- | --- |
| `plan.rs` の `push_static_element_ops` | `ElementKind` の振り分け（`Image`・`SurfaceOutOfRange`・`Surface`）で、数字だけの element定義は `flatten_surface` を `is_top_level=false` で再帰・画像は束縛と `placement` を見て `BlitOp` を積み、描画メソッドは `element.method` をそのまま運ぶ | 同じ（署名 `(out_ops, visited, world, atlas, surface_id, binds, pattern, offset_x, offset_y)`・振り分けの 3 腕・`method: element.method.clone()`）。`54d2abc3..3d6b0629` で `plan.rs` の差分は 0 行 | 一致 |
| `plan.rs` の `flatten_surface` | 重ねる番号＝「有効な着せ替えの pattern0」∪「この段にコマを持つ id」・animation-sort の 2 段・コマが在ればコマ、無ければ index が 0 の pattern0・一番上と部品は読む欄（`get`／`part_get`）だけが違う | 同じ（`merged_ids` の作り方・`frame_of` の 2 つの読み・`find(|p| p.index == 0)`） | 一致 |
| `plan.rs` の `compute_extent`・`flatten_extent` | 数えるのは束縛の在る画像の element の原寸・element定義の子（X,Y を足して再帰）・全部の着せ替えの pattern0（`min_by_key(index)`）。画像の element の X,Y は数えない（後続 `extent-element-offset`）。命令の経路と pattern0 の取り方が食い違う（既存・直さない） | 同じ（`min_by_key(|p| p.index)` と命令の経路の `index == 0` の食い違いもそのまま）。`plan.rs` は 821 行 | 一致 |
| `emo2_boot/mod.rs` の `spawn_seriko(` | `spawn_seriko(resolver, static_binds, bind_resolver, loop_config, out)` の 5 引数。台本の時計 `let clock = TalkClock::new(clock_fn);` が別に在る | 同じ（`crates/areka-seriko/src/actor.rs` の `pub fn spawn_seriko<O>` の 5 引数・`mod.rs` の呼び出し 1 か所・`TalkClock` の行も同じ） | 一致 |
| `emo2_boot/mod.rs` の `LoopTickerConfig` | 刻みの起動の `LoopTickerConfig::clock` に `seriko_clock` を渡す | `spawn_loop_ticker(LoopTickerConfig::default(), …)`。`LoopTickerConfig` は `interval: Duration`・`clock: Box<dyn Fn() -> MonotonicMs + Send>` の 2 欄とも `pub` | 一致（下の補足 2） |
| `presenter/hub.rs` の `ShowSurface` の腕 | 指令の入口で `apply_show` へ渡すだけの腕。`apply_show` を呼ぶのは 4 か所（`hub.rs`・`visibility.rs` の `show_target`・`refresh.rs`・`replace.rs`） | 同じ（腕は `self.apply_show(world, target, surface_id, binds, pattern, reply)` の 1 行・呼び出しは 4 か所）。`PresentCommand` は `ShowSurface`・`Hide`・`InvalidateCache`・`ReplaceTarget` の 4 種 | 一致 |
| `presenter/visibility.rs` の `show_target` | `last_show`（面・着せ替え・コマの 3 つ組）で `apply_show` を通し直し、応答を読んでから見えるようにする。`PresentTarget` に `ownership`（`VisibilityOwnership::External`）・`visible`・`current_surface_id`・`last_show` が在る | 同じ（`show_target(&mut self, world, target) -> Result<(), PresentError>`・`last_show.clone()` → `apply_show(..., Some(tx))` → `rx.recv()`） | 一致 |

あわせて、設計が「変更 0」「同じ形」と前提する所の差分も 0 行だった: `areka-seriko` の全ファイル・`areka-emo-compose` の `nesting.rs`・`pattern.rs`・`world.rs`・`lib.rs`・`areka-emo-present` の `presenter/`・`command.rs`・`areka` の `src/` 全部・`areka-emo-atlas`。`spine.rs` は 1,000 行ちょうどのまま。

### 補足（食い違いではないが、実装で踏む所）

1. **element定義の `base` の行も動く絵の分解の入力になる**: `element-base-method` で読み手の `decode_elements` が `is_image_element_method`（`overlay`・`base` の 2 語）の行を同じ `Element` にし、`fold.rs` の `normalize_element` はどちらも `ComposeMethod::Overlay` で置く。したがって `elementN,base,<動く絵>,X,Y` も `overlay` の行と同じに分解され、子を置く element の描画メソッドは `Overlay` を運ぶ。設計（「置いた element の描画メソッドをそのまま運ぶ」・分解は描画メソッドを見ない）のままで正しく、直しは要らない。検体（タスク 1.2）に `base` の行は無い。
2. **刻みの時計は `areka-ghost` の私的な関数**: `LoopTickerConfig::default()` の時計は `crates/areka-ghost/src/ticker.rs` の `fn real_clock()`（`GetTickCount64`・`pub` でない）。「刻みと同じ時計を 1 つ作って両方へ渡す」は、`mod.rs` で `GetTickCount64` を読む `seriko_clock` を作り、`LoopTickerConfig { interval: 16 ms, clock: Box::new(move || MonotonicMs(seriko_clock())) }` と書けば `areka-ghost` に触らずに済む（`areka` は `windows` の `Win32::System::SystemInformation` を `log_history.rs` で既に使っている）。触るファイルの表は変わらない。
3. **新しく起票された `areka-P0-draw-methods-canon`**（`element-base-method` の完了時の棚卸・優先）は、残りの描画メソッドを `method.rs`・`plan.rs` ほかで描く spec。同じウェーブで並走していないので本 spec の重なりは 0。本 spec の Non-Goal「`overlay` 以外の描画メソッド」とも矛盾しない。

### 前の数字（`emo2` の 1 コマの時間・要件 7.3）

**測り方**: 完了 `areka-P0-recompose-budget` の道具（`tools/perf/`）と同じ手順。短時間水準（7 分）・release・実の `pasta.dll` を 32bit のヘルパで載せた `emo2`＋バルーン `emo2-kakukaku`・`RUST_LOG=info,areka_emo_present=debug`（採取スクリプトの固定値）。1 コマの時間は `perf(apply_show): 段階別計時` の行の `t_total_us`（表示の指令 1 回の適用の始めから記録までの全区間）を、開始 60 秒を除いた定常状態で数える（`remeasure-2026-08-15.md` §2 と同じ数え方）。

```powershell
cargo build -p areka --release
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc --release
# ヘルパを target\release\ へ写す（README §13 の注意どおり）
cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2   # 検体は target\nar-samples\manual\emo2 に展開される
pwsh -File tools/perf/invoke-perf-run.ps1 -Profile short -Build release `
    -GhostRoot <wt>\target\nar-samples\manual\emo2\ghost\emo2 `
    -BalloonRoot <wt>\target\nar-samples\manual\emo2\balloon\emo2-kakukaku `
    -OutDir <wt>\target\perf\p0ap-before-short-release-2 -AutoQuiet
python tools/perf/judge-perf.py <out>\run.log <out>\cpu.csv --mode baseline --build release --emit-metrics
```

- **採った日時**: 2026-10-06 02:19〜02:27（日本時間・UTC 2026-10-05T17:19:34Z〜17:27:04Z）・実走 432 秒・areka の終了コード 0
- **ソース**: `git_head = 96be31d2`（本 spec の実装の前・main `3d6b0629` 取り込み後）。`git_dirty_files = 1` は本 research.md の書きかけだけ（コードの変更 0）
- **機械**: `NAGI`・Intel Core Ultra 9 185H（物理 16・論理 22）・メモリ 32 GB・Intel Arc Graphics（ドライバ 32.0.101.8860）・Windows 11 Pro 10.0.26300・PowerShell 7.6.6
- **画面**: primary_dpi 192・作者基準 96 → k = 2.0。キャラの面 382×547 → 764×1094（8 月の計測と同じ条件）
- **静かさ**: `-AutoQuiet` の起動前の確認で `QUIET`（マシン全体の CPU 平均 9.8%・最大 26.6%・重いプロセス 0）。確かめるのは起動前だけで、走行中の他のセッションの負荷は道具が見ていない
- **生データ**: `target\perf\p0ap-before-short-release-2\`（リポジトリ外・`target` の下）。判定の全文は `target\perf\p0ap-before-verdict.txt`
- **`emo2` の中身の確認**: 展開した検体に `element*,base`・`interval,always`・動く絵（APNG・WebP・GIF）は 0 件。本 spec の足す経路を `emo2` は通らない前提のとおり

**数字（定常状態・344 適用・nearest-rank）**

| 段 | p50 | p95 | 最大 | 平均 |
| --- | ---: | ---: | ---: | ---: |
| `t_cache_us` | 6 µs | 11 µs | 90 µs | 7 µs |
| `t_compose_us` | 0 µs | 5,272 µs | 7,290 µs | 1,396 µs |
| `t_mask_us` | 0 µs | 732 µs | 1,072 µs | 175 µs |
| `t_upload_us` | 0 µs | 666 µs | 872 µs | 191 µs |
| **`t_total_us`（1 コマの時間）** | **69,084 µs** | **604,897 µs** | 2,572,976 µs | 164,520 µs |

- 命中率 55.2%（190／344）（定常）・キャラ面（`TargetId(0)`／1000）は 42.1%（全区間・適用 316 回／合計 416 回の内訳）。合成し直し（不命中）は定常で 154 回
- コマ適用間隔（判定式⑴の窓 C・キャラ面）: p50 92.2 ms・p95 370.1 ms（n=49）
- catch-up（`loop_ticker`）: 定常 252 件・全区間 257 件
- 新規確保（定常）: `alloc_compose_dst` 1・`alloc_mask` 1
- areka の CPU（1 コア換算・定常）: 平均 18.7%・p50 17.0%・p95 38.6%
- `--mode verdict --build release` の総合は不合格（⑴⑵⑶⑷a）。合否は本 spec の要件ではなく、前後の比べの材料として残す

**読むときの注意（後の数字と比べる前に）**

1. **この走行は 8 月より桁で遅い**: `draw-load-parity` の最終（2026-08-23・長時間水準）は `t_total_us` の定常 p50 2,784 µs・p95 26,520 µs、catch-up 96 件だった。今回は p50 69 ms・p95 605 ms・catch-up 252 件。しかも段の 4 つ（照会・合成・マスク・転写）の和は不命中の行でも 3 ms 前後で、`t_total_us` との差の大部分は**どの段にも入っていない**（`show.rs` で最後の `mark(Stage::MaskGen)` の後から `emit` までの、装着への書き込みと `info!` の区間。`present-gpu-transform-scale` でリサンプルの段が無くなった後は、段の和が合計を覆わない）。原因は本 spec の範囲外で、ここでは調べていない。
2. **機械は他のセッションと共有している**: 同じ測り方を 3 回起こし、1 回目（00:39〜00:44）と 3 回目（02:30〜02:35）は起動前の確認が 4 回とも `NOT_QUIET`（全体の CPU 平均 13〜86%・他のワークツリーの `rustc`・`link`、1 回目は別の `areka` も働いていた）で起動しなかった。採れたのは 2 回目の 1 本だけで、走行中に負荷が戻った可能性は消せない。1 本の数字はばらつきの大きさを語らない。
3. **後の数字の採り方の提案**: 実装の後は、同じ道具の交互取得（`perf-loop.ps1 prepare-ab`／`measure-ab`＝実装の前の実行体を `bin-A` へ退避し A→B→A→B を同じセッションで 7 分ずつ）で比べると、上の 1・2 の影響を同じ条件で打ち消せる。本節の 1 本は「同じ機械・同じ測り方の前の数字」の記録として残す。

> 注: 上の証拠のファイル（`target\perf\p0ap-*`・`target\p0ap-test.log`）は 2026-10-06 の全ワークツリーの `target\` の掃除で消えた。数字はレビューが掃除の前に判定ファイルと突き合わせて一致を確かめている。

## 後の数字（2026-10-06・タスク 6.2・要件 7.3・7.5・7.6）

### 測り方（`emo2` の前後・要件 7.3）

1.1 の「後の数字の採り方の提案」どおり、同じ道具の交互取得で採った。1.1 の 1 本と同じ機械・同じ水準（短時間 7 分）・同じ release・同じ `RUST_LOG`（採取スクリプトの固定値）・同じ数え方（開始 60 秒を除いた定常状態の `t_total_us`・nearest-rank）。

```powershell
# A（前）＝ 96be31d2 を target\ の下へ別の作業ツリーとして出し、そこの道具で bin-A を作る
git worktree add --detach <wt>\target\p0ap-base-wt 96be31d2
pwsh -File <wt>\target\p0ap-base-wt\tools\perf\perf-loop.ps1 prepare-ab -Iter 1 -OutRoot <wt>\target\perf-loop
# B（後）＝ 今の HEAD。A1 → B1 → A2 → B2 を同じセッションで 7 分ずつ・比べまで
pwsh -File <wt>\tools\perf\perf-loop.ps1 measure-ab -Iter 1 -OutRoot <wt>\target\perf-loop `
    -GhostRoot <wt>\target\nar-samples\manual\emo2\ghost\emo2 `
    -BalloonRoot <wt>\target\nar-samples\manual\emo2\balloon\emo2-kakukaku
python tools/perf/judge-perf.py <走行>\run.log <走行>\cpu.csv --mode baseline --build release
```

- **A（前）**: `96be31d2`（1.1 の前の数字と同じ HEAD）。`3d6b0629` との差は検体とテストだけで、製品のコードは同じ（`git diff --stat 3d6b0629 96be31d2 -- crates` は検体 13 個・`shell_target.rs` の `#[cfg(test)]` の 4 行・テスト 1 本）。`areka.exe` の SHA-256 `97783ddc…a954dc`
- **B（後）**: `72fd10ed`（タスク 6.1 まで・作業ツリーに差分 0）。`areka.exe` の SHA-256 `4cfe1e02…78ac32`
- どちらも 32bit のヘルパを作って載せた実 `pasta.dll`（4 本とも `shiori_helper_present = True`）。ビルドは `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`（道具の既定）
- **採った日時**: 2026-10-06 16:05〜16:38（日本時間・UTC 07:05:11〜07:38:19）
- **機械**: 1.1 と同じ `NAGI`（Core Ultra 9 185H・32 GB・Arc Graphics・Windows 11 Pro 10.0.26300）
- **静かさ**: 道具の決まり（起動前は静かでなければ 60 秒待って最大 3 回確かめ直す・走行後も同じ）に従った。A1 と A2 は起動前の 1 回目が `NOT_QUIET` で、待った後の確認で `QUIET` になってから起動した。A1 は走行後の 1 回目も `NOT_QUIET`（2 回目で `QUIET`）。B1・B2 は 1 回目から `QUIET`。4 本とも道具の決まりの中で採れた（中止 0 本）
- **生データ**: `target\perf-loop\draw-load-parity\iter-1\{A1,B1,A2,B2,bin-A,bin-B}\`・比べ `compare.txt`・各走行の集計 `baseline.txt`（リポジトリ外・`target` の下）

**数字（定常状態・`t_total_us`＝1 コマの時間・全対象）**

| 走行 | 側 | 適用 | p50 | p95 | 最大 | 平均 | 命中率 | 合成し直し（不命中） |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| A1 | 前 | 412 | 900 µs | 3,605 µs | 45,524 µs | 1,491 µs | 64.1% | 148 |
| B1 | 後 | 389 | 1,462 µs | 3,577 µs | 203,701 µs | 2,099 µs | 54.8% | 176 |
| A2 | 前 | 379 | 1,476 µs | 3,921 µs | 359,645 µs | 3,170 µs | 50.4% | 188 |
| B2 | 後 | 387 | 1,518 µs | 3,550 µs | 6,491 µs | 1,526 µs | 53.2% | 181 |

- キャラの面（`TargetId(0)`・サーフェス 1000）だけの p50／p95: A1 1,121／3,494 µs・B1 1,584／3,477 µs・A2 1,552／3,397 µs・B2 1,559／3,454 µs
- 同じ形を 2 回測ったばらつき（A1 と A2）の方が、前と後の差より大きい: p50 は A の 2 本で 900 と 1,476 µs・B の 2 本で 1,462 と 1,518 µs。p95 は A 3,605・3,921 µs、B 3,577・3,550 µs で、後の方が小さい。1 コマの時間が目に見えて落ちた跡は無い
- 段の和（照会＋合成＋マスク＋転写）は、キャラの面（`TargetId(0)`）だけで 4 本とも p95 2.2〜2.5 ms・最大 3.1〜3.9 ms。16 ms を超えた適用は A1 0・B1 2・A2 3・B2 0 本で、超えた行は全部が段の外の区間（段の和は最大 1.9 ms・段の外は 81〜360 ms）。全対象では段の和が p95 2.3〜2.6 ms・最大 3.7〜4.2 ms、16 ms を超えた適用が A1 1（`TargetId(3)` の 45.5 ms・命中）・B1 2・A2 3・B2 0 本、段の外は 45〜360 ms。前の側（A1・A2）にも同じ形で出る＝本 spec の前からの性質
- 合成し直し（不命中）は前 148・188 回、後 176・181 回で、前の 2 本の幅（40 回）の中。`emo2` は台本と乱数で毎回違う流れになるので、回数そのものの前後の一致は要件 7.2 の決定論テスト（6.1）が受け持つ
- `perf-compare.py` の採否（主指標はアイドルの CPU）: 前の平均 12.13%・後 11.98%（差 −0.15%・ばらつき 0.45%）。副指標の catch-up が前 15・18 件、後 20・17 件で、平均の比べで「悪化」と出た（判定は `WORSE`）。前の 2 本の差（3 件）と後の 2 本の差（3 件）の中に収まる揺れで、コマの適用間隔の p95 は前 161・901 ms → 後 198・162 ms・確保の数は前 0・2 → 後 0・0。性能改善のループの採否の物差しであり、本 spec の要件（目に見えて落とさない）の合否ではない

**1.1 の 1 本との違い**: 1.1 の 1 本（同じ `96be31d2`）は p50 69,084 µs・p95 604,897 µs だったが、同じ実行体の系統を交互に採った今回の A は p50 0.9〜1.5 ms・p95 3.6〜3.9 ms だった。差は実行体ではなく、1.1 の走行の時の機械の状態（他のセッションの負荷）と読む。1.1 の覚え書きの「段の外の区間が遅い」は今回も外れ値（16 ms を超えた行）としてだけ残っている（下の「対処」）。

### 動く絵 1 つの回数と時間（要件 7.5）

**作ったシェル**: `emo2` の一式を `target\p0ap-film\{f3,f4}\emo2` へ写し、キャラの面の定常のサーフェス 1000 の surface*ブレスの頭に element定義を 1 行だけ足した（`element0,overlay,<動く絵>,0,0`）。ほかは `emo2` のまま（台本・まばたき・口の動き・着せ替えはそのまま動く）。

| 名前 | 動く絵 | コマ | 待ち時間 | 繰り返し |
| --- | --- | ---: | --- | --- |
| f3（3 枚以下） | 検体 `shell/rgb.apng` をそのまま | 2 | 100・100 ms | 終わりなし |
| f4（4 枚以上） | 検体 `shell/surface1.png`（＝`basic.apng`）の `acTL` の繰り返しを 0（終わりなし）に、4 つの `fcTL` の待ち時間を 100 ms に書き替えたもの（CRC を付け直しただけ・絵のバイト列は同じ） | 4 | 100 ms × 4 | 終わりなし |

f4 を書き替えたのは、検体の `basic.apng` のままでは定常状態を測れないため: 合計 2 回で止まる（約 0.8 秒）うえ、待ち時間 0 のコマ（飛ばされる）と 1 ms のコマ（16 ms の刻みではほぼ見つからない）があり、実際に出るコマが 2〜3 枚になって「4 枚以上」を測ったことにならない。`alpha.webp`（3 枚）も合計 3 回で止まり、待ち 0 のコマを持つので同じ理由で使わなかった。

測り方は前後の数字と同じ（`invoke-perf-run.ps1 -Profile short -Build release -AutoQuiet -Goal draw-load-parity`・実行体は B と同じ `bin-B`・覚えの置き場 `AREKA_PROFILE_DIR` だけ走行ごとに `target\p0ap-film\profile-<名前>` へ分けた）。採った日時は 2026-10-06 16:40〜16:54（日本時間）・どちらも起動前の確認 1 回目で `QUIET`・終了コード 0・ヘルパあり。生データは `target\p0ap-film\run-{f3,f4}\`。

**数字（定常状態・キャラの面 `TargetId(0)`・約 360 秒）**

| | `emo2`（B1・B2） | f3（2 コマ） | f4（4 コマ） |
| --- | ---: | ---: | ---: |
| 適用の回数 | 291・308（0.8〜0.9 回／秒） | 3,854（10.7 回／秒） | 3,877（10.8 回／秒） |
| 合成し直し（不命中） | 156・157（0.44 回／秒） | 402（1.12 回／秒） | 3,877（10.8 回／秒） |
| 命中率 | 46.4%・49.0% | 89.6% | 0.0% |
| 異なる鍵の数 | 93・102 | 119 | 223 |
| `t_total_us` p50 | 1,584・1,559 µs | 2,749 µs | 4,013 µs |
| `t_total_us` p95 | 3,477・3,454 µs | 3,603 µs | 6,223 µs |
| `t_total_us` 最大 | 203,701・6,491 µs | 314,545 µs | 235,746 µs |
| 段の和 p95／最大 | 2.2〜2.5／3.1〜3.7 ms | 1.7／4.6 ms | 3.0／5.0 ms |
| 合成（不命中の行）p50／p95 | 1,221〜1,343／2,076〜2,184 µs | 1,306／2,435 µs | 1,333／2,476 µs |
| 16 ms を超えた適用 | 2・0 本 | 46 本（1.19%） | 49 本（1.26%） |
| コマの適用間隔 p95（判定式⑴の窓） | 198・162 ms | 99 ms | 112 ms |
| アイドルの CPU（平均・1 コア換算） | 11.72・12.24% | 11.08% | 16.44% |

- **回数**: 動く絵のコマが替わるたび（100 ms ごと）に 1 回適用され、コマが替わらない刻みでは適用されない（約 10 回／秒＝待ち時間どおり）。2 コマの f3 は 1 周の後はほぼ命中し、合成し直しは `emo2` だけのときより 0.7 回／秒多いだけ（まばたき・口の絵と動く絵の組が席 3 つを超えた分）。4 コマの f4 は命中 0%＝コマが替わるたびに合成し直す。設計の見立て（「3 枚以下は 1 周の後は命中し続け、4 枚以上は替わるたびに合成し直す」）のとおり
- **1 コマの時間**: p95 は f3 3.6 ms・f4 6.2 ms で 16 ms に収まる。合成そのもの（段の和）は最大でも 5.0 ms
- **16 ms を超えた適用**: f3 で 46 本・f4 で 49 本（どちらも全体の約 1.2%・最大 315 ms・236 ms）。超えた行は**全部が段の外の区間**（段の和は最大 4 µs〜4.0 ms、段の外は 17〜315 ms）で、`show.rs` の最後に記録した段（不命中は `mark(Stage::MaskGen)`、命中は `mark(Stage::CacheLookup)`）から `timing.emit(` までの、装着への書き込みと表示成立点の `info!` の区間。同じ形の外れ値は前の実行体（A2: 3 本・最大 360 ms）にも、動く絵の無い後の実行体（B1: 2 本・最大 204 ms）にも出ており、割合も同じ程度（A2 1.08%）。動く絵は適用の回数を約 13 倍にするので、外れ値の本数も同じ割合で増える

### 16 ミリ秒に収まらない分の数字と対処（要件 7.5・開発者への報告）

- **収まっている**: 1 コマの時間の p95（f3 3.6 ms・f4 6.2 ms）と、合成し直しの段の和の最大（5.0 ms）
- **収まっていない**: 全適用の約 1.2%（f3 46 本・f4 49 本／約 6 分）が 16 ms を超え、最大 315 ms。原因の区間は本 spec の足した経路ではなく、1.1 の覚え書きの「段の外の区間」（本 spec の前からの問題・A の実行体でも同じ形で出る）
- **対処の案（実装しない）**: 1.1 の覚え書き「範囲外・完了時に起票」の起票に、本節の数字（動く絵があると適用が 13 倍になり、外れ値の本数も比例して増える）を添える。直す先は `show.rs` の最後に記録した段（不命中は `MaskGen`・命中は `CacheLookup`）〜 `emit` の区間で、何が待っているか（装着への書き込み・`info!` の書き出し）を段に分けて測るところから。本 spec の中では直さない（要件 7.5 の「後続へ起票する」）
- f4 のアイドルの CPU（16.4%）は `emo2` だけのとき（11.7〜12.2%）より 4〜5 ポイント高い。コマが替わるたびの合成（10.8 回／秒・1 回 1.3 ms 前後）とマスク・転写の分で、席の数を増やしても 4 コマ以上の絵は 1 周で席を使い切るので減らない（下の席の数）

### 合成の結果を覚える席の数（要件 7.6）

**変えない（変更 0）**。`crates/areka-emo-present/src/cache.rs` の `const CAPACITY: usize = 3;` は本 spec の基点から 1 文字も変わっていない。

```
$ git diff 3d6b0629 -- crates/areka-emo-present/src/cache.rs | wc -l
0
$ git log --oneline 3d6b0629..HEAD -- crates/areka-emo-present/src/cache.rs | wc -l
0
```

（確かめた HEAD は `72fd10ed`・作業ツリーの差分 0。）f3 の数字のとおり 3 枚以下の絵は 3 席で 1 周の後は命中し、f4 の 4 枚以上の絵は席を増やしても絵のコマ数ぶんの席が要るので、席の数で吸収する形にしない。

## 実機の確かめ（2026-10-06・タスク 6.3・要件 9.5・10.6）

### 結論

**APNG と動く WebP の両方で、確かめる 6 項目は全部合格**: 自動アニメーション／`always`／サーフェスを切り替えても途切れない（終わりなしは巻き戻さない・回数つきは続けて見えていれば続き、見えなかった後は 1 枚目から）／バルーンの面が動く／隠れている間は合成が 0 回／バルーンが出た瞬間のコマが正しい。`emo2` そのままの見た目は、前の実行体（`96be31d2`）と今の実行体で、59 個のサーフェス × 2 スコープ（118 枚）の読み戻しの PNG がバイト単位で同じだった。

確かめの途中で、本 spec の範囲外の不具合を 1 件見つけた（下の「範囲外」・**要起票**）。本 spec の振る舞いとは関係しない（前の実行体でも同じに出る）。

### 作ったゴーストと走らせ方

- **置き場**（全部ワークツリーの `target\` の下・絶対パス）: 組み立ての手順 `target\p63\build.sh`・MCP の読み手 `target\p63\mcp.py`・起動 `target\p63\run.ps1`。`emo2` の検体 `target\nar-samples\manual\emo2` は書き替えず、写しを `target\p63\ghost\emo2`（試験用）・`target\p63\plain\emo2`（比べる用）へ作った
- **シェル**: `emo2` のシェルの写しに、検体 `crates/areka-emo-compose/tests/fixtures/animated-playback/shell/` のサーフェスを**番号に 9000 を足して**書き足した（`emo2` の番号と当たらないように・絵は `shell\master\p0ap\` の下・`surface1.png`→`surface9001.png`・`surface2.png`→`surface9002.png`）。中身は検体と同じ（9000＝一番上に `rgb.apng` 2 か所・`alpha.webp`・部品 9020・手書きの `always`／9001＝`basic.apng` だけで建つ／9002／9003／9010＝合計 0 の `always`／9011＝`bind+always`／9020＝部品・`rgb.webp`・50 ms の `always`／9030・9031）。比べのために、同じ絵を置く対と WebP の写しを足した:

  | 番号 | 中身 | 役 |
  | --- | --- | --- |
  | 9004 | 灰の地＋`rgb.webp` を (4,4) | 9003 の WebP 版（終わりなし） |
  | 9005 | `surface9005.png`＝`alpha.webp` だけで建つ | 9001 の WebP 版（合計 3 回） |
  | 9007・9008 | 9003・9004 に黄 4×4 を (12,12) へ足したもの | 同じ絵を置く対（切り替えて途切れないか） |
  | 9012・9013 | 灰の地＋`basic.apng` を (0,0)、9013 は黄を足す | 回数つき APNG の対 |
  | 9014・9015 | 灰の地＋`alpha.webp` を (0,0)、9015 は黄を足す | 回数つき WebP の対 |

- **バルーン**: `emo2-kakukaku` の写し `target\p63\balloon\kaku63` に面を 4 つ足した。10＝`rgb.apng`（終わりなし）・11＝`alpha.webp`（合計 3 回）・12＝`rgb.webp`（終わりなし）・13＝`basic.apng`（合計 2 回）。面 0 は `emo2` のまま
- **台本**: MCP の `sakurascript` がまだ無い（`NG:not implemented yet`・`mcp-kanade-tools` の受け持ち）ので、写しの辞書 `dic/boot.pasta` の起動の場面（4 つの時間帯・`halt`・`OnFirstBoot`）を 1 本の台本に差し替え、`pasta.toml` の自発トークの間隔を 3,600 秒にして割り込みを止めた。台本は `\s[番号]\_w[ミリ秒]` と `\b[番号]・\_w[…]`・`\b[-1]` を並べたもの（APNG の部 → WebP の部の順・全文は `build.sh`）
- **実行体**: 今の実行体は 6.2 の `bin-B`（`72fd10ed`・`areka.exe` の SHA-256 `4cfe1e02…`・`HEAD` `b26100bb` とはコードの差 0）、前の実行体は `bin-A`（`96be31d2`・`97783ddc…`）。どちらも 32bit のヘルパ付き
- **起動**: `AREKA_*`／`WINTF_*` を全部外し、`AREKA_APP_SMOKE_EXIT_MS`（120,000／100,000／40,000／45,000 ms）・`AREKA_PROFILE_DIR`（走行ごとに `target\p63\<走行>\profile`）・`NO_COLOR=1`・`RUST_LOG=info,areka_seriko=debug,areka_emo_present=debug,areka_emo_compose=debug` で起こした。止まったのは全部が有界の自動終了（終了コード 0・「待受を閉じた」の記録あり）で、強制で止めたものは 0
- **読み戻し**: `mcp.py` が `http://127.0.0.1:9801/api/mcp/v1`（記録の「待受を始めた」から読む）へ `dump_surface`（引数なし＝今の姿）と `dump_balloon`（引数なし）を交互に送り続け（1 往復およそ 20〜30 ms）、返った PNG を中身の SHA-1 の名前で `target\p63\<走行>\` に保存した（`poll.tsv` に時刻・本文・SHA-1）。PNG は原寸（キャラ 16×16・8×8。画面は拡大率 200% で 32×32＝記録の `native_w=16 … scaled_w=32`）。判定は PNG を読んで決め手の画素の色を見た（`runs.py`・`seg.py`）
- **記録の判定**: 表示層の `perf(apply_show): 段階別計時`（対象・面の番号・`cache_hit`・`key_hash`）を時刻つきで並べ（`perf.py`）、seriko の「時計が生まれた」と、表示層の `apply(Hide)`・`show_target: 可視化した` と突き合わせた
- **走行**: r1（1 回目・2 分）・r2（切り替えの時刻をコマの境目から外した 2 回目・100 秒）・plain-A／plain-B（`emo2` の比べ）・r3-binA（同じ試験用ゴーストを前の実行体で・対照）

### 色の読み方

検体の README の決め手の画素どおり。灰＝地 (128,128,128)・赤 (255,0,0)・青 (0,0,255 または 0,0,254)・緑 (0,255,0 または 0,254,0)・黄 (255,255,0)・水色 (0,255,255)・赤紫＝部品 (255,0,255)。`rgb.*` の左半分の白は α を持たない絵の左上の色なので抜かれて地の灰が見える（`use_self_alpha,1` の下の既存の扱い）。

### APNG の結果

`面#n` の n は、`perf.py` がその面で鍵（`key_hash`）の現れた順に振った番号で、コマの番号ではない（`basic.apng` は #1＝2 番・#2＝全透明、`alpha.webp` は #1＝2 番。`rgb.*` は #0＝0 番・#1＝1 番）。

| 確かめたこと | 読み戻し（r1・r2） | 記録（r2・最初に出た時刻からのミリ秒・`*` は合成し直し） | 判定 |
| --- | --- | --- | --- |
| 自動アニメーション（element定義・終わりなし `rgb.apng`） | 9003 の (11,11) が赤↔青を約 100 ms ごとに交互（r2 の 3 秒で 30 回） | 9003 の適用は 2 種類の鍵が交互に約 100 ms ごと。合成し直しは最初の 2 回だけ（以後は命中） | 合 |
| 自動アニメーション（`surface9001.png`＝`basic.apng`・合計 2 回） | 0 番（左半分が赤）→ 2 番（(2,2) に青 2×2）→ 0 番 → 2 番 → 全部透明で止まる。待ち 0 の 1 番は 1 度も出ない | `#0+0* #1+345* #0+408 #1+758 #2+808*`（待ち 333・0・70・1 の累積 333・404・737・808 に、16 ms の刻みを足した時刻）。その後は適用 0 回 | 合 |
| `always`（一番上・部品） | 9000 を出していた間の r2 の 111 回の読み戻し（絵が替わるごとに区切って 68 区間）が全部、(0,0) が最初の 100 ms は灰（1 周目の最初の待ちの前は何も出さない）、以後は黄↔水色。部品の (8,8) も最初の 50 ms は赤紫、以後は黄↔水色 | `loop 一番上の always の再生が生まれた`・`part always の時計が生まれた part=9020` が 9000 を出した時刻に 1 回ずつ | 合 |
| 同じ絵を 2 か所（要件 3.4） | 9000 の 111 回の読み戻し（68 区間）が全部、(4,0) と (12,0) が同じ色 | — | 合 |
| 合計 0 の `always`（9010）・`bind+always`（9011） | 9010 は (0,0) に水色（1 周だけ評価した絵）で動かない。9011 は灰の地だけで動かない | `WARN … always の待ち時間の合計が 0 ゆえ非採録 … surface_id=9010` が読み込みで 1 回・`DEBUG … 未駆動 interval 語彙ゆえ非採録 … vocab="bind+always"` が 1 回。どちらも適用は 1 回だけ | 合 |
| 切り替えても巻き戻さない（終わりなし） | 9003 → 9007 → 9003 → 9002（`rgb.apng` が無い）→ 9003 → 9001 → 9012 → 9013 → 9003 → 9012 → 9000 の全部で、`rgb.apng` の時計は**最初の 1 回しか生まれない**（`film:28` が 08:18:16.557 に 1 回）。9002 の後に戻った瞬間は青（1 番・経過 8.154 秒で周の 0.1〜0.2 秒の位置）で、巻き戻しなら赤になる所。9000 へ切り替えた直後の 1 枚も続きのコマ | 9003・9007 の適用 521 本のうち、切り替えの瞬間以外は全部が `film:28` の生まれた時刻からの 100 ms の境目の 25 ms 以内（途中で位相がずれていない） | 合 |
| 回数つきは続けて見えていれば続き（要件 2.7） | 9012 を 500 ms 出して 9013（同じ `basic.apng`）へ: 9013 の最初は 2 周目の 0 番、258 ms 後に 2 番、309 ms 後に透明で止まる | `9012#0+0* #1+351* #0+409 9013#0+500* 9013#1+758* 9013#2+809*`（9012 の出た時刻からの累積がそのまま続く）。時計は 9012 で 1 回生まれ、9013 では生まれない | 合 |
| 回数つきは見えなかった後は 1 枚目から（要件 2.3） | 9003 を 500 ms 挟んで 9012 へ戻すと、また 0 番から 2 周して透明で止まる | `9012#0+2501* #1+2842* #0+2909 #1+3259 #2+3309*`（戻った時刻から 0・341・408・758・808）。`film:24` の時計が戻った時刻に**もう 1 回生まれる** | 合 |
| バルーンの面が動く（面 10＝`rgb.apng`） | 8×8 の右半分が赤↔青を約 100 ms ごとに交互 | `10#0+0* 10#1+143* 10#0+242 10#1+344 …`（合成し直しは最初の 2 回だけ） | 合 |
| 隠れている間は合成 0（面 10） | `\b[-1]` の 2 秒の間、`dump_balloon` は隠す前の最後の絵（青）を返し続ける（失敗しない） | `apply(Hide)` 08:18:39.403 から台本の `[10]` が届く 08:18:41.403 まで、バルーンの対象（`TargetId(1)`）の適用は 0 本。その後 `show_target` までの適用は、`[10]` の同じコマ（隠す前と同じ `key_hash`）の通し 1 本（41.403）と `show_target` の通し直し 1 本（41.445）だけ。どちらも命中・合成 0（`t_compose_us=0`）・転送 0（`t_upload_us=0`） | 合 |
| 出た瞬間のコマ（終わりなし） | 出た直後の読み戻しは時計どおりのコマ | 見えるようになった `show_target` の通し直し（08:18:41.445）のコマは **0 番**。面 10 の時計が生まれてから 4.199 秒で、周の境目 4.2 秒と記録の時刻の差 1 ms（16 ms の刻みと時計の読み方の粒度の内）＝0 番が正しい。次の切り替えは 4.316 秒で 1 番。その前の 41.403 の通しは隠れている間のもので画面に出ていない | 合 |
| 出た瞬間のコマ（回数つき・面 13＝`basic.apng`） | 隠す前は透明で止まっていたのが、出た瞬間に 0 番（左半分が赤）から始め直し、2 周して透明で止まる | `apply(Hide)` 08:18:44.811 → `show_target` 08:18:45.845。その間の適用は `show_target` の通し直し 1 本（45.844・命中・合成 0・転送 0）だけ・`film:4` の時計が 08:18:45.845746 にもう 1 回生まれる。適用は `13#0+8633 #1+8992 #0+9050 #1+9392 #2+9458`（出した時刻から 0・359・417・759・825） | 合 |

### 動く WebP の結果

| 確かめたこと | 読み戻し（r1・r2） | 記録（r2・最初に出た時刻からのミリ秒・`*` は合成し直し） | 判定 |
| --- | --- | --- | --- |
| 自動アニメーション（element定義・終わりなし `rgb.webp`） | 9004 の (11,11) が赤↔青を約 100 ms ごとに交互（`rgb.apng` の 9003 と同じ絵になる＝同じ PNG） | 9004 の適用は 2 種類の鍵が交互に約 100 ms ごと | 合 |
| 自動アニメーション（`surface9005.png`＝`alpha.webp`・合計 3 回） | 0 番（左半分が赤）→ 2 番（右に緑 4×7・(2,2) に青 2×2）を 3 周して 2 番で止まる。待ち 0 の 1 番は 1 度も出ない | `#0+0* #1+100* #0+166 #1+283 #0+342 #1+458`（待ち 100・0・70 の累積 100・170・270・340・440 に刻みを足した時刻）。その後は適用 0 回 | 合 |
| 部品の中の動く絵（9000 の部品 9020 の `rgb.webp`） | 9000 の 111 回の読み戻し（68 区間）が全部、(12,12) が赤か青で、時間とともに入れ替わる | `film:29` の時計が 9000 を出した時刻に 1 回生まれる | 合 |
| 切り替えても巻き戻さない（終わりなし） | 9004 → 9008 → 9004 → 9002 → 9004。9008 へ切り替えた瞬間は青（1 番・時計の周の 0.147 秒の位置）で、巻き戻しなら赤 | `rgb.webp` の時計（`film:29`）は 9000 で生まれた 1 回だけ。9000 の後、9010・9011・バルーンの部（約 12 秒・`rgb.webp` はどこにも見えない）を挟んで 9004 を出しても生まれ直さない＝見えない間も進んでいる（要件 3.3）。9004・9008 の適用 92 本は、切り替えの瞬間の 2 本と下の 4 本を除いて全部が `film:29` の生まれた時刻からの 100 ms の境目の 25 ms 以内。4 本は境目から約 30 ms 遅れた単発で、次の適用は境目に戻っている（位相はずれていない・下の「観測」） | 合 |
| 回数つきは続けて見えていれば続き | 9014 を 250 ms 出して 9015 へ: 9015 は 2 周目の 0 番から続き、3 周目の 2 番で止まる | `9014#0+0* #1+108* #0+166 9015#0+250* 9015#1+283* #0+341 #1+458`（9014 の出た時刻からの累積がそのまま続く）。時計は 9014 で生まれ、9015 では生まれない | 合 |
| 回数つきは見えなかった後は 1 枚目から | 9004 を 500 ms 挟んで 9014 へ戻すと、また 0 番から 3 周 | `9014#0+2250* #1+2367* #0+2426 #1+2525 #0+2583 #1+2700`（戻った時刻から 0・117・176・275・333・450）。`film:23` の時計がもう 1 回生まれる | 合 |
| バルーンの面が動く（面 12＝`rgb.webp`） | 8×8 の右半分が赤↔青を約 100 ms ごとに交互 | `12#0+0* 12#1+100* 12#0+199 …` | 合 |
| 隠れている間は合成 0（面 12） | `\b[-1]` の 2 秒の間、`dump_balloon` は最後の絵を返し続ける | `apply(Hide)` 08:19:05.111 から台本の `[12]` が届く 08:19:07.111 まで、`TargetId(1)` の適用は 0 本。その後 `show_target` までの適用は、`[12]` の同じコマの通し 1 本（07.111）と `show_target` の通し直し 1 本（07.128）だけ。どちらも命中・合成 0・転送 0 | 合 |
| 出た瞬間のコマ（終わりなし） | 出た直後は時計どおりのコマ | 見えるようになった `show_target` の通し直し（08:19:07.128）のコマは **0 番**。面 12 の時計が生まれてから 4.233 秒＝周の中の 0.033 秒＝0 番が正しい | 合 |
| 出た瞬間のコマ（回数つき・面 11＝`alpha.webp`） | 2 番で止まっていたのが、出た瞬間に 0 番から始め直し、3 周して 2 番で止まる | `apply(Hide)` 08:19:10.511 → `show_target` 08:19:11.537。その間の適用は `show_target` の通し直し 1 本（11.536・命中・合成 0・転送 0）だけ・`film:2` の時計がもう 1 回生まれる。適用は `11#0+8633 #1+8758 #0+8808 #1+8916 #0+8975 #1+9092`（出した時刻から 0・125・175・283・342・459） | 合 |

- バルーンの面の時計はシェルと別の番号で動いた（`slot=Balloon part=film:1〜4`・シェルは `slot=Shell part=film:23〜64`・要件 6.6）
- バルーンの面の合成し直しは、どの面も各コマを初めて出したときだけ（面 10・11・12 は 2 回、13 は 3 回）。以後は全部が命中
- 1 回目（r1）も同じ結果だった。ただし台本の待ち時間が 100 ms の倍数で、切り替えがコマの境目に重なったので、2 回目は待ち時間を 3,050・2,030・1,530 ms にずらして撮り直した

### 記録の抜き書き（r2）

```
08:18:16.071826Z WARN  areka_seriko::table: seriko table: always の待ち時間の合計が 0 ゆえ非採録（1 周だけ評価した絵のまま・要件 4.6） surface_id=9010 animation_id=0
08:18:16.071907Z DEBUG areka_seriko::table: seriko table: 未駆動 interval 語彙ゆえ非採録（元語彙保持・要件 8.2/11.4） surface_id=9011 animation_id=0 vocab="bind+always"
08:18:16.557923Z DEBUG areka_seriko::parts: seriko: part always の時計が生まれた scope="0" slot=Shell part=film:28 animation_id=0 finite=false
08:18:28.206177Z DEBUG areka_seriko::parts: seriko: part always の時計が生まれた scope="0" slot=Shell part=film:24 animation_id=0 finite=true
08:18:30.706575Z DEBUG areka_seriko::parts: seriko: part always の時計が生まれた scope="0" slot=Shell part=film:24 animation_id=0 finite=true
08:18:32.206026Z DEBUG areka_seriko::looper: seriko: loop 一番上の always の再生が生まれた scope="0" slot=Shell animation_id=0
08:18:39.403041Z DEBUG areka_emo_present::presenter::hub: apply(Hide): 非表示へ target_id=TargetId(1) was_visible=true
08:18:41.445300Z DEBUG areka_emo_present::presenter::visibility: show_target: 可視化した target=TargetId(1) surface_id=10 ownership=External stage_generation=2
08:18:44.811431Z DEBUG areka_emo_present::presenter::hub: apply(Hide): 非表示へ target_id=TargetId(1) was_visible=true
08:18:45.845023Z DEBUG areka_emo_present::presenter::visibility: show_target: 可視化した target=TargetId(1) surface_id=13 ownership=External stage_generation=3
08:18:45.845746Z DEBUG areka_seriko::parts: seriko: part always の時計が生まれた scope="0" slot=Balloon part=film:4 animation_id=0 finite=true
```

（`film:28`＝`rgb.apng` は 1 回だけ。`film:24`＝`basic.apng` は 9012 を出したときと、9003 を挟んで戻したときの 2 回で、9013 へ切り替えたときは生まれない。）

### `emo2` の見た目が変わらないこと

- `target\p63\plain\emo2`（`emo2` の写し・手を加えていない）を前の実行体（`bin-A`）と今の実行体（`bin-B`）でそれぞれ起こし、`emo2` の `surfaces.txt` に在る 59 個のサーフェスを `dump_surface`（`surface` を指定＝経過 0 の単体の絵）でスコープ 0 と 1 の両方から読み戻した（118 枚）
- **118 枚とも、返った PNG の SHA-1 が前と後で一致**（`plain-A\plain.tsv` と `plain-B\plain.tsv` の差分 0・失敗 0）。2 本とも ERROR 0 件
- 走っている `emo2` の 1 コマの時間と合成し直しの回数は、6.2「後の数字」（同じ実行体の交互取得）を正本とする

### 対照（前の実行体で同じ試験用ゴースト・r3-binA）

前の実行体（`96be31d2`）では、同じ台本で 9003・9004・9001・9005・9012〜9015・バルーンの面 10・13 の読み戻しが**最初のコマのまま 1 度も変わらない**（例: 9003 は 3 秒間ずっと (11,11) が赤）。9010 は灰の地だけ（`always` を駆動しない）。今の実行体との違いは本 spec の足した振る舞いだけ。

### 範囲外（要起票）

1. **【要起票】バルーンの文字の描画範囲が 0 以下に潰れると、文字の層が毎フレーム供給面（swap chain）を作ろうとして失敗し、ERROR を毎フレーム出し続ける**（`areka-emo-text`）。検体の 8×8 のバルーンの面（`emo2-kakukaku` の余白 36〜56 px が面より大きい）を出すと、`WARN areka_emo_text::region: 解決後の validrect が退化している（幅/高さ ≤ 0）… left=36.0 top=46.0 right=-36.0 bottom=-48.0` に続いて `ERROR areka_emo_text::surface: D3D/DXGI/WUC 呼び出しが失敗 hresult=-2005270527 context="create_composition_swap_chain"`（0x887A0001）と `ERROR areka::emo2_boot::frame::scale_text: emo2 text: present_frame が失敗（…次フレーム再試行…）` が**フレームごとに 1 組**出る。バルーンを隠した後も、ゴーストが終わるまで止まらない（WARN・ERROR 2 種の組の数: r1＝11,295 組〔約 100 秒・ERROR 22,590 行〕・r2＝9,466 組・r3-binA＝2,889 組。hresult は -2005270527＝0x887A0001）。前の実行体（r3-binA・`96be31d2`）でも同じに出る＝本 spec の前からの性質。直す先の候補は「描画範囲が空なら供給面を作らない（文字を描かない）」と「同じ原因の記録を 1 回にする」。本 spec の確かめには影響しなかった（`dump_balloon` はバルーンの面の絵を返し、絵と動きの判定はそれで行った）
2. **（起票済み・新しい起票は不要）MCP の `sakurascript`・`raise_event` が `NG:not implemented yet`** のため、台本を外から流せず、ゴーストの写しの辞書を書き替えて確かめた。受け持ちは `areka-P0-mcp-kanade-tools`（`.kiro/specs/areka-P0-mcp-kanade-tools/` が在り、roadmap の C5 の候補に載っている）

### 確かめの途中の観測（起票はしない・数字だけ残す）

- r1 で 1 度だけ、プロセス全体の記録が 1.77 秒止まった（08:12:05.32〜08:12:07.09・表示・seriko・MCP のどのスレッドも記録なし・`perf(apply_show)` の最大は 39.5 ms で、適用の途中ではない）。r2・r3 では起きなかった。原因は特定していない（機械を他のセッションと共有している）。このため r1 では 9005 の動きの途中が読み戻しに映らず、r2 で撮り直した
- r1 で 1 度、表示のスレッドが 0.4 秒遅れた直後に、溜まったバルーンの指令 4 件が同じ刻み（`tick=3105`）の中で続けて適用された（4 件とも命中・最後の 1 件が時計どおりのコマ）。古いコマが画面に残ったフレームは無い
- r2 の WebP の部で、コマの切り替えの適用が境目から約 30 ms 遅れた単発が 4 本あった（08:18:48.337・50.337・52.336・56.336＝ほぼ 2 秒おき・どれも次の適用は境目に戻る）。6.2「16 ミリ秒に収まらない分」の外れ値と同じ種類と見ている（本 spec の前からの性質・起票の候補は 1.1／6.2 の覚え書きのとおり）
- 台本の待ち時間に対して、実際の切り替えは 20〜50 ms 遅れた（`\_w[3050]` の後の切り替えが 3.096 秒）。判定は台本の時刻でなく記録の時刻で行ったので、結果には影響しない

### 証拠の置き場（リポジトリ外・`target` の下）

- 組み立てと道具: `target\p63\{build.sh,run.ps1,mcp.py,runs.py,seg.py,perf.py,phase.py}`
- 走行: `target\p63\{r1,r2,r3-binA,plain-A,plain-B}\`（`run.log`・`poll.tsv` または `plain.tsv`・読み戻した PNG〔SHA-1 の頭 12 桁の名前〕・`surface_runs.tsv`・`perf0.txt`／`perf1.txt`）
- 主な絵（r2）: 9003 の 0 番 `beb223486754.png`・1 番 `9c1822b107ed.png`／`basic.apng` の 0 番 `c613e2afc829.png`・2 番 `314ac24fd2c3.png`・全透明 `34a788b043c0.png`／`alpha.webp` の 2 番 `b31ccc70b285.png`／バルーンの面 10・12 の 0 番 `31e69349d797.png`・1 番 `2a396c53718c.png`／9000 の最初の 1 枚 `d0522daad724.png`
