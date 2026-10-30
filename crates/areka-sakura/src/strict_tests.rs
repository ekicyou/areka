//! strict の語彙の檻（spec: mcp-strict-errors 要件 2.1・2.3・2.4・2.6・2.8）。
//!
//! 16 の検出点の名前・種類・文面の組、位置の数え方（前から 1 度だけ数え、戻ったら数え直す）、
//! 記録 1 件の宛先と欄と本文の形を固定する。

use super::*;

/// 16 の点と、その（名前・種類・文面）の組。文面は `check_script` の ⑶ の表と同じ定数。
#[test]
fn sixteen_points_name_kind_message() {
    use DetectionPoint::*;
    let expected: [(DetectionPoint, &str, &str, &str); 16] = [
        (
            ReadUnknownTag,
            "ReadUnknownTag",
            "unknown_tag",
            MSG_UNKNOWN_TAG,
        ),
        (
            ReadUnclosed,
            "ReadUnclosed",
            "unreadable_argument",
            MSG_UNCLOSED,
        ),
        (
            ReadDefaulted,
            "ReadDefaulted",
            "unreadable_argument",
            MSG_DEFAULTED,
        ),
        (
            ReadMarkerIgnored,
            "ReadMarkerIgnored",
            "ignored",
            MSG_IGNORED,
        ),
        (
            CommandUnhandled,
            "CommandUnhandled",
            "unknown_command",
            MSG_UNKNOWN_COMMAND,
        ),
        (
            ChoiceTimeoutUnreadable,
            "ChoiceTimeoutUnreadable",
            "unreadable_argument",
            MSG_DEFAULTED,
        ),
        (FontIgnored, "FontIgnored", "ignored", MSG_IGNORED),
        (
            FontUnknownKey,
            "FontUnknownKey",
            "unknown_tag",
            MSG_UNKNOWN_TAG,
        ),
        (
            AnchorReopened,
            "AnchorReopened",
            "unpaired_tag",
            MSG_ANCHOR_REOPENED,
        ),
        (
            AnchorUnclosed,
            "AnchorUnclosed",
            "unpaired_tag",
            MSG_ANCHOR_UNCLOSED,
        ),
        (
            AnchorStrayClose,
            "AnchorStrayClose",
            "unpaired_tag",
            MSG_ANCHOR_STRAY_CLOSE,
        ),
        (
            SurfaceUnresolved,
            "SurfaceUnresolved",
            "missing_surface",
            MSG_MISSING_SURFACE,
        ),
        (
            SurfaceAbsent,
            "SurfaceAbsent",
            "missing_surface",
            MSG_MISSING_SURFACE,
        ),
        (
            BalloonInvalid,
            "BalloonInvalid",
            "missing_balloon",
            MSG_MISSING_BALLOON,
        ),
        (
            BalloonAbsent,
            "BalloonAbsent",
            "missing_balloon",
            MSG_MISSING_BALLOON,
        ),
        (
            BalloonBoxMissing,
            "BalloonBoxMissing",
            "missing_balloon",
            MSG_MISSING_BALLOON,
        ),
    ];
    assert_eq!(DetectionPoint::ALL.len(), 16);
    for (i, (point, name, kind, message)) in expected.into_iter().enumerate() {
        assert_eq!(DetectionPoint::ALL[i], point, "ALL の {i} 番目");
        assert_eq!(point.name(), name);
        assert_eq!(format!("{point:?}"), name, "名前は列挙子と同じ綴り");
        assert_eq!(point.kind(), kind, "{name}");
        assert_eq!(point.message(), message, "{name}");
        assert_eq!(
            point.finding(),
            dola::cue::CueFinding {
                point: name,
                kind,
                message
            }
        );
    }
}

