# ギャップ分析: balloon-script-offset-align

> 2026-11-27・kiro-validate-gap。基準は main `af6b9001` の上の worktree（`f3a09add`）。行番号はこの時点のもの。
> 議題の番号は requirements.md の「未決の議題」の番号。brief の `crates/areka/src/placement/…` は古い。今は `crates/areka-placement/src/…`（brief の「見つけた穴」⑴）。

## 1. 分析の要約

- **受け口の型は揃っている**: `\![set,alignmenttodesktop]`・`\![lock,balloonmove]` と同じ道（受け手の表 `CANONICAL_ROWS` → `MoveCueSink` → `PositionCue` → フレームの move drain → 置き方の側の関数 1 つ）に 2 つの命令を足すだけで、要件 1 の受け付けと `check_script` の答え（要件 1.3）は済む。向きの 6 語は、今の P5 の 2 つの値（`BalloonSide` と `BalloonXMode`）の組み合わせでどれも表せる＝新しい幾何は要らない。
- **値を重ねる場所の候補は 1 つに絞れる**: シェルの切り替え・面の `windowposition`（バルーンの切り替えもここを通る）・バルーン位置初期化の 3 つの道が、どれも `reseed::resolve_standard_scopes`（`reseed.rs:170`）で標準の位置を解き直している。ここで descript の値に台本の値を重ねれば、要件 4・5.3〜5.5 の大半が 1 か所で済む。
- **足りない仕組みが 1 つある＝「台本の基準点＋利用者のずらし」の分け方**: 今の追従の値（`BalloonFollow.offset`）と記憶の `BalloonOffset` は「シェルからの相対の位置そのもの」1 つで、descript のずらしと利用者のドラッグを分けて持たない。ukadoc の「この値にユーザーがバルーンドラッグ移動で決定した座標が加算され」（要件 2.3）と「台本の値を記憶へ保存しない」（要件 5.6）を両立させるには、重ね方の決めが要る（要件の議題 5）。`persist.rs` に触らずに済む形はある（4.3 節）。
- **押し戻しの画面は 3 か所が同じ書き方**（キャラ窓の矩形を `work_area_for_window` へ渡す）なので変える手間は小さいが、**既定の振る舞いが変わる**ことと、**DPI の違う画面の境で窓の大きさが変わって選ぶ画面が揺れる**恐れがある（議題 1・調べ 6.4）。SSP 2.9.34 はそもそもバルーンを押し戻さない（既存の測り）。
- 規模は **M（8〜12 タスク）**、危うさは **中**（受け口と値の層は既存の型どおり・重ね方と押し戻しの画面に新しい決めがある）。

## 2. 今の作りの調べ

### 2.1 受け口（runtime の crate）

| 所 | 事実 |
|---|---|
| `crates/areka-runtime/src/consumer_ledger.rs:321-380` | 表 `CANONICAL_ROWS` が正本。`set` は選別子つきで多くの担当と分け合う（`zorder`・`choicetimeout`・`balloontimeout`・`windowstate`・`alignmenttodesktop`・`sticky-window` …）。`("set", Some("balloonoffset"))`・`("set", Some("balloonalign"))` はまだ無い＝今は担当なし（`consumer_of` が `None`・`check_script` は `unknown_command`）。 |
| `crates/areka-runtime/src/consumer_ledger_test_support.rs:102-103` | 見本の表（`samples`）。表の行と一致することを `samples_cover_exactly_the_ledger_rows` が判定する＝2 行の見本を足す所。 |
| `crates/areka-runtime/src/move_cue.rs:262-300` | `MoveCueSink::emit` が `(名前, 第 1 引数)` で自己選別する。吸着（`280-286`）と lock（`290-298`）は `actor_scope`（`329-340`）で実行者のスコープを読み、数でなければ `move_cue_bad_actor` の `warn!` で捨てる（要件 1.9 の前半はこの型のまま）。余分な引数は読まない（lock の先例・要件 1.6）。 |
| `crates/areka-runtime/src/move_cue.rs:37-55` | 線を流れる `PositionCue`。変種を足すと `frame/move_hold.rs` と `frame/drain_resnap.rs` の網羅 `match` が要る。 |
| `crates/areka-runtime/src/frame/move_hold.rs:117-162` | `touched_scopes`。吸着と lock は `Some(vec![scope])`（`133-135`）。初期化（`ResetBalloonPos`）は預かりの中の吸着・lock とだけ順を保つ（`150-158`）＝新しい 2 つの変種もこの並びに入れないと、`resetballoonpos` と台本の値の前後が入れ替わりうる。 |
| `crates/areka-runtime/src/frame/drain_resnap.rs:213-243` | 届いた指令を種類ごとに placement の関数へ渡す。窓の一式（`GhostWindows`）が無い間は線が保留を兼ねる（`159-161`）＝起動の台本が窓より先に来ても取りこぼさない。 |
| `crates/areka-runtime/src/move_cue_args.rs` | `\![move]` の引数の読み（純粋）。符号つきの整数の読みはここに先例がある。 |

### 2.2 置き方の値（placement の crate）

