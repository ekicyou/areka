# Requirements Document

> 本文の事実は **2026-10-04・本ブランチ**（main `e2a373b5`＝棚卸㉑の後）で引き直したもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、brief が議題として残した点と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code など）でゴーストを作る人は、送った台本の結果を「目で」確かめたい。SSP の MCP には `dump_surface`（今の見た目、または指定した surface 単体）と `dump_balloon`（最後に描いたバルーン）があり、透過 PNG を返す。areka の窓は GPU で合成しているので OS のスクリーンショットでは撮れず、エージェントは areka の画面を確かめる手段を持たない。

**今の状態**: `mcp-tool-entrances` の着地で、areka の `tools/list` には SSP と同じ定義の `dump_surface`・`dump_balloon` が並び、引数の検査と `ghost_name` の解決までは済んでいる。中身は 2 本とも `NG:not implemented yet` を返すダミー（`crates/areka/src/mcp/dump_surface.rs`・`dump_balloon.rs` の `handle`）。合成済みの絵を読み戻す口（`crates/areka-emo-present/src/presenter/read.rs` の `EmoPresenter::read_back`）と文字の層を読み戻す口（`crates/areka-emo-text/src/surface.rs` の `TextSurface::read_back`）は在るが、本番の経路からは呼ばれていない。PNG と base64 の符号化は本番のコードに無い。

**何を変えるか**: `dump_surface(scope?, surface?, ghost_name?)` は、`surface` を省くと今表示している姿（動いているアニメーションと着せ替えを含み、拡大縮小と半透明を掛ける前）を、`surface` を渡すとその surface 単体を初期状態で、透過 PNG で返す。`dump_balloon(scope?, ghost_name?)` は、そのスコープのバルーンを最後に描いた内容で（今は隠れていても、描いた内容が残っていれば）返す。画像は `{type:"image", mimeType:"image/png"}` の content、本文は SSP と同じ文言にする。

> 起票: 2026-09-29 `/kiro-discovery`（SSP MCP の移植の 3 段目の 1 本）。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 の実測）、ツール定義の逐語は [doc/ssp-mcp/tools-list-ssp-2.9.05.json](../../../doc/ssp-mcp/tools-list-ssp-2.9.05.json)。経緯と再測定は同じフォルダの [brief.md](brief.md)。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人**: `sakurascript` で表情や台詞を送っても、areka では結果の絵を受け取れない。SSP 向けに書いた「送る → `dump_surface`／`dump_balloon` で見る」の手順が areka では `NG:not implemented yet` で止まる。
- **シェルを作る人**: surface を 1 つずつ単体で描かせて確かめたい（今の画面を変えずに）。

### 正典（ukadoc で確かめたもの）

- SSP ヘルプ「その他の機能」（<https://ssp.shillest.net/ukadoc/ssphelp/other.html>）: `dump_surface` は「サーフィスIDを省略すると現在表示中の姿(拡大縮小・透明度の適用前)を、指定するとそのサーフィス単体を初期状態で描画したものを返す」「1回1枚」。`dump_balloon` は「最後に描画された状態を返す」。
- ukadoc `\![execute,dumpballoon,…]`: 「その時点でバルーンに描かれている内容がそのまま出力される。拡大縮小や半透明(フェード含む)は適用前の等倍の画像になる」「バルーンが非表示でも、描画内容が残っていれば出力される」。
- ukadoc `\![execute,dumpsurface,…]`: スコープ ID の指定で「着せ替え状態などのスコープに依存する処理が変わる」。
- 結果の文言は SSP 2.9.05 の実測（survey §3）。**実測していないもの**（下の裁定 3〜6・8）は ukadoc にも記述が無く、areka の文言を暫定で置いた。

### いま何が起きているか（2026-10-04 実測）

