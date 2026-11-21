use std::{collections::BTreeMap, str::FromStr};

use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, ResultExt, bail, message};
use gix_hash::{Kind, ObjectId};

/// The effect of a validation diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Suppress the diagnostic.
    Ignore,
    /// Report the diagnostic without rejecting the object.
    Warning,
    /// Report the diagnostic and reject the object.
    Error,
}

impl FromStr for Severity {
    type Err = gix_error::Error;
    fn from_str(value: &str) -> Result<Self> {
        match value {
            "ignore" => Ok(Self::Ignore),
            "warn" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => bail!(message!("Unknown fsck severity {value:?}").validation()),
        }
    }
}

#[derive(Clone, Copy)]
enum DefaultSeverity {
    Fatal,
    Error,
    Warning,
    Info,
    Ignore,
}

impl DefaultSeverity {
    const fn resolve(self, strict: bool) -> Severity {
        match self {
            Self::Fatal | Self::Error => Severity::Error,
            Self::Warning if strict => Severity::Error,
            Self::Warning | Self::Info => Severity::Warning,
            Self::Ignore => Severity::Ignore,
        }
    }
}

macro_rules! message_ids {
    ($($variant:ident => ($name:literal, $severity:ident, $description:literal),)*) => {
        /// Stable Git-compatible identifiers for raw object validation diagnostics.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum MessageId {
            $(#[doc = $description] $variant,)*
        }
        impl MessageId {
            /// The identifier used by Git's fsck configuration.
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $name,)* }
            }
            /// A short description independent of the object or repository.
            pub const fn description(self) -> &'static str {
                match self { $(Self::$variant => $description,)* }
            }
            pub(super) const fn default_severity(self, strict: bool) -> Severity {
                match self { $(Self::$variant => DefaultSeverity::$severity.resolve(strict),)* }
            }
        }
        impl FromStr for MessageId {
            type Err = gix_error::Error;
            fn from_str(name: &str) -> Result<Self> {
                const IDS: &[MessageId] = &[$(MessageId::$variant,)*];
                IDS.iter().copied().find(|id| name.eq_ignore_ascii_case(id.as_str()))
                    .ok_or_else(|| message!("Unknown fsck message identifier {name:?}").not_found().raise())
            }
        }
    }
}

