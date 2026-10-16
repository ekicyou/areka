# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: areka の開発者と、実機で確かめをするセッション。全体テスト（`tools/test-all.ps1`）を回した後に `target\debug\areka.exe` で 32bit の SHIORI のゴースト（例: claudia の `yaya.dll`）を起こすと、SHIORI を読めずに落ちる（`0x800700C1`＝形式が違う）。毎回この罠を踏み、落ちた理由もログからは分からない。

**今の状態**: areka は `areka.exe` の隣の `shiori-host32-helper.exe` を 32bit の SHIORI の受け口として起こす。全体テストの後は、この隣の helper が x64 版になっていることがある（i686 版は `target\i686-pc-windows-msvc\debug\` にしか無い）。煙テストなどは自分で i686 の helper を `areka.exe` の隣へ揃えるが、手で起こす実機には揃える仕組みが無い。2026-10-06 の実機の確かめでは、`areka.exe` と i686 の helper を同じフォルダへ写して避けた。

**何を変えるか**: ① 全体テストの後でも、開発者が `target\debug\areka.exe` をそのまま使って 32bit の SHIORI のゴーストを起こせるようにする。② helper の向きが違うときは、落ちる前に「helper が 32bit 版でないので 32bit の SHIORI を読めない」と分かる記録が 1 行出るようにする（ログ無しの失敗にしない）。

> 起票: 2026-10-07 `ghost-standard-balloon` の完了時の棚卸（同 spec の tasks.md の Implementation Notes 6.2・最終検証 ⑶）。再測定: 2026-10-10 棚卸㉓・2026-10-16 棚卸㉕（ウェーブ C7 の空き席）。要件生成: 2026-10-16（main `1e6590d8`）。詳細は brief.md。

## Introduction

本仕様は、開発の手順の罠を 2 つの面から塞ぐ。1 つは全体テストのスクリプトの後始末（終わったときに `areka.exe` の隣の helper が i686 版であること）、もう 1 つは areka 自身の知らせ（隣の helper が 32bit 版でないときに、理由の分かる 1 行を残すこと）。前者で全体テストの後の罠を消し、後者で全体テスト以外の操作（開発者がワークスペースの根で素の `cargo build` を打ったときなど）で同じ状態になっても、理由がログから 1 行で分かるようにする。配布物は helper が常に i686 版なので、製品の振る舞いは変えない（区分 C＝開発の手順の罠）。

要件生成時に引き直した事実（設計はこれを再検証する）:

- `tools/test-all.ps1`（70 行）の段の順は、i686 のターゲットの導入 → i686 の成果物（helper・偽 DLL 2 つ）のビルド → （`-Format` のとき整形）→ fmt の検査 → x64 のワークスペース全テスト（`cargo test --workspace --no-fail-fast -j 4`）→ i686 でしか走らないテスト（helper と ipc）→ 公開前の確認 → 文字コードの判定 → リリースノートの道具の自分の検査 → 俯瞰の HTML の道具の自分の検査 → （`-License` のとき 2 段）。段が赤でも止めずに最後まで回し、段ごとの合否の一覧（ASCII）を出して、1 つでも赤なら終了コード 1 で終わる。
- helper の置き場は `crates/areka/src/boot_config.rs` の `default_helper_exe_path`（起動した exe の本当の場所の親＋`shiori-host32-helper.exe`）。helper を起こすのは `crates/shiori-host32-host/src/process_host.rs` の `spawn`、つなぐ手順は `crates/areka-ghost/src/shiori_wiring.rs` の `real_connect`（起こす → HELLO を待つ → LOAD の ack が `[1]` か確かめる）。本番の areka には helper の場所を外から指す口が無い（テストの側だけ `HOST32_HELPER_EXE` を読む）。
- x64 版の helper が 32bit の DLL を読もうとすると、helper の中で LOAD が失敗し（helper の標準エラーへの観測のみ）、areka の側には「SHIORI の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: [0]」の形の文だけが残る。helper の向きが原因だとは書かれない。
- 煙テスト（`crates/areka/tests/smoke_boot_loop_exit.rs` の `ensure_i686_artifact`）は、x64 の全テストの段の中で、隣の helper が i686 版でなければ `target\i686-pc-windows-msvc\{debug,release}\` から写して置く。i686 かどうかは実行ファイルの見出しの機械の種類で見分ける（`is_i686_pe`）。
- helper のクレートには結合テストのフォルダが無く、ほかのクレートが helper を依存に持つことも無い。ギャップ分析（research.md 3 節）で静的に確かめた結果、全体テストの x64 の段は helper を `target\debug\` へ出さず、x64 版を隣へ置くのはワークスペースの根での素の `cargo build`（`default-members` が無いので全クレートが対象）である。作り直しが要らないときも出力を結び直すので、全体テストの後に一度 `cargo build` を打つと、写した i686 版が x64 版へ戻る（実測は設計の最初のタスクで行う）。下の要件は「どの操作で x64 版になるか」に依らない形で書く。
- PR#258 で、helper の約 42 秒かかるテスト（応答が返らない SHIORI を待ち切るテスト）は i686 の段だけで走る形になった（`crates/shiori-host32-helper/src/main.rs` のそのテストの `cfg`）。helper の単体テストのうち 32bit の DLL を読むものは x64 では理由付きで飛ばされ、i686 の段でだけ走る。

## Boundary Context

- **In scope**:
  - 全体テストのスクリプトが終わったときの、`target\debug\` の `areka.exe` の隣の helper の向き（i686 版であること）と、揃えられなかったときの知らせ。
  - areka が隣の helper を起こすときに、その helper が 32bit 版でないと分かる記録を 1 行出すこと。
  - `README.md` と `.kiro/steering/tech.md` のビルドの手順を `cargo build -p areka` に替えること（罠の入口である素の `cargo build` を手順から外す）。
- **Out of scope**:
  - 配布物の helper の置き方（`tools/package.ps1`・すでに i686 版）と、arm64 の helper。
  - 32bit の SHIORI の受け口の仕組みそのもの（areka と helper のやりとり・LOAD の手順・失敗の扱い）。
  - helper の置き場の決め方（`areka.exe` の隣）。別の場所を探しに行くこと・外から helper の場所を指す口を足すことはしない（brief の案 ⑶ の「開発時だけの探し方」は取らない。同じウェーブの `baseware-root-list` と重なるため）。
  - `target\release\` の helper（全体テストは debug だけを作る）。
  - 全体テスト以外の操作で開発者が作った x64 版の helper を、自動で置き換えること（記録で知らせるところまで）。
  - 根の `Cargo.toml` に `default-members` を置いて素の `cargo build` から helper を外すこと（32 クレートを並べ直す保守と、書き忘れたクレートが黙って抜ける危うさのため取らない）。
  - 全体テストの x64 の段から helper を外すこと（x64 の段はもともと隣の helper を上書きしないので目的に効かない。時間の短縮だけのための変更は混ぜない）。
- **Adjacent expectations**:
  - 上流: フルテストの速度調整（PR#258・spec なし）。helper の約 42 秒かかるテストを i686 の段だけで走らせる形は保つ。
  - 同じウェーブ C7 の `baseware-root-list`: helper の置き場を決める関数のファイル（`boot_config.rs`）を触る。本仕様はそのファイルの振る舞いを変えない。
  - `clippy-199-lints`: clippy の段を足すと決めたときは同じ `tools/test-all.ps1` を触る（ファイルが重なる）。
  - 下流: 実機の確かめをするすべての spec（`mcp-receiver-bugs` の件 2＝実機で 32bit のゴーストを起こす、など）。全体テストの後に、写す手間なしで `target\debug\areka.exe` を使える前提になる。
  - `release-code-signing`: 配布物の helper を扱う。本仕様は配布物に触れない。

## Requirements

### Requirement 1: 全体テストの後の helper の向き

**Objective:** As a areka の開発者, I want 全体テストを回した後でも `target\debug\areka.exe` をそのまま使って 32bit の SHIORI のゴーストを起こしたい, so that 実機の確かめのたびに helper を写して避ける手間と、罠を踏むことが無くなる

#### Acceptance Criteria

1. When 全体テストのスクリプトが最後の段まで回り終えた, the 全体テストのスクリプト shall `target\debug\` の `areka.exe` の隣に i686（32bit）版の `shiori-host32-helper.exe` が在る状態で終わる。
2. If 途中のいずれかの段（i686 の成果物のビルドを除く）が赤で終わった, the 全体テストのスクリプト shall 段が赤でも止めずに最後まで回す今の決まりを保ち、1 と同じ状態で終わる。
3. If i686 版の helper を `areka.exe` の隣へ揃えられなかった（i686 版のファイルが無い・置き換えが断られた など）, the 全体テストのスクリプト shall 段ごとの合否の一覧に、揃えられなかったことの分かる赤の行を出し、終了コード 1 で終わる。揃えの段の合否は「終わったときに隣が i686 版か」だけで決める。i686 の成果物のビルドが赤でも、前の i686 版が残っていればそれを隣へ置いて揃えの段は緑とし、ビルドの失敗はビルドの段の赤で知らせる（同じ失敗で二重に赤にしない）。
4. When 開発者が全体テストのスクリプトの直後に `target\debug\areka.exe` で 32bit の SHIORI のゴースト（例: claudia の `yaya.dll`）を起こした, the areka shall 実行ファイルの形式の違い（`0x800700C1`）で SHIORI の読み込みに失敗せずに、そのゴーストを起こす。
5. The 全体テストのスクリプト shall 端末へ出す新しい文（一覧の行を含む）を、今の一覧の行と同じく ASCII の文字だけで書く。
6. The 開発の手順の文書（`README.md` と `.kiro/steering/tech.md` のビルドの手順） shall areka を作る手順を、helper を x64 で作らない `cargo build -p areka` で示す（ワークスペースの根の素の `cargo build` を手順として示さない）。

### Requirement 2: 全体テストが確かめる範囲を減らさない

**Objective:** As a areka の開発者, I want helper の向きを直しても全体テストが確かめる範囲が今より減らないこと, so that 罠を塞いだ代わりに見落としが増えることが無い

#### Acceptance Criteria

1. The 全体テストのスクリプト shall 今ある段（i686 のターゲットの導入・i686 の成果物のビルド・整形と fmt の検査・x64 のワークスペース全テスト・i686 でしか走らないテスト・公開前の確認・文字コードの判定・リリースノートの道具の自分の検査・俯瞰の HTML の道具の自分の検査・`-License` のときの 2 段）を 1 つも外さずに回し続ける。
2. The 全体テストのスクリプト shall helper の単体テストを、全体テストの 1 回の中で少なくとも 1 回（i686 でしか走らないテストの段で）走らせ続ける。
3. The 全体テストのスクリプト shall i686 の helper を `areka.exe` の隣に揃えて使う今のテスト（煙テスト・MCP の `get_log` の実走 など）を、今と同じ条件で走らせ、緑のまま通す。
4. The 全体テストのスクリプト shall helper の約 42 秒かかるテスト（応答が返らない SHIORI を待ち切るテスト）を、今と同じく i686 の段でだけ走らせる（x64 の段へ戻さない）。

### Requirement 3: helper の向きが違うときの記録

**Objective:** As a areka の開発者, I want `areka.exe` の隣の helper が 32bit 版でないときに、32bit の SHIORI を読めない理由をログの 1 行で知りたい, so that 全体テスト以外の操作で同じ状態になっても、原因を探し回らずに直せる

#### Acceptance Criteria

1. When areka が `areka.exe` の隣の helper を使ってゴーストの SHIORI を読もうとし、その helper が 32bit（i686）の実行ファイルでない, the areka shall SHIORI の読み込みの失敗を記録するより前に、失敗の重さ（エラー）の記録を 1 行出す。
2. The 1 の記録 shall 1 行の中に、helper のパス・その helper が 32bit 版でないので 32bit の SHIORI を読めないこと・直し方（i686 版の helper を `areka.exe` の隣へ置く）を含む。
3. When 1 の記録を出した, the areka shall そのゴーストを、今の「SHIORI を読めなかった」ときと同じ扱いで続ける（新しい窓・メッセージボックス・別の止め方を足さない）。
4. While `areka.exe` の隣の helper が 32bit（i686）の実行ファイルである, the areka shall 1 の記録を出さず、ゴーストの起こし方を今と変えない（配布物の helper は常にこの場合）。
5. If helper の向きを確かめられなかった（helper のファイルが無い・在るが読めない・実行ファイルの形をしていない など）, the areka shall 確かめられなかったことと helper のパスを記録に 1 行残し、その後の起こし方は今と変えない（黙って飛ばさない）。
6. The areka shall helper を探す場所を `areka.exe` の隣から広げず、helper の場所を外から指す口も足さない。
