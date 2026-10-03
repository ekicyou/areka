# Requirements Document

## Project Description (Input)

**誰の問題か**: これから文字とバルーンの列（`shell-balloon`・`balloon-font-file`・`text-typesetting`・`talk-fast-forward`・`text-ruby`・`balloon-markers`・`balloon-scroll-fade`・`text-reveal-fade`・`text-align-shadow-canon`・`choice-marker-styling`・`anchor-tag-canon`・`balloon-lifecycle-events`）に着手する実装者とレビュアー。

**現状**: バルーンの文字まわりのソースに、1 ファイル 1,000 行の上限（番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`）に張り付いたファイルが 6 本ある。どれも番人の例外の表に無いので、1,000 行を超えた時点で全体テストが赤になる。後続の spec はどれもこの 6 本のどれかに行を足すが、各 brief は「分割が先」と書くだけで、誰がどう切るかを決めていない。任せると同じファイルを別々の spec が違う形で切り、互いの移動に巻き込まれる。テストの登録の 2〜3 行を足すだけでも上限に当たる。

**何が変わるべきか**: 6 本を、振る舞いを 1 つも変えずに役割の単位で分け、後続の spec が足す余地（目安 700 行以下）を作る。既存のテストは書き換えず、公開している名前の道筋も変えない。分けた先のモジュールには、どの spec がそこへ足すかを 1〜2 行で残す。前例は完了 `areka-P0-file-slimming`（振る舞いを変えない・テストは兄弟ファイル・`#[path]` で子モジュール）。

## Introduction

本 spec は、文字とバルーンのソース 6 本の**置き場所だけ**を変える。動かすのはコードの場所であって、コードの中身でも利用者に見える振る舞いでもない。受け入れの中心は 3 つ——**振る舞いが変わらない**・**既存のテストが書き換えなしで同じ本数・同じ結果**・**公開している名前の道筋が変わらない**——で、この 3 つを証跡で示せない変更は本 spec の成果物ではない。

対象の行数は、要件定義の時点（main `1ce4c74e`）で測り直した。brief の起票時（main `03e8d7d6`）の値と 7 本とも同じである。

| ファイル | 行数 | 足す予定の spec（brief より） |
|---|---:|---|
| `crates/areka-emo-text/src/actor.rs` | 975 | `shell-balloon`・`balloon-font-file`・`text-reveal-fade`・`balloon-markers`・`anchor-tag-canon` |
| `crates/areka-emo-text/src/layout.rs` | 977 | `text-typesetting`・`text-ruby`・`balloon-markers`・`text-align-shadow-canon` |
| `crates/areka-emo-text/src/viewbox_draw.rs` | 914 | `text-typesetting`・`text-reveal-fade`・`balloon-font-file`・`choice-marker-styling`・`anchor-tag-canon` |
| `crates/areka-emo-text/src/viewbox.rs` | 871 | `balloon-markers`・`text-reveal-fade`・`balloon-scroll-fade` |
| `crates/areka/src/input_events/balloon.rs` | 930 | `talk-fast-forward`・`balloon-markers`・`shell-balloon`・`anchor-tag-canon`・`balloon-lifecycle-events` |
| `crates/areka/src/emo2_boot/balloon_visibility.rs` | 923 | `shell-balloon`・`balloon-lifecycle-events` |
| `crates/areka-emo-text/src/region.rs`（任意） | 977 | 本体 488 行＋内蔵テスト 482 行（489〜970 行目の `#[cfg(test)] mod tests { … }`）。`shell-balloon` は本体を変えずに済む見込み |

> 注（要件定義時の再測定による是正）: brief は「`layout.rs` の 343〜765 行目が 1 本の関数 `layout_with_cursor_warn`（約 420 行）」と書くが、実際は公開の入口 `layout_with_cursor_warn`（343〜377 行目・約 35 行）と、私有の本体 `layout_inner`（378〜765 行目・約 388 行）の 2 本である。「その大きい関数を割るかどうか」は brief のとおり設計で決める（本書は割る・割らないのどちらにも中立）。

