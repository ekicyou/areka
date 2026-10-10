# 設計バリデーション: areka-P0-mcp-stdio-bridge

> 2026-10-10・本ブランチ（main `414d43eb` の上）。入力は `requirements.md`・`design.md`・`research.md`（10〜14 章）・`.kiro/steering/`（product・tech・structure・focus・workflow・roadmap）。設計の主張は `crates/areka-mcp/src/`（`port.rs`・`handler.rs`・`dispatch.rs`・`server.rs`・`lib.rs`・`help.rs`・`help_tests.rs`・`tools/mod.rs`）・`crates/areka/src/{main,boot_config}.rs`・`tools/package.ps1`・`Cargo.lock`・cargo の登録簿の rmcp 3.5.0 と hyper 1.11.1 のソースで突き合わせた（読むだけ・建てていない・机の番号へはつないでいない）。コードは「何の定義か」で指す。

## レビュー要約

設計は、要件が設計へ送った議題（控えの置き場・確かめる要求・候補の共有・help の配線・待つ時間・rmcp での組み方・JSON でない行の扱い）に 1 つずつ答え（D-1〜D-16）、要件 1〜8 の全項目を部品と流れに対応づけている。rmcp 3.5.0 の API に関する主張はソースで裏が取れ、`tools/package.ps1` の 5 か所も実物にある。実装へ進める水準にある。残る心配は「本物の受け口で未検証の経路」「areka の決まりと違う細部」「並ぶ要求と Link の入れ替えの競合」の 3 つで、どれも設計の骨格を変えずに直せる。

## 重大な問題（3 件まで）

### 🔴 問題 1: 「同じ接続で確かめてから送る」（D-4）は本物の受け口を相手に動かしていない

- **懸念**: D-4 は、接続ごとに確かめの `initialize` と本当の要求を **1 本の keep-alive 接続**で送り、2 つ目に `Connection: close` を付ける。`research.md` 11.1 の試しの受け口は PowerShell の偽物で、受けた方法の並びに各要求の前の `initialize` が無い＝この 2 段の送りは試していない。本物の受け口（`server.rs` の `accept_loop` が接続ごとに hyper の `http1::Builder` で回す・`dispatch::forward` が本文を集めて rmcp へ渡す）で、1 接続 2 要求が意図どおり通ることは設計の前提であって証拠が無い。
- **影響**: 要件 2.3・2.11・4.5 の核になる経路。hyper 側の keep-alive の扱いが想定と違えば、全要求が 4.1（No response）に倒れる。加えて、Desktop の要求 1 件ごとに areka の記録（`dispatch::handle` の「要求に応えた」の `debug!`）に `initialize` が 1 行ずつ余分に残る。
- **提案**: ⑴ 実装の最初のタスクで `http_tests.rs` を**偽の受け口だけでなく `areka_mcp::start(&[0], …)` の本物の受け口**でも回す（名乗りが合えば同じ接続で 2 要求が通ること・`Connection: close` の後に受け口が閉じること）。崩れたら、確かめと本当の要求を別々の接続にせず（入れ替わりの隙）、D-4 の形のまま hyper 側の設定を見直す。⑵ 記録の雑音は差の記録（8.4）か design の Monitoring に「areka の debug の記録に確かめの `initialize` が要求ごとに 1 行増える」と書いておく。
- **根拠**: 要件 2.3・2.11・4.5。`design.md` の D-4・「つながっているときの要求 1 件」の図・`research.md` 11.1 の「偽の受け口が受けた方法の並び」。

### 🔴 問題 2: 控えの置き場を `canonicalize` で求めるのは areka 本体の決まりと違う（D-7）

