//! 標準入出力の輸送（`stdio.rs`）のテスト: メモリ上の管（`tokio::io::duplex`）を標準入出力の代わりに差す。

use rmcp::model::ServerJsonRpcMessage;
use rmcp::transport::Transport;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, DuplexStream};

use super::LineTransport;

type Pipes = (
    LineTransport<DuplexStream, DuplexStream>,
    DuplexStream,
    BufReader<DuplexStream>,
);

/// 管の大きさ。大きな行はこれを越えるので、書く側と読む側を同時に回す。
const PIPE: usize = 64 * 1024;

const MIB: usize = 1024 * 1024;

/// 輸送と、相手（Desktop の役）の書き口・読み口。
fn pipes() -> Pipes {
    pipes_with_output(PIPE)
}

fn pipes_with_output(output_size: usize) -> Pipes {
    let (input, stdin) = tokio::io::duplex(PIPE);
    let (stdout, output) = tokio::io::duplex(output_size);
    (
        LineTransport::new(stdin, stdout),
        input,
        BufReader::new(output),
    )
}

fn ping(id: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": "ping"}).to_string()
}

/// 文を 1 つ持つツールの結果の応答。
fn response(id: Value, text: &str) -> ServerJsonRpcMessage {
    serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {"content": [{"type": "text", "text": text}]},
    }))
    .expect("応答の形として読める")
}

/// 次の要求を JSON の値で受け取る。
async fn received(transport: &mut LineTransport<DuplexStream, DuplexStream>) -> Value {
    let message = transport.receive().await.expect("要求が 1 件届く");
    serde_json::to_value(&message).expect("要求は JSON にできる")
}

/// 標準出力の次の 1 行。行末が LF 1 つで、JSON として読めることも見る（要件 1.1・1.4）。
async fn next_line(output: &mut BufReader<DuplexStream>) -> Value {
    let mut line = String::new();
    output.read_line(&mut line).await.expect("1 行読める");
    assert!(line.ends_with('\n'), "行末が LF でない: {line:?}");
    assert!(!line.contains('\r'), "CR が混ざった: {line:?}");
    serde_json::from_str(&line).expect("標準出力の 1 行は JSON")
}

/// 輸送を閉じてから、標準出力に残った全部の行。どの行も LF 1 つで終わり、JSON として読める
/// （要件 1.4・5.1）。行が足りないときに待ち続けないよう、閉じてから終わりまで読む。
async fn rest_of_lines(
    mut transport: LineTransport<DuplexStream, DuplexStream>,
    output: &mut BufReader<DuplexStream>,
) -> Vec<Value> {
    transport.close().await.expect("閉じられる");
    let mut rest = String::new();
    output
        .read_to_string(&mut rest)
        .await
        .expect("終わりまで UTF-8 で読める");
    assert!(
        rest.is_empty() || rest.ends_with('\n'),
        "行末が LF でない: {rest:?}"
    );
    assert!(!rest.contains('\r'), "CR が混ざった: {rest:?}");
    rest.split_terminator('\n')
        .map(|line| serde_json::from_str(line).expect("標準出力の 1 行は JSON"))
        .collect()
}

fn parse_error_line() -> Value {
    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}})
}

fn invalid_request_line() -> Value {
    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "Invalid Request"}})
}

/// 数の `id` と文字列の `id` が、要求でも応答でもそのまま通る（要件 1.1）。
/// 行末の CRLF と前後の空白は落ちる。
#[tokio::test]
async fn number_and_string_ids_round_trip() {
    let (mut transport, mut input, mut output) = pipes();
    let lines = format!("{}\r\n \t{} \t\n", ping(json!(1)), ping(json!("1")));
    input.write_all(lines.as_bytes()).await.expect("書ける");

    for id in [json!(1), json!("1")] {
        let message = received(&mut transport).await;
        assert_eq!(message["id"], id);
        assert_eq!(message["method"], "ping");

        transport
            .send(response(id.clone(), "ok"))
            .await
            .expect("応答を書ける");
        let line = next_line(&mut output).await;
        assert_eq!(line["id"], id);
        assert_eq!(line["result"]["content"][0]["text"], "ok");
    }
}

/// 中身に改行を含む応答も 1 行で書く（要件 1.4）: 標準出力の LF は行末の 1 つだけ。
#[tokio::test]
async fn response_with_newlines_is_written_as_one_line() {
    let (mut transport, _input, mut output) = pipes();
    let text = "1 行目\n2 行目\r\n3 行目";
    transport
        .send(response(json!(7), text))
        .await
        .expect("応答を書ける");

    let lines = rest_of_lines(transport, &mut output).await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["result"]["content"][0]["text"], text);
}