要件定義の時点で、**ソースの字面を読んで判定する既存の構造テストが 4 つ**、この 6 本の中身を名指ししていることを確かめた（Requirement 5）。分割でコードが動くと、これらは赤になるか、黙って見る範囲が縮む。brief の「触るのは 6 本と新しいファイルとモジュールの宣言だけ」には含まれていないため、本書で扱いを定める。

## Boundary Context

- **In scope**: 上の 6 本の、振る舞いを変えない分割／`use` とモジュールの宣言の付け替え／分けた先のモジュールの doc に 1〜2 行／分割で動いたコードに合わせた、既存の構造テストの「読むファイルの一覧」の追随（Requirement 5）／`crates/areka-emo-text/src/lib.rs` の doc のうち子モジュールの一覧を述べる段落の更新／（任意）`region.rs` の内蔵テストの兄弟ファイルへの移設。
- **Out of scope**: 振る舞いの変更すべて／キーや欄の追加／`crates/areka-emo-text/src/state.rs` と `crates/areka-parsers/src/balloon/` の作り替え／既存のテストファイルの分割／1,000 行に遠いファイルの分割／`crates/areka/src/main.rs`（946 行・射程内だが本 spec の対象外）／他の spec の brief やコード中の注釈に書かれた file:line の書き換え。
- **Out of boundary**: 文字まわりの spec が共有する表（`CueCommand::Custom` の腕・`BalloonModel` の欄・`TextLayerRuntime` の欄）を、並走できる形へ作り替えること。分割してもこの共有は残る。**文字まわりの spec は本 spec の後も互いに直列**である。
- **Adjacent expectations**: 同じウェーブ C1 の他の spec が触る場所——`crates/areka/src/emo2_boot/frame/`・`crates/areka/src/placement/`・`crates/areka/src/install/`・`crates/areka/src/main.rs`・`crates/areka-sakura/`・`crates/wintf/`・`crates/areka-kanade/`・`crates/areka/src/emo2_boot/user_break_cue.rs`・`crates/areka/src/emo2_boot/balloon_visibility_phase.rs`・`crates/areka/src/update/`——には触らない。後続の spec（直後は `shell-balloon`）は、本 spec が着地した後の形の上で、自分の design の前に rebase して作業する。全体テスト（`tools/test-all.ps1`）は i686 の host-32 成果物が用意されていることを前提とする（既存の前提であり本 spec は変えない）。

## Requirements

### Requirement 1: 6 本の分割と後続が足す余地

**Objective:** As a 文字とバルーンの列の実装者, I want 上限に張り付いた 6 本が役割の単位で分かれていて、どこへ足せばよいかがモジュールの doc から分かる, so that 自分の spec がテストの登録の数行を足しただけで上限に当たらず、別の spec と違う形で同じファイルを切り合わずに済む

#### Acceptance Criteria

1. When 分割が終わったとき, the file-split 実装 shall 上の表の 6 本と、分割で新しく作ったファイルのそれぞれを 700 行以下を目安に収める。
2. If 役割の切れ目を守るために 700 行を超えるファイルが残るとき, the file-split 実装 shall そのファイルの行数と、後続の spec が足す余地をどう確保したかを設計に記録する。どのファイルも 1,000 行の上限は超えない（目安の 700 行は強制値ではないが、上限の 1,000 行は強制値である）。
3. The file-split 実装 shall 6 本を役割の単位で分け、行数を揃えるための機械的な切り方（同じ役割を行数の都合で 2 つに割る・違う役割を行数合わせで 1 つにまとめる）をしない。切れ目の出発点は brief の Approach（`actor.rs`＝実行時の状態／1 コマの描画の流れ／指令の振り分け、`layout.rs`＝配置の本体／見える範囲の計算／仕上げ、`viewbox.rs`・`viewbox_draw.rs`＝どこを描き直すかの計画／装飾つきの描画、`input_events/balloon.rs`＝選択肢のクリック／中断のダブルクリック／ドラッグ、`emo2_boot/balloon_visibility.rs`＝見える・隠すの判断／時間切れ／窓への反映）とし、最終的な切れ目は設計で確定する。
4. The file-split 実装 shall 分割で新しく作ったモジュールそれぞれの doc に、そのモジュールの役割と、そこへ行を足す予定の後続の spec の名前を 1〜2 行で書く。spec は名前で書き、台帳の番号は使わない。
5. When 分割が終わったとき, the file-split 実装 shall 子モジュールの一覧を述べている既存の doc（`crates/areka-emo-text/src/lib.rs` 冒頭の層規律の段落など）を、分けた後の実際の子モジュールと食い違わない状態にする。