| 所 | 事実 |
|---|---|
| `crates/areka-placement/src/config.rs:57-80` | `ScopeConfig` に `balloon_alignment`（`Left`／`Right`／`Auto`＝`none`）・`balloon_offset`（descript の `balloon.offsetx/offsety` の生値。後で `windowposition` の調整量が足される加算の欄）・`balloon_offset_descript_x`（右に出すときの符号の反転に使う descript の x の分）・`balloon_x_mode`（`Side`／`CenterTop`／`CenterBottom`）。y の descript の分は別に持たない。 |
| `crates/areka-placement/src/lib.rs:480-506, 530-539` | `scale_then_merge`＝descript のずらしをシェル軸の k で物理 px へ換算（`apply_author_balloon_offset_scale`）→ バルーンの `windowposition` を合流（`merge_scope_windowpositions`）。合流は `balloon_x_mode` を `windowposition.x` の語で**上書きする**（`lib.rs:625`）。 |
| `crates/areka-placement/src/resolver.rs:337-385` | P5。`balloon_x_mode` が `CenterTop`／`CenterBottom` なら `keyword_balloon_pos`（`183-206`）、`Side` なら `balloon_alignment` で左右（`Auto` は作業領域の中央で 1 度だけ決める・`363-369`）。右に出すときは descript の x の分だけ符号を反転（`372-376`）。 |
| `crates/areka-placement/src/reseed.rs:170-261` | `resolve_standard_scopes`＝`build_placement_config` → スコープごとに窓の DPI で `scale_then_merge` → `resolve_placement`（記憶は重ねない）。呼び手は 3 つ: シェルの切り替え（`apply_shell_descript`・`reseed.rs:60-88`。この後で記憶を重ねる `apply_restored_placements`）・面の `windowposition`（`face_windowposition.rs:76`。バルーンの切り替えも面の書き手を通る＝`frame/switch.rs:618-630`）・バルーン位置初期化（`reset_balloon_pos.rs:61`）。 |
| `crates/areka-placement/src/reseed.rs:99-129` | `reinstall_balloon`＝`BalloonFollow` を標準の基準で入れ直し、キーワードの素材・`BalloonLimit` を入れ直し、利用者が動かした印（`BalloonUserMoved`）を基準の形で付け外しし、`follow_balloon` で置き直す。 |
| `crates/areka-placement/src/follow/offset_space.rs:79-140` | `BalloonFollow`＝キャラ窓の左上からバルーンの左上への**1 つの**相対の値と基準対（値＋表示 DPI）。書き手は 4 本だけ（`new`・`reestablish`・`anchor_base_dpi`・`apply_rescaled`）。拡大率の変化は基準から引き直す（`frame/balloon_offset_follow.rs`）＝基準を台本の値で据え直せば要件 4.3 は今の仕組みで効く。 |
| `crates/areka-placement/src/follow/balloon_drag.rs:74-133, 195-306` | ドラッグ中は `balloon_pos − char_pos` で基準を焼き直し（`reestablish`）、離したときに同じ式の値を `balloon_offset_entries`→`persist_entries` で記憶へ書く（`271-291`）。記憶の `BalloonOffset` を書くのはここだけ（ほかに書き手は無い）。 |
| `crates/areka-placement/src/persist.rs:545-570` | 起動とシェルの切り替えの記憶の重ね（`merge_scope`）: 記憶のずらしがあれば標準を**置き換える**（未係留の基準・キーワードの素材を落とす）。＝今は「利用者のずらし」は加算でなく置き換え。 |
| `crates/areka-placement/src/face_windowposition.rs:50-102` | 面の `windowposition` が変わると、`BalloonUserMoved` の無いスコープだけ標準へ置き直す。 |
| `crates/areka-placement/src/reset_balloon_pos.rs:44-104` | 初期化は `resolve_standard_scopes` → `reinstall_balloon` → 記憶の鍵を消す。キャラ窓・吸着の向き（`AnchorSource`）には触らない。 |
| `crates/areka-placement/src/script_alignment.rs:55-164` | 台本の吸着の向き。値は**キャラ窓の Component**（`Anchored`・`AnchorSource.by_script`）に持つ。シェルの切り替えは `by_script` の窓の向きを保つ（`reseed.rs:133-156`）。ゴーストの切り替え・読み直しは窓ごと作り直すので消える。＝寿命の先例そのもの（§8 の 151 行目）。 |
| `crates/areka-placement/src/balloon_lock.rs:49-98` | 台本の lock は Resource の帳簿（`BalloonLockLedger`）。起動で据え直し（`areka-session/src/ghost_session.rs:273`）、シェルの切り替えは descript の段だけ取り替える（`frame/switch.rs:562`）。lock が止めるのはバルーンの窓の `DragConfig.move_window` だけで、置き方の計算は読まない。 |

### 2.3 押し戻しの関門（3 か所＋判定し直し）

| 関門 | 所 | 見る画面 |
|---|---|---|
| 起動 | `balloon_limit.rs:181-220`（`apply_balloon_limit_one`） | `rect_at(p.char_pos, p.char_size)` を `work_area_for_window`（`189-190`） |
| 書き込み | `follow/window_move.rs:937-1048`（`apply_balloon_limit_gate`） | キャラ窓の `WindowPos` の矩形（`1001-1017`）。doc（`902-909`）は「バルーン窓自身の帰属で引かないのは、はみ出したバルーンが隣モニタへ帰属して基準が振動するのを避けるため」と書く。 |
| ドラッグを離したとき | `follow/balloon_drag.rs:437-455`（`apply_release_limit_correction`） | キャラ窓の矩形 |
| 判定し直し | `follow/other_chars.rs:31`（`rejudge_all_balloons`） | 書き込みの関門を通る（`follow_balloon`） |

