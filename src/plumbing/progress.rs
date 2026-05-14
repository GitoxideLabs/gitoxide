use std::{
    fmt::{Display, Formatter},
    io::StdoutLock,
};

#[derive(Clone)]
enum Usage {
    /// It's not reasonable to implement it as the prerequisites don't apply.
    NotApplicable(&'static str),
    /// We have no intention to implement it, but that can change if there is demand.
    NotPlanned(&'static str),
    /// We definitely want to implement this configuration value.
    Planned(&'static str),
    /// The configuration is already used, possibly with a given `deviation`.
    InUse(&'static str),
    /// Needs analysis, unclear how it works or what it does.
    Puzzled,
}
use Usage::*;

impl Display for Usage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Puzzled => f.write_str("❓")?,
            NotApplicable(reason) => write!(f, "not applicable because {reason}")?,
            NotPlanned(reason) => write!(f, "not planned || {reason}")?,
            Planned(note) => {
                if !note.is_empty() {
                    write!(f, "planned || {note}")?;
                }
            }
            InUse(deviation) => {
                if !deviation.is_empty() {
                    write!(f, "❗️{deviation}")?;
                }
            }
        }
        Ok(())
    }
}

impl Usage {
    pub const fn icon(&self) -> &'static str {
        match self {
            Puzzled => "❓",
            NotApplicable(_) => "❌",
            Planned(_) => "🕒",
            NotPlanned(_) => "🤔",
            InUse(deviation) => {
                if deviation.is_empty() {
                    "✅"
                } else {
                    "👌️"
                }
            }
        }
    }
}

#[derive(Clone)]
struct Record {
    config: &'static str,
    usage: Usage,
}