### Requirement 2: 振る舞いを変えない

**Objective:** As a レビュアーと利用者, I want 分割の前後でゴーストの見た目・反応・ログの文言が 1 つも変わらない, so that 大きな移動の差分を、中身を読み直さずに受け入れられる

#### Acceptance Criteria

1. The file-split 実装 shall 分割の前後で、バルーンの文字の描画・選択肢のクリックとホバー・ダブルクリックでの中断・バルーンのドラッグ・バルーンの表示と非表示と時間切れの振る舞いを変えない。
2. The file-split 実装 shall コードを項目（関数・型・定数・`impl` の塊）の単位で動かし、項目の中身を書き換えない。許す差分は、`use` とモジュールの宣言、移動に伴って必要になる crate の内側に閉じた可視性の付与、Requirement 1.4 の doc の行、意味の変わらない整形の折り返しだけとする。
3. Where 設計が `layout.rs` の大きい関数（公開の入口 `layout_with_cursor_warn` とその本体 `layout_inner`）を複数の関数へ割ると裁定したとき, the file-split 実装 shall Requirement 2.2 の例外をその関数だけに限り、割った理由を設計に記録し、割った後も既存のテストが Requirement 4 のとおり書き換えなしで緑であることで振る舞いが同じであることを示す。設計が割らないと裁定したときは、Requirement 2.2 の例外は 0 件である。
4. The file-split 実装 shall ログの文言・レベル・出る条件を変えない。ログの発生元の名前（モジュールの道筋）は、移動に伴って元の名前を頭に持つ子の道筋へ変わることだけを許し、`RUST_LOG` の前置きの絞り込み（例 `areka_emo_text::actor=debug`）が分割前と同じ行を拾うことを保つ。
5. If 発生元の名前の完全一致でログを判定している箇所が、移動するログの発生元に掛かっていると設計で分かったとき, the file-split 実装 shall そのログを出す項目を動かさずに元のモジュールへ残す。要件定義の時点で、6 本の発生元の名前を完全一致で判定する箇所は 0 件である。
6. The file-split 実装 shall 依存（`Cargo.toml`・`Cargo.lock`）を 1 行も変えない。
7. When `cargo build` を実行したとき, the areka ワークスペース shall 分割の前に比べて警告の件数を 1 件も増やさない。

### Requirement 3: 公開している名前の道筋を変えない

**Objective:** As a 6 本を呼んでいる他のファイル・他の crate・後続の spec, I want 分割の後も今と同じ道筋で同じ名前に届く, so that 呼び出し側を 1 行も直さずに済み、後続の spec の brief に書かれた名前もそのまま通じる

#### Acceptance Criteria

1. When 分割が終わったとき, the file-split 実装 shall 6 本が分割前に `pub`・`pub(crate)`・`pub(super)` で公開していたすべての名前に、分割前と同じ道筋と同じ可視性で届く状態を保つ。動かした名前は、元のモジュールからの再輸出で今の道筋を残す。
2. The file-split 実装 shall 分割で新しく作った子モジュールを、元のモジュールの外から見えない形で宣言し、新しい公開の道筋を 0 本に保つ。
3. The file-split 実装 shall 名前を呼んでいる側の本番のコード（6 本と分けた先の新しいファイル以外のモジュール・他の crate）と、examples・統合テスト（`tests/` の下）の変更を 0 行にする。6 本に接続されている兄弟のテストファイルの `use` の付け替えは Requirement 4.3 が、構造テストの追随は Requirement 5 が扱う。
4. The file-split 実装 shall `crates/areka/src/emo2_boot/balloon_visibility.rs` の子モジュール `balloon_visibility_phase.rs` が親から引いている名前に、分割前と同じ書き方で届く状態を保ち、`balloon_visibility_phase.rs` の変更を 0 行にする。

