# Requirements Document

## Introduction

### 誰が困っているか

areka でゴースト `emo2` を動かす利用者が、相方（エモ）側のバルーンでダブルクリックメニューを開くと、**先頭の選択肢が描かれない**。`menu.pasta` の 3 本のメニュー（`menu.pasta:15`／`:33`／`:62`）すべてで先頭の項目（「おしゃべり頻度」「しゃべくり」「調整」）が消え、利用者には「メニューの項目が足りない」と見える。`areka-P0-emo2-conformance-e2e` の実機一周走行 A が A14 でこの症状を踏み、開発者裁定（2026-09-05）「別 spec を切って先に直し、その後に一周を採り直す」により本仕様が起票された（登記: 同 spec `verification/acceptance-record.md` §13.2 #1／#2・§13.3・`tasks.md` Implementation Notes）。本仕様の完遂は **M1 完成判定（e2e）の前提**である。

### いま何が起きているか（本ブランチで実測・2026-09-05・HEAD `36d1c323`＝`cursor-tag-canon` マージ済み）

- 相方側バルーン `emo2-kakukaku` の文字描画範囲は、`balloonk0s.txt:4-7`（`validrect.top,40`／`bottom,-70`／`left,24`／`right,-48`）を画像 288×203 へ解決すると **(24,40)-(240,133)＝幅 216・高さ 93px**（`crates/areka-emo-text/tests/shipped_fixture_region_test.rs:21,147` が固定済み）。フォントは `descript.txt:25-26` の `Yu Gothic UI`・`font.height,28`。
- areka の行送りは `line_pitch = ceil(font.height × 1.25)`（係数の正本 `crates/areka-emo-text/src/state.rs:59-66`・実装 `draw.rs:476-479`・構造テスト用の `FixedMetrics` も同じ既定係数を読む `layout.rs:131-137`）。`font.height,28` では **35px** となり、3 行目（`\_l[5em,2lh]`＝原点から 2 行ぶん）の行矩形は y110..138 で下端 133 を 5px あふれる。
- あふれ判定 `LayoutEngine::visible_window`（`layout.rs:634-680`・判定は `:653-654`）が「最新行の遠端 > 境界」で 1 行ぶんのスクロールを返し、描画（`draw.rs:766-774` の `.skip(first_visible_line)`）が**先頭行**を対象から外す。文字の供給面は文字描画範囲ちょうどの寸（`actor.rs:661-671`）で逃げ場がない。
- `font.height` は文字描画基盤へ **em サイズ**としてそのまま渡している（`draw.rs:313,321,340-353`）。Yu Gothic UI 28em の実行ボックス丈は約 37.24px（`draw_format_metrics_tests.rs:417-450` で固定済み・比 1.3301）。SSP の GDI 的な解釈（`lfHeight`＝セル丈）とは食い違う疑いがあったが、**2026-09-05 の 200% 表示画像の比較（SSP と areka で字の大きさが一致）により、SSP も em 解釈であると確定した**（Requirement 1.1）。
- 係数 1.25 は正典値ではなく **areka 裁量値**である。完了 spec `areka-P0-emo-text-layer` の `design.md:725`（補足正準「SSP の行間はユーザ設定＝正典値なしのため areka 裁量値」）・`:733-736`（DPI/スケール契約表「フォントサイズの写像＝`font.height` の値そのまま」）に明記され、同 `research.md:200` が「SSP 実測との視覚差が出る可能性」をリスクとして登記していた箇所そのもの。ukadoc の `font.height` は「使用するフォントの高さ方向の大きさ（単位はピクセル：ポイントではない）」とだけ記し、em とセル丈を区別しない。`doc/COMPAT_ARCHITECTURE.md` §8（沈黙ルール対応表）には本裁量の登記が**ない**。
- SSP は同じバルーン・同じ `font.height,28` で 3 行を 93px に収める（開発者の SSP 実機スクショから逆算して ≈31px/行）。SSP 本体は本開発機の `C:\wintools\ssp\ssp.exe` に在る（完了 spec `areka-P0-balloon-offset-dpi` `research.md:409` で実測に使用済み）。2026-09-05 の 200% 画像では行送り ≈ 58〜60 物理 px ＝ **29〜30 image px（＝28＋2）**。
- 脳（pasta）は 3 件返し、kanade も 3 件登録し（走行 A の生ログ `choice_count=3`）、配置も 3 行を正しい座標に置く。欠けるのは**縦の寸法**だけである。
- 関連症状（同根）: `descript.txt:14` の `wordwrappoint.x,-34` は画像幅 288 基準で **254** に解決され（`region.rs:251-258`・明示値はクランプされない）、文字描画範囲の右端 **240** の外に出る。`\_l[5em,…]` 後の「閉じる」「もどる」（x164..248）は折り返されずに置かれるが、供給面が文字描画範囲ちょうどのため右端 8px が欠ける。→ 要件ディスカッション議題 1（2026-09-05）で裁定: `wordwrappoint` は折返し基準・`validrect` は絶対上限（Requirement 6）。実フォントの文字送り（Yu Gothic UI は仮名の送りが em より狭い＝プロポーショナル）では「閉じる」3 文字は ≈ 69px → x164..≈233 で右端 240 に収まる見込み。brief の「x164..248」は全角＝em の仮定による机上値であった。