- ダミーの `handle(world, ghost, args, reply)` は UI スレッドで呼ばれ、検査を通った引数（`scope: Option<i64>`・`surface: Option<i64>`・`ghost_name`）と解決済みのゴーストを受け取る。画像を付ける口 `areka_mcp::tools::outcome::with_image(outcome, png_base64)` は在る（base64 済みの文字列を受ける）。
- `EmoPresenter::read_back` は合成済みの絵を**原寸**（画面の拡大率に依らない）で返す。未表示の対象では `error!` を出して失敗を返す＝MCP から呼ぶたびに ERROR の記録が出うる（brief「見つけた穴」）。表示の有無は同じファイルの `target_visible`・`current_surface_id` で先に確かめられる。
- バルーンは背景（合成の層・原寸）と文字（別の層・画面の拡大率を掛けた大きさ）の 2 枚に分かれている＝拡大率が 1 でないとき 2 枚の大きさが合わない（brief の議題）。
- 待ちの上限は `mcp-tool-entrances` が決めた 10 秒（超えると `NG:areka did not respond within 10 seconds`）。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | `dump_balloon` の「拡大の前」（brief の議題 ⒜⒝⒞） | **原寸（拡大率 1 のときの大きさ）で返す**。⒞「拡大の後の大きさで返す」は取らない。⒜ 文字を縮めて重ねる／⒝ 拡大率 1 で描き直す、のどちらで作るかは設計で決める | ukadoc「拡大縮小や半透明…は適用前の等倍の画像」・brief の Out of Boundary「拡大縮小・半透明の適用後の画像」。⒜と⒝は字の鮮明さが違いうるので、設計で選んだ結果を要件ディスカッション後の設計討議に出す | 3.3 |
| 2 | `scope` の省略 | **0** | 定義の description「Default 0」 | 1.2・3.2 |
| 3 | 存在しないスコープ | そのゴーストにキャラクターの窓が無いスコープ（負の数を含む）は、2 本とも **`NG:No such scope in this ghost`** | `dump_surface` は SSP の実測の文言。`dump_balloon` の存在しないスコープは未実測＝同じ文言に揃える | 4.1 |
| 4 | `surface` 省略で、そのスコープが何も表示していない（`\s[-1]` で隠した・まだ一度も出していない） | **`NG:No surface is currently shown in this scope`**（areka の文言） | SSP は未実測。空の画像を返すより、理由の分かる失敗のほうがエージェントが次の手を選べる | 4.3 |
| 5 | そのスコープのバルーンをまだ一度も描いていない・描いた内容が残っていない | **`NG:No balloon has been drawn in this scope yet`**（areka の文言） | SSP は未実測。ukadoc は「描画内容が残っていれば出力される」＝残っていなければ出ない | 4.4 |
| 6 | 窓の無いゴースト（窓への結線が成立せず記録だけの起動へ倒れた） | 2 本とも **`NG:This ghost has no window`**（areka の文言） | `mcp-tool-entrances` 要件 3.8「窓が要るツールが窓の無いときにどう答えるかは各 spec が `NG:` で決める」 | 4.5 |
| 7 | 指定した surface の「初期状態」 | **そのスコープの今の着せ替えを掛け、アニメーションは始まる前の姿**。今の画面は変えない | ukadoc `dumpsurface`「スコープ ID…で、着せ替え状態などのスコープに依存する処理が変わる」・SSP ヘルプ「単体を初期状態で描画」。アニメーションの途中のコマは台本の `dumpsurface` のアニメーション ID の役目（本 spec の外） | 2.1・2.2 |
| 8 | 指定した surface のときの成功の本文 | **`OK:scope <n>, surface <id> rendered alone in its initial state (before scaling and transparency)`**（areka の文言） | SSP は「今の見た目」の本文しか実測していない（survey §3）。「as currently shown」と書くと嘘になるので別の文にする。実測できたら SSP の文言へ差し替える | 2.4 |
| 9 | 今話している途中の呼び出し | **拒まず、その時点の絵を返す**（文字が途中まででもよい） | `dump_balloon` の description「While the ghost is still talking the text may be partially drawn; wait until get_status no longer reports talking」＝待つのは呼ぶ側 | 3.5 |
| 10 | シェルの絵の中の「箱」（`shell-balloon` の `\b[名前]`）に書いた台詞 | `dump_balloon` は**バルーンの窓だけ**を返し、箱の文字は含めない。`dump_surface`（省略）に箱の文字が写るかは**設計で調べて差の一覧に書く**（どちらでも本 spec の合否を変えない） | 箱はキャラクターの窓の子で、SSP に対応物が無い（areka の拡張） | 3.6 |
| 11 | 画像の大きさの上限 | **設けない**（1 回 1 枚・原寸） | SSP も原寸で返す。シェルの絵は高々数千 px 四方 | 5.5 |

## Boundary Context

- **In scope**:
  - `dump_surface` の 2 つの経路（今の見た目／指定した surface 単体）と `dump_balloon` の中身。
  - 結果の形（本文＋透過 PNG の画像 1 枚）と、SSP と同じ成功・失敗の文言。
  - 本番で画面の絵を読み戻す口・PNG と base64 の符号化。
  - 決定論テストと実機確認、SSP との差の一覧への追記。