/// 種類は 7 つだけ。予約の種類（`missing_animation`・`unknown_entity`）は出ない（要件 2.6・2.8）。
#[test]
fn kinds_are_the_seven_without_reserved() {
    let mut kinds: Vec<&str> = DetectionPoint::ALL.iter().map(|p| p.kind()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(
        kinds,
        [
            "ignored",
            "missing_balloon",
            "missing_surface",
            "unknown_command",
            "unknown_tag",
            "unpaired_tag",
            "unreadable_argument",
        ]
    );
}

/// 文面の定数 10 個の文字列そのもの（`check_script` の答えを 1 バイトも変えない）。文面は `:` を含まない。
#[test]
fn message_constants_verbatim() {
    let all = [
        (
            MSG_UNKNOWN_TAG,
            "areka does not know this tag; playback drops it",
        ),
        (
            MSG_UNKNOWN_COMMAND,
            "no part of areka handles this \\! command; playback ignores it",
        ),
        (
            MSG_MISSING_SURFACE,
            "no such surface in the current shell; playback does not change the surface",
        ),
        (
            MSG_MISSING_BALLOON,
            "no such balloon ID in the current balloon; playback does not change the balloon",
        ),
        (
            MSG_UNCLOSED,
            "the bracket or quote is not closed; playback drops everything from here to the end",
        ),
        (
            MSG_DEFAULTED,
            "the argument is missing or unreadable; playback uses the default value",
        ),
        (MSG_IGNORED, "areka accepts this but it has no effect yet"),
        (
            MSG_ANCHOR_UNCLOSED,
            "the anchor is not closed; playback extends it to the end of the script",
        ),
        (
            MSG_ANCHOR_REOPENED,
            "a new anchor opens before the previous one closes; playback closes the previous one here",
        ),
        (
            MSG_ANCHOR_STRAY_CLOSE,
            "no anchor is open; playback ignores this close",
        ),
    ];
    for (got, want) in all {
        assert_eq!(got, want);
        assert!(!got.contains(':'), "{got}");
    }
    // `\!` は逆斜線 1 つ（逃がしが化けていないこと）。
    assert!(
        MSG_UNKNOWN_COMMAND
            .as_bytes()
            .windows(3)
            .any(|w| w == b" \\!")
    );
    assert!(!MSG_UNKNOWN_COMMAND.contains("\\\\"));
}

/// 位置は文字の数。同じ範囲は数え直さず、前へ進めば差分だけ数え、戻ったら先頭から数え直す。
#[test]
fn char_cursor_counts_chars_forward_and_recounts_backward() {
    let script = "あい\\s[99]う\\x";
    // バイト: あ=0..3 い=3..6 \s[99]=6..12 う=12..15 \x=15..17
    let mut cursor = CharCursor::new(script);
    let p = cursor.place(6..12);
    assert_eq!((p.start, p.end, p.text.as_str()), (2, 8, "\\s[99]"));
    // 同じ命令に 2 件目。
    let p = cursor.place(6..12);
    assert_eq!((p.start, p.end, p.text.as_str()), (2, 8, "\\s[99]"));
    let p = cursor.place(15..17);
    assert_eq!((p.start, p.end, p.text.as_str()), (9, 11, "\\x"));
    // 戻った（台本の順でない）ときも正しい数になる。
    let p = cursor.place(0..3);
    assert_eq!((p.start, p.end, p.text.as_str()), (0, 1, "あ"));
    let p = cursor.place(12..15);
    assert_eq!((p.start, p.end, p.text.as_str()), (8, 9, "う"));
}

/// 記録 1 件は `areka::log::error` へ、欄 `ghost`・`point` 付きで、本文は `<kind> <start>-<end> <message>: <text>`。
#[test]
fn record_writes_one_event_with_convention_fields() {
    assert_eq!(RECORD_TARGET, "areka::log::error");
    let place = Place {
        start: 2,
        end: 11,
        text: "\\s[99999]".to_string(),
    };
    let ((), events) = log_capture_kit::capture(|| {
        record("さくら", DetectionPoint::SurfaceAbsent.finding(), &place)
    });
    assert_eq!(events.len(), 1);
    let ev = &events[0];
    assert_eq!(ev.target, RECORD_TARGET);
    assert_eq!(ev.level, tracing::Level::WARN);
    assert_eq!(ev.field_str("ghost"), Some("さくら"));
    assert_eq!(ev.field_str("point"), Some("SurfaceAbsent"));
    assert_eq!(
        ev.message(),
        "missing_surface 2-11 no such surface in the current shell; playback does not change the surface: \\s[99999]"
    );
}

/// 差し込み口は外から実装でき、束ねたものは複製できる。
#[test]
fn hooks_bundle_ghost_and_judges() {
    struct Judges;
    impl StrictJudges for Judges {
        fn command_handled(&self, name: &str, _first: Option<&str>) -> bool {
            name == "open"
        }
        fn font(&self, args: &[String]) -> Option<DetectionPoint> {
            (args.first().map(String::as_str) == Some("sub")).then_some(DetectionPoint::FontIgnored)
        }
    }
    let hooks = StrictHooks {
        ghost: "さくら".into(),
        judges: std::sync::Arc::new(Judges),
    };
    let copy = hooks.clone();
    assert_eq!(&*copy.ghost, "さくら");
    assert!(copy.judges.command_handled("open", None));
    assert!(!copy.judges.command_handled("nosuch", None));
    assert_eq!(
        copy.judges.font(&["sub".to_string(), "1".to_string()]),
        Some(DetectionPoint::FontIgnored)
    );
}
