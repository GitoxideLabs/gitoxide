use gix_object::{CommitRef, CommitRefIter};

use crate::fixture_name;

const SIGNATURE: & [u8] = b"-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEEdjYp/sh4j8NRKLX27gKdHl60AwAFAl7q2DsACgkQ7gKdHl60\nAwDvewgAkL5UjEztzeVXlzceom0uCrAkCw9wSGLTmYcMKW3JwEaTRgQ4FX+sDuFT\nLZ8DoPu3UHUP0QnKrUwHulTTlKcOAvsczHbVPIKtXCxo6QpUfhsJQwz/J29kiE4L\nsOd+lqKGnn4oati/de2xwqNGi081fO5KILX75z6KfsAe7Qz7R3jxRF4uzHI033O+\nJc2Y827XeaELxW40SmzoLanWgEcdreXf3PstXEWW77CAu0ozXmvYj56vTviVybxx\nG7bc8lwc+SSKVe2VVB+CCfVbs0i541gmghUpZfMhUgaqttcCH8ysrUJDhne1BLG8\nCrOJIWTwAeEDtomV1p76qrMeqr1GFg==\n=qlSN\n-----END PGP SIGNATURE-----\n";

const LONG_MESSAGE: &str = "Merge tag 'thermal-v5.8-rc1' of git://git.kernel.org/pub/scm/linux/kernel/git/thermal/linux

Pull thermal updates from Daniel Lezcano:

 - Add the hwmon support on the i.MX SC (Anson Huang)

 - Thermal framework cleanups (self-encapsulation, pointless stubs,
   private structures) (Daniel Lezcano)

 - Use the PM QoS frequency changes for the devfreq cooling device
   (Matthias Kaehlcke)

 - Remove duplicate error messages from platform_get_irq() error
   handling (Markus Elfring)

 - Add support for the bandgap sensors (Keerthy)

 - Statically initialize .get_mode/.set_mode ops (Andrzej Pietrasiewicz)

 - Add Renesas R-Car maintainer entry (Niklas Söderlund)

 - Fix error checking after calling ti_bandgap_get_sensor_data() for the
   TI SoC thermal (Sudip Mukherjee)

 - Add latency constraint for the idle injection, the DT binding and the
   change the registering function (Daniel Lezcano)

 - Convert the thermal framework binding to the Yaml schema (Amit
   Kucheria)

 - Replace zero-length array with flexible-array on i.MX 8MM (Gustavo A.
   R. Silva)

 - Thermal framework cleanups (alphabetic order for heads, replace
   module.h by export.h, make file naming consistent) (Amit Kucheria)

 - Merge tsens-common into the tsens driver (Amit Kucheria)

 - Fix platform dependency for the Qoriq driver (Geert Uytterhoeven)

 - Clean up the rcar_thermal_update_temp() function in the rcar thermal
   driver (Niklas Söderlund)

 - Fix the TMSAR register for the TMUv2 on the Qoriq platform (Yuantian
   Tang)

 - Export GDDV, OEM vendor variables, and don't require IDSP for the
   int340x thermal driver - trivial conflicts fixed (Matthew Garrett)

* tag 'thermal-v5.8-rc1' of git://git.kernel.org/pub/scm/linux/kernel/git/thermal/linux: (48 commits)
  thermal/int340x_thermal: Don't require IDSP to exist
  thermal/int340x_thermal: Export OEM vendor variables
  thermal/int340x_thermal: Export GDDV
  thermal: qoriq: Update the settings for TMUv2
  thermal: rcar_thermal: Clean up rcar_thermal_update_temp()
  thermal: qoriq: Add platform dependencies
  drivers: thermal: tsens: Merge tsens-common.c into tsens.c
  thermal/of: Rename of-thermal.c
  thermal/governors: Prefix all source files with gov_
  thermal/drivers/user_space: Sort headers alphabetically
  thermal/drivers/of-thermal: Sort headers alphabetically
  thermal/drivers/cpufreq_cooling: Replace module.h with export.h
  thermal/drivers/cpufreq_cooling: Sort headers alphabetically
  thermal/drivers/clock_cooling: Include export.h
  thermal/drivers/clock_cooling: Sort headers alphabetically
  thermal/drivers/thermal_hwmon: Include export.h
  thermal/drivers/thermal_hwmon: Sort headers alphabetically
  thermal/drivers/thermal_helpers: Include export.h
  thermal/drivers/thermal_helpers: Sort headers alphabetically
  thermal/core: Replace module.h with export.h
  ...
";

const MERGE_TAG: &str = "object 8d485da0ddee79d0e6713405694253d401e41b93
type commit
tag thermal-v5.8-rc1
tagger Daniel Lezcano <daniel.lezcano@linaro.org> 1591979433 +0200

- Add the hwmon support on the i.MX SC (Anson Huang)

- Thermal framework cleanups (self-encapsulation, pointless stubs,
  private structures) (Daniel Lezcano)