static GIT_CONFIG: &[Record] = &[
    Record {
        config: "attr.tree",
        usage: Planned("Decides where to read git attributes from - this will affect APIs"),
    },
    Record {
        config: "core.symlinks",
        usage: Planned("Needed to handle checkouts faithfully"),
    },
    Record {
        config: "core.hideDotFiles",
        usage: Planned("Seems useful, but needs demand from Windows users"),
    },
    Record {
        config: "core.packedGitWindowSize",
        usage: NotPlanned("An optimization for handling many large packs more efficiently seems unnecessary"),
    },
    Record {
        config: "core.packedGitLimit",
        usage: NotApplicable("We target 64-bit systems only and don't use a windowing mechanism"),
    },
    Record {
        config: "core.alternateRefsCommand",
        usage: NotPlanned(
            "There is no need as we can perform the required operation in-binary. This could happen though if there is a use-case and demand.",
        ),
    },
    Record {
        config: "core.alternateRefsPrefixes",
        usage: NotPlanned("Seems like a niche feature, but can be implemented if there is demand"),
    },
    Record {
        config: "core.compression",
        usage: Planned("Allow to remove similar hardcoded value - passing it through will be some effort"),
    },
    Record {
        config: "core.looseCompression",
        usage: Planned(""),
    },
    Record {
        config: "core.sparseCheckout",
        usage: Planned("We want to support huge repos and be the fastest in doing so"),
    },
    Record {
        config: "core.sparseCheckoutCone",
        usage: Planned("This is a nice improvement over 'sparseCheckout' alone and should one day be available too"),
    },
    Record {
        config: "core.gitProxy",
        usage: NotPlanned(
            "The transport mechanism works differently enough to not support it for now, but of course it's possible to add support if there is demand",
        ),
    },
    Record {
        config: "checkout.defaultRemote",
        usage: Planned("Needed for correct checkout behaviour, similar to what git does"),
    },
    Record {
        config: "core.untrackedCache",
        usage: Planned("Needed for fast worktree operation"),
    },
    Record {
        config: "checkout.guess",
        usage: Planned(""),
    },
    Record {
        config: "checkout.thresholdForParallelism",
        usage: NotApplicable("Parallelism is efficient enough to always run with benefit"),
    },
    Record {
        config: "feature.manyFiles",
        usage: Planned("Big repositories are on the roadmap"),
    },
    Record {
        config: "core.preloadIndex",
        usage: Planned(
            "It's enabled by default and allows parallel stat checks - it's using a lot of CPU for just minor performance boosts though",
        ),
    },
    Record {
        config: "commitGraph.generationVersion",
        usage: NotPlanned(
            "Couldn't find a test that would require corrected generation numbers, even 'git' has no test for this.",
        ),
    },
    Record {
        config: "commitGraph.maxNewFilters",
        usage: NotPlanned("Can be considered when the underlying feature is actually used or needed"),
    },
    Record {
        config: "commitGraph.readChangedPaths",
        usage: NotPlanned("Can be considered when the underlying feature is actually used or needed"),
    },
    Record {
        config: "index.sparse",
        usage: Planned("We can read sparse indices and support for it will be added early on"),
    },
    Record {
        config: "sparse.expectFilesOutsideOfPatterns",
        usage: Planned("A feature definitely worth having"),
    },
    Record {
        config: "submodule.recurse",
        usage: Planned(
            "Very relevant for doing the right thing during checkouts. Note that 'clone' isn't affected by it, even though we could make it so for good measure.",
        ),
    },
    Record {
        config: "submodule.propagateBranches",
        usage: NotPlanned("It is experimental, let's see how it pans out"),
    },
    Record {
        config: "submodule.alternateLocation",
        usage: NotPlanned("Not currently supported when we clone either"),
    },
    Record {
        config: "submodule.alternateErrorStrategy",
        usage: NotPlanned("Not currently supported when we clone either"),
    },
    Record {
        config: "submodule.fetchJobs",
        usage: Planned("Relevant for fetching"),
    },
    Record {
        config: "branch.autoSetupRebase",
        usage: Planned("For when we allow setting up upstream branches"),
    },
    Record {
        config: "branch.<name>.rebase",
        usage: Planned("For when we can merge, rebase should be supported"),
    },
    Record {
        config: "branch.<name>.description",
        usage: NotPlanned("No plan to implement 'format-patch' or 'request-pull' summary"),
    },
    Record {
        config: "core.fsync",
        usage: Planned("More safety for disk write operations is a good thing, definitely on the server"),
    },
    Record {
        config: "core.fsyncMethod",
        usage: Planned("Needed to support 'core.fsync'"),
    },
    Record {
        config: "core.sharedRepository",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "core.createObject",
        usage: NotPlanned(
            "It's valuable not to do writes unless needed on the lowest level, but we hope to avoid issues by not writing duplicate objects in the first place",
        ),
    },
    Record {
        config: "clone.filterSubmodules,",
        usage: Planned(
            "Currently object filtering isn't supported, a prerequisite for this, see '--filter=blob:none' for more",
        ),
    },
    Record {
        config: "clone.rejectShallow",
        usage: Planned("Probably trivial to implement once there is protocol support for shallow clones"),
    },
    Record {
        config: "receive.shallowUpdate",
        usage: NotPlanned(
            "It looks like a server-only setting that allows boundaries to change if refs are pushed that are outside of the boundary.",
        ),
    },
    Record {
        config: "fetch.recurseSubmodules",
        usage: Planned("Seems useful for 'cargo' as well"),
    },
    Record {
        config: "fetch.fsckObjects",
        usage: Puzzled,
    },
    Record {
        config: "fetch.fsck.<msg-id>",
        usage: Puzzled,
    },
    Record {
        config: "fetch.fsck.skipList",
        usage: Puzzled,
    },
    Record {
        config: "fetch.unpackLimit",
        usage: Planned(""),
    },
    Record {
        config: "fetch.prune",
        usage: Planned(""),
    },
    Record {
        config: "fetch.pruneTags",
        usage: Planned(""),
    },
    Record {
        config: "fetch.writeCommitGraph",
        usage: Planned(""),
    },
    Record {
        config: "fetch.parallel",
        usage: Planned(""),
    },
    Record {
        config: "fetch.showForcedUpdates",
        usage: NotApplicable("We don't support advice"),
    },
    Record {
        config: "fetch.output",
        usage: NotPlanned("'gix' might support it, but there is no intention on copying the 'git' CLI"),
    },
    Record {
        config: "remotes.<group>",
        usage: Planned(
            "Useful for multi-remote fetches as part of the standard API, maybe just 'group(name) -> Option<Vec<Remote>>'",
        ),
    },
    Record {
        config: "advice.updateSparsePath",
        usage: NotApplicable("gitoxide does not yet have an 'advice' system"),
    },
    Record {
        config: "core.sparseCheckout",
        usage: Planned(
            "Together with 'index.sparse' and 'core.sparseCheckoutCone', configures if the index should be written sparse or not",
        ),
    },
    Record {
        config: "core.sparseCheckoutCone",
        usage: Planned("Non-cone mode is deprecated but should still fail gracefully if encountered"),
    },
    Record {
        config: "core.splitIndex",
        usage: NotPlanned(
            "We don't want to be able to create split indices, but we will read them. It's (somewhat) superseded by sparse indices",
        ),
    },
    Record {
        config: "splitIndex.maxPercentageChange",
        usage: NotPlanned("Seems like it's superseded by sparse indices"),
    },
    Record {
        config: "splitIndex.sharedIndexExpire",
        usage: NotPlanned("Seems like it's superseded by sparse indices"),
    },
    Record {
        config: "index.sparse",
        usage: Planned(
            "Together with 'core.sparseCheckout' and 'core.sparseCheckoutCone', configures if the index should be written sparse or not",
        ),
    },
    Record {
        config: "index.version",
        usage: Planned(
            "Once V4 indices can be written, we need to be able to set a desired version. For now we write the smallest possible index version only.",
        ),
    },
    Record {
        config: "http.<url>.*",
        usage: Planned("Definitely needed for correctness, testing against baseline is a must"),
    },
    Record {
        config: "http.proxySSLCert",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.proxySSLKey",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.proxySSLCertPasswordProtected",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.proxySSLCAInfo",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.emptyAuth",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.delegation",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.cookieFile",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.saveCookies",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.curloptResolve",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslCipherList",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslCipherList",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslCert",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslKey",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslCertPasswordProtected",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslCertPasswordProtected",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslCAPath",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslBackend",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.pinnedPubkey",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.sslTry",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.maxRequests",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.minSessions",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.postBuffer",
        usage: Planned(
            "Relevant when implementing push, we should understand how memory allocation works when streaming",
        ),
    },
    Record {
        config: "http.noEPSV",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "http.<url>.*",
        usage: Planned(
            "It's a vital part of git configuration. It's unclear how to get a baseline from 'git' for this one.",
        ),
    },
    Record {
        config: "init.templateDir",
        usage: NotPlanned(
            "'git' expects this dir to be a valid git dir - I'd find additive template dirs more interesting, or changes done afterwards procedurally. Maybe this needs a 'init_or_open' semantic to be really useful",
        ),
    },
    Record {
        config: "sparse.expectFilesOutsideOfPatterns",
        usage: NotPlanned("TODO"),
    },
    Record {
        config: "remote.<name>.promisor",
        usage: Planned("Required for big monorepos, and typically used in conjunction with sparse indices"),
    },
    Record {
        config: "remote.<name>.partialCloneFilter",
        usage: Planned("Required for big monorepos, and typically used in conjunction with sparse indices"),
    },
    Record {
        config: "merge.directoryRenames",
        usage: NotPlanned("On demand"),
    },
    Record {
        config: "merge.verbosity",
        usage: NotApplicable("Only useful for Git CLI"),
    },
    Record {
        config: "status.renameLimit",
        usage: Planned(
            "Definitely needed to do status properly, even though it doesn't have to be there for day one. The same as 'diff.renameLimit'",
        ),
    },
    Record {
        config: "status.renames",
        usage: Planned("The same as 'diff.renames'"),
    },
    Record {
        config: "transfer.credentialsInUrl",
        usage: Planned(
            "Currently we are likely to expose passwords in errors or in other places, and it's better to by default not do that",
        ),
    },
    Record {
        config: "diff.*.cacheTextConv",
        usage: NotPlanned(
            "It seems to slow to do that, and persisting results to save a relatively cheap computation doesn't seem right",
        ),
    },
];

