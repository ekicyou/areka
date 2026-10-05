# 設計の検証（作り直した版）: areka-P0-animated-image-playback

> 2026-10-05・対話なしの再検証（`/kiro-design` の検証の段）。対象は作り直した `design.md`（コミット `cf1e0ac1`・「子サーフェスへの分解」の版）。
> **本書は、却下された形（`PatternState` に絵のコマの欄を足し、合成の後で絵の番号を差し替える形）を検証した前の報告を置き換える。** 前の報告の指摘は、形そのものが変わったので引き継がない。
> 設計が「コードを読んで確かめた」と書いている事実は、設計を信じずに本ブランチの実物を読んで引き直した。コードは「何の定義か＋ファイル」で指す。
> 本書は `design.md`・`requirements.md`・`research.md`・`spec.json` を書き換えていない。重い cargo のビルドは走らせていない。

## 判定

**GO（条件つき）** — 作り直した形は、開発者の言う「本質的な案」（動く絵を子へ分解し、子は手書きの `always` の部品と同じ入口・同じ時計・同じ計算を通る）になっている。却下された形の持ち込みは無い。実物のコードとも合っている。
ただし下の重大な指摘 3 件を設計討議で片付けてから、タスク生成へ進むこと。3 件とも形の作り直しは要らない（1 は設計書の 1 節の直し、2 は他セッション・文書との調整、3 は決まりを 1 つ足すか「残る遅れ」に書いて裁定をもらう）。

## まとめ

- 設計の核（`always` を合成・外形・見える部品の 3 か所で根から扱う／部品を指す鍵に種類を足す／繰り返しは 1 つの純粋な計算）は筋が通っていて、要件の裁定（2.3・2.7・2.8・3.2・3.3・2.9・6.x・1.13）を 1 つずつ満たす道が示されている。
- 弱いのは表示層の「合成の先送り」の節。書かれた条件のままだと、窓を出す通し直しが自分の早い戻りに掛かり、また `last_show` を読む既存の口（MCP の読み戻し）が壊れる。
- 「スコープが膨らむなら他セッションと調整」という開発者の条件は、設計書に調整点が挙がっているだけで、まだ済んでいない。要件・brief・roadmap の文面も古い約束のまま残っている。

## 実物と照らした結果（設計の主張の裏取り）

合っていたもの（設計の「Existing Architecture Analysis」ほか）。

| 設計の主張 | 確かめた所 | 結果 |
| --- | --- | --- |
| 合成が重ねるのは「有効な着せ替えの pattern0」と「この段にコマを持つ animation」だけ | `flatten_surface`（`crates/areka-emo-compose/src/plan.rs`）の合流対象の番号の集め方 | 合っている。`always` はコマが無ければ描かれない |
| 外形に入るのは束縛の在る画像・element定義の子・着せ替えの種類の pattern0 だけ | `flatten_extent`（同上） | 合っている |
| 部品の入口と時計が在る・今はシェルの面だけ | `push_static_element_ops` の `ElementKind::Surface` の腕（同上）／`NestTable::visible_parts`（`nesting.rs`）／`PartClocks`（`crates/areka-seriko/src/parts.rs`）／`LoopRuntime::on_tick` の `with_parts`（`looper.rs`） | 合っている |
| seriko は `always` を落とす | `AnimationTable::from_world` の `Interval::Other` の腕（`crates/areka-seriko/src/table.rs`）。読み手は `always` を `Interval::Other("always")` で運ぶ（`crates/areka-parsers/src/shell/decode_tests_lenient_input_tests.rs` の期待値） | 合っている |
| バルーンを出すときは最後の入力で通し直す | `EmoPresenter::show_target`（`crates/areka-emo-present/src/presenter/visibility.rs`） | 合っている |
| 隠れている対象への指令も今は毎回合成まで走る | `apply_show`（`presenter/show.rs`）。所有権で分かれるのは末尾の可視化だけ | 合っている |
| 届けの相が表示層の照会 2 本を読んでいる・台帳はゴーストごと | `run_status_report_phase`・`report_balloons`・`BalloonStatusLedger`（`crates/areka/src/emo2_boot/frame/status_report.rs`） | 合っている |
| `compose_key_hash` は一番上の欄しか混ぜていない | `compose_key_hash`（`presenter/timing.rs`）。`pattern.iter()` だけをなめる | 合っている |
| 動く絵の表: `frames[0]` は親・コマと待ち時間は同じ長さ・2 枚以上 | `Animation`・`AtlasTable::with_frames`・`AtlasTable::animation`（`crates/areka-emo-atlas/src/table.rs`） | 合っている（分解の検査①②は、表を組むときの `assert!` と同じ中身。二重だが害は無い） |
| シェルもバルーンも `bind_atlas` を通る | 呼び手は `ShellTarget::build_world`（`crates/areka-emo-present/src/shell_target.rs`）と `build_balloon_target_from_faces`（`balloon.rs`）の 2 か所（本番）。どちらも 1 度だけ呼ぶ | 合っている |
| 行数（今） | `plan.rs` 821・`nesting.rs` 329・`pattern.rs` 203・`world.rs` 371・`table.rs` 616・`timeline.rs` 506・`looper.rs` 528・`parts.rs` 339・`state.rs` 632・`actor.rs` 660・`show.rs` 426・`timing.rs` 304・`status_report.rs` 191・`mod.rs` 883・`spine.rs` 1,000 | 全部合っている |
| `emo2` は `always` を持たない | リポジトリ内の `*.txt` に `interval,always` は 0 件（`target/` を除く） | 合っている。足した腕は `emo2` では通らない |
| `LoopFrame` の書き下しは seriko の中だけ | `LoopFrame {` の書き下しは `table.rs` と `timeline.rs`（その中のテスト）だけ。`emo2` のまばたきの照合（`looper_parts_emo2_tests.rs`・`spine_seriko_loop_tests.rs`）には無い | 合っている（要件 7.7 の「書き換えずに通す」は守れる） |