### Requirement 4: 既存のテストを書き換えず、同じ本数・同じ結果

**Objective:** As a レビュアー, I want 分割の前後で全体テストが同じ本数・同じ結果で、テストの判定が 1 つも書き換えられていないことを証跡で確かめられる, so that 「テストを直して緑にした」のではなく「振る舞いが同じだから緑」だと言える

#### Acceptance Criteria

1. When 分割の前と後でそれぞれ全体テスト（`tools/test-all.ps1`）を走らせたとき, the areka ワークスペース shall 同じ本数のテストが走り、各テストの結果（成功・失敗・無視）が前後で同じになる。
2. The file-split 実装 shall 分割前と分割後の全体テストの本数と結果を証跡として採り、両者の一致を示す。証跡の無い「全緑」の申告は完了の根拠にしない。
3. The file-split 実装 shall 既存のテストの関数・判定・入力・期待値・属性・注釈を書き換えない。書き換えるテストは 0 本である。許す変更は、`use` とモジュールの道筋の付け替え、および Requirement 5 が定める構造テストの「読むファイルの一覧」の追随だけとする。
4. The file-split 実装 shall テストを 1 本も足さず、1 本も消さない（足すテスト 0 本・消すテスト 0 本）。
5. If 分割で既存のテストがコンパイルできなくなったとき, the file-split 実装 shall テストの判定を書き換えるのではなく、元のモジュールでの名前の束ね直し（可視性を上げない素の `use`）か、テスト側の `use` の付け替えで解く。
6. The file-split 実装 shall 既存のテストファイルを分割しない。

### Requirement 5: ソースの字面を読む構造テストの追随

**Objective:** As a 層規律や走査点の数を見張っている既存の構造テストの持ち主, I want 分割でコードが動いても、見張りが同じ約束を同じ範囲で見続ける, so that テストが赤になることも、見る範囲が黙って縮んで素通りすることも起きない

#### Acceptance Criteria

1. The file-split 実装 shall 要件定義の時点で確かめた次の 4 つの構造テストを、分割でコードが動いた後も同じ約束を判定する状態に保つ。
   - `crates/areka-emo-text/src/lib.rs` の層規律の見張り（`windows` 依存が純粋層に無いことを、純粋層のファイルの一覧で読んで判定する。`src/*.rs` の実ファイルがどちらかの一覧に必ず載っていることも突き合わせる）
   - `crates/areka-emo-text/src/layout_cursor_overflow_tests.rs` の「行を閉じる入口の数」（`layout.rs` の字面で `finish_line(` と `finish_pending_line(` を数え、`finish_pending_line` が `layout.rs` の私有関数であることを判定する）
   - `crates/areka-emo-text/src/layout_styled_tests.rs` の「行送りの式へ届く点は 1 つだけ」（配置層の 3 ファイルの字面を読む）
   - `crates/areka/src/emo2_boot/frame_attach_tests.rs` の「状態表の走査点の登記」（`crates/areka-emo-text/src/actor.rs` を走査点の 1 つとして登記している）
