# Requirements Document

## Introduction

areka は二人立ちゴースト（本体＝sakura／相方＝kero）を表示するが、**バルーン資産の選択が本体側に固定**されている。正典（ukadoc）はバルーン画像を **`balloons{ID}.png`＝本体側**・**`balloonk{ID}.png`＝相方側（三人目以降も含む）** の別系列と定め、各面には対応する面別上書き設定 `balloons{ID}s.txt` / `balloonk{ID}s.txt`（`windowposition`・`validrect` 等でバルーン既定設定 `descript.txt` を上書き）が付く。ところが現状の areka は、①バルーン画像の列挙が `balloons` 接頭辞のみを受理し、②面別上書きも本体側の面 0（`balloons0s.txt`）を 1 回だけ読んで全 scope で共有し、③窓配置採寸もバルーン面 0 を「全スコープ共通」として 1 回だけ計測する。結果、相方側 scope にも本体側バルーンの見た目・`windowposition`・寸法が適用され、**相方専用の `balloonk0.png` / `balloonk0s.txt` は実行時に一切使われない**。この欠陥は `sylphya` 実機サインオフ（2026-07-24）で、本体と相方のバルーン枠が同一形状であることとして開発者が発見した。

症状の切り分けは discovery で完了している。疑われた「`\b[ID]` バルーン面切替の未対応」は**原因ではない**——`\b` 経路は `completed/areka-P0-balloon-face-cue` が一気通貫で実装済みかつ実描画檻で固定されている。**原因は「scope 別バルーン資産の選択が存在しないこと」**であり、バルーン定義のパーサ層は `balloonk0s.txt` のマージ解析を既に実装・檻化済み（資産は読める・配線が捨てている）。

本仕様（W5・挙動バグ）は、scope ごとに正典どおりのバルーン系列を解決し、その系列の面と面別上書き設定を **表示・窓配置採寸・バルーン文字層の三面すべて**へ反映することを確立する。正典のフォールバック規約（`balloonk{ID}` が無ければ**同一 ID の**本体側 `balloons{ID}` を使う）に従うため、`balloonk*` を持たないバルーンでは現行と同一の見た目・配置が保たれる（後方互換）。完了時、実機（ゴースト emo2＋バルーン emo2-kakukaku）で本体側と相方側のバルーン枠形状・表示位置が互いに異なることが目視で確認できる。

上流の W4 `emo-dpi-scaling` は本仕様の席を意図的に保全して着地している（scope 別バルーン寸の供給点をループ内に残し、scope をキーとするバルーン定義の記憶口を導入済み）。同時に W4 は表示スケール k の再追従経路を新設したため、本仕様は **装着と再追従が同一の scope→文字層アクタ写像を用いる義務**、および W4 が未所有残件として登記した **「適用 k が同値なら再追従しない」判定の穴**（scope 別バルーン資産が当事者）も引き受ける。

## Boundary Context

- **In scope**:
  - scope 別のバルーン系列解決（本体側 scope＝`balloons{ID}` 系列／相方側 scope＝`balloonk{ID}` 系列）。
  - 正典準拠の **ID 単位フォールバック**（`balloonk{ID}` 不在時は同一 ID の `balloons{ID}` へ縮退・系列一括ではない）。
  - scope 別のバルーン定義（バルーン既定設定に面別上書き `balloonk{ID}s.txt` / `balloons{ID}s.txt` をマージした結果）の保持と適用——特に `windowposition` / `validrect`。
  - 窓配置採寸の scope 別化（各 scope が解決したバルーン面 0 の実寸を用いる・表示スケール k 追従込み）。
  - バルーン窓の**初期既定位置**を当該 scope の `windowposition`（数値指定）から決定すること（基本位置＝現行と同一・永続値があればそちらが優先）。
  - バルーン文字層が当該 scope の定義で文字描画領域を解決すること、および装着と k 再追従が同一の scope→アクタ写像を用いること。
  - 適用 k が同値のままバルーン面の実寸／文字描画領域が変化した場合の再追従（W4 からの申し送り残件）。
  - 判定分岐の決定論檻（系列選択・フォールバック分岐・定義マージ実値・scope 別採寸）＋実機目視サインオフ。
  - フォールバック発生・解決結果・失敗経路の観測可能なログ。
