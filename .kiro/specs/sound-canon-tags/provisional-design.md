# 暫定の設計: sound-canon-tags（2026-11-19）

## 1. 前置き

- これは**暫定の設計**である。正式な `design.md` ではない。要件定義（`requirements.md`）もまだ無い。brief と、pasta の骨子・取り決めから暫定の要件を起こし、設計の段まで一度降りて、上位の決定へ差し戻しが要る矛盾を探すために書いた。
- 日付: 2026-11-19。基にした main: `49b6f3ab5`（ワークツリー `claude/great-darwin-31de86`・HEAD `018d102c0`）。
- 実行して確かめた物は 0。コードは読んだだけ。正典は ukadoc の MCP で引いた（URL と逐語の引用を付けた）。
- 読んだ物:
  - areka: `.kiro/specs/sound-canon-tags/brief.md`・隣の brief 3 本（`parallel-visual-script`・`talk-fast-forward`・`sakura-time-directives`）・`talk-script-metadata` と `property-catalog-lists` の brief の音にかかわる行・網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![sound,…]` の 9 行と別名の 2 行。
  - areka のコード: `crates/dola/src/cue/{runtime,sink,schedule,command}.rs`・`crates/areka-sakura/src/{compile,drive}.rs`・`crates/areka-parsers/src/sakura/decode.rs`・`crates/areka-runtime/src/{zorder_cue,consumer_ledger,talk_clock}.rs`・`crates/areka/src/session/runtime_boot.rs`・`crates/areka-ghost/src/{sink,dispatcher,ticker,prop_get_sink}.rs`・`crates/areka-sylphya/src/vocab/dotted.rs`。
  - pasta（読むだけ）: 骨子 `final-model-2026-11-19.md` の全部・取り決めの brief の「決定（2026-11-19…最終形）」の節。
  - areka の側の見立て（参考）: `areka-novel-skip-contents-2026-11-19.md`・`areka-novel-final-form-check-2026-11-19.md` の音の行。
  - ukadoc: サウンドコマンドの 11 項目・オプションの注記・サウンドのプロパティ・`OnSound*` の 4 イベント。

## 2. 暫定の要件

出どころの略: 【brief】＝この単位の brief、【骨子】＝`final-model-2026-11-19.md`、【取り決め】＝pasta の `novel-areka-contract/brief.md` の最終形の節。

1. `\_v[ファイル名]` か `\![sound,play,ファイル名,オプション…]` を受けたら、areka は、その wave・mp3・wma を 1 回だけ鳴らす。【brief Scope】
2. `\8[ファイル名]` を受けたら、areka は、その wave を 1 回だけ鳴らす。【brief Scope】
3. `\![sound,loop,ファイル名]` を受けたら、areka は、止められるまで繰り返して鳴らす。ループの音は `\_V` の待ちの対象にしない。【brief Current State・取り決め「命令の表」】
4. `\![sound,stop,ファイル名]` を受けたら、areka は、その名前の音を止めて手放す。名前が無ければ、そのゴーストの音を全部止める。【brief・取り決め「止めるときは名前を付ける」】
5. `\![sound,pause,…]`・`\![sound,resume,…]` を受けたら、areka は、名前の音（無ければ全部）を一時停止・再開する。【brief Scope】
6. `\_V` か `\![sound,wait]` に着いたら、areka は、鳴っている単発の音が終わるまで、その流れを待たせる。鳴っていなければ待たない。待ちが解けた後の合図は、台本に書いた間隔を保つ（待った長さだけ後ろへずれる）。【brief Desired Outcome・Constraints】
7. `\![sound,load,…]` を受けたら先読みし、`\![sound,option,…]` を受けたら鳴っている音のオプションを変える。オプションは `--volume`・`--balance`・`--rate`・`--seektime` を持つ。【brief Scope】
8. ループの音を 2 本以上、単発の音を何本でも、同時に重ねて鳴らせる。同じファイルの単発も重ねられる。【取り決め「曲・環境の音・効果音・声」・骨子 2.4 `＆環境音：空調、遠い街`】
9. 音の寿命は、トークの寿命に結び付けない。トークが終わっても、置き換わっても、中断されても、鳴っている音は続く。【骨子 2.4「書いていない物には触らない（前の音が続く）」・2.2 の 1 本ずつ進む形】
10. ゴーストを切り替える・降ろす・areka を終えるとき、areka は、そのゴーストの音を全部止めて手放す。【brief Desired Outcome「食い違わない」・仮定 A4】
11. areka は、音を「ループ」と「単発」に分けて覚える（分け目はタグの形。`loop` で始めた物がループ、ほかは単発）。「単発を全部止める」操作を、ほかの受け手から呼べる口として出す。【取り決め「出し直しの印…頭で、鳴っている途中の単発の音と声を止め」】
12. 並走の流れから届いた音のタグは、主の流れから届いた物と同じに効く。並走の流れを止めても、もう鳴っている音は止めない。【取り決め「並走」・`parallel-visual-script` の brief】
13. クリックの早送りで飛び越えた単発の音は鳴らさない。ループ・停止・一時停止・オプションは当てる。早送りは `\_V` の待ちを切り上げる（音は鳴り続ける）。【brief Adjacent `talk-fast-forward`・仮定 A6】
14. `currentghost.sound.count`・`currentghost.sound(要素名).…`・`currentghost.sound.index(ID).…` の 18 葉が本物の値を返す。`playing`・`pause`・`position` は SET で効く。【brief Desired Outcome】
15. ファイルが無い・形式が読めない・音の装置が無いとき、areka は落ちず、`warn!` を 1 行残して先へ進む。メッセージボックスは出さない。`\_V` は待たない。【brief Desired Outcome・Constraints】
16. 読み込み・デコード・再生は UI スレッドの外で行う。台本のスレッドも止めない。【brief Approach】
17. 相対パスは、台本を実行したゴーストの `ghost/master` から解く（`talk-script-metadata` の関数を使う）。絶対パスはそのまま。【brief Approach】
18. 音ごとの音量を、時間をかけて変えられる作りにしておく（この単位では口を出さない。後のフェードの土台）。【brief Desired Outcome「後で…積める形」・骨子 2.9 フェーズ 2「曲のフェード」】
19. 音楽 CD（`cdplay`）・動画（`--window`・`--sound-only`）・`--spectrum*`・`--eq*` は作らない。受けたら `warn!` を 1 行残して、その部分だけ読み捨てる。【brief Scope Out・仮定 A8】
20. 待ち・止め・一時停止・ファイルが無いとき・ゴーストの切り替えを、偽の再生の土台と注入した時刻で、決定論のテストで固定する。【brief Scope In】

## 3. 今の作り（確かめた事実）

- **音の層は無い。** `rodio|cpal|PlaySound|XAudio2|MediaFoundation|Win32_Media|IMFSourceReader|MFStartup|kira` を、リポジトリの `Cargo.toml` と `*.rs` で grep して 0 件（`target`・`vendors` を除く）。`crates` の `*.rs`（テストを除く）で `sound` を grep して当たるのは `crates/areka-sylphya/src/vocab/dotted.rs:160` の注釈 1 行だけ。
- **網羅台帳**: `\![sound,…]` の 9 行のうち 8 行（`cdplay`・`load`・`loop`・`option`・`pause`・`play`・`resume`・`stop`）は `status = "absent"`・`owner = ""`（`doc/ukadoc-coverage/ledger/sakura-script.toml:3444-3565`）。`\![sound,wait]` は `vocabulary-only`・`owner = "sakura-time-directives"`（同 `3567-3579`）。`\_v`・`\_V` は別名の行（同 `3925`・`4361`）。
- **`\_v`・`\_V` の今の道**: 引数つきの腕の無いタグは `decode_passthrough_tag` で `Instruction::Raw` になる（`crates/areka-parsers/src/sakura/decode.rs:476-484`）。引数の無い腕の無いタグは `decode_passthrough_bare`（同 `505-508`）。`Raw` は compile が合図を作らず `warn!`「知らないタグを捨てる」を残す（`crates/areka-sakura/src/compile.rs:411-419`）。
- **`\![sound,…]` の今の道**: `decode_passthrough_bang` が `GenericCommand{name, raw_args}` にし（`decode.rs:492-497`）、compile が長さ 0 の `CueCommand::Custom`（汎用の運び手）にして台本に積む（`compile.rs:282-288`）。受け手は居ない。
- **運び手と受け手の決まり**: `CueCommand::Custom{command, params}` は命令ごとの型を増やさず、受け手が名前で自分宛てを選ぶ（`crates/dola/src/cue/command.rs:118-133`・`crates/dola/src/cue/sink.rs:100-150`）。「名前＋第 1 引数」の持ち主は `ConsumerLedger` に 1 行で登記し、1 つの出現に受け手は高々 1 つ（`crates/areka-runtime/src/consumer_ledger.rs:1-55`）。受け手の手本は `ZOrderCueSink`（`crates/areka-runtime/src/zorder_cue.rs:74-120`）。受け手の並びは `runtime_sinks`（`crates/areka/src/session/runtime_boot.rs:80-93`）。
- **受け手はトークごとに複製される**: `BootCueSink::clone_box` と `begin_talk(talk_id, origin)`（`crates/areka-ghost/src/sink.rs:35-45`）。台本に付く情報は今は送り手の種類だけ（実行するゴーストの置き場は `talk-script-metadata` が足す）。
- **待ちの区切り**: dola の `BarrierKind` は `WaitForInput`（`\x` の受け皿・今は compile が出さない）と `WaitForChoice` の 2 つ（`command.rs:75-86`）。区切りに着くと `TimedSchedule::tick` は止まり、外からの `notify_barrier_resolved` でだけ解ける（`crates/dola/src/cue/schedule.rs:180-231`）。**解けた後の合図の時刻は付け替えない**（到達の判定は「アンカー＋相対」のまま・同 `188-193`）。途中の区切りの後ろの時刻をどう付け替えるかは、`talk-fast-forward` の brief の議題 ⑴ で未決（同 brief 88 行目）。
- **台本のスレッド**: `spawn_talk` がトークごとに 1 本。`SakuraMsg` は `Start`・`Tick`・`Close`・`ResolveChoice` の 4 つ（`crates/areka-sakura/src/drive.rs:208-215`）。刻みは ticker の 50ms（`crates/areka-ghost/src/ticker.rs:66`・時刻源は `GetTickCount64`・同 `77-80`）。
- **トークの枠は 1 つ**: dispatcher は新しいトークの前に前のトークを閉じる（`crates/areka-ghost/src/dispatcher.rs:328`・`420`）。受け手の複製は破棄される。音をトークの受け手の中に持つと、置き換えで消える。
- **プロパティ**: 18 葉は名前だけの表 `SOUND_PROP_NAMES`（`dotted.rs:170` 以降）。値の導出は無い。`\![get,property,…]` は `SylphyaReader::resolve_dotted_str` で引く（`crates/areka-ghost/src/prop_get_sink.rs:1-30`）。
- **依存**: ワークスペースは `windows` クレートを使っている（`Cargo.toml:19-51`）。Media Foundation・XAudio2 の features は入っていない（上の grep 0 件）。

### 正典（ukadoc を引いた・2026-11-19）

- `\![sound,loop,ファイル名]`:「ループ再生する。\_Vの対象にならない。再生を止める場合は\![sound,stop]を実行する。」（<https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-sound-loop>）。**形にオプションの欄が無い。**
- `\![sound,stop,ファイル名]`:「再生を完全に停止する。pauseと異なり音声データを解放し完全に再生停止する。このコマンドで停止しないと、別の音声データを再生できない場合がある。ファイル名を省略した場合、全再生中のサウンドを停止する。」（同 `#bang-sound-stop`）。
- `currentghost.sound.count`:「現在のゴーストが管理しているメディアの数。\![sound,play]・\![sound,loop]・\![sound,preload]などで開かれ、まだ閉じられていないものすべてが対象。」（<https://ukatech.shillest.net/ukadoc-spec/list_propertysystem.html#currentghost.sound.count>）。
- `currentghost.sound(要素名)`:「要素名が一致する最も新しいものから情報を取得。要素名は\![sound,play,～]などで指定したファイル名/パス/URLそのもの。」（同 `#currentghost.sound`）。
- オプションの注記（同 `list_sakura_script.html#bang-sound-option.note`）: `--volume=0～100`・`--balance`・`--rate`・`--seektime`・`--window`・`--sound-only`・`--spectrum*`・`--eq*`・`--eq-preamp`。**時間をかけて音量を変えるオプション（フェード）は無い。** 止める命令にも時間の欄は無い。
- `\_V`:「\_vのファイル再生が終了するのを待つ。既に終了しているか存在しない場合は無視される。」（同 `#_V-2`）。
- イベント: `OnSoundStop`（Reference0 再生ファイル名・Reference1 終了理由）・`OnSoundLoop`・`OnSoundError`・`OnSoundSpectrum`（<https://ukatech.shillest.net/ukadoc-spec/list_shiori_event.html#OnSoundStop> ほか）。brief の Scope は、これらに触れていない。

