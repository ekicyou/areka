//! 本物の結線（sylphya の写し → 展開）で、翻訳の前の展開と再生側の文字の並びを確かめる
//! （spec `areka-P0-translate-pipeline` 要件 2.2・2.5・2.6）。
//!
//! 利用者名は偽の SHIORI の `username` の照会（「太郎」）から sylphya へ入り、翻訳用と再生用の
//! 写しの源（[`SystemVarWiring::FromSylphya`]）の両方がそこから読む。再生側は dispatcher の
//! 受け口に届いた文字の cue を並べて見る。切替の送り先（要件 3.5）は、2 体の実行系を実際に
//! 作る `crates/areka` の切替の土台の側（`ghost_session_switch_translate_tests.rs`）で確かめる。
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_kanade::{MonotonicMs, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_sakura::contract::{CueCommand, CuePayload, CueSink, SystemVarSnapshot, TalkCue};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

use crate::dispatcher::DispatcherMsg;
use crate::runtime::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};

/// 利用者名を「太郎」と答え、挨拶に `greeting` を返し、`OnTranslate` の Reference を `seen` へ
/// 送って `translated`（`None`＝204）で答える偽の SHIORI（他の照会は 204）。
struct UsernameShiori {
    greeting: &'static str,
    translated: Option<&'static str>,
    seen: Sender<Vec<String>>,
}

impl ShioriBackend for UsernameShiori {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        match id {
            "username" => Ok(Some("太郎".to_string())),
            "OnBoot" => Ok(Some(self.greeting.to_string())),
            "OnTranslate" => {
                let _ = self.seen.send(references.to_vec());
                Ok(self.translated.map(str::to_string))
            }
            _ => Ok(None),
        }
    }

    fn notify(
        &mut self,
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        Ok(())
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        Ok(ExitKind::Clean)
    }

    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }
}

/// 受け口に届いた文字の cue の中身を順に連ねて記録する。
#[derive(Clone)]
struct TextRecorder(Arc<Mutex<String>>);

impl CueSink for TextRecorder {
    fn emit(&mut self, cue: TalkCue) {
        if let CueCommand::Text(text) = cue.command {
            self.0.lock().expect("text mutex poisoned").push_str(&text);
        }
    }
}

/// 翻訳を通さない今日の再生の文字の並び: 台詞を `username`＝「太郎」の写しでそのまま
/// 組み立てたときの文字の cue の並び（再生側の展開＝`compile` の `SystemVar` の腕）。
fn untranslated_text(script: &str) -> String {
    let mut snapshot = SystemVarSnapshot::default();
    snapshot.insert("username", "太郎");
    let compiled = areka_sakura::compile(&areka_parsers::sakura::parse(script), &snapshot);
    compiled
        .sheet
        .cues()
        .iter()
        .filter_map(|cue| match &cue.payload {
            CuePayload::Command(CueCommand::Text(text)) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 本物の結線（`FromSylphya`）で起こし、挨拶の `OnTranslate` の Reference と、再生側に
/// 届いた文字の並び（`want` 文字に届くまで Tick を注入する）を返す。
fn boot_and_play(
    tag: &str,
    greeting: &'static str,
    translated: Option<&'static str>,
    want: usize,
) -> (Vec<String>, String) {
    let temp = temp_path_kit::TempPath::new(&format!("ghost-translate-e2e-{tag}"));
    let root = temp.path();
    let ghost_master = root.join("ghost").join("master");
    let shell_master = root.join("shell").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::create_dir_all(&shell_master).expect("create shell/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,TestGhost\nshiori,dummy.dll\n",
    )
    .expect("write ghost descript.txt");
    std::fs::write(
        shell_master.join("descript.txt"),
        "charset,UTF-8\nname,TestShell\n",
    )
    .expect("write shell descript.txt");

    let (seen_tx, seen_rx) = channel();
    let text = Arc::new(Mutex::new(String::new()));
    let options = GhostBootOptions {
        ghost_root: root.to_path_buf(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(UsernameShiori {
                greeting,
                translated,
                seen: seen_tx,
            }) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(TextRecorder(Arc::clone(&text)))],
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
    let references = seen_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("OnTranslate never reached the SHIORI during boot");

    // 挨拶が再生の側へ渡ってから文字が `want` 文字そろうまで、合成の時刻で Tick を注入する
    // （1 回で 1 秒進める・壁時計の期限は宙吊りを防ぐ上限だけ）。
    let mut now = 1;
    let deadline = Instant::now() + Duration::from_secs(10);
    while text.lock().expect("text mutex poisoned").chars().count() < want
        && Instant::now() < deadline
    {
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher alive while playing the greeting");
        now += 1000;
        std::thread::yield_now();
    }

    let (done_tx, done_rx) = channel();
    std::thread::spawn(move || {
        let result = runtime.shutdown(areka_kanade::CloseReason::System);
        let _ = done_tx.send(result.is_ok());
    });
    assert_eq!(
        done_rx.recv_timeout(Duration::from_secs(10)),
        Ok(true),
        "shutdown should complete within 10s"
    );
    drop(temp);
    let played = text.lock().expect("text mutex poisoned").clone();
    (references, played)
}

/// `username` を持つゴーストの台詞 `%usernameさん` は `OnTranslate` に `太郎さん` で届き
/// （要件 2.2）、204 のとき再生側の文字の並びは翻訳を通さない今日の並びと同じ（要件 2.5）。
#[test]
fn expanded_script_reaches_on_translate_and_204_plays_as_today() {
    const GREETING: &str = r"\0%usernameさん\e";
    let today = untranslated_text(GREETING);
    assert_eq!(today, "太郎さん", "今日の再生は再生側で展開する");

    let (references, played) = boot_and_play("no-content", GREETING, None, today.chars().count());

    assert_eq!(
        (references[0].as_str(), references[2].as_str()),
        (r"\0太郎さん\e", "OnBoot"),
        "OnTranslate の Reference0 は展開の後の台詞"
    );
    assert_eq!(played, today, "204 のとき再生側の文字の並びは今日と同じ");
}

/// 翻訳の結果に `%username` の綴りが残っていれば、再生時に今日と同じ規則で展開される
/// （要件 2.6）。元の挨拶には `%` が無いので、`太郎さん` は翻訳の結果からしか来ない。
#[test]
fn spelling_left_by_translation_is_expanded_at_playback() {
    const GREETING: &str = r"\0こんにちは\e";
    let (references, played) = boot_and_play(
        "replaced",
        GREETING,
        Some(r"\0%usernameさん\e"),
        "太郎さん".chars().count(),
    );

    assert_eq!(references[0], GREETING, "元の挨拶が翻訳へ渡る");
    assert_eq!(
        played, "太郎さん",
        "翻訳の結果に残った綴りは再生側で展開する"
    );
}