2. When 分割で動いたコードが、構造テストが読んでいたファイルから別のファイルへ移ったとき, the file-split 実装 shall そのテストの「読むファイルの一覧」（一覧そのもの・一覧の件数を固定する数・登記の道筋）を、移った先を含むように追随させ、分割後に読む範囲が分割前に読んでいたコードをすべて含むようにする。
3. The file-split 実装 shall 構造テストの判定と期待値（数える回数・判定する性質）を変えない。
4. If 追随させるには構造テストが保証する性質そのもの（例: 「`finish_pending_line` は `layout.rs` の私有関数で、呼び出し元が同じファイルに閉じている」）を変える必要があるとき, the file-split 実装 shall その項目を動かさずに元のファイルへ残す。
5. When 分割で `crates/areka-emo-text/src/` に新しいファイルを足したとき, the file-split 実装 shall そのファイルを層規律の見張りの 2 つの一覧（純粋層として読む一覧・読まない一覧）のどちらかへ、親のモジュールと同じ層として載せる。
6. The file-split 実装 shall 設計の時点で、上の 4 つ以外に 6 本（と任意の `region.rs`）の中身を字面や道筋で名指ししている見張りが無いかをリポジトリ全体で数え直し、見つかったものも Requirement 5.2〜5.4 と同じ扱いにする。
7. The file-split 実装 shall brief の「触るファイル」の外で変更したファイル（上の構造テストのファイルを含む）を設計に一覧で示し、その一覧に Boundary Context の「同じウェーブの他の spec が触る場所」が 1 つも含まれないことを示す。

### Requirement 6: 番人の例外の表と、触ってはいけない場所

**Objective:** As a 同じウェーブ C1 で並走する spec と、1,000 行の番人, I want 本 spec が番人の例外を増やさず、他の spec の持ち場に 1 行も触れない, so that 並走しても互いの差分がぶつからず、上限の約束が暗黙に緩まない

#### Acceptance Criteria

1. The file-split 実装 shall 番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外の表（`OVER_LIMIT_ALLOWED`）と件数の定数（`OVER_LIMIT_ALLOWED_COUNT`）を 1 行も変えない。
2. When 分割が終わったとき, the areka ワークスペース shall 番人のテストを緑で通す（1,000 行を超えるファイルが例外の表の外に 0 本）。
3. The file-split 実装 shall Boundary Context の「同じウェーブの他の spec が触る場所」の変更を 0 行にする。
4. The file-split 実装 shall `crates/areka-emo-text/src/state.rs` と `crates/areka-parsers/src/balloon/` の変更を 0 行にする。

### Requirement 7: `region.rs` の内蔵テストの移設（任意）

**Objective:** As a `region.rs` を次に触る spec の実装者, I want 本体と同じファイルに同居している約 480 行のテストが兄弟ファイルに出ている, so that 本体に行を足しても上限に当たらない

#### Acceptance Criteria

1. The file-split 実装 shall `region.rs` の内蔵テストを移すかどうかを設計で裁定し、裁定とその理由を設計に記録する。
2. Where 設計が移すと裁定したとき, the file-split 実装 shall `region.rs` の本体を 1 行も動かさず、`#[cfg(test)] mod tests { … }` の中身だけを同じディレクトリの兄弟ファイルへ移し、`region.rs` には接続の宣言だけを残す。
3. Where 設計が移すと裁定したとき, the file-split 実装 shall 移したテストの中身を、行頭の字下げの変化を除いて 1 文字も変えない（空白を無視した比較で一致する）。Requirement 4 の本数と結果の一致も同じく満たす。
4. Where 設計が移さないと裁定したとき, the file-split 実装 shall `region.rs` の変更を 0 行にする。

### Requirement 8: 実機での確かめ

**Objective:** As a 開発者, I want 分割の後のビルドで実際のゴーストを 1 回起こし、今日と同じに動くことを確かめる, so that テストに映らない配線の取り違えが残っていないと言える

#### Acceptance Criteria

1. When 分割が終わったとき, the file-split 実装 shall 分割後のビルドで emo2 を起こし、会話の文字の表示・選択肢のホバーとクリック・ダブルクリックでの中断・バルーンの表示と非表示が分割前と同じに動くことを 1 回確かめ、その結果をログと一緒に記録する。
2. If 実機の確かめで分割前と違う振る舞いが見つかったとき, the file-split 実装 shall 完了とせず、原因を分割の差分の中で特定して直す。
3. The file-split 実装 shall 実機の確かめに使う一時の置き場を、ワークツリーの `target\` の下だけに置く。