### 何を変えるか

`font.height` の意味（em）と行送りの式（`1lh = 1em + 行間`・行間の既定 2px）を **裁定**（ukadoc の記述＋SSP と areka の表示画像の比較・開発者方針「SSP 実測主義は取らない」2026-09-05）で確定し、正典表と裁量記録を書き換え、実装（フォントサイズの導出・行送り・行ボックス丈・`lh` 係数・選択肢の帯）を同じ源から導き直す。既存の決定論テストの期待値は**緩めずに再導出**し、実物 `emo2-kakukaku` × `menu.pasta` 3 台本で先頭行が落ちないことを固定する新規の決定論テストを加える。関連症状「閉じる」右端欠けは areka の挙動として裁定し、必要な是正を行う。完遂後、`emo2-conformance-e2e` が走行 A〜D を採り直す。

## Boundary Context

- **In scope**（利用者・運用者から見える範囲）:
  - `font.height` の意味論の確定（裁定・SSP の画素実測は行わない）と、行送りの式（`line_pitch`・`1lh`・行間の既定）の確定。確定した値を正典表（完了 spec `emo-text-layer` の design 補足正準・DPI/スケール契約表）と `doc/COMPAT_ARCHITECTURE.md` §8 へ記録する。
  - areka が同じバルーン・同じ `font.height` で SSP と同じ行数を収め、行のインクが重ならず、文字の見た目の大きさが SSP と一致すること。
  - `\_l[N lh]`／`\_l[N em]` の着地が新しい行送りへ自動追随すること（算出式は本仕様が所有・解決は `cursor-tag-canon` の完了実装が引数で受ける）。
  - 選択肢のハイライト帯・ヒット帯が同じ源から導かれ、文字の下（descent）が切れないこと。
  - 折返し基準（`wordwrappoint`＝超えたら折り返す）と描画範囲（`validrect`＝超えてはならない上限）の二段構えの裁定と、描画範囲を超えそうな文字の無条件折返しの実装（開発者裁定 2026-09-05）。
  - 既存の決定論テストの期待値の再導出（緩めない）と、新規の決定論テスト（実物 `emo2-kakukaku` × `menu.pasta` 3 台本・SSP 実測値の固定・折返し閾値の裁定）。
- **Out of scope**:
  - `\_l` の座標語彙・書字方向の座標解決（`areka-P0-cursor-tag-canon`・完了・PR#137）。
  - あふれ判定の式そのもの（`visible_window`）の変更。本仕様は式を変えず、呼び側（行矩形の丈・境界）だけが追随する（`cursor-tag-canon` Requirement 2.8 と整合）。変えたい場合は別途裁定。
  - 行送り方向へ後戻りした行があふれ判定の境界で見せる挙動（`areka-P0-text-decoration-canon` brief「追加登記 4」）。式の変更を要するため本仕様は**引き受けない**。相互参照のみ（Requirement 9.4）。
  - `\f[align]`／文字装飾（`text-decoration-canon`・W13）。本仕様は `draw.rs` に触るので decoration より**前**に着地させる。
  - バルーン縦書きの受口（`balloon-vertical-canon`・完了）。縦書きの列送りは同じ式を軸読み替えで適用するだけで、意味論を新設しない。
  - kanade／pasta 側（脳は正しく 3 件返している）。
  - バルーン資産（`emo2-kakukaku` を含む）の是正。`wordwrappoint` を文字描画範囲の内へ直す等はバルーン側の裁量であり、本仕様は areka の挙動を定めるだけで fixture は改変しない。
- **Adjacent expectations**:
  - **Upstream**: `areka-P0-cursor-tag-canon`（本ブランチの HEAD `36d1c323` に取り込み済み。`lh` の係数は `CursorBasis.line_pitch` として引数で受ける形＝`cursor_tag.rs:120-125`）。完了 spec `areka-P0-emo-text-layer`（正典表の改訂対象）・`areka-P0-balloon-vertical-canon`（領域解決）。
  - **Downstream**: `areka-P0-emo2-conformance-e2e`（本仕様の完遂を待って走行 A〜D を採り直し、M1 完成判定へ。同 spec の `acceptance-record.md` §13.2 の「引受先が実在することの確認」欄を本仕様が埋める）・`areka-P0-text-decoration-canon`（`draw.rs` を後から触る）・`areka-P0-choice-marker-styling`・`areka-P0-balloon-canon-residue`（バルーン記述の粗さの台帳）。
  - **正典参照**: ukadoc `descript_balloon` の `font.height`（単位は px）・ukadoc `\_l[x,y]` の `XXlh`「1lh＝1em＋行間」（`cursor-tag-canon` `requirements.md:193` 付録 A の逐語）。ukadoc に行間の既定値の記述がないため、**裁定で既定値 2px を定め、areka 裁量として `doc/COMPAT_ARCHITECTURE.md` §8 に登記する**（SSP の画素実測は義務にしない＝開発者方針 2026-09-05）。

