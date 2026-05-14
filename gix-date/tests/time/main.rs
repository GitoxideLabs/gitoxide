use gix_date::Time;

mod baseline;
mod format;
mod parse;

#[test]
fn is_set() {
    assert!(!Time::default().is_set());
    assert!(
        Time {
            seconds: 1,
            ..Default::default()
        }
        .is_set()
    );
}

mod write_to {
    use bstr::ByteSlice;
    use gix_date::{SecondsSinceUnixEpoch, Time, parse::TimeBuf};

    #[test]
    fn invalid() {
        let time = Time {
            seconds: 0,
            offset: (100 * 60 * 60) + 30 * 60,
        };
        let err = time.write_to(&mut Vec::new()).unwrap_err();
        assert_eq!(err.to_string(), "Cannot represent offsets larger than +-9900");
    }

    #[test]
    fn valid_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
        for (time, expected) in [
            (
                Time {
                    seconds: SecondsSinceUnixEpoch::MAX,
                    offset: 0,
                },
                "9223372036854775807 +0000",
            ),
            (
                Time {
                    seconds: SecondsSinceUnixEpoch::MIN,
                    offset: 0,
                },
                "-9223372036854775808 +0000",
            ),
            (
                Time {
                    seconds: 500,
                    offset: 9000,
                },
                "500 +0230",
            ),
            (
                Time {
                    seconds: 189009009,
                    offset: -36000,
                },
                "189009009 -1000",
            ),
            (Time { seconds: 0, offset: 0 }, "0 +0000"),
            (
                Time {
                    seconds: 0,
                    offset: -24 * 60 * 60,
                },
                "0 -2400",
            ),
            (
                Time {
                    seconds: 0,
                    offset: 24 * 60 * 60,
                },
                "0 +2400",
            ),
            (
                Time {
                    seconds: 0,
                    offset: (25 * 60 * 60) + 30 * 60,
                },
                "0 +2530",
            ),
            (
                Time {
                    seconds: 0,
                    offset: (-25 * 60 * 60) - 30 * 60,
                },
                "0 -2530",
            ),
            (
                Time {
                    seconds: 0,
                    offset: (99 * 60 * 60) + 59 * 60,
                },
                "0 +9959",
            ),
        ] {
            let mut output = Vec::new();
            time.write_to(&mut output)?;
            assert_eq!(output.as_bstr(), expected);
            assert_eq!(time.size(), output.len());

            let mut buf = TimeBuf::default();
            assert_eq!(time.to_str(&mut buf), expected);

            let actual = output.as_bstr().to_string().parse::<Time>().expect("round-trippable");
            assert_eq!(time, actual);
        }
        Ok(())
    }

    #[test]
    fn max() -> gix_testtools::Result {
        let mut buf = Vec::new();
        Time::MAX.write_to(&mut buf)?;
        assert_eq!(Time::MAX.size(), 25, "The serialized size of the largest possible time");

        let expected = "9223372036854775807 +9959";
        assert_eq!(buf.as_bstr(), expected);
        assert_eq!(buf.len(), Time::MAX.size());

        let mut buf = TimeBuf::default();
        assert_eq!(Time::MAX.to_str(&mut buf), expected);
        Ok(())
    }

    #[test]
    fn min() -> gix_testtools::Result {
        let mut buf = Vec::new();
        Time::MIN.write_to(&mut buf)?;
        assert_eq!(
            Time::MIN.size(),
            26,
            "The serialized size of the smallest possible time"
        );

        let expected = "-9223372036854775808 -9959";
        assert_eq!(buf.as_bstr(), expected);
        assert_eq!(buf.len(), Time::MIN.size());

        let mut buf = TimeBuf::default();
        assert_eq!(Time::MIN.to_str(&mut buf), expected);
        Ok(())
    }
}