- 3 か所とも「キャラ窓の矩形 → `work_area_for_window`（窓の中心の画面・外なら最も近い画面）→ `limit_correction`」の同じ形。式は `limit_correction`（`balloon_limit.rs:123-126`）1 本。
- 記録は 3 か所とも `[balloon-limit] Clamp`（`balloon_limit.rs:207-216`・`window_move.rs:1035-1046`・`balloon_drag.rs:474-483`）。要件 7.3 の 2 つの欄（見た画面・台本の値が効いていたか）はこの 3 行に足す。
- `doc/COMPAT_ARCHITECTURE.md` 451 行目（§8）: **SSP 2.9.34 は押し込まない**（2026-11-16・11-20 の測り）。areka が押し込むのは ukadoc の「画面内に維持」を取るためで、「areka はキャラクター窓のモニターの作業領域で収める（完了 spec `areka-P0-windowposition-limit` の決めのまま）」。
- 既存の SSP の測り（`.kiro/specs/completed/balloon-flip-ssp-review/research.md` 8.3〜8.4 節）が `\![set,balloonoffset,@-250,@0]`・`@-310,@0`・`@0,@0` を打っている: バルーンは x が −250 だけ動き（左に出るバルーン）、`@0,@0` で元へ戻り、`\![move]` で窓を動かしてもずらしは保たれ、画面の外へ全部出ても戻されなかった。＝`@` は「descript の値からの相対」で、累積ではない。

### 2.4 寿命の道

| 場面 | 今の作り | 台本の値の Component をキャラ窓に持つと |
|---|---|---|
| ゴーストの切り替え・読み直し | 窓の一式を作り直す（`ghost_session.rs:259-273`）。帳簿は据え直し | 窓と一緒に消える（要件 5.2 は自然に満ちる。5.7 の記録「捨てたスコープ」は消える前に残す所が要る） |
| SHIORI だけの読み直し | 窓は残る | 保つ（要件 5.7） |
| シェルの切り替え | 窓は残り `apply_shell_descript` が標準を解き直す | 解き直しの中で重ねれば保つ（要件 5.3） |
| バルーンの切り替え | 面の書き手が `resolve_standard_scopes` で解き直す（`switch.rs:618-630`） | 同上（要件 5.4） |
| `\![execute,resetballoonpos]`・メニュー | `resolve_standard_scopes` → `reinstall_balloon` → 記憶の鍵を消す | 重ねれば台本の値で決まる位置へ戻る（要件 5.5 の仮定）。台本の値も消すなら 1 行（議題 3） |

## 3. 要件と今の作りの対応

| 要件 | 今ある物 | 足りない物 | 札 |
|---|---|---|---|
| 1.1〜1.3 受け付け | 表・`MoveCueSink`・`PositionCue` の型 | 表の 2 行・見本 2 行・`PositionCue` の 2 変種・drain と預かりの腕 | Missing（小） |
| 1.4〜1.8 引数の読み | `parse_alignment_word`（大小を区別しない・空は欠け）・`move_cue_args.rs` の数の読み | `@` つきの符号つき整数の読み（軸ごと）・`@` だけ・空・数でない字の扱い | Missing／議題 4 |
| 1.9 スコープ | `actor_scope`・`windows.char_window(scope)` の不在の `warn!`（`script_alignment.rs:74-82`） | 同じ型をなぞる | Missing（小） |
| 2.1〜2.2 基準点 | P5 の式・`scale_then_merge` | 重ねる層（descript の生値を台本の値で置き換え／足す）と、それを解き直しに渡す口 | Missing |
| 2.3 利用者のずらしの加算 | 無い（追従の値は 1 つ・記憶は置き換え） | 重ね方の決め | **Constraint／議題 5** |
| 2.4 座標の決まり | シェル軸の換算・右の符号反転（`resolver.rs:372-376`） | 台本の値を descript の値と同じ欄で通せば足りる。右の符号が SSP でも同じかは未測 | Unknown（調べ 6.1） |
| 2.5 置き換える範囲 | `windowposition` の調整量は合流で後から足される・面の `move` は毎フレームの相（`follow/move_shift.rs`） | 台本の値を「換算の前の descript の生値」だけに当てれば、残りは今のまま足される | 決め（議題 6）・作りは小 |
| 2.6 出ている間の即時反映 | `reinstall_balloon` が `follow_balloon` で置き直す | 指令の当て口 | Missing（小） |
| 2.7 出る前の値 | 窓が無い間は線が保留（`drain_resnap.rs:159-161`）・窓があれば非表示でも `BalloonFollow` へ書ける | バルーンの大きさが未確定の窓（`reseed.rs:342-346` が `balloon_size_unknown` で外す）の扱い | Unknown（調べ 6.3） |
| 3.1〜3.6 向き | 6 語は `(BalloonSide, BalloonXMode)` の組で表せる: `left`→`Side+Left`・`right`→`Side+Right`・`center`/`top`→`CenterTop`・`bottom`→`CenterBottom`・`none`/他→`Side+Auto` | 台本の語の読みと、合流（`lib.rs:625`）の**後で**向きを上書きする口 | Missing（小） |
| 3.7 側を入れ替えない | 今の areka は初めの配置の後にバルーンの側を入れ替えない（`resolver.rs:361-362` の注・実行時の並べ替えは `balloon-flip-ssp-review` で取り除いた＝§8 451 行目） | 無し（今のまま満ちる） | — |
| 3.8〜3.9 向きを変えたとき | 同上 | 利用者のずらしの扱い（議題 5） | 議題 5 |
| 4.1〜4.3 どの道でも同じ値 | 追従・ドラッグ・拡大率の追随はどれも `BalloonFollow` を読む＝据え直せば揃う。解き直しの 3 つの道は `resolve_standard_scopes` に集まる | 重ねる口を `resolve_standard_scopes` に 1 つ | Missing（小） |
| 4.4〜4.5 値の無いスコープ・他のスコープ | スコープごとの `ScopeConfig`・窓ごとの Component | 無し | — |
| 5.1〜5.4, 5.7 寿命 | `script_alignment` の先例（窓の Component） | Component を 1 つ | Missing（小） |
| 5.5 初期化 | `reset_balloon_positions` | 重ねる口を通れば今のまま。消すなら 1 行 | 議題 3 |
| 5.6 保存しない | ドラッグを離したときだけ記憶を書く | 台本の分が記憶に入らない形（4.3 節） | **Constraint**（`persist.rs` に触らない） |
| 6.1〜6.5 押し戻しの画面 | 3 か所同じ書き方 | 画面の選び方を 1 つの関数にまとめて 3 か所から呼ぶ | 議題 1・調べ 6.4 |
| 6.6 DPI の読み手 | `ghost_session_scope_scale.rs:79-108`（窓ごとの表） | 無し（形を作れるようにするだけ） | — |
| 7 記録 | 各関数の `info!`／`warn!` の型・`[balloon-limit] Clamp` | 欄 2 つと新しい事象語 | Missing（小） |
| 8.1 テスト | 檻の支え（`follow_test_support.rs` ほか）は読むだけなら使える | 新しい兄弟のファイル | Constraint（約束の 6 本は変えない） |
| 8.2 台帳 | `doc/ukadoc-coverage/ledger/sakura-script.toml:3044・3088`（`absent`・`A2`） | 2 行を書き換え | Missing（小） |
| 8.3 §8 | 151・152・423・451 行目が先例 | 別々の行と 451 行目の改め | Missing（小） |
| 9 SSP と実機 | 既存の SSP の測りの手順（上の 2.3 節） | 未測の場面（6 章） | Research |