## Requirements

### Requirement 1: `font.height` の意味と行送りの式を裁定で確定する

**Objective:** 運用者として、`font.height` の意味と行送りの式・行間の既定値を、ukadoc の記述と手元の証跡から裁定で確定したい（SSP 実測主義は取らない＝開発者方針 2026-09-05）。それにより areka の文字層が既存バルーン資産を SSP と同じ行数・同じ文字の大きさで扱える。

**裁定（開発者・2026-09-05・設計ディスカッション議題 1）**: 決定 1＝`font.height` は **em サイズ**（文字描画基盤へ渡すフォントサイズ・現行の解釈のまま）。決定 2＝行送り `line_pitch = font.height + 行間`・行間の既定は **定数 2 image px**。

#### Acceptance Criteria

1. The 本仕様 shall `font.height` を em サイズ（DirectWrite の fontSize へ値のまま渡す・現行どおり）と確定し、根拠として ⑴ ukadoc `font.height`「使用するフォントの高さ方向の大きさ。(単位はピクセル：ポイントではない)」⑵ ukadoc `\f[height]`「スタイルシートのサイズ指定も可能」（CSS の font-size は em の大きさ）⑶ 2026-09-05 の SSP と areka の 200% 表示画像 2 枚で字の大きさ（1 文字の送り ≈ 45 物理 px・インク丈 ≈ 45 物理 px）が一致したこと、を記録する。
2. The 本仕様 shall 行送りの式を `line_pitch = font.height + 行間`（ukadoc `\_l` の「1lh＝1em＋行間」を実体化・`ceil` なし）とし、行間の既定値を定数 2 image px と確定して記録する。根拠: ⑴ SSP の 200% 画像の行送り ≈ 58〜60 物理 px（29〜30 image px・`font.height,28`）⑵ 里々 wiki「選択肢 › 2 段組メニュー」の経験則「戻りたい行数×14」（既定フォント 12 で 1 行 14px＝行間 2）⑶ ukadoc は行間の既定値に沈黙。
3. The 本仕様 shall 根拠となる画像 2 枚（SSP・areka・2026-09-05・拡大率 200%）と読み取り値の表（1 文字の送り・インク丈・行送り・物理 px・目視 ±5px・バルーン画像幅 400 image px が画面上 ≈ 800px であることで等倍を確認）を本仕様の `verification/evidence/` に保存する。
4. The 本仕様 shall 確定した式に `font.height,28` を代入した行送り 30 が、文字描画範囲の高さ 93px に 3 行を収める（3 行目の下端 40+60+28 = 128 ≤ 133）ことを示す。
5. If 確定値と食い違う証跡（他バルーンでの行数差など）が後から見つかった, then the 本仕様 shall 推測で埋めずに証跡と帰結を並べて開発者の裁定へ回す（SSP の画素実測を義務にしない）。
6. The 本仕様 shall SSP の画素実測を DoD に含めない。行間の値は `TextLayerConfig` で変えられる形にし、裁量（根拠つき）として `doc/COMPAT_ARCHITECTURE.md` §8 に登記する（Requirement 2.3）。
7. The 本仕様 shall 「`font.height`＝セル丈」の解釈（研究の候補 α）を採らなかった理由（画像比較で字の大きさが SSP と一致＝em）と、brief の「係数 1.0 は行ボックス丈 37.24 が行送りを超えてインクが重なる」が行ボックスとインクの取り違えであったこと（インク丈 ≈ 22 image px < 30）を記録する。

### Requirement 2: 正典表と裁量記録の改訂

**Objective:** 運用者として、確定した意味論が正典表と裁量記録に一箇所ずつ書かれ、実装と食い違わない状態にしたい。それにより後続の spec（`text-decoration-canon`・`choice-marker-styling`）が同じ表を引ける。

#### Acceptance Criteria