- **Out of scope**:
  - `\b[ID]` バルーン面切替経路の変更（`completed/areka-P0-balloon-face-cue` の完成領域・**無改変で緑維持**が受け入れ条件）。
  - `\p[2]` 以降専用系列（`balloonp{scope}def{ID}`）の解決とその 3 段フォールバック連鎖（M1 は二人立ちまで）。正典上、相方側系列 `balloonk*` が三人目以降も兼ねる。
  - バルーン面 ID の偶数／奇数＝左右向きセット意味論、および表示位置に応じた左右面の自動切替。
  - ghost descript の `balloon.defaultsurface` / `kero.balloon.defaultsurface` / `char*.balloon.defaultsurface` による初期表示面宣言への追従（emo2 は両キー無宣言＝既定 0 で現状差が出ない。語彙は互換対応表へ記録する）。
  - バルーンの表示／非表示ライフサイクル（可視条件・talk 終了後の自動 hide・再表示）＝`areka-P0-balloon-visibility`（W6）の領分。
  - 多面バルーンの面別上書き網羅（emo2 の各系列は面 0 の 1 枚のみ）。
  - 実行時のバルーン再読込（`\![reload,balloon]` 等）による系列再解決。
  - 入力ウィンドウ系列（`balloonc*`）・装飾系列（`arrow*` / `marker*` / `sstp*` 等）の相方側対応。
  - 表示スケール k の導出規約・丸め権威の変更（W4 `emo-dpi-scaling` の着地形を消費するのみ）。
  - キャラ窓の基準原点（下端中央）の変更。
  - `windowposition.x` のキーワード指定（`center` / `top` / `bottom`＝シェル中央上・中央下への固定）。数値指定のみを実装し、キーワードは語彙記録＋縮退シームとして残す。
  - `windowposition.limit`（バルーンを強制的に画面内へ維持する 0/1・正典既定 1）。現行はバルーン窓を非クランプで扱っており、本仕様でその方針を変えない（語彙記録の対象）。
  - バルーン位置の永続化・復元の規約（`completed/areka-P0-position-persist` の所有。本仕様は永続値が無いときの既定値の供給元を変えるのみ）。
- **Adjacent expectations**:
  - バルーン定義のパース（既定設定＋面別上書きの 2 層マージ）は `areka-parsers` に実装・檻化済みであり、本仕様はそれを **どのファイルで呼ぶか** を scope 別にするだけで、パーサ自体の改造を前提としない。
  - 表示スケール k の実値・丸め権威・再追従の起点は W4 `emo-dpi-scaling` の着地形をそのまま利用する。本仕様は「k を適用する対象がバルーン寸／定義において scope 別になる」ことのみを変える。
  - 境界候補（discovery の `Boundary Candidates`）: バルーン画像列挙（`crates/areka-emo-present/src/balloon.rs`）・起動時資産構築（`crates/areka/src/emo2_boot/assets.rs`）・表示結線（`crates/areka/src/emo2_boot/frame.rs`）・窓配置採寸（`crates/areka/src/placement/measure.rs`）・文字層再追従（`crates/areka-emo-text/src/actor.rs`）。
  - バルーン窓の初期既定位置の算出規則（基本位置＋オフセット）は既に配置解決層に存在し、そのオフセット供給欄は emo2 で未使用である。本仕様は**供給元を増やす**のであって、配置規則そのものを作り直さない。
  - **W5 同居 3 本**（`dpi-window-vanish` ∥ `collision-dpi-hittest` ∥ `choice-select-events`）とはファイル集合が互いに素であること。`dpi-window-vanish` は配置層を境界に掲げ、かつ診断やり直しにより編集集合が未確定であるため、本仕様が配置解決の中核（`placement/resolver.rs`）へ改造を要すると設計フェーズで判明した場合は、着手順を同ウェーブ内で裁定し干渉台帳へ登記する（エスケープ条項）。
  - **W6 への申し送り**: `balloon-visibility`（W6）は本仕様が確定させた scope 別バルーン定義の実形へ後着で再突合する。`bindoption-exclusivity`（W6）は同一ファイル内の別ハンクに着地するため、本仕様の先行着地後に相手が rebase する。
  - 正典が沈黙する箇所は areka 裁量で決定し、互換対応表（`doc/COMPAT_ARCHITECTURE.md`）へ記録する。