**問い「曲と環境の音を同時に・効果音を重ねる・名前で止める、が正典で成り立つか」の答え**:

| 事 | ukadoc の字面 | 判定 |
| -- | ------------- | ---- |
| 名前で止める | 上の `stop` の引用（ファイル名を取る） | 成り立つ（明文） |
| 複数の音を同時に持つ | 「全再生中のサウンド」「管理しているメディアの数」「要素名が一致する最も新しいもの」 | 示唆は在る。「同時に鳴る」と言い切った文は無い |
| 逆向きの注意 | 「このコマンドで停止しないと、別の音声データを再生できない場合がある」 | 同時に鳴らせない場合を言っているとも読める |
| ループとオプション | `loop` の形にオプションの欄が無い | 曲の音量は `option` で後から変える形になる |

areka は、要件 8 のとおりに作れば成り立つ。SSP で成り立つかは、字面では決まらない（骨子の 7 章も「SSP の実機で確かめていない物: …音のタグ」と書いている）。9 章の 1。

## 4. 設計

### 4.1 構成

```
台本のスレッド（トークごと・並走の流れごと）          音のスレッド（1 本・areka が生きている間）
  SoundCueSink ──SoundMsg（mpsc）──────────────▶ SoundActor
   （名前 "sound" を自分で選ぶ）                      ├ SoundTable（要素の表・純粋な状態機械）
                                                      └ SoundBackend（trait）
  ◀── 待ちの解け（dispatcher 経由の伝言）──────          ├ 本番: Media Foundation で読む＋XAudio2 で混ぜる
ghost のスレッド: プロパティの GET／SET ──────────▶        └ テスト: 偽の土台（時刻を注入）
切り替え・終了の所: StopAllOf(ゴースト) ─────────▶
```