1. When Requirement 1 の意味論が確定した, the 本仕様 shall 新しい正典表（`font.height` の意味・行送りの式・行間の既定値・折返し基準と描画範囲の二段構え）を本仕様の `design.md` に置き、`doc/COMPAT_ARCHITECTURE.md` §8 を上書きの記録先とし（Requirement 2.3）、完了 spec `areka-P0-emo-text-layer` の `design.md` 補足正準（行送りピッチの行）と DPI/スケール契約表（「フォントサイズの写像」の行）は**表の中身を書き換えず**、その直後に「本仕様で改訂・正本は `doc/COMPAT_ARCHITECTURE.md` §8 と本仕様の design」の 1 行注記だけを加える（開発者裁定 2026-09-05・要件ディスカッション議題 2＝アーカイブ非改変の先例〔`cursor-tag-canon`・§8 `:210`〕に揃えた折衷）。
2. The 本仕様 shall 同 `research.md` のリスク登記「行送りピッチ 1.25 係数: SSP 実測との視覚差が出る可能性」に消化済みの注記（本仕様名・日付）を加える。
3. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8（沈黙ルール対応表）へ行を追加し、「`font.height` の意味・行送りの式・行間の既定」（裁定 2026-09-05・ukadoc と表示画像の比較が根拠）と「折返し基準 `wordwrappoint` と描画範囲 `validrect` の二段構え」（開発者裁定 2026-09-05）を、裁量・根拠（実測値／裁定）・出典 spec つきで記録し、完了 spec `emo-text-layer` の表を上書きした事実を明記する。
4. The 本仕様 shall 製品コード（`crates/areka-emo-text/src/` の非テストファイル・テストと `examples/` の doc コメント）と現行の正典表・裁量記録（Requirement 2.1〜2.3 の改訂先）に残る係数 1.25 の記述を洗い出し、改訂後に「`1.25` を**現行の**行送り係数として述べる記述がそこに残っていない」ことを機械的に（同一行に `1.25` と `line_pitch`／`行送り`／`係数` のいずれかを含む行の全文検索で）示す。対象外: DPI 拡大率 k としての `1.25`（`region.rs`・`tests/scale_invariance_test.rs`・`crates/areka/src/placement/`）、履歴として旧式を述べる記述（`roadmap.md` の根因記述・e2e の記録・他の完了 spec のアーカイブ）、および「本仕様で改訂」の注記つきで旧式を引用する記述。
5. The 本仕様 shall `cursor-tag-canon` の要件「`lh` を『行高さ（1em＋行間）』として解釈する」（同 `requirements.md:63`）を改訂せず、本仕様の式がその定義を実体化するものであることを本仕様の記録に明記する。

### Requirement 3: 行送りと文字の大きさが SSP と一致する

**Objective:** 利用者として、SSP で使っていたバルーンを areka でそのまま使ったとき、同じ行数が収まり、行の文字が重ならず、文字の大きさも同じに見えてほしい。

#### Acceptance Criteria

1. When バルーンが `font.height` を宣言している, the emo テキスト層 shall 行送りピッチ `font.height + 2`（Requirement 1.2）で行を送り、`emo2-kakukaku`（`font.height,28`・高さ 93px）で 3 行を文字描画範囲に収める。
2. The emo テキスト層 shall 隣接する行のインクが重ならないようにする（`font.height,28` の Yu Gothic UI ではインク丈 ≈ 22 image px < ピッチ 30）。行ボックス丈（ascent＋descent＝37.24）がピッチを超えること自体は許容し、判定はインクで行う（Requirement 8.5）。
3. The emo テキスト層 shall 文字描画基盤へ渡すフォントサイズを `font.height` の値のまま（em）とし、文字の大きさとベースライン位置を本仕様の前後で変えない（決定 1）。
4. The emo テキスト層 shall 1 文字の送り幅を実フォントの実測（現行の probe）から得る現行の仕組みを変えない（Yu Gothic UI は仮名の送りが em より狭いプロポーショナルなフォントであり、全角＝em を仮定しない）。
5. The emo テキスト層 shall 行送りピッチ・`lh` の係数・比率つき改行の行送り量・選択肢の帯の上限を同じ一つの式（`TextLayerConfig::line_pitch`）から導き、いずれかだけが別の係数を持たないようにする。
6. The emo テキスト層 shall 選択肢のハイライト帯・ヒット帯を行ボックス丈とピッチから導く現行の防御式（行ボックス丈を `[font.height, max(font.height, ピッチ)]` に収める）を保ち、帯が隣接行の帯と重ならず、選択肢の文字のインクが帯の外へ出ないことを実フォントの読み戻しで確かめる（Requirement 5.6・8.5）。If 読み戻しでインクが帯の外へ出た, then the 本仕様 shall 帯をピッチより広げず（隣接行の帯と重なるため）、はみ出しの画素数を数値で報告して開発者の裁定を待つ。
   - **裁定（2026-09-06・直前の「裁定を待つ」条項に対する回答）**: 読み戻しで実際にインクが帯の外へ出た（実フォント Yu Gothic UI・`font.height,28`・帯の下端 y21 に対して文字のインクの下端 y22 ＝ **1 画素**のはみ出し）。開発者の裁定は「**1 画素のはみ出しを許容する**」。したがって the emo テキスト層 shall 帯の下端からのインクのはみ出しを **1 画素以内**に収め、2 画素以上のはみ出しが出た場合は帯を広げずに数値を添えて改めて開発者の裁定を仰ぐ。採らなかった案は「行間を 3 にする（行送り 31）」（正典表の行間の既定 2・行送り 30 を覆すことになり、SSP 表示画像の読み取り 29〜30 とも合わない）と、「帯を行ボックス丈で決める」（帯が隣接する行の帯と重なり、どの選択肢を指しているかの一意性が壊れる）。利用者から見える差は、選択肢を指したときの塗りの下端から文字が 1 画素はみ出すだけである。