## Requirements

### Requirement 1: scope 別バルーン系列の解決とフォールバック

**Objective:** 既存の伺か資産を持ち込むユーザとして、バルーンが定義した相方側の枠（`balloonk*`）が相方に適用され、相方側の定義が無いバルーンでは従来どおり本体側の枠が使われることを求める。これにより、既存バルーン資産が正典どおりの見た目で動く。

#### Acceptance Criteria

1. When ゴースト起動時にバルーン資産を読み込むとき、the areka バルーン資産解決 shall scope ごとに第一候補の系列を割り当て、本体側 scope（`\0`）には `balloons{ID}` 系列を、相方側 scope（`\1` 以降）には `balloonk{ID}` 系列を割り当てる。
2. When 相方側 scope のある面 ID について `balloonk{ID}` の画像が存在しないとき、the areka バルーン資産解決 shall 同一 ID の本体側画像 `balloons{ID}` を当該面の代替として採用する。
3. The areka バルーン資産解決 shall 前項のフォールバックを**面 ID 単位**で判定し、ある ID の欠落を理由に当該 scope の系列全体を本体側へ切り替えない（`balloonk0` があり `balloonk1` が無い場合、面 0 は相方側・面 1 は本体側となる）。
4. Where バルーンが `balloonk*` 画像を 1 枚も含まないとき、the areka バルーン資産解決 shall 全 scope に本体側系列を割り当て、本仕様適用前と同一の面集合を得る。
5. The areka バルーン資産解決 shall 正典でバルーン面系列と定義されていないファイル（入力ウィンドウ用 `balloonc*`・装飾用画像等）をバルーン面として採用しない。
6. While M1 の二人立ち構成であるとき、the areka バルーン資産解決 shall scope 1 以降を相方側系列として扱い、`\p[2]` 以降専用系列（`balloonp{scope}def{ID}`）の解決は行わない。
7. If ある scope について面 ID 0 のバルーン面が 1 つも解決できないとき、then the areka バルーン資産解決 shall 失敗理由をエラーレベルでログに記録したうえでエラーを返し、既存の失敗経路（バルーン未配線・ダミー窓への縮退を含む）へ伝播させ、無言で空のバルーンを表示するログ無し経路を作らない（プロセス終了ポリシー自体の変更は本仕様の対象外）。

### Requirement 2: scope 別バルーン定義（面別上書き）の適用

**Objective:** ゴースト作者として、相方側バルーンの `windowposition` や `validrect` が `balloonk{ID}s.txt` の記述どおりに効くことを求める。これにより、相方のバルーンを本体とは別の位置・別のテキスト範囲で表示できる。

#### Acceptance Criteria

1. When scope ごとの面が解決したとき、the areka バルーン定義 shall バルーン既定設定（`descript.txt`）へ当該 scope が採用した面の面別上書き設定をマージした定義を、その scope 専用の定義として保持する。
2. When 相方側 scope が `balloonk{ID}` を採用したとき、the areka バルーン定義 shall `balloonk{ID}s.txt` の `windowposition` および `validrect` を当該 scope へ適用し、本体側 `balloons{ID}s.txt` の値を適用しない。
3. When ある scope の面が ID 単位フォールバックにより本体側画像へ解決されたとき、the areka バルーン定義 shall 同一 ID の本体側面別上書き設定を当該面の上書き層として用いる（正典は面別上書きを「対応する ID のサーフェスに対して」適用すると定めるため、本体側画像へ縮退した面には本体側の上書き層が対応する＝正典の帰結。Requirement 7.4 により解釈として対応表へ記録する）。
4. When 採用した面に対応する面別上書きファイルが存在しないとき、the areka バルーン定義 shall バルーン既定設定の値のみを用い、その欠落を失敗として扱わない。
5. The areka バルーン定義 shall 面別上書き設定で指定されなかった項目についてバルーン既定設定の値を継承し、指定された項目のみを上書きする。
6. While バルーンを表示しているとき、the areka shall 各 scope のバルーンの初期表示面を当該 scope が解決した系列の面 ID 0 とする（ghost descript の `balloon.defaultsurface` / `kero.balloon.defaultsurface` 宣言への追従は Out of scope＝正典既定値 0 のみを実装し、Requirement 7.4 により語彙を対応表へ記録する。左右向きの偶奇規約は本仕様で導入しない）。