- **Out of scope**:
  - 台本の `\![execute,dumpsurface,…]`／`\![execute,dumpballoon,…]`（ファイルへの書き出し・出力先の制限・完了イベント・複数の surface・アニメーション ID の連番）。本 spec の部品で後から足せる（起票は棚卸で判断）。
  - 拡大縮小・半透明を掛けた後の画像、窓全体やデスクトップのスクリーンショット。
  - ツールの定義・引数の型の検査・`ghost_name` の解決・待ちの上限（`mcp-tool-entrances` のまま変えない）。
  - 3 人目以降のキャラクターの窓（`extra-character-windows`）。窓ができれば同じ規則で撮れる形にするが、本 spec では窓を足さない。
  - `Cargo.toml` の変更（新しい依存 0）。
- **Adjacent expectations**:
  - `mcp-tool-entrances` から受け取るもの: 検査を通った引数・解決済みのゴースト・結果を作る関数（`value`／`ok`／`ng`／`with_image`）・10 秒の待ちの上限。
  - 同じウェーブの約束（`.kiro/steering/roadmap.md`「ウェーブ編成」の C3）: `crates/areka/src/mcp/mod.rs`・`crates/areka-mcp/src/handler.rs`・`areka-emo-compose` を触らない。本番の読み戻しの口は `crates/areka/src/emo2_boot/frame/wiring.rs` だけに開ける。足すファイルは自分のツールのファイルの子モジュールにする。触る要が出たら止めて報告する。
  - `mcp-expression-table`（並走）: 失敗の文言 `NG:No such surface ID. Check get_expression_table tool` は相手のツールの名前を挙げるだけで、相手の着地を待たない。
  - `mcp-log-history`（並走）: MCP の呼び出しが原因の ERROR の記録を出さない（要件 4.6）ので、相手の error 種別の履歴に本 spec 由来の行は積まれない。

## Requirements

### Requirement 1: `dump_surface`（`surface` 省略）＝今の見た目を返す

**Objective:** As a AI エージェントでゴーストを作る人, I want 今画面に出ているキャラクターの姿を画像で受け取れること, so that `sakurascript` で送った表情や着せ替えの結果を目で確かめられる

#### Acceptance Criteria

1. When `surface` を省いた `dump_surface` が届き、そのスコープのキャラクターが表示されている, the areka shall そのスコープの今の合成済みの絵（動いているアニメーションのその時点のコマと、着せ替えを含む）を透過 PNG 1 枚で返す。
2. When `scope` が省かれている, the areka shall スコープ 0 として扱う。
3. The areka shall 画像の大きさを、画面の拡大率（DPI・拡大縮小の設定）と窓の半透明の設定に依らない原寸とし、それらを掛ける前の画素を返す。
4. When 1.1 に成功する, the areka shall 本文を `OK:scope <スコープ番号>, surface <今の surface ID> as currently shown (with running animations and dressups, before scaling and transparency)`・`isError: false` とし、その後に画像の content を 1 つ続ける（例: `OK:scope 0, surface 3 as currently shown (…)`）。
5. The areka shall 読み戻しによって画面の表示・再生中の台詞・アニメーションの進みを変えない。

### Requirement 2: `dump_surface`（`surface` 指定）＝その surface 単体を返す

**Objective:** As a シェルを作る人と AI エージェントの利用者, I want 画面を変えずに、指定した surface の絵を単体で受け取れること, so that 表情の一覧を 1 枚ずつ確かめられる

#### Acceptance Criteria

1. When `surface` を渡した `dump_surface` が届き、その ID がそのスコープのシェルに在る, the areka shall その surface を単体で、アニメーションが始まる前の初期状態で描いた透過 PNG 1 枚を返す。
2. The areka shall 2.1 の絵に、そのスコープの今の着せ替えの状態を掛ける。
3. The areka shall 2.1 の処理で画面の表示（今の surface・再生中のアニメーション・台詞）を変えない（そのスコープが今何も表示していなくても、指定した surface は返す）。
4. When 2.1 に成功する, the areka shall 本文を `OK:scope <スコープ番号>, surface <指定した ID> rendered alone in its initial state (before scaling and transparency)`・`isError: false` とし、その後に画像の content を 1 つ続ける。
5. The areka shall 2.1 の画像の大きさを、要件 1.3 と同じく原寸とする。

### Requirement 3: `dump_balloon`＝最後に描いたバルーンを返す

**Objective:** As a AI エージェントでゴーストを作る人, I want バルーンに描かれた台詞を画像で受け取れること, so that 文字の折り返し・選択肢・装飾の出方を目で確かめられる

