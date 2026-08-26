# Contributing

We are happy to have you and help you get started. If you have questions, feel free to
[start a discussion][discussions].

Before beginning implementation, open a [discussion][discussions] for any code submission expected to
add or modify more than ~500 source lines of code (SLOC).

We recommend running `just test` during development to assure CI is green before pushing. See the
[collaboration guide] for our workflow and the [development guide] for implementation practices.

## Prevent agent impersonation

AI agents communicating through a person's account must identify themselves, for example in issue or
PR descriptions and comments. AI assistance that does not replace the person as the speaker, such as
proofreading or wording polish, does not require identification.

Attributing AI assistance in commit metadata, for example with an `Assisted-by:` or `Co-authored-by:` trailer,
is welcome but not required.

[collaboration guide]: https://github.com/GitoxideLabs/gitoxide/blob/main/COLLABORATING.md
[development guide]: https://github.com/GitoxideLabs/gitoxide/blob/main/DEVELOPMENT.md
[discussions]: https://github.com/GitoxideLabs/gitoxide/discussions