## 4. 実装の道の候補

### 4.1 受け口（どの案でも共通）

- 表に `("set", Some("balloonoffset"), MoveSink)`・`("set", Some("balloonalign"), MoveSink)`、見本に 2 行。
- `MoveCueSink::emit` に 2 つの腕。`actor_scope` で実行者を読み、引数を読み（読みは `move_cue_args.rs` か placement の新しいファイル）、`PositionCue::SetBalloonOffset { scope, .. }`・`SetBalloonAlign { scope, word }` を送る。
- `move_hold.rs` の `touched_scopes` は吸着と同じ `Some(vec![scope])`、初期化の順の並び（`150-158`）にも入れる。
- `drain_resnap.rs` の腕から placement の当て口 1 つずつを呼ぶ。

### 4.2 値の層の置き方

**案 A: 解き直しの中で重ねる（既存を伸ばす）**
- 値はキャラ窓の新しい Component（例: 台本の基準点と向き。`script_alignment` と同じく窓と一緒に生き死にする）に持ち、新しいファイル（`crates/areka-placement/src/` に 1 本＋`lib.rs` の `mod`）に型と当て口を置く。
- `resolve_standard_scopes`（`reseed.rs:206-220`）のスコープごとの段で、`scale_then_merge` の**前**に descript の生値を台本の基準点で置き換え（絶対）または足し（`@`）、`balloon_offset_descript_x` も同じ値にする（右の符号の決まりが同じになる）。`scale_then_merge` の**後**に `balloon_alignment`／`balloon_x_mode` を台本の向きで上書きする（合流が `balloon_x_mode` を上書きするため・`lib.rs:625`）。
- 指令の当て口は `reset_balloon_pos` と同じ形（`resolve_standard_scopes` → そのスコープだけ `reinstall_balloon`）。記憶の鍵は消さない。
- ✅ シェル・バルーンの切り替え・面の `windowposition`・初期化の 4 つの道が 1 か所で揃う。換算・右の符号・キーワードの再導出・拡大率の追随は今の仕組みのまま効く。
- ❌ `reseed.rs` は roadmap の ④ の触るファイルの列に無い（brief の「触るファイル」には条件つきで載る）＝同じウェーブの重なりを確かめる（`sylphya-sqlite-kvs` の列にも無い）。
- ❌ そのままでは利用者のずらしを捨てる（`reinstall_balloon` は標準で据え直す）。記憶のずらしがあるスコープはシェルの切り替えで記憶が標準を置き換える（`persist.rs:556-564`）ので台本の値が消える。＝4.3 節の重ね方が要る。

**案 B: 追従の値にだけ差を足す（新しい層だけ・解き直しに触らない）**
- 指令のとき「台本の値で解いた標準 − 台本の値の無い標準」の差を `BalloonFollow` に足して据え直す。
- ✅ `reseed.rs` に触らない。利用者のずらしを保つ（要件 2.3 の仮定どおり）。
- ❌ シェル・バルーンの切り替え・面の `windowposition`・初期化の後に差が消える（どれも標準で据え直す）＝要件 5.3〜5.5 を満たすには結局その 3 つの道に差を足す所が要り、口が 4 か所に散る。差は向きを変えたときに窓の大きさに依るので、大きさが変わると古くなる。

**案 C（組み合わせ）: 案 A の重ね＋利用者のずらしは差で扱う**
- 標準は案 A で台本の値を重ねて解く。利用者が動かしたスコープ（`BalloonUserMoved`）だけ、差（台本の値で解いた標準 − 無しで解いた標準）を記憶の値に足して据え、ドラッグを離したときは記憶へ「今の相対の位置 − 差」を書く（`balloon_drag.rs:277` の 1 か所・`persist.rs` は呼ぶだけ）。シェルの切り替えは `apply_restored_placements` の**後**で差を足す（`reseed.rs:73-84`）。
- ✅ ukadoc の「台本の値＋利用者のずらし」と要件 5.6 を両立し、`persist.rs` を変えない。
- ❌ 解き直しを 2 回（台本あり・無し）する。差の計算と、どの道で差を足すかの不変条件を新しく持つ＝テストの組み合わせが増える。

### 4.3 利用者のずらしと記憶（議題 5 の材料）

今の記憶は「シェルからの相対の位置そのもの」（`balloon_drag.rs:271-291`）で、起動とシェルの切り替えで標準を置き換える（`persist.rs:556-564`）。台本の値が効いている間のドラッグをそのまま書くと、台本の分が記憶に入り、次の起動（台本の値は無い）で台本の位置へ出る＝要件 5.6 の趣旨に反する。`persist.rs` に触らずに取れる形は 3 つ:

| 形 | 書く所 | 振る舞い |
|---|---|---|
| ⑴ 差を引いて書く（案 C） | `balloon_drag.rs` の書く値 | 記憶は「descript の既定＋利用者のずらし」のまま。次の起動は台本の値無しの位置＋利用者のずらし |
| ⑵ 台本の値が効いている間のドラッグは書かない | `balloon_drag.rs` の書く前の分かれ道 | 記憶は前の値のまま。そのドラッグは次の起動へ持ち越さない（記録は残す） |
| ⑶ そのまま書く | 変えない | 台本の分が記憶に入る＝要件 5.6 に反する（比べのために挙げる） |

指令の前のドラッグを足し続けるか（要件 2.3 の仮定）は、⑴なら差の足し方で足し続けられ、⑵の形なら指令で標準へ据え直して捨てる（`resetballoonpos` と同じ据え直し・記憶は消さない）のが素直。

### 4.4 押し戻しの画面（議題 1 の材料）

| 案 | 作り | 振る舞いの変わり |
|---|---|---|
| a. どのスコープもバルーン自身の画面 | 3 か所の「キャラ窓の矩形」を「置こうとするバルーンの矩形」に替え、選び方を `balloon_limit.rs` の関数 1 つにまとめる（`window_move.rs` は呼ぶ行の差し替えだけで行は増えない） | 台本の値の無いバルーンも変わる。2026-11-01 の ssl1 の形（本体が境にまたがり、バルーンの矩形 `-1355..-483` の中心が左の画面）では、押し戻されずに左の画面に残る＝**台本を使わずに本体とバルーンが分かれる**。SSP は押し戻さないので SSP に近づく向き |
| b. 台本の値が効くスコープだけバルーン自身の画面 | a と同じ関数に「Component の有無」を渡す（関門はどれもキャラ窓を引けている） | 既定は変わらない。分かれた形は台本でだけ作れる。決まりが 2 つになる |
| c. 今のまま（本体の画面） | 欄を足すだけ | 段 4 は作れない（要件 9.4） |

- a・b どちらも、矩形が 1 つの画面に収まっていても 2 つの画面にまたがれば中心の画面へ押し戻す（要件 6.2 の「どれか 1 つの作業領域に収まっている間」はまたぐ形を含まない）。
- `window_move.rs:907-909` の「振動」の注: 押し込んだ結果は同じ画面に収まるので、同じ大きさのまま当て直しても選ぶ画面は変わらない（`limit_correction` の冪等）。揺れが起きうるのは、**画面をまたいだときにバルーンの窓の大きさが変わる**（DPI の違う画面へ移ると `OnBalloonScaling` で窓の大きさが変わる＝`shell-scale-linkage`）ときで、中心が境の向こうへ戻ると選ぶ画面が行き来しうる。決定論のテストでは窓の大きさが変わらないので見えない＝実機で確かめる（調べ 6.4）。

## 5. 規模と危うさ

- **規模: M（8〜12 タスク）**。受け口（表・線・預かり・drain）2〜3、値の層と当て口 2〜3、押し戻しの画面と記録 1〜2、台帳・§8 1、SSP の測り 1、実機の段 4 と知らせ 1。案 C を採ると重ね方のテストで 1〜2 増える。
- **危うさ: 中**。受け口と寿命は既存の型どおり（低）。利用者のずらしとの重ね方と、押し戻しの画面の変更（既定の振る舞い・DPI の境での揺れ）に新しい決めがある（中）。外の依存は無い。

## 6. 調べること（設計の段へ持ち越す）

1. **SSP: 右に出すバルーンの台本の x の符号**（descript の `balloon.offsetx` は右に出すとき左向きが＋＝§8 451 行目の「残した決まり」）。台本の基準点も同じかを測る。`center`・`bottom` のときの x の向きも。
2. **SSP: 利用者のドラッグと台本の値の重ね方**: 指令の前にドラッグした分が残るか、指令の後のドラッグが次の起動に持ち越されるか（台本の分が記憶に入るか）、`\![set,balloonalign]` で向きを変えたときにドラッグの分がどうなるか。
3. **SSP: 寿命**: シェルの切り替え・バルーンの切り替え・`\![execute,resetballoonpos]`・メニューの初期化の後に台本の値が残るか。`\![set,balloonoffset]` の寿命（ukadoc は沈黙）。
4. **実機: DPI の違う画面の境での揺れ**（4.4 節）と、バルーンの窓が別の DPI の画面へ移ったときの `WM_DPICHANGED` の推奨の矩形が位置をずらさないか。段 4 の確かめで一緒に見る。
5. **バルーンの大きさが決まる前の指令**（要件 2.7）: 起動の台本の最初で指令が来たとき、`resolve_standard_scopes` の `running_scope` が `balloon_size_unknown` で外す場面があるか。あれば預かり（`move_hold.rs`）で待つか、大きさが決まったときに当て直すか。
6. **同じフレームの見え方**: 指令と同じ台本の最初の台詞でバルーンが出るとき、move drain が表示の指令の後に当たる（`drain_resnap.rs:141-142` の並び）ので、1 フレームだけ古い位置に出ないか。出るなら状態の持ち方で 0 フレームにする（フレームを遅らせる解は取らない）。
7. **同じウェーブの重なり**: `reseed.rs`・`balloon_drag.rs`・`reset_balloon_pos.rs`・`config.rs`・`lib.rs`（placement）・`ghost_session.rs`（Resource の帳簿を選ぶとき）が、C28 の他の 5 本の触るファイルに入らないか（roadmap の表の ② は placement では `persist.rs` だけ）。