| 置き場 | 足す物 |
| ------ | ------ |
| 新しい crate `areka-sound` | `SoundCommand`（運び手の字の列を読む純粋な関数）・`SoundTable`・`SoundBackend`・`SoundActor`（`areka-actor` の `spawn_actor` に乗る）・本番の土台 |
| `areka-parsers` の `sakura/decode.rs` | `\_v[f]` → `GenericCommand{"sound", ["play", f]}`、`\_V` → `GenericCommand{"sound", ["wait"]}`、`\8[f]` → 同じ運び手（別名として畳む。台帳の別名の行と同じ向き） |
| `dola` の `cue` | 外の出来事を待つ区切り `BarrierKind::WaitForExternal{key}`（`key` は不透明な字。dola は語彙を持たない）・`CuePlayer::resolve_external(key)`・状態 `WaitingForExternal` |
| `areka-sakura` の `compile.rs`・`drive.rs` | `sound,wait` を読んだら、運び手の合図の直後に `WaitForExternal{"sound"}` を置く（`set,choicetimeout` を名前で読む前例と同じ所・`compile.rs:289-306`）。`SakuraMsg::ResolveExternal{key}` を足す |
| `areka-ghost` の `dispatcher.rs` | `ResolveExternal{talk_id, key}` を、番号の合うトークへ回す（`ResolveChoice` の回し方と同じ） |
| `areka-runtime` | `sound_cue.rs` の `SoundCueSink`・`ConsumerLedger` に `("sound", None)` の 1 行 |
| `areka` の `session/runtime_boot.rs` | 音のスレッドを起こす・受け手の並びに `SoundCueSink` を足す・切り替えと終了で `StopAllOf` を送る |
| プロパティ（`areka-sylphya`・`areka-ghost`） | 18 葉の GET を、問い合わせの時に音のスレッドへ聞く口へつなぐ。SET の 3 葉を `SoundMsg` に直す |

