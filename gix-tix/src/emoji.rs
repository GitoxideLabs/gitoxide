use gix::bstr::{BStr, ByteSlice};

pub(crate) fn leading(message: &BStr) -> Option<&str> {
    let grapheme = message.graphemes().next()?;
    let first = grapheme.chars().next()?;
    if grapheme.contains('\u{fe0e}') {
        return None;
    }
    let emoji = if matches!(first, '#' | '*' | '0'..='9') {
        grapheme.contains('\u{20e3}')
    } else {
        // Unicode 17.0 Emoji property, excluding ASCII keycap bases handled above.
        // https://www.unicode.org/Public/17.0.0/ucd/emoji/emoji-data.txt
        matches!(first,
            '\u{a9}'
            | '\u{ae}'
            | '\u{203c}'
            | '\u{2049}'
            | '\u{2122}'
            | '\u{2139}'
            | '\u{2194}'..='\u{2199}'
            | '\u{21a9}'..='\u{21aa}'
            | '\u{231a}'..='\u{231b}'
            | '\u{2328}'
            | '\u{23cf}'
            | '\u{23e9}'..='\u{23f3}'
            | '\u{23f8}'..='\u{23fa}'
            | '\u{24c2}'
            | '\u{25aa}'..='\u{25ab}'
            | '\u{25b6}'
            | '\u{25c0}'
            | '\u{25fb}'..='\u{25fe}'
            | '\u{2600}'..='\u{2604}'
            | '\u{260e}'
            | '\u{2611}'
            | '\u{2614}'..='\u{2615}'
            | '\u{2618}'
            | '\u{261d}'
            | '\u{2620}'
            | '\u{2622}'..='\u{2623}'
            | '\u{2626}'
            | '\u{262a}'
            | '\u{262e}'..='\u{262f}'
            | '\u{2638}'..='\u{263a}'
            | '\u{2640}'
            | '\u{2642}'
            | '\u{2648}'..='\u{2653}'
            | '\u{265f}'..='\u{2660}'
            | '\u{2663}'
            | '\u{2665}'..='\u{2666}'
            | '\u{2668}'
            | '\u{267b}'
            | '\u{267e}'..='\u{267f}'
            | '\u{2692}'..='\u{2697}'
            | '\u{2699}'
            | '\u{269b}'..='\u{269c}'
            | '\u{26a0}'..='\u{26a1}'
            | '\u{26a7}'
            | '\u{26aa}'..='\u{26ab}'
            | '\u{26b0}'..='\u{26b1}'
            | '\u{26bd}'..='\u{26be}'
            | '\u{26c4}'..='\u{26c5}'
            | '\u{26c8}'
            | '\u{26ce}'..='\u{26cf}'
            | '\u{26d1}'
            | '\u{26d3}'..='\u{26d4}'
            | '\u{26e9}'..='\u{26ea}'
            | '\u{26f0}'..='\u{26f5}'
            | '\u{26f7}'..='\u{26fa}'
            | '\u{26fd}'
            | '\u{2702}'
            | '\u{2705}'
            | '\u{2708}'..='\u{270d}'
            | '\u{270f}'
            | '\u{2712}'
            | '\u{2714}'
            | '\u{2716}'
            | '\u{271d}'
            | '\u{2721}'
            | '\u{2728}'
            | '\u{2733}'..='\u{2734}'
            | '\u{2744}'
            | '\u{2747}'
            | '\u{274c}'
            | '\u{274e}'
            | '\u{2753}'..='\u{2755}'
            | '\u{2757}'
            | '\u{2763}'..='\u{2764}'
            | '\u{2795}'..='\u{2797}'
            | '\u{27a1}'
            | '\u{27b0}'
            | '\u{27bf}'
            | '\u{2934}'..='\u{2935}'
            | '\u{2b05}'..='\u{2b07}'
            | '\u{2b1b}'..='\u{2b1c}'
            | '\u{2b50}'
            | '\u{2b55}'
            | '\u{3030}'
            | '\u{303d}'
            | '\u{3297}'
            | '\u{3299}'
            | '\u{1f004}'
            | '\u{1f0cf}'
            | '\u{1f170}'..='\u{1f171}'
            | '\u{1f17e}'..='\u{1f17f}'
            | '\u{1f18e}'
            | '\u{1f191}'..='\u{1f19a}'
            | '\u{1f1e6}'..='\u{1f1ff}'
            | '\u{1f201}'..='\u{1f202}'
            | '\u{1f21a}'
            | '\u{1f22f}'
            | '\u{1f232}'..='\u{1f23a}'
            | '\u{1f250}'..='\u{1f251}'
            | '\u{1f300}'..='\u{1f321}'
            | '\u{1f324}'..='\u{1f393}'
            | '\u{1f396}'..='\u{1f397}'
            | '\u{1f399}'..='\u{1f39b}'
            | '\u{1f39e}'..='\u{1f3f0}'
            | '\u{1f3f3}'..='\u{1f3f5}'
            | '\u{1f3f7}'..='\u{1f4fd}'
            | '\u{1f4ff}'..='\u{1f53d}'
            | '\u{1f549}'..='\u{1f54e}'
            | '\u{1f550}'..='\u{1f567}'
            | '\u{1f56f}'..='\u{1f570}'
            | '\u{1f573}'..='\u{1f57a}'
            | '\u{1f587}'
            | '\u{1f58a}'..='\u{1f58d}'
            | '\u{1f590}'
            | '\u{1f595}'..='\u{1f596}'
            | '\u{1f5a4}'..='\u{1f5a5}'
            | '\u{1f5a8}'
            | '\u{1f5b1}'..='\u{1f5b2}'
            | '\u{1f5bc}'
            | '\u{1f5c2}'..='\u{1f5c4}'
            | '\u{1f5d1}'..='\u{1f5d3}'
            | '\u{1f5dc}'..='\u{1f5de}'
            | '\u{1f5e1}'
            | '\u{1f5e3}'
            | '\u{1f5e8}'
            | '\u{1f5ef}'
            | '\u{1f5f3}'
            | '\u{1f5fa}'..='\u{1f64f}'
            | '\u{1f680}'..='\u{1f6c5}'
            | '\u{1f6cb}'..='\u{1f6d2}'
            | '\u{1f6d5}'..='\u{1f6d8}'
            | '\u{1f6dc}'..='\u{1f6e5}'
            | '\u{1f6e9}'
            | '\u{1f6eb}'..='\u{1f6ec}'
            | '\u{1f6f0}'
            | '\u{1f6f3}'..='\u{1f6fc}'
            | '\u{1f7e0}'..='\u{1f7eb}'
            | '\u{1f7f0}'
            | '\u{1f90c}'..='\u{1f93a}'
            | '\u{1f93c}'..='\u{1f945}'
            | '\u{1f947}'..='\u{1f9ff}'
            | '\u{1fa70}'..='\u{1fa7c}'
            | '\u{1fa80}'..='\u{1fa8a}'
            | '\u{1fa8e}'..='\u{1fac6}'
            | '\u{1fac8}'
            | '\u{1facd}'..='\u{1fadc}'
            | '\u{1fadf}'..='\u{1faea}'
            | '\u{1faef}'..='\u{1faf8}'
        )
    };
    emoji.then_some(grapheme)
}