## 7. 設計の段への勧め

- 受け口は 4.1 節の形（`script_alignment`・`balloon_lock` の型）。値はキャラ窓の Component（寿命が §8 151 行目の先例と同じになり、ゴーストの切り替えでの据え直しが要らない）を第一の候補にする。
- 値の層は案 A を土台に、利用者のずらしの扱い（議題 5）しだいで案 C の差を足す。どちらでも `persist.rs` と約束の 6 本の試験の支えに触らずに済む（新しい型を `ScopePlacement` に足さない＝`follow_test_support.rs:383`・`frame_persist_test_support.rs:200, 314`・`move_cue_apply_move_tests.rs:33`・`persist_finalize_save_tests.rs:125` が欄を全部書いて組んでいる）。
- 押し戻しの画面の選び方は `balloon_limit.rs` の関数 1 つにまとめ、3 か所の関門と記録の 2 つの欄をそこから出す。

## 8. SSP の測り（2026-11-27・要件の討議の議題 7）

SSP の MCP（`sakurascript` strict・`get_property`）で、起動中の「窓際のぱすたさんDEBUG」の `\0` を測った。本体の窓 `currentghost.scope(0).rect` = `-2581,833,-1750,1859`（幅 831）・`currentmonitor.dpi` = 144（倍率 1.5）・バルーンの幅 654・高さ 675。測る前のバルーン `currentghost.balloon.scope(0).rect` = `-2521,920,-1867,1595`、最後に `\![set,balloonoffset,@0,@0]\![set,balloonalign,none]` で同じ値へ戻した。strict の誤りの記録は 0 件。reload・シェルやバルーンの切り替え・ゴーストの切り替えはしていない。

| 台本（順に打った・値は前の台本から残る） | バルーンの矩形 | 読み |
|---|---|---|
| `@0,@0` ＋ `left` | `-2521,920,-1867,1595` | 測る前と同じ（この検体の既定の側は左） |
| `right` | `-2464,920,-1810,1595` | 左端 − 本体の右端 = −714＝descript の x（476 の 1.5 倍）が本体の側（左）向き |
| `100,0`（絶対）・right のまま | `-1900,833,-1246,1508` | 左端 − 本体の右端 = −150＝台本の x 100 を 1.5 倍し、右側では本体の側（左）向き。y 0 で上端が本体の上端 |
| `left` | `-3085,833,-2431,1508` | 右端 − 本体の左端 = +150＝左側では本体の側（右）向き |
| `center` | `-2343,195,-1689,870` | 中心 − 本体の中心 = +149.5（右向き）。下端 870 は本体の窓の上端 833 から 37 下 |
| `bottom` | `-2343,1120,-1689,1795` | x は center と同じ。上端 1120 は本体の窓の下端 1859 と接しない |
| `left` ＋ `\![execute,resetballoonpos]` | `-3085,833,-2431,1508` | 初期化の後も台本の値（left・100,0）は残る |
| `@0,100` | `-2521,983,-1867,1658` | 軸ごとに混ぜられる（x は descript、y は本体の上端 +150） |
| `@,abc` | `-2521,833,-1867,1508` | `@` だけは `@0`、`abc` は 0 として黙って当てる（誤りの記録なし） |

- **x の符号**（要件 2.4）: 台本の基準点の x は descript のずらしと同じ決まり＝シェルの倍率で換算し、左右どちらでも正の値が本体の側を向く（右側で符号を反転）。`center`・`bottom` では反転しない（正が右向き）。＝要件 2.4 の仮定どおり。
- **寿命**: 別々の台本をまたいで値が残る。`\![execute,resetballoonpos]` は台本の値を消さない（要件 5.5 どおり。ただしこの測りには利用者のドラッグの分が無い）。
- **`center`・`bottom` の y**: 本体の窓の上下の端ではなく、別の基準（面の見える範囲か当たりの矩形と見られる）に接する。areka の今の `keyword_balloon_pos` の基準との突き合わせは設計の段で行う。
- **壊れた値**: SSP は数でない字を 0 として黙って当てる。要件 1.7 は「当てずに警告」＝SSP と違う。討議で決め直し、「当てずに警告」のままとした（§8 に行を足す）。
- **測っていないもの**: 利用者のドラッグとの重ね方（マウスの操作が要る）・シェルとバルーンの切り替えの後の寿命（検体のゴーストのシェルとバルーンを変えることになる）。設計の段で、画面の操作の机を取ってから測るかを決める。

### 8.1 実装の段の残りの測り（2026-11-27・タスク 1・開発者の許しを得て排他テストの机を持って測った）

測る前に本体の窓を `\![move,--X=0,--Y=-600,--time=0,--base=me]` で上へ動かした（元の位置ではバルーンが画面の下端で押し戻され、向きを変えても矩形が変わらなかった）。本体 `-2404,629,-1924,1259`。測り終えてから本体の位置・バルーン・台本の値（`@0,@0`・`none`）を元へ戻した。シェルはこの検体に 1 つしか無いので、シェルの切り替えは `\![reload,shell]` で代えた。