#### Acceptance Criteria

1. When `dump_balloon` が届き、そのスコープのバルーンに描いた内容が残っている, the areka shall バルーンの背景の絵とその上の文字を重ねた、最後に描いた内容の透過 PNG 1 枚を返す。
2. When `scope` が省かれている, the areka shall スコープ 0 として扱う。
3. The areka shall 画像の大きさを、画面の拡大率に依らないバルーンの原寸とし、拡大縮小と半透明（消えていく途中のフェードを含む）を掛ける前の画素を返す。拡大率が 1 でないときも、背景と文字の位置がずれない。
4. While バルーンが隠れている（閉じた・時間切れで消えた）, when `dump_balloon` が届く, the areka shall 描いた内容が残っている限り、3.1 と同じく最後に描いた内容を返す。
5. While ゴーストが話している途中, when `dump_balloon` が届く, the areka shall 拒まずに、その時点までに描いた内容を返す。
6. The areka shall `dump_balloon` の画像にバルーンの窓の内容だけを含め、シェルの絵の中の箱（`\b[名前]`）に書いた文字を含めない。
7. When 3.1 に成功する, the areka shall 本文を `OK:balloon of scope <スコープ番号> as last drawn (before scaling and transparency; kept even if the balloon is hidden now)`・`isError: false` とし、その後に画像の content を 1 つ続ける。
8. The areka shall 読み戻しによってバルーンの表示・隠れている状態・再生中の台詞を変えない。

### Requirement 4: 撮れないときの答え

**Objective:** As a AI エージェントの利用者, I want 撮れない理由が決まった文言で返ること, so that エージェントが呼び方の誤りか、ゴーストの状態かを見分けて次の手を選べる

#### Acceptance Criteria

1. If `scope` が、そのゴーストにキャラクターの窓のあるスコープのどれでもない（負の数を含む）, then the areka shall `dump_surface`・`dump_balloon` のどちらでも本文 `NG:No such scope in this ghost`・`isError: true` で答える。
2. If `dump_surface` の `surface` が、そのスコープのシェルに無い ID である（負の数を含む）, then the areka shall 本文 `NG:No such surface ID. Check get_expression_table tool`・`isError: true` で答える。
3. If `surface` を省いた `dump_surface` が届き、そのスコープが何も表示していない（隠している・まだ一度も表示していない）, then the areka shall 本文 `NG:No surface is currently shown in this scope`・`isError: true` で答える。
4. If `dump_balloon` が届き、そのスコープのバルーンに描いた内容が残っていない（まだ一度も描いていない場合を含む）, then the areka shall 本文 `NG:No balloon has been drawn in this scope yet`・`isError: true` で答える。
5. If 対象のゴーストに窓が無い（窓への結線が成立しなかった起動）, then the areka shall 2 本とも本文 `NG:This ghost has no window`・`isError: true` で答える。
6. The areka shall 4.1〜4.5 を、読み戻しを試みる前に確かめて答え、ERROR の記録を出さない（`debug!` までにとどめる）。
7. If 4.1〜4.5 のどれにも当たらないのに読み戻し・符号化が失敗する, then the areka shall 本文 `NG:` ＋理由の英文・`isError: true` で答え、`error!` を 1 件残す（ツール名・スコープ・失敗の理由を載せる）。黙って空の画像や成功の本文を返さない。
8. The areka shall 失敗の結果に画像の content を付けない（content は本文 1 つだけ）。
9. The areka shall 複数の条件に当たるとき、窓の無いゴースト（4.5）→ スコープ（4.1）→ surface ID（4.2）→ 表示・描画の有無（4.3・4.4）の順で最初に当たった文言を返す。

### Requirement 5: 画像の形

**Objective:** As a AI エージェントの利用者, I want 返る画像がそのまま開ける正しい透過 PNG であること, so that エージェントの画像の読み取りや、人が保存して開く使い方で色と透け方が画面どおりに見える

#### Acceptance Criteria

1. The areka shall 画像の content を `type: "image"`・`mimeType: "image/png"`・`data`＝PNG のバイト列の base64（標準の文字集合・`=` の詰めあり・改行なし）とする。
2. The areka shall PNG を 8 ビットの RGBA（アルファは乗算していない値）で作り、完全に透明な画素・半透明の画素・不透明な画素の色とアルファが、画面で合成に使っている絵と一致するようにする（乗算済みの値を戻すときの丸めの差は 1 段階まで）。
3. The areka shall PNG の幅と高さを、読み戻した絵の幅と高さに一致させる（余白を足さない・切り落とさない）。
4. The areka shall 1 回の呼び出しで画像を 1 枚だけ返す。
5. The areka shall 画像の大きさに上限を設けず、原寸のまま返す。

