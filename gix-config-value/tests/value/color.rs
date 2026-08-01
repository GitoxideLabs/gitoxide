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

    #[test]
    fn non_bright() {
        assert_eq!(Name::from_str("normal"), Ok(Name::Normal));
        assert_eq!(Name::from_str("-1"), Ok(Name::Normal));
        assert_eq!(Name::from_str("default"), Ok(Name::Default));
        assert_eq!(Name::from_str("black"), Ok(Name::Black));
        assert_eq!(Name::from_str("red"), Ok(Name::Red));
        assert_eq!(Name::from_str("green"), Ok(Name::Green));
        assert_eq!(Name::from_str("yellow"), Ok(Name::Yellow));
        assert_eq!(Name::from_str("blue"), Ok(Name::Blue));
        assert_eq!(Name::from_str("magenta"), Ok(Name::Magenta));
        assert_eq!(Name::from_str("cyan"), Ok(Name::Cyan));
        assert_eq!(Name::from_str("white"), Ok(Name::White));
    }

    #[test]
    fn bright() {
        assert_eq!(Name::from_str("brightblack"), Ok(Name::BrightBlack));
        assert_eq!(Name::from_str("brightred"), Ok(Name::BrightRed));
        assert_eq!(Name::from_str("brightgreen"), Ok(Name::BrightGreen));
        assert_eq!(Name::from_str("brightyellow"), Ok(Name::BrightYellow));
        assert_eq!(Name::from_str("brightblue"), Ok(Name::BrightBlue));
        assert_eq!(Name::from_str("brightmagenta"), Ok(Name::BrightMagenta));
        assert_eq!(Name::from_str("brightcyan"), Ok(Name::BrightCyan));
        assert_eq!(Name::from_str("brightwhite"), Ok(Name::BrightWhite));
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
                Name::from_str(input),
                Ok(expected),
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
        assert_eq!(Name::from_str("255"), Ok(Name::Ansi(255)));
        assert_eq!(Name::from_str("0"), Ok(Name::Ansi(0)));
    }

    #[test]
    fn hex() {
        assert_eq!(Name::from_str("#ff0010"), Ok(Name::Rgb(255, 0, 16)));
        assert_eq!(Name::from_str("#ffffff"), Ok(Name::Rgb(255, 255, 255)));
        assert_eq!(Name::from_str("#000000"), Ok(Name::Rgb(0, 0, 0)));
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

    #[test]
    fn non_inverted() {
        assert_eq!(Attribute::from_str("reset"), Ok(Attribute::RESET));
        assert_eq!(Attribute::from_str("bold"), Ok(Attribute::BOLD));
        assert_eq!(Attribute::from_str("dim"), Ok(Attribute::DIM));
        assert_eq!(Attribute::from_str("ul"), Ok(Attribute::UL));
        assert_eq!(Attribute::from_str("blink"), Ok(Attribute::BLINK));
        assert_eq!(Attribute::from_str("reverse"), Ok(Attribute::REVERSE));
        assert_eq!(Attribute::from_str("italic"), Ok(Attribute::ITALIC));
        assert_eq!(Attribute::from_str("strike"), Ok(Attribute::STRIKE));
    }

    #[test]
    fn inverted_no_dash() {
        assert_eq!(Attribute::from_str("nobold"), Ok(Attribute::NO_BOLD));
        assert_eq!(Attribute::from_str("nodim"), Ok(Attribute::NO_DIM));
        assert_eq!(Attribute::from_str("noul"), Ok(Attribute::NO_UL));
        assert_eq!(Attribute::from_str("noblink"), Ok(Attribute::NO_BLINK));
        assert_eq!(Attribute::from_str("noreverse"), Ok(Attribute::NO_REVERSE));
        assert_eq!(Attribute::from_str("noitalic"), Ok(Attribute::NO_ITALIC));
        assert_eq!(Attribute::from_str("nostrike"), Ok(Attribute::NO_STRIKE));
    }

    #[test]
    fn inverted_dashed() {
        assert_eq!(Attribute::from_str("no-bold"), Ok(Attribute::NO_BOLD));
        assert_eq!(Attribute::from_str("no-dim"), Ok(Attribute::NO_DIM));
        assert_eq!(Attribute::from_str("no-ul"), Ok(Attribute::NO_UL));
        assert_eq!(Attribute::from_str("no-blink"), Ok(Attribute::NO_BLINK));
        assert_eq!(Attribute::from_str("no-reverse"), Ok(Attribute::NO_REVERSE));
        assert_eq!(Attribute::from_str("no-italic"), Ok(Attribute::NO_ITALIC));
        assert_eq!(Attribute::from_str("no-strike"), Ok(Attribute::NO_STRIKE));
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
