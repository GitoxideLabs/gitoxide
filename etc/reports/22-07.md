Despite having to put all efforts into `cargo` and everything related to cloning, I think this time of the year makes it a bit harder to kick off such efforts given summer vacations and all. Thus far I haven't gotten feedback on how to proceed with the `gitoxide` integration and decided to get some long-standing issues off my desk before trying harder to get a response.

And the one crate that has been a huge thorn in my eye was `git-config`, so I thought to myself: let's get this done, quickly. And what followed was _a whole month_ of restless work to get the crate towards 1.0.

## Pushing `git-config` towards 1.0

While Svetlin was already making valuable contributions to add support for handling `include.path` and conditional includes via `includeIf`, the crate lacked behind in 'style', naming, and code structure, while being generally quite unknown to me which I considered technical debt. When taking out the loan by merging without proper review, I was quite aware one day it had to be paid back.
So I started combing through each line of code to not only make it similar in structure to the ones in its sibling crates, but also to validate their functionality. All 4500 lines!

###### testing

In the course of this, 127 tests were added on top of the already impressive test suite to nearly 300, luring out a surprising amount of bugs in all levels of the library. I also revived and for the first time ran the fuzzer, powered by `cargo-fuzz`, to try over a billion different inputs for the parser. That run though was after it immediately found a crash in the parser, showing me that from now on fuzzing of parsers will be standard fare. Thanks to `cargo fuzz` it's also super easy to do, something that is a game changer for me as someone who always thought it must be too complicated to setup.

###### fuzzing

All that work also led to various performance improvements that allowed to run the fuzzer with 2x the throughput indicating the parser became twice as fast at least in the case that no allocation is performed.

Windows support was seemingly perfected as I was lucky enough to find a couple of carriage-return related issues.

###### lossless serialization

Something absolutely worth highlighting is the absolutely wonderful serialization capability that was there all the time without my knowledge. How many times did I think I should reduce all that complexity that was necessary to support lossless parsing and mutation of configuration files, and I am absolutely glad I didn't. Instead I was able to find out that, after a couple of tweaks, it was absolutely 'perfect', as it will reproduce a parsed config file perfectly when serializing it, and all that while making it accessible programmatically. I was surprised myself that even the strangest round-trip tests I could imagine did operate without any loss.

###### filtered value access

Some work went into making it possible to filter values by metadata, like the source of the value, or the associated ownership of its source file. That way it becomes possible to securely query git configuration file and simply ignore values that might be controlled by an attacker. That way, `gitoxide` won't have to reject operating on a repository entirely, but instead only uses values it can trust.

###### cascaded and complete configuration

And last but not least, one critical feature was somewhat missing, being the ability to flexibly load, resolve includes and combine git configuration files from various sources. With an API for that now in place, `git-repository` could finally perform a complete bootstrap sequence and provide cascaded and fully resolved configuration. Now for the first time, the startup is not only more correct than ever, but the API user is also able to query committer and author information.

## Community

### Pathspec-Parsing

We are nearly there and the parsing part of pathspec should be merged in the next couple of days. Let's see when implementing the corresponding matching is possible - for now it's not even on the list as it won't be required to deal move the `cargo` integration forward

## Rust Foundation sponsorship: cargo shallow clones update

This section I want to keep in all sponsor updates moving forward to the end of the year.

Early after the grant timeframe began I reached out via the shallow clone ticket to get a conversation going on how to proceed, but will have to try harder to reach the folks who can provide the answers. Even though I took some liberty in finishing `git-config` it's clear that soon I have to align my work towards integrating `gitoxide` into `cargo` with or without feedback, to be able to implement what's needed to make it superior to what `git2` is currently offering.

At this time there is no reason to worry, but the clock is ticking for me :D.

## Outlook

With `git-config` being off the table and a source of joy, I will push `rev-spec` resolution next as it's started and thus should be finished, despite not being directly needed for solving any `cargo` issue either. However, it shouldn't take long™️, and then the integration work can finally begin. Technically `gitoxide` can already perform clones, and it will be time to bring all the parts together into `git-repository`, and I am sure it will be fun and very enlightening.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).