命令ごとの型は増やさない（`Custom{command:"sound", params}` のまま運ぶ）。`SoundCommand` は受け手の中だけの型である。

### 4.2 主な型

- `SoundCommand`: `Play{name, opts}`・`Loop{name}`・`Stop{name?}`・`Pause{name?}`・`Resume{name?}`・`Load{name, opts}`・`Option{name, opts}`・`Wait`。読めない形は `Err` で、受け手が `warn!` を 1 行残す。
- `SoundMsg`（音のスレッドへの伝言）: `Command{ghost, base_dir, cmd, skipped}`（`skipped` は早送りで飛び越えた印）・`NotifyWhenOneShotsEnd{ghost, reply}`・`StopOneShots{ghost}`（出し直しの印の受け手が呼ぶ口・要件 11）・`StopAllOf{ghost}`・`Query{…, reply}`・`Set{…}`・土台からの `Ended{id}`／`Looped{id}`／`Failed{id, why}`。
- `SoundTable` の要素: `id`（開いた順の通し番号）・`ghost`・`name`（台本に書いた字のまま）・`path`（解いた後）・`looping`・`preload`・`state`（`Loaded`／`Playing`／`Paused`）・`volume`・`balance`・`rate`・`gain_ramp`（後のフェード用。この単位では常に無し）。並びは新しい順（`index(ID)` の「新しく開かれたものから順に 0,1,2」）。
- 要素が表から消える時: `stop`・単発の終わり・ゴーストの切り替え。`load` だけした物は残る（`count` の「まだ閉じられていないものすべて」）。

### 4.3 流れ

- **鳴らす**: 合図が着く → `SoundCueSink` が `SoundCommand` に読み、`begin_talk` で受けた付帯情報（実行するゴースト・`ghost/master`）を添えて送る → 音のスレッドがパスを解き、表に要素を作り、土台に開かせる。受け手は返事を待たない（要件 16）。
- **同じ名前をもう一度**: 単発は、新しい要素を足して重ねる（要件 8）。ループは、鳴っている同じ名前の要素を止めて、頭から鳴らし直す（重ねない）。仮定 A2。
- **待つ（`\_V`）**: `sound,wait` の合図が着く → 受け手が `NotifyWhenOneShotsEnd` を送る → 同じ刻みで再生機が `WaitForExternal` に着いて止まる → 音のスレッドは、その時に鳴っている、そのゴーストの単発の要素の組を控え、全部が終わった時（無ければすぐ）に `reply` を打つ → dispatcher 経由で `SakuraMsg::ResolveExternal` が台本のスレッドに届き、区切りが解ける。合図の配りと区切りへの到達は同じ `tick` の中で、返事は必ずその後の伝言として着く（順は台本のスレッドの作りで決まる。追い越さない）。
- **止まっている物・一時停止の物**: 一時停止の単発は、終わるまで待つ（再開されなければ解けない。トークの置き換えか早送りで抜ける）。
- **トークの置き換え・中断**: 再生機ごと破棄される。待ちの登録は、返事の送り先が閉じていれば黙って捨てず `debug!` を残して消す。音は続く（要件 9）。
- **ゴーストの切り替え・終了**: 切り替えの所と終了の所が `StopAllOf` を送る。areka の終了では、音のスレッドを止めてから土台を閉じる。
- **舞台を閉じる**: 音の層は何もしない。止めるのは pasta（`＆曲：なし`）。仮定 A5。