食い違い・書き漏れ（小さいもの。直しは末尾の「軽い直し」へ）。

- 「`LoopTrigger` の腕を 2 つと決め打つ 2 か所」とあるが、実物は 3 か所: `from_world` の `let (Random | BindRandom) = trigger`（`table.rs`）・`gate`（`parts.rs`）・`on_tick` の抽選の `match anim.trigger`（`looper.rs`）。
- `NestTable::from_world` の「行を載せる条件」は `children`・`bind_targets`・`bind_ids` のどれかが空でないこと（`nesting.rs`）。`films`・`always_rest` を足すなら、この条件にも足さないと、動く絵だけを置いたサーフェスの行が載らない。設計は欄を足すとだけ書いている。
- `NestReport::from_world` の element の振り分けは、`Image` と「在る `Surface`」以外を全部「無い番号」に数える（`nesting.rs`）。設計は「`Film` を読み飛ばす」と書いているので方針は正しい。ここは `match` の `_` の腕なので、コンパイラは漏れを教えてくれない（テストで固定すること）。

## 観点ごとの検証

### 1. 本当に「本質的な案」か（却下された形の持ち込みは無いか）

持ち込みは無い、と判断する。

- 却下された形は「合成の後で絵の番号を差し替える」「動く絵だけの計算と時計」だった。新しい形は、動く絵を置いた element を `ElementKind::Film` に替えて element の順の位置で子を平坦化し、時計は `PartClocks` の同じ 1 本の評価の経路、計算は手書きの `always` と同じ `always_at`、経過 0 の決まり（`rest_index`）も外形の決まりも手書きと共通。`PatternState` の「動く絵の子の欄」は、部品の欄（`parts`）と同じ役の「子の欄」であって、合成の後の差し替えではない。
- 正直に言うと、子は「手書きの部品と字面まで同じ」ではない。コマがサーフェスでなく絵を直接指すので、`FilmSheets`（面の表とは別の Resource）・`Cell::Picture`・`LoopFrame.picture`・`visible_films`・`set_film` と、絵を指すための腕が 6 か所ほどに在る。作者が今この形を手で書く道は無い（書けるようになるのは後続の `import`）。
- 設計が挙げた不採用の理由（コマごとのサーフェスは上限いっぱいで 1,025 個・面の一覧をなめる 7 か所の全部に現れる・作者の番号の空間に入る）は実物と合っていて、妥当。**ただし「手書きで定義できるようになる → 自動に定義分解する」という開発者の言い方との差なので、設計討議で一言確かめる価値がある**（重大な指摘には数えない）。
- brief の「コマを 1 枚ずつ持ち」「element 0」の字面とは違い、子は element を 0 個持つ。1 枚目を element に敷くと透明な所から透ける（`push_static_element_ops` は element を必ず描き、コマはその上に重なる）ので、敷かないのが正しい。