/// 空の行・空白だけの行は飛ばし、標準出力へ何も書かない（要件 1.5・5.1）。
#[tokio::test]
async fn blank_lines_are_skipped_without_output() {
    let (mut transport, mut input, mut output) = pipes();
    let lines = format!("\n\r\n   \n\t \r\n{}\n\n", ping(json!(1)));
    input.write_all(lines.as_bytes()).await.expect("書ける");
    drop(input);

    assert_eq!(received(&mut transport).await["id"], 1);
    assert!(transport.receive().await.is_none());
    assert_eq!(
        rest_of_lines(transport, &mut output).await,
        Vec::<Value>::new()
    );
}

/// JSON として読めない行には `-32700`・`id` は `null` を 1 行返し、次の行はそのまま通る（要件 1.6）。
#[tokio::test]
async fn non_json_lines_get_parse_error_and_the_next_line_passes() {
    let (mut transport, mut input, mut output) = pipes();
    let broken: [&[u8]; 4] = [
        b"not json",
        br#"{"jsonrpc":"2.0","id":1,"method":"ping""#,
        br#"{"jsonrpc":"2.0","id":1,"method":"ping"} x"#,
        b"\xff\xfe",
    ];
    for line in broken {
        input.write_all(line).await.expect("書ける");
        input.write_all(b"\n").await.expect("書ける");
    }
    input
        .write_all(format!("{}\n", ping(json!(2))).as_bytes())
        .await
        .expect("書ける");
    drop(input);

    assert_eq!(received(&mut transport).await["id"], 2);
    assert!(transport.receive().await.is_none());
    assert_eq!(
        rest_of_lines(transport, &mut output).await,
        vec![parse_error_line(); broken.len()]
    );
}

/// JSON として読めるが要求の形でない行（配列・数・文字列・欄の形違い）には `-32600`・`id` は `null`
/// を 1 行返し、次の行はそのまま通る（要件 1.7）。
#[tokio::test]
async fn non_object_lines_get_invalid_request_and_the_next_line_passes() {
    let (mut transport, mut input, mut output) = pipes();
    let wrong = [
        "[1,2]",
        "42",
        r#""text""#,
        "null",
        "{}",
        r#"{"jsonrpc":"2.0","id":1}"#,
        r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":7}"#,
    ];
    let lines = format!("{}\n{}\n", wrong.join("\n"), ping(json!(2)));
    input.write_all(lines.as_bytes()).await.expect("書ける");
    drop(input);

    assert_eq!(received(&mut transport).await["id"], 2);
    assert!(transport.receive().await.is_none());
    assert_eq!(
        rest_of_lines(transport, &mut output).await,
        vec![invalid_request_line(); wrong.len()]
    );
}

/// 入力の終わりで輸送が終わる（要件 1.9）。行末の無い最後の行も 1 行として届く。
#[tokio::test]
async fn end_of_input_ends_the_transport() {
    let (mut transport, mut input, _output) = pipes();
    input
        .write_all(ping(json!(1)).as_bytes())
        .await
        .expect("書ける");
    drop(input);

    assert_eq!(received(&mut transport).await["id"], 1);
    assert!(transport.receive().await.is_none());
}

/// 数 MiB の行が欠けずに通る（要件 1.8）: 要求の行も応答の行も。
#[tokio::test]
async fn multi_mebibyte_lines_are_not_truncated() {
    let (mut transport, mut input, mut output) = pipes();
    let big = "a".repeat(4 * MIB);

    let line = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "tool", "arguments": {"text": big}},
    });
    let writer = tokio::spawn(async move {
        input
            .write_all(format!("{line}\n").as_bytes())
            .await
            .expect("書ける");
        input
    });
    let message = received(&mut transport).await;
    assert_eq!(message["params"]["arguments"]["text"], big);
    let _input = writer.await.expect("書く側が終わる");

    let sender = tokio::spawn(transport.send(response(json!(1), &big)));
    let line = next_line(&mut output).await;
    assert_eq!(line["result"]["content"][0]["text"], big);
    sender.await.expect("書く側が終わる").expect("応答を書ける");
}

