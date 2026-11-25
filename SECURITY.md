# Security Policy

## Resource exhaustion

Gitoxide uses fallible allocations and structural checks where practical, but does not bound the total memory used by an operation.
Compressed objects and delta chains can require much more memory than their size on disk.

Services processing untrusted repositories or packs should run those operations in separate worker processes with OS-enforced
memory limits. Limit worker concurrency and execution time as well, and handle worker termination as a failed operation.
Repository trust settings control access to configuration and related resources; they do not provide memory containment.

## Reporting a Vulnerability

Please feel free to [draft a GitHub advisory](https://github.com/GitoxideLabs/gitoxide/security/advisories/new), and I will work with you to disclose and or resolve the issue
responsibly.

If this doesn't seem like the right approach or there are questions, please feel free to reach out to the email used in Sebastian Thiel's commits.

Thank you.