### 4.4 再生の土台

- 読むのは Media Foundation の Source Reader（wav・mp3・wma を PCM にする）。混ぜるのは XAudio2（要素 1 つに source voice 1 つ）。短いファイルは全部読んでから渡し、長いファイルは少しずつ読んで足す。
- XAudio2 を併せる理由: ループの継ぎ目に間を空けない・効果音の出だしを速くする・何本でも重ねる・音量とバランスと速度を要素ごとに持つ・終わりとループの折り返しを土台から知らせてもらう。
- 採らなかった形: Media Foundation の Media Engine を要素ごとに 1 つ（書く量は減るが、ループの継ぎ目・出だしの遅れ・多数の重ねで不利。後の `--spectrum`・`--eq` にも手が届かない）。
- 音量を時間で変える仕組み（要件 18）は、音のスレッドが、動いている変化が在る間だけ短い刻みで `SetVolume` を進める形にする。この単位ではテストだけが使う。

### 4.5 状態の持ち方・時刻・値の出し方

- 状態の正は、音のスレッドの `SoundTable` 1 つ。トークの受け手・UI・ghost のスレッドは写しを持たない。
- 音の終わりは「変わった時点が分かる」ので、土台からのプッシュで受ける。`\_V` は推し量った長さで待たない。
- `position` は絶えず変わるので、問い合わせの時に土台へ聞く（プル）。毎フレーム写さない。
- 待ちが解けた後の合図の時刻は、「解けた時刻＋区切りからの相対」で付け替える。到達の判定は「開始＋相対」の足し算の形を保つ（`talk-fast-forward` の brief の議題 ⑴ と同じ仕組み。6 章）。更新が遅れても、音の終わりの時刻は土台の知らせの時刻を使い、丸めない。
- 1 フレーム遅らせる所は無い。鳴らし始めは、伝言が着いたらすぐ。読み込みの遅れは `\![sound,load,…]`（正典の先読み）で消せる。

### 4.6 失敗の扱い

| 事 | 動き |
| -- | ---- |
| ファイルが無い（正典「存在しない場合は無視」） | `warn!` 1 行・要素を作らない・`\_V` は待たない |
| 形式が読めない・デコードの途中で失敗 | `warn!` 1 行・要素を消す・待っている流れを解く |
| 音の装置が無い・土台が起きない | 起動の時に `warn!` 1 行。以後の命令は、命令ごとに `debug!`。台本は進む |
| 名前の合う要素が無い `stop`／`pause`／`option` | `debug!` 1 行（正典に「無視」の明文は無い。仮定） |
| 範囲の外のオプションの値 | 正典の範囲へ収める（`--rate` は土台の上限へ）。`warn!` 1 行 |
| 作らないオプション・`cdplay` | `warn!` 1 行で、その部分だけ読み捨てる |

### 4.7 ほかの単位との境

- `parallel-visual-script`: 並走の流れは自分の再生機と受け手の複製を持つ。`SoundCueSink` は「主か並走か」を見ない。並走の中の `\_V` は、その流れだけを止める。
- `talk-fast-forward`: 早送りで飛び越えた合図に印を付けて配るのは、あちらの持ち分（6 章）。音の層は印を読むだけ。
- `sakura-time-directives`: `\![sound,wait]` の台帳の行の持ち主を、この単位へ付け替える。
- 出し直しの印 `\![play,again]` の受け手（ノベルの `play` の命令を受ける単位）: 頭で `StopOneShots` を呼ぶ。音の層は `play` の命令を自分では読まない（受け手は高々 1 つの決まり）。
- ノベルの音の拡張（フェード・区分ごとの音量）: この単位の外。4.4 の仕組みと `SoundTable` の `gain_ramp` に乗る。

## 5. pasta に期待する事