| 台本（順に打った） | バルーンの矩形 | 読み |
|---|---|---|
| `@0,@0` ＋ `right`（元のバルーン） | `-2638,716,-1984,1391` | 左端 − 本体の右端 = −714（descript の 476×1.5 が本体の側向き）＝ 8 節と同じ |
| `100,0` ＋ `right`（元のバルーン） | `-2074,629,-1420,1304` | 左端 − 本体の右端 = −150 ＝ 8 節と同じ |
| `\![change,balloon,kakukaku for emo-gs]` の後に台詞だけ | `-1675,436,-1075,772` | 次の行と同じ |
| 同じバルーンで `100,0` ＋ `right` を打ち直す | `-1675,436,-1075,772` | 切り替えの前の台本の値がそのまま効いていた＝**バルーンの切り替えで台本の値は残る**（要件 5.4 どおり） |
| 同じバルーンで `@0,@0` ＋ `right` | `-2239,523,-1639,859` | 比べのため。このバルーンでは `100,0` との差が x +564・y −87 で、元のバルーンの式（台本の x×1.5 を本体の側向き）と合わない。要因は特定していない（バルーンの側の値と見られる） |
| （値は `100,0`・`right`）`\![reload,shell]` の後に台詞だけ（2 回とも同じ） | `-2452,436,-1852,772` | y は台本の値のまま。x だけが変わる |
| 続けて `\![set,balloonalign,right]` だけを打つ | `-1673,436,-1073,772` | 読み直しの前の位置へ戻る＝**シェルの読み直しで台本の向きだけが落ち、台本のずらしは残る** |
| 続けて `left`・`none` | どちらも `-1673,436,-1073,772` | このバルーンでは左に置くと画面の左端を越えるため、右へ置き直されたと見られる。明示の `none` は読み直しの直後の状態とも違う |

- **バルーンの切り替え**: 台本の値は残る＝要件 5.4 と一致。
- **シェルの切り替え（読み直し）**: SSP は台本のずらしを保ち、台本の向きを落とす。要件 5.3（両方を保つ）と**食い違う**。ukadoc の `\![set,balloonalign]` は「ゴースト終了まで有効」と書くので、SSP の振る舞いは正典に合わない実装の癖と見られる。開発者と決め直す（要件 9.4）。
- **利用者のドラッグとの重ね方（測れなかった）**: SendInput（`mouse_event` の相対と絶対の 2 通り）でバルーンの窓（`Tmessageform`・指す点の窓の型と pid を確かめてから押した）をドラッグしたが、SSP のバルーンは動かなかった（物理の座標の矩形・プロパティの矩形とも変わらず）。本体の窓が重なる所を避けた点からでも同じ。areka の決め（要件 2.3・3.9・5.8＝利用者のずらしを台本の値の上に重ねる）は測りで裏付けていない。
- 座標の換算: SSP のプロパティの座標 × 4/3 ＝ Per-Monitor v2 の物理の座標（左の画面の物理の座標は `-3413,260`〜`0,2393`）。

## 9. 設計の段の調べと決め（2026-11-27・kiro-spec-design）

- **Discovery Scope**: Extension（既存の受け口・標準の位置の解き・押し戻しの関門を伸ばす）＝light discovery。外の依存は無く、Web の調べはしていない。読んだ規則: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、雛形 `.kiro/settings/templates/specs/design.md`・`research.md`、steering の `product.md`・`tech.md`・`structure.md`（1,000 行の上限と兄弟のテストの置き方）・`roadmap.md` の C28 の行。

### 9.1 調べで分かったこと（6 章の持ち越しへの答え）