### Requirement 3: 窓配置採寸と初期既定位置の scope 別化

**Objective:** ユーザとして、相方のバルーン窓が相方自身の枠寸と `windowposition` に基づいて配置されることを求める。これにより、相方のバルーンが本体側の寸法前提でずれて表示されることがなくなり、バルーン作者が相方専用に指定した表示位置が実際に効く。

#### Acceptance Criteria

1. When ゴースト窓の配置採寸を行うとき、the areka 窓配置採寸 shall 各 scope が解決したバルーン面 0 の実寸をその scope のバルーン寸として採り、全 scope 共通の 1 回の採寸へ畳まない。
2. When バルーン窓の初期既定位置を決定するとき、the areka 窓配置 shall 当該 scope の定義の `windowposition`（数値指定）を**基本位置からの調整量**として適用し、基本位置は現行と同一（バルーンをキャラ窓の隣接辺へ置き、バルーン上端をキャラ画像上端へ揃えた位置）とする。
3. The areka 窓配置 shall `windowposition.x` を「シェル側が正」として解釈し、当該 scope のバルーンがキャラ窓のどちら側に置かれるかに応じて画面座標系の符号へ変換する。`windowposition.y` は「下が正」＝画面座標系と同符号ゆえ変換しない。
4. Where 当該 scope の定義に `windowposition` の数値指定が無いとき、the areka 窓配置 shall 正典既定値 0 を用い、本仕様適用前と同一の初期既定位置を与える。
5. While 永続化されたバルーン相対位置が存在するとき、the areka 窓配置 shall 永続値を優先し、`windowposition` の適用を初期既定位置の供給にとどめる（保存・復元の既存規約と優先順位を変更しない）。
6. When 表示スケール係数 k が 1.0 以外であるとき、the areka 窓配置採寸 shall 各 scope のバルーン寸および `windowposition` 由来の調整量に対し既存の k 適用規約と丸め権威をそのまま適用し、本仕様で新たな丸め規約を導入しない。
7. While 全 scope が同一のバルーン面へ解決される（`balloonk*` 不在時）とき、the areka 窓配置採寸 shall 全 scope に同一のバルーン寸を与え、本仕様適用前の採寸結果と一致させる。
8. The areka 窓配置採寸 shall キャラ窓の基準原点（下端中央）およびバルーン相対位置の保存・復元の基準を変更しない。

### Requirement 4: バルーン文字層の scope 別追従

**Objective:** ユーザとして、相方のバルーン内の文字が相方の `validrect` の内側に、DPI 変化後も正しい寸で描画されることを求める。これにより、枠だけが相方用で文字範囲が本体用という不整合が起きない。

#### Acceptance Criteria

1. When バルーン表示面へ文字層を装着するとき、the areka バルーン文字層 shall 当該 scope の定義（Requirement 2）を用いて文字描画領域を解決する。
2. The areka バルーン文字層 shall 装着時と表示スケール再追従時で同一の scope→文字層アクタ写像を用い、一方だけが別のアクタを指す状態を作らない。
3. When 表示スケール係数 k が変化したとき、the areka バルーン文字層 shall 当該 scope の定義とその時点の面実寸で文字層の描画資源を再構築し、旧寸の文字供給面を残さない。
4. When 適用中の k が同値のまま当該 scope のバルーン面の実寸または文字描画領域が変化したとき、the areka バルーン文字層 shall 文字層を再構築する（k の同値のみを根拠に再追従を省略しない）。
5. While 適用中の k・面実寸・文字描画領域のいずれも変化していないとき、the areka バルーン文字層 shall 再構築を行わず、毎フレームの再結線を発生させない。
6. If 再追従の対象となる文字層アクタが未装着であるとき、then the areka バルーン文字層 shall 何も行わずその旨をログに記録する（装着経路を二重化しない）。
7. The areka バルーン文字層 shall 文字のリビール進行・確定行などの純粋状態を再構築によって失わない。

### Requirement 5: 既存挙動の非回帰

**Objective:** 保守者として、完成済みの `\b` 面切替とバルーン可視性の挙動が本仕様で壊れないことを求める。これにより、隣接する完成領域・後続仕様の前提が保たれる。

#### Acceptance Criteria