#[cfg(test)]
mod tests {
    use super::leading;
    use gix::bstr::ByteSlice;

    #[test]
    fn leading_emoji_keeps_the_complete_grapheme() {
        for emoji in [
            "✅",
            "⚡",
            "©",
            "©️",
            "👩🏽‍💻",
            "👨‍👩‍👧‍👦",
            "🇩🇪",
            "1️⃣",
            "9\u{20e3}",
            "#️⃣",
            "*️⃣",
            "\u{1faea}",
            "\u{1facd}",
            "\u{1f3f4}\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}",
        ] {
            let message = format!("{emoji} Checks passed");
            assert_eq!(
                leading(message.as_bytes().as_bstr()),
                Some(emoji),
                "the gutter retains each emoji's modifiers, joiners, and presentation"
            );
        }
    }

    #[test]
    fn ordinary_text_and_text_presentation_stay_in_the_message() {
        for message in [
            "",
            "Checks passed",
            "123 tests passed",
            "# Heading",
            "* list item",
            "1\u{fe0f} without a keycap",
            "👁\u{fe0e} text presentation",
            "中",
            "e\u{301}",
            "☆",
            "\u{1f100}",
            "\u{1fa8b}",
            " ✅ not the first character",
        ] {
            assert_eq!(
                leading(message.as_bytes().as_bstr()),
                None,
                "only a leading emoji becomes a gutter marker: {message:?}"
            );
        }
        assert_eq!(
            leading(b"\xff\xf0\x9f\x98\x80".as_bstr()),
            None,
            "invalid UTF-8 is ordinary message data"
        );
    }
}