`PartKey::{Surface, Film}`・`ElementKind::Film` は健全。

- `SurfaceIndex` に載せないので、`EmoWorld::surface_ids`・`surface`・`dangling_pattern_targets`（`world.rs`）・`attach_hit_regions`（`hit_import.rs`・`ElementKind::Surface` だけを拾う）・`ShellTarget` の焼く前の除外（`shell_target.rs`・欄の綴りで見る）に現れない。設計の表のとおり。
- `ElementKind` を突き合わせる本番の所は `plan.rs`（2 か所）・`nesting.rs`（3 か所）・`atlas_bind.rs`（1 か所）・`hit_import.rs`（1 か所）で、設計の表はこれを覆っている。`atlas_bind::bind_atlas` は「`Image` でなければ束縛しない」なので、分解を**束縛の後**に置く順は必須（設計どおり）。同じ面の表に `bind_atlas` を 2 度呼ぶと子の定義が古いまま残るが、今の呼び手は 1 度だけ。前提として設計に 1 行書いておくとよい。
- `FilmId` は親の `ElementId` の値。シェルとバルーンは別のアトラス・別の面の表・時計の鍵に面の種類が入るので当たらない。

### 2. 0 フレームの主張と、欄の 3 つの意味

- 「`always` はコマが無ければ経過 0 を定義から描く」は、空のコマで合成される 3 つの場面（`apply` が出す空の `Show`〔`crates/areka-seriko/src/state.rs`〕・差し替えの `RebasedShow`〔`actor.rs` の `rebased`・コマを運ばない〕・バルーンの装着）のどれにも効く。成り立つ。
- **`always` でない animation と混ざらないか**: 「載っていない＝経過 0 を描く」は `is_always_interval` が真の animation にだけ当てる、と設計は明記している。`random`・`bind+random` の「載っていない＝描かない／着せ替えなら pattern0」は今のまま。`bind+always` は読み手が `Interval::Other` で運び、`is_bind_interval`（`plan.rs`）にも `is_always_interval` にも掛からないので、今までどおり何も描かれない（要件 4.9）。あいまいさは無い。
- **今あるシェルの絵は変わるか**: `always` を書いていないシェル（`emo2` を含む）は変わらない。`always` を書いているシェルは、今まで出なかった経過 0 の絵が出て、外形が全部の pattern の和集合に広がる。これは要件 4 が求める変化で、設計は Open Questions 3 に挙げている。
- **既存の決定論テスト**: `PatternState` の今の関数と `PatternFrame` は変えず、`Default` は空のまま。seriko が「経過 0 と同じコマを載せない」ので、空の `PatternState` の鍵（`ComposeKey`）は今と同じ。`table.rs` の中の 1 本（`always` を「採らない語」の例に使う檻）は書き換えが要る、と設計が自分で挙げている。
- 気を付ける所が 1 つ。手書きの `always` で pattern0 の待ちが 0 のとき、周期（＝待ちの合計）の境目で「前の周の最後のコマ」と「次の周の pattern0」が同じ時刻になり、後ろが勝つ。**最後のコマは 0 ミリ秒しか出ない**（例: 待ち 0・100 の 2 コマは、ずっと pattern0 だけが見える）。今の `frame_at`（`timeline.rs`）の「待ちは出す前の待ち」という決まりからの帰結で、設計の誤りではないが、作者が驚く所なので対応表（要件 10.2）に書き、`timeline_repeat_tests.rs` に 1 本置くこと。

### 3. 時刻（`SerikoClock`）と、表示層の早い戻り

時刻は筋が通っている。