7. The emo テキスト層 shall `\n[half]` 等の比率つき改行（行送り量 = 行送りピッチ × 比率）の意味を変えず、比率だけが新しいピッチに掛かるようにする。
8. While 書字方向が縦書き（`vertical_rl`／`vertical_lr`）である, the emo テキスト層 shall 同じ式を列送り（行送り軸の読み替え）にそのまま適用し、縦書き専用の係数や意味論を新設しない。
9. The emo テキスト層 shall フォント名・`font.height` 欠落時の既定値（`ＭＳ ゴシック`・12px）・`font.height,0` の縮退（警告＋既定値）の挙動を変えない。
10. If フォントの実寸（ascent／descent）を取得できない, then the emo テキスト層 shall 警告ログ（フォント名・縮退値を含む）を出して縮退し、ログ無しで別の寸法へ落ちないようにする（現行の縮退経路を保つ）。
11. While 拡大率 k が 1 以外（1.25・2.0）である, the emo テキスト層 shall 行送り・行数・折返しの決定を画像座標空間で行い、k によって行数が変わらないようにする（既存の `scale_invariance_test.rs` の不変性を新しい式でも保つ）。

### Requirement 4: `\_l` の `lh`／`em` が新しい行送りへ追随する

**Objective:** 利用者として、`menu.pasta` の `\_l[5em,2lh]` のように行高さ単位で位置を指定した選択肢が、改行で送った行とぴったり同じ高さに並んでほしい。

#### Acceptance Criteria

1. When `\_l[x,N lh]` を受け取る, the emo テキスト層 shall `N lh` を Requirement 1 で確定した行送りピッチの N 倍として解決し、`\n` を N 回送った行と同じ行送り位置に着地させる（`menu.pasta:15` では `\_l[5em,2lh]` の「閉じる」が `\n` 2 回で送った 3 行目と同じ高さに置かれる）。
2. When `\_l[N em,y]` を受け取る, the emo テキスト層 shall `N em` を `font.height` の N 倍として解決し、`font.height,28` の `5em` を 140px として着地させる（現行と同じ）。
3. The emo テキスト層 shall `\_l` の座標語彙・原点・書字方向ごとの解決規則（`cursor-tag-canon` の完了実装）を変えず、係数の値だけが本仕様の式へ差し替わるようにする。
4. The 本仕様 shall `cursor-tag-canon` が「値が変わると赤になる」形で固定した `\_l` の決定論テスト（横書き・縦書き 3 方向）を、新しいピッチで期待値を再導出して保ち、テストの本数・名前を減らさない。

### Requirement 5: `emo2-kakukaku` のメニューで全選択肢が見える

**Objective:** 利用者として、エモ側バルーンのダブルクリックメニュー 3 本すべてで、選択肢が 1 つも欠けずに表示されてほしい。

#### Acceptance Criteria

1. When `menu.pasta:15`（メインメニュー: 「おしゃべり頻度」「エモの位置調整」「閉じる」）を相方側バルーン `emo2-kakukaku` に表示する, the emo テキスト層 shall 3 つの選択肢すべてを文字描画範囲の内に置き、スクロールを発生させない（先頭可視行 = 0）。
2. When `menu.pasta:33`（おしゃべり頻度: 「しゃべくり」「ほどよく」「たまーに」「もどる」・4 項目 3 行）を表示する, the emo テキスト層 shall 4 つの選択肢すべてを文字描画範囲の内に置き、スクロールを発生させない。
3. When `menu.pasta:62`（位置調整: 「調整」「もどる」・2 項目・2 行目は空）を表示する, the emo テキスト層 shall 2 つの選択肢すべてを文字描画範囲の内に置き、スクロールを発生させない。
4. The emo テキスト層 shall 上記 3 台本で、各選択肢の行矩形の下端が文字描画範囲の下端 133 を超えないようにする。
5. The emo テキスト層 shall 本体側バルーン `emo2`（`balloons0s.txt`・文字描画範囲 (36,46)-(356,168)・高さ 122px）で、収まる行数が新しい式から導かれる 4 行（4 行目の下端 46+90+28 = 164 ≤ 168）になり、本体側で新たに行が落ちる退行を起こさない。行容量が変わることによる既存テストの前提の導き直しは Requirement 7.3、e2e への申し送りは Requirement 10.2 が受ける。
6. While 選択肢の行にマウスが乗っている（hover）, the emo テキスト層 shall ハイライト帯とヒット帯を Requirement 3.5 の同じ源から導き、選択肢の文字の下（descent）が帯の外へ出ないようにする（実機不具合「選択肢の文字の下が切れる」を再発させない）。