| # | 期待 | 骨子・取り決めと |
| - | ---- | ---------------- |
| 1 | 曲と環境の音は `\![sound,loop,名前]`、効果音と声は `\![sound,play,名前]` で出す | 合っている（取り決め「命令の表」「曲・環境の音・効果音・声 \| 正典の `\![sound,loop／play／stop,名前]`」） |
| 2 | 「名前」は、`ghost/master` からのファイルのパス（拡張子つき）。areka は拡張子を補わず、名前からファイルを探さない | 合っているが、空白が在る。骨子 2.4 の例は `＆曲：雨音` で、柱の値をファイルのパスへ直す所がどこにも書かれていない（別名表は「タグの頭の並び」の置き換え・骨子 2.4）。正典は「要素名は…指定したファイル名/パス/URLそのもの」 |
| 3 | 曲を替えるときは、前の曲を `\![sound,stop,前の名前]` で止めてから、新しい曲を `loop` で出す。名前の無い `stop` は使わない（環境の音も止まる） | 合っている（「止めるときは名前を付ける」） |
| 4 | 同じ曲は打たない。打てば、areka は頭から鳴らし直す | 合っている（「同じ曲は pasta が打たない」） |
| 5 | 環境の音を並べたとき（`＆環境音：空調、遠い街`）は、名前ごとに `loop` を 1 つずつ出し、止めるのも名前ごと | 合っている（骨子 2.4「柱には、値を並べて書ける」） |
| 6 | 出し直しの 1 本（`\![play,again]`）には、単発の音を書かない。曲は、替わる時だけ `stop`＋`loop` を書く。areka は、出し直しの頭でループを止めない | 合っている（取り決め「頭で、鳴っている途中の単発の音と声を止め」・骨子 4 章 C「瞬間の命令は出さない」）。「単発」の線は、areka ではタグの形で引く（9 章の 3） |
| 7 | 高速再生の 1 本には、単発の音を書かない。areka は `\_q` を見て音を変えない | 合っている（骨子 2.6「`\x` と瞬間の命令を出さない」） |
| 8 | 並走の区間に音を書いてよい。区間の中の `\_V` は、その区間の流れだけを待たせる | 合っている（取り決め「区間に書ける物は、見た目…と、音と、待ち」） |
| 9 | 曲の音量を変えたいときは、`loop` の後に `\![sound,option,名前,--volume=値]` を出す（`loop` にオプションは付けられない） | 骨子・取り決めに文が無い（空白）。正典の `loop` の形による |
| 10 | 区分ごとの音量（遊ぶ人の設定）は、pasta が掛け合わせて `--volume` に書く。areka は「曲」「効果音」「声」の区分を持たない | 未決の所と重なる（取り決め「要件定義へ送ること」の「遊ぶ人の設定（文字の速さ・音量・窓の形）を伝える命令」）。食い違いではない |
| 11 | 曲のフェードは、正典のタグだけでは出さない（出せない）。フェーズ 2 まで、切り替えは瞬間 | 合っている（骨子 2.9 フェーズ 2「曲のフェード」）。出す形は未決（8 章の 1） |
| 12 | 舞台を閉じる・演劇を終えるときに曲を止めたければ、pasta が `stop` を書く（`＆曲：なし`） | 取り決めに文が無い（空白）。取り決めの「閉じるとき、ベースウェアは、残る命令を全部『無い』に戻し」は `play` の命令の話で、音は正典のタグなので入らない、と読んだ（仮定 A5） |
| 13 | 音のイベント（`OnSoundStop` など）を pasta は当てにしない | 合っている（骨子 4 章 D「声は、効果音と同じに、作者がト書きで鳴らす」。声の終わりを待つオートは最終形に無い） |

イベント（GET か NOTIFY か・Reference の並び）で、この単位が pasta に新しく求める物は無い。

## 6. ほかの単位に期待する事

- **`talk-script-metadata`（先に着地）**: `begin_talk` で、実行するゴーストと `ghost/master` が受け手に届く事。相対パスを解く関数が、`SoundCueSink` から呼べる置き場（`areka` 本体でなく、`areka-runtime` か下の crate）に在る事。
- **途中の区切りの後ろの時刻の付け替え（`talk-fast-forward` の議題 ⑴）**: `\x` と `\_V` は、同じ仕組みが要る。`talk-fast-forward`（か、その後半 `click-wait-tag`）が先なら、音はそれに乗る。音が先なら、音が dola の `cue/schedule.rs`・`runtime.rs` に作り、`\x` が乗る。**同じウェーブに置かない**（dola の `cue/{schedule,runtime,command}.rs`・`areka-sakura` の `compile.rs`・`drive.rs`・`areka-ghost` の `dispatcher.rs` が重なる）。付け替えは、`TalkClock::observe_cue` の起点の推し量り（`crates/areka-runtime/src/talk_clock.rs:36-58`）と、seriko の口パクの写しにも及ぶ（`talk-fast-forward` の brief 64〜65 行目）。
- **`talk-fast-forward`**: 早送りで飛び越えて配った合図に、印を付ける事（受け手が見分けられる形）。早送りが `WaitForExternal` の区切りも解く事。
- **`parallel-visual-script`**: 並走の流れの受け手の組に `SoundCueSink` を入れる事（「見た目と音だけに絞る」の、音の側）。並走の流れにも `begin_talk` の付帯情報が届く事。
- **ノベルの `play` の命令を受ける単位**: `\![play,again]` の頭で `StopOneShots` を呼ぶ事。
- **`property-catalog-lists`**: 18 葉の値の導出を、この単位が出す事への同意（同 brief 76 行目「残るのは値の導出」）。`property-query-channels`: `\![set,property,…]` の道が開いている事（SET の 3 葉）。
- **`dola-interrupt-ticks`・`talk-anchor-interrupt-ticks`**: 台本のスレッドの刻み（今は 50ms）が細かくなる事。音の鳴り始めの時刻の正確さは、これに従う。
- **`sakura-time-directives`**: `\![sound,wait]` の台帳の行の持ち主の付け替え。
- **windows-rs の移行（先進坑）**: Media Foundation と XAudio2 の束ね方（今は `windows` クレートの features を足す前提）。

