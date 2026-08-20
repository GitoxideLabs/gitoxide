use gix_config_value::Color;

#[test]
fn from_utf8_str() -> crate::Result {
    assert_eq!(
        Color::try_from("red bold")?.to_string(),
        "red bold",
        "UTF-8 strings use the same color parser as byte strings"
    );
    Ok(())
}

mod name {
    use std::str::FromStr;

    use gix_config_value::color::Name;

    fn name(input: &str) -> Name {
        Name::from_str(input).expect("valid color name")
    }

    #[test]
    fn non_bright() {
        assert_eq!(name("normal"), Name::Normal);
        assert_eq!(name("-1"), Name::Normal);
        assert_eq!(name("default"), Name::Default);
        assert_eq!(name("black"), Name::Black);
        assert_eq!(name("red"), Name::Red);
        assert_eq!(name("green"), Name::Green);
        assert_eq!(name("yellow"), Name::Yellow);
        assert_eq!(name("blue"), Name::Blue);
        assert_eq!(name("magenta"), Name::Magenta);
        assert_eq!(name("cyan"), Name::Cyan);
        assert_eq!(name("white"), Name::White);
    }

    #[test]
    fn bright() {
        assert_eq!(name("brightblack"), Name::BrightBlack);
        assert_eq!(name("brightred"), Name::BrightRed);
        assert_eq!(name("brightgreen"), Name::BrightGreen);
        assert_eq!(name("brightyellow"), Name::BrightYellow);
        assert_eq!(name("brightblue"), Name::BrightBlue);
        assert_eq!(name("brightmagenta"), Name::BrightMagenta);
        assert_eq!(name("brightcyan"), Name::BrightCyan);
        assert_eq!(name("brightwhite"), Name::BrightWhite);
    }

    #[test]
    fn any_case() {
        for (input, expected) in [
            ("RED", Name::Red),
            ("Normal", Name::Normal),
            ("DEFAULT", Name::Default),
            ("BrightRed", Name::BrightRed),
            ("brightBLUE", Name::BrightBlue),
            ("BRIGHTWHITE", Name::BrightWhite),
        ] {
            assert_eq!(
                name(input),
                expected,
                "{input:?}: color names and the 'bright' prefix are case-insensitive, like in Git"
            );
        }
    }

    #[test]
    fn bright_only_applies_to_standard_colors() {
        for input in ["bright0", "bright1", "bright255", "bright-1", "bright#ff0010"] {
            assert!(
                Name::from_str(input).is_err(),
                "{input:?}: 'bright' may only precede one of the eight standard colors, like in Git"
            );
        }
    }

    #[test]
    fn ansi() {
        assert_eq!(name("255"), Name::Ansi(255));
        assert_eq!(name("0"), Name::Ansi(0));
    }

    #[test]
    fn hex() {
        assert_eq!(name("#ff0010"), Name::Rgb(255, 0, 16));
        assert_eq!(name("#ffffff"), Name::Rgb(255, 255, 255));
        assert_eq!(name("#000000"), Name::Rgb(0, 0, 0));
    }

    #[test]
    fn invalid() {
        assert!(Name::from_str("-2").is_err());
        assert!(Name::from_str("brightnormal").is_err());
        assert!(Name::from_str("brightdefault").is_err());
        assert!(Name::from_str("").is_err());
        assert!(Name::from_str("bright").is_err());
        assert!(Name::from_str("256").is_err());
        assert!(Name::from_str("#").is_err());
        assert!(Name::from_str("#fff").is_err());
        assert!(Name::from_str("#gggggg").is_err());
        assert!(Name::from_str("#=»©=").is_err());
    }
}

mod attribute {
    use std::str::FromStr;

    use gix_config_value::color::Attribute;

    fn attribute(input: &str) -> Attribute {
        Attribute::from_str(input).expect("valid color attribute")
    }

    #[test]
    fn non_inverted() {
        assert_eq!(attribute("reset"), Attribute::RESET);
        assert_eq!(attribute("bold"), Attribute::BOLD);
        assert_eq!(attribute("dim"), Attribute::DIM);
        assert_eq!(attribute("ul"), Attribute::UL);
        assert_eq!(attribute("blink"), Attribute::BLINK);
        assert_eq!(attribute("reverse"), Attribute::REVERSE);
        assert_eq!(attribute("italic"), Attribute::ITALIC);
        assert_eq!(attribute("strike"), Attribute::STRIKE);
    }