### Requirement 6: 折返し基準と描画範囲の二段構え（「閉じる」右端欠けの裁定）

**Objective:** 利用者として、バルーン定義の折返し位置が描画範囲の外に置かれていても、文字が描画範囲の外へはみ出したり欠けたりせず、行の中に収まって読めてほしい。運用者として、折返し位置と描画範囲の意味を裁定として記録し、バルーン側の粗さと areka の挙動を切り分けたい。

**裁定（開発者・2026-09-05・要件ディスカッション議題 1）**: `wordwrappoint` は「ここを超えたら折り返す」**折返し基準**（「」」等の行末禁則文字は基準を超えてぶら下がってよい＝折返しの遅延）。`validrect` は「ここを超えてはならない」**描画範囲の絶対上限**（超えそうなら折返し基準に関わらず無条件に折り返す）。web ページの文字列折返しと同じ二段構えであり、ukadoc の記述（`validrect`＝「テキスト描画範囲」・`wordwrappoint.x` 未指定時は「validrect.right まで書けるものとして扱う」）と整合する。areka の現状は折返し基準だけで折り返し、描画範囲を上限として扱っていない（`layout.rs:315,393` は `wrap_threshold` のみ参照・禁則処理なし）。

#### Acceptance Criteria

1. The 本仕様 shall 上記の裁定（折返し基準＝`wordwrappoint`・絶対上限＝`validrect`）を正典表と裁量記録（Requirement 2）に記録し、SSP が同条件（`emo2-kakukaku`・`menu.pasta:15` の「閉じる」）でどう表示するかの実測（Requirement 1 と同じ撮影から読む）を裁定の裏付け欄に添える。
2. The emo テキスト層 shall 行内軸で文字を置くとき、折返し基準（`wordwrappoint`・未指定なら描画範囲の当該辺）を超えたら折り返し、かつ文字の遠端が描画範囲の当該辺（横書き `validrect.right`・縦書き `validrect.bottom`）を超えそうなときは折返し基準に関わらず無条件に折り返して、描画範囲の外に文字を置かない。
3. While 折返し基準が描画範囲の当該辺の外（本 fixture では 254 > 240）に解決されている, the emo テキスト層 shall 実効の折返し位置を描画範囲の当該辺とし、文字を描画範囲の外へ置かず、供給面（文字を描く面）の寸法を描画範囲ちょうどのまま変えない（描画範囲を広げる案は裁定に反するため採らない）。
4. While 折返し基準が描画範囲の内（本体側 `emo2` では 351 ≤ 356）にある, the emo テキスト層 shall 本仕様の前後で折返しの位置を変えない（行送り以外の差を出さない）。
5. When 実フォント（Yu Gothic UI・em 28）で「閉じる」の文字送りを測った, the 本仕様 shall `menu.pasta:15` の「閉じる」（`\_l[5em,…]`＝x164 起点・3 文字・送り合計 ≈ 69px の見込み → x164..≈233）が描画範囲の右端 240 の内に収まることを決定論テストで示し、brief の「x164..248（1 文字 28px）」が全角＝em の仮定による机上値であったことを記録する。
6. If 確定した文字送りでも「閉じる」が描画範囲の右端に収まらない, then the emo テキスト層 shall 欠けさせずに無条件に折り返し（Requirement 6.2）、その結果として先頭行があふれる場合は描画範囲の外へ折返し基準を置いたバルーン定義側の粗さとして記録する（areka 側で描画範囲を広げて救済しない）。
7. When 折返し基準が描画範囲の外に解決されたバルーンを読み込んだ, the emo テキスト層 shall その事実を警告ログ（バルーン名・解決値・辺の値）で 1 回記録し、バルーン側の粗さとして `areka-P0-balloon-canon-residue` の台帳へ登記する（fixture は改変しない。本 fixture の原因は `balloonk0s.txt` が `wordwrappoint.x` を上書きせず共通 `descript.txt:14` の `-34` を継ぐこと。本体側 `balloons0s.txt` は `-49` を自ら上書きしている）。
8. The 本仕様 shall 選ばなかった案（⑴ 供給面を折返し基準まで広げる＝描画範囲を超えて描くため裁定に反する ⑵ 現状維持＝描画範囲で面を切って 8px 欠ける ⑶ 折返し基準を描画範囲へ丸め込むだけ＝「絶対上限」の意味論を持たず禁則の遅延も表せない）とその帰結を記録する。
9. The 本仕様 shall 行末禁則文字のぶら下がり（折返しの遅延）を本仕様では実装せず、裁定の意味論（禁則文字は折返し基準を超えてよいが描画範囲は超えない）を記録し、設計フェーズまでに実在する引受先 spec を確認して登記する（候補: `areka-P0-text-decoration-canon` brief の追加登記〔`layout.rs` の行の置き場所を扱う〕）。