## 7. 置いた仮定

- A1. `\_V` が待つ相手は、区切りに着いた時に鳴っている、そのゴーストの単発の音の全部（後から鳴り始めた物は待たない）。正典は「`\_v` のファイル再生が終了するのを待つ」で、複数のときの読みを言っていない。
- A2. 同じ名前の `play` は重ねる。同じ名前の `loop` は鳴らし直す（重ねない）。正典に文は無い。
- A3. トークの終わり・置き換え・中断で、音は止めない。骨子の 1 本ずつ進む形（曲が 1 本をまたいで続く）が、これを前提にしている。
- A4. ゴーストの切り替え・降ろし・areka の終了で、そのゴーストの音は全部止める（正典の「現在のゴーストが管理しているメディア」から）。SHIORI だけの読み直し（`\![reload,shiori]`）では止めない。
- A5. 舞台を閉じても、音の層は何もしない。
- A6. 早送りで飛び越えた単発は鳴らさない（まとめて鳴ると騒音になる）。areka の側の前の見立て（`areka-novel-skip-contents-2026-11-19.md` の表「単発の音と声 \| 鳴らさない」「曲の柱…\| 当てる」）と同じ向き。
- A7. `\8[ファイル名]` の基準のフォルダは `\_v` と同じ（`ghost/master`）。正典は「自ゴーストフォルダにあるwaveを再生する」で、どのフォルダかを言っていない。
- A8. `--spectrum*`・`--eq*`・`--eq-preamp`・`OnSoundSpectrum` は、この単位の外（brief の Scope は「オプションのうち音にかかわるもの」で、線が引かれていない）。
- A9. 要素名の突き合わせは、台本に書いた字のまま比べる（パスを解いてから比べない）。
- A10. URL の名前は鳴らさない（`warn!` 1 行）。
- A11. `OnSoundStop`・`OnSoundLoop`・`OnSoundError` は、この単位に入れる（8 章の 4 の答えで外れる）。
- A12. `\_q`（字を一度に出す区間）の中でも、`\_V` は待つ。
- A13. `windows` クレートに Media Foundation と XAudio2 の features を足して呼べる（x64・arm64）。

## 8. 決まっていない所

1. **フェードを出す形はどれか**（正典のタグに areka 独自のオプションを足す `\![sound,option,名前,--volume=0,--fade=1500]`・`\![sound,stop,名前,--fade=1500]` か、`play` の命令か）。答えで変わる所: 口を `SoundCueSink` に足すか、`play` の命令の受け手から呼ぶ口にするか。4.4 の仕組みは変わらない。
2. **`\_V` は、どの音を待つか**（全部の単発か・最後に鳴らした 1 本か）。SSP の MCP で測る。答えで変わる所: `NotifyWhenOneShotsEnd` が控える組。
3. **同じ名前をもう一度鳴らしたとき、SSP はどうするか**（重ねる・鳴らし直す・無視）。SSP の MCP で測る。答えで変わる所: `SoundTable` の同名の扱いと、pasta の「同じ曲は打たない」が要る度合い。
4. **`OnSoundStop`・`OnSoundLoop`・`OnSoundError` を、この単位に入れるか**。答えで変わる所: kanade へのイベントの道（2〜3 タスク）。
5. **舞台を閉じる時に、areka が音を止めるか**。答えで変わる所: `shell-multi-compose` の閉じる所から `StopAllOf` を呼ぶか。
6. **早送りで飛び越えた単発を鳴らすか**。答えで変わる所: `talk-fast-forward` が配る印が要るか。
7. **トークの中断（ダブルクリック）で、単発の音と声を止めるか**。SSP の MCP で測る。答えで変わる所: 中断の所から `StopOneShots` を呼ぶか。
8. **`--spectrum*`・`--eq*` を、いつ・どの単位で作るか**。答えで変わる所: 4.4 の土台に解析と加工の段を足すか（規模が 6 タスクほど増える）。
9. **`\8` の基準のフォルダ**。SSP の MCP で測る。答えで変わる所: 別名に畳むか、別の運び手の名前にするか。
10. **規模が 20 を超えたときの切り方**（brief は「タグの受け口とプロパティで切る」）。答えで変わる所: 要件 14 と 8 章の 4 が別の単位になる。