- 時計を渡さない起動（今の `spawn_seriko`・`spine.rs` の檻・seriko のテスト）は「直前の刻みの時刻」を使う。これは今の `refresh_parts` が `last_seen` を使うのと同じで、既存のテストは動かない。時計を注入するテストは注入した値だけで決まる。実際の時計を読む経路は本番の結線だけ。「注入した時刻が観測を追い越さない」決まりにも触れない（待つ対象の条件を時刻の前進が壊す形は足していない）。
- 別スレッドの刻みが、合図を処理した時刻より僅かに前の値で届くことは起きる。設計は「経過は 0 で止める」と書いている。**加えて、出来事の時刻を `LoopRuntime` の `last_seen`（単調性の番人・`on_tick` の頭）に入れないことを明記すること**。入れると、その僅かに前の刻みが「非単調」として捨てられる。
- Goals の「時計の開始は、表示の出来事が運ぶ時刻そのもの」は、窓の知らせには当たるが、台本の合図（`\s`・`\b`・着せ替え）は「seriko が処理するときに時計を読む」。言い方を揃えること（中身は妥当）。

表示層の早い戻りは、**このままでは安全でない**（重大な指摘 1）。

### 4. 要件の裁定の覆い方

| 要件 | 設計の道 | 判定 |
| --- | --- | --- |
| 2.3・2.7・2.8（回数つきは現れたら始め直す・見え続けていれば続き） | 回数つきの時計は「評価のとき見えていなければ捨てる」。面が隠れたら全部捨てる | シェルは成り立つ。面の切り替えの直後の `refresh` でも捨てることが書かれていない（軽い直し）。バルーンは重大な指摘 3 |
| 3.2・3.3（終わりなしは巻き戻さない） | 時計の鍵に面の番号が無い・終わりなしは捨てない | 成り立つ |
| 2.9（遅れたら過ぎた分だけ進む） | 今のコマは「今 − 開始」だけで決まる | 成り立つ |
| 6.1〜6.7（バルーン） | 同じ分解・同じ時計（鍵に面の種類）・窓の知らせ・先送り | 仕組みは 1 つ。6.4 は重大な指摘 1 を直せば成り立つ。6.3 は、外から所有される対象への指令が可視性を変えない今の作り（`apply_show` の `visualize`）で成り立つ |
| 1.13（透ける形は表示中のコマ） | マスクは合成のたびに原寸のバイトから作る（`apply_show`） | 成り立つ。仕組みの追加 0 |
| 4.x（`always`） | 引き金・経過 0・`always_at`・一番上は切り替えで再生を捨てる（`on_surface_changed`） | 成り立つ |
| 7.x（動かないシェル） | 動く絵 0・`always` 0 なら足した腕を通らない | 成り立つ。`emo2` でも窓の知らせ（開く・閉じるたびに 1 通）は seriko へ届くが、表の門が偽なので指令は 0 件。回数を数えるテスト（Integration 5-①）で固定される |
| 8.x（記録） | 記録は表を作るときだけ | 成り立つ。バルーンはスコープごとに表を作るので、同じ絵でもスコープの数だけ `warn!` が出る（設計は明記済み。要件 8.4 の「読み込み 1 回につき 1 回」との読み合わせを対応表に 1 行） |

### 5. 外形

- `always` の外形を全部の pattern の和集合にするのは、外形を「コマに依らない静的な量」（`Extent` の定義・`plan.rs`）のまま保つ唯一の形で、妥当。
- 動く絵の子を「画像の element と同じ 1 行」で数えるのは良い判断。未着手の `areka-P0-extent-element-offset` が直す 1 行（画像の element の X,Y を足していない）を直せば、子も一緒に直る。
- `flatten_extent` を `plan_extent.rs` へ移すのは、`plan.rs` を 1,000 行以下に保つために要る。roadmap は「`flatten_extent` は `extent-element-offset` の持ち物（`plan.rs`）」と書いているので、約束の書き直しが要る（重大な指摘 2）。
- 今の `flatten_extent` は着せ替えの pattern0 を「index が最小の pattern」で取り、命令の経路は「index が 0 の pattern」で取る（既存の食い違い）。本 spec の範囲ではないが、`always` の和集合を足すとき同じ関数を触るので、移すだけで直さないことを設計に 1 行書いておくと、レビューが迷わない。

### 6. ファイルの大きさ・テスト・タスクの数