### Requirement 6: ゴーストと利用者を邪魔しない

**Objective:** As a areka を常駐させている利用者, I want 画像を撮られている間もゴーストがいつもどおり動くこと, so that エージェントが何度撮っても、描画が引っかかったり会話が止まったりしない

#### Acceptance Criteria

1. While `dump_surface`・`dump_balloon` を処理している, the areka shall ゴーストの台詞の再生・アニメーション・メニュー・クリックの受け取りを止めない（処理は 1 回の呼び出しの中で終わり、返事を待って UI を塞がない）。
2. The areka shall 2 本の処理で、ゴーストへイベントを送らず、台本を再生せず、ファイルを書き出さない（どれも 0 回）。
3. The areka shall 待ちの上限と、終了の途中の答えを `mcp-tool-entrances` のまま（10 秒・`NG:areka did not respond within 10 seconds`・`NG:areka is shutting down`）とし、本 spec で変えない。
4. When `ghost_name` を渡した呼び出しが届く, the areka shall `mcp-tool-entrances` が解決したゴーストの絵を返す（解決の失敗の文言は相手の spec のまま）。

### Requirement 7: 決定論テスト・差の一覧・実機確認

**Objective:** As a 後でこのツールや描画の層を触る開発者, I want 画像の中身と文言がテストで固定され、SSP との差が一覧に残ること, so that 色や大きさが変わったとき・文言がずれたときに赤で分かる

#### Acceptance Criteria

1. The areka shall 符号化を決定論テストで固定する: 既知の画素（完全に透明・半透明・不透明を含む）を PNG にし、読み戻した幅・高さ・各画素の RGBA が要件 5.2・5.3 のとおりであること、base64 が既知の入力（長さが 3 の倍数・余り 1・余り 2・空）で正しいこと。
2. The areka shall 要件 4.1〜4.5・4.9 の判断（どの文言を返すか）を、窓や GPU を要しない判断として持ち、各場合を決定論テストで固定する（スコープ省略＝0・負のスコープ・窓の無いスコープ・無い surface ID・負の surface ID・何も表示していない・バルーン未描画・窓の無いゴースト・複数に当たるときの順）。
3. The areka shall 成功の本文 3 種（要件 1.4・2.4・3.7）を、スコープ番号と surface ID を変えた例で決定論テストに固定する。
4. The areka shall 絵の中身を、実際の描画を通るテストで固定する: ⑴ 今の見た目が表示中の surface の画素と一致する、⑵ 指定した surface が画面を変えずにその surface の画素で返る、⑶ バルーンが背景と文字の両方を含み、拡大率が 1 でないときも原寸で返る、⑷ 隠れたバルーンでも最後の内容が返る。GPU を使うテストはこのリポジトリの定石（常時テストに入れる条件・画面外の描画先での画素の検証）に従う。
5. The areka shall `mcp-tool-entrances` が置いた 2 本のダミーのテスト（`NG:not implemented yet` を固定するもの）を、本 spec の振る舞いを固定するテストへ書き換える（`NG:not implemented yet` を返す経路を 2 本に残さない）。
6. The areka shall SSP との差の一覧（`doc/ssp-mcp/` の下）に、areka の文言を置いた 5 件（要件 2.4・4.3・4.4・4.5 と、`dump_balloon` の存在しないスコープ）と、シェルの絵の中の箱の文字が `dump_surface` に写るかどうかを、「SSP は未実測」の印とともに書く。
7. The areka shall 実機で確かめ、結果を本 spec の `verification/signoff.md` に残す: ⑴ 既定ゴースト（emo2）で `dump_surface`（省略）が今の姿の透過 PNG を返し、開いて画面と同じに見える、⑵ `sakurascript` で表情を変えた後に撮ると変わった姿が返る、⑶ `surface` 指定で画面が変わらずにその surface が返る、⑷ 台詞を出した後の `dump_balloon` が背景と文字の入った画像を返し、バルーンが消えた後も同じ内容が返る、⑸ 画面の拡大率が 1 でない環境で ⑴ と ⑷ が原寸で返る、⑹ 存在しないスコープと surface ID が要件 4.1・4.2 の文言で返る、⑺ 撮っている間もゴーストの描画と会話が止まらず、ERROR の記録が増えない。