## 9. 差し戻しが要りそうな所

設計が成り立たなくなる矛盾は、見つからなかった。上位の決定に、確かめか、決めの書き足しが要る所が 3 つ在る。

1. **フェーズ 1（今の SSP）で、曲・環境の音・効果音が同時に鳴るか。** 取り決め「命令の表」は「曲・環境の音・効果音・声 | 正典の `\![sound,loop／play／stop,名前]`」、骨子 2.9 はフェーズ 1 に「曲と効果音」を入れている。ukadoc は、同時に鳴ると言い切っておらず、「このコマンドで停止しないと、別の音声データを再生できない場合がある」とも書く（3 章の表）。SSP の実機で、ループ 2 本＋単発の重ねを測る事が要る。鳴らない場合は、フェーズ 1 の中身（環境の音・効果音の重ね）が変わる。areka の設計は変わらない。
2. **フェードの置き場。** 骨子 2.8「正典に在る物は、正典のタグで出す。`play` の命令は、正典に無い物だけにする」は、「正典のタグに、正典に無いオプションを足す」形を言っていない。フェーズ 2「曲のフェード（SSP が対応すれば）」は、SSP が同じオプションを足す形と読める。取り決めに 1 行、決めが要る（8 章の 1）。
3. **出し直しで止まる「単発」に入る物。** 取り決め「出し直しの印」は「頭で、鳴っている途中の単発の音と声を止め」。areka が見分けられるのはタグの形だけなので、`play` で鳴らした、ループさせない曲（終わりの曲・ジングル）も止まる。設定の画面から `＞演劇再開` で戻ると（骨子 4 章 C「出し直しの印付きで、もう一度出す」）、その行で鳴らした声と、流れている終わりの曲が切れて、鳴り直さない。これでよいか、止めない印が要るかの決めが要る。「声」と「効果音」は、タグの形では見分けられない（どちらも `play`）。今の決定は両方を止めるので、見分けは要らない。

## 10. 規模の見立て

- 暫定の設計での数: **22〜28 タスク**。
  - 音の crate の骨組みと命令の読み 2・要素の表 2・土台（読む・混ぜる・長いファイル・スレッドと知らせ）5〜6・パーサーの 3 綴り 1・dola の区切りと解き 2・時刻の付け替え 0〜2（`talk-fast-forward` が先なら 0）・compile と drive と dispatcher 2・受け手と登記と起動 2・パスと持ち主 1・オプションと先読み 2・切り替えと終了 1・プロパティ GET と SET 3・失敗の道 1・早送りと `StopOneShots` 1〜2・イベント 0〜3・台帳と互換の文書 1・x64 と arm64 の実機 1。
- brief の見立ては M〜L（14〜20）。差は +8 ほど。増えた分: 途中の区切り（`\_V`）のための dola と台本のスレッドの道・長いファイルを少しずつ読む道・イベント・早送りと出し直しのための口。
- brief の切り方（タグの受け口／プロパティ）で切ると、受け口 17〜21・プロパティとイベント 5〜7。受け口の側は、時刻の付け替えを `talk-fast-forward` に任せられれば 20 に収まる。

## 11. 確かめた物と机の上だけの物

- **実行して確かめた物**: 0。
- **コードを読んで確かめた物**: 3 章の全部（音の層が無い事・タグの今の道・運び手と受け手の決まり・区切りが解けた後に時刻を付け替えない事・トークの枠が 1 つで受け手が複製される事・台帳の行）。
- **ukadoc を引いて確かめた物**: 3 章の「正典」の引用と表。フェードのオプションが無い事（オプションの注記の全文を読んだ）。`loop` の形にオプションの欄が無い事。
- **読んでいない見立て（机の上）**:
  - Media Foundation と XAudio2 の動き（ループの継ぎ目・出だしの遅れ・arm64・既定の装置が替わった時・`--rate` の上限）。
  - `windows` クレートの features で両方が呼べる事（A13）。
  - sylphya に、問い合わせの時に値を引く口が在るか（窓の矩形の前例が在る、という覚えだけ。コードは開いていない）。
  - ゴーストの切り替えと終了の所（`session/ghost_switch.rs` ほか）に `StopAllOf` を足せる形か。
  - `\8` の今の道（字句解析の表を全部は見ていない）。
  - SSP の実機の動きの全部（同時再生・同名の鳴らし直し・`\_V` の相手・中断の時の音・知らないオプションの読み捨て）。
  - pasta の側が柱の値をファイルのパスへ直せるか。