### Requirement 7: 既存の決定論テストの期待値を緩めずに再導出する

**Objective:** 運用者として、行送りの式が変わっても既存の決定論テストが「同じ意図を新しい値で」検証し続け、許容幅を広げたり本数を減らしたりしていないことを確認できるようにしたい。

#### Acceptance Criteria

1. The 本仕様 shall 行送りピッチ・行ボックス丈・フォントサイズの導出に数値で依存する既存の決定論テストを洗い出し（少なくとも: `draw_format_metrics_tests.rs`・`layout_wrap_tests.rs`・`layout_visible_window_tests.rs`・`layout_cursor_*_tests.rs`・`layout_segmented_tests.rs`・`viewbox_draw_frame_render_tests.rs`・`viewbox_draw_live_diff_tests.rs`・`actor_choice_contract_tests.rs`・`state_cue_apply_tests.rs`・`choice_tests.rs`・`tests/draw_readback_test.rs`・`tests/viewbox_scroll_test.rs`・`tests/viewbox_blit_spike.rs`・`tests/pipeline_test.rs`・`tests/scale_invariance_test.rs`・`tests/emo2_fixture_e2e_test.rs`・`examples/emo-text-layer/`、および `research.md` §3.3 が追加で挙げる `state_reveal_tests.rs`・`viewbox_axis_tests.rs`・`viewbox_dirty_tests.rs`・`viewbox_plan_commit_tests.rs`・`actor_tests.rs`・`actor_scale_refresh_tests.rs`・`viewbox_draw_choice_hover_tests.rs`・`viewbox_draw_png_dump_tests.rs`・`layout_cursor_tests.rs`・`cursor_tag_tests.rs`・`cursor_tag_resolve_tests.rs`・`tests/choice_fixture_test.rs`、および設計バリデーション（2026-09-05）で追加した `draw_oracle_tests.rs`・`viewbox_draw_oracle_regression_tests.rs`＝計 32 ファイル。`research.md` §3.3 の一覧を正本とし、着手時に crate 全域の検索で漏れがないことを確かめる）、それぞれの期待値を新しい式から**計算で導出**して更新する。
2. The 本仕様 shall 期待値の更新にあたり、許容幅（±px・比率）を広げず、`assert_eq` を範囲判定へ置き換えず、テストを `#[ignore]` にせず、テストの本数と名前を減らさない（陳腐化したテストを除外する場合は、検証対象が仕様判断で退役した根拠を個別に記録する）。
3. The 本仕様 shall 「3 行が収まり 4 行目であふれる」等の容量前提を持つテスト（`viewbox_draw_live_diff_tests.rs` の矩形寸法・`tests/viewbox_scroll_test.rs` のコンパイル時検査・`examples/emo-text-layer/scenario.rs` の 3 行前提）について、新しいピッチでも同じ前提が成り立つように寸法を導き直し、前提が崩れて検証が空振りになる（緑のまま意味を失う）ことを防ぐ。
4. The 本仕様 shall 参照描画との画素等価比較（`viewbox_draw_oracle_regression_tests.rs`・`viewbox_draw_live_diff_tests.rs`）が両側とも同じ寸法で動くことを確認したうえで、注入した差分を検出する負の対照（`live_diff_detects_injected_divergence`）が新しい式でも赤になることを示す。
5. The 本仕様 shall `cursor_tag_test_support.rs` のように行送りを定数注入しているテストについて、注入値と doc コメントを新しい式の値へ揃える（期待値の計算は変わらないが、旧式の値を「正典」と述べる記述を残さない）。
6. The 本仕様 shall 更新後の全テストを対象クレート（`areka-emo-text`）とワークスペース全体で実行し、終了コードで合否を判定する（`| tail` 等で終了コードを隠さない）。

### Requirement 8: 新規の決定論テスト

**Objective:** 運用者として、本仕様が直した症状と確定した数値が、実機を起動せずに毎回のテストで検証されるようにしたい。

#### Acceptance Criteria