/// 最初の行の頭の BOM は落とす。2 行目からの BOM は落とさない（JSON として読めない行）。
#[tokio::test]
async fn bom_is_dropped_only_on_the_first_line() {
    let (mut transport, mut input, mut output) = pipes();
    let lines = format!("\u{feff}{}\n\u{feff}{}\n", ping(json!(1)), ping(json!(2)));
    input.write_all(lines.as_bytes()).await.expect("書ける");
    drop(input);

    assert_eq!(received(&mut transport).await["id"], 1);
    assert!(transport.receive().await.is_none());
    assert_eq!(
        rest_of_lines(transport, &mut output).await,
        vec![parse_error_line()]
    );
}

/// 同時の書き込みが 1 行の途中で混ざらない（要件 1.4）: rmcp は `send` の未来を並べて走らせる。
/// 1 件が管より大きいので、鍵が無ければ書いている途中で次の 1 件が割り込む。
#[tokio::test]
async fn concurrent_sends_do_not_interleave_within_a_line() {
    let (mut transport, _input, mut output) = pipes();
    let count = 8;
    let senders: Vec<_> = (0..count)
        .map(|id| {
            let text = id.to_string().repeat(4 * PIPE);
            tokio::spawn(transport.send(response(json!(id), &text)))
        })
        .collect();

    let mut ids = Vec::new();
    for _ in 0..count {
        let line = next_line(&mut output).await;
        let id = line["id"].as_u64().expect("数の id");
        assert_eq!(
            line["result"]["content"][0]["text"],
            id.to_string().repeat(4 * PIPE)
        );
        ids.push(id);
    }
    ids.sort_unstable();
    assert_eq!(ids, (0..count).collect::<Vec<_>>());
    for sender in senders {
        sender.await.expect("書く側が終わる").expect("応答を書ける");
    }
}

/// 読んでいる途中で `receive` が捨てられても、読みかけの行は次の `receive` で続きから揃う
/// （rmcp は `receive` を `select!` の中で回し、応答を書く番が来るたびに捨てて呼び直す）。
#[tokio::test]
async fn dropped_receive_keeps_the_partial_line() {
    let (mut transport, mut input, _output) = pipes();
    let line = ping(json!(1));
    let (head, tail) = line.split_at(line.len() / 2);

    input.write_all(head.as_bytes()).await.expect("書ける");
    {
        let mut receive = std::pin::pin!(transport.receive());
        assert!(futures::poll!(&mut receive).is_pending());
    }
    input
        .write_all(format!("{tail}\n").as_bytes())
        .await
        .expect("書ける");
    drop(input);

    assert_eq!(received(&mut transport).await["id"], 1);
}

/// 行末の無い最後の行を読んでいる途中で `receive` が捨てられ、そのまま入力が終わっても、
/// 読みかけは最後の 1 行として届く。
#[tokio::test]
async fn dropped_receive_keeps_the_last_line_without_a_line_end() {
    let (mut transport, mut input, _output) = pipes();
    input
        .write_all(ping(json!(1)).as_bytes())
        .await
        .expect("書ける");
    {
        let mut receive = std::pin::pin!(transport.receive());
        assert!(futures::poll!(&mut receive).is_pending());
    }
    drop(input);

    assert_eq!(received(&mut transport).await["id"], 1);
    assert!(transport.receive().await.is_none());
}

/// 行の形のエラーを書いている途中で `receive` が捨てられても、エラーの行は欠けずに 1 行で出る。
/// 標準出力の管を小さくして、エラーの行が 1 度では書き切れないようにする。
#[tokio::test]
async fn dropped_receive_does_not_cut_the_error_line() {
    let (mut transport, mut input, mut output) = pipes_with_output(8);
    input.write_all(b"not json\n").await.expect("書ける");
    {
        let mut receive = std::pin::pin!(transport.receive());
        assert!(futures::poll!(&mut receive).is_pending());
    }
    input
        .write_all(format!("{}\n", ping(json!(1))).as_bytes())
        .await
        .expect("書ける");

    let reader = tokio::spawn(async move {
        let mut written = Vec::new();
        output
            .read_to_end(&mut written)
            .await
            .expect("終わりまで読める");
        written
    });
    assert_eq!(received(&mut transport).await["id"], 1);
    transport.close().await.expect("閉じられる");

    let written = reader.await.expect("読む側が終わる");
    assert_eq!(written.last(), Some(&b'\n'));
    let line: Value = serde_json::from_slice(&written).expect("欠けていない 1 行の JSON");
    assert_eq!(line, parse_error_line());
}