message_ids! {
    NulInHeader => ("nulInHeader", Fatal, "A header contains a NUL byte"),
    UnterminatedHeader => ("unterminatedHeader", Fatal, "The final header line has no newline"),
    BadHeaderContinuation => ("badHeaderContinuation", Error, "A continued header is malformed"),
    BadDate => ("badDate", Error, "An identity timestamp is invalid"),
    BadDateOverflow => ("badDateOverflow", Error, "An identity timestamp exceeds the supported range"),
    BadEmail => ("badEmail", Error, "An identity email field is malformed"),
    BadGpgsig => ("badGpgsig", Error, "A signature header is malformed"),
    BadName => ("badName", Error, "An identity name contains an unexpected delimiter"),
    BadObjectSha1 => ("badObjectSha1", Error, "A tag target object identifier is malformed"),
    BadParentSha1 => ("badParentSha1", Error, "A commit parent identifier is malformed"),
    BadTimezone => ("badTimezone", Error, "An identity timezone is malformed"),
    BadTree => ("badTree", Error, "A tree entry cannot be decoded"),
    BadTreeSha1 => ("badTreeSha1", Error, "A commit tree identifier is malformed"),
    BadType => ("badType", Error, "A tag target kind is invalid"),
    DuplicateEntries => ("duplicateEntries", Error, "A tree contains duplicate entry names"),
    GitAttributesBlob => ("gitattributesBlob", Error, "An attributes entry does not name a blob"),
    GitAttributesLarge => ("gitattributesLarge", Error, "An attributes blob exceeds the size limit"),
    GitAttributesLineLength => ("gitattributesLineLength", Error, "An attributes line exceeds the length limit"),
    GitAttributesMissing => ("gitattributesMissing", Error, "An attributes blob is unavailable"),
    GitModulesBlob => ("gitmodulesBlob", Error, "A submodule configuration entry does not name a blob"),
    GitModulesLarge => ("gitmodulesLarge", Error, "A submodule configuration exceeds the size limit"),
    GitModulesMissing => ("gitmodulesMissing", Error, "A submodule configuration blob is unavailable"),
    GitModulesName => ("gitmodulesName", Error, "A submodule name is unsafe"),
    GitModulesPath => ("gitmodulesPath", Error, "A submodule path is unsafe"),
    GitModulesSymlink => ("gitmodulesSymlink", Error, "A submodule configuration entry is a symbolic link"),
    GitModulesUpdate => ("gitmodulesUpdate", Error, "A submodule configuration requests command execution"),
    GitModulesUrl => ("gitmodulesUrl", Error, "A submodule URL is unsafe"),
    MissingAuthor => ("missingAuthor", Error, "A commit has no author header"),
    MissingCommitter => ("missingCommitter", Error, "A commit has no committer header"),
    MissingEmail => ("missingEmail", Error, "An identity has no email field"),
    MissingNameBeforeEmail => ("missingNameBeforeEmail", Error, "An identity email lacks its preceding name field"),
    MissingObject => ("missingObject", Error, "A tag has no object header"),
    MissingSpaceBeforeDate => ("missingSpaceBeforeDate", Error, "An identity date lacks its separating space"),
    MissingSpaceBeforeEmail => ("missingSpaceBeforeEmail", Error, "An identity email lacks its separating space"),
    MissingTag => ("missingTag", Error, "A tag name line is incomplete"),
    MissingTagEntry => ("missingTagEntry", Error, "A tag has no name header"),
    MissingTree => ("missingTree", Error, "A commit has no tree header"),
    MissingType => ("missingType", Error, "A tag target kind line is incomplete"),
    MissingTypeEntry => ("missingTypeEntry", Error, "A tag has no target kind header"),
    MultipleAuthors => ("multipleAuthors", Error, "A commit has multiple author headers"),
    TreeNotSorted => ("treeNotSorted", Error, "Tree entries do not follow Git's ordering"),
    UnknownType => ("unknownType", Error, "An object has an unknown kind"),
    ZeroPaddedDate => ("zeroPaddedDate", Error, "An identity timestamp has leading zeroes"),
    EmptyName => ("emptyName", Warning, "A tree entry has an empty name"),
    FullPathname => ("fullPathname", Warning, "A tree entry name contains a slash"),
    HasDot => ("hasDot", Warning, "A tree entry is named dot"),
    HasDotdot => ("hasDotdot", Warning, "A tree entry is named dot-dot"),
    HasDotgit => ("hasDotgit", Warning, "A tree entry aliases the Git directory"),
    LargePathname => ("largePathname", Warning, "A tree entry name exceeds the configured length"),
    NullSha1 => ("nullSha1", Warning, "A tree entry has a null object identifier"),
    NulInCommit => ("nulInCommit", Warning, "A commit body contains a NUL byte"),
    ZeroPaddedFilemode => ("zeroPaddedFilemode", Warning, "A tree entry mode has leading zeroes"),
    BadFilemode => ("badFilemode", Info, "A tree entry mode is nonstandard"),
    BadTagName => ("badTagName", Info, "An annotated tag contains an invalid ref name"),
    GitAttributesSymlink => ("gitattributesSymlink", Info, "An attributes entry is a symbolic link"),
    GitIgnoreSymlink => ("gitignoreSymlink", Info, "An ignore entry is a symbolic link"),
    GitModulesParse => ("gitmodulesParse", Info, "A submodule configuration cannot be parsed"),
    MailmapSymlink => ("mailmapSymlink", Info, "A mailmap entry is a symbolic link"),
    MissingTaggerEntry => ("missingTaggerEntry", Info, "A tag has no tagger identity"),
    ExtraHeaderEntry => ("extraHeaderEntry", Ignore, "A tag contains an unexpected extra header"),
}