- Use the PM QoS frequency changes for the devfreq cooling device (Matthias
  Kaehlcke)

- Remove duplicate error messages from platform_get_irq() error handling
  (Markus Elfring)

- Add support for the bandgap sensors (Keerthy)

- Statically initialize .get_mode/.set_mode ops (Andrzej Pietrasiewicz)

- Add Renesas R-Car maintainer entry (Niklas Söderlund)

- Fix error checking after calling ti_bandgap_get_sensor_data() for the TI SoC
  thermal (Sudip Mukherjee)

- Add latency constraint for the idle injection, the DT binding and the change
  the registering function (Daniel Lezcano)

- Convert the thermal framework binding to the Yaml schema (Amit Kucheria)

- Replace zero-length array with flexible-array on i.MX 8MM (Gustavo A. R. Silva)

- Thermal framework cleanups (alphabetic order for heads, replace module.h by
  export.h, make file naming consistent) (Amit Kucheria)

- Merge tsens-common into the tsens driver (Amit Kucheria)

- Fix platform dependency for the Qoriq driver (Geert Uytterhoeven)

- Clean up the rcar_thermal_update_temp() function in the rcar thermal driver
  (Niklas Söderlund)

- Fix the TMSAR register for the TMUv2 on the Qoriq platform (Yuantian Tang)

- Export GDDV, OEM vendor variables, and don't require IDSP for the int340x
  thermal driver - trivial conflicts fixed (Matthew Garrett)
-----BEGIN PGP SIGNATURE-----

iQEzBAABCAAdFiEEGn3N4YVz0WNVyHskqDIjiipP6E8FAl7jra8ACgkQqDIjiipP
6E+ugAgApBF6FsHoonWIvoSrzBrrbU2oqhEJA42Mx+iY/UnXi01I79vZ/8WpZt7M
D1J01Kf0PUhRbywoKaoCX3Oh9ZO9PKq4N9ZC8yqdoD6GLl+rC9Wmr7Ui+c80klcv
M9rYhpPYfNXTFj0saSbbFWNNhP4TvhzGsNj8foYVQDKyhjbSmNE5ipZlbmP23jlr
O53SmJAwS5zxLOd8QA5nfSWP9FYYMuCR2AHj8BUCmxiAjXZLPNB/Hz2RRBr7q0MF
zRo/4HJ04mSQYp0kluP/EBhz9g2wM/htIPyWRveB/ByKEYt3UNKjB++PJmPbu5UG
dS3aXZhRfaPqpdsWrMB9fY7ll+oyfw==
=T+RI
-----END PGP SIGNATURE-----
";

const OTHER_SIGNATURE: &[u8] = b"-----BEGIN PGP SIGNATURE-----

wsBcBAABCAAQBQJeqxW4CRBK7hj4Ov3rIwAAdHIIAFD98qgN/k8ybukCLf6kpzvi
5V8gf6BflONXc/oIDySurW7kfS9/r6jOgu08UN8KlQx4Q4g8yY7PROABhwGI70B3
+mHPFcParQf5FBDDZ3GNNpJdlaI9eqzEnFk8AmHmyKHfuGLoclXUObXQ3oe3fmT7
QdTC7JTyk/bPnZ9HQKw7depa3+7Kw4wv4DG8QcW3BG6B9bcE15qaWmOiq0ryRXsv
k7D0LqGSXjU5wrQrKnemC7nWhmQsqaXDe89XXmliClCAx4/bepPiXK0eT/DNIKUr
iyBBl69jASy41Ug/BlFJbw4+ItkShpXwkJKuBBV/JExChmvbxYWaS7QnyYC9UO0=
=HLmy
-----END PGP SIGNATURE-----

";

mod method {
    use gix_object::CommitRef;
    use pretty_assertions::assert_eq;

    use crate::{fixture_name, hex_to_id};

    #[test]
    fn tree() -> crate::Result {
        let fixture = fixture_name("commit", "unsigned.txt");
        let commit = CommitRef::from_bytes(&fixture)?;
        assert_eq!(commit.tree(), hex_to_id("1b2dfb4ac5e42080b682fc676e9738c94ce6d54d"));
        assert_eq!(commit.tree, "1b2dfb4ac5e42080b682fc676e9738c94ce6d54d");
        Ok(())
    }
}

#[test]
fn invalid() {
    let fixture = fixture_name("commit", "unsigned.txt");
    let partial_commit = &fixture[..fixture.len() / 2];
    assert_eq!(
        CommitRef::from_bytes(partial_commit).unwrap_err().to_string(),
        if cfg!(feature = "verbose-object-parsing-errors") {
            "object parsing failed at ``\nexpected `author <signature>`"
        } else {
            "object parsing failed"
        }
    );
    assert_eq!(
        CommitRefIter::from_bytes(partial_commit)
            .take_while(Result::is_ok)
            .count(),
        1,
        "we can decode some fields before failing"
    );
}

mod from_bytes;
mod iter;
mod message;