- **懸念**: D-7 と Config は、exe の本当の場所を `std::env::current_exe()` を `std::fs::canonicalize` で解いて求める。areka 本体の `boot_config.rs` の `exe_location` は自前の `follow_exe_links`／`probe_link` でリンクを辿り、`resolve_root_from` の注釈は「`canonicalize` は使わない＝長いパスの接頭辞（`\\?\`）と失敗の口を持ち込まない」と明記している。中継だけが違う決まりを持つと、同じ机で areka の記憶（`profile\areka\`）と中継の控え（`profile\areka\mcp-bridge\`）の根の求め方が食い違いうる（ネットワークドライブ・`subst` のドライブで `canonicalize` が失敗して「控えなし」に倒れる・記録のパスに `\\?\` が付く）。
- **影響**: 要件 3.3（起動し直しても控えが読める）・7.4（README に書く置き場の説明）。失敗しても 3.10 で続くので致命ではないが、「Desktop を先に起動してもツールが並ぶ」が机によって効かなくなる。
- **提案**: help が載せる `command` は既に `exe_location()` で解いた本当の場所なので、中継はリンクごしに起こされる前提を置かなくてよい。`current_exe()` の親をそのまま（`std::path::absolute` 程度で）使い、`canonicalize` を外す。areka 本体と同じ辿り方が要るなら `follow_exe_links` を `areka-mcp` 側へ出して共有する（本 spec の範囲を広げるので設計ディスカッションで決める）。
- **根拠**: 要件 3.3・3.10・7.4。`design.md` の D-7・Config の「Responsibilities & Constraints」・Data Models の置き場。

### 🔴 問題 3: 並ぶ要求と Link の捨て直しが競合する（D-6・D-13）

- **懸念**: rmcp の受け手は要求ごとに仕事を起こす（D-13）。Bridge は `link: Arc<Mutex<Option<(Link, u16)>>>` を持ち、「書く前の失敗」を見た仕事が `link` を捨てて探し直す（D-6）。rmcp の `RunningService` は落とすと `DropGuard` が働き手を取り消すので、同じ Link を使っている途中の別の仕事（長い `tools/call` など）は、areka が答えていても輸送の失敗で終わる。設計はそれを「書いた後」として扱う（安全側）ので Desktop には「No response」が返る。さらに 2 つの仕事が同時に失敗を見ると、片方が作ったばかりの新しい Link をもう片方が捨てる。
- **影響**: 要件 2.10（起動し直した areka へ届く）・4.1（本当は届いた応答が No response になる）・5.4（複数の要求が互いに影響しない）。areka の起動し直しの直後に Desktop が複数の要求を送る場面で起きやすい。
- **提案**: Link に世代（番号）を持たせ、捨てるときは「自分が使った世代と同じときだけ」捨てる（`Arc::ptr_eq` か世代の比較）。別の仕事が既に入れ替えていれば捨てずに新しい Link で 1 回だけ送る。`service_tests.rs` に「長い要求の途中で別の要求が書く前の失敗を見る」場面を 1 本足す（受け口を止めて別の名乗りを立てる既存の案に、同時の要求を重ねる形）。
- **根拠**: 要件 2.10・4.1・4.5・5.4。`design.md` の D-6・D-13・Bridge の「Responsibilities & Constraints」（`handle_request` の順 ⑶）・Error Strategy の「覚えた番号に接続できない・名乗りが違う（書く前）」の行。

## 設計の強み

- **rmcp の API の主張がソースで裏付けられ、退けた形の理由も測った事実で書かれている**: `serve_directly_with_ct`（`service`・`transport`・`peer_info`・`ct` を取って `RunningService` を返す）・`StreamableHttpClientTransport::with_client(client, config)`・`RunningService::peer_info()`・`StreamableHttpClient` の必須 3 つと既定 2 つ・`StreamableHttpPostResponse` の 3 通り・`PeerRequestOptions`（`non_exhaustive`・`timeout`）・`DiscoverResult::from_server_info`・`ProtocolVersion::KNOWN_VERSIONS`・`DynamicTransportError.error`・stdio の輸送が `Syntax`／`Eof` を `debug!` だけで捨て `Data` に `-32600` を返すこと・`serve_server` が最初の問い合わせの失敗で `Err` を返すこと・`negotiate_protocol_version` が `pub(crate)` で呼べないこと・`QuitReason::Closed` の掃き出し 5 秒・hyper の `header_read_timeout` 既定 30 秒・`transport-streamable-http-client` が `client-side-sse`＋`transport-worker` だけで `reqwest` を引かないこと・`Cargo.lock` に `httparse` 1.10.1 と `sse-stream` があり `want`・`try-lock` が無いこと。全部そのとおりだった。
- **「約束の外のファイル」を隠さず表にして諮り、代案まで添えている**（D-14 と「他の spec と重なりうるファイル」の表）: roadmap C5 ⑦ の約束（新しいクレート・`Cargo.lock`・謝辞・`help.rs`・`help_tests.rs`・`package.ps1`・README の自分の行）に対し、`main.rs`・`dispatch.rs`・`server.rs`・`lib.rs`・`handler.rs`・`server_tests.rs`・`testkit.rs` を触る理由と行数を挙げ、取らない場合の形（`help.rs` の中で `current_exe`）と欠点も書いてある。開発者の方針（2026-10-05「設計は本質の形で」）どおり。

## 細かな点（重大ではない・実装で直せる）

- `main.rs` の窓なしの指定: 設計は `#![windows_subsystem = "windows"]`（areka 本体と同じ）と書くが、areka 本体の `main.rs` は `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`。同じにするなら debug では窓ありにする形。
- `MCP_PATH` は `dispatch.rs` では `pub(crate)` でなく素の `const`（私的）。`pub` にする点は変わらない。
- D-12「中継の記録の文は英語（ASCII）」に対し、2.5 の読めない値の `warn!` は `areka_mcp::candidates_from_env_value` の日本語の文がそのまま中継の標準エラーへ出る。Desktop の記録は UTF-8 なので害は無いが、design に「共有の関数の記録は日本語のまま」と一言書くか、中継側で英語の 1 件を足す。
- rmcp の stdio の輸送は先頭の UTF-8 の BOM を落とす。`LineTransport` の `receive` にもその 1 行を入れておく（Desktop は送らない見込みだが、`-32700` に倒す理由も無い）。
- 3.9 の「最初の問い合わせには areka の今のサーバーの情報」は、Link が生きている間は `peer_info()`（Link を作ったときの情報）で答える。areka が別の版で起動し直した直後に Desktop が `initialize` を送ると古い情報になるが、続く `tools/list` が書く前の失敗 → 探し直しで新しくなるので実害は小さい。差の記録に一言。
- `research.md` 11.3・11.4 の数値（exe の大きさ・接続が断られるまでの時間・8 要求 0.44 秒）は `target\` の試しの産物で git に無く、本レビューでは確かめていない。設計の判断（並べて開いて見切る・`areka-mcp` を依存にしてよい）は数値の桁が変わらなければ揺らがない。

## 最終判定

**GO**。既存の層（`areka-mcp` の定数の共有・`exe_location` の決まり・`tracing`・兄弟ファイルのテスト・`target` の下の一時フォルダ・`publish = false # 理由`）に沿い、要件の全項目に部品と流れが対応づいていて、rmcp の API の主張は全部ソースで裏が取れた。上の 3 件は設計ディスカッションで決めればよく（問題 1 はテストの順番、問題 2 は 1 関数の置き換え、問題 3 は Link の世代 1 つ）、骨格を変えない。

次の手: `/kiro-design-discussion areka-P0-mcp-stdio-bridge` で問題 1〜3 と D-14 の「約束の外のファイル」を諮り、決まったら `/kiro-spec-tasks areka-P0-mcp-stdio-bridge`。