- 行数の見込みは実物の今の値と合い、どれも 1,000 行以下に収まる見通し。`mod.rs`（883 → 約 892）は `areka-P0-balloon-lifecycle-events` も触るので、余裕は合わせて見ること。`spine.rs`（1,000 行ちょうど）に触らない工夫（時計つきの起動を別の関数にする）は妥当。
- テストの計画は、要件 9.1 の列挙を全部覆っている。足りないのは 3 本: ①隠れているバルーンの読み戻し（重大な指摘 1）②窓を閉じてすぐ出したときの回数つきの絵（重大な指摘 3）③「合成が描いた先」と「見える部品」の突き合わせ（`research.md` の Risks に書いてあるが、`design.md` の Testing Strategy に載っていない）。
- タスクの数 22〜26 は現実的。数えると、合成 6（分解・鍵と見える部品・欄・経過 0・外形の移動と追加・検体）＋ seriko 7（計算・表・部品の時計・一番上・バルーンの面・状態・アクターと時計）＋表示 2 ＋結線 3（知らせ・時計・E2E）＋文書 3（台帳・対応表・申し送りと roadmap）＋測定 1 ＋実機 1 ＝ 23 前後。**roadmap の本 spec の行は「M（14〜17）」「`plan.rs` に触らない」のままなので、規模を L へ直す**（重大な指摘 2 に含める）。

## 重大な指摘

### 🔴 指摘 1: 合成の先送り（`apply_show` の早い戻り）は、書かれた条件のままだと 2 か所を壊す

**何が起きるか**

設計の「合成の先送り」は、`apply_show`（`crates/areka-emo-present/src/presenter/show.rs`）の頭で「外から所有され・見えておらず・面の番号と着せ替えが同じ・面が確立済み」なら `last_show` のコマだけを差し替えて戻る、と書いている。

1. **窓を出す通し直しが、自分の早い戻りに掛かる**。`show_target`（`presenter/visibility.rs`）は、対象がまだ見えていない状態で、`last_show` と同じ入力で `apply_show` を呼ぶ。上の 4 条件を全部満たすので、書かれたとおりに実装すると合成されずに戻り、その後に可視化される。出るのは**隠れる前の古い絵**である。System Flows の節は「コマだけが違う指令」と書いていて、そう読めば通し直し（コマも同じ）は掛からないが、Components の節の条件にはそれが無い。設計の決め手（出た瞬間に正しいコマ）が、書かれていない条件に乗っている。
2. **`last_show` を読む既存の口が壊れる**。`EmoPresenter::last_shown`（`presenter/snapshot.rs`）と `read_back`（`presenter/read.rs`）は、`last_show` の 3 つ組で合成の覚えを引く。先送りで `last_show` に「まだ合成していないコマ」を入れると、引き当てが外れる。MCP の `dump_balloon`（`crates/areka/src/mcp/dump_balloon.rs`）は「隠していても最後に表示した絵が残る」前提で `last_shown` を呼んでいるので、動く絵を置いたバルーンが隠れている間、`the last shown picture is no longer kept` の失敗を返すようになる（完了 `areka-P0-mcp-dump-images` の後退）。`last_show` の意味が「最後に表示が成立した入力」から「最後に届いた入力」へ黙って変わることが根にある。

**直し方（形は変えない）**

- `last_show` は「最後に表示が成立した入力」のまま触らない。先送りしたコマは、対象ごとの別の欄（例: 「出すときに使うコマ」）に置く。
- `show_target` は、その欄が在ればそのコマで、**先送りを通らない入口**から合成の漏斗を通す（早い戻りを `apply_show` の外側の入口に置き、`show_target` と `refresh_scale` は内側を直接呼ぶ、など）。成立したら欄を空にする。
- 早い戻りでも応答（`reply`）は必ず 1 回返す（`show_target` は「全経路で高々 1 回応答してから戻る」ことに頼っている）。
- テストを足す: 隠れている間にコマだけが違う指令を送った後、`last_shown` が絵を返すこと／`show_target` の後の絵が最後の指令のコマであること（これは既に計画に在る）／`dump_balloon` が隠れているバルーンで失敗しないこと。

**どの要件に効くか**: 6.4・3.7・6.1、完了 spec `areka-P0-mcp-dump-images` の非退行。

### 🔴 指摘 2: 他セッション・文書との調整が、挙がっているだけで済んでいない

**何が起きているか**

開発者の裁定は「スコープが膨らむなら関係しそうな他セッションと調整」。設計は調整点を 3 つ挙げたが（`plan.rs` を `areka-P0-element-base-method` と分け合う・`flatten_extent` の持ち主・`emo2_boot/mod.rs`）、どれも「先に決める」で止まっている。実物も古いままである。

