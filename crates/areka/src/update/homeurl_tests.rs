//! `homeurl` の照会の写しと窓口の写しが、前後の空白を落として空を None にすることのテスト。

use std::sync::mpsc;

use areka_kanade::KanadeMsg;
use areka_kanade::resources::ResourceOutcome;

use super::desk::copy_homeurl_for_test;
use super::procedure::UpdatePorts;
use super::worker::ports_for_test;

const CASES: [(&str, Option<&str>); 3] = [
    (" ", None),
    ("https://x/ ", Some("https://x/")),
    ("https://x/", Some("https://x/")),
];

fn query_copy(value: &str) -> Option<String> {
    let (kanade, rx) = mpsc::channel();
    let ports = ports_for_test(kanade);
    let value = value.to_owned();
    let answerer = std::thread::spawn(move || {
        let Ok(KanadeMsg::ResourceQuery { ids, reply }) =
            rx.recv_timeout(std::time::Duration::from_secs(5))
        else {
            panic!("ResourceQuery のはず");
        };
        let answers = vec![ResourceOutcome::Value(value), ResourceOutcome::NoContent];
        let _ = reply.send(ids.into_iter().zip(answers).collect());
    });
    let found = ports.resources().expect("照会に答えが返る");
    answerer.join().expect("答える側が赤を出さない");
    found.homeurl
}

/// 照会の写しは空白だけを None に・前後の空白を落とす。
#[test]
fn resource_query_trims_the_homeurl() {
    for (input, want) in CASES {
        assert_eq!(query_copy(input).as_deref(), want, "{input:?}");
    }
}

/// 窓口の写しも同じ。
#[test]
fn desk_copy_trims_the_homeurl() {
    for (input, want) in CASES {
        let got = copy_homeurl_for_test(ResourceOutcome::Value(input.to_owned()));
        assert_eq!(got.as_deref(), want, "{input:?}");
    }
}