1. The 本仕様 shall 実物 `emo2-kakukaku` の `descript.txt`＋`balloonk0s.txt` を 288×203 で解決し（(24,40)-(240,133)・折返し基準 254・描画範囲の右端＝絶対上限 240）、`menu.pasta:15`／`:33`／`:62` の 3 台本を実 parser → 実 compile → 実 state → 実領域解決 → 実配置で通して、3 台本すべてで先頭可視行が 0 であること、および各選択肢の行矩形が文字描画範囲の縦の内に収まることを固定する決定論テストを加える。
2. The 本仕様 shall 上記を折返し方式 2 通り（1 文字ずつ／budoux による分節）の両方で実行し、結果が同一であることを固定する。
3. The 本仕様 shall 裁定値（`font.height,28`・Yu Gothic UI で行送り 30・1 文字の送り ≈ 23・インク丈 ≈ 22 image px）を実フォントの読み戻しテストで定数として固定し、根拠画像の読み取り値（物理 px・2026-09-05・`verification/evidence/`）をコメントで添える。SSP の画素実測は行わない。
4. The 本仕様 shall Requirement 6 の裁定を決定論テストで固定する: (a) 折返し基準が描画範囲の外に解決されている `emo2-kakukaku` の実 fixture で、確定した文字送りのもと「閉じる」「もどる」が描画範囲の右端 240 の内に収まり折り返されないこと、(b) 純粋層の固定寸法で、折返し基準が描画範囲の外にある領域に描画範囲を超える長さの文字列を置くと折返し基準に達する前に無条件で折り返され、どの文字の遠端も描画範囲を超えないこと（横書きと縦書き）、(c) 本体側 `emo2` の折返し位置が本仕様の前後で変わらないこと。
5. The 本仕様 shall 行ボックス丈が行送りピッチを超えない（隣接行のインクが重ならない）ことを、実フォントの読み戻しで 2 行を並べて固定する。
6. The 本仕様 shall 新規テストを本番ファイルと同じディレクトリの兄弟ファイル（`<stem>_<theme>_tests.rs`）または `tests/` へ置き、1 ファイル 1,000 行以下を守り、行数の見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の例外表に触れない。
7. The 本仕様 shall 新規テストのうち少なくとも 1 本について、行送りの式を旧式（1.25）へ戻すと赤になることを示す（判定が生きていることの対照）。

### Requirement 9: 変えないもの（境界の固定）

**Objective:** 運用者として、本仕様が触ってよい範囲と触ってはならない範囲を、テストと記録で確認できるようにしたい。

#### Acceptance Criteria

1. The emo テキスト層 shall あふれ判定の式（最新行の遠端 > 境界・最小スキップの探索・全行超過時の飽和）を変えず、同じ行矩形の入力に対して本仕様の前後で同じ先頭可視行・同じオフセットを返す（`layout_visible_window_tests.rs` の再導出は行矩形の丈が変わることによる期待値の更新のみで、判定の分岐を増減しない）。
2. The emo テキスト層 shall `\_l` の座標語彙・原点・書字方向ごとの解決規則、`\c` の全消去、比率つき改行の意味、reveal のペース（配送された再生時間から導出）を変えない。
3. The 本仕様 shall バルーン fixture（`crates/pilot/examples/shiori-host-32/fixtures/emo2/` 配下）・kanade・pasta・sakura（parser／compile）を改変しない。
4. The 本仕様 shall `text-decoration-canon` brief「追加登記 4」（行送り方向へ後戻りした行があふれ判定の境界の外に残る所見）を引き受けず、本仕様の記録から同登記を相互参照するにとどめる。同登記が `layout_cursor_overflow_tests.rs:113-166` で固定している現状値は、行送り（13 → 12）と「ちょうど 3 行」の境界前提の変化による再導出のみ行う（行矩形の丈＝`font_height` は不変）。
5. The 本仕様 shall `draw.rs` を触る唯一の進行中 spec であること（`text-decoration-canon` は本仕様の後）を着手時に確認し、`draw.rs`・`layout.rs`・`state.rs`・`region.rs`・`actor.rs`・`choice.rs` の各ファイルが 1,000 行以下のまま着地するようにする。
6. The 本仕様 shall 意味論の確定・実装の追随・テストの再導出を同じブランチの連続したコミット列で揃え、正典表と実装がずれた状態を中間コミットに残さない。

### Requirement 10: 下流への引き渡し

**Objective:** e2e の運用者として、本仕様の完遂後に一周走行を採り直すための前提（何が変わり、何が変わっていないか）を、走行前に読めるようにしたい。

#### Acceptance Criteria

1. When 本仕様の実装が着地した, the 本仕様 shall `areka-P0-emo2-conformance-e2e` の `verification/acceptance-record.md` §13.2 #1／#2 の「引受先が実在することの確認」欄に、本仕様のディレクトリと確認日を記入する。
2. The 本仕様 shall 利用者から見える変化（行送りの値・文字の大きさ・「閉じる」の表示・本体側バルーンの行容量が 3 行から 4 行へ増えること）と変わらないもの（`\_l` の語彙・あふれ判定の式・本体側バルーンの表示）を、e2e の走行 A〜D の再走前に読める形で 1 箇所にまとめる。
3. The 本仕様 shall `.kiro/steering/roadmap.md` の W12 裁定枠 A′ の行を完了へ更新し、`text-decoration-canon` brief の「追加登記 4」に本仕様が引き受けなかった旨の相互参照を加える。
4. The 本仕様 shall 実機走行を DoD に含めない（実機の一周は e2e が採り直す）。Requirement 8.3 の実フォント読み戻しは DoD に含め、SSP の画素実測は含めない（Requirement 1.6）。