- `requirements.md` の Boundary Context は今も「`plan.rs`・`areka-emo-present` ほかに触らない。**破るなら止めて開発者へ報告する**」と書いている。設計はこの約束を外したと宣言しているが、要件の側に裁定の日付つきの書き足しが無い（裁定で境界を変えたら、要件・brief・steering まで追随させる決まり）。
- steering `roadmap.md` の本 spec の行は「M（14〜17）」「`bind_atlas` の直後＝parsers・`manifest.rs`・`plan.rs` に触らない」。`extent-element-offset` の行は「`plan.rs` の `flatten_extent`」。C4 の列は `element-base-method`（C4-③）と本 spec（C4-⑫）を並走に置いている。
- `element-base-method` は `push_static_element_ops`（`plan.rs`）に描画メソッドの扱いを足す見込みで、本 spec は同じ関数の `ElementKind` の振り分けに腕を足し、同じファイルから外形の約 130 行を抜く。並走のままだと、後から入る側の取り込みが大きく割れる。

**直し方**

- タスク生成の前に、開発者（棚卸のセッション）と次を決めて書く: ①`element-base-method` と本 spec のどちらが先に main へ入るか（外形の移動は本 spec だけが行う、で固定してよい）②roadmap の 3 行（本 spec の規模と触るファイル・`extent-element-offset` の持ち物が `plan_extent.rs` へ移ること・C4 の列の並走の印）③`requirements.md` の Boundary Context と `brief.md` の同じ約束の行に、裁定の日付と「設計の『触るファイルと並走の重なり』が正本」の書き足し。
- タスクの頭に「main を取り込んで `plan.rs` の今の形を引き直す」を置く。

**どの要件に効くか**: 10.6・10.7、開発者裁定（2026-10-05 設計討議）。

### 🔴 指摘 3: バルーンを閉じてすぐ出すと、回数つきの絵の古いコマが 1〜2 フレーム見える道が在る

**何が起きるか**

回数つきの絵の始め直し（要件 2.3）は、バルーンでは「閉じた知らせ → seriko が時計を捨てて欄を経過 0 に戻す指令を出す → 表示層がその指令を受ける」の往復で成り立つ。知らせはフレームの終わりに出て、seriko は別スレッドなので、戻りの指令が着くのは 1〜2 フレーム後になる。その前に窓がもう一度出ると（台本が終わって閉じ、すぐ次の台本で出る場面）、`show_target` は古い入力（止まった最後のコマ）で合成する。見えるのは「最後のコマが 1〜2 フレーム → 1 枚目」で、要件 2.3（表示されていなかった状態から表示されたら 1 枚目から）と、「1 フレーム遅らせる解は取らない」決まりに触れる。

設計の「残る遅れ」の節は、スレッドの境の遅れを「全部の指令に同じだけ掛かるので、コマを出しておく時間は変わらない」と書いているが、この場面では**絵の中身**が違う。

**直し方（どちらかを設計討議で選ぶ）**

- 案 A（0 フレーム）: 窓を出すとき（`show_target`）、通し直す入力から「回数つきの子の欄」を外す（＝経過 0）。回数つきかどうかは対象の面の表の `FilmSheet.laps` で分かるので、表示層は時刻も seriko も要らない。決まりは「隠れていた対象を出すとき、回数つきの絵は必ず経過 0 から」の 1 つ。飛んでいる途中の古い指令が後から着く僅かな窓は残るので、それは「残る遅れ」に書く。
- 案 B: 何も足さず、「残る遅れ」にこの場面を書いて開発者の裁定をもらう。

どちらでも、テスト（閉じる → 戻りの指令が着く前に出す → 最初の絵が経過 0）を `presenter_film_tests.rs` に置くこと。

シェルの側（`\s` で消えて現れる）は seriko の中だけで閉じるので、この道は無い。

**どの要件に効くか**: 2.3・6.1。

## 良い所