/// Controls structural validation independently of a repository's configuration.
#[derive(Clone, Debug)]
pub struct Options {
    /// Promote ordinary warnings to errors, as Git's strict receive validation does.
    /// Informational messages such as missing taggers remain warnings.
    pub strict: bool,
    /// Maximum tree entry name length before reporting `largePathname` (4096 by default).
    pub max_tree_entry_length: usize,
    overrides: BTreeMap<MessageId, Severity>,
    skipped: gix_hashtable::HashSet<ObjectId>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            strict: false,
            max_tree_entry_length: 4096,
            overrides: BTreeMap::new(),
            skipped: Default::default(),
        }
    }
}

impl Options {
    /// Use Git's strict object policy, promoting ordinary warnings to errors.
    pub fn strict() -> Self {
        Self {
            strict: true,
            ..Self::default()
        }
    }

    /// Resolve a message's severity, applying explicit overrides after strict defaults.
    pub fn severity(&self, id: MessageId) -> Severity {
        self.overrides
            .get(&id)
            .copied()
            .unwrap_or_else(|| id.default_severity(self.strict))
    }

    /// Override a message. Fatal malformed headers cannot be demoted.
    pub fn set_severity(&mut self, id: MessageId, severity: Severity) -> Result<()> {
        if matches!(id, MessageId::NulInHeader | MessageId::UnterminatedHeader) && severity != Severity::Error {
            bail!(message!("Cannot demote fatal fsck message {}", id.as_str()).validation());
        }
        self.overrides.insert(id, severity);
        Ok(())
    }

    /// Parse a Git-style message setting, including `largePathname=error:<length>`.
    /// Unknown identifiers are distinguished by a not-found error so callers can warn or reject them.
    pub fn set_message(&mut self, name: &str, value: &str) -> Result<()> {
        let id: MessageId = name.parse()?;
        let (severity, limit) = if id == MessageId::LargePathname {
            value
                .split_once(':')
                .map_or((value, None), |(severity, limit)| (severity, Some(limit)))
        } else {
            (value, None)
        };
        let limit = limit
            .map(|limit| {
                limit
                    .parse::<usize>()
                    .or_raise(|| message("Invalid fsck pathname length"))
            })
            .transpose()?;
        self.set_severity(id, severity.parse()?)?;
        if let Some(limit) = limit {
            self.max_tree_entry_length = limit;
        }
        Ok(())
    }

    /// Suppress structural diagnostics for this object, without bypassing connectivity checks.
    pub fn skip_object(&mut self, id: ObjectId) {
        self.skipped.insert(id);
    }

    /// Whether an object is exempt from structural diagnostics.
    pub fn is_skipped(&self, id: &gix_hash::oid) -> bool {
        self.skipped.contains(id)
    }

    /// Read skip-list entries from bytes, accepting comments and surrounding whitespace.
    /// Malformed IDs and IDs of the wrong object format are rejected.
    pub fn extend_skip_list(&mut self, bytes: &[u8], hash: Kind) -> Result<()> {
        for line in bytes.lines() {
            let line = line.split(|byte| *byte == b'#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let id = ObjectId::from_hex(line).or_raise(|| message("Invalid object ID in fsck skip list"))?;
            if id.kind() != hash {
                bail!(message("Fsck skip-list object uses a different hash format").validation());
            }
            self.skip_object(id);
        }
        Ok(())
    }
}

/// A structured fsck diagnostic. Its message identifier supplies the description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// The object that caused the diagnostic.
    pub object_id: ObjectId,
    /// Git-compatible message identifier.
    pub id: MessageId,
    /// Effective severity after configuration and strict mode.
    pub severity: Severity,
}

impl Diagnostic {
    /// Construct a diagnostic unless its severity or skip-list entry suppresses it.
    pub fn new(object_id: ObjectId, id: MessageId, options: &Options) -> Option<Self> {
        let severity = options.severity(id);
        (severity != Severity::Ignore && !options.is_skipped(&object_id)).then_some(Self {
            object_id,
            id,
            severity,
        })
    }
}