    #[test]
    fn inverted_no_dash() {
        assert_eq!(attribute("nobold"), Attribute::NO_BOLD);
        assert_eq!(attribute("nodim"), Attribute::NO_DIM);
        assert_eq!(attribute("noul"), Attribute::NO_UL);
        assert_eq!(attribute("noblink"), Attribute::NO_BLINK);
        assert_eq!(attribute("noreverse"), Attribute::NO_REVERSE);
        assert_eq!(attribute("noitalic"), Attribute::NO_ITALIC);
        assert_eq!(attribute("nostrike"), Attribute::NO_STRIKE);
    }

    #[test]
    fn inverted_dashed() {
        assert_eq!(attribute("no-bold"), Attribute::NO_BOLD);
        assert_eq!(attribute("no-dim"), Attribute::NO_DIM);
        assert_eq!(attribute("no-ul"), Attribute::NO_UL);
        assert_eq!(attribute("no-blink"), Attribute::NO_BLINK);
        assert_eq!(attribute("no-reverse"), Attribute::NO_REVERSE);
        assert_eq!(attribute("no-italic"), Attribute::NO_ITALIC);
        assert_eq!(attribute("no-strike"), Attribute::NO_STRIKE);
    }

    #[test]
    fn invalid() {
        assert!(Attribute::from_str("no-reset").is_err());
        assert!(Attribute::from_str("noreset").is_err());
        assert!(Attribute::from_str("a").is_err());
        assert!(Attribute::from_str("no bold").is_err());
        assert!(Attribute::from_str("").is_err());
        assert!(Attribute::from_str("no").is_err());
        assert!(Attribute::from_str("no-").is_err());
    }
}

mod from_git {
    use bstr::BStr;
    use gix_config_value::Color;

    #[test]
    fn reset() {
        assert_eq!(color("reset"), "reset");
        assert_eq!(color("RESET"), "reset");
        assert_eq!(color("red Reset"), "red reset");
    }

    #[test]
    fn empty() {
        assert_eq!(color(""), "");
    }

    #[test]
    fn at_most_two_colors() {
        assert!(try_color("red green blue").is_err());
    }

    #[test]
    fn attribute_before_color_name() {
        assert_eq!(color("bold red"), "red bold");
    }

    #[test]
    fn color_name_before_attribute() {
        assert_eq!(color("red bold"), "red bold");
    }

    #[test]
    fn attribute_fg_bg() {
        assert_eq!(color("ul blue red"), "blue red ul");
    }

    #[test]
    fn fg_bg_attribute() {
        assert_eq!(color("blue red ul"), "blue red ul");
    }

    #[test]
    fn multiple_attributes() {
        assert_eq!(
            color("blue bold dim ul blink reverse"),
            "blue bold dim ul blink reverse"
        );
    }

    #[test]
    fn reset_then_multiple_attributes() {
        assert_eq!(
            color("blue bold dim ul blink reverse reset"),
            "blue bold dim ul blink reverse reset"
        );
    }

    #[test]
    fn long_color_spec() {
        assert_eq!(
            color("254 255 bold dim ul blink reverse"),
            "254 255 bold dim ul blink reverse"
        );
        let input = "#ffffff #ffffff bold nobold dim nodim italic noitalic ul noul blink noblink reverse noreverse strike nostrike";
        let expected = "#ffffff #ffffff bold dim italic ul blink reverse strike nodim nobold noitalic noul noblink noreverse nostrike";
        assert_eq!(color(input), expected);
    }

    #[test]
    fn normal_default_can_clear_backgrounds() {
        assert_eq!(color("normal default"), "normal default");
    }

    #[test]
    fn color_names_ignore_case() {
        assert_eq!(color("RED brightBLUE bold"), "red brightblue bold");
    }

    #[test]
    fn default_can_combine_with_attributes() {
        assert_eq!(
            color("default default no-reverse bold"),
            "default default bold noreverse"
        );
    }

    fn color<'a>(name: impl Into<&'a BStr>) -> String {
        try_color(name).expect("input color is expected to be valid")
    }

    fn try_color<'a>(name: impl Into<&'a BStr>) -> crate::Result<String> {
        Ok(Color::try_from(name.into())?.to_string())
    }
}