/// A programmatic way to record and display progress.
pub fn show_progress() -> anyhow::Result<()> {
    let sorted = {
        let mut v: Vec<_> = GIT_CONFIG.into();
        v.extend(gix::config::Tree.sections().iter().flat_map(|section| {
            fn to_record(key: &dyn gix::config::tree::Key) -> Record {
                let config = key.logical_name();
                let note = key.note().map(|note| match note {
                    gix::config::tree::Note::Deviation(n) | gix::config::tree::Note::Informative(n) => n.to_string(),
                });
                let link = key.link().map(|link| match link {
                    gix::config::tree::Link::FallbackKey(key) => {
                        format!("fallback is '{fallback}'", fallback = key.logical_name())
                    }
                    gix::config::tree::Link::EnvironmentOverride(name) => format!("overridden by '{name}'"),
                });

                let deviation = match (note, link) {
                    (Some(n), Some(l)) => format!("{n}. {l}"),
                    (Some(n), None) | (None, Some(n)) => n,
                    (None, None) => "".to_string(),
                };

                let deviation = &*Box::leak(deviation.into_boxed_str());
                Record {
                    config: Box::leak(config.into_boxed_str()),
                    usage: InUse(deviation),
                }
            }
            section
                .sub_sections()
                .iter()
                .flat_map(|sub_section| sub_section.keys().iter().map(|key| to_record(*key)))
                .chain(section.keys().iter().map(|key| to_record(*key)))
        }));
        v.sort_by_key(|r| r.config);
        v.dedup_by_key(|r| r.config);
        v
    };

    let mut perfect = 0;
    let mut deviation = 0;
    let mut notplanned = 0;
    let mut not_applicable = 0;
    let mut planned = 0;

    for s in &sorted {
        match s.usage {
            NotApplicable(_) => not_applicable += 1,
            NotPlanned(_) => notplanned += 1,
            Planned(_) => planned += 1,
            InUse(dev) => {
                if dev.is_empty() {
                    perfect += 1;
                } else {
                    deviation += 1;
                }
            }
            Puzzled => {}
        }
    }

    let width: Option<usize> = terminal_size::terminal_size()
        .map(|(width, _height)| width.0)
        .map(std::convert::Into::into);

    let mut stdout = std::io::stdout().lock();
    for Record { config, usage } in &sorted {
        use std::io::Write;
        write!(stdout, "{icon} {config: <50}: ", icon = usage.icon())?;

        if let Some(width) = width {
            write_with_linewrap(&mut stdout, &usage.to_string(), width)?;
        } else {
            writeln!(stdout, "{usage}")?;
        }
    }

    println!(
        "\nTotal records: {nr_sorted} ({perfect_icon} = {perfect}, {deviation_icon} = {deviation}, {planned_icon} = {planned}, {ondemand_icon} = {notplanned}, {not_applicable_icon} = {not_applicable})",
        nr_sorted = sorted.len(),
        perfect_icon = InUse("").icon(),
        deviation_icon = InUse("dev").icon(),
        planned_icon = Planned("").icon(),
        ondemand_icon = NotPlanned("").icon(),
        not_applicable_icon = NotApplicable("").icon(),
    );
    Ok(())
}

fn write_with_linewrap(stdout: &mut StdoutLock<'_>, text: &str, width: usize) -> Result<(), std::io::Error> {
    use std::io::Write;
    let icon_and_config_width = 55;
    let width_after_config = width.saturating_sub(icon_and_config_width);
    let mut idx = 0;
    for word in text.split(' ') {
        // +1 for the space after each word
        let word_len = word.chars().count() + 1;

        if idx + word_len > width_after_config {
            writeln!(stdout)?;
            for _ in 0..icon_and_config_width {
                write!(stdout, " ")?;
            }
            idx = 0;
        }

        write!(stdout, "{word} ")?;
        idx += word_len;
    }
    writeln!(stdout)?;
    Ok(())
}