- **`always` を根で直した**。前の版の 4 つのつまずき（コマが届くまで絵が無い・外形・透ける・番号）を、「合成が `always` を知らない」という 1 つの原因にまとめ、経過 0・外形・見える部品の 3 か所に同じ決まり（`is_always_interval`・`rest_index`）を置いた。手書きの `always` と自動の分解が同じ決まりで動き、最初の指令から絵が出る（0 フレーム）。空の `PatternState` が「全部が経過 0」と等しいので、合成の鍵も余分に増えない。
- **番号の問題を型で消した**。子はサーフェスの番号を持たず、面の索引に載らない。`\s[番号]`・別名・作者の定義から子へ届く道が型から無く、遮る検査が 0 個で済む。面の一覧をなめる 7 か所のうち 5 か所が変更 0 になる。
- **触るファイルを全部さらけ出した**。約束を外した所に印を付け、ほかの spec の持ち物との重なりを表にしている。調整の材料として、そのまま使える。
- **繰り返しが 1 つの純粋な計算**（`always_at`）。回数つきの始め直しが「回数を持つ時計は見えなくなったら捨てる」という、子に限らない決まりになっていて、状態（止まった・動いている）を別に持たない。

## 軽い直し（開発者に問わずに直せるもの）

1. Components の「合成の先送り」の条件に、System Flows と同じ「コマだけが違う」を書く（指摘 1 の直しを入れるなら、そちらで置き換わる）。
2. 「`LoopTrigger` の腕を決め打つ 2 か所」を 3 か所へ直す（`table.rs` の `from_world`・`parts.rs` の `gate`・`looper.rs` の `on_tick` の抽選）。
3. `NestTable::from_world` の「行を載せる条件」に `films`・`always_rest` を足す、と書く。
4. 面の切り替え・着せ替えの変化の直後の `refresh` も、見えなくなった回数つきの時計を捨てる、と書く（今は「評価のとき」としか書いていない。`\s[A]` → `\s[B]` → `\s[A]` が刻みの間に続くと、捨てずに続きになる）。
5. 出来事の時刻は `LoopRuntime` の `last_seen`（単調性の番人）に入れない、と書く。
6. Goals の「時計の開始は、表示の出来事が運ぶ時刻そのもの」を、「窓の知らせは送り手の時刻・台本の合図は seriko が処理した時刻」へ揃える。
7. 動く絵の子の命令（`BlitOp`）は、置いた element の描画メソッドをそのまま運ぶ、と書く（element の位置・順と同じく「静止画のときと同じ」にする）。
8. 「同じ面の表に `bind_atlas` を呼ぶのは 1 度だけ」を分解の前提に書く。
9. `flatten_extent` の既存の食い違い（着せ替えの pattern0 を「index が最小」で取る）は、移すだけで直さない、と書く。
10. 対応表（要件 10.2）に書くことへ 2 行足す: 手書きの `always` で pattern0 の待ちが 0 のとき最後のコマは 0 ミリ秒になること／バルーンの記録はスコープごとに 1 回出ること。
11. Testing Strategy に 3 本足す: 「合成が描いた先」と「見える部品」の突き合わせ（`research.md` の Risks に在るもの）／pattern0 の待ちが 0 の `always` の周の境目／`NestReport` が `ElementKind::Film` を「無い番号」に数えないこと。
12. バルーンの表の差し替えの後、窓が開いたままのスコープの時計は「次の刻みの時刻」で生まれる（知らせは変化が無いので出ない）。その間は経過 0 の絵が出るので絵は欠けない。Flows に 1 行書く。

## 設計討議へ持ち込むこと

- 重大な指摘 1〜3。
- 設計の Open Questions 3 件（刻みは 16 ミリ秒のままでよいか／隠れているバルーンへ指令が流れ続けてよいか／手書きの `always` の外形は和集合でよいか）。3 件とも答えで作業が変わるので、開発者に問うのが正しい。本検証の見立ては、3 件とも設計の仮の案のままで進めてよい（刻みの作り替えは今のアニメーション全部に関わるので別の spec）。
- 「子は手書きの部品と字面まで同じではない（コマが絵を直接指す）」ことを、開発者が「本質的な案」と認めるかの一言の確認。

## 次の一手

1. 設計討議（`/kiro-design-discussion areka-P0-animated-image-playback`）で、重大な指摘 3 件と Open Questions 3 件を片付ける。
2. 軽い直し 12 件を `design.md` に入れ、指摘 2 の文書（`requirements.md` の Boundary Context・`brief.md`・roadmap の 3 行）を直す。
3. その後に `/kiro-spec-tasks areka-P0-animated-image-playback`。