| 持ち越し | 分かったこと | 設計への効き |
|---|---|---|
| 6.5 大きさが決まる前の指令 | `running_scope`（`reseed.rs`）はバルーンの `WindowPos.size` が無いと `balloon_size_unknown` で外す。バルーン窓は spawn で大きさを持つので、窓の一式がある間に外れるのは spawn の直後の仮の状態だけ。キャラ窓の大きさは最初の表示まで仮（`move_hold.rs` の `Landed` が実表示寸の着地を見る） | 2 つの変種は `\![move]` の動かす窓と同じく、キャラ窓の表示が決まるまで預かる。窓があって解けないときは値を持ったまま `script_balloon_place_deferred` |
| 6.6 同じ巡の見え方 | `frame.rs` の並びは 取り出し（表示の線）→ 差し替え → 可視性 → 窓寸の照合 → … → 再スナップ → move drain。可視性の相と move drain は同じ `frame_system` の中 | 同じ巡で位置を書く＝0 フレーム。可視と位置が画面で同じコマに載るかは段 4 で目で見る（design の Open Question 6） |
| 6.7 同じウェーブの重なり | `runtime_sink: None` を書く placement のファイルは `follow_test_support.rs`・`persist_finalize_save_tests.rs` だけ。`reseed.rs`・`balloon_drag.rs`・`lib.rs`・`consumer_ledger_test_support.rs` は C28 の他の 5 本の列に無い | roadmap ④ へ 5 ファイルを足し、`move_cue_args.rs`・`resolver.rs` を外す（承認は設計の討議） |
| `window_move.rs` の上限 | `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表 `OVER_LIMIT_ALLOWED` に載っている（2 件のうち 1 件） | 関門の作業領域を引く数行を 1 回の呼び出しへ替え、差し引きの行を増やさない |
| 値の持ち主 | `script_alignment.rs` はキャラ窓の Component（`AnchorSource.by_script`）。`balloon_lock.rs` は Resource の帳簿で、起動で据え直す結線が `areka-session/src/ghost_session.rs` にある。`spawn.rs` に `on_remove` の hook の先例 | キャラ窓の Component `ScriptBalloonPlacement`。捨てたスコープの記録は `on_remove` の hook |
| 起動の関門 | `apply_balloon_limit_one` は spawn の前のデータの変換で World を持たない。台本の値は保存せず、ゴーストの切り替え・読み直しで窓ごと消えるので、起動の時点では必ず無い | 起動の関門は同じ選び方の関数を「台本の値なし」で呼ぶ |
| `center`・`bottom` の y（8 節） | areka の `keyword_balloon_pos` はキャラ窓の上下の端に接する。SSP は別の基準（本体の窓の上端から 37 px 下・下端と接しない） | areka の今の基準を使い §8 に違いを記録（design の Open Question 3） |
| 記憶の形 | 記憶の `BalloonOffset` の書き手は `balloon_drag.rs` の 1 か所。`persist::merge_scope` は記憶があれば標準を置き換え、基準を未係留（`dpi: None`）にする | 書く値から台本の値の分を引く（`balloon_drag.rs`）・シェルの切り替えで未係留のスコープに分を足す（`reseed.rs`）。`persist.rs` は変えない |

### 9.2 Synthesis

- **一般化**: 2 つの命令はどちらも「descript の値を台本の値で置き換える 1 層」で、違いは重ねる段（ずらしは換算の前・向きは合流の後）だけ。当て口は私有の 1 本を共に通る。「台本の値の分」は記憶へ書くとき（5.8）・記憶から戻すとき（5.3）・利用者のずらしを新しい値へ載せるとき（2.3・3.9）の 3 か所で同じ式（台本ありの標準 − descript だけの標準）。
- **作るか使うか**: 新しい幾何・新しい換算・新しい帰属の決まりは作らない。P5・`keyword_balloon_pos`・`scale_then_merge`・`work_area_for_window`・`limit_correction`・`reinstall_balloon`・`rescale_follow_offset` をそのまま使う。新しいのは値の Component・重ねの 2 関数・分の計算・画面の選び方の 1 関数だけ。
- **削ったもの**: 分を Component に持ち回る形（DPI とバルーンの大きさの変化で古くなり、追随の札を 2 つ持つことになる）をやめ、使う時点で 2 回解く。`move_cue_args.rs` と `resolver.rs` の変更をやめた（読みは placement・式は変えない）。`ScopeConfig`・`ScopePlacement` に欄を足さない。

### 9.3 Design Decisions

#### Decision: 値の持ち主はキャラ窓の Component
- **Alternatives**: ⑴ キャラ窓の Component（`script_alignment` の型） ⑵ Resource の帳簿（`balloon_lock` の型）。
- **Selected**: ⑴。寿命（5.1〜5.4・5.7）が窓の生き死にと一致し、ゴーストの切り替えでの据え直しの結線が要らない。スコープの鍵と窓の対応を二重に持たない。
- **Trade-offs**: 捨てた記録（7.5）は hook で出すので、アプリの終了でも 1 行出る（ゴーストの終わり＝捨てるので意味は合う）。

#### Decision: 重ねは `resolve_standard_scopes` の中・ずらしは換算の前・向きは合流の後（research の案 C）
- **Alternatives**: 案 A（重ねだけ・利用者のずらしを捨てる）・案 B（追従の値に差を足すだけ）・案 C（A の重ね＋利用者のずらしは差で扱う）。
- **Selected**: 案 C。3 つの道（シェル・面・初期化）が呼び手を変えずに台本の値を読む。換算・右の符号・`windowposition` の調整量・キーワードの再導出・倍率の追随は今の仕組みのまま効く。
- **Trade-offs**: 利用者が動かしたスコープとドラッグの離しで 2 回解く。

#### Decision: 利用者のずらしと記憶（決まったこと 3）
- 追従の値＝台本ありの標準＋利用者のずらし。命令では `今の追従の値 + (新しい標準 − 前の標準)` で据え直す（印のあるスコープだけ）。記憶＝離した相対 − 分。記憶から戻すと記憶＋分。分は使う時点の窓の大きさと DPI で 2 回解いて出す。分を出せないときは記憶へ書かない（台本の分が記憶に入る方を避ける）。

#### Decision: 押し戻しの画面（決まったこと 1）
- `balloon_limit.rs` に `limit_work_area(snapshot, char_rect, balloon_rect, script)` を 1 つ置き、3 か所が呼ぶ。台本の値の有無は Component の有無。帰属の決まりは `work_area_for_window` のまま。

#### Decision: 2 つの変種は表示を待って預かる
- 仮の大きさで解くと、右に出すバルーンと `center`・`bottom` の位置が着地の後にずれる。`\![move]` の動かす窓と同じ `settle` を使い、最初の表示の巡に当てる（同じ巡の move drain なので遅れない）。

### 9.4 Risks & Mitigations
- DPI の境の揺れ — 押し込みの冪等を決定論のテストで固定し、大きさの変化を伴う形は段 4 の実機で見る（design の Open Question 4）。
- バルーンの窓だけが別の DPI の画面へ移ったときの本体との隙間（左に出すバルーンの幅が変わる） — 段 4 で見る。要件に触れる食い違いは 9.4 で決め直す。
- `sylphya-sqlite-kvs` が `follow_test_support.rs` の口を変えると、それを読む新しいテストが着地の順で壊れうる — 新しいテストは支えの公開の口だけを使い、組み立ては `script_balloon_test_support.rs` に持つ。

## 10. 設計の討議で決めたこと（2026-11-27）

- 検証（`design-validation.md`）の重い点 1（預かるとバルーンが descript の位置で見えてから跳ぶ）は、預からずに届いた巡で当て、着地の前なら印を付けて着地した巡に解き直す形で解いた（状態の持ち方で 0 フレーム）。
- 重い点 2（バルーンだけが DPI の違う画面へ移ると、左に出すバルーンと本体の隙間が変わる）は、ukadoc の「接する」を保つ側を取った。バルーン窓の寸が DPI の相で変わった時点が分かる（`frame/dpi.rs` の `GhostWindowKind::Balloon` の腕・`reconcile_window_size` の前後の寸）ので、そこから押しで据え直す。
- `center`・`top`・`bottom` の縦の基準の SSP との違い（8 節）は、この spec では areka の今の幾何のまま §8 に記録し、合わせるかの調べは `balloon-directional-offset-keys` の件 2 へ渡した。
- 残りの項目は `design.md` の「決まったこと」のとおり。