1. When `\b[ID]` によるバルーン面切替が発行されたとき、the areka shall その ID を当該 scope が解決した系列内の面 ID として解釈し、切替および `\b[-1]` による非表示の既存挙動を変えない。
2. The areka shall 既存の `\b` 面切替回帰檻（非表示→再表示の発行順序と再表示後の描画一致、およびバルーン宛指令が漏れないこと）を緑のまま維持する。
3. The areka shall バルーンの表示／非表示ライフサイクル（可視条件・自動 hide・再表示）を本仕様で変更しない。
4. When バルーンが `balloonk*` 画像を含まないとき、the areka shall 表示・採寸の結果および解決される面集合を本仕様適用前と同一に保つ（初期既定位置は Requirement 3.2 による正典化の対象であり、この同一性の範囲外）。
5. The areka shall 本体側 scope の表示・採寸・文字描画を本仕様の前後で同一に保ち、配置については永続値が存在する限り同一に保つ（本体側で変化し得るのは `windowposition` を反映した初期既定位置のみ）。
6. The areka shall バルーン側の面テーブル（面の集合およびアニメーション／ループ定義）を各 scope が解決した系列と整合させ、ある scope のバルーンが別 scope の系列由来の定義で駆動される状態を作らない。

### Requirement 6: 解決結果とフォールバックの観測性

**Objective:** 実機サインオフを行う開発者として、どの scope がどの系列・どの面ファイルを採用したかがログから確定できることを求める。これにより、目視で違和感があったときに原因を配線かバルーン資産かへ即座に切り分けられる。

#### Acceptance Criteria

1. When scope 別のバルーン系列解決が完了したとき、the areka shall 各 scope が採用した系列と面 ID の解決結果をログに記録する。
2. When ある scope の面が ID 単位フォールバックにより本体側へ縮退したとき、the areka shall 縮退の事実（scope・面 ID・採用ファイル）を警告レベルでログに記録する。
3. When scope 別のバルーン定義から `windowposition` / `validrect` が確定したとき、the areka shall その実値を scope とともにログに記録する。
4. If バルーン資産の読み取りまたは解決に失敗したとき、then the areka shall 失敗理由をエラーレベルで記録したうえでエラーを返し、ログの無い失敗経路を作らない。

### Requirement 7: 検証と正典整合の記録

**Objective:** 開発者として、判定分岐が決定論的に檻化され、かつ実機で相方側バルーンの違いが目視確認されることを求める。これにより、檻が隠す欠陥（実機でのみ現れる配置ずれ）を残したまま完了と宣言しない。

#### Acceptance Criteria

1. The 開発チーム shall scope→系列選択・ID 単位フォールバック分岐・scope 別定義マージ（相方側 `windowposition` / `validrect` の実値）・scope 別採寸・文字層再追従の判定分岐を、決定論的な自動テストで網羅的に檻化する。
2. When 全 scope 共通のバルーン寸・全 scope 共通のバルーン定義を前提とする既存テストが本仕様と矛盾するとき、the 開発チーム shall 当該テストおよびその前提を述べる doc コメントを本仕様の挙動に合わせて更新する（矛盾するテスト・陳腐化した注記を放置しない）。
3. When 実機サインオフを行うとき、the 開発チーム shall ゴースト emo2 とバルーン emo2-kakukaku を絶対パスで起動し、本体側と相方側のバルーンの枠形状および表示位置が互いに異なることを目視で確認したうえで、Requirement 6 のログにより各 scope の採用系列・採用ファイル・確定寸が相異なることを突合する。
4. Where 正典が沈黙する箇所、または正典条文の解釈により areka の挙動を決定した箇所（`\b[ID]` を系列内 ID とみなす解釈、フォールバック時の面別上書き層の対応、未実装項目の縮退等）があるとき、the 開発チーム shall その決定と根拠区分（正典整合／areka 裁量）を互換対応表（`doc/COMPAT_ARCHITECTURE.md`）へ記録する。
5. When 本仕様の完了を判定するとき、the 開発チーム shall ワークスペース全体のテストが緑であることを確認する。
6. When `windowposition` の x 方向の基本位置の解釈を確定するとき、the 開発チーム shall 実機表示で確認したうえで確定する（正典は y 方向の基本位置のみ明示し、x 方向の基本位置を明示していない）。確定した解釈は Requirement 7.4 により対応表へ記録する。
