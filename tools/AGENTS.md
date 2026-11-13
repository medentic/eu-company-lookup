# Tooling and evidence instructions

Read [README.md](README.md) and the linked development guides before changing
collectors or reports. Root [agent instructions](../AGENTS.md) still apply.

- Prefer offline tool tests and explicit replay. Live requests require the user's
  bounded selection; never run every ignored Rust test or an unbounded queue.
- Preserve observations, source hashes, review revisions and append-only journals.
  Do not rewrite expected output solely to make a regression pass.
- Report successful, negative, operationally failed and ambiguous observations
  separately. A completed request count is not a count of reviewed identifiers.
- Keep resumability and locking intact. Do not stop or clean up another collector's
  active process, executable or evidence directory.
- Use synthetic records in committed tool tests. Keep real queues, responses and
  provenance local-only, including in Git history and generated archives.
- Bound disposable logging; keep mandatory audit logs complete. Do not confuse
  regenerable reports with original evidence when reclaiming space.
